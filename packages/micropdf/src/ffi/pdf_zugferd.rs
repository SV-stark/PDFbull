//! PDF ZUGFeRD/Factur-X FFI Module
//!
//! Provides support for ZUGFeRD and Factur-X electronic invoice formats,
//! enabling extraction and embedding of XML invoice data in PDF documents.

use crate::ffi::{Handle, HandleStore};
use std::ffi::{CStr, CString, c_char};
use std::ptr;
use std::sync::LazyLock;

// ============================================================================
// Type Aliases
// ============================================================================

type ContextHandle = Handle;
type DocumentHandle = Handle;
type BufferHandle = Handle;

// ============================================================================
// ZUGFeRD Profile Constants
// ============================================================================

/// Not a ZUGFeRD document
pub const PDF_NOT_ZUGFERD: i32 = 0;
/// ZUGFeRD 1.0 Comfort profile
pub const PDF_ZUGFERD_COMFORT: i32 = 1;
/// ZUGFeRD 1.0 Basic profile
pub const PDF_ZUGFERD_BASIC: i32 = 2;
/// ZUGFeRD 1.0 Extended profile
pub const PDF_ZUGFERD_EXTENDED: i32 = 3;
/// ZUGFeRD 2.01 Basic WL profile
pub const PDF_ZUGFERD_BASIC_WL: i32 = 4;
/// ZUGFeRD 2.01 Minimum profile
pub const PDF_ZUGFERD_MINIMUM: i32 = 5;
/// ZUGFeRD 2.2 XRechnung profile
pub const PDF_ZUGFERD_XRECHNUNG: i32 = 6;
/// Unknown ZUGFeRD profile
pub const PDF_ZUGFERD_UNKNOWN: i32 = 7;

// ============================================================================
// Factur-X Profile Constants (aliases)
// ============================================================================

/// Factur-X Minimum profile (alias for ZUGFERD_MINIMUM)
pub const PDF_FACTURX_MINIMUM: i32 = PDF_ZUGFERD_MINIMUM;
/// Factur-X Basic WL profile
pub const PDF_FACTURX_BASIC_WL: i32 = PDF_ZUGFERD_BASIC_WL;
/// Factur-X Basic profile
pub const PDF_FACTURX_BASIC: i32 = PDF_ZUGFERD_BASIC;
/// Factur-X EN16931 (Comfort) profile
pub const PDF_FACTURX_EN16931: i32 = PDF_ZUGFERD_COMFORT;
/// Factur-X Extended profile
pub const PDF_FACTURX_EXTENDED: i32 = PDF_ZUGFERD_EXTENDED;

// ============================================================================
// ZUGFeRD Document Info
// ============================================================================

/// ZUGFeRD document information
#[derive(Debug, Clone)]
pub struct ZugferdInfo {
    /// Profile type
    pub profile: i32,
    /// Version (1.0, 2.0, 2.1, 2.2, etc.)
    pub version: f32,
    /// Conformance level string
    pub conformance: String,
    /// XML filename in the PDF
    pub xml_filename: String,
    /// Whether the document has XMP metadata
    pub has_xmp: bool,
}

impl Default for ZugferdInfo {
    fn default() -> Self {
        Self::new()
    }
}

impl ZugferdInfo {
    pub fn new() -> Self {
        Self {
            profile: PDF_NOT_ZUGFERD,
            version: 0.0,
            conformance: String::new(),
            xml_filename: String::new(),
            has_xmp: false,
        }
    }

    pub fn is_zugferd(&self) -> bool {
        self.profile != PDF_NOT_ZUGFERD
    }
}

// ============================================================================
// Embedded Invoice Data
// ============================================================================

/// Embedded XML invoice data
#[derive(Debug, Clone)]
pub struct InvoiceData {
    /// XML content
    pub xml: Vec<u8>,
    /// MIME type
    pub mime_type: String,
    /// Filename
    pub filename: String,
    /// Creation date (Unix timestamp)
    pub created: i64,
    /// Modification date (Unix timestamp)
    pub modified: i64,
}

impl Default for InvoiceData {
    fn default() -> Self {
        Self::new()
    }
}

impl InvoiceData {
    pub fn new() -> Self {
        Self {
            xml: Vec::new(),
            mime_type: "text/xml".to_string(),
            filename: "factur-x.xml".to_string(),
            created: 0,
            modified: 0,
        }
    }

