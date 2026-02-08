//! Print Production FFI - Page boxes, N-Up, booklets, tiling, validation
//!
//! This module provides FFI functions for print production features with `mp_` prefix.

#![allow(unsafe_op_in_unsafe_fn)]

use crate::ffi::Handle;
use std::ffi::{CStr, CString, c_char, c_float, c_int};
use std::ptr;

// ============================================================================
// Page Box Types
// ============================================================================

/// Page box type constants
pub const NP_BOX_MEDIA: c_int = 0;
pub const NP_BOX_CROP: c_int = 1;
pub const NP_BOX_BLEED: c_int = 2;
pub const NP_BOX_TRIM: c_int = 3;
pub const NP_BOX_ART: c_int = 4;

/// Unit type constants
pub const NP_UNIT_POINT: c_int = 0;
pub const NP_UNIT_INCH: c_int = 1;
pub const NP_UNIT_MM: c_int = 2;
pub const NP_UNIT_CM: c_int = 3;

/// Page size constants
pub const NP_SIZE_LETTER: c_int = 0;
pub const NP_SIZE_LEGAL: c_int = 1;
pub const NP_SIZE_A4: c_int = 2;
pub const NP_SIZE_A3: c_int = 3;
pub const NP_SIZE_A5: c_int = 4;
pub const NP_SIZE_TABLOID: c_int = 5;

/// Binding type constants
pub const NP_BIND_SADDLE_STITCH: c_int = 0;
pub const NP_BIND_PERFECT: c_int = 1;
pub const NP_BIND_SIDE_STITCH: c_int = 2;
pub const NP_BIND_WIRE_O: c_int = 3;

/// Validation mode constants
pub const NP_VALIDATE_STRUCTURE: c_int = 0;
pub const NP_VALIDATE_FULL: c_int = 1;
pub const NP_VALIDATE_PDFA: c_int = 2;

// ============================================================================
// Rectangle Structure
// ============================================================================

/// Rectangle for page box dimensions
#[repr(C)]
pub struct NpRectangle {
    pub llx: c_float,
    pub lly: c_float,
    pub urx: c_float,
    pub ury: c_float,
}

// ============================================================================
// Validation Result Structure
// ============================================================================

/// Validation result
#[repr(C)]
pub struct NpValidationResult {
    pub valid: c_int,
    pub page_count: c_int,
    pub error_count: c_int,
    pub warning_count: c_int,
    pub repairs_applied: c_int,
}

// ============================================================================
// Opaque Handle Types
// ============================================================================

/// Page box manager handle
pub type PageBoxManagerHandle = Handle;

/// N-Up generator handle
pub type NupGeneratorHandle = Handle;

/// Booklet generator handle
pub type BookletGeneratorHandle = Handle;

/// Poster generator handle
pub type PosterGeneratorHandle = Handle;

/// PDF validator handle
pub type ValidatorHandle = Handle;

// ============================================================================
// Page Box Functions
// ============================================================================

/// Create a page box manager for a PDF file
///
/// # Safety
/// `pdf_path` must be a valid null-terminated UTF-8 string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mp_page_box_manager_create(
    pdf_path: *const c_char,
) -> PageBoxManagerHandle {
    if pdf_path.is_null() {
        return 0;
    }

    let path = match CStr::from_ptr(pdf_path).to_str() {
        Ok(s) => s,
        Err(_) => return 0,
    };

    use crate::enhanced::page_boxes::PageBoxManager;

    match PageBoxManager::new(path) {
        Ok(manager) => Box::into_raw(Box::new(manager)) as PageBoxManagerHandle,
        Err(_) => 0,
    }
}

/// Free a page box manager
///
/// # Safety
/// `handle` must be a valid handle created by `mp_page_box_manager_create`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mp_page_box_manager_free(handle: PageBoxManagerHandle) {
    if handle != 0 {
        use crate::enhanced::page_boxes::PageBoxManager;
        drop(Box::from_raw(handle as *mut PageBoxManager));
    }
}

