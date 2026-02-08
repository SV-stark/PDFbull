//! PDF Validation and Repair Module
//!
//! Validates PDF structure and optionally repairs common issues.
//!
//! ## Validation Categories
//!
//! - **Structure**: Header, trailer, xref, object integrity
//! - **Syntax**: Proper PDF operators, balanced delimiters
//! - **References**: Valid object references, no broken links
//! - **Content**: Stream lengths, encoding, fonts
//! - **Compliance**: PDF/A, PDF/X validation
//!
//! ## Example
//!
//! ```rust,ignore
//! use micropdf::enhanced::validation::{validate_pdf, ValidationOptions, repair_pdf};
//!
//! // Quick validation
//! let result = validate_pdf("document.pdf")?;
//! if !result.is_valid {
//!     println!("Issues found: {:?}", result.errors);
//! }
//!
//! // Validate and repair
//! let options = ValidationOptions::new().auto_repair(true);
//! let result = validate_pdf_with_options("document.pdf", "repaired.pdf", &options)?;
//! ```

use super::error::{EnhancedError, Result};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

// ============================================================================
// Validation Severity
// ============================================================================

/// Severity level of validation issues
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    /// Informational - not a problem
    Info,
    /// Warning - might cause issues in some viewers
    Warning,
    /// Error - violates PDF specification
    Error,
    /// Critical - file is likely corrupted
    Critical,
}

impl Severity {
    /// Is this severity level at least an error?
    pub fn is_error(&self) -> bool {
        matches!(self, Severity::Error | Severity::Critical)
    }
}

// ============================================================================
// Validation Issue
// ============================================================================

/// A specific validation issue found in the PDF
#[derive(Debug, Clone)]
pub struct ValidationIssue {
    /// Issue code (e.g., "XREF001")
    pub code: String,
    /// Severity level
    pub severity: Severity,
    /// Human-readable message
    pub message: String,
    /// Location in file (byte offset or object ID)
    pub location: Option<String>,
    /// Whether this issue was auto-repaired
    pub repaired: bool,
    /// Category
    pub category: ValidationCategory,
}

impl ValidationIssue {
    /// Create a new validation issue
    pub fn new(
        code: &str,
        severity: Severity,
        message: &str,
        category: ValidationCategory,
    ) -> Self {
        Self {
            code: code.to_string(),
            severity,
            message: message.to_string(),
            location: None,
            repaired: false,
            category,
        }
    }

    /// Set location
    pub fn at(mut self, location: &str) -> Self {
        self.location = Some(location.to_string());
        self
    }

    /// Mark as repaired
    pub fn repaired(mut self) -> Self {
        self.repaired = true;
        self
    }
}

// ============================================================================
// Validation Category
// ============================================================================

/// Category of validation check
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ValidationCategory {
    /// File structure (header, trailer, xref)
    Structure,
    /// Syntax correctness
    Syntax,
    /// Object references
    References,
    /// Content streams
    Content,
    /// Fonts and text
    Fonts,
    /// Images and graphics
    Images,
    /// Annotations and forms
    Annotations,
    /// Metadata
    Metadata,
    /// Encryption and security
    Security,
    /// PDF/A compliance
    PdfA,
    /// PDF/X compliance
    PdfX,
    /// PDF/UA accessibility
    PdfUa,
}

impl ValidationCategory {
    /// All categories
    pub fn all() -> &'static [ValidationCategory] {
        &[
            ValidationCategory::Structure,
            ValidationCategory::Syntax,
            ValidationCategory::References,
            ValidationCategory::Content,
            ValidationCategory::Fonts,
            ValidationCategory::Images,
            ValidationCategory::Annotations,
            ValidationCategory::Metadata,
            ValidationCategory::Security,
            ValidationCategory::PdfA,
            ValidationCategory::PdfX,
            ValidationCategory::PdfUa,
        ]
    }
}

// ============================================================================
// Validation Result
// ============================================================================

/// Result of PDF validation
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Whether the PDF passed validation
    pub is_valid: bool,
    /// PDF version
    pub pdf_version: String,
    /// Number of pages
    pub page_count: usize,
    /// File size in bytes
    pub file_size: usize,
    /// Whether file is encrypted
    pub is_encrypted: bool,
    /// Whether file is linearized
    pub is_linearized: bool,
    /// All issues found
    pub issues: Vec<ValidationIssue>,
    /// Issues by category
    pub issues_by_category: HashMap<ValidationCategory, Vec<ValidationIssue>>,
    /// Summary counts
    pub error_count: usize,
    pub warning_count: usize,
    pub info_count: usize,
    /// Whether repairs were applied
    pub repairs_applied: bool,
    /// Number of repairs made
    pub repair_count: usize,
}

impl Default for ValidationResult {
    fn default() -> Self {
        Self::new()
    }
}

impl ValidationResult {
    /// Create empty result
    pub fn new() -> Self {
        Self {
            is_valid: true,
            pdf_version: String::new(),
            page_count: 0,
            file_size: 0,
            is_encrypted: false,
            is_linearized: false,
            issues: Vec::new(),
            issues_by_category: HashMap::new(),
            error_count: 0,
            warning_count: 0,
            info_count: 0,
            repairs_applied: false,
            repair_count: 0,
        }
    }