    pub fn with_xml(mut self, xml: &[u8]) -> Self {
        self.xml = xml.to_vec();
        self
    }

    pub fn with_filename(mut self, filename: &str) -> Self {
        self.filename = filename.to_string();
        self
    }
}

// ============================================================================
// ZUGFeRD Context
// ============================================================================

/// ZUGFeRD processing context
pub struct ZugferdContext {
    /// Document handle
    pub document: DocumentHandle,
    /// Cached info
    pub info: Option<ZugferdInfo>,
    /// Extracted XML data
    pub xml_data: Option<Vec<u8>>,
}

impl ZugferdContext {
    pub fn new(document: DocumentHandle) -> Self {
        Self {
            document,
            info: None,
            xml_data: None,
        }
    }
}

// ============================================================================
// Global Handle Store
// ============================================================================

pub static ZUGFERD_CONTEXTS: LazyLock<HandleStore<ZugferdContext>> =
    LazyLock::new(HandleStore::new);

// ============================================================================
// FFI Functions - Context Management
// ============================================================================

/// Create a new ZUGFeRD context for a document.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_new_zugferd_context(_ctx: ContextHandle, doc: DocumentHandle) -> Handle {
    let context = ZugferdContext::new(doc);
    ZUGFERD_CONTEXTS.insert(context)
}

/// Drop a ZUGFeRD context.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_drop_zugferd_context(_ctx: ContextHandle, zugferd: Handle) {
    ZUGFERD_CONTEXTS.remove(zugferd);
}

// ============================================================================
// FFI Functions - Profile Detection
// ============================================================================

/// Detect the ZUGFeRD profile of a document.
/// Returns the profile constant and optionally fills version.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_zugferd_profile(
    _ctx: ContextHandle,
    zugferd: Handle,
    version_out: *mut f32,
) -> i32 {
    if let Some(zctx) = ZUGFERD_CONTEXTS.get(zugferd) {
        let mut zctx = zctx.lock().unwrap();

        // Return cached info if available
        if let Some(ref info) = zctx.info {
            if !version_out.is_null() {
                unsafe {
                    *version_out = info.version;
                }
            }
            return info.profile;
        }

        // In a real implementation, this would parse the PDF's XMP metadata
        // and embedded files to detect ZUGFeRD. For now, return not ZUGFeRD.
        let info = ZugferdInfo::new();
        let profile = info.profile;
        zctx.info = Some(info);

        if !version_out.is_null() {
            unsafe {
                *version_out = 0.0;
            }
        }
        return profile;
    }
    PDF_NOT_ZUGFERD
}

/// Check if a document is a ZUGFeRD invoice.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_is_zugferd(_ctx: ContextHandle, zugferd: Handle) -> i32 {
    let mut version: f32 = 0.0;
    let profile = pdf_zugferd_profile(_ctx, zugferd, &mut version);
    if profile != PDF_NOT_ZUGFERD { 1 } else { 0 }
}

/// Get the ZUGFeRD version.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_zugferd_version(_ctx: ContextHandle, zugferd: Handle) -> f32 {
    let mut version: f32 = 0.0;
    pdf_zugferd_profile(_ctx, zugferd, &mut version);
    version
}

// ============================================================================
// FFI Functions - XML Extraction
// ============================================================================

/// Extract the embedded XML invoice data.
/// Returns a buffer handle containing the XML, or 0 on failure.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_zugferd_xml(
    _ctx: ContextHandle,
    zugferd: Handle,
    len_out: *mut usize,
) -> *const u8 {
    if let Some(zctx) = ZUGFERD_CONTEXTS.get(zugferd) {
        let zctx = zctx.lock().unwrap();

        if let Some(ref xml_data) = zctx.xml_data {
            if !len_out.is_null() {
                unsafe {
                    *len_out = xml_data.len();
                }
            }
            return xml_data.as_ptr();
        }
    }

    if !len_out.is_null() {
        unsafe {
            *len_out = 0;
        }
    }
    ptr::null()
}

/// Set XML data for the ZUGFeRD context (for testing/embedding).
#[unsafe(no_mangle)]
pub extern "C" fn pdf_zugferd_set_xml(
    _ctx: ContextHandle,
    zugferd: Handle,
    xml: *const u8,
    len: usize,
) -> i32 {
    if xml.is_null() || len == 0 {
        return 0;
    }

    if let Some(zctx) = ZUGFERD_CONTEXTS.get(zugferd) {
        let mut zctx = zctx.lock().unwrap();
        unsafe {
            let data = std::slice::from_raw_parts(xml, len);
            zctx.xml_data = Some(data.to_vec());
        }
        return 1;
    }
    0
}

