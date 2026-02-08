//! FFI Compatibility Aliases
//!
//! This module provides compatibility aliases for functions that have different
//! names in different APIs (e.g., Go bindings expecting MuPDF-style names).
//!
//! These are thin wrappers that call the actual implementations.

use super::buffer::Buffer;
use super::colorspace::{ColorspaceHandle, FZ_COLORSPACE_RGB};
use super::cookie::{
    fz_cookie_abort, fz_cookie_get_progress, fz_cookie_reset, fz_cookie_should_abort,
};
use super::document::{Document, PAGES, fz_count_pages, fz_load_page};
use super::geometry::fz_matrix;
use super::pixmap::Pixmap;
use super::stext::{Rect, STEXT_PAGES, StextPage, fz_stext_page_as_text};
use super::{BUFFERS, DOCUMENTS, Handle, PIXMAPS};
use std::os::raw::c_char;

// ============================================================================
// Cookie Compatibility Aliases
// ============================================================================

/// Alias for fz_cookie_abort (MuPDF naming convention)
#[unsafe(no_mangle)]
pub extern "C" fn fz_abort_cookie(ctx: Handle, cookie: Handle) {
    fz_cookie_abort(ctx, cookie)
}

/// Alias for fz_cookie_should_abort (MuPDF naming convention)
#[unsafe(no_mangle)]
pub extern "C" fn fz_cookie_is_aborted(ctx: Handle, cookie: Handle) -> i32 {
    fz_cookie_should_abort(ctx, cookie)
}

/// Alias for fz_cookie_get_progress (MuPDF naming convention)
#[unsafe(no_mangle)]
pub extern "C" fn fz_cookie_progress(ctx: Handle, cookie: Handle) -> i32 {
    fz_cookie_get_progress(ctx, cookie)
}

/// Alias for fz_cookie_reset (MuPDF naming convention)
#[unsafe(no_mangle)]
pub extern "C" fn fz_reset_cookie(ctx: Handle, cookie: Handle) {
    fz_cookie_reset(ctx, cookie)
}

// ============================================================================
// Document Compatibility Functions
// ============================================================================

/// Open document from a buffer
///
/// # Safety
/// Caller must ensure magic is a valid null-terminated C string.
#[unsafe(no_mangle)]
pub extern "C" fn fz_open_document_with_buffer(
    _ctx: Handle,
    _magic: *const c_char,
    data: *const u8,
    len: usize,
) -> Handle {
    if data.is_null() || len == 0 {
        return 0;
    }

    // SAFETY: Caller guarantees data points to readable memory of len bytes
    let slice = unsafe { std::slice::from_raw_parts(data, len) };
    let vec = slice.to_vec();

    let doc = Document::new(vec);
    DOCUMENTS.insert(doc)
}

/// Lookup a named destination in PDF
///
/// # Safety
/// Caller must ensure name is a valid null-terminated C string.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_lookup_named_dest(_ctx: Handle, _doc: Handle, name: *const c_char) -> i32 {
    if name.is_null() {
        return -1;
    }

    // Named destinations would need to be parsed from the PDF
    // For now, return -1 (not found) as a placeholder
    // A full implementation would search the PDF's name tree
    -1
}

// ============================================================================
// Text Extraction Compatibility Functions
// ============================================================================

/// Create stext page from a document page
///
/// This extracts text from a PDF page into a structured text page.
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_stext_page_from_page(
    _ctx: Handle,
    page: Handle,
    _options: *const std::ffi::c_void,
) -> Handle {
    // Get page info to determine bounds
    let (page_width, page_height) = if let Some(page_arc) = PAGES.get(page) {
        if let Ok(guard) = page_arc.lock() {
            // bounds is [x0, y0, x1, y1]
            let w = guard.bounds[2] - guard.bounds[0];
            let h = guard.bounds[3] - guard.bounds[1];
            (w, h)
        } else {
            (612.0, 792.0) // Default to letter size
        }
    } else {
        (612.0, 792.0)
    };

    // Create a new stext page with the page bounds
    let stext_page = StextPage {
        refs: 1,
        mediabox: Rect {
            x0: 0.0,
            y0: 0.0,
            x1: page_width,
            y1: page_height,
        },
        blocks: Vec::new(),
    };

    // In a full implementation, this would extract text from the page
    // by parsing content streams. For now, return an empty stext page.
    STEXT_PAGES.insert(stext_page)
}

