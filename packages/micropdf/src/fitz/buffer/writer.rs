//! BufferWriter for accumulating data into a buffer

use super::core::Buffer;
use bytes::{BufMut, Bytes, BytesMut};
use std::io::{self, Write};

/// A writer that accumulates data into a buffer.
pub struct BufferWriter {
    inner: BytesMut,
}

impl BufferWriter {
    /// Create a new buffer writer.
    pub fn new() -> Self {
        Self {
            inner: BytesMut::with_capacity(256),
        }
    }

    /// Create a new buffer writer with the specified capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            inner: BytesMut::with_capacity(capacity),
        }
    }

    /// Convert the writer into a Buffer.
    pub fn into_buffer(self) -> Buffer {
        Buffer::from_bytes_mut(self.inner)
    }

    /// Get a reference to the accumulated data.
    pub fn as_slice(&self) -> &[u8] {
        &self.inner
    }

    /// Get the current length.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Write a byte.
    pub fn write_byte(&mut self, byte: u8) {
        self.inner.put_u8(byte);
    }

    /// Write a 16-bit integer in big-endian format.
    pub fn write_u16_be(&mut self, value: u16) {
        self.inner.put_u16(value);
    }

    /// Write a 32-bit integer in big-endian format.
    pub fn write_u32_be(&mut self, value: u32) {
        self.inner.put_u32(value);
    }

    /// Write a 16-bit integer in little-endian format.
    pub fn write_u16_le(&mut self, value: u16) {
        self.inner.put_u16_le(value);
    }

    /// Write a 32-bit integer in little-endian format.
    pub fn write_u32_le(&mut self, value: u32) {
        self.inner.put_u32_le(value);
    }

    /// Convert the writer into Bytes.
    pub fn into_bytes(self) -> Bytes {
        self.inner.freeze()
    }

    /// Clear the writer's contents.
    pub fn clear(&mut self) {
        self.inner.clear();
    }
}

impl Default for BufferWriter {
    fn default() -> Self {
        Self::new()
    }
}

impl Write for BufferWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

// Parallel buffer operations (when rayon feature is enabled)
#[cfg(feature = "parallel")]
pub mod parallel {
    use super::*;
    use rayon::prelude::*;

    /// Process multiple buffers in parallel.
    pub fn process_buffers<F, R>(buffers: &[Buffer], f: F) -> Vec<R>
    where
        F: Fn(&Buffer) -> R + Sync + Send,
        R: Send,
    {
        buffers.par_iter().map(f).collect()
    }

    /// Apply a transformation to buffer data in parallel chunks.
    pub fn parallel_transform<F>(buffer: &Buffer, chunk_size: usize, f: F) -> Buffer
    where
        F: Fn(&[u8]) -> Vec<u8> + Sync + Send,
    {
        let data = buffer.to_vec();
        let chunks: Vec<Vec<u8>> = data.par_chunks(chunk_size).map(f).collect();

        let total_len: usize = chunks.iter().map(|c| c.len()).sum();
        let mut result = BytesMut::with_capacity(total_len);
        for chunk in chunks {
            result.extend_from_slice(&chunk);
        }
        Buffer::from_bytes_mut(result)
    }
}

// Async buffer operations (when async feature is enabled)
#[cfg(feature = "async")]
pub mod async_ops {
    use super::*;
    use crate::fitz::error::{Error, Result};
    use std::pin::Pin;
    use std::task::{Context, Poll};
    use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

    /// Read a buffer from an async reader.
    pub async fn read_buffer<R: AsyncRead + Unpin>(
        reader: &mut R,
        max_size: usize,
    ) -> Result<Buffer> {
        let mut data = Vec::with_capacity(max_size.min(8192));
        let mut chunk = [0u8; 8192];

        loop {
            let n = reader.read(&mut chunk).await.map_err(Error::System)?;
            if n == 0 {
                break;
            }
            if data.len() + n > max_size {
                return Err(Error::generic("Buffer size limit exceeded"));
            }
            data.extend_from_slice(&chunk[..n]);
        }

        Ok(Buffer::from_data(data))
    }

    /// Write a buffer to an async writer.
    pub async fn write_buffer<W: AsyncWrite + Unpin>(
        writer: &mut W,
        buffer: &Buffer,
    ) -> Result<()> {
        writer
            .write_all(&buffer.to_vec())
            .await
            .map_err(Error::System)
    }

