//! CBZ/CBR (Comic Book Archives) FFI Module
//!
//! Provides support for comic book archive formats, including ZIP-based CBZ
//! and RAR-based CBR, with image sequence handling and ComicInfo.xml metadata.

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
// CBZ Format Constants
// ============================================================================

/// CBZ (ZIP-based) format
pub const CBZ_FORMAT_CBZ: i32 = 0;
/// CBR (RAR-based) format
pub const CBZ_FORMAT_CBR: i32 = 1;
/// CB7 (7z-based) format
pub const CBZ_FORMAT_CB7: i32 = 2;
/// CBT (TAR-based) format
pub const CBZ_FORMAT_CBT: i32 = 3;

// ============================================================================
// Image Format Constants
// ============================================================================

/// JPEG image
pub const CBZ_IMAGE_JPEG: i32 = 0;
/// PNG image
pub const CBZ_IMAGE_PNG: i32 = 1;
/// GIF image
pub const CBZ_IMAGE_GIF: i32 = 2;
/// BMP image
pub const CBZ_IMAGE_BMP: i32 = 3;
/// TIFF image
pub const CBZ_IMAGE_TIFF: i32 = 4;
/// WebP image
pub const CBZ_IMAGE_WEBP: i32 = 5;
/// JPEG 2000 image
pub const CBZ_IMAGE_JP2: i32 = 6;
/// Unknown image format
pub const CBZ_IMAGE_UNKNOWN: i32 = 99;

// ============================================================================
// Reading Direction Constants
// ============================================================================

/// Left-to-right reading (Western comics)
pub const CBZ_READ_LTR: i32 = 0;
/// Right-to-left reading (Manga)
pub const CBZ_READ_RTL: i32 = 1;

// ============================================================================
// Manga Constants (Yes/No/Unknown)
// ============================================================================

/// Unknown manga status
pub const CBZ_MANGA_UNKNOWN: i32 = 0;
/// Is manga (right-to-left)
pub const CBZ_MANGA_YES: i32 = 1;
/// Not manga (left-to-right)
pub const CBZ_MANGA_NO: i32 = 2;
/// Manga with right-to-left and left-to-right mixed
pub const CBZ_MANGA_YES_RTL: i32 = 3;

// ============================================================================
// Supported Image Extensions
// ============================================================================

const SUPPORTED_EXTENSIONS: &[&str] = &[
    ".bmp", ".gif", ".hdp", ".j2k", ".jb2", ".jbig2", ".jp2", ".jpeg", ".jpg", ".jpx", ".jxr",
    ".pam", ".pbm", ".pgm", ".pkm", ".png", ".pnm", ".ppm", ".tif", ".tiff", ".wdp", ".webp",
];

// ============================================================================
// ComicInfo Metadata
// ============================================================================

/// ComicInfo.xml metadata structure
#[derive(Debug, Clone, Default)]
pub struct ComicInfo {
    /// Title of the comic
    pub title: Option<String>,
    /// Series name
    pub series: Option<String>,
    /// Issue number
    pub number: Option<String>,
    /// Volume number
    pub volume: Option<i32>,
    /// Alternate series
    pub alternate_series: Option<String>,
    /// Alternate number
    pub alternate_number: Option<String>,
    /// Story arc
    pub story_arc: Option<String>,
    /// Series group
    pub series_group: Option<String>,
    /// Summary/description
    pub summary: Option<String>,
    /// Notes
    pub notes: Option<String>,
    /// Year of publication
    pub year: Option<i32>,
    /// Month of publication
    pub month: Option<i32>,
    /// Day of publication
    pub day: Option<i32>,
    /// Writer(s)
    pub writer: Option<String>,
    /// Penciller(s)
    pub penciller: Option<String>,
    /// Inker(s)
    pub inker: Option<String>,
    /// Colorist(s)
    pub colorist: Option<String>,
    /// Letterer(s)
    pub letterer: Option<String>,
    /// Cover artist(s)
    pub cover_artist: Option<String>,
    /// Editor(s)
    pub editor: Option<String>,
    /// Publisher
    pub publisher: Option<String>,
    /// Imprint
    pub imprint: Option<String>,
    /// Genre(s)
    pub genre: Option<String>,
    /// Tags
    pub tags: Option<String>,
    /// Web link
    pub web: Option<String>,
    /// Page count
    pub page_count: Option<i32>,
    /// Language (ISO code)
    pub language_iso: Option<String>,
    /// Format (e.g., "Trade Paperback")
    pub format: Option<String>,
    /// Black and white (true/false)
    pub black_and_white: bool,
    /// Manga reading direction
    pub manga: i32,
    /// Characters
    pub characters: Option<String>,
    /// Teams
    pub teams: Option<String>,
    /// Locations
    pub locations: Option<String>,
    /// Age rating
    pub age_rating: Option<String>,
    /// Community rating (0-5)
    pub community_rating: Option<f32>,
    /// Scan information
    pub scan_information: Option<String>,
}

