//! PDF String Extraction FFI Functions

use super::super::Handle;
use super::refcount::with_obj;
use super::types::{PdfObjHandle, PdfObjType};
use std::ffi::c_char;
use std::sync::{LazyLock, Mutex};

static STRING_STORAGE: LazyLock<Mutex<Vec<Vec<u8>>>> = LazyLock::new(|| Mutex::new(Vec::new()));

#[unsafe(no_mangle)]
pub extern "C" fn pdf_to_string(
    _ctx: Handle,
    obj: PdfObjHandle,
    sizep: *mut usize,
) -> *const c_char {
    let data = with_obj(obj, None, |o| match &o.obj_type {
        PdfObjType::String(s) => Some(s.clone()),
        _ => None,
    });

    match data {
        Some(s) => {
            if !sizep.is_null() {
                unsafe {
                    *sizep = s.len();
                }
            }
            let ptr = s.as_ptr() as *const c_char;
            if let Ok(mut storage) = STRING_STORAGE.lock() {
                storage.push(s);
            }
            ptr
        }
        None => {
            if !sizep.is_null() {
                unsafe {
                    *sizep = 0;
                }
            }
            std::ptr::null()
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn pdf_to_str_buf(_ctx: Handle, obj: PdfObjHandle) -> *const c_char {
    pdf_to_string(_ctx, obj, std::ptr::null_mut())
}

#[unsafe(no_mangle)]
pub extern "C" fn pdf_to_str_len(_ctx: Handle, obj: PdfObjHandle) -> usize {
    with_obj(obj, 0, |o| match &o.obj_type {
        PdfObjType::String(s) => s.len(),
        _ => 0,
    })
}

#[cfg(test)]
mod tests {
    use super::super::create::pdf_new_string;
    use super::*;

    #[test]
    fn test_pdf_to_string_valid() {
        let ctx = 0;
        let data = b"Hello, World!";
        let obj = pdf_new_string(ctx, data.as_ptr() as *const c_char, data.len());

        let mut size = 0usize;
        let result = pdf_to_string(ctx, obj, &mut size as *mut usize);

        assert!(!result.is_null());
        assert_eq!(size, data.len());
    }

    #[test]
    fn test_pdf_to_string_null_sizep() {
        let ctx = 0;
        let data = b"Test string";
        let obj = pdf_new_string(ctx, data.as_ptr() as *const c_char, data.len());

        let result = pdf_to_string(ctx, obj, std::ptr::null_mut());

        assert!(!result.is_null());
    }

    #[test]
    fn test_pdf_to_string_invalid_obj() {
        let ctx = 0;
        let invalid_obj = 9999; // Non-existent handle
        let mut size = 0usize;

        let result = pdf_to_string(ctx, invalid_obj, &mut size as *mut usize);

        assert!(result.is_null());
        assert_eq!(size, 0);
    }

    #[test]
    fn test_pdf_to_str_buf() {
        let ctx = 0;
        let data = b"Buffer test";
        let obj = pdf_new_string(ctx, data.as_ptr() as *const c_char, data.len());

        let result = pdf_to_str_buf(ctx, obj);

        assert!(!result.is_null());
    }

    #[test]
    fn test_pdf_to_str_buf_invalid() {
        let ctx = 0;
        let invalid_obj = 9999;

        let result = pdf_to_str_buf(ctx, invalid_obj);

        assert!(result.is_null());
    }

    #[test]
    fn test_pdf_to_str_len() {
        let ctx = 0;
        let data = b"Length test";
        let obj = pdf_new_string(ctx, data.as_ptr() as *const c_char, data.len());

        let len = pdf_to_str_len(ctx, obj);

        assert_eq!(len, data.len());
    }

    #[test]
    fn test_pdf_to_str_len_invalid() {
        let ctx = 0;
        let invalid_obj = 9999;

        let len = pdf_to_str_len(ctx, invalid_obj);

        assert_eq!(len, 0);
    }

    #[test]
    fn test_pdf_to_str_len_empty_string() {
        let ctx = 0;
        let data = b"";
        let obj = pdf_new_string(ctx, data.as_ptr() as *const c_char, data.len());

        let len = pdf_to_str_len(ctx, obj);

        assert_eq!(len, 0);
    }
}
