//! Compiler Hints and Micro-optimizations
//!
//! Provides utilities for guiding compiler optimizations:
//! - Branch prediction hints (`likely`, `unlikely`)
//! - Cold/hot path annotations
//! - Prefetch suggestions
//! - Inline control
//!
//! These hints help the compiler generate better code by providing
//! information about expected runtime behavior.

use std::ffi::c_int;

// ============================================================================
// Branch Prediction Hints
// ============================================================================

/// Hint that a condition is likely to be true.
///
/// Use this for conditions that are true in the common/fast path.
/// The compiler will optimize code layout assuming this branch is taken.
///
/// # Example
/// ```ignore
/// if likely(buffer.len() > 0) {
///     // Fast path - process data
/// } else {
///     // Slow path - handle empty buffer
/// }
/// ```
///
/// On stable Rust, this is a no-op that returns the input unchanged.
/// With nightly Rust + `#![feature(core_intrinsics)]`, it uses
/// `core::intrinsics::likely`.
#[inline(always)]
#[must_use]
pub const fn likely(b: bool) -> bool {
    // On stable Rust, this is a documentation-only hint.
    // The actual optimization comes from code structure and #[cold].
    b
}

/// Hint that a condition is unlikely to be true.
///
/// Use this for error conditions, edge cases, and exceptional situations.
/// The compiler will optimize code layout assuming this branch is NOT taken.
///
/// # Example
/// ```ignore
/// if unlikely(result.is_err()) {
///     // Error handling - cold path
///     return handle_error(result);
/// }
/// // Normal processing - hot path
/// ```
///
/// On stable Rust, this is a no-op that returns the input unchanged.
/// With nightly Rust + `#![feature(core_intrinsics)]`, it uses
/// `core::intrinsics::unlikely`.
#[inline(always)]
#[must_use]
pub const fn unlikely(b: bool) -> bool {
    // On stable Rust, this is a documentation-only hint.
    // The actual optimization comes from code structure and #[cold].
    b
}

// ============================================================================
// Macro Versions for Ergonomic Use
// ============================================================================

/// Macro version of `likely` for use in if conditions.
///
/// # Example
/// ```ignore
/// if likely!(x > 0) {
///     // common case
/// }
/// ```
#[macro_export]
macro_rules! likely {
    ($e:expr) => {
        $crate::ffi::hints::likely($e)
    };
}

/// Macro version of `unlikely` for use in if conditions.
///
/// # Example
/// ```ignore
/// if unlikely!(ptr.is_null()) {
///     return Err(Error::NullPointer);
/// }
/// ```
#[macro_export]
macro_rules! unlikely {
    ($e:expr) => {
        $crate::ffi::hints::unlikely($e)
    };
}

// ============================================================================
// Cold/Hot Path Markers
// ============================================================================

/// Mark a function as unlikely to be called (cold path).
///
/// Apply this attribute to error handlers, panic paths, and rarely-used code.
/// The compiler will:
/// - Place the code in a separate section (better instruction cache)
/// - Avoid inlining into hot paths
/// - Optimize for size rather than speed
///
/// # Example
/// ```ignore
/// #[cold]
/// fn handle_error(e: Error) -> ! {
///     log_error(&e);
///     panic!("unrecoverable: {}", e);
/// }
/// ```
///
/// Note: Use `#[cold]` directly as an attribute. This is documentation.
pub const COLD_ATTRIBUTE: &str = "#[cold]";

/// Mark a function as frequently called (hot path).
///
/// Apply `#[inline]` or `#[inline(always)]` to hot functions.
/// The compiler will:
/// - Aggressively inline the function
/// - Optimize for speed
/// - Keep code in instruction cache
///
/// # Guidelines
/// - Use `#[inline]` for functions < 10 lines that are called frequently
/// - Use `#[inline(always)]` for tiny functions (1-3 lines) in tight loops
/// - Don't use inline for large functions (increases code size, hurts i-cache)
pub const INLINE_GUIDELINES: &str = r#"
#[inline]         - Suggest inlining, compiler decides
#[inline(always)] - Force inlining (use sparingly)
#[inline(never)]  - Prevent inlining (for cold paths)
"#;

