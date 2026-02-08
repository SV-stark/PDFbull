//! PDF Portfolio/Collection FFI Module
//!
//! Provides support for PDF portfolios (packages/collections), including
//! embedded file management, collection structure, and navigator schema.

use crate::ffi::{Handle, HandleStore};
use std::collections::HashMap;
use std::ffi::{CStr, CString, c_char};
use std::ptr;
use std::sync::LazyLock;

// ============================================================================
// Type Aliases
// ============================================================================

type ContextHandle = Handle;
type DocumentHandle = Handle;
type BufferHandle = Handle;
type PdfObjHandle = Handle;

// ============================================================================
// AF Relationship Constants
// ============================================================================

/// Source document
pub const PDF_AF_RELATIONSHIP_SOURCE: i32 = 0;
/// Data for the document
pub const PDF_AF_RELATIONSHIP_DATA: i32 = 1;
/// Alternative representation
pub const PDF_AF_RELATIONSHIP_ALTERNATIVE: i32 = 2;
/// Supplement to the document
pub const PDF_AF_RELATIONSHIP_SUPPLEMENT: i32 = 3;
/// Encrypted payload
pub const PDF_AF_RELATIONSHIP_ENCRYPTED_PAYLOAD: i32 = 4;
/// Form data
pub const PDF_AF_RELATIONSHIP_FORM_DATA: i32 = 5;
/// Schema definition
pub const PDF_AF_RELATIONSHIP_SCHEMA: i32 = 6;
/// Unspecified relationship
pub const PDF_AF_RELATIONSHIP_UNSPECIFIED: i32 = 7;

// ============================================================================
// Collection Sort Constants
// ============================================================================

/// Sort by name
pub const PDF_COLLECTION_SORT_NAME: i32 = 0;
/// Sort by modification date
pub const PDF_COLLECTION_SORT_MODIFIED: i32 = 1;
/// Sort by creation date
pub const PDF_COLLECTION_SORT_CREATED: i32 = 2;
/// Sort by size
pub const PDF_COLLECTION_SORT_SIZE: i32 = 3;
/// Sort by description
pub const PDF_COLLECTION_SORT_DESCRIPTION: i32 = 4;

// ============================================================================
// Collection View Constants
// ============================================================================

/// Detail view
pub const PDF_COLLECTION_VIEW_DETAILS: i32 = 0;
/// Tile view
pub const PDF_COLLECTION_VIEW_TILE: i32 = 1;
/// Hidden view (document appears normally)
pub const PDF_COLLECTION_VIEW_HIDDEN: i32 = 2;
/// Custom navigator
pub const PDF_COLLECTION_VIEW_CUSTOM: i32 = 3;

// ============================================================================
// Filespec Parameters
// ============================================================================

/// Parameters for an embedded file
#[derive(Debug, Clone)]
pub struct FilespecParams {
    /// Filename
    pub filename: String,
    /// MIME type
    pub mime_type: String,
    /// File size
    pub size: i64,
    /// Creation date (Unix timestamp)
    pub created: i64,
    /// Modification date (Unix timestamp)
    pub modified: i64,
    /// Description
    pub description: String,
    /// AF relationship
    pub af_relationship: i32,
}

impl Default for FilespecParams {
    fn default() -> Self {
        Self::new()
    }
}

impl FilespecParams {
    pub fn new() -> Self {
        Self {
            filename: String::new(),
            mime_type: "application/octet-stream".to_string(),
            size: 0,
            created: 0,
            modified: 0,
            description: String::new(),
            af_relationship: PDF_AF_RELATIONSHIP_UNSPECIFIED,
        }
    }

    pub fn with_filename(mut self, name: &str) -> Self {
        self.filename = name.to_string();
        self
    }

    pub fn with_mime_type(mut self, mime: &str) -> Self {
        self.mime_type = mime.to_string();
        self
    }
}

// ============================================================================
// Embedded File Entry
// ============================================================================

/// An embedded file in the portfolio
#[derive(Debug, Clone)]
pub struct EmbeddedFile {
    /// File parameters
    pub params: FilespecParams,
    /// File contents
    pub contents: Vec<u8>,
    /// Checksum (MD5)
    pub checksum: Option<[u8; 16]>,
}

