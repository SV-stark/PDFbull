//! PDF Object Value Extraction FFI Functions

use super::super::Handle;
use super::refcount::with_obj;
use super::types::{PdfObjHandle, PdfObjType};
use std::ffi::{CString, c_char};
use std::sync::{LazyLock, Mutex};

// Static storage for returned name strings
static NAME_STORAGE: LazyLock<Mutex<Vec<CString>>> = LazyLock::new(|| Mutex::new(Vec::new()));

#[unsafe(no_mangle)]
pub extern "C" fn pdf_to_bool(_ctx: Handle, obj: PdfObjHandle) -> i32 {
    with_obj(obj, 0, |o| match &o.obj_type {
        PdfObjType::Bool(b) => i32::from(*b),
        _ => 0,
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn pdf_to_int(_ctx: Handle, obj: PdfObjHandle) -> i32 {
    with_obj(obj, 0, |o| match &o.obj_type {
        PdfObjType::Int(i) => *i as i32,
        PdfObjType::Real(f) => *f as i32,
        _ => 0,
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn pdf_to_int64(_ctx: Handle, obj: PdfObjHandle) -> i64 {
    with_obj(obj, 0, |o| match &o.obj_type {
        PdfObjType::Int(i) => *i,
        PdfObjType::Real(f) => *f as i64,
        _ => 0,
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn pdf_to_real(_ctx: Handle, obj: PdfObjHandle) -> f32 {
    with_obj(obj, 0.0, |o| match &o.obj_type {
        PdfObjType::Real(f) => *f as f32,
        PdfObjType::Int(i) => *i as f32,
        _ => 0.0,
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn pdf_to_name(_ctx: Handle, obj: PdfObjHandle) -> *const c_char {
    static EMPTY: &[u8] = b"\0";

    let name = with_obj(obj, None, |o| match &o.obj_type {
        PdfObjType::Name(s) => Some(s.clone()),
        _ => None,
    });

    match name {
        Some(s) => {
            if let Ok(cstring) = CString::new(s) {
                let ptr = cstring.as_ptr();
                if let Ok(mut storage) = NAME_STORAGE.lock() {
                    storage.push(cstring);
                }
                ptr
            } else {
                EMPTY.as_ptr() as *const c_char
            }
        }
        None => EMPTY.as_ptr() as *const c_char,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn pdf_to_num(_ctx: Handle, obj: PdfObjHandle) -> i32 {
    with_obj(obj, 0, |o| match &o.obj_type {
        PdfObjType::Indirect { num, .. } => *num,
        _ => 0,
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn pdf_to_gen(_ctx: Handle, obj: PdfObjHandle) -> i32 {
    with_obj(obj, 0, |o| match &o.obj_type {
        PdfObjType::Indirect { generation, .. } => *generation,
        _ => 0,
    })
}

// ============================================================================
// PDF Object Value Extraction with Defaults
// ============================================================================

#[unsafe(no_mangle)]
pub extern "C" fn pdf_to_bool_default(_ctx: Handle, obj: PdfObjHandle, def: i32) -> i32 {
    with_obj(obj, def, |o| match &o.obj_type {
        PdfObjType::Bool(b) => i32::from(*b),
        _ => def,
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn pdf_to_int_default(_ctx: Handle, obj: PdfObjHandle, def: i32) -> i32 {
    with_obj(obj, def, |o| match &o.obj_type {
        PdfObjType::Int(i) => *i as i32,
        PdfObjType::Real(f) => *f as i32,
        _ => def,
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn pdf_to_real_default(_ctx: Handle, obj: PdfObjHandle, def: f32) -> f32 {
    with_obj(obj, def, |o| match &o.obj_type {
        PdfObjType::Real(f) => *f as f32,
        PdfObjType::Int(i) => *i as f32,
        _ => def,
    })
}
