//! FFI exports for HTML to PDF conversion
//!
//! Provides C-compatible functions for converting HTML/CSS to PDF.

use crate::enhanced::html_to_pdf::{HtmlToPdfOptions, PageSize, html_file_to_pdf, html_to_pdf};
use crate::ffi::Handle;
use std::ffi::CStr;
use std::os::raw::c_char;

// ============================================================================
// Handle Types
// ============================================================================

/// HTML to PDF options handle
pub type HtmlToPdfOptionsHandle = Handle;

// ============================================================================
// Error Codes
// ============================================================================

/// Success
pub const MP_HTML_SUCCESS: i32 = 0;
/// Invalid parameter
pub const MP_HTML_ERROR_INVALID_PARAM: i32 = -1;
/// Conversion failed
pub const MP_HTML_ERROR_CONVERSION: i32 = -2;
/// File not found
pub const MP_HTML_ERROR_NOT_FOUND: i32 = -3;
/// IO error
pub const MP_HTML_ERROR_IO: i32 = -4;

// ============================================================================
// Page Size Constants
// ============================================================================

/// Letter page size (612x792 pt)
pub const MP_PAGE_SIZE_LETTER: i32 = 0;
/// Legal page size (612x1008 pt)
pub const MP_PAGE_SIZE_LEGAL: i32 = 1;
/// A3 page size
pub const MP_PAGE_SIZE_A3: i32 = 2;
/// A4 page size (595x842 pt)
pub const MP_PAGE_SIZE_A4: i32 = 3;
/// A5 page size
pub const MP_PAGE_SIZE_A5: i32 = 4;

// ============================================================================
// Options Functions
// ============================================================================

/// Create default HTML to PDF options
///
/// # Returns
/// Handle to the options, or 0 on failure
#[unsafe(no_mangle)]
pub extern "C" fn mp_html_options_create() -> HtmlToPdfOptionsHandle {
    let options = Box::new(HtmlToPdfOptions::default());
    Box::into_raw(options) as Handle
}

/// Free HTML to PDF options
///
/// # Safety
/// - `handle` must be a valid options handle
#[unsafe(no_mangle)]
pub extern "C" fn mp_html_options_free(handle: HtmlToPdfOptionsHandle) {
    if handle != 0 {
        unsafe {
            let _ = Box::from_raw(handle as *mut HtmlToPdfOptions);
        }
    }
}

/// Set page size
///
/// # Safety
/// - `handle` must be a valid options handle
#[unsafe(no_mangle)]
pub extern "C" fn mp_html_options_set_page_size(
    handle: HtmlToPdfOptionsHandle,
    page_size: i32,
) -> i32 {
    if handle == 0 {
        return MP_HTML_ERROR_INVALID_PARAM;
    }

    let options = unsafe { &mut *(handle as *mut HtmlToPdfOptions) };

    let size = match page_size {
        MP_PAGE_SIZE_LETTER => PageSize::Letter,
        MP_PAGE_SIZE_LEGAL => PageSize::Legal,
        MP_PAGE_SIZE_A3 => PageSize::A3,
        MP_PAGE_SIZE_A4 => PageSize::A4,
        MP_PAGE_SIZE_A5 => PageSize::A5,
        _ => return MP_HTML_ERROR_INVALID_PARAM,
    };

    let (w, h) = size.dimensions();
    options.page_width = w;
    options.page_height = h;

    MP_HTML_SUCCESS
}

/// Set custom page size in points
///
/// # Safety
/// - `handle` must be a valid options handle
#[unsafe(no_mangle)]
pub extern "C" fn mp_html_options_set_page_size_custom(
    handle: HtmlToPdfOptionsHandle,
    width: f32,
    height: f32,
) -> i32 {
    if handle == 0 {
        return MP_HTML_ERROR_INVALID_PARAM;
    }

    let options = unsafe { &mut *(handle as *mut HtmlToPdfOptions) };
    options.page_width = width;
    options.page_height = height;

    MP_HTML_SUCCESS
}

