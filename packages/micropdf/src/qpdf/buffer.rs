//! Buffer utilities for QPDF-compatible operations
//!
//! Provides buffer types for efficient data handling in PDF operations.

use super::error::{QpdfError, Result};
use std::ops::{Deref, DerefMut};

/// A growable buffer for PDF data
#[derive(Debug, Clone)]
pub struct Buffer {
    data: Vec<u8>,
}

impl Buffer {
    /// Create a new empty buffer
    pub fn new() -> Self {
        Self { data: Vec::new() }
    }

    /// Create a buffer with the given capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            data: Vec::with_capacity(capacity),
        }
    }

    /// Create a buffer from existing data
    pub fn from_vec(data: Vec<u8>) -> Self {
        Self { data }
    }

    /// Create a buffer from a slice
    pub fn from_slice(data: &[u8]) -> Self {
        Self {
            data: data.to_vec(),
        }
    }

    /// Create a buffer from a string
    pub fn from_string(s: String) -> Self {
        Self {
            data: s.into_bytes(),
        }
    }

    /// Get the size of the buffer
    pub fn size(&self) -> usize {
        self.data.len()
    }

    /// Check if the buffer is empty
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Get the buffer data as a slice
    pub fn as_slice(&self) -> &[u8] {
        &self.data
    }

    /// Get the buffer data as a mutable slice
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        &mut self.data
    }

    /// Get the underlying vector
    pub fn into_vec(self) -> Vec<u8> {
        self.data
    }

    /// Append data to the buffer
    pub fn append(&mut self, data: &[u8]) {
        self.data.extend_from_slice(data);
    }

    /// Append a single byte
    pub fn push(&mut self, byte: u8) {
        self.data.push(byte);
    }

    /// Clear the buffer
    pub fn clear(&mut self) {
        self.data.clear();
    }

    /// Reserve capacity
    pub fn reserve(&mut self, additional: usize) {
        self.data.reserve(additional);
    }

    /// Truncate the buffer to the given length
    pub fn truncate(&mut self, len: usize) {
        self.data.truncate(len);
    }

    /// Convert to a string (returns error if not valid UTF-8)
    pub fn to_string(&self) -> Result<String> {
        String::from_utf8(self.data.clone())
            .map_err(|e| QpdfError::Parse(format!("Invalid UTF-8: {}", e)))
    }

    /// Convert to a string, replacing invalid UTF-8 sequences
    pub fn to_string_lossy(&self) -> String {
        String::from_utf8_lossy(&self.data).into_owned()
    }
}

impl Default for Buffer {
    fn default() -> Self {
        Self::new()
    }
}

impl Deref for Buffer {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl DerefMut for Buffer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

impl From<Vec<u8>> for Buffer {
    fn from(data: Vec<u8>) -> Self {
        Self::from_vec(data)
    }
}

impl From<&[u8]> for Buffer {
    fn from(data: &[u8]) -> Self {
        Self::from_slice(data)
    }
}

impl From<String> for Buffer {
    fn from(s: String) -> Self {
        Self::from_string(s)
    }
}

impl From<&str> for Buffer {
    fn from(s: &str) -> Self {
        Self::from_slice(s.as_bytes())
    }
}

impl AsRef<[u8]> for Buffer {
    fn as_ref(&self) -> &[u8] {
        &self.data
    }
}

impl AsMut<[u8]> for Buffer {
    fn as_mut(&mut self) -> &mut [u8] {
        &mut self.data
    }
}

/// Input source trait for reading PDF data
pub trait InputSource: Send {
    /// Get the name/description of this input source
    fn name(&self) -> &str;

    /// Read data into the buffer, returning the number of bytes read
    fn read(&mut self, buf: &mut [u8]) -> Result<usize>;

    /// Seek to a position in the input
    fn seek(&mut self, pos: u64) -> Result<()>;

    /// Get the current position
    fn tell(&self) -> u64;

    /// Get the total size of the input
    fn size(&self) -> Result<u64>;

    /// Read all data from current position
    fn read_all(&mut self) -> Result<Vec<u8>> {
        let current_pos = self.tell();
        let total_size = self.size()?;
        let remaining = (total_size - current_pos) as usize;

        let mut buffer = vec![0u8; remaining];
        let mut total_read = 0;

        while total_read < remaining {
            let n = self.read(&mut buffer[total_read..])?;
            if n == 0 {
                break;
            }
            total_read += n;
        }

        buffer.truncate(total_read);
        Ok(buffer)
    }

