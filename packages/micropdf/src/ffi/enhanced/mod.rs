//! Enhanced FFI - Functions beyond MuPDF API with `mp_` prefix
//!
//! This module provides additional PDF manipulation functions that go beyond
//! the MuPDF API, using the `mp_` prefix to distinguish them.
//!
//! ## Module Structure
//!
//! - **Document Operations**: `mp_merge_pdfs`, `mp_split_pdf`, etc.
//! - **Security** (`security`): Digital signatures, encryption, permissions

use super::Handle;
use crate::enhanced::page_ops;
use std::ffi::CStr;

// Sub-modules
pub mod document_composition;
pub mod html_to_pdf;
pub mod print_production;
pub mod security;

/// Write PDF to file
///
/// # Safety
/// Caller must ensure path is a valid null-terminated C string.
#[unsafe(no_mangle)]
pub extern "C" fn mp_write_pdf(_ctx: Handle, _doc: Handle, _path: *const std::ffi::c_char) -> i32 {
    // Placeholder for PDF writing functionality
    // This would use the enhanced PdfWriter
    0
}

/// Add blank page to PDF
#[unsafe(no_mangle)]
pub extern "C" fn mp_add_blank_page(_ctx: Handle, _doc: Handle, width: f32, height: f32) -> i32 {
    if width <= 0.0 || height <= 0.0 {
        return -1;
    }
    // Placeholder - would use PdfWriter::add_blank_page
    0
}

/// Merge multiple PDFs into a single output file
///
/// # Arguments
/// * `_ctx` - Context handle (currently unused)
/// * `paths` - Pointer to array of null-terminated C string paths
/// * `count` - Number of PDF paths in the array
/// * `output_path` - Null-terminated C string path for merged output
///
/// # Returns
/// * Number of pages in the merged PDF on success
/// * -1 on error (invalid inputs, missing files, merge failure)
///
/// # Safety
/// Caller must ensure:
/// * `paths` points to an array of at least `count` valid C string pointers
/// * Each path pointer in the array points to a valid null-terminated C string
/// * `output_path` points to a valid null-terminated C string
/// * All pointed-to memory remains valid for the duration of the call
///
/// # Example
/// ```c
/// const char* inputs[] = {"doc1.pdf", "doc2.pdf", "doc3.pdf"};
/// int page_count = mp_merge_pdfs(ctx, inputs, 3, "merged.pdf");
/// if (page_count > 0) {
///     printf("Merged %d pages\n", page_count);
/// }
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn mp_merge_pdfs(
    _ctx: Handle,
    paths: *const *const std::ffi::c_char,
    count: i32,
    output_path: *const std::ffi::c_char,
) -> i32 {
    // Validate inputs
    if paths.is_null() || output_path.is_null() || count <= 0 {
        eprintln!("mp_merge_pdfs: Invalid parameters");
        return -1;
    }

    // Convert C strings to Rust Strings
    let mut input_paths = Vec::with_capacity(count as usize);

    for i in 0..count {
        // SAFETY: We check that paths is not null and i is within bounds
        let path_ptr = unsafe { *paths.offset(i as isize) };

        if path_ptr.is_null() {
            eprintln!("mp_merge_pdfs: Null path at index {}", i);
            return -1;
        }

        // SAFETY: We validated path_ptr is not null
        let path_str = match unsafe { CStr::from_ptr(path_ptr) }.to_str() {
            Ok(s) => s.to_string(),
            Err(e) => {
                eprintln!("mp_merge_pdfs: Invalid UTF-8 in path {}: {}", i, e);
                return -1;
            }
        };

        input_paths.push(path_str);
    }

    // Convert output path
    // SAFETY: We validated output_path is not null
    let output_str = match unsafe { CStr::from_ptr(output_path) }.to_str() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("mp_merge_pdfs: Invalid UTF-8 in output path: {}", e);
            return -1;
        }
    };

    // Perform the merge
    match page_ops::merge_pdf(&input_paths, output_str) {
        Ok(page_count) => page_count as i32,
        Err(e) => {
            eprintln!("mp_merge_pdfs: Merge failed: {}", e);
            -1
        }
    }
}

/// Split PDF into separate files
///
/// # Safety
/// Caller must ensure input_path and output_dir are valid null-terminated C strings.
#[unsafe(no_mangle)]
pub extern "C" fn mp_split_pdf(
    _ctx: Handle,
    input_path: *const std::ffi::c_char,
    output_dir: *const std::ffi::c_char,
) -> i32 {
    if input_path.is_null() || output_dir.is_null() {
        return -1;
    }

    let input = match unsafe { CStr::from_ptr(input_path) }.to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };

    let output = match unsafe { CStr::from_ptr(output_dir) }.to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };

    match page_ops::split_pdf(input, output) {
        Ok(_) => 0,
        Err(e) => {
            eprintln!("mp_split_pdf: Split failed: {:?}", e);
            -1
        }
    }
}

