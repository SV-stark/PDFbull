//! PDF Object Comparison FFI Functions

use super::super::Handle;
use super::refcount::with_obj;
use super::types::{PdfObjHandle, PdfObjType};

#[unsafe(no_mangle)]
pub extern "C" fn pdf_objcmp(_ctx: Handle, a: PdfObjHandle, b: PdfObjHandle) -> i32 {
    let obj_a = with_obj(a, None, |o| Some(o.obj_type.clone()));
    let obj_b = with_obj(b, None, |o| Some(o.obj_type.clone()));

    match (obj_a, obj_b) {
        (Some(a_type), Some(b_type)) => i32::from(!a_type.shallow_eq(&b_type)),
        _ => 1,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn pdf_name_eq(_ctx: Handle, a: PdfObjHandle, b: PdfObjHandle) -> i32 {
    let name_a = with_obj(a, None, |o| match &o.obj_type {
        PdfObjType::Name(s) => Some(s.clone()),
        _ => None,
    });

    let name_b = with_obj(b, None, |o| match &o.obj_type {
        PdfObjType::Name(s) => Some(s.clone()),
        _ => None,
    });

    match (name_a, name_b) {
        (Some(a_name), Some(b_name)) => i32::from(a_name == b_name),
        _ => 0,
    }
}
