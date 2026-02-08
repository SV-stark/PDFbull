//! Buffered I/O Utilities
//!
//! Provides high-performance buffered writing:
//! - `BufferedWriter`: Batches small writes to reduce syscalls
//! - `VectoredWriter`: Uses scatter-gather I/O (writev)
//! - `AsyncWriter`: Async write support with tokio (optional)
//!
//! Benefits:
//! - Reduced syscall overhead (batching)
//! - Zero-copy with vectored I/O
//! - Background flushing with async

use std::fs::File;
use std::io::{self, BufWriter, IoSlice, Seek, SeekFrom, Write};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, LazyLock, Mutex};

use super::{Handle, HandleStore};

// ============================================================================
// Buffered Writer
// ============================================================================

/// Default buffer size (64KB)
pub const DEFAULT_BUFFER_SIZE: usize = 64 * 1024;

/// Statistics for buffered writer
#[derive(Debug, Default)]
pub struct WriterStats {
    /// Number of write calls
    pub writes: AtomicU64,
    /// Bytes written
    pub bytes_written: AtomicU64,
    /// Number of flushes
    pub flushes: AtomicU64,
    /// Bytes in buffer (current)
    pub buffered: AtomicU64,
}

/// High-performance buffered writer
pub struct BufferedWriter {
    /// Inner buffered writer
    inner: BufWriter<File>,
    /// Buffer capacity
    capacity: usize,
    /// Statistics
    stats: WriterStats,
    /// File path (for debugging)
    path: String,
}

impl BufferedWriter {
    /// Create a new buffered writer to a file
    pub fn new<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        Self::with_capacity(path, DEFAULT_BUFFER_SIZE)
    }

    /// Create with specific buffer capacity
    pub fn with_capacity<P: AsRef<Path>>(path: P, capacity: usize) -> io::Result<Self> {
        let path_str = path.as_ref().to_string_lossy().to_string();
        let file = File::create(path.as_ref())?;
        let inner = BufWriter::with_capacity(capacity, file);

        Ok(Self {
            inner,
            capacity,
            stats: WriterStats::default(),
            path: path_str,
        })
    }

    /// Open existing file for appending
    pub fn append<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        Self::append_with_capacity(path, DEFAULT_BUFFER_SIZE)
    }

    /// Open for append with specific capacity
    pub fn append_with_capacity<P: AsRef<Path>>(path: P, capacity: usize) -> io::Result<Self> {
        let path_str = path.as_ref().to_string_lossy().to_string();
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path.as_ref())?;
        let inner = BufWriter::with_capacity(capacity, file);

        Ok(Self {
            inner,
            capacity,
            stats: WriterStats::default(),
            path: path_str,
        })
    }

    /// Get buffer capacity
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Get current buffer usage
    pub fn buffered(&self) -> usize {
        self.inner.buffer().len()
    }

    /// Get path
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Get statistics
    pub fn stats(&self) -> &WriterStats {
        &self.stats
    }

    /// Write bytes
    pub fn write_bytes(&mut self, data: &[u8]) -> io::Result<usize> {
        self.stats.writes.fetch_add(1, Ordering::Relaxed);
        self.stats
            .bytes_written
            .fetch_add(data.len() as u64, Ordering::Relaxed);

        let written = self.inner.write(data)?;

        self.stats
            .buffered
            .store(self.buffered() as u64, Ordering::Relaxed);

        Ok(written)
    }

    /// Write a single byte
    pub fn write_byte(&mut self, byte: u8) -> io::Result<()> {
        self.write_bytes(&[byte])?;
        Ok(())
    }

    /// Write a string
    pub fn write_str(&mut self, s: &str) -> io::Result<usize> {
        self.write_bytes(s.as_bytes())
    }

    /// Write a line (with newline)
    pub fn write_line(&mut self, s: &str) -> io::Result<usize> {
        let written = self.write_str(s)?;
        self.write_byte(b'\n')?;
        Ok(written + 1)
    }

    /// Flush buffer to file
    pub fn flush(&mut self) -> io::Result<()> {
        self.stats.flushes.fetch_add(1, Ordering::Relaxed);
        self.inner.flush()?;
        self.stats.buffered.store(0, Ordering::Relaxed);
        Ok(())
    }

    /// Sync to disk (flush + fsync)
    pub fn sync(&mut self) -> io::Result<()> {
        self.flush()?;
        self.inner.get_mut().sync_all()
    }

    /// Get current file position
    pub fn position(&mut self) -> io::Result<u64> {
        // Flush first to ensure position is accurate
        self.flush()?;
        self.inner.get_mut().stream_position()
    }

    /// Seek to position (flushes buffer first)
    pub fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.flush()?;
        self.inner.get_mut().seek(pos)
    }
}