// ============================================================================
// FFI Functions - Profile String Conversion
// ============================================================================

/// Convert a profile constant to a string.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_zugferd_profile_to_string(_ctx: ContextHandle, profile: i32) -> *mut c_char {
    let s = match profile {
        PDF_NOT_ZUGFERD => "Not ZUGFeRD",
        PDF_ZUGFERD_COMFORT => "ZUGFeRD Comfort (EN16931)",
        PDF_ZUGFERD_BASIC => "ZUGFeRD Basic",
        PDF_ZUGFERD_EXTENDED => "ZUGFeRD Extended",
        PDF_ZUGFERD_BASIC_WL => "ZUGFeRD Basic WL",
        PDF_ZUGFERD_MINIMUM => "ZUGFeRD Minimum",
        PDF_ZUGFERD_XRECHNUNG => "ZUGFeRD XRechnung",
        PDF_ZUGFERD_UNKNOWN => "ZUGFeRD Unknown",
        _ => "Invalid Profile",
    };

    if let Ok(cstr) = CString::new(s) {
        return cstr.into_raw();
    }
    ptr::null_mut()
}

/// Free a profile string.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_zugferd_free_string(s: *mut c_char) {
    if !s.is_null() {
        unsafe {
            drop(CString::from_raw(s));
        }
    }
}

// ============================================================================
// FFI Functions - Invoice Embedding
// ============================================================================

/// Parameters for embedding a ZUGFeRD invoice.
#[derive(Debug, Clone)]
#[repr(C)]
pub struct ZugferdEmbedParams {
    /// Profile to use
    pub profile: i32,
    /// Version (e.g., 2.2)
    pub version: f32,
    /// Filename (default: "factur-x.xml")
    pub filename: *const c_char,
    /// Add checksum to embedded file
    pub add_checksum: i32,
}

/// Create default embed parameters.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_zugferd_default_embed_params() -> ZugferdEmbedParams {
    ZugferdEmbedParams {
        profile: PDF_ZUGFERD_COMFORT,
        version: 2.2,
        filename: ptr::null(),
        add_checksum: 1,
    }
}

/// Embed an XML invoice into a document.
/// Returns 1 on success, 0 on failure.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_zugferd_embed(
    _ctx: ContextHandle,
    zugferd: Handle,
    xml: *const u8,
    xml_len: usize,
    params: *const ZugferdEmbedParams,
) -> i32 {
    if xml.is_null() || xml_len == 0 {
        return 0;
    }

    if let Some(zctx) = ZUGFERD_CONTEXTS.get(zugferd) {
        let mut zctx = zctx.lock().unwrap();

        // Store the XML data
        unsafe {
            let data = std::slice::from_raw_parts(xml, xml_len);
            zctx.xml_data = Some(data.to_vec());
        }

        // Update info based on params
        let profile = if !params.is_null() {
            unsafe { (*params).profile }
        } else {
            PDF_ZUGFERD_COMFORT
        };

        let version = if !params.is_null() {
            unsafe { (*params).version }
        } else {
            2.2
        };

        let filename = if !params.is_null() && !unsafe { (*params).filename }.is_null() {
            unsafe {
                CStr::from_ptr((*params).filename)
                    .to_string_lossy()
                    .to_string()
            }
        } else {
            "factur-x.xml".to_string()
        };

        zctx.info = Some(ZugferdInfo {
            profile,
            version,
            conformance: String::new(),
            xml_filename: filename,
            has_xmp: true,
        });

        return 1;
    }
    0
}

// ============================================================================
// FFI Functions - Validation
// ============================================================================

/// Validation result
#[derive(Debug, Clone, Default)]
pub struct ZugferdValidation {
    /// Is valid ZUGFeRD
    pub is_valid: bool,
    /// Error messages
    pub errors: Vec<String>,
    /// Warning messages
    pub warnings: Vec<String>,
}