/// Get page count from page box manager
///
/// # Safety
/// `handle` must be a valid handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mp_page_box_manager_page_count(handle: PageBoxManagerHandle) -> c_int {
    if handle == 0 {
        return 0;
    }

    use crate::enhanced::page_boxes::PageBoxManager;
    let manager = &*(handle as *const PageBoxManager);
    manager.page_count() as c_int
}

/// Get a page box
///
/// # Safety
/// `handle` must be valid. `rect_out` must point to valid memory.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mp_page_box_get(
    handle: PageBoxManagerHandle,
    page: c_int,
    box_type: c_int,
    rect_out: *mut NpRectangle,
) -> c_int {
    if handle == 0 || rect_out.is_null() {
        return -1;
    }

    use crate::enhanced::page_boxes::{BoxType, PageBoxManager};

    let manager = &*(handle as *const PageBoxManager);

    let bt = match box_type {
        NP_BOX_MEDIA => BoxType::MediaBox,
        NP_BOX_CROP => BoxType::CropBox,
        NP_BOX_BLEED => BoxType::BleedBox,
        NP_BOX_TRIM => BoxType::TrimBox,
        NP_BOX_ART => BoxType::ArtBox,
        _ => return -2,
    };

    if let Some(boxes) = manager.get_page_boxes(page as usize) {
        if let Some(rect) = boxes.get(bt) {
            (*rect_out).llx = rect.llx;
            (*rect_out).lly = rect.lly;
            (*rect_out).urx = rect.urx;
            (*rect_out).ury = rect.ury;
            return 0;
        }
    }

    -3
}

/// Set a page box
///
/// # Safety
/// `handle` must be valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mp_page_box_set(
    handle: PageBoxManagerHandle,
    page: c_int,
    box_type: c_int,
    llx: c_float,
    lly: c_float,
    urx: c_float,
    ury: c_float,
) -> c_int {
    if handle == 0 {
        return -1;
    }

    use crate::enhanced::page_boxes::{BoxType, PageBoxManager, Rectangle};

    let manager = &mut *(handle as *mut PageBoxManager);

    let bt = match box_type {
        NP_BOX_MEDIA => BoxType::MediaBox,
        NP_BOX_CROP => BoxType::CropBox,
        NP_BOX_BLEED => BoxType::BleedBox,
        NP_BOX_TRIM => BoxType::TrimBox,
        NP_BOX_ART => BoxType::ArtBox,
        _ => return -2,
    };

    let rect = Rectangle::new(llx, lly, urx, ury);

    match manager.set_box(page as usize, bt, rect) {
        Ok(_) => 0,
        Err(_) => -3,
    }
}

/// Add bleed to all pages
///
/// # Safety
/// `handle` must be valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mp_page_box_add_bleed(
    handle: PageBoxManagerHandle,
    bleed: c_float,
    unit: c_int,
) -> c_int {
    if handle == 0 {
        return -1;
    }

    use crate::enhanced::page_boxes::{PageBoxManager, Unit};

    let manager = &mut *(handle as *mut PageBoxManager);

    let u = match unit {
        NP_UNIT_POINT => Unit::Point,
        NP_UNIT_INCH => Unit::Inch,
        NP_UNIT_MM => Unit::Mm,
        NP_UNIT_CM => Unit::Cm,
        _ => Unit::Point,
    };

    manager.add_bleed(bleed, u);
    0
}

/// Save page box changes
///
/// # Safety
/// `handle` and `output_path` must be valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mp_page_box_save(
    handle: PageBoxManagerHandle,
    output_path: *const c_char,
) -> c_int {
    if handle == 0 || output_path.is_null() {
        return -1;
    }

    let path = match CStr::from_ptr(output_path).to_str() {
        Ok(s) => s,
        Err(_) => return -2,
    };

    use crate::enhanced::page_boxes::PageBoxManager;
    let manager = &*(handle as *const PageBoxManager);

    match manager.save(path) {
        Ok(_) => 0,
        Err(_) => -3,
    }
}

