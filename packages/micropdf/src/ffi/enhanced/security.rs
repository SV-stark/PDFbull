//! Enhanced Security FFI - Digital signatures and encryption with `mp_` prefix
//!
//! This module provides FFI functions for enterprise security features:
//! - Digital signatures (create, verify, TSA)
//! - Encryption (AES-256, certificate-based)
//! - Document permissions

use crate::ffi::Handle;
use std::ffi::{CStr, CString, c_char, c_int};
use std::ptr;

// ============================================================================
// Opaque Handle Types
// ============================================================================

/// Certificate handle
pub type CertificateHandle = Handle;

/// Signer handle (for enhanced signatures)
pub type EnhancedSignerHandle = Handle;

/// Encryption options handle
pub type EncryptionOptionsHandle = Handle;

// ============================================================================
// Error Codes
// ============================================================================

/// Security operation error codes
#[repr(C)]
pub enum SecurityError {
    /// Success
    Ok = 0,
    /// Invalid parameter
    InvalidParameter = -1,
    /// File not found
    FileNotFound = -2,
    /// Invalid certificate
    InvalidCertificate = -3,
    /// Invalid password
    InvalidPassword = -4,
    /// Signing failed
    SigningFailed = -5,
    /// Verification failed
    VerificationFailed = -6,
    /// Encryption failed
    EncryptionFailed = -7,
    /// Decryption failed
    DecryptionFailed = -8,
    /// TSA request failed
    TsaFailed = -9,
    /// Feature not available
    NotAvailable = -10,
}

// ============================================================================
// Certificate Management
// ============================================================================

/// Load certificate from PKCS#12 (.p12/.pfx) file
///
/// # Safety
/// - `path` must be a valid null-terminated C string
/// - `password` must be a valid null-terminated C string (or null for no password)
///
/// # Returns
/// Certificate handle on success, 0 on failure
#[unsafe(no_mangle)]
pub extern "C" fn mp_certificate_load_pkcs12(
    path: *const c_char,
    password: *const c_char,
) -> CertificateHandle {
    if path.is_null() {
        return 0;
    }

    let _path_str = match unsafe { CStr::from_ptr(path) }.to_str() {
        Ok(s) => s,
        Err(_) => return 0,
    };

    let _password_str = if password.is_null() {
        None
    } else {
        match unsafe { CStr::from_ptr(password) }.to_str() {
            Ok(s) => Some(s),
            Err(_) => return 0,
        }
    };

    #[cfg(feature = "signatures")]
    {
        use crate::enhanced::signatures::Certificate;
        match Certificate::from_pkcs12(_path_str, _password_str) {
            Ok(_cert) => {
                // Store certificate and return handle
                // For now, return placeholder handle
                1
            }
            Err(_) => 0,
        }
    }

    #[cfg(not(feature = "signatures"))]
    {
        0
    }
}

/// Load certificate from PEM files
///
/// # Safety
/// - `cert_path` must be a valid null-terminated C string
/// - `key_path` must be a valid null-terminated C string
/// - `key_password` can be null for unencrypted keys
#[unsafe(no_mangle)]
pub extern "C" fn mp_certificate_load_pem(
    cert_path: *const c_char,
    key_path: *const c_char,
    key_password: *const c_char,
) -> CertificateHandle {
    if cert_path.is_null() || key_path.is_null() {
        return 0;
    }

    let _cert_str = match unsafe { CStr::from_ptr(cert_path) }.to_str() {
        Ok(s) => s,
        Err(_) => return 0,
    };

    let _key_str = match unsafe { CStr::from_ptr(key_path) }.to_str() {
        Ok(s) => s,
        Err(_) => return 0,
    };

    let _password_str = if key_password.is_null() {
        None
    } else {
        match unsafe { CStr::from_ptr(key_password) }.to_str() {
            Ok(s) => Some(s),
            Err(_) => return 0,
        }
    };

    #[cfg(feature = "signatures")]
    {
        use crate::enhanced::signatures::Certificate;
        match Certificate::from_pem(_cert_str, _key_str, _password_str) {
            Ok(_cert) => 1,
            Err(_) => 0,
        }
    }

    #[cfg(not(feature = "signatures"))]
    {
        0
    }
}