impl ComicInfo {
    pub fn new() -> Self {
        Self {
            manga: CBZ_MANGA_UNKNOWN,
            ..Default::default()
        }
    }
}

// ============================================================================
// CBZ Page
// ============================================================================

/// A page in the comic book
#[derive(Debug, Clone)]
pub struct CbzPage {
    /// Page index (0-based)
    pub index: i32,
    /// File name within archive
    pub filename: String,
    /// Image format
    pub format: i32,
    /// Image width (pixels)
    pub width: i32,
    /// Image height (pixels)
    pub height: i32,
    /// Image data (raw bytes)
    pub data: Vec<u8>,
    /// Page type (e.g., "FrontCover", "Story")
    pub page_type: Option<String>,
    /// Is double page spread
    pub double_page: bool,
    /// Bookmark name
    pub bookmark: Option<String>,
}

impl CbzPage {
    pub fn new(index: i32, filename: &str) -> Self {
        Self {
            index,
            filename: filename.to_string(),
            format: CBZ_IMAGE_UNKNOWN,
            width: 0,
            height: 0,
            data: Vec::new(),
            page_type: None,
            double_page: false,
            bookmark: None,
        }
    }

    pub fn detect_format(&mut self) {
        let lower = self.filename.to_lowercase();
        self.format = if lower.ends_with(".jpg") || lower.ends_with(".jpeg") {
            CBZ_IMAGE_JPEG
        } else if lower.ends_with(".png") {
            CBZ_IMAGE_PNG
        } else if lower.ends_with(".gif") {
            CBZ_IMAGE_GIF
        } else if lower.ends_with(".bmp") {
            CBZ_IMAGE_BMP
        } else if lower.ends_with(".tif") || lower.ends_with(".tiff") {
            CBZ_IMAGE_TIFF
        } else if lower.ends_with(".webp") {
            CBZ_IMAGE_WEBP
        } else if lower.ends_with(".jp2") || lower.ends_with(".j2k") || lower.ends_with(".jpx") {
            CBZ_IMAGE_JP2
        } else {
            CBZ_IMAGE_UNKNOWN
        };
    }
}

// ============================================================================
// CBZ Document
// ============================================================================

/// CBZ/CBR document structure
pub struct CbzDocument {
    /// Context handle
    pub context: ContextHandle,
    /// Archive format
    pub format: i32,
    /// Metadata
    pub info: ComicInfo,
    /// Pages (sorted by natural order)
    pub pages: Vec<CbzPage>,
    /// File entries in archive
    pub entries: Vec<String>,
    /// Raw archive data
    pub archive_data: Vec<u8>,
}

impl CbzDocument {
    pub fn new(context: ContextHandle) -> Self {
        Self {
            context,
            format: CBZ_FORMAT_CBZ,
            info: ComicInfo::new(),
            pages: Vec::new(),
            entries: Vec::new(),
            archive_data: Vec::new(),
        }
    }

    pub fn page_count(&self) -> i32 {
        self.pages.len() as i32
    }

    pub fn get_page(&self, index: i32) -> Option<&CbzPage> {
        self.pages.get(index as usize)
    }

    pub fn add_page(&mut self, page: CbzPage) {
        self.pages.push(page);
    }

    pub fn sort_pages(&mut self) {
        // Natural sort order (like MuPDF's cbz_strnatcmp)
        self.pages
            .sort_by(|a, b| natural_cmp(&a.filename, &b.filename));
    }

    pub fn is_image_file(name: &str) -> bool {
        let lower = name.to_lowercase();
        SUPPORTED_EXTENSIONS.iter().any(|ext| lower.ends_with(ext))
    }