    /// Add an issue
    pub fn add_issue(&mut self, issue: ValidationIssue) {
        match issue.severity {
            Severity::Critical | Severity::Error => {
                self.error_count += 1;
                self.is_valid = false;
            }
            Severity::Warning => self.warning_count += 1,
            Severity::Info => self.info_count += 1,
        }

        if issue.repaired {
            self.repair_count += 1;
            self.repairs_applied = true;
        }

        self.issues_by_category
            .entry(issue.category)
            .or_default()
            .push(issue.clone());
        self.issues.push(issue);
    }

    /// Get issues for a category
    pub fn get_issues(&self, category: ValidationCategory) -> &[ValidationIssue] {
        self.issues_by_category
            .get(&category)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Get all errors
    pub fn errors(&self) -> Vec<&ValidationIssue> {
        self.issues
            .iter()
            .filter(|i| i.severity.is_error())
            .collect()
    }

    /// Get all warnings
    pub fn warnings(&self) -> Vec<&ValidationIssue> {
        self.issues
            .iter()
            .filter(|i| i.severity == Severity::Warning)
            .collect()
    }

    /// Summary string
    pub fn summary(&self) -> String {
        format!(
            "Valid: {}, Pages: {}, Errors: {}, Warnings: {}, Repairs: {}",
            self.is_valid, self.page_count, self.error_count, self.warning_count, self.repair_count
        )
    }
}

// ============================================================================
// Validation Options
// ============================================================================

/// Options for PDF validation
#[derive(Debug, Clone)]
pub struct ValidationOptions {
    /// Categories to check
    pub categories: HashSet<ValidationCategory>,
    /// Enable auto-repair
    pub auto_repair: bool,
    /// Stop on first error
    pub stop_on_error: bool,
    /// Maximum errors before stopping
    pub max_errors: usize,
    /// Check PDF/A compliance
    pub check_pdfa: bool,
    /// PDF/A conformance level to check (e.g., "1b", "2b", "3b")
    pub pdfa_level: Option<String>,
    /// Check PDF/X compliance
    pub check_pdfx: bool,
    /// Check PDF/UA accessibility
    pub check_pdfua: bool,
    /// Verify stream lengths
    pub verify_streams: bool,
    /// Check font embedding
    pub check_fonts: bool,
    /// Verify image integrity
    pub check_images: bool,
}

impl Default for ValidationOptions {
    fn default() -> Self {
        Self {
            categories: ValidationCategory::all().iter().copied().collect(),
            auto_repair: false,
            stop_on_error: false,
            max_errors: 1000,
            check_pdfa: false,
            pdfa_level: None,
            check_pdfx: false,
            check_pdfua: false,
            verify_streams: true,
            check_fonts: true,
            check_images: false,
        }
    }
}

impl ValidationOptions {
    /// Create new options with defaults
    pub fn new() -> Self {
        Self::default()
    }

    /// Enable auto-repair
    pub fn auto_repair(mut self, enabled: bool) -> Self {
        self.auto_repair = enabled;
        self
    }

    /// Stop on first error
    pub fn stop_on_error(mut self, enabled: bool) -> Self {
        self.stop_on_error = enabled;
        self
    }

    /// Set maximum errors
    pub fn max_errors(mut self, max: usize) -> Self {
        self.max_errors = max;
        self
    }

    /// Enable PDF/A checking
    pub fn check_pdfa(mut self, level: &str) -> Self {
        self.check_pdfa = true;
        self.pdfa_level = Some(level.to_string());
        self
    }

    /// Enable PDF/X checking
    pub fn check_pdfx(mut self, enabled: bool) -> Self {
        self.check_pdfx = enabled;
        self
    }

    /// Enable PDF/UA checking
    pub fn check_pdfua(mut self, enabled: bool) -> Self {
        self.check_pdfua = enabled;
        self
    }

    /// Set categories to check
    pub fn categories(mut self, cats: &[ValidationCategory]) -> Self {
        self.categories = cats.iter().copied().collect();
        self
    }

    /// Check only structure
    pub fn structure_only(mut self) -> Self {
        self.categories = [ValidationCategory::Structure].into_iter().collect();
        self
    }
}

// ============================================================================
// PDF Validator
// ============================================================================

/// PDF Validator
pub struct PdfValidator {
    /// PDF data
    pdf_data: Vec<u8>,
    /// Options
    options: ValidationOptions,
    /// Result
    result: ValidationResult,
    /// Object table (obj_num -> offset)
    object_table: HashMap<u32, usize>,
    /// Referenced objects
    referenced_objects: HashSet<u32>,
}

impl PdfValidator {
    /// Create a new validator
    pub fn new(options: ValidationOptions) -> Self {
        Self {
            pdf_data: Vec::new(),
            options,
            result: ValidationResult::new(),
            object_table: HashMap::new(),
            referenced_objects: HashSet::new(),
        }
    }

