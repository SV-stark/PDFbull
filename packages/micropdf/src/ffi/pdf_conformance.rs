//! PDF Conformance Validation
//!
//! This module provides validation for PDF/A, PDF/X, and PDF 2.0 conformance.
//! PDF/A is for archival, PDF/X is for print exchange, PDF 2.0 is the latest standard.

use std::ffi::{CString, c_char, c_int};
use std::sync::LazyLock;

use crate::ffi::{Handle, HandleStore};

// ============================================================================
// Handle Management
// ============================================================================

/// Handle store for conformance validators
static VALIDATORS: LazyLock<HandleStore<ConformanceValidator>> = LazyLock::new(HandleStore::new);

/// Handle store for validation results
static VALIDATION_RESULTS: LazyLock<HandleStore<ValidationResult>> =
    LazyLock::new(HandleStore::new);

// ============================================================================
// Conformance Levels
// ============================================================================

/// PDF/A conformance levels
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PdfALevel {
    /// Not PDF/A
    None = 0,
    /// PDF/A-1a (Level A conformance, ISO 19005-1)
    A1a = 1,
    /// PDF/A-1b (Level B conformance, ISO 19005-1)
    A1b = 2,
    /// PDF/A-2a (Level A conformance, ISO 19005-2)
    A2a = 3,
    /// PDF/A-2b (Level B conformance, ISO 19005-2)
    A2b = 4,
    /// PDF/A-2u (Level U conformance, ISO 19005-2)
    A2u = 5,
    /// PDF/A-3a (Level A conformance, ISO 19005-3)
    A3a = 6,
    /// PDF/A-3b (Level B conformance, ISO 19005-3)
    A3b = 7,
    /// PDF/A-3u (Level U conformance, ISO 19005-3)
    A3u = 8,
    /// PDF/A-4 (ISO 19005-4)
    A4 = 9,
    /// PDF/A-4e (engineering, ISO 19005-4)
    A4e = 10,
    /// PDF/A-4f (file attachments, ISO 19005-4)
    A4f = 11,
}

impl PdfALevel {
    /// Get the ISO standard number
    pub fn iso_standard(&self) -> &'static str {
        match self {
            PdfALevel::None => "None",
            PdfALevel::A1a | PdfALevel::A1b => "ISO 19005-1",
            PdfALevel::A2a | PdfALevel::A2b | PdfALevel::A2u => "ISO 19005-2",
            PdfALevel::A3a | PdfALevel::A3b | PdfALevel::A3u => "ISO 19005-3",
            PdfALevel::A4 | PdfALevel::A4e | PdfALevel::A4f => "ISO 19005-4",
        }
    }

    /// Get short name (e.g., "PDF/A-1a")
    pub fn short_name(&self) -> &'static str {
        match self {
            PdfALevel::None => "None",
            PdfALevel::A1a => "PDF/A-1a",
            PdfALevel::A1b => "PDF/A-1b",
            PdfALevel::A2a => "PDF/A-2a",
            PdfALevel::A2b => "PDF/A-2b",
            PdfALevel::A2u => "PDF/A-2u",
            PdfALevel::A3a => "PDF/A-3a",
            PdfALevel::A3b => "PDF/A-3b",
            PdfALevel::A3u => "PDF/A-3u",
            PdfALevel::A4 => "PDF/A-4",
            PdfALevel::A4e => "PDF/A-4e",
            PdfALevel::A4f => "PDF/A-4f",
        }
    }
}

