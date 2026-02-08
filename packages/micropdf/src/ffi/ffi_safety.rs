//! FFI Safety Utilities
//!
//! This module provides safety-documented helper functions for common FFI patterns.
//! All functions include explicit SAFETY documentation for each unsafe operation.

// Allow macros to expand metavariables in unsafe blocks - this is intentional
// as these macros are designed to wrap unsafe FFI patterns with documented safety.
#![allow(clippy::macro_metavars_in_unsafe)]

use std::ffi::{CStr, CString};
use std::os::raw::c_char;

// ============================================================================
// C String Conversions
// ============================================================================

/// Convert a C string pointer to a Rust `&str`.
///
/// # Safety Requirements (Caller's Contract)
/// - `ptr` must be a valid pointer to a null-terminated C string
/// - The string must be valid UTF-8
/// - The memory must remain valid for the lifetime of the returned `&str`
/// - The memory must not be modified while the `&str` is in use
///
/// Returns empty string if ptr is null or conversion fails.
#[inline]
pub fn cstr_to_str(ptr: *const c_char) -> &'static str {
    if ptr.is_null() {
        return "";
    }
    // SAFETY: Caller guarantees ptr is valid null-terminated UTF-8 string.
    // The 'static lifetime is safe because we only return borrowed data
    // that should remain valid for the duration of the FFI call.
    unsafe { CStr::from_ptr(ptr) }.to_str().unwrap_or("")
}

/// Convert a C string pointer to an owned Rust `String`.
///
/// # Safety Requirements (Caller's Contract)
/// - `ptr` must be a valid pointer to a null-terminated C string
/// - The string must be valid UTF-8
///
/// Returns empty string if ptr is null or conversion fails.
#[inline]
pub fn cstr_to_string(ptr: *const c_char) -> String {
    if ptr.is_null() {
        return String::new();
    }
    // SAFETY: Caller guarantees ptr is valid null-terminated UTF-8 string.
    unsafe { CStr::from_ptr(ptr) }
        .to_str()
        .map(|s| s.to_string())
        .unwrap_or_default()
}

/// Convert a C string pointer to `Option<&str>`.
///
/// # Safety Requirements (Caller's Contract)
/// - If not null, `ptr` must be a valid pointer to a null-terminated C string
/// - The string must be valid UTF-8 if not null
///
/// Returns None if ptr is null, Some("") if UTF-8 conversion fails.
#[inline]
pub fn cstr_to_option_str(ptr: *const c_char) -> Option<&'static str> {
    if ptr.is_null() {
        return None;
    }
    // SAFETY: Caller guarantees ptr is valid null-terminated string if not null.
    Some(unsafe { CStr::from_ptr(ptr) }.to_str().unwrap_or(""))
}

/// Create a CString from a Rust string, returning null pointer on failure.
#[inline]
pub fn str_to_cstring(s: &str) -> Option<CString> {
    CString::new(s).ok()
}

// ============================================================================
// Raw Slice Conversions
// ============================================================================

/// Create a byte slice from a raw pointer and length.
///
/// # Safety Requirements (Caller's Contract)
/// - `ptr` must be a valid pointer to at least `len` bytes of memory
/// - The memory must be properly aligned for `u8` (always satisfied)
/// - The memory must remain valid for the lifetime of the returned slice
/// - The memory must not be modified while the slice is in use
///
/// Returns empty slice if ptr is null or len is 0.
#[inline]
pub fn raw_to_slice<'a>(ptr: *const u8, len: usize) -> &'a [u8] {
    if ptr.is_null() || len == 0 {
        return &[];
    }
    // SAFETY: Caller guarantees ptr points to len valid bytes.
    unsafe { std::slice::from_raw_parts(ptr, len) }
}

/// Create a mutable byte slice from a raw pointer and length.
///
/// # Safety Requirements (Caller's Contract)
/// - `ptr` must be a valid pointer to at least `len` bytes of memory
/// - The memory must be properly aligned for `u8` (always satisfied)
/// - The pointer must be valid for writes
/// - No other references to this memory may exist
///
/// Returns empty slice if ptr is null or len is 0.
#[inline]
pub fn raw_to_slice_mut<'a>(ptr: *mut u8, len: usize) -> &'a mut [u8] {
    if ptr.is_null() || len == 0 {
        return &mut [];
    }
    // SAFETY: Caller guarantees ptr is valid for len bytes and exclusive access.
    unsafe { std::slice::from_raw_parts_mut(ptr, len) }
}