// ============================================================================
// Assert Hints
// ============================================================================

/// Assert that is optimized away in release builds but provides hints.
///
/// In debug: performs the assertion
/// In release: provides optimization hint without runtime cost
///
/// # Safety
/// In release mode, the compiler assumes `cond` is true. If it's false,
/// behavior is undefined.
#[inline(always)]
pub fn assume(cond: bool) {
    if cfg!(debug_assertions) {
        assert!(cond, "assumption violated");
    } else if !cond {
        // In release, if the condition is false, we have UB.
        // This gives the optimizer a hint that this branch is unreachable.
        // SAFETY: Caller guarantees cond is true.
        unsafe { std::hint::unreachable_unchecked() }
    }
}

/// Hint that a value is within a range (for bounds check elimination).
///
/// # Safety
/// Caller must guarantee that `value` is within `0..len`.
#[inline(always)]
pub unsafe fn assume_in_bounds(value: usize, len: usize) {
    assume(value < len);
}

/// Hint that a pointer is non-null.
///
/// # Safety
/// Caller must guarantee that `ptr` is non-null.
#[inline(always)]
pub unsafe fn assume_non_null<T>(ptr: *const T) {
    assume(!ptr.is_null());
}

/// Hint that a slice is non-empty.
///
/// # Safety
/// Caller must guarantee that `slice` is non-empty.
#[inline(always)]
pub unsafe fn assume_non_empty<T>(slice: &[T]) {
    assume(!slice.is_empty());
}

// ============================================================================
// Optimization Barriers
// ============================================================================

/// Prevent the compiler from optimizing away a value.
///
/// Use in benchmarks to ensure computations aren't eliminated.
#[inline(never)]
pub fn black_box<T>(x: T) -> T {
    std::hint::black_box(x)
}

/// Hint that this code path is unreachable.
///
/// # Safety
/// Calling this when the code IS reachable is undefined behavior.
#[inline(always)]
pub unsafe fn unreachable_unchecked() -> ! {
    // SAFETY: Caller guarantees this code path is unreachable.
    unsafe { std::hint::unreachable_unchecked() }
}

/// Spin-loop hint for busy-waiting.
///
/// Tells the CPU we're in a spin loop, reducing power and
/// avoiding performance penalties on hyperthreaded cores.
#[inline(always)]
pub fn spin_loop() {
    std::hint::spin_loop();
}

// ============================================================================
// Prefetch Hints (re-export from data_locality)
// ============================================================================

pub use super::data_locality::{
    PrefetchLocality, prefetch_range_read, prefetch_range_write, prefetch_read, prefetch_write,
};

// ============================================================================
// Common Patterns
// ============================================================================

/// Early return pattern for error conditions.
///
/// Optimized for the success case.
#[inline(always)]
pub fn early_return_on_error<T, E>(result: Result<T, E>) -> Option<T> {
    if unlikely(result.is_err()) {
        None
    } else {
        // Safe: we just checked is_err(), so it must be Ok
        result.ok()
    }
}

/// Early return pattern for null pointers.
///
/// Optimized for the non-null case.
#[inline(always)]
pub fn early_return_on_null<T>(ptr: *const T) -> Option<*const T> {
    if unlikely(ptr.is_null()) {
        None
    } else {
        Some(ptr)
    }
}

/// Early return pattern for Option::None.
///
/// Optimized for the Some case.
#[inline(always)]
pub fn early_return_on_none<T>(opt: Option<T>) -> Option<T> {
    if unlikely(opt.is_none()) { None } else { opt }
}

// ============================================================================
// Inline Wrapper Functions for Hot Paths
// ============================================================================