/// Convert stext page to buffer
///
/// Extracts text from stext page and stores in a buffer.
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_buffer_from_stext_page(ctx: Handle, stext: Handle) -> Handle {
    // Get text from stext page
    let text_ptr = fz_stext_page_as_text(ctx, stext);

    if text_ptr.is_null() {
        // Return empty buffer
        let buffer = Buffer::new(0);
        return BUFFERS.insert(buffer);
    }

    // SAFETY: fz_stext_page_as_text returns a valid C string
    let c_str = unsafe { std::ffi::CStr::from_ptr(text_ptr) };
    let text_bytes = c_str.to_bytes();

    // Create buffer with the text data
    let buffer = Buffer::from_data(text_bytes);

    BUFFERS.insert(buffer)
}

// ============================================================================
// Pixmap Compatibility Functions
// ============================================================================

/// Create pixmap from page
///
/// Renders a page to a pixmap with the given transformation and colorspace.
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_pixmap_from_page(
    _ctx: Handle,
    page: Handle,
    ctm: fz_matrix,
    cs: ColorspaceHandle,
    alpha: i32,
) -> Handle {
    // Get page dimensions
    let (width, height) = if let Some(page_arc) = PAGES.get(page) {
        if let Ok(guard) = page_arc.lock() {
            let page_w = guard.bounds[2] - guard.bounds[0];
            let page_h = guard.bounds[3] - guard.bounds[1];
            // Apply transformation to get final dimensions
            let w = (page_w * ctm.a.abs() + page_h * ctm.c.abs()).ceil() as i32;
            let h = (page_w * ctm.b.abs() + page_h * ctm.d.abs()).ceil() as i32;
            (w.max(1), h.max(1))
        } else {
            (612, 792)
        }
    } else {
        return 0;
    };

    // Use provided colorspace or default to RGB
    let colorspace = if cs != 0 { cs } else { FZ_COLORSPACE_RGB };

    // Create pixmap
    let pixmap = Pixmap::new(colorspace, width, height, alpha != 0);

    // In a full implementation, this would render the page content
    // to the pixmap. For now, return a blank pixmap.
    PIXMAPS.insert(pixmap)
}

#[cfg(test)]
mod tests {
    use super::super::cookie::fz_new_cookie;
    use super::*;

    #[test]
    fn test_cookie_aliases() {
        let cookie = fz_new_cookie(0);
        assert_ne!(cookie, 0);

        // Test aliases work
        assert_eq!(fz_cookie_is_aborted(0, cookie), 0);
        assert_eq!(fz_cookie_progress(0, cookie), 0);

        fz_abort_cookie(0, cookie);
        assert_eq!(fz_cookie_is_aborted(0, cookie), 1);

        fz_reset_cookie(0, cookie);
        assert_eq!(fz_cookie_is_aborted(0, cookie), 0);
    }

    #[test]
    fn test_open_document_with_buffer() {
        // Minimal valid PDF
        let pdf_data = b"%PDF-1.4\n1 0 obj<</Type/Catalog/Pages 2 0 R>>endobj\n2 0 obj<</Type/Pages/Count 1/Kids[3 0 R]>>endobj\n3 0 obj<</Type/Page/MediaBox[0 0 612 792]/Parent 2 0 R>>endobj\nxref\n0 4\n0000000000 65535 f \n0000000009 00000 n \n0000000052 00000 n \n0000000101 00000 n \ntrailer<</Size 4/Root 1 0 R>>\nstartxref\n178\n%%EOF";

        let doc =
            fz_open_document_with_buffer(0, std::ptr::null(), pdf_data.as_ptr(), pdf_data.len());
        assert_ne!(doc, 0);

        let page_count = fz_count_pages(0, doc);
        assert!(page_count >= 1);
    }

    #[test]
    fn test_new_stext_page_from_page() {
        // Create a document and page first
        let pdf_data = b"%PDF-1.4\n1 0 obj<</Type/Catalog/Pages 2 0 R>>endobj\n2 0 obj<</Type/Pages/Count 1/Kids[3 0 R]>>endobj\n3 0 obj<</Type/Page/MediaBox[0 0 612 792]/Parent 2 0 R>>endobj\nxref\n0 4\n0000000000 65535 f \n0000000009 00000 n \n0000000052 00000 n \n0000000101 00000 n \ntrailer<</Size 4/Root 1 0 R>>\nstartxref\n178\n%%EOF";

        let doc =
            fz_open_document_with_buffer(0, std::ptr::null(), pdf_data.as_ptr(), pdf_data.len());
        let page = fz_load_page(0, doc, 0);

        let stext = fz_new_stext_page_from_page(0, page, std::ptr::null());
        assert_ne!(stext, 0);
    }

    #[test]
    fn test_new_buffer_from_stext_page() {
        use super::super::stext::fz_new_stext_page;

        // Create stext page
        let stext = fz_new_stext_page(0, 0.0, 0.0, 612.0, 792.0);
        assert_ne!(stext, 0);

        // Convert to buffer
        let buf = fz_new_buffer_from_stext_page(0, stext);
        assert_ne!(buf, 0);
    }
}
