//! FFI bindings for fz_document_writer (Document Writers)
//!
//! This module provides functions for writing documents in various formats
//! including PDF, SVG, text, Office formats, and image formats.

use super::ffi_safety::{cstr_to_str, cstr_to_string};
use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::sync::{LazyLock, Mutex};

use crate::ffi::stext::Rect;
use crate::ffi::{Handle, HandleStore};

/// Global store for document writers
pub static WRITERS: LazyLock<HandleStore<DocumentWriter>> = LazyLock::new(HandleStore::new);

/// Global store for writer devices (borrowed devices from begin_page)
pub static WRITER_DEVICES: LazyLock<HandleStore<WriterDevice>> = LazyLock::new(HandleStore::new);

/// Document writer format types
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriterFormat {
    Pdf = 0,
    Svg = 1,
    Text = 2,
    Html = 3,
    Xhtml = 4,
    Odt = 5,
    Docx = 6,
    Ps = 7,
    Pcl = 8,
    Pclm = 9,
    Pwg = 10,
    Cbz = 11,
    Csv = 12,
    Pdfocr = 13,
    Png = 14,
    Jpeg = 15,
    Pam = 16,
    Pnm = 17,
    Pgm = 18,
    Ppm = 19,
    Pbm = 20,
    Pkm = 21,
}

impl WriterFormat {
    /// Parse format from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "pdf" => Some(WriterFormat::Pdf),
            "svg" => Some(WriterFormat::Svg),
            "text" | "txt" => Some(WriterFormat::Text),
            "html" => Some(WriterFormat::Html),
            "xhtml" => Some(WriterFormat::Xhtml),
            "odt" => Some(WriterFormat::Odt),
            "docx" => Some(WriterFormat::Docx),
            "ps" | "postscript" => Some(WriterFormat::Ps),
            "pcl" => Some(WriterFormat::Pcl),
            "pclm" => Some(WriterFormat::Pclm),
            "pwg" => Some(WriterFormat::Pwg),
            "cbz" => Some(WriterFormat::Cbz),
            "csv" => Some(WriterFormat::Csv),
            "pdfocr" | "ocr" => Some(WriterFormat::Pdfocr),
            "png" => Some(WriterFormat::Png),
            "jpeg" | "jpg" => Some(WriterFormat::Jpeg),
            "pam" => Some(WriterFormat::Pam),
            "pnm" => Some(WriterFormat::Pnm),
            "pgm" => Some(WriterFormat::Pgm),
            "ppm" => Some(WriterFormat::Ppm),
            "pbm" => Some(WriterFormat::Pbm),
            "pkm" => Some(WriterFormat::Pkm),
            _ => None,
        }
    }

    /// Get file extension for format
    pub fn extension(&self) -> &'static str {
        match self {
            WriterFormat::Pdf => "pdf",
            WriterFormat::Svg => "svg",
            WriterFormat::Text => "txt",
            WriterFormat::Html => "html",
            WriterFormat::Xhtml => "xhtml",
            WriterFormat::Odt => "odt",
            WriterFormat::Docx => "docx",
            WriterFormat::Ps => "ps",
            WriterFormat::Pcl => "pcl",
            WriterFormat::Pclm => "pclm",
            WriterFormat::Pwg => "pwg",
            WriterFormat::Cbz => "cbz",
            WriterFormat::Csv => "csv",
            WriterFormat::Pdfocr => "pdf",
            WriterFormat::Png => "png",
            WriterFormat::Jpeg => "jpg",
            WriterFormat::Pam => "pam",
            WriterFormat::Pnm => "pnm",
            WriterFormat::Pgm => "pgm",
            WriterFormat::Ppm => "ppm",
            WriterFormat::Pbm => "pbm",
            WriterFormat::Pkm => "pkm",
        }
    }

    /// Check if format is multi-page
    pub fn is_multipage(&self) -> bool {
        matches!(
            self,
            WriterFormat::Pdf
                | WriterFormat::Svg
                | WriterFormat::Text
                | WriterFormat::Html
                | WriterFormat::Xhtml
                | WriterFormat::Odt
                | WriterFormat::Docx
                | WriterFormat::Ps
                | WriterFormat::Pcl
                | WriterFormat::Pclm
                | WriterFormat::Pwg
                | WriterFormat::Cbz
                | WriterFormat::Csv
                | WriterFormat::Pdfocr
        )
    }

    /// Check if format is image-based
    pub fn is_image(&self) -> bool {
        matches!(
            self,
            WriterFormat::Png
                | WriterFormat::Jpeg
                | WriterFormat::Pam
                | WriterFormat::Pnm
                | WriterFormat::Pgm
                | WriterFormat::Ppm
                | WriterFormat::Pbm
                | WriterFormat::Pkm
        )
    }
}

/// Writer state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriterState {
    Created,
    PageInProgress,
    Closed,
}

/// Document writer options
#[derive(Debug, Clone, Default)]
pub struct WriterOptions {
    /// Raw options string
    pub raw: String,
    /// Parsed key-value pairs
    pub options: HashMap<String, String>,
    /// Resolution (for image output)
    pub resolution: Option<f32>,
    /// Compression level
    pub compression: Option<i32>,
    /// Quality (for JPEG)
    pub quality: Option<i32>,
    /// Color mode
    pub colorspace: Option<String>,
    /// Page size template
    pub page_size: Option<String>,
    /// Linearize PDF
    pub linearize: bool,
    /// Encrypt PDF
    pub encrypt: bool,
    /// OCR language
    pub ocr_language: Option<String>,
}

impl WriterOptions {
    /// Parse options from string
    pub fn parse(opts_str: &str) -> Self {
        let mut options = WriterOptions {
            raw: opts_str.to_string(),
            ..Default::default()
        };

        for part in opts_str.split(',') {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }

            if let Some((key, value)) = part.split_once('=') {
                let key = key.trim().to_lowercase();
                let value = value.trim();
                options.options.insert(key.clone(), value.to_string());

                match key.as_str() {
                    "resolution" | "res" | "dpi" => {
                        options.resolution = value.parse().ok();
                    }
                    "compression" => {
                        options.compression = value.parse().ok();
                    }
                    "quality" => {
                        options.quality = value.parse().ok();
                    }
                    "colorspace" | "cs" => {
                        options.colorspace = Some(value.to_string());
                    }
                    "pagesize" | "page-size" => {
                        options.page_size = Some(value.to_string());
                    }
                    "language" | "lang" | "ocr-language" => {
                        options.ocr_language = Some(value.to_string());
                    }
                    _ => {}
                }
            } else {
                // Boolean flag
                match part.to_lowercase().as_str() {
                    "linearize" | "linear" => options.linearize = true,
                    "encrypt" | "encrypted" => options.encrypt = true,
                    _ => {
                        options.options.insert(part.to_string(), String::new());
                    }
                }
            }
        }

