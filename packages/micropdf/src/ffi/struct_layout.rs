//! Structure Layout Optimization
//!
//! Utilities for cache-efficient struct layouts:
//! - `CacheAligned<T>`: Wrapper ensuring cache line alignment
//! - `Padded<T, N>`: Explicit padding for false sharing avoidance
//! - Layout analysis utilities and compile-time assertions
//!
//! Benefits:
//! - Eliminates false sharing in concurrent access
//! - Improves cache utilization for hot structs
//! - Documents and enforces layout decisions

use std::mem::{align_of, size_of};
use std::ops::{Deref, DerefMut};
use std::sync::LazyLock;

use super::HandleStore;

// ============================================================================
// Cache Line Constants
// ============================================================================

/// Cache line size for x86_64 and ARM64 (typically 64 bytes)
pub const CACHE_LINE_SIZE: usize = 64;

/// Double cache line for prefetch-friendly layouts
pub const DOUBLE_CACHE_LINE: usize = 128;

/// Typical page size
pub const PAGE_SIZE: usize = 4096;

// ============================================================================
// Cache-Aligned Wrapper
// ============================================================================

/// Wrapper that aligns the inner value to a cache line boundary.
///
/// Use this for frequently accessed data that needs to avoid false sharing.
///
/// # Example
/// ```ignore
/// // Without alignment, two atomics might share a cache line
/// struct BadCounters {
///     counter_a: AtomicU64,  // Might share cache line with counter_b
///     counter_b: AtomicU64,
/// }
///
/// // With alignment, each counter gets its own cache line
/// struct GoodCounters {
///     counter_a: CacheAligned<AtomicU64>,
///     counter_b: CacheAligned<AtomicU64>,
/// }
/// ```
#[repr(C, align(64))]
#[derive(Debug)]
pub struct CacheAligned<T> {
    value: T,
}

impl<T> CacheAligned<T> {
    /// Create a new cache-aligned value
    #[inline]
    pub const fn new(value: T) -> Self {
        Self { value }
    }

    /// Get the inner value
    #[inline]
    pub fn into_inner(self) -> T {
        self.value
    }
}

impl<T: Default> Default for CacheAligned<T> {
    fn default() -> Self {
        Self::new(T::default())
    }
}

impl<T: Clone> Clone for CacheAligned<T> {
    fn clone(&self) -> Self {
        Self::new(self.value.clone())
    }
}

impl<T: Copy> Copy for CacheAligned<T> {}

impl<T> Deref for CacheAligned<T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T> DerefMut for CacheAligned<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}

// ============================================================================
// Padded Wrapper
// ============================================================================

/// Wrapper that pads the inner value to a specific size.
///
/// Useful for ensuring structs don't share cache lines.
#[repr(C)]
#[derive(Debug)]
pub struct Padded<T, const N: usize> {
    value: T,
    _pad: [u8; N],
}

impl<T, const N: usize> Padded<T, N> {
    /// Create a new padded value
    #[inline]
    pub fn new(value: T) -> Self {
        Self {
            value,
            _pad: [0; N],
        }
    }

    /// Get the inner value
    #[inline]
    pub fn into_inner(self) -> T {
        self.value
    }
}

impl<T: Default, const N: usize> Default for Padded<T, N> {
    fn default() -> Self {
        Self::new(T::default())
    }
}

impl<T: Clone, const N: usize> Clone for Padded<T, N> {
    fn clone(&self) -> Self {
        Self::new(self.value.clone())
    }
}

impl<T: Copy, const N: usize> Copy for Padded<T, N> {}

impl<T, const N: usize> Deref for Padded<T, N> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T, const N: usize> DerefMut for Padded<T, N> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}

// ============================================================================
// Layout Info
// ============================================================================

/// Information about a struct's memory layout
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct LayoutInfo {
    /// Size in bytes
    pub size: usize,
    /// Alignment in bytes
    pub align: usize,
    /// Number of cache lines spanned
    pub cache_lines: usize,
    /// Padding bytes (estimated)
    pub padding: usize,
}

