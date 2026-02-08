//! Data Locality Optimization
//!
//! Utilities for improving cache utilization and memory access patterns:
//! - `PageAlignedBuffer`: Page-aligned allocations for large buffers
//! - `Prefetch`: CPU prefetch hints for sequential/random access
//! - `SoA` patterns: Struct-of-Arrays for SIMD-friendly data layouts
//!
//! Benefits:
//! - Reduced TLB misses with page-aligned data
//! - Improved cache hit rates with prefetching
//! - Better vectorization with SoA layouts

use std::alloc::{self, Layout};
use std::ffi::c_int;
use std::mem::{align_of, size_of};
use std::ptr::NonNull;
use std::sync::LazyLock;
use std::sync::atomic::{AtomicU64, Ordering};

use super::HandleStore;
use super::struct_layout::{CACHE_LINE_SIZE, PAGE_SIZE};

// ============================================================================
// Prefetch Hints
// ============================================================================

/// Prefetch locality hints (how soon data will be reused)
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrefetchLocality {
    /// Non-temporal: data used once, don't pollute cache
    NonTemporal = 0,
    /// Low: data might be reused eventually
    Low = 1,
    /// Medium: data will be reused soon
    Medium = 2,
    /// High: data will be reused very soon (keep in all cache levels)
    High = 3,
}

/// Prefetch for read access
///
/// Hints to the CPU that we'll read from this address soon.
/// This is a no-op on platforms without prefetch support.
#[inline]
pub fn prefetch_read<T>(ptr: *const T, locality: PrefetchLocality) {
    #[cfg(target_arch = "x86_64")]
    unsafe {
        match locality {
            PrefetchLocality::NonTemporal => {
                std::arch::x86_64::_mm_prefetch(ptr as *const i8, std::arch::x86_64::_MM_HINT_NTA);
            }
            PrefetchLocality::Low => {
                std::arch::x86_64::_mm_prefetch(ptr as *const i8, std::arch::x86_64::_MM_HINT_T2);
            }
            PrefetchLocality::Medium => {
                std::arch::x86_64::_mm_prefetch(ptr as *const i8, std::arch::x86_64::_MM_HINT_T1);
            }
            PrefetchLocality::High => {
                std::arch::x86_64::_mm_prefetch(ptr as *const i8, std::arch::x86_64::_MM_HINT_T0);
            }
        }
    }

    #[cfg(target_arch = "aarch64")]
    unsafe {
        // ARM64 prefetch using PRFM instruction via inline assembly
        match locality {
            PrefetchLocality::NonTemporal => {
                std::arch::asm!("prfm pldl1strm, [{0}]", in(reg) ptr, options(nostack, preserves_flags));
            }
            PrefetchLocality::Low | PrefetchLocality::Medium => {
                std::arch::asm!("prfm pldl2keep, [{0}]", in(reg) ptr, options(nostack, preserves_flags));
            }
            PrefetchLocality::High => {
                std::arch::asm!("prfm pldl1keep, [{0}]", in(reg) ptr, options(nostack, preserves_flags));
            }
        }
    }

    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    {
        let _ = (ptr, locality); // No-op on unsupported platforms
    }
}

/// Prefetch for write access
///
/// Hints to the CPU that we'll write to this address soon.
#[inline]
pub fn prefetch_write<T>(ptr: *mut T, locality: PrefetchLocality) {
    #[cfg(target_arch = "x86_64")]
    unsafe {
        // x86_64 doesn't have separate write prefetch, use read prefetch
        // with exclusive hint via PREFETCHW when available
        match locality {
            PrefetchLocality::NonTemporal => {
                std::arch::x86_64::_mm_prefetch(ptr as *const i8, std::arch::x86_64::_MM_HINT_NTA);
            }
            _ => {
                std::arch::x86_64::_mm_prefetch(ptr as *const i8, std::arch::x86_64::_MM_HINT_T0);
            }
        }
    }

    #[cfg(target_arch = "aarch64")]
    unsafe {
        match locality {
            PrefetchLocality::NonTemporal => {
                std::arch::asm!("prfm pstl1strm, [{0}]", in(reg) ptr, options(nostack, preserves_flags));
            }
            PrefetchLocality::Low | PrefetchLocality::Medium => {
                std::arch::asm!("prfm pstl2keep, [{0}]", in(reg) ptr, options(nostack, preserves_flags));
            }
            PrefetchLocality::High => {
                std::arch::asm!("prfm pstl1keep, [{0}]", in(reg) ptr, options(nostack, preserves_flags));
            }
        }
    }

    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    {
        let _ = (ptr, locality);
    }
}