        options
    }

    /// Get option value
    pub fn get(&self, key: &str) -> Option<&String> {
        self.options.get(key)
    }

    /// Check if option exists
    pub fn has(&self, key: &str) -> bool {
        self.options.contains_key(key)
    }
}

/// Page data for writer
#[derive(Debug, Clone)]
pub struct WriterPage {
    pub index: i32,
    pub mediabox: Rect,
    pub content: Vec<u8>,
}

/// Document writer
pub struct DocumentWriter {
    pub format: WriterFormat,
    pub path: Option<String>,
    pub output: Option<Handle>, // Handle to output stream
    pub buffer: Option<Handle>, // Handle to buffer
    pub options: WriterOptions,
    pub state: WriterState,
    pub pages: Vec<WriterPage>,
    pub current_page: Option<WriterPage>,
    pub current_device: Option<Handle>,
    pub page_count: i32,
}

impl DocumentWriter {
    /// Create a new document writer
    pub fn new(format: WriterFormat, path: Option<String>, options: WriterOptions) -> Self {
        Self {
            format,
            path,
            output: None,
            buffer: None,
            options,
            state: WriterState::Created,
            pages: Vec::new(),
            current_page: None,
            current_device: None,
            page_count: 0,
        }
    }

    /// Create writer with output handle
    pub fn with_output(format: WriterFormat, output: Handle, options: WriterOptions) -> Self {
        let mut writer = Self::new(format, None, options);
        writer.output = Some(output);
        writer
    }

    /// Create writer with buffer handle
    pub fn with_buffer(format: WriterFormat, buffer: Handle, options: WriterOptions) -> Self {
        let mut writer = Self::new(format, None, options);
        writer.buffer = Some(buffer);
        writer
    }

    /// Begin a new page
    pub fn begin_page(&mut self, mediabox: Rect) -> bool {
        if self.state == WriterState::Closed {
            return false;
        }

        // End current page if one is in progress
        if self.state == WriterState::PageInProgress {
            self.end_page();
        }

        self.current_page = Some(WriterPage {
            index: self.page_count,
            mediabox,
            content: Vec::new(),
        });
        self.state = WriterState::PageInProgress;
        true
    }

    /// End current page
    pub fn end_page(&mut self) -> bool {
        if self.state != WriterState::PageInProgress {
            return false;
        }

        if let Some(page) = self.current_page.take() {
            self.pages.push(page);
            self.page_count += 1;
        }

        self.current_device = None;
        self.state = WriterState::Created;
        true
    }

    /// Close the writer
    pub fn close(&mut self) -> bool {
        if self.state == WriterState::Closed {
            return false;
        }

        // End current page if in progress
        if self.state == WriterState::PageInProgress {
            self.end_page();
        }

        // Write output based on format
        self.write_output();

        self.state = WriterState::Closed;
        true
    }

    /// Write output (format-specific)
    fn write_output(&self) {
        match self.format {
            WriterFormat::Pdf => self.write_pdf(),
            WriterFormat::Svg => self.write_svg(),
            WriterFormat::Text => self.write_text(),
            WriterFormat::Html | WriterFormat::Xhtml => self.write_html(),
            WriterFormat::Cbz => self.write_cbz(),
            WriterFormat::Png
            | WriterFormat::Jpeg
            | WriterFormat::Pam
            | WriterFormat::Pnm
            | WriterFormat::Pgm
            | WriterFormat::Ppm
            | WriterFormat::Pbm
            | WriterFormat::Pkm => self.write_image(),
            _ => {
                // Other formats use generic write
            }
        }
    }

    /// Write PDF output
    fn write_pdf(&self) {
        // Generate PDF structure
        let mut pdf_data = Vec::new();

        // PDF header
        pdf_data.extend_from_slice(b"%PDF-1.7\n");
        pdf_data.extend_from_slice(b"%\xE2\xE3\xCF\xD3\n");

        let mut obj_offsets = Vec::new();
        let mut obj_num = 1;

        // Catalog (object 1)
        obj_offsets.push(pdf_data.len());
        let catalog = format!(
            "{} 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n",
            obj_num
        );
        pdf_data.extend_from_slice(catalog.as_bytes());
        obj_num += 1;

        // Pages (object 2)
        obj_offsets.push(pdf_data.len());
        let page_refs: String = (0..self.pages.len())
            .map(|i| format!("{} 0 R", 3 + i))
            .collect::<Vec<_>>()
            .join(" ");
        let pages = format!(
            "{} 0 obj\n<< /Type /Pages /Kids [ {} ] /Count {} >>\nendobj\n",
            obj_num,
            page_refs,
            self.pages.len()
        );
        pdf_data.extend_from_slice(pages.as_bytes());
        obj_num += 1;

        // Page objects
        for page in &self.pages {
            obj_offsets.push(pdf_data.len());
            let page_obj = format!(
                "{} 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [ {} {} {} {} ] >>\nendobj\n",
                obj_num, page.mediabox.x0, page.mediabox.y0, page.mediabox.x1, page.mediabox.y1
            );
            pdf_data.extend_from_slice(page_obj.as_bytes());
            obj_num += 1;
        }

        // Xref table
        let xref_offset = pdf_data.len();
        pdf_data.extend_from_slice(b"xref\n");
        pdf_data.extend_from_slice(format!("0 {}\n", obj_num).as_bytes());
        pdf_data.extend_from_slice(b"0000000000 65535 f \n");
        for offset in &obj_offsets {
            pdf_data.extend_from_slice(format!("{:010} 00000 n \n", offset).as_bytes());
        }

        // Trailer
        pdf_data.extend_from_slice(b"trailer\n");
        pdf_data.extend_from_slice(format!("<< /Size {} /Root 1 0 R >>\n", obj_num).as_bytes());
        pdf_data.extend_from_slice(b"startxref\n");
        pdf_data.extend_from_slice(format!("{}\n", xref_offset).as_bytes());
        pdf_data.extend_from_slice(b"%%EOF\n");

        // Write to file if path specified
        if let Some(ref path) = self.path {
            let _ = std::fs::write(path, &pdf_data);
        }
    }

    /// Write SVG output
    fn write_svg(&self) {
        for (i, page) in self.pages.iter().enumerate() {
            let width = page.mediabox.x1 - page.mediabox.x0;
            let height = page.mediabox.y1 - page.mediabox.y0;

            let svg = format!(
                r#"<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}" viewBox="{} {} {} {}">
  <!-- Page {} -->
</svg>
"#,
                width,
                height,
                page.mediabox.x0,
                page.mediabox.y0,
                width,
                height,
                i + 1
            );

            if let Some(ref path) = self.path {
                let page_path = if self.pages.len() > 1 {
                    format!("{}_{}.svg", path.trim_end_matches(".svg"), i + 1)
                } else {
                    path.clone()
                };
                let _ = std::fs::write(page_path, svg.as_bytes());
            }
        }
    }

