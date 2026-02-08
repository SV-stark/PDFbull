//! FFI exports for Document Composition Framework (Category 3)
//!
//! Provides C-compatible API for:
//! - Document templates and page layouts
//! - Flowables (Paragraph, Table, Image, etc.)
//! - Typography and styles
//! - Table of Contents generation

use crate::enhanced::flowables::{
    CondPageBreak, FlowContext, HorizontalRule, Image, KeepTogether, KeepWithNext, ListItem,
    PageBreak, Paragraph, Spacer, Story, WrapResult,
};
use crate::enhanced::platypus::{DocTemplate, Frame, PageTemplate};
use crate::enhanced::table::{CellRef, StyleCommand, Table, TableStyle, VAlign};
use crate::enhanced::toc::{TableOfContents, TocBuilder, TocEntry, TocLevelStyle};
use crate::enhanced::typography::{ParagraphStyle, StyleSheet, TextAlign};
use crate::ffi::Handle;
use std::ffi::{CStr, c_char};
use std::ptr;

// ============================================================================
// Error Codes
// ============================================================================

const SUCCESS: i32 = 0;
const ERR_NULL_POINTER: i32 = -1;
const ERR_INVALID_UTF8: i32 = -2;
const ERR_INVALID_HANDLE: i32 = -3;
const ERR_OPERATION_FAILED: i32 = -4;

// ============================================================================
// DocTemplate Functions
// ============================================================================

/// Create a new document template
///
/// # Safety
/// The filename must be a valid null-terminated C string
#[unsafe(no_mangle)]
pub extern "C" fn mp_doc_template_create(filename: *const c_char) -> Handle {
    if filename.is_null() {
        return 0;
    }

    let filename_str = match unsafe { CStr::from_ptr(filename) }.to_str() {
        Ok(s) => s,
        Err(_) => return 0,
    };

    let template = Box::new(DocTemplate::new(filename_str));
    Box::into_raw(template) as Handle
}

/// Free a document template
///
/// # Safety
/// The handle must be valid or null
#[unsafe(no_mangle)]
pub extern "C" fn mp_doc_template_free(handle: Handle) {
    if handle != 0 {
        unsafe {
            let _ = Box::from_raw(handle as *mut DocTemplate);
        }
    }
}

/// Set document page size
///
/// # Safety
/// The handle must be valid
#[unsafe(no_mangle)]
pub extern "C" fn mp_doc_template_set_page_size(handle: Handle, width: f32, height: f32) -> i32 {
    if handle == 0 {
        return ERR_INVALID_HANDLE;
    }

    unsafe {
        let template = &mut *(handle as *mut DocTemplate);
        template.page_width = width;
        template.page_height = height;
    }
    SUCCESS
}

/// Set document margins
///
/// # Safety
/// The handle must be valid
#[unsafe(no_mangle)]
pub extern "C" fn mp_doc_template_set_margins(
    handle: Handle,
    left: f32,
    right: f32,
    top: f32,
    bottom: f32,
) -> i32 {
    if handle == 0 {
        return ERR_INVALID_HANDLE;
    }

    unsafe {
        let template = &mut *(handle as *mut DocTemplate);
        template.left_margin = left;
        template.right_margin = right;
        template.top_margin = top;
        template.bottom_margin = bottom;
    }
    SUCCESS
}

// ============================================================================
// Frame Functions
// ============================================================================

/// Create a new frame
///
/// # Safety
/// The id must be a valid null-terminated C string
#[unsafe(no_mangle)]
pub extern "C" fn mp_frame_create(
    id: *const c_char,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
) -> Handle {
    if id.is_null() {
        return 0;
    }

    let id_str = match unsafe { CStr::from_ptr(id) }.to_str() {
        Ok(s) => s,
        Err(_) => return 0,
    };

    let frame = Box::new(Frame::new(id_str, x, y, width, height));
    Box::into_raw(frame) as Handle
}

/// Free a frame
///
/// # Safety
/// The handle must be valid or null
#[unsafe(no_mangle)]
pub extern "C" fn mp_frame_free(handle: Handle) {
    if handle != 0 {
        unsafe {
            let _ = Box::from_raw(handle as *mut Frame);
        }
    }
}

