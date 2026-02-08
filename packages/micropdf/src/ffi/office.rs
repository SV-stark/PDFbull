//! Office Document Formats FFI Module
//!
//! Provides support for Microsoft Office Open XML formats (DOCX, XLSX, PPTX)
//! and OpenDocument formats (ODT, ODS, ODP). These formats are ZIP-based
//! archives containing XML content.

use crate::ffi::{Handle, HandleStore};
use std::collections::HashMap;
use std::ffi::{CStr, CString, c_char};
use std::ptr;
use std::sync::LazyLock;

// ============================================================================
// Type Aliases
// ============================================================================

type ContextHandle = Handle;
type StreamHandle = Handle;
type ArchiveHandle = Handle;

// ============================================================================
// Document Type Constants
// ============================================================================

/// Microsoft Word document (DOCX)
pub const OFFICE_TYPE_DOCX: i32 = 0;
/// Microsoft Excel spreadsheet (XLSX)
pub const OFFICE_TYPE_XLSX: i32 = 1;
/// Microsoft PowerPoint presentation (PPTX)
pub const OFFICE_TYPE_PPTX: i32 = 2;
/// OpenDocument Text (ODT)
pub const OFFICE_TYPE_ODT: i32 = 3;
/// OpenDocument Spreadsheet (ODS)
pub const OFFICE_TYPE_ODS: i32 = 4;
/// OpenDocument Presentation (ODP)
pub const OFFICE_TYPE_ODP: i32 = 5;
/// Unknown office format
pub const OFFICE_TYPE_UNKNOWN: i32 = 99;

// ============================================================================
// Content Type Constants
// ============================================================================

/// Paragraph content
pub const OFFICE_CONTENT_PARAGRAPH: i32 = 0;
/// Table content
pub const OFFICE_CONTENT_TABLE: i32 = 1;
/// Image content
pub const OFFICE_CONTENT_IMAGE: i32 = 2;
/// Heading content
pub const OFFICE_CONTENT_HEADING: i32 = 3;
/// List content
pub const OFFICE_CONTENT_LIST: i32 = 4;
/// Page break
pub const OFFICE_CONTENT_PAGE_BREAK: i32 = 5;
/// Section break
pub const OFFICE_CONTENT_SECTION_BREAK: i32 = 6;
/// Drawing/shape
pub const OFFICE_CONTENT_DRAWING: i32 = 7;
/// Chart
pub const OFFICE_CONTENT_CHART: i32 = 8;
/// Hyperlink
pub const OFFICE_CONTENT_HYPERLINK: i32 = 9;
/// Cell (spreadsheet)
pub const OFFICE_CONTENT_CELL: i32 = 10;
/// Row (spreadsheet)
pub const OFFICE_CONTENT_ROW: i32 = 11;
/// Slide (presentation)
pub const OFFICE_CONTENT_SLIDE: i32 = 12;
/// Text run
pub const OFFICE_CONTENT_RUN: i32 = 13;

// ============================================================================
// Text Alignment Constants
// ============================================================================

/// Left alignment
pub const OFFICE_ALIGN_LEFT: i32 = 0;
/// Center alignment
pub const OFFICE_ALIGN_CENTER: i32 = 1;
/// Right alignment
pub const OFFICE_ALIGN_RIGHT: i32 = 2;
/// Justify alignment
pub const OFFICE_ALIGN_JUSTIFY: i32 = 3;

// ============================================================================
// Cell Type Constants (Spreadsheet)
// ============================================================================

/// Empty cell
pub const OFFICE_CELL_EMPTY: i32 = 0;
/// String value
pub const OFFICE_CELL_STRING: i32 = 1;
/// Number value
pub const OFFICE_CELL_NUMBER: i32 = 2;
/// Boolean value
pub const OFFICE_CELL_BOOLEAN: i32 = 3;
/// Formula
pub const OFFICE_CELL_FORMULA: i32 = 4;
/// Error
pub const OFFICE_CELL_ERROR: i32 = 5;
/// Date
pub const OFFICE_CELL_DATE: i32 = 6;

// ============================================================================
// Text Style
// ============================================================================

/// Text style properties
#[derive(Debug, Clone, Default)]
pub struct TextStyle {
    /// Font family
    pub font_family: Option<String>,
    /// Font size (points)
    pub font_size: f32,
    /// Bold
    pub bold: bool,
    /// Italic
    pub italic: bool,
    /// Underline
    pub underline: bool,
    /// Strike through
    pub strike: bool,
    /// Subscript
    pub subscript: bool,
    /// Superscript
    pub superscript: bool,
    /// Text color (RGB)
    pub color: Option<u32>,
    /// Highlight color (RGB)
    pub highlight: Option<u32>,
}

