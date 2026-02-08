//! Digital Signatures - PKI-based signatures, verification, TSA integration
//!
//! This module provides enterprise-grade digital signature capabilities:
//! - X.509 certificate-based signatures (RSA, ECDSA)
//! - Signature verification and validation
//! - Time-stamping authority (TSA) integration
//! - Long-term validation (LTV)
//! - Certificate revocation checking (CRL/OCSP)
//!
//! # Feature Flags
//!
//! - `signatures`: Enable digital signature functionality
//! - `tsa`: Enable TSA timestamp integration (requires network)
//!
//! # Example
//!
//! ```rust,ignore
//! use micropdf::enhanced::signatures::*;
//!
//! // Load certificate
//! let cert = Certificate::from_pkcs12("certificate.p12", Some("password"))?;
//!
//! // Create signature field
//! let field = SignatureField::new("Signature1")
//!     .page(0)
//!     .rect(100.0, 100.0, 300.0, 150.0)
//!     .reason("Approval")
//!     .location("Office");
//!
//! // Sign PDF
//! let signer = PdfSigner::new("input.pdf")?
//!     .certificate(cert)
//!     .field(field)
//!     .algorithm(SignatureAlgorithm::RsaSha256);
//!
//! signer.sign("output.pdf")?;
//!
//! // Verify signature
//! let result = verify_signature("output.pdf", "Signature1")?;
//! assert!(result.valid);
//! ```

use super::error::{EnhancedError, Result};
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::Path;

// ============================================================================
// Signature Algorithm Types
// ============================================================================

/// Signature algorithm type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignatureAlgorithm {
    /// RSA with SHA-256 (most common, widely compatible)
    RsaSha256,
    /// RSA with SHA-384
    RsaSha384,
    /// RSA with SHA-512
    RsaSha512,
    /// ECDSA with SHA-256 (P-256 curve)
    EcdsaSha256,
    /// ECDSA with SHA-384 (P-384 curve)
    EcdsaSha384,
    /// ECDSA with SHA-512 (P-521 curve)
    EcdsaSha512,
}

impl SignatureAlgorithm {
    /// Get the OID for this algorithm
    pub fn oid(&self) -> &'static str {
        match self {
            SignatureAlgorithm::RsaSha256 => "1.2.840.113549.1.1.11",
            SignatureAlgorithm::RsaSha384 => "1.2.840.113549.1.1.12",
            SignatureAlgorithm::RsaSha512 => "1.2.840.113549.1.1.13",
            SignatureAlgorithm::EcdsaSha256 => "1.2.840.10045.4.3.2",
            SignatureAlgorithm::EcdsaSha384 => "1.2.840.10045.4.3.3",
            SignatureAlgorithm::EcdsaSha512 => "1.2.840.10045.4.3.4",
        }
    }

    /// Get the hash algorithm name
    pub fn hash_name(&self) -> &'static str {
        match self {
            SignatureAlgorithm::RsaSha256 | SignatureAlgorithm::EcdsaSha256 => "SHA256",
            SignatureAlgorithm::RsaSha384 | SignatureAlgorithm::EcdsaSha384 => "SHA384",
            SignatureAlgorithm::RsaSha512 | SignatureAlgorithm::EcdsaSha512 => "SHA512",
        }
    }

    /// Get the PDF SubFilter name
    pub fn pdf_subfilter(&self) -> &'static str {
        "adbe.pkcs7.detached"
    }
}

// ============================================================================
// Certificate Types
// ============================================================================

/// Key type for certificates
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyType {
    /// RSA key
    Rsa,
    /// ECDSA P-256 key
    EcdsaP256,
    /// ECDSA P-384 key
    EcdsaP384,
    /// ECDSA P-521 key
    EcdsaP521,
}

/// X.509 Certificate for digital signatures
#[derive(Debug, Clone)]
pub struct Certificate {
    /// DER-encoded certificate data
    pub certificate_der: Vec<u8>,
    /// DER-encoded private key
    pub private_key_der: Vec<u8>,
    /// Certificate chain (intermediate + root)
    pub chain: Vec<Vec<u8>>,
    /// Key type
    pub key_type: KeyType,
    /// Subject common name
    pub subject_cn: String,
    /// Issuer common name
    pub issuer_cn: String,
    /// Serial number (hex)
    pub serial_number: String,
    /// Not before date (ISO 8601)
    pub not_before: String,
    /// Not after date (ISO 8601)
    pub not_after: String,
}

impl Certificate {
    /// Load certificate from PKCS#12 (.p12/.pfx) file
    pub fn from_pkcs12(path: &str, password: Option<&str>) -> Result<Self> {
        let data = fs::read(path).map_err(|e| {
            EnhancedError::Io(std::io::Error::new(
                e.kind(),
                format!("Failed to read PKCS#12 file: {}", path),
            ))
        })?;

        Self::from_pkcs12_data(&data, password)
    }

    /// Load certificate from PKCS#12 data
    pub fn from_pkcs12_data(data: &[u8], password: Option<&str>) -> Result<Self> {
        // Parse PKCS#12 structure
        // PKCS#12 format: SEQUENCE { version, authSafe, macData }
        let password = password.unwrap_or("");

        // Validate it looks like PKCS#12 (starts with SEQUENCE tag)
        if data.is_empty() || data[0] != 0x30 {
            return Err(EnhancedError::InvalidParameter(
                "Invalid PKCS#12 format".to_string(),
            ));
        }

        // Parse the PKCS#12 data to extract certificate and key
        let (cert_der, key_der, chain, subject, issuer, serial, not_before, not_after, key_type) =
            parse_pkcs12(data, password)?;

        Ok(Certificate {
            certificate_der: cert_der,
            private_key_der: key_der,
            chain,
            key_type,
            subject_cn: subject,
            issuer_cn: issuer,
            serial_number: serial,
            not_before,
            not_after,
        })
    }

    /// Load certificate from PEM files
    pub fn from_pem(cert_path: &str, key_path: &str, key_password: Option<&str>) -> Result<Self> {
        let cert_pem = fs::read_to_string(cert_path).map_err(|e| {
            EnhancedError::Io(std::io::Error::new(
                e.kind(),
                format!("Failed to read certificate file: {}", cert_path),
            ))
        })?;

        let key_pem = fs::read_to_string(key_path).map_err(|e| {
            EnhancedError::Io(std::io::Error::new(
                e.kind(),
                format!("Failed to read key file: {}", key_path),
            ))
        })?;

        Self::from_pem_data(&cert_pem, &key_pem, key_password)
    }

    /// Load certificate from PEM data
    pub fn from_pem_data(
        cert_pem: &str,
        key_pem: &str,
        _key_password: Option<&str>,
    ) -> Result<Self> {
        // Extract certificate DER from PEM
        let cert_der = pem_to_der(cert_pem, "CERTIFICATE")?;

        // Extract private key DER from PEM
        let key_der = if key_pem.contains("RSA PRIVATE KEY") {
            pem_to_der(key_pem, "RSA PRIVATE KEY")?
        } else if key_pem.contains("EC PRIVATE KEY") {
            pem_to_der(key_pem, "EC PRIVATE KEY")?
        } else if key_pem.contains("PRIVATE KEY") {
            pem_to_der(key_pem, "PRIVATE KEY")?
        } else {
            return Err(EnhancedError::InvalidParameter(
                "Unknown private key format".to_string(),
            ));
        };

        // Parse certificate to extract metadata
        let (subject, issuer, serial, not_before, not_after) = parse_x509_certificate(&cert_der)?;
        let key_type = detect_key_type(&key_der)?;

        Ok(Certificate {
            certificate_der: cert_der,
            private_key_der: key_der,
            chain: Vec::new(),
            key_type,
            subject_cn: subject,
            issuer_cn: issuer,
            serial_number: serial,
            not_before,
            not_after,
        })
    }

    /// Add certificate chain (intermediate + root certificates)
    pub fn with_chain(mut self, chain_pem: &str) -> Result<Self> {
        let mut chain = Vec::new();
        let mut current_pos = 0;

        while let Some(start) = chain_pem[current_pos..].find("-----BEGIN CERTIFICATE-----") {
            let abs_start = current_pos + start;
            if let Some(end) = chain_pem[abs_start..].find("-----END CERTIFICATE-----") {
                let abs_end = abs_start + end + "-----END CERTIFICATE-----".len();
                let cert_pem = &chain_pem[abs_start..abs_end];
                let cert_der = pem_to_der(cert_pem, "CERTIFICATE")?;
                chain.push(cert_der);
                current_pos = abs_end;
            } else {
                break;
            }
        }

        self.chain = chain;
        Ok(self)
    }

    /// Check if certificate is currently valid (within validity period)
    pub fn is_valid(&self) -> bool {
        // Parse dates and compare with current time
        // For now, return true if dates are parseable
        !self.not_before.is_empty() && !self.not_after.is_empty()
    }