/// Get frame available width
///
/// # Safety
/// The handle must be valid
#[unsafe(no_mangle)]
pub extern "C" fn mp_frame_available_width(handle: Handle) -> f32 {
    if handle == 0 {
        return 0.0;
    }

    unsafe {
        let frame = &*(handle as *const Frame);
        frame.available_width()
    }
}

/// Get frame available height
///
/// # Safety
/// The handle must be valid
#[unsafe(no_mangle)]
pub extern "C" fn mp_frame_available_height(handle: Handle) -> f32 {
    if handle == 0 {
        return 0.0;
    }

    unsafe {
        let frame = &*(handle as *const Frame);
        frame.available_height()
    }
}

// ============================================================================
// Story Functions
// ============================================================================

/// Create a new story
#[unsafe(no_mangle)]
pub extern "C" fn mp_story_create() -> Handle {
    let story = Box::new(Story::new());
    Box::into_raw(story) as Handle
}

/// Free a story
///
/// # Safety
/// The handle must be valid or null
#[unsafe(no_mangle)]
pub extern "C" fn mp_story_free(handle: Handle) {
    if handle != 0 {
        unsafe {
            let _ = Box::from_raw(handle as *mut Story);
        }
    }
}

/// Get story element count
///
/// # Safety
/// The handle must be valid
#[unsafe(no_mangle)]
pub extern "C" fn mp_story_len(handle: Handle) -> usize {
    if handle == 0 {
        return 0;
    }

    unsafe {
        let story = &*(handle as *const Story);
        story.len()
    }
}

// ============================================================================
// Paragraph Functions
// ============================================================================

/// Create a paragraph
///
/// # Safety
/// The text must be a valid null-terminated C string
#[unsafe(no_mangle)]
pub extern "C" fn mp_paragraph_create(text: *const c_char) -> Handle {
    if text.is_null() {
        return 0;
    }

    let text_str = match unsafe { CStr::from_ptr(text) }.to_str() {
        Ok(s) => s,
        Err(_) => return 0,
    };

    let para = Box::new(Paragraph::new(text_str));
    Box::into_raw(para) as Handle
}

/// Free a paragraph
///
/// # Safety
/// The handle must be valid or null
#[unsafe(no_mangle)]
pub extern "C" fn mp_paragraph_free(handle: Handle) {
    if handle != 0 {
        unsafe {
            let _ = Box::from_raw(handle as *mut Paragraph);
        }
    }
}

/// Set paragraph font size
///
/// # Safety
/// The handle must be valid
#[unsafe(no_mangle)]
pub extern "C" fn mp_paragraph_set_font_size(handle: Handle, size: f32) -> i32 {
    if handle == 0 {
        return ERR_INVALID_HANDLE;
    }

    unsafe {
        let para = &mut *(handle as *mut Paragraph);
        para.style.font_size = size;
    }
    SUCCESS
}

/// Set paragraph leading
///
/// # Safety
/// The handle must be valid
#[unsafe(no_mangle)]
pub extern "C" fn mp_paragraph_set_leading(handle: Handle, leading: f32) -> i32 {
    if handle == 0 {
        return ERR_INVALID_HANDLE;
    }

    unsafe {
        let para = &mut *(handle as *mut Paragraph);
        para.style.leading = leading;
    }
    SUCCESS
}

// ============================================================================
// Spacer Functions
// ============================================================================

/// Create a spacer
#[unsafe(no_mangle)]
pub extern "C" fn mp_spacer_create(height: f32) -> Handle {
    let spacer = Box::new(Spacer::new(height));
    Box::into_raw(spacer) as Handle
}

/// Free a spacer
///
/// # Safety
/// The handle must be valid or null
#[unsafe(no_mangle)]
pub extern "C" fn mp_spacer_free(handle: Handle) {
    if handle != 0 {
        unsafe {
            let _ = Box::from_raw(handle as *mut Spacer);
        }
    }
}

// ============================================================================
// Table Functions
// ============================================================================

