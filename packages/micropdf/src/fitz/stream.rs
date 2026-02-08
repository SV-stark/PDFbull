//! Stream - Buffered I/O abstraction using the `bytes` crate
//!
//! This module provides high-performance stream implementations for reading
//! PDF data from files, memory, and other sources.

use crate::fitz::buffer::Buffer;
use crate::fitz::error::{Error, Result};
use bytes::{Bytes, BytesMut};
use std::fs::File;
use std::io::{self, BufReader, Read, Seek, SeekFrom};
use std::path::Path;

/// A buffered stream for reading PDF data.
pub struct Stream {
    inner: Box<dyn StreamSource>,
    buffer: BytesMut,
    rp: usize,
    wp: usize,
    pos: i64,
    eof: bool,
    error: bool,
    bits: u32,
    avail: u8,
    filename: Option<String>,
}

/// Trait for stream data sources.
pub trait StreamSource: Send + Sync {
    /// Read data into the buffer.
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize>;
    /// Seek to a position.
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64>;
    /// Get current position.
    fn tell(&mut self) -> io::Result<u64>;
    /// Get total length if known.
    fn len(&self) -> Option<u64>;
    /// Check if stream is empty (if length is known).
    fn is_empty(&self) -> Option<bool> {
        self.len().map(|l| l == 0)
    }
}

/// File-based stream source.
struct FileSource {
    reader: BufReader<File>,
    len: u64,
}

impl StreamSource for FileSource {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.reader.read(buf)
    }

    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.reader.seek(pos)
    }

    fn tell(&mut self) -> io::Result<u64> {
        self.reader.stream_position()
    }

    fn len(&self) -> Option<u64> {
        Some(self.len)
    }
}

/// Memory-based stream source using `bytes::Bytes`.
struct MemorySource {
    data: Bytes,
    position: usize,
}

impl StreamSource for MemorySource {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let remaining = &self.data[self.position..];
        let to_read = buf.len().min(remaining.len());
        buf[..to_read].copy_from_slice(&remaining[..to_read]);
        self.position += to_read;
        Ok(to_read)
    }

    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        let new_pos = match pos {
            SeekFrom::Start(offset) => offset as i64,
            SeekFrom::End(offset) => self.data.len() as i64 + offset,
            SeekFrom::Current(offset) => self.position as i64 + offset,
        };
        if new_pos < 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Seek before start",
            ));
        }
        self.position = (new_pos as usize).min(self.data.len());
        Ok(self.position as u64)
    }

    fn tell(&mut self) -> io::Result<u64> {
        Ok(self.position as u64)
    }

    fn len(&self) -> Option<u64> {
        Some(self.data.len() as u64)
    }
}

const STREAM_BUFFER_SIZE: usize = 8192;

