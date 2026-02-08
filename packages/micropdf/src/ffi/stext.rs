//! FFI bindings for fz_stext (Structured Text Extraction)
//!
//! This module provides C-compatible exports for structured text extraction operations.
//! Used for text search, format conversion, accessibility, and OCR integration.

use super::{Handle, HandleStore};
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::sync::LazyLock;

// ============================================================================
// Types and Constants
// ============================================================================

/// Stext options flags
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StextFlag {
    PreserveLigatures = 1,
    PreserveWhitespace = 2,
    PreserveImages = 4,
    InhibitSpaces = 8,
    Dehyphenate = 16,
    PreserveSpans = 32,
    Clip = 64,
    UseCidForUnknownUnicode = 128,
    CollectStructure = 256,
    AccurateBboxes = 512,
    CollectVectors = 1024,
    IgnoreActualtext = 2048,
    Segment = 4096,
    ParagraphBreak = 8192,
    TableHunt = 16384,
    CollectStyles = 32768,
    UseGidForUnknownUnicode = 65536,
    ClipRect = 131072,             // 1 << 17
    AccurateAscenders = 262144,    // 1 << 18
    AccurateSideBearings = 524288, // 1 << 19
}

/// Block types
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StextBlockType {
    #[default]
    Text = 0,
    Image = 1,
    Struct = 2,
    Vector = 3,
    Grid = 4,
}

/// Text justification
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StextTextJustify {
    #[default]
    Unknown = 0,
    Left = 1,
    Centre = 2,
    Right = 3,
    Full = 4,
}

/// Character flags
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StextCharFlag {
    Strikeout = 1,
    Underline = 2,
    Synthetic = 4,
    Bold = 8,
    Filled = 16,
    Stroked = 32,
    Clipped = 64,
    UnicodeIsCid = 128,
    UnicodeIsGid = 256,
    SyntheticLarge = 512,
}

/// Selection mode
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SelectMode {
    #[default]
    Chars = 0,
    Words = 1,
    Lines = 2,
}

/// Search options
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SearchOption {
    #[default]
    Exact = 0,
    IgnoreCase = 1,
    IgnoreDiacritics = 2,
    Regexp = 4,
    KeepLines = 8,
    KeepParagraphs = 16,
    KeepHyphens = 32,
}

// ============================================================================
// Structures
// ============================================================================

/// Quad structure for text bounds
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct Quad {
    pub ul_x: f32,
    pub ul_y: f32,
    pub ur_x: f32,
    pub ur_y: f32,
    pub ll_x: f32,
    pub ll_y: f32,
    pub lr_x: f32,
    pub lr_y: f32,
}

/// Point structure
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

/// Rectangle structure
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct Rect {
    pub x0: f32,
    pub y0: f32,
    pub x1: f32,
    pub y1: f32,
}

/// Structured text character
#[derive(Debug, Clone)]
pub struct StextChar {
    pub c: i32,     // Unicode character value
    pub bidi: u16,  // BiDi level (even=LTR, odd=RTL)
    pub flags: u16, // Character flags
    pub argb: u32,  // sRGB color (alpha, r, g, b)
    pub origin: Point,
    pub quad: Quad,
    pub size: f32,
    pub font: Option<Handle>, // Font handle
}

impl Default for StextChar {
    fn default() -> Self {
        Self {
            c: 0,
            bidi: 0,
            flags: 0,
            argb: 0xFF000000, // Opaque black
            origin: Point::default(),
            quad: Quad::default(),
            size: 12.0,
            font: None,
        }
    }
}

/// Structured text line
#[derive(Debug, Clone, Default)]
pub struct StextLine {
    pub wmode: u8, // 0 = horizontal, 1 = vertical
    pub flags: u8,
    pub dir: Point, // Normalized baseline direction
    pub bbox: Rect,
    pub chars: Vec<StextChar>,
}

/// Structured text block
#[derive(Debug, Clone)]
pub struct StextBlock {
    pub block_type: StextBlockType,
    pub id: i32,
    pub bbox: Rect,
    pub lines: Vec<StextLine>,       // For text blocks
    pub image: Option<Handle>,       // For image blocks
    pub struct_down: Option<Handle>, // For struct blocks
    pub struct_index: i32,
    pub text_flags: i32,
}

impl Default for StextBlock {
    fn default() -> Self {
        Self {
            block_type: StextBlockType::Text,
            id: 0,
            bbox: Rect::default(),
            lines: Vec::new(),
            image: None,
            struct_down: None,
            struct_index: 0,
            text_flags: 0,
        }
    }
}

/// Structured text page
#[derive(Debug, Clone)]
pub struct StextPage {
    pub refs: i32,
    pub mediabox: Rect,
    pub blocks: Vec<StextBlock>,
}

impl Default for StextPage {
    fn default() -> Self {
        Self {
            refs: 1,
            mediabox: Rect {
                x0: 0.0,
                y0: 0.0,
                x1: 612.0,
                y1: 792.0,
            },
            blocks: Vec::new(),
        }
    }
}

/// Stext options
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct StextOptions {
    pub flags: i32,
    pub scale: f32,
    pub clip: Rect,
}

/// C-compatible quad for FFI
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct FzQuad {
    pub ul: [f32; 2],
    pub ur: [f32; 2],
    pub ll: [f32; 2],
    pub lr: [f32; 2],
}

// ============================================================================
// Handle Store
// ============================================================================

pub static STEXT_PAGES: LazyLock<HandleStore<StextPage>> = LazyLock::new(HandleStore::new);

// Thread-local storage for search results and text output
thread_local! {
    static SEARCH_QUADS: std::cell::RefCell<Vec<FzQuad>> = const { std::cell::RefCell::new(Vec::new()) };
    static TEXT_OUTPUT: std::cell::RefCell<Option<CString>> = const { std::cell::RefCell::new(None) };
}

// ============================================================================
// Page Functions
// ============================================================================