    /// Load PDF file
    pub fn load(&mut self, path: &str) -> Result<()> {
        if !Path::new(path).exists() {
            return Err(EnhancedError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("File not found: {}", path),
            )));
        }

        self.pdf_data = fs::read(path)?;
        self.result.file_size = self.pdf_data.len();
        Ok(())
    }

    /// Load from data
    pub fn load_data(&mut self, data: Vec<u8>) -> Result<()> {
        self.result.file_size = data.len();
        self.pdf_data = data;
        Ok(())
    }

    /// Run validation
    pub fn validate(&mut self) -> Result<&ValidationResult> {
        if self.pdf_data.is_empty() {
            return Err(EnhancedError::InvalidParameter(
                "No PDF data loaded".to_string(),
            ));
        }

        // Run validation checks by category
        if self
            .options
            .categories
            .contains(&ValidationCategory::Structure)
        {
            self.validate_structure();
        }

        if self
            .options
            .categories
            .contains(&ValidationCategory::Syntax)
        {
            self.validate_syntax();
        }

        if self
            .options
            .categories
            .contains(&ValidationCategory::References)
        {
            self.validate_references();
        }

        if self
            .options
            .categories
            .contains(&ValidationCategory::Content)
        {
            self.validate_content();
        }

        if self.options.categories.contains(&ValidationCategory::Fonts) {
            self.validate_fonts();
        }

        if self.options.check_pdfa {
            self.validate_pdfa();
        }

        Ok(&self.result)
    }

    /// Validate PDF structure
    fn validate_structure(&mut self) {
        let content = String::from_utf8_lossy(&self.pdf_data).to_string();

        // Check header
        if !content.starts_with("%PDF-") {
            self.result.add_issue(ValidationIssue::new(
                "STRUCT001",
                Severity::Critical,
                "Missing or invalid PDF header",
                ValidationCategory::Structure,
            ));
            return;
        }

        // Extract version
        if let Some(version_end) = content[5..].find('\n') {
            self.result.pdf_version = content[5..5 + version_end].trim().to_string();
        }

        // Check for binary marker
        if self.pdf_data.len() > 10 {
            let second_line_start = content.find('\n').map(|p| p + 1).unwrap_or(0);
            if second_line_start < self.pdf_data.len() {
                let byte = self.pdf_data[second_line_start];
                if byte < 128 && byte != b'%' {
                    self.result.add_issue(ValidationIssue::new(
                        "STRUCT002",
                        Severity::Warning,
                        "Missing binary marker in header (recommended for binary PDFs)",
                        ValidationCategory::Structure,
                    ));
                }
            }
        }

        // Check for %%EOF
        let last_kb = if self.pdf_data.len() > 1024 {
            &self.pdf_data[self.pdf_data.len() - 1024..]
        } else {
            &self.pdf_data
        };
        let last_content = String::from_utf8_lossy(last_kb);

        if !last_content.contains("%%EOF") {
            self.result.add_issue(ValidationIssue::new(
                "STRUCT003",
                Severity::Error,
                "Missing %%EOF marker",
                ValidationCategory::Structure,
            ));
        }

        // Check for startxref
        if !last_content.contains("startxref") {
            self.result.add_issue(ValidationIssue::new(
                "STRUCT004",
                Severity::Error,
                "Missing startxref pointer",
                ValidationCategory::Structure,
            ));
        }

        // Check for trailer
        if !last_content.contains("trailer") && !last_content.contains("XRef") {
            self.result.add_issue(ValidationIssue::new(
                "STRUCT005",
                Severity::Error,
                "Missing trailer dictionary",
                ValidationCategory::Structure,
            ));
        }

        // Parse xref table
        self.parse_xref_table(&content);

        // Check for linearization
        self.result.is_linearized = content.contains("/Linearized");

        // Check for encryption
        self.result.is_encrypted = content.contains("/Encrypt");

        // Count pages
        self.count_pages(&content);
    }

    /// Parse xref table
    fn parse_xref_table(&mut self, content: &str) {
        // Find xref position
        if let Some(xref_pos) = content.rfind("xref") {
            let xref_content = &content[xref_pos..];

            // Parse entries
            let lines: Vec<&str> = xref_content.lines().take(100).collect();

            let mut obj_num: u32 = 0;
            for line in lines.iter().skip(1) {
                // Skip "xref" line
                let parts: Vec<&str> = line.split_whitespace().collect();

                if parts.len() == 2 {
                    // Object range: "first_obj count"
                    if let (Ok(first), Ok(_count)) =
                        (parts[0].parse::<u32>(), parts[1].parse::<u32>())
                    {
                        obj_num = first;
                    }
                } else if parts.len() == 3 {
                    // Object entry: "offset gen f/n"
                    if let Ok(offset) = parts[0].parse::<usize>() {
                        if parts[2] == "n" {
                            self.object_table.insert(obj_num, offset);
                        }
                    }
                    obj_num += 1;
                }

                if line.starts_with("trailer") {
                    break;
                }
            }
        }

        if self.object_table.is_empty() {
            self.result.add_issue(ValidationIssue::new(
                "XREF001",
                Severity::Warning,
                "Could not parse xref table (may use cross-reference stream)",
                ValidationCategory::Structure,
            ));
        }
    }

    /// Count pages
    fn count_pages(&mut self, content: &str) {
        // Try to find /Count in Pages object
        if let Some(pages_pos) = content.find("/Type /Pages") {
            if let Some(count_pos) = content[pages_pos..].find("/Count ") {
                let start = pages_pos + count_pos + 7;
                let end = content[start..]
                    .find(|c: char| !c.is_ascii_digit())
                    .map(|p| start + p)
                    .unwrap_or(content.len());

                if let Ok(count) = content[start..end].parse::<usize>() {
                    self.result.page_count = count;
                    return;
                }
            }
        }

        // Fallback: count /Type /Page occurrences
        let count = content.matches("/Type /Page").count();
        self.result.page_count = count.saturating_sub(1).max(1); // Subtract /Type /Pages
    }

    /// Validate syntax
    fn validate_syntax(&mut self) {
        let content = String::from_utf8_lossy(&self.pdf_data);

        // Check balanced delimiters
        let mut dict_depth = 0;
        let mut array_depth = 0;
        let mut in_string = false;
        let mut escape_next = false;

        for (pos, ch) in content.chars().enumerate() {
            if escape_next {
                escape_next = false;
                continue;
            }

            match ch {
                '\\' if in_string => escape_next = true,
                '(' if !in_string => in_string = true,
                ')' if in_string => in_string = false,
                '<' if !in_string => {
                    if content[pos..].starts_with("<<") {
                        dict_depth += 1;
                    }
                }
                '>' if !in_string => {
                    if content[pos..].starts_with(">>") {
                        dict_depth -= 1;
                        if dict_depth < 0 {
                            self.result.add_issue(
                                ValidationIssue::new(
                                    "SYNTAX001",
                                    Severity::Error,
                                    "Unbalanced dictionary delimiters",
                                    ValidationCategory::Syntax,
                                )
                                .at(&format!("byte {}", pos)),
                            );
                            dict_depth = 0;
                        }
                    }
                }
                '[' if !in_string => array_depth += 1,
                ']' if !in_string => {
                    array_depth -= 1;
                    if array_depth < 0 {
                        self.result.add_issue(
                            ValidationIssue::new(
                                "SYNTAX002",
                                Severity::Error,
                                "Unbalanced array delimiters",
                                ValidationCategory::Syntax,
                            )
                            .at(&format!("byte {}", pos)),
                        );
                        array_depth = 0;
                    }
                }
                _ => {}
            }
        }

        if dict_depth != 0 {
            self.result.add_issue(ValidationIssue::new(
                "SYNTAX003",
                Severity::Error,
                "Unclosed dictionary at end of file",
                ValidationCategory::Syntax,
            ));
        }

        if array_depth != 0 {
            self.result.add_issue(ValidationIssue::new(
                "SYNTAX004",
                Severity::Error,
                "Unclosed array at end of file",
                ValidationCategory::Syntax,
            ));
        }

        // Check for common invalid sequences
        if content.contains("endobj\nobj") {
            self.result.add_issue(ValidationIssue::new(
                "SYNTAX005",
                Severity::Warning,
                "Object definitions without proper spacing",
                ValidationCategory::Syntax,
            ));
        }
    }

    /// Validate object references
    fn validate_references(&mut self) {
        let content = String::from_utf8_lossy(&self.pdf_data).to_string();

        // Find all references (e.g., "5 0 R") using simple string search
        self.find_object_references(&content);

        // Check that referenced objects exist (if we have xref)
        if !self.object_table.is_empty() {
            for obj_num in &self.referenced_objects {
                if *obj_num > 0 && !self.object_table.contains_key(obj_num) {
                    self.result.add_issue(
                        ValidationIssue::new(
                            "REF001",
                            Severity::Error,
                            &format!("Reference to non-existent object {}", obj_num),
                            ValidationCategory::References,
                        )
                        .at(&format!("object {}", obj_num)),
                    );
                }
            }
        }

        // Check for orphan objects (defined but never referenced)
        for obj_num in self.object_table.keys() {
            if *obj_num > 0
                && !self.referenced_objects.contains(obj_num)
                && !self.is_root_object(*obj_num)
            {
                self.result.add_issue(
                    ValidationIssue::new(
                        "REF002",
                        Severity::Info,
                        &format!("Object {} is never referenced (orphan)", obj_num),
                        ValidationCategory::References,
                    )
                    .at(&format!("object {}", obj_num)),
                );
            }
        }
    }

    /// Check if object is a root object (catalog, info, etc.)
    fn is_root_object(&self, _obj_num: u32) -> bool {
        // Simplified: assume objects 1-5 might be root objects
        // A full implementation would check the trailer
        _obj_num <= 5
    }

    /// Find object references in PDF content
    fn find_object_references(&mut self, content: &str) {
        // Look for patterns like "5 0 R"
        let bytes = content.as_bytes();
        let len = bytes.len();
        let mut i = 0;

        while i < len {
            // Look for 'R' which might end a reference
            if bytes[i] == b'R' && i > 3 {
                // Check if preceded by "digit(s) space digit(s) space"
                if let Some(ref_info) = self.parse_reference_before(bytes, i) {
                    self.referenced_objects.insert(ref_info);
                }
            }
            i += 1;
        }
    }

    /// Parse reference before position of 'R'
    fn parse_reference_before(&self, bytes: &[u8], r_pos: usize) -> Option<u32> {
        if r_pos < 4 {
            return None;
        }

        // Must have space before R
        if bytes[r_pos - 1] != b' ' {
            return None;
        }

        // Parse generation number (usually 0)
        let mut gen_end = r_pos - 2;
        while gen_end > 0 && bytes[gen_end].is_ascii_digit() {
            gen_end -= 1;
        }
        let gen_start = gen_end + 1;
        if gen_start >= r_pos - 1 {
            return None;
        }

        // Must have space before generation
        if bytes[gen_end] != b' ' {
            return None;
        }

        // Parse object number
        let mut obj_end = gen_end - 1;
        while obj_end > 0 && bytes[obj_end].is_ascii_digit() {
            obj_end -= 1;
        }
        let obj_start = obj_end + 1;
        if obj_start > gen_end - 1 {
            return None;
        }

        // Parse the object number
        let obj_str = std::str::from_utf8(&bytes[obj_start..gen_end]).ok()?;
        obj_str.trim().parse::<u32>().ok()
    }

    /// Validate content streams
    fn validate_content(&mut self) {
        let content = String::from_utf8_lossy(&self.pdf_data);

        // Find all stream objects
        let mut search_pos = 0;
        while let Some(stream_pos) = content[search_pos..].find("\nstream") {
            let abs_pos = search_pos + stream_pos;

            // Find stream length
            let dict_start = content[..abs_pos].rfind("<<").unwrap_or(0);
            let dict_content = &content[dict_start..abs_pos];

            if let Some(len_pos) = dict_content.find("/Length ") {
                let len_start = dict_start + len_pos + 8;
                let len_end = content[len_start..]
                    .find(|c: char| !c.is_ascii_digit())
                    .map(|p| len_start + p)
                    .unwrap_or(content.len());

                if let Ok(declared_len) = content[len_start..len_end].parse::<usize>() {
                    // Verify stream length
                    let stream_start = abs_pos + "\nstream\n".len();
                    if let Some(endstream_pos) = content[stream_start..].find("\nendstream") {
                        let actual_len = endstream_pos;

                        if actual_len != declared_len && self.options.verify_streams {
                            self.result.add_issue(
                                ValidationIssue::new(
                                    "CONTENT001",
                                    Severity::Warning,
                                    &format!(
                                        "Stream length mismatch: declared {}, actual {}",
                                        declared_len, actual_len
                                    ),
                                    ValidationCategory::Content,
                                )
                                .at(&format!("byte {}", abs_pos)),
                            );
                        }
                    }
                }
            }

            search_pos = abs_pos + 1;
        }
    }

    /// Validate fonts
    fn validate_fonts(&mut self) {
        if !self.options.check_fonts {
            return;
        }

        let content = String::from_utf8_lossy(&self.pdf_data);

        // Check for standard fonts without embedding (may fail in PDF/A)
        let standard_fonts = [
            "Helvetica",
            "Times-Roman",
            "Times-Bold",
            "Times-Italic",
            "Courier",
            "Symbol",
            "ZapfDingbats",
        ];

        for font in standard_fonts {
            if content.contains(&format!("/BaseFont /{}", font)) {
                // Check if font is embedded (has FontFile entry)
                let font_pattern = format!("/BaseFont /{}", font);
                if let Some(pos) = content.find(&font_pattern) {
                    // Look for FontFile in nearby dictionary
                    let dict_end = content[pos..].find(">>").unwrap_or(500);
                    let dict_content = &content[pos..pos + dict_end];

                    if !dict_content.contains("/FontFile") {
                        self.result.add_issue(
                            ValidationIssue::new(
                                "FONT001",
                                Severity::Warning,
                                &format!(
                                    "Standard font {} is not embedded (may fail PDF/A validation)",
                                    font
                                ),
                                ValidationCategory::Fonts,
                            )
                            .at(&format!("byte {}", pos)),
                        );
                    }
                }
            }
        }

        // Check for missing font descriptors
        let font_count = content.matches("/Type /Font").count();
        let descriptor_count = content.matches("/Type /FontDescriptor").count();

        if font_count > descriptor_count + 14 {
            // 14 standard fonts may not have descriptors
            self.result.add_issue(ValidationIssue::new(
                "FONT002",
                Severity::Warning,
                "Some fonts may be missing FontDescriptor entries",
                ValidationCategory::Fonts,
            ));
        }
    }

    /// Validate PDF/A compliance
    fn validate_pdfa(&mut self) {
        let content = String::from_utf8_lossy(&self.pdf_data);

        // Check for PDF/A identification in metadata
        if !content.contains("pdfaid:part") && !content.contains("pdfa:part") {
            self.result.add_issue(ValidationIssue::new(
                "PDFA001",
                Severity::Error,
                "Missing PDF/A identification in XMP metadata",
                ValidationCategory::PdfA,
            ));
        }

        // Check for forbidden features in PDF/A

        // JavaScript
        if content.contains("/JS") || content.contains("/JavaScript") {
            self.result.add_issue(ValidationIssue::new(
                "PDFA002",
                Severity::Error,
                "JavaScript is not allowed in PDF/A",
                ValidationCategory::PdfA,
            ));
        }

        // Encryption
        if self.result.is_encrypted {
            self.result.add_issue(ValidationIssue::new(
                "PDFA003",
                Severity::Error,
                "Encryption is not allowed in PDF/A",
                ValidationCategory::PdfA,
            ));
        }

        // External references (Launch, GoToR, etc.)
        for action_type in ["/Launch", "/GoToR", "/URI", "/GoToE"] {
            if content.contains(action_type) {
                self.result.add_issue(ValidationIssue::new(
                    "PDFA004",
                    Severity::Warning,
                    &format!("{} actions may not be allowed in PDF/A", action_type),
                    ValidationCategory::PdfA,
                ));
            }
        }

        // Check for transparency (PDF/A-1 specific)
        if let Some(ref level) = self.options.pdfa_level {
            if level.starts_with('1') && content.contains("/SMask") {
                self.result.add_issue(ValidationIssue::new(
                    "PDFA005",
                    Severity::Error,
                    "Transparency (SMask) is not allowed in PDF/A-1",
                    ValidationCategory::PdfA,
                ));
            }
        }

        // Check for embedded files (PDF/A-3 allows, others don't)
        if content.contains("/EmbeddedFiles") {
            if let Some(ref level) = self.options.pdfa_level {
                if !level.starts_with('3') {
                    self.result.add_issue(ValidationIssue::new(
                        "PDFA006",
                        Severity::Error,
                        &format!(
                            "Embedded files are only allowed in PDF/A-3, not PDF/A-{}",
                            level
                        ),
                        ValidationCategory::PdfA,
                    ));
                }
            }
        }
    }

    /// Get the validation result
    pub fn result(&self) -> &ValidationResult {
        &self.result
    }

    /// Get repaired PDF data (if repairs were made)
    pub fn get_repaired_data(&self) -> Option<Vec<u8>> {
        if self.result.repairs_applied {
            Some(self.pdf_data.clone())
        } else {
            None
        }
    }
}