    /// Write text output
    fn write_text(&self) {
        let mut text = String::new();
        for (i, _page) in self.pages.iter().enumerate() {
            if i > 0 {
                text.push_str("\n\n--- Page Break ---\n\n");
            }
            text.push_str(&format!("--- Page {} ---\n", i + 1));
            // Page content would be added here
        }

        if let Some(ref path) = self.path {
            let _ = std::fs::write(path, text.as_bytes());
        }
    }

    /// Write HTML output
    fn write_html(&self) {
        let doctype = if self.format == WriterFormat::Xhtml {
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE html PUBLIC "-//W3C//DTD XHTML 1.0 Strict//EN" "http://www.w3.org/TR/xhtml1/DTD/xhtml1-strict.dtd">
"#
        } else {
            "<!DOCTYPE html>\n"
        };

        let mut html = String::from(doctype);
        html.push_str(
            "<html>\n<head>\n<meta charset=\"UTF-8\">\n<title>Document</title>\n</head>\n<body>\n",
        );

        for (i, _page) in self.pages.iter().enumerate() {
            html.push_str(&format!("<div class=\"page\" id=\"page-{}\">\n", i + 1));
            html.push_str("</div>\n");
        }

        html.push_str("</body>\n</html>\n");

        if let Some(ref path) = self.path {
            let _ = std::fs::write(path, html.as_bytes());
        }
    }

    /// Write CBZ (Comic Book ZIP) output
    fn write_cbz(&self) {
        // CBZ is just a ZIP file with images
        // For now, create a minimal structure
        if let Some(ref _path) = self.path {
            // Would use zip crate to create actual CBZ
        }
    }

    /// Write image output
    fn write_image(&self) {
        // Image output writes one file per page
        for (i, _page) in self.pages.iter().enumerate() {
            if let Some(ref path) = self.path {
                let ext = self.format.extension();
                let page_path = if self.pages.len() > 1 {
                    let base = path.trim_end_matches(&format!(".{}", ext));
                    format!("{}_{}.{}", base, i + 1, ext)
                } else {
                    path.clone()
                };

                // Would render page to image and save
                // For now, create placeholder
                let _ = std::fs::write(page_path, b"");
            }
        }
    }
}

/// Writer device (borrowed from begin_page)
pub struct WriterDevice {
    pub writer: Handle,
    pub page_index: i32,
}

// ============================================================================
// FFI Functions - Option Utilities
// ============================================================================

/// Check if options string has a specific key
#[unsafe(no_mangle)]
pub extern "C" fn fz_has_option(
    _ctx: Handle,
    opts: *const c_char,
    key: *const c_char,
    val: *mut *const c_char,
) -> i32 {
    if opts.is_null() || key.is_null() {
        return 0;
    }

    let opts_str = unsafe {
        match CStr::from_ptr(opts).to_str() {
            Ok(s) => s,
            Err(_) => return 0,
        }
    };

    let key_str = unsafe {
        match CStr::from_ptr(key).to_str() {
            Ok(s) => s,
            Err(_) => return 0,
        }
    };

    let options = WriterOptions::parse(opts_str);

    if let Some(value) = options.get(key_str) {
        if !val.is_null() {
            // Store the value pointer (caller must not free)
            if let Ok(cstr) = CString::new(value.as_str()) {
                unsafe {
                    *val = cstr.into_raw();
                }
            }
        }
        1
    } else {
        0
    }
}

/// Check if option 'a' matches reference option 'b'
#[unsafe(no_mangle)]
pub extern "C" fn fz_option_eq(a: *const c_char, b: *const c_char) -> i32 {
    if a.is_null() || b.is_null() {
        return 0;
    }

    let a_str = unsafe {
        match CStr::from_ptr(a).to_str() {
            Ok(s) => s,
            Err(_) => return 0,
        }
    };

    let b_str = unsafe {
        match CStr::from_ptr(b).to_str() {
            Ok(s) => s,
            Err(_) => return 0,
        }
    };

    // a could be "foo" or "foo,bar..." but b must be exactly "foo"
    let a_first = a_str.split(',').next().unwrap_or("");
    if a_first == b_str { 1 } else { 0 }
}

/// Copy an option value to destination buffer
#[unsafe(no_mangle)]
pub extern "C" fn fz_copy_option(
    _ctx: Handle,
    val: *const c_char,
    dest: *mut c_char,
    maxlen: usize,
) -> usize {
    if val.is_null() || dest.is_null() || maxlen == 0 {
        return 0;
    }

    let val_str = unsafe {
        match CStr::from_ptr(val).to_str() {
            Ok(s) => s,
            Err(_) => return 0,
        }
    };

    // Find end of value (comma or end of string)
    let value_end = val_str.find(',').unwrap_or(val_str.len());
    let value = &val_str[..value_end];

    let bytes = value.as_bytes();
    let copy_len = bytes.len().min(maxlen - 1);

    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), dest as *mut u8, copy_len);
        *dest.add(copy_len) = 0; // Null terminator
    }

    // Return bytes that didn't fit
    if bytes.len() >= maxlen {
        bytes.len() + 1 - maxlen
    } else {
        0
    }
}

// ============================================================================
// FFI Functions - Document Writer Creation
// ============================================================================

/// Create a new document writer by format
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_document_writer(
    _ctx: Handle,
    path: *const c_char,
    format: *const c_char,
    options: *const c_char,
) -> Handle {
    let path_str = if path.is_null() {
        None
    } else {
        unsafe { CStr::from_ptr(path).to_str().ok().map(String::from) }
    };

    let format_str = if format.is_null() {
        // Try to infer from path extension
        path_str
            .as_ref()
            .and_then(|p| p.rsplit('.').next())
            .unwrap_or("pdf")
    } else {
        unsafe { CStr::from_ptr(format).to_str().unwrap_or("pdf") }
    };

    let writer_format = WriterFormat::from_str(format_str).unwrap_or(WriterFormat::Pdf);

    let opts = if options.is_null() {
        WriterOptions::default()
    } else {
        let opts_str = unsafe { CStr::from_ptr(options).to_str().unwrap_or("") };
        WriterOptions::parse(opts_str)
    };

    let writer = DocumentWriter::new(writer_format, path_str, opts);
    WRITERS.insert(writer)
}

/// Create document writer with output handle
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_document_writer_with_output(
    _ctx: Handle,
    out: Handle,
    format: *const c_char,
    options: *const c_char,
) -> Handle {
    let format_str = if format.is_null() {
        "pdf"
    } else {
        unsafe { CStr::from_ptr(format).to_str().unwrap_or("pdf") }
    };

    let writer_format = WriterFormat::from_str(format_str).unwrap_or(WriterFormat::Pdf);

    let opts = if options.is_null() {
        WriterOptions::default()
    } else {
        let opts_str = unsafe { CStr::from_ptr(options).to_str().unwrap_or("") };
        WriterOptions::parse(opts_str)
    };

    let writer = DocumentWriter::with_output(writer_format, out, opts);
    WRITERS.insert(writer)
}