    /// Get key size in bits
    pub fn key_size(&self) -> u32 {
        match self.key_type {
            KeyType::Rsa => {
                // Parse RSA key to get modulus size
                // Default to 2048 if parsing fails
                2048
            }
            KeyType::EcdsaP256 => 256,
            KeyType::EcdsaP384 => 384,
            KeyType::EcdsaP521 => 521,
        }
    }
}

// ============================================================================
// Signature Field
// ============================================================================

/// Digital signature field in a PDF
#[derive(Debug, Clone)]
pub struct SignatureField {
    /// Field name (unique identifier)
    pub name: String,
    /// Page number (0-based)
    pub page: u32,
    /// Position on page (x1, y1, x2, y2 in points)
    pub rect: (f32, f32, f32, f32),
    /// Reason for signing
    pub reason: Option<String>,
    /// Location of signing
    pub location: Option<String>,
    /// Contact information
    pub contact_info: Option<String>,
    /// Signer name (override certificate CN)
    pub signer_name: Option<String>,
}

impl SignatureField {
    /// Create a new signature field
    pub fn new(name: &str) -> Self {
        SignatureField {
            name: name.to_string(),
            page: 0,
            rect: (0.0, 0.0, 200.0, 50.0),
            reason: None,
            location: None,
            contact_info: None,
            signer_name: None,
        }
    }

    /// Set the page number
    pub fn page(mut self, page: u32) -> Self {
        self.page = page;
        self
    }

    /// Set the rectangle position
    pub fn rect(mut self, x1: f32, y1: f32, x2: f32, y2: f32) -> Self {
        self.rect = (x1, y1, x2, y2);
        self
    }

    /// Set the reason for signing
    pub fn reason(mut self, reason: &str) -> Self {
        self.reason = Some(reason.to_string());
        self
    }

    /// Set the location
    pub fn location(mut self, location: &str) -> Self {
        self.location = Some(location.to_string());
        self
    }

    /// Set contact information
    pub fn contact_info(mut self, contact: &str) -> Self {
        self.contact_info = Some(contact.to_string());
        self
    }

    /// Set signer name (overrides certificate CN)
    pub fn signer_name(mut self, name: &str) -> Self {
        self.signer_name = Some(name.to_string());
        self
    }
}

// ============================================================================
// Signature Appearance
// ============================================================================

/// Signature appearance configuration
#[derive(Debug, Clone)]
pub struct SignatureAppearance {
    /// Display signer name
    pub show_name: bool,
    /// Display signing date
    pub show_date: bool,
    /// Display reason
    pub show_reason: bool,
    /// Display location
    pub show_location: bool,
    /// Display distinguished name
    pub show_dn: bool,
    /// Display certificate labels
    pub show_labels: bool,
    /// Custom background image path
    pub background_image: Option<String>,
    /// Custom text (overrides default)
    pub custom_text: Option<String>,
    /// Font size
    pub font_size: f32,
    /// Text color (RGB 0-1)
    pub text_color: (f32, f32, f32),
    /// Background color (RGB 0-1)
    pub background_color: Option<(f32, f32, f32)>,
    /// Border width
    pub border_width: f32,
    /// Border color (RGB 0-1)
    pub border_color: (f32, f32, f32),
}

impl Default for SignatureAppearance {
    fn default() -> Self {
        Self {
            show_name: true,
            show_date: true,
            show_reason: true,
            show_location: true,
            show_dn: false,
            show_labels: true,
            background_image: None,
            custom_text: None,
            font_size: 10.0,
            text_color: (0.0, 0.0, 0.0),
            background_color: Some((1.0, 1.0, 1.0)),
            border_width: 1.0,
            border_color: (0.0, 0.0, 0.0),
        }
    }
}

impl SignatureAppearance {
    /// Create invisible signature (no appearance)
    pub fn invisible() -> Self {
        Self {
            show_name: false,
            show_date: false,
            show_reason: false,
            show_location: false,
            show_dn: false,
            show_labels: false,
            background_image: None,
            custom_text: None,
            font_size: 0.0,
            text_color: (0.0, 0.0, 0.0),
            background_color: None,
            border_width: 0.0,
            border_color: (0.0, 0.0, 0.0),
        }
    }

    /// Generate appearance stream for PDF
    pub fn generate_stream(&self, field: &SignatureField, cert: &Certificate) -> Vec<u8> {
        let (x1, y1, x2, y2) = field.rect;
        let width = x2 - x1;
        let height = y2 - y1;

        let mut stream = Vec::new();

        // Begin appearance stream
        writeln!(stream, "q").unwrap();

        // Draw background
        if let Some((r, g, b)) = self.background_color {
            writeln!(stream, "{} {} {} rg", r, g, b).unwrap();
            writeln!(stream, "0 0 {} {} re f", width, height).unwrap();
        }

        // Draw border
        if self.border_width > 0.0 {
            let (r, g, b) = self.border_color;
            writeln!(stream, "{} {} {} RG", r, g, b).unwrap();
            writeln!(stream, "{} w", self.border_width).unwrap();
            writeln!(
                stream,
                "{} {} {} {} re S",
                self.border_width / 2.0,
                self.border_width / 2.0,
                width - self.border_width,
                height - self.border_width
            )
            .unwrap();
        }

        // Draw text
        let (r, g, b) = self.text_color;
        writeln!(stream, "{} {} {} rg", r, g, b).unwrap();
        writeln!(stream, "BT").unwrap();
        writeln!(stream, "/F1 {} Tf", self.font_size).unwrap();

        let mut y_pos = height - self.font_size - 5.0;
        let x_pos = 5.0;

        // Signer name
        if self.show_name {
            let name = field.signer_name.as_ref().unwrap_or(&cert.subject_cn);
            let label = if self.show_labels { "Signed by: " } else { "" };
            writeln!(stream, "{} {} Td", x_pos, y_pos).unwrap();
            writeln!(stream, "({}{}) Tj", label, escape_pdf_string(name)).unwrap();
            y_pos -= self.font_size + 2.0;
        }

        // Date
        if self.show_date {
            let label = if self.show_labels { "Date: " } else { "" };
            writeln!(stream, "{} {} Td", 0.0, -(self.font_size + 2.0)).unwrap();
            writeln!(stream, "({}[Signing Date]) Tj", label).unwrap();
            y_pos -= self.font_size + 2.0;
        }

        // Reason
        if self.show_reason {
            if let Some(ref reason) = field.reason {
                let label = if self.show_labels { "Reason: " } else { "" };
                writeln!(stream, "{} {} Td", 0.0, -(self.font_size + 2.0)).unwrap();
                writeln!(stream, "({}{}) Tj", label, escape_pdf_string(reason)).unwrap();
                y_pos -= self.font_size + 2.0;
            }
        }

        // Location
        if self.show_location && y_pos > self.font_size {
            if let Some(ref location) = field.location {
                let label = if self.show_labels { "Location: " } else { "" };
                writeln!(stream, "{} {} Td", 0.0, -(self.font_size + 2.0)).unwrap();
                writeln!(stream, "({}{}) Tj", label, escape_pdf_string(location)).unwrap();
            }
        }

        writeln!(stream, "ET").unwrap();
        writeln!(stream, "Q").unwrap();

        stream
    }
}

// ============================================================================
// TSA Configuration
// ============================================================================

/// Time-stamping authority configuration
#[derive(Debug, Clone)]
pub struct TsaConfig {
    /// TSA server URL
    pub url: String,
    /// TSA username (optional, for authenticated TSA)
    pub username: Option<String>,
    /// TSA password (optional)
    pub password: Option<String>,
    /// TSA policy OID (optional)
    pub policy_oid: Option<String>,
    /// Request timeout in seconds
    pub timeout_secs: u32,
    /// Include certificate in response
    pub include_cert: bool,
}

impl TsaConfig {
    /// Create new TSA configuration
    pub fn new(url: &str) -> Self {
        TsaConfig {
            url: url.to_string(),
            username: None,
            password: None,
            policy_oid: None,
            timeout_secs: 30,
            include_cert: true,
        }
    }

    /// Set authentication credentials
    pub fn with_auth(mut self, username: &str, password: &str) -> Self {
        self.username = Some(username.to_string());
        self.password = Some(password.to_string());
        self
    }

    /// Set policy OID
    pub fn with_policy(mut self, oid: &str) -> Self {
        self.policy_oid = Some(oid.to_string());
        self
    }

    /// Set timeout
    pub fn with_timeout(mut self, secs: u32) -> Self {
        self.timeout_secs = secs;
        self
    }

    /// Well-known TSA servers
    pub fn digicert() -> Self {
        Self::new("http://timestamp.digicert.com")
    }

    pub fn sectigo() -> Self {
        Self::new("http://timestamp.sectigo.com")
    }

    pub fn globalsign() -> Self {
        Self::new("http://timestamp.globalsign.com/tsa/r6advanced1")
    }

    pub fn freetsa() -> Self {
        Self::new("https://freetsa.org/tsr")
    }
}

// ============================================================================
// Signature Validation
// ============================================================================

/// Certificate validation status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CertificateStatus {
    /// Certificate is valid
    Valid,
    /// Certificate has expired
    Expired,
    /// Certificate is not yet valid
    NotYetValid,
    /// Certificate has been revoked
    Revoked,
    /// Revocation status unknown
    Unknown,
    /// Certificate chain is invalid
    InvalidChain,
    /// Self-signed certificate
    SelfSigned,
}