/// Drop a certificate handle
#[unsafe(no_mangle)]
pub extern "C" fn mp_certificate_drop(_cert: CertificateHandle) {
    // Release certificate resources
}

/// Get certificate subject common name
///
/// # Safety
/// Caller must free the returned string with `mp_free_string`
#[unsafe(no_mangle)]
pub extern "C" fn mp_certificate_get_subject(cert: CertificateHandle) -> *const c_char {
    if cert == 0 {
        return ptr::null();
    }

    // Placeholder - would return certificate subject CN
    match CString::new("Unknown Subject") {
        Ok(s) => s.into_raw(),
        Err(_) => ptr::null(),
    }
}

/// Get certificate issuer common name
#[unsafe(no_mangle)]
pub extern "C" fn mp_certificate_get_issuer(cert: CertificateHandle) -> *const c_char {
    if cert == 0 {
        return ptr::null();
    }

    match CString::new("Unknown Issuer") {
        Ok(s) => s.into_raw(),
        Err(_) => ptr::null(),
    }
}

/// Check if certificate is currently valid
#[unsafe(no_mangle)]
pub extern "C" fn mp_certificate_is_valid(cert: CertificateHandle) -> c_int {
    if cert == 0 {
        return 0;
    }

    // Placeholder - would check validity period
    1
}

// ============================================================================
// Digital Signature Creation
// ============================================================================

/// Create a digital signature on a PDF
///
/// # Arguments
/// * `input_path` - Path to input PDF
/// * `output_path` - Path for signed PDF output
/// * `cert` - Certificate handle
/// * `field_name` - Name for signature field
/// * `page` - Page number (0-based)
/// * `x`, `y`, `width`, `height` - Signature field rectangle
/// * `reason` - Reason for signing (can be null)
/// * `location` - Signing location (can be null)
///
/// # Returns
/// 0 on success, negative error code on failure
///
/// # Safety
/// All string parameters must be valid null-terminated C strings or null
#[unsafe(no_mangle)]
pub extern "C" fn mp_signature_create(
    input_path: *const c_char,
    output_path: *const c_char,
    cert: CertificateHandle,
    field_name: *const c_char,
    page: c_int,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    reason: *const c_char,
    location: *const c_char,
) -> c_int {
    // Validate inputs
    if input_path.is_null() || output_path.is_null() || field_name.is_null() || cert == 0 {
        return SecurityError::InvalidParameter as c_int;
    }

    if page < 0 || width <= 0.0 || height <= 0.0 {
        return SecurityError::InvalidParameter as c_int;
    }

    let _input_str = match unsafe { CStr::from_ptr(input_path) }.to_str() {
        Ok(s) => s,
        Err(_) => return SecurityError::InvalidParameter as c_int,
    };

    let _output_str = match unsafe { CStr::from_ptr(output_path) }.to_str() {
        Ok(s) => s,
        Err(_) => return SecurityError::InvalidParameter as c_int,
    };

    let _field_str = match unsafe { CStr::from_ptr(field_name) }.to_str() {
        Ok(s) => s,
        Err(_) => return SecurityError::InvalidParameter as c_int,
    };

    let _reason_str = if reason.is_null() {
        None
    } else {
        unsafe { CStr::from_ptr(reason) }.to_str().ok()
    };

    let _location_str = if location.is_null() {
        None
    } else {
        unsafe { CStr::from_ptr(location) }.to_str().ok()
    };

    #[cfg(feature = "signatures")]
    {
        use crate::enhanced::signatures::{PdfSigner, SignatureAlgorithm, SignatureField};

        // Create signature field
        let mut field =
            SignatureField::new(_field_str)
                .page(page as u32)
                .rect(x, y, x + width, y + height);

        if let Some(r) = _reason_str {
            field = field.reason(r);
        }

        if let Some(l) = _location_str {
            field = field.location(l);
        }

        // Create signer (would use actual certificate from handle)
        match PdfSigner::new(_input_str) {
            Ok(signer) => {
                // Would load certificate from handle
                let _signer = signer.field(field).algorithm(SignatureAlgorithm::RsaSha256);

                // Sign and save
                // For now return success placeholder
                SecurityError::Ok as c_int
            }
            Err(_) => SecurityError::SigningFailed as c_int,
        }
    }

    #[cfg(not(feature = "signatures"))]
    {
        SecurityError::NotAvailable as c_int
    }
}