impl Default for EmbeddedFile {
    fn default() -> Self {
        Self::new()
    }
}

impl EmbeddedFile {
    pub fn new() -> Self {
        Self {
            params: FilespecParams::new(),
            contents: Vec::new(),
            checksum: None,
        }
    }

    pub fn with_contents(mut self, data: &[u8]) -> Self {
        self.contents = data.to_vec();
        self.params.size = data.len() as i64;
        self
    }

    pub fn with_filename(mut self, name: &str) -> Self {
        self.params.filename = name.to_string();
        self
    }
}

// ============================================================================
// Collection Schema Field
// ============================================================================

/// A field in the collection schema
#[derive(Debug, Clone)]
pub struct SchemaField {
    /// Field key
    pub key: String,
    /// Display name
    pub name: String,
    /// Field type (S=string, D=date, N=number, F=filename)
    pub field_type: char,
    /// Sort order (0=none, 1=ascending, 2=descending)
    pub order: i32,
    /// Is visible
    pub visible: bool,
    /// Is editable
    pub editable: bool,
}

impl Default for SchemaField {
    fn default() -> Self {
        Self::new()
    }
}

impl SchemaField {
    pub fn new() -> Self {
        Self {
            key: String::new(),
            name: String::new(),
            field_type: 'S',
            order: 0,
            visible: true,
            editable: false,
        }
    }

    pub fn string_field(key: &str, name: &str) -> Self {
        Self {
            key: key.to_string(),
            name: name.to_string(),
            field_type: 'S',
            order: 0,
            visible: true,
            editable: false,
        }
    }

    pub fn date_field(key: &str, name: &str) -> Self {
        Self {
            key: key.to_string(),
            name: name.to_string(),
            field_type: 'D',
            order: 0,
            visible: true,
            editable: false,
        }
    }

    pub fn number_field(key: &str, name: &str) -> Self {
        Self {
            key: key.to_string(),
            name: name.to_string(),
            field_type: 'N',
            order: 0,
            visible: true,
            editable: false,
        }
    }
}

// ============================================================================
// Portfolio Context
// ============================================================================

/// Portfolio management context
pub struct Portfolio {
    /// Document handle
    pub document: DocumentHandle,
    /// Embedded files by name
    pub files: HashMap<String, EmbeddedFile>,
    /// Collection schema fields
    pub schema: Vec<SchemaField>,
    /// Initial view mode
    pub view: i32,
    /// Initial document (cover sheet)
    pub initial_document: Option<String>,
    /// Sort field
    pub sort_field: Option<String>,
    /// Sort ascending
    pub sort_ascending: bool,
}

impl Portfolio {
    pub fn new(document: DocumentHandle) -> Self {
        Self {
            document,
            files: HashMap::new(),
            schema: Vec::new(),
            view: PDF_COLLECTION_VIEW_DETAILS,
            initial_document: None,
            sort_field: None,
            sort_ascending: true,
        }
    }

    pub fn add_file(&mut self, name: &str, file: EmbeddedFile) {
        self.files.insert(name.to_string(), file);
    }

    pub fn remove_file(&mut self, name: &str) -> Option<EmbeddedFile> {
        self.files.remove(name)
    }

    pub fn get_file(&self, name: &str) -> Option<&EmbeddedFile> {
        self.files.get(name)
    }

    pub fn file_count(&self) -> usize {
        self.files.len()
    }
}

// ============================================================================
// Global Handle Store
// ============================================================================

pub static PORTFOLIOS: LazyLock<HandleStore<Portfolio>> = LazyLock::new(HandleStore::new);

// ============================================================================
// FFI Functions - Portfolio Management
// ============================================================================

/// Create a new portfolio context for a document.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_new_portfolio(_ctx: ContextHandle, doc: DocumentHandle) -> Handle {
    let portfolio = Portfolio::new(doc);
    PORTFOLIOS.insert(portfolio)
}

/// Drop a portfolio context.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_drop_portfolio(_ctx: ContextHandle, portfolio: Handle) {
    PORTFOLIOS.remove(portfolio);
}

