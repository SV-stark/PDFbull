//! PDF Signature FFI Module
//!
//! Provides support for PDF digital signatures, including signature
//! verification, signing, and certificate handling.

use crate::ffi::{Handle, HandleStore};
use std::collections::HashMap;
use std::ffi::{CStr, CString, c_char};
use std::ptr;
use std::sync::{Arc, LazyLock, Mutex};

// ============================================================================
// Type Aliases
// ============================================================================

type ContextHandle = Handle;
type DocumentHandle = Handle;
type AnnotHandle = Handle;
type PdfObjHandle = Handle;
type StreamHandle = Handle;

// ============================================================================
// Signature Error Types
// ============================================================================

/// Signature verification error codes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(C)]
pub enum SignatureError {
    #[default]
    Okay = 0,
    NoSignatures = 1,
    NoCertificate = 2,
    DigestFailure = 3,
    SelfSigned = 4,
    SelfSignedInChain = 5,
    NotTrusted = 6,
    NotSigned = 7,
    Unknown = 8,
}

impl SignatureError {
    pub fn from_i32(value: i32) -> Self {
        match value {
            0 => SignatureError::Okay,
            1 => SignatureError::NoSignatures,
            2 => SignatureError::NoCertificate,
            3 => SignatureError::DigestFailure,
            4 => SignatureError::SelfSigned,
            5 => SignatureError::SelfSignedInChain,
            6 => SignatureError::NotTrusted,
            7 => SignatureError::NotSigned,
            _ => SignatureError::Unknown,
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            SignatureError::Okay => "signature is valid",
            SignatureError::NoSignatures => "no signatures found",
            SignatureError::NoCertificate => "certificate not found",
            SignatureError::DigestFailure => "digest verification failed",
            SignatureError::SelfSigned => "certificate is self-signed",
            SignatureError::SelfSignedInChain => "self-signed certificate in chain",
            SignatureError::NotTrusted => "certificate is not trusted",
            SignatureError::NotSigned => "document is not signed",
            SignatureError::Unknown => "unknown error",
        }
    }
}

// ============================================================================
// Signature Appearance Flags
// ============================================================================

/// Flags for signature appearance
pub const PDF_SIGNATURE_SHOW_LABELS: i32 = 1;
pub const PDF_SIGNATURE_SHOW_DN: i32 = 2;
pub const PDF_SIGNATURE_SHOW_DATE: i32 = 4;
pub const PDF_SIGNATURE_SHOW_TEXT_NAME: i32 = 8;
pub const PDF_SIGNATURE_SHOW_GRAPHIC_NAME: i32 = 16;
pub const PDF_SIGNATURE_SHOW_LOGO: i32 = 32;

/// Default signature appearance
pub const PDF_SIGNATURE_DEFAULT_APPEARANCE: i32 = PDF_SIGNATURE_SHOW_LABELS
    | PDF_SIGNATURE_SHOW_DN
    | PDF_SIGNATURE_SHOW_DATE
    | PDF_SIGNATURE_SHOW_TEXT_NAME
    | PDF_SIGNATURE_SHOW_GRAPHIC_NAME
    | PDF_SIGNATURE_SHOW_LOGO;

// ============================================================================
// Distinguished Name Structure
// ============================================================================

/// X.500 Distinguished Name for certificate identification
#[derive(Debug, Clone, Default)]
pub struct DistinguishedName {
    /// Common Name (CN)
    pub cn: Option<String>,
    /// Organization (O)
    pub o: Option<String>,
    /// Organizational Unit (OU)
    pub ou: Option<String>,
    /// Email address
    pub email: Option<String>,
    /// Country (C)
    pub c: Option<String>,
}

impl DistinguishedName {
    pub fn new() -> Self {
        Self::default()
    }

    /// Format the DN as a string
    pub fn format(&self) -> String {
        let mut parts = Vec::new();
        if let Some(ref cn) = self.cn {
            parts.push(format!("CN={}", cn));
        }
        if let Some(ref o) = self.o {
            parts.push(format!("O={}", o));
        }
        if let Some(ref ou) = self.ou {
            parts.push(format!("OU={}", ou));
        }
        if let Some(ref email) = self.email {
            parts.push(format!("EMAIL={}", email));
        }
        if let Some(ref c) = self.c {
            parts.push(format!("C={}", c));
        }
        parts.join(", ")
    }
}

// ============================================================================
// C-compatible DN structure
// ============================================================================

/// C-compatible Distinguished Name structure for FFI
#[repr(C)]
pub struct FfiDistinguishedName {
    pub cn: *const c_char,
    pub o: *const c_char,
    pub ou: *const c_char,
    pub email: *const c_char,
    pub c: *const c_char,
}