/// Add watermark to PDF pages
///
/// # Safety
/// Caller must ensure all string parameters are valid null-terminated C strings.
#[unsafe(no_mangle)]
pub extern "C" fn mp_add_watermark(
    _ctx: Handle,
    input_path: *const std::ffi::c_char,
    output_path: *const std::ffi::c_char,
    text: *const std::ffi::c_char,
    _x: f32,
    _y: f32,
    font_size: f32,
    opacity: f32,
) -> i32 {
    if input_path.is_null() || output_path.is_null() || text.is_null() {
        return -1;
    }

    if font_size <= 0.0 || !(0.0..=1.0).contains(&opacity) {
        return -1;
    }

    // Placeholder - would use Watermark::apply
    0
}

/// Overlay one PDF on top of another
///
/// Takes the content from overlay_path and places it on top of base_path,
/// writing the result to output_path. Each page of the overlay is placed
/// on the corresponding page of the base document.
///
/// # Safety
/// Caller must ensure all string parameters are valid null-terminated C strings.
#[unsafe(no_mangle)]
pub extern "C" fn mp_overlay_pdf(
    _ctx: Handle,
    base_path: *const std::ffi::c_char,
    output_path: *const std::ffi::c_char,
    overlay_path: *const std::ffi::c_char,
    _opacity: f32,
) -> i32 {
    if base_path.is_null() || output_path.is_null() || overlay_path.is_null() {
        return -1;
    }

    let base = match unsafe { CStr::from_ptr(base_path) }.to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };

    let output = match unsafe { CStr::from_ptr(output_path) }.to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };

    let overlay = match unsafe { CStr::from_ptr(overlay_path) }.to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };

    // Use the overlay module to merge overlay content onto base PDF
    // Empty pages array means apply overlay to all pages
    match crate::enhanced::overlay::merge_overlay(base, overlay, output, &[]) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("mp_overlay_pdf: Overlay merge failed: {:?}", e);
            -1
        }
    }
}

/// Optimize PDF (compress, remove duplicates, etc.)
///
/// # Safety
/// Caller must ensure path is a valid null-terminated C string.
#[unsafe(no_mangle)]
pub extern "C" fn mp_optimize_pdf(_ctx: Handle, path: *const std::ffi::c_char) -> i32 {
    if path.is_null() {
        return -1;
    }
    // Placeholder - would use optimization functions
    0
}

/// Linearize PDF for fast web viewing
///
/// # Safety
/// Caller must ensure path is a valid null-terminated C string.
#[unsafe(no_mangle)]
pub extern "C" fn mp_linearize_pdf(
    _ctx: Handle,
    input_path: *const std::ffi::c_char,
    output_path: *const std::ffi::c_char,
) -> i32 {
    if input_path.is_null() || output_path.is_null() {
        return -1;
    }
    // Placeholder - would use linearize function
    0
}

/// Draw line on PDF page
#[unsafe(no_mangle)]
pub extern "C" fn mp_draw_line(
    _ctx: Handle,
    _page: Handle,
    _x0: f32,
    _y0: f32,
    _x1: f32,
    _y1: f32,
    r: f32,
    g: f32,
    b: f32,
    alpha: f32,
    line_width: f32,
) -> i32 {
    if !(0.0..=1.0).contains(&r) || !(0.0..=1.0).contains(&g) || !(0.0..=1.0).contains(&b) {
        return -1;
    }

    if !(0.0..=1.0).contains(&alpha) {
        return -1;
    }

    if line_width <= 0.0 {
        return -1;
    }

    // Placeholder - would use DrawingContext::draw_line
    0
}

/// Draw rectangle on PDF page
#[unsafe(no_mangle)]
pub extern "C" fn mp_draw_rectangle(
    _ctx: Handle,
    _page: Handle,
    _x: f32,
    _y: f32,
    width: f32,
    height: f32,
    r: f32,
    g: f32,
    b: f32,
    alpha: f32,
    _fill: i32,
) -> i32 {
    if width <= 0.0 || height <= 0.0 {
        return -1;
    }

    if !(0.0..=1.0).contains(&r) || !(0.0..=1.0).contains(&g) || !(0.0..=1.0).contains(&b) {
        return -1;
    }

    if !(0.0..=1.0).contains(&alpha) {
        return -1;
    }

    // Placeholder - would use DrawingContext::draw_rect
    0
}

/// Draw circle on PDF page
#[unsafe(no_mangle)]
pub extern "C" fn mp_draw_circle(
    _ctx: Handle,
    _page: Handle,
    _x: f32,
    _y: f32,
    radius: f32,
    r: f32,
    g: f32,
    b: f32,
    alpha: f32,
    _fill: i32,
) -> i32 {
    if radius <= 0.0 {
        return -1;
    }

    if !(0.0..=1.0).contains(&r) || !(0.0..=1.0).contains(&g) || !(0.0..=1.0).contains(&b) {
        return -1;
    }

    if !(0.0..=1.0).contains(&alpha) {
        return -1;
    }

    // Placeholder - would use DrawingContext::draw_circle
    0
}

/// Highlight rectangle definition for creating overlay PDFs
#[repr(C)]
pub struct HighlightRect {
    pub page: i32,   // 0-based page number
    pub x: f32,      // X position from left (in points)
    pub y: f32,      // Y position from top (in points) - will be transformed to PDF coords
    pub width: f32,  // Width in points
    pub height: f32, // Height in points
    pub r: f32,      // Red component (0.0-1.0)
    pub g: f32,      // Green component (0.0-1.0)
    pub b: f32,      // Blue component (0.0-1.0)
    pub alpha: f32,  // Alpha/opacity (0.0-1.0)
}