/// Create a new empty stext page
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_stext_page(_ctx: Handle, x0: f32, y0: f32, x1: f32, y1: f32) -> Handle {
    let page = StextPage {
        refs: 1,
        mediabox: Rect { x0, y0, x1, y1 },
        blocks: Vec::new(),
    };
    STEXT_PAGES.insert(page)
}

/// Keep (increment reference count) stext page
#[unsafe(no_mangle)]
pub extern "C" fn fz_keep_stext_page(_ctx: Handle, page: Handle) -> Handle {
    if let Some(arc) = STEXT_PAGES.get(page) {
        if let Ok(mut p) = arc.lock() {
            p.refs += 1;
        }
    }
    page
}

/// Drop (decrement reference count) stext page
#[unsafe(no_mangle)]
pub extern "C" fn fz_drop_stext_page(_ctx: Handle, page: Handle) {
    if page == 0 {
        return;
    }

    let should_drop = if let Some(arc) = STEXT_PAGES.get(page) {
        if let Ok(mut p) = arc.lock() {
            p.refs -= 1;
            p.refs <= 0
        } else {
            false
        }
    } else {
        false
    };

    if should_drop {
        STEXT_PAGES.remove(page);
    }
}

/// Get stext page mediabox
#[unsafe(no_mangle)]
pub extern "C" fn fz_stext_page_mediabox(
    _ctx: Handle,
    page: Handle,
    x0: *mut f32,
    y0: *mut f32,
    x1: *mut f32,
    y1: *mut f32,
) {
    if let Some(arc) = STEXT_PAGES.get(page) {
        if let Ok(p) = arc.lock() {
            if !x0.is_null() {
                unsafe { *x0 = p.mediabox.x0 };
            }
            if !y0.is_null() {
                unsafe { *y0 = p.mediabox.y0 };
            }
            if !x1.is_null() {
                unsafe { *x1 = p.mediabox.x1 };
            }
            if !y1.is_null() {
                unsafe { *y1 = p.mediabox.y1 };
            }
        }
    }
}

// ============================================================================
// Block Functions
// ============================================================================

/// Get first block from stext page
#[unsafe(no_mangle)]
pub extern "C" fn fz_stext_first_block(_ctx: Handle, page: Handle) -> i32 {
    if let Some(arc) = STEXT_PAGES.get(page) {
        if let Ok(p) = arc.lock() {
            if !p.blocks.is_empty() {
                return 0; // Return index of first block
            }
        }
    }
    -1
}

/// Get next block index
#[unsafe(no_mangle)]
pub extern "C" fn fz_stext_next_block(_ctx: Handle, page: Handle, block_idx: i32) -> i32 {
    if let Some(arc) = STEXT_PAGES.get(page) {
        if let Ok(p) = arc.lock() {
            let next_idx = block_idx + 1;
            if (next_idx as usize) < p.blocks.len() {
                return next_idx;
            }
        }
    }
    -1
}

/// Get block type
#[unsafe(no_mangle)]
pub extern "C" fn fz_stext_block_type(_ctx: Handle, page: Handle, block_idx: i32) -> i32 {
    if let Some(arc) = STEXT_PAGES.get(page) {
        if let Ok(p) = arc.lock() {
            if let Some(block) = p.blocks.get(block_idx as usize) {
                return block.block_type as i32;
            }
        }
    }
    -1
}

/// Get block bounding box
#[unsafe(no_mangle)]
pub extern "C" fn fz_stext_block_bbox(
    _ctx: Handle,
    page: Handle,
    block_idx: i32,
    x0: *mut f32,
    y0: *mut f32,
    x1: *mut f32,
    y1: *mut f32,
) {
    if let Some(arc) = STEXT_PAGES.get(page) {
        if let Ok(p) = arc.lock() {
            if let Some(block) = p.blocks.get(block_idx as usize) {
                if !x0.is_null() {
                    unsafe { *x0 = block.bbox.x0 };
                }
                if !y0.is_null() {
                    unsafe { *y0 = block.bbox.y0 };
                }
                if !x1.is_null() {
                    unsafe { *x1 = block.bbox.x1 };
                }
                if !y1.is_null() {
                    unsafe { *y1 = block.bbox.y1 };
                }
            }
        }
    }
}

// ============================================================================
// Line Functions
// ============================================================================

/// Get first line in a text block
#[unsafe(no_mangle)]
pub extern "C" fn fz_stext_first_line(_ctx: Handle, page: Handle, block_idx: i32) -> i32 {
    if let Some(arc) = STEXT_PAGES.get(page) {
        if let Ok(p) = arc.lock() {
            if let Some(block) = p.blocks.get(block_idx as usize) {
                if block.block_type == StextBlockType::Text && !block.lines.is_empty() {
                    return 0;
                }
            }
        }
    }
    -1
}

/// Get next line index
#[unsafe(no_mangle)]
pub extern "C" fn fz_stext_next_line(
    _ctx: Handle,
    page: Handle,
    block_idx: i32,
    line_idx: i32,
) -> i32 {
    if let Some(arc) = STEXT_PAGES.get(page) {
        if let Ok(p) = arc.lock() {
            if let Some(block) = p.blocks.get(block_idx as usize) {
                let next_idx = line_idx + 1;
                if (next_idx as usize) < block.lines.len() {
                    return next_idx;
                }
            }
        }
    }
    -1
}

/// Get line bounding box
#[unsafe(no_mangle)]
pub extern "C" fn fz_stext_line_bbox(
    _ctx: Handle,
    page: Handle,
    block_idx: i32,
    line_idx: i32,
    x0: *mut f32,
    y0: *mut f32,
    x1: *mut f32,
    y1: *mut f32,
) {
    if let Some(arc) = STEXT_PAGES.get(page) {
        if let Ok(p) = arc.lock() {
            if let Some(block) = p.blocks.get(block_idx as usize) {
                if let Some(line) = block.lines.get(line_idx as usize) {
                    if !x0.is_null() {
                        unsafe { *x0 = line.bbox.x0 };
                    }
                    if !y0.is_null() {
                        unsafe { *y0 = line.bbox.y0 };
                    }
                    if !x1.is_null() {
                        unsafe { *x1 = line.bbox.x1 };
                    }
                    if !y1.is_null() {
                        unsafe { *y1 = line.bbox.y1 };
                    }
                }
            }
        }
    }
}