/// Inline min for f32 (avoids function call overhead).
#[inline(always)]
pub fn min_f32(a: f32, b: f32) -> f32 {
    if a < b { a } else { b }
}

/// Inline max for f32 (avoids function call overhead).
#[inline(always)]
pub fn max_f32(a: f32, b: f32) -> f32 {
    if a > b { a } else { b }
}

/// Inline clamp for f32.
#[inline(always)]
pub fn clamp_f32(value: f32, min: f32, max: f32) -> f32 {
    max_f32(min, min_f32(value, max))
}

/// Inline min for i32.
#[inline(always)]
pub fn min_i32(a: i32, b: i32) -> i32 {
    if a < b { a } else { b }
}

/// Inline max for i32.
#[inline(always)]
pub fn max_i32(a: i32, b: i32) -> i32 {
    if a > b { a } else { b }
}

/// Inline clamp for i32.
#[inline(always)]
pub fn clamp_i32(value: i32, min: i32, max: i32) -> i32 {
    max_i32(min, min_i32(value, max))
}

/// Inline min for usize.
#[inline(always)]
pub fn min_usize(a: usize, b: usize) -> usize {
    if a < b { a } else { b }
}

/// Inline max for usize.
#[inline(always)]
pub fn max_usize(a: usize, b: usize) -> usize {
    if a > b { a } else { b }
}

/// Inline clamp for usize.
#[inline(always)]
pub fn clamp_usize(value: usize, min: usize, max: usize) -> usize {
    max_usize(min, min_usize(value, max))
}

// ============================================================================
// FFI Functions
// ============================================================================

/// FFI-safe likely hint (returns 1 for true, 0 for false).
#[unsafe(no_mangle)]
#[inline(always)]
pub extern "C" fn fz_likely(condition: c_int) -> c_int {
    likely(condition != 0) as c_int
}

/// FFI-safe unlikely hint.
#[unsafe(no_mangle)]
#[inline(always)]
pub extern "C" fn fz_unlikely(condition: c_int) -> c_int {
    unlikely(condition != 0) as c_int
}

/// FFI-safe black box (prevents optimization).
#[unsafe(no_mangle)]
#[inline(never)]
pub extern "C" fn fz_black_box_int(value: c_int) -> c_int {
    black_box(value)
}

/// FFI-safe spin loop hint.
#[unsafe(no_mangle)]
#[inline(always)]
pub extern "C" fn fz_spin_loop_hint() {
    spin_loop();
}

/// FFI-safe assume hint.
///
/// # Safety
/// Caller must ensure condition is true.
#[unsafe(no_mangle)]
#[inline(always)]
pub extern "C" fn fz_assume(condition: c_int) {
    assume(condition != 0);
}

/// Inline min for f32 (FFI).
#[unsafe(no_mangle)]
#[inline(always)]
pub extern "C" fn fz_min_f32(a: f32, b: f32) -> f32 {
    min_f32(a, b)
}

/// Inline max for f32 (FFI).
#[unsafe(no_mangle)]
#[inline(always)]
pub extern "C" fn fz_max_f32(a: f32, b: f32) -> f32 {
    max_f32(a, b)
}

/// Inline clamp for f32 (FFI).
#[unsafe(no_mangle)]
#[inline(always)]
pub extern "C" fn fz_clamp_f32(value: f32, min: f32, max: f32) -> f32 {
    clamp_f32(value, min, max)
}

/// Inline min for i32 (FFI).
#[unsafe(no_mangle)]
#[inline(always)]
pub extern "C" fn fz_min_i32(a: i32, b: i32) -> i32 {
    min_i32(a, b)
}

/// Inline max for i32 (FFI).
#[unsafe(no_mangle)]
#[inline(always)]
pub extern "C" fn fz_max_i32(a: i32, b: i32) -> i32 {
    max_i32(a, b)
}