/// Page dimensions for highlight overlay creation
#[repr(C)]
pub struct PageDim {
    pub width: f32,
    pub height: f32,
}

/// Create a highlight overlay PDF with colored, semi-transparent rectangles
///
/// This function creates a multi-page PDF with highlight rectangles that can
/// be overlaid onto another PDF using mp_overlay_pdf.
///
/// # Arguments
/// * `output_path` - Path for the output overlay PDF
/// * `page_dims` - Array of page dimensions (width, height in points)
/// * `page_count` - Number of pages
/// * `highlights` - Array of highlight rectangles
/// * `highlight_count` - Number of highlight rectangles
///
/// # Returns
/// * 0 on success
/// * -1 on error
///
/// # Safety
/// Caller must ensure:
/// - output_path is a valid null-terminated C string
/// - page_dims points to at least page_count elements
/// - highlights points to at least highlight_count elements
#[unsafe(no_mangle)]
pub extern "C" fn mp_create_highlight_overlay(
    output_path: *const std::ffi::c_char,
    page_dims: *const PageDim,
    page_count: i32,
    highlights: *const HighlightRect,
    highlight_count: i32,
) -> i32 {
    // Validate inputs
    if output_path.is_null() {
        eprintln!("mp_create_highlight_overlay: Null output path");
        return -1;
    }
    if page_count <= 0 {
        eprintln!("mp_create_highlight_overlay: Invalid page count");
        return -1;
    }
    if page_dims.is_null() {
        eprintln!("mp_create_highlight_overlay: Null page dimensions");
        return -1;
    }

    // Convert output path
    let output = match unsafe { CStr::from_ptr(output_path) }.to_str() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("mp_create_highlight_overlay: Invalid output path: {}", e);
            return -1;
        }
    };

    // Read page dimensions
    let dims: Vec<(f32, f32)> = (0..page_count as usize)
        .map(|i| {
            let dim = unsafe { &*page_dims.add(i) };
            (dim.width, dim.height)
        })
        .collect();

    // Read highlights
    let rects: Vec<HighlightRect> = if highlights.is_null() || highlight_count <= 0 {
        Vec::new()
    } else {
        (0..highlight_count as usize)
            .map(|i| unsafe { std::ptr::read(highlights.add(i)) })
            .collect()
    };

    // Group highlights by page
    let mut by_page: std::collections::HashMap<i32, Vec<&HighlightRect>> =
        std::collections::HashMap::new();
    for rect in &rects {
        by_page.entry(rect.page).or_default().push(rect);
    }

    // Create the overlay PDF
    let mut writer = crate::enhanced::writer::PdfWriter::new();

    for page_num in 0..page_count {
        let (width, height) = dims[page_num as usize];

        // Get highlights for this page
        let page_highlights: Vec<(f32, f32, f32, f32, f32, f32, f32, f32)> =
            if let Some(rects) = by_page.get(&page_num) {
                rects
                    .iter()
                    .filter(|r| {
                        // Validate color and alpha values
                        (0.0..=1.0).contains(&r.r)
                            && (0.0..=1.0).contains(&r.g)
                            && (0.0..=1.0).contains(&r.b)
                            && (0.0..=1.0).contains(&r.alpha)
                            && r.width > 0.0
                            && r.height > 0.0
                    })
                    .map(|r| (r.x, r.y, r.width, r.height, r.r, r.g, r.b, r.alpha))
                    .collect()
            } else {
                Vec::new()
            };

        // Add page (with or without highlights)
        if page_highlights.is_empty() {
            if let Err(e) = writer.add_blank_page(width, height) {
                eprintln!(
                    "mp_create_highlight_overlay: Failed to add blank page {}: {:?}",
                    page_num, e
                );
                return -1;
            }
        } else {
            if let Err(e) = writer.add_highlight_page(width, height, &page_highlights) {
                eprintln!(
                    "mp_create_highlight_overlay: Failed to add highlight page {}: {:?}",
                    page_num, e
                );
                return -1;
            }
        }
    }

    // Save the PDF
    if let Err(e) = writer.save(output) {
        eprintln!("mp_create_highlight_overlay: Failed to save PDF: {:?}", e);
        return -1;
    }

    0
}

/// Text element for creating text overlay PDFs
#[repr(C)]
pub struct TextOverlayElement {
    /// Text content (null-terminated C string)
    pub text: *const std::ffi::c_char,
    /// X position from left in points
    pub x: f32,
    /// Y position from top in points (will be converted to PDF coordinates)
    pub y: f32,
    /// Bounding box height in points (for proper vertical positioning)
    pub height: f32,
    /// Font size in points
    pub font_size: f32,
    /// Font name (null-terminated C string, use "F1" for default)
    pub font_name: *const std::ffi::c_char,
    /// Red component (0.0-1.0)
    pub r: f32,
    /// Green component (0.0-1.0)
    pub g: f32,
    /// Blue component (0.0-1.0)
    pub b: f32,
    /// Text render mode (0=visible, 3=invisible for OCR)
    pub render_mode: i32,
}