/// Get line writing mode (0=horizontal, 1=vertical)
#[unsafe(no_mangle)]
pub extern "C" fn fz_stext_line_wmode(
    _ctx: Handle,
    page: Handle,
    block_idx: i32,
    line_idx: i32,
) -> i32 {
    if let Some(arc) = STEXT_PAGES.get(page) {
        if let Ok(p) = arc.lock() {
            if let Some(block) = p.blocks.get(block_idx as usize) {
                if let Some(line) = block.lines.get(line_idx as usize) {
                    return line.wmode as i32;
                }
            }
        }
    }
    0
}

// ============================================================================
// Character Functions
// ============================================================================

/// Get first character in a line
#[unsafe(no_mangle)]
pub extern "C" fn fz_stext_first_char(
    _ctx: Handle,
    page: Handle,
    block_idx: i32,
    line_idx: i32,
) -> i32 {
    if let Some(arc) = STEXT_PAGES.get(page) {
        if let Ok(p) = arc.lock() {
            if let Some(block) = p.blocks.get(block_idx as usize) {
                if let Some(line) = block.lines.get(line_idx as usize) {
                    if !line.chars.is_empty() {
                        return 0;
                    }
                }
            }
        }
    }
    -1
}

/// Get next character index
#[unsafe(no_mangle)]
pub extern "C" fn fz_stext_next_char(
    _ctx: Handle,
    page: Handle,
    block_idx: i32,
    line_idx: i32,
    char_idx: i32,
) -> i32 {
    if let Some(arc) = STEXT_PAGES.get(page) {
        if let Ok(p) = arc.lock() {
            if let Some(block) = p.blocks.get(block_idx as usize) {
                if let Some(line) = block.lines.get(line_idx as usize) {
                    let next_idx = char_idx + 1;
                    if (next_idx as usize) < line.chars.len() {
                        return next_idx;
                    }
                }
            }
        }
    }
    -1
}

/// Get character unicode value
#[unsafe(no_mangle)]
pub extern "C" fn fz_stext_char_value(
    _ctx: Handle,
    page: Handle,
    block_idx: i32,
    line_idx: i32,
    char_idx: i32,
) -> i32 {
    if let Some(arc) = STEXT_PAGES.get(page) {
        if let Ok(p) = arc.lock() {
            if let Some(block) = p.blocks.get(block_idx as usize) {
                if let Some(line) = block.lines.get(line_idx as usize) {
                    if let Some(ch) = line.chars.get(char_idx as usize) {
                        return ch.c;
                    }
                }
            }
        }
    }
    0
}

/// Get character origin point
#[unsafe(no_mangle)]
pub extern "C" fn fz_stext_char_origin(
    _ctx: Handle,
    page: Handle,
    block_idx: i32,
    line_idx: i32,
    char_idx: i32,
    x: *mut f32,
    y: *mut f32,
) {
    if let Some(arc) = STEXT_PAGES.get(page) {
        if let Ok(p) = arc.lock() {
            if let Some(block) = p.blocks.get(block_idx as usize) {
                if let Some(line) = block.lines.get(line_idx as usize) {
                    if let Some(ch) = line.chars.get(char_idx as usize) {
                        if !x.is_null() {
                            unsafe { *x = ch.origin.x };
                        }
                        if !y.is_null() {
                            unsafe { *y = ch.origin.y };
                        }
                    }
                }
            }
        }
    }
}

/// Get character quad
#[unsafe(no_mangle)]
pub extern "C" fn fz_stext_char_quad(
    _ctx: Handle,
    page: Handle,
    block_idx: i32,
    line_idx: i32,
    char_idx: i32,
    quad: *mut FzQuad,
) {
    if quad.is_null() {
        return;
    }

    if let Some(arc) = STEXT_PAGES.get(page) {
        if let Ok(p) = arc.lock() {
            if let Some(block) = p.blocks.get(block_idx as usize) {
                if let Some(line) = block.lines.get(line_idx as usize) {
                    if let Some(ch) = line.chars.get(char_idx as usize) {
                        unsafe {
                            (*quad).ul = [ch.quad.ul_x, ch.quad.ul_y];
                            (*quad).ur = [ch.quad.ur_x, ch.quad.ur_y];
                            (*quad).ll = [ch.quad.ll_x, ch.quad.ll_y];
                            (*quad).lr = [ch.quad.lr_x, ch.quad.lr_y];
                        }
                    }
                }
            }
        }
    }
}

/// Get character font size
#[unsafe(no_mangle)]
pub extern "C" fn fz_stext_char_size(
    _ctx: Handle,
    page: Handle,
    block_idx: i32,
    line_idx: i32,
    char_idx: i32,
) -> f32 {
    if let Some(arc) = STEXT_PAGES.get(page) {
        if let Ok(p) = arc.lock() {
            if let Some(block) = p.blocks.get(block_idx as usize) {
                if let Some(line) = block.lines.get(line_idx as usize) {
                    if let Some(ch) = line.chars.get(char_idx as usize) {
                        return ch.size;
                    }
                }
            }
        }
    }
    0.0
}

// ============================================================================
// Text Extraction Functions
// ============================================================================