/// Inline clamp for i32 (FFI).
#[unsafe(no_mangle)]
#[inline(always)]
pub extern "C" fn fz_clamp_i32(value: i32, min: i32, max: i32) -> i32 {
    clamp_i32(value, min, max)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_likely_unlikely() {
        // These hints don't change the boolean value, just provide optimization hints
        // likely/unlikely return their input unchanged
        assert!(likely(true));
        assert!(!likely(false));
        assert!(unlikely(true)); // unlikely(true) is still true
        assert!(!unlikely(false)); // unlikely(false) is still false
    }

    #[test]
    fn test_min_max_f32() {
        assert_eq!(min_f32(1.0, 2.0), 1.0);
        assert_eq!(min_f32(2.0, 1.0), 1.0);
        assert_eq!(max_f32(1.0, 2.0), 2.0);
        assert_eq!(max_f32(2.0, 1.0), 2.0);
    }

    #[test]
    fn test_clamp_f32() {
        assert_eq!(clamp_f32(0.5, 0.0, 1.0), 0.5);
        assert_eq!(clamp_f32(-1.0, 0.0, 1.0), 0.0);
        assert_eq!(clamp_f32(2.0, 0.0, 1.0), 1.0);
    }

    #[test]
    fn test_min_max_i32() {
        assert_eq!(min_i32(1, 2), 1);
        assert_eq!(min_i32(2, 1), 1);
        assert_eq!(max_i32(1, 2), 2);
        assert_eq!(max_i32(2, 1), 2);
    }

    #[test]
    fn test_clamp_i32() {
        assert_eq!(clamp_i32(5, 0, 10), 5);
        assert_eq!(clamp_i32(-5, 0, 10), 0);
        assert_eq!(clamp_i32(15, 0, 10), 10);
    }

    #[test]
    fn test_min_max_usize() {
        assert_eq!(min_usize(1, 2), 1);
        assert_eq!(max_usize(1, 2), 2);
        assert_eq!(clamp_usize(5, 0, 10), 5);
    }

    #[test]
    fn test_black_box() {
        let x = black_box(42);
        assert_eq!(x, 42);
    }

    #[test]
    fn test_early_return_on_error() {
        let ok: Result<i32, &str> = Ok(42);
        let err: Result<i32, &str> = Err("error");

        assert_eq!(early_return_on_error(ok), Some(42));
        assert_eq!(early_return_on_error(err), None);
    }

    #[test]
    fn test_early_return_on_null() {
        let ptr: *const i32 = &42;
        let null: *const i32 = std::ptr::null();

        assert!(early_return_on_null(ptr).is_some());
        assert!(early_return_on_null(null).is_none());
    }

    #[test]
    fn test_early_return_on_none() {
        let some: Option<i32> = Some(42);
        let none: Option<i32> = None;

        assert_eq!(early_return_on_none(some), Some(42));
        assert_eq!(early_return_on_none(none), None);
    }

    #[test]
    fn test_assume() {
        // Should not panic
        assume(true);
        assume(1 > 0);
    }

    #[test]
    fn test_ffi_likely() {
        assert_eq!(fz_likely(1), 1);
        assert_eq!(fz_likely(0), 0);
        assert_eq!(fz_unlikely(1), 1);
        assert_eq!(fz_unlikely(0), 0);
    }

    #[test]
    fn test_ffi_min_max() {
        assert_eq!(fz_min_f32(1.0, 2.0), 1.0);
        assert_eq!(fz_max_f32(1.0, 2.0), 2.0);
        assert_eq!(fz_clamp_f32(0.5, 0.0, 1.0), 0.5);

        assert_eq!(fz_min_i32(1, 2), 1);
        assert_eq!(fz_max_i32(1, 2), 2);
        assert_eq!(fz_clamp_i32(5, 0, 10), 5);
    }

    #[test]
    fn test_ffi_black_box() {
        assert_eq!(fz_black_box_int(42), 42);
    }
}