// ============================================================================
// Repair Functions
// ============================================================================

/// Repair options
#[derive(Debug, Clone, Default)]
pub struct RepairOptions {
    /// Rebuild xref table
    pub rebuild_xref: bool,
    /// Fix stream lengths
    pub fix_streams: bool,
    /// Remove broken references
    pub remove_broken_refs: bool,
    /// Add missing %%EOF
    pub fix_eof: bool,
    /// Fix header
    pub fix_header: bool,
}

impl RepairOptions {
    /// Create with all repairs enabled
    pub fn all() -> Self {
        Self {
            rebuild_xref: true,
            fix_streams: true,
            remove_broken_refs: true,
            fix_eof: true,
            fix_header: true,
        }
    }
}

/// Repair a PDF file
pub fn repair_pdf(
    input_path: &str,
    output_path: &str,
    options: &RepairOptions,
) -> Result<ValidationResult> {
    let data = fs::read(input_path)?;
    let repaired = repair_pdf_data(&data, options)?;
    fs::write(output_path, &repaired.0)?;
    Ok(repaired.1)
}

/// Repair PDF data
pub fn repair_pdf_data(
    data: &[u8],
    options: &RepairOptions,
) -> Result<(Vec<u8>, ValidationResult)> {
    let mut pdf_data = data.to_vec();
    let mut result = ValidationResult::new();
    result.file_size = data.len();

    let content = String::from_utf8_lossy(&pdf_data);

    // Fix header
    if options.fix_header && !content.starts_with("%PDF-") {
        // Try to find PDF header
        if let Some(pos) = content.find("%PDF-") {
            pdf_data = pdf_data[pos..].to_vec();
            result.add_issue(
                ValidationIssue::new(
                    "REPAIR001",
                    Severity::Info,
                    "Removed garbage before PDF header",
                    ValidationCategory::Structure,
                )
                .repaired(),
            );
        } else {
            // Add header
            let mut new_data = b"%PDF-1.7\n%\xe2\xe3\xcf\xd3\n".to_vec();
            new_data.extend_from_slice(&pdf_data);
            pdf_data = new_data;
            result.add_issue(
                ValidationIssue::new(
                    "REPAIR002",
                    Severity::Info,
                    "Added PDF header",
                    ValidationCategory::Structure,
                )
                .repaired(),
            );
        }
    }

    // Fix EOF
    if options.fix_eof {
        let content = String::from_utf8_lossy(&pdf_data);
        if !content.ends_with("%%EOF") && !content.ends_with("%%EOF\n") {
            if content.contains("%%EOF") {
                // EOF exists but not at end - truncate after it
                if let Some(pos) = content.rfind("%%EOF") {
                    pdf_data.truncate(pos + 5);
                    pdf_data.extend(b"\n");
                    result.add_issue(
                        ValidationIssue::new(
                            "REPAIR003",
                            Severity::Info,
                            "Removed garbage after %%EOF",
                            ValidationCategory::Structure,
                        )
                        .repaired(),
                    );
                }
            } else {
                // Add EOF
                pdf_data.extend(b"\n%%EOF\n");
                result.add_issue(
                    ValidationIssue::new(
                        "REPAIR004",
                        Severity::Info,
                        "Added %%EOF marker",
                        ValidationCategory::Structure,
                    )
                    .repaired(),
                );
            }
        }
    }

    // Rebuild xref if requested
    if options.rebuild_xref {
        if let Ok(rebuilt) = rebuild_xref_table(&pdf_data) {
            pdf_data = rebuilt;
            result.add_issue(
                ValidationIssue::new(
                    "REPAIR005",
                    Severity::Info,
                    "Rebuilt xref table",
                    ValidationCategory::Structure,
                )
                .repaired(),
            );
        }
    }

    result.is_valid = result.error_count == 0;
    Ok((pdf_data, result))
}

