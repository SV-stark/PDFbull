//! Memory-Mapped File I/O
//!
//! Provides memory-mapped file reading for efficient large PDF handling:
//! - `MappedFile`: Read-only memory-mapped file
//! - `MappedBuffer`: Memory-mapped buffer with lazy page loading
//! - `MappedRegion`: Sub-region of a mapped file
//!
//! Benefits:
//! - No memory copies for file reading
//! - Kernel-managed page caching
//! - Lazy loading (pages loaded on access)
//! - Efficient random access

use memmap2::{Mmap, MmapOptions};
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::ops::Range;
use std::path::Path;
use std::sync::{Arc, LazyLock, Mutex, RwLock};

use super::{Handle, HandleStore};

// ============================================================================
// Memory-Mapped File
// ============================================================================

/// Memory-mapped file for efficient large file access
pub struct MappedFile {
    /// Memory mapping
    mmap: Mmap,
    /// File path (for debugging)
    path: String,
    /// File size
    size: usize,
    /// Access statistics
    stats: MappedFileStats,
}

/// Statistics for mapped file access
#[derive(Debug, Default)]
pub struct MappedFileStats {
    /// Number of read operations
    pub reads: std::sync::atomic::AtomicU64,
    /// Bytes read
    pub bytes_read: std::sync::atomic::AtomicU64,
    /// Number of slice operations
    pub slices: std::sync::atomic::AtomicU64,
}

impl MappedFile {
    /// Open a file as memory-mapped
    pub fn open<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let path_str = path.as_ref().to_string_lossy().to_string();
        let file = File::open(path.as_ref())?;
        let metadata = file.metadata()?;
        let size = metadata.len() as usize;

        // SAFETY: File is opened read-only, mmap is safe
        let mmap = unsafe { MmapOptions::new().map(&file)? };