/// Create a table with given dimensions
#[unsafe(no_mangle)]
pub extern "C" fn mp_table_create(rows: usize, cols: usize) -> Handle {
    let data: Vec<Vec<&str>> = (0..rows).map(|_| (0..cols).map(|_| "").collect()).collect();
    let table = Box::new(Table::new(data));
    Box::into_raw(table) as Handle
}

/// Free a table
///
/// # Safety
/// The handle must be valid or null
#[unsafe(no_mangle)]
pub extern "C" fn mp_table_free(handle: Handle) {
    if handle != 0 {
        unsafe {
            let _ = Box::from_raw(handle as *mut Table);
        }
    }
}

/// Get table row count
///
/// # Safety
/// The handle must be valid
#[unsafe(no_mangle)]
pub extern "C" fn mp_table_num_rows(handle: Handle) -> usize {
    if handle == 0 {
        return 0;
    }

    unsafe {
        let table = &*(handle as *const Table);
        table.num_rows()
    }
}

/// Get table column count
///
/// # Safety
/// The handle must be valid
#[unsafe(no_mangle)]
pub extern "C" fn mp_table_num_cols(handle: Handle) -> usize {
    if handle == 0 {
        return 0;
    }

    unsafe {
        let table = &*(handle as *const Table);
        table.num_cols()
    }
}

// ============================================================================
// TableStyle Functions
// ============================================================================

/// Create a table style
#[unsafe(no_mangle)]
pub extern "C" fn mp_table_style_create() -> Handle {
    let style = Box::new(TableStyle::new());
    Box::into_raw(style) as Handle
}

/// Free a table style
///
/// # Safety
/// The handle must be valid or null
#[unsafe(no_mangle)]
pub extern "C" fn mp_table_style_free(handle: Handle) {
    if handle != 0 {
        unsafe {
            let _ = Box::from_raw(handle as *mut TableStyle);
        }
    }
}

/// Add grid to table style
///
/// # Safety
/// The handle must be valid
#[unsafe(no_mangle)]
pub extern "C" fn mp_table_style_add_grid(
    handle: Handle,
    weight: f32,
    r: f32,
    g: f32,
    b: f32,
) -> Handle {
    if handle == 0 {
        return 0;
    }

    unsafe {
        let style = Box::from_raw(handle as *mut TableStyle);
        let new_style = Box::new(style.grid(weight, (r, g, b)));
        Box::into_raw(new_style) as Handle
    }
}

/// Add background to table style
///
/// # Safety
/// The handle must be valid
#[unsafe(no_mangle)]
pub extern "C" fn mp_table_style_add_background(
    handle: Handle,
    start_col: i32,
    start_row: i32,
    end_col: i32,
    end_row: i32,
    r: f32,
    g: f32,
    b: f32,
) -> Handle {
    if handle == 0 {
        return 0;
    }

    unsafe {
        let style = Box::from_raw(handle as *mut TableStyle);
        let new_style =
            Box::new(style.background((start_col, start_row), (end_col, end_row), (r, g, b)));
        Box::into_raw(new_style) as Handle
    }
}

// ============================================================================
// TOC Functions
// ============================================================================

/// Create a table of contents
#[unsafe(no_mangle)]
pub extern "C" fn mp_toc_create() -> Handle {
    let toc = Box::new(TableOfContents::new());
    Box::into_raw(toc) as Handle
}

/// Free a table of contents
///
/// # Safety
/// The handle must be valid or null
#[unsafe(no_mangle)]
pub extern "C" fn mp_toc_free(handle: Handle) {
    if handle != 0 {
        unsafe {
            let _ = Box::from_raw(handle as *mut TableOfContents);
        }
    }
}

/// Set TOC title
///
/// # Safety
/// Both handle and title must be valid
#[unsafe(no_mangle)]
pub extern "C" fn mp_toc_set_title(handle: Handle, title: *const c_char) -> Handle {
    if handle == 0 || title.is_null() {
        return 0;
    }

    let title_str = match unsafe { CStr::from_ptr(title) }.to_str() {
        Ok(s) => s,
        Err(_) => return 0,
    };

    unsafe {
        let toc = Box::from_raw(handle as *mut TableOfContents);
        let new_toc = Box::new(toc.with_title(title_str));
        Box::into_raw(new_toc) as Handle
    }
}