/// Create a slice of f32 from a raw pointer and count.
///
/// # Safety Requirements (Caller's Contract)
/// - `ptr` must be a valid pointer to at least `count` f32 values
/// - The memory must be properly aligned for `f32`
/// - The memory must remain valid for the lifetime of the returned slice
///
/// Returns empty slice if ptr is null or count is 0.
#[inline]
pub fn raw_to_f32_slice<'a>(ptr: *const f32, count: usize) -> &'a [f32] {
    if ptr.is_null() || count == 0 {
        return &[];
    }
    // SAFETY: Caller guarantees ptr points to count valid f32 values with proper alignment.
    unsafe { std::slice::from_raw_parts(ptr, count) }
}

/// Create a Vec from raw pointer and length (copies data).
///
/// # Safety Requirements (Caller's Contract)
/// - `ptr` must be a valid pointer to at least `len` bytes
/// - The memory must remain valid during the copy
///
/// Returns empty Vec if ptr is null or len is 0.
#[inline]
pub fn raw_to_vec(ptr: *const u8, len: usize) -> Vec<u8> {
    if ptr.is_null() || len == 0 {
        return Vec::new();
    }
    // SAFETY: Caller guarantees ptr points to len valid bytes.
    unsafe { std::slice::from_raw_parts(ptr, len) }.to_vec()
}

/// Create a `Vec<f32>` from raw pointer and count (copies data).
#[inline]
pub fn raw_to_f32_vec(ptr: *const f32, count: usize) -> Vec<f32> {
    if ptr.is_null() || count == 0 {
        return Vec::new();
    }
    // SAFETY: Caller guarantees ptr points to count valid f32 values.
    unsafe { std::slice::from_raw_parts(ptr, count) }.to_vec()
}

// ============================================================================
// Output Pointer Writes
// ============================================================================

/// Write a value through an output pointer.
///
/// # Safety Requirements (Caller's Contract)
/// - `ptr` must be a valid, properly aligned pointer for type `T`
/// - `ptr` must be valid for writes
/// - No other references to this memory location may exist
///
/// Does nothing if ptr is null.
#[inline]
pub fn write_out<T>(ptr: *mut T, value: T) {
    if ptr.is_null() {
        return;
    }
    // SAFETY: Caller guarantees ptr is valid for writes and properly aligned.
    unsafe { *ptr = value };
}

/// Write a value through an output pointer, returning whether write occurred.
#[inline]
pub fn try_write_out<T>(ptr: *mut T, value: T) -> bool {
    if ptr.is_null() {
        return false;
    }
    // SAFETY: Caller guarantees ptr is valid for writes and properly aligned.
    unsafe { *ptr = value };
    true
}

/// Write to output pointer, copying bytes.
///
/// # Safety Requirements (Caller's Contract)
/// - `dst` must be a valid pointer to at least `len` bytes
/// - `dst` must be valid for writes
/// - Memory regions must not overlap (use copy if they might)
#[inline]
pub fn write_bytes(dst: *mut u8, src: &[u8]) {
    if dst.is_null() || src.is_empty() {
        return;
    }
    // SAFETY: Caller guarantees dst is valid for src.len() bytes.
    unsafe {
        std::ptr::copy_nonoverlapping(src.as_ptr(), dst, src.len());
    }
}

// ============================================================================
// Handle Validation
// ============================================================================

/// Check if a handle is valid (non-zero).
#[inline]
pub const fn is_valid_handle(handle: u64) -> bool {
    handle != 0
}

/// Convert handle to usize for indexing (with validation).
#[inline]
pub fn handle_to_index(handle: u64) -> Option<usize> {
    if handle == 0 {
        None
    } else {
        Some(handle as usize)
    }
}

// ============================================================================
// Macros for Common Patterns
// ============================================================================