    /// Async buffer reader.
    pub struct AsyncBufferReader {
        data: Bytes,
        position: usize,
    }

    impl AsyncBufferReader {
        pub fn new(buffer: Buffer) -> Self {
            Self {
                data: buffer.to_bytes(),
                position: 0,
            }
        }
    }

    impl AsyncRead for AsyncBufferReader {
        fn poll_read(
            mut self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
            buf: &mut tokio::io::ReadBuf<'_>,
        ) -> Poll<io::Result<()>> {
            let remaining = &self.data[self.position..];
            let to_read = buf.remaining().min(remaining.len());
            buf.put_slice(&remaining[..to_read]);
            self.position += to_read;
            Poll::Ready(Ok(()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buffer_writer() {
        let mut writer = BufferWriter::new();
        writer.write_all(b"Hello").unwrap();
        writer.write_byte(b' ');
        writer.write_all(b"World").unwrap();

        let buf = writer.into_buffer();
        assert_eq!(buf.to_vec(), b"Hello World");
    }

    #[test]
    fn test_buffer_writer_integers() {
        let mut writer = BufferWriter::new();
        writer.write_u16_be(0x1234);
        writer.write_u32_be(0x12345678);
        writer.write_u16_le(0x1234);
        writer.write_u32_le(0x12345678);

        let buf = writer.into_buffer();
        assert_eq!(buf.len(), 12);
    }

    #[test]
    fn test_buffer_writer_clear() {
        let mut writer = BufferWriter::new();
        writer.write_all(b"Test").unwrap();
        assert_eq!(writer.len(), 4);

        writer.clear();
        assert_eq!(writer.len(), 0);
        assert!(writer.is_empty());
    }

    #[test]
    fn test_buffer_writer_with_capacity() {
        let writer = BufferWriter::with_capacity(1024);
        assert_eq!(writer.len(), 0);
        assert!(writer.is_empty());
    }

    #[test]
    fn test_buffer_writer_as_slice() {
        let mut writer = BufferWriter::new();
        writer.write_all(b"Test data").unwrap();

        let slice = writer.as_slice();
        assert_eq!(slice, b"Test data");
    }

    #[test]
    fn test_buffer_writer_into_bytes() {
        let mut writer = BufferWriter::new();
        writer.write_all(b"Convert to bytes").unwrap();

        let bytes = writer.into_bytes();
        assert_eq!(bytes.as_ref(), b"Convert to bytes");
    }

    #[test]
    fn test_buffer_writer_default() {
        let writer = BufferWriter::default();
        assert_eq!(writer.len(), 0);
        assert!(writer.is_empty());
    }

    #[test]
    fn test_buffer_writer_write_trait() {
        let mut writer = BufferWriter::new();

        // Test Write::write
        let n = writer.write(b"Hello").unwrap();
        assert_eq!(n, 5);

        // Test Write::flush
        writer.flush().unwrap();

        assert_eq!(writer.len(), 5);
        assert_eq!(writer.as_slice(), b"Hello");
    }

    #[test]
    fn test_buffer_writer_multiple_writes() {
        let mut writer = BufferWriter::new();

        writer.write_byte(0x01);
        writer.write_u16_be(0x0203);
        writer.write_u32_be(0x04050607);
        writer.write_u16_le(0x0809);
        writer.write_u32_le(0x0A0B0C0D);

        let buf = writer.into_buffer();
        assert_eq!(buf.len(), 13); // 1 + 2 + 4 + 2 + 4
    }

    #[test]
    fn test_buffer_writer_empty() {
        let writer = BufferWriter::new();
        assert!(writer.is_empty());
        assert_eq!(writer.len(), 0);

        let buf = writer.into_buffer();
        assert!(buf.is_empty());
    }

    #[test]
    fn test_buffer_writer_large_write() {
        let mut writer = BufferWriter::new();
        let large_data = vec![0xAA; 10000];

        writer.write_all(&large_data).unwrap();
        assert_eq!(writer.len(), 10000);

        let buf = writer.into_buffer();
        assert_eq!(buf.len(), 10000);
    }

    #[test]
    fn test_buffer_writer_sequential_bytes() {
        let mut writer = BufferWriter::new();

        for i in 0..10u8 {
            writer.write_byte(i);
        }

        let buf = writer.into_buffer();
        assert_eq!(buf.to_vec(), vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
    }
}