// ============================================================================
// Byte Range Structure
// ============================================================================

/// Byte range for signature coverage
#[derive(Debug, Clone, Default)]
#[repr(C)]
pub struct ByteRange {
    pub offset: i64,
    pub length: i64,
}

// ============================================================================
// Signature Information
// ============================================================================

/// Signature field information
#[derive(Debug, Clone)]
pub struct SignatureInfo {
    /// Whether the field is signed
    pub is_signed: bool,
    /// Signer's distinguished name
    pub signer_dn: Option<DistinguishedName>,
    /// Signing reason
    pub reason: Option<String>,
    /// Signing location
    pub location: Option<String>,
    /// Signing date (Unix timestamp)
    pub date: i64,
    /// Byte ranges covered by signature
    pub byte_ranges: Vec<ByteRange>,
    /// Signature contents (PKCS#7 data)
    pub contents: Vec<u8>,
    /// Whether document changed since signing
    pub incremental_change: bool,
    /// Digest verification status
    pub digest_status: SignatureError,
    /// Certificate verification status
    pub certificate_status: SignatureError,
}

impl Default for SignatureInfo {
    fn default() -> Self {
        Self::new()
    }
}

impl SignatureInfo {
    pub fn new() -> Self {
        Self {
            is_signed: false,
            signer_dn: None,
            reason: None,
            location: None,
            date: 0,
            byte_ranges: Vec::new(),
            contents: Vec::new(),
            incremental_change: false,
            digest_status: SignatureError::NotSigned,
            certificate_status: SignatureError::NotSigned,
        }
    }
}

// ============================================================================
// PKCS#7 Signer (for signing documents)
// ============================================================================

/// PKCS#7 Signer for creating digital signatures
#[derive(Debug)]
pub struct Pkcs7Signer {
    /// Signer's distinguished name
    pub dn: DistinguishedName,
    /// Private key (placeholder - actual implementation would use crypto library)
    pub private_key: Vec<u8>,
    /// Certificate chain (placeholder)
    pub certificate: Vec<u8>,
    /// Maximum digest size
    pub max_digest_size: usize,
}

impl Pkcs7Signer {
    pub fn new(cn: &str) -> Self {
        Self {
            dn: DistinguishedName {
                cn: Some(cn.to_string()),
                ..Default::default()
            },
            private_key: Vec::new(),
            certificate: Vec::new(),
            max_digest_size: 8192, // Default PKCS#7 size
        }
    }

    /// Get the signer's distinguished name
    pub fn get_signing_name(&self) -> &DistinguishedName {
        &self.dn
    }

    /// Get maximum digest size
    pub fn max_digest_size(&self) -> usize {
        self.max_digest_size
    }

    /// Create a digest (placeholder - actual implementation would use crypto)
    pub fn create_digest(&self, _data: &[u8]) -> Vec<u8> {
        // Placeholder: In a real implementation, this would:
        // 1. Hash the data
        // 2. Create a PKCS#7 signature using the private key
        // 3. Return the signature bytes
        vec![0u8; self.max_digest_size]
    }
}

// ============================================================================
// PKCS#7 Verifier (for verifying signatures)
// ============================================================================

/// PKCS#7 Verifier for validating digital signatures
#[derive(Debug)]
pub struct Pkcs7Verifier {
    /// Trusted certificate store (placeholder)
    pub trusted_certs: Vec<Vec<u8>>,
}

impl Default for Pkcs7Verifier {
    fn default() -> Self {
        Self::new()
    }
}

impl Pkcs7Verifier {
    pub fn new() -> Self {
        Self {
            trusted_certs: Vec::new(),
        }
    }

    /// Add a trusted certificate
    pub fn add_trusted_cert(&mut self, cert: Vec<u8>) {
        self.trusted_certs.push(cert);
    }

    /// Check certificate validity (placeholder)
    pub fn check_certificate(&self, _signature: &[u8]) -> SignatureError {
        // Placeholder: In a real implementation, this would:
        // 1. Parse the PKCS#7 signature
        // 2. Extract the certificate chain
        // 3. Validate against trusted roots
        SignatureError::Okay
    }

    /// Check digest validity (placeholder)
    pub fn check_digest(&self, _data: &[u8], _signature: &[u8]) -> SignatureError {
        // Placeholder: In a real implementation, this would:
        // 1. Parse the PKCS#7 signature
        // 2. Extract the signed hash
        // 3. Compute hash of data
        // 4. Compare hashes
        SignatureError::Okay
    }