/// Check if a document is a portfolio.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_is_portfolio(_ctx: ContextHandle, portfolio: Handle) -> i32 {
    if let Some(p) = PORTFOLIOS.get(portfolio) {
        let p = p.lock().unwrap();
        if !p.files.is_empty() || !p.schema.is_empty() {
            return 1;
        }
    }
    0
}

// ============================================================================
// FFI Functions - Embedded File Management
// ============================================================================

/// Add an embedded file to the portfolio.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_portfolio_add_file(
    _ctx: ContextHandle,
    portfolio: Handle,
    name: *const c_char,
    data: *const u8,
    len: usize,
    mime_type: *const c_char,
) -> i32 {
    if name.is_null() || data.is_null() || len == 0 {
        return 0;
    }

    if let Some(p) = PORTFOLIOS.get(portfolio) {
        let mut p = p.lock().unwrap();

        let filename = unsafe { CStr::from_ptr(name).to_string_lossy().to_string() };

        let mime = if !mime_type.is_null() {
            unsafe { CStr::from_ptr(mime_type).to_string_lossy().to_string() }
        } else {
            "application/octet-stream".to_string()
        };

        let contents = unsafe { std::slice::from_raw_parts(data, len) };

        let file = EmbeddedFile::new()
            .with_filename(&filename)
            .with_contents(contents);

        let mut file = file;
        file.params.mime_type = mime;

        p.add_file(&filename, file);
        return 1;
    }
    0
}

/// Get the number of embedded files.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_portfolio_count(_ctx: ContextHandle, portfolio: Handle) -> i32 {
    if let Some(p) = PORTFOLIOS.get(portfolio) {
        let p = p.lock().unwrap();
        return p.file_count() as i32;
    }
    0
}

/// Get the name of an embedded file by index.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_portfolio_get_name(
    _ctx: ContextHandle,
    portfolio: Handle,
    index: i32,
) -> *mut c_char {
    if let Some(p) = PORTFOLIOS.get(portfolio) {
        let p = p.lock().unwrap();
        if let Some(name) = p.files.keys().nth(index as usize) {
            if let Ok(cstr) = CString::new(name.clone()) {
                return cstr.into_raw();
            }
        }
    }
    ptr::null_mut()
}

/// Get embedded file contents.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_portfolio_get_file(
    _ctx: ContextHandle,
    portfolio: Handle,
    name: *const c_char,
    len_out: *mut usize,
) -> *const u8 {
    if name.is_null() {
        return ptr::null();
    }

    if let Some(p) = PORTFOLIOS.get(portfolio) {
        let p = p.lock().unwrap();
        let filename = unsafe { CStr::from_ptr(name).to_string_lossy() };

        if let Some(file) = p.get_file(&filename) {
            if !len_out.is_null() {
                unsafe {
                    *len_out = file.contents.len();
                }
            }
            return file.contents.as_ptr();
        }
    }

    if !len_out.is_null() {
        unsafe {
            *len_out = 0;
        }
    }
    ptr::null()
}

/// Remove an embedded file.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_portfolio_remove_file(
    _ctx: ContextHandle,
    portfolio: Handle,
    name: *const c_char,
) -> i32 {
    if name.is_null() {
        return 0;
    }

    if let Some(p) = PORTFOLIOS.get(portfolio) {
        let mut p = p.lock().unwrap();
        let filename = unsafe { CStr::from_ptr(name).to_string_lossy() };

        if p.remove_file(&filename).is_some() {
            return 1;
        }
    }
    0
}

// ============================================================================
// FFI Functions - File Parameters
// ============================================================================

/// Get file size.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_portfolio_get_file_size(
    _ctx: ContextHandle,
    portfolio: Handle,
    name: *const c_char,
) -> i64 {
    if name.is_null() {
        return -1;
    }

    if let Some(p) = PORTFOLIOS.get(portfolio) {
        let p = p.lock().unwrap();
        let filename = unsafe { CStr::from_ptr(name).to_string_lossy() };

        if let Some(file) = p.get_file(&filename) {
            return file.params.size;
        }
    }
    -1
}