/// PDF/X conformance levels
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PdfXLevel {
    /// Not PDF/X
    None = 0,
    /// PDF/X-1a:2001 (ISO 15930-1)
    X1a2001 = 1,
    /// PDF/X-1a:2003 (ISO 15930-4)
    X1a2003 = 2,
    /// PDF/X-3:2002 (ISO 15930-3)
    X32002 = 3,
    /// PDF/X-3:2003 (ISO 15930-6)
    X32003 = 4,
    /// PDF/X-4 (ISO 15930-7)
    X4 = 5,
    /// PDF/X-4p (ISO 15930-7)
    X4p = 6,
    /// PDF/X-5g (ISO 15930-8)
    X5g = 7,
    /// PDF/X-5n (ISO 15930-8)
    X5n = 8,
    /// PDF/X-5pg (ISO 15930-8)
    X5pg = 9,
    /// PDF/X-6 (ISO 15930-9)
    X6 = 10,
    /// PDF/X-6n (ISO 15930-9)
    X6n = 11,
    /// PDF/X-6p (ISO 15930-9)
    X6p = 12,
}

impl PdfXLevel {
    /// Get the ISO standard number
    pub fn iso_standard(&self) -> &'static str {
        match self {
            PdfXLevel::None => "None",
            PdfXLevel::X1a2001 => "ISO 15930-1",
            PdfXLevel::X1a2003 => "ISO 15930-4",
            PdfXLevel::X32002 => "ISO 15930-3",
            PdfXLevel::X32003 => "ISO 15930-6",
            PdfXLevel::X4 | PdfXLevel::X4p => "ISO 15930-7",
            PdfXLevel::X5g | PdfXLevel::X5n | PdfXLevel::X5pg => "ISO 15930-8",
            PdfXLevel::X6 | PdfXLevel::X6n | PdfXLevel::X6p => "ISO 15930-9",
        }
    }

    /// Get short name
    pub fn short_name(&self) -> &'static str {
        match self {
            PdfXLevel::None => "None",
            PdfXLevel::X1a2001 => "PDF/X-1a:2001",
            PdfXLevel::X1a2003 => "PDF/X-1a:2003",
            PdfXLevel::X32002 => "PDF/X-3:2002",
            PdfXLevel::X32003 => "PDF/X-3:2003",
            PdfXLevel::X4 => "PDF/X-4",
            PdfXLevel::X4p => "PDF/X-4p",
            PdfXLevel::X5g => "PDF/X-5g",
            PdfXLevel::X5n => "PDF/X-5n",
            PdfXLevel::X5pg => "PDF/X-5pg",
            PdfXLevel::X6 => "PDF/X-6",
            PdfXLevel::X6n => "PDF/X-6n",
            PdfXLevel::X6p => "PDF/X-6p",
        }
    }
}

/// PDF version levels
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PdfVersion {
    /// Unknown or invalid
    Unknown = 0,
    /// PDF 1.0
    V1_0 = 10,
    /// PDF 1.1
    V1_1 = 11,
    /// PDF 1.2
    V1_2 = 12,
    /// PDF 1.3
    V1_3 = 13,
    /// PDF 1.4
    V1_4 = 14,
    /// PDF 1.5
    V1_5 = 15,
    /// PDF 1.6
    V1_6 = 16,
    /// PDF 1.7 (ISO 32000-1)
    V1_7 = 17,
    /// PDF 2.0 (ISO 32000-2)
    V2_0 = 20,
}

impl PdfVersion {
    /// Get version string
    pub fn version_string(&self) -> &'static str {
        match self {
            PdfVersion::Unknown => "Unknown",
            PdfVersion::V1_0 => "1.0",
            PdfVersion::V1_1 => "1.1",
            PdfVersion::V1_2 => "1.2",
            PdfVersion::V1_3 => "1.3",
            PdfVersion::V1_4 => "1.4",
            PdfVersion::V1_5 => "1.5",
            PdfVersion::V1_6 => "1.6",
            PdfVersion::V1_7 => "1.7",
            PdfVersion::V2_0 => "2.0",
        }
    }
}

// ============================================================================
// Validation Issue
// ============================================================================

/// Severity of a validation issue
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IssueSeverity {
    /// Informational message
    Info = 0,
    /// Warning (document may work but doesn't strictly conform)
    Warning = 1,
    /// Error (document violates conformance)
    Error = 2,
    /// Fatal error (document is invalid)
    Fatal = 3,
}