impl LayoutInfo {
    /// Get layout info for a type
    #[inline]
    pub fn of<T>() -> Self {
        let size = size_of::<T>();
        let align = align_of::<T>();
        let cache_lines = (size + CACHE_LINE_SIZE - 1) / CACHE_LINE_SIZE;

        Self {
            size,
            align,
            cache_lines,
            padding: 0, // Can't determine without field info
        }
    }

    /// Check if size is a power of two
    #[inline]
    pub fn is_power_of_two_size(&self) -> bool {
        self.size.is_power_of_two()
    }

    /// Check if naturally aligned to its size
    #[inline]
    pub fn is_naturally_aligned(&self) -> bool {
        self.align >= self.size || self.size % self.align == 0
    }

    /// Check if cache-line aligned
    #[inline]
    pub fn is_cache_aligned(&self) -> bool {
        self.align >= CACHE_LINE_SIZE
    }

    /// Check if fits in one cache line
    #[inline]
    pub fn fits_in_cache_line(&self) -> bool {
        self.size <= CACHE_LINE_SIZE
    }
}

// ============================================================================
// Layout Analysis Utilities
// ============================================================================

/// Analyze and print layout information for debugging
pub fn analyze_layout<T>(name: &str) -> LayoutInfo {
    let info = LayoutInfo::of::<T>();
    eprintln!(
        "Layout of {}: size={}, align={}, cache_lines={}, cache_aligned={}, fits_in_line={}",
        name,
        info.size,
        info.align,
        info.cache_lines,
        info.is_cache_aligned(),
        info.fits_in_cache_line()
    );
    info
}

/// Calculate padding needed for cache line alignment
#[inline]
pub const fn cache_padding(size: usize) -> usize {
    let remainder = size % CACHE_LINE_SIZE;
    if remainder == 0 {
        0
    } else {
        CACHE_LINE_SIZE - remainder
    }
}

/// Round up to next cache line boundary
#[inline]
pub const fn round_to_cache_line(size: usize) -> usize {
    (size + CACHE_LINE_SIZE - 1) & !(CACHE_LINE_SIZE - 1)
}

/// Round up to next page boundary
#[inline]
pub const fn round_to_page(size: usize) -> usize {
    (size + PAGE_SIZE - 1) & !(PAGE_SIZE - 1)
}

// ============================================================================
// Compile-Time Assertions
// ============================================================================

/// Macro to assert struct layout at compile time
#[macro_export]
macro_rules! assert_layout {
    ($ty:ty, size = $size:expr) => {
        const _: () = assert!(
            std::mem::size_of::<$ty>() == $size,
            concat!(
                "Size assertion failed for ",
                stringify!($ty),
                ": expected ",
                stringify!($size)
            )
        );
    };
    ($ty:ty, align = $align:expr) => {
        const _: () = assert!(
            std::mem::align_of::<$ty>() == $align,
            concat!(
                "Alignment assertion failed for ",
                stringify!($ty),
                ": expected ",
                stringify!($align)
            )
        );
    };
    ($ty:ty, size = $size:expr, align = $align:expr) => {
        $crate::assert_layout!($ty, size = $size);
        $crate::assert_layout!($ty, align = $align);
    };
}

/// Macro to assert struct fits in N cache lines
#[macro_export]
macro_rules! assert_cache_lines {
    ($ty:ty, $lines:expr) => {
        const _: () = assert!(
            std::mem::size_of::<$ty>() <= $lines * $crate::ffi::struct_layout::CACHE_LINE_SIZE,
            concat!(
                "Cache line assertion failed for ",
                stringify!($ty),
                ": exceeds ",
                stringify!($lines),
                " cache lines"
            )
        );
    };
}

// ============================================================================
// Optimized Hot Structs
// ============================================================================

/// Cache-optimized Point (8 bytes, fits many per cache line)
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PackedPoint {
    pub x: f32,
    pub y: f32,
}

// 8 points fit in one 64-byte cache line
const _: () = assert!(size_of::<PackedPoint>() == 8);
const _: () = assert!(64 / size_of::<PackedPoint>() == 8);

/// Cache-optimized Rect (16 bytes, 4 per cache line)
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PackedRect {
    pub x0: f32,
    pub y0: f32,
    pub x1: f32,
    pub y1: f32,
}