impl Write for BufferedWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.write_bytes(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        BufferedWriter::flush(self)
    }
}

impl Drop for BufferedWriter {
    fn drop(&mut self) {
        // Best effort flush on drop
        let _ = self.flush();
    }
}

// ============================================================================
// Vectored Writer (scatter-gather I/O)
// ============================================================================

/// Writer using vectored I/O for efficient multi-buffer writes
pub struct VectoredWriter {
    /// File handle
    file: File,
    /// Pending buffers
    pending: Vec<Vec<u8>>,
    /// Maximum pending bytes before auto-flush
    max_pending: usize,
    /// Current pending bytes
    pending_bytes: usize,
    /// Statistics
    stats: WriterStats,
    /// File path
    path: String,
}

impl VectoredWriter {
    /// Create a new vectored writer
    pub fn new<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        Self::with_max_pending(path, 256 * 1024) // 256KB default
    }

    /// Create with specific max pending bytes
    pub fn with_max_pending<P: AsRef<Path>>(path: P, max_pending: usize) -> io::Result<Self> {
        let path_str = path.as_ref().to_string_lossy().to_string();
        let file = File::create(path.as_ref())?;

        Ok(Self {
            file,
            pending: Vec::with_capacity(16),
            max_pending,
            pending_bytes: 0,
            stats: WriterStats::default(),
            path: path_str,
        })
    }

    /// Queue a buffer for writing
    pub fn queue(&mut self, data: Vec<u8>) -> io::Result<()> {
        self.stats.writes.fetch_add(1, Ordering::Relaxed);

        self.pending_bytes += data.len();
        self.pending.push(data);

        self.stats
            .buffered
            .store(self.pending_bytes as u64, Ordering::Relaxed);

        // Auto-flush if too much pending
        if self.pending_bytes >= self.max_pending {
            self.flush()?;
        }

        Ok(())
    }

    /// Queue a slice (copies data)
    pub fn queue_slice(&mut self, data: &[u8]) -> io::Result<()> {
        self.queue(data.to_vec())
    }

    /// Flush all pending buffers using vectored I/O
    pub fn flush(&mut self) -> io::Result<()> {
        if self.pending.is_empty() {
            return Ok(());
        }

        self.stats.flushes.fetch_add(1, Ordering::Relaxed);

        // Build IoSlice array
        let slices: Vec<IoSlice> = self.pending.iter().map(|b| IoSlice::new(b)).collect();

        // Write all buffers in one syscall
        let bytes_written = self.file.write_vectored(&slices)?;
        self.stats
            .bytes_written
            .fetch_add(bytes_written as u64, Ordering::Relaxed);

        // Clear pending
        self.pending.clear();
        self.pending_bytes = 0;
        self.stats.buffered.store(0, Ordering::Relaxed);

        Ok(())
    }

    /// Get number of pending buffers
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    /// Get pending bytes
    pub fn pending_bytes(&self) -> usize {
        self.pending_bytes
    }

    /// Get statistics
    pub fn stats(&self) -> &WriterStats {
        &self.stats
    }

    /// Sync to disk
    pub fn sync(&mut self) -> io::Result<()> {
        self.flush()?;
        self.file.sync_all()
    }
}

impl Write for VectoredWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let len = buf.len();
        self.queue_slice(buf)?;
        Ok(len)
    }

    fn flush(&mut self) -> io::Result<()> {
        VectoredWriter::flush(self)
    }

    fn write_vectored(&mut self, bufs: &[IoSlice<'_>]) -> io::Result<usize> {
        let mut total = 0;
        for buf in bufs {
            self.queue_slice(buf)?;
            total += buf.len();
        }
        Ok(total)
    }
}

impl Drop for VectoredWriter {
    fn drop(&mut self) {
        let _ = self.flush();
    }
}

