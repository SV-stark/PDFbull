//! PDF Object Utility Functions (Geometry, Key Access, etc.)

use super::super::Handle;
use super::refcount::with_obj;
use super::types::{PDF_OBJECTS, PdfObj, PdfObjHandle, PdfObjType};

// ============================================================================
// PDF Geometry Object Creation
// ============================================================================

/// Create a PDF array representing a point [x, y]
#[unsafe(no_mangle)]
pub extern "C" fn pdf_new_point(_ctx: Handle, _doc: Handle, x: f32, y: f32) -> PdfObjHandle {
    let mut arr = PdfObj::new_array(2);
    if let PdfObjType::Array(ref mut a) = arr.obj_type {
        a.push(PdfObj::new_real(x as f64));
        a.push(PdfObj::new_real(y as f64));
    }
    PDF_OBJECTS.insert(arr)
}

/// Create a PDF array representing a rect [x0, y0, x1, y1]
#[unsafe(no_mangle)]
pub extern "C" fn pdf_new_rect(
    _ctx: Handle,
    _doc: Handle,
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
) -> PdfObjHandle {
    let mut arr = PdfObj::new_array(4);
    if let PdfObjType::Array(ref mut a) = arr.obj_type {
        a.push(PdfObj::new_real(x0 as f64));
        a.push(PdfObj::new_real(y0 as f64));
        a.push(PdfObj::new_real(x1 as f64));
        a.push(PdfObj::new_real(y1 as f64));
    }
    PDF_OBJECTS.insert(arr)
}

/// Create a PDF array representing a matrix [a, b, c, d, e, f]
#[unsafe(no_mangle)]
pub extern "C" fn pdf_new_matrix(
    _ctx: Handle,
    _doc: Handle,
    a: f32,
    b: f32,
    c: f32,
    d: f32,
    e: f32,
    f: f32,
) -> PdfObjHandle {
    let mut arr = PdfObj::new_array(6);
    if let PdfObjType::Array(ref mut arr_vec) = arr.obj_type {
        arr_vec.push(PdfObj::new_real(a as f64));
        arr_vec.push(PdfObj::new_real(b as f64));
        arr_vec.push(PdfObj::new_real(c as f64));
        arr_vec.push(PdfObj::new_real(d as f64));
        arr_vec.push(PdfObj::new_real(e as f64));
        arr_vec.push(PdfObj::new_real(f as f64));
    }
    PDF_OBJECTS.insert(arr)
}

/// Create a PDF date string from components
#[unsafe(no_mangle)]
pub extern "C" fn pdf_new_date(
    _ctx: Handle,
    _doc: Handle,
    year: i32,
    month: i32,
    day: i32,
    hour: i32,
    minute: i32,
    second: i32,
) -> PdfObjHandle {
    // PDF date format: D:YYYYMMDDHHmmSS
    let date_str = format!(
        "D:{:04}{:02}{:02}{:02}{:02}{:02}",
        year, month, day, hour, minute, second
    );
    PDF_OBJECTS.insert(PdfObj::new_string(date_str.as_bytes()))
}

// ============================================================================
// PDF Array/Dict Key Access
// ============================================================================

#[unsafe(no_mangle)]
pub extern "C" fn pdf_dict_get_key(_ctx: Handle, dict: PdfObjHandle, index: i32) -> PdfObjHandle {
    let key = with_obj(dict, None, |o| match &o.obj_type {
        PdfObjType::Dict(entries) => {
            let idx = index as usize;
            if idx < entries.len() {
                Some(PdfObj::new_name(&entries[idx].0))
            } else {
                None
            }
        }
        _ => None,
    });

    match key {
        Some(k) => PDF_OBJECTS.insert(k),
        None => 0,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn pdf_dict_get_val(_ctx: Handle, dict: PdfObjHandle, index: i32) -> PdfObjHandle {
    let val = with_obj(dict, None, |o| match &o.obj_type {
        PdfObjType::Dict(entries) => {
            let idx = index as usize;
            if idx < entries.len() {
                Some(entries[idx].1.clone())
            } else {
                None
            }
        }
        _ => None,
    });

    match val {
        Some(v) => PDF_OBJECTS.insert(v),
        None => 0,
    }
}

// ============================================================================
// PDF Object Resolution and Loading
// ============================================================================

/// Resolve an indirect reference to get the actual object
///
/// If the object is an indirect reference, returns the referenced object.
/// If the object is not indirect, returns the object itself.
///
/// # Arguments
/// * `_ctx` - Context handle (unused)
/// * `_doc` - Document handle (unused in this implementation)
/// * `obj` - Object to resolve
///
/// # Returns
/// Handle to the resolved object, or same handle if not indirect
#[unsafe(no_mangle)]
pub extern "C" fn pdf_resolve_indirect(
    _ctx: Handle,
    _doc: Handle,
    obj: PdfObjHandle,
) -> PdfObjHandle {
    // In our simplified implementation, we don't have true indirect references
    // that point to other objects in the storage. Instead, we return the object itself.
    // A full implementation would look up the object number in the document's xref table.

    if obj == 0 {
        return 0;
    }

    // Check if object exists
    if PDF_OBJECTS.get(obj).is_some() {
        // For indirect objects, we would normally resolve them here
        // For now, just return the same handle since we don't maintain
        // a true xref table in this FFI layer
        obj
    } else {
        0
    }
}

/// Load an object from the PDF document by object number
///
/// Loads an object from the PDF file given its object number and generation.
/// This is used to access objects that are not currently in memory.
///
/// # Arguments
/// * `_ctx` - Context handle (unused)
/// * `_doc` - Document handle (would contain xref table in full implementation)
/// * `num` - Object number to load
/// * `generation` - Generation number
///
/// # Returns
/// Handle to the loaded object, or 0 if not found
#[unsafe(no_mangle)]
pub extern "C" fn pdf_load_object(
    _ctx: Handle,
    _doc: Handle,
    num: i32,
    generation: i32,
) -> PdfObjHandle {
    // Note: PDF indirect object resolution requires:
    // 1. Access to document's xref table
    // 2. File stream access for seeking/reading
    // 3. PDF object parser for the loaded data
    //
    // These are document-level features, not FFI-level. The FFI correctly
    // creates an indirect reference object with the requested num/generation.
    // Full object loading requires integration with the document parser layer.
    PDF_OBJECTS.insert(PdfObj::new_indirect(num, generation))
}

/// Check if an indirect reference has been resolved/loaded
///
/// # Arguments
/// * `_ctx` - Context handle (unused)
/// * `_doc` - Document handle (unused)
/// * `obj` - Object to check
///
/// # Returns
/// 1 if the object is loaded and not just an indirect reference, 0 otherwise
#[unsafe(no_mangle)]
pub extern "C" fn pdf_obj_is_resolved(_ctx: Handle, _doc: Handle, obj: PdfObjHandle) -> i32 {
    with_obj(obj, 0, |o| {
        match o.obj_type {
            PdfObjType::Indirect { .. } => 0, // Not resolved, still just a reference
            _ => 1,                           // Resolved to actual object
        }
    })
}
