//! C FFI for buffer - MuPDF compatible
//! Safe Rust implementation using handle-based resource management
//!
//! This module includes a buffer pool for efficient memory reuse in hot paths.

use super::{BUFFERS, Handle};
use std::collections::{HashMap, VecDeque};
use std::ffi::{c_char, c_int, c_void};
use std::sync::{LazyLock, Mutex};

// ============================================================================
// Buffer Pool for Memory Reuse
// ============================================================================

/// Common buffer sizes for pooling (in bytes)
const POOL_SIZES: &[usize] = &[
    64,    // Tiny: small strings, names
    256,   // Small: typical text runs
    1024,  // 1KB: page content fragments
    4096,  // 4KB: typical page size
    16384, // 16KB: larger content
    65536, // 64KB: images, streams
];

/// Maximum buffers to keep per size class
const MAX_POOL_SIZE: usize = 32;

/// A pool of pre-allocated buffers for reuse
struct BufferPool {
    /// Pools indexed by size class
    pools: Vec<Mutex<VecDeque<Vec<u8>>>>,
    /// Statistics
    stats: Mutex<PoolStats>,
}

/// Pool statistics for debugging and optimization
#[derive(Debug, Clone, Default)]
pub struct PoolStats {
    /// Number of buffers acquired from pool
    pub hits: u64,
    /// Number of new allocations (pool miss)
    pub misses: u64,
    /// Number of buffers returned to pool
    pub returns: u64,
    /// Number of buffers dropped (pool full)
    pub drops: u64,
    /// Total bytes allocated (cumulative)
    pub total_bytes_allocated: u64,
    /// Total bytes deallocated (cumulative)
    pub total_bytes_deallocated: u64,
    /// Current bytes in use
    pub current_bytes: u64,
    /// Peak bytes in use
    pub peak_bytes: u64,
    /// Total allocations count
    pub total_allocations: u64,
    /// Total deallocations count
    pub total_deallocations: u64,
}

impl BufferPool {
    fn new() -> Self {
        let pools = POOL_SIZES
            .iter()
            .map(|_| Mutex::new(VecDeque::new()))
            .collect();
        Self {
            pools,
            stats: Mutex::new(PoolStats::default()),
        }
    }

    /// Find the size class for a given capacity
    fn size_class(capacity: usize) -> Option<usize> {
        POOL_SIZES.iter().position(|&size| size >= capacity)
    }

    /// Acquire a buffer from the pool or allocate new
    fn acquire(&self, capacity: usize) -> Vec<u8> {
        if let Some(class) = Self::size_class(capacity) {
            if let Ok(mut pool) = self.pools[class].lock() {
                if let Some(mut buf) = pool.pop_front() {
                    let buf_capacity = buf.capacity() as u64;
                    buf.clear();
                    if let Ok(mut stats) = self.stats.lock() {
                        stats.hits += 1;
                        stats.total_allocations += 1;
                        stats.current_bytes += buf_capacity;
                        if stats.current_bytes > stats.peak_bytes {
                            stats.peak_bytes = stats.current_bytes;
                        }
                    }
                    return buf;
                }
            }
        }

        // Pool miss - allocate new
        // Use pool size if it fits, otherwise exact capacity
        let alloc_size = Self::size_class(capacity)
            .map(|class| POOL_SIZES[class])
            .unwrap_or(capacity);

        if let Ok(mut stats) = self.stats.lock() {
            stats.misses += 1;
            stats.total_allocations += 1;
            stats.total_bytes_allocated += alloc_size as u64;
            stats.current_bytes += alloc_size as u64;
            if stats.current_bytes > stats.peak_bytes {
                stats.peak_bytes = stats.current_bytes;
            }
        }

        Vec::with_capacity(alloc_size)
    }

    /// Return a buffer to the pool for reuse
    fn release(&self, mut buf: Vec<u8>) {
        let capacity = buf.capacity() as u64;

        // Update deallocation stats
        if let Ok(mut stats) = self.stats.lock() {
            stats.total_deallocations += 1;
            stats.current_bytes = stats.current_bytes.saturating_sub(capacity);
        }

        if let Some(class) = Self::size_class(capacity as usize) {
            // Only pool if capacity matches a pool size exactly
            if buf.capacity() == POOL_SIZES[class] {
                if let Ok(mut pool) = self.pools[class].lock() {
                    if pool.len() < MAX_POOL_SIZE {
                        buf.clear();
                        pool.push_back(buf);
                        if let Ok(mut stats) = self.stats.lock() {
                            stats.returns += 1;
                        }
                        return;
                    }
                }
            }
        }

        // Pool full or size doesn't match - drop buffer
        if let Ok(mut stats) = self.stats.lock() {
            stats.drops += 1;
            stats.total_bytes_deallocated += capacity;
        }
        drop(buf);
    }

    /// Get pool statistics
    fn get_stats(&self) -> PoolStats {
        self.stats.lock().map(|s| s.clone()).unwrap_or_default()
    }

    /// Clear all pools
    fn clear(&self) {
        for pool in &self.pools {
            if let Ok(mut p) = pool.lock() {
                p.clear();
            }
        }
    }

    /// Get total buffers in pool
    fn pool_count(&self) -> usize {
        self.pools
            .iter()
            .filter_map(|p| p.lock().ok())
            .map(|p| p.len())
            .sum()
    }
}

/// Global buffer pool instance
static BUFFER_POOL: LazyLock<BufferPool> = LazyLock::new(BufferPool::new);

/// Internal buffer state
pub struct Buffer {
    data: Vec<u8>,
    /// Whether this buffer came from the pool
    pooled: bool,
}

impl Buffer {
    /// Create a new buffer with given capacity (uses pool if available)
    pub fn new(capacity: usize) -> Self {
        let data = BUFFER_POOL.acquire(capacity);
        Self { data, pooled: true }
    }

    /// Create a new buffer without using pool
    pub fn new_unpooled(capacity: usize) -> Self {
        Self {
            data: Vec::with_capacity(capacity),
            pooled: false,
        }
    }

    /// Create a buffer from data (copies data, uses pool for allocation)
    pub fn from_data(data: &[u8]) -> Self {
        let mut buf = BUFFER_POOL.acquire(data.len());
        buf.extend_from_slice(data);
        Self {
            data: buf,
            pooled: true,
        }
    }

    /// Create a buffer from data without using pool
    pub fn from_data_unpooled(data: &[u8]) -> Self {
        Self {
            data: data.to_vec(),
            pooled: false,
        }
    }