// ============================================================================
// Async Writer (optional, requires tokio feature)
// ============================================================================

#[cfg(feature = "async")]
pub mod async_writer {
    use super::*;
    use std::pin::Pin;
    use std::task::{Context, Poll};
    use tokio::fs::File as AsyncFile;
    use tokio::io::{AsyncWrite, AsyncWriteExt, BufWriter as AsyncBufWriter};

    /// Async buffered writer
    pub struct AsyncWriter {
        inner: AsyncBufWriter<AsyncFile>,
        capacity: usize,
        stats: Arc<WriterStats>,
        path: String,
    }

    impl AsyncWriter {
        /// Create a new async writer
        pub async fn new<P: AsRef<Path>>(path: P) -> io::Result<Self> {
            Self::with_capacity(path, DEFAULT_BUFFER_SIZE).await
        }

        /// Create with specific capacity
        pub async fn with_capacity<P: AsRef<Path>>(path: P, capacity: usize) -> io::Result<Self> {
            let path_str = path.as_ref().to_string_lossy().to_string();
            let file = AsyncFile::create(path.as_ref()).await?;
            let inner = AsyncBufWriter::with_capacity(capacity, file);

            Ok(Self {
                inner,
                capacity,
                stats: Arc::new(WriterStats::default()),
                path: path_str,
            })
        }

        /// Write bytes asynchronously
        pub async fn write_bytes(&mut self, data: &[u8]) -> io::Result<usize> {
            self.stats.writes.fetch_add(1, Ordering::Relaxed);
            self.stats
                .bytes_written
                .fetch_add(data.len() as u64, Ordering::Relaxed);

            self.inner.write_all(data).await?;
            Ok(data.len())
        }

        /// Write string
        pub async fn write_str(&mut self, s: &str) -> io::Result<usize> {
            self.write_bytes(s.as_bytes()).await
        }

        /// Flush buffer
        pub async fn flush(&mut self) -> io::Result<()> {
            self.stats.flushes.fetch_add(1, Ordering::Relaxed);
            self.inner.flush().await
        }

        /// Sync to disk
        pub async fn sync(&mut self) -> io::Result<()> {
            self.flush().await?;
            self.inner.get_mut().sync_all().await
        }

        /// Get statistics
        pub fn stats(&self) -> &WriterStats {
            &self.stats
        }
    }

    impl AsyncWrite for AsyncWriter {
        fn poll_write(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            buf: &[u8],
        ) -> Poll<io::Result<usize>> {
            self.stats.writes.fetch_add(1, Ordering::Relaxed);
            self.stats
                .bytes_written
                .fetch_add(buf.len() as u64, Ordering::Relaxed);
            Pin::new(&mut self.inner).poll_write(cx, buf)
        }

        fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
            self.stats.flushes.fetch_add(1, Ordering::Relaxed);
            Pin::new(&mut self.inner).poll_flush(cx)
        }

        fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
            Pin::new(&mut self.inner).poll_shutdown(cx)
        }
    }
}

// ============================================================================
// Handle Store
// ============================================================================

pub static BUFFERED_WRITERS: LazyLock<HandleStore<BufferedWriter>> =
    LazyLock::new(HandleStore::new);
pub static VECTORED_WRITERS: LazyLock<HandleStore<VectoredWriter>> =
    LazyLock::new(HandleStore::new);

// ============================================================================
// FFI Functions
// ============================================================================

use std::ffi::{CStr, c_char, c_int};

/// FFI writer statistics
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct FfiWriterStats {
    pub writes: u64,
    pub bytes_written: u64,
    pub flushes: u64,
    pub buffered: u64,
}

impl From<&WriterStats> for FfiWriterStats {
    fn from(stats: &WriterStats) -> Self {
        Self {
            writes: stats.writes.load(Ordering::Relaxed),
            bytes_written: stats.bytes_written.load(Ordering::Relaxed),
            flushes: stats.flushes.load(Ordering::Relaxed),
            buffered: stats.buffered.load(Ordering::Relaxed),
        }
    }
}

// --- Buffered Writer FFI ---

/// Create a new buffered writer
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_buffered_writer(_ctx: Handle, path: *const c_char) -> Handle {
    if path.is_null() {
        return 0;
    }

    let path_str = unsafe { CStr::from_ptr(path).to_str().unwrap_or("") };

    match BufferedWriter::new(path_str) {
        Ok(writer) => BUFFERED_WRITERS.insert(writer),
        Err(_) => 0,
    }
}