/// Prefetch a range of memory for sequential read
#[inline]
pub fn prefetch_range_read(ptr: *const u8, len: usize, locality: PrefetchLocality) {
    let mut offset = 0;
    while offset < len {
        prefetch_read(unsafe { ptr.add(offset) }, locality);
        offset += CACHE_LINE_SIZE;
    }
}

/// Prefetch a range of memory for sequential write
#[inline]
pub fn prefetch_range_write(ptr: *mut u8, len: usize, locality: PrefetchLocality) {
    let mut offset = 0;
    while offset < len {
        prefetch_write(unsafe { ptr.add(offset) }, locality);
        offset += CACHE_LINE_SIZE;
    }
}

// ============================================================================
// Page-Aligned Buffer
// ============================================================================

/// A buffer allocated at a page boundary
///
/// Page alignment (typically 4KB) provides:
/// - Single TLB entry for the buffer
/// - Efficient mmap/DMA operations
/// - Better huge page support
pub struct PageAlignedBuffer {
    ptr: NonNull<u8>,
    len: usize,
    capacity: usize,
}

impl PageAlignedBuffer {
    /// Allocate a new page-aligned buffer
    pub fn new(capacity: usize) -> Option<Self> {
        if capacity == 0 {
            return Some(Self {
                ptr: NonNull::dangling(),
                len: 0,
                capacity: 0,
            });
        }

        // Round up to page boundary
        let aligned_capacity = (capacity + PAGE_SIZE - 1) & !(PAGE_SIZE - 1);

        let layout = Layout::from_size_align(aligned_capacity, PAGE_SIZE).ok()?;
        let ptr = unsafe { alloc::alloc_zeroed(layout) };

        NonNull::new(ptr).map(|ptr| Self {
            ptr,
            len: 0,
            capacity: aligned_capacity,
        })
    }

    /// Create from existing data (copies data)
    pub fn from_slice(data: &[u8]) -> Option<Self> {
        let mut buf = Self::new(data.len())?;
        buf.extend_from_slice(data);
        Some(buf)
    }

    /// Get raw pointer
    #[inline]
    pub fn as_ptr(&self) -> *const u8 {
        self.ptr.as_ptr()
    }

    /// Get mutable raw pointer
    #[inline]
    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        self.ptr.as_ptr()
    }

    /// Get length
    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    /// Check if empty
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Get capacity
    #[inline]
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Get as slice
    #[inline]
    pub fn as_slice(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.ptr.as_ptr(), self.len) }
    }

    /// Get as mutable slice
    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        unsafe { std::slice::from_raw_parts_mut(self.ptr.as_ptr(), self.len) }
    }

    /// Extend from slice
    pub fn extend_from_slice(&mut self, data: &[u8]) -> bool {
        if self.len + data.len() > self.capacity {
            return false;
        }

        unsafe {
            std::ptr::copy_nonoverlapping(
                data.as_ptr(),
                self.ptr.as_ptr().add(self.len),
                data.len(),
            );
        }
        self.len += data.len();
        true
    }

    /// Set length (unsafe: caller must ensure data is initialized)
    ///
    /// # Safety
    /// Caller must ensure all bytes up to `new_len` are initialized.
    #[inline]
    pub unsafe fn set_len(&mut self, new_len: usize) {
        debug_assert!(new_len <= self.capacity);
        self.len = new_len;
    }

    /// Clear the buffer
    #[inline]
    pub fn clear(&mut self) {
        self.len = 0;
    }

    /// Prefetch for sequential read
    #[inline]
    pub fn prefetch_read(&self, locality: PrefetchLocality) {
        prefetch_range_read(self.ptr.as_ptr(), self.len, locality);
    }

    /// Prefetch for sequential write
    #[inline]
    pub fn prefetch_write(&mut self, locality: PrefetchLocality) {
        prefetch_range_write(self.ptr.as_ptr(), self.capacity, locality);
    }

    /// Check if pointer is page-aligned
    #[inline]
    pub fn is_page_aligned(&self) -> bool {
        (self.ptr.as_ptr() as usize) % PAGE_SIZE == 0
    }

    /// Get number of pages used
    #[inline]
    pub fn pages_used(&self) -> usize {
        if self.len == 0 {
            0
        } else {
            (self.len + PAGE_SIZE - 1) / PAGE_SIZE
        }
    }
}

