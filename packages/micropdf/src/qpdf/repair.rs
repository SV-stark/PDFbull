//! PDF Repair functionality
//!
//! This module provides functionality to repair damaged or malformed PDF files.

use super::error::{QpdfError, Result};

/// Types of issues that can be repaired
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RepairIssue {
    /// Invalid or missing PDF header
    InvalidHeader,
    /// Damaged or missing cross-reference table
    DamagedXref,
    /// Missing or incorrect stream lengths
    StreamLength,
    /// Invalid object numbers
    InvalidObjectNumbers,
    /// Dangling object references
    DanglingReferences,
    /// Missing required objects
    MissingObjects,
    /// Invalid page tree
    InvalidPageTree,
    /// Encoding issues
    EncodingIssues,
    /// Incorrect endstream/endobj markers
    InvalidMarkers,
}

/// Result of a repair operation
#[derive(Debug, Clone)]
pub struct RepairResult {
    /// Issues that were found and repaired
    pub repaired: Vec<RepairIssue>,
    /// Issues that were found but could not be repaired
    pub unrepaired: Vec<(RepairIssue, String)>,
    /// Warnings during repair
    pub warnings: Vec<String>,
    /// Whether the repair was successful overall
    pub success: bool,
}

impl RepairResult {
    /// Create a successful repair result
    pub fn success() -> Self {
        Self {
            repaired: Vec::new(),
            unrepaired: Vec::new(),
            warnings: Vec::new(),
            success: true,
        }
    }

    /// Add a repaired issue
    pub fn add_repaired(&mut self, issue: RepairIssue) {
        self.repaired.push(issue);
    }

    /// Add an unrepaired issue
    pub fn add_unrepaired(&mut self, issue: RepairIssue, reason: String) {
        self.unrepaired.push((issue, reason));
        self.success = false;
    }

    /// Add a warning
    pub fn add_warning(&mut self, warning: String) {
        self.warnings.push(warning);
    }
}

/// Configuration for PDF repair
#[derive(Debug, Clone)]
pub struct RepairConfig {
    /// Attempt to recover cross-reference table
    pub recover_xref: bool,
    /// Fix stream lengths
    pub fix_stream_lengths: bool,
    /// Remove dangling references
    pub remove_dangling_refs: bool,
    /// Rebuild page tree if needed
    pub rebuild_page_tree: bool,
    /// Maximum recovery attempts
    pub max_recovery_attempts: u32,
}

impl Default for RepairConfig {
    fn default() -> Self {
        Self {
            recover_xref: true,
            fix_stream_lengths: true,
            remove_dangling_refs: true,
            rebuild_page_tree: true,
            max_recovery_attempts: 10,
        }
    }
}

/// Repair a PDF file
pub fn repair_pdf(data: &[u8], config: &RepairConfig) -> Result<(Vec<u8>, RepairResult)> {
    let mut result = RepairResult::success();
    let mut output = data.to_vec();

    // Check and fix PDF header
    if !data.starts_with(b"%PDF-") {
        if let Some(repaired) = fix_header(&mut output) {
            result.add_repaired(RepairIssue::InvalidHeader);
        }
    }

    // Try to recover XRef if configured
    if config.recover_xref {
        match recover_xref(&output) {
            Ok(Some(fixed_data)) => {
                output = fixed_data;
                result.add_repaired(RepairIssue::DamagedXref);
            }
            Ok(None) => {
                // XRef is fine, no action needed
            }
            Err(e) => {
                result.add_warning(format!("XRef recovery attempted but failed: {}", e));
            }
        }
    }

    // Fix stream lengths if configured
    if config.fix_stream_lengths {
        match fix_stream_lengths(&output) {
            Ok(Some(fixed_data)) => {
                output = fixed_data;
                result.add_repaired(RepairIssue::StreamLength);
            }
            Ok(None) => {}
            Err(e) => {
                result.add_warning(format!("Stream length fix failed: {}", e));
            }
        }
    }

    Ok((output, result))
}

/// Fix missing or incorrect PDF header
fn fix_header(data: &mut Vec<u8>) -> Option<()> {
    // Find where PDF content actually starts
    let pdf_marker = b"%PDF-";
    if let Some(pos) = data.windows(pdf_marker.len()).position(|w| w == pdf_marker) {
        if pos > 0 {
            // Remove garbage before header
            data.drain(0..pos);
            return Some(());
        }
        return None; // Header is fine
    }

    // No header found, prepend one
    let header = b"%PDF-1.7\n";
    let mut new_data = header.to_vec();
    new_data.extend_from_slice(data);
    *data = new_data;
    Some(())
}

/// Try to recover a damaged cross-reference table
fn recover_xref(data: &[u8]) -> Result<Option<Vec<u8>>> {
    // Find all object definitions
    let mut objects = Vec::new();
    let mut pos = 0;

    while pos < data.len() {
        // Look for object definition pattern: "N G obj"
        if let Some(obj_info) = find_object_def(&data[pos..]) {
            objects.push((pos + obj_info.0, obj_info.1, obj_info.2));
            pos += obj_info.0 + 1;
        } else {
            break;
        }
    }

    if objects.is_empty() {
        return Err(QpdfError::Repair("No objects found in PDF".to_string()));
    }

    // Check if XRef needs rebuilding
    if has_valid_xref(data) {
        return Ok(None); // XRef is fine
    }

    // Rebuild XRef table
    let rebuilt_data = rebuild_xref(data, &objects)?;
    Ok(Some(rebuilt_data))
}