impl TextStyle {
    pub fn new() -> Self {
        Self {
            font_size: 11.0,
            ..Default::default()
        }
    }
}

// ============================================================================
// Paragraph Style
// ============================================================================

/// Paragraph style properties
#[derive(Debug, Clone, Default)]
pub struct ParagraphStyle {
    /// Text alignment
    pub alignment: i32,
    /// Line spacing (multiplier)
    pub line_spacing: f32,
    /// Space before (points)
    pub space_before: f32,
    /// Space after (points)
    pub space_after: f32,
    /// First line indent (points)
    pub first_line_indent: f32,
    /// Left indent (points)
    pub left_indent: f32,
    /// Right indent (points)
    pub right_indent: f32,
    /// Outline level (0-9, 0 = body text)
    pub outline_level: i32,
}

impl ParagraphStyle {
    pub fn new() -> Self {
        Self {
            line_spacing: 1.0,
            space_after: 8.0,
            ..Default::default()
        }
    }
}

// ============================================================================
// Content Element
// ============================================================================

/// A content element in an office document
#[derive(Debug, Clone)]
pub struct ContentElement {
    /// Content type
    pub content_type: i32,
    /// Text content
    pub text: Option<String>,
    /// Text style
    pub text_style: TextStyle,
    /// Paragraph style
    pub para_style: ParagraphStyle,
    /// Children elements
    pub children: Vec<ContentElement>,
    /// Attributes
    pub attributes: HashMap<String, String>,
}

impl ContentElement {
    pub fn new(content_type: i32) -> Self {
        Self {
            content_type,
            text: None,
            text_style: TextStyle::new(),
            para_style: ParagraphStyle::new(),
            children: Vec::new(),
            attributes: HashMap::new(),
        }
    }

    pub fn paragraph(text: &str) -> Self {
        let mut elem = Self::new(OFFICE_CONTENT_PARAGRAPH);
        elem.text = Some(text.to_string());
        elem
    }

    pub fn heading(text: &str, level: i32) -> Self {
        let mut elem = Self::new(OFFICE_CONTENT_HEADING);
        elem.text = Some(text.to_string());
        elem.para_style.outline_level = level;
        elem
    }
}

// ============================================================================
// Spreadsheet Cell
// ============================================================================

/// A cell in a spreadsheet
#[derive(Debug, Clone)]
pub struct SpreadsheetCell {
    /// Row index (0-based)
    pub row: i32,
    /// Column index (0-based)
    pub col: i32,
    /// Cell type
    pub cell_type: i32,
    /// String value
    pub string_value: Option<String>,
    /// Numeric value
    pub number_value: f64,
    /// Boolean value
    pub bool_value: bool,
    /// Formula
    pub formula: Option<String>,
    /// Text style
    pub style: TextStyle,
}

impl SpreadsheetCell {
    pub fn new(row: i32, col: i32) -> Self {
        Self {
            row,
            col,
            cell_type: OFFICE_CELL_EMPTY,
            string_value: None,
            number_value: 0.0,
            bool_value: false,
            formula: None,
            style: TextStyle::new(),
        }
    }

    pub fn string(row: i32, col: i32, value: &str) -> Self {
        let mut cell = Self::new(row, col);
        cell.cell_type = OFFICE_CELL_STRING;
        cell.string_value = Some(value.to_string());
        cell
    }

    pub fn number(row: i32, col: i32, value: f64) -> Self {
        let mut cell = Self::new(row, col);
        cell.cell_type = OFFICE_CELL_NUMBER;
        cell.number_value = value;
        cell
    }

    /// Get column letter (A, B, ... Z, AA, AB, ...)
    pub fn column_letter(&self) -> String {
        let mut result = String::new();
        let mut col = self.col + 1;
        while col > 0 {
            let rem = ((col - 1) % 26) as u8;
            result.insert(0, (b'A' + rem) as char);
            col = (col - 1) / 26;
        }
        result
    }

    /// Get cell reference (A1, B2, etc.)
    pub fn reference(&self) -> String {
        format!("{}{}", self.column_letter(), self.row + 1)
    }
}

// ============================================================================
// Spreadsheet Sheet
// ============================================================================

/// A sheet in a spreadsheet
#[derive(Debug, Clone)]
pub struct Sheet {
    /// Sheet name
    pub name: String,
    /// Sheet index
    pub index: i32,
    /// Cells (row-major order)
    pub cells: Vec<SpreadsheetCell>,
    /// Row count
    pub row_count: i32,
    /// Column count
    pub col_count: i32,
}

impl Sheet {
    pub fn new(name: &str, index: i32) -> Self {
        Self {
            name: name.to_string(),
            index,
            cells: Vec::new(),
            row_count: 0,
            col_count: 0,
        }
    }

