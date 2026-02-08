//! PDF Object Type Checking FFI Functions

use super::super::Handle;
use super::refcount::with_obj;
use super::types::{PdfObjHandle, PdfObjType};

#[unsafe(no_mangle)]
pub extern "C" fn pdf_is_null(_ctx: Handle, obj: PdfObjHandle) -> i32 {
    with_obj(obj, 1, |o| {
        i32::from(matches!(o.obj_type, PdfObjType::Null))
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn pdf_is_bool(_ctx: Handle, obj: PdfObjHandle) -> i32 {
    with_obj(obj, 0, |o| {
        i32::from(matches!(o.obj_type, PdfObjType::Bool(_)))
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn pdf_is_int(_ctx: Handle, obj: PdfObjHandle) -> i32 {
    with_obj(obj, 0, |o| {
        i32::from(matches!(o.obj_type, PdfObjType::Int(_)))
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn pdf_is_real(_ctx: Handle, obj: PdfObjHandle) -> i32 {
    with_obj(obj, 0, |o| {
        i32::from(matches!(o.obj_type, PdfObjType::Real(_)))
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn pdf_is_number(_ctx: Handle, obj: PdfObjHandle) -> i32 {
    with_obj(obj, 0, |o| {
        i32::from(matches!(
            o.obj_type,
            PdfObjType::Int(_) | PdfObjType::Real(_)
        ))
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn pdf_is_name(_ctx: Handle, obj: PdfObjHandle) -> i32 {
    with_obj(obj, 0, |o| {
        i32::from(matches!(o.obj_type, PdfObjType::Name(_)))
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn pdf_is_string(_ctx: Handle, obj: PdfObjHandle) -> i32 {
    with_obj(obj, 0, |o| {
        i32::from(matches!(o.obj_type, PdfObjType::String(_)))
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn pdf_is_array(_ctx: Handle, obj: PdfObjHandle) -> i32 {
    with_obj(obj, 0, |o| {
        i32::from(matches!(o.obj_type, PdfObjType::Array(_)))
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn pdf_is_dict(_ctx: Handle, obj: PdfObjHandle) -> i32 {
    with_obj(obj, 0, |o| {
        i32::from(matches!(o.obj_type, PdfObjType::Dict(_)))
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn pdf_is_indirect(_ctx: Handle, obj: PdfObjHandle) -> i32 {
    with_obj(obj, 0, |o| {
        i32::from(matches!(o.obj_type, PdfObjType::Indirect { .. }))
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn pdf_is_stream(_ctx: Handle, obj: PdfObjHandle) -> i32 {
    with_obj(obj, 0, |o| {
        i32::from(matches!(o.obj_type, PdfObjType::Stream { .. }))
    })
}
