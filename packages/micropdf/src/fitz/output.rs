//! Output Stream - MuPDF Compatible
//!
//! Provides output abstraction for writing to files, buffers, and custom sinks.

use crate::fitz::buffer::Buffer;
use crate::fitz::error::{Error, Result};
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::Path;

/// Output writer trait for abstracting output destinations
pub trait OutputWriter: Write + Send {
    /// Seek to a position
    fn seek(&mut self, offset: i64, whence: SeekFrom) -> Result<u64>;

    /// Get current position
    fn tell(&mut self) -> Result<u64>;

    /// Flush any buffered data
    fn flush_output(&mut self) -> Result<()>;

    /// Truncate output at current position
    fn truncate(&mut self) -> Result<()>;

    /// Reset to initial state (if supported)
    fn reset(&mut self) -> Result<()> {
        Err(Error::Generic(
            "Reset not supported for this output type".into(),
        ))
    }
}

/// Seek position
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeekFrom {
    /// Seek from start of stream
    Start(u64),
    /// Seek from current position
    Current(i64),
    /// Seek from end of stream
    End(i64),
}

impl From<SeekFrom> for std::io::SeekFrom {
    fn from(seek: SeekFrom) -> Self {
        match seek {
            SeekFrom::Start(n) => std::io::SeekFrom::Start(n),
            SeekFrom::Current(n) => std::io::SeekFrom::Current(n),
            SeekFrom::End(n) => std::io::SeekFrom::End(n),
        }
    }
}

/// Output stream for writing data
pub struct Output {
    writer: Box<dyn OutputWriter>,
}

impl Output {
    /// Create output from a file path
    pub fn from_path<P: AsRef<Path>>(path: P, append: bool) -> Result<Self> {
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(!append)
            .append(append)
            .open(path)
            .map_err(Error::System)?;

        Ok(Self {
            writer: Box::new(FileOutput::new(file)),
        })
    }

    /// Create output from an existing file
    pub fn from_file(file: File) -> Self {
        Self {
            writer: Box::new(FileOutput::new(file)),
        }
    }

    /// Create output to a buffer
    pub fn from_buffer(buffer: Buffer) -> Self {
        Self {
            writer: Box::new(BufferOutput::new(buffer)),
        }
    }

    /// Create output from any writer
    pub fn from_writer<W: OutputWriter + 'static>(writer: W) -> Self {
        Self {
            writer: Box::new(writer),
        }
    }

    /// Write raw data
    pub fn write_data(&mut self, data: &[u8]) -> Result<()> {
        self.writer.write_all(data).map_err(Error::System)
    }

    /// Write a buffer
    pub fn write_buffer(&mut self, buffer: &Buffer) -> Result<()> {
        self.write_data(buffer.as_slice())
    }

    /// Write a string
    pub fn write_string(&mut self, s: &str) -> Result<()> {
        self.write_data(s.as_bytes())
    }

    /// Write a formatted string
    pub fn write_printf(&mut self, _fmt: &str, args: std::fmt::Arguments) -> Result<()> {
        use std::fmt::Write as FmtWrite;
        let mut s = String::new();
        s.write_fmt(args)
            .map_err(|e| Error::Generic(e.to_string()))?;
        self.write_string(&s)
    }

    /// Write a single byte
    pub fn write_byte(&mut self, byte: u8) -> Result<()> {
        self.write_data(&[byte])
    }

    /// Write a single character
    pub fn write_char(&mut self, c: char) -> Result<()> {
        let mut buf = [0u8; 4];
        let encoded = c.encode_utf8(&mut buf);
        self.write_data(encoded.as_bytes())
    }

    /// Write i16 big-endian
    pub fn write_i16_be(&mut self, value: i16) -> Result<()> {
        self.write_data(&value.to_be_bytes())
    }

    /// Write i16 little-endian
    pub fn write_i16_le(&mut self, value: i16) -> Result<()> {
        self.write_data(&value.to_le_bytes())
    }

    /// Write u16 big-endian
    pub fn write_u16_be(&mut self, value: u16) -> Result<()> {
        self.write_data(&value.to_be_bytes())
    }

    /// Write u16 little-endian
    pub fn write_u16_le(&mut self, value: u16) -> Result<()> {
        self.write_data(&value.to_le_bytes())
    }

    /// Write i32 big-endian
    pub fn write_i32_be(&mut self, value: i32) -> Result<()> {
        self.write_data(&value.to_be_bytes())
    }

    /// Write i32 little-endian
    pub fn write_i32_le(&mut self, value: i32) -> Result<()> {
        self.write_data(&value.to_le_bytes())
    }

    /// Write u32 big-endian
    pub fn write_u32_be(&mut self, value: u32) -> Result<()> {
        self.write_data(&value.to_be_bytes())
    }

    /// Write u32 little-endian
    pub fn write_u32_le(&mut self, value: u32) -> Result<()> {
        self.write_data(&value.to_le_bytes())
    }

    /// Write i64 big-endian
    pub fn write_i64_be(&mut self, value: i64) -> Result<()> {
        self.write_data(&value.to_be_bytes())
    }

    /// Write i64 little-endian
    pub fn write_i64_le(&mut self, value: i64) -> Result<()> {
        self.write_data(&value.to_le_bytes())
    }

    /// Write u64 big-endian
    pub fn write_u64_be(&mut self, value: u64) -> Result<()> {
        self.write_data(&value.to_be_bytes())
    }

    /// Write u64 little-endian
    pub fn write_u64_le(&mut self, value: u64) -> Result<()> {
        self.write_data(&value.to_le_bytes())
    }

    /// Write f32 big-endian
    pub fn write_f32_be(&mut self, value: f32) -> Result<()> {
        self.write_u32_be(value.to_bits())
    }

    /// Write f32 little-endian
    pub fn write_f32_le(&mut self, value: f32) -> Result<()> {
        self.write_u32_le(value.to_bits())
    }

    /// Write f64 big-endian
    pub fn write_f64_be(&mut self, value: f64) -> Result<()> {
        self.write_u64_be(value.to_bits())
    }

    /// Write f64 little-endian
    pub fn write_f64_le(&mut self, value: f64) -> Result<()> {
        self.write_u64_le(value.to_bits())
    }

    /// Seek to a position
    pub fn seek(&mut self, offset: i64, whence: SeekFrom) -> Result<u64> {
        self.writer.seek(offset, whence)
    }

    /// Get current position
    pub fn tell(&mut self) -> Result<u64> {
        self.writer.tell()
    }

    /// Flush buffered data
    pub fn flush(&mut self) -> Result<()> {
        self.writer.flush_output()
    }

    /// Close the output (flushes data)
    pub fn close(&mut self) -> Result<()> {
        self.flush()
    }

    /// Truncate at current position
    pub fn truncate(&mut self) -> Result<()> {
        self.writer.truncate()
    }

    /// Reset to initial state
    pub fn reset(&mut self) -> Result<()> {
        self.writer.reset()
    }
}

