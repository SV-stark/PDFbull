//! XPS (XML Paper Specification) Document FFI Module
//!
//! Provides support for Microsoft's XPS document format, including
//! document parsing, page rendering, and content extraction.

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
type DeviceHandle = Handle;
type ArchiveHandle = Handle;

// ============================================================================
// XPS Content Type Constants
// ============================================================================

/// Fixed document sequence
pub const XPS_CONTENT_FIXED_DOC_SEQ: i32 = 0;
/// Fixed document
pub const XPS_CONTENT_FIXED_DOC: i32 = 1;
/// Fixed page
pub const XPS_CONTENT_FIXED_PAGE: i32 = 2;
/// Font resource
pub const XPS_CONTENT_FONT: i32 = 3;
/// Image resource
pub const XPS_CONTENT_IMAGE: i32 = 4;
/// ICC profile
pub const XPS_CONTENT_ICC_PROFILE: i32 = 5;
/// Remote resource dictionary
pub const XPS_CONTENT_RESOURCE_DICT: i32 = 6;
/// Print ticket
pub const XPS_CONTENT_PRINT_TICKET: i32 = 7;
/// Thumbnail
pub const XPS_CONTENT_THUMBNAIL: i32 = 8;

// ============================================================================
// XPS Relationship Type Constants
// ============================================================================

/// Core properties relationship
pub const XPS_REL_CORE_PROPERTIES: i32 = 0;
/// Digital signature relationship
pub const XPS_REL_DIGITAL_SIGNATURE: i32 = 1;
/// Thumbnail relationship
pub const XPS_REL_THUMBNAIL: i32 = 2;
/// Print ticket relationship
pub const XPS_REL_PRINT_TICKET: i32 = 3;
/// Restricted font relationship
pub const XPS_REL_RESTRICTED_FONT: i32 = 4;
/// Required resource relationship
pub const XPS_REL_REQUIRED_RESOURCE: i32 = 5;

// ============================================================================
// XPS Part
// ============================================================================

/// An XPS part (file within the package)
#[derive(Debug, Clone)]
pub struct XpsPart {
    /// Part name (path within package)
    pub name: String,
    /// Part data
    pub data: Vec<u8>,
    /// Content type
    pub content_type: String,
}

impl XpsPart {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            data: Vec::new(),
            content_type: String::new(),
        }
    }

    pub fn with_data(mut self, data: &[u8]) -> Self {
        self.data = data.to_vec();
        self
    }

    pub fn with_content_type(mut self, content_type: &str) -> Self {
        self.content_type = content_type.to_string();
        self
    }
}

// ============================================================================
// XPS Fixed Page
// ============================================================================

/// An XPS fixed page
#[derive(Debug, Clone)]
pub struct XpsFixedPage {
    /// Page name/path
    pub name: String,
    /// Page number (0-based)
    pub number: i32,
    /// Page width in 1/96 inch units
    pub width: f32,
    /// Page height in 1/96 inch units
    pub height: f32,
    /// Page content (XAML)
    pub content: String,
}

impl XpsFixedPage {
    pub fn new(name: &str, number: i32) -> Self {
        Self {
            name: name.to_string(),
            number,
            width: 816.0,   // 8.5" at 96 DPI
            height: 1056.0, // 11" at 96 DPI
            content: String::new(),
        }
    }

    pub fn with_size(mut self, width: f32, height: f32) -> Self {
        self.width = width;
        self.height = height;
        self
    }
}

// ============================================================================
// XPS Fixed Document
// ============================================================================

/// An XPS fixed document
#[derive(Debug, Clone)]
pub struct XpsFixedDocument {
    /// Document name/path
    pub name: String,
    /// Outline/structure path
    pub outline: Option<String>,
    /// Pages in this document
    pub pages: Vec<XpsFixedPage>,
}

impl XpsFixedDocument {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            outline: None,
            pages: Vec::new(),
        }
    }

    pub fn add_page(&mut self, page: XpsFixedPage) {
        self.pages.push(page);
    }
}