        Ok(Self {
            mmap,
            path: path_str,
            size,
            stats: MappedFileStats::default(),
        })
    }

    /// Open with specific options
    pub fn open_with_options<P: AsRef<Path>>(
        path: P,
        offset: u64,
        len: Option<usize>,
    ) -> io::Result<Self> {
        let path_str = path.as_ref().to_string_lossy().to_string();
        let file = File::open(path.as_ref())?;
        let metadata = file.metadata()?;
        let file_size = metadata.len() as usize;

        let mut opts = MmapOptions::new();
        opts.offset(offset);

        if let Some(l) = len {
            opts.len(l);
        }

        // SAFETY: File is opened read-only
        let mmap = unsafe { opts.map(&file)? };
        let size = mmap.len();

        Ok(Self {
            mmap,
            path: path_str,
            size,
            stats: MappedFileStats::default(),
        })
    }

    /// Get file size
    pub fn len(&self) -> usize {
        self.size
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.size == 0
    }

    /// Get file path
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Get a slice of the mapped data
    pub fn slice(&self, range: Range<usize>) -> Option<&[u8]> {
        if range.end <= self.size {
            self.stats
                .slices
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            Some(&self.mmap[range])
        } else {
            None
        }
    }

    /// Get all data
    pub fn as_slice(&self) -> &[u8] {
        &self.mmap[..]
    }

    /// Read bytes at offset
    pub fn read_at(&self, offset: usize, len: usize) -> Option<&[u8]> {
        if offset + len <= self.size {
            self.stats
                .reads
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            self.stats
                .bytes_read
                .fetch_add(len as u64, std::sync::atomic::Ordering::Relaxed);
            Some(&self.mmap[offset..offset + len])
        } else {
            None
        }
    }

    /// Read a single byte
    pub fn read_byte(&self, offset: usize) -> Option<u8> {
        if offset < self.size {
            Some(self.mmap[offset])
        } else {
            None
        }
    }

    /// Read u16 big-endian
    pub fn read_u16_be(&self, offset: usize) -> Option<u16> {
        if offset + 2 <= self.size {
            let bytes = &self.mmap[offset..offset + 2];
            Some(u16::from_be_bytes([bytes[0], bytes[1]]))
        } else {
            None
        }
    }

    /// Read u32 big-endian
    pub fn read_u32_be(&self, offset: usize) -> Option<u32> {
        if offset + 4 <= self.size {
            let bytes = &self.mmap[offset..offset + 4];
            Some(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
        } else {
            None
        }
    }

    /// Read u64 big-endian
    pub fn read_u64_be(&self, offset: usize) -> Option<u64> {
        if offset + 8 <= self.size {
            let bytes = &self.mmap[offset..offset + 8];
            Some(u64::from_be_bytes([
                bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
            ]))
        } else {
            None
        }
    }

    /// Search for a byte sequence
    pub fn find(&self, needle: &[u8]) -> Option<usize> {
        self.mmap.windows(needle.len()).position(|w| w == needle)
    }

    /// Search for a byte sequence from offset
    pub fn find_from(&self, offset: usize, needle: &[u8]) -> Option<usize> {
        if offset >= self.size {
            return None;
        }
        self.mmap[offset..]
            .windows(needle.len())
            .position(|w| w == needle)
            .map(|p| p + offset)
    }

    /// Reverse search for a byte sequence
    pub fn rfind(&self, needle: &[u8]) -> Option<usize> {
        self.mmap.windows(needle.len()).rposition(|w| w == needle)
    }

    /// Get statistics
    pub fn stats(&self) -> &MappedFileStats {
        &self.stats
    }

    /// Advise the kernel about access pattern
    #[cfg(unix)]
    pub fn advise_sequential(&self) -> io::Result<()> {
        self.mmap.advise(memmap2::Advice::Sequential)
    }

    /// Advise random access pattern
    #[cfg(unix)]
    pub fn advise_random(&self) -> io::Result<()> {
        self.mmap.advise(memmap2::Advice::Random)
    }

    /// Advise that data will be needed soon
    #[cfg(unix)]
    pub fn advise_willneed(&self) -> io::Result<()> {
        self.mmap.advise(memmap2::Advice::WillNeed)
    }

    /// Advise that data won't be needed (no-op, not universally supported)
    #[cfg(unix)]
    pub fn advise_dontneed(&self) -> io::Result<()> {
        // DontNeed not available in memmap2 - would need libc::madvise directly
        Ok(())
    }
}

// ============================================================================
// Mapped Buffer (with cursor)
// ============================================================================

/// Memory-mapped buffer with read cursor
pub struct MappedBuffer {
    /// Underlying mapped file
    file: Arc<MappedFile>,
    /// Current read position
    position: usize,
}

impl MappedBuffer {
    /// Create from mapped file
    pub fn new(file: Arc<MappedFile>) -> Self {
        Self { file, position: 0 }
    }