    pub fn add_entry(&mut self, name: &str) {
        if Self::is_image_file(name) {
            let index = self.pages.len() as i32;
            let mut page = CbzPage::new(index, name);
            page.detect_format();
            self.pages.push(page);
        }
        self.entries.push(name.to_string());
    }
}

// ============================================================================
// Natural Sort Comparison
// ============================================================================

fn natural_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    let mut a_chars = a.chars().peekable();
    let mut b_chars = b.chars().peekable();

    loop {
        match (a_chars.peek(), b_chars.peek()) {
            (None, None) => return std::cmp::Ordering::Equal,
            (None, Some(_)) => return std::cmp::Ordering::Less,
            (Some(_), None) => return std::cmp::Ordering::Greater,
            (Some(&ac), Some(&bc)) => {
                if ac.is_ascii_digit() && bc.is_ascii_digit() {
                    // Parse numbers
                    let mut a_num = 0u64;
                    while let Some(&c) = a_chars.peek() {
                        if c.is_ascii_digit() {
                            a_num = a_num * 10 + c.to_digit(10).unwrap() as u64;
                            a_chars.next();
                        } else {
                            break;
                        }
                    }

                    let mut b_num = 0u64;
                    while let Some(&c) = b_chars.peek() {
                        if c.is_ascii_digit() {
                            b_num = b_num * 10 + c.to_digit(10).unwrap() as u64;
                            b_chars.next();
                        } else {
                            break;
                        }
                    }

                    match a_num.cmp(&b_num) {
                        std::cmp::Ordering::Equal => continue,
                        other => return other,
                    }
                } else {
                    // Compare characters (case-insensitive)
                    let a_upper = ac.to_ascii_uppercase();
                    let b_upper = bc.to_ascii_uppercase();

                    match a_upper.cmp(&b_upper) {
                        std::cmp::Ordering::Equal => {
                            a_chars.next();
                            b_chars.next();
                            continue;
                        }
                        other => return other,
                    }
                }
            }
        }
    }
}

// ============================================================================
// Global Handle Store
// ============================================================================

pub static CBZ_DOCUMENTS: LazyLock<HandleStore<CbzDocument>> = LazyLock::new(HandleStore::new);

// ============================================================================
// FFI Functions - Document Management
// ============================================================================

/// Create a new CBZ document.
#[unsafe(no_mangle)]
pub extern "C" fn cbz_new_document(ctx: ContextHandle) -> Handle {
    let doc = CbzDocument::new(ctx);
    CBZ_DOCUMENTS.insert(doc)
}

/// Drop a CBZ document.
#[unsafe(no_mangle)]
pub extern "C" fn cbz_drop_document(_ctx: ContextHandle, doc: Handle) {
    CBZ_DOCUMENTS.remove(doc);
}

/// Open a CBZ document from a file path.
#[unsafe(no_mangle)]
pub extern "C" fn cbz_open_document(ctx: ContextHandle, filename: *const c_char) -> Handle {
    if filename.is_null() {
        return 0;
    }

    let path = unsafe { CStr::from_ptr(filename).to_string_lossy() };

    let mut doc = CbzDocument::new(ctx);

    // Detect format from extension
    let lower = path.to_lowercase();
    doc.format = if lower.ends_with(".cbz") {
        CBZ_FORMAT_CBZ
    } else if lower.ends_with(".cbr") {
        CBZ_FORMAT_CBR
    } else if lower.ends_with(".cb7") {
        CBZ_FORMAT_CB7
    } else if lower.ends_with(".cbt") {
        CBZ_FORMAT_CBT
    } else {
        CBZ_FORMAT_CBZ
    };

    CBZ_DOCUMENTS.insert(doc)
}

/// Open a CBZ document from a stream.
#[unsafe(no_mangle)]
pub extern "C" fn cbz_open_document_with_stream(
    ctx: ContextHandle,
    _stream: StreamHandle,
) -> Handle {
    let doc = CbzDocument::new(ctx);
    CBZ_DOCUMENTS.insert(doc)
}

/// Open a CBZ document from an archive.
#[unsafe(no_mangle)]
pub extern "C" fn cbz_open_document_with_archive(
    ctx: ContextHandle,
    _archive: ArchiveHandle,
) -> Handle {
    let doc = CbzDocument::new(ctx);
    CBZ_DOCUMENTS.insert(doc)
}

