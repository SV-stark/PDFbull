//! PDF Object Marking, Dirty Tracking, and Parent Management FFI Functions

use super::super::Handle;
use super::refcount::{with_obj, with_obj_mut};
use super::types::PdfObjHandle;

// ============================================================================
// PDF Object Marking
// ============================================================================

#[unsafe(no_mangle)]
pub extern "C" fn pdf_obj_marked(_ctx: Handle, obj: PdfObjHandle) -> i32 {
    with_obj(obj, 0, |o| i32::from(o.marked))
}

#[unsafe(no_mangle)]
pub extern "C" fn pdf_mark_obj(_ctx: Handle, obj: PdfObjHandle) -> i32 {
    with_obj_mut(obj, 0, |o| {
        let was_marked = o.marked;
        o.marked = true;
        i32::from(was_marked)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn pdf_unmark_obj(_ctx: Handle, obj: PdfObjHandle) {
    with_obj_mut(obj, (), |o| {
        o.marked = false;
    });
}

// ============================================================================
// PDF Object Dirty Tracking
// ============================================================================

#[unsafe(no_mangle)]
pub extern "C" fn pdf_obj_is_dirty(_ctx: Handle, obj: PdfObjHandle) -> i32 {
    with_obj(obj, 0, |o| i32::from(o.dirty))
}

#[unsafe(no_mangle)]
pub extern "C" fn pdf_dirty_obj(_ctx: Handle, obj: PdfObjHandle) {
    with_obj_mut(obj, (), |o| {
        o.dirty = true;
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn pdf_clean_obj(_ctx: Handle, obj: PdfObjHandle) {
    with_obj_mut(obj, (), |o| {
        o.dirty = false;
    });
}

// ============================================================================
// PDF Object Parent
// ============================================================================

#[unsafe(no_mangle)]
pub extern "C" fn pdf_set_obj_parent(_ctx: Handle, obj: PdfObjHandle, num: i32) {
    with_obj_mut(obj, (), |o| {
        o.parent_num = num;
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn pdf_obj_parent_num(_ctx: Handle, obj: PdfObjHandle) -> i32 {
    with_obj(obj, 0, |o| o.parent_num)
}

#[unsafe(no_mangle)]
pub extern "C" fn pdf_obj_refs(_ctx: Handle, obj: PdfObjHandle) -> i32 {
    with_obj(obj, 0, |o| o.refs)
}