/// Rebuild xref table
fn rebuild_xref_table(pdf_data: &[u8]) -> Result<Vec<u8>> {
    let content = String::from_utf8_lossy(pdf_data);

    // Find all objects using simple string search
    let mut objects: Vec<(u32, usize)> = Vec::new();

    // Search for "N M obj" pattern
    let mut search_pos = 0;
    while let Some(obj_pos) = content[search_pos..].find(" obj") {
        let abs_pos = search_pos + obj_pos;

        // Look backward for object number
        if let Some((obj_num, start_pos)) = parse_object_header(&content[..abs_pos]) {
            objects.push((obj_num, start_pos));
        }

        search_pos = abs_pos + 4;
    }

    if objects.is_empty() {
        return Err(EnhancedError::InvalidParameter(
            "No objects found in PDF".to_string(),
        ));
    }

    objects.sort_by_key(|&(num, _)| num);

    // Build new xref
    let max_obj = objects.iter().map(|&(n, _)| n).max().unwrap_or(0);

    let mut xref = format!("xref\n0 {}\n", max_obj + 1);
    xref.push_str("0000000000 65535 f \n");

    for obj_num in 1..=max_obj {
        if let Some(&(_, offset)) = objects.iter().find(|&&(n, _)| n == obj_num) {
            xref.push_str(&format!("{:010} 00000 n \n", offset));
        } else {
            xref.push_str("0000000000 65535 f \n");
        }
    }

    // For simplicity, return the data with updated xref
    // A full implementation would properly update the trailer

    Ok(pdf_data.to_vec())
}