/// Extract plain text from stext page
#[unsafe(no_mangle)]
pub extern "C" fn fz_stext_page_as_text(_ctx: Handle, page: Handle) -> *const c_char {
    if let Some(arc) = STEXT_PAGES.get(page) {
        if let Ok(p) = arc.lock() {
            let mut text = String::new();

            for block in &p.blocks {
                if block.block_type == StextBlockType::Text {
                    for line in &block.lines {
                        for ch in &line.chars {
                            if let Some(c) = char::from_u32(ch.c as u32) {
                                text.push(c);
                            }
                        }
                        text.push('\n');
                    }
                }
            }

            if let Ok(cstr) = CString::new(text) {
                TEXT_OUTPUT.with(|cell| {
                    *cell.borrow_mut() = Some(cstr);
                });
                return TEXT_OUTPUT.with(|cell| {
                    cell.borrow()
                        .as_ref()
                        .map(|s| s.as_ptr())
                        .unwrap_or(std::ptr::null())
                });
            }
        }
    }
    std::ptr::null()
}

/// Print stext page as HTML
#[unsafe(no_mangle)]
pub extern "C" fn fz_print_stext_page_as_html(
    _ctx: Handle,
    _output: Handle,
    page: Handle,
    _id: i32,
) -> *const c_char {
    if let Some(arc) = STEXT_PAGES.get(page) {
        if let Ok(p) = arc.lock() {
            let mut html = String::from("<div class=\"page\">\n");

            for block in &p.blocks {
                if block.block_type == StextBlockType::Text {
                    html.push_str("<p>");
                    for line in &block.lines {
                        html.push_str("<span>");
                        for ch in &line.chars {
                            if let Some(c) = char::from_u32(ch.c as u32) {
                                match c {
                                    '<' => html.push_str("&lt;"),
                                    '>' => html.push_str("&gt;"),
                                    '&' => html.push_str("&amp;"),
                                    '"' => html.push_str("&quot;"),
                                    _ => html.push(c),
                                }
                            }
                        }
                        html.push_str("</span><br/>\n");
                    }
                    html.push_str("</p>\n");
                }
            }
            html.push_str("</div>\n");

            if let Ok(cstr) = CString::new(html) {
                TEXT_OUTPUT.with(|cell| {
                    *cell.borrow_mut() = Some(cstr);
                });
                return TEXT_OUTPUT.with(|cell| {
                    cell.borrow()
                        .as_ref()
                        .map(|s| s.as_ptr())
                        .unwrap_or(std::ptr::null())
                });
            }
        }
    }
    std::ptr::null()
}

/// Print stext page as XML
#[unsafe(no_mangle)]
pub extern "C" fn fz_print_stext_page_as_xml(
    _ctx: Handle,
    _output: Handle,
    page: Handle,
    _id: i32,
) -> *const c_char {
    if let Some(arc) = STEXT_PAGES.get(page) {
        if let Ok(p) = arc.lock() {
            let mut xml = String::from("<?xml version=\"1.0\"?>\n<page>\n");

            for (bi, block) in p.blocks.iter().enumerate() {
                if block.block_type == StextBlockType::Text {
                    xml.push_str(&format!(
                        "<block type=\"text\" id=\"{}\" bbox=\"{} {} {} {}\">\n",
                        bi, block.bbox.x0, block.bbox.y0, block.bbox.x1, block.bbox.y1
                    ));

                    for line in &block.lines {
                        xml.push_str(&format!(
                            "<line wmode=\"{}\" bbox=\"{} {} {} {}\">\n",
                            line.wmode, line.bbox.x0, line.bbox.y0, line.bbox.x1, line.bbox.y1
                        ));

                        for ch in &line.chars {
                            let c = char::from_u32(ch.c as u32).unwrap_or('?');
                            let c_escaped = match c {
                                '<' => "&lt;".to_string(),
                                '>' => "&gt;".to_string(),
                                '&' => "&amp;".to_string(),
                                '"' => "&quot;".to_string(),
                                _ => c.to_string(),
                            };
                            xml.push_str(&format!(
                                "<char c=\"{}\" x=\"{}\" y=\"{}\" size=\"{}\"/>\n",
                                c_escaped, ch.origin.x, ch.origin.y, ch.size
                            ));
                        }

                        xml.push_str("</line>\n");
                    }
                    xml.push_str("</block>\n");
                }
            }
            xml.push_str("</page>\n");

            if let Ok(cstr) = CString::new(xml) {
                TEXT_OUTPUT.with(|cell| {
                    *cell.borrow_mut() = Some(cstr);
                });
                return TEXT_OUTPUT.with(|cell| {
                    cell.borrow()
                        .as_ref()
                        .map(|s| s.as_ptr())
                        .unwrap_or(std::ptr::null())
                });
            }
        }
    }
    std::ptr::null()
}

/// Print stext page as JSON
#[unsafe(no_mangle)]
pub extern "C" fn fz_print_stext_page_as_json(
    _ctx: Handle,
    _output: Handle,
    page: Handle,
    _scale: f32,
) -> *const c_char {
    if let Some(arc) = STEXT_PAGES.get(page) {
        if let Ok(p) = arc.lock() {
            let mut json = String::from("{\"blocks\":[\n");
            let mut first_block = true;

            for block in &p.blocks {
                if block.block_type == StextBlockType::Text {
                    if !first_block {
                        json.push_str(",\n");
                    }
                    first_block = false;

                    json.push_str(&format!(
                        "{{\"type\":\"text\",\"bbox\":[{},{},{},{}],\"lines\":[\n",
                        block.bbox.x0, block.bbox.y0, block.bbox.x1, block.bbox.y1
                    ));

                    let mut first_line = true;
                    for line in &block.lines {
                        if !first_line {
                            json.push_str(",\n");
                        }
                        first_line = false;

                        let mut line_text = String::new();
                        for ch in &line.chars {
                            if let Some(c) = char::from_u32(ch.c as u32) {
                                // Escape JSON special characters
                                match c {
                                    '"' => line_text.push_str("\\\""),
                                    '\\' => line_text.push_str("\\\\"),
                                    '\n' => line_text.push_str("\\n"),
                                    '\r' => line_text.push_str("\\r"),
                                    '\t' => line_text.push_str("\\t"),
                                    _ if c.is_control() => {
                                        line_text.push_str(&format!("\\u{:04x}", c as u32))
                                    }
                                    _ => line_text.push(c),
                                }
                            }
                        }

                        json.push_str(&format!(
                            "{{\"bbox\":[{},{},{},{}],\"text\":\"{}\"}}",
                            line.bbox.x0, line.bbox.y0, line.bbox.x1, line.bbox.y1, line_text
                        ));
                    }
                    json.push_str("\n]}");
                }
            }
            json.push_str("\n]}");

            if let Ok(cstr) = CString::new(json) {
                TEXT_OUTPUT.with(|cell| {
                    *cell.borrow_mut() = Some(cstr);
                });
                return TEXT_OUTPUT.with(|cell| {
                    cell.borrow()
                        .as_ref()
                        .map(|s| s.as_ptr())
                        .unwrap_or(std::ptr::null())
                });
            }
        }
    }
    std::ptr::null()
}

