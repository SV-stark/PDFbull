//! PDF Object Reference Counting FFI Functions

use super::super::Handle;
use super::types::{PDF_OBJECTS, PdfObj, PdfObjHandle};

#[unsafe(no_mangle)]
pub extern "C" fn pdf_keep_obj(_ctx: Handle, obj: PdfObjHandle) -> PdfObjHandle {
    if let Some(arc) = PDF_OBJECTS.get(obj) {
        if let Ok(mut guard) = arc.lock() {
            guard.refs += 1;
        }
    }
    obj
}

#[unsafe(no_mangle)]
pub extern "C" fn pdf_drop_obj(_ctx: Handle, obj: PdfObjHandle) {
    if let Some(arc) = PDF_OBJECTS.get(obj) {
        let should_remove = {
            if let Ok(mut guard) = arc.lock() {
                guard.refs -= 1;
                guard.refs <= 0
            } else {
                false
            }
        };
        if should_remove {
            PDF_OBJECTS.remove(obj);
        }
    }
}

// Helper functions for accessing objects
pub(super) fn with_obj<T, F: FnOnce(&PdfObj) -> T>(obj: PdfObjHandle, default: T, f: F) -> T {
    PDF_OBJECTS
        .get(obj)
        .and_then(|arc| arc.lock().ok().map(|guard| f(&guard)))
        .unwrap_or(default)
}

pub(super) fn with_obj_mut<T, F: FnOnce(&mut PdfObj) -> T>(
    obj: PdfObjHandle,
    default: T,
    f: F,
) -> T {
    PDF_OBJECTS
        .get(obj)
        .and_then(|arc| arc.lock().ok().map(|mut guard| f(&mut guard)))
        .unwrap_or(default)
}