/// Image element for background in text overlay PDFs
#[repr(C)]
pub struct ImageOverlayElement {
    /// X position from left in points
    pub x: f32,
    /// Y position from top in points
    pub y: f32,
    /// Width in points
    pub width: f32,
    /// Height in points
    pub height: f32,
    /// Image data pointer
    pub data: *const u8,
    /// Image data length
    pub data_len: usize,
    /// Image format (0=PNG, 1=JPEG)
    pub format: i32,
}

/// Register a TTF font for use in text overlays
///
/// # Arguments
/// * `font_name` - Name to reference this font (e.g., "F1")
/// * `font_data` - TTF font file data
/// * `data_len` - Length of font data
///
/// # Returns
/// * Font handle (>0) on success
/// * 0 on error
///
/// # Safety
/// Caller must ensure font_name is a valid null-terminated string
/// and font_data points to valid memory of at least data_len bytes.
#[unsafe(no_mangle)]
pub extern "C" fn mp_register_font(
    font_name: *const std::ffi::c_char,
    font_data: *const u8,
    data_len: usize,
) -> u64 {
    use std::sync::LazyLock;
    use std::sync::Mutex;

    // Font registry - maps handle to (name, data)
    static FONT_REGISTRY: LazyLock<Mutex<std::collections::HashMap<u64, (String, Vec<u8>)>>> =
        LazyLock::new(|| Mutex::new(std::collections::HashMap::new()));
    static NEXT_HANDLE: LazyLock<Mutex<u64>> = LazyLock::new(|| Mutex::new(1));

    if font_name.is_null() || font_data.is_null() || data_len == 0 {
        eprintln!("mp_register_font: Invalid parameters");
        return 0;
    }

    let name = match unsafe { CStr::from_ptr(font_name) }.to_str() {
        Ok(s) => s.to_string(),
        Err(_) => return 0,
    };

    let data = unsafe { std::slice::from_raw_parts(font_data, data_len) }.to_vec();

    // Verify it's a valid TTF
    if ttf_parser::Face::parse(&data, 0).is_err() {
        eprintln!("mp_register_font: Invalid TTF data");
        return 0;
    }

    let mut registry = FONT_REGISTRY.lock().unwrap();
    let mut next = NEXT_HANDLE.lock().unwrap();
    let handle = *next;
    *next += 1;
    registry.insert(handle, (name, data));

    handle
}

/// Get registered font data by handle
fn get_registered_font(handle: u64) -> Option<(String, Vec<u8>)> {
    use std::sync::LazyLock;
    use std::sync::Mutex;

    static FONT_REGISTRY: LazyLock<Mutex<std::collections::HashMap<u64, (String, Vec<u8>)>>> =
        LazyLock::new(|| Mutex::new(std::collections::HashMap::new()));

    FONT_REGISTRY.lock().ok()?.get(&handle).cloned()
}

/// Create a text overlay PDF with text elements and optional background image
///
/// This function creates a single-page PDF with:
/// - Text positioned at specific coordinates (from OCR/Textract)
/// - Optional background image
/// - Text can be invisible (render_mode=3) for searchable PDFs
///
/// # Arguments
/// * `output_path` - Path for the output PDF
/// * `width` - Page width in points
/// * `height` - Page height in points
/// * `font_handles` - Array of font handles (from mp_register_font)
/// * `font_count` - Number of fonts
/// * `texts` - Array of text elements
/// * `text_count` - Number of text elements
/// * `image` - Optional background image (NULL if none)
///
/// # Returns
/// * 0 on success
/// * -1 on error
///
/// # Safety
/// Caller must ensure all pointers are valid and arrays have the specified counts.
#[unsafe(no_mangle)]
pub extern "C" fn mp_create_text_overlay(
    output_path: *const std::ffi::c_char,
    width: f32,
    height: f32,
    font_handles: *const u64,
    font_count: i32,
    texts: *const TextOverlayElement,
    text_count: i32,
    image: *const ImageOverlayElement,
) -> i32 {
    use crate::enhanced::writer::{ImageElement, ImageFormat, PdfWriter, TextElement};

    // Validate inputs
    if output_path.is_null() {
        eprintln!("mp_create_text_overlay: Null output path");
        return -1;
    }
    if width <= 0.0 || height <= 0.0 {
        eprintln!("mp_create_text_overlay: Invalid page dimensions");
        return -1;
    }

    let output = match unsafe { CStr::from_ptr(output_path) }.to_str() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("mp_create_text_overlay: Invalid output path: {}", e);
            return -1;
        }
    };

    let mut writer = PdfWriter::new();

    // Register fonts
    if !font_handles.is_null() && font_count > 0 {
        for i in 0..font_count as usize {
            let handle = unsafe { *font_handles.add(i) };
            if let Some((name, data)) = get_registered_font(handle) {
                if let Err(e) = writer.add_ttf_font(&name, data) {
                    eprintln!(
                        "mp_create_text_overlay: Failed to add font {}: {:?}",
                        name, e
                    );
                    // Continue with other fonts
                }
            }
        }
    }

    // Convert text elements
    let text_elements: Vec<TextElement> = if texts.is_null() || text_count <= 0 {
        Vec::new()
    } else {
        (0..text_count as usize)
            .filter_map(|i| {
                let elem = unsafe { &*texts.add(i) };

                let text = if elem.text.is_null() {
                    return None;
                } else {
                    match unsafe { CStr::from_ptr(elem.text) }.to_str() {
                        Ok(s) => s.to_string(),
                        Err(_) => return None,
                    }
                };

                let font_name = if elem.font_name.is_null() {
                    "F1".to_string()
                } else {
                    match unsafe { CStr::from_ptr(elem.font_name) }.to_str() {
                        Ok(s) => s.to_string(),
                        Err(_) => "F1".to_string(),
                    }
                };

                Some(TextElement {
                    text,
                    x: elem.x,
                    y: elem.y,
                    height: elem.height,
                    font_size: elem.font_size,
                    font_name,
                    color: (
                        elem.r.clamp(0.0, 1.0),
                        elem.g.clamp(0.0, 1.0),
                        elem.b.clamp(0.0, 1.0),
                    ),
                    render_mode: elem.render_mode,
                })
            })
            .collect()
    };

    // Convert image element if present
    let image_element = if image.is_null() {
        None
    } else {
        let img = unsafe { &*image };
        if img.data.is_null() || img.data_len == 0 {
            None
        } else {
            let data = unsafe { std::slice::from_raw_parts(img.data, img.data_len) }.to_vec();
            let format = if img.format == 1 {
                ImageFormat::Jpeg
            } else {
                ImageFormat::Png
            };
            Some(ImageElement::new(
                img.x, img.y, img.width, img.height, data, format,
            ))
        }
    };

    // Add the text overlay page
    if let Err(e) =
        writer.add_text_overlay_page(width, height, &text_elements, image_element.as_ref())
    {
        eprintln!("mp_create_text_overlay: Failed to add page: {:?}", e);
        return -1;
    }

    // Save the PDF
    if let Err(e) = writer.save(output) {
        eprintln!("mp_create_text_overlay: Failed to save PDF: {:?}", e);
        return -1;
    }

    0
}