impl Drop for PageAlignedBuffer {
    fn drop(&mut self) {
        if self.capacity > 0 {
            let layout = Layout::from_size_align(self.capacity, PAGE_SIZE).unwrap();
            unsafe {
                alloc::dealloc(self.ptr.as_ptr(), layout);
            }
        }
    }
}

// Safety: PageAlignedBuffer owns its allocation
unsafe impl Send for PageAlignedBuffer {}
unsafe impl Sync for PageAlignedBuffer {}

// ============================================================================
// Struct-of-Arrays Pattern
// ============================================================================

/// A struct-of-arrays container for Point data
///
/// Instead of: `Vec<Point>` where Point is { x: f32, y: f32 }
/// Uses: `PointSoA` with separate `xs: Vec<f32>` and `ys: Vec<f32>`
///
/// Benefits:
/// - SIMD-friendly: process all X values, then all Y values
/// - Cache-friendly: sequential access to same-type data
/// - Vectorization: compiler can auto-vectorize loops
pub struct PointSoA {
    pub xs: Vec<f32>,
    pub ys: Vec<f32>,
}

impl PointSoA {
    /// Create empty SoA
    pub fn new() -> Self {
        Self {
            xs: Vec::new(),
            ys: Vec::new(),
        }
    }

    /// Create with capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            xs: Vec::with_capacity(capacity),
            ys: Vec::with_capacity(capacity),
        }
    }

    /// Number of points
    #[inline]
    pub fn len(&self) -> usize {
        self.xs.len()
    }

    /// Check if empty
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.xs.is_empty()
    }

    /// Add a point
    #[inline]
    pub fn push(&mut self, x: f32, y: f32) {
        self.xs.push(x);
        self.ys.push(y);
    }

    /// Get point at index
    #[inline]
    pub fn get(&self, index: usize) -> Option<(f32, f32)> {
        self.xs.get(index).map(|&x| (x, self.ys[index]))
    }

    /// Clear all points
    #[inline]
    pub fn clear(&mut self) {
        self.xs.clear();
        self.ys.clear();
    }

    /// Transform all points by a matrix (SIMD-friendly)
    pub fn transform(&mut self, a: f32, b: f32, c: f32, d: f32, e: f32, f: f32) {
        let len = self.xs.len();

        // Process in chunks for better cache utilization
        for i in 0..len {
            let x = self.xs[i];
            let y = self.ys[i];
            self.xs[i] = x * a + y * c + e;
            self.ys[i] = x * b + y * d + f;
        }
    }

    /// Prefetch for read
    pub fn prefetch_read(&self, locality: PrefetchLocality) {
        prefetch_range_read(self.xs.as_ptr() as *const u8, self.xs.len() * 4, locality);
        prefetch_range_read(self.ys.as_ptr() as *const u8, self.ys.len() * 4, locality);
    }
}

impl Default for PointSoA {
    fn default() -> Self {
        Self::new()
    }
}

/// A struct-of-arrays container for Rect data
pub struct RectSoA {
    pub x0s: Vec<f32>,
    pub y0s: Vec<f32>,
    pub x1s: Vec<f32>,
    pub y1s: Vec<f32>,
}

impl RectSoA {
    /// Create empty SoA
    pub fn new() -> Self {
        Self {
            x0s: Vec::new(),
            y0s: Vec::new(),
            x1s: Vec::new(),
            y1s: Vec::new(),
        }
    }