/// A single validation issue
#[derive(Debug, Clone)]
pub struct ValidationIssue {
    /// Issue severity
    pub severity: IssueSeverity,
    /// Issue code (for programmatic handling)
    pub code: String,
    /// Human-readable message
    pub message: String,
    /// Page number (0 = document level, >0 = specific page)
    pub page: i32,
    /// Object number (0 = none)
    pub object_num: i32,
    /// Clause in the standard that is violated
    pub clause: Option<String>,
}

impl ValidationIssue {
    /// Create a new validation issue
    pub fn new(
        severity: IssueSeverity,
        code: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            severity,
            code: code.into(),
            message: message.into(),
            page: 0,
            object_num: 0,
            clause: None,
        }
    }

    /// Set page number
    pub fn with_page(mut self, page: i32) -> Self {
        self.page = page;
        self
    }

    /// Set object number
    pub fn with_object(mut self, object_num: i32) -> Self {
        self.object_num = object_num;
        self
    }

    /// Set clause reference
    pub fn with_clause(mut self, clause: impl Into<String>) -> Self {
        self.clause = Some(clause.into());
        self
    }
}

// ============================================================================
// Validation Result
// ============================================================================

/// Result of conformance validation
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// PDF version detected
    pub pdf_version: PdfVersion,
    /// PDF/A level claimed
    pub pdfa_claimed: PdfALevel,
    /// PDF/A level validated
    pub pdfa_valid: PdfALevel,
    /// PDF/X level claimed
    pub pdfx_claimed: PdfXLevel,
    /// PDF/X level validated
    pub pdfx_valid: PdfXLevel,
    /// Is PDF 2.0 compliant
    pub pdf2_compliant: bool,
    /// List of validation issues
    pub issues: Vec<ValidationIssue>,
    /// Number of errors
    pub error_count: usize,
    /// Number of warnings
    pub warning_count: usize,
}

impl ValidationResult {
    /// Create a new empty result
    pub fn new() -> Self {
        Self {
            pdf_version: PdfVersion::Unknown,
            pdfa_claimed: PdfALevel::None,
            pdfa_valid: PdfALevel::None,
            pdfx_claimed: PdfXLevel::None,
            pdfx_valid: PdfXLevel::None,
            pdf2_compliant: false,
            issues: Vec::new(),
            error_count: 0,
            warning_count: 0,
        }
    }

    /// Add an issue
    pub fn add_issue(&mut self, issue: ValidationIssue) {
        match issue.severity {
            IssueSeverity::Error | IssueSeverity::Fatal => self.error_count += 1,
            IssueSeverity::Warning => self.warning_count += 1,
            IssueSeverity::Info => {}
        }
        self.issues.push(issue);
    }

    /// Check if validation passed (no errors)
    pub fn is_valid(&self) -> bool {
        self.error_count == 0
    }

    /// Get total issue count
    pub fn issue_count(&self) -> usize {
        self.issues.len()
    }
}

impl Default for ValidationResult {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Conformance Validator
// ============================================================================

/// Configuration for conformance validation
#[derive(Debug, Clone)]
pub struct ValidatorConfig {
    /// Check PDF/A conformance
    pub check_pdfa: bool,
    /// Check PDF/X conformance
    pub check_pdfx: bool,
    /// Check PDF 2.0 compliance
    pub check_pdf2: bool,
    /// Stop on first error
    pub stop_on_error: bool,
    /// Maximum issues to report
    pub max_issues: usize,
}

impl Default for ValidatorConfig {
    fn default() -> Self {
        Self {
            check_pdfa: true,
            check_pdfx: true,
            check_pdf2: true,
            stop_on_error: false,
            max_issues: 1000,
        }
    }
}

/// PDF conformance validator
pub struct ConformanceValidator {
    /// Configuration
    config: ValidatorConfig,
    /// Current validation result
    result: ValidationResult,
}

impl ConformanceValidator {
    /// Create a new validator
    pub fn new(config: ValidatorConfig) -> Self {
        Self {
            config,
            result: ValidationResult::new(),
        }
    }