    /// Get signatory information from signature
    pub fn get_signatory(&self, _signature: &[u8]) -> Option<DistinguishedName> {
        // Placeholder: In a real implementation, this would parse the
        // PKCS#7 signature and extract the signer's DN
        Some(DistinguishedName::new())
    }
}

// ============================================================================
// Global Handle Stores
// ============================================================================

pub static SIGNERS: LazyLock<HandleStore<Pkcs7Signer>> = LazyLock::new(HandleStore::new);
pub static VERIFIERS: LazyLock<HandleStore<Pkcs7Verifier>> = LazyLock::new(HandleStore::new);
pub static DISTINGUISHED_NAMES: LazyLock<HandleStore<DistinguishedName>> =
    LazyLock::new(HandleStore::new);
pub static SIGNATURE_INFOS: LazyLock<HandleStore<SignatureInfo>> = LazyLock::new(HandleStore::new);

// Store signatures per document
pub static DOC_SIGNATURES: LazyLock<Mutex<HashMap<DocumentHandle, Vec<SignatureInfo>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

// ============================================================================
// FFI Functions - Signature Query
// ============================================================================

/// Check if a signature field is signed.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_signature_is_signed(
    _ctx: ContextHandle,
    doc: DocumentHandle,
    _field: PdfObjHandle,
) -> i32 {
    let store = DOC_SIGNATURES.lock().unwrap();
    if let Some(sigs) = store.get(&doc) {
        if !sigs.is_empty() && sigs.iter().any(|s| s.is_signed) {
            return 1;
        }
    }
    0
}

/// Count signatures in document.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_count_signatures(_ctx: ContextHandle, doc: DocumentHandle) -> i32 {
    let store = DOC_SIGNATURES.lock().unwrap();
    if let Some(sigs) = store.get(&doc) {
        return sigs.len() as i32;
    }
    0
}

/// Get signature byte range.
/// Returns number of ranges, fills byte_range array.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_signature_byte_range(
    _ctx: ContextHandle,
    doc: DocumentHandle,
    _signature: PdfObjHandle,
    byte_range: *mut ByteRange,
) -> i32 {
    let store = DOC_SIGNATURES.lock().unwrap();
    if let Some(sigs) = store.get(&doc) {
        if let Some(sig) = sigs.first() {
            if !byte_range.is_null() && !sig.byte_ranges.is_empty() {
                unsafe {
                    *byte_range = sig.byte_ranges[0].clone();
                }
            }
            return sig.byte_ranges.len() as i32;
        }
    }
    0
}

/// Get signature contents (PKCS#7 data).
/// Returns size of contents, allocates and fills contents pointer.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_signature_contents(
    _ctx: ContextHandle,
    doc: DocumentHandle,
    _signature: PdfObjHandle,
    contents: *mut *mut c_char,
) -> usize {
    let store = DOC_SIGNATURES.lock().unwrap();
    if let Some(sigs) = store.get(&doc) {
        if let Some(sig) = sigs.first() {
            if !contents.is_null() && !sig.contents.is_empty() {
                let len = sig.contents.len();
                // Allocate using Box to ensure proper memory management
                let mut boxed = sig.contents.clone().into_boxed_slice();
                let ptr = boxed.as_mut_ptr() as *mut c_char;
                // SAFETY: We deliberately leak this memory to transfer ownership to the C caller.
                // The caller is responsible for freeing this memory using the appropriate
                // deallocation function (e.g., fz_free or standard C free if allocated via malloc).
                // Memory layout: contiguous array of len bytes, allocated via Rust's global allocator.
                // The corresponding cleanup should use: Box::from_raw(std::slice::from_raw_parts_mut(ptr, len))
                std::mem::forget(boxed);
                // SAFETY: contents was checked for null above
                unsafe {
                    *contents = ptr;
                }
                return len;
            }
        }
    }
    0
}

/// Check if document has incremental changes since signing.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_signature_incremental_change_since_signing(
    _ctx: ContextHandle,
    doc: DocumentHandle,
    _signature: PdfObjHandle,
) -> i32 {
    let store = DOC_SIGNATURES.lock().unwrap();
    if let Some(sigs) = store.get(&doc) {
        if let Some(sig) = sigs.first() {
            return if sig.incremental_change { 1 } else { 0 };
        }
    }
    0
}

// ============================================================================
// FFI Functions - Signature Verification
// ============================================================================