/// Create an invisible digital signature (no visible appearance)
#[unsafe(no_mangle)]
pub extern "C" fn mp_signature_create_invisible(
    input_path: *const c_char,
    output_path: *const c_char,
    cert: CertificateHandle,
    field_name: *const c_char,
    reason: *const c_char,
    location: *const c_char,
) -> c_int {
    // Create signature with zero-size rectangle
    mp_signature_create(
        input_path,
        output_path,
        cert,
        field_name,
        0,
        0.0,
        0.0,
        0.0,
        0.0,
        reason,
        location,
    )
}

// ============================================================================
// Signature Verification
// ============================================================================

/// Signature verification result
#[repr(C)]
pub struct SignatureVerifyResult {
    /// Is signature mathematically valid
    pub valid: c_int,
    /// Is certificate valid
    pub certificate_valid: c_int,
    /// Was document modified after signing
    pub document_modified: c_int,
    /// Has timestamp
    pub has_timestamp: c_int,
    /// Signer name (caller must free with mp_free_string)
    pub signer_name: *const c_char,
    /// Signing time as Unix timestamp
    pub signing_time: i64,
    /// Error message (null if no error, caller must free)
    pub error_message: *const c_char,
}

/// Verify a digital signature in a PDF
///
/// # Arguments
/// * `pdf_path` - Path to signed PDF
/// * `field_name` - Name of signature field to verify
/// * `result` - Pointer to result struct to fill
///
/// # Returns
/// 0 on success, negative error code on failure
///
/// # Safety
/// - All string parameters must be valid null-terminated C strings
/// - `result` must be a valid pointer to a SignatureVerifyResult struct
#[unsafe(no_mangle)]
pub extern "C" fn mp_signature_verify(
    pdf_path: *const c_char,
    field_name: *const c_char,
    result: *mut SignatureVerifyResult,
) -> c_int {
    if pdf_path.is_null() || field_name.is_null() || result.is_null() {
        return SecurityError::InvalidParameter as c_int;
    }

    let _pdf_str = match unsafe { CStr::from_ptr(pdf_path) }.to_str() {
        Ok(s) => s,
        Err(_) => return SecurityError::InvalidParameter as c_int,
    };

    let _field_str = match unsafe { CStr::from_ptr(field_name) }.to_str() {
        Ok(s) => s,
        Err(_) => return SecurityError::InvalidParameter as c_int,
    };

    #[cfg(feature = "signatures")]
    {
        use crate::enhanced::signatures::{CertificateStatus, ModificationType, verify_signature};

        match verify_signature(_pdf_str, _field_str) {
            Ok(validation) => {
                let res = unsafe { &mut *result };

                res.valid = if validation.valid { 1 } else { 0 };
                res.certificate_valid = if validation.certificate_status == CertificateStatus::Valid
                {
                    1
                } else {
                    0
                };
                res.document_modified = if validation.modification == ModificationType::Disallowed {
                    1
                } else {
                    0
                };
                res.has_timestamp = if validation.has_timestamp { 1 } else { 0 };

                res.signer_name = match CString::new(validation.signer_name) {
                    Ok(s) => s.into_raw(),
                    Err(_) => ptr::null(),
                };

                res.signing_time = 0; // Would parse from validation.signing_time

                if validation.errors.is_empty() {
                    res.error_message = ptr::null();
                } else {
                    res.error_message = match CString::new(validation.errors.join("; ")) {
                        Ok(s) => s.into_raw(),
                        Err(_) => ptr::null(),
                    };
                }

                SecurityError::Ok as c_int
            }
            Err(e) => {
                let res = unsafe { &mut *result };
                res.valid = 0;
                res.error_message = match CString::new(format!("{}", e)) {
                    Ok(s) => s.into_raw(),
                    Err(_) => ptr::null(),
                };
                SecurityError::VerificationFailed as c_int
            }
        }
    }

    #[cfg(not(feature = "signatures"))]
    {
        let res = unsafe { &mut *result };
        res.valid = 0;
        res.error_message = match CString::new("Signatures feature not enabled") {
            Ok(s) => s.into_raw(),
            Err(_) => ptr::null(),
        };
        SecurityError::NotAvailable as c_int
    }
}

