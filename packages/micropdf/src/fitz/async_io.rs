//! Async I/O utilities using Tokio
//!
//! This module provides asynchronous I/O operations for PDF processing,
//! enabling non-blocking file operations and concurrent processing.

use bytes::{Bytes, BytesMut};
use std::path::Path;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader};

use crate::fitz::buffer::Buffer;
use crate::fitz::error::{Error, Result};

/// Read a file asynchronously into a Buffer.
///
/// # Example
/// ```ignore
/// use micropdf::fitz::async_io::read_file;
///
/// #[tokio::main]
/// async fn main() {
///     let buffer = read_file("document.pdf").await.unwrap();
///     println!("Read {} bytes", buffer.len());
/// }
/// ```
pub async fn read_file<P: AsRef<Path>>(path: P) -> Result<Buffer> {
    let mut file = File::open(path).await.map_err(Error::System)?;
    let mut data = Vec::new();
    file.read_to_end(&mut data).await.map_err(Error::System)?;
    Ok(Buffer::from_data(data))
}

/// Read a file asynchronously with a size limit.
pub async fn read_file_limited<P: AsRef<Path>>(path: P, max_size: usize) -> Result<Buffer> {
    let file = File::open(path).await.map_err(Error::System)?;
    let mut reader = BufReader::new(file);
    let mut data = Vec::with_capacity(max_size.min(8192));
    let mut chunk = [0u8; 8192];

    loop {
        let n = reader.read(&mut chunk).await.map_err(Error::System)?;
        if n == 0 {
            break;
        }
        if data.len() + n > max_size {
            return Err(Error::generic("File size exceeds limit"));
        }
        data.extend_from_slice(&chunk[..n]);
    }

    Ok(Buffer::from_data(data))
}

/// Write a buffer to a file asynchronously.
///
/// # Example
/// ```ignore
/// use micropdf::fitz::async_io::write_file;
/// use micropdf::fitz::buffer::Buffer;
///
/// #[tokio::main]
/// async fn main() {
///     let buffer = Buffer::from_slice(b"Hello, PDF!");
///     write_file("output.txt", &buffer).await.unwrap();
/// }
/// ```
pub async fn write_file<P: AsRef<Path>>(path: P, buffer: &Buffer) -> Result<()> {
    let mut file = File::create(path).await.map_err(Error::System)?;
    file.write_all(&buffer.to_vec())
        .await
        .map_err(Error::System)?;
    file.flush().await.map_err(Error::System)?;
    Ok(())
}

/// Copy a file asynchronously.
pub async fn copy_file<P: AsRef<Path>, Q: AsRef<Path>>(src: P, dst: Q) -> Result<u64> {
    tokio::fs::copy(src, dst).await.map_err(Error::System)
}

/// Check if a file exists asynchronously.
pub async fn file_exists<P: AsRef<Path>>(path: P) -> bool {
    tokio::fs::metadata(path).await.is_ok()
}

/// Get file size asynchronously.
pub async fn file_size<P: AsRef<Path>>(path: P) -> Result<u64> {
    let metadata = tokio::fs::metadata(path).await.map_err(Error::System)?;
    Ok(metadata.len())
}

/// Read multiple files concurrently.
///
/// # Example
/// ```ignore
/// use micropdf::fitz::async_io::read_files_concurrent;
///
/// #[tokio::main]
/// async fn main() {
///     let paths = vec!["file1.pdf", "file2.pdf", "file3.pdf"];
///     let results = read_files_concurrent(&paths).await;
/// }
/// ```
pub async fn read_files_concurrent<P: AsRef<Path> + Send + Sync>(
    paths: &[P],
) -> Vec<Result<Buffer>> {
    let futures: Vec<_> = paths
        .iter()
        .map(|path| {
            let path = path.as_ref().to_path_buf();
            async move { read_file(&path).await }
        })
        .collect();

    futures::future::join_all(futures).await
}

/// Write multiple buffers to files concurrently.
pub async fn write_files_concurrent<P: AsRef<Path> + Send + Sync>(
    paths: &[P],
    buffers: &[Buffer],
) -> Vec<Result<()>> {
    if paths.len() != buffers.len() {
        return vec![Err(Error::generic("Mismatched paths and buffers count"))];
    }

    let futures: Vec<_> = paths
        .iter()
        .zip(buffers.iter())
        .map(|(path, buffer)| {
            let path = path.as_ref().to_path_buf();
            let data = buffer.to_vec();
            async move {
                let mut file = File::create(&path).await.map_err(Error::System)?;
                file.write_all(&data).await.map_err(Error::System)?;
                file.flush().await.map_err(Error::System)?;
                Ok(())
            }
        })
        .collect();

    futures::future::join_all(futures).await
}