/// Free a registered font
///
/// # Safety
/// Handle must be a valid font handle from mp_register_font.
#[unsafe(no_mangle)]
pub extern "C" fn mp_font_free(handle: u64) {
    use std::sync::LazyLock;
    use std::sync::Mutex;

    static FONT_REGISTRY: LazyLock<Mutex<std::collections::HashMap<u64, (String, Vec<u8>)>>> =
        LazyLock::new(|| Mutex::new(std::collections::HashMap::new()));

    if let Ok(mut registry) = FONT_REGISTRY.lock() {
        registry.remove(&handle);
    }
}

/// Restore bookmarks to a PDF file from JSON
///
/// # Arguments
/// * `input_path` - Path to input PDF
/// * `output_path` - Path to output PDF with bookmarks
/// * `bookmarks_json` - JSON string containing bookmarks array
///
/// # JSON Format
/// ```json
/// [
///   {"title": "Section 1", "page": 1, "children": [
///     {"title": "Subsection 1.1", "page": 2, "children": []}
///   ]}
/// ]
/// ```
///
/// # Returns
/// * 0 on success
/// * -1 on error
///
/// # Safety
/// Caller must ensure all paths and JSON are valid null-terminated C strings.
#[unsafe(no_mangle)]
pub extern "C" fn mp_restore_bookmarks(
    input_path: *const std::ffi::c_char,
    output_path: *const std::ffi::c_char,
    bookmarks_json: *const std::ffi::c_char,
) -> i32 {
    // Validate inputs
    if input_path.is_null() || output_path.is_null() || bookmarks_json.is_null() {
        eprintln!("mp_restore_bookmarks: Null parameter");
        return -1;
    }

    // Convert C strings
    let input = match unsafe { CStr::from_ptr(input_path) }.to_str() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("mp_restore_bookmarks: Invalid input path: {}", e);
            return -1;
        }
    };

    let output = match unsafe { CStr::from_ptr(output_path) }.to_str() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("mp_restore_bookmarks: Invalid output path: {}", e);
            return -1;
        }
    };

    let json_str = match unsafe { CStr::from_ptr(bookmarks_json) }.to_str() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("mp_restore_bookmarks: Invalid JSON: {}", e);
            return -1;
        }
    };

    // Parse JSON
    let bookmarks: Vec<BookmarkJson> = match serde_json::from_str(json_str) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("mp_restore_bookmarks: Failed to parse JSON: {}", e);
            return -1;
        }
    };

    // Convert to internal bookmark format
    let internal_bookmarks: Vec<crate::enhanced::bookmarks::Bookmark> = bookmarks
        .into_iter()
        .map(|b| convert_bookmark_json(b))
        .collect();

    if internal_bookmarks.is_empty() {
        // No bookmarks, just copy the file
        if let Err(e) = std::fs::copy(input, output) {
            eprintln!("mp_restore_bookmarks: Failed to copy file: {}", e);
            return -1;
        }
        return 0;
    }

    // Read input PDF
    let mut pdf_data = match std::fs::read(input) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("mp_restore_bookmarks: Failed to read input: {}", e);
            return -1;
        }
    };

    // Get page objects and max object number
    let (page_objects, max_obj_num) = {
        let content = String::from_utf8_lossy(&pdf_data);
        let page_objects = match find_page_objects(&content) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("mp_restore_bookmarks: Failed to find page objects: {}", e);
                return -1;
            }
        };

        // Calculate max object number
        let mut max_obj_num = 0i32;
        let mut search_pos = 0;
        while let Some(obj_pos) = content[search_pos..].find(" 0 obj") {
            let before = &content[..search_pos + obj_pos];
            if let Some(last_space) = before.rfind(|c: char| c.is_ascii_whitespace() || c == '\n') {
                if let Ok(num) = before[last_space + 1..].parse::<i32>() {
                    max_obj_num = max_obj_num.max(num);
                }
            }
            search_pos = search_pos + obj_pos + 6;
        }

        (page_objects, max_obj_num)
    };

    // Insert all bookmarks at once using the bookmark writer
    let next_obj_num = max_obj_num + 1;
    if let Err(e) = crate::enhanced::bookmark_writer::insert_bookmarks_into_pdf(
        &mut pdf_data,
        &internal_bookmarks,
        &page_objects,
        next_obj_num,
    ) {
        eprintln!("mp_restore_bookmarks: Failed to insert bookmarks: {:?}", e);
        return -1;
    }

    // Write output
    if let Err(e) = std::fs::write(output, &pdf_data) {
        eprintln!("mp_restore_bookmarks: Failed to write output: {}", e);
        return -1;
    }

    0
}