const _: () = assert!(size_of::<PackedRect>() == 16);
const _: () = assert!(64 / size_of::<PackedRect>() == 4);

/// Cache-optimized Matrix (24 bytes)
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PackedMatrix {
    pub a: f32,
    pub b: f32,
    pub c: f32,
    pub d: f32,
    pub e: f32,
    pub f: f32,
}

const _: () = assert!(size_of::<PackedMatrix>() == 24);

impl Default for PackedMatrix {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl PackedMatrix {
    pub const IDENTITY: Self = Self {
        a: 1.0,
        b: 0.0,
        c: 0.0,
        d: 1.0,
        e: 0.0,
        f: 0.0,
    };

    #[inline]
    pub fn new(a: f32, b: f32, c: f32, d: f32, e: f32, f: f32) -> Self {
        Self { a, b, c, d, e, f }
    }

    #[inline]
    pub fn transform_point(&self, x: f32, y: f32) -> (f32, f32) {
        (
            x * self.a + y * self.c + self.e,
            x * self.b + y * self.d + self.f,
        )
    }

    #[inline]
    pub fn concat(&self, other: &Self) -> Self {
        Self {
            a: self.a * other.a + self.b * other.c,
            b: self.a * other.b + self.b * other.d,
            c: self.c * other.a + self.d * other.c,
            d: self.c * other.b + self.d * other.d,
            e: self.e * other.a + self.f * other.c + other.e,
            f: self.e * other.b + self.f * other.d + other.f,
        }
    }
}

/// Cache-optimized Quad (32 bytes, 2 per cache line)
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PackedQuad {
    pub ul: PackedPoint, // Upper-left
    pub ur: PackedPoint, // Upper-right
    pub ll: PackedPoint, // Lower-left
    pub lr: PackedPoint, // Lower-right
}

const _: () = assert!(size_of::<PackedQuad>() == 32);
const _: () = assert!(64 / size_of::<PackedQuad>() == 2);

impl PackedQuad {
    #[inline]
    pub fn from_rect(r: &PackedRect) -> Self {
        Self {
            ul: PackedPoint { x: r.x0, y: r.y0 },
            ur: PackedPoint { x: r.x1, y: r.y0 },
            ll: PackedPoint { x: r.x0, y: r.y1 },
            lr: PackedPoint { x: r.x1, y: r.y1 },
        }
    }