    /// Open a file
    pub fn open<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let file = MappedFile::open(path)?;
        Ok(Self::new(Arc::new(file)))
    }

    /// Get current position
    pub fn position(&self) -> usize {
        self.position
    }

    /// Set position
    pub fn set_position(&mut self, pos: usize) {
        self.position = pos.min(self.file.len());
    }

    /// Get remaining bytes
    pub fn remaining(&self) -> usize {
        self.file.len().saturating_sub(self.position)
    }

    /// Check if at end
    pub fn is_eof(&self) -> bool {
        self.position >= self.file.len()
    }

    /// Peek byte without advancing
    pub fn peek(&self) -> Option<u8> {
        self.file.read_byte(self.position)
    }

    /// Peek n bytes without advancing
    pub fn peek_bytes(&self, n: usize) -> Option<&[u8]> {
        self.file.read_at(self.position, n)
    }

    /// Read and advance position
    pub fn read_byte(&mut self) -> Option<u8> {
        let byte = self.file.read_byte(self.position)?;
        self.position += 1;
        Some(byte)
    }

    /// Read n bytes and advance
    pub fn read_bytes(&mut self, n: usize) -> Option<&[u8]> {
        let bytes = self.file.read_at(self.position, n)?;
        self.position += n;
        Some(bytes)
    }

    /// Skip n bytes
    pub fn skip(&mut self, n: usize) {
        self.position = (self.position + n).min(self.file.len());
    }

    /// Read until delimiter (not including delimiter)
    pub fn read_until(&mut self, delimiter: u8) -> Option<&[u8]> {
        let start = self.position;
        let data = &self.file.as_slice()[start..];

        if let Some(pos) = data.iter().position(|&b| b == delimiter) {
            self.position = start + pos + 1; // Skip delimiter
            Some(&data[..pos])
        } else {
            None
        }
    }

    /// Read a line (until newline)
    pub fn read_line(&mut self) -> Option<&[u8]> {
        let start = self.position;
        let data = &self.file.as_slice()[start..];

        // Find newline
        if let Some(pos) = data.iter().position(|&b| b == b'\n') {
            // Handle \r\n
            let end = if pos > 0 && data[pos - 1] == b'\r' {
                pos - 1
            } else {
                pos
            };
            self.position = start + pos + 1;
            Some(&data[..end])
        } else if !data.is_empty() {
            // Return rest of file
            self.position = self.file.len();
            Some(data)
        } else {
            None
        }
    }

    /// Get underlying file
    pub fn file(&self) -> &Arc<MappedFile> {
        &self.file
    }
}

impl Read for MappedBuffer {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let available = self.remaining();
        let to_read = buf.len().min(available);

        if to_read == 0 {
            return Ok(0);
        }

        let data = self.file.read_at(self.position, to_read).unwrap();
        buf[..to_read].copy_from_slice(data);
        self.position += to_read;

        Ok(to_read)
    }
}

impl Seek for MappedBuffer {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        let new_pos = match pos {
            SeekFrom::Start(p) => p as i64,
            SeekFrom::End(p) => self.file.len() as i64 + p,
            SeekFrom::Current(p) => self.position as i64 + p,
        };

        if new_pos < 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Seek before start",
            ));
        }

        self.position = (new_pos as usize).min(self.file.len());
        Ok(self.position as u64)
    }
}

// ============================================================================
// Lazy Page Region
// ============================================================================

/// Page size for lazy loading (4KB)
pub const PAGE_SIZE: usize = 4096;

/// Lazy-loaded region of a mapped file
pub struct LazyRegion {
    /// Underlying file
    file: Arc<MappedFile>,
    /// Start offset in file
    start: usize,
    /// Length of region
    len: usize,
    /// Loaded pages bitmap
    loaded_pages: RwLock<Vec<bool>>,
}

impl LazyRegion {
    /// Create a new lazy region
    pub fn new(file: Arc<MappedFile>, start: usize, len: usize) -> Self {
        let num_pages = (len + PAGE_SIZE - 1) / PAGE_SIZE;
        Self {
            file,
            start,
            len,
            loaded_pages: RwLock::new(vec![false; num_pages]),
        }
    }