    pub fn add_cell(&mut self, cell: SpreadsheetCell) {
        if cell.row >= self.row_count {
            self.row_count = cell.row + 1;
        }
        if cell.col >= self.col_count {
            self.col_count = cell.col + 1;
        }
        self.cells.push(cell);
    }

    pub fn get_cell(&self, row: i32, col: i32) -> Option<&SpreadsheetCell> {
        self.cells.iter().find(|c| c.row == row && c.col == col)
    }
}

// ============================================================================
// Slide
// ============================================================================

/// A slide in a presentation
#[derive(Debug, Clone)]
pub struct Slide {
    /// Slide number (1-based)
    pub number: i32,
    /// Slide title
    pub title: Option<String>,
    /// Slide content elements
    pub content: Vec<ContentElement>,
    /// Speaker notes
    pub notes: Option<String>,
    /// Layout name
    pub layout: String,
}

impl Slide {
    pub fn new(number: i32) -> Self {
        Self {
            number,
            title: None,
            content: Vec::new(),
            notes: None,
            layout: "Title and Content".to_string(),
        }
    }
}

// ============================================================================
// Document Metadata
// ============================================================================

/// Office document metadata
#[derive(Debug, Clone, Default)]
pub struct OfficeMetadata {
    /// Document title
    pub title: Option<String>,
    /// Subject
    pub subject: Option<String>,
    /// Creator/author
    pub creator: Option<String>,
    /// Keywords
    pub keywords: Option<String>,
    /// Description
    pub description: Option<String>,
    /// Last modified by
    pub last_modified_by: Option<String>,
    /// Creation date
    pub created: Option<String>,
    /// Last modified date
    pub modified: Option<String>,
    /// Revision number
    pub revision: Option<i32>,
    /// Category
    pub category: Option<String>,
    /// Application name
    pub application: Option<String>,
}

impl OfficeMetadata {
    pub fn new() -> Self {
        Self::default()
    }
}

// ============================================================================
// Office Document
// ============================================================================

/// Office document structure
pub struct OfficeDocument {
    /// Context handle
    pub context: ContextHandle,
    /// Document type
    pub doc_type: i32,
    /// Metadata
    pub metadata: OfficeMetadata,
    /// Content elements (for DOCX/ODT)
    pub content: Vec<ContentElement>,
    /// Sheets (for XLSX/ODS)
    pub sheets: Vec<Sheet>,
    /// Slides (for PPTX/ODP)
    pub slides: Vec<Slide>,
    /// Page width (points)
    pub page_width: f32,
    /// Page height (points)
    pub page_height: f32,
    /// Margin top
    pub margin_top: f32,
    /// Margin bottom
    pub margin_bottom: f32,
    /// Margin left
    pub margin_left: f32,
    /// Margin right
    pub margin_right: f32,
}

impl OfficeDocument {
    pub fn new(context: ContextHandle, doc_type: i32) -> Self {
        Self {
            context,
            doc_type,
            metadata: OfficeMetadata::new(),
            content: Vec::new(),
            sheets: Vec::new(),
            slides: Vec::new(),
            page_width: 612.0, // US Letter
            page_height: 792.0,
            margin_top: 72.0, // 1 inch
            margin_bottom: 72.0,
            margin_left: 72.0,
            margin_right: 72.0,
        }
    }

    pub fn docx(context: ContextHandle) -> Self {
        Self::new(context, OFFICE_TYPE_DOCX)
    }

    pub fn xlsx(context: ContextHandle) -> Self {
        Self::new(context, OFFICE_TYPE_XLSX)
    }

    pub fn pptx(context: ContextHandle) -> Self {
        let mut doc = Self::new(context, OFFICE_TYPE_PPTX);
        // Standard 16:9 slide size
        doc.page_width = 960.0;
        doc.page_height = 540.0;
        doc
    }

    pub fn add_paragraph(&mut self, text: &str) {
        self.content.push(ContentElement::paragraph(text));
    }

    pub fn add_heading(&mut self, text: &str, level: i32) {
        self.content.push(ContentElement::heading(text, level));
    }

    pub fn add_sheet(&mut self, name: &str) -> i32 {
        let index = self.sheets.len() as i32;
        self.sheets.push(Sheet::new(name, index));
        index
    }

    pub fn add_slide(&mut self) -> i32 {
        let number = (self.slides.len() + 1) as i32;
        self.slides.push(Slide::new(number));
        number
    }
}

// ============================================================================
// Global Handle Store
// ============================================================================

pub static OFFICE_DOCUMENTS: LazyLock<HandleStore<OfficeDocument>> =
    LazyLock::new(HandleStore::new);