/// Set page margins in points
///
/// # Safety
/// - `handle` must be a valid options handle
#[unsafe(no_mangle)]
pub extern "C" fn mp_html_options_set_margins(
    handle: HtmlToPdfOptionsHandle,
    top: f32,
    right: f32,
    bottom: f32,
    left: f32,
) -> i32 {
    if handle == 0 {
        return MP_HTML_ERROR_INVALID_PARAM;
    }

    let options = unsafe { &mut *(handle as *mut HtmlToPdfOptions) };
    options.margin_top = top;
    options.margin_right = right;
    options.margin_bottom = bottom;
    options.margin_left = left;

    MP_HTML_SUCCESS
}

/// Set scale factor
///
/// # Safety
/// - `handle` must be a valid options handle
#[unsafe(no_mangle)]
pub extern "C" fn mp_html_options_set_scale(handle: HtmlToPdfOptionsHandle, scale: f32) -> i32 {
    if handle == 0 {
        return MP_HTML_ERROR_INVALID_PARAM;
    }

    let options = unsafe { &mut *(handle as *mut HtmlToPdfOptions) };
    options.scale = scale;

    MP_HTML_SUCCESS
}

/// Enable/disable JavaScript execution
///
/// # Safety
/// - `handle` must be a valid options handle
#[unsafe(no_mangle)]
pub extern "C" fn mp_html_options_set_javascript(
    handle: HtmlToPdfOptionsHandle,
    enabled: i32,
) -> i32 {
    if handle == 0 {
        return MP_HTML_ERROR_INVALID_PARAM;
    }

    let options = unsafe { &mut *(handle as *mut HtmlToPdfOptions) };
    options.enable_javascript = enabled != 0;

    MP_HTML_SUCCESS
}

/// Enable/disable print background
///
/// # Safety
/// - `handle` must be a valid options handle
#[unsafe(no_mangle)]
pub extern "C" fn mp_html_options_set_print_background(
    handle: HtmlToPdfOptionsHandle,
    enabled: i32,
) -> i32 {
    if handle == 0 {
        return MP_HTML_ERROR_INVALID_PARAM;
    }

    let options = unsafe { &mut *(handle as *mut HtmlToPdfOptions) };
    options.print_background = enabled != 0;

    MP_HTML_SUCCESS
}

/// Enable/disable landscape orientation
///
/// # Safety
/// - `handle` must be a valid options handle
#[unsafe(no_mangle)]
pub extern "C" fn mp_html_options_set_landscape(
    handle: HtmlToPdfOptionsHandle,
    landscape: i32,
) -> i32 {
    if handle == 0 {
        return MP_HTML_ERROR_INVALID_PARAM;
    }

    let options = unsafe { &mut *(handle as *mut HtmlToPdfOptions) };
    if landscape != 0 && !options.landscape {
        std::mem::swap(&mut options.page_width, &mut options.page_height);
        options.landscape = true;
    } else if landscape == 0 && options.landscape {
        std::mem::swap(&mut options.page_width, &mut options.page_height);
        options.landscape = false;
    }

    MP_HTML_SUCCESS
}

/// Set user stylesheet
///
/// # Safety
/// - `handle` must be a valid options handle
/// - `css` must be a valid null-terminated UTF-8 string
#[unsafe(no_mangle)]
pub extern "C" fn mp_html_options_set_stylesheet(
    handle: HtmlToPdfOptionsHandle,
    css: *const c_char,
) -> i32 {
    if handle == 0 || css.is_null() {
        return MP_HTML_ERROR_INVALID_PARAM;
    }

    let options = unsafe { &mut *(handle as *mut HtmlToPdfOptions) };
    let css_str = unsafe { CStr::from_ptr(css) }.to_string_lossy();
    options.user_stylesheet = Some(css_str.to_string());

    MP_HTML_SUCCESS
}

/// Set base URL for resolving relative paths
///
/// # Safety
/// - `handle` must be a valid options handle
/// - `url` must be a valid null-terminated UTF-8 string
#[unsafe(no_mangle)]
pub extern "C" fn mp_html_options_set_base_url(
    handle: HtmlToPdfOptionsHandle,
    url: *const c_char,
) -> i32 {
    if handle == 0 || url.is_null() {
        return MP_HTML_ERROR_INVALID_PARAM;
    }

    let options = unsafe { &mut *(handle as *mut HtmlToPdfOptions) };
    let url_str = unsafe { CStr::from_ptr(url) }.to_string_lossy();
    options.base_url = Some(url_str.to_string());

    MP_HTML_SUCCESS
}