/// Get file MIME type.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_portfolio_get_mime_type(
    _ctx: ContextHandle,
    portfolio: Handle,
    name: *const c_char,
) -> *mut c_char {
    if name.is_null() {
        return ptr::null_mut();
    }

    if let Some(p) = PORTFOLIOS.get(portfolio) {
        let p = p.lock().unwrap();
        let filename = unsafe { CStr::from_ptr(name).to_string_lossy() };

        if let Some(file) = p.get_file(&filename) {
            if let Ok(cstr) = CString::new(file.params.mime_type.clone()) {
                return cstr.into_raw();
            }
        }
    }
    ptr::null_mut()
}

/// Set file description.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_portfolio_set_description(
    _ctx: ContextHandle,
    portfolio: Handle,
    name: *const c_char,
    description: *const c_char,
) -> i32 {
    if name.is_null() {
        return 0;
    }

    if let Some(p) = PORTFOLIOS.get(portfolio) {
        let mut p = p.lock().unwrap();
        let filename = unsafe { CStr::from_ptr(name).to_string_lossy().to_string() };

        if let Some(file) = p.files.get_mut(&filename) {
            file.params.description = if !description.is_null() {
                unsafe { CStr::from_ptr(description).to_string_lossy().to_string() }
            } else {
                String::new()
            };
            return 1;
        }
    }
    0
}

// ============================================================================
// FFI Functions - Collection Schema
// ============================================================================

/// Add a schema field.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_portfolio_add_schema_field(
    _ctx: ContextHandle,
    portfolio: Handle,
    key: *const c_char,
    name: *const c_char,
    field_type: c_char,
) -> i32 {
    if key.is_null() || name.is_null() {
        return 0;
    }

    if let Some(p) = PORTFOLIOS.get(portfolio) {
        let mut p = p.lock().unwrap();

        let k = unsafe { CStr::from_ptr(key).to_string_lossy().to_string() };
        let n = unsafe { CStr::from_ptr(name).to_string_lossy().to_string() };

        let field = SchemaField {
            key: k,
            name: n,
            field_type: field_type as u8 as char,
            order: 0,
            visible: true,
            editable: false,
        };

        p.schema.push(field);
        return 1;
    }
    0
}

/// Get schema field count.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_portfolio_schema_field_count(_ctx: ContextHandle, portfolio: Handle) -> i32 {
    if let Some(p) = PORTFOLIOS.get(portfolio) {
        let p = p.lock().unwrap();
        return p.schema.len() as i32;
    }
    0
}

/// Get schema field key by index.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_portfolio_schema_field_key(
    _ctx: ContextHandle,
    portfolio: Handle,
    index: i32,
) -> *mut c_char {
    if let Some(p) = PORTFOLIOS.get(portfolio) {
        let p = p.lock().unwrap();
        if let Some(field) = p.schema.get(index as usize) {
            if let Ok(cstr) = CString::new(field.key.clone()) {
                return cstr.into_raw();
            }
        }
    }
    ptr::null_mut()
}

// ============================================================================
// FFI Functions - Collection Settings
// ============================================================================

/// Set the initial view mode.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_portfolio_set_view(_ctx: ContextHandle, portfolio: Handle, view: i32) -> i32 {
    if let Some(p) = PORTFOLIOS.get(portfolio) {
        let mut p = p.lock().unwrap();
        p.view = view;
        return 1;
    }
    0
}

/// Get the initial view mode.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_portfolio_get_view(_ctx: ContextHandle, portfolio: Handle) -> i32 {
    if let Some(p) = PORTFOLIOS.get(portfolio) {
        let p = p.lock().unwrap();
        return p.view;
    }
    PDF_COLLECTION_VIEW_DETAILS
}

/// Set the initial document (cover sheet).
#[unsafe(no_mangle)]
pub extern "C" fn pdf_portfolio_set_initial_document(
    _ctx: ContextHandle,
    portfolio: Handle,
    name: *const c_char,
) -> i32 {
    if let Some(p) = PORTFOLIOS.get(portfolio) {
        let mut p = p.lock().unwrap();
        p.initial_document = if !name.is_null() {
            Some(unsafe { CStr::from_ptr(name).to_string_lossy().to_string() })
        } else {
            None
        };
        return 1;
    }
    0
}