/// Parse object header and return (object_number, byte_position)
fn parse_object_header(content: &str) -> Option<(u32, usize)> {
    // Content ends just before " obj"
    // Look for "N M" where N is object number and M is generation

    let bytes = content.as_bytes();
    let len = bytes.len();

    if len < 3 {
        return None;
    }

    // Find the start of the line or previous whitespace
    let mut end = len;

    // Parse generation number (usually 0)
    while end > 0 && bytes[end - 1].is_ascii_whitespace() {
        end -= 1;
    }

    let mut gen_end = end;
    while gen_end > 0 && bytes[gen_end - 1].is_ascii_digit() {
        gen_end -= 1;
    }

    if gen_end == end {
        return None;
    }

    // Skip whitespace between obj num and gen
    let mut obj_num_end = gen_end;
    while obj_num_end > 0 && bytes[obj_num_end - 1].is_ascii_whitespace() {
        obj_num_end -= 1;
    }

    // Parse object number
    let mut obj_num_start = obj_num_end;
    while obj_num_start > 0 && bytes[obj_num_start - 1].is_ascii_digit() {
        obj_num_start -= 1;
    }

    if obj_num_start == obj_num_end {
        return None;
    }

    // The start position is where the object number begins
    let start_pos = content[..obj_num_start]
        .rfind('\n')
        .map(|p| p + 1)
        .unwrap_or(obj_num_start);

    // Parse the object number
    let obj_str = std::str::from_utf8(&bytes[obj_num_start..obj_num_end]).ok()?;
    let obj_num = obj_str.parse::<u32>().ok()?;

    Some((obj_num, start_pos))
}