// ============================================================================
// File Output
// ============================================================================

struct FileOutput {
    file: File,
}

impl FileOutput {
    fn new(file: File) -> Self {
        Self { file }
    }
}

impl Write for FileOutput {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.file.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.file.flush()
    }
}

impl OutputWriter for FileOutput {
    fn seek(&mut self, _offset: i64, whence: SeekFrom) -> Result<u64> {
        use std::io::Seek;
        self.file.seek(whence.into()).map_err(Error::System)
    }

    fn tell(&mut self) -> Result<u64> {
        use std::io::Seek;
        self.file.stream_position().map_err(Error::System)
    }

    fn flush_output(&mut self) -> Result<()> {
        self.file.flush().map_err(Error::System)
    }

    fn truncate(&mut self) -> Result<()> {
        let pos = self.tell()?;
        self.file.set_len(pos).map_err(Error::System)
    }
}

// ============================================================================
// Buffer Output
// ============================================================================

struct BufferOutput {
    data: Vec<u8>,
    position: usize,
}

impl BufferOutput {
    fn new(buffer: Buffer) -> Self {
        let data = buffer.to_vec();
        Self {
            position: data.len(),
            data,
        }
    }

    #[allow(dead_code)]
    fn to_buffer(&self) -> Buffer {
        Buffer::from_data(self.data.clone())
    }
}