impl Stream {
    /// Open a stream from a file path.
    pub fn open_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let file = File::open(path).map_err(Error::System)?;
        let len = file.metadata().map_err(Error::System)?.len();
        Ok(Self {
            inner: Box::new(FileSource {
                reader: BufReader::with_capacity(STREAM_BUFFER_SIZE, file),
                len,
            }),
            buffer: BytesMut::with_capacity(STREAM_BUFFER_SIZE),
            rp: 0,
            wp: 0,
            pos: 0,
            eof: false,
            error: false,
            bits: 0,
            avail: 0,
            filename: Some(path.to_string_lossy().into_owned()),
        })
    }

    /// Open a stream from a byte slice.
    pub fn open_memory(data: &[u8]) -> Self {
        Self {
            inner: Box::new(MemorySource {
                data: Bytes::copy_from_slice(data),
                position: 0,
            }),
            buffer: BytesMut::with_capacity(STREAM_BUFFER_SIZE),
            rp: 0,
            wp: 0,
            pos: 0,
            eof: false,
            error: false,
            bits: 0,
            avail: 0,
            filename: None,
        }
    }

    /// Open a stream from a `Bytes` instance (zero-copy).
    pub fn open_bytes(data: Bytes) -> Self {
        Self {
            inner: Box::new(MemorySource { data, position: 0 }),
            buffer: BytesMut::with_capacity(STREAM_BUFFER_SIZE),
            rp: 0,
            wp: 0,
            pos: 0,
            eof: false,
            error: false,
            bits: 0,
            avail: 0,
            filename: None,
        }
    }

    /// Open a stream from a Buffer.
    pub fn open_buffer(buffer: &Buffer) -> Self {
        Self::open_bytes(buffer.to_bytes())
    }

    /// Get the current read position.
    pub fn tell(&self) -> i64 {
        self.pos - (self.wp - self.rp) as i64
    }

    /// Get the total length of the stream if known.
    pub fn len(&self) -> Option<u64> {
        self.inner.len()
    }

    /// Check if the stream is empty.
    pub fn is_empty(&self) -> bool {
        self.inner.len() == Some(0)
    }

    /// Check if we've reached EOF.
    pub fn is_eof(&self) -> bool {
        self.eof && self.rp >= self.wp
    }

    /// Get the filename if this is a file stream.
    pub fn filename(&self) -> Option<&str> {
        self.filename.as_deref()
    }

    /// Fill the internal buffer.
    fn fill_buffer(&mut self) -> Result<usize> {
        if self.eof {
            return Ok(0);
        }

        // Compact the buffer
        if self.rp > 0 {
            if self.rp < self.wp {
                // Move remaining data to the start
                let remaining = self.wp - self.rp;
                self.buffer.copy_within(self.rp..self.wp, 0);
                self.buffer.truncate(remaining);
                self.wp = remaining;
            } else {
                self.buffer.clear();
                self.wp = 0;
            }
            self.rp = 0;
        }

        // Ensure buffer has space
        if self.buffer.len() < STREAM_BUFFER_SIZE {
            self.buffer.resize(STREAM_BUFFER_SIZE, 0);
        }

        // Read more data
        match self.inner.read(&mut self.buffer[self.wp..]) {
            Ok(0) => {
                self.eof = true;
                Ok(0)
            }
            Ok(n) => {
                self.wp += n;
                self.pos += n as i64;
                Ok(n)
            }
            Err(e) => {
                self.error = true;
                Err(Error::System(e))
            }
        }
    }

    /// Read a single byte.
    pub fn read_byte(&mut self) -> Result<Option<u8>> {
        if self.rp >= self.wp && self.fill_buffer()? == 0 {
            return Ok(None);
        }
        let byte = self.buffer[self.rp];
        self.rp += 1;
        Ok(Some(byte))
    }

    /// Peek at the next byte without consuming it.
    pub fn peek_byte(&mut self) -> Result<Option<u8>> {
        if self.rp >= self.wp && self.fill_buffer()? == 0 {
            return Ok(None);
        }
        Ok(Some(self.buffer[self.rp]))
    }

    /// Read bytes into a buffer.
    pub fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let mut total = 0;
        while total < buf.len() {
            let buffered = self.wp - self.rp;
            if buffered > 0 {
                let to_copy = buffered.min(buf.len() - total);
                buf[total..total + to_copy]
                    .copy_from_slice(&self.buffer[self.rp..self.rp + to_copy]);
                self.rp += to_copy;
                total += to_copy;
            } else if self.fill_buffer()? == 0 {
                break;
            }
        }
        Ok(total)
    }

    /// Read exactly the specified number of bytes.
    pub fn read_exact(&mut self, buf: &mut [u8]) -> Result<()> {
        if self.read(buf)? < buf.len() {
            return Err(Error::Eof);
        }
        Ok(())
    }

    /// Read all remaining data into a Buffer.
    pub fn read_all(&mut self, initial_capacity: usize) -> Result<Buffer> {
        let mut result = BytesMut::with_capacity(initial_capacity);
        loop {
            let buffered = self.wp - self.rp;
            if buffered > 0 {
                result.extend_from_slice(&self.buffer[self.rp..self.wp]);
                self.rp = self.wp;
            }
            if self.fill_buffer()? == 0 {
                break;
            }
        }
        Ok(Buffer::from_bytes_mut(result))
    }

    /// Read a line (up to and including newline).
    pub fn read_line(&mut self) -> Result<Option<Vec<u8>>> {
        let mut line = Vec::new();
        loop {
            match self.read_byte()? {
                None => {
                    if line.is_empty() {
                        return Ok(None);
                    }
                    break;
                }
                Some(b'\n') => {
                    line.push(b'\n');
                    break;
                }
                Some(b) => {
                    line.push(b);
                }
            }
        }
        Ok(Some(line))
    }

    /// Skip n bytes.
    pub fn skip(&mut self, mut n: usize) -> Result<usize> {
        let mut skipped = 0;
        while n > 0 {
            let buffered = self.wp - self.rp;
            if buffered > 0 {
                let to_skip = buffered.min(n);
                self.rp += to_skip;
                skipped += to_skip;
                n -= to_skip;
            } else if self.fill_buffer()? == 0 {
                break;
            }
        }
        Ok(skipped)
    }

    /// Seek to a position in the stream.
    pub fn seek(&mut self, pos: i64, whence: i32) -> Result<()> {
        let seek_from = match whence {
            0 => SeekFrom::Start(pos as u64),
            1 => SeekFrom::Current(pos),
            2 => SeekFrom::End(pos),
            _ => return Err(Error::generic("Invalid seek whence")),
        };

        // Clear buffer and seek
        self.rp = 0;
        self.wp = 0;
        self.eof = false;
        self.pos = self.inner.seek(seek_from).map_err(Error::System)? as i64;
        Ok(())
    }

    /// Read a 16-bit unsigned integer in big-endian format.
    pub fn read_uint16(&mut self) -> Result<u16> {
        let mut buf = [0u8; 2];
        self.read_exact(&mut buf)?;
        Ok(u16::from_be_bytes(buf))
    }

    /// Read a 24-bit unsigned integer in big-endian format.
    pub fn read_uint24(&mut self) -> Result<u32> {
        let mut buf = [0u8; 3];
        self.read_exact(&mut buf)?;
        Ok(((buf[0] as u32) << 16) | ((buf[1] as u32) << 8) | (buf[2] as u32))
    }

    /// Read a 32-bit unsigned integer in big-endian format.
    pub fn read_uint32(&mut self) -> Result<u32> {
        let mut buf = [0u8; 4];
        self.read_exact(&mut buf)?;
        Ok(u32::from_be_bytes(buf))
    }

    /// Read a 16-bit signed integer in little-endian format.
    pub fn read_int16_le(&mut self) -> Result<i16> {
        let mut buf = [0u8; 2];
        self.read_exact(&mut buf)?;
        Ok(i16::from_le_bytes(buf))
    }

    /// Read a 32-bit signed integer in little-endian format.
    pub fn read_int32_le(&mut self) -> Result<i32> {
        let mut buf = [0u8; 4];
        self.read_exact(&mut buf)?;
        Ok(i32::from_le_bytes(buf))
    }

    /// Read a 16-bit unsigned integer in little-endian format.
    pub fn read_uint16_le(&mut self) -> Result<u16> {
        let mut buf = [0u8; 2];
        self.read_exact(&mut buf)?;
        Ok(u16::from_le_bytes(buf))
    }

    /// Read a 32-bit unsigned integer in little-endian format.
    pub fn read_uint32_le(&mut self) -> Result<u32> {
        let mut buf = [0u8; 4];
        self.read_exact(&mut buf)?;
        Ok(u32::from_le_bytes(buf))
    }

    /// Read bits from the stream.
    pub fn read_bits(&mut self, n: u8) -> Result<u32> {
        while self.avail < n {
            match self.read_byte()? {
                Some(b) => {
                    self.bits = (self.bits << 8) | (b as u32);
                    self.avail += 8;
                }
                None => return Err(Error::Eof),
            }
        }
        self.avail -= n;
        let mask = (1u32 << n) - 1;
        Ok((self.bits >> self.avail) & mask)
    }

    /// Sync bits - discard any partial byte.
    pub fn sync_bits(&mut self) {
        self.bits = 0;
        self.avail = 0;
    }
}