/// Check signature digest.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_check_digest(
    _ctx: ContextHandle,
    verifier: Handle,
    doc: DocumentHandle,
    _signature: PdfObjHandle,
) -> i32 {
    let store = DOC_SIGNATURES.lock().unwrap();
    if let Some(sigs) = store.get(&doc) {
        if let Some(sig) = sigs.first() {
            if let Some(verifier_arc) = VERIFIERS.get(verifier) {
                let v = verifier_arc.lock().unwrap();
                return v.check_digest(&[], &sig.contents) as i32;
            }
            return sig.digest_status as i32;
        }
    }
    SignatureError::NotSigned as i32
}

/// Check signature certificate.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_check_certificate(
    _ctx: ContextHandle,
    verifier: Handle,
    doc: DocumentHandle,
    _signature: PdfObjHandle,
) -> i32 {
    let store = DOC_SIGNATURES.lock().unwrap();
    if let Some(sigs) = store.get(&doc) {
        if let Some(sig) = sigs.first() {
            if let Some(verifier_arc) = VERIFIERS.get(verifier) {
                let v = verifier_arc.lock().unwrap();
                return v.check_certificate(&sig.contents) as i32;
            }
            return sig.certificate_status as i32;
        }
    }
    SignatureError::NotSigned as i32
}

/// Get signature error description.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_signature_error_description(err: i32) -> *const c_char {
    let error = SignatureError::from_i32(err);
    match error {
        SignatureError::Okay => c"signature is valid".as_ptr(),
        SignatureError::NoSignatures => c"no signatures found".as_ptr(),
        SignatureError::NoCertificate => c"certificate not found".as_ptr(),
        SignatureError::DigestFailure => c"digest verification failed".as_ptr(),
        SignatureError::SelfSigned => c"certificate is self-signed".as_ptr(),
        SignatureError::SelfSignedInChain => c"self-signed certificate in chain".as_ptr(),
        SignatureError::NotTrusted => c"certificate is not trusted".as_ptr(),
        SignatureError::NotSigned => c"document is not signed".as_ptr(),
        SignatureError::Unknown => c"unknown error".as_ptr(),
    }
}

// ============================================================================
// FFI Functions - Distinguished Name
// ============================================================================

/// Get signatory information from signature.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_signature_get_signatory(
    _ctx: ContextHandle,
    verifier: Handle,
    doc: DocumentHandle,
    _signature: PdfObjHandle,
) -> Handle {
    let store = DOC_SIGNATURES.lock().unwrap();
    if let Some(sigs) = store.get(&doc) {
        if let Some(sig) = sigs.first() {
            if let Some(ref dn) = sig.signer_dn {
                return DISTINGUISHED_NAMES.insert(dn.clone());
            }
            // Try to get from verifier
            if let Some(verifier_arc) = VERIFIERS.get(verifier) {
                let v = verifier_arc.lock().unwrap();
                if let Some(dn) = v.get_signatory(&sig.contents) {
                    return DISTINGUISHED_NAMES.insert(dn);
                }
            }
        }
    }
    0
}

/// Drop a distinguished name.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_signature_drop_distinguished_name(_ctx: ContextHandle, dn: Handle) {
    DISTINGUISHED_NAMES.remove(dn);
}

/// Format distinguished name as string.
/// Caller must free the returned string.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_signature_format_distinguished_name(
    _ctx: ContextHandle,
    dn: Handle,
) -> *const c_char {
    if let Some(dn_arc) = DISTINGUISHED_NAMES.get(dn) {
        let dn_guard = dn_arc.lock().unwrap();
        let formatted = dn_guard.format();
        if let Ok(cstr) = CString::new(formatted) {
            return cstr.into_raw();
        }
    }
    ptr::null()
}

/// Get DN component (CN).
#[unsafe(no_mangle)]
pub extern "C" fn pdf_dn_cn(_ctx: ContextHandle, dn: Handle) -> *const c_char {
    if let Some(dn_arc) = DISTINGUISHED_NAMES.get(dn) {
        let dn_guard = dn_arc.lock().unwrap();
        if let Some(ref cn) = dn_guard.cn {
            if let Ok(cstr) = CString::new(cn.clone()) {
                return cstr.into_raw();
            }
        }
    }
    ptr::null()
}

/// Get DN component (O).
#[unsafe(no_mangle)]
pub extern "C" fn pdf_dn_o(_ctx: ContextHandle, dn: Handle) -> *const c_char {
    if let Some(dn_arc) = DISTINGUISHED_NAMES.get(dn) {
        let dn_guard = dn_arc.lock().unwrap();
        if let Some(ref o) = dn_guard.o {
            if let Ok(cstr) = CString::new(o.clone()) {
                return cstr.into_raw();
            }
        }
    }
    ptr::null()
}

