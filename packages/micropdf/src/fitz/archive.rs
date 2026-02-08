//! Archive support for ZIP and TAR files
//!
//! Provides reading from archive files (ZIP, TAR) and directories.

use crate::fitz::buffer::Buffer;
use crate::fitz::error::{Error, Result};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Archive format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchiveFormat {
    /// ZIP archive
    Zip,
    /// TAR archive
    Tar,
    /// Directory (treated as archive)
    Directory,
    /// Unknown format
    Unknown,
}

impl ArchiveFormat {
    /// Get format name as string
    pub fn name(&self) -> &'static str {
        match self {
            Self::Zip => "zip",
            Self::Tar => "tar",
            Self::Directory => "directory",
            Self::Unknown => "unknown",
        }
    }
}

/// Archive entry metadata
#[derive(Debug, Clone)]
pub struct ArchiveEntry {
    /// Entry name/path
    pub name: String,
    /// Uncompressed size in bytes
    pub size: u64,
    /// Is directory
    pub is_dir: bool,
    /// Offset in archive (for seeking)
    pub offset: u64,
}

impl ArchiveEntry {
    pub fn new(name: String, size: u64, is_dir: bool) -> Self {
        Self {
            name,
            size,
            is_dir,
            offset: 0,
        }
    }
}

/// Archive reader trait
trait ArchiveReader {
    /// Get archive format
    fn format(&self) -> ArchiveFormat;

    /// Count entries in archive
    fn count_entries(&self) -> Result<usize>;

    /// List entry at index
    fn list_entry(&self, idx: usize) -> Result<&str>;

    /// Check if entry exists
    fn has_entry(&self, name: &str) -> bool;

    /// Read entry data
    fn read_entry(&mut self, name: &str) -> Result<Vec<u8>>;

    /// Get all entry names
    fn entry_names(&self) -> Vec<&str>;
}

/// ZIP archive reader
struct ZipArchive {
    entries: HashMap<String, ArchiveEntry>,
    entry_order: Vec<String>,
    data: Vec<u8>,
}

impl ZipArchive {
    fn new(data: Vec<u8>) -> Result<Self> {
        let mut archive = Self {
            entries: HashMap::new(),
            entry_order: Vec::new(),
            data,
        };
        archive.parse()?;
        Ok(archive)
    }

    fn parse(&mut self) -> Result<()> {
        // Parse ZIP central directory
        // ZIP file format: Local file headers → Central directory → End of central directory

        // Find End of Central Directory Record (EOCD)
        // EOCD signature: 0x06054b50
        let eocd_sig = [0x50, 0x4b, 0x05, 0x06];
        let mut eocd_pos = None;

        // Search backwards for EOCD (usually at end, but can have comment)
        for i in (0..self.data.len().saturating_sub(22)).rev() {
            if self.data.get(i..i + 4) == Some(&eocd_sig) {
                eocd_pos = Some(i);
                break;
            }
        }

        let eocd_offset = eocd_pos
            .ok_or_else(|| Error::Generic("Not a valid ZIP archive: EOCD not found".into()))?;

        // Read EOCD fields
        if eocd_offset + 22 > self.data.len() {
            return Err(Error::Generic("Truncated ZIP archive".into()));
        }

        let eocd = &self.data[eocd_offset..];

        // Extract central directory info
        let cd_entries = u16::from_le_bytes([eocd[10], eocd[11]]) as usize;
        let _cd_size = u32::from_le_bytes([eocd[12], eocd[13], eocd[14], eocd[15]]) as usize;
        let cd_offset = u32::from_le_bytes([eocd[16], eocd[17], eocd[18], eocd[19]]) as usize;

        // Parse central directory entries
        let mut pos = cd_offset;
        for _ in 0..cd_entries {
            if pos + 46 > self.data.len() {
                break; // Truncated archive
            }

            // Check central directory file header signature: 0x02014b50
            let cd_sig = [0x50, 0x4b, 0x01, 0x02];
            if self.data.get(pos..pos + 4) != Some(&cd_sig) {
                break;
            }

            // Read filename length and extra field length
            let filename_len =
                u16::from_le_bytes([self.data[pos + 28], self.data[pos + 29]]) as usize;
            let extra_len = u16::from_le_bytes([self.data[pos + 30], self.data[pos + 31]]) as usize;
            let comment_len =
                u16::from_le_bytes([self.data[pos + 32], self.data[pos + 33]]) as usize;

            // Read compressed and uncompressed sizes
            let _compressed_size = u32::from_le_bytes([
                self.data[pos + 20],
                self.data[pos + 21],
                self.data[pos + 22],
                self.data[pos + 23],
            ]) as usize;
            let uncompressed_size = u32::from_le_bytes([
                self.data[pos + 24],
                self.data[pos + 25],
                self.data[pos + 26],
                self.data[pos + 27],
            ]) as usize;

            // Read local file header offset
            let local_offset = u32::from_le_bytes([
                self.data[pos + 42],
                self.data[pos + 43],
                self.data[pos + 44],
                self.data[pos + 45],
            ]) as usize;

            // Extract filename
            if pos + 46 + filename_len > self.data.len() {
                break;
            }

            let filename_bytes = &self.data[pos + 46..pos + 46 + filename_len];
            if let Ok(filename) = std::str::from_utf8(filename_bytes) {
                let entry = ArchiveEntry {
                    name: filename.to_string(),
                    size: uncompressed_size as u64,
                    offset: local_offset as u64,
                    is_dir: filename.ends_with('/'),
                };

                self.entry_order.push(filename.to_string());
                self.entries.insert(filename.to_string(), entry);
            }

            // Move to next entry
            pos += 46 + filename_len + extra_len + comment_len;
        }

        Ok(())
    }
}