// ============================================================================
// FFI Functions - Document Properties
// ============================================================================

/// Get document format.
#[unsafe(no_mangle)]
pub extern "C" fn cbz_get_format(_ctx: ContextHandle, doc: Handle) -> i32 {
    if let Some(d) = CBZ_DOCUMENTS.get(doc) {
        let d = d.lock().unwrap();
        return d.format;
    }
    CBZ_FORMAT_CBZ
}

/// Get page count.
#[unsafe(no_mangle)]
pub extern "C" fn cbz_page_count(_ctx: ContextHandle, doc: Handle) -> i32 {
    if let Some(d) = CBZ_DOCUMENTS.get(doc) {
        let d = d.lock().unwrap();
        return d.page_count();
    }
    0
}

/// Add an entry to the document.
#[unsafe(no_mangle)]
pub extern "C" fn cbz_add_entry(_ctx: ContextHandle, doc: Handle, name: *const c_char) -> i32 {
    if name.is_null() {
        return 0;
    }

    if let Some(d) = CBZ_DOCUMENTS.get(doc) {
        let mut d = d.lock().unwrap();
        let entry_name = unsafe { CStr::from_ptr(name).to_string_lossy().to_string() };
        d.add_entry(&entry_name);
        return 1;
    }
    0
}

/// Sort pages by natural order.
#[unsafe(no_mangle)]
pub extern "C" fn cbz_sort_pages(_ctx: ContextHandle, doc: Handle) -> i32 {
    if let Some(d) = CBZ_DOCUMENTS.get(doc) {
        let mut d = d.lock().unwrap();
        d.sort_pages();
        return 1;
    }
    0
}

// ============================================================================
// FFI Functions - Page Access
// ============================================================================

/// Get page filename.
#[unsafe(no_mangle)]
pub extern "C" fn cbz_get_page_filename(
    _ctx: ContextHandle,
    doc: Handle,
    page_num: i32,
) -> *mut c_char {
    if let Some(d) = CBZ_DOCUMENTS.get(doc) {
        let d = d.lock().unwrap();
        if let Some(page) = d.get_page(page_num) {
            if let Ok(cstr) = CString::new(page.filename.clone()) {
                return cstr.into_raw();
            }
        }
    }
    ptr::null_mut()
}

/// Get page image format.
#[unsafe(no_mangle)]
pub extern "C" fn cbz_get_page_format(_ctx: ContextHandle, doc: Handle, page_num: i32) -> i32 {
    if let Some(d) = CBZ_DOCUMENTS.get(doc) {
        let d = d.lock().unwrap();
        if let Some(page) = d.get_page(page_num) {
            return page.format;
        }
    }
    CBZ_IMAGE_UNKNOWN
}

/// Get page dimensions.
#[unsafe(no_mangle)]
pub extern "C" fn cbz_get_page_size(
    _ctx: ContextHandle,
    doc: Handle,
    page_num: i32,
    width: *mut i32,
    height: *mut i32,
) -> i32 {
    if let Some(d) = CBZ_DOCUMENTS.get(doc) {
        let d = d.lock().unwrap();
        if let Some(page) = d.get_page(page_num) {
            if !width.is_null() {
                unsafe {
                    *width = page.width;
                }
            }
            if !height.is_null() {
                unsafe {
                    *height = page.height;
                }
            }
            return 1;
        }
    }
    0
}

/// Set page dimensions.
#[unsafe(no_mangle)]
pub extern "C" fn cbz_set_page_size(
    _ctx: ContextHandle,
    doc: Handle,
    page_num: i32,
    width: i32,
    height: i32,
) -> i32 {
    if let Some(d) = CBZ_DOCUMENTS.get(doc) {
        let mut d = d.lock().unwrap();
        if let Some(page) = d.pages.get_mut(page_num as usize) {
            page.width = width;
            page.height = height;
            return 1;
        }
    }
    0
}

/// Check if page is double-page spread.
#[unsafe(no_mangle)]
pub extern "C" fn cbz_page_is_double(_ctx: ContextHandle, doc: Handle, page_num: i32) -> i32 {
    if let Some(d) = CBZ_DOCUMENTS.get(doc) {
        let d = d.lock().unwrap();
        if let Some(page) = d.get_page(page_num) {
            return if page.double_page { 1 } else { 0 };
        }
    }
    0
}

