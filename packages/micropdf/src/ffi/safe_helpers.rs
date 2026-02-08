//! Safe helper functions for FFI operations
//!
//! This module provides safe abstractions over common unsafe FFI patterns.

use std::ffi::CStr;
use std::os::raw::c_char;

/// Safely convert a C string pointer to a Rust &str
///
/// Returns None if the pointer is null or invalid UTF-8
pub fn c_str_to_str<'a>(ptr: *const c_char) -> Option<&'a str> {
    if ptr.is_null() {
        return None;
    }

    unsafe { CStr::from_ptr(ptr).to_str().ok() }
}

/// Safely copy a Rust string to a C buffer
///
/// Returns the number of bytes written (excluding null terminator)
/// Returns -1 if buffer is null or too small
pub fn str_to_c_buffer(src: &str, dst: *mut c_char, size: i32) -> i32 {
    if dst.is_null() || size <= 0 {
        return -1;
    }

    let src_bytes = src.as_bytes();
    let copy_len = src_bytes.len().min((size - 1) as usize);

    unsafe {
        std::ptr::copy_nonoverlapping(src_bytes.as_ptr(), dst as *mut u8, copy_len);
        *dst.add(copy_len) = 0; // Null terminate
    }

    copy_len as i32
}

/// Safely read a C string buffer
///
/// Returns None if the pointer is null or invalid
#[allow(dead_code)]
pub fn read_c_string(ptr: *const c_char) -> Option<String> {
    c_str_to_str(ptr).map(|s| s.to_string())
}

/// Safely copy data from a pointer to a Vec
///
/// Returns None if pointer is null or len is invalid
pub fn copy_from_ptr<T: Copy>(ptr: *const T, len: usize) -> Option<Vec<T>> {
    if ptr.is_null() || len == 0 {
        return None;
    }

    let mut vec = Vec::with_capacity(len);
    unsafe {
        std::ptr::copy_nonoverlapping(ptr, vec.as_mut_ptr(), len);
        vec.set_len(len);
    }
    Some(vec)
}

/// Safely write data from a slice to a pointer
///
/// Returns false if pointer is null
#[allow(dead_code)]
pub fn copy_to_ptr<T: Copy>(src: &[T], dst: *mut T) -> bool {
    if dst.is_null() || src.is_empty() {
        return false;
    }

    unsafe {
        std::ptr::copy_nonoverlapping(src.as_ptr(), dst, src.len());
    }
    true
}

/// Safely write a single value to a pointer
///
/// Returns false if pointer is null
pub fn write_ptr<T>(value: T, dst: *mut T) -> bool {
    if dst.is_null() {
        return false;
    }

    unsafe {
        *dst = value;
    }
    true
}

/// Validate color components are in range [0.0, 1.0]
#[allow(dead_code)]
pub fn validate_color_components(components: &[f32]) -> bool {
    components.iter().all(|&c| (0.0..=1.0).contains(&c))
}

/// Validate a single color component is in range [0.0, 1.0]
#[allow(dead_code)]
pub fn validate_color(c: f32) -> bool {
    (0.0..=1.0).contains(&c)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    #[test]
    fn test_c_str_to_str() {
        assert_eq!(c_str_to_str(std::ptr::null()), None);

        let c_string = CString::new("Hello").unwrap();
        assert_eq!(c_str_to_str(c_string.as_ptr()), Some("Hello"));
    }

    #[test]
    fn test_str_to_c_buffer() {
        let mut buf = [0i8; 10];
        let written = str_to_c_buffer("Hi", buf.as_mut_ptr(), 10);
        assert_eq!(written, 2);

        let result = unsafe { CStr::from_ptr(buf.as_ptr()) };
        assert_eq!(result.to_str().unwrap(), "Hi");
    }

    #[test]
    fn test_str_to_c_buffer_truncation() {
        let mut buf = [0i8; 5];
        let written = str_to_c_buffer("Hello World", buf.as_mut_ptr(), 5);
        assert_eq!(written, 4); // "Hell" + null
    }

    #[test]
    fn test_copy_from_ptr() {
        let data = [1u8, 2, 3, 4, 5];
        let result = copy_from_ptr(data.as_ptr(), data.len());
        assert_eq!(result, Some(vec![1, 2, 3, 4, 5]));

        assert_eq!(copy_from_ptr::<u8>(std::ptr::null(), 5), None);
    }

    #[test]
    fn test_copy_to_ptr() {
        let src = vec![1u8, 2, 3];
        let mut dst = vec![0u8; 3];

        assert!(copy_to_ptr(&src, dst.as_mut_ptr()));
        assert_eq!(dst, vec![1, 2, 3]);

        assert!(!copy_to_ptr(&src, std::ptr::null_mut()));
    }

    #[test]
    fn test_write_ptr() {
        let mut value = 0i32;
        assert!(write_ptr(42, &mut value as *mut i32));
        assert_eq!(value, 42);

        assert!(!write_ptr(42, std::ptr::null_mut()));
    }

    #[test]
    fn test_validate_color() {
        assert!(validate_color(0.0));
        assert!(validate_color(0.5));
        assert!(validate_color(1.0));
        assert!(!validate_color(-0.1));
        assert!(!validate_color(1.1));
    }

    #[test]
    fn test_validate_color_components() {
        assert!(validate_color_components(&[0.0, 0.5, 1.0]));
        assert!(!validate_color_components(&[-0.1, 0.5, 1.0]));
        assert!(!validate_color_components(&[0.0, 0.5, 1.1]));
    }
}