/// Create document writer with buffer handle
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_document_writer_with_buffer(
    _ctx: Handle,
    buf: Handle,
    format: *const c_char,
    options: *const c_char,
) -> Handle {
    let format_str = if format.is_null() {
        "pdf"
    } else {
        unsafe { CStr::from_ptr(format).to_str().unwrap_or("pdf") }
    };

    let writer_format = WriterFormat::from_str(format_str).unwrap_or(WriterFormat::Pdf);

    let opts = if options.is_null() {
        WriterOptions::default()
    } else {
        let opts_str = unsafe { CStr::from_ptr(options).to_str().unwrap_or("") };
        WriterOptions::parse(opts_str)
    };

    let writer = DocumentWriter::with_buffer(writer_format, buf, opts);
    WRITERS.insert(writer)
}

// ============================================================================
// FFI Functions - Format-Specific Writers
// ============================================================================

/// Create PDF writer
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_pdf_writer(
    _ctx: Handle,
    path: *const c_char,
    options: *const c_char,
) -> Handle {
    let path_str = if path.is_null() {
        None
    } else {
        unsafe { CStr::from_ptr(path).to_str().ok().map(String::from) }
    };

    let opts = if options.is_null() {
        WriterOptions::default()
    } else {
        let opts_str = unsafe { CStr::from_ptr(options).to_str().unwrap_or("") };
        WriterOptions::parse(opts_str)
    };

    let writer = DocumentWriter::new(WriterFormat::Pdf, path_str, opts);
    WRITERS.insert(writer)
}

/// Create PDF writer with output
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_pdf_writer_with_output(
    _ctx: Handle,
    out: Handle,
    options: *const c_char,
) -> Handle {
    let opts = if options.is_null() {
        WriterOptions::default()
    } else {
        let opts_str = unsafe { CStr::from_ptr(options).to_str().unwrap_or("") };
        WriterOptions::parse(opts_str)
    };

    let writer = DocumentWriter::with_output(WriterFormat::Pdf, out, opts);
    WRITERS.insert(writer)
}

/// Create SVG writer
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_svg_writer(
    _ctx: Handle,
    path: *const c_char,
    options: *const c_char,
) -> Handle {
    let path_str = if path.is_null() {
        None
    } else {
        unsafe { CStr::from_ptr(path).to_str().ok().map(String::from) }
    };

    let opts = if options.is_null() {
        WriterOptions::default()
    } else {
        let opts_str = unsafe { CStr::from_ptr(options).to_str().unwrap_or("") };
        WriterOptions::parse(opts_str)
    };

    let writer = DocumentWriter::new(WriterFormat::Svg, path_str, opts);
    WRITERS.insert(writer)
}

/// Create SVG writer with output
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_svg_writer_with_output(
    _ctx: Handle,
    out: Handle,
    options: *const c_char,
) -> Handle {
    let opts = if options.is_null() {
        WriterOptions::default()
    } else {
        let opts_str = unsafe { CStr::from_ptr(options).to_str().unwrap_or("") };
        WriterOptions::parse(opts_str)
    };

    let writer = DocumentWriter::with_output(WriterFormat::Svg, out, opts);
    WRITERS.insert(writer)
}

/// Create text writer
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_text_writer(
    _ctx: Handle,
    format: *const c_char,
    path: *const c_char,
    options: *const c_char,
) -> Handle {
    let path_str = if path.is_null() {
        None
    } else {
        unsafe { CStr::from_ptr(path).to_str().ok().map(String::from) }
    };

    // Format can be "text", "html", "xhtml"
    let format_str = if format.is_null() {
        "text"
    } else {
        unsafe { CStr::from_ptr(format).to_str().unwrap_or("text") }
    };

    let writer_format = match format_str {
        "html" => WriterFormat::Html,
        "xhtml" => WriterFormat::Xhtml,
        _ => WriterFormat::Text,
    };

    let opts = if options.is_null() {
        WriterOptions::default()
    } else {
        let opts_str = unsafe { CStr::from_ptr(options).to_str().unwrap_or("") };
        WriterOptions::parse(opts_str)
    };

    let writer = DocumentWriter::new(writer_format, path_str, opts);
    WRITERS.insert(writer)
}

/// Create text writer with output
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_text_writer_with_output(
    _ctx: Handle,
    format: *const c_char,
    out: Handle,
    options: *const c_char,
) -> Handle {
    let format_str = if format.is_null() {
        "text"
    } else {
        unsafe { CStr::from_ptr(format).to_str().unwrap_or("text") }
    };

    let writer_format = match format_str {
        "html" => WriterFormat::Html,
        "xhtml" => WriterFormat::Xhtml,
        _ => WriterFormat::Text,
    };

    let opts = if options.is_null() {
        WriterOptions::default()
    } else {
        let opts_str = unsafe { CStr::from_ptr(options).to_str().unwrap_or("") };
        WriterOptions::parse(opts_str)
    };

    let writer = DocumentWriter::with_output(writer_format, out, opts);
    WRITERS.insert(writer)
}

/// Create ODT writer
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_odt_writer(
    _ctx: Handle,
    path: *const c_char,
    options: *const c_char,
) -> Handle {
    let path_str = if path.is_null() {
        None
    } else {
        unsafe { CStr::from_ptr(path).to_str().ok().map(String::from) }
    };

    let opts = if options.is_null() {
        WriterOptions::default()
    } else {
        let opts_str = unsafe { CStr::from_ptr(options).to_str().unwrap_or("") };
        WriterOptions::parse(opts_str)
    };

    let writer = DocumentWriter::new(WriterFormat::Odt, path_str, opts);
    WRITERS.insert(writer)
}

/// Create ODT writer with output
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_odt_writer_with_output(
    _ctx: Handle,
    out: Handle,
    options: *const c_char,
) -> Handle {
    let opts = if options.is_null() {
        WriterOptions::default()
    } else {
        let opts_str = unsafe { CStr::from_ptr(options).to_str().unwrap_or("") };
        WriterOptions::parse(opts_str)
    };

    let writer = DocumentWriter::with_output(WriterFormat::Odt, out, opts);
    WRITERS.insert(writer)
}

/// Create DOCX writer
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_docx_writer(
    _ctx: Handle,
    path: *const c_char,
    options: *const c_char,
) -> Handle {
    let path_str = if path.is_null() {
        None
    } else {
        unsafe { CStr::from_ptr(path).to_str().ok().map(String::from) }
    };

    let opts = if options.is_null() {
        WriterOptions::default()
    } else {
        let opts_str = unsafe { CStr::from_ptr(options).to_str().unwrap_or("") };
        WriterOptions::parse(opts_str)
    };

    let writer = DocumentWriter::new(WriterFormat::Docx, path_str, opts);
    WRITERS.insert(writer)
}