/// JSON bookmark structure for parsing
#[derive(serde::Deserialize)]
struct BookmarkJson {
    title: String,
    page: usize,
    #[serde(default)]
    children: Vec<BookmarkJson>,
}

/// Convert JSON bookmark to internal format
fn convert_bookmark_json(json: BookmarkJson) -> crate::enhanced::bookmarks::Bookmark {
    let mut bookmark = crate::enhanced::bookmarks::Bookmark::new(json.title, json.page);
    for child in json.children {
        bookmark.add_child(convert_bookmark_json(child));
    }
    bookmark
}

/// Add a single bookmark to PDF data
fn add_bookmark_to_data(
    pdf_data: &mut Vec<u8>,
    bookmark: &crate::enhanced::bookmarks::Bookmark,
) -> Result<(), String> {
    // Get page count and page objects first (immutable borrow)
    let (page_count, page_objects, max_obj_num) = {
        let content = String::from_utf8_lossy(pdf_data);
        let page_count = count_pages_in_content(&content);
        let page_objects = find_page_objects(&content)?;

        // Calculate max object number
        let mut max_obj_num = 0i32;
        let mut search_pos = 0;
        while let Some(obj_pos) = content[search_pos..].find(" 0 obj") {
            let before = &content[..search_pos + obj_pos];
            if let Some(last_space) = before.rfind(|c: char| c.is_ascii_whitespace() || c == '\n') {
                if let Ok(num) = before[last_space + 1..].parse::<i32>() {
                    max_obj_num = max_obj_num.max(num);
                }
            }
            search_pos = search_pos + obj_pos + 6;
        }

        (page_count, page_objects, max_obj_num)
    };

    if page_count == 0 {
        return Err("Could not determine page count".into());
    }

    // Validate bookmark
    if bookmark.title.is_empty() {
        return Err("Bookmark title cannot be empty".into());
    }
    if bookmark.page > page_count {
        return Err(format!(
            "Page {} exceeds document page count {}",
            bookmark.page, page_count
        ));
    }

    // Calculate next object number
    // insert_bookmarks_into_pdf will handle creating/updating Outlines in Catalog
    let next_obj_num = max_obj_num + 1;

    // Insert bookmark - this function will create Outlines and update Catalog
    crate::enhanced::bookmark_writer::insert_bookmarks_into_pdf(
        pdf_data,
        &[bookmark.clone()],
        &page_objects,
        next_obj_num,
    )
    .map_err(|e| format!("Failed to insert bookmark: {:?}", e))?;

    Ok(())
}

/// Count pages in PDF content
fn count_pages_in_content(content: &str) -> usize {
    // Try both patterns for /Type /Pages and /Type/Pages
    let pages_patterns = ["/Type /Pages", "/Type/Pages"];

    for pattern in pages_patterns {
        if let Some(pages_pos) = content.find(pattern) {
            let search_region = &content[pages_pos..std::cmp::min(pages_pos + 500, content.len())];
            if let Some(count_pos) = search_region.find("/Count") {
                let after_count = &search_region[count_pos + 6..];
                let trimmed = after_count.trim_start();
                let num_end = trimmed
                    .find(|c: char| !c.is_ascii_digit())
                    .unwrap_or(trimmed.len());
                if num_end > 0 {
                    if let Ok(count) = trimmed[..num_end].parse::<usize>() {
                        if count > 0 {
                            return count;
                        }
                    }
                }
            }
        }
    }

    // Fallback: count /Type /Page occurrences (both with and without space)
    let mut count = 0;
    let page_patterns = ["/Type /Page", "/Type/Page"];

    for pattern in page_patterns {
        let pattern_len = pattern.len();
        let mut pos = 0;
        while let Some(found) = content[pos..].find(pattern) {
            let abs_pos = pos + found;
            let after = if abs_pos + pattern_len < content.len() {
                &content[abs_pos + pattern_len..]
            } else {
                ""
            };
            // Skip /Type /Pages and /Type/Pages
            if !after.starts_with('s') && !after.starts_with('S') {
                count += 1;
            }
            pos = abs_pos + pattern_len;
        }
    }

    count
}