// ============================================================================
// XPS Resource
// ============================================================================

/// An XPS resource entry
#[derive(Debug, Clone)]
pub struct XpsResource {
    /// Resource key
    pub key: String,
    /// Resource URI
    pub uri: String,
    /// Resource data (parsed XML or raw bytes)
    pub data: Vec<u8>,
}

impl XpsResource {
    pub fn new(key: &str, uri: &str) -> Self {
        Self {
            key: key.to_string(),
            uri: uri.to_string(),
            data: Vec::new(),
        }
    }
}

// ============================================================================
// XPS Font Cache Entry
// ============================================================================

/// Cached font entry
#[derive(Debug, Clone)]
pub struct XpsFontEntry {
    /// Font name/path
    pub name: String,
    /// Font data
    pub data: Vec<u8>,
    /// Is obfuscated
    pub obfuscated: bool,
}

// ============================================================================
// XPS Document
// ============================================================================

/// XPS document structure
pub struct XpsDocument {
    /// Context handle
    pub context: ContextHandle,
    /// Start part (fixed document sequence)
    pub start_part: String,
    /// Fixed documents
    pub documents: Vec<XpsFixedDocument>,
    /// All parts in the package
    pub parts: HashMap<String, XpsPart>,
    /// Font cache
    pub fonts: HashMap<String, XpsFontEntry>,
    /// Resource dictionaries
    pub resources: HashMap<String, Vec<XpsResource>>,
    /// Link targets
    pub targets: HashMap<String, i32>,
    /// Total page count
    pub page_count: i32,
}

impl XpsDocument {
    pub fn new(context: ContextHandle) -> Self {
        Self {
            context,
            start_part: String::new(),
            documents: Vec::new(),
            parts: HashMap::new(),
            fonts: HashMap::new(),
            resources: HashMap::new(),
            targets: HashMap::new(),
            page_count: 0,
        }
    }

    pub fn add_document(&mut self, doc: XpsFixedDocument) {
        self.page_count += doc.pages.len() as i32;
        self.documents.push(doc);
    }

    pub fn add_part(&mut self, part: XpsPart) {
        self.parts.insert(part.name.clone(), part);
    }

    pub fn get_part(&self, name: &str) -> Option<&XpsPart> {
        self.parts.get(name)
    }

    pub fn has_part(&self, name: &str) -> bool {
        self.parts.contains_key(name)
    }

    pub fn get_page(&self, index: i32) -> Option<&XpsFixedPage> {
        let mut current = 0;
        for doc in &self.documents {
            for page in &doc.pages {
                if current == index {
                    return Some(page);
                }
                current += 1;
            }
        }
        None
    }
}

// ============================================================================
// Global Handle Store
// ============================================================================

pub static XPS_DOCUMENTS: LazyLock<HandleStore<XpsDocument>> = LazyLock::new(HandleStore::new);

// ============================================================================
// FFI Functions - Document Management
// ============================================================================

/// Create a new XPS document.
#[unsafe(no_mangle)]
pub extern "C" fn xps_new_document(ctx: ContextHandle) -> Handle {
    let doc = XpsDocument::new(ctx);
    XPS_DOCUMENTS.insert(doc)
}

/// Drop an XPS document.
#[unsafe(no_mangle)]
pub extern "C" fn xps_drop_document(_ctx: ContextHandle, doc: Handle) {
    XPS_DOCUMENTS.remove(doc);
}

/// Open an XPS document from a file path.
#[unsafe(no_mangle)]
pub extern "C" fn xps_open_document(ctx: ContextHandle, filename: *const c_char) -> Handle {
    if filename.is_null() {
        return 0;
    }

    let _path = unsafe { CStr::from_ptr(filename).to_string_lossy() };

    // Create a new document (actual parsing would happen here)
    let doc = XpsDocument::new(ctx);
    XPS_DOCUMENTS.insert(doc)
}

