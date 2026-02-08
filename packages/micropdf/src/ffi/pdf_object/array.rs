//! PDF Array Operations FFI Functions

use super::super::Handle;
use super::refcount::{with_obj, with_obj_mut};
use super::types::{PDF_OBJECTS, PdfObj, PdfObjHandle, PdfObjType};

#[unsafe(no_mangle)]
pub extern "C" fn pdf_array_len(_ctx: Handle, array: PdfObjHandle) -> i32 {
    with_obj(array, 0, |o| match &o.obj_type {
        PdfObjType::Array(arr) => arr.len() as i32,
        _ => 0,
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn pdf_array_push(_ctx: Handle, array: PdfObjHandle, obj: PdfObjHandle) {
    let obj_to_push = with_obj(obj, None, |o| Some(o.clone()));

    if let Some(obj_clone) = obj_to_push {
        with_obj_mut(array, (), |arr| {
            if let PdfObjType::Array(ref mut a) = arr.obj_type {
                a.push(obj_clone);
                arr.dirty = true;
            }
        });
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn pdf_array_push_int(_ctx: Handle, array: PdfObjHandle, x: i64) {
    with_obj_mut(array, (), |arr| {
        if let PdfObjType::Array(ref mut a) = arr.obj_type {
            a.push(PdfObj::new_int(x));
            arr.dirty = true;
        }
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn pdf_array_push_real(_ctx: Handle, array: PdfObjHandle, x: f64) {
    with_obj_mut(array, (), |arr| {
        if let PdfObjType::Array(ref mut a) = arr.obj_type {
            a.push(PdfObj::new_real(x));
            arr.dirty = true;
        }
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn pdf_array_push_bool(_ctx: Handle, array: PdfObjHandle, x: i32) {
    with_obj_mut(array, (), |arr| {
        if let PdfObjType::Array(ref mut a) = arr.obj_type {
            a.push(PdfObj::new_bool(x != 0));
            arr.dirty = true;
        }
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn pdf_array_delete(_ctx: Handle, array: PdfObjHandle, index: i32) {
    with_obj_mut(array, (), |arr| {
        if let PdfObjType::Array(ref mut a) = arr.obj_type {
            let idx = index as usize;
            if idx < a.len() {
                a.remove(idx);
                arr.dirty = true;
            }
        }
    });
}

// ============================================================================
// PDF Array Get/Put Operations
// ============================================================================

#[unsafe(no_mangle)]
pub extern "C" fn pdf_array_get(_ctx: Handle, array: PdfObjHandle, index: i32) -> PdfObjHandle {
    let obj = with_obj(array, None, |o| match &o.obj_type {
        PdfObjType::Array(arr) => {
            let idx = index as usize;
            if idx < arr.len() {
                Some(arr[idx].clone())
            } else {
                None
            }
        }
        _ => None,
    });

    match obj {
        Some(o) => PDF_OBJECTS.insert(o),
        None => 0, // Return null handle
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn pdf_array_put(_ctx: Handle, array: PdfObjHandle, index: i32, obj: PdfObjHandle) {
    let obj_to_put = with_obj(obj, None, |o| Some(o.clone()));

    if let Some(obj_clone) = obj_to_put {
        with_obj_mut(array, (), |arr| {
            if let PdfObjType::Array(ref mut a) = arr.obj_type {
                let idx = index as usize;
                if idx < a.len() {
                    a[idx] = obj_clone;
                    arr.dirty = true;
                } else if idx == a.len() {
                    // Allow appending at the end
                    a.push(obj_clone);
                    arr.dirty = true;
                }
            }
        });
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn pdf_array_insert(
    _ctx: Handle,
    array: PdfObjHandle,
    index: i32,
    obj: PdfObjHandle,
) {
    let obj_to_insert = with_obj(obj, None, |o| Some(o.clone()));

    if let Some(obj_clone) = obj_to_insert {
        with_obj_mut(array, (), |arr| {
            if let PdfObjType::Array(ref mut a) = arr.obj_type {
                let idx = (index as usize).min(a.len());
                a.insert(idx, obj_clone);
                arr.dirty = true;
            }
        });
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn pdf_array_push_name(
    _ctx: Handle,
    array: PdfObjHandle,
    name: *const std::ffi::c_char,
) {
    use std::ffi::CStr;

    if name.is_null() {
        return;
    }

    let name_str = unsafe { CStr::from_ptr(name) }.to_str().unwrap_or("");

    with_obj_mut(array, (), |arr| {
        if let PdfObjType::Array(ref mut a) = arr.obj_type {
            a.push(PdfObj::new_name(name_str));
            arr.dirty = true;
        }
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn pdf_array_push_string(
    _ctx: Handle,
    array: PdfObjHandle,
    str: *const std::ffi::c_char,
    len: usize,
) {
    if str.is_null() || len == 0 {
        return;
    }

    let data = unsafe { std::slice::from_raw_parts(str as *const u8, len) };

    with_obj_mut(array, (), |arr| {
        if let PdfObjType::Array(ref mut a) = arr.obj_type {
            a.push(PdfObj::new_string(data));
            arr.dirty = true;
        }
    });
}