/// Get DN component (OU).
#[unsafe(no_mangle)]
pub extern "C" fn pdf_dn_ou(_ctx: ContextHandle, dn: Handle) -> *const c_char {
    if let Some(dn_arc) = DISTINGUISHED_NAMES.get(dn) {
        let dn_guard = dn_arc.lock().unwrap();
        if let Some(ref ou) = dn_guard.ou {
            if let Ok(cstr) = CString::new(ou.clone()) {
                return cstr.into_raw();
            }
        }
    }
    ptr::null()
}

/// Get DN component (email).
#[unsafe(no_mangle)]
pub extern "C" fn pdf_dn_email(_ctx: ContextHandle, dn: Handle) -> *const c_char {
    if let Some(dn_arc) = DISTINGUISHED_NAMES.get(dn) {
        let dn_guard = dn_arc.lock().unwrap();
        if let Some(ref email) = dn_guard.email {
            if let Ok(cstr) = CString::new(email.clone()) {
                return cstr.into_raw();
            }
        }
    }
    ptr::null()
}

/// Get DN component (C - country).
#[unsafe(no_mangle)]
pub extern "C" fn pdf_dn_c(_ctx: ContextHandle, dn: Handle) -> *const c_char {
    if let Some(dn_arc) = DISTINGUISHED_NAMES.get(dn) {
        let dn_guard = dn_arc.lock().unwrap();
        if let Some(ref c) = dn_guard.c {
            if let Ok(cstr) = CString::new(c.clone()) {
                return cstr.into_raw();
            }
        }
    }
    ptr::null()
}

// ============================================================================
// FFI Functions - Signer Management
// ============================================================================

/// Create a new PKCS#7 signer.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_pkcs7_signer_new(_ctx: ContextHandle, cn: *const c_char) -> Handle {
    let cn_str = if !cn.is_null() {
        unsafe { CStr::from_ptr(cn).to_str().unwrap_or("Unknown") }
    } else {
        "Unknown"
    };

    let signer = Pkcs7Signer::new(cn_str);
    SIGNERS.insert(signer)
}

/// Keep (increment reference to) a signer.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_pkcs7_keep_signer(_ctx: ContextHandle, signer: Handle) -> Handle {
    SIGNERS.keep(signer)
}

/// Drop a signer.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_drop_signer(_ctx: ContextHandle, signer: Handle) {
    SIGNERS.remove(signer);
}

/// Get signer's distinguished name.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_pkcs7_signer_get_name(_ctx: ContextHandle, signer: Handle) -> Handle {
    if let Some(signer_arc) = SIGNERS.get(signer) {
        let s = signer_arc.lock().unwrap();
        return DISTINGUISHED_NAMES.insert(s.dn.clone());
    }
    0
}

/// Get signer's max digest size.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_pkcs7_signer_max_digest_size(_ctx: ContextHandle, signer: Handle) -> usize {
    if let Some(signer_arc) = SIGNERS.get(signer) {
        let s = signer_arc.lock().unwrap();
        return s.max_digest_size();
    }
    0
}

// ============================================================================
// FFI Functions - Verifier Management
// ============================================================================

/// Create a new PKCS#7 verifier.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_pkcs7_verifier_new(_ctx: ContextHandle) -> Handle {
    let verifier = Pkcs7Verifier::new();
    VERIFIERS.insert(verifier)
}

/// Drop a verifier.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_drop_verifier(_ctx: ContextHandle, verifier: Handle) {
    VERIFIERS.remove(verifier);
}

/// Add a trusted certificate to verifier.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_pkcs7_verifier_add_cert(
    _ctx: ContextHandle,
    verifier: Handle,
    cert: *const u8,
    len: usize,
) {
    if cert.is_null() || len == 0 {
        return;
    }

    if let Some(verifier_arc) = VERIFIERS.get(verifier) {
        let mut v = verifier_arc.lock().unwrap();
        let cert_data = unsafe { std::slice::from_raw_parts(cert, len).to_vec() };
        v.add_trusted_cert(cert_data);
    }
}

// ============================================================================
// FFI Functions - Signing Operations
// ============================================================================