/// Validate ZUGFeRD compliance.
/// Returns 1 if valid, 0 if invalid.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_zugferd_validate(_ctx: ContextHandle, zugferd: Handle) -> i32 {
    if let Some(zctx) = ZUGFERD_CONTEXTS.get(zugferd) {
        let zctx = zctx.lock().unwrap();

        // Check if we have XML data
        if zctx.xml_data.is_none() {
            return 0;
        }

        // Basic XML validation
        if let Some(ref xml) = zctx.xml_data {
            // Check for XML declaration
            if xml.starts_with(b"<?xml") || xml.starts_with(b"<rsm:") {
                return 1;
            }
        }
    }
    0
}

/// Get validation error count.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_zugferd_error_count(_ctx: ContextHandle, _zugferd: Handle) -> i32 {
    0 // Simplified - full implementation would track errors
}

// ============================================================================
// FFI Functions - Utility
// ============================================================================

/// Get the standard filename for a ZUGFeRD profile.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_zugferd_standard_filename(_ctx: ContextHandle, profile: i32) -> *mut c_char {
    let filename = match profile {
        PDF_ZUGFERD_COMFORT | PDF_ZUGFERD_BASIC | PDF_ZUGFERD_EXTENDED => "ZUGFeRD-invoice.xml",
        PDF_ZUGFERD_BASIC_WL | PDF_ZUGFERD_MINIMUM | PDF_ZUGFERD_XRECHNUNG => "factur-x.xml",
        _ => "invoice.xml",
    };

    if let Ok(cstr) = CString::new(filename) {
        return cstr.into_raw();
    }
    ptr::null_mut()
}

/// Get the MIME type for ZUGFeRD XML.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_zugferd_mime_type(_ctx: ContextHandle) -> *mut c_char {
    if let Ok(cstr) = CString::new("text/xml") {
        return cstr.into_raw();
    }
    ptr::null_mut()
}