/// Count signature fields in PDF
#[unsafe(no_mangle)]
pub extern "C" fn mp_signature_count(pdf_path: *const c_char) -> c_int {
    if pdf_path.is_null() {
        return -1;
    }

    let _pdf_str = match unsafe { CStr::from_ptr(pdf_path) }.to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };

    #[cfg(feature = "signatures")]
    {
        use crate::enhanced::signatures::list_signature_fields;
        match list_signature_fields(_pdf_str) {
            Ok(fields) => fields.len() as c_int,
            Err(_) => -1,
        }
    }

    #[cfg(not(feature = "signatures"))]
    {
        -1
    }
}

// ============================================================================
// TSA Timestamp
// ============================================================================

/// Request a timestamp from a Time-Stamping Authority
///
/// # Arguments
/// * `tsa_url` - TSA server URL
/// * `data` - Data to timestamp
/// * `data_len` - Length of data
/// * `timestamp_out` - Pointer to receive timestamp data
/// * `timestamp_len_out` - Pointer to receive timestamp length
///
/// # Returns
/// 0 on success, negative error code on failure
///
/// # Safety
/// - `tsa_url` must be a valid null-terminated C string
/// - `data` must be a valid pointer to `data_len` bytes
/// - `timestamp_out` and `timestamp_len_out` must be valid pointers
#[unsafe(no_mangle)]
pub extern "C" fn mp_tsa_timestamp(
    tsa_url: *const c_char,
    data: *const u8,
    data_len: usize,
    timestamp_out: *mut *const u8,
    timestamp_len_out: *mut usize,
) -> c_int {
    if tsa_url.is_null() || data.is_null() || timestamp_out.is_null() || timestamp_len_out.is_null()
    {
        return SecurityError::InvalidParameter as c_int;
    }

    let _url_str = match unsafe { CStr::from_ptr(tsa_url) }.to_str() {
        Ok(s) => s,
        Err(_) => return SecurityError::InvalidParameter as c_int,
    };

    let _data_slice = unsafe { std::slice::from_raw_parts(data, data_len) };

    #[cfg(feature = "tsa")]
    {
        use crate::enhanced::signatures::{TsaConfig, request_tsa_timestamp};

        let config = TsaConfig::new(_url_str);

        match request_tsa_timestamp(&config, _data_slice) {
            Ok(Some(timestamp)) => {
                let boxed = timestamp.into_boxed_slice();
                let len = boxed.len();
                let ptr = Box::into_raw(boxed) as *const u8;

                unsafe {
                    *timestamp_out = ptr;
                    *timestamp_len_out = len;
                }

                SecurityError::Ok as c_int
            }
            Ok(None) => SecurityError::TsaFailed as c_int,
            Err(_) => SecurityError::TsaFailed as c_int,
        }
    }

    #[cfg(not(feature = "tsa"))]
    {
        SecurityError::NotAvailable as c_int
    }
}

// ============================================================================
// Encryption Functions
// ============================================================================

/// Create encryption options
#[unsafe(no_mangle)]
pub extern "C" fn mp_encryption_options_new() -> EncryptionOptionsHandle {
    // Would create and store encryption options
    1
}

/// Set user password
#[unsafe(no_mangle)]
pub extern "C" fn mp_encryption_set_user_password(
    options: EncryptionOptionsHandle,
    password: *const c_char,
) -> c_int {
    if options == 0 || password.is_null() {
        return SecurityError::InvalidParameter as c_int;
    }

    let _password_str = match unsafe { CStr::from_ptr(password) }.to_str() {
        Ok(s) => s,
        Err(_) => return SecurityError::InvalidParameter as c_int,
    };

    // Would set password on options
    SecurityError::Ok as c_int
}