/// Create DOCX writer with output
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_docx_writer_with_output(
    _ctx: Handle,
    out: Handle,
    options: *const c_char,
) -> Handle {
    let opts = if options.is_null() {
        WriterOptions::default()
    } else {
        let opts_str = unsafe { CStr::from_ptr(options).to_str().unwrap_or("") };
        WriterOptions::parse(opts_str)
    };

    let writer = DocumentWriter::with_output(WriterFormat::Docx, out, opts);
    WRITERS.insert(writer)
}

/// Create PS writer
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_ps_writer(
    _ctx: Handle,
    path: *const c_char,
    options: *const c_char,
) -> Handle {
    let path_str = if path.is_null() {
        None
    } else {
        unsafe { CStr::from_ptr(path).to_str().ok().map(String::from) }
    };

    let opts = if options.is_null() {
        WriterOptions::default()
    } else {
        let opts_str = unsafe { CStr::from_ptr(options).to_str().unwrap_or("") };
        WriterOptions::parse(opts_str)
    };

    let writer = DocumentWriter::new(WriterFormat::Ps, path_str, opts);
    WRITERS.insert(writer)
}

/// Create PS writer with output
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_ps_writer_with_output(
    _ctx: Handle,
    out: Handle,
    options: *const c_char,
) -> Handle {
    let opts = if options.is_null() {
        WriterOptions::default()
    } else {
        let opts_str = unsafe { CStr::from_ptr(options).to_str().unwrap_or("") };
        WriterOptions::parse(opts_str)
    };

    let writer = DocumentWriter::with_output(WriterFormat::Ps, out, opts);
    WRITERS.insert(writer)
}

/// Create PCL writer
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_pcl_writer(
    _ctx: Handle,
    path: *const c_char,
    options: *const c_char,
) -> Handle {
    let path_str = if path.is_null() {
        None
    } else {
        unsafe { CStr::from_ptr(path).to_str().ok().map(String::from) }
    };

    let opts = if options.is_null() {
        WriterOptions::default()
    } else {
        let opts_str = unsafe { CStr::from_ptr(options).to_str().unwrap_or("") };
        WriterOptions::parse(opts_str)
    };

    let writer = DocumentWriter::new(WriterFormat::Pcl, path_str, opts);
    WRITERS.insert(writer)
}

/// Create PCL writer with output
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_pcl_writer_with_output(
    _ctx: Handle,
    out: Handle,
    options: *const c_char,
) -> Handle {
    let opts = if options.is_null() {
        WriterOptions::default()
    } else {
        let opts_str = unsafe { CStr::from_ptr(options).to_str().unwrap_or("") };
        WriterOptions::parse(opts_str)
    };

    let writer = DocumentWriter::with_output(WriterFormat::Pcl, out, opts);
    WRITERS.insert(writer)
}

/// Create PCLM writer
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_pclm_writer(
    _ctx: Handle,
    path: *const c_char,
    options: *const c_char,
) -> Handle {
    let path_str = if path.is_null() {
        None
    } else {
        unsafe { CStr::from_ptr(path).to_str().ok().map(String::from) }
    };

    let opts = if options.is_null() {
        WriterOptions::default()
    } else {
        let opts_str = unsafe { CStr::from_ptr(options).to_str().unwrap_or("") };
        WriterOptions::parse(opts_str)
    };

    let writer = DocumentWriter::new(WriterFormat::Pclm, path_str, opts);
    WRITERS.insert(writer)
}

/// Create PCLM writer with output
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_pclm_writer_with_output(
    _ctx: Handle,
    out: Handle,
    options: *const c_char,
) -> Handle {
    let opts = if options.is_null() {
        WriterOptions::default()
    } else {
        let opts_str = unsafe { CStr::from_ptr(options).to_str().unwrap_or("") };
        WriterOptions::parse(opts_str)
    };

    let writer = DocumentWriter::with_output(WriterFormat::Pclm, out, opts);
    WRITERS.insert(writer)
}

/// Create PWG writer
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_pwg_writer(
    _ctx: Handle,
    path: *const c_char,
    options: *const c_char,
) -> Handle {
    let path_str = if path.is_null() {
        None
    } else {
        unsafe { CStr::from_ptr(path).to_str().ok().map(String::from) }
    };

    let opts = if options.is_null() {
        WriterOptions::default()
    } else {
        let opts_str = unsafe { CStr::from_ptr(options).to_str().unwrap_or("") };
        WriterOptions::parse(opts_str)
    };

    let writer = DocumentWriter::new(WriterFormat::Pwg, path_str, opts);
    WRITERS.insert(writer)
}

/// Create PWG writer with output
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_pwg_writer_with_output(
    _ctx: Handle,
    out: Handle,
    options: *const c_char,
) -> Handle {
    let opts = if options.is_null() {
        WriterOptions::default()
    } else {
        let opts_str = unsafe { CStr::from_ptr(options).to_str().unwrap_or("") };
        WriterOptions::parse(opts_str)
    };

    let writer = DocumentWriter::with_output(WriterFormat::Pwg, out, opts);
    WRITERS.insert(writer)
}

/// Create CBZ writer
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_cbz_writer(
    _ctx: Handle,
    path: *const c_char,
    options: *const c_char,
) -> Handle {
    let path_str = if path.is_null() {
        None
    } else {
        unsafe { CStr::from_ptr(path).to_str().ok().map(String::from) }
    };

    let opts = if options.is_null() {
        WriterOptions::default()
    } else {
        let opts_str = unsafe { CStr::from_ptr(options).to_str().unwrap_or("") };
        WriterOptions::parse(opts_str)
    };

    let writer = DocumentWriter::new(WriterFormat::Cbz, path_str, opts);
    WRITERS.insert(writer)
}

/// Create CBZ writer with output
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_cbz_writer_with_output(
    _ctx: Handle,
    out: Handle,
    options: *const c_char,
) -> Handle {
    let opts = if options.is_null() {
        WriterOptions::default()
    } else {
        let opts_str = unsafe { CStr::from_ptr(options).to_str().unwrap_or("") };
        WriterOptions::parse(opts_str)
    };

    let writer = DocumentWriter::with_output(WriterFormat::Cbz, out, opts);
    WRITERS.insert(writer)
}

/// Create CSV writer
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_csv_writer(
    _ctx: Handle,
    path: *const c_char,
    options: *const c_char,
) -> Handle {
    let path_str = if path.is_null() {
        None
    } else {
        unsafe { CStr::from_ptr(path).to_str().ok().map(String::from) }
    };

    let opts = if options.is_null() {
        WriterOptions::default()
    } else {
        let opts_str = unsafe { CStr::from_ptr(options).to_str().unwrap_or("") };
        WriterOptions::parse(opts_str)
    };

    let writer = DocumentWriter::new(WriterFormat::Csv, path_str, opts);
    WRITERS.insert(writer)
}