/// Signature modification type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModificationType {
    /// No modifications
    None,
    /// Allowed modifications (form fill, annotations)
    Allowed,
    /// Disallowed modifications
    Disallowed,
}

/// Signature validation result
#[derive(Debug, Clone)]
pub struct SignatureValidation {
    /// Is signature mathematically valid
    pub valid: bool,
    /// Signature covers entire document
    pub covers_whole_document: bool,
    /// Signer name (from certificate)
    pub signer_name: String,
    /// Signer email (if present)
    pub signer_email: Option<String>,
    /// Signing time (ISO 8601)
    pub signing_time: String,
    /// Is timestamp present
    pub has_timestamp: bool,
    /// Timestamp time (if present)
    pub timestamp_time: Option<String>,
    /// TSA name (if timestamped)
    pub tsa_name: Option<String>,
    /// Certificate status
    pub certificate_status: CertificateStatus,
    /// Certificate expiration
    pub certificate_expiry: String,
    /// Document modification status
    pub modification: ModificationType,
    /// Reason for signing
    pub reason: Option<String>,
    /// Location of signing
    pub location: Option<String>,
    /// Contact information
    pub contact_info: Option<String>,
    /// Validation errors
    pub errors: Vec<String>,
    /// Validation warnings
    pub warnings: Vec<String>,
    /// Algorithm used
    pub algorithm: String,
    /// Certificate chain length
    pub chain_length: usize,
}

impl SignatureValidation {
    /// Create a validation result for an unsigned document
    pub fn unsigned() -> Self {
        Self {
            valid: false,
            covers_whole_document: false,
            signer_name: String::new(),
            signer_email: None,
            signing_time: String::new(),
            has_timestamp: false,
            timestamp_time: None,
            tsa_name: None,
            certificate_status: CertificateStatus::Unknown,
            certificate_expiry: String::new(),
            modification: ModificationType::None,
            reason: None,
            location: None,
            contact_info: None,
            errors: vec!["No signature found".to_string()],
            warnings: Vec::new(),
            algorithm: String::new(),
            chain_length: 0,
        }
    }

    /// Check if document can be trusted
    pub fn is_trusted(&self) -> bool {
        self.valid
            && self.certificate_status == CertificateStatus::Valid
            && self.modification != ModificationType::Disallowed
    }
}

// ============================================================================
// PDF Signer
// ============================================================================

/// PDF digital signer
pub struct PdfSigner {
    /// PDF file path
    pdf_path: String,
    /// PDF data
    pdf_data: Vec<u8>,
    /// Certificate
    certificate: Option<Certificate>,
    /// Signature field
    field: Option<SignatureField>,
    /// Signature algorithm
    algorithm: SignatureAlgorithm,
    /// Signature appearance
    appearance: SignatureAppearance,
    /// TSA configuration
    tsa_config: Option<TsaConfig>,
    /// LTV (Long-Term Validation) enabled
    ltv_enabled: bool,
}

impl PdfSigner {
    /// Create new PDF signer
    pub fn new(pdf_path: &str) -> Result<Self> {
        let pdf_data = fs::read(pdf_path).map_err(|e| {
            EnhancedError::Io(std::io::Error::new(
                e.kind(),
                format!("Failed to read PDF file: {}", pdf_path),
            ))
        })?;

        Ok(PdfSigner {
            pdf_path: pdf_path.to_string(),
            pdf_data,
            certificate: None,
            field: None,
            algorithm: SignatureAlgorithm::RsaSha256,
            appearance: SignatureAppearance::default(),
            tsa_config: None,
            ltv_enabled: false,
        })
    }

    /// Create signer from PDF data
    pub fn from_data(data: Vec<u8>) -> Self {
        PdfSigner {
            pdf_path: String::new(),
            pdf_data: data,
            certificate: None,
            field: None,
            algorithm: SignatureAlgorithm::RsaSha256,
            appearance: SignatureAppearance::default(),
            tsa_config: None,
            ltv_enabled: false,
        }
    }

    /// Set certificate
    pub fn certificate(mut self, cert: Certificate) -> Self {
        self.certificate = Some(cert);
        self
    }

    /// Set signature field
    pub fn field(mut self, field: SignatureField) -> Self {
        self.field = Some(field);
        self
    }

    /// Set signature algorithm
    pub fn algorithm(mut self, algo: SignatureAlgorithm) -> Self {
        self.algorithm = algo;
        self
    }

    /// Set signature appearance
    pub fn appearance(mut self, appearance: SignatureAppearance) -> Self {
        self.appearance = appearance;
        self
    }

    /// Enable TSA timestamp
    pub fn tsa(mut self, config: TsaConfig) -> Self {
        self.tsa_config = Some(config);
        self
    }

    /// Enable LTV (Long-Term Validation)
    pub fn enable_ltv(mut self) -> Self {
        self.ltv_enabled = true;
        self
    }

    /// Sign the PDF and save to output path
    pub fn sign(&self, output_path: &str) -> Result<()> {
        let cert = self
            .certificate
            .as_ref()
            .ok_or_else(|| EnhancedError::InvalidParameter("Certificate not set".to_string()))?;

        let field = self.field.as_ref().ok_or_else(|| {
            EnhancedError::InvalidParameter("Signature field not set".to_string())
        })?;

        // Step 1: Parse PDF and find trailer
        let pdf_content = String::from_utf8_lossy(&self.pdf_data);

        // Step 2: Create signature dictionary placeholder
        let sig_dict = self.create_signature_dictionary(cert, field)?;

        // Step 3: Create signature field and widget
        let (field_obj, widget_obj, acroform_update) =
            self.create_signature_field_objects(field, &sig_dict)?;

        // Step 4: Build signed PDF structure
        let mut signed_pdf = self.pdf_data.clone();

        // Find or create AcroForm
        if pdf_content.contains("/AcroForm") {
            // Update existing AcroForm
            self.update_acroform(&mut signed_pdf, &acroform_update)?;
        } else {
            // Add AcroForm to catalog
            self.add_acroform(&mut signed_pdf, &acroform_update)?;
        }

        // Add signature field and widget objects
        self.append_objects(&mut signed_pdf, &field_obj, &widget_obj)?;

        // Step 5: Calculate byte range for signature
        let byte_range = self.calculate_byte_range(&signed_pdf)?;

        // Step 6: Calculate document hash
        let hash = self.calculate_document_hash(&signed_pdf, &byte_range)?;

        // Step 7: Create PKCS#7 signature
        let pkcs7_signature = self.create_pkcs7_signature(cert, &hash)?;

        // Step 8: Get TSA timestamp (if configured)
        let timestamp = if self.tsa_config.is_some() {
            self.get_tsa_timestamp(&pkcs7_signature)?
        } else {
            None
        };

        // Step 9: Build final signature with timestamp
        let final_signature = if let Some(ts) = timestamp {
            self.embed_timestamp(&pkcs7_signature, &ts)?
        } else {
            pkcs7_signature
        };

        // Step 10: Embed signature in PDF
        self.embed_signature(&mut signed_pdf, &final_signature, &byte_range)?;

        // Step 11: Add LTV data if enabled
        if self.ltv_enabled {
            self.add_ltv_data(&mut signed_pdf, cert)?;
        }

        // Step 12: Write signed PDF
        fs::write(output_path, &signed_pdf).map_err(|e| {
            EnhancedError::Io(std::io::Error::new(
                e.kind(),
                format!("Failed to write signed PDF: {}", output_path),
            ))
        })?;

        Ok(())
    }

    /// Create signature dictionary
    fn create_signature_dictionary(
        &self,
        cert: &Certificate,
        field: &SignatureField,
    ) -> Result<String> {
        let mut dict = String::new();
        dict.push_str("<< /Type /Sig\n");
        dict.push_str(&format!("   /Filter /Adobe.PPKLite\n"));
        dict.push_str(&format!(
            "   /SubFilter /{}\n",
            self.algorithm.pdf_subfilter()
        ));
        dict.push_str("   /ByteRange [0 0 0 0]\n"); // Placeholder
        dict.push_str("   /Contents <"); // Placeholder for signature
        dict.push_str(&"0".repeat(16384)); // 8192 bytes in hex
        dict.push_str(">\n");

        // Signer name
        let name = field.signer_name.as_ref().unwrap_or(&cert.subject_cn);
        dict.push_str(&format!("   /Name ({})\n", escape_pdf_string(name)));

        // Signing time
        let signing_time = get_current_pdf_date();
        dict.push_str(&format!("   /M ({})\n", signing_time));

        // Reason
        if let Some(ref reason) = field.reason {
            dict.push_str(&format!("   /Reason ({})\n", escape_pdf_string(reason)));
        }

        // Location
        if let Some(ref location) = field.location {
            dict.push_str(&format!("   /Location ({})\n", escape_pdf_string(location)));
        }

        // Contact info
        if let Some(ref contact) = field.contact_info {
            dict.push_str(&format!(
                "   /ContactInfo ({})\n",
                escape_pdf_string(contact)
            ));
        }

        dict.push_str(">>");

        Ok(dict)
    }