// ============================================================================
// N-Up Functions
// ============================================================================

/// Create N-Up layout
///
/// # Safety
/// All pointers must be valid null-terminated UTF-8 strings.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mp_create_nup(
    input_path: *const c_char,
    output_path: *const c_char,
    cols: c_int,
    rows: c_int,
    page_size: c_int,
) -> c_int {
    if input_path.is_null() || output_path.is_null() {
        return -1;
    }

    let input = match CStr::from_ptr(input_path).to_str() {
        Ok(s) => s,
        Err(_) => return -2,
    };

    let output = match CStr::from_ptr(output_path).to_str() {
        Ok(s) => s,
        Err(_) => return -2,
    };

    use crate::enhanced::nup::{NupLayout, NupOptions, create_grid};
    use crate::enhanced::page_boxes::PageSize;

    let size = match page_size {
        NP_SIZE_LETTER => PageSize::Letter,
        NP_SIZE_LEGAL => PageSize::Legal,
        NP_SIZE_A4 => PageSize::A4,
        NP_SIZE_A3 => PageSize::A3,
        NP_SIZE_A5 => PageSize::A5,
        NP_SIZE_TABLOID => PageSize::Tabloid,
        _ => PageSize::Letter,
    };

    match create_grid(input, output, cols as usize, rows as usize, size) {
        Ok(_) => 0,
        Err(_) => -3,
    }
}

/// Create 2-up layout
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mp_create_2up(
    input_path: *const c_char,
    output_path: *const c_char,
    page_size: c_int,
) -> c_int {
    mp_create_nup(input_path, output_path, 1, 2, page_size)
}

/// Create 4-up layout
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mp_create_4up(
    input_path: *const c_char,
    output_path: *const c_char,
    page_size: c_int,
) -> c_int {
    mp_create_nup(input_path, output_path, 2, 2, page_size)
}

/// Create 9-up layout
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mp_create_9up(
    input_path: *const c_char,
    output_path: *const c_char,
    page_size: c_int,
) -> c_int {
    mp_create_nup(input_path, output_path, 3, 3, page_size)
}

// ============================================================================
// Booklet Functions
// ============================================================================

/// Create saddle-stitch booklet
///
/// # Safety
/// All pointers must be valid null-terminated UTF-8 strings.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mp_create_saddle_stitch_booklet(
    input_path: *const c_char,
    output_path: *const c_char,
) -> c_int {
    if input_path.is_null() || output_path.is_null() {
        return -1;
    }

    let input = match CStr::from_ptr(input_path).to_str() {
        Ok(s) => s,
        Err(_) => return -2,
    };

    let output = match CStr::from_ptr(output_path).to_str() {
        Ok(s) => s,
        Err(_) => return -2,
    };

    use crate::enhanced::booklet::create_saddle_stitch_booklet;

    match create_saddle_stitch_booklet(input, output) {
        Ok(_) => 0,
        Err(_) => -3,
    }
}

/// Create booklet with options
///
/// # Safety
/// All pointers must be valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mp_create_booklet(
    input_path: *const c_char,
    output_path: *const c_char,
    binding_type: c_int,
    page_size: c_int,
    add_blanks: c_int,
) -> c_int {
    if input_path.is_null() || output_path.is_null() {
        return -1;
    }

    let input = match CStr::from_ptr(input_path).to_str() {
        Ok(s) => s,
        Err(_) => return -2,
    };

    let output = match CStr::from_ptr(output_path).to_str() {
        Ok(s) => s,
        Err(_) => return -2,
    };

    use crate::enhanced::booklet::{BindingMethod, BookletOptions, create_booklet_with_options};
    use crate::enhanced::page_boxes::PageSize;

    let binding = match binding_type {
        NP_BIND_SADDLE_STITCH => BindingMethod::SaddleStitch,
        NP_BIND_PERFECT => BindingMethod::PerfectBinding,
        NP_BIND_SIDE_STITCH => BindingMethod::SideStitch,
        NP_BIND_WIRE_O => BindingMethod::WireO,
        _ => BindingMethod::SaddleStitch,
    };

    let size = match page_size {
        NP_SIZE_LETTER => PageSize::Letter,
        NP_SIZE_LEGAL => PageSize::Legal,
        NP_SIZE_A4 => PageSize::A4,
        NP_SIZE_A3 => PageSize::A3,
        NP_SIZE_TABLOID => PageSize::Tabloid,
        _ => PageSize::Letter,
    };

    let options = BookletOptions::new()
        .binding(binding)
        .sheet_size(size)
        .add_blank_pages(add_blanks != 0);

    match create_booklet_with_options(input, output, &options) {
        Ok(_) => 0,
        Err(_) => -3,
    }
}