/// Create CSV writer with output
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_csv_writer_with_output(
    _ctx: Handle,
    out: Handle,
    options: *const c_char,
) -> Handle {
    let opts = if options.is_null() {
        WriterOptions::default()
    } else {
        let opts_str = unsafe { CStr::from_ptr(options).to_str().unwrap_or("") };
        WriterOptions::parse(opts_str)
    };

    let writer = DocumentWriter::with_output(WriterFormat::Csv, out, opts);
    WRITERS.insert(writer)
}

/// Create PDF OCR writer
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_pdfocr_writer(
    _ctx: Handle,
    path: *const c_char,
    options: *const c_char,
) -> Handle {
    let path_str = if path.is_null() {
        None
    } else {
        unsafe { CStr::from_ptr(path).to_str().ok().map(String::from) }
    };

    let opts = if options.is_null() {
        WriterOptions::default()
    } else {
        let opts_str = unsafe { CStr::from_ptr(options).to_str().unwrap_or("") };
        WriterOptions::parse(opts_str)
    };

    let writer = DocumentWriter::new(WriterFormat::Pdfocr, path_str, opts);
    WRITERS.insert(writer)
}

/// Create PDF OCR writer with output
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_pdfocr_writer_with_output(
    _ctx: Handle,
    out: Handle,
    options: *const c_char,
) -> Handle {
    let opts = if options.is_null() {
        WriterOptions::default()
    } else {
        let opts_str = unsafe { CStr::from_ptr(options).to_str().unwrap_or("") };
        WriterOptions::parse(opts_str)
    };

    let writer = DocumentWriter::with_output(WriterFormat::Pdfocr, out, opts);
    WRITERS.insert(writer)
}

// ============================================================================
// FFI Functions - Pixmap Writers
// ============================================================================

/// Create PNG pixmap writer
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_png_pixmap_writer(
    _ctx: Handle,
    path: *const c_char,
    options: *const c_char,
) -> Handle {
    let path_str = if path.is_null() {
        None
    } else {
        unsafe { CStr::from_ptr(path).to_str().ok().map(String::from) }
    };

    let opts = if options.is_null() {
        WriterOptions::default()
    } else {
        let opts_str = unsafe { CStr::from_ptr(options).to_str().unwrap_or("") };
        WriterOptions::parse(opts_str)
    };

    let writer = DocumentWriter::new(WriterFormat::Png, path_str, opts);
    WRITERS.insert(writer)
}

/// Create JPEG pixmap writer
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_jpeg_pixmap_writer(
    _ctx: Handle,
    path: *const c_char,
    options: *const c_char,
) -> Handle {
    let path_str = if path.is_null() {
        None
    } else {
        unsafe { CStr::from_ptr(path).to_str().ok().map(String::from) }
    };

    let opts = if options.is_null() {
        WriterOptions::default()
    } else {
        let opts_str = unsafe { CStr::from_ptr(options).to_str().unwrap_or("") };
        WriterOptions::parse(opts_str)
    };

    let writer = DocumentWriter::new(WriterFormat::Jpeg, path_str, opts);
    WRITERS.insert(writer)
}

/// Create PAM pixmap writer
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_pam_pixmap_writer(
    _ctx: Handle,
    path: *const c_char,
    options: *const c_char,
) -> Handle {
    let path_str = if path.is_null() {
        None
    } else {
        unsafe { CStr::from_ptr(path).to_str().ok().map(String::from) }
    };

    let opts = if options.is_null() {
        WriterOptions::default()
    } else {
        let opts_str = unsafe { CStr::from_ptr(options).to_str().unwrap_or("") };
        WriterOptions::parse(opts_str)
    };

    let writer = DocumentWriter::new(WriterFormat::Pam, path_str, opts);
    WRITERS.insert(writer)
}

/// Create PNM pixmap writer
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_pnm_pixmap_writer(
    _ctx: Handle,
    path: *const c_char,
    options: *const c_char,
) -> Handle {
    let path_str = if path.is_null() {
        None
    } else {
        unsafe { CStr::from_ptr(path).to_str().ok().map(String::from) }
    };

    let opts = if options.is_null() {
        WriterOptions::default()
    } else {
        let opts_str = unsafe { CStr::from_ptr(options).to_str().unwrap_or("") };
        WriterOptions::parse(opts_str)
    };

    let writer = DocumentWriter::new(WriterFormat::Pnm, path_str, opts);
    WRITERS.insert(writer)
}

/// Create PGM pixmap writer
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_pgm_pixmap_writer(
    _ctx: Handle,
    path: *const c_char,
    options: *const c_char,
) -> Handle {
    let path_str = if path.is_null() {
        None
    } else {
        unsafe { CStr::from_ptr(path).to_str().ok().map(String::from) }
    };

    let opts = if options.is_null() {
        WriterOptions::default()
    } else {
        let opts_str = unsafe { CStr::from_ptr(options).to_str().unwrap_or("") };
        WriterOptions::parse(opts_str)
    };

    let writer = DocumentWriter::new(WriterFormat::Pgm, path_str, opts);
    WRITERS.insert(writer)
}

/// Create PPM pixmap writer
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_ppm_pixmap_writer(
    _ctx: Handle,
    path: *const c_char,
    options: *const c_char,
) -> Handle {
    let path_str = if path.is_null() {
        None
    } else {
        unsafe { CStr::from_ptr(path).to_str().ok().map(String::from) }
    };

    let opts = if options.is_null() {
        WriterOptions::default()
    } else {
        let opts_str = unsafe { CStr::from_ptr(options).to_str().unwrap_or("") };
        WriterOptions::parse(opts_str)
    };

    let writer = DocumentWriter::new(WriterFormat::Ppm, path_str, opts);
    WRITERS.insert(writer)
}

/// Create PBM pixmap writer
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_pbm_pixmap_writer(
    _ctx: Handle,
    path: *const c_char,
    options: *const c_char,
) -> Handle {
    let path_str = if path.is_null() {
        None
    } else {
        unsafe { CStr::from_ptr(path).to_str().ok().map(String::from) }
    };

    let opts = if options.is_null() {
        WriterOptions::default()
    } else {
        let opts_str = unsafe { CStr::from_ptr(options).to_str().unwrap_or("") };
        WriterOptions::parse(opts_str)
    };

    let writer = DocumentWriter::new(WriterFormat::Pbm, path_str, opts);
    WRITERS.insert(writer)
}