/// Set sort order.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_portfolio_set_sort(
    _ctx: ContextHandle,
    portfolio: Handle,
    field: *const c_char,
    ascending: i32,
) -> i32 {
    if let Some(p) = PORTFOLIOS.get(portfolio) {
        let mut p = p.lock().unwrap();
        p.sort_field = if !field.is_null() {
            Some(unsafe { CStr::from_ptr(field).to_string_lossy().to_string() })
        } else {
            None
        };
        p.sort_ascending = ascending != 0;
        return 1;
    }
    0
}

// ============================================================================
// FFI Functions - Utility
// ============================================================================

/// Free a string returned by portfolio functions.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_portfolio_free_string(s: *mut c_char) {
    if !s.is_null() {
        unsafe {
            drop(CString::from_raw(s));
        }
    }
}

/// Get AF relationship string.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_af_relationship_to_string(
    _ctx: ContextHandle,
    relationship: i32,
) -> *mut c_char {
    let s = match relationship {
        PDF_AF_RELATIONSHIP_SOURCE => "Source",
        PDF_AF_RELATIONSHIP_DATA => "Data",
        PDF_AF_RELATIONSHIP_ALTERNATIVE => "Alternative",
        PDF_AF_RELATIONSHIP_SUPPLEMENT => "Supplement",
        PDF_AF_RELATIONSHIP_ENCRYPTED_PAYLOAD => "EncryptedPayload",
        PDF_AF_RELATIONSHIP_FORM_DATA => "FormData",
        PDF_AF_RELATIONSHIP_SCHEMA => "Schema",
        PDF_AF_RELATIONSHIP_UNSPECIFIED => "Unspecified",
        _ => "Unknown",
    };

    if let Ok(cstr) = CString::new(s) {
        return cstr.into_raw();
    }
    ptr::null_mut()
}

