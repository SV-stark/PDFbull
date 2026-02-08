//! PDF Linearization (Web Optimization)
//!
//! This module provides support for reading and writing linearized PDFs,
//! which are optimized for page-at-a-time downloading over the web.
//!
//! # Linearization Overview
//!
//! A linearized PDF has the following structure:
//! 1. Header and linearization dictionary
//! 2. First page cross-reference table and trailer
//! 3. Document catalog, first page, and related objects
//! 4. Hint stream
//! 5. Remaining pages
//! 6. Remaining objects
//! 7. Final cross-reference table and trailer

use super::error::{QpdfError, Result};
use std::collections::HashMap;

/// Linearization parameters from the linearization dictionary
#[derive(Debug, Clone)]
pub struct LinearizationParams {
    /// File length
    pub file_length: u64,
    /// Primary hint stream offset
    pub hint_offset: u64,
    /// Primary hint stream length
    pub hint_length: u64,
    /// Overflow hint stream offset (optional)
    pub overflow_hint_offset: Option<u64>,
    /// Overflow hint stream length (optional)
    pub overflow_hint_length: Option<u64>,
    /// First page object number
    pub first_page_obj: u32,
    /// End of first page
    pub first_page_end: u64,
    /// Number of pages
    pub page_count: u32,
    /// Offset of main XRef table
    pub main_xref_offset: u64,
    /// First page number (usually 0)
    pub first_page_num: u32,
}

/// Page offset hint table entry
#[derive(Debug, Clone, Default)]
pub struct PageOffsetHintEntry {
    /// Number of objects on the page minus minimum
    pub objects_delta: u32,
    /// Page length minus minimum
    pub length_delta: u64,
    /// Number of shared object references
    pub shared_count: u32,
    /// Shared object identifiers
    pub shared_identifiers: Vec<u32>,
    /// Shared object numerators
    pub shared_numerators: Vec<u32>,
    /// Content stream offset delta
    pub content_offset_delta: u64,
    /// Content stream length delta
    pub content_length_delta: u64,
}

/// Page offset hint table header
#[derive(Debug, Clone)]
pub struct PageOffsetHintHeader {
    /// Minimum object count per page
    pub min_objects: u32,
    /// First page object location
    pub first_page_loc: u64,
    /// Bits for object count delta
    pub bits_objects: u32,
    /// Minimum page length
    pub min_length: u64,
    /// Bits for length delta
    pub bits_length: u32,
    /// Minimum content stream offset
    pub min_content_offset: u64,
    /// Bits for content offset delta
    pub bits_content_offset: u32,
    /// Minimum content stream length
    pub min_content_length: u64,
    /// Bits for content length delta
    pub bits_content_length: u32,
    /// Bits for shared reference count
    pub bits_shared: u32,
    /// Bits for shared group identifier
    pub bits_shared_group: u32,
    /// Bits for shared numerator
    pub bits_shared_numerator: u32,
    /// Shared group denominator
    pub shared_denominator: u32,
}

/// Shared object hint table entry
#[derive(Debug, Clone)]
pub struct SharedObjectHintEntry {
    /// Object group length delta
    pub length_delta: u64,
    /// Signature flag
    pub signature: bool,
    /// Number of objects in group minus minimum
    pub objects_delta: u32,
}

/// Shared object hint table header
#[derive(Debug, Clone)]
pub struct SharedObjectHintHeader {
    /// Object number of first shared object
    pub first_object: u32,
    /// Location of first shared object
    pub first_object_loc: u64,
    /// Number of shared object entries
    pub entry_count: u32,
    /// Number of first page shared objects
    pub first_page_count: u32,
    /// Minimum group length
    pub min_length: u64,
    /// Bits for group length delta
    pub bits_length: u32,
}

/// Linearization validation result
#[derive(Debug, Clone)]
pub struct LinearizationCheckResult {
    /// Whether the file is linearized
    pub is_linearized: bool,
    /// Errors found during validation
    pub errors: Vec<String>,
    /// Warnings found during validation
    pub warnings: Vec<String>,
}