// ============================================================================
// Search Functions
// ============================================================================

/// Search for text in stext page
#[unsafe(no_mangle)]
pub extern "C" fn fz_search_stext_page(
    _ctx: Handle,
    page: Handle,
    needle: *const c_char,
    hit_mark: *mut i32,
    hit_bbox: *mut FzQuad,
    hit_max: i32,
) -> i32 {
    if needle.is_null() || hit_max <= 0 {
        return 0;
    }

    let needle_str = match unsafe { CStr::from_ptr(needle) }.to_str() {
        Ok(s) => s.to_lowercase(),
        Err(_) => return 0,
    };

    if let Some(arc) = STEXT_PAGES.get(page) {
        if let Ok(p) = arc.lock() {
            let mut hits = 0;
            let mut hit_idx = 0;

            for block in &p.blocks {
                if block.block_type != StextBlockType::Text {
                    continue;
                }

                for line in &block.lines {
                    // Build line text
                    let line_text: String = line
                        .chars
                        .iter()
                        .filter_map(|ch| char::from_u32(ch.c as u32))
                        .collect::<String>()
                        .to_lowercase();

                    // Find all occurrences
                    let mut search_pos = 0;
                    while let Some(pos) = line_text[search_pos..].find(&needle_str) {
                        let actual_pos = search_pos + pos;

                        if hit_idx < hit_max {
                            // Calculate bounding quad for the match
                            let start_char = actual_pos;
                            let end_char = (actual_pos + needle_str.len()).min(line.chars.len());

                            if start_char < line.chars.len() && end_char > 0 {
                                let first_ch = &line.chars[start_char];
                                let last_ch = &line.chars[end_char.saturating_sub(1)];

                                if !hit_bbox.is_null() {
                                    unsafe {
                                        let quad = hit_bbox.add(hit_idx as usize);
                                        (*quad).ul = [first_ch.quad.ul_x, first_ch.quad.ul_y];
                                        (*quad).ur = [last_ch.quad.ur_x, last_ch.quad.ur_y];
                                        (*quad).ll = [first_ch.quad.ll_x, first_ch.quad.ll_y];
                                        (*quad).lr = [last_ch.quad.lr_x, last_ch.quad.lr_y];
                                    }
                                }

                                if !hit_mark.is_null() {
                                    unsafe {
                                        *hit_mark.add(hit_idx as usize) = hits;
                                    }
                                }

                                hit_idx += 1;
                            }
                        }

                        hits += 1;
                        search_pos = actual_pos + 1;
                    }
                }
            }

            return hit_idx;
        }
    }
    0
}

// ============================================================================
// Selection Functions
// ============================================================================

/// Highlight text selection between two points
#[unsafe(no_mangle)]
pub extern "C" fn fz_highlight_selection(
    _ctx: Handle,
    page: Handle,
    a_x: f32,
    a_y: f32,
    b_x: f32,
    b_y: f32,
    quads: *mut FzQuad,
    max_quads: i32,
) -> i32 {
    if max_quads <= 0 {
        return 0;
    }

    if let Some(arc) = STEXT_PAGES.get(page) {
        if let Ok(p) = arc.lock() {
            let mut quad_count = 0;

            // Simple implementation: find all chars between points
            let min_x = a_x.min(b_x);
            let max_x = a_x.max(b_x);
            let min_y = a_y.min(b_y);
            let max_y = a_y.max(b_y);

            for block in &p.blocks {
                if block.block_type != StextBlockType::Text {
                    continue;
                }

                for line in &block.lines {
                    // Check if line overlaps selection
                    if line.bbox.y1 < min_y || line.bbox.y0 > max_y {
                        continue;
                    }

                    for ch in &line.chars {
                        let cx = (ch.quad.ul_x + ch.quad.ur_x) / 2.0;
                        let cy = (ch.quad.ul_y + ch.quad.ll_y) / 2.0;

                        if cx >= min_x && cx <= max_x && cy >= min_y && cy <= max_y {
                            if quad_count < max_quads && !quads.is_null() {
                                unsafe {
                                    let quad = quads.add(quad_count as usize);
                                    (*quad).ul = [ch.quad.ul_x, ch.quad.ul_y];
                                    (*quad).ur = [ch.quad.ur_x, ch.quad.ur_y];
                                    (*quad).ll = [ch.quad.ll_x, ch.quad.ll_y];
                                    (*quad).lr = [ch.quad.lr_x, ch.quad.lr_y];
                                }
                            }
                            quad_count += 1;
                        }
                    }
                }
            }

            return quad_count.min(max_quads);
        }
    }
    0
}