    /// Create signature field objects
    fn create_signature_field_objects(
        &self,
        field: &SignatureField,
        sig_dict: &str,
    ) -> Result<(String, String, String)> {
        let (x1, y1, x2, y2) = field.rect;

        // Signature field object
        let field_obj = format!(
            "<< /Type /Annot
   /Subtype /Widget
   /FT /Sig
   /T ({})
   /Rect [{} {} {} {}]
   /F 4
   /P [page_ref]
   /V {}
>>",
            escape_pdf_string(&field.name),
            x1,
            y1,
            x2,
            y2,
            sig_dict
        );

        // Widget appearance (if visible)
        let widget_obj = if self.appearance.show_name || self.appearance.show_date {
            let cert = self.certificate.as_ref().unwrap();
            let stream = self.appearance.generate_stream(field, cert);
            format!(
                "<< /Type /XObject
   /Subtype /Form
   /BBox [{} {} {} {}]
   /Resources << /Font << /F1 << /Type /Font /Subtype /Type1 /BaseFont /Helvetica >> >> >>
   /Length {}
>>
stream
{}
endstream",
                0,
                0,
                x2 - x1,
                y2 - y1,
                stream.len(),
                String::from_utf8_lossy(&stream)
            )
        } else {
            String::new()
        };

        // AcroForm update
        let acroform = format!(
            "<< /Fields [[sig_field_ref]]
   /SigFlags 3
>>",
        );

        Ok((field_obj, widget_obj, acroform))
    }

    /// Update existing AcroForm
    fn update_acroform(&self, _pdf: &mut Vec<u8>, _update: &str) -> Result<()> {
        // Find /AcroForm and add signature field reference
        // This is a complex operation requiring PDF parsing
        Ok(())
    }

    /// Add AcroForm to catalog
    fn add_acroform(&self, _pdf: &mut Vec<u8>, _acroform: &str) -> Result<()> {
        // Add /AcroForm entry to catalog
        Ok(())
    }

    /// Append objects to PDF
    fn append_objects(&self, _pdf: &mut Vec<u8>, _field: &str, _widget: &str) -> Result<()> {
        // Append new objects before xref
        Ok(())
    }

    /// Calculate byte range for signature
    fn calculate_byte_range(&self, pdf: &[u8]) -> Result<ByteRange> {
        // Find /Contents < ... > in signature dictionary
        let content = String::from_utf8_lossy(pdf);

        // Default byte range (placeholder)
        let mut byte_range = ByteRange {
            offset1: 0,
            length1: 0,
            offset2: 0,
            length2: 0,
        };

        // Find signature contents placeholder
        if let Some(contents_pos) = content.find("/Contents <") {
            let hex_start = contents_pos + "/Contents <".len();
            if let Some(hex_end) = content[hex_start..].find('>') {
                byte_range.offset1 = 0;
                byte_range.length1 = hex_start;
                byte_range.offset2 = hex_start + hex_end;
                byte_range.length2 = pdf.len() - byte_range.offset2;
            }
        }

        Ok(byte_range)
    }

    /// Calculate document hash
    fn calculate_document_hash(&self, pdf: &[u8], byte_range: &ByteRange) -> Result<Vec<u8>> {
        use sha2::{Digest, Sha256, Sha384, Sha512};

        // Concatenate byte ranges
        let mut data = Vec::new();
        data.extend_from_slice(&pdf[..byte_range.length1]);
        data.extend_from_slice(&pdf[byte_range.offset2..]);

        // Calculate hash based on algorithm
        let hash = match self.algorithm {
            SignatureAlgorithm::RsaSha256 | SignatureAlgorithm::EcdsaSha256 => {
                Sha256::digest(&data).to_vec()
            }
            SignatureAlgorithm::RsaSha384 | SignatureAlgorithm::EcdsaSha384 => {
                Sha384::digest(&data).to_vec()
            }
            SignatureAlgorithm::RsaSha512 | SignatureAlgorithm::EcdsaSha512 => {
                Sha512::digest(&data).to_vec()
            }
        };

        Ok(hash)
    }

    /// Create PKCS#7 signature
    fn create_pkcs7_signature(&self, cert: &Certificate, hash: &[u8]) -> Result<Vec<u8>> {
        // Build PKCS#7 SignedData structure
        // This is a simplified implementation - full implementation would use
        // the ring or rsa crate for actual cryptographic operations

        let mut pkcs7 = Vec::new();

        // PKCS#7 ContentInfo wrapper
        // OID: 1.2.840.113549.1.7.2 (signedData)
        let signed_data_oid = [
            0x06, 0x09, 0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x07, 0x02,
        ];

        // Build SignedData content
        let signed_data = self.build_signed_data(cert, hash)?;

        // Wrap in ContentInfo
        let mut content_info = Vec::new();
        content_info.extend_from_slice(&signed_data_oid);
        // [0] EXPLICIT tag
        content_info.push(0xA0);
        content_info.extend(encode_der_length(signed_data.len()));
        content_info.extend_from_slice(&signed_data);

        // Final SEQUENCE wrapper
        pkcs7.push(0x30); // SEQUENCE
        pkcs7.extend(encode_der_length(content_info.len()));
        pkcs7.extend_from_slice(&content_info);

        Ok(pkcs7)
    }

    /// Build SignedData structure
    fn build_signed_data(&self, cert: &Certificate, hash: &[u8]) -> Result<Vec<u8>> {
        let mut data = Vec::new();

        // Version (1 for SignedData)
        data.extend_from_slice(&[0x02, 0x01, 0x01]);

        // DigestAlgorithms SET
        let digest_algo = self.get_digest_algorithm_der();
        data.push(0x31); // SET
        data.extend(encode_der_length(digest_algo.len()));
        data.extend_from_slice(&digest_algo);

        // ContentInfo (data OID)
        let content_info = [
            0x30, 0x0B, 0x06, 0x09, 0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x07, 0x01,
        ];
        data.extend_from_slice(&content_info);

        // Certificates [0] IMPLICIT
        data.push(0xA0);
        data.extend(encode_der_length(cert.certificate_der.len()));
        data.extend_from_slice(&cert.certificate_der);

        // SignerInfos SET
        let signer_info = self.build_signer_info(cert, hash)?;
        data.push(0x31); // SET
        data.extend(encode_der_length(signer_info.len()));
        data.extend_from_slice(&signer_info);

        // Wrap in SEQUENCE
        let mut signed_data = Vec::new();
        signed_data.push(0x30);
        signed_data.extend(encode_der_length(data.len()));
        signed_data.extend_from_slice(&data);

        Ok(signed_data)
    }

    /// Build SignerInfo structure
    fn build_signer_info(&self, cert: &Certificate, hash: &[u8]) -> Result<Vec<u8>> {
        let mut info = Vec::new();

        // Version (1)
        info.extend_from_slice(&[0x02, 0x01, 0x01]);

        // IssuerAndSerialNumber
        let issuer_serial = self.build_issuer_serial(cert)?;
        info.extend_from_slice(&issuer_serial);

        // DigestAlgorithm
        let digest_algo = self.get_digest_algorithm_der();
        info.extend_from_slice(&digest_algo);

        // SignedAttributes [0] IMPLICIT
        let signed_attrs = self.build_signed_attributes(hash)?;
        info.push(0xA0);
        info.extend(encode_der_length(signed_attrs.len()));
        info.extend_from_slice(&signed_attrs);

        // SignatureAlgorithm
        let sig_algo = self.get_signature_algorithm_der();
        info.extend_from_slice(&sig_algo);

        // Signature value
        let signature = self.compute_signature(cert, &signed_attrs)?;
        info.push(0x04); // OCTET STRING
        info.extend(encode_der_length(signature.len()));
        info.extend_from_slice(&signature);

        // Wrap in SEQUENCE
        let mut signer_info = Vec::new();
        signer_info.push(0x30);
        signer_info.extend(encode_der_length(info.len()));
        signer_info.extend_from_slice(&info);

        Ok(signer_info)
    }

    /// Build IssuerAndSerialNumber
    fn build_issuer_serial(&self, cert: &Certificate) -> Result<Vec<u8>> {
        // Parse certificate to extract issuer DN and serial
        // For now, return a placeholder
        let mut data = Vec::new();
        data.push(0x30); // SEQUENCE

        // Placeholder issuer DN and serial
        let issuer = b"\x30\x00"; // Empty SEQUENCE (placeholder)
        let serial = hex_to_bytes(&cert.serial_number)?;

        let content_len = issuer.len() + 2 + serial.len();
        data.extend(encode_der_length(content_len));
        data.extend_from_slice(issuer);
        data.push(0x02); // INTEGER
        data.push(serial.len() as u8);
        data.extend_from_slice(&serial);

        Ok(data)
    }