// ============================================================================
// FFI Functions - Document Management
// ============================================================================

/// Create a new office document.
#[unsafe(no_mangle)]
pub extern "C" fn office_new_document(ctx: ContextHandle, doc_type: i32) -> Handle {
    let doc = OfficeDocument::new(ctx, doc_type);
    OFFICE_DOCUMENTS.insert(doc)
}

/// Create a new DOCX document.
#[unsafe(no_mangle)]
pub extern "C" fn office_new_docx(ctx: ContextHandle) -> Handle {
    let doc = OfficeDocument::docx(ctx);
    OFFICE_DOCUMENTS.insert(doc)
}

/// Create a new XLSX document.
#[unsafe(no_mangle)]
pub extern "C" fn office_new_xlsx(ctx: ContextHandle) -> Handle {
    let doc = OfficeDocument::xlsx(ctx);
    OFFICE_DOCUMENTS.insert(doc)
}

/// Create a new PPTX document.
#[unsafe(no_mangle)]
pub extern "C" fn office_new_pptx(ctx: ContextHandle) -> Handle {
    let doc = OfficeDocument::pptx(ctx);
    OFFICE_DOCUMENTS.insert(doc)
}

/// Drop an office document.
#[unsafe(no_mangle)]
pub extern "C" fn office_drop_document(_ctx: ContextHandle, doc: Handle) {
    OFFICE_DOCUMENTS.remove(doc);
}

/// Open an office document from a file path.
#[unsafe(no_mangle)]
pub extern "C" fn office_open_document(ctx: ContextHandle, filename: *const c_char) -> Handle {
    if filename.is_null() {
        return 0;
    }

    let path = unsafe { CStr::from_ptr(filename).to_string_lossy() };

    // Detect type from extension
    let lower = path.to_lowercase();
    let doc_type = if lower.ends_with(".docx") {
        OFFICE_TYPE_DOCX
    } else if lower.ends_with(".xlsx") {
        OFFICE_TYPE_XLSX
    } else if lower.ends_with(".pptx") {
        OFFICE_TYPE_PPTX
    } else if lower.ends_with(".odt") {
        OFFICE_TYPE_ODT
    } else if lower.ends_with(".ods") {
        OFFICE_TYPE_ODS
    } else if lower.ends_with(".odp") {
        OFFICE_TYPE_ODP
    } else {
        OFFICE_TYPE_UNKNOWN
    };

    let doc = OfficeDocument::new(ctx, doc_type);
    OFFICE_DOCUMENTS.insert(doc)
}

// ============================================================================
// FFI Functions - Document Properties
// ============================================================================

/// Get document type.
#[unsafe(no_mangle)]
pub extern "C" fn office_get_type(_ctx: ContextHandle, doc: Handle) -> i32 {
    if let Some(d) = OFFICE_DOCUMENTS.get(doc) {
        let d = d.lock().unwrap();
        return d.doc_type;
    }
    OFFICE_TYPE_UNKNOWN
}

/// Get page/slide count.
#[unsafe(no_mangle)]
pub extern "C" fn office_page_count(_ctx: ContextHandle, doc: Handle) -> i32 {
    if let Some(d) = OFFICE_DOCUMENTS.get(doc) {
        let d = d.lock().unwrap();
        return match d.doc_type {
            OFFICE_TYPE_XLSX | OFFICE_TYPE_ODS => d.sheets.len() as i32,
            OFFICE_TYPE_PPTX | OFFICE_TYPE_ODP => d.slides.len() as i32,
            _ => 1, // DOCX/ODT typically have 1 continuous document
        };
    }
    0
}

/// Get page dimensions.
#[unsafe(no_mangle)]
pub extern "C" fn office_get_page_size(
    _ctx: ContextHandle,
    doc: Handle,
    width: *mut f32,
    height: *mut f32,
) -> i32 {
    if let Some(d) = OFFICE_DOCUMENTS.get(doc) {
        let d = d.lock().unwrap();
        if !width.is_null() {
            unsafe {
                *width = d.page_width;
            }
        }
        if !height.is_null() {
            unsafe {
                *height = d.page_height;
            }
        }
        return 1;
    }
    0
}

/// Set page dimensions.
#[unsafe(no_mangle)]
pub extern "C" fn office_set_page_size(
    _ctx: ContextHandle,
    doc: Handle,
    width: f32,
    height: f32,
) -> i32 {
    if let Some(d) = OFFICE_DOCUMENTS.get(doc) {
        let mut d = d.lock().unwrap();
        d.page_width = width;
        d.page_height = height;
        return 1;
    }
    0
}