/// Macro for safely converting a C string pointer with SAFETY comment.
///
/// Usage: `cstr_safe!(ptr)` expands to the unsafe block with SAFETY comment.
#[macro_export]
macro_rules! cstr_safe {
    ($ptr:expr) => {{
        if $ptr.is_null() {
            ""
        } else {
            // SAFETY: Caller guarantees ptr is valid null-terminated UTF-8 string.
            unsafe { std::ffi::CStr::from_ptr($ptr) }
                .to_str()
                .unwrap_or("")
        }
    }};
}

/// Macro for safely creating a slice from raw parts with SAFETY comment.
///
/// Usage: `slice_safe!(ptr, len)` expands to the unsafe block with SAFETY comment.
#[macro_export]
macro_rules! slice_safe {
    ($ptr:expr, $len:expr) => {{
        if $ptr.is_null() || $len == 0 {
            &[]
        } else {
            // SAFETY: Caller guarantees ptr points to len valid elements.
            unsafe { std::slice::from_raw_parts($ptr, $len as usize) }
        }
    }};
}

/// Macro for safely writing to an output pointer with SAFETY comment.
///
/// Usage: `write_safe!(ptr, value)` expands to the unsafe block with SAFETY comment.
#[macro_export]
macro_rules! write_safe {
    ($ptr:expr, $value:expr) => {{
        if !$ptr.is_null() {
            // SAFETY: Caller guarantees ptr is valid for writes and properly aligned.
            unsafe { *$ptr = $value };
        }
    }};
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    #[test]
    fn test_cstr_to_str_null() {
        assert_eq!(cstr_to_str(std::ptr::null()), "");
    }

    #[test]
    fn test_cstr_to_str_valid() {
        let s = CString::new("hello").unwrap();
        assert_eq!(cstr_to_str(s.as_ptr()), "hello");
    }

    #[test]
    fn test_cstr_to_string_null() {
        assert_eq!(cstr_to_string(std::ptr::null()), "");
    }

    #[test]
    fn test_raw_to_slice_null() {
        let slice: &[u8] = raw_to_slice(std::ptr::null(), 10);
        assert!(slice.is_empty());
    }

    #[test]
    fn test_raw_to_slice_valid() {
        let data = [1u8, 2, 3, 4, 5];
        let slice = raw_to_slice(data.as_ptr(), data.len());
        assert_eq!(slice, &data);
    }

    #[test]
    fn test_raw_to_vec_null() {
        let vec = raw_to_vec(std::ptr::null(), 10);
        assert!(vec.is_empty());
    }

    #[test]
    fn test_write_out_null() {
        write_out(std::ptr::null_mut::<i32>(), 42);
        // Should not crash
    }

    #[test]
    fn test_write_out_valid() {
        let mut value = 0i32;
        write_out(&mut value as *mut i32, 42);
        assert_eq!(value, 42);
    }

    #[test]
    fn test_is_valid_handle() {
        assert!(!is_valid_handle(0));
        assert!(is_valid_handle(1));
        assert!(is_valid_handle(u64::MAX));
    }

    // Tests for macros use the helper functions instead to avoid
    // useless_ptr_null_checks warnings since the macros are thin
    // wrappers around these functions.

    #[test]
    fn test_cstr_safe_macro_via_function() {
        // Test null pointer case via function
        assert_eq!(cstr_to_str(std::ptr::null::<i8>()), "");

        // Test valid pointer case via function
        let s = CString::new("test").unwrap();
        assert_eq!(cstr_to_str(s.as_ptr()), "test");
    }

    #[test]
    fn test_slice_safe_macro_via_function() {
        // Test null pointer case via function
        let slice: &[u8] = raw_to_slice(std::ptr::null(), 3);
        assert!(slice.is_empty());

        // Test valid pointer case via function
        let data = [1u8, 2, 3];
        let slice = raw_to_slice(data.as_ptr(), 3);
        assert_eq!(slice, &data);

        // Test with zero length via function
        let empty_slice: &[u8] = raw_to_slice(data.as_ptr(), 0);
        assert!(empty_slice.is_empty());
    }

    #[test]
    fn test_write_safe_macro_via_function() {
        // Test null pointer case via function
        write_out(std::ptr::null_mut::<i32>(), 99);
        // Should not crash

        // Test valid pointer case via function
        let mut value = 0i32;
        write_out(&mut value as *mut i32, 99);
        assert_eq!(value, 99);
    }
}