    /// Create with capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            x0s: Vec::with_capacity(capacity),
            y0s: Vec::with_capacity(capacity),
            x1s: Vec::with_capacity(capacity),
            y1s: Vec::with_capacity(capacity),
        }
    }

    /// Number of rects
    #[inline]
    pub fn len(&self) -> usize {
        self.x0s.len()
    }

    /// Check if empty
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.x0s.is_empty()
    }

    /// Add a rect
    #[inline]
    pub fn push(&mut self, x0: f32, y0: f32, x1: f32, y1: f32) {
        self.x0s.push(x0);
        self.y0s.push(y0);
        self.x1s.push(x1);
        self.y1s.push(y1);
    }

    /// Get rect at index
    #[inline]
    pub fn get(&self, index: usize) -> Option<(f32, f32, f32, f32)> {
        self.x0s
            .get(index)
            .map(|&x0| (x0, self.y0s[index], self.x1s[index], self.y1s[index]))
    }

    /// Clear all rects
    #[inline]
    pub fn clear(&mut self) {
        self.x0s.clear();
        self.y0s.clear();
        self.x1s.clear();
        self.y1s.clear();
    }

    /// Compute widths (SIMD-friendly)
    pub fn widths(&self) -> Vec<f32> {
        self.x1s
            .iter()
            .zip(&self.x0s)
            .map(|(x1, x0)| x1 - x0)
            .collect()
    }

    /// Compute heights (SIMD-friendly)
    pub fn heights(&self) -> Vec<f32> {
        self.y1s
            .iter()
            .zip(&self.y0s)
            .map(|(y1, y0)| y1 - y0)
            .collect()
    }
}

impl Default for RectSoA {
    fn default() -> Self {
        Self::new()
    }
}

/// A struct-of-arrays container for Color data (CMYK)
pub struct ColorSoA {
    pub cs: Vec<f32>,
    pub ms: Vec<f32>,
    pub ys: Vec<f32>,
    pub ks: Vec<f32>,
    pub alphas: Vec<f32>,
}

impl ColorSoA {
    /// Create empty SoA
    pub fn new() -> Self {
        Self {
            cs: Vec::new(),
            ms: Vec::new(),
            ys: Vec::new(),
            ks: Vec::new(),
            alphas: Vec::new(),
        }
    }

    /// Create with capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            cs: Vec::with_capacity(capacity),
            ms: Vec::with_capacity(capacity),
            ys: Vec::with_capacity(capacity),
            ks: Vec::with_capacity(capacity),
            alphas: Vec::with_capacity(capacity),
        }
    }

    /// Number of colors
    #[inline]
    pub fn len(&self) -> usize {
        self.cs.len()
    }

    /// Check if empty
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.cs.is_empty()
    }

    /// Add a color
    #[inline]
    pub fn push(&mut self, c: f32, m: f32, y: f32, k: f32, alpha: f32) {
        self.cs.push(c);
        self.ms.push(m);
        self.ys.push(y);
        self.ks.push(k);
        self.alphas.push(alpha);
    }

    /// Clear all colors
    #[inline]
    pub fn clear(&mut self) {
        self.cs.clear();
        self.ms.clear();
        self.ys.clear();
        self.ks.clear();
        self.alphas.clear();
    }
}

impl Default for ColorSoA {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Statistics
// ============================================================================

/// Statistics for data locality operations
#[derive(Debug, Default)]
pub struct LocalityStats {
    pub page_allocs: AtomicU64,
    pub page_bytes: AtomicU64,
    pub prefetch_reads: AtomicU64,
    pub prefetch_writes: AtomicU64,
}

static LOCALITY_STATS: LazyLock<LocalityStats> = LazyLock::new(LocalityStats::default);

impl LocalityStats {
    /// Get current stats
    pub fn get() -> LocalityStatsSnapshot {
        LocalityStatsSnapshot {
            page_allocs: LOCALITY_STATS.page_allocs.load(Ordering::Relaxed),
            page_bytes: LOCALITY_STATS.page_bytes.load(Ordering::Relaxed),
            prefetch_reads: LOCALITY_STATS.prefetch_reads.load(Ordering::Relaxed),
            prefetch_writes: LOCALITY_STATS.prefetch_writes.load(Ordering::Relaxed),
        }
    }