// ============================================================================
// FFI Functions - Metadata
// ============================================================================

/// Get document title.
#[unsafe(no_mangle)]
pub extern "C" fn office_get_title(_ctx: ContextHandle, doc: Handle) -> *mut c_char {
    if let Some(d) = OFFICE_DOCUMENTS.get(doc) {
        let d = d.lock().unwrap();
        if let Some(ref title) = d.metadata.title {
            if let Ok(cstr) = CString::new(title.clone()) {
                return cstr.into_raw();
            }
        }
    }
    ptr::null_mut()
}

/// Set document title.
#[unsafe(no_mangle)]
pub extern "C" fn office_set_title(_ctx: ContextHandle, doc: Handle, title: *const c_char) -> i32 {
    if title.is_null() {
        return 0;
    }

    if let Some(d) = OFFICE_DOCUMENTS.get(doc) {
        let mut d = d.lock().unwrap();
        let t = unsafe { CStr::from_ptr(title).to_string_lossy().to_string() };
        d.metadata.title = Some(t);
        return 1;
    }
    0
}

/// Get document creator/author.
#[unsafe(no_mangle)]
pub extern "C" fn office_get_creator(_ctx: ContextHandle, doc: Handle) -> *mut c_char {
    if let Some(d) = OFFICE_DOCUMENTS.get(doc) {
        let d = d.lock().unwrap();
        if let Some(ref creator) = d.metadata.creator {
            if let Ok(cstr) = CString::new(creator.clone()) {
                return cstr.into_raw();
            }
        }
    }
    ptr::null_mut()
}

/// Set document creator/author.
#[unsafe(no_mangle)]
pub extern "C" fn office_set_creator(
    _ctx: ContextHandle,
    doc: Handle,
    creator: *const c_char,
) -> i32 {
    if creator.is_null() {
        return 0;
    }

    if let Some(d) = OFFICE_DOCUMENTS.get(doc) {
        let mut d = d.lock().unwrap();
        let c = unsafe { CStr::from_ptr(creator).to_string_lossy().to_string() };
        d.metadata.creator = Some(c);
        return 1;
    }
    0
}

// ============================================================================
// FFI Functions - DOCX Content
// ============================================================================

/// Add paragraph to document.
#[unsafe(no_mangle)]
pub extern "C" fn office_add_paragraph(
    _ctx: ContextHandle,
    doc: Handle,
    text: *const c_char,
) -> i32 {
    if text.is_null() {
        return 0;
    }

    if let Some(d) = OFFICE_DOCUMENTS.get(doc) {
        let mut d = d.lock().unwrap();
        let t = unsafe { CStr::from_ptr(text).to_string_lossy().to_string() };
        d.add_paragraph(&t);
        return 1;
    }
    0
}

/// Add heading to document.
#[unsafe(no_mangle)]
pub extern "C" fn office_add_heading(
    _ctx: ContextHandle,
    doc: Handle,
    text: *const c_char,
    level: i32,
) -> i32 {
    if text.is_null() {
        return 0;
    }

    if let Some(d) = OFFICE_DOCUMENTS.get(doc) {
        let mut d = d.lock().unwrap();
        let t = unsafe { CStr::from_ptr(text).to_string_lossy().to_string() };
        d.add_heading(&t, level);
        return 1;
    }
    0
}

/// Get content element count.
#[unsafe(no_mangle)]
pub extern "C" fn office_content_count(_ctx: ContextHandle, doc: Handle) -> i32 {
    if let Some(d) = OFFICE_DOCUMENTS.get(doc) {
        let d = d.lock().unwrap();
        return d.content.len() as i32;
    }
    0
}

// ============================================================================
// FFI Functions - XLSX Sheets
// ============================================================================

/// Add sheet to spreadsheet.
#[unsafe(no_mangle)]
pub extern "C" fn office_add_sheet(_ctx: ContextHandle, doc: Handle, name: *const c_char) -> i32 {
    if name.is_null() {
        return -1;
    }

    if let Some(d) = OFFICE_DOCUMENTS.get(doc) {
        let mut d = d.lock().unwrap();
        let n = unsafe { CStr::from_ptr(name).to_string_lossy().to_string() };
        return d.add_sheet(&n);
    }
    -1
}

/// Get sheet count.
#[unsafe(no_mangle)]
pub extern "C" fn office_sheet_count(_ctx: ContextHandle, doc: Handle) -> i32 {
    if let Some(d) = OFFICE_DOCUMENTS.get(doc) {
        let d = d.lock().unwrap();
        return d.sheets.len() as i32;
    }
    0
}