/// Copy text from selection
#[unsafe(no_mangle)]
pub extern "C" fn fz_copy_selection(
    _ctx: Handle,
    page: Handle,
    a_x: f32,
    a_y: f32,
    b_x: f32,
    b_y: f32,
    crlf: i32,
) -> *const c_char {
    if let Some(arc) = STEXT_PAGES.get(page) {
        if let Ok(p) = arc.lock() {
            let mut text = String::new();
            let line_ending = if crlf != 0 { "\r\n" } else { "\n" };

            let min_x = a_x.min(b_x);
            let max_x = a_x.max(b_x);
            let min_y = a_y.min(b_y);
            let max_y = a_y.max(b_y);

            for block in &p.blocks {
                if block.block_type != StextBlockType::Text {
                    continue;
                }

                for line in &block.lines {
                    if line.bbox.y1 < min_y || line.bbox.y0 > max_y {
                        continue;
                    }

                    let mut line_has_selection = false;
                    for ch in &line.chars {
                        let cx = (ch.quad.ul_x + ch.quad.ur_x) / 2.0;
                        let cy = (ch.quad.ul_y + ch.quad.ll_y) / 2.0;

                        if cx >= min_x && cx <= max_x && cy >= min_y && cy <= max_y {
                            if let Some(c) = char::from_u32(ch.c as u32) {
                                text.push(c);
                                line_has_selection = true;
                            }
                        }
                    }

                    if line_has_selection {
                        text.push_str(line_ending);
                    }
                }
            }

            if let Ok(cstr) = CString::new(text) {
                TEXT_OUTPUT.with(|cell| {
                    *cell.borrow_mut() = Some(cstr);
                });
                return TEXT_OUTPUT.with(|cell| {
                    cell.borrow()
                        .as_ref()
                        .map(|s| s.as_ptr())
                        .unwrap_or(std::ptr::null())
                });
            }
        }
    }
    std::ptr::null()
}

/// Copy text from rectangle
#[unsafe(no_mangle)]
pub extern "C" fn fz_copy_rectangle(
    ctx: Handle,
    page: Handle,
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
    crlf: i32,
) -> *const c_char {
    fz_copy_selection(ctx, page, x0, y0, x1, y1, crlf)
}

// ============================================================================
// Segmentation Functions
// ============================================================================