impl LinearizationCheckResult {
    /// Create a new result indicating not linearized
    pub fn not_linearized() -> Self {
        Self {
            is_linearized: false,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// Create a new result indicating linearized with no issues
    pub fn valid() -> Self {
        Self {
            is_linearized: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// Check if the linearization is valid (linearized with no errors)
    pub fn is_valid(&self) -> bool {
        self.is_linearized && self.errors.is_empty()
    }

    /// Add an error
    pub fn add_error(&mut self, error: String) {
        self.errors.push(error);
    }

    /// Add a warning
    pub fn add_warning(&mut self, warning: String) {
        self.warnings.push(warning);
    }
}

/// Check if a PDF file is linearized
///
/// This checks if the file starts with a linearization parameter dictionary.
pub fn is_linearized(data: &[u8]) -> bool {
    // Look for linearization dictionary near the start of the file
    // It should be within the first few kilobytes
    let search_range = data.len().min(4096);
    let search_data = &data[..search_range];

    // Look for "/Linearized" in the search range
    let linearized_marker = b"/Linearized";
    search_data
        .windows(linearized_marker.len())
        .any(|window| window == linearized_marker)
}

/// Check linearization and return detailed results
pub fn check_linearization(data: &[u8]) -> Result<LinearizationCheckResult> {
    if !is_linearized(data) {
        return Ok(LinearizationCheckResult::not_linearized());
    }

    let mut result = LinearizationCheckResult::valid();

    // Parse linearization dictionary
    match parse_linearization_dict(data) {
        Ok(params) => {
            // Validate file length
            if params.file_length != data.len() as u64 {
                result.add_warning(format!(
                    "File length mismatch: linearization dict says {}, actual is {}",
                    params.file_length,
                    data.len()
                ));
            }

            // Validate hint stream
            if params.hint_offset >= data.len() as u64 {
                result.add_error("Hint stream offset beyond file end".to_string());
            }

            // Additional validation would go here...
        }
        Err(e) => {
            result.add_error(format!("Failed to parse linearization dictionary: {}", e));
        }
    }

    Ok(result)
}

/// Parse linearization dictionary from PDF data
fn parse_linearization_dict(data: &[u8]) -> Result<LinearizationParams> {
    // Find the linearization dictionary
    let linearized_marker = b"/Linearized";
    let start = data
        .windows(linearized_marker.len())
        .position(|window| window == linearized_marker)
        .ok_or_else(|| QpdfError::Linearization("Linearization marker not found".to_string()))?;

    // Find dictionary boundaries
    let dict_start = data[..start]
        .iter()
        .rposition(|&b| b == b'<' && data.get(start.saturating_sub(1)) == Some(&b'<'))
        .ok_or_else(|| {
            QpdfError::Linearization("Linearization dictionary start not found".to_string())
        })?;

    let dict_end = data[start..]
        .windows(2)
        .position(|window| window == b">>")
        .map(|pos| start + pos + 2)
        .ok_or_else(|| {
            QpdfError::Linearization("Linearization dictionary end not found".to_string())
        })?;

    let dict_data = &data[dict_start..dict_end];
    let dict_str = std::str::from_utf8(dict_data)
        .map_err(|_| QpdfError::Linearization("Invalid UTF-8 in linearization dict".to_string()))?;

    // Parse required values
    let parse_value = |key: &str| -> Result<u64> {
        let key_pos = dict_str.find(key).ok_or_else(|| {
            QpdfError::Linearization(format!("Missing {} in linearization dict", key))
        })?;

        let value_start = key_pos + key.len();
        let value_str = dict_str[value_start..]
            .trim_start()
            .split_whitespace()
            .next()
            .ok_or_else(|| QpdfError::Linearization(format!("Invalid {} value", key)))?;

        value_str
            .parse()
            .map_err(|_| QpdfError::Linearization(format!("Invalid {} value: {}", key, value_str)))
    };

    let file_length = parse_value("/L")?;
    let hint_offset = parse_value("/H").unwrap_or(0);
    let first_page_obj = parse_value("/O")? as u32;
    let first_page_end = parse_value("/E")?;
    let page_count = parse_value("/N")? as u32;
    let main_xref_offset = parse_value("/T")?;

    Ok(LinearizationParams {
        file_length,
        hint_offset,
        hint_length: 0, // Would need more parsing
        overflow_hint_offset: None,
        overflow_hint_length: None,
        first_page_obj,
        first_page_end,
        page_count,
        main_xref_offset,
        first_page_num: 0,
    })
}

/// Configuration for creating a linearized PDF
#[derive(Debug, Clone)]
pub struct LinearizationConfig {
    /// Whether to include outline (bookmark) data in the first page section
    pub include_outline: bool,
    /// Whether to include thumbnail images in hint tables
    pub include_thumbnails: bool,
}

impl Default for LinearizationConfig {
    fn default() -> Self {
        Self {
            include_outline: true,
            include_thumbnails: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_linearized_false() {
        let data = b"%PDF-1.7\n1 0 obj\n<</Type/Catalog>>\nendobj\n";
        assert!(!is_linearized(data));
    }

    #[test]
    fn test_is_linearized_true() {
        let data = b"%PDF-1.7\n1 0 obj\n<</Linearized 1/L 1000/O 5/E 500/N 10/T 900>>\nendobj\n";
        assert!(is_linearized(data));
    }

    #[test]
    fn test_check_linearization_not_linearized() {
        let data = b"%PDF-1.7\n1 0 obj\n<</Type/Catalog>>\nendobj\n";
        let result = check_linearization(data).unwrap();
        assert!(!result.is_linearized);
    }
}