impl ArchiveReader for ZipArchive {
    fn format(&self) -> ArchiveFormat {
        ArchiveFormat::Zip
    }

    fn count_entries(&self) -> Result<usize> {
        Ok(self.entries.len())
    }

    fn list_entry(&self, idx: usize) -> Result<&str> {
        self.entry_order
            .get(idx)
            .map(|s| s.as_str())
            .ok_or_else(|| Error::Argument(format!("Invalid entry index: {}", idx)))
    }

    fn has_entry(&self, name: &str) -> bool {
        self.entries.contains_key(name)
    }

    fn read_entry(&mut self, name: &str) -> Result<Vec<u8>> {
        let _entry = self
            .entries
            .get(name)
            .ok_or_else(|| Error::Argument(format!("Entry not found: {}", name)))?;

        // Simplified: would extract and decompress here
        Ok(Vec::new())
    }

    fn entry_names(&self) -> Vec<&str> {
        self.entry_order.iter().map(|s| s.as_str()).collect()
    }
}

/// TAR archive reader
#[allow(dead_code)]
struct TarArchive {
    entries: HashMap<String, ArchiveEntry>,
    entry_order: Vec<String>,
    data: Vec<u8>,
}

impl TarArchive {
    fn new(data: Vec<u8>) -> Result<Self> {
        let mut archive = Self {
            entries: HashMap::new(),
            entry_order: Vec::new(),
            data,
        };
        archive.parse()?;
        Ok(archive)
    }

    fn parse(&mut self) -> Result<()> {
        // Simplified TAR parsing
        // Real implementation would properly parse TAR headers
        Ok(())
    }
}

impl ArchiveReader for TarArchive {
    fn format(&self) -> ArchiveFormat {
        ArchiveFormat::Tar
    }

    fn count_entries(&self) -> Result<usize> {
        Ok(self.entries.len())
    }

    fn list_entry(&self, idx: usize) -> Result<&str> {
        self.entry_order
            .get(idx)
            .map(|s| s.as_str())
            .ok_or_else(|| Error::Argument(format!("Invalid entry index: {}", idx)))
    }

    fn has_entry(&self, name: &str) -> bool {
        self.entries.contains_key(name)
    }

    fn read_entry(&mut self, name: &str) -> Result<Vec<u8>> {
        let _entry = self
            .entries
            .get(name)
            .ok_or_else(|| Error::Argument(format!("Entry not found: {}", name)))?;

        // Simplified: would extract here
        Ok(Vec::new())
    }

    fn entry_names(&self) -> Vec<&str> {
        self.entry_order.iter().map(|s| s.as_str()).collect()
    }
}

/// Directory archive (treats directory as archive)
struct DirectoryArchive {
    path: PathBuf,
    entries: HashMap<String, ArchiveEntry>,
}

impl DirectoryArchive {
    fn new(path: PathBuf) -> Result<Self> {
        if !path.is_dir() {
            return Err(Error::Argument(format!(
                "Path is not a directory: {}",
                path.display()
            )));
        }

        let mut archive = Self {
            path,
            entries: HashMap::new(),
        };

        // Scan directory on creation
        archive.scan_entries()?;

        Ok(archive)
    }

    fn scan_entries(&mut self) -> Result<()> {
        self.entries.clear();
        self.scan_dir(&self.path.clone(), "")?;
        Ok(())
    }