/// Segment stext page (analyze structure)
#[unsafe(no_mangle)]
pub extern "C" fn fz_segment_stext_page(_ctx: Handle, page: Handle) -> i32 {
    if let Some(arc) = STEXT_PAGES.get(page) {
        if let Ok(mut p) = arc.lock() {
            // Simple segmentation: group blocks by vertical proximity
            let mut changes = 0;

            if p.blocks.len() > 1 {
                // Sort blocks by vertical position
                p.blocks.sort_by(|a, b| {
                    a.bbox
                        .y0
                        .partial_cmp(&b.bbox.y0)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
                changes = 1;
            }

            return changes;
        }
    }
    0
}

/// Break paragraphs in stext page
#[unsafe(no_mangle)]
pub extern "C" fn fz_paragraph_break(_ctx: Handle, page: Handle) {
    if let Some(arc) = STEXT_PAGES.get(page) {
        if let Ok(mut p) = arc.lock() {
            // Analyze line spacing to detect paragraph breaks
            for block in &mut p.blocks {
                if block.block_type != StextBlockType::Text || block.lines.len() < 2 {
                    continue;
                }

                // Calculate average line height
                let mut total_height = 0.0;
                for line in &block.lines {
                    total_height += line.bbox.y1 - line.bbox.y0;
                }
                let avg_height = total_height / block.lines.len() as f32;

                // Mark lines with larger gaps as paragraph breaks
                for i in 1..block.lines.len() {
                    let gap = block.lines[i].bbox.y0 - block.lines[i - 1].bbox.y1;
                    if gap > avg_height * 0.5 {
                        // Mark with flag (bit 0 = joined)
                        block.lines[i].flags |= 0x02; // Using bit 1 for paragraph break
                    }
                }
            }
        }
    }
}

/// Hunt for tables in stext page
#[unsafe(no_mangle)]
pub extern "C" fn fz_table_hunt(_ctx: Handle, page: Handle) {
    if let Some(arc) = STEXT_PAGES.get(page) {
        if let Ok(mut p) = arc.lock() {
            // Simple table detection: look for aligned columns
            for block in &mut p.blocks {
                if block.block_type != StextBlockType::Text || block.lines.len() < 2 {
                    continue;
                }

                // Collect x positions of word starts
                let mut x_positions: Vec<f32> = Vec::new();

                for line in &block.lines {
                    let mut in_word = false;
                    for ch in &line.chars {
                        if ch.c == ' ' as i32 {
                            in_word = false;
                        } else if !in_word {
                            x_positions.push(ch.origin.x);
                            in_word = true;
                        }
                    }
                }

                // If we have many aligned x positions, might be a table
                // (This is a simplified heuristic)
                if x_positions.len() > 10 {
                    // Sort and look for clusters
                    x_positions
                        .sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

                    // Count positions within tolerance
                    let tolerance = 5.0;
                    let mut cluster_count = 1;
                    for i in 1..x_positions.len() {
                        if x_positions[i] - x_positions[i - 1] > tolerance {
                            cluster_count += 1;
                        }
                    }

                    // Mark as potential table if multiple columns detected
                    if cluster_count >= 3 {
                        block.text_flags |= 0x4000; // FZ_STEXT_TABLE_HUNT flag
                    }
                }
            }
        }
    }
}

// ============================================================================
// Options Functions
// ============================================================================

/// Parse stext options from string
#[unsafe(no_mangle)]
pub extern "C" fn fz_parse_stext_options(
    _ctx: Handle,
    opts: *mut StextOptions,
    string: *const c_char,
) -> *mut StextOptions {
    if opts.is_null() {
        return std::ptr::null_mut();
    }

    let options = unsafe { &mut *opts };

    if !string.is_null() {
        if let Ok(s) = unsafe { CStr::from_ptr(string) }.to_str() {
            for part in s.split(',') {
                let part = part.trim();
                match part {
                    "preserve-ligatures" => options.flags |= StextFlag::PreserveLigatures as i32,
                    "preserve-whitespace" => options.flags |= StextFlag::PreserveWhitespace as i32,
                    "preserve-images" => options.flags |= StextFlag::PreserveImages as i32,
                    "inhibit-spaces" => options.flags |= StextFlag::InhibitSpaces as i32,
                    "dehyphenate" => options.flags |= StextFlag::Dehyphenate as i32,
                    "preserve-spans" => options.flags |= StextFlag::PreserveSpans as i32,
                    "clip" => options.flags |= StextFlag::Clip as i32,
                    "structure" => options.flags |= StextFlag::CollectStructure as i32,
                    "segment" => options.flags |= StextFlag::Segment as i32,
                    "paragraph-break" => options.flags |= StextFlag::ParagraphBreak as i32,
                    "table-hunt" => options.flags |= StextFlag::TableHunt as i32,
                    _ => {
                        // Check for scale=N
                        if let Some(scale_str) = part.strip_prefix("scale=") {
                            if let Ok(scale) = scale_str.parse::<f32>() {
                                options.scale = scale;
                            }
                        }
                    }
                }
            }
        }
    }

    opts
}

/// Create default stext options
#[unsafe(no_mangle)]
pub extern "C" fn fz_default_stext_options(
    _ctx: Handle,
    opts: *mut StextOptions,
) -> *mut StextOptions {
    if opts.is_null() {
        return std::ptr::null_mut();
    }

    unsafe {
        (*opts).flags = 0;
        (*opts).scale = 1.0;
        (*opts).clip = Rect::default();
    }

    opts
}

// ============================================================================
// Block Manipulation Functions
// ============================================================================

/// Add a text block to stext page
#[unsafe(no_mangle)]
pub extern "C" fn fz_add_stext_block(
    _ctx: Handle,
    page: Handle,
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
) -> i32 {
    if let Some(arc) = STEXT_PAGES.get(page) {
        if let Ok(mut p) = arc.lock() {
            let block = StextBlock {
                block_type: StextBlockType::Text,
                id: p.blocks.len() as i32,
                bbox: Rect { x0, y0, x1, y1 },
                lines: Vec::new(),
                image: None,
                struct_down: None,
                struct_index: 0,
                text_flags: 0,
            };
            let idx = p.blocks.len() as i32;
            p.blocks.push(block);
            return idx;
        }
    }
    -1
}

/// Add a line to a text block
#[unsafe(no_mangle)]
pub extern "C" fn fz_add_stext_line(
    _ctx: Handle,
    page: Handle,
    block_idx: i32,
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
) -> i32 {
    if let Some(arc) = STEXT_PAGES.get(page) {
        if let Ok(mut p) = arc.lock() {
            if let Some(block) = p.blocks.get_mut(block_idx as usize) {
                let line = StextLine {
                    wmode: 0,
                    flags: 0,
                    dir: Point { x: 1.0, y: 0.0 },
                    bbox: Rect { x0, y0, x1, y1 },
                    chars: Vec::new(),
                };
                let idx = block.lines.len() as i32;
                block.lines.push(line);
                return idx;
            }
        }
    }
    -1
}

/// Add a character to a line
#[unsafe(no_mangle)]
pub extern "C" fn fz_add_stext_char(
    _ctx: Handle,
    page: Handle,
    block_idx: i32,
    line_idx: i32,
    c: i32,
    x: f32,
    y: f32,
    size: f32,
) -> i32 {
    if let Some(arc) = STEXT_PAGES.get(page) {
        if let Ok(mut p) = arc.lock() {
            if let Some(block) = p.blocks.get_mut(block_idx as usize) {
                if let Some(line) = block.lines.get_mut(line_idx as usize) {
                    let ch = StextChar {
                        c,
                        bidi: 0,
                        flags: 0,
                        argb: 0xFF000000,
                        origin: Point { x, y },
                        quad: Quad {
                            ul_x: x,
                            ul_y: y - size,
                            ur_x: x + size * 0.6, // Approximate width
                            ur_y: y - size,
                            ll_x: x,
                            ll_y: y,
                            lr_x: x + size * 0.6,
                            lr_y: y,
                        },
                        size,
                        font: None,
                    };
                    let idx = line.chars.len() as i32;
                    line.chars.push(ch);
                    return idx;
                }
            }
        }
    }
    -1
}

// ============================================================================
// Block/Line/Char Count Functions
// ============================================================================

/// Get number of blocks in page
#[unsafe(no_mangle)]
pub extern "C" fn fz_stext_block_count(_ctx: Handle, page: Handle) -> i32 {
    if let Some(arc) = STEXT_PAGES.get(page) {
        if let Ok(p) = arc.lock() {
            return p.blocks.len() as i32;
        }
    }
    0
}

/// Get number of lines in block
#[unsafe(no_mangle)]
pub extern "C" fn fz_stext_line_count(_ctx: Handle, page: Handle, block_idx: i32) -> i32 {
    if let Some(arc) = STEXT_PAGES.get(page) {
        if let Ok(p) = arc.lock() {
            if let Some(block) = p.blocks.get(block_idx as usize) {
                return block.lines.len() as i32;
            }
        }
    }
    0
}

/// Get number of chars in line
#[unsafe(no_mangle)]
pub extern "C" fn fz_stext_char_count(
    _ctx: Handle,
    page: Handle,
    block_idx: i32,
    line_idx: i32,
) -> i32 {
    if let Some(arc) = STEXT_PAGES.get(page) {
        if let Ok(p) = arc.lock() {
            if let Some(block) = p.blocks.get(block_idx as usize) {
                if let Some(line) = block.lines.get(line_idx as usize) {
                    return line.chars.len() as i32;
                }
            }
        }
    }
    0
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stext_page_create() {
        let ctx = 0;
        let page = fz_new_stext_page(ctx, 0.0, 0.0, 612.0, 792.0);
        assert!(page > 0);
        fz_drop_stext_page(ctx, page);
    }

    #[test]
    fn test_stext_add_content() {
        let ctx = 0;
        let page = fz_new_stext_page(ctx, 0.0, 0.0, 612.0, 792.0);

        let block_idx = fz_add_stext_block(ctx, page, 0.0, 0.0, 100.0, 50.0);
        assert_eq!(block_idx, 0);

        let line_idx = fz_add_stext_line(ctx, page, block_idx, 0.0, 0.0, 100.0, 12.0);
        assert_eq!(line_idx, 0);

        let char_idx =
            fz_add_stext_char(ctx, page, block_idx, line_idx, 'H' as i32, 0.0, 12.0, 12.0);
        assert_eq!(char_idx, 0);

        let char_idx =
            fz_add_stext_char(ctx, page, block_idx, line_idx, 'i' as i32, 8.0, 12.0, 12.0);
        assert_eq!(char_idx, 1);

        assert_eq!(fz_stext_block_count(ctx, page), 1);
        assert_eq!(fz_stext_line_count(ctx, page, 0), 1);
        assert_eq!(fz_stext_char_count(ctx, page, 0, 0), 2);

        fz_drop_stext_page(ctx, page);
    }

    #[test]
    fn test_stext_text_extraction() {
        let ctx = 0;
        let page = fz_new_stext_page(ctx, 0.0, 0.0, 612.0, 792.0);

        let block = fz_add_stext_block(ctx, page, 0.0, 0.0, 100.0, 50.0);
        let line = fz_add_stext_line(ctx, page, block, 0.0, 0.0, 100.0, 12.0);

        for (i, c) in "Hello".chars().enumerate() {
            fz_add_stext_char(ctx, page, block, line, c as i32, (i * 8) as f32, 12.0, 12.0);
        }

        let text = fz_stext_page_as_text(ctx, page);
        assert!(!text.is_null());

        let text_str = unsafe { CStr::from_ptr(text) }.to_str().unwrap();
        assert!(text_str.contains("Hello"));

        fz_drop_stext_page(ctx, page);
    }

    #[test]
    fn test_stext_search() {
        let ctx = 0;
        let page = fz_new_stext_page(ctx, 0.0, 0.0, 612.0, 792.0);

        let block = fz_add_stext_block(ctx, page, 0.0, 0.0, 200.0, 50.0);
        let line = fz_add_stext_line(ctx, page, block, 0.0, 0.0, 200.0, 12.0);

        for (i, c) in "Hello World".chars().enumerate() {
            fz_add_stext_char(ctx, page, block, line, c as i32, (i * 8) as f32, 12.0, 12.0);
        }

        let needle = CString::new("World").unwrap();
        let mut quads = [FzQuad::default(); 10];
        let mut marks = [0i32; 10];

        let hits = fz_search_stext_page(
            ctx,
            page,
            needle.as_ptr(),
            marks.as_mut_ptr(),
            quads.as_mut_ptr(),
            10,
        );
        assert_eq!(hits, 1);

        fz_drop_stext_page(ctx, page);
    }

    #[test]
    fn test_stext_options() {
        let ctx = 0;
        let mut opts = StextOptions::default();

        let options_str = CString::new("preserve-images,dehyphenate,scale=2.0").unwrap();
        fz_parse_stext_options(ctx, &mut opts, options_str.as_ptr());

        assert!(opts.flags & StextFlag::PreserveImages as i32 != 0);
        assert!(opts.flags & StextFlag::Dehyphenate as i32 != 0);
        assert!((opts.scale - 2.0).abs() < 0.001);
    }

    #[test]
    fn test_stext_html_output() {
        let ctx = 0;
        let page = fz_new_stext_page(ctx, 0.0, 0.0, 612.0, 792.0);

        let block = fz_add_stext_block(ctx, page, 0.0, 0.0, 100.0, 50.0);
        let line = fz_add_stext_line(ctx, page, block, 0.0, 0.0, 100.0, 12.0);

        for (i, c) in "<Test>".chars().enumerate() {
            fz_add_stext_char(ctx, page, block, line, c as i32, (i * 8) as f32, 12.0, 12.0);
        }

        let html = fz_print_stext_page_as_html(ctx, 0, page, 0);
        assert!(!html.is_null());

        let html_str = unsafe { CStr::from_ptr(html) }.to_str().unwrap();
        assert!(html_str.contains("&lt;Test&gt;")); // Properly escaped

        fz_drop_stext_page(ctx, page);
    }

    #[test]
    fn test_stext_json_output() {
        let ctx = 0;
        let page = fz_new_stext_page(ctx, 0.0, 0.0, 612.0, 792.0);

        let block = fz_add_stext_block(ctx, page, 0.0, 0.0, 100.0, 50.0);
        let line = fz_add_stext_line(ctx, page, block, 0.0, 0.0, 100.0, 12.0);

        for (i, c) in "Test".chars().enumerate() {
            fz_add_stext_char(ctx, page, block, line, c as i32, (i * 8) as f32, 12.0, 12.0);
        }

        let json = fz_print_stext_page_as_json(ctx, 0, page, 1.0);
        assert!(!json.is_null());

        let json_str = unsafe { CStr::from_ptr(json) }.to_str().unwrap();
        assert!(json_str.contains("\"blocks\""));
        assert!(json_str.contains("\"text\":\"Test\""));

        fz_drop_stext_page(ctx, page);
    }

    #[test]
    fn test_stext_selection() {
        let ctx = 0;
        let page = fz_new_stext_page(ctx, 0.0, 0.0, 612.0, 792.0);

        let block = fz_add_stext_block(ctx, page, 0.0, 0.0, 100.0, 50.0);
        let line = fz_add_stext_line(ctx, page, block, 0.0, 0.0, 100.0, 12.0);

        for (i, c) in "Select Me".chars().enumerate() {
            fz_add_stext_char(ctx, page, block, line, c as i32, (i * 8) as f32, 12.0, 12.0);
        }

        let text = fz_copy_selection(ctx, page, 0.0, 0.0, 100.0, 20.0, 0);
        assert!(!text.is_null());

        let text_str = unsafe { CStr::from_ptr(text) }.to_str().unwrap();
        assert!(text_str.contains("Select Me"));

        fz_drop_stext_page(ctx, page);
    }
}