/// Create PKM pixmap writer
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_pkm_pixmap_writer(
    _ctx: Handle,
    path: *const c_char,
    options: *const c_char,
) -> Handle {
    let path_str = if path.is_null() {
        None
    } else {
        unsafe { CStr::from_ptr(path).to_str().ok().map(String::from) }
    };

    let opts = if options.is_null() {
        WriterOptions::default()
    } else {
        let opts_str = unsafe { CStr::from_ptr(options).to_str().unwrap_or("") };
        WriterOptions::parse(opts_str)
    };

    let writer = DocumentWriter::new(WriterFormat::Pkm, path_str, opts);
    WRITERS.insert(writer)
}

// ============================================================================
// FFI Functions - Page Lifecycle
// ============================================================================

/// Begin writing a page
#[unsafe(no_mangle)]
pub extern "C" fn fz_begin_page(
    _ctx: Handle,
    wri: Handle,
    mediabox_x0: f32,
    mediabox_y0: f32,
    mediabox_x1: f32,
    mediabox_y1: f32,
) -> Handle {
    let mediabox = Rect {
        x0: mediabox_x0,
        y0: mediabox_y0,
        x1: mediabox_x1,
        y1: mediabox_y1,
    };

    if let Some(writer_arc) = WRITERS.get(wri) {
        let mut writer = writer_arc.lock().unwrap();
        if writer.begin_page(mediabox) {
            // Create a device handle for this page
            let device = WriterDevice {
                writer: wri,
                page_index: writer.page_count,
            };
            let dev_handle = WRITER_DEVICES.insert(device);
            writer.current_device = Some(dev_handle);
            return dev_handle;
        }
    }
    0
}

/// End writing a page
#[unsafe(no_mangle)]
pub extern "C" fn fz_end_page(_ctx: Handle, wri: Handle) {
    if let Some(writer_arc) = WRITERS.get(wri) {
        let mut writer = writer_arc.lock().unwrap();
        if let Some(dev_handle) = writer.current_device {
            WRITER_DEVICES.remove(dev_handle);
        }
        writer.end_page();
    }
}

/// Write a full document
#[unsafe(no_mangle)]
pub extern "C" fn fz_write_document(_ctx: Handle, wri: Handle, doc: Handle) {
    // Get page count from document and write each page
    if let Some(_doc_arc) = crate::ffi::DOCUMENTS.get(doc) {
        if let Some(writer_arc) = WRITERS.get(wri) {
            let mut writer = writer_arc.lock().unwrap();
            // For a basic implementation, we'd iterate through document pages
            // and call begin_page/end_page for each
            // This requires document page enumeration which depends on document.rs

            // For now, create a single empty page as placeholder
            let mediabox = Rect {
                x0: 0.0,
                y0: 0.0,
                x1: 612.0,
                y1: 792.0,
            }; // Letter size
            writer.begin_page(mediabox);
            writer.end_page();
        }
    }
}

/// Close the document writer
#[unsafe(no_mangle)]
pub extern "C" fn fz_close_document_writer(_ctx: Handle, wri: Handle) {
    if let Some(writer_arc) = WRITERS.get(wri) {
        let mut writer = writer_arc.lock().unwrap();
        writer.close();
    }
}

/// Drop the document writer
#[unsafe(no_mangle)]
pub extern "C" fn fz_drop_document_writer(_ctx: Handle, wri: Handle) {
    // Close if not already closed
    if let Some(writer_arc) = WRITERS.get(wri) {
        {
            let mut writer = writer_arc.lock().unwrap();
            if writer.state != WriterState::Closed {
                writer.close();
            }
        }
    }
    WRITERS.remove(wri);
}

// ============================================================================
// FFI Functions - Writer Info
// ============================================================================

/// Get writer format
#[unsafe(no_mangle)]
pub extern "C" fn fz_document_writer_format(wri: Handle) -> i32 {
    if let Some(writer_arc) = WRITERS.get(wri) {
        let writer = writer_arc.lock().unwrap();
        writer.format as i32
    } else {
        -1
    }
}

/// Get writer page count
#[unsafe(no_mangle)]
pub extern "C" fn fz_document_writer_page_count(wri: Handle) -> i32 {
    if let Some(writer_arc) = WRITERS.get(wri) {
        let writer = writer_arc.lock().unwrap();
        writer.page_count
    } else {
        0
    }
}

/// Check if writer is closed
#[unsafe(no_mangle)]
pub extern "C" fn fz_document_writer_is_closed(wri: Handle) -> i32 {
    if let Some(writer_arc) = WRITERS.get(wri) {
        let writer = writer_arc.lock().unwrap();
        if writer.state == WriterState::Closed {
            1
        } else {
            0
        }
    } else {
        1
    }
}

// ============================================================================
// FFI Functions - OCR Progress Callback
// ============================================================================

/// Global storage for OCR progress callbacks
static OCR_PROGRESS_CALLBACKS: LazyLock<Mutex<HashMap<Handle, OcrProgressCallback>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// OCR progress callback wrapper
pub struct OcrProgressCallback {
    pub callback: extern "C" fn(Handle, *mut std::ffi::c_void, i32, i32) -> i32,
    pub user_data: *mut std::ffi::c_void,
}

// Safety: callback is a function pointer, user_data is externally managed
unsafe impl Send for OcrProgressCallback {}
unsafe impl Sync for OcrProgressCallback {}