/// Create buffered writer with capacity
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_buffered_writer_with_capacity(
    _ctx: Handle,
    path: *const c_char,
    capacity: usize,
) -> Handle {
    if path.is_null() {
        return 0;
    }

    let path_str = unsafe { CStr::from_ptr(path).to_str().unwrap_or("") };

    match BufferedWriter::with_capacity(path_str, capacity) {
        Ok(writer) => BUFFERED_WRITERS.insert(writer),
        Err(_) => 0,
    }
}

/// Drop buffered writer
#[unsafe(no_mangle)]
pub extern "C" fn fz_drop_buffered_writer(_ctx: Handle, writer: Handle) {
    let _ = BUFFERED_WRITERS.remove(writer);
}

/// Write bytes to buffered writer
#[unsafe(no_mangle)]
pub extern "C" fn fz_buffered_write(
    _ctx: Handle,
    writer: Handle,
    data: *const u8,
    len: usize,
) -> c_int {
    if data.is_null() {
        return -1;
    }

    let Some(w) = BUFFERED_WRITERS.get(writer) else {
        return -1;
    };

    let data_slice = unsafe { std::slice::from_raw_parts(data, len) };
    let mut guard = w.lock().unwrap();

    match guard.write_bytes(data_slice) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

/// Write string to buffered writer
#[unsafe(no_mangle)]
pub extern "C" fn fz_buffered_write_string(
    _ctx: Handle,
    writer: Handle,
    s: *const c_char,
) -> c_int {
    if s.is_null() {
        return -1;
    }

    let Some(w) = BUFFERED_WRITERS.get(writer) else {
        return -1;
    };

    let str_slice = unsafe { CStr::from_ptr(s).to_str().unwrap_or("") };
    let mut guard = w.lock().unwrap();

    match guard.write_str(str_slice) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

/// Flush buffered writer
#[unsafe(no_mangle)]
pub extern "C" fn fz_buffered_flush(_ctx: Handle, writer: Handle) -> c_int {
    let Some(w) = BUFFERED_WRITERS.get(writer) else {
        return -1;
    };

    let mut guard = w.lock().unwrap();
    match guard.flush() {
        Ok(()) => 0,
        Err(_) => -1,
    }
}

/// Sync buffered writer to disk
#[unsafe(no_mangle)]
pub extern "C" fn fz_buffered_sync(_ctx: Handle, writer: Handle) -> c_int {
    let Some(w) = BUFFERED_WRITERS.get(writer) else {
        return -1;
    };

    let mut guard = w.lock().unwrap();
    match guard.sync() {
        Ok(()) => 0,
        Err(_) => -1,
    }
}

/// Get buffered writer stats
#[unsafe(no_mangle)]
pub extern "C" fn fz_buffered_writer_stats(_ctx: Handle, writer: Handle) -> FfiWriterStats {
    BUFFERED_WRITERS
        .get(writer)
        .map(|w| FfiWriterStats::from(w.lock().unwrap().stats()))
        .unwrap_or_default()
}

/// Get current buffer usage
#[unsafe(no_mangle)]
pub extern "C" fn fz_buffered_writer_buffered(_ctx: Handle, writer: Handle) -> usize {
    BUFFERED_WRITERS
        .get(writer)
        .map(|w| w.lock().unwrap().buffered())
        .unwrap_or(0)
}

// --- Vectored Writer FFI ---

/// Create a new vectored writer
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_vectored_writer(_ctx: Handle, path: *const c_char) -> Handle {
    if path.is_null() {
        return 0;
    }

    let path_str = unsafe { CStr::from_ptr(path).to_str().unwrap_or("") };

    match VectoredWriter::new(path_str) {
        Ok(writer) => VECTORED_WRITERS.insert(writer),
        Err(_) => 0,
    }
}

/// Drop vectored writer
#[unsafe(no_mangle)]
pub extern "C" fn fz_drop_vectored_writer(_ctx: Handle, writer: Handle) {
    let _ = VECTORED_WRITERS.remove(writer);
}

/// Queue data for vectored write
#[unsafe(no_mangle)]
pub extern "C" fn fz_vectored_queue(
    _ctx: Handle,
    writer: Handle,
    data: *const u8,
    len: usize,
) -> c_int {
    if data.is_null() {
        return -1;
    }

    let Some(w) = VECTORED_WRITERS.get(writer) else {
        return -1;
    };

    let data_slice = unsafe { std::slice::from_raw_parts(data, len) };
    let mut guard = w.lock().unwrap();

    match guard.queue_slice(data_slice) {
        Ok(()) => 0,
        Err(_) => -1,
    }
}

/// Flush vectored writer (uses writev)
#[unsafe(no_mangle)]
pub extern "C" fn fz_vectored_flush(_ctx: Handle, writer: Handle) -> c_int {
    let Some(w) = VECTORED_WRITERS.get(writer) else {
        return -1;
    };

    let mut guard = w.lock().unwrap();
    match guard.flush() {
        Ok(()) => 0,
        Err(_) => -1,
    }
}

/// Sync vectored writer
#[unsafe(no_mangle)]
pub extern "C" fn fz_vectored_sync(_ctx: Handle, writer: Handle) -> c_int {
    let Some(w) = VECTORED_WRITERS.get(writer) else {
        return -1;
    };

    let mut guard = w.lock().unwrap();
    match guard.sync() {
        Ok(()) => 0,
        Err(_) => -1,
    }
}

/// Get vectored writer stats
#[unsafe(no_mangle)]
pub extern "C" fn fz_vectored_writer_stats(_ctx: Handle, writer: Handle) -> FfiWriterStats {
    VECTORED_WRITERS
        .get(writer)
        .map(|w| FfiWriterStats::from(w.lock().unwrap().stats()))
        .unwrap_or_default()
}

/// Get pending buffer count
#[unsafe(no_mangle)]
pub extern "C" fn fz_vectored_writer_pending_count(_ctx: Handle, writer: Handle) -> usize {
    VECTORED_WRITERS
        .get(writer)
        .map(|w| w.lock().unwrap().pending_count())
        .unwrap_or(0)
}

/// Get pending bytes
#[unsafe(no_mangle)]
pub extern "C" fn fz_vectored_writer_pending_bytes(_ctx: Handle, writer: Handle) -> usize {
    VECTORED_WRITERS
        .get(writer)
        .map(|w| w.lock().unwrap().pending_bytes())
        .unwrap_or(0)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;
    use tempfile::NamedTempFile;

    #[test]
    fn test_buffered_writer_basic() {
        let temp = NamedTempFile::new().unwrap();
        let path = temp.path();

        {
            let mut writer = BufferedWriter::new(path).unwrap();
            writer.write_str("Hello, ").unwrap();
            writer.write_str("World!").unwrap();
            writer.flush().unwrap();
        }

        let content = std::fs::read_to_string(path).unwrap();
        assert_eq!(content, "Hello, World!");
    }

    #[test]
    fn test_buffered_writer_line() {
        let temp = NamedTempFile::new().unwrap();
        let path = temp.path();

        {
            let mut writer = BufferedWriter::new(path).unwrap();
            writer.write_line("Line 1").unwrap();
            writer.write_line("Line 2").unwrap();
            writer.flush().unwrap();
        }

        let content = std::fs::read_to_string(path).unwrap();
        assert_eq!(content, "Line 1\nLine 2\n");
    }

    #[test]
    fn test_buffered_writer_stats() {
        let temp = NamedTempFile::new().unwrap();
        let path = temp.path();

        let mut writer = BufferedWriter::new(path).unwrap();
        writer.write_bytes(b"test").unwrap();
        writer.write_bytes(b"data").unwrap();
        writer.flush().unwrap();

        let stats = writer.stats();
        assert_eq!(stats.writes.load(Ordering::Relaxed), 2);
        assert_eq!(stats.bytes_written.load(Ordering::Relaxed), 8);
        assert_eq!(stats.flushes.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn test_buffered_writer_capacity() {
        let temp = NamedTempFile::new().unwrap();
        let path = temp.path();

        let writer = BufferedWriter::with_capacity(path, 1024).unwrap();
        assert_eq!(writer.capacity(), 1024);
    }

    #[test]
    #[cfg(unix)] // Vectored I/O works differently on Windows
    fn test_vectored_writer_basic() {
        let temp = NamedTempFile::new().unwrap();
        let path = temp.path();

        {
            let mut writer = VectoredWriter::new(path).unwrap();
            writer.queue(b"Hello".to_vec()).unwrap();
            writer.queue(b" ".to_vec()).unwrap();
            writer.queue(b"World".to_vec()).unwrap();
            assert_eq!(writer.pending_count(), 3);
            writer.flush().unwrap();
            assert_eq!(writer.pending_count(), 0);
        }

        let content = std::fs::read_to_string(path).unwrap();
        assert_eq!(content, "Hello World");
    }

    #[test]
    #[cfg(unix)] // Vectored I/O works differently on Windows
    fn test_vectored_writer_auto_flush() {
        let temp = NamedTempFile::new().unwrap();
        let path = temp.path();

        {
            // Small max_pending to trigger auto-flush
            let mut writer = VectoredWriter::with_max_pending(path, 10).unwrap();
            writer.queue(b"12345".to_vec()).unwrap();
            writer.queue(b"67890".to_vec()).unwrap();
            // Should have auto-flushed after second write (>= 10 bytes)
            assert_eq!(writer.pending_count(), 0);
        }

        let content = std::fs::read_to_string(path).unwrap();
        assert_eq!(content, "1234567890");
    }

    #[test]
    #[cfg(unix)] // Vectored I/O works differently on Windows
    fn test_vectored_writer_stats() {
        let temp = NamedTempFile::new().unwrap();
        let path = temp.path();

        let mut writer = VectoredWriter::new(path).unwrap();
        writer.queue(b"abc".to_vec()).unwrap();
        writer.queue(b"def".to_vec()).unwrap();
        writer.flush().unwrap();

        let stats = writer.stats();
        assert_eq!(stats.writes.load(Ordering::Relaxed), 2);
        assert_eq!(stats.bytes_written.load(Ordering::Relaxed), 6);
        assert_eq!(stats.flushes.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn test_ffi_buffered_writer() {
        let temp = NamedTempFile::new().unwrap();
        let path = std::ffi::CString::new(temp.path().to_str().unwrap()).unwrap();

        let handle = fz_new_buffered_writer(0, path.as_ptr());
        assert_ne!(handle, 0);

        let data = b"FFI test";
        let result = fz_buffered_write(0, handle, data.as_ptr(), data.len());
        assert_eq!(result, 0);

        let result = fz_buffered_flush(0, handle);
        assert_eq!(result, 0);

        fz_drop_buffered_writer(0, handle);

        let content = std::fs::read_to_string(temp.path()).unwrap();
        assert_eq!(content, "FFI test");
    }

    #[test]
    #[cfg(unix)] // Vectored I/O works differently on Windows
    fn test_ffi_vectored_writer() {
        let temp = NamedTempFile::new().unwrap();
        let path = std::ffi::CString::new(temp.path().to_str().unwrap()).unwrap();

        let handle = fz_new_vectored_writer(0, path.as_ptr());
        assert_ne!(handle, 0);

        let data1 = b"Hello";
        let data2 = b" World";
        fz_vectored_queue(0, handle, data1.as_ptr(), data1.len());
        fz_vectored_queue(0, handle, data2.as_ptr(), data2.len());

        assert_eq!(fz_vectored_writer_pending_count(0, handle), 2);

        fz_vectored_flush(0, handle);
        assert_eq!(fz_vectored_writer_pending_count(0, handle), 0);

        fz_drop_vectored_writer(0, handle);

        let content = std::fs::read_to_string(temp.path()).unwrap();
        assert_eq!(content, "Hello World");
    }

    #[test]
    fn test_ffi_writer_stats() {
        let temp = NamedTempFile::new().unwrap();
        let path = std::ffi::CString::new(temp.path().to_str().unwrap()).unwrap();

        let handle = fz_new_buffered_writer(0, path.as_ptr());

        let data = b"stats";
        fz_buffered_write(0, handle, data.as_ptr(), data.len());
        fz_buffered_flush(0, handle);

        let stats = fz_buffered_writer_stats(0, handle);
        assert_eq!(stats.writes, 1);
        assert_eq!(stats.bytes_written, 5);
        assert_eq!(stats.flushes, 1);

        fz_drop_buffered_writer(0, handle);
    }
}