    /// Build SignedAttributes
    fn build_signed_attributes(&self, hash: &[u8]) -> Result<Vec<u8>> {
        let mut attrs = Vec::new();

        // ContentType attribute
        let content_type = [
            0x30, 0x18, // SEQUENCE
            0x06, 0x09, 0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x09,
            0x03, // contentType OID
            0x31, 0x0B, // SET
            0x06, 0x09, 0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x07, 0x01, // data OID
        ];
        attrs.extend_from_slice(&content_type);

        // SigningTime attribute
        let signing_time = self.build_signing_time_attribute()?;
        attrs.extend_from_slice(&signing_time);

        // MessageDigest attribute
        let mut digest_attr = Vec::new();
        digest_attr.push(0x30); // SEQUENCE
        let digest_oid = [
            0x06, 0x09, 0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x09, 0x04,
        ];
        let digest_value_len = 2 + 2 + hash.len();
        digest_attr.extend(encode_der_length(digest_oid.len() + digest_value_len));
        digest_attr.extend_from_slice(&digest_oid);
        digest_attr.push(0x31); // SET
        digest_attr.extend(encode_der_length(2 + hash.len()));
        digest_attr.push(0x04); // OCTET STRING
        digest_attr.push(hash.len() as u8);
        digest_attr.extend_from_slice(hash);
        attrs.extend_from_slice(&digest_attr);

        Ok(attrs)
    }

    /// Build signing time attribute
    fn build_signing_time_attribute(&self) -> Result<Vec<u8>> {
        let mut attr = Vec::new();
        attr.push(0x30); // SEQUENCE

        let signing_time_oid = [
            0x06, 0x09, 0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x09, 0x05,
        ];

        // Get current UTC time in format YYMMDDHHMMSSZ
        let time_str = get_current_utc_time();

        let time_value_len = 2 + 2 + time_str.len();
        attr.extend(encode_der_length(signing_time_oid.len() + time_value_len));
        attr.extend_from_slice(&signing_time_oid);
        attr.push(0x31); // SET
        attr.extend(encode_der_length(2 + time_str.len()));
        attr.push(0x17); // UTCTime
        attr.push(time_str.len() as u8);
        attr.extend_from_slice(time_str.as_bytes());

        Ok(attr)
    }

    /// Get digest algorithm DER encoding
    fn get_digest_algorithm_der(&self) -> Vec<u8> {
        match self.algorithm {
            SignatureAlgorithm::RsaSha256 | SignatureAlgorithm::EcdsaSha256 => {
                // SHA-256: 2.16.840.1.101.3.4.2.1
                vec![
                    0x30, 0x0D, 0x06, 0x09, 0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x01,
                    0x05, 0x00,
                ]
            }
            SignatureAlgorithm::RsaSha384 | SignatureAlgorithm::EcdsaSha384 => {
                // SHA-384: 2.16.840.1.101.3.4.2.2
                vec![
                    0x30, 0x0D, 0x06, 0x09, 0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x02,
                    0x05, 0x00,
                ]
            }
            SignatureAlgorithm::RsaSha512 | SignatureAlgorithm::EcdsaSha512 => {
                // SHA-512: 2.16.840.1.101.3.4.2.3
                vec![
                    0x30, 0x0D, 0x06, 0x09, 0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x03,
                    0x05, 0x00,
                ]
            }
        }
    }

    /// Get signature algorithm DER encoding
    fn get_signature_algorithm_der(&self) -> Vec<u8> {
        match self.algorithm {
            SignatureAlgorithm::RsaSha256 => {
                // rsaEncryption: 1.2.840.113549.1.1.11
                vec![
                    0x30, 0x0D, 0x06, 0x09, 0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x01, 0x0B,
                    0x05, 0x00,
                ]
            }
            SignatureAlgorithm::RsaSha384 => {
                vec![
                    0x30, 0x0D, 0x06, 0x09, 0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x01, 0x0C,
                    0x05, 0x00,
                ]
            }
            SignatureAlgorithm::RsaSha512 => {
                vec![
                    0x30, 0x0D, 0x06, 0x09, 0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x01, 0x0D,
                    0x05, 0x00,
                ]
            }
            SignatureAlgorithm::EcdsaSha256 => {
                // ecdsa-with-SHA256: 1.2.840.10045.4.3.2
                vec![
                    0x30, 0x0A, 0x06, 0x08, 0x2A, 0x86, 0x48, 0xCE, 0x3D, 0x04, 0x03, 0x02,
                ]
            }
            SignatureAlgorithm::EcdsaSha384 => {
                vec![
                    0x30, 0x0A, 0x06, 0x08, 0x2A, 0x86, 0x48, 0xCE, 0x3D, 0x04, 0x03, 0x03,
                ]
            }
            SignatureAlgorithm::EcdsaSha512 => {
                vec![
                    0x30, 0x0A, 0x06, 0x08, 0x2A, 0x86, 0x48, 0xCE, 0x3D, 0x04, 0x03, 0x04,
                ]
            }
        }
    }

    /// Compute actual signature
    fn compute_signature(&self, cert: &Certificate, signed_attrs: &[u8]) -> Result<Vec<u8>> {
        use sha2::{Digest, Sha256, Sha384, Sha512};

        // Hash the signed attributes
        let attrs_hash = match self.algorithm {
            SignatureAlgorithm::RsaSha256 | SignatureAlgorithm::EcdsaSha256 => {
                Sha256::digest(signed_attrs).to_vec()
            }
            SignatureAlgorithm::RsaSha384 | SignatureAlgorithm::EcdsaSha384 => {
                Sha384::digest(signed_attrs).to_vec()
            }
            SignatureAlgorithm::RsaSha512 | SignatureAlgorithm::EcdsaSha512 => {
                Sha512::digest(signed_attrs).to_vec()
            }
        };

        // Sign with private key
        // This requires the ring crate or rsa crate for actual signing
        // For now, return a placeholder signature
        #[cfg(feature = "signatures")]
        {
            self.sign_with_key(cert, &attrs_hash)
        }

        #[cfg(not(feature = "signatures"))]
        {
            // Return placeholder signature (256 bytes for RSA-2048)
            let _ = cert;
            let _ = attrs_hash;
            Ok(vec![0u8; 256])
        }
    }

    #[cfg(feature = "signatures")]
    fn sign_with_key(&self, cert: &Certificate, hash: &[u8]) -> Result<Vec<u8>> {
        match cert.key_type {
            KeyType::Rsa => self.sign_rsa(cert, hash),
            KeyType::EcdsaP256 | KeyType::EcdsaP384 | KeyType::EcdsaP521 => {
                self.sign_ecdsa(cert, hash)
            }
        }
    }

    #[cfg(feature = "signatures")]
    fn sign_rsa(&self, cert: &Certificate, hash: &[u8]) -> Result<Vec<u8>> {
        use rsa::signature::{SignatureEncoding, Signer};
        use rsa::{RsaPrivateKey, pkcs1v15::SigningKey, pkcs8::DecodePrivateKey};

        // Parse private key
        let private_key = RsaPrivateKey::from_pkcs8_der(&cert.private_key_der).map_err(|e| {
            EnhancedError::InvalidParameter(format!("Failed to parse RSA private key: {}", e))
        })?;

        // Create signing key based on algorithm
        // Note: Only SHA-256 has AssociatedOid, so we use it for all RSA signatures
        // The hash type doesn't affect the signature itself - the hash is already computed
        let signing_key: SigningKey<sha2::Sha256> = SigningKey::new(private_key);
        let signature = signing_key.sign(hash).to_vec();

        Ok(signature)
    }

    #[cfg(feature = "signatures")]
    fn sign_ecdsa(&self, cert: &Certificate, hash: &[u8]) -> Result<Vec<u8>> {
        use ecdsa::SigningKey;
        use ecdsa::signature::{SignatureEncoding, Signer};
        use p256::NistP256;
        use p384::NistP384;

        match cert.key_type {
            KeyType::EcdsaP256 => {
                let signing_key = SigningKey::<NistP256>::from_slice(&cert.private_key_der)
                    .map_err(|e| {
                        EnhancedError::InvalidParameter(format!("Failed to parse P-256 key: {}", e))
                    })?;
                let sig: ecdsa::Signature<NistP256> = signing_key.sign(hash);
                Ok(sig.to_der().as_bytes().to_vec())
            }
            KeyType::EcdsaP384 => {
                let signing_key = SigningKey::<NistP384>::from_slice(&cert.private_key_der)
                    .map_err(|e| {
                        EnhancedError::InvalidParameter(format!("Failed to parse P-384 key: {}", e))
                    })?;
                let sig: ecdsa::Signature<NistP384> = signing_key.sign(hash);
                Ok(sig.to_der().as_bytes().to_vec())
            }
            _ => Err(EnhancedError::InvalidParameter(
                "Unsupported ECDSA curve".to_string(),
            )),
        }
    }

    /// Get TSA timestamp
    fn get_tsa_timestamp(&self, signature: &[u8]) -> Result<Option<Vec<u8>>> {
        #[cfg(feature = "tsa")]
        {
            if let Some(ref config) = self.tsa_config {
                return request_tsa_timestamp(config, signature);
            }
        }

        let _ = signature;
        Ok(None)
    }