// ============================================================================
// Poster/Tiling Functions
// ============================================================================

/// Create poster tiles
///
/// # Safety
/// All pointers must be valid null-terminated UTF-8 strings.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mp_create_poster(
    input_path: *const c_char,
    output_path: *const c_char,
    tile_size: c_int,
    overlap_mm: c_float,
    cut_marks: c_int,
) -> c_int {
    if input_path.is_null() || output_path.is_null() {
        return -1;
    }

    let input = match CStr::from_ptr(input_path).to_str() {
        Ok(s) => s,
        Err(_) => return -2,
    };

    let output = match CStr::from_ptr(output_path).to_str() {
        Ok(s) => s,
        Err(_) => return -2,
    };

    use crate::enhanced::page_boxes::{PageSize, Unit};
    use crate::enhanced::poster::{PosterOptions, create_poster_with_options};

    let size = match tile_size {
        NP_SIZE_LETTER => PageSize::Letter,
        NP_SIZE_LEGAL => PageSize::Legal,
        NP_SIZE_A4 => PageSize::A4,
        NP_SIZE_A3 => PageSize::A3,
        _ => PageSize::Letter,
    };

    let options = PosterOptions::new()
        .tile_size(size)
        .overlap(overlap_mm, Unit::Mm)
        .cut_marks(cut_marks != 0);

    match create_poster_with_options(input, output, &options) {
        Ok(_) => 0,
        Err(_) => -3,
    }
}

/// Calculate number of tiles needed for a poster
///
/// # Safety
/// `pdf_path` must be valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mp_poster_tile_count(
    pdf_path: *const c_char,
    tile_size: c_int,
    overlap_mm: c_float,
) -> c_int {
    if pdf_path.is_null() {
        return -1;
    }

    let path = match CStr::from_ptr(pdf_path).to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };

    use crate::enhanced::page_boxes::{PageSize, Unit};
    use crate::enhanced::poster::{PosterGenerator, PosterOptions};

    let size = match tile_size {
        NP_SIZE_LETTER => PageSize::Letter,
        NP_SIZE_A4 => PageSize::A4,
        _ => PageSize::Letter,
    };

    let options = PosterOptions::new()
        .tile_size(size)
        .overlap(overlap_mm, Unit::Mm);

    let mut generator = PosterGenerator::new(options);
    match generator.load(path) {
        Ok(_) => generator.tile_count() as c_int,
        Err(_) => -1,
    }
}

// ============================================================================
// Validation Functions
// ============================================================================

/// Validate a PDF file
///
/// # Safety
/// `pdf_path` must be valid. `result_out` must point to valid memory.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mp_validate_pdf(
    pdf_path: *const c_char,
    mode: c_int,
    result_out: *mut NpValidationResult,
) -> c_int {
    if pdf_path.is_null() || result_out.is_null() {
        return -1;
    }

    let path = match CStr::from_ptr(pdf_path).to_str() {
        Ok(s) => s,
        Err(_) => return -2,
    };

    use crate::enhanced::validation::{ValidationOptions, validate_pdf, validate_pdf_with_options};

    let options = match mode {
        NP_VALIDATE_STRUCTURE => ValidationOptions::new().structure_only(),
        NP_VALIDATE_PDFA => ValidationOptions::new().check_pdfa("2b"),
        _ => ValidationOptions::new(),
    };

    match validate_pdf_with_options(path, &options) {
        Ok(result) => {
            (*result_out).valid = if result.is_valid { 1 } else { 0 };
            (*result_out).page_count = result.page_count as c_int;
            (*result_out).error_count = result.error_count as c_int;
            (*result_out).warning_count = result.warning_count as c_int;
            (*result_out).repairs_applied = if result.repairs_applied { 1 } else { 0 };
            0
        }
        Err(_) => -3,
    }
}