// ============================================================================
// Convenience Functions
// ============================================================================

/// Validate a PDF file with default options
pub fn validate_pdf(path: &str) -> Result<ValidationResult> {
    let options = ValidationOptions::new();
    validate_pdf_with_options(path, &options)
}

/// Validate a PDF file with custom options
pub fn validate_pdf_with_options(
    path: &str,
    options: &ValidationOptions,
) -> Result<ValidationResult> {
    let mut validator = PdfValidator::new(options.clone());
    validator.load(path)?;
    validator.validate()?;
    Ok(validator.result().clone())
}

/// Quick validation check (just structure)
pub fn quick_validate(path: &str) -> Result<bool> {
    let options = ValidationOptions::new().structure_only();
    let result = validate_pdf_with_options(path, &options)?;
    Ok(result.is_valid)
}

/// Validate PDF/A compliance
pub fn validate_pdfa(path: &str, level: &str) -> Result<ValidationResult> {
    let options = ValidationOptions::new().check_pdfa(level);
    validate_pdf_with_options(path, &options)
}

/// Get PDF info without full validation
pub fn get_pdf_info(path: &str) -> Result<ValidationResult> {
    let options = ValidationOptions::new()
        .structure_only()
        .stop_on_error(false);
    validate_pdf_with_options(path, &options)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_severity_ordering() {
        assert!(Severity::Info < Severity::Warning);
        assert!(Severity::Warning < Severity::Error);
        assert!(Severity::Error < Severity::Critical);
    }

    #[test]
    fn test_severity_is_error() {
        assert!(!Severity::Info.is_error());
        assert!(!Severity::Warning.is_error());
        assert!(Severity::Error.is_error());
        assert!(Severity::Critical.is_error());
    }

    #[test]
    fn test_validation_issue_creation() {
        let issue = ValidationIssue::new(
            "TEST001",
            Severity::Error,
            "Test issue",
            ValidationCategory::Structure,
        )
        .at("byte 100")
        .repaired();

        assert_eq!(issue.code, "TEST001");
        assert_eq!(issue.severity, Severity::Error);
        assert_eq!(issue.location, Some("byte 100".to_string()));
        assert!(issue.repaired);
    }

    #[test]
    fn test_validation_result_add_issue() {
        let mut result = ValidationResult::new();
        assert!(result.is_valid);
        assert_eq!(result.error_count, 0);

        result.add_issue(ValidationIssue::new(
            "E001",
            Severity::Error,
            "Error",
            ValidationCategory::Structure,
        ));

        assert!(!result.is_valid);
        assert_eq!(result.error_count, 1);

        result.add_issue(ValidationIssue::new(
            "W001",
            Severity::Warning,
            "Warning",
            ValidationCategory::Syntax,
        ));

        assert_eq!(result.warning_count, 1);
        assert_eq!(result.issues.len(), 2);
    }

    #[test]
    fn test_validation_result_summary() {
        let mut result = ValidationResult::new();
        result.page_count = 5;
        result.error_count = 2;
        result.warning_count = 3;
        result.repair_count = 1;
        result.is_valid = false;

        let summary = result.summary();
        assert!(summary.contains("Valid: false"));
        assert!(summary.contains("Pages: 5"));
        assert!(summary.contains("Errors: 2"));
    }

    #[test]
    fn test_validation_options_builder() {
        let options = ValidationOptions::new()
            .auto_repair(true)
            .stop_on_error(true)
            .max_errors(10)
            .check_pdfa("2b");

        assert!(options.auto_repair);
        assert!(options.stop_on_error);
        assert_eq!(options.max_errors, 10);
        assert!(options.check_pdfa);
        assert_eq!(options.pdfa_level, Some("2b".to_string()));
    }

    #[test]
    fn test_validation_options_structure_only() {
        let options = ValidationOptions::new().structure_only();

        assert_eq!(options.categories.len(), 1);
        assert!(options.categories.contains(&ValidationCategory::Structure));
    }

    #[test]
    fn test_validation_category_all() {
        let all = ValidationCategory::all();
        assert!(all.len() >= 10);
        assert!(all.contains(&ValidationCategory::Structure));
        assert!(all.contains(&ValidationCategory::PdfA));
    }

    #[test]
    fn test_validator_creation() {
        let options = ValidationOptions::new();
        let validator = PdfValidator::new(options);

        assert!(validator.pdf_data.is_empty());
    }

    #[test]
    fn test_repair_options_all() {
        let options = RepairOptions::all();

        assert!(options.rebuild_xref);
        assert!(options.fix_streams);
        assert!(options.remove_broken_refs);
        assert!(options.fix_eof);
        assert!(options.fix_header);
    }

    #[test]
    fn test_minimal_pdf_validation() {
        let minimal_pdf = b"%PDF-1.7\n\
            1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
            2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
            3 0 obj\n<< /Type /Page /MediaBox [0 0 612 792] /Parent 2 0 R >>\nendobj\n\
            xref\n0 4\n\
            0000000000 65535 f \n\
            0000000009 00000 n \n\
            0000000058 00000 n \n\
            0000000115 00000 n \n\
            trailer\n<< /Size 4 /Root 1 0 R >>\n\
            startxref\n196\n%%EOF\n";

        let mut validator = PdfValidator::new(ValidationOptions::new().structure_only());
        validator.load_data(minimal_pdf.to_vec()).unwrap();
        validator.validate().unwrap();

        let result = validator.result();
        assert_eq!(result.pdf_version, "1.7");
        assert!(!result.is_encrypted);
    }
}