/// Sign a signature field.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_sign_signature(
    _ctx: ContextHandle,
    _widget: AnnotHandle,
    signer: Handle,
    date: i64,
    _reason: *const c_char,
    _location: *const c_char,
) {
    if let Some(signer_arc) = SIGNERS.get(signer) {
        let s = signer_arc.lock().unwrap();

        // Create signature info
        let sig_info = SignatureInfo {
            is_signed: true,
            signer_dn: Some(s.dn.clone()),
            reason: None,   // Would parse from _reason
            location: None, // Would parse from _location
            date,
            byte_ranges: vec![ByteRange {
                offset: 0,
                length: 0,
            }],
            contents: s.create_digest(&[]),
            incremental_change: false,
            digest_status: SignatureError::Okay,
            certificate_status: SignatureError::Okay,
        };

        // Store in document signatures (using widget as doc handle for now)
        let mut store = DOC_SIGNATURES.lock().unwrap();
        store.entry(_widget).or_default().push(sig_info);
    }
}

/// Clear a signature from a widget.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_clear_signature(_ctx: ContextHandle, widget: AnnotHandle) {
    let mut store = DOC_SIGNATURES.lock().unwrap();
    store.remove(&widget);
}

/// Set signature value on a field.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_signature_set_value(
    _ctx: ContextHandle,
    doc: DocumentHandle,
    _field: PdfObjHandle,
    signer: Handle,
    stime: i64,
) {
    if let Some(signer_arc) = SIGNERS.get(signer) {
        let s = signer_arc.lock().unwrap();

        let sig_info = SignatureInfo {
            is_signed: true,
            signer_dn: Some(s.dn.clone()),
            reason: None,
            location: None,
            date: stime,
            byte_ranges: Vec::new(),
            contents: s.create_digest(&[]),
            incremental_change: false,
            digest_status: SignatureError::Okay,
            certificate_status: SignatureError::Okay,
        };

        let mut store = DOC_SIGNATURES.lock().unwrap();
        store.entry(doc).or_default().push(sig_info);
    }
}

// ============================================================================
// FFI Functions - Signature Info Formatting
// ============================================================================

/// Format signature info as string.
/// Caller must free the returned string.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_signature_info(
    _ctx: ContextHandle,
    name: *const c_char,
    dn: Handle,
    reason: *const c_char,
    location: *const c_char,
    date: i64,
    include_labels: i32,
) -> *const c_char {
    let mut parts = Vec::new();

    // Name
    if !name.is_null() {
        let name_str = unsafe { CStr::from_ptr(name).to_str().unwrap_or("") };
        if !name_str.is_empty() {
            if include_labels != 0 {
                parts.push(format!("Signed by: {}", name_str));
            } else {
                parts.push(name_str.to_string());
            }
        }
    }

    // DN
    if dn != 0 {
        if let Some(dn_arc) = DISTINGUISHED_NAMES.get(dn) {
            let dn_guard = dn_arc.lock().unwrap();
            let dn_str = dn_guard.format();
            if !dn_str.is_empty() {
                if include_labels != 0 {
                    parts.push(format!("DN: {}", dn_str));
                } else {
                    parts.push(dn_str);
                }
            }
        }
    }

    // Reason
    if !reason.is_null() {
        let reason_str = unsafe { CStr::from_ptr(reason).to_str().unwrap_or("") };
        if !reason_str.is_empty() {
            if include_labels != 0 {
                parts.push(format!("Reason: {}", reason_str));
            } else {
                parts.push(reason_str.to_string());
            }
        }
    }

    // Location
    if !location.is_null() {
        let loc_str = unsafe { CStr::from_ptr(location).to_str().unwrap_or("") };
        if !loc_str.is_empty() {
            if include_labels != 0 {
                parts.push(format!("Location: {}", loc_str));
            } else {
                parts.push(loc_str.to_string());
            }
        }
    }

    // Date
    if date != 0 {
        if include_labels != 0 {
            parts.push(format!("Date: {}", date));
        } else {
            parts.push(format!("{}", date));
        }
    }

    let result = parts.join("\n");
    if let Ok(cstr) = CString::new(result) {
        return cstr.into_raw();
    }
    ptr::null()
}

/// Free a string allocated by signature functions.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_signature_free_string(_ctx: ContextHandle, s: *mut c_char) {
    if !s.is_null() {
        unsafe {
            drop(CString::from_raw(s));
        }
    }
}

// ============================================================================
// FFI Functions - Additional Signature Management
// ============================================================================