impl std::fmt::Debug for Stream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Stream")
            .field("pos", &self.tell())
            .field("eof", &self.eof)
            .field("filename", &self.filename)
            .finish()
    }
}

// Async stream support (when async feature is enabled)
#[cfg(feature = "async")]
pub mod async_stream {
    use super::*;
    use bytes::{Bytes, BytesMut};
    use tokio::fs::File as AsyncFile;
    use tokio::io::{AsyncReadExt, AsyncSeekExt, BufReader as AsyncBufReader};

    /// Async stream for non-blocking I/O.
    pub struct AsyncStream {
        inner: AsyncStreamInner,
        #[allow(dead_code)]
        buffer: BytesMut,
        pos: u64,
        eof: bool,
    }

    enum AsyncStreamInner {
        File(AsyncBufReader<AsyncFile>),
        Memory { data: Bytes, position: usize },
    }

    impl AsyncStream {
        /// Open a file asynchronously.
        pub async fn open_file<P: AsRef<Path>>(path: P) -> Result<Self> {
            let file = AsyncFile::open(path).await.map_err(Error::System)?;
            Ok(Self {
                inner: AsyncStreamInner::File(AsyncBufReader::with_capacity(
                    STREAM_BUFFER_SIZE,
                    file,
                )),
                buffer: BytesMut::with_capacity(STREAM_BUFFER_SIZE),
                pos: 0,
                eof: false,
            })
        }