/// Get sheet name.
#[unsafe(no_mangle)]
pub extern "C" fn office_get_sheet_name(
    _ctx: ContextHandle,
    doc: Handle,
    sheet_idx: i32,
) -> *mut c_char {
    if let Some(d) = OFFICE_DOCUMENTS.get(doc) {
        let d = d.lock().unwrap();
        if let Some(sheet) = d.sheets.get(sheet_idx as usize) {
            if let Ok(cstr) = CString::new(sheet.name.clone()) {
                return cstr.into_raw();
            }
        }
    }
    ptr::null_mut()
}

/// Set cell string value.
#[unsafe(no_mangle)]
pub extern "C" fn office_set_cell_string(
    _ctx: ContextHandle,
    doc: Handle,
    sheet_idx: i32,
    row: i32,
    col: i32,
    value: *const c_char,
) -> i32 {
    if value.is_null() {
        return 0;
    }

    if let Some(d) = OFFICE_DOCUMENTS.get(doc) {
        let mut d = d.lock().unwrap();
        if let Some(sheet) = d.sheets.get_mut(sheet_idx as usize) {
            let v = unsafe { CStr::from_ptr(value).to_string_lossy().to_string() };
            let cell = SpreadsheetCell::string(row, col, &v);
            sheet.add_cell(cell);
            return 1;
        }
    }
    0
}

/// Set cell number value.
#[unsafe(no_mangle)]
pub extern "C" fn office_set_cell_number(
    _ctx: ContextHandle,
    doc: Handle,
    sheet_idx: i32,
    row: i32,
    col: i32,
    value: f64,
) -> i32 {
    if let Some(d) = OFFICE_DOCUMENTS.get(doc) {
        let mut d = d.lock().unwrap();
        if let Some(sheet) = d.sheets.get_mut(sheet_idx as usize) {
            let cell = SpreadsheetCell::number(row, col, value);
            sheet.add_cell(cell);
            return 1;
        }
    }
    0
}

/// Get cell value as string.
#[unsafe(no_mangle)]
pub extern "C" fn office_get_cell_string(
    _ctx: ContextHandle,
    doc: Handle,
    sheet_idx: i32,
    row: i32,
    col: i32,
) -> *mut c_char {
    if let Some(d) = OFFICE_DOCUMENTS.get(doc) {
        let d = d.lock().unwrap();
        if let Some(sheet) = d.sheets.get(sheet_idx as usize) {
            if let Some(cell) = sheet.get_cell(row, col) {
                let value = match cell.cell_type {
                    OFFICE_CELL_STRING => cell.string_value.clone().unwrap_or_default(),
                    OFFICE_CELL_NUMBER => cell.number_value.to_string(),
                    OFFICE_CELL_BOOLEAN => cell.bool_value.to_string(),
                    _ => String::new(),
                };
                if let Ok(cstr) = CString::new(value) {
                    return cstr.into_raw();
                }
            }
        }
    }
    ptr::null_mut()
}

// ============================================================================
// FFI Functions - PPTX Slides
// ============================================================================

/// Add slide to presentation.
#[unsafe(no_mangle)]
pub extern "C" fn office_add_slide(_ctx: ContextHandle, doc: Handle) -> i32 {
    if let Some(d) = OFFICE_DOCUMENTS.get(doc) {
        let mut d = d.lock().unwrap();
        return d.add_slide();
    }
    -1
}

/// Get slide count.
#[unsafe(no_mangle)]
pub extern "C" fn office_slide_count(_ctx: ContextHandle, doc: Handle) -> i32 {
    if let Some(d) = OFFICE_DOCUMENTS.get(doc) {
        let d = d.lock().unwrap();
        return d.slides.len() as i32;
    }
    0
}

/// Set slide title.
#[unsafe(no_mangle)]
pub extern "C" fn office_set_slide_title(
    _ctx: ContextHandle,
    doc: Handle,
    slide_num: i32,
    title: *const c_char,
) -> i32 {
    if title.is_null() {
        return 0;
    }

    if let Some(d) = OFFICE_DOCUMENTS.get(doc) {
        let mut d = d.lock().unwrap();
        if let Some(slide) = d.slides.get_mut((slide_num - 1) as usize) {
            let t = unsafe { CStr::from_ptr(title).to_string_lossy().to_string() };
            slide.title = Some(t);
            return 1;
        }
    }
    0
}

/// Get slide title.
#[unsafe(no_mangle)]
pub extern "C" fn office_get_slide_title(
    _ctx: ContextHandle,
    doc: Handle,
    slide_num: i32,
) -> *mut c_char {
    if let Some(d) = OFFICE_DOCUMENTS.get(doc) {
        let d = d.lock().unwrap();
        if let Some(slide) = d.slides.get((slide_num - 1) as usize) {
            if let Some(ref title) = slide.title {
                if let Ok(cstr) = CString::new(title.clone()) {
                    return cstr.into_raw();
                }
            }
        }
    }
    ptr::null_mut()
}