/// Set page double-page spread flag.
#[unsafe(no_mangle)]
pub extern "C" fn cbz_set_page_double(
    _ctx: ContextHandle,
    doc: Handle,
    page_num: i32,
    double: i32,
) -> i32 {
    if let Some(d) = CBZ_DOCUMENTS.get(doc) {
        let mut d = d.lock().unwrap();
        if let Some(page) = d.pages.get_mut(page_num as usize) {
            page.double_page = double != 0;
            return 1;
        }
    }
    0
}

// ============================================================================
// FFI Functions - ComicInfo Metadata
// ============================================================================

/// Get comic title.
#[unsafe(no_mangle)]
pub extern "C" fn cbz_get_title(_ctx: ContextHandle, doc: Handle) -> *mut c_char {
    if let Some(d) = CBZ_DOCUMENTS.get(doc) {
        let d = d.lock().unwrap();
        if let Some(ref title) = d.info.title {
            if let Ok(cstr) = CString::new(title.clone()) {
                return cstr.into_raw();
            }
        }
    }
    ptr::null_mut()
}

/// Set comic title.
#[unsafe(no_mangle)]
pub extern "C" fn cbz_set_title(_ctx: ContextHandle, doc: Handle, title: *const c_char) -> i32 {
    if title.is_null() {
        return 0;
    }

    if let Some(d) = CBZ_DOCUMENTS.get(doc) {
        let mut d = d.lock().unwrap();
        let t = unsafe { CStr::from_ptr(title).to_string_lossy().to_string() };
        d.info.title = Some(t);
        return 1;
    }
    0
}

/// Get series name.
#[unsafe(no_mangle)]
pub extern "C" fn cbz_get_series(_ctx: ContextHandle, doc: Handle) -> *mut c_char {
    if let Some(d) = CBZ_DOCUMENTS.get(doc) {
        let d = d.lock().unwrap();
        if let Some(ref series) = d.info.series {
            if let Ok(cstr) = CString::new(series.clone()) {
                return cstr.into_raw();
            }
        }
    }
    ptr::null_mut()
}

/// Set series name.
#[unsafe(no_mangle)]
pub extern "C" fn cbz_set_series(_ctx: ContextHandle, doc: Handle, series: *const c_char) -> i32 {
    if series.is_null() {
        return 0;
    }

    if let Some(d) = CBZ_DOCUMENTS.get(doc) {
        let mut d = d.lock().unwrap();
        let s = unsafe { CStr::from_ptr(series).to_string_lossy().to_string() };
        d.info.series = Some(s);
        return 1;
    }
    0
}

/// Get issue number.
#[unsafe(no_mangle)]
pub extern "C" fn cbz_get_number(_ctx: ContextHandle, doc: Handle) -> *mut c_char {
    if let Some(d) = CBZ_DOCUMENTS.get(doc) {
        let d = d.lock().unwrap();
        if let Some(ref num) = d.info.number {
            if let Ok(cstr) = CString::new(num.clone()) {
                return cstr.into_raw();
            }
        }
    }
    ptr::null_mut()
}

/// Set issue number.
#[unsafe(no_mangle)]
pub extern "C" fn cbz_set_number(_ctx: ContextHandle, doc: Handle, number: *const c_char) -> i32 {
    if number.is_null() {
        return 0;
    }

    if let Some(d) = CBZ_DOCUMENTS.get(doc) {
        let mut d = d.lock().unwrap();
        let n = unsafe { CStr::from_ptr(number).to_string_lossy().to_string() };
        d.info.number = Some(n);
        return 1;
    }
    0
}

/// Get writer.
#[unsafe(no_mangle)]
pub extern "C" fn cbz_get_writer(_ctx: ContextHandle, doc: Handle) -> *mut c_char {
    if let Some(d) = CBZ_DOCUMENTS.get(doc) {
        let d = d.lock().unwrap();
        if let Some(ref writer) = d.info.writer {
            if let Ok(cstr) = CString::new(writer.clone()) {
                return cstr.into_raw();
            }
        }
    }
    ptr::null_mut()
}