/// Find page objects in PDF content
fn find_page_objects(content: &str) -> Result<std::collections::HashMap<usize, i32>, String> {
    let mut page_objects = std::collections::HashMap::new();
    let mut page_num = 1;

    // Try multiple patterns for page objects
    let patterns = ["/Type /Page", "/Type/Page"];

    for pattern in patterns {
        let mut search_pos = 0;
        while let Some(type_pos) = content[search_pos..].find(pattern) {
            let abs_type_pos = search_pos + type_pos;
            let pattern_len = pattern.len();

            // Skip /Type /Pages and /Type/Pages
            let after_pattern = &content[abs_type_pos + pattern_len..];
            if after_pattern.starts_with("s") || after_pattern.starts_with("S") {
                search_pos = abs_type_pos + pattern_len;
                continue;
            }

            // Find object number by looking backwards for "N 0 obj"
            let before = &content[..abs_type_pos];
            if let Some(obj_pos) = before.rfind(" 0 obj") {
                let before_obj = &before[..obj_pos];
                let obj_num: i32 = before_obj
                    .chars()
                    .rev()
                    .take_while(|c| c.is_ascii_digit())
                    .collect::<String>()
                    .chars()
                    .rev()
                    .collect::<String>()
                    .parse()
                    .unwrap_or(0);

                if obj_num > 0 && !page_objects.values().any(|&v| v == obj_num) {
                    page_objects.insert(page_num, obj_num);
                    page_num += 1;
                }
            }

            search_pos = abs_type_pos + pattern_len;
        }
    }

    // If still empty, try parsing the /Pages /Kids array
    if page_objects.is_empty() {
        if let Some(pages_pos) = content.find("/Type /Pages") {
            let after_pages = &content[pages_pos..std::cmp::min(pages_pos + 2000, content.len())];
            if let Some(kids_pos) = after_pages.find("/Kids") {
                let after_kids = &after_pages[kids_pos + 5..];
                if let Some(bracket_start) = after_kids.find('[') {
                    if let Some(bracket_end) = after_kids[bracket_start..].find(']') {
                        let kids_array =
                            &after_kids[bracket_start + 1..bracket_start + bracket_end];
                        // Parse "N 0 R" patterns
                        let parts: Vec<&str> = kids_array.split_whitespace().collect();
                        let mut i = 0;
                        let mut page_idx = 1;
                        while i + 2 < parts.len() {
                            if parts[i + 2] == "R" {
                                if let Ok(obj_num) = parts[i].parse::<i32>() {
                                    page_objects.insert(page_idx, obj_num);
                                    page_idx += 1;
                                }
                            }
                            i += 3;
                        }
                    }
                }
            }
        }
    }

    // Still empty? Try /Type/Pages (no space)
    if page_objects.is_empty() {
        if let Some(pages_pos) = content.find("/Type/Pages") {
            let after_pages = &content[pages_pos..std::cmp::min(pages_pos + 2000, content.len())];
            if let Some(kids_pos) = after_pages.find("/Kids") {
                let after_kids = &after_pages[kids_pos + 5..];
                if let Some(bracket_start) = after_kids.find('[') {
                    if let Some(bracket_end) = after_kids[bracket_start..].find(']') {
                        let kids_array =
                            &after_kids[bracket_start + 1..bracket_start + bracket_end];
                        let parts: Vec<&str> = kids_array.split_whitespace().collect();
                        let mut i = 0;
                        let mut page_idx = 1;
                        while i + 2 < parts.len() {
                            if parts[i + 2] == "R" {
                                if let Ok(obj_num) = parts[i].parse::<i32>() {
                                    page_objects.insert(page_idx, obj_num);
                                    page_idx += 1;
                                }
                            }
                            i += 3;
                        }
                    }
                }
            }
        }
    }

    if page_objects.is_empty() {
        return Err("No page objects found".into());
    }

    Ok(page_objects)
}