/// Set header HTML
///
/// # Safety
/// - `handle` must be a valid options handle
/// - `html` must be a valid null-terminated UTF-8 string
#[unsafe(no_mangle)]
pub extern "C" fn mp_html_options_set_header(
    handle: HtmlToPdfOptionsHandle,
    html: *const c_char,
) -> i32 {
    if handle == 0 || html.is_null() {
        return MP_HTML_ERROR_INVALID_PARAM;
    }

    let options = unsafe { &mut *(handle as *mut HtmlToPdfOptions) };
    let html_str = unsafe { CStr::from_ptr(html) }.to_string_lossy();
    options.header_html = Some(html_str.to_string());

    MP_HTML_SUCCESS
}

/// Set footer HTML
///
/// # Safety
/// - `handle` must be a valid options handle
/// - `html` must be a valid null-terminated UTF-8 string
#[unsafe(no_mangle)]
pub extern "C" fn mp_html_options_set_footer(
    handle: HtmlToPdfOptionsHandle,
    html: *const c_char,
) -> i32 {
    if handle == 0 || html.is_null() {
        return MP_HTML_ERROR_INVALID_PARAM;
    }

    let options = unsafe { &mut *(handle as *mut HtmlToPdfOptions) };
    let html_str = unsafe { CStr::from_ptr(html) }.to_string_lossy();
    options.footer_html = Some(html_str.to_string());

    MP_HTML_SUCCESS
}

// ============================================================================
// Conversion Functions
// ============================================================================

/// Convert HTML string to PDF
///
/// # Safety
/// - `html` must be a valid null-terminated UTF-8 string
/// - `output_path` must be a valid null-terminated UTF-8 string
/// - `options` must be a valid options handle, or 0 for defaults
#[unsafe(no_mangle)]
pub extern "C" fn mp_html_to_pdf(
    html: *const c_char,
    output_path: *const c_char,
    options: HtmlToPdfOptionsHandle,
) -> i32 {
    if html.is_null() || output_path.is_null() {
        return MP_HTML_ERROR_INVALID_PARAM;
    }

    let html_str = unsafe { CStr::from_ptr(html) }.to_string_lossy();
    let output_str = unsafe { CStr::from_ptr(output_path) }.to_string_lossy();

    let opts = if options == 0 {
        HtmlToPdfOptions::default()
    } else {
        unsafe { &*(options as *const HtmlToPdfOptions) }.clone()
    };

    match html_to_pdf(&html_str, &output_str, &opts) {
        Ok(()) => MP_HTML_SUCCESS,
        Err(_) => MP_HTML_ERROR_CONVERSION,
    }
}

/// Convert HTML file to PDF
///
/// # Safety
/// - `html_path` must be a valid null-terminated UTF-8 string
/// - `output_path` must be a valid null-terminated UTF-8 string
/// - `options` must be a valid options handle, or 0 for defaults
#[unsafe(no_mangle)]
pub extern "C" fn mp_html_file_to_pdf(
    html_path: *const c_char,
    output_path: *const c_char,
    options: HtmlToPdfOptionsHandle,
) -> i32 {
    if html_path.is_null() || output_path.is_null() {
        return MP_HTML_ERROR_INVALID_PARAM;
    }

    let html_path_str = unsafe { CStr::from_ptr(html_path) }.to_string_lossy();
    let output_str = unsafe { CStr::from_ptr(output_path) }.to_string_lossy();

    let opts = if options == 0 {
        HtmlToPdfOptions::default()
    } else {
        unsafe { &*(options as *const HtmlToPdfOptions) }.clone()
    };

    match html_file_to_pdf(&html_path_str, &output_str, &opts) {
        Ok(()) => MP_HTML_SUCCESS,
        Err(e) => {
            if e.to_string().contains("not found") {
                MP_HTML_ERROR_NOT_FOUND
            } else {
                MP_HTML_ERROR_CONVERSION
            }
        }
    }
}

/// Get page width from options
///
/// # Safety
/// - `handle` must be a valid options handle
#[unsafe(no_mangle)]
pub extern "C" fn mp_html_options_get_page_width(handle: HtmlToPdfOptionsHandle) -> f32 {
    if handle == 0 {
        return 0.0;
    }
    let options = unsafe { &*(handle as *const HtmlToPdfOptions) };
    options.page_width
}

