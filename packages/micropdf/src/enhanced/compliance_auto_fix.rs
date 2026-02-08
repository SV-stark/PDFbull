//! Auto-fix common compliance issues
//!
//! Automatically repair common PDF compliance problems

use super::compliance::{ComplianceValidation, preflight_check};
use super::error::{EnhancedError, Result};
use std::path::Path;

/// Auto-fix options
#[derive(Debug, Clone)]
pub struct AutoFixOptions {
    /// Remove JavaScript
    pub remove_javascript: bool,
    /// Remove encryption (if possible)
    pub remove_encryption: bool,
    /// Add missing metadata
    pub add_metadata: bool,
    /// Fix structure issues
    pub fix_structure: bool,
    /// Flatten transparency
    pub flatten_transparency: bool,
    /// Embed fonts (if possible)
    pub embed_fonts: bool,
}

impl Default for AutoFixOptions {
    fn default() -> Self {
        Self {
            remove_javascript: true,
            remove_encryption: false, // Requires password
            add_metadata: true,
            fix_structure: true,
            flatten_transparency: false, // Requires rendering
            embed_fonts: false,          // Requires font files
        }
    }
}

/// Auto-fix result
#[derive(Debug, Clone)]
pub struct AutoFixResult {
    pub fixed_issues: Vec<String>,
    pub remaining_issues: Vec<String>,
    pub validation: ComplianceValidation,
}

/// Automatically fix common compliance issues
pub fn auto_fix_compliance(
    pdf_path: &str,
    output_path: &str,
    options: &AutoFixOptions,
) -> Result<AutoFixResult> {
    if !Path::new(pdf_path).exists() {
        return Err(EnhancedError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("PDF file not found: {}", pdf_path),
        )));
    }

    let mut fixed_issues = Vec::new();
    let mut remaining_issues = Vec::new();

    // Read PDF
    let pdf_data = std::fs::read(pdf_path)?;
    let mut content = String::from_utf8_lossy(&pdf_data).to_string();

    // 1. Remove JavaScript
    if options.remove_javascript && (content.contains("/JavaScript") || content.contains("/JS")) {
        content = content.replace("/JavaScript", "/Removed_JavaScript");
        content = content.replace("/JS", "/Removed_JS");
        fixed_issues.push("Removed JavaScript".to_string());
    }

    // 2. Add missing metadata
    if options.add_metadata {
        if !content.contains("/Info") {
            content = add_basic_metadata(&content)?;
            fixed_issues.push("Added basic metadata".to_string());
        }
    }

    // 3. Fix structure issues
    if options.fix_structure {
        // Ensure Catalog exists
        if !content.contains("/Type/Catalog") {
            remaining_issues.push("Missing Catalog - requires manual repair".to_string());
        }

        // Ensure Pages tree exists
        if !content.contains("/Type/Pages") {
            remaining_issues.push("Missing Pages tree - requires manual repair".to_string());
        }

        // Fix missing xref (simplified)
        if !content.contains("xref") {
            remaining_issues.push("Missing xref table - requires full PDF rebuild".to_string());
        }
    }

    // 4. Handle encryption
    if options.remove_encryption && content.contains("/Encrypt") {
        remaining_issues.push("Encryption removal requires password and decryption".to_string());
    }

    // 5. Handle transparency
    if options.flatten_transparency && content.contains("/ca") {
        remaining_issues.push("Transparency flattening requires rendering engine".to_string());
    }

    // 6. Handle font embedding
    if options.embed_fonts {
        let font_count = content.matches("/Type/Font").count();
        if font_count > 0 {
            remaining_issues.push(format!("Font embedding requires {} font files", font_count));
        }
    }

    // Write fixed PDF
    std::fs::write(output_path, content.as_bytes())?;

    // Validate the fixed PDF
    let validation = preflight_check(output_path)?;

    Ok(AutoFixResult {
        fixed_issues,
        remaining_issues,
        validation,
    })
}

/// Add basic metadata to PDF
fn add_basic_metadata(content: &str) -> Result<String> {
    let metadata = r#"
/Info <<
  /Title(Untitled Document)
  /Producer(MicroPDF)
  /CreationDate(D:20260112000000Z)
  /ModDate(D:20260112000000Z)
>>"#;

    let mut result = content.to_string();

    // Find trailer and add Info reference
    if let Some(trailer_pos) = result.find("trailer") {
        if let Some(dict_start) = result[trailer_pos..].find("<<") {
            result.insert_str(trailer_pos + dict_start + 2, metadata);
        }
    }

    Ok(result)
}

/// Quick fix for common issues
pub fn quick_fix(pdf_path: &str, output_path: &str) -> Result<AutoFixResult> {
    let options = AutoFixOptions {
        remove_javascript: true,
        remove_encryption: false,
        add_metadata: true,
        fix_structure: true,
        flatten_transparency: false,
        embed_fonts: false,
    };

    auto_fix_compliance(pdf_path, output_path, &options)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auto_fix_options_default() {
        let options = AutoFixOptions::default();
        assert!(options.remove_javascript);
        assert!(options.add_metadata);
        assert!(options.fix_structure);
        assert!(!options.flatten_transparency);
    }

    #[test]
    fn test_add_basic_metadata() {
        let content = "test trailer << >> test";
        let result = add_basic_metadata(content);
        assert!(result.is_ok());
        let fixed = result.unwrap();
        assert!(fixed.contains("/Info"));
        assert!(fixed.contains("/Title"));
    }
}