/// Add a signature to document (for testing/simulation).
#[unsafe(no_mangle)]
pub extern "C" fn pdf_add_signature(
    _ctx: ContextHandle,
    doc: DocumentHandle,
    cn: *const c_char,
    date: i64,
) -> i32 {
    let cn_str = if !cn.is_null() {
        unsafe { CStr::from_ptr(cn).to_str().unwrap_or("Unknown") }
    } else {
        "Unknown"
    };

    let sig_info = SignatureInfo {
        is_signed: true,
        signer_dn: Some(DistinguishedName {
            cn: Some(cn_str.to_string()),
            ..Default::default()
        }),
        reason: None,
        location: None,
        date,
        byte_ranges: vec![
            ByteRange {
                offset: 0,
                length: 1000,
            },
            ByteRange {
                offset: 2000,
                length: 3000,
            },
        ],
        contents: vec![0u8; 256], // Placeholder signature data
        incremental_change: false,
        digest_status: SignatureError::Okay,
        certificate_status: SignatureError::Okay,
    };

    let mut store = DOC_SIGNATURES.lock().unwrap();
    let sigs = store.entry(doc).or_default();
    let idx = sigs.len() as i32;
    sigs.push(sig_info);
    idx
}

/// Get signature at index.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_get_signature(
    _ctx: ContextHandle,
    doc: DocumentHandle,
    index: i32,
) -> Handle {
    let store = DOC_SIGNATURES.lock().unwrap();
    if let Some(sigs) = store.get(&doc) {
        if let Some(sig) = sigs.get(index as usize) {
            return SIGNATURE_INFOS.insert(sig.clone());
        }
    }
    0
}

/// Drop a signature info handle.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_drop_signature_info(_ctx: ContextHandle, sig: Handle) {
    SIGNATURE_INFOS.remove(sig);
}