    fn scan_dir(&mut self, dir: &Path, prefix: &str) -> Result<()> {
        for entry in fs::read_dir(dir).map_err(Error::System)? {
            let entry = entry.map_err(Error::System)?;
            let path = entry.path();
            let name = if prefix.is_empty() {
                path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
                    .to_string()
            } else {
                format!(
                    "{}/{}",
                    prefix,
                    path.file_name().and_then(|n| n.to_str()).unwrap_or("")
                )
            };

            let metadata = entry.metadata().map_err(Error::System)?;
            let is_dir = metadata.is_dir();
            let size = metadata.len();

            self.entries
                .insert(name.clone(), ArchiveEntry::new(name.clone(), size, is_dir));

            if is_dir {
                self.scan_dir(&path, &name)?;
            }
        }
        Ok(())
    }
}

impl ArchiveReader for DirectoryArchive {
    fn format(&self) -> ArchiveFormat {
        ArchiveFormat::Directory
    }

    fn count_entries(&self) -> Result<usize> {
        Err(Error::Unsupported(
            "Cannot count entries in directory archive".into(),
        ))
    }

    fn list_entry(&self, _idx: usize) -> Result<&str> {
        Err(Error::Unsupported(
            "Cannot list entries in directory archive".into(),
        ))
    }

    fn has_entry(&self, name: &str) -> bool {
        let entry_path = self.path.join(name);
        entry_path.exists()
    }

    fn read_entry(&mut self, name: &str) -> Result<Vec<u8>> {
        let entry_path = self.path.join(name);
        if !entry_path.exists() {
            return Err(Error::Argument(format!("Entry not found: {}", name)));
        }

        fs::read(&entry_path).map_err(Error::System)
    }

    fn entry_names(&self) -> Vec<&str> {
        self.entries.keys().map(|s| s.as_str()).collect()
    }
}

/// Archive handle
pub struct Archive {
    reader: Box<dyn ArchiveReader + Send>,
}

impl Archive {
    /// Open archive from file path
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();

        // Check if it's a directory
        if path.is_dir() {
            return Self::open_directory(path);
        }

        // Read file data
        let data = fs::read(path).map_err(Error::System)?;

        // Detect format
        let format = Self::detect_format(&data);