/// Set writer.
#[unsafe(no_mangle)]
pub extern "C" fn cbz_set_writer(_ctx: ContextHandle, doc: Handle, writer: *const c_char) -> i32 {
    if writer.is_null() {
        return 0;
    }

    if let Some(d) = CBZ_DOCUMENTS.get(doc) {
        let mut d = d.lock().unwrap();
        let w = unsafe { CStr::from_ptr(writer).to_string_lossy().to_string() };
        d.info.writer = Some(w);
        return 1;
    }
    0
}

/// Get publisher.
#[unsafe(no_mangle)]
pub extern "C" fn cbz_get_publisher(_ctx: ContextHandle, doc: Handle) -> *mut c_char {
    if let Some(d) = CBZ_DOCUMENTS.get(doc) {
        let d = d.lock().unwrap();
        if let Some(ref pub_) = d.info.publisher {
            if let Ok(cstr) = CString::new(pub_.clone()) {
                return cstr.into_raw();
            }
        }
    }
    ptr::null_mut()
}

/// Set publisher.
#[unsafe(no_mangle)]
pub extern "C" fn cbz_set_publisher(
    _ctx: ContextHandle,
    doc: Handle,
    publisher: *const c_char,
) -> i32 {
    if publisher.is_null() {
        return 0;
    }

    if let Some(d) = CBZ_DOCUMENTS.get(doc) {
        let mut d = d.lock().unwrap();
        let p = unsafe { CStr::from_ptr(publisher).to_string_lossy().to_string() };
        d.info.publisher = Some(p);
        return 1;
    }
    0
}

/// Get year.
#[unsafe(no_mangle)]
pub extern "C" fn cbz_get_year(_ctx: ContextHandle, doc: Handle) -> i32 {
    if let Some(d) = CBZ_DOCUMENTS.get(doc) {
        let d = d.lock().unwrap();
        return d.info.year.unwrap_or(0);
    }
    0
}

/// Set year.
#[unsafe(no_mangle)]
pub extern "C" fn cbz_set_year(_ctx: ContextHandle, doc: Handle, year: i32) -> i32 {
    if let Some(d) = CBZ_DOCUMENTS.get(doc) {
        let mut d = d.lock().unwrap();
        d.info.year = if year > 0 { Some(year) } else { None };
        return 1;
    }
    0
}

/// Get manga reading direction.
#[unsafe(no_mangle)]
pub extern "C" fn cbz_get_manga(_ctx: ContextHandle, doc: Handle) -> i32 {
    if let Some(d) = CBZ_DOCUMENTS.get(doc) {
        let d = d.lock().unwrap();
        return d.info.manga;
    }
    CBZ_MANGA_UNKNOWN
}

/// Set manga reading direction.
#[unsafe(no_mangle)]
pub extern "C" fn cbz_set_manga(_ctx: ContextHandle, doc: Handle, manga: i32) -> i32 {
    if let Some(d) = CBZ_DOCUMENTS.get(doc) {
        let mut d = d.lock().unwrap();
        d.info.manga = manga;
        return 1;
    }
    0
}

/// Get summary.
#[unsafe(no_mangle)]
pub extern "C" fn cbz_get_summary(_ctx: ContextHandle, doc: Handle) -> *mut c_char {
    if let Some(d) = CBZ_DOCUMENTS.get(doc) {
        let d = d.lock().unwrap();
        if let Some(ref summary) = d.info.summary {
            if let Ok(cstr) = CString::new(summary.clone()) {
                return cstr.into_raw();
            }
        }
    }
    ptr::null_mut()
}

/// Set summary.
#[unsafe(no_mangle)]
pub extern "C" fn cbz_set_summary(_ctx: ContextHandle, doc: Handle, summary: *const c_char) -> i32 {
    if summary.is_null() {
        return 0;
    }

    if let Some(d) = CBZ_DOCUMENTS.get(doc) {
        let mut d = d.lock().unwrap();
        let s = unsafe { CStr::from_ptr(summary).to_string_lossy().to_string() };
        d.info.summary = Some(s);
        return 1;
    }
    0
}

// ============================================================================
// FFI Functions - Utility
// ============================================================================

