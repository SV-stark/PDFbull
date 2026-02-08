//! PDF Object Creation FFI Functions

use super::super::Handle;
use super::types::{PDF_OBJECTS, PdfObj, PdfObjHandle};
use std::ffi::{CStr, c_char};

#[unsafe(no_mangle)]
pub extern "C" fn pdf_new_null(_ctx: Handle) -> PdfObjHandle {
    PDF_OBJECTS.insert(PdfObj::new_null())
}

#[unsafe(no_mangle)]
pub extern "C" fn pdf_new_bool(_ctx: Handle, b: i32) -> PdfObjHandle {
    PDF_OBJECTS.insert(PdfObj::new_bool(b != 0))
}

#[unsafe(no_mangle)]
pub extern "C" fn pdf_new_int(_ctx: Handle, i: i64) -> PdfObjHandle {
    PDF_OBJECTS.insert(PdfObj::new_int(i))
}

#[unsafe(no_mangle)]
pub extern "C" fn pdf_new_real(_ctx: Handle, f: f32) -> PdfObjHandle {
    PDF_OBJECTS.insert(PdfObj::new_real(f as f64))
}

#[unsafe(no_mangle)]
pub extern "C" fn pdf_new_name(_ctx: Handle, str: *const c_char) -> PdfObjHandle {
    if str.is_null() {
        return PDF_OBJECTS.insert(PdfObj::new_name(""));
    }
    let name = unsafe { CStr::from_ptr(str) }.to_str().unwrap_or("");
    PDF_OBJECTS.insert(PdfObj::new_name(name))
}

#[unsafe(no_mangle)]
pub extern "C" fn pdf_new_string(_ctx: Handle, str: *const c_char, len: usize) -> PdfObjHandle {
    if str.is_null() || len == 0 {
        return PDF_OBJECTS.insert(PdfObj::new_string(&[]));
    }
    let data = unsafe { std::slice::from_raw_parts(str as *const u8, len) };
    PDF_OBJECTS.insert(PdfObj::new_string(data))
}

#[unsafe(no_mangle)]
pub extern "C" fn pdf_new_text_string(_ctx: Handle, s: *const c_char) -> PdfObjHandle {
    if s.is_null() {
        return PDF_OBJECTS.insert(PdfObj::new_string(&[]));
    }
    let text = unsafe { CStr::from_ptr(s) }.to_str().unwrap_or("");
    PDF_OBJECTS.insert(PdfObj::new_string(text.as_bytes()))
}

#[unsafe(no_mangle)]
pub extern "C" fn pdf_new_indirect(
    _ctx: Handle,
    _doc: Handle,
    num: i32,
    generation: i32,
) -> PdfObjHandle {
    PDF_OBJECTS.insert(PdfObj::new_indirect(num, generation))
}

#[unsafe(no_mangle)]
pub extern "C" fn pdf_new_array(_ctx: Handle, _doc: Handle, initialcap: i32) -> PdfObjHandle {
    PDF_OBJECTS.insert(PdfObj::new_array(initialcap.max(0) as usize))
}

#[unsafe(no_mangle)]
pub extern "C" fn pdf_new_dict(_ctx: Handle, _doc: Handle, initialcap: i32) -> PdfObjHandle {
    PDF_OBJECTS.insert(PdfObj::new_dict(initialcap.max(0) as usize))
}