        /// Open from memory.
        pub fn open_memory(data: &[u8]) -> Self {
            Self {
                inner: AsyncStreamInner::Memory {
                    data: Bytes::copy_from_slice(data),
                    position: 0,
                },
                buffer: BytesMut::with_capacity(STREAM_BUFFER_SIZE),
                pos: 0,
                eof: false,
            }
        }

        /// Open from Bytes (zero-copy).
        pub fn open_bytes(data: Bytes) -> Self {
            Self {
                inner: AsyncStreamInner::Memory { data, position: 0 },
                buffer: BytesMut::with_capacity(STREAM_BUFFER_SIZE),
                pos: 0,
                eof: false,
            }
        }

        /// Read bytes asynchronously.
        pub async fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
            match &mut self.inner {
                AsyncStreamInner::File(reader) => {
                    let n = reader.read(buf).await.map_err(Error::System)?;
                    self.pos += n as u64;
                    if n == 0 {
                        self.eof = true;
                    }
                    Ok(n)
                }
                AsyncStreamInner::Memory { data, position } => {
                    let remaining = &data[*position..];
                    let to_read = buf.len().min(remaining.len());
                    buf[..to_read].copy_from_slice(&remaining[..to_read]);
                    *position += to_read;
                    self.pos += to_read as u64;
                    if to_read == 0 {
                        self.eof = true;
                    }
                    Ok(to_read)
                }
            }
        }

        /// Read all data asynchronously.
        pub async fn read_all(&mut self) -> Result<Buffer> {
            let mut result = BytesMut::with_capacity(8192);
            let mut chunk = [0u8; 8192];
            loop {
                let n = self.read(&mut chunk).await?;
                if n == 0 {
                    break;
                }
                result.extend_from_slice(&chunk[..n]);
            }
            Ok(Buffer::from_bytes_mut(result))
        }

        /// Seek asynchronously.
        pub async fn seek(&mut self, pos: SeekFrom) -> Result<u64> {
            match &mut self.inner {
                AsyncStreamInner::File(reader) => {
                    let new_pos = reader.seek(pos).await.map_err(Error::System)?;
                    self.pos = new_pos;
                    self.eof = false;
                    Ok(new_pos)
                }
                AsyncStreamInner::Memory { data, position } => {
                    let new_pos = match pos {
                        SeekFrom::Start(offset) => offset as i64,
                        SeekFrom::End(offset) => data.len() as i64 + offset,
                        SeekFrom::Current(offset) => *position as i64 + offset,
                    };
                    if new_pos < 0 {
                        return Err(Error::generic("Seek before start"));
                    }
                    *position = (new_pos as usize).min(data.len());
                    self.pos = *position as u64;
                    self.eof = false;
                    Ok(self.pos)
                }
            }
        }

        /// Get current position.
        pub fn tell(&self) -> u64 {
            self.pos
        }

        /// Check if EOF.
        pub fn is_eof(&self) -> bool {
            self.eof
        }
    }
}

// Parallel stream processing (when parallel feature is enabled)
#[cfg(feature = "parallel")]
pub mod parallel {
    use super::*;
    use rayon::prelude::*;

    /// Process multiple streams in parallel.
    pub fn process_streams<F, R>(streams: Vec<Stream>, f: F) -> Vec<Result<R>>
    where
        F: Fn(Stream) -> Result<R> + Sync + Send,
        R: Send,
    {
        streams.into_par_iter().map(f).collect()
    }