/// Free a string returned by CBZ functions.
#[unsafe(no_mangle)]
pub extern "C" fn cbz_free_string(s: *mut c_char) {
    if !s.is_null() {
        unsafe {
            drop(CString::from_raw(s));
        }
    }
}

/// Check if filename is a supported image.
#[unsafe(no_mangle)]
pub extern "C" fn cbz_is_image_file(_ctx: ContextHandle, filename: *const c_char) -> i32 {
    if filename.is_null() {
        return 0;
    }

    let name = unsafe { CStr::from_ptr(filename).to_string_lossy() };
    if CbzDocument::is_image_file(&name) {
        1
    } else {
        0
    }
}

/// Get format name string.
#[unsafe(no_mangle)]
pub extern "C" fn cbz_format_name(_ctx: ContextHandle, format: i32) -> *mut c_char {
    let name = match format {
        CBZ_FORMAT_CBZ => "CBZ (ZIP)",
        CBZ_FORMAT_CBR => "CBR (RAR)",
        CBZ_FORMAT_CB7 => "CB7 (7z)",
        CBZ_FORMAT_CBT => "CBT (TAR)",
        _ => "Unknown",
    };

    if let Ok(cstr) = CString::new(name) {
        return cstr.into_raw();
    }
    ptr::null_mut()
}