/// Add entry to TOC
///
/// # Safety
/// Both handle and title must be valid
#[unsafe(no_mangle)]
pub extern "C" fn mp_toc_add_entry(
    handle: Handle,
    title: *const c_char,
    level: u8,
    page: usize,
) -> Handle {
    if handle == 0 || title.is_null() {
        return 0;
    }

    let title_str = match unsafe { CStr::from_ptr(title) }.to_str() {
        Ok(s) => s,
        Err(_) => return 0,
    };

    unsafe {
        let toc = Box::from_raw(handle as *mut TableOfContents);
        let entry = TocEntry::new(title_str, level, page);
        let new_toc = Box::new(toc.add_entry(entry));
        Box::into_raw(new_toc) as Handle
    }
}

// ============================================================================
// TocBuilder Functions
// ============================================================================

/// Create a TOC builder
#[unsafe(no_mangle)]
pub extern "C" fn mp_toc_builder_create() -> Handle {
    let builder = Box::new(TocBuilder::new());
    Box::into_raw(builder) as Handle
}

/// Free a TOC builder
///
/// # Safety
/// The handle must be valid or null
#[unsafe(no_mangle)]
pub extern "C" fn mp_toc_builder_free(handle: Handle) {
    if handle != 0 {
        unsafe {
            let _ = Box::from_raw(handle as *mut TocBuilder);
        }
    }
}

/// Add heading to TOC builder
///
/// # Safety
/// Both handle and title must be valid
#[unsafe(no_mangle)]
pub extern "C" fn mp_toc_builder_add_heading(
    handle: Handle,
    title: *const c_char,
    level: u8,
    page: usize,
) -> i32 {
    if handle == 0 || title.is_null() {
        return ERR_NULL_POINTER;
    }

    let title_str = match unsafe { CStr::from_ptr(title) }.to_str() {
        Ok(s) => s,
        Err(_) => return ERR_INVALID_UTF8,
    };

    unsafe {
        let builder = &mut *(handle as *mut TocBuilder);
        builder.add_heading(title_str, level, page, None);
    }
    SUCCESS
}

// ============================================================================
// ParagraphStyle Functions
// ============================================================================

/// Create a paragraph style
///
/// # Safety
/// The name must be a valid null-terminated C string
#[unsafe(no_mangle)]
pub extern "C" fn mp_paragraph_style_create(name: *const c_char) -> Handle {
    if name.is_null() {
        return 0;
    }

    let name_str = match unsafe { CStr::from_ptr(name) }.to_str() {
        Ok(s) => s,
        Err(_) => return 0,
    };

    let style = Box::new(ParagraphStyle::new(name_str));
    Box::into_raw(style) as Handle
}

/// Free a paragraph style
///
/// # Safety
/// The handle must be valid or null
#[unsafe(no_mangle)]
pub extern "C" fn mp_paragraph_style_free(handle: Handle) {
    if handle != 0 {
        unsafe {
            let _ = Box::from_raw(handle as *mut ParagraphStyle);
        }
    }
}

/// Set paragraph style font size
///
/// # Safety
/// The handle must be valid
#[unsafe(no_mangle)]
pub extern "C" fn mp_paragraph_style_set_font_size(handle: Handle, size: f32) -> Handle {
    if handle == 0 {
        return 0;
    }

    unsafe {
        let style = Box::from_raw(handle as *mut ParagraphStyle);
        let new_style = Box::new(style.with_font_size(size));
        Box::into_raw(new_style) as Handle
    }
}

/// Set paragraph style leading
///
/// # Safety
/// The handle must be valid
#[unsafe(no_mangle)]
pub extern "C" fn mp_paragraph_style_set_leading(handle: Handle, leading: f32) -> Handle {
    if handle == 0 {
        return 0;
    }

    unsafe {
        let style = Box::from_raw(handle as *mut ParagraphStyle);
        let new_style = Box::new(style.with_leading(leading));
        Box::into_raw(new_style) as Handle
    }
}