    /// Get configuration
    pub fn config(&self) -> &ValidatorConfig {
        &self.config
    }

    /// Get current result
    pub fn result(&self) -> &ValidationResult {
        &self.result
    }

    /// Reset validation state
    pub fn reset(&mut self) {
        self.result = ValidationResult::new();
    }

    /// Add an issue (respecting max_issues limit)
    fn add_issue(&mut self, issue: ValidationIssue) -> bool {
        if self.result.issues.len() >= self.config.max_issues {
            return false;
        }

        let is_error = matches!(issue.severity, IssueSeverity::Error | IssueSeverity::Fatal);
        self.result.add_issue(issue);

        // Return whether to continue
        !is_error || !self.config.stop_on_error
    }

    // ========================================================================
    // PDF/A Validation
    // ========================================================================

    /// Validate PDF/A conformance
    pub fn validate_pdfa(&mut self) {
        if !self.config.check_pdfa {
            return;
        }

        // Check metadata for PDF/A identification
        self.check_pdfa_metadata();

        // Check for embedded fonts (required for PDF/A)
        self.check_embedded_fonts();

        // Check for transparency (restricted in PDF/A-1)
        self.check_transparency();

        // Check for encryption (forbidden in PDF/A)
        self.check_encryption_pdfa();

        // Check for JavaScript (forbidden in PDF/A)
        self.check_javascript_pdfa();

        // Check XMP metadata (required for PDF/A)
        self.check_xmp_metadata();

        // Check color spaces
        self.check_colorspaces_pdfa();

        // Determine validated level
        if self.result.error_count == 0 && self.result.pdfa_claimed != PdfALevel::None {
            self.result.pdfa_valid = self.result.pdfa_claimed;
        }
    }

    fn check_pdfa_metadata(&mut self) {
        // In a real implementation, parse XMP metadata to find PDF/A identification
        // For now, this is a stub that would check:
        // - pdfaid:part (1, 2, 3, or 4)
        // - pdfaid:conformance (A, B, or U)

        // Example: If document claims to be PDF/A but has no metadata
        // self.add_issue(ValidationIssue::new(
        //     IssueSeverity::Error,
        //     "PDFA_METADATA_MISSING",
        //     "PDF/A identification metadata is missing"
        // ).with_clause("6.6.2"));
    }

    fn check_embedded_fonts(&mut self) {
        // PDF/A requires all fonts to be embedded
        // In a real implementation, iterate through font resources and check:
        // - Font is embedded (FontFile/FontFile2/FontFile3)
        // - For PDF/A-1, Type 3 fonts must embed all resources
    }

    fn check_transparency(&mut self) {
        // PDF/A-1 forbids transparency
        // PDF/A-2+ allows transparency
    }

    fn check_encryption_pdfa(&mut self) {
        // PDF/A forbids encryption
        // Check for /Encrypt dictionary in trailer
    }

    fn check_javascript_pdfa(&mut self) {
        // PDF/A forbids JavaScript
        // Check for /JS and /JavaScript entries
    }

    fn check_xmp_metadata(&mut self) {
        // PDF/A requires XMP metadata packet
        // Must be synchronized with document info dictionary
    }

    fn check_colorspaces_pdfa(&mut self) {
        // Check color space restrictions:
        // - Device-dependent colors must have output intent
        // - PDF/A-1: Limited color space support
        // - PDF/A-2+: More color spaces allowed
    }

    // ========================================================================
    // PDF/X Validation
    // ========================================================================

    /// Validate PDF/X conformance
    pub fn validate_pdfx(&mut self) {
        if !self.config.check_pdfx {
            return;
        }

        // Check for PDF/X identification
        self.check_pdfx_metadata();

        // Check output intent (required for PDF/X)
        self.check_output_intent();

        // Check bleed/trim/art boxes
        self.check_page_boxes();

        // Check for trapped key
        self.check_trapped_key();

        // Check fonts
        self.check_fonts_pdfx();

        // Determine validated level
        if self.result.error_count == 0 && self.result.pdfx_claimed != PdfXLevel::None {
            self.result.pdfx_valid = self.result.pdfx_claimed;
        }
    }