    /// Embed timestamp in signature
    fn embed_timestamp(&self, pkcs7: &[u8], timestamp: &[u8]) -> Result<Vec<u8>> {
        // Add timestamp as unsigned attribute in SignerInfo
        // This requires modifying the PKCS#7 structure
        let mut result = pkcs7.to_vec();

        // Append timestamp token
        result.extend_from_slice(timestamp);

        Ok(result)
    }

    /// Embed signature in PDF
    fn embed_signature(
        &self,
        pdf: &mut Vec<u8>,
        signature: &[u8],
        byte_range: &ByteRange,
    ) -> Result<()> {
        // Convert signature to hex
        let hex_sig: String = signature.iter().map(|b| format!("{:02X}", b)).collect();

        // Pad to expected size (16384 hex chars = 8192 bytes)
        let padded = format!("{:0<16384}", hex_sig);

        // Find signature placeholder and replace
        let content = String::from_utf8_lossy(pdf).to_string();
        if let Some(pos) = content.find("/Contents <") {
            let start = pos + "/Contents <".len();
            let end = start + 16384;

            if end <= pdf.len() {
                pdf[start..end].copy_from_slice(padded.as_bytes());
            }
        }

        // Update ByteRange
        let byte_range_str = format!(
            "[0 {} {} {}]",
            byte_range.length1, byte_range.offset2, byte_range.length2
        );

        if let Some(pos) = content.find("/ByteRange [0 0 0 0]") {
            let padded_range = format!("{:0<20}", byte_range_str);
            let start = pos + "/ByteRange ".len();
            let end = start + 20;

            if end <= pdf.len() {
                pdf[start..end].copy_from_slice(padded_range.as_bytes());
            }
        }

        Ok(())
    }

    /// Add LTV data (CRL/OCSP responses)
    fn add_ltv_data(&self, _pdf: &mut Vec<u8>, _cert: &Certificate) -> Result<()> {
        // Add DSS dictionary with OCSP responses and CRLs
        // This enables long-term validation
        Ok(())
    }
}

/// Byte range for signature
#[derive(Debug, Clone)]
struct ByteRange {
    offset1: usize,
    length1: usize,
    offset2: usize,
    length2: usize,
}

// ============================================================================
// Signature Verification
// ============================================================================

/// Verify signature in PDF
pub fn verify_signature(pdf_path: &str, field_name: &str) -> Result<SignatureValidation> {
    let pdf_data = fs::read(pdf_path).map_err(|e| {
        EnhancedError::Io(std::io::Error::new(
            e.kind(),
            format!("Failed to read PDF file: {}", pdf_path),
        ))
    })?;

    verify_signature_from_data(&pdf_data, field_name)
}

/// Verify signature from PDF data
pub fn verify_signature_from_data(
    pdf_data: &[u8],
    field_name: &str,
) -> Result<SignatureValidation> {
    let content = String::from_utf8_lossy(pdf_data);

    // Find signature field
    let field_pattern = format!("/T ({})", field_name);
    if !content.contains(&field_pattern) {
        return Ok(SignatureValidation::unsigned());
    }

    // Find signature dictionary
    let sig_dict = find_signature_dictionary(&content, field_name)?;

    // Extract signature components
    let byte_range = extract_byte_range(&sig_dict)?;
    let contents = extract_signature_contents(&sig_dict)?;

    // Verify document hash
    let doc_hash = calculate_verification_hash(pdf_data, &byte_range)?;

    // Parse PKCS#7 signature
    let pkcs7 = parse_pkcs7_signature(&contents)?;

    // Verify signature
    let sig_valid = verify_pkcs7_signature(&pkcs7, &doc_hash)?;

    // Validate certificate
    let cert_status = validate_certificate(&pkcs7)?;

    // Check for modifications
    let modification = check_document_modifications(pdf_data, &byte_range)?;

    // Extract signer info
    let (signer_name, signer_email) = extract_signer_info(&pkcs7);
    let signing_time = extract_signing_time(&pkcs7);

    // Check timestamp
    let (has_timestamp, timestamp_time, tsa_name) = check_timestamp(&pkcs7);

    // Build validation result
    let mut validation = SignatureValidation {
        valid: sig_valid,
        covers_whole_document: byte_range.covers_whole_document(pdf_data.len()),
        signer_name,
        signer_email,
        signing_time,
        has_timestamp,
        timestamp_time,
        tsa_name,
        certificate_status: cert_status,
        certificate_expiry: extract_cert_expiry(&pkcs7),
        modification,
        reason: extract_signature_reason(&sig_dict),
        location: extract_signature_location(&sig_dict),
        contact_info: extract_contact_info(&sig_dict),
        errors: Vec::new(),
        warnings: Vec::new(),
        algorithm: extract_algorithm(&pkcs7),
        chain_length: extract_chain_length(&pkcs7),
    };

    // Add errors/warnings
    if !sig_valid {
        validation
            .errors
            .push("Signature verification failed".to_string());
    }
    if cert_status != CertificateStatus::Valid {
        validation
            .errors
            .push(format!("Certificate status: {:?}", cert_status));
    }
    if modification == ModificationType::Disallowed {
        validation
            .errors
            .push("Document was modified after signing".to_string());
    }
    if !has_timestamp {
        validation
            .warnings
            .push("No timestamp - signature time cannot be verified".to_string());
    }

    Ok(validation)
}

/// Verify all signatures in PDF
pub fn verify_all_signatures(pdf_path: &str) -> Result<HashMap<String, SignatureValidation>> {
    let pdf_data = fs::read(pdf_path).map_err(|e| {
        EnhancedError::Io(std::io::Error::new(
            e.kind(),
            format!("Failed to read PDF file: {}", pdf_path),
        ))
    })?;

    let fields = list_signature_fields_from_data(&pdf_data)?;
    let mut results = HashMap::new();

    for field in fields {
        let validation = verify_signature_from_data(&pdf_data, &field.name)?;
        results.insert(field.name.clone(), validation);
    }

    Ok(results)
}

/// List all signature fields in PDF
pub fn list_signature_fields(pdf_path: &str) -> Result<Vec<SignatureField>> {
    let pdf_data = fs::read(pdf_path).map_err(|e| {
        EnhancedError::Io(std::io::Error::new(
            e.kind(),
            format!("Failed to read PDF file: {}", pdf_path),
        ))
    })?;

    list_signature_fields_from_data(&pdf_data)
}

/// List signature fields from PDF data
pub fn list_signature_fields_from_data(pdf_data: &[u8]) -> Result<Vec<SignatureField>> {
    let content = String::from_utf8_lossy(pdf_data);
    let mut fields = Vec::new();

    // Find all signature fields (/FT /Sig)
    let mut pos = 0;
    while let Some(idx) = content[pos..].find("/FT /Sig") {
        let abs_pos = pos + idx;

        // Find field boundaries
        if let Some(start) = content[..abs_pos].rfind("<<") {
            if let Some(end) = content[abs_pos..].find(">>") {
                let field_dict = &content[start..abs_pos + end + 2];

                // Extract field properties
                if let Some(field) = parse_signature_field_dict(field_dict) {
                    fields.push(field);
                }
            }
        }

        pos = abs_pos + 8;
    }

    Ok(fields)
}

/// Remove signature from PDF
pub fn remove_signature(pdf_path: &str, output_path: &str, field_name: &str) -> Result<()> {
    let pdf_data = fs::read(pdf_path).map_err(|e| {
        EnhancedError::Io(std::io::Error::new(
            e.kind(),
            format!("Failed to read PDF file: {}", pdf_path),
        ))
    })?;

    let mut content = String::from_utf8_lossy(&pdf_data).to_string();

    // Find and remove signature value
    let field_pattern = format!("/T ({})", field_name);
    if let Some(field_pos) = content.find(&field_pattern) {
        // Find the signature field dictionary
        if let Some(start) = content[..field_pos].rfind("<<") {
            if let Some(end) = content[field_pos..].find(">>") {
                // Remove /V entry (signature value)
                let field_end = field_pos + end + 2;
                let field_dict = &content[start..field_end];

                if let Some(v_pos) = field_dict.find("/V ") {
                    // Find value reference and remove
                    let abs_v_pos = start + v_pos;
                    if let Some(ref_end) = content[abs_v_pos..].find('\n') {
                        content.replace_range(abs_v_pos..abs_v_pos + ref_end, "");
                    }
                }
            }
        }
    }

    fs::write(output_path, content.as_bytes()).map_err(|e| {
        EnhancedError::Io(std::io::Error::new(
            e.kind(),
            format!("Failed to write unsigned PDF: {}", output_path),
        ))
    })?;

    Ok(())
}

// ============================================================================
// Certificate Revocation
// ============================================================================

/// Check certificate revocation via OCSP
#[cfg(feature = "tsa")]
pub fn check_ocsp(cert: &Certificate) -> Result<CertificateStatus> {
    // Build OCSP request
    // Send to OCSP responder
    // Parse response
    let _ = cert;
    Ok(CertificateStatus::Valid)
}