/// Find an object definition starting at the given position
fn find_object_def(data: &[u8]) -> Option<(usize, u32, u32)> {
    // Simple pattern matching for "N G obj"
    let obj_marker = b" obj";

    for (i, window) in data.windows(obj_marker.len()).enumerate() {
        if window == obj_marker {
            // Look back for object number and generation
            let prefix = &data[..i];
            let prefix_str = std::str::from_utf8(prefix).ok()?;

            // Find the last two numbers before "obj"
            let parts: Vec<&str> = prefix_str.split_whitespace().rev().take(2).collect();
            if parts.len() == 2 {
                if let (Ok(generation), Ok(obj)) =
                    (parts[0].parse::<u32>(), parts[1].parse::<u32>())
                {
                    return Some((i, obj, generation));
                }
            }
        }
    }
    None
}

/// Check if the PDF has a valid XRef table
fn has_valid_xref(data: &[u8]) -> bool {
    // Look for xref keyword
    data.windows(4).any(|w| w == b"xref") || data.windows(7).any(|w| w == b"/XRef") // XRef stream
}

/// Rebuild the XRef table for the given objects
fn rebuild_xref(data: &[u8], objects: &[(usize, u32, u32)]) -> Result<Vec<u8>> {
    // This is a simplified implementation
    // A full implementation would properly construct the XRef table

    let mut output = data.to_vec();

    // Find startxref location
    if let Some(startxref_pos) = output.windows(9).rposition(|w| w == b"startxref") {
        // Remove old xref and trailer
        output.truncate(startxref_pos);
    }

    // Build new xref table
    let xref_offset = output.len();
    output.extend_from_slice(b"\nxref\n");
    output.extend_from_slice(format!("0 {}\n", objects.len() + 1).as_bytes());
    output.extend_from_slice(b"0000000000 65535 f \n");

    for (offset, _obj, generation) in objects {
        output.extend_from_slice(format!("{:010} {:05} n \n", offset, generation).as_bytes());
    }

    // Add trailer
    output.extend_from_slice(b"trailer\n<<\n");
    output.extend_from_slice(format!("/Size {}\n", objects.len() + 1).as_bytes());
    output.extend_from_slice(b">>\n");
    output.extend_from_slice(b"startxref\n");
    output.extend_from_slice(format!("{}\n", xref_offset).as_bytes());
    output.extend_from_slice(b"%%EOF\n");

    Ok(output)
}

/// Fix incorrect stream lengths
fn fix_stream_lengths(data: &[u8]) -> Result<Option<Vec<u8>>> {
    // Look for stream/endstream pairs and verify lengths
    let stream_marker = b"stream";
    let endstream_marker = b"endstream";

    let mut output = data.to_vec();
    let mut modified = false;

    // Find all streams
    let mut pos = 0;
    while pos < output.len() {
        if let Some(stream_pos) = output[pos..]
            .windows(stream_marker.len())
            .position(|w| w == stream_marker)
        {
            let abs_stream_pos = pos + stream_pos;

            // Find endstream
            if let Some(endstream_pos) = output[abs_stream_pos..]
                .windows(endstream_marker.len())
                .position(|w| w == endstream_marker)
            {
                // Calculate actual stream length
                let stream_start = abs_stream_pos + stream_marker.len();
                // Skip newline after "stream"
                let content_start = if output.get(stream_start) == Some(&b'\r') {
                    if output.get(stream_start + 1) == Some(&b'\n') {
                        stream_start + 2
                    } else {
                        stream_start + 1
                    }
                } else if output.get(stream_start) == Some(&b'\n') {
                    stream_start + 1
                } else {
                    stream_start
                };

                let actual_length = abs_stream_pos + endstream_pos - content_start;

                // Look for /Length in the dictionary before stream
                // This is simplified - a real implementation would parse the dict
                let _dict_end = abs_stream_pos;
                // ... would update /Length here if needed

                pos = abs_stream_pos + endstream_pos + endstream_marker.len();
            } else {
                pos += 1;
            }
        } else {
            break;
        }
    }

    if modified { Ok(Some(output)) } else { Ok(None) }
}

/// Quick validation of a PDF file
pub fn quick_validate(data: &[u8]) -> bool {
    // Check header
    if !data.starts_with(b"%PDF-") {
        return false;
    }

    // Check for %%EOF at end (allowing for trailing whitespace)
    let eof_marker = b"%%EOF";
    let end_search = &data[data.len().saturating_sub(100)..];
    if !end_search
        .windows(eof_marker.len())
        .any(|w| w == eof_marker)
    {
        return false;
    }

    // Check for at least one object
    if !data.windows(4).any(|w| w == b" obj") {
        return false;
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quick_validate_valid() {
        let data = b"%PDF-1.7\n1 0 obj\n<</Type/Catalog>>\nendobj\n%%EOF\n";
        assert!(quick_validate(data));
    }

    #[test]
    fn test_quick_validate_no_header() {
        let data = b"1 0 obj\n<</Type/Catalog>>\nendobj\n%%EOF\n";
        assert!(!quick_validate(data));
    }

    #[test]
    fn test_quick_validate_no_eof() {
        let data = b"%PDF-1.7\n1 0 obj\n<</Type/Catalog>>\nendobj\n";
        assert!(!quick_validate(data));
    }
}