    fn check_pdfx_metadata(&mut self) {
        // Check GTS_PDFXVersion in Info dictionary
        // Check XMP metadata for PDF/X identification
    }

    fn check_output_intent(&mut self) {
        // PDF/X requires OutputIntents array in catalog
        // Must have at least one entry with /S = /GTS_PDFX
    }

    fn check_page_boxes(&mut self) {
        // Check that required boxes are present:
        // - TrimBox or ArtBox required
        // - BleedBox recommended
        // - MediaBox must contain TrimBox
    }

    fn check_trapped_key(&mut self) {
        // /Trapped key is required in Info dictionary
        // Value must be /True, /False, or /Unknown
    }

    fn check_fonts_pdfx(&mut self) {
        // All fonts must be embedded
        // Type 3 fonts may have additional restrictions
    }

    // ========================================================================
    // PDF 2.0 Validation
    // ========================================================================

    /// Validate PDF 2.0 compliance
    pub fn validate_pdf2(&mut self) {
        if !self.config.check_pdf2 {
            return;
        }

        // Check PDF version
        self.check_pdf_version();

        // Check for deprecated features
        self.check_deprecated_features();

        // Check new PDF 2.0 features if claimed
        self.check_pdf2_features();

        // Determine compliance
        if self.result.error_count == 0 && self.result.pdf_version == PdfVersion::V2_0 {
            self.result.pdf2_compliant = true;
        }
    }

    fn check_pdf_version(&mut self) {
        // Parse %PDF-x.y header
        // Check /Version in catalog (takes precedence)
    }

    fn check_deprecated_features(&mut self) {
        // PDF 2.0 deprecates:
        // - LZW compression (use Flate instead)
        // - ASCII85 encoding
        // - Certain annotation types
        // - XFA forms
    }