    /// Get region length
    pub fn len(&self) -> usize {
        self.len
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Access data at offset (triggers page load)
    pub fn get(&self, offset: usize) -> Option<&[u8]> {
        if offset >= self.len {
            return None;
        }

        // Mark page as loaded
        let page_idx = offset / PAGE_SIZE;
        {
            let mut loaded = self.loaded_pages.write().unwrap();
            loaded[page_idx] = true;
        }

        // Return slice from underlying mmap
        let file_offset = self.start + offset;
        let remaining = self.len - offset;
        self.file.read_at(file_offset, remaining)
    }

    /// Get a slice of the region
    pub fn slice(&self, range: Range<usize>) -> Option<&[u8]> {
        if range.end > self.len {
            return None;
        }

        // Mark all pages in range as loaded
        let start_page = range.start / PAGE_SIZE;
        let end_page = (range.end + PAGE_SIZE - 1) / PAGE_SIZE;
        {
            let mut loaded = self.loaded_pages.write().unwrap();
            for page_idx in start_page..end_page.min(loaded.len()) {
                loaded[page_idx] = true;
            }
        }

        let file_start = self.start + range.start;
        let len = range.end - range.start;
        self.file.read_at(file_start, len)
    }

    /// Check how many pages are loaded
    pub fn loaded_page_count(&self) -> usize {
        self.loaded_pages
            .read()
            .unwrap()
            .iter()
            .filter(|&&l| l)
            .count()
    }

    /// Total page count
    pub fn total_page_count(&self) -> usize {
        (self.len + PAGE_SIZE - 1) / PAGE_SIZE
    }

    /// Get load ratio
    pub fn load_ratio(&self) -> f64 {
        let total = self.total_page_count();
        if total == 0 {
            return 0.0;
        }
        self.loaded_page_count() as f64 / total as f64
    }
}

// ============================================================================
// Handle Store
// ============================================================================

pub static MAPPED_FILES: LazyLock<HandleStore<MappedFile>> = LazyLock::new(HandleStore::new);
pub static MAPPED_BUFFERS: LazyLock<HandleStore<MappedBuffer>> = LazyLock::new(HandleStore::new);

// ============================================================================
// FFI Functions
// ============================================================================

use std::ffi::{CStr, c_char, c_int};

/// FFI statistics structure
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct FfiMappedFileStats {
    pub size: usize,
    pub reads: u64,
    pub bytes_read: u64,
    pub slices: u64,
}

/// Open a memory-mapped file
#[unsafe(no_mangle)]
pub extern "C" fn fz_open_mapped_file(_ctx: Handle, path: *const c_char) -> Handle {
    if path.is_null() {
        return 0;
    }

    let path_str = unsafe { CStr::from_ptr(path).to_str().unwrap_or("") };

    match MappedFile::open(path_str) {
        Ok(file) => MAPPED_FILES.insert(file),
        Err(_) => 0,
    }
}

/// Close a memory-mapped file
#[unsafe(no_mangle)]
pub extern "C" fn fz_close_mapped_file(_ctx: Handle, file: Handle) {
    let _ = MAPPED_FILES.remove(file);
}

/// Get mapped file size
#[unsafe(no_mangle)]
pub extern "C" fn fz_mapped_file_size(_ctx: Handle, file: Handle) -> usize {
    MAPPED_FILES
        .get(file)
        .map(|f| f.lock().unwrap().len())
        .unwrap_or(0)
}

/// Get mapped file stats
#[unsafe(no_mangle)]
pub extern "C" fn fz_mapped_file_stats(_ctx: Handle, file: Handle) -> FfiMappedFileStats {
    MAPPED_FILES
        .get(file)
        .map(|f| {
            let guard = f.lock().unwrap();
            FfiMappedFileStats {
                size: guard.len(),
                reads: guard.stats.reads.load(std::sync::atomic::Ordering::Relaxed),
                bytes_read: guard
                    .stats
                    .bytes_read
                    .load(std::sync::atomic::Ordering::Relaxed),
                slices: guard
                    .stats
                    .slices
                    .load(std::sync::atomic::Ordering::Relaxed),
            }
        })
        .unwrap_or_default()
}

/// Read bytes from mapped file
#[unsafe(no_mangle)]
pub extern "C" fn fz_mapped_file_read(
    _ctx: Handle,
    file: Handle,
    offset: usize,
    dst: *mut u8,
    len: usize,
) -> c_int {
    if dst.is_null() {
        return -1;
    }

    let Some(f) = MAPPED_FILES.get(file) else {
        return -1;
    };

    let guard = f.lock().unwrap();
    let Some(data) = guard.read_at(offset, len) else {
        return -1;
    };

    unsafe {
        std::ptr::copy_nonoverlapping(data.as_ptr(), dst, len);
    }

    0
}

/// Search for bytes in mapped file
#[unsafe(no_mangle)]
pub extern "C" fn fz_mapped_file_find(
    _ctx: Handle,
    file: Handle,
    needle: *const u8,
    needle_len: usize,
) -> i64 {
    if needle.is_null() || needle_len == 0 {
        return -1;
    }

    let Some(f) = MAPPED_FILES.get(file) else {
        return -1;
    };

    let needle_slice = unsafe { std::slice::from_raw_parts(needle, needle_len) };
    let guard = f.lock().unwrap();

    match guard.find(needle_slice) {
        Some(pos) => pos as i64,
        None => -1,
    }
}

/// Reverse search for bytes
#[unsafe(no_mangle)]
pub extern "C" fn fz_mapped_file_rfind(
    _ctx: Handle,
    file: Handle,
    needle: *const u8,
    needle_len: usize,
) -> i64 {
    if needle.is_null() || needle_len == 0 {
        return -1;
    }

    let Some(f) = MAPPED_FILES.get(file) else {
        return -1;
    };

    let needle_slice = unsafe { std::slice::from_raw_parts(needle, needle_len) };
    let guard = f.lock().unwrap();

    match guard.rfind(needle_slice) {
        Some(pos) => pos as i64,
        None => -1,
    }
}

/// Advise kernel about access pattern
#[cfg(unix)]
#[unsafe(no_mangle)]
pub extern "C" fn fz_mapped_file_advise(_ctx: Handle, file: Handle, advice: c_int) -> c_int {
    let Some(f) = MAPPED_FILES.get(file) else {
        return -1;
    };

    let guard = f.lock().unwrap();
    let result = match advice {
        0 => guard.advise_sequential(),
        1 => guard.advise_random(),
        2 => guard.advise_willneed(),
        3 => guard.advise_dontneed(),
        _ => return -1,
    };

    match result {
        Ok(()) => 0,
        Err(_) => -1,
    }
}

#[cfg(not(unix))]
#[unsafe(no_mangle)]
pub extern "C" fn fz_mapped_file_advise(_ctx: Handle, _file: Handle, _advice: c_int) -> c_int {
    0 // No-op on non-Unix
}

/// Create a mapped buffer from file handle
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_mapped_buffer(_ctx: Handle, path: *const c_char) -> Handle {
    if path.is_null() {
        return 0;
    }

    let path_str = unsafe { CStr::from_ptr(path).to_str().unwrap_or("") };

    match MappedBuffer::open(path_str) {
        Ok(buf) => MAPPED_BUFFERS.insert(buf),
        Err(_) => 0,
    }
}

/// Drop mapped buffer
#[unsafe(no_mangle)]
pub extern "C" fn fz_drop_mapped_buffer(_ctx: Handle, buf: Handle) {
    let _ = MAPPED_BUFFERS.remove(buf);
}

/// Get buffer position
#[unsafe(no_mangle)]
pub extern "C" fn fz_mapped_buffer_position(_ctx: Handle, buf: Handle) -> usize {
    MAPPED_BUFFERS
        .get(buf)
        .map(|b| b.lock().unwrap().position())
        .unwrap_or(0)
}

/// Set buffer position
#[unsafe(no_mangle)]
pub extern "C" fn fz_mapped_buffer_seek(_ctx: Handle, buf: Handle, pos: usize) {
    if let Some(b) = MAPPED_BUFFERS.get(buf) {
        b.lock().unwrap().set_position(pos);
    }
}

/// Get remaining bytes
#[unsafe(no_mangle)]
pub extern "C" fn fz_mapped_buffer_remaining(_ctx: Handle, buf: Handle) -> usize {
    MAPPED_BUFFERS
        .get(buf)
        .map(|b| b.lock().unwrap().remaining())
        .unwrap_or(0)
}

/// Read byte from buffer
#[unsafe(no_mangle)]
pub extern "C" fn fz_mapped_buffer_read_byte(_ctx: Handle, buf: Handle) -> c_int {
    MAPPED_BUFFERS
        .get(buf)
        .and_then(|b| b.lock().unwrap().read_byte())
        .map(|b| b as c_int)
        .unwrap_or(-1)
}

/// Read bytes from buffer
#[unsafe(no_mangle)]
pub extern "C" fn fz_mapped_buffer_read(
    _ctx: Handle,
    buf: Handle,
    dst: *mut u8,
    len: usize,
) -> c_int {
    if dst.is_null() {
        return -1;
    }

    let Some(b) = MAPPED_BUFFERS.get(buf) else {
        return -1;
    };

    let mut guard = b.lock().unwrap();
    let Some(data) = guard.read_bytes(len) else {
        return -1;
    };

    unsafe {
        std::ptr::copy_nonoverlapping(data.as_ptr(), dst, len);
    }

    0
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn create_test_file(content: &[u8]) -> NamedTempFile {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(content).unwrap();
        file.flush().unwrap();
        file
    }

    #[test]
    fn test_mapped_file_open() {
        let content = b"Hello, memory-mapped world!";
        let temp = create_test_file(content);

        let mapped = MappedFile::open(temp.path()).unwrap();
        assert_eq!(mapped.len(), content.len());
        assert_eq!(mapped.as_slice(), content);
    }

    #[test]
    fn test_mapped_file_read_at() {
        let content = b"0123456789ABCDEF";
        let temp = create_test_file(content);

        let mapped = MappedFile::open(temp.path()).unwrap();

        assert_eq!(mapped.read_at(0, 4), Some(&b"0123"[..]));
        assert_eq!(mapped.read_at(10, 4), Some(&b"ABCD"[..]));
        assert_eq!(mapped.read_at(100, 1), None);
    }

    #[test]
    fn test_mapped_file_find() {
        let content = b"The quick brown fox jumps over the lazy dog";
        let temp = create_test_file(content);

        let mapped = MappedFile::open(temp.path()).unwrap();

        assert_eq!(mapped.find(b"quick"), Some(4));
        assert_eq!(mapped.find(b"fox"), Some(16));
        assert_eq!(mapped.find(b"cat"), None);
    }

    #[test]
    fn test_mapped_file_rfind() {
        let content = b"abcabc";
        let temp = create_test_file(content);

        let mapped = MappedFile::open(temp.path()).unwrap();

        assert_eq!(mapped.rfind(b"abc"), Some(3));
        assert_eq!(mapped.find(b"abc"), Some(0));
    }

    #[test]
    fn test_mapped_file_integers() {
        let content: [u8; 16] = [
            0x00, 0x01, // u16 = 1
            0x00, 0x00, 0x00, 0x02, // u32 = 2
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x03, // u64 = 3
            0xFF, 0xFF,
        ];
        let temp = create_test_file(&content);

        let mapped = MappedFile::open(temp.path()).unwrap();

        assert_eq!(mapped.read_u16_be(0), Some(1));
        assert_eq!(mapped.read_u32_be(2), Some(2));
        assert_eq!(mapped.read_u64_be(6), Some(3));
    }

    #[test]
    fn test_mapped_buffer_read() {
        let content = b"Line1\nLine2\nLine3";
        let temp = create_test_file(content);

        let mut buf = MappedBuffer::open(temp.path()).unwrap();

        assert_eq!(buf.position(), 0);
        assert_eq!(buf.read_byte(), Some(b'L'));
        assert_eq!(buf.position(), 1);

        buf.set_position(0);
        assert_eq!(buf.read_bytes(5), Some(&b"Line1"[..]));
    }

    #[test]
    fn test_mapped_buffer_read_line() {
        let content = b"Line1\nLine2\r\nLine3";
        let temp = create_test_file(content);

        let mut buf = MappedBuffer::open(temp.path()).unwrap();

        assert_eq!(buf.read_line(), Some(&b"Line1"[..]));
        assert_eq!(buf.read_line(), Some(&b"Line2"[..]));
        assert_eq!(buf.read_line(), Some(&b"Line3"[..]));
        assert_eq!(buf.read_line(), None);
    }

    #[test]
    fn test_mapped_buffer_seek() {
        let content = b"0123456789";
        let temp = create_test_file(content);

        let mut buf = MappedBuffer::open(temp.path()).unwrap();

        buf.seek(SeekFrom::Start(5)).unwrap();
        assert_eq!(buf.position(), 5);

        buf.seek(SeekFrom::Current(2)).unwrap();
        assert_eq!(buf.position(), 7);

        buf.seek(SeekFrom::End(-3)).unwrap();
        assert_eq!(buf.position(), 7);
    }

    #[test]
    fn test_lazy_region() {
        let content = vec![0u8; PAGE_SIZE * 3];
        let temp = create_test_file(&content);

        let mapped = Arc::new(MappedFile::open(temp.path()).unwrap());
        let region = LazyRegion::new(mapped, 0, content.len());

        assert_eq!(region.total_page_count(), 3);
        assert_eq!(region.loaded_page_count(), 0);

        // Access first page
        region.get(0);
        assert_eq!(region.loaded_page_count(), 1);

        // Access last page
        region.get(PAGE_SIZE * 2);
        assert_eq!(region.loaded_page_count(), 2);
    }

    #[test]
    fn test_ffi_mapped_file() {
        let content = b"FFI test content";
        let temp = create_test_file(content);

        let path = std::ffi::CString::new(temp.path().to_str().unwrap()).unwrap();
        let handle = fz_open_mapped_file(0, path.as_ptr());
        assert_ne!(handle, 0);

        let size = fz_mapped_file_size(0, handle);
        assert_eq!(size, content.len());

        let mut buf = vec![0u8; 4];
        let result = fz_mapped_file_read(0, handle, 0, buf.as_mut_ptr(), 4);
        assert_eq!(result, 0);
        assert_eq!(&buf, b"FFI ");

        fz_close_mapped_file(0, handle);
    }

    #[test]
    fn test_ffi_mapped_file_find() {
        let content = b"Find the needle in the haystack";
        let temp = create_test_file(content);

        let path = std::ffi::CString::new(temp.path().to_str().unwrap()).unwrap();
        let handle = fz_open_mapped_file(0, path.as_ptr());

        let needle = b"needle";
        let pos = fz_mapped_file_find(0, handle, needle.as_ptr(), needle.len());
        assert_eq!(pos, 9);

        let not_found = b"xyz";
        let pos = fz_mapped_file_find(0, handle, not_found.as_ptr(), not_found.len());
        assert_eq!(pos, -1);

        fz_close_mapped_file(0, handle);
    }

    #[test]
    fn test_ffi_mapped_buffer() {
        let content = b"Buffer test";
        let temp = create_test_file(content);

        let path = std::ffi::CString::new(temp.path().to_str().unwrap()).unwrap();
        let handle = fz_new_mapped_buffer(0, path.as_ptr());
        assert_ne!(handle, 0);

        assert_eq!(fz_mapped_buffer_position(0, handle), 0);
        assert_eq!(fz_mapped_buffer_remaining(0, handle), content.len());

        let byte = fz_mapped_buffer_read_byte(0, handle);
        assert_eq!(byte, b'B' as c_int);
        assert_eq!(fz_mapped_buffer_position(0, handle), 1);

        fz_mapped_buffer_seek(0, handle, 7);
        assert_eq!(fz_mapped_buffer_position(0, handle), 7);

        fz_drop_mapped_buffer(0, handle);
    }
}