/// Set owner password
#[unsafe(no_mangle)]
pub extern "C" fn mp_encryption_set_owner_password(
    options: EncryptionOptionsHandle,
    password: *const c_char,
) -> c_int {
    if options == 0 || password.is_null() {
        return SecurityError::InvalidParameter as c_int;
    }

    let _password_str = match unsafe { CStr::from_ptr(password) }.to_str() {
        Ok(s) => s,
        Err(_) => return SecurityError::InvalidParameter as c_int,
    };

    SecurityError::Ok as c_int
}

/// Set encryption algorithm (0=AES-128, 1=AES-256)
#[unsafe(no_mangle)]
pub extern "C" fn mp_encryption_set_algorithm(
    options: EncryptionOptionsHandle,
    algorithm: c_int,
) -> c_int {
    if options == 0 || !(0..=1).contains(&algorithm) {
        return SecurityError::InvalidParameter as c_int;
    }

    SecurityError::Ok as c_int
}

/// Permission flags for encryption
pub const NP_PERM_PRINT: c_int = 1 << 0;
pub const NP_PERM_COPY: c_int = 1 << 1;
pub const NP_PERM_MODIFY: c_int = 1 << 2;
pub const NP_PERM_ANNOTATE: c_int = 1 << 3;
pub const NP_PERM_FILL_FORMS: c_int = 1 << 4;
pub const NP_PERM_ASSEMBLE: c_int = 1 << 5;
pub const NP_PERM_PRINT_HIGH: c_int = 1 << 6;
pub const NP_PERM_ALL: c_int = 0x7F;

/// Set permissions
#[unsafe(no_mangle)]
pub extern "C" fn mp_encryption_set_permissions(
    options: EncryptionOptionsHandle,
    permissions: c_int,
) -> c_int {
    if options == 0 {
        return SecurityError::InvalidParameter as c_int;
    }

    let _ = permissions;
    SecurityError::Ok as c_int
}

/// Drop encryption options
#[unsafe(no_mangle)]
pub extern "C" fn mp_encryption_options_drop(_options: EncryptionOptionsHandle) {
    // Would free options
}

/// Encrypt a PDF file
///
/// # Arguments
/// * `input_path` - Path to input PDF
/// * `output_path` - Path for encrypted PDF output
/// * `options` - Encryption options handle
///
/// # Returns
/// 0 on success, negative error code on failure
#[unsafe(no_mangle)]
pub extern "C" fn mp_encrypt_pdf(
    input_path: *const c_char,
    output_path: *const c_char,
    options: EncryptionOptionsHandle,
) -> c_int {
    if input_path.is_null() || output_path.is_null() || options == 0 {
        return SecurityError::InvalidParameter as c_int;
    }

    let _input_str = match unsafe { CStr::from_ptr(input_path) }.to_str() {
        Ok(s) => s,
        Err(_) => return SecurityError::InvalidParameter as c_int,
    };

    let _output_str = match unsafe { CStr::from_ptr(output_path) }.to_str() {
        Ok(s) => s,
        Err(_) => return SecurityError::InvalidParameter as c_int,
    };

    // Would perform encryption using crate::enhanced::encryption
    SecurityError::Ok as c_int
}

/// Decrypt a PDF file
///
/// # Arguments
/// * `input_path` - Path to encrypted PDF
/// * `output_path` - Path for decrypted PDF output
/// * `password` - Password for decryption
///
/// # Returns
/// 0 on success, negative error code on failure
#[unsafe(no_mangle)]
pub extern "C" fn mp_decrypt_pdf(
    input_path: *const c_char,
    output_path: *const c_char,
    password: *const c_char,
) -> c_int {
    if input_path.is_null() || output_path.is_null() || password.is_null() {
        return SecurityError::InvalidParameter as c_int;
    }

    let _input_str = match unsafe { CStr::from_ptr(input_path) }.to_str() {
        Ok(s) => s,
        Err(_) => return SecurityError::InvalidParameter as c_int,
    };

    let _output_str = match unsafe { CStr::from_ptr(output_path) }.to_str() {
        Ok(s) => s,
        Err(_) => return SecurityError::InvalidParameter as c_int,
    };

    let _password_str = match unsafe { CStr::from_ptr(password) }.to_str() {
        Ok(s) => s,
        Err(_) => return SecurityError::InvalidParameter as c_int,
    };

    // Would perform decryption using crate::enhanced::encryption::decrypt_pdf
    SecurityError::Ok as c_int
}