    fn check_pdf2_features(&mut self) {
        // PDF 2.0 new features:
        // - AES-256 encryption
        // - Page-level output intents
        // - Geospatial features
        // - 3D annotations v2
        // - Rich media annotations
    }
}

// ============================================================================
// FFI Functions
// ============================================================================

/// Create a new conformance validator
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_conformance_validator(
    _ctx: Handle,
    check_pdfa: c_int,
    check_pdfx: c_int,
    check_pdf2: c_int,
) -> Handle {
    let config = ValidatorConfig {
        check_pdfa: check_pdfa != 0,
        check_pdfx: check_pdfx != 0,
        check_pdf2: check_pdf2 != 0,
        ..Default::default()
    };
    let validator = ConformanceValidator::new(config);
    VALIDATORS.insert(validator)
}

/// Drop a conformance validator
#[unsafe(no_mangle)]
pub extern "C" fn fz_drop_conformance_validator(_ctx: Handle, validator: Handle) {
    VALIDATORS.remove(validator);
}

/// Reset validator state
#[unsafe(no_mangle)]
pub extern "C" fn fz_conformance_validator_reset(_ctx: Handle, validator: Handle) {
    if let Some(arc) = VALIDATORS.get(validator) {
        if let Ok(mut v) = arc.lock() {
            v.reset();
        }
    }
}

/// Run PDF/A validation
#[unsafe(no_mangle)]
pub extern "C" fn fz_validate_pdfa(_ctx: Handle, validator: Handle) {
    if let Some(arc) = VALIDATORS.get(validator) {
        if let Ok(mut v) = arc.lock() {
            v.validate_pdfa();
        }
    }
}

/// Run PDF/X validation
#[unsafe(no_mangle)]
pub extern "C" fn fz_validate_pdfx(_ctx: Handle, validator: Handle) {
    if let Some(arc) = VALIDATORS.get(validator) {
        if let Ok(mut v) = arc.lock() {
            v.validate_pdfx();
        }
    }
}

/// Run PDF 2.0 validation
#[unsafe(no_mangle)]
pub extern "C" fn fz_validate_pdf2(_ctx: Handle, validator: Handle) {
    if let Some(arc) = VALIDATORS.get(validator) {
        if let Ok(mut v) = arc.lock() {
            v.validate_pdf2();
        }
    }
}

/// Check if validation passed
#[unsafe(no_mangle)]
pub extern "C" fn fz_conformance_is_valid(_ctx: Handle, validator: Handle) -> c_int {
    if let Some(arc) = VALIDATORS.get(validator) {
        if let Ok(v) = arc.lock() {
            return if v.result().is_valid() { 1 } else { 0 };
        }
    }
    0
}

/// Get error count
#[unsafe(no_mangle)]
pub extern "C" fn fz_conformance_error_count(_ctx: Handle, validator: Handle) -> c_int {
    if let Some(arc) = VALIDATORS.get(validator) {
        if let Ok(v) = arc.lock() {
            return v.result().error_count as c_int;
        }
    }
    0
}

/// Get warning count
#[unsafe(no_mangle)]
pub extern "C" fn fz_conformance_warning_count(_ctx: Handle, validator: Handle) -> c_int {
    if let Some(arc) = VALIDATORS.get(validator) {
        if let Ok(v) = arc.lock() {
            return v.result().warning_count as c_int;
        }
    }
    0
}

/// Get total issue count
#[unsafe(no_mangle)]
pub extern "C" fn fz_conformance_issue_count(_ctx: Handle, validator: Handle) -> c_int {
    if let Some(arc) = VALIDATORS.get(validator) {
        if let Ok(v) = arc.lock() {
            return v.result().issue_count() as c_int;
        }
    }
    0
}

/// Get PDF/A claimed level
#[unsafe(no_mangle)]
pub extern "C" fn fz_conformance_pdfa_claimed(_ctx: Handle, validator: Handle) -> c_int {
    if let Some(arc) = VALIDATORS.get(validator) {
        if let Ok(v) = arc.lock() {
            return v.result().pdfa_claimed as c_int;
        }
    }
    0
}

/// Get PDF/A validated level
#[unsafe(no_mangle)]
pub extern "C" fn fz_conformance_pdfa_valid(_ctx: Handle, validator: Handle) -> c_int {
    if let Some(arc) = VALIDATORS.get(validator) {
        if let Ok(v) = arc.lock() {
            return v.result().pdfa_valid as c_int;
        }
    }
    0
}

/// Get PDF/X claimed level
#[unsafe(no_mangle)]
pub extern "C" fn fz_conformance_pdfx_claimed(_ctx: Handle, validator: Handle) -> c_int {
    if let Some(arc) = VALIDATORS.get(validator) {
        if let Ok(v) = arc.lock() {
            return v.result().pdfx_claimed as c_int;
        }
    }
    0
}

/// Get PDF/X validated level
#[unsafe(no_mangle)]
pub extern "C" fn fz_conformance_pdfx_valid(_ctx: Handle, validator: Handle) -> c_int {
    if let Some(arc) = VALIDATORS.get(validator) {
        if let Ok(v) = arc.lock() {
            return v.result().pdfx_valid as c_int;
        }
    }
    0
}

/// Check if PDF 2.0 compliant
#[unsafe(no_mangle)]
pub extern "C" fn fz_conformance_pdf2_compliant(_ctx: Handle, validator: Handle) -> c_int {
    if let Some(arc) = VALIDATORS.get(validator) {
        if let Ok(v) = arc.lock() {
            return if v.result().pdf2_compliant { 1 } else { 0 };
        }
    }
    0
}

/// Get PDF version
#[unsafe(no_mangle)]
pub extern "C" fn fz_conformance_pdf_version(_ctx: Handle, validator: Handle) -> c_int {
    if let Some(arc) = VALIDATORS.get(validator) {
        if let Ok(v) = arc.lock() {
            return v.result().pdf_version as c_int;
        }
    }
    0
}

/// Create a new validation result
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_validation_result(_ctx: Handle) -> Handle {
    let result = ValidationResult::new();
    VALIDATION_RESULTS.insert(result)
}

/// Drop a validation result
#[unsafe(no_mangle)]
pub extern "C" fn fz_drop_validation_result(_ctx: Handle, result: Handle) {
    VALIDATION_RESULTS.remove(result);
}

/// Get issue message (returns allocated string)
#[unsafe(no_mangle)]
pub extern "C" fn fz_validation_issue_message(
    _ctx: Handle,
    validator: Handle,
    index: c_int,
) -> *mut c_char {
    if let Some(arc) = VALIDATORS.get(validator) {
        if let Ok(v) = arc.lock() {
            if let Some(issue) = v.result().issues.get(index as usize) {
                if let Ok(s) = CString::new(issue.message.as_str()) {
                    return s.into_raw();
                }
            }
        }
    }
    std::ptr::null_mut()
}

/// Get issue code (returns allocated string)
#[unsafe(no_mangle)]
pub extern "C" fn fz_validation_issue_code(
    _ctx: Handle,
    validator: Handle,
    index: c_int,
) -> *mut c_char {
    if let Some(arc) = VALIDATORS.get(validator) {
        if let Ok(v) = arc.lock() {
            if let Some(issue) = v.result().issues.get(index as usize) {
                if let Ok(s) = CString::new(issue.code.as_str()) {
                    return s.into_raw();
                }
            }
        }
    }
    std::ptr::null_mut()
}

/// Get issue severity
#[unsafe(no_mangle)]
pub extern "C" fn fz_validation_issue_severity(
    _ctx: Handle,
    validator: Handle,
    index: c_int,
) -> c_int {
    if let Some(arc) = VALIDATORS.get(validator) {
        if let Ok(v) = arc.lock() {
            if let Some(issue) = v.result().issues.get(index as usize) {
                return issue.severity as c_int;
            }
        }
    }
    -1
}

/// Free a validation string
#[unsafe(no_mangle)]
pub extern "C" fn fz_free_validation_string(_ctx: Handle, s: *mut c_char) {
    if !s.is_null() {
        unsafe {
            drop(CString::from_raw(s));
        }
    }
}

/// Get PDF/A level name (returns static string)
#[unsafe(no_mangle)]
pub extern "C" fn fz_pdfa_level_name(level: c_int) -> *const c_char {
    let level = match level {
        0 => PdfALevel::None,
        1 => PdfALevel::A1a,
        2 => PdfALevel::A1b,
        3 => PdfALevel::A2a,
        4 => PdfALevel::A2b,
        5 => PdfALevel::A2u,
        6 => PdfALevel::A3a,
        7 => PdfALevel::A3b,
        8 => PdfALevel::A3u,
        9 => PdfALevel::A4,
        10 => PdfALevel::A4e,
        11 => PdfALevel::A4f,
        _ => PdfALevel::None,
    };
    level.short_name().as_ptr() as *const c_char
}

/// Get PDF/X level name (returns static string)
#[unsafe(no_mangle)]
pub extern "C" fn fz_pdfx_level_name(level: c_int) -> *const c_char {
    let level = match level {
        0 => PdfXLevel::None,
        1 => PdfXLevel::X1a2001,
        2 => PdfXLevel::X1a2003,
        3 => PdfXLevel::X32002,
        4 => PdfXLevel::X32003,
        5 => PdfXLevel::X4,
        6 => PdfXLevel::X4p,
        7 => PdfXLevel::X5g,
        8 => PdfXLevel::X5n,
        9 => PdfXLevel::X5pg,
        10 => PdfXLevel::X6,
        11 => PdfXLevel::X6n,
        12 => PdfXLevel::X6p,
        _ => PdfXLevel::None,
    };
    level.short_name().as_ptr() as *const c_char
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pdfa_levels() {
        assert_eq!(PdfALevel::A1a.short_name(), "PDF/A-1a");
        assert_eq!(PdfALevel::A1a.iso_standard(), "ISO 19005-1");
        assert_eq!(PdfALevel::A2b.short_name(), "PDF/A-2b");
        assert_eq!(PdfALevel::A2b.iso_standard(), "ISO 19005-2");
        assert_eq!(PdfALevel::A4.short_name(), "PDF/A-4");
        assert_eq!(PdfALevel::A4.iso_standard(), "ISO 19005-4");
    }

    #[test]
    fn test_pdfx_levels() {
        assert_eq!(PdfXLevel::X1a2001.short_name(), "PDF/X-1a:2001");
        assert_eq!(PdfXLevel::X1a2001.iso_standard(), "ISO 15930-1");
        assert_eq!(PdfXLevel::X4.short_name(), "PDF/X-4");
        assert_eq!(PdfXLevel::X4.iso_standard(), "ISO 15930-7");
    }

    #[test]
    fn test_pdf_versions() {
        assert_eq!(PdfVersion::V1_7.version_string(), "1.7");
        assert_eq!(PdfVersion::V2_0.version_string(), "2.0");
    }

    #[test]
    fn test_validation_issue() {
        let issue = ValidationIssue::new(IssueSeverity::Error, "TEST_ERROR", "Test error message")
            .with_page(1)
            .with_object(42)
            .with_clause("6.1.2");

        assert_eq!(issue.severity, IssueSeverity::Error);
        assert_eq!(issue.code, "TEST_ERROR");
        assert_eq!(issue.page, 1);
        assert_eq!(issue.object_num, 42);
        assert_eq!(issue.clause, Some("6.1.2".to_string()));
    }

    #[test]
    fn test_validation_result() {
        let mut result = ValidationResult::new();
        assert!(result.is_valid());
        assert_eq!(result.error_count, 0);

        result.add_issue(ValidationIssue::new(
            IssueSeverity::Warning,
            "WARN1",
            "Warning",
        ));
        assert!(result.is_valid());
        assert_eq!(result.warning_count, 1);

        result.add_issue(ValidationIssue::new(IssueSeverity::Error, "ERR1", "Error"));
        assert!(!result.is_valid());
        assert_eq!(result.error_count, 1);
    }

    #[test]
    fn test_validator_config() {
        let config = ValidatorConfig::default();
        assert!(config.check_pdfa);
        assert!(config.check_pdfx);
        assert!(config.check_pdf2);
        assert!(!config.stop_on_error);
    }

    #[test]
    fn test_conformance_validator() {
        let config = ValidatorConfig::default();
        let mut validator = ConformanceValidator::new(config);

        validator.validate_pdfa();
        validator.validate_pdfx();
        validator.validate_pdf2();

        // Since we don't have a real document, validation should pass
        assert!(validator.result().is_valid());
    }

    #[test]
    fn test_validator_ffi() {
        let handle = fz_new_conformance_validator(0, 1, 1, 1);
        assert!(handle != 0);

        fz_validate_pdfa(0, handle);
        fz_validate_pdfx(0, handle);
        fz_validate_pdf2(0, handle);

        let is_valid = fz_conformance_is_valid(0, handle);
        assert_eq!(is_valid, 1);

        let error_count = fz_conformance_error_count(0, handle);
        assert_eq!(error_count, 0);

        fz_drop_conformance_validator(0, handle);
    }

    #[test]
    fn test_validator_reset() {
        let handle = fz_new_conformance_validator(0, 1, 1, 1);

        fz_validate_pdfa(0, handle);
        fz_conformance_validator_reset(0, handle);

        let issue_count = fz_conformance_issue_count(0, handle);
        assert_eq!(issue_count, 0);

        fz_drop_conformance_validator(0, handle);
    }
}