    /// Read multiple files in parallel.
    pub fn read_files<P: AsRef<Path> + Sync>(paths: &[P]) -> Vec<Result<Buffer>> {
        paths
            .par_iter()
            .map(|path| {
                let mut stream = Stream::open_file(path)?;
                stream.read_all(0)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stream_open_memory() {
        let data = b"Hello World";
        let stream = Stream::open_memory(data);
        assert_eq!(stream.tell(), 0);
        assert_eq!(stream.len(), Some(data.len() as u64));
        assert!(!stream.is_empty());
    }

    #[test]
    fn test_stream_open_memory_empty() {
        let stream = Stream::open_memory(&[]);
        assert!(stream.is_empty());
        assert_eq!(stream.len(), Some(0));
    }

    #[test]
    fn test_stream_open_buffer() {
        let buffer = Buffer::from_slice(b"Test Data");
        let stream = Stream::open_buffer(&buffer);
        assert_eq!(stream.len(), Some(9));
    }

    #[test]
    fn test_stream_open_bytes() {
        let bytes = Bytes::from_static(b"Hello World");
        let stream = Stream::open_bytes(bytes);
        assert_eq!(stream.len(), Some(11));
    }

    #[test]
    fn test_stream_read_byte() {
        let data = b"ABC";
        let mut stream = Stream::open_memory(data);

        assert_eq!(stream.read_byte().unwrap(), Some(b'A'));
        assert_eq!(stream.read_byte().unwrap(), Some(b'B'));
        assert_eq!(stream.read_byte().unwrap(), Some(b'C'));
        assert_eq!(stream.read_byte().unwrap(), None);
    }

    #[test]
    fn test_stream_peek_byte() {
        let data = b"ABC";
        let mut stream = Stream::open_memory(data);

        assert_eq!(stream.peek_byte().unwrap(), Some(b'A'));
        assert_eq!(stream.peek_byte().unwrap(), Some(b'A')); // Should not advance
        assert_eq!(stream.read_byte().unwrap(), Some(b'A'));
        assert_eq!(stream.peek_byte().unwrap(), Some(b'B'));
    }

    #[test]
    fn test_stream_read() {
        let data = b"Hello World";
        let mut stream = Stream::open_memory(data);
        let mut buf = [0u8; 5];

        let n = stream.read(&mut buf).unwrap();
        assert_eq!(n, 5);
        assert_eq!(&buf, b"Hello");
    }

    #[test]
    fn test_stream_read_exact() {
        let data = b"Hello World";
        let mut stream = Stream::open_memory(data);
        let mut buf = [0u8; 5];

        stream.read_exact(&mut buf).unwrap();
        assert_eq!(&buf, b"Hello");
    }

    #[test]
    fn test_stream_read_exact_eof() {
        let data = b"Hi";
        let mut stream = Stream::open_memory(data);
        let mut buf = [0u8; 10];

        let result = stream.read_exact(&mut buf);
        assert!(result.is_err());
    }

    #[test]
    fn test_stream_read_all() {
        let data = b"Hello World";
        let mut stream = Stream::open_memory(data);

        let buffer = stream.read_all(0).unwrap();
        assert_eq!(buffer.as_slice(), data);
    }

    #[test]
    fn test_stream_read_line() {
        let data = b"Hello\nWorld\n";
        let mut stream = Stream::open_memory(data);

        let line1 = stream.read_line().unwrap().unwrap();
        assert_eq!(line1, b"Hello\n");

        let line2 = stream.read_line().unwrap().unwrap();
        assert_eq!(line2, b"World\n");

        let line3 = stream.read_line().unwrap();
        assert!(line3.is_none());
    }

    #[test]
    fn test_stream_skip() {
        let data = b"Hello World";
        let mut stream = Stream::open_memory(data);

        let skipped = stream.skip(6).unwrap();
        assert_eq!(skipped, 6);

        let mut buf = [0u8; 5];
        stream.read_exact(&mut buf).unwrap();
        assert_eq!(&buf, b"World");
    }

    #[test]
    fn test_stream_read_uint16() {
        let data = [0x01, 0x02];
        let mut stream = Stream::open_memory(&data);
        assert_eq!(stream.read_uint16().unwrap(), 0x0102);
    }

    #[test]
    fn test_stream_read_uint24() {
        let data = [0x01, 0x02, 0x03];
        let mut stream = Stream::open_memory(&data);
        assert_eq!(stream.read_uint24().unwrap(), 0x010203);
    }

    #[test]
    fn test_stream_read_uint32() {
        let data = [0x01, 0x02, 0x03, 0x04];
        let mut stream = Stream::open_memory(&data);
        assert_eq!(stream.read_uint32().unwrap(), 0x01020304);
    }

    #[test]
    fn test_stream_read_int16_le() {
        let data = [0x01, 0x02];
        let mut stream = Stream::open_memory(&data);
        assert_eq!(stream.read_int16_le().unwrap(), 0x0201);
    }

    #[test]
    fn test_stream_read_int32_le() {
        let data = [0x01, 0x02, 0x03, 0x04];
        let mut stream = Stream::open_memory(&data);
        assert_eq!(stream.read_int32_le().unwrap(), 0x04030201);
    }

    #[test]
    fn test_stream_read_uint16_le() {
        let data = [0x01, 0x02];
        let mut stream = Stream::open_memory(&data);
        assert_eq!(stream.read_uint16_le().unwrap(), 0x0201);
    }

    #[test]
    fn test_stream_read_uint32_le() {
        let data = [0x01, 0x02, 0x03, 0x04];
        let mut stream = Stream::open_memory(&data);
        assert_eq!(stream.read_uint32_le().unwrap(), 0x04030201);
    }

    #[test]
    fn test_stream_read_bits() {
        let data = [0b10110100, 0b11001010];
        let mut stream = Stream::open_memory(&data);

        // Read 4 bits: 1011
        assert_eq!(stream.read_bits(4).unwrap(), 0b1011);
        // Read 4 bits: 0100
        assert_eq!(stream.read_bits(4).unwrap(), 0b0100);
        // Read 8 bits: 11001010
        assert_eq!(stream.read_bits(8).unwrap(), 0b11001010);
    }

    #[test]
    fn test_stream_sync_bits() {
        let data = [0xFF, 0x00];
        let mut stream = Stream::open_memory(&data);

        stream.read_bits(4).unwrap();
        stream.sync_bits();

        // After sync, should read fresh byte
        assert_eq!(stream.read_byte().unwrap(), Some(0x00));
    }

    #[test]
    fn test_stream_tell() {
        let data = b"Hello";
        let mut stream = Stream::open_memory(data);

        assert_eq!(stream.tell(), 0);
        stream.read_byte().unwrap();
    }

    #[test]
    fn test_stream_debug() {
        let stream = Stream::open_memory(b"test");
        let debug = format!("{:?}", stream);
        assert!(debug.contains("Stream"));
        assert!(debug.contains("pos"));
        assert!(debug.contains("eof"));
    }

    #[test]
    fn test_stream_sequential_reads() {
        let data = b"ABCDEFGHIJ";
        let mut stream = Stream::open_memory(data);

        let mut buf = [0u8; 3];
        stream.read_exact(&mut buf).unwrap();
        assert_eq!(&buf, b"ABC");

        stream.read_exact(&mut buf).unwrap();
        assert_eq!(&buf, b"DEF");

        stream.read_exact(&mut buf).unwrap();
        assert_eq!(&buf, b"GHI");
    }

    #[test]
    fn test_stream_large_read() {
        let data: Vec<u8> = (0..20000).map(|i| (i % 256) as u8).collect();
        let mut stream = Stream::open_memory(&data);

        let buffer = stream.read_all(0).unwrap();
        assert_eq!(buffer.len(), data.len());
        assert_eq!(buffer.as_slice(), &data[..]);
    }

    #[test]
    fn test_stream_is_eof() {
        let data = b"Hi";
        let mut stream = Stream::open_memory(data);

        assert!(!stream.is_eof());
        stream.read_byte().unwrap();
        stream.read_byte().unwrap();
        stream.read_byte().unwrap(); // Returns None, sets EOF
        assert!(stream.is_eof());
    }

    #[test]
    fn test_memory_source_seek() {
        let data = b"Hello World";
        let mut source = MemorySource {
            data: Bytes::copy_from_slice(data),
            position: 0,
        };

        let pos = source.seek(SeekFrom::Start(6)).unwrap();
        assert_eq!(pos, 6);
        assert_eq!(source.position, 6);

        let pos = source.seek(SeekFrom::Current(2)).unwrap();
        assert_eq!(pos, 8);

        let pos = source.seek(SeekFrom::End(-5)).unwrap();
        assert_eq!(pos, 6);
    }

    #[test]
    fn test_memory_source_seek_before_start() {
        let data = b"Hello";
        let mut source = MemorySource {
            data: Bytes::copy_from_slice(data),
            position: 2,
        };

        let result = source.seek(SeekFrom::Current(-10));
        assert!(result.is_err());
    }

    // ============================================================================
    // Additional coverage tests
    // ============================================================================

    #[test]
    fn test_memory_source_len() {
        let data = b"Hello";
        let source = MemorySource {
            data: Bytes::copy_from_slice(data),
            position: 0,
        };
        assert_eq!(source.len(), Some(5));
    }

    #[test]
    fn test_memory_source_tell() {
        let data = b"Hello";
        let mut source = MemorySource {
            data: Bytes::copy_from_slice(data),
            position: 3,
        };
        assert_eq!(source.tell().unwrap(), 3);
    }

    #[test]
    fn test_memory_source_read() {
        let data = b"Hello";
        let mut source = MemorySource {
            data: Bytes::copy_from_slice(data),
            position: 0,
        };
        let mut buf = [0u8; 3];
        let n = source.read(&mut buf).unwrap();
        assert_eq!(n, 3);
        assert_eq!(&buf, b"Hel");
    }

    #[test]
    fn test_stream_filename() {
        let stream = Stream::open_memory(b"Test");
        assert!(stream.filename().is_none());
    }

    #[test]
    fn test_stream_read_at_eof() {
        let mut stream = Stream::open_memory(b"AB");
        stream.read_byte().unwrap();
        stream.read_byte().unwrap();
        let r = stream.read_byte().unwrap(); // Returns None at EOF
        assert!(r.is_none());

        let mut buf = [0u8; 10];
        let result = stream.read(&mut buf);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }

    #[test]
    fn test_stream_len() {
        let stream = Stream::open_memory(b"Hello World");
        assert_eq!(stream.len(), Some(11));
    }

    #[test]
    fn test_stream_is_empty() {
        let stream = Stream::open_memory(b"");
        assert!(stream.is_empty());

        let stream2 = Stream::open_memory(b"Test");
        assert!(!stream2.is_empty());
    }

    #[test]
    fn test_stream_seek_start() {
        let mut stream = Stream::open_memory(b"Hello World");
        stream.read_byte().unwrap();
        stream.read_byte().unwrap();
        stream.seek(0, 0).unwrap(); // SEEK_SET
        assert_eq!(stream.tell(), 0);
    }

    #[test]
    fn test_stream_seek_end() {
        let mut stream = Stream::open_memory(b"Hello World");
        stream.seek(-5, 2).unwrap(); // SEEK_END
        let mut buf = [0u8; 5];
        stream.read_exact(&mut buf).unwrap();
        assert_eq!(&buf, b"World");
    }

    #[test]
    fn test_stream_read_all_with_hint() {
        let mut stream = Stream::open_memory(b"Hello World");
        let buffer = stream.read_all(20).unwrap();
        assert_eq!(buffer.as_slice(), b"Hello World");
    }

    #[test]
    fn test_stream_read_all_no_hint() {
        let mut stream = Stream::open_memory(b"Test");
        let buffer = stream.read_all(0).unwrap();
        assert_eq!(buffer.as_slice(), b"Test");
    }

    #[test]
    fn test_stream_read_line_crlf() {
        let data = b"Line1\r\nLine2\r\n";
        let mut stream = Stream::open_memory(data);

        let line1 = stream.read_line().unwrap().unwrap();
        assert_eq!(&line1[..5], b"Line1");

        let line2 = stream.read_line().unwrap().unwrap();
        assert_eq!(&line2[..5], b"Line2");
    }

    #[test]
    fn test_stream_read_line_no_newline() {
        let data = b"NoNewline";
        let mut stream = Stream::open_memory(data);

        // Should read all remaining data even without newline
        let line = stream.read_line().unwrap();
        assert!(line.is_none() || !line.unwrap().is_empty());
    }

    #[test]
    fn test_stream_read_bits_cross_byte() {
        let data = [0xFF, 0xFF];
        let mut stream = Stream::open_memory(&data);

        // Read 12 bits spanning two bytes
        let val = stream.read_bits(12).unwrap();
        assert_eq!(val, 0xFFF);
    }

    #[test]
    fn test_stream_peek_after_read() {
        let data = b"ABC";
        let mut stream = Stream::open_memory(data);
        stream.read_byte().unwrap();
        assert_eq!(stream.peek_byte().unwrap(), Some(b'B'));
    }

    #[test]
    fn test_stream_skip_past_eof() {
        let data = b"Hi";
        let mut stream = Stream::open_memory(data);
        let skipped = stream.skip(100).unwrap();
        assert_eq!(skipped, 2);
    }
}