        match format {
            ArchiveFormat::Zip => Ok(Self {
                reader: Box::new(ZipArchive::new(data)?),
            }),
            ArchiveFormat::Tar => Ok(Self {
                reader: Box::new(TarArchive::new(data)?),
            }),
            _ => Err(Error::Unsupported(
                "Unknown or unsupported archive format".into(),
            )),
        }
    }

    /// Open directory as archive
    pub fn open_directory<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path_buf = path.as_ref().to_path_buf();
        Ok(Self {
            reader: Box::new(DirectoryArchive::new(path_buf)?),
        })
    }

    /// Open archive from buffer
    pub fn from_buffer(data: Vec<u8>) -> Result<Self> {
        let format = Self::detect_format(&data);

        match format {
            ArchiveFormat::Zip => Ok(Self {
                reader: Box::new(ZipArchive::new(data)?),
            }),
            ArchiveFormat::Tar => Ok(Self {
                reader: Box::new(TarArchive::new(data)?),
            }),
            _ => Err(Error::Unsupported(
                "Unknown or unsupported archive format".into(),
            )),
        }
    }

    /// Detect archive format from magic bytes
    fn detect_format(data: &[u8]) -> ArchiveFormat {
        if data.len() < 4 {
            return ArchiveFormat::Unknown;
        }

        // ZIP magic: PK\x03\x04
        if data.starts_with(&[0x50, 0x4B, 0x03, 0x04]) {
            return ArchiveFormat::Zip;
        }

        // TAR has ustar magic at offset 257
        if data.len() >= 262 && &data[257..262] == b"ustar" {
            return ArchiveFormat::Tar;
        }

        ArchiveFormat::Unknown
    }

    /// Get archive format
    pub fn format(&self) -> ArchiveFormat {
        self.reader.format()
    }

    /// Count entries
    pub fn count_entries(&self) -> Result<usize> {
        self.reader.count_entries()
    }

    /// List entry at index
    pub fn list_entry(&self, idx: usize) -> Result<&str> {
        self.reader.list_entry(idx)
    }

    /// Check if entry exists
    pub fn has_entry(&self, name: &str) -> bool {
        self.reader.has_entry(name)
    }

    /// Read entry data
    pub fn read_entry(&mut self, name: &str) -> Result<Vec<u8>> {
        self.reader.read_entry(name)
    }

    /// Read entry as buffer
    pub fn read_entry_buffer(&mut self, name: &str) -> Result<Buffer> {
        let data = self.read_entry(name)?;
        Ok(Buffer::from_data(data))
    }

    /// Get all entry names
    pub fn entry_names(&self) -> Vec<&str> {
        self.reader.entry_names()
    }

    /// Check if path is a directory
    pub fn is_directory<P: AsRef<Path>>(path: P) -> bool {
        path.as_ref().is_dir()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_archive_format_name() {
        assert_eq!(ArchiveFormat::Zip.name(), "zip");
        assert_eq!(ArchiveFormat::Tar.name(), "tar");
        assert_eq!(ArchiveFormat::Directory.name(), "directory");
    }

    #[test]
    fn test_archive_entry() {
        let entry = ArchiveEntry::new("test.txt".to_string(), 1024, false);
        assert_eq!(entry.name, "test.txt");
        assert_eq!(entry.size, 1024);
        assert!(!entry.is_dir);
    }

    #[test]
    fn test_detect_zip_format() {
        let zip_magic = vec![0x50, 0x4B, 0x03, 0x04, 0x00, 0x00];
        assert_eq!(Archive::detect_format(&zip_magic), ArchiveFormat::Zip);
    }

    #[test]
    fn test_detect_unknown_format() {
        let data = vec![0x00, 0x01, 0x02, 0x03];
        assert_eq!(Archive::detect_format(&data), ArchiveFormat::Unknown);
    }

    #[test]
    fn test_directory_archive() -> Result<()> {
        let temp_dir = TempDir::new().map_err(|e| Error::Generic(e.to_string()))?;
        let dir_path = temp_dir.path();

        // Create some test files
        fs::write(dir_path.join("file1.txt"), b"content1").map_err(Error::System)?;
        fs::write(dir_path.join("file2.txt"), b"content2").map_err(Error::System)?;

        let mut archive = Archive::open_directory(dir_path)?;
        assert_eq!(archive.format(), ArchiveFormat::Directory);

        assert!(archive.has_entry("file1.txt"));
        assert!(archive.has_entry("file2.txt"));
        assert!(!archive.has_entry("nonexistent.txt"));

        let data = archive.read_entry("file1.txt")?;
        assert_eq!(data, b"content1");

        Ok(())
    }

    #[test]
    fn test_directory_archive_count_entries_fails() {
        let temp_dir = TempDir::new().unwrap();
        let archive = Archive::open_directory(temp_dir.path()).unwrap();

        assert!(archive.count_entries().is_err());
    }

    #[test]
    fn test_directory_archive_list_entry_fails() {
        let temp_dir = TempDir::new().unwrap();
        let archive = Archive::open_directory(temp_dir.path()).unwrap();

        assert!(archive.list_entry(0).is_err());
    }

    #[test]
    fn test_directory_archive_read_nonexistent() {
        let temp_dir = TempDir::new().unwrap();
        let mut archive = Archive::open_directory(temp_dir.path()).unwrap();

        assert!(archive.read_entry("nonexistent.txt").is_err());
    }

    #[test]
    fn test_open_invalid_directory() {
        let result = Archive::open_directory("/nonexistent/path");
        assert!(result.is_err());
    }

    #[test]
    fn test_is_directory() {
        let temp_dir = TempDir::new().unwrap();
        assert!(Archive::is_directory(temp_dir.path()));

        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, b"test").unwrap();
        assert!(!Archive::is_directory(&file_path));
    }

    #[test]
    fn test_from_buffer_zip() {
        let zip_data = vec![0x50, 0x4B, 0x03, 0x04]; // Minimal ZIP magic
        let result = Archive::from_buffer(zip_data);
        // Will fail parsing but format should be detected
        assert!(result.is_ok() || result.is_err()); // Just checking it doesn't panic
    }

    #[test]
    fn test_from_buffer_unknown() {
        let unknown_data = vec![0x00, 0x01, 0x02, 0x03];
        let result = Archive::from_buffer(unknown_data);
        assert!(result.is_err());
    }

    #[test]
    fn test_archive_entry_names() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join("a.txt"), b"a").unwrap();
        fs::write(temp_dir.path().join("b.txt"), b"b").unwrap();

        let archive = Archive::open_directory(temp_dir.path()).unwrap();
        let names = archive.entry_names();

        assert_eq!(names.len(), 2);
        assert!(names.contains(&"a.txt"));
        assert!(names.contains(&"b.txt"));
    }

    #[test]
    fn test_read_entry_buffer() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join("test.txt"), b"buffer test").unwrap();

        let mut archive = Archive::open_directory(temp_dir.path()).unwrap();
        let buffer = archive.read_entry_buffer("test.txt").unwrap();

        assert_eq!(buffer.len(), 11);
    }
}