    /// Reset stats
    pub fn reset() {
        LOCALITY_STATS.page_allocs.store(0, Ordering::Relaxed);
        LOCALITY_STATS.page_bytes.store(0, Ordering::Relaxed);
        LOCALITY_STATS.prefetch_reads.store(0, Ordering::Relaxed);
        LOCALITY_STATS.prefetch_writes.store(0, Ordering::Relaxed);
    }
}

/// Snapshot of locality stats
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct LocalityStatsSnapshot {
    pub page_allocs: u64,
    pub page_bytes: u64,
    pub prefetch_reads: u64,
    pub prefetch_writes: u64,
}

// ============================================================================
// Handle Store
// ============================================================================

pub static PAGE_BUFFERS: LazyLock<HandleStore<PageAlignedBuffer>> = LazyLock::new(HandleStore::new);
pub static POINT_SOAS: LazyLock<HandleStore<PointSoA>> = LazyLock::new(HandleStore::new);
pub static RECT_SOAS: LazyLock<HandleStore<RectSoA>> = LazyLock::new(HandleStore::new);

// ============================================================================
// FFI Functions
// ============================================================================

use super::Handle;

/// Create a new page-aligned buffer
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_page_aligned_buffer(_ctx: Handle, capacity: usize) -> Handle {
    LOCALITY_STATS.page_allocs.fetch_add(1, Ordering::Relaxed);

    match PageAlignedBuffer::new(capacity) {
        Some(buf) => {
            LOCALITY_STATS
                .page_bytes
                .fetch_add(buf.capacity() as u64, Ordering::Relaxed);
            PAGE_BUFFERS.insert(buf)
        }
        None => 0,
    }
}

/// Drop a page-aligned buffer
#[unsafe(no_mangle)]
pub extern "C" fn fz_drop_page_aligned_buffer(_ctx: Handle, buf: Handle) {
    let _ = PAGE_BUFFERS.remove(buf);
}

/// Get page-aligned buffer length
#[unsafe(no_mangle)]
pub extern "C" fn fz_page_buffer_len(_ctx: Handle, buf: Handle) -> usize {
    PAGE_BUFFERS
        .get(buf)
        .map(|b| b.lock().unwrap().len())
        .unwrap_or(0)
}

/// Get page-aligned buffer capacity
#[unsafe(no_mangle)]
pub extern "C" fn fz_page_buffer_capacity(_ctx: Handle, buf: Handle) -> usize {
    PAGE_BUFFERS
        .get(buf)
        .map(|b| b.lock().unwrap().capacity())
        .unwrap_or(0)
}

/// Write to page-aligned buffer
#[unsafe(no_mangle)]
pub extern "C" fn fz_page_buffer_write(
    _ctx: Handle,
    buf: Handle,
    data: *const u8,
    len: usize,
) -> c_int {
    if data.is_null() {
        return -1;
    }

    let Some(b) = PAGE_BUFFERS.get(buf) else {
        return -1;
    };

    let data_slice = unsafe { std::slice::from_raw_parts(data, len) };
    let mut guard = b.lock().unwrap();

    if guard.extend_from_slice(data_slice) {
        0
    } else {
        -1
    }
}

/// Read from page-aligned buffer
#[unsafe(no_mangle)]
pub extern "C" fn fz_page_buffer_read(
    _ctx: Handle,
    buf: Handle,
    offset: usize,
    dst: *mut u8,
    len: usize,
) -> c_int {
    if dst.is_null() {
        return -1;
    }

    let Some(b) = PAGE_BUFFERS.get(buf) else {
        return -1;
    };

    let guard = b.lock().unwrap();
    let slice = guard.as_slice();

    if offset + len > slice.len() {
        return -1;
    }

    unsafe {
        std::ptr::copy_nonoverlapping(slice.as_ptr().add(offset), dst, len);
    }

    0
}

/// Prefetch page buffer for read
#[unsafe(no_mangle)]
pub extern "C" fn fz_page_buffer_prefetch_read(_ctx: Handle, buf: Handle, locality: c_int) {
    LOCALITY_STATS
        .prefetch_reads
        .fetch_add(1, Ordering::Relaxed);

    let locality = match locality {
        0 => PrefetchLocality::NonTemporal,
        1 => PrefetchLocality::Low,
        2 => PrefetchLocality::Medium,
        _ => PrefetchLocality::High,
    };

    if let Some(b) = PAGE_BUFFERS.get(buf) {
        b.lock().unwrap().prefetch_read(locality);
    }
}

/// Prefetch page buffer for write
#[unsafe(no_mangle)]
pub extern "C" fn fz_page_buffer_prefetch_write(_ctx: Handle, buf: Handle, locality: c_int) {
    LOCALITY_STATS
        .prefetch_writes
        .fetch_add(1, Ordering::Relaxed);

    let locality = match locality {
        0 => PrefetchLocality::NonTemporal,
        1 => PrefetchLocality::Low,
        2 => PrefetchLocality::Medium,
        _ => PrefetchLocality::High,
    };

    if let Some(b) = PAGE_BUFFERS.get(buf) {
        b.lock().unwrap().prefetch_write(locality);
    }
}

/// Get data locality stats
#[unsafe(no_mangle)]
pub extern "C" fn fz_locality_stats(_ctx: Handle) -> LocalityStatsSnapshot {
    LocalityStats::get()
}

/// Reset data locality stats
#[unsafe(no_mangle)]
pub extern "C" fn fz_locality_stats_reset(_ctx: Handle) {
    LocalityStats::reset();
}

// --- Point SoA FFI ---

/// Create a new Point SoA
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_point_soa(_ctx: Handle, capacity: usize) -> Handle {
    POINT_SOAS.insert(PointSoA::with_capacity(capacity))
}

/// Drop Point SoA
#[unsafe(no_mangle)]
pub extern "C" fn fz_drop_point_soa(_ctx: Handle, soa: Handle) {
    let _ = POINT_SOAS.remove(soa);
}

/// Push point to SoA
#[unsafe(no_mangle)]
pub extern "C" fn fz_point_soa_push(_ctx: Handle, soa: Handle, x: f32, y: f32) {
    if let Some(s) = POINT_SOAS.get(soa) {
        s.lock().unwrap().push(x, y);
    }
}

/// Get point count
#[unsafe(no_mangle)]
pub extern "C" fn fz_point_soa_len(_ctx: Handle, soa: Handle) -> usize {
    POINT_SOAS
        .get(soa)
        .map(|s| s.lock().unwrap().len())
        .unwrap_or(0)
}

/// Transform all points
#[unsafe(no_mangle)]
pub extern "C" fn fz_point_soa_transform(
    _ctx: Handle,
    soa: Handle,
    a: f32,
    b: f32,
    c: f32,
    d: f32,
    e: f32,
    f: f32,
) {
    if let Some(s) = POINT_SOAS.get(soa) {
        s.lock().unwrap().transform(a, b, c, d, e, f);
    }
}

// --- Rect SoA FFI ---

/// Create a new Rect SoA
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_rect_soa(_ctx: Handle, capacity: usize) -> Handle {
    RECT_SOAS.insert(RectSoA::with_capacity(capacity))
}

/// Drop Rect SoA
#[unsafe(no_mangle)]
pub extern "C" fn fz_drop_rect_soa(_ctx: Handle, soa: Handle) {
    let _ = RECT_SOAS.remove(soa);
}

/// Push rect to SoA
#[unsafe(no_mangle)]
pub extern "C" fn fz_rect_soa_push(_ctx: Handle, soa: Handle, x0: f32, y0: f32, x1: f32, y1: f32) {
    if let Some(s) = RECT_SOAS.get(soa) {
        s.lock().unwrap().push(x0, y0, x1, y1);
    }
}

/// Get rect count
#[unsafe(no_mangle)]
pub extern "C" fn fz_rect_soa_len(_ctx: Handle, soa: Handle) -> usize {
    RECT_SOAS
        .get(soa)
        .map(|s| s.lock().unwrap().len())
        .unwrap_or(0)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_page_aligned_buffer_new() {
        let buf = PageAlignedBuffer::new(1024).unwrap();
        assert!(buf.is_page_aligned());
        assert_eq!(buf.len(), 0);
        assert!(buf.capacity() >= 1024);
        assert_eq!(buf.capacity() % PAGE_SIZE, 0);
    }

    #[test]
    fn test_page_aligned_buffer_write() {
        let mut buf = PageAlignedBuffer::new(PAGE_SIZE).unwrap();
        let data = b"Hello, World!";

        assert!(buf.extend_from_slice(data));
        assert_eq!(buf.len(), data.len());
        assert_eq!(buf.as_slice(), data);
    }

    #[test]
    fn test_page_aligned_buffer_from_slice() {
        let data = vec![1u8; 5000];
        let buf = PageAlignedBuffer::from_slice(&data).unwrap();

        assert_eq!(buf.len(), 5000);
        assert!(buf.is_page_aligned());
        assert!(buf.pages_used() >= 2);
    }

    #[test]
    fn test_page_aligned_buffer_capacity_overflow() {
        let mut buf = PageAlignedBuffer::new(PAGE_SIZE).unwrap();
        let data = vec![0u8; PAGE_SIZE + 1];

        // Should fail - too much data (exceeds page capacity)
        assert!(!buf.extend_from_slice(&data));
    }

    #[test]
    fn test_point_soa() {
        let mut soa = PointSoA::new();
        soa.push(1.0, 2.0);
        soa.push(3.0, 4.0);

        assert_eq!(soa.len(), 2);
        assert_eq!(soa.get(0), Some((1.0, 2.0)));
        assert_eq!(soa.get(1), Some((3.0, 4.0)));
    }

    #[test]
    fn test_point_soa_transform() {
        let mut soa = PointSoA::new();
        soa.push(1.0, 0.0);
        soa.push(0.0, 1.0);

        // Scale by 2
        soa.transform(2.0, 0.0, 0.0, 2.0, 0.0, 0.0);

        assert_eq!(soa.get(0), Some((2.0, 0.0)));
        assert_eq!(soa.get(1), Some((0.0, 2.0)));
    }

    #[test]
    fn test_rect_soa() {
        let mut soa = RectSoA::new();
        soa.push(0.0, 0.0, 100.0, 200.0);
        soa.push(10.0, 20.0, 50.0, 80.0);

        assert_eq!(soa.len(), 2);

        let widths = soa.widths();
        assert_eq!(widths, vec![100.0, 40.0]);

        let heights = soa.heights();
        assert_eq!(heights, vec![200.0, 60.0]);
    }

    #[test]
    fn test_color_soa() {
        let mut soa = ColorSoA::new();
        soa.push(0.0, 1.0, 0.5, 0.2, 1.0);

        assert_eq!(soa.len(), 1);
        assert_eq!(soa.cs[0], 0.0);
        assert_eq!(soa.ms[0], 1.0);
        assert_eq!(soa.ys[0], 0.5);
        assert_eq!(soa.ks[0], 0.2);
        assert_eq!(soa.alphas[0], 1.0);
    }

    #[test]
    fn test_ffi_page_buffer() {
        let handle = fz_new_page_aligned_buffer(0, 4096);
        assert_ne!(handle, 0);

        let data = b"test data";
        let result = fz_page_buffer_write(0, handle, data.as_ptr(), data.len());
        assert_eq!(result, 0);

        assert_eq!(fz_page_buffer_len(0, handle), 9);
        assert!(fz_page_buffer_capacity(0, handle) >= 4096);

        fz_drop_page_aligned_buffer(0, handle);
    }

    #[test]
    fn test_ffi_point_soa() {
        let handle = fz_new_point_soa(0, 10);
        assert_ne!(handle, 0);

        fz_point_soa_push(0, handle, 1.0, 2.0);
        fz_point_soa_push(0, handle, 3.0, 4.0);
        assert_eq!(fz_point_soa_len(0, handle), 2);

        // Transform: scale by 2
        fz_point_soa_transform(0, handle, 2.0, 0.0, 0.0, 2.0, 0.0, 0.0);

        fz_drop_point_soa(0, handle);
    }

    #[test]
    fn test_ffi_rect_soa() {
        let handle = fz_new_rect_soa(0, 10);
        assert_ne!(handle, 0);

        fz_rect_soa_push(0, handle, 0.0, 0.0, 100.0, 50.0);
        assert_eq!(fz_rect_soa_len(0, handle), 1);

        fz_drop_rect_soa(0, handle);
    }

    #[test]
    fn test_ffi_locality_stats() {
        fz_locality_stats_reset(0);

        let handle = fz_new_page_aligned_buffer(0, 1024);
        fz_page_buffer_prefetch_read(0, handle, 3);

        let stats = fz_locality_stats(0);
        assert!(stats.page_allocs >= 1);
        assert!(stats.prefetch_reads >= 1);

        fz_drop_page_aligned_buffer(0, handle);
    }

    #[test]
    fn test_prefetch_locality_enum() {
        assert_eq!(PrefetchLocality::NonTemporal as i32, 0);
        assert_eq!(PrefetchLocality::Low as i32, 1);
        assert_eq!(PrefetchLocality::Medium as i32, 2);
        assert_eq!(PrefetchLocality::High as i32, 3);
    }
}