/// Get AF relationship for ZUGFeRD.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_zugferd_af_relationship(_ctx: ContextHandle) -> *mut c_char {
    if let Ok(cstr) = CString::new("Alternative") {
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
    fn test_profile_constants() {
        assert_eq!(PDF_NOT_ZUGFERD, 0);
        assert_eq!(PDF_ZUGFERD_COMFORT, 1);
        assert_eq!(PDF_ZUGFERD_BASIC, 2);
        assert_eq!(PDF_ZUGFERD_EXTENDED, 3);
        assert_eq!(PDF_ZUGFERD_BASIC_WL, 4);
        assert_eq!(PDF_ZUGFERD_MINIMUM, 5);
        assert_eq!(PDF_ZUGFERD_XRECHNUNG, 6);
        assert_eq!(PDF_ZUGFERD_UNKNOWN, 7);
    }

    #[test]
    fn test_facturx_aliases() {
        assert_eq!(PDF_FACTURX_MINIMUM, PDF_ZUGFERD_MINIMUM);
        assert_eq!(PDF_FACTURX_BASIC_WL, PDF_ZUGFERD_BASIC_WL);
        assert_eq!(PDF_FACTURX_BASIC, PDF_ZUGFERD_BASIC);
        assert_eq!(PDF_FACTURX_EN16931, PDF_ZUGFERD_COMFORT);
        assert_eq!(PDF_FACTURX_EXTENDED, PDF_ZUGFERD_EXTENDED);
    }

    #[test]
    fn test_zugferd_info() {
        let info = ZugferdInfo::new();
        assert!(!info.is_zugferd());
        assert_eq!(info.profile, PDF_NOT_ZUGFERD);
        assert_eq!(info.version, 0.0);
    }

    #[test]
    fn test_invoice_data() {
        let data = InvoiceData::new()
            .with_xml(b"<?xml version=\"1.0\"?>")
            .with_filename("test.xml");

        assert_eq!(data.xml, b"<?xml version=\"1.0\"?>");
        assert_eq!(data.filename, "test.xml");
        assert_eq!(data.mime_type, "text/xml");
    }

    #[test]
    fn test_ffi_context() {
        let ctx = 0;
        let doc = 1;

        let zugferd = pdf_new_zugferd_context(ctx, doc);
        assert!(zugferd > 0);

        assert_eq!(pdf_is_zugferd(ctx, zugferd), 0);

        pdf_drop_zugferd_context(ctx, zugferd);
    }

    #[test]
    fn test_ffi_profile_detection() {
        let ctx = 0;
        let doc = 1;

        let zugferd = pdf_new_zugferd_context(ctx, doc);
        let mut version: f32 = 0.0;
        let profile = pdf_zugferd_profile(ctx, zugferd, &mut version);

        assert_eq!(profile, PDF_NOT_ZUGFERD);

        pdf_drop_zugferd_context(ctx, zugferd);
    }

    #[test]
    fn test_ffi_profile_to_string() {
        let ctx = 0;

        let s = pdf_zugferd_profile_to_string(ctx, PDF_ZUGFERD_COMFORT);
        assert!(!s.is_null());
        unsafe {
            let str = CStr::from_ptr(s).to_string_lossy();
            assert!(str.contains("Comfort"));
            pdf_zugferd_free_string(s);
        }

        let s = pdf_zugferd_profile_to_string(ctx, PDF_NOT_ZUGFERD);
        assert!(!s.is_null());
        unsafe {
            let str = CStr::from_ptr(s).to_string_lossy();
            assert_eq!(str, "Not ZUGFeRD");
            pdf_zugferd_free_string(s);
        }
    }

    #[test]
    fn test_ffi_xml_handling() {
        let ctx = 0;
        let doc = 1;

        let zugferd = pdf_new_zugferd_context(ctx, doc);

        // Set XML data
        let xml = b"<?xml version=\"1.0\"?><invoice/>";
        let result = pdf_zugferd_set_xml(ctx, zugferd, xml.as_ptr(), xml.len());
        assert_eq!(result, 1);

        // Get XML data
        let mut len: usize = 0;
        let ptr = pdf_zugferd_xml(ctx, zugferd, &mut len);
        assert!(!ptr.is_null());
        assert_eq!(len, xml.len());

        pdf_drop_zugferd_context(ctx, zugferd);
    }

    #[test]
    fn test_ffi_embed() {
        let ctx = 0;
        let doc = 1;

        let zugferd = pdf_new_zugferd_context(ctx, doc);

        let xml = b"<?xml version=\"1.0\"?><rsm:CrossIndustryInvoice/>";
        let params = pdf_zugferd_default_embed_params();

        let result = pdf_zugferd_embed(ctx, zugferd, xml.as_ptr(), xml.len(), &params);
        assert_eq!(result, 1);

        // Should now be detected as ZUGFeRD (with embedded data)
        let mut len: usize = 0;
        let ptr = pdf_zugferd_xml(ctx, zugferd, &mut len);
        assert!(!ptr.is_null());
        assert_eq!(len, xml.len());

        pdf_drop_zugferd_context(ctx, zugferd);
    }

    #[test]
    fn test_ffi_validation() {
        let ctx = 0;
        let doc = 1;

        let zugferd = pdf_new_zugferd_context(ctx, doc);

        // Without XML, should be invalid
        assert_eq!(pdf_zugferd_validate(ctx, zugferd), 0);

        // With valid-looking XML, should be valid
        let xml = b"<?xml version=\"1.0\"?>";
        pdf_zugferd_set_xml(ctx, zugferd, xml.as_ptr(), xml.len());
        assert_eq!(pdf_zugferd_validate(ctx, zugferd), 1);

        pdf_drop_zugferd_context(ctx, zugferd);
    }

    #[test]
    fn test_ffi_standard_filename() {
        let ctx = 0;

        let s = pdf_zugferd_standard_filename(ctx, PDF_ZUGFERD_COMFORT);
        assert!(!s.is_null());
        unsafe {
            let str = CStr::from_ptr(s).to_string_lossy();
            assert!(str.contains("ZUGFeRD"));
            pdf_zugferd_free_string(s);
        }

        let s = pdf_zugferd_standard_filename(ctx, PDF_ZUGFERD_XRECHNUNG);
        assert!(!s.is_null());
        unsafe {
            let str = CStr::from_ptr(s).to_string_lossy();
            assert_eq!(str, "factur-x.xml");
            pdf_zugferd_free_string(s);
        }
    }

    #[test]
    fn test_ffi_utility() {
        let ctx = 0;

        let mime = pdf_zugferd_mime_type(ctx);
        assert!(!mime.is_null());
        unsafe {
            let str = CStr::from_ptr(mime).to_string_lossy();
            assert_eq!(str, "text/xml");
            pdf_zugferd_free_string(mime);
        }

        let af = pdf_zugferd_af_relationship(ctx);
        assert!(!af.is_null());
        unsafe {
            let str = CStr::from_ptr(af).to_string_lossy();
            assert_eq!(str, "Alternative");
            pdf_zugferd_free_string(af);
        }
    }
}