/// Open an XPS document from a stream.
#[unsafe(no_mangle)]
pub extern "C" fn xps_open_document_with_stream(
    ctx: ContextHandle,
    _stream: StreamHandle,
) -> Handle {
    let doc = XpsDocument::new(ctx);
    XPS_DOCUMENTS.insert(doc)
}

/// Open an XPS document from an archive (directory).
#[unsafe(no_mangle)]
pub extern "C" fn xps_open_document_with_directory(
    ctx: ContextHandle,
    _archive: ArchiveHandle,
) -> Handle {
    let doc = XpsDocument::new(ctx);
    XPS_DOCUMENTS.insert(doc)
}

// ============================================================================
// FFI Functions - Page Access
// ============================================================================

/// Count pages in the document.
#[unsafe(no_mangle)]
pub extern "C" fn xps_count_pages(_ctx: ContextHandle, doc: Handle) -> i32 {
    if let Some(d) = XPS_DOCUMENTS.get(doc) {
        let d = d.lock().unwrap();
        return d.page_count;
    }
    0
}

/// Get page dimensions.
#[unsafe(no_mangle)]
pub extern "C" fn xps_get_page_size(
    _ctx: ContextHandle,
    doc: Handle,
    page_num: i32,
    width: *mut f32,
    height: *mut f32,
) -> i32 {
    if let Some(d) = XPS_DOCUMENTS.get(doc) {
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

/// Get page name/path.
#[unsafe(no_mangle)]
pub extern "C" fn xps_get_page_name(
    _ctx: ContextHandle,
    doc: Handle,
    page_num: i32,
) -> *mut c_char {
    if let Some(d) = XPS_DOCUMENTS.get(doc) {
        let d = d.lock().unwrap();
        if let Some(page) = d.get_page(page_num) {
            if let Ok(cstr) = CString::new(page.name.clone()) {
                return cstr.into_raw();
            }
        }
    }
    ptr::null_mut()
}

// ============================================================================
// FFI Functions - Document Structure
// ============================================================================

/// Count fixed documents.
#[unsafe(no_mangle)]
pub extern "C" fn xps_count_documents(_ctx: ContextHandle, doc: Handle) -> i32 {
    if let Some(d) = XPS_DOCUMENTS.get(doc) {
        let d = d.lock().unwrap();
        return d.documents.len() as i32;
    }
    0
}

/// Get document name.
#[unsafe(no_mangle)]
pub extern "C" fn xps_get_document_name(
    _ctx: ContextHandle,
    doc: Handle,
    doc_num: i32,
) -> *mut c_char {
    if let Some(d) = XPS_DOCUMENTS.get(doc) {
        let d = d.lock().unwrap();
        if let Some(fixed_doc) = d.documents.get(doc_num as usize) {
            if let Ok(cstr) = CString::new(fixed_doc.name.clone()) {
                return cstr.into_raw();
            }
        }
    }
    ptr::null_mut()
}

/// Count pages in a specific document.
#[unsafe(no_mangle)]
pub extern "C" fn xps_count_pages_in_document(
    _ctx: ContextHandle,
    doc: Handle,
    doc_num: i32,
) -> i32 {
    if let Some(d) = XPS_DOCUMENTS.get(doc) {
        let d = d.lock().unwrap();
        if let Some(fixed_doc) = d.documents.get(doc_num as usize) {
            return fixed_doc.pages.len() as i32;
        }
    }
    0
}

// ============================================================================
// FFI Functions - Part Access
// ============================================================================

/// Check if a part exists.
#[unsafe(no_mangle)]
pub extern "C" fn xps_has_part(_ctx: ContextHandle, doc: Handle, name: *const c_char) -> i32 {
    if name.is_null() {
        return 0;
    }

    if let Some(d) = XPS_DOCUMENTS.get(doc) {
        let d = d.lock().unwrap();
        let part_name = unsafe { CStr::from_ptr(name).to_string_lossy() };
        return if d.has_part(&part_name) { 1 } else { 0 };
    }
    0
}

/// Get part data.
#[unsafe(no_mangle)]
pub extern "C" fn xps_get_part_data(
    _ctx: ContextHandle,
    doc: Handle,
    name: *const c_char,
    len_out: *mut usize,
) -> *const u8 {
    if name.is_null() {
        return ptr::null();
    }

    if let Some(d) = XPS_DOCUMENTS.get(doc) {
        let d = d.lock().unwrap();
        let part_name = unsafe { CStr::from_ptr(name).to_string_lossy() };
        if let Some(part) = d.get_part(&part_name) {
            if !len_out.is_null() {
                unsafe {
                    *len_out = part.data.len();
                }
            }
            return part.data.as_ptr();
        }
    }

    if !len_out.is_null() {
        unsafe {
            *len_out = 0;
        }
    }
    ptr::null()
}

/// Get part content type.
#[unsafe(no_mangle)]
pub extern "C" fn xps_get_part_content_type(
    _ctx: ContextHandle,
    doc: Handle,
    name: *const c_char,
) -> *mut c_char {
    if name.is_null() {
        return ptr::null_mut();
    }

    if let Some(d) = XPS_DOCUMENTS.get(doc) {
        let d = d.lock().unwrap();
        let part_name = unsafe { CStr::from_ptr(name).to_string_lossy() };
        if let Some(part) = d.get_part(&part_name) {
            if let Ok(cstr) = CString::new(part.content_type.clone()) {
                return cstr.into_raw();
            }
        }
    }
    ptr::null_mut()
}

/// Add a part to the document.
#[unsafe(no_mangle)]
pub extern "C" fn xps_add_part(
    _ctx: ContextHandle,
    doc: Handle,
    name: *const c_char,
    data: *const u8,
    len: usize,
    content_type: *const c_char,
) -> i32 {
    if name.is_null() || data.is_null() {
        return 0;
    }

    if let Some(d) = XPS_DOCUMENTS.get(doc) {
        let mut d = d.lock().unwrap();

        let part_name = unsafe { CStr::from_ptr(name).to_string_lossy().to_string() };
        let part_data = unsafe { std::slice::from_raw_parts(data, len) };
        let ct = if !content_type.is_null() {
            unsafe { CStr::from_ptr(content_type).to_string_lossy().to_string() }
        } else {
            String::new()
        };

        let part = XpsPart::new(&part_name)
            .with_data(part_data)
            .with_content_type(&ct);
        d.add_part(part);
        return 1;
    }
    0
}

// ============================================================================
// FFI Functions - Font Cache
// ============================================================================

/// Lookup a cached font.
#[unsafe(no_mangle)]
pub extern "C" fn xps_lookup_font(_ctx: ContextHandle, doc: Handle, uri: *const c_char) -> i32 {
    if uri.is_null() {
        return 0;
    }

    if let Some(d) = XPS_DOCUMENTS.get(doc) {
        let d = d.lock().unwrap();
        let font_uri = unsafe { CStr::from_ptr(uri).to_string_lossy() };
        return if d.fonts.contains_key(font_uri.as_ref()) {
            1
        } else {
            0
        };
    }
    0
}

/// Get font count.
#[unsafe(no_mangle)]
pub extern "C" fn xps_font_count(_ctx: ContextHandle, doc: Handle) -> i32 {
    if let Some(d) = XPS_DOCUMENTS.get(doc) {
        let d = d.lock().unwrap();
        return d.fonts.len() as i32;
    }
    0
}

// ============================================================================
// FFI Functions - Link Targets
// ============================================================================

/// Add a link target.
#[unsafe(no_mangle)]
pub extern "C" fn xps_add_target(
    _ctx: ContextHandle,
    doc: Handle,
    name: *const c_char,
    page: i32,
) -> i32 {
    if name.is_null() {
        return 0;
    }

    if let Some(d) = XPS_DOCUMENTS.get(doc) {
        let mut d = d.lock().unwrap();
        let target_name = unsafe { CStr::from_ptr(name).to_string_lossy().to_string() };
        d.targets.insert(target_name, page);
        return 1;
    }
    0
}

/// Lookup a link target.
#[unsafe(no_mangle)]
pub extern "C" fn xps_lookup_target(_ctx: ContextHandle, doc: Handle, name: *const c_char) -> i32 {
    if name.is_null() {
        return -1;
    }

    if let Some(d) = XPS_DOCUMENTS.get(doc) {
        let d = d.lock().unwrap();
        let target_name = unsafe { CStr::from_ptr(name).to_string_lossy() };
        if let Some(&page) = d.targets.get(target_name.as_ref()) {
            return page;
        }
    }
    -1
}

// ============================================================================
// FFI Functions - Utility
// ============================================================================

/// Free a string returned by XPS functions.
#[unsafe(no_mangle)]
pub extern "C" fn xps_free_string(s: *mut c_char) {
    if !s.is_null() {
        unsafe {
            drop(CString::from_raw(s));
        }
    }
}

/// Resolve a relative URL.
#[unsafe(no_mangle)]
pub extern "C" fn xps_resolve_url(
    _ctx: ContextHandle,
    base_uri: *const c_char,
    path: *const c_char,
    output: *mut c_char,
    output_size: i32,
) -> i32 {
    if path.is_null() || output.is_null() || output_size <= 0 {
        return 0;
    }

    let base = if !base_uri.is_null() {
        unsafe { CStr::from_ptr(base_uri).to_string_lossy().to_string() }
    } else {
        String::new()
    };

    let rel_path = unsafe { CStr::from_ptr(path).to_string_lossy() };

    // Simple URL resolution
    let resolved = if rel_path.starts_with('/') {
        rel_path.to_string()
    } else if base.is_empty() {
        format!("/{}", rel_path)
    } else {
        let base_dir = base.rsplit_once('/').map(|(d, _)| d).unwrap_or("");
        format!("{}/{}", base_dir, rel_path)
    };

    let bytes = resolved.as_bytes();
    let copy_len = bytes.len().min((output_size - 1) as usize);

    unsafe {
        ptr::copy_nonoverlapping(bytes.as_ptr(), output as *mut u8, copy_len);
        *output.add(copy_len) = 0;
    }

    1
}

/// Get content type string.
#[unsafe(no_mangle)]
pub extern "C" fn xps_content_type_string(_ctx: ContextHandle, content_type: i32) -> *mut c_char {
    let s = match content_type {
        XPS_CONTENT_FIXED_DOC_SEQ => "application/vnd.ms-package.xps-fixeddocumentsequence+xml",
        XPS_CONTENT_FIXED_DOC => "application/vnd.ms-package.xps-fixeddocument+xml",
        XPS_CONTENT_FIXED_PAGE => "application/vnd.ms-package.xps-fixedpage+xml",
        XPS_CONTENT_FONT => "application/vnd.ms-opentype",
        XPS_CONTENT_IMAGE => "image/png",
        XPS_CONTENT_ICC_PROFILE => "application/vnd.ms-color.iccprofile",
        XPS_CONTENT_RESOURCE_DICT => "application/vnd.ms-package.xps-resourcedictionary+xml",
        XPS_CONTENT_PRINT_TICKET => "application/vnd.ms-printing.printticket+xml",
        XPS_CONTENT_THUMBNAIL => "image/png",
        _ => "application/octet-stream",
    };

    if let Ok(cstr) = CString::new(s) {
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
    fn test_content_type_constants() {
        assert_eq!(XPS_CONTENT_FIXED_DOC_SEQ, 0);
        assert_eq!(XPS_CONTENT_FIXED_PAGE, 2);
        assert_eq!(XPS_CONTENT_FONT, 3);
    }

    #[test]
    fn test_xps_part() {
        let part = XpsPart::new("/Documents/1/Pages/1.fpage")
            .with_data(b"<FixedPage/>")
            .with_content_type("application/vnd.ms-package.xps-fixedpage+xml");

        assert_eq!(part.name, "/Documents/1/Pages/1.fpage");
        assert_eq!(part.data, b"<FixedPage/>");
    }

    #[test]
    fn test_xps_fixed_page() {
        let page = XpsFixedPage::new("/Documents/1/Pages/1.fpage", 0).with_size(816.0, 1056.0);

        assert_eq!(page.number, 0);
        assert_eq!(page.width, 816.0);
        assert_eq!(page.height, 1056.0);
    }

    #[test]
    fn test_xps_fixed_document() {
        let mut doc = XpsFixedDocument::new("/Documents/1/FixedDocument.fdoc");
        doc.add_page(XpsFixedPage::new("/Documents/1/Pages/1.fpage", 0));
        doc.add_page(XpsFixedPage::new("/Documents/1/Pages/2.fpage", 1));

        assert_eq!(doc.pages.len(), 2);
    }

    #[test]
    fn test_xps_document() {
        let mut doc = XpsDocument::new(0);

        let mut fixed_doc = XpsFixedDocument::new("/Documents/1/FixedDocument.fdoc");
        fixed_doc.add_page(XpsFixedPage::new("/Documents/1/Pages/1.fpage", 0));
        fixed_doc.add_page(XpsFixedPage::new("/Documents/1/Pages/2.fpage", 1));
        doc.add_document(fixed_doc);

        assert_eq!(doc.page_count, 2);
        assert!(doc.get_page(0).is_some());
        assert!(doc.get_page(1).is_some());
        assert!(doc.get_page(2).is_none());
    }

    #[test]
    fn test_ffi_document() {
        let ctx = 0;

        let doc = xps_new_document(ctx);
        assert!(doc > 0);

        assert_eq!(xps_count_pages(ctx, doc), 0);
        assert_eq!(xps_count_documents(ctx, doc), 0);

        xps_drop_document(ctx, doc);
    }

    #[test]
    fn test_ffi_parts() {
        let ctx = 0;
        let doc = xps_new_document(ctx);

        let name = CString::new("/test.xml").unwrap();
        let data = b"<test/>";
        let ct = CString::new("application/xml").unwrap();

        assert_eq!(xps_has_part(ctx, doc, name.as_ptr()), 0);

        let result = xps_add_part(
            ctx,
            doc,
            name.as_ptr(),
            data.as_ptr(),
            data.len(),
            ct.as_ptr(),
        );
        assert_eq!(result, 1);

        assert_eq!(xps_has_part(ctx, doc, name.as_ptr()), 1);

        xps_drop_document(ctx, doc);
    }

    #[test]
    fn test_ffi_targets() {
        let ctx = 0;
        let doc = xps_new_document(ctx);

        let name = CString::new("section1").unwrap();

        assert_eq!(xps_lookup_target(ctx, doc, name.as_ptr()), -1);

        xps_add_target(ctx, doc, name.as_ptr(), 5);
        assert_eq!(xps_lookup_target(ctx, doc, name.as_ptr()), 5);

        xps_drop_document(ctx, doc);
    }

    #[test]
    fn test_ffi_content_type_string() {
        let ctx = 0;

        let s = xps_content_type_string(ctx, XPS_CONTENT_FIXED_PAGE);
        assert!(!s.is_null());
        unsafe {
            let str = CStr::from_ptr(s).to_string_lossy();
            assert!(str.contains("fixedpage"));
            xps_free_string(s);
        }
    }

    #[test]
    fn test_ffi_resolve_url() {
        let ctx = 0;
        let mut output = [0u8; 256];

        let base = CString::new("/Documents/1/Pages/1.fpage").unwrap();
        let path = CString::new("../Resources/image.png").unwrap();

        let result = xps_resolve_url(
            ctx,
            base.as_ptr(),
            path.as_ptr(),
            output.as_mut_ptr() as *mut c_char,
            256,
        );
        assert_eq!(result, 1);
    }
}