// ============================================================================
// FFI Functions - Utility
// ============================================================================

/// Free a string returned by office functions.
#[unsafe(no_mangle)]
pub extern "C" fn office_free_string(s: *mut c_char) {
    if !s.is_null() {
        unsafe {
            drop(CString::from_raw(s));
        }
    }
}

/// Get document type name.
#[unsafe(no_mangle)]
pub extern "C" fn office_type_name(_ctx: ContextHandle, doc_type: i32) -> *mut c_char {
    let name = match doc_type {
        OFFICE_TYPE_DOCX => "Microsoft Word (DOCX)",
        OFFICE_TYPE_XLSX => "Microsoft Excel (XLSX)",
        OFFICE_TYPE_PPTX => "Microsoft PowerPoint (PPTX)",
        OFFICE_TYPE_ODT => "OpenDocument Text (ODT)",
        OFFICE_TYPE_ODS => "OpenDocument Spreadsheet (ODS)",
        OFFICE_TYPE_ODP => "OpenDocument Presentation (ODP)",
        _ => "Unknown",
    };

    if let Ok(cstr) = CString::new(name) {
        return cstr.into_raw();
    }
    ptr::null_mut()
}

/// Get file extension for document type.
#[unsafe(no_mangle)]
pub extern "C" fn office_type_extension(_ctx: ContextHandle, doc_type: i32) -> *mut c_char {
    let ext = match doc_type {
        OFFICE_TYPE_DOCX => ".docx",
        OFFICE_TYPE_XLSX => ".xlsx",
        OFFICE_TYPE_PPTX => ".pptx",
        OFFICE_TYPE_ODT => ".odt",
        OFFICE_TYPE_ODS => ".ods",
        OFFICE_TYPE_ODP => ".odp",
        _ => "",
    };

    if let Ok(cstr) = CString::new(ext) {
        return cstr.into_raw();
    }
    ptr::null_mut()
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_constants() {
        assert_eq!(OFFICE_TYPE_DOCX, 0);
        assert_eq!(OFFICE_TYPE_XLSX, 1);
        assert_eq!(OFFICE_TYPE_PPTX, 2);
    }

    #[test]
    fn test_content_constants() {
        assert_eq!(OFFICE_CONTENT_PARAGRAPH, 0);
        assert_eq!(OFFICE_CONTENT_TABLE, 1);
    }

    #[test]
    fn test_cell_reference() {
        let cell = SpreadsheetCell::new(0, 0);
        assert_eq!(cell.reference(), "A1");

        let cell2 = SpreadsheetCell::new(4, 2);
        assert_eq!(cell2.reference(), "C5");

        let cell3 = SpreadsheetCell::new(0, 25);
        assert_eq!(cell3.reference(), "Z1");

        let cell4 = SpreadsheetCell::new(0, 26);
        assert_eq!(cell4.reference(), "AA1");
    }

    #[test]
    fn test_text_style() {
        let style = TextStyle::new();
        assert_eq!(style.font_size, 11.0);
        assert!(!style.bold);
    }

    #[test]
    fn test_paragraph_style() {
        let style = ParagraphStyle::new();
        assert_eq!(style.line_spacing, 1.0);
        assert_eq!(style.alignment, OFFICE_ALIGN_LEFT);
    }

    #[test]
    fn test_content_element() {
        let para = ContentElement::paragraph("Hello, World!");
        assert_eq!(para.content_type, OFFICE_CONTENT_PARAGRAPH);
        assert_eq!(para.text, Some("Hello, World!".to_string()));

        let heading = ContentElement::heading("Chapter 1", 1);
        assert_eq!(heading.content_type, OFFICE_CONTENT_HEADING);
        assert_eq!(heading.para_style.outline_level, 1);
    }

    #[test]
    fn test_spreadsheet_cell() {
        let cell = SpreadsheetCell::string(0, 0, "Hello");
        assert_eq!(cell.cell_type, OFFICE_CELL_STRING);
        assert_eq!(cell.string_value, Some("Hello".to_string()));

        let num_cell = SpreadsheetCell::number(1, 1, 42.5);
        assert_eq!(num_cell.cell_type, OFFICE_CELL_NUMBER);
        assert_eq!(num_cell.number_value, 42.5);
    }

    #[test]
    fn test_sheet() {
        let mut sheet = Sheet::new("Sheet1", 0);
        sheet.add_cell(SpreadsheetCell::string(0, 0, "A1"));
        sheet.add_cell(SpreadsheetCell::number(1, 2, 100.0));

        assert_eq!(sheet.row_count, 2);
        assert_eq!(sheet.col_count, 3);
        assert!(sheet.get_cell(0, 0).is_some());
    }

    #[test]
    fn test_office_document_docx() {
        let mut doc = OfficeDocument::docx(0);
        doc.add_paragraph("First paragraph");
        doc.add_heading("Introduction", 1);

        assert_eq!(doc.doc_type, OFFICE_TYPE_DOCX);
        assert_eq!(doc.content.len(), 2);
    }

    #[test]
    fn test_office_document_xlsx() {
        let mut doc = OfficeDocument::xlsx(0);
        let idx = doc.add_sheet("Sales");

        if let Some(sheet) = doc.sheets.get_mut(idx as usize) {
            sheet.add_cell(SpreadsheetCell::string(0, 0, "Product"));
            sheet.add_cell(SpreadsheetCell::number(1, 0, 100.0));
        }

        assert_eq!(doc.doc_type, OFFICE_TYPE_XLSX);
        assert_eq!(doc.sheets.len(), 1);
    }

    #[test]
    fn test_office_document_pptx() {
        let mut doc = OfficeDocument::pptx(0);
        doc.add_slide();

        if let Some(slide) = doc.slides.get_mut(0) {
            slide.title = Some("Welcome".to_string());
        }

        assert_eq!(doc.doc_type, OFFICE_TYPE_PPTX);
        assert_eq!(doc.slides.len(), 1);
    }

    #[test]
    fn test_ffi_docx() {
        let ctx = 0;
        let doc = office_new_docx(ctx);
        assert!(doc > 0);

        assert_eq!(office_get_type(ctx, doc), OFFICE_TYPE_DOCX);

        let text = CString::new("Hello, World!").unwrap();
        office_add_paragraph(ctx, doc, text.as_ptr());
        assert_eq!(office_content_count(ctx, doc), 1);

        office_drop_document(ctx, doc);
    }

    #[test]
    fn test_ffi_xlsx() {
        let ctx = 0;
        let doc = office_new_xlsx(ctx);

        let name = CString::new("Data").unwrap();
        let sheet_idx = office_add_sheet(ctx, doc, name.as_ptr());
        assert_eq!(sheet_idx, 0);
        assert_eq!(office_sheet_count(ctx, doc), 1);

        let value = CString::new("Hello").unwrap();
        office_set_cell_string(ctx, doc, 0, 0, 0, value.as_ptr());
        office_set_cell_number(ctx, doc, 0, 0, 1, 42.0);

        let result = office_get_cell_string(ctx, doc, 0, 0, 0);
        assert!(!result.is_null());
        unsafe {
            let s = CStr::from_ptr(result).to_string_lossy();
            assert_eq!(s, "Hello");
            office_free_string(result);
        }

        office_drop_document(ctx, doc);
    }

    #[test]
    fn test_ffi_pptx() {
        let ctx = 0;
        let doc = office_new_pptx(ctx);

        let slide_num = office_add_slide(ctx, doc);
        assert_eq!(slide_num, 1);
        assert_eq!(office_slide_count(ctx, doc), 1);

        let title = CString::new("Introduction").unwrap();
        office_set_slide_title(ctx, doc, 1, title.as_ptr());

        let result = office_get_slide_title(ctx, doc, 1);
        assert!(!result.is_null());
        unsafe {
            let s = CStr::from_ptr(result).to_string_lossy();
            assert_eq!(s, "Introduction");
            office_free_string(result);
        }

        office_drop_document(ctx, doc);
    }

    #[test]
    fn test_ffi_metadata() {
        let ctx = 0;
        let doc = office_new_docx(ctx);

        let title = CString::new("My Document").unwrap();
        let creator = CString::new("John Doe").unwrap();

        office_set_title(ctx, doc, title.as_ptr());
        office_set_creator(ctx, doc, creator.as_ptr());

        let t = office_get_title(ctx, doc);
        unsafe {
            let s = CStr::from_ptr(t).to_string_lossy();
            assert_eq!(s, "My Document");
            office_free_string(t);
        }

        office_drop_document(ctx, doc);
    }

    #[test]
    fn test_ffi_type_name() {
        let ctx = 0;

        let name = office_type_name(ctx, OFFICE_TYPE_DOCX);
        unsafe {
            let s = CStr::from_ptr(name).to_string_lossy();
            assert!(s.contains("Word"));
            office_free_string(name);
        }

        let ext = office_type_extension(ctx, OFFICE_TYPE_XLSX);
        unsafe {
            let s = CStr::from_ptr(ext).to_string_lossy();
            assert_eq!(s, ".xlsx");
            office_free_string(ext);
        }
    }
}