/// Find or create Outlines dictionary
fn find_or_create_outlines(pdf_data: &mut Vec<u8>) -> Result<i32, String> {
    let content = String::from_utf8_lossy(pdf_data);

    // Look for existing /Outlines in Catalog
    if let Some(outlines_pos) = content.find("/Outlines") {
        let after_outlines = &content[outlines_pos + 9..];
        let parts: Vec<&str> = after_outlines.split_whitespace().take(2).collect();
        if parts.len() >= 2 && parts[1] == "0" {
            if let Ok(outline_obj_num) = parts[0].parse::<i32>() {
                return Ok(outline_obj_num);
            }
        }
    }

    // Find max object number and catalog position
    let mut max_obj_num = 0;
    let mut catalog_obj_num = 0;
    let mut search_pos = 0;

    while let Some(obj_pos) = content[search_pos..].find(" 0 obj") {
        let abs_pos = search_pos + obj_pos;
        let before = &content[..abs_pos];
        if let Some(last_space) = before.rfind(|c: char| c.is_ascii_whitespace() || c == '\n') {
            if let Ok(num) = before[last_space + 1..].parse::<i32>() {
                max_obj_num = max_obj_num.max(num);

                // Check if this is the Catalog
                let after = &content[abs_pos..];
                if after.contains("/Type /Catalog") || after.contains("/Type/Catalog") {
                    catalog_obj_num = num;
                }
            }
        }
        search_pos = abs_pos + 6;
    }

    if catalog_obj_num == 0 {
        return Err("Catalog object not found".into());
    }

    let outline_obj_num = max_obj_num + 1;

    // Create Outlines object
    let outlines_obj = format!(
        "{} 0 obj\n<<\n/Type /Outlines\n/Count 0\n>>\nendobj\n",
        outline_obj_num
    );

    // Find insertion point - try multiple patterns
    // 1. Try "\nxref\n" (traditional xref table)
    // 2. Try "startxref" (appears before xref offset)
    // 3. Try "%%EOF" (end of file marker)
    let insert_pos = {
        let xref_patterns: &[&[u8]] = &[b"\nxref\n", b"\nxref ", b"xref\n"];
        let mut found_pos = None;

        for pattern in xref_patterns {
            if let Some(pos) = pdf_data.windows(pattern.len()).position(|w| w == *pattern) {
                found_pos = Some(pos);
                break;
            }
        }

        // Fallback: find "startxref" and insert before it
        if found_pos.is_none() {
            if let Some(pos) = pdf_data.windows(9).position(|w| w == b"startxref") {
                // Go back to find a newline before startxref
                let mut start = pos;
                while start > 0 && pdf_data[start - 1] != b'\n' {
                    start -= 1;
                }
                found_pos = Some(start);
            }
        }

        // Last fallback: find "%%EOF" and insert before it
        if found_pos.is_none() {
            if let Some(pos) = pdf_data.windows(5).position(|w| w == b"%%EOF") {
                let mut start = pos;
                while start > 0 && pdf_data[start - 1] != b'\n' {
                    start -= 1;
                }
                found_pos = Some(start);
            }
        }

        found_pos.ok_or("Could not find insertion point (no xref, startxref, or %%EOF)")?
    };

    // Insert Outlines object at found position
    for (i, byte) in outlines_obj.as_bytes().iter().enumerate() {
        pdf_data.insert(insert_pos + i, *byte);
    }

    // Add /Outlines reference to Catalog
    let catalog_pattern = format!("{} 0 obj", catalog_obj_num);
    if let Some(catalog_pos) = pdf_data
        .windows(catalog_pattern.len())
        .position(|w| w == catalog_pattern.as_bytes())
    {
        // Find >> in catalog
        let catalog_section = &pdf_data[catalog_pos..];
        if let Some(end_pos) = catalog_section.windows(2).position(|w| w == b">>") {
            let insert_pos = catalog_pos + end_pos;
            let outlines_ref = format!("/Outlines {} 0 R\n", outline_obj_num);
            for (i, byte) in outlines_ref.as_bytes().iter().enumerate() {
                pdf_data.insert(insert_pos + i, *byte);
            }
        }
    }

    Ok(outline_obj_num)
}

use serde;
use serde_json;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_blank_page_invalid_dimensions() {
        assert_eq!(mp_add_blank_page(0, 0, -10.0, 100.0), -1);
        assert_eq!(mp_add_blank_page(0, 0, 100.0, 0.0), -1);
    }

    #[test]
    fn test_merge_pdfs_null_paths() {
        assert_eq!(
            mp_merge_pdfs(0, std::ptr::null(), 0, c"out.pdf".as_ptr()),
            -1
        );
    }

    #[test]
    fn test_split_pdf_null_path() {
        assert_eq!(mp_split_pdf(0, std::ptr::null(), c"/tmp".as_ptr()), -1);
    }

    #[test]
    fn test_add_watermark_null_text() {
        assert_eq!(
            mp_add_watermark(
                0,
                c"in.pdf".as_ptr(),
                c"out.pdf".as_ptr(),
                std::ptr::null(),
                0.0,
                0.0,
                12.0,
                0.5
            ),
            -1
        );
    }

    #[test]
    fn test_add_watermark_invalid_opacity() {
        assert_eq!(
            mp_add_watermark(
                0,
                c"in.pdf".as_ptr(),
                c"out.pdf".as_ptr(),
                c"TEST".as_ptr(),
                0.0,
                0.0,
                12.0,
                1.5
            ),
            -1
        );
    }

    #[test]
    fn test_draw_line_invalid_color() {
        assert_eq!(
            mp_draw_line(0, 0, 0.0, 0.0, 100.0, 100.0, 1.5, 0.5, 0.5, 1.0, 1.0),
            -1
        );
    }

    #[test]
    fn test_draw_rectangle_invalid_dimensions() {
        assert_eq!(
            mp_draw_rectangle(0, 0, 0.0, 0.0, -10.0, 100.0, 0.5, 0.5, 0.5, 1.0, 1),
            -1
        );
    }

    #[test]
    fn test_draw_circle_invalid_radius() {
        assert_eq!(
            mp_draw_circle(0, 0, 50.0, 50.0, -10.0, 0.5, 0.5, 0.5, 1.0, 1),
            -1
        );
    }
}