/// Get image format name string.
#[unsafe(no_mangle)]
pub extern "C" fn cbz_image_format_name(_ctx: ContextHandle, format: i32) -> *mut c_char {
    let name = match format {
        CBZ_IMAGE_JPEG => "JPEG",
        CBZ_IMAGE_PNG => "PNG",
        CBZ_IMAGE_GIF => "GIF",
        CBZ_IMAGE_BMP => "BMP",
        CBZ_IMAGE_TIFF => "TIFF",
        CBZ_IMAGE_WEBP => "WebP",
        CBZ_IMAGE_JP2 => "JPEG 2000",
        _ => "Unknown",
    };

    if let Ok(cstr) = CString::new(name) {
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
    fn test_format_constants() {
        assert_eq!(CBZ_FORMAT_CBZ, 0);
        assert_eq!(CBZ_FORMAT_CBR, 1);
    }

    #[test]
    fn test_image_format_constants() {
        assert_eq!(CBZ_IMAGE_JPEG, 0);
        assert_eq!(CBZ_IMAGE_PNG, 1);
    }

    #[test]
    fn test_manga_constants() {
        assert_eq!(CBZ_MANGA_UNKNOWN, 0);
        assert_eq!(CBZ_MANGA_YES, 1);
        assert_eq!(CBZ_MANGA_NO, 2);
    }

    #[test]
    fn test_natural_sort() {
        assert_eq!(natural_cmp("page1", "page2"), std::cmp::Ordering::Less);
        assert_eq!(natural_cmp("page2", "page10"), std::cmp::Ordering::Less);
        assert_eq!(natural_cmp("page10", "page2"), std::cmp::Ordering::Greater);
        assert_eq!(natural_cmp("Page1", "page1"), std::cmp::Ordering::Equal);
    }

    #[test]
    fn test_is_image_file() {
        assert!(CbzDocument::is_image_file("page001.jpg"));
        assert!(CbzDocument::is_image_file("cover.png"));
        assert!(CbzDocument::is_image_file("IMAGE.JPEG"));
        assert!(!CbzDocument::is_image_file("ComicInfo.xml"));
        assert!(!CbzDocument::is_image_file("readme.txt"));
    }

    #[test]
    fn test_cbz_page() {
        let mut page = CbzPage::new(0, "page001.jpg");
        page.detect_format();
        assert_eq!(page.format, CBZ_IMAGE_JPEG);

        let mut page2 = CbzPage::new(1, "cover.png");
        page2.detect_format();
        assert_eq!(page2.format, CBZ_IMAGE_PNG);
    }

    #[test]
    fn test_comic_info() {
        let mut info = ComicInfo::new();
        info.title = Some("Batman #1".to_string());
        info.series = Some("Batman".to_string());
        info.number = Some("1".to_string());
        info.manga = CBZ_MANGA_NO;

        assert_eq!(info.title, Some("Batman #1".to_string()));
        assert_eq!(info.manga, CBZ_MANGA_NO);
    }

    #[test]
    fn test_cbz_document() {
        let mut doc = CbzDocument::new(0);

        doc.add_entry("page002.jpg");
        doc.add_entry("page001.jpg");
        doc.add_entry("page010.jpg");
        doc.add_entry("ComicInfo.xml"); // Should not be added as page

        assert_eq!(doc.page_count(), 3);

        doc.sort_pages();
        assert_eq!(doc.pages[0].filename, "page001.jpg");
        assert_eq!(doc.pages[1].filename, "page002.jpg");
        assert_eq!(doc.pages[2].filename, "page010.jpg");
    }

    #[test]
    fn test_ffi_document() {
        let ctx = 0;

        let doc = cbz_new_document(ctx);
        assert!(doc > 0);

        assert_eq!(cbz_page_count(ctx, doc), 0);
        assert_eq!(cbz_get_format(ctx, doc), CBZ_FORMAT_CBZ);

        cbz_drop_document(ctx, doc);
    }

    #[test]
    fn test_ffi_entries() {
        let ctx = 0;
        let doc = cbz_new_document(ctx);

        let name1 = CString::new("page001.jpg").unwrap();
        let name2 = CString::new("page002.png").unwrap();

        cbz_add_entry(ctx, doc, name1.as_ptr());
        cbz_add_entry(ctx, doc, name2.as_ptr());

        assert_eq!(cbz_page_count(ctx, doc), 2);

        let filename = cbz_get_page_filename(ctx, doc, 0);
        assert!(!filename.is_null());
        unsafe {
            let s = CStr::from_ptr(filename).to_string_lossy();
            assert_eq!(s, "page001.jpg");
            cbz_free_string(filename);
        }

        assert_eq!(cbz_get_page_format(ctx, doc, 0), CBZ_IMAGE_JPEG);
        assert_eq!(cbz_get_page_format(ctx, doc, 1), CBZ_IMAGE_PNG);

        cbz_drop_document(ctx, doc);
    }

    #[test]
    fn test_ffi_metadata() {
        let ctx = 0;
        let doc = cbz_new_document(ctx);

        let title = CString::new("Spider-Man #1").unwrap();
        cbz_set_title(ctx, doc, title.as_ptr());

        let result = cbz_get_title(ctx, doc);
        assert!(!result.is_null());
        unsafe {
            let s = CStr::from_ptr(result).to_string_lossy();
            assert_eq!(s, "Spider-Man #1");
            cbz_free_string(result);
        }

        cbz_set_year(ctx, doc, 2023);
        assert_eq!(cbz_get_year(ctx, doc), 2023);

        cbz_set_manga(ctx, doc, CBZ_MANGA_YES);
        assert_eq!(cbz_get_manga(ctx, doc), CBZ_MANGA_YES);

        cbz_drop_document(ctx, doc);
    }

    #[test]
    fn test_ffi_page_size() {
        let ctx = 0;
        let doc = cbz_new_document(ctx);

        let name = CString::new("page001.jpg").unwrap();
        cbz_add_entry(ctx, doc, name.as_ptr());

        cbz_set_page_size(ctx, doc, 0, 1920, 2560);

        let mut w: i32 = 0;
        let mut h: i32 = 0;
        cbz_get_page_size(ctx, doc, 0, &mut w, &mut h);
        assert_eq!(w, 1920);
        assert_eq!(h, 2560);

        cbz_drop_document(ctx, doc);
    }

    #[test]
    fn test_ffi_format_names() {
        let ctx = 0;

        let name = cbz_format_name(ctx, CBZ_FORMAT_CBZ);
        unsafe {
            let s = CStr::from_ptr(name).to_string_lossy();
            assert!(s.contains("ZIP"));
            cbz_free_string(name);
        }

        let img_name = cbz_image_format_name(ctx, CBZ_IMAGE_PNG);
        unsafe {
            let s = CStr::from_ptr(img_name).to_string_lossy();
            assert_eq!(s, "PNG");
            cbz_free_string(img_name);
        }
    }

    #[test]
    fn test_ffi_is_image() {
        let ctx = 0;

        let jpg = CString::new("page.jpg").unwrap();
        let xml = CString::new("ComicInfo.xml").unwrap();

        assert_eq!(cbz_is_image_file(ctx, jpg.as_ptr()), 1);
        assert_eq!(cbz_is_image_file(ctx, xml.as_ptr()), 0);
    }
}