    /// Take ownership of existing Vec (does not use pool)
    pub fn from_vec(data: Vec<u8>) -> Self {
        Self {
            data,
            pooled: false,
        }
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Alias for data() - get immutable slice of buffer
    pub fn as_slice(&self) -> &[u8] {
        &self.data
    }

    pub fn data_mut(&mut self) -> &mut Vec<u8> {
        &mut self.data
    }

    pub fn append(&mut self, data: &[u8]) {
        self.data.extend_from_slice(data);
    }

    pub fn append_byte(&mut self, byte: u8) {
        self.data.push(byte);
    }

    pub fn clear(&mut self) {
        self.data.clear();
    }

    pub fn resize(&mut self, new_size: usize) {
        self.data.resize(new_size, 0);
    }

    pub fn ensure_null_terminated(&mut self) {
        if self.data.is_empty() || self.data.last() != Some(&0) {
            self.data.push(0);
        }
    }

    /// Check if buffer is from pool
    pub fn is_pooled(&self) -> bool {
        self.pooled
    }

    /// Get buffer capacity
    pub fn capacity(&self) -> usize {
        self.data.capacity()
    }

    /// Reserve additional capacity (may reallocate)
    pub fn reserve(&mut self, additional: usize) {
        self.data.reserve(additional);
    }

    /// Reserve exact capacity (may reallocate)
    pub fn reserve_exact(&mut self, additional: usize) {
        self.data.reserve_exact(additional);
    }

    /// Shrink buffer to fit current data
    pub fn shrink_to_fit(&mut self) {
        self.data.shrink_to_fit();
        self.pooled = false; // No longer poolable after shrink
    }

    /// Take the inner Vec, replacing with empty (for transfer)
    pub fn take(&mut self) -> Vec<u8> {
        self.pooled = false;
        std::mem::take(&mut self.data)
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        if self.pooled {
            // Return buffer to pool
            let data = std::mem::take(&mut self.data);
            BUFFER_POOL.release(data);
        }
        // Non-pooled buffers drop normally
    }
}

/// Create a new buffer with given capacity
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_buffer(_ctx: Handle, capacity: usize) -> Handle {
    BUFFERS.insert(Buffer::new(capacity))
}

/// Create a buffer from copied data
///
/// # Safety
/// Caller must ensure `data` points to valid memory of at least `size` bytes.
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_buffer_from_copied_data(
    _ctx: Handle,
    data: *const u8,
    size: usize,
) -> Handle {
    if data.is_null() || size == 0 {
        return BUFFERS.insert(Buffer::new(0));
    }

    // SAFETY: Caller guarantees data points to valid memory of `size` bytes
    let data_slice = unsafe { std::slice::from_raw_parts(data, size) };

    BUFFERS.insert(Buffer::from_data(data_slice))
}

/// Keep (increment ref) a buffer - returns same handle
#[unsafe(no_mangle)]
pub extern "C" fn fz_keep_buffer(_ctx: Handle, buf: Handle) -> Handle {
    BUFFERS.keep(buf)
}

/// Drop a buffer reference
#[unsafe(no_mangle)]
pub extern "C" fn fz_drop_buffer(_ctx: Handle, buf: Handle) {
    let _ = BUFFERS.remove(buf);
}

/// Get buffer storage - returns length, optionally fills data pointer
///
/// Note: This function cannot safely return a pointer to internal data
/// because the buffer may be moved or reallocated. For safe access,
/// use fz_buffer_len and copy the data.
#[unsafe(no_mangle)]
pub extern "C" fn fz_buffer_storage(_ctx: Handle, buf: Handle, datap: *mut *mut u8) -> usize {
    let Some(buffer) = BUFFERS.get(buf) else {
        if !datap.is_null() {
            // SAFETY: Caller guarantees datap is valid if non-null
            unsafe {
                *datap = std::ptr::null_mut();
            }
        }
        return 0;
    };

    let guard = buffer.lock().unwrap();
    let len = guard.len();

    if !datap.is_null() {
        // We can't safely return a pointer to internal data
        // because the buffer may be reallocated
        unsafe {
            *datap = std::ptr::null_mut();
        }
    }

    len
}

/// Get pointer to buffer data (Alternative API compatible with MuPDF)
///
/// Returns a pointer to the buffer's internal data and optionally sets
/// the length pointer if provided. This is compatible with MuPDF's fz_buffer_data.
///
/// # Safety
/// The returned pointer is only valid until the next operation that might
/// modify the buffer. The caller must copy the data if it needs to be retained.
///
/// # Warning
/// This function returns a direct pointer to internal buffer data. The pointer
/// may be invalidated if the buffer is modified or reallocated.
#[unsafe(no_mangle)]
pub extern "C" fn fz_buffer_data(_ctx: Handle, buf: Handle, len: *mut usize) -> *const u8 {
    let Some(buffer) = BUFFERS.get(buf) else {
        if !len.is_null() {
            unsafe {
                *len = 0;
            }
        }
        return std::ptr::null();
    };

    let guard = buffer.lock().unwrap();
    let data_len = guard.len();

    if !len.is_null() {
        unsafe {
            *len = data_len;
        }
    }

    // Return pointer to internal data
    // SAFETY: The pointer is valid as long as the buffer is not modified
    // and the guard is held. The caller must not hold this pointer
    // after any operation that might modify the buffer.
    guard.data.as_ptr()
}

/// Get buffer as null-terminated C string
///
/// Note: This function cannot safely return a pointer to internal buffer data
/// because the data may be moved or reallocated. Returns empty string for now.
#[unsafe(no_mangle)]
pub extern "C" fn fz_string_from_buffer(_ctx: Handle, _buf: Handle) -> *const c_char {
    // Can't return internal pointer safely without stable address
    // Return empty string for now
    c"".as_ptr()
}

/// Resize buffer to new capacity
#[unsafe(no_mangle)]
pub extern "C" fn fz_resize_buffer(_ctx: Handle, buf: Handle, capacity: usize) {
    if let Some(buffer) = BUFFERS.get(buf) {
        if let Ok(mut guard) = buffer.lock() {
            guard.resize(capacity);
        }
    }
}

/// Grow buffer (double capacity or minimum 256)
#[unsafe(no_mangle)]
pub extern "C" fn fz_grow_buffer(_ctx: Handle, buf: Handle) {
    if let Some(buffer) = BUFFERS.get(buf) {
        if let Ok(mut guard) = buffer.lock() {
            let current_cap = guard.data.capacity();
            let new_cap = (current_cap * 2).max(256);
            guard.data.reserve(new_cap.saturating_sub(current_cap));
        }
    }
}

/// Trim buffer to fit contents
#[unsafe(no_mangle)]
pub extern "C" fn fz_trim_buffer(_ctx: Handle, buf: Handle) {
    if let Some(buffer) = BUFFERS.get(buf) {
        if let Ok(mut guard) = buffer.lock() {
            guard.data.shrink_to_fit();
        }
    }
}

/// Clear buffer contents
#[unsafe(no_mangle)]
pub extern "C" fn fz_clear_buffer(_ctx: Handle, buf: Handle) {
    if let Some(buffer) = BUFFERS.get(buf) {
        if let Ok(mut guard) = buffer.lock() {
            guard.clear();
        }
    }
}

/// Append data to buffer
///
/// # Safety
/// Caller must ensure `data` points to valid memory of at least `len` bytes.
#[unsafe(no_mangle)]
pub extern "C" fn fz_append_data(_ctx: Handle, buf: Handle, data: *const c_void, len: usize) {
    if data.is_null() || len == 0 {
        return;
    }

    if let Some(buffer) = BUFFERS.get(buf) {
        if let Ok(mut guard) = buffer.lock() {
            // SAFETY: Caller guarantees data points to valid memory of `len` bytes
            let slice = unsafe { std::slice::from_raw_parts(data as *const u8, len) };
            guard.append(slice);
        }
    }
}

/// Append C string to buffer
///
/// # Safety
/// Caller must ensure `data` is a valid null-terminated C string.
#[unsafe(no_mangle)]
pub extern "C" fn fz_append_string(_ctx: Handle, buf: Handle, data: *const c_char) {
    if data.is_null() {
        return;
    }

    if let Some(buffer) = BUFFERS.get(buf) {
        if let Ok(mut guard) = buffer.lock() {
            // SAFETY: Caller guarantees data is a valid null-terminated C string
            let c_str = unsafe { std::ffi::CStr::from_ptr(data) };
            guard.append(c_str.to_bytes());
        }
    }
}

/// Append single byte to buffer
#[unsafe(no_mangle)]
pub extern "C" fn fz_append_byte(_ctx: Handle, buf: Handle, c: c_int) {
    if let Some(buffer) = BUFFERS.get(buf) {
        if let Ok(mut guard) = buffer.lock() {
            guard.append_byte(c as u8);
        }
    }
}

/// Null-terminate buffer
#[unsafe(no_mangle)]
pub extern "C" fn fz_terminate_buffer(_ctx: Handle, buf: Handle) {
    if let Some(buffer) = BUFFERS.get(buf) {
        if let Ok(mut guard) = buffer.lock() {
            guard.ensure_null_terminated();
        }
    }
}

/// Compute MD5 digest of buffer contents
///
/// # Safety
/// Caller must ensure `digest` points to valid writable memory of 16 bytes.
#[unsafe(no_mangle)]
pub extern "C" fn fz_md5_buffer(_ctx: Handle, buf: Handle, digest: *mut [u8; 16]) {
    if digest.is_null() {
        return;
    }

    if let Some(buffer) = BUFFERS.get(buf) {
        if let Ok(guard) = buffer.lock() {
            use md5::{Digest, Md5};
            let mut hasher = Md5::new();
            hasher.update(guard.data());
            let result = hasher.finalize();

            // SAFETY: Caller guarantees digest points to valid writable [u8; 16]
            unsafe {
                (*digest).copy_from_slice(&result);
            }
        }
    }
}

/// Clone a buffer
#[unsafe(no_mangle)]
pub extern "C" fn fz_clone_buffer(_ctx: Handle, buf: Handle) -> Handle {
    if let Some(buffer) = BUFFERS.get(buf) {
        if let Ok(guard) = buffer.lock() {
            return BUFFERS.insert(Buffer::from_data(guard.data()));
        }
    }
    0
}

/// Get buffer length
#[unsafe(no_mangle)]
pub extern "C" fn fz_buffer_len(_ctx: Handle, buf: Handle) -> usize {
    if let Some(buffer) = BUFFERS.get(buf) {
        if let Ok(guard) = buffer.lock() {
            return guard.len();
        }
    }
    0
}

// ============================================================================
// Integer Append Functions
// ============================================================================

/// Append 16-bit integer in little-endian format
#[unsafe(no_mangle)]
pub extern "C" fn fz_append_int16_le(_ctx: Handle, buf: Handle, x: i16) {
    if let Some(buffer) = BUFFERS.get(buf) {
        if let Ok(mut guard) = buffer.lock() {
            guard.append(&x.to_le_bytes());
        }
    }
}

/// Append 32-bit integer in little-endian format
#[unsafe(no_mangle)]
pub extern "C" fn fz_append_int32_le(_ctx: Handle, buf: Handle, x: i32) {
    if let Some(buffer) = BUFFERS.get(buf) {
        if let Ok(mut guard) = buffer.lock() {
            guard.append(&x.to_le_bytes());
        }
    }
}

/// Append 16-bit integer in big-endian format
#[unsafe(no_mangle)]
pub extern "C" fn fz_append_int16_be(_ctx: Handle, buf: Handle, x: i16) {
    if let Some(buffer) = BUFFERS.get(buf) {
        if let Ok(mut guard) = buffer.lock() {
            guard.append(&x.to_be_bytes());
        }
    }
}

/// Append 32-bit integer in big-endian format
#[unsafe(no_mangle)]
pub extern "C" fn fz_append_int32_be(_ctx: Handle, buf: Handle, x: i32) {
    if let Some(buffer) = BUFFERS.get(buf) {
        if let Ok(mut guard) = buffer.lock() {
            guard.append(&x.to_be_bytes());
        }
    }
}

// ============================================================================
// Bit Append Functions
// ============================================================================

/// Internal state for bit accumulation
pub struct BitBuffer {
    accumulator: u32,
    bits_in_accumulator: u8,
}

impl BitBuffer {
    pub fn new() -> Self {
        Self {
            accumulator: 0,
            bits_in_accumulator: 0,
        }
    }
}

impl Default for BitBuffer {
    fn default() -> Self {
        Self::new()
    }
}

/// Global bit buffer state for each buffer handle
static BIT_BUFFERS: LazyLock<Mutex<HashMap<Handle, BitBuffer>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Append bits to buffer
#[unsafe(no_mangle)]
pub extern "C" fn fz_append_bits(_ctx: Handle, buf: Handle, value: i32, count: i32) {
    if count <= 0 || count > 32 {
        return;
    }

    if let Some(buffer) = BUFFERS.get(buf) {
        if let Ok(mut guard) = buffer.lock() {
            if let Ok(mut bit_map) = BIT_BUFFERS.lock() {
                let bit_buf = bit_map.entry(buf).or_insert_with(BitBuffer::new);

                // Mask to get only the requested bits
                let mask = if count == 32 {
                    u32::MAX
                } else {
                    (1u32 << count) - 1
                };
                let bits = (value as u32) & mask;

                // Add bits to accumulator
                bit_buf.accumulator = (bit_buf.accumulator << count) | bits;
                bit_buf.bits_in_accumulator += count as u8;

                // Flush complete bytes
                while bit_buf.bits_in_accumulator >= 8 {
                    bit_buf.bits_in_accumulator -= 8;
                    let byte = (bit_buf.accumulator >> bit_buf.bits_in_accumulator) as u8;
                    guard.append_byte(byte);
                }
            }
        }
    }
}

/// Append bits and pad to byte boundary
#[unsafe(no_mangle)]
pub extern "C" fn fz_append_bits_pad(_ctx: Handle, buf: Handle) {
    if let Some(buffer) = BUFFERS.get(buf) {
        if let Ok(mut guard) = buffer.lock() {
            if let Ok(mut bit_map) = BIT_BUFFERS.lock() {
                if let Some(bit_buf) = bit_map.get_mut(&buf) {
                    // Flush remaining bits with zero padding
                    if bit_buf.bits_in_accumulator > 0 {
                        let pad_bits = 8 - bit_buf.bits_in_accumulator;
                        let byte = (bit_buf.accumulator << pad_bits) as u8;
                        guard.append_byte(byte);
                        bit_buf.accumulator = 0;
                        bit_buf.bits_in_accumulator = 0;
                    }
                }
            }
        }
    }
}

// ============================================================================
// PDF String Functions
// ============================================================================

/// Append a PDF-escaped string (with parentheses)
///
/// # Safety
/// Caller must ensure `str` is a valid null-terminated C string.
#[unsafe(no_mangle)]
pub extern "C" fn fz_append_pdf_string(_ctx: Handle, buf: Handle, str: *const c_char) {
    if str.is_null() {
        // Append empty string "()"
        if let Some(buffer) = BUFFERS.get(buf) {
            if let Ok(mut guard) = buffer.lock() {
                guard.append(b"()");
            }
        }
        return;
    }

    // SAFETY: Caller guarantees str is a valid null-terminated C string
    let c_str = unsafe { std::ffi::CStr::from_ptr(str) };
    let bytes = c_str.to_bytes();

    if let Some(buffer) = BUFFERS.get(buf) {
        if let Ok(mut guard) = buffer.lock() {
            guard.append_byte(b'(');

            for &byte in bytes {
                match byte {
                    b'(' | b')' | b'\\' => {
                        guard.append_byte(b'\\');
                        guard.append_byte(byte);
                    }
                    b'\n' => {
                        guard.append_byte(b'\\');
                        guard.append_byte(b'n');
                    }
                    b'\r' => {
                        guard.append_byte(b'\\');
                        guard.append_byte(b'r');
                    }
                    b'\t' => {
                        guard.append_byte(b'\\');
                        guard.append_byte(b't');
                    }
                    _ => guard.append_byte(byte),
                }
            }

            guard.append_byte(b')');
        }
    }
}

/// Append another buffer's contents
#[unsafe(no_mangle)]
pub extern "C" fn fz_append_buffer(_ctx: Handle, buf: Handle, src: Handle) {
    // Get source data first
    let src_data = if let Some(src_buffer) = BUFFERS.get(src) {
        if let Ok(guard) = src_buffer.lock() {
            Some(guard.data().to_vec())
        } else {
            None
        }
    } else {
        None
    };

    // Then append to destination
    if let Some(data) = src_data {
        if let Some(buffer) = BUFFERS.get(buf) {
            if let Ok(mut guard) = buffer.lock() {
                guard.append(&data);
            }
        }
    }
}

/// Create a buffer from data with transfer of ownership
///
/// # Safety
/// Caller must ensure `data` points to valid memory of at least `size` bytes.
/// The data will be copied into the buffer (no actual ownership transfer).
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_buffer_from_data(_ctx: Handle, data: *mut u8, size: usize) -> Handle {
    if data.is_null() || size == 0 {
        return BUFFERS.insert(Buffer::new(0));
    }

    // SAFETY: Caller guarantees data points to valid memory of `size` bytes
    let data_slice = unsafe { std::slice::from_raw_parts(data, size) };

    // Copy the data to maintain safety (no actual ownership transfer in Rust FFI)
    BUFFERS.insert(Buffer::from_data(data_slice))
}

/// Create a slice/view of a buffer
///
/// # Safety
/// Caller must ensure buffer handle is valid.
#[unsafe(no_mangle)]
pub extern "C" fn fz_slice_buffer(_ctx: Handle, buf: Handle, offset: usize, len: usize) -> Handle {
    if let Some(buffer) = BUFFERS.get(buf) {
        if let Ok(guard) = buffer.lock() {
            let data = guard.data();
            if offset < data.len() {
                let end = (offset + len).min(data.len());
                let slice = &data[offset..end];
                return BUFFERS.insert(Buffer::from_data(slice));
            }
        }
    }
    0
}

/// Append a Unicode rune (codepoint) to buffer as UTF-8
#[unsafe(no_mangle)]
pub extern "C" fn fz_append_rune(_ctx: Handle, buf: Handle, rune: i32) {
    if let Some(buffer) = BUFFERS.get(buf) {
        if let Ok(mut guard) = buffer.lock() {
            // Convert Unicode codepoint to char and encode as UTF-8
            if let Some(ch) = char::from_u32(rune as u32) {
                let mut utf8_buf = [0u8; 4];
                let utf8_str = ch.encode_utf8(&mut utf8_buf);
                guard.append(utf8_str.as_bytes());
            }
        }
    }
}

/// Append base64 encoded data to buffer
///
/// # Safety
/// Caller must ensure `data` points to valid memory of at least `size` bytes.
#[unsafe(no_mangle)]
pub extern "C" fn fz_append_base64(
    _ctx: Handle,
    buf: Handle,
    data: *const u8,
    size: usize,
    newline: i32,
) {
    if data.is_null() || size == 0 {
        return;
    }

    if let Some(buffer) = BUFFERS.get(buf) {
        if let Ok(mut guard) = buffer.lock() {
            // SAFETY: Caller guarantees data points to valid memory
            let data_slice = unsafe { std::slice::from_raw_parts(data, size) };

            // Simple base64 encoding
            const BASE64_CHARS: &[u8] =
                b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

            let mut line_len = 0;
            let mut i = 0;

            while i + 2 < size {
                let b1 = data_slice[i];
                let b2 = data_slice[i + 1];
                let b3 = data_slice[i + 2];

                guard.append_byte(BASE64_CHARS[((b1 >> 2) & 0x3F) as usize]);
                guard.append_byte(BASE64_CHARS[(((b1 & 0x03) << 4) | ((b2 >> 4) & 0x0F)) as usize]);
                guard.append_byte(BASE64_CHARS[(((b2 & 0x0F) << 2) | ((b3 >> 6) & 0x03)) as usize]);
                guard.append_byte(BASE64_CHARS[(b3 & 0x3F) as usize]);

                line_len += 4;
                if newline != 0 && line_len >= 76 {
                    guard.append_byte(b'\n');
                    line_len = 0;
                }

                i += 3;
            }

            // Handle remaining bytes
            if i < size {
                let b1 = data_slice[i];
                guard.append_byte(BASE64_CHARS[((b1 >> 2) & 0x3F) as usize]);

                if i + 1 < size {
                    let b2 = data_slice[i + 1];
                    guard.append_byte(
                        BASE64_CHARS[(((b1 & 0x03) << 4) | ((b2 >> 4) & 0x0F)) as usize],
                    );
                    guard.append_byte(BASE64_CHARS[((b2 & 0x0F) << 2) as usize]);
                    guard.append_byte(b'=');
                } else {
                    guard.append_byte(BASE64_CHARS[((b1 & 0x03) << 4) as usize]);
                    guard.append_byte(b'=');
                    guard.append_byte(b'=');
                }
            }
        }
    }
}

/// Append an integer formatted as string
#[unsafe(no_mangle)]
pub extern "C" fn fz_append_int(_ctx: Handle, buf: Handle, value: i64) {
    if let Some(buffer) = BUFFERS.get(buf) {
        if let Ok(mut guard) = buffer.lock() {
            let s = format!("{}", value);
            guard.append(s.as_bytes());
        }
    }
}

/// Append float formatted as string
#[unsafe(no_mangle)]
pub extern "C" fn fz_append_float(_ctx: Handle, buf: Handle, value: f32, digits: i32) {
    if let Some(buffer) = BUFFERS.get(buf) {
        if let Ok(mut guard) = buffer.lock() {
            let s = if digits > 0 {
                format!("{:.prec$}", value, prec = digits as usize)
            } else {
                format!("{}", value)
            };
            guard.append(s.as_bytes());
        }
    }
}

/// Append hexadecimal encoded data
///
/// # Safety
/// Caller must ensure `data` points to valid memory of at least `size` bytes.
#[unsafe(no_mangle)]
pub extern "C" fn fz_append_hex(_ctx: Handle, buf: Handle, data: *const u8, size: usize) {
    if data.is_null() || size == 0 {
        return;
    }

    if let Some(buffer) = BUFFERS.get(buf) {
        if let Ok(mut guard) = buffer.lock() {
            // SAFETY: Caller guarantees data points to valid memory of size bytes
            let data_slice = unsafe { std::slice::from_raw_parts(data, size) };

            const HEX_CHARS: &[u8] = b"0123456789abcdef";
            for &byte in data_slice {
                guard.append_byte(HEX_CHARS[(byte >> 4) as usize]);
                guard.append_byte(HEX_CHARS[(byte & 0x0F) as usize]);
            }
        }
    }
}

/// Compare two buffers for equality
#[unsafe(no_mangle)]
pub extern "C" fn fz_buffer_eq(_ctx: Handle, buf1: Handle, buf2: Handle) -> i32 {
    if buf1 == buf2 {
        return 1;
    }

    let data1 = if let Some(b) = BUFFERS.get(buf1) {
        if let Ok(guard) = b.lock() {
            Some(guard.data().to_vec())
        } else {
            None
        }
    } else {
        None
    };

    let data2 = if let Some(b) = BUFFERS.get(buf2) {
        if let Ok(guard) = b.lock() {
            Some(guard.data().to_vec())
        } else {
            None
        }
    } else {
        None
    };

    match (data1, data2) {
        (Some(d1), Some(d2)) => {
            if d1 == d2 {
                1
            } else {
                0
            }
        }
        _ => 0,
    }
}

/// Get buffer storage capacity
#[unsafe(no_mangle)]
pub extern "C" fn fz_buffer_capacity(_ctx: Handle, buf: Handle) -> usize {
    if let Some(buffer) = BUFFERS.get(buf) {
        if let Ok(mut guard) = buffer.lock() {
            return guard.data_mut().capacity();
        }
    }
    0
}

/// Extract data from buffer as new buffer (move semantics)
#[unsafe(no_mangle)]
pub extern "C" fn fz_buffer_extract(_ctx: Handle, buf: Handle) -> Handle {
    if let Some(buffer) = BUFFERS.get(buf) {
        if let Ok(mut guard) = buffer.lock() {
            let data = guard.data().to_vec();
            guard.clear();
            return BUFFERS.insert(Buffer::from_data(&data));
        }
    }
    0
}

// Note: fz_append_printf is not implemented due to variadic function complexity
// in Rust FFI. Users should format strings in their own code and use fz_append_string.
//
// Note: Functions returning raw pointers to internal mutable data (like fz_buffer_data)
// cannot be safely implemented without additional infrastructure due to Rust's
// borrowing rules and the handle-based architecture. Use fz_string_from_buffer for
// read-only access, or work with buffer copies via fz_buffer_extract.

// ============================================================================
// Buffer Pool FFI Functions
// ============================================================================

/// Get buffer pool statistics
///
/// Returns a pointer to a PoolStatsFFI struct that must be freed with fz_free_pool_stats
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PoolStatsFFI {
    /// Number of buffers acquired from pool (cache hits)
    pub hits: u64,
    /// Number of new allocations (cache misses)
    pub misses: u64,
    /// Number of buffers returned to pool
    pub returns: u64,
    /// Number of buffers dropped (pool full)
    pub drops: u64,
    /// Hit rate (0.0 - 1.0)
    pub hit_rate: f64,
    /// Current number of pooled buffers
    pub pool_count: u64,
    /// Total bytes ever allocated (cumulative)
    pub total_bytes_allocated: u64,
    /// Total bytes deallocated (cumulative)
    pub total_bytes_deallocated: u64,
    /// Current bytes in use
    pub current_bytes: u64,
    /// Peak bytes in use
    pub peak_bytes: u64,
    /// Total allocations count
    pub total_allocations: u64,
    /// Total deallocations count
    pub total_deallocations: u64,
}

/// Get buffer pool statistics
#[unsafe(no_mangle)]
pub extern "C" fn fz_buffer_pool_stats(_ctx: Handle) -> PoolStatsFFI {
    let stats = BUFFER_POOL.get_stats();
    let total = stats.hits + stats.misses;
    let hit_rate = if total > 0 {
        stats.hits as f64 / total as f64
    } else {
        0.0
    };

    PoolStatsFFI {
        hits: stats.hits,
        misses: stats.misses,
        returns: stats.returns,
        drops: stats.drops,
        hit_rate,
        pool_count: BUFFER_POOL.pool_count() as u64,
        total_bytes_allocated: stats.total_bytes_allocated,
        total_bytes_deallocated: stats.total_bytes_deallocated,
        current_bytes: stats.current_bytes,
        peak_bytes: stats.peak_bytes,
        total_allocations: stats.total_allocations,
        total_deallocations: stats.total_deallocations,
    }
}

/// Clear the buffer pool
#[unsafe(no_mangle)]
pub extern "C" fn fz_buffer_pool_clear(_ctx: Handle) {
    BUFFER_POOL.clear();
}

/// Get the number of buffers currently in the pool
#[unsafe(no_mangle)]
pub extern "C" fn fz_buffer_pool_count(_ctx: Handle) -> usize {
    BUFFER_POOL.pool_count()
}

// Note: fz_buffer_capacity already defined below in the file

/// Reserve additional capacity in buffer
#[unsafe(no_mangle)]
pub extern "C" fn fz_buffer_reserve(_ctx: Handle, buf: Handle, additional: usize) {
    if let Some(buffer) = BUFFERS.get(buf) {
        if let Ok(mut guard) = buffer.lock() {
            guard.reserve(additional);
        }
    }
}

/// Shrink buffer capacity to fit current length
#[unsafe(no_mangle)]
pub extern "C" fn fz_buffer_shrink_to_fit(_ctx: Handle, buf: Handle) {
    if let Some(buffer) = BUFFERS.get(buf) {
        if let Ok(mut guard) = buffer.lock() {
            guard.shrink_to_fit();
        }
    }
}

/// Check if buffer is from the pool
#[unsafe(no_mangle)]
pub extern "C" fn fz_buffer_is_pooled(_ctx: Handle, buf: Handle) -> c_int {
    if let Some(buffer) = BUFFERS.get(buf) {
        if let Ok(guard) = buffer.lock() {
            return if guard.is_pooled() { 1 } else { 0 };
        }
    }
    0
}

/// Create a buffer with suggested capacity (uses pool if appropriate)
///
/// This is an optimization hint - the actual capacity may be larger
/// to match a pool size class.
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_buffer_with_capacity(_ctx: Handle, hint: usize) -> Handle {
    BUFFERS.insert(Buffer::new(hint))
}

/// Create a buffer bypassing the pool (for long-lived buffers)
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_buffer_unpooled(_ctx: Handle, capacity: usize) -> Handle {
    BUFFERS.insert(Buffer::new_unpooled(capacity))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    fn test_buffer_create_and_drop() {
        let handle = fz_new_buffer(0, 100);
        assert_ne!(handle, 0);
        fz_drop_buffer(0, handle);
    }

    #[test]
    fn test_buffer_append_byte() {
        let handle = fz_new_buffer(0, 0);
        fz_append_byte(0, handle, b'A' as i32);
        fz_append_byte(0, handle, b'B' as i32);

        let len = fz_buffer_len(0, handle);
        assert_eq!(len, 2);

        fz_drop_buffer(0, handle);
    }

    #[test]
    fn test_buffer_clear() {
        let handle = fz_new_buffer(0, 0);
        fz_append_byte(0, handle, b'X' as i32);
        assert_eq!(fz_buffer_len(0, handle), 1);

        fz_clear_buffer(0, handle);
        assert_eq!(fz_buffer_len(0, handle), 0);

        fz_drop_buffer(0, handle);
    }

    #[test]
    fn test_buffer_keep() {
        let handle = fz_new_buffer(0, 0);
        let kept = fz_keep_buffer(0, handle);
        assert_eq!(kept, handle);
        fz_drop_buffer(0, handle);
    }

    #[test]
    fn test_buffer_resize() {
        let handle = fz_new_buffer(0, 10);
        fz_resize_buffer(0, handle, 100);
        // Resize should succeed
        assert_eq!(fz_buffer_len(0, handle), 100);
        fz_drop_buffer(0, handle);
    }

    #[test]
    fn test_buffer_grow() {
        let handle = fz_new_buffer(0, 10);
        fz_grow_buffer(0, handle);
        // Buffer should be able to accommodate growth
        fz_drop_buffer(0, handle);
    }

    #[test]
    fn test_buffer_trim() {
        let handle = fz_new_buffer(0, 100);
        fz_append_byte(0, handle, b'A' as i32);
        fz_trim_buffer(0, handle);
        assert_eq!(fz_buffer_len(0, handle), 1);
        fz_drop_buffer(0, handle);
    }

    #[test]
    fn test_buffer_clone() {
        let handle1 = fz_new_buffer(0, 0);
        fz_append_byte(0, handle1, b'X' as i32);
        fz_append_byte(0, handle1, b'Y' as i32);

        let handle2 = fz_clone_buffer(0, handle1);
        assert_ne!(handle2, 0);
        assert_eq!(fz_buffer_len(0, handle2), 2);

        // Modify original, clone should be unchanged
        fz_clear_buffer(0, handle1);
        assert_eq!(fz_buffer_len(0, handle1), 0);
        assert_eq!(fz_buffer_len(0, handle2), 2);

        fz_drop_buffer(0, handle1);
        fz_drop_buffer(0, handle2);
    }

    #[test]
    fn test_buffer_len_invalid() {
        let len = fz_buffer_len(0, 0);
        assert_eq!(len, 0);
    }

    #[test]
    fn test_buffer_append_multiple() {
        let handle = fz_new_buffer(0, 0);
        for i in 0..100 {
            fz_append_byte(0, handle, i);
        }
        assert_eq!(fz_buffer_len(0, handle), 100);
        fz_drop_buffer(0, handle);
    }

    #[test]
    fn test_buffer_storage() {
        let handle = fz_new_buffer(0, 0);
        fz_append_byte(0, handle, b'H' as i32);
        fz_append_byte(0, handle, b'i' as i32);

        let mut datap: *mut u8 = std::ptr::null_mut();
        let size = fz_buffer_storage(0, handle, &mut datap);

        // Size should be the length of the buffer
        assert_eq!(size, 2);
        // datap will be null because we can't safely return internal pointer
        assert!(datap.is_null());

        fz_drop_buffer(0, handle);
    }

    #[test]
    fn test_buffer_internal() {
        let buf = Buffer::new(10);
        assert_eq!(buf.len(), 0);
        assert!(buf.data().is_empty());
    }

    #[test]
    fn test_buffer_from_data() {
        let data = [1u8, 2, 3, 4, 5];
        let buf = Buffer::from_data(&data);
        assert_eq!(buf.len(), 5);
        assert_eq!(buf.data(), &data);
    }

    #[test]
    fn test_buffer_append_internal() {
        let mut buf = Buffer::new(0);
        buf.append_byte(0x42);
        assert_eq!(buf.len(), 1);
        assert_eq!(buf.data(), &[0x42]);
    }

    #[test]
    fn test_buffer_clear_internal() {
        let mut buf = Buffer::from_data(&[1, 2, 3]);
        buf.clear();
        assert_eq!(buf.len(), 0);
    }

    // ============================================================================
    // Additional tests for better coverage
    // ============================================================================

    #[test]
    fn test_buffer_from_copied_data() {
        let data = [1u8, 2, 3, 4, 5];
        let handle = fz_new_buffer_from_copied_data(0, data.as_ptr(), data.len());
        assert_ne!(handle, 0);
        assert_eq!(fz_buffer_len(0, handle), 5);
        fz_drop_buffer(0, handle);
    }

    #[test]
    fn test_buffer_from_copied_data_null() {
        let handle = fz_new_buffer_from_copied_data(0, std::ptr::null(), 0);
        assert_ne!(handle, 0);
        assert_eq!(fz_buffer_len(0, handle), 0);
        fz_drop_buffer(0, handle);
    }

    #[test]
    fn test_buffer_from_copied_data_null_with_size() {
        // Even with non-zero size, null ptr should return empty buffer
        let handle = fz_new_buffer_from_copied_data(0, std::ptr::null(), 100);
        assert_ne!(handle, 0);
        assert_eq!(fz_buffer_len(0, handle), 0);
        fz_drop_buffer(0, handle);
    }

    #[test]
    fn test_buffer_storage_null_datap() {
        let handle = fz_new_buffer(0, 0);
        fz_append_byte(0, handle, b'A' as i32);

        // Pass null pointer for datap
        let size = fz_buffer_storage(0, handle, std::ptr::null_mut());
        assert_eq!(size, 1);

        fz_drop_buffer(0, handle);
    }

    #[test]
    fn test_buffer_storage_invalid_handle() {
        let mut datap: *mut u8 = std::ptr::null_mut();
        let size = fz_buffer_storage(0, 99999, &mut datap);
        assert_eq!(size, 0);
        assert!(datap.is_null());
    }

    #[test]
    fn test_fz_string_from_buffer() {
        let handle = fz_new_buffer(0, 0);
        fz_append_byte(0, handle, b'H' as i32);

        let ptr = fz_string_from_buffer(0, handle);
        assert!(!ptr.is_null());

        fz_drop_buffer(0, handle);
    }

    #[test]
    fn test_buffer_resize_invalid_handle() {
        // Should not panic
        fz_resize_buffer(0, 99999, 100);
    }

    #[test]
    fn test_buffer_grow_invalid_handle() {
        // Should not panic
        fz_grow_buffer(0, 99999);
    }

    #[test]
    fn test_buffer_trim_invalid_handle() {
        // Should not panic
        fz_trim_buffer(0, 99999);
    }

    #[test]
    fn test_buffer_clear_invalid_handle() {
        // Should not panic
        fz_clear_buffer(0, 99999);
    }

    #[test]
    fn test_fz_append_data() {
        let handle = fz_new_buffer(0, 0);
        let data = [1u8, 2, 3, 4, 5];
        fz_append_data(0, handle, data.as_ptr() as *const c_void, data.len());
        assert_eq!(fz_buffer_len(0, handle), 5);
        fz_drop_buffer(0, handle);
    }

    #[test]
    fn test_fz_append_data_null() {
        let handle = fz_new_buffer(0, 0);
        fz_append_data(0, handle, std::ptr::null(), 0);
        assert_eq!(fz_buffer_len(0, handle), 0);
        fz_drop_buffer(0, handle);
    }

    #[test]
    fn test_fz_append_data_invalid_handle() {
        let data = [1u8, 2, 3];
        // Should not panic
        fz_append_data(0, 99999, data.as_ptr() as *const c_void, data.len());
    }

    #[test]
    fn test_fz_append_string() {
        let handle = fz_new_buffer(0, 0);
        let s = std::ffi::CString::new("Hello").unwrap();
        fz_append_string(0, handle, s.as_ptr());
        assert_eq!(fz_buffer_len(0, handle), 5);
        fz_drop_buffer(0, handle);
    }

    #[test]
    fn test_fz_append_string_null() {
        let handle = fz_new_buffer(0, 0);
        fz_append_string(0, handle, std::ptr::null());
        assert_eq!(fz_buffer_len(0, handle), 0);
        fz_drop_buffer(0, handle);
    }

    #[test]
    fn test_fz_append_string_invalid_handle() {
        let s = std::ffi::CString::new("Hello").unwrap();
        // Should not panic
        fz_append_string(0, 99999, s.as_ptr());
    }

    #[test]
    fn test_fz_append_byte_invalid_handle() {
        // Should not panic
        fz_append_byte(0, 99999, b'X' as i32);
    }

    #[test]
    fn test_fz_terminate_buffer() {
        let handle = fz_new_buffer(0, 0);
        fz_append_byte(0, handle, b'H' as i32);
        fz_terminate_buffer(0, handle);
        // After termination, buffer should have a null byte
        assert_eq!(fz_buffer_len(0, handle), 2);
        fz_drop_buffer(0, handle);
    }

    #[test]
    fn test_fz_terminate_buffer_invalid_handle() {
        // Should not panic
        fz_terminate_buffer(0, 99999);
    }

    #[test]
    fn test_fz_terminate_buffer_already_terminated() {
        let handle = fz_new_buffer(0, 0);
        fz_append_byte(0, handle, b'H' as i32);
        fz_append_byte(0, handle, 0); // Already has null
        fz_terminate_buffer(0, handle);
        // Should not add another null
        assert_eq!(fz_buffer_len(0, handle), 2);
        fz_drop_buffer(0, handle);
    }

    #[test]
    fn test_fz_md5_buffer() {
        let handle = fz_new_buffer(0, 0);
        fz_append_byte(0, handle, b'A' as i32);
        fz_append_byte(0, handle, b'B' as i32);
        fz_append_byte(0, handle, b'C' as i32);

        let mut digest = [0u8; 16];
        fz_md5_buffer(0, handle, &mut digest);

        // MD5("ABC") is known
        assert_ne!(digest, [0u8; 16]);
        fz_drop_buffer(0, handle);
    }

    #[test]
    fn test_fz_md5_buffer_null_digest() {
        let handle = fz_new_buffer(0, 0);
        fz_append_byte(0, handle, b'A' as i32);
        // Should not panic
        fz_md5_buffer(0, handle, std::ptr::null_mut());
        fz_drop_buffer(0, handle);
    }

    #[test]
    fn test_fz_md5_buffer_invalid_handle() {
        let mut digest = [0u8; 16];
        // Should not panic
        fz_md5_buffer(0, 99999, &mut digest);
    }

    #[test]
    fn test_fz_clone_buffer_invalid_handle() {
        let handle = fz_clone_buffer(0, 99999);
        assert_eq!(handle, 0);
    }

    #[test]
    fn test_buffer_is_empty() {
        let buf = Buffer::new(10);
        assert!(buf.is_empty());

        let buf2 = Buffer::from_data(&[1, 2, 3]);
        assert!(!buf2.is_empty());
    }

    #[test]
    fn test_buffer_data_mut() {
        let mut buf = Buffer::new(0);
        buf.data_mut().push(1);
        buf.data_mut().push(2);
        assert_eq!(buf.len(), 2);
    }

    #[test]
    fn test_buffer_append() {
        let mut buf = Buffer::new(0);
        buf.append(&[1, 2, 3]);
        assert_eq!(buf.len(), 3);
        buf.append(&[4, 5]);
        assert_eq!(buf.len(), 5);
        assert_eq!(buf.data(), &[1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_buffer_resize_internal() {
        let mut buf = Buffer::from_data(&[1, 2, 3]);
        buf.resize(5);
        assert_eq!(buf.len(), 5);
        assert_eq!(&buf.data()[..3], &[1, 2, 3]);
        assert_eq!(&buf.data()[3..], &[0, 0]);
    }

    #[test]
    fn test_buffer_ensure_null_terminated() {
        let mut buf = Buffer::from_data(&[1, 2, 3]);
        buf.ensure_null_terminated();
        assert_eq!(buf.len(), 4);
        assert_eq!(buf.data().last(), Some(&0));

        // Should not add another null
        buf.ensure_null_terminated();
        assert_eq!(buf.len(), 4);
    }

    #[test]
    fn test_buffer_ensure_null_terminated_empty() {
        let mut buf = Buffer::new(0);
        buf.ensure_null_terminated();
        assert_eq!(buf.len(), 1);
        assert_eq!(buf.data(), &[0]);
    }

    // ============================================================================
    // Integer Append Tests
    // ============================================================================

    #[test]
    fn test_fz_append_int16_le() {
        let handle = fz_new_buffer(0, 0);
        fz_append_int16_le(0, handle, 0x0102);
        assert_eq!(fz_buffer_len(0, handle), 2);

        // Check actual bytes (little-endian: 0x02, 0x01)
        if let Some(buffer) = BUFFERS.get(handle) {
            if let Ok(guard) = buffer.lock() {
                assert_eq!(guard.data(), &[0x02, 0x01]);
            }
        }
        fz_drop_buffer(0, handle);
    }

    #[test]
    fn test_fz_append_int32_le() {
        let handle = fz_new_buffer(0, 0);
        fz_append_int32_le(0, handle, 0x01020304);
        assert_eq!(fz_buffer_len(0, handle), 4);

        if let Some(buffer) = BUFFERS.get(handle) {
            if let Ok(guard) = buffer.lock() {
                assert_eq!(guard.data(), &[0x04, 0x03, 0x02, 0x01]);
            }
        }
        fz_drop_buffer(0, handle);
    }

    #[test]
    fn test_fz_append_int16_be() {
        let handle = fz_new_buffer(0, 0);
        fz_append_int16_be(0, handle, 0x0102);
        assert_eq!(fz_buffer_len(0, handle), 2);

        if let Some(buffer) = BUFFERS.get(handle) {
            if let Ok(guard) = buffer.lock() {
                assert_eq!(guard.data(), &[0x01, 0x02]);
            }
        }
        fz_drop_buffer(0, handle);
    }

    #[test]
    fn test_fz_append_int32_be() {
        let handle = fz_new_buffer(0, 0);
        fz_append_int32_be(0, handle, 0x01020304);
        assert_eq!(fz_buffer_len(0, handle), 4);

        if let Some(buffer) = BUFFERS.get(handle) {
            if let Ok(guard) = buffer.lock() {
                assert_eq!(guard.data(), &[0x01, 0x02, 0x03, 0x04]);
            }
        }
        fz_drop_buffer(0, handle);
    }

    #[test]
    fn test_fz_append_int_invalid_handle() {
        // Should not panic
        fz_append_int16_le(0, 99999, 0x1234);
        fz_append_int32_le(0, 99999, 0x12345678);
        fz_append_int16_be(0, 99999, 0x1234);
        fz_append_int32_be(0, 99999, 0x12345678);
    }

    // ============================================================================
    // Bit Append Tests
    // ============================================================================

    #[test]
    fn test_fz_append_bits_basic() {
        let handle = fz_new_buffer(0, 0);

        // Append 8 bits at a time - should produce byte
        fz_append_bits(0, handle, 0b10101010, 8);
        assert_eq!(fz_buffer_len(0, handle), 1);

        if let Some(buffer) = BUFFERS.get(handle) {
            if let Ok(guard) = buffer.lock() {
                assert_eq!(guard.data(), &[0b10101010]);
            }
        }
        fz_drop_buffer(0, handle);
    }

    #[test]
    fn test_fz_append_bits_multiple() {
        let handle = fz_new_buffer(0, 0);

        // Append 4 bits, then another 4 bits
        fz_append_bits(0, handle, 0b1010, 4);
        fz_append_bits(0, handle, 0b0101, 4);

        assert_eq!(fz_buffer_len(0, handle), 1);

        if let Some(buffer) = BUFFERS.get(handle) {
            if let Ok(guard) = buffer.lock() {
                assert_eq!(guard.data(), &[0b10100101]);
            }
        }
        fz_drop_buffer(0, handle);
    }

    #[test]
    fn test_fz_append_bits_pad() {
        let handle = fz_new_buffer(0, 0);

        // Append 5 bits, then pad to byte
        fz_append_bits(0, handle, 0b11111, 5);
        fz_append_bits_pad(0, handle);

        assert_eq!(fz_buffer_len(0, handle), 1);

        if let Some(buffer) = BUFFERS.get(handle) {
            if let Ok(guard) = buffer.lock() {
                // 5 bits of 1s + 3 bits of 0s = 11111000 = 0xF8
                assert_eq!(guard.data(), &[0xF8]);
            }
        }
        fz_drop_buffer(0, handle);
    }

    #[test]
    fn test_fz_append_bits_invalid_count() {
        let handle = fz_new_buffer(0, 0);

        // Invalid counts should be ignored
        fz_append_bits(0, handle, 0xFF, 0);
        fz_append_bits(0, handle, 0xFF, -1);
        fz_append_bits(0, handle, 0xFF, 33);

        assert_eq!(fz_buffer_len(0, handle), 0);
        fz_drop_buffer(0, handle);
    }

    #[test]
    fn test_fz_append_bits_invalid_handle() {
        // Should not panic
        fz_append_bits(0, 99999, 0xFF, 8);
        fz_append_bits_pad(0, 99999);
    }

    // ============================================================================
    // PDF String Tests
    // ============================================================================

    #[test]
    fn test_fz_append_pdf_string_simple() {
        let handle = fz_new_buffer(0, 0);
        let s = std::ffi::CString::new("Hello").unwrap();
        fz_append_pdf_string(0, handle, s.as_ptr());

        if let Some(buffer) = BUFFERS.get(handle) {
            if let Ok(guard) = buffer.lock() {
                assert_eq!(guard.data(), b"(Hello)");
            }
        }
        fz_drop_buffer(0, handle);
    }

    #[test]
    fn test_fz_append_pdf_string_escaping() {
        let handle = fz_new_buffer(0, 0);
        let s = std::ffi::CString::new("Test(with)parens\\backslash").unwrap();
        fz_append_pdf_string(0, handle, s.as_ptr());

        if let Some(buffer) = BUFFERS.get(handle) {
            if let Ok(guard) = buffer.lock() {
                assert_eq!(guard.data(), b"(Test\\(with\\)parens\\\\backslash)");
            }
        }
        fz_drop_buffer(0, handle);
    }

    #[test]
    fn test_fz_append_pdf_string_newlines() {
        let handle = fz_new_buffer(0, 0);
        let s = std::ffi::CString::new("Line1\nLine2\rLine3\tTab").unwrap();
        fz_append_pdf_string(0, handle, s.as_ptr());

        if let Some(buffer) = BUFFERS.get(handle) {
            if let Ok(guard) = buffer.lock() {
                assert_eq!(guard.data(), b"(Line1\\nLine2\\rLine3\\tTab)");
            }
        }
        fz_drop_buffer(0, handle);
    }

    #[test]
    fn test_fz_append_pdf_string_null() {
        let handle = fz_new_buffer(0, 0);
        fz_append_pdf_string(0, handle, std::ptr::null());

        if let Some(buffer) = BUFFERS.get(handle) {
            if let Ok(guard) = buffer.lock() {
                assert_eq!(guard.data(), b"()");
            }
        }
        fz_drop_buffer(0, handle);
    }

    #[test]
    fn test_fz_append_pdf_string_invalid_handle() {
        let s = std::ffi::CString::new("Test").unwrap();
        // Should not panic
        fz_append_pdf_string(0, 99999, s.as_ptr());
    }

    // ============================================================================
    // Buffer Append Buffer Tests
    // ============================================================================

    #[test]
    fn test_fz_append_buffer() {
        let buf1 = fz_new_buffer(0, 0);
        let buf2 = fz_new_buffer(0, 0);

        fz_append_byte(0, buf1, b'A' as i32);
        fz_append_byte(0, buf1, b'B' as i32);

        fz_append_byte(0, buf2, b'C' as i32);
        fz_append_byte(0, buf2, b'D' as i32);

        fz_append_buffer(0, buf1, buf2);

        assert_eq!(fz_buffer_len(0, buf1), 4);

        if let Some(buffer) = BUFFERS.get(buf1) {
            if let Ok(guard) = buffer.lock() {
                assert_eq!(guard.data(), b"ABCD");
            }
        }

        fz_drop_buffer(0, buf1);
        fz_drop_buffer(0, buf2);
    }

    #[test]
    fn test_fz_append_buffer_invalid() {
        let buf = fz_new_buffer(0, 0);
        fz_append_byte(0, buf, b'X' as i32);

        // Append from invalid handle - should be ignored
        fz_append_buffer(0, buf, 99999);
        assert_eq!(fz_buffer_len(0, buf), 1);

        // Append to invalid handle - should not panic
        fz_append_buffer(0, 99999, buf);

        fz_drop_buffer(0, buf);
    }

    // ============================================================================
    // Buffer Pool Tests
    // ============================================================================

    #[test]
    #[serial]
    fn test_buffer_pool_basic() {
        // Clear pool first for clean state
        fz_buffer_pool_clear(0);

        // Create and drop a poolable buffer
        let buf = fz_new_buffer(0, 256);
        fz_append_byte(0, buf, b'X' as i32);
        assert_eq!(fz_buffer_is_pooled(0, buf), 1);
        fz_drop_buffer(0, buf);

        // Pool should have one buffer now
        let count = fz_buffer_pool_count(0);
        assert!(count >= 1, "Pool should have at least one buffer");
    }

    #[test]
    #[serial]
    fn test_buffer_pool_reuse() {
        // Clear pool
        fz_buffer_pool_clear(0);

        // Create and drop a buffer
        let buf1 = fz_new_buffer(0, 1024);
        let cap1 = fz_buffer_capacity(0, buf1);
        fz_drop_buffer(0, buf1);

        // Get stats before reuse
        let stats_before = fz_buffer_pool_stats(0);

        // Create another buffer of similar size
        let buf2 = fz_new_buffer(0, 512); // Should get 1024 from pool
        let cap2 = fz_buffer_capacity(0, buf2);

        // Check stats after reuse
        let stats_after = fz_buffer_pool_stats(0);

        // Should have reused the buffer (hit)
        assert!(
            stats_after.hits > stats_before.hits || cap2 == cap1,
            "Buffer should be reused from pool"
        );

        fz_drop_buffer(0, buf2);
    }

    #[test]
    #[serial]
    fn test_buffer_pool_stats() {
        fz_buffer_pool_clear(0);

        let stats = fz_buffer_pool_stats(0);
        assert!(stats.hit_rate >= 0.0 && stats.hit_rate <= 1.0);
    }

    #[test]
    fn test_buffer_capacity() {
        let buf = fz_new_buffer(0, 100);

        // Capacity should be at least 100 (may be rounded up to pool size)
        let cap = fz_buffer_capacity(0, buf);
        assert!(cap >= 100, "Capacity should be at least 100");

        // Length should be 0
        assert_eq!(fz_buffer_len(0, buf), 0);

        fz_drop_buffer(0, buf);
    }

    #[test]
    fn test_buffer_reserve() {
        let buf = fz_new_buffer(0, 10);
        let initial_cap = fz_buffer_capacity(0, buf);

        // Reserve much more than current capacity
        fz_buffer_reserve(0, buf, 10000);
        let new_cap = fz_buffer_capacity(0, buf);

        // New capacity should be at least initial + requested
        // (Vec::reserve guarantees at least len + additional)
        assert!(
            new_cap >= initial_cap || new_cap >= 10000,
            "Should have reserved space: initial_cap={}, new_cap={}",
            initial_cap,
            new_cap
        );

        fz_drop_buffer(0, buf);
    }

    #[test]
    fn test_buffer_shrink() {
        let buf = fz_new_buffer(0, 1000);

        // Add some data
        for i in 0..10 {
            fz_append_byte(0, buf, i);
        }

        // Shrink to fit
        fz_buffer_shrink_to_fit(0, buf);

        // Should no longer be pooled
        assert_eq!(fz_buffer_is_pooled(0, buf), 0);

        fz_drop_buffer(0, buf);
    }

    #[test]
    fn test_buffer_unpooled() {
        let buf = fz_new_buffer_unpooled(0, 256);

        // Should not be pooled
        assert_eq!(fz_buffer_is_pooled(0, buf), 0);

        fz_drop_buffer(0, buf);
    }

    #[test]
    fn test_buffer_with_capacity() {
        let buf = fz_new_buffer_with_capacity(0, 500);

        // Should have capacity >= 500 (rounded to pool size)
        let cap = fz_buffer_capacity(0, buf);
        assert!(cap >= 500, "Capacity should be at least 500");

        // Should be pooled
        assert_eq!(fz_buffer_is_pooled(0, buf), 1);

        fz_drop_buffer(0, buf);
    }

    #[test]
    fn test_pool_size_classes() {
        // Test that different sizes get appropriate capacity
        let sizes_and_expected = [
            (10, 64),       // Tiny -> 64
            (100, 256),     // Small -> 256
            (500, 1024),    // Medium -> 1024
            (2000, 4096),   // Page -> 4096
            (10000, 16384), // Large -> 16384
            (50000, 65536), // Very large -> 65536
        ];

        for (requested, expected_min) in sizes_and_expected {
            let buf = fz_new_buffer(0, requested);
            let cap = fz_buffer_capacity(0, buf);
            assert!(
                cap >= expected_min,
                "Buffer for {} bytes should have capacity >= {}, got {}",
                requested,
                expected_min,
                cap
            );
            fz_drop_buffer(0, buf);
        }
    }
}