    /// Read a line (up to newline or EOF)
    fn read_line(&mut self) -> Result<String> {
        let mut line = Vec::new();
        let mut buf = [0u8; 1];

        loop {
            let n = self.read(&mut buf)?;
            if n == 0 {
                break;
            }
            if buf[0] == b'\n' {
                break;
            }
            if buf[0] != b'\r' {
                line.push(buf[0]);
            }
        }

        String::from_utf8(line).map_err(|e| QpdfError::Parse(format!("Invalid UTF-8: {}", e)))
    }
}

/// File-based input source
pub struct FileInputSource {
    name: String,
    file: std::fs::File,
    size: u64,
    position: u64,
}

impl FileInputSource {
    /// Create a new file input source
    pub fn new(path: &str) -> Result<Self> {
        use std::io::{Read, Seek, SeekFrom};

        let mut file = std::fs::File::open(path)?;
        let size = file.seek(SeekFrom::End(0))?;
        file.seek(SeekFrom::Start(0))?;

        Ok(Self {
            name: path.to_string(),
            file,
            size,
            position: 0,
        })
    }
}

impl InputSource for FileInputSource {
    fn name(&self) -> &str {
        &self.name
    }

    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        use std::io::Read;
        let n = self.file.read(buf)?;
        self.position += n as u64;
        Ok(n)
    }

    fn seek(&mut self, pos: u64) -> Result<()> {
        use std::io::{Seek, SeekFrom};
        self.file.seek(SeekFrom::Start(pos))?;
        self.position = pos;
        Ok(())
    }

    fn tell(&self) -> u64 {
        self.position
    }

    fn size(&self) -> Result<u64> {
        Ok(self.size)
    }
}

/// Memory-based input source
pub struct BufferInputSource {
    name: String,
    data: Vec<u8>,
    position: usize,
}

impl BufferInputSource {
    /// Create a new buffer input source
    pub fn new(name: &str, data: Vec<u8>) -> Self {
        Self {
            name: name.to_string(),
            data,
            position: 0,
        }
    }

    /// Create from a slice
    pub fn from_slice(name: &str, data: &[u8]) -> Self {
        Self::new(name, data.to_vec())
    }
}

impl InputSource for BufferInputSource {
    fn name(&self) -> &str {
        &self.name
    }

    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let remaining = self.data.len() - self.position;
        let to_read = buf.len().min(remaining);
        buf[..to_read].copy_from_slice(&self.data[self.position..self.position + to_read]);
        self.position += to_read;
        Ok(to_read)
    }

    fn seek(&mut self, pos: u64) -> Result<()> {
        let pos = pos as usize;
        if pos > self.data.len() {
            return Err(QpdfError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Seek position beyond end of buffer",
            )));
        }
        self.position = pos;
        Ok(())
    }

    fn tell(&self) -> u64 {
        self.position as u64
    }

    fn size(&self) -> Result<u64> {
        Ok(self.data.len() as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buffer_basic() {
        let mut buf = Buffer::new();
        assert!(buf.is_empty());

        buf.append(b"Hello");
        assert_eq!(buf.size(), 5);

        buf.push(b' ');
        buf.append(b"World");
        assert_eq!(buf.as_slice(), b"Hello World");
    }

    #[test]
    fn test_buffer_from_string() {
        let buf = Buffer::from("test string");
        assert_eq!(buf.to_string().unwrap(), "test string");
    }

    #[test]
    fn test_buffer_input_source() {
        let mut source = BufferInputSource::new("test", b"Hello, World!".to_vec());

        let mut buf = [0u8; 5];
        assert_eq!(source.read(&mut buf).unwrap(), 5);
        assert_eq!(&buf, b"Hello");

        assert_eq!(source.tell(), 5);
        assert_eq!(source.size().unwrap(), 13);

        source.seek(7).unwrap();
        let mut buf = [0u8; 6];
        assert_eq!(source.read(&mut buf).unwrap(), 6);
        assert_eq!(&buf, b"World!");
    }
}