impl Write for BufferOutput {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        // If writing beyond current length, extend buffer
        if self.position >= self.data.len() {
            self.data.extend_from_slice(buf);
            self.position += buf.len();
        } else {
            // Overwrite existing data
            let end = self.position + buf.len();
            if end > self.data.len() {
                // Partial overwrite, then extend
                let overwrite_len = self.data.len() - self.position;
                self.data[self.position..].copy_from_slice(&buf[..overwrite_len]);
                self.data.extend_from_slice(&buf[overwrite_len..]);
            } else {
                // Full overwrite
                self.data[self.position..end].copy_from_slice(buf);
            }
            self.position += buf.len();
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl OutputWriter for BufferOutput {
    fn seek(&mut self, _offset: i64, whence: SeekFrom) -> Result<u64> {
        let new_pos = match whence {
            SeekFrom::Start(n) => n as i64,
            SeekFrom::Current(n) => self.position as i64 + n,
            SeekFrom::End(n) => self.data.len() as i64 + n,
        };

        if new_pos < 0 {
            return Err(Error::Generic("Seek before start of buffer".into()));
        }

        self.position = new_pos as usize;
        Ok(self.position as u64)
    }

    fn tell(&mut self) -> Result<u64> {
        Ok(self.position as u64)
    }

    fn flush_output(&mut self) -> Result<()> {
        Ok(())
    }

    fn truncate(&mut self) -> Result<()> {
        self.data.truncate(self.position);
        Ok(())
    }

    fn reset(&mut self) -> Result<()> {
        self.data.clear();
        self.position = 0;
        Ok(())
    }
}

// ============================================================================
// Memory Output (Vec<u8>)
// ============================================================================

pub struct MemoryOutput {
    data: Vec<u8>,
    position: usize,
}

impl MemoryOutput {
    pub fn new() -> Self {
        Self {
            data: Vec::new(),
            position: 0,
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            data: Vec::with_capacity(capacity),
            position: 0,
        }
    }

    pub fn into_vec(self) -> Vec<u8> {
        self.data
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.data
    }
}

impl Default for MemoryOutput {
    fn default() -> Self {
        Self::new()
    }
}

impl Write for MemoryOutput {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if self.position >= self.data.len() {
            self.data.extend_from_slice(buf);
            self.position += buf.len();
        } else {
            let end = self.position + buf.len();
            if end > self.data.len() {
                let overwrite_len = self.data.len() - self.position;
                self.data[self.position..].copy_from_slice(&buf[..overwrite_len]);
                self.data.extend_from_slice(&buf[overwrite_len..]);
            } else {
                self.data[self.position..end].copy_from_slice(buf);
            }
            self.position += buf.len();
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl OutputWriter for MemoryOutput {
    fn seek(&mut self, _offset: i64, whence: SeekFrom) -> Result<u64> {
        let new_pos = match whence {
            SeekFrom::Start(n) => n as i64,
            SeekFrom::Current(n) => self.position as i64 + n,
            SeekFrom::End(n) => self.data.len() as i64 + n,
        };

        if new_pos < 0 {
            return Err(Error::Generic("Seek before start of output".into()));
        }

        self.position = new_pos as usize;
        Ok(self.position as u64)
    }

    fn tell(&mut self) -> Result<u64> {
        Ok(self.position as u64)
    }

    fn flush_output(&mut self) -> Result<()> {
        Ok(())
    }

    fn truncate(&mut self) -> Result<()> {
        self.data.truncate(self.position);
        Ok(())
    }

    fn reset(&mut self) -> Result<()> {
        self.data.clear();
        self.position = 0;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_memory_output() {
        let mut output = MemoryOutput::new();
        output.write_all(b"Hello, ").unwrap();
        output.write_all(b"World!").unwrap();
        assert_eq!(output.as_slice(), b"Hello, World!");
    }

    #[test]
    fn test_memory_output_seek() {
        let mut output = MemoryOutput::new();
        output.write_all(b"Hello").unwrap();

        // Seek to start and overwrite
        output.seek(0, SeekFrom::Start(0)).unwrap();
        output.write_all(b"Jello").unwrap();
        assert_eq!(output.as_slice(), b"Jello");
    }

    #[test]
    fn test_output_from_buffer() {
        let buffer = Buffer::new(1024);
        let mut output = Output::from_buffer(buffer);

        output.write_string("Test").unwrap();
        output.write_byte(b' ').unwrap();
        output.write_string("data").unwrap();

        output.flush().unwrap();
    }

    #[test]
    fn test_output_write_integers() {
        let output = MemoryOutput::new();
        let mut out = Output::from_writer(output);

        out.write_i16_be(0x1234).unwrap();
        out.write_u32_le(0xDEADBEEF).unwrap();
        out.write_i64_be(0x0123456789ABCDEF).unwrap();

        assert!(out.tell().unwrap() > 0);
    }

    #[test]
    fn test_file_output() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();

        let mut output = Output::from_path(path, false).unwrap();
        output.write_string("Test file output").unwrap();
        output.close().unwrap();

        let content = std::fs::read_to_string(path).unwrap();
        assert_eq!(content, "Test file output");
    }

    #[test]
    fn test_output_truncate() {
        let mut output = MemoryOutput::new();
        output.write_all(b"Hello, World!").unwrap();
        output.seek(5, SeekFrom::Start(5)).unwrap();
        output.truncate().unwrap();
        assert_eq!(output.as_slice(), b"Hello");
    }

    #[test]
    fn test_output_reset() {
        let mut output = MemoryOutput::new();
        output.write_all(b"Data").unwrap();
        output.reset().unwrap();
        assert_eq!(output.as_slice(), b"");
        assert_eq!(output.tell().unwrap(), 0);
    }

    #[test]
    fn test_output_write_floats() {
        let output = MemoryOutput::new();
        let mut out = Output::from_writer(output);

        out.write_f32_be(std::f32::consts::PI).unwrap();
        out.write_f64_le(std::f64::consts::E).unwrap();

        assert_eq!(out.tell().unwrap(), 12); // 4 + 8 bytes
    }
}