/// Set paragraph style alignment
///
/// # Safety
/// The handle must be valid
#[unsafe(no_mangle)]
pub extern "C" fn mp_paragraph_style_set_alignment(handle: Handle, align: i32) -> Handle {
    if handle == 0 {
        return 0;
    }

    let alignment = match align {
        0 => TextAlign::Left,
        1 => TextAlign::Right,
        2 => TextAlign::Center,
        3 => TextAlign::Justify,
        _ => TextAlign::Left,
    };

    unsafe {
        let style = Box::from_raw(handle as *mut ParagraphStyle);
        let new_style = Box::new(style.with_alignment(alignment));
        Box::into_raw(new_style) as Handle
    }
}

// ============================================================================
// StyleSheet Functions
// ============================================================================

/// Create a stylesheet with default styles
#[unsafe(no_mangle)]
pub extern "C" fn mp_stylesheet_create() -> Handle {
    let sheet = Box::new(StyleSheet::new());
    Box::into_raw(sheet) as Handle
}

/// Free a stylesheet
///
/// # Safety
/// The handle must be valid or null
#[unsafe(no_mangle)]
pub extern "C" fn mp_stylesheet_free(handle: Handle) {
    if handle != 0 {
        unsafe {
            let _ = Box::from_raw(handle as *mut StyleSheet);
        }
    }
}

/// Add style to stylesheet
///
/// # Safety
/// Both handles must be valid
#[unsafe(no_mangle)]
pub extern "C" fn mp_stylesheet_add_style(sheet_handle: Handle, style_handle: Handle) -> i32 {
    if sheet_handle == 0 || style_handle == 0 {
        return ERR_INVALID_HANDLE;
    }

    unsafe {
        let sheet = &mut *(sheet_handle as *mut StyleSheet);
        let style = Box::from_raw(style_handle as *mut ParagraphStyle);
        sheet.add_style(*style);
    }
    SUCCESS
}

// ============================================================================
// Image Functions
// ============================================================================

/// Create an image from file path
///
/// # Safety
/// The path must be a valid null-terminated C string
#[unsafe(no_mangle)]
pub extern "C" fn mp_image_create(path: *const c_char) -> Handle {
    if path.is_null() {
        return 0;
    }

    let path_str = match unsafe { CStr::from_ptr(path) }.to_str() {
        Ok(s) => s,
        Err(_) => return 0,
    };

    let image = Box::new(Image::from_file(path_str));
    Box::into_raw(image) as Handle
}

/// Free an image
///
/// # Safety
/// The handle must be valid or null
#[unsafe(no_mangle)]
pub extern "C" fn mp_image_free(handle: Handle) {
    if handle != 0 {
        unsafe {
            let _ = Box::from_raw(handle as *mut Image);
        }
    }
}

/// Set image width
///
/// # Safety
/// The handle must be valid
#[unsafe(no_mangle)]
pub extern "C" fn mp_image_set_width(handle: Handle, width: f32) -> Handle {
    if handle == 0 {
        return 0;
    }

    unsafe {
        let image = Box::from_raw(handle as *mut Image);
        let new_image = Box::new(image.with_width(width));
        Box::into_raw(new_image) as Handle
    }
}

/// Set image height
///
/// # Safety
/// The handle must be valid
#[unsafe(no_mangle)]
pub extern "C" fn mp_image_set_height(handle: Handle, height: f32) -> Handle {
    if handle == 0 {
        return 0;
    }

    unsafe {
        let image = Box::from_raw(handle as *mut Image);
        let new_image = Box::new(image.with_height(height));
        Box::into_raw(new_image) as Handle
    }
}

// ============================================================================
// Horizontal Rule Functions
// ============================================================================

/// Create a horizontal rule
#[unsafe(no_mangle)]
pub extern "C" fn mp_hr_create() -> Handle {
    let hr = Box::new(HorizontalRule::new());
    Box::into_raw(hr) as Handle
}

/// Free a horizontal rule
///
/// # Safety
/// The handle must be valid or null
#[unsafe(no_mangle)]
pub extern "C" fn mp_hr_free(handle: Handle) {
    if handle != 0 {
        unsafe {
            let _ = Box::from_raw(handle as *mut HorizontalRule);
        }
    }
}