    #[inline]
    pub fn bounds(&self) -> PackedRect {
        PackedRect {
            x0: self.ul.x.min(self.ur.x).min(self.ll.x).min(self.lr.x),
            y0: self.ul.y.min(self.ur.y).min(self.ll.y).min(self.lr.y),
            x1: self.ul.x.max(self.ur.x).max(self.ll.x).max(self.lr.x),
            y1: self.ul.y.max(self.ur.y).max(self.ll.y).max(self.lr.y),
        }
    }
}

/// Cache-line sized color (fits CMYK + alpha + metadata in one line)
#[repr(C, align(64))]
#[derive(Debug, Clone, Copy)]
pub struct CacheLineColor {
    pub c: f32,          // 4
    pub m: f32,          // 8
    pub y: f32,          // 12
    pub k: f32,          // 16
    pub alpha: f32,      // 20
    pub colorspace: u32, // 24
    _pad: [u8; 40],      // 64 total
}

impl Default for CacheLineColor {
    fn default() -> Self {
        Self {
            c: 0.0,
            m: 0.0,
            y: 0.0,
            k: 0.0,
            alpha: 1.0,
            colorspace: 0,
            _pad: [0; 40],
        }
    }
}

const _: () = assert!(size_of::<CacheLineColor>() == 64);
const _: () = assert!(align_of::<CacheLineColor>() == 64);

// ============================================================================
// FFI Structures
// ============================================================================

/// FFI-safe layout info
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct FfiLayoutInfo {
    pub size: usize,
    pub align: usize,
    pub cache_lines: usize,
    pub is_cache_aligned: i32,
    pub fits_in_cache_line: i32,
}

impl From<LayoutInfo> for FfiLayoutInfo {
    fn from(info: LayoutInfo) -> Self {
        Self {
            size: info.size,
            align: info.align,
            cache_lines: info.cache_lines,
            is_cache_aligned: info.is_cache_aligned() as i32,
            fits_in_cache_line: info.fits_in_cache_line() as i32,
        }
    }
}

// ============================================================================
// Handle Store
// ============================================================================

pub static LAYOUT_INFOS: LazyLock<HandleStore<LayoutInfo>> = LazyLock::new(HandleStore::new);

// ============================================================================
// FFI Functions
// ============================================================================

use std::ffi::c_int;

/// Get cache line size constant
#[unsafe(no_mangle)]
pub extern "C" fn fz_cache_line_size() -> usize {
    CACHE_LINE_SIZE
}

/// Get page size constant
#[unsafe(no_mangle)]
pub extern "C" fn fz_page_size() -> usize {
    PAGE_SIZE
}

/// Calculate padding needed for cache alignment
#[unsafe(no_mangle)]
pub extern "C" fn fz_cache_padding(size: usize) -> usize {
    cache_padding(size)
}

/// Round size to cache line boundary
#[unsafe(no_mangle)]
pub extern "C" fn fz_round_to_cache_line(size: usize) -> usize {
    round_to_cache_line(size)
}

/// Round size to page boundary
#[unsafe(no_mangle)]
pub extern "C" fn fz_round_to_page(size: usize) -> usize {
    round_to_page(size)
}

/// Get layout info for Point
#[unsafe(no_mangle)]
pub extern "C" fn fz_layout_point() -> FfiLayoutInfo {
    LayoutInfo::of::<PackedPoint>().into()
}

/// Get layout info for Rect
#[unsafe(no_mangle)]
pub extern "C" fn fz_layout_rect() -> FfiLayoutInfo {
    LayoutInfo::of::<PackedRect>().into()
}

/// Get layout info for Matrix
#[unsafe(no_mangle)]
pub extern "C" fn fz_layout_matrix() -> FfiLayoutInfo {
    LayoutInfo::of::<PackedMatrix>().into()
}

/// Get layout info for Quad
#[unsafe(no_mangle)]
pub extern "C" fn fz_layout_quad() -> FfiLayoutInfo {
    LayoutInfo::of::<PackedQuad>().into()
}

/// Check if a size fits in N cache lines
#[unsafe(no_mangle)]
pub extern "C" fn fz_fits_in_cache_lines(size: usize, lines: usize) -> c_int {
    (size <= lines * CACHE_LINE_SIZE) as c_int
}

/// Check if a pointer is cache-aligned
#[unsafe(no_mangle)]
pub extern "C" fn fz_is_cache_aligned(ptr: *const std::ffi::c_void) -> c_int {
    ((ptr as usize) % CACHE_LINE_SIZE == 0) as c_int
}

/// Check if a pointer is page-aligned
#[unsafe(no_mangle)]
pub extern "C" fn fz_is_page_aligned(ptr: *const std::ffi::c_void) -> c_int {
    ((ptr as usize) % PAGE_SIZE == 0) as c_int
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_aligned() {
        let aligned: CacheAligned<u64> = CacheAligned::new(42);
        assert_eq!(*aligned, 42);
        assert_eq!(align_of::<CacheAligned<u64>>(), 64);

        // Verify actual alignment
        let ptr = &aligned as *const _ as usize;
        assert_eq!(ptr % 64, 0, "CacheAligned should be 64-byte aligned");
    }

    #[test]
    fn test_padded() {
        let padded: Padded<u32, 60> = Padded::new(123);
        assert_eq!(*padded, 123);
        assert_eq!(size_of::<Padded<u32, 60>>(), 64); // 4 + 60
    }

    #[test]
    fn test_layout_info() {
        let info = LayoutInfo::of::<u64>();
        assert_eq!(info.size, 8);
        assert_eq!(info.align, 8);
        assert!(info.fits_in_cache_line());
    }

    #[test]
    fn test_cache_padding() {
        assert_eq!(cache_padding(0), 0);
        assert_eq!(cache_padding(1), 63);
        assert_eq!(cache_padding(32), 32);
        assert_eq!(cache_padding(64), 0);
        assert_eq!(cache_padding(65), 63);
    }

    #[test]
    fn test_round_to_cache_line() {
        assert_eq!(round_to_cache_line(0), 0);
        assert_eq!(round_to_cache_line(1), 64);
        assert_eq!(round_to_cache_line(64), 64);
        assert_eq!(round_to_cache_line(65), 128);
        assert_eq!(round_to_cache_line(100), 128);
    }

    #[test]
    fn test_round_to_page() {
        assert_eq!(round_to_page(0), 0);
        assert_eq!(round_to_page(1), 4096);
        assert_eq!(round_to_page(4096), 4096);
        assert_eq!(round_to_page(4097), 8192);
    }

    #[test]
    fn test_packed_point() {
        let p = PackedPoint { x: 1.0, y: 2.0 };
        assert_eq!(size_of_val(&p), 8);
    }

    #[test]
    fn test_packed_rect() {
        let r = PackedRect {
            x0: 0.0,
            y0: 0.0,
            x1: 100.0,
            y1: 100.0,
        };
        assert_eq!(size_of_val(&r), 16);
    }

    #[test]
    fn test_packed_matrix() {
        let m = PackedMatrix::IDENTITY;
        assert_eq!(size_of_val(&m), 24);

        let (x, y) = m.transform_point(10.0, 20.0);
        assert_eq!(x, 10.0);
        assert_eq!(y, 20.0);
    }

    #[test]
    fn test_packed_matrix_concat() {
        let scale = PackedMatrix::new(2.0, 0.0, 0.0, 2.0, 0.0, 0.0);
        let translate = PackedMatrix::new(1.0, 0.0, 0.0, 1.0, 10.0, 20.0);

        let combined = scale.concat(&translate);
        let (x, y) = combined.transform_point(5.0, 5.0);

        // Scale by 2, then translate by (10, 20)
        assert_eq!(x, 20.0); // 5*2 + 10
        assert_eq!(y, 30.0); // 5*2 + 20
    }

    #[test]
    fn test_packed_quad() {
        let r = PackedRect {
            x0: 0.0,
            y0: 0.0,
            x1: 10.0,
            y1: 20.0,
        };
        let q = PackedQuad::from_rect(&r);

        assert_eq!(q.ul.x, 0.0);
        assert_eq!(q.ul.y, 0.0);
        assert_eq!(q.lr.x, 10.0);
        assert_eq!(q.lr.y, 20.0);

        let bounds = q.bounds();
        assert_eq!(bounds.x0, 0.0);
        assert_eq!(bounds.y0, 0.0);
        assert_eq!(bounds.x1, 10.0);
        assert_eq!(bounds.y1, 20.0);
    }

    #[test]
    fn test_cache_line_color() {
        let c = CacheLineColor::default();
        assert_eq!(size_of_val(&c), 64);
        assert_eq!(align_of_val(&c), 64);
    }

    #[test]
    fn test_ffi_cache_line_size() {
        assert_eq!(fz_cache_line_size(), 64);
    }

    #[test]
    fn test_ffi_layout_point() {
        let info = fz_layout_point();
        assert_eq!(info.size, 8);
        assert_eq!(info.fits_in_cache_line, 1);
    }

    #[test]
    fn test_ffi_layout_rect() {
        let info = fz_layout_rect();
        assert_eq!(info.size, 16);
        assert_eq!(info.fits_in_cache_line, 1);
    }

    #[test]
    fn test_ffi_layout_matrix() {
        let info = fz_layout_matrix();
        assert_eq!(info.size, 24);
        assert_eq!(info.fits_in_cache_line, 1);
    }

    #[test]
    fn test_ffi_is_cache_aligned() {
        let aligned: CacheAligned<u64> = CacheAligned::new(0);
        let ptr = &*aligned as *const u64 as *const std::ffi::c_void;
        assert_eq!(fz_is_cache_aligned(ptr), 1);
    }

    #[test]
    fn test_ffi_fits_in_cache_lines() {
        assert_eq!(fz_fits_in_cache_lines(64, 1), 1);
        assert_eq!(fz_fits_in_cache_lines(65, 1), 0);
        assert_eq!(fz_fits_in_cache_lines(128, 2), 1);
        assert_eq!(fz_fits_in_cache_lines(129, 2), 0);
    }
}