/// Set OCR progress callback for writer
#[unsafe(no_mangle)]
pub extern "C" fn fz_pdfocr_writer_set_progress(
    _ctx: Handle,
    wri: Handle,
    progress: extern "C" fn(Handle, *mut std::ffi::c_void, i32, i32) -> i32,
    user_data: *mut std::ffi::c_void,
) {
    let mut callbacks = OCR_PROGRESS_CALLBACKS.lock().unwrap();
    callbacks.insert(
        wri,
        OcrProgressCallback {
            callback: progress,
            user_data,
        },
    );
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    #[test]
    fn test_writer_format_from_str() {
        assert_eq!(WriterFormat::from_str("pdf"), Some(WriterFormat::Pdf));
        assert_eq!(WriterFormat::from_str("PDF"), Some(WriterFormat::Pdf));
        assert_eq!(WriterFormat::from_str("svg"), Some(WriterFormat::Svg));
        assert_eq!(WriterFormat::from_str("png"), Some(WriterFormat::Png));
        assert_eq!(WriterFormat::from_str("jpeg"), Some(WriterFormat::Jpeg));
        assert_eq!(WriterFormat::from_str("jpg"), Some(WriterFormat::Jpeg));
        assert_eq!(WriterFormat::from_str("unknown"), None);
    }

    #[test]
    fn test_writer_format_extension() {
        assert_eq!(WriterFormat::Pdf.extension(), "pdf");
        assert_eq!(WriterFormat::Svg.extension(), "svg");
        assert_eq!(WriterFormat::Png.extension(), "png");
        assert_eq!(WriterFormat::Jpeg.extension(), "jpg");
    }

    #[test]
    fn test_writer_format_multipage() {
        assert!(WriterFormat::Pdf.is_multipage());
        assert!(WriterFormat::Svg.is_multipage());
        assert!(!WriterFormat::Png.is_multipage());
        assert!(!WriterFormat::Jpeg.is_multipage());
    }

    #[test]
    fn test_writer_options_parse() {
        let opts = WriterOptions::parse("resolution=300,quality=85,linearize");
        assert_eq!(opts.resolution, Some(300.0));
        assert_eq!(opts.quality, Some(85));
        assert!(opts.linearize);
        assert!(!opts.encrypt);
    }

    #[test]
    fn test_writer_options_has() {
        let opts = WriterOptions::parse("foo=bar,baz");
        assert!(opts.has("foo"));
        assert!(opts.has("baz"));
        assert!(!opts.has("qux"));
    }

    #[test]
    fn test_new_document_writer() {
        let ctx = 1;
        let path = CString::new("/tmp/test.pdf").unwrap();
        let format = CString::new("pdf").unwrap();

        let wri = fz_new_document_writer(ctx, path.as_ptr(), format.as_ptr(), std::ptr::null());
        assert!(wri > 0);

        fz_drop_document_writer(ctx, wri);
    }

    #[test]
    fn test_pdf_writer() {
        let ctx = 1;
        let path = CString::new("/tmp/test_pdf.pdf").unwrap();

        let wri = fz_new_pdf_writer(ctx, path.as_ptr(), std::ptr::null());
        assert!(wri > 0);

        // Begin a page
        let dev = fz_begin_page(ctx, wri, 0.0, 0.0, 612.0, 792.0);
        assert!(dev > 0);

        // End the page
        fz_end_page(ctx, wri);

        // Close and drop
        fz_close_document_writer(ctx, wri);
        fz_drop_document_writer(ctx, wri);
    }

    #[test]
    fn test_svg_writer() {
        let ctx = 1;
        let path = CString::new("/tmp/test.svg").unwrap();

        let wri = fz_new_svg_writer(ctx, path.as_ptr(), std::ptr::null());
        assert!(wri > 0);

        fz_drop_document_writer(ctx, wri);
    }

    #[test]
    fn test_text_writer() {
        let ctx = 1;
        let format = CString::new("text").unwrap();
        let path = CString::new("/tmp/test.txt").unwrap();

        let wri = fz_new_text_writer(ctx, format.as_ptr(), path.as_ptr(), std::ptr::null());
        assert!(wri > 0);

        fz_drop_document_writer(ctx, wri);
    }

    #[test]
    fn test_png_writer() {
        let ctx = 1;
        let path = CString::new("/tmp/test.png").unwrap();

        let wri = fz_new_png_pixmap_writer(ctx, path.as_ptr(), std::ptr::null());
        assert!(wri > 0);

        fz_drop_document_writer(ctx, wri);
    }

    #[test]
    fn test_option_eq() {
        let a1 = CString::new("foo").unwrap();
        let a2 = CString::new("foo,bar").unwrap();
        let b = CString::new("foo").unwrap();

        assert_eq!(fz_option_eq(a1.as_ptr(), b.as_ptr()), 1);
        assert_eq!(fz_option_eq(a2.as_ptr(), b.as_ptr()), 1);

        let c = CString::new("bar").unwrap();
        assert_eq!(fz_option_eq(a1.as_ptr(), c.as_ptr()), 0);
    }

    #[test]
    fn test_copy_option() {
        let ctx = 1;
        let val = CString::new("hello,world").unwrap();
        let mut dest = [0i8; 32];

        let overflow = fz_copy_option(ctx, val.as_ptr(), dest.as_mut_ptr(), 32);
        assert_eq!(overflow, 0);

        let result = unsafe { CStr::from_ptr(dest.as_ptr()) };
        assert_eq!(result.to_str().unwrap(), "hello");
    }

    #[test]
    fn test_writer_page_count() {
        let ctx = 1;

        let wri = fz_new_pdf_writer(ctx, std::ptr::null(), std::ptr::null());
        assert_eq!(fz_document_writer_page_count(wri), 0);

        fz_begin_page(ctx, wri, 0.0, 0.0, 612.0, 792.0);
        fz_end_page(ctx, wri);
        assert_eq!(fz_document_writer_page_count(wri), 1);

        fz_begin_page(ctx, wri, 0.0, 0.0, 612.0, 792.0);
        fz_end_page(ctx, wri);
        assert_eq!(fz_document_writer_page_count(wri), 2);

        fz_drop_document_writer(ctx, wri);
    }

    #[test]
    fn test_writer_is_closed() {
        let ctx = 1;

        let wri = fz_new_pdf_writer(ctx, std::ptr::null(), std::ptr::null());
        assert_eq!(fz_document_writer_is_closed(wri), 0);

        fz_close_document_writer(ctx, wri);
        assert_eq!(fz_document_writer_is_closed(wri), 1);

        fz_drop_document_writer(ctx, wri);
    }

    #[test]
    fn test_all_format_writers() {
        let ctx = 1;

        // Test each format-specific writer
        let writers = vec![
            fz_new_pdf_writer(ctx, std::ptr::null(), std::ptr::null()),
            fz_new_svg_writer(ctx, std::ptr::null(), std::ptr::null()),
            fz_new_odt_writer(ctx, std::ptr::null(), std::ptr::null()),
            fz_new_docx_writer(ctx, std::ptr::null(), std::ptr::null()),
            fz_new_ps_writer(ctx, std::ptr::null(), std::ptr::null()),
            fz_new_pcl_writer(ctx, std::ptr::null(), std::ptr::null()),
            fz_new_pclm_writer(ctx, std::ptr::null(), std::ptr::null()),
            fz_new_pwg_writer(ctx, std::ptr::null(), std::ptr::null()),
            fz_new_cbz_writer(ctx, std::ptr::null(), std::ptr::null()),
            fz_new_csv_writer(ctx, std::ptr::null(), std::ptr::null()),
            fz_new_pdfocr_writer(ctx, std::ptr::null(), std::ptr::null()),
            fz_new_png_pixmap_writer(ctx, std::ptr::null(), std::ptr::null()),
            fz_new_jpeg_pixmap_writer(ctx, std::ptr::null(), std::ptr::null()),
            fz_new_pam_pixmap_writer(ctx, std::ptr::null(), std::ptr::null()),
            fz_new_pnm_pixmap_writer(ctx, std::ptr::null(), std::ptr::null()),
            fz_new_pgm_pixmap_writer(ctx, std::ptr::null(), std::ptr::null()),
            fz_new_ppm_pixmap_writer(ctx, std::ptr::null(), std::ptr::null()),
            fz_new_pbm_pixmap_writer(ctx, std::ptr::null(), std::ptr::null()),
            fz_new_pkm_pixmap_writer(ctx, std::ptr::null(), std::ptr::null()),
        ];

        for wri in writers {
            assert!(wri > 0);
            fz_drop_document_writer(ctx, wri);
        }
    }
}