/// Async buffer for streaming data.
pub struct AsyncBuffer {
    inner: BytesMut,
}

impl AsyncBuffer {
    /// Create a new async buffer.
    pub fn new() -> Self {
        Self {
            inner: BytesMut::with_capacity(8192),
        }
    }

    /// Create with a specific capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            inner: BytesMut::with_capacity(capacity),
        }
    }

    /// Append data.
    pub fn append(&mut self, data: &[u8]) {
        self.inner.extend_from_slice(data);
    }

    /// Get the length.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Clear the buffer.
    pub fn clear(&mut self) {
        self.inner.clear();
    }

    /// Convert to Buffer.
    pub fn into_buffer(self) -> Buffer {
        Buffer::from_bytes_mut(self.inner)
    }

    /// Convert to Bytes.
    pub fn freeze(self) -> Bytes {
        self.inner.freeze()
    }

    /// Get as slice.
    pub fn as_slice(&self) -> &[u8] {
        &self.inner
    }
}

impl Default for AsyncBuffer {
    fn default() -> Self {
        Self::new()
    }
}

/// Run an async operation with a timeout.
pub async fn with_timeout<F, T>(duration: std::time::Duration, future: F) -> Result<T>
where
    F: std::future::Future<Output = Result<T>>,
{
    match tokio::time::timeout(duration, future).await {
        Ok(result) => result,
        Err(_) => Err(Error::generic("Operation timed out")),
    }
}

/// Spawn a task to run in the background.
pub fn spawn<F>(future: F) -> tokio::task::JoinHandle<F::Output>
where
    F: std::future::Future + Send + 'static,
    F::Output: Send + 'static,
{
    tokio::spawn(future)
}

/// Run multiple async operations concurrently and collect results.
pub async fn concurrent<F, T>(futures: Vec<F>) -> Vec<T>
where
    F: std::future::Future<Output = T>,
    T: Send,
{
    futures::future::join_all(futures).await
}