/// Clear all signatures from document.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_clear_all_signatures(_ctx: ContextHandle, doc: DocumentHandle) {
    let mut store = DOC_SIGNATURES.lock().unwrap();
    store.remove(&doc);
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signature_error_types() {
        assert_eq!(SignatureError::from_i32(0), SignatureError::Okay);
        assert_eq!(SignatureError::from_i32(3), SignatureError::DigestFailure);
        assert_eq!(SignatureError::from_i32(99), SignatureError::Unknown);

        assert_eq!(SignatureError::Okay.description(), "signature is valid");
        assert_eq!(
            SignatureError::DigestFailure.description(),
            "digest verification failed"
        );
    }

    #[test]
    fn test_distinguished_name() {
        let dn = DistinguishedName {
            cn: Some("John Doe".to_string()),
            o: Some("ACME Corp".to_string()),
            ou: Some("Engineering".to_string()),
            email: Some("john@acme.com".to_string()),
            c: Some("US".to_string()),
        };

        let formatted = dn.format();
        assert!(formatted.contains("CN=John Doe"));
        assert!(formatted.contains("O=ACME Corp"));
        assert!(formatted.contains("OU=Engineering"));
        assert!(formatted.contains("EMAIL=john@acme.com"));
        assert!(formatted.contains("C=US"));
    }

    #[test]
    fn test_pkcs7_signer() {
        let signer = Pkcs7Signer::new("Test Signer");
        assert_eq!(signer.dn.cn, Some("Test Signer".to_string()));
        assert_eq!(signer.max_digest_size(), 8192);

        let digest = signer.create_digest(&[1, 2, 3, 4]);
        assert_eq!(digest.len(), 8192);
    }

    #[test]
    fn test_pkcs7_verifier() {
        let mut verifier = Pkcs7Verifier::new();
        assert!(verifier.trusted_certs.is_empty());

        verifier.add_trusted_cert(vec![1, 2, 3, 4]);
        assert_eq!(verifier.trusted_certs.len(), 1);

        assert_eq!(verifier.check_certificate(&[]), SignatureError::Okay);
        assert_eq!(verifier.check_digest(&[], &[]), SignatureError::Okay);
    }

    #[test]
    fn test_signature_info() {
        let info = SignatureInfo::new();
        assert!(!info.is_signed);
        assert!(info.signer_dn.is_none());
        assert_eq!(info.digest_status, SignatureError::NotSigned);
    }

    #[test]
    fn test_ffi_signer_lifecycle() {
        let ctx = 0;
        let cn = CString::new("Test").unwrap();

        let signer = pdf_pkcs7_signer_new(ctx, cn.as_ptr());
        assert!(signer > 0);

        let max_size = pdf_pkcs7_signer_max_digest_size(ctx, signer);
        assert_eq!(max_size, 8192);

        let dn = pdf_pkcs7_signer_get_name(ctx, signer);
        assert!(dn > 0);

        pdf_signature_drop_distinguished_name(ctx, dn);
        pdf_drop_signer(ctx, signer);
    }

    #[test]
    fn test_ffi_verifier_lifecycle() {
        let ctx = 0;

        let verifier = pdf_pkcs7_verifier_new(ctx);
        assert!(verifier > 0);

        let cert = vec![1u8, 2, 3, 4];
        pdf_pkcs7_verifier_add_cert(ctx, verifier, cert.as_ptr(), cert.len());

        pdf_drop_verifier(ctx, verifier);
    }

    #[test]
    fn test_ffi_add_signature() {
        let ctx = 0;
        let doc: DocumentHandle = 888;
        let cn = CString::new("Signer").unwrap();

        // Add signature
        let idx = pdf_add_signature(ctx, doc, cn.as_ptr(), 1234567890);
        assert_eq!(idx, 0);

        // Count signatures
        assert_eq!(pdf_count_signatures(ctx, doc), 1);

        // Check if signed
        assert_eq!(pdf_signature_is_signed(ctx, doc, 0), 1);

        // Check incremental change
        assert_eq!(
            pdf_signature_incremental_change_since_signing(ctx, doc, 0),
            0
        );

        // Get signatory
        let verifier = pdf_pkcs7_verifier_new(ctx);
        let dn = pdf_signature_get_signatory(ctx, verifier, doc, 0);
        assert!(dn > 0);

        // Format DN
        let formatted = pdf_signature_format_distinguished_name(ctx, dn);
        assert!(!formatted.is_null());
        unsafe {
            let s = CStr::from_ptr(formatted).to_str().unwrap();
            assert!(s.contains("CN=Signer"));
            pdf_signature_free_string(ctx, formatted as *mut c_char);
        }

        pdf_signature_drop_distinguished_name(ctx, dn);
        pdf_drop_verifier(ctx, verifier);

        // Clear signatures
        pdf_clear_all_signatures(ctx, doc);
        assert_eq!(pdf_count_signatures(ctx, doc), 0);
    }

    #[test]
    fn test_ffi_signature_error_description() {
        let desc = pdf_signature_error_description(SignatureError::Okay as i32);
        assert!(!desc.is_null());
        let s = unsafe { CStr::from_ptr(desc).to_str().unwrap() };
        assert_eq!(s, "signature is valid");

        let desc2 = pdf_signature_error_description(SignatureError::DigestFailure as i32);
        let s2 = unsafe { CStr::from_ptr(desc2).to_str().unwrap() };
        assert_eq!(s2, "digest verification failed");
    }

    #[test]
    fn test_ffi_signature_info_format() {
        let ctx = 0;
        let name = CString::new("John Doe").unwrap();
        let reason = CString::new("Approved").unwrap();
        let location = CString::new("New York").unwrap();

        // Create a DN
        let dn_handle = DISTINGUISHED_NAMES.insert(DistinguishedName {
            cn: Some("John Doe".to_string()),
            o: Some("ACME".to_string()),
            ..Default::default()
        });

        let info = pdf_signature_info(
            ctx,
            name.as_ptr(),
            dn_handle,
            reason.as_ptr(),
            location.as_ptr(),
            1234567890,
            1, // include labels
        );

        assert!(!info.is_null());
        unsafe {
            let s = CStr::from_ptr(info).to_str().unwrap();
            assert!(s.contains("Signed by: John Doe"));
            assert!(s.contains("Reason: Approved"));
            assert!(s.contains("Location: New York"));
            pdf_signature_free_string(ctx, info as *mut c_char);
        }

        DISTINGUISHED_NAMES.remove(dn_handle);
    }

    #[test]
    fn test_byte_range() {
        let br = ByteRange {
            offset: 100,
            length: 500,
        };
        assert_eq!(br.offset, 100);
        assert_eq!(br.length, 500);
    }

    #[test]
    fn test_dn_components() {
        let ctx = 0;
        let dn_handle = DISTINGUISHED_NAMES.insert(DistinguishedName {
            cn: Some("Test CN".to_string()),
            o: Some("Test O".to_string()),
            ou: Some("Test OU".to_string()),
            email: Some("test@test.com".to_string()),
            c: Some("US".to_string()),
        });

        let cn = pdf_dn_cn(ctx, dn_handle);
        assert!(!cn.is_null());
        unsafe {
            assert_eq!(CStr::from_ptr(cn).to_str().unwrap(), "Test CN");
            pdf_signature_free_string(ctx, cn as *mut c_char);
        }

        let o = pdf_dn_o(ctx, dn_handle);
        assert!(!o.is_null());
        unsafe {
            assert_eq!(CStr::from_ptr(o).to_str().unwrap(), "Test O");
            pdf_signature_free_string(ctx, o as *mut c_char);
        }

        DISTINGUISHED_NAMES.remove(dn_handle);
    }
}