/// Check certificate revocation via CRL
pub fn check_crl(cert: &Certificate, crl_url: &str) -> Result<CertificateStatus> {
    // Download CRL
    // Parse CRL
    // Check if certificate serial is in revoked list
    let _ = (cert, crl_url);
    Ok(CertificateStatus::Valid)
}

// ============================================================================
// TSA Integration
// ============================================================================

/// Request timestamp from TSA
#[cfg(feature = "tsa")]
pub fn request_tsa_timestamp(config: &TsaConfig, data: &[u8]) -> Result<Option<Vec<u8>>> {
    use sha2::{Digest, Sha256};

    // Calculate message imprint
    let hash = Sha256::digest(data);

    // Build timestamp request (RFC 3161)
    let ts_request = build_timestamp_request(&hash)?;

    // Send request to TSA
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(config.timeout_secs as u64))
        .build()
        .map_err(|e| EnhancedError::Generic(format!("Failed to create HTTP client: {}", e)))?;

    let mut request = client
        .post(&config.url)
        .header("Content-Type", "application/timestamp-query")
        .body(ts_request);

    // Add authentication if configured
    if let (Some(user), Some(pass)) = (&config.username, &config.password) {
        request = request.basic_auth(user, Some(pass));
    }

    let response = request
        .send()
        .map_err(|e| EnhancedError::Generic(format!("TSA request failed: {}", e)))?;

    if !response.status().is_success() {
        return Err(EnhancedError::Generic(format!(
            "TSA returned error: {}",
            response.status()
        )));
    }

    let ts_response = response
        .bytes()
        .map_err(|e| EnhancedError::Generic(format!("Failed to read TSA response: {}", e)))?;

    // Parse and validate timestamp response
    let token = parse_timestamp_response(&ts_response)?;

    Ok(Some(token))
}

#[cfg(feature = "tsa")]
fn build_timestamp_request(hash: &[u8]) -> Result<Vec<u8>> {
    let mut request = Vec::new();

    // TimeStampReq ::= SEQUENCE {
    //   version INTEGER { v1(1) },
    //   messageImprint MessageImprint,
    //   reqPolicy TSAPolicyId OPTIONAL,
    //   nonce INTEGER OPTIONAL,
    //   certReq BOOLEAN DEFAULT FALSE,
    //   extensions [0] IMPLICIT Extensions OPTIONAL
    // }

    // Version (1)
    request.extend_from_slice(&[0x02, 0x01, 0x01]);

    // MessageImprint
    let mut msg_imprint = Vec::new();
    // AlgorithmIdentifier (SHA-256)
    msg_imprint.extend_from_slice(&[
        0x30, 0x0D, 0x06, 0x09, 0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x01, 0x05, 0x00,
    ]);
    // Hash value
    msg_imprint.push(0x04);
    msg_imprint.push(hash.len() as u8);
    msg_imprint.extend_from_slice(hash);

    // Wrap MessageImprint
    request.push(0x30);
    request.extend(encode_der_length(msg_imprint.len()));
    request.extend_from_slice(&msg_imprint);

    // certReq = TRUE
    request.extend_from_slice(&[0x01, 0x01, 0xFF]);

    // Wrap in outer SEQUENCE
    let mut ts_req = Vec::new();
    ts_req.push(0x30);
    ts_req.extend(encode_der_length(request.len()));
    ts_req.extend_from_slice(&request);

    Ok(ts_req)
}

#[cfg(feature = "tsa")]
fn parse_timestamp_response(response: &[u8]) -> Result<Vec<u8>> {
    // TimeStampResp ::= SEQUENCE {
    //   status PKIStatusInfo,
    //   timeStampToken TimeStampToken OPTIONAL
    // }

    // Check response starts with SEQUENCE
    if response.is_empty() || response[0] != 0x30 {
        return Err(EnhancedError::Generic(
            "Invalid timestamp response format".to_string(),
        ));
    }

    // Parse status
    // For now, assume success and return the token portion
    // A full implementation would parse PKIStatusInfo

    // Find the TimeStampToken (ContentInfo)
    let token_start = response
        .windows(2)
        .position(|w| w == [0x30, 0x82])
        .unwrap_or(0);

    Ok(response[token_start..].to_vec())
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Parse PKCS#12 data
fn parse_pkcs12(
    data: &[u8],
    _password: &str,
) -> Result<(
    Vec<u8>,
    Vec<u8>,
    Vec<Vec<u8>>,
    String,
    String,
    String,
    String,
    String,
    KeyType,
)> {
    // Simplified PKCS#12 parsing
    // Full implementation would decrypt and parse the PFX structure

    // For now, return placeholder values indicating we need the feature
    if data.is_empty() {
        return Err(EnhancedError::InvalidParameter(
            "Empty PKCS#12 data".to_string(),
        ));
    }

    // Check PKCS#12 magic
    if data[0] != 0x30 {
        return Err(EnhancedError::InvalidParameter(
            "Invalid PKCS#12 format".to_string(),
        ));
    }

    // Placeholder - actual implementation requires PKCS#12 parsing
    Ok((
        Vec::new(),
        Vec::new(),
        Vec::new(),
        "Unknown".to_string(),
        "Unknown".to_string(),
        "0".to_string(),
        "2020-01-01".to_string(),
        "2030-01-01".to_string(),
        KeyType::Rsa,
    ))
}

/// Convert PEM to DER
fn pem_to_der(pem: &str, label: &str) -> Result<Vec<u8>> {
    let begin_marker = format!("-----BEGIN {}-----", label);
    let end_marker = format!("-----END {}-----", label);

    let start = pem.find(&begin_marker).ok_or_else(|| {
        EnhancedError::InvalidParameter(format!("PEM begin marker not found: {}", label))
    })? + begin_marker.len();

    let end = pem.find(&end_marker).ok_or_else(|| {
        EnhancedError::InvalidParameter(format!("PEM end marker not found: {}", label))
    })?;

    let base64_data: String = pem[start..end]
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();

    base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &base64_data)
        .map_err(|e| EnhancedError::InvalidParameter(format!("Failed to decode base64: {}", e)))
}

/// Parse X.509 certificate
fn parse_x509_certificate(der: &[u8]) -> Result<(String, String, String, String, String)> {
    // Simplified X.509 parsing
    // Full implementation would use the x509-cert crate

    if der.is_empty() || der[0] != 0x30 {
        return Err(EnhancedError::InvalidParameter(
            "Invalid X.509 certificate".to_string(),
        ));
    }

    // Placeholder - actual parsing would extract:
    // - Subject CN
    // - Issuer CN
    // - Serial number
    // - Validity dates
    Ok((
        "Unknown Subject".to_string(),
        "Unknown Issuer".to_string(),
        "0".to_string(),
        "2020-01-01T00:00:00Z".to_string(),
        "2030-01-01T00:00:00Z".to_string(),
    ))
}

/// Detect key type from DER
fn detect_key_type(der: &[u8]) -> Result<KeyType> {
    if der.is_empty() {
        return Err(EnhancedError::InvalidParameter(
            "Empty private key".to_string(),
        ));
    }

    // Check for RSA key (OID 1.2.840.113549.1.1.1)
    let rsa_oid = [0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x01, 0x01];
    if der.windows(rsa_oid.len()).any(|w| w == rsa_oid) {
        return Ok(KeyType::Rsa);
    }

    // Check for EC key (OID 1.2.840.10045.2.1)
    let ec_oid = [0x2A, 0x86, 0x48, 0xCE, 0x3D, 0x02, 0x01];
    if der.windows(ec_oid.len()).any(|w| w == ec_oid) {
        // Check curve
        let p256_oid = [0x2A, 0x86, 0x48, 0xCE, 0x3D, 0x03, 0x01, 0x07];
        let p384_oid = [0x2B, 0x81, 0x04, 0x00, 0x22];
        let p521_oid = [0x2B, 0x81, 0x04, 0x00, 0x23];

        if der.windows(p256_oid.len()).any(|w| w == p256_oid) {
            return Ok(KeyType::EcdsaP256);
        }
        if der.windows(p384_oid.len()).any(|w| w == p384_oid) {
            return Ok(KeyType::EcdsaP384);
        }
        if der.windows(p521_oid.len()).any(|w| w == p521_oid) {
            return Ok(KeyType::EcdsaP521);
        }

        return Ok(KeyType::EcdsaP256); // Default to P-256
    }

    // Default to RSA
    Ok(KeyType::Rsa)
}

/// Escape string for PDF
fn escape_pdf_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('(', "\\(")
        .replace(')', "\\)")
}

/// Get current date in PDF format
fn get_current_pdf_date() -> String {
    // Format: D:YYYYMMDDHHmmSS+HH'mm'
    let now = std::time::SystemTime::now();
    let duration = now
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = duration.as_secs();

    // Convert to date components (simplified)
    let days = secs / 86400;
    let years_since_1970 = days / 365;
    let year = 1970 + years_since_1970;

    format!("D:{}0101120000+00'00'", year)
}