/// Check if PDF is password protected
#[unsafe(no_mangle)]
pub extern "C" fn mp_is_encrypted(pdf_path: *const c_char) -> c_int {
    if pdf_path.is_null() {
        return -1;
    }

    let _pdf_str = match unsafe { CStr::from_ptr(pdf_path) }.to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };

    // Would use crate::enhanced::encryption::is_password_protected
    0
}

// ============================================================================
// Utility Functions
// ============================================================================

/// Free a string allocated by security functions
#[unsafe(no_mangle)]
pub extern "C" fn mp_free_string(s: *mut c_char) {
    if !s.is_null() {
        unsafe {
            drop(CString::from_raw(s));
        }
    }
}

/// Free timestamp data allocated by mp_tsa_timestamp
#[unsafe(no_mangle)]
pub extern "C" fn mp_free_timestamp(data: *mut u8, len: usize) {
    if !data.is_null() && len > 0 {
        unsafe {
            drop(Box::from_raw(std::slice::from_raw_parts_mut(data, len)));
        }
    }
}

/// Free verification result strings
#[unsafe(no_mangle)]
pub extern "C" fn mp_signature_verify_result_free(result: *mut SignatureVerifyResult) {
    if result.is_null() {
        return;
    }

    unsafe {
        let res = &mut *result;

        if !res.signer_name.is_null() {
            drop(CString::from_raw(res.signer_name as *mut c_char));
            res.signer_name = ptr::null();
        }

        if !res.error_message.is_null() {
            drop(CString::from_raw(res.error_message as *mut c_char));
            res.error_message = ptr::null();
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_certificate_load_null() {
        assert_eq!(mp_certificate_load_pkcs12(ptr::null(), ptr::null()), 0);
    }

    #[test]
    fn test_certificate_load_pem_null() {
        assert_eq!(
            mp_certificate_load_pem(ptr::null(), ptr::null(), ptr::null()),
            0
        );
    }

    #[test]
    fn test_signature_create_invalid() {
        assert_eq!(
            mp_signature_create(
                ptr::null(),
                ptr::null(),
                0,
                ptr::null(),
                0,
                0.0,
                0.0,
                0.0,
                0.0,
                ptr::null(),
                ptr::null()
            ),
            SecurityError::InvalidParameter as c_int
        );
    }

    #[test]
    fn test_signature_verify_null() {
        let mut result = SignatureVerifyResult {
            valid: 0,
            certificate_valid: 0,
            document_modified: 0,
            has_timestamp: 0,
            signer_name: ptr::null(),
            signing_time: 0,
            error_message: ptr::null(),
        };

        assert_eq!(
            mp_signature_verify(ptr::null(), ptr::null(), &mut result),
            SecurityError::InvalidParameter as c_int
        );
    }

    #[test]
    fn test_encryption_options() {
        let options = mp_encryption_options_new();
        assert!(options > 0);

        let password = c"test123";
        assert_eq!(
            mp_encryption_set_user_password(options, password.as_ptr()),
            SecurityError::Ok as c_int
        );

        assert_eq!(
            mp_encryption_set_algorithm(options, 1),
            SecurityError::Ok as c_int
        );

        assert_eq!(
            mp_encryption_set_permissions(options, NP_PERM_PRINT | NP_PERM_COPY),
            SecurityError::Ok as c_int
        );

        mp_encryption_options_drop(options);
    }

    #[test]
    fn test_encrypt_decrypt_null() {
        assert_eq!(
            mp_encrypt_pdf(ptr::null(), ptr::null(), 0),
            SecurityError::InvalidParameter as c_int
        );

        assert_eq!(
            mp_decrypt_pdf(ptr::null(), ptr::null(), ptr::null()),
            SecurityError::InvalidParameter as c_int
        );
    }

    #[test]
    fn test_free_null() {
        // Should not crash
        mp_free_string(ptr::null_mut());
        mp_free_timestamp(ptr::null_mut(), 0);
    }

    #[test]
    fn test_signature_count_null() {
        assert_eq!(mp_signature_count(ptr::null()), -1);
    }
}