/// Get page height from options
///
/// # Safety
/// - `handle` must be a valid options handle
#[unsafe(no_mangle)]
pub extern "C" fn mp_html_options_get_page_height(handle: HtmlToPdfOptionsHandle) -> f32 {
    if handle == 0 {
        return 0.0;
    }
    let options = unsafe { &*(handle as *const HtmlToPdfOptions) };
    options.page_height
}

/// Get content width (page width minus margins)
///
/// # Safety
/// - `handle` must be a valid options handle
#[unsafe(no_mangle)]
pub extern "C" fn mp_html_options_get_content_width(handle: HtmlToPdfOptionsHandle) -> f32 {
    if handle == 0 {
        return 0.0;
    }
    let options = unsafe { &*(handle as *const HtmlToPdfOptions) };
    options.content_width()
}

/// Get content height (page height minus margins)
///
/// # Safety
/// - `handle` must be a valid options handle
#[unsafe(no_mangle)]
pub extern "C" fn mp_html_options_get_content_height(handle: HtmlToPdfOptionsHandle) -> f32 {
    if handle == 0 {
        return 0.0;
    }
    let options = unsafe { &*(handle as *const HtmlToPdfOptions) };
    options.content_height()
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    #[test]
    fn test_options_create_free() {
        let handle = mp_html_options_create();
        assert_ne!(handle, 0);
        mp_html_options_free(handle);
    }

    #[test]
    fn test_options_page_size() {
        let handle = mp_html_options_create();
        assert_eq!(
            mp_html_options_set_page_size(handle, MP_PAGE_SIZE_A4),
            MP_HTML_SUCCESS
        );

        let width = mp_html_options_get_page_width(handle);
        assert!((width - 595.28).abs() < 0.01);

        mp_html_options_free(handle);
    }

    #[test]
    fn test_options_margins() {
        let handle = mp_html_options_create();
        assert_eq!(
            mp_html_options_set_margins(handle, 72.0, 72.0, 72.0, 72.0),
            MP_HTML_SUCCESS
        );

        let content_width = mp_html_options_get_content_width(handle);
        assert!((content_width - (612.0 - 144.0)).abs() < 0.01);

        mp_html_options_free(handle);
    }

    #[test]
    fn test_options_scale() {
        let handle = mp_html_options_create();
        assert_eq!(mp_html_options_set_scale(handle, 1.5), MP_HTML_SUCCESS);
        mp_html_options_free(handle);
    }

    #[test]
    fn test_options_landscape() {
        let handle = mp_html_options_create();
        let orig_width = mp_html_options_get_page_width(handle);
        let orig_height = mp_html_options_get_page_height(handle);

        assert_eq!(mp_html_options_set_landscape(handle, 1), MP_HTML_SUCCESS);

        let new_width = mp_html_options_get_page_width(handle);
        let new_height = mp_html_options_get_page_height(handle);

        assert!((new_width - orig_height).abs() < 0.01);
        assert!((new_height - orig_width).abs() < 0.01);

        mp_html_options_free(handle);
    }

    #[test]
    fn test_options_stylesheet() {
        let handle = mp_html_options_create();
        let css = CString::new("body { color: red; }").unwrap();
        assert_eq!(
            mp_html_options_set_stylesheet(handle, css.as_ptr()),
            MP_HTML_SUCCESS
        );
        mp_html_options_free(handle);
    }

    #[test]
    fn test_html_to_pdf_basic() {
        let html = CString::new("<html><body><h1>Hello</h1></body></html>").unwrap();
        let output = CString::new("/tmp/test_html_output.pdf").unwrap();

        let result = mp_html_to_pdf(html.as_ptr(), output.as_ptr(), 0);
        assert_eq!(result, MP_HTML_SUCCESS);

        // Cleanup
        let _ = std::fs::remove_file("/tmp/test_html_output.pdf");
    }

    #[test]
    fn test_null_params() {
        assert_eq!(
            mp_html_options_set_page_size(0, MP_PAGE_SIZE_A4),
            MP_HTML_ERROR_INVALID_PARAM
        );
        assert_eq!(
            mp_html_to_pdf(std::ptr::null(), std::ptr::null(), 0),
            MP_HTML_ERROR_INVALID_PARAM
        );
    }
}