/// Set horizontal rule thickness
///
/// # Safety
/// The handle must be valid
#[unsafe(no_mangle)]
pub extern "C" fn mp_hr_set_thickness(handle: Handle, thickness: f32) -> Handle {
    if handle == 0 {
        return 0;
    }

    unsafe {
        let hr = Box::from_raw(handle as *mut HorizontalRule);
        let new_hr = Box::new(hr.with_thickness(thickness));
        Box::into_raw(new_hr) as Handle
    }
}

// ============================================================================
// ListItem Functions
// ============================================================================

/// Create a bullet list item
///
/// # Safety
/// The text must be a valid null-terminated C string
#[unsafe(no_mangle)]
pub extern "C" fn mp_list_item_bullet(text: *const c_char) -> Handle {
    if text.is_null() {
        return 0;
    }

    let text_str = match unsafe { CStr::from_ptr(text) }.to_str() {
        Ok(s) => s,
        Err(_) => return 0,
    };

    let item = Box::new(ListItem::bullet(text_str));
    Box::into_raw(item) as Handle
}

/// Create a numbered list item
///
/// # Safety
/// The text must be a valid null-terminated C string
#[unsafe(no_mangle)]
pub extern "C" fn mp_list_item_numbered(number: usize, text: *const c_char) -> Handle {
    if text.is_null() {
        return 0;
    }

    let text_str = match unsafe { CStr::from_ptr(text) }.to_str() {
        Ok(s) => s,
        Err(_) => return 0,
    };

    let item = Box::new(ListItem::numbered(number, text_str));
    Box::into_raw(item) as Handle
}

/// Free a list item
///
/// # Safety
/// The handle must be valid or null
#[unsafe(no_mangle)]
pub extern "C" fn mp_list_item_free(handle: Handle) {
    if handle != 0 {
        unsafe {
            let _ = Box::from_raw(handle as *mut ListItem);
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    #[test]
    fn test_doc_template_create() {
        let filename = CString::new("test.pdf").unwrap();
        let handle = mp_doc_template_create(filename.as_ptr());
        assert!(handle != 0);
        mp_doc_template_free(handle);
    }

    #[test]
    fn test_frame_create() {
        let id = CString::new("main").unwrap();
        let handle = mp_frame_create(id.as_ptr(), 72.0, 72.0, 468.0, 648.0);
        assert!(handle != 0);

        let width = mp_frame_available_width(handle);
        assert!(width > 0.0);

        mp_frame_free(handle);
    }

    #[test]
    fn test_story_create() {
        let handle = mp_story_create();
        assert!(handle != 0);

        let len = mp_story_len(handle);
        assert_eq!(len, 0);

        mp_story_free(handle);
    }

    #[test]
    fn test_paragraph_create() {
        let text = CString::new("Hello, World!").unwrap();
        let handle = mp_paragraph_create(text.as_ptr());
        assert!(handle != 0);
        mp_paragraph_free(handle);
    }

    #[test]
    fn test_table_create() {
        let handle = mp_table_create(3, 4);
        assert!(handle != 0);

        let rows = mp_table_num_rows(handle);
        let cols = mp_table_num_cols(handle);
        assert_eq!(rows, 3);
        assert_eq!(cols, 4);

        mp_table_free(handle);
    }

    #[test]
    fn test_toc_create() {
        let handle = mp_toc_create();
        assert!(handle != 0);

        let title = CString::new("Contents").unwrap();
        let handle = mp_toc_set_title(handle, title.as_ptr());
        assert!(handle != 0);

        let entry_title = CString::new("Chapter 1").unwrap();
        let handle = mp_toc_add_entry(handle, entry_title.as_ptr(), 1, 1);
        assert!(handle != 0);

        mp_toc_free(handle);
    }

    #[test]
    fn test_stylesheet_create() {
        let handle = mp_stylesheet_create();
        assert!(handle != 0);
        mp_stylesheet_free(handle);
    }

    #[test]
    fn test_image_create() {
        let path = CString::new("test.png").unwrap();
        let handle = mp_image_create(path.as_ptr());
        assert!(handle != 0);

        let handle = mp_image_set_width(handle, 200.0);
        assert!(handle != 0);

        mp_image_free(handle);
    }
}