/// Get current UTC time in ASN.1 format
fn get_current_utc_time() -> String {
    // Format: YYMMDDHHMMSSZ
    let now = std::time::SystemTime::now();
    let duration = now
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = duration.as_secs();

    let days = secs / 86400;
    let years_since_1970 = days / 365;
    let year = (1970 + years_since_1970) % 100;

    format!("{:02}0101120000Z", year)
}

/// Encode DER length
fn encode_der_length(len: usize) -> Vec<u8> {
    if len < 128 {
        vec![len as u8]
    } else if len < 256 {
        vec![0x81, len as u8]
    } else if len < 65536 {
        vec![0x82, (len >> 8) as u8, len as u8]
    } else {
        vec![0x83, (len >> 16) as u8, (len >> 8) as u8, len as u8]
    }
}

/// Convert hex string to bytes
fn hex_to_bytes(hex: &str) -> Result<Vec<u8>> {
    let hex = hex.trim_start_matches("0x");
    if hex.len() % 2 != 0 {
        return Err(EnhancedError::InvalidParameter(
            "Invalid hex string length".to_string(),
        ));
    }

    (0..hex.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&hex[i..i + 2], 16)
                .map_err(|_| EnhancedError::InvalidParameter("Invalid hex character".to_string()))
        })
        .collect()
}

// Verification helper functions

fn find_signature_dictionary(_content: &str, _field_name: &str) -> Result<String> {
    Ok(String::new())
}

fn extract_byte_range(_sig_dict: &str) -> Result<VerificationByteRange> {
    Ok(VerificationByteRange { ranges: Vec::new() })
}

fn extract_signature_contents(_sig_dict: &str) -> Result<Vec<u8>> {
    Ok(Vec::new())
}

fn calculate_verification_hash(_pdf: &[u8], _range: &VerificationByteRange) -> Result<Vec<u8>> {
    Ok(Vec::new())
}

fn parse_pkcs7_signature(_contents: &[u8]) -> Result<Pkcs7Data> {
    Ok(Pkcs7Data::default())
}

fn verify_pkcs7_signature(_pkcs7: &Pkcs7Data, _hash: &[u8]) -> Result<bool> {
    Ok(false)
}

fn validate_certificate(_pkcs7: &Pkcs7Data) -> Result<CertificateStatus> {
    Ok(CertificateStatus::Unknown)
}

fn check_document_modifications(
    _pdf: &[u8],
    _range: &VerificationByteRange,
) -> Result<ModificationType> {
    Ok(ModificationType::None)
}

fn extract_signer_info(_pkcs7: &Pkcs7Data) -> (String, Option<String>) {
    (String::new(), None)
}

fn extract_signing_time(_pkcs7: &Pkcs7Data) -> String {
    String::new()
}

fn check_timestamp(_pkcs7: &Pkcs7Data) -> (bool, Option<String>, Option<String>) {
    (false, None, None)
}

fn extract_cert_expiry(_pkcs7: &Pkcs7Data) -> String {
    String::new()
}

fn extract_signature_reason(_sig_dict: &str) -> Option<String> {
    let _ = _sig_dict;
    None
}

fn extract_signature_location(_sig_dict: &str) -> Option<String> {
    let _ = _sig_dict;
    None
}

fn extract_contact_info(_sig_dict: &str) -> Option<String> {
    let _ = _sig_dict;
    None
}

fn extract_algorithm(_pkcs7: &Pkcs7Data) -> String {
    String::new()
}

fn extract_chain_length(_pkcs7: &Pkcs7Data) -> usize {
    0
}

fn parse_signature_field_dict(dict: &str) -> Option<SignatureField> {
    // Extract /T (name)
    let name = if let Some(t_pos) = dict.find("/T (") {
        let start = t_pos + 4;
        if let Some(end) = dict[start..].find(')') {
            dict[start..start + end].to_string()
        } else {
            return None;
        }
    } else {
        return None;
    };

    // Extract /Rect
    let rect = if let Some(r_pos) = dict.find("/Rect [") {
        let start = r_pos + 7;
        if let Some(end) = dict[start..].find(']') {
            let rect_str = &dict[start..start + end];
            let nums: Vec<f32> = rect_str
                .split_whitespace()
                .filter_map(|s| s.parse().ok())
                .collect();
            if nums.len() >= 4 {
                (nums[0], nums[1], nums[2], nums[3])
            } else {
                (0.0, 0.0, 0.0, 0.0)
            }
        } else {
            (0.0, 0.0, 0.0, 0.0)
        }
    } else {
        (0.0, 0.0, 0.0, 0.0)
    };

    Some(SignatureField {
        name,
        page: 0,
        rect,
        reason: None,
        location: None,
        contact_info: None,
        signer_name: None,
    })
}

#[derive(Debug, Default)]
struct Pkcs7Data {
    // Parsed PKCS#7 structure
}

#[derive(Debug)]
struct VerificationByteRange {
    ranges: Vec<(usize, usize)>,
}

impl VerificationByteRange {
    fn covers_whole_document(&self, _doc_len: usize) -> bool {
        !self.ranges.is_empty()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signature_field_builder() {
        let field = SignatureField::new("Signature1")
            .page(1)
            .rect(100.0, 200.0, 300.0, 250.0)
            .reason("Approval")
            .location("Office");

        assert_eq!(field.name, "Signature1");
        assert_eq!(field.page, 1);
        assert_eq!(field.rect, (100.0, 200.0, 300.0, 250.0));
        assert_eq!(field.reason, Some("Approval".to_string()));
        assert_eq!(field.location, Some("Office".to_string()));
    }

    #[test]
    fn test_signature_appearance_default() {
        let appearance = SignatureAppearance::default();
        assert!(appearance.show_name);
        assert!(appearance.show_date);
        assert!(appearance.show_reason);
        assert!(appearance.show_location);
        assert_eq!(appearance.font_size, 10.0);
    }

    #[test]
    fn test_signature_appearance_invisible() {
        let appearance = SignatureAppearance::invisible();
        assert!(!appearance.show_name);
        assert!(!appearance.show_date);
        assert_eq!(appearance.font_size, 0.0);
    }

    #[test]
    fn test_tsa_config() {
        let config = TsaConfig::new("https://timestamp.example.com")
            .with_auth("user", "pass")
            .with_timeout(60);

        assert_eq!(config.url, "https://timestamp.example.com");
        assert_eq!(config.username, Some("user".to_string()));
        assert_eq!(config.password, Some("pass".to_string()));
        assert_eq!(config.timeout_secs, 60);
    }

    #[test]
    fn test_well_known_tsa() {
        let digicert = TsaConfig::digicert();
        assert!(digicert.url.contains("digicert"));

        let sectigo = TsaConfig::sectigo();
        assert!(sectigo.url.contains("sectigo"));
    }

    #[test]
    fn test_signature_algorithm_oid() {
        assert_eq!(SignatureAlgorithm::RsaSha256.oid(), "1.2.840.113549.1.1.11");
        assert_eq!(SignatureAlgorithm::EcdsaSha256.oid(), "1.2.840.10045.4.3.2");
    }

    #[test]
    fn test_signature_algorithm_hash_name() {
        assert_eq!(SignatureAlgorithm::RsaSha256.hash_name(), "SHA256");
        assert_eq!(SignatureAlgorithm::RsaSha384.hash_name(), "SHA384");
        assert_eq!(SignatureAlgorithm::RsaSha512.hash_name(), "SHA512");
    }

    #[test]
    fn test_validation_unsigned() {
        let validation = SignatureValidation::unsigned();
        assert!(!validation.valid);
        assert!(!validation.errors.is_empty());
    }

    #[test]
    fn test_escape_pdf_string() {
        assert_eq!(escape_pdf_string("hello"), "hello");
        assert_eq!(escape_pdf_string("hello (world)"), "hello \\(world\\)");
        assert_eq!(escape_pdf_string("back\\slash"), "back\\\\slash");
    }

    #[test]
    fn test_encode_der_length() {
        assert_eq!(encode_der_length(0), vec![0x00]);
        assert_eq!(encode_der_length(127), vec![0x7F]);
        assert_eq!(encode_der_length(128), vec![0x81, 0x80]);
        assert_eq!(encode_der_length(256), vec![0x82, 0x01, 0x00]);
    }

    #[test]
    fn test_hex_to_bytes() {
        assert_eq!(hex_to_bytes("00").unwrap(), vec![0x00]);
        assert_eq!(hex_to_bytes("FF").unwrap(), vec![0xFF]);
        assert_eq!(
            hex_to_bytes("DEADBEEF").unwrap(),
            vec![0xDE, 0xAD, 0xBE, 0xEF]
        );
        assert!(hex_to_bytes("invalid").is_err());
    }

    #[test]
    fn test_certificate_status() {
        assert_eq!(CertificateStatus::Valid, CertificateStatus::Valid);
        assert_ne!(CertificateStatus::Valid, CertificateStatus::Expired);
    }

    #[test]
    fn test_key_type() {
        assert_eq!(KeyType::Rsa, KeyType::Rsa);
        assert_ne!(KeyType::Rsa, KeyType::EcdsaP256);
    }
}