/// Run multiple async operations concurrently with a limit on parallelism.
pub async fn concurrent_limited<F, T>(futures: Vec<F>, limit: usize) -> Vec<T>
where
    F: std::future::Future<Output = T> + Send,
    T: Send,
{
    use futures::stream::{self, StreamExt};

    stream::iter(futures)
        .buffer_unordered(limit)
        .collect()
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_async_buffer() {
        let mut buf = AsyncBuffer::new();
        assert!(buf.is_empty());

        buf.append(b"Hello");
        assert_eq!(buf.len(), 5);

        buf.append(b" World");
        assert_eq!(buf.len(), 11);

        let buffer = buf.into_buffer();
        assert_eq!(buffer.as_slice(), b"Hello World");
    }

    #[tokio::test]
    async fn test_async_buffer_with_capacity() {
        let buf = AsyncBuffer::with_capacity(1024);
        assert!(buf.is_empty());
    }

    #[tokio::test]
    async fn test_async_buffer_clear() {
        let mut buf = AsyncBuffer::new();
        buf.append(b"Hello");
        assert_eq!(buf.len(), 5);
        buf.clear();
        assert!(buf.is_empty());
    }

    #[tokio::test]
    async fn test_async_buffer_as_slice() {
        let mut buf = AsyncBuffer::new();
        buf.append(b"Test");
        assert_eq!(buf.as_slice(), b"Test");
    }

    #[tokio::test]
    async fn test_async_buffer_freeze() {
        let mut buf = AsyncBuffer::new();
        buf.append(b"Freeze");
        let bytes = buf.freeze();
        assert_eq!(&bytes[..], b"Freeze");
    }

    #[tokio::test]
    async fn test_async_buffer_default() {
        let buf: AsyncBuffer = Default::default();
        assert!(buf.is_empty());
    }

    #[tokio::test]
    async fn test_read_write_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.txt");

        let buffer = Buffer::from_slice(b"Hello, World!");
        write_file(&path, &buffer).await.unwrap();

        let read_buffer = read_file(&path).await.unwrap();
        assert_eq!(read_buffer.as_slice(), b"Hello, World!");
    }

    #[tokio::test]
    async fn test_file_exists() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.txt");

        assert!(!file_exists(&path).await);

        write_file(&path, &Buffer::from_slice(b"test"))
            .await
            .unwrap();

        assert!(file_exists(&path).await);
    }

    #[tokio::test]
    async fn test_file_size() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.txt");

        let data = b"Hello, World!";
        write_file(&path, &Buffer::from_slice(data)).await.unwrap();

        let size = file_size(&path).await.unwrap();
        assert_eq!(size, data.len() as u64);
    }

    #[tokio::test]
    async fn test_copy_file() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("src.txt");
        let dst = dir.path().join("dst.txt");

        let data = b"Copy me!";
        write_file(&src, &Buffer::from_slice(data)).await.unwrap();

        let copied = copy_file(&src, &dst).await.unwrap();
        assert_eq!(copied, data.len() as u64);

        let read_buffer = read_file(&dst).await.unwrap();
        assert_eq!(read_buffer.as_slice(), data);
    }

    #[tokio::test]
    async fn test_concurrent() {
        async fn task_1() -> i32 {
            1
        }
        async fn task_2() -> i32 {
            2
        }
        async fn task_3() -> i32 {
            3
        }

        let futures: Vec<std::pin::Pin<Box<dyn std::future::Future<Output = i32> + Send>>> =
            vec![Box::pin(task_1()), Box::pin(task_2()), Box::pin(task_3())];

        let results = concurrent(futures).await;
        assert_eq!(results, vec![1, 2, 3]);
    }

    #[tokio::test]
    async fn test_with_timeout_success() {
        let result = with_timeout(std::time::Duration::from_secs(1), async {
            Ok::<_, Error>(42)
        })
        .await;

        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn test_with_timeout_failure() {
        let result = with_timeout(std::time::Duration::from_millis(1), async {
            tokio::time::sleep(std::time::Duration::from_secs(10)).await;
            Ok::<_, Error>(42)
        })
        .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_read_files_concurrent() {
        let dir = tempdir().unwrap();
        let path1 = dir.path().join("file1.txt");
        let path2 = dir.path().join("file2.txt");

        write_file(&path1, &Buffer::from_slice(b"File 1"))
            .await
            .unwrap();
        write_file(&path2, &Buffer::from_slice(b"File 2"))
            .await
            .unwrap();

        let paths = vec![path1, path2];
        let results = read_files_concurrent(&paths).await;

        assert_eq!(results.len(), 2);
        assert!(results[0].is_ok());
        assert!(results[1].is_ok());
    }

    #[tokio::test]
    async fn test_write_files_concurrent() {
        let dir = tempdir().unwrap();
        let path1 = dir.path().join("write1.txt");
        let path2 = dir.path().join("write2.txt");

        let buf1 = Buffer::from_slice(b"Content 1");
        let buf2 = Buffer::from_slice(b"Content 2");

        let paths = vec![path1.clone(), path2.clone()];
        let buffers = vec![buf1, buf2];
        let results = write_files_concurrent(&paths, &buffers).await;

        assert_eq!(results.len(), 2);
        assert!(results[0].is_ok());
        assert!(results[1].is_ok());

        let read1 = read_file(&path1).await.unwrap();
        let read2 = read_file(&path2).await.unwrap();
        assert_eq!(read1.as_slice(), b"Content 1");
        assert_eq!(read2.as_slice(), b"Content 2");
    }

    #[tokio::test]
    async fn test_write_files_concurrent_mismatch() {
        let dir = tempdir().unwrap();
        let path1 = dir.path().join("write1.txt");

        let buf1 = Buffer::from_slice(b"Content 1");
        let buf2 = Buffer::from_slice(b"Content 2");

        let paths = vec![path1];
        let buffers = vec![buf1, buf2];
        let results = write_files_concurrent(&paths, &buffers).await;

        // Should return error because of mismatch
        assert_eq!(results.len(), 1);
        assert!(results[0].is_err());
    }

    #[tokio::test]
    async fn test_read_file_limited() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("large.txt");

        let data = vec![b'X'; 1000];
        write_file(&path, &Buffer::from_slice(&data)).await.unwrap();

        // Read with limit
        let result = read_file_limited(&path, 100).await;
        assert!(result.is_err()); // Should exceed limit
    }

    #[tokio::test]
    async fn test_concurrent_limited() {
        let futures: Vec<std::pin::Pin<Box<dyn std::future::Future<Output = i32> + Send>>> = vec![
            Box::pin(async { 1 }),
            Box::pin(async { 2 }),
            Box::pin(async { 3 }),
            Box::pin(async { 4 }),
        ];

        let results = concurrent_limited(futures, 2).await;
        assert_eq!(results.len(), 4);
        // Note: order may not be preserved due to unordered buffering
    }

    #[tokio::test]
    async fn test_spawn() {
        let handle = spawn(async { 42 });
        let result = handle.await.unwrap();
        assert_eq!(result, 42);
    }
}