/// Quick validation check
///
/// # Safety
/// `pdf_path` must be valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mp_quick_validate(pdf_path: *const c_char) -> c_int {
    if pdf_path.is_null() {
        return -1;
    }

    let path = match CStr::from_ptr(pdf_path).to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };

    use crate::enhanced::validation::quick_validate;

    match quick_validate(path) {
        Ok(valid) => {
            if valid {
                1
            } else {
                0
            }
        }
        Err(_) => -1,
    }
}

/// Repair a PDF file
///
/// # Safety
/// All pointers must be valid null-terminated UTF-8 strings.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mp_repair_pdf(
    input_path: *const c_char,
    output_path: *const c_char,
) -> c_int {
    if input_path.is_null() || output_path.is_null() {
        return -1;
    }

    let input = match CStr::from_ptr(input_path).to_str() {
        Ok(s) => s,
        Err(_) => return -2,
    };

    let output = match CStr::from_ptr(output_path).to_str() {
        Ok(s) => s,
        Err(_) => return -2,
    };

    use crate::enhanced::validation::{RepairOptions, repair_pdf};

    match repair_pdf(input, output, &RepairOptions::all()) {
        Ok(_) => 0,
        Err(_) => -3,
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constants() {
        assert_eq!(NP_BOX_MEDIA, 0);
        assert_eq!(NP_BOX_CROP, 1);
        assert_eq!(NP_BOX_BLEED, 2);
        assert_eq!(NP_BOX_TRIM, 3);
        assert_eq!(NP_BOX_ART, 4);
    }

    #[test]
    fn test_unit_constants() {
        assert_eq!(NP_UNIT_POINT, 0);
        assert_eq!(NP_UNIT_INCH, 1);
        assert_eq!(NP_UNIT_MM, 2);
        assert_eq!(NP_UNIT_CM, 3);
    }

    #[test]
    fn test_page_size_constants() {
        assert_eq!(NP_SIZE_LETTER, 0);
        assert_eq!(NP_SIZE_A4, 2);
    }

    #[test]
    fn test_binding_constants() {
        assert_eq!(NP_BIND_SADDLE_STITCH, 0);
        assert_eq!(NP_BIND_PERFECT, 1);
    }

    #[test]
    fn test_validation_constants() {
        assert_eq!(NP_VALIDATE_STRUCTURE, 0);
        assert_eq!(NP_VALIDATE_FULL, 1);
        assert_eq!(NP_VALIDATE_PDFA, 2);
    }

    #[test]
    fn test_rectangle_struct() {
        let rect = NpRectangle {
            llx: 0.0,
            lly: 0.0,
            urx: 612.0,
            ury: 792.0,
        };
        assert_eq!(rect.llx, 0.0);
        assert_eq!(rect.urx, 612.0);
    }

    #[test]
    fn test_validation_result_struct() {
        let result = NpValidationResult {
            valid: 1,
            page_count: 5,
            error_count: 0,
            warning_count: 2,
            repairs_applied: 0,
        };
        assert_eq!(result.valid, 1);
        assert_eq!(result.page_count, 5);
    }

    #[test]
    fn test_null_safety() {
        unsafe {
            assert_eq!(mp_page_box_manager_create(ptr::null()), 0);
            assert_eq!(mp_page_box_manager_page_count(0), 0);
            assert_eq!(mp_create_nup(ptr::null(), ptr::null(), 2, 2, 0), -1);
            assert_eq!(mp_quick_validate(ptr::null()), -1);
        }
    }
}