/// Get view mode string.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_collection_view_to_string(_ctx: ContextHandle, view: i32) -> *mut c_char {
    let s = match view {
        PDF_COLLECTION_VIEW_DETAILS => "D",
        PDF_COLLECTION_VIEW_TILE => "T",
        PDF_COLLECTION_VIEW_HIDDEN => "H",
        PDF_COLLECTION_VIEW_CUSTOM => "C",
        _ => "D",
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
    fn test_af_relationship_constants() {
        assert_eq!(PDF_AF_RELATIONSHIP_SOURCE, 0);
        assert_eq!(PDF_AF_RELATIONSHIP_DATA, 1);
        assert_eq!(PDF_AF_RELATIONSHIP_ALTERNATIVE, 2);
        assert_eq!(PDF_AF_RELATIONSHIP_SUPPLEMENT, 3);
        assert_eq!(PDF_AF_RELATIONSHIP_UNSPECIFIED, 7);
    }

    #[test]
    fn test_collection_view_constants() {
        assert_eq!(PDF_COLLECTION_VIEW_DETAILS, 0);
        assert_eq!(PDF_COLLECTION_VIEW_TILE, 1);
        assert_eq!(PDF_COLLECTION_VIEW_HIDDEN, 2);
        assert_eq!(PDF_COLLECTION_VIEW_CUSTOM, 3);
    }

    #[test]
    fn test_filespec_params() {
        let params = FilespecParams::new()
            .with_filename("test.pdf")
            .with_mime_type("application/pdf");

        assert_eq!(params.filename, "test.pdf");
        assert_eq!(params.mime_type, "application/pdf");
    }

    #[test]
    fn test_embedded_file() {
        let file = EmbeddedFile::new()
            .with_filename("data.txt")
            .with_contents(b"Hello, World!");

        assert_eq!(file.params.filename, "data.txt");
        assert_eq!(file.contents, b"Hello, World!");
        assert_eq!(file.params.size, 13);
    }

    #[test]
    fn test_schema_field() {
        let field = SchemaField::string_field("name", "Name");
        assert_eq!(field.key, "name");
        assert_eq!(field.name, "Name");
        assert_eq!(field.field_type, 'S');

        let field = SchemaField::date_field("modified", "Modified");
        assert_eq!(field.field_type, 'D');

        let field = SchemaField::number_field("size", "Size");
        assert_eq!(field.field_type, 'N');
    }

    #[test]
    fn test_portfolio() {
        let mut portfolio = Portfolio::new(1);
        assert_eq!(portfolio.file_count(), 0);

        let file = EmbeddedFile::new()
            .with_filename("test.txt")
            .with_contents(b"test");
        portfolio.add_file("test.txt", file);
        assert_eq!(portfolio.file_count(), 1);

        assert!(portfolio.get_file("test.txt").is_some());
        assert!(portfolio.get_file("missing.txt").is_none());

        portfolio.remove_file("test.txt");
        assert_eq!(portfolio.file_count(), 0);
    }

    #[test]
    fn test_ffi_portfolio() {
        let ctx = 0;
        let doc = 1;

        let portfolio = pdf_new_portfolio(ctx, doc);
        assert!(portfolio > 0);

        assert_eq!(pdf_portfolio_count(ctx, portfolio), 0);
        assert_eq!(pdf_is_portfolio(ctx, portfolio), 0);

        pdf_drop_portfolio(ctx, portfolio);
    }

    #[test]
    fn test_ffi_add_file() {
        let ctx = 0;
        let doc = 1;

        let portfolio = pdf_new_portfolio(ctx, doc);

        let name = CString::new("test.txt").unwrap();
        let data = b"Hello!";
        let mime = CString::new("text/plain").unwrap();

        let result = pdf_portfolio_add_file(
            ctx,
            portfolio,
            name.as_ptr(),
            data.as_ptr(),
            data.len(),
            mime.as_ptr(),
        );
        assert_eq!(result, 1);
        assert_eq!(pdf_portfolio_count(ctx, portfolio), 1);

        // Now it should be detected as a portfolio
        assert_eq!(pdf_is_portfolio(ctx, portfolio), 1);

        pdf_drop_portfolio(ctx, portfolio);
    }

    #[test]
    fn test_ffi_get_file() {
        let ctx = 0;
        let doc = 1;

        let portfolio = pdf_new_portfolio(ctx, doc);

        let name = CString::new("data.bin").unwrap();
        let data = b"\x00\x01\x02\x03";

        pdf_portfolio_add_file(
            ctx,
            portfolio,
            name.as_ptr(),
            data.as_ptr(),
            data.len(),
            ptr::null(),
        );

        let mut len: usize = 0;
        let ptr = pdf_portfolio_get_file(ctx, portfolio, name.as_ptr(), &mut len);
        assert!(!ptr.is_null());
        assert_eq!(len, 4);

        pdf_drop_portfolio(ctx, portfolio);
    }

    #[test]
    fn test_ffi_schema() {
        let ctx = 0;
        let doc = 1;

        let portfolio = pdf_new_portfolio(ctx, doc);

        let key = CString::new("name").unwrap();
        let name = CString::new("File Name").unwrap();

        let result = pdf_portfolio_add_schema_field(
            ctx,
            portfolio,
            key.as_ptr(),
            name.as_ptr(),
            b'S' as c_char,
        );
        assert_eq!(result, 1);
        assert_eq!(pdf_portfolio_schema_field_count(ctx, portfolio), 1);

        pdf_drop_portfolio(ctx, portfolio);
    }

    #[test]
    fn test_ffi_view_settings() {
        let ctx = 0;
        let doc = 1;

        let portfolio = pdf_new_portfolio(ctx, doc);

        pdf_portfolio_set_view(ctx, portfolio, PDF_COLLECTION_VIEW_TILE);
        assert_eq!(
            pdf_portfolio_get_view(ctx, portfolio),
            PDF_COLLECTION_VIEW_TILE
        );

        pdf_drop_portfolio(ctx, portfolio);
    }

    #[test]
    fn test_ffi_af_relationship_string() {
        let ctx = 0;

        let s = pdf_af_relationship_to_string(ctx, PDF_AF_RELATIONSHIP_ALTERNATIVE);
        assert!(!s.is_null());
        unsafe {
            let str = CStr::from_ptr(s).to_string_lossy();
            assert_eq!(str, "Alternative");
            pdf_portfolio_free_string(s);
        }
    }

    #[test]
    fn test_ffi_view_string() {
        let ctx = 0;

        let s = pdf_collection_view_to_string(ctx, PDF_COLLECTION_VIEW_DETAILS);
        assert!(!s.is_null());
        unsafe {
            let str = CStr::from_ptr(s).to_string_lossy();
            assert_eq!(str, "D");
            pdf_portfolio_free_string(s);
        }
    }
}
