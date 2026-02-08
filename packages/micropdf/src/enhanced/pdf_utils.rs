//! PDF Utilities - Low-level PDF manipulation helpers
//!
//! This module provides utilities for parsing and manipulating PDF files
//! at the object level using internal PDF parsing.

use super::error::{EnhancedError, Result};
use std::fs;
use std::path::Path;

/// Parse a PDF file and extract basic structure
pub struct PdfParser {
    data: Vec<u8>,
}

impl PdfParser {
    /// Create a new PDF parser from file
    pub fn from_file(path: &str) -> Result<Self> {
        if !Path::new(path).exists() {
            return Err(EnhancedError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("PDF file not found: {}", path),
            )));
        }

        let data = fs::read(path)?;
        if !data.starts_with(b"%PDF-") {
            return Err(EnhancedError::InvalidParameter(
                "Not a valid PDF file".into(),
            ));
        }

        Ok(Self { data })
    }

    /// Get page count from PDF
    pub fn page_count(&self) -> Result<usize> {
        let content = String::from_utf8_lossy(&self.data);
        let mut max_count = 1;

        // Look for /Type /Pages and /Count
        for line in content.lines() {
            if line.contains("/Type") && line.contains("/Pages") {
                if let Some(count_pos) = line.find("/Count") {
                    let after_count = &line[count_pos + 6..];
                    if let Some(num_end) =
                        after_count.find(|c: char| !c.is_ascii_digit() && c != ' ')
                    {
                        if let Ok(count) = after_count[..num_end].trim().parse::<usize>() {
                            max_count = max_count.max(count);
                        }
                    }
                }
            }
        }

        Ok(max_count)
    }

    /// Get raw PDF data
    pub fn data(&self) -> &[u8] {
        &self.data
    }
}

/// Extract a page from PDF by parsing PDF structure
pub fn extract_page_object(pdf_path: &str, page_num: usize, output_path: &str) -> Result<()> {
    use super::page_ops::extract_single_page;
    extract_single_page(pdf_path, page_num, output_path)
}

/// Resize page MediaBox by modifying PDF structure
pub fn resize_page_mediabox(
    input_path: &str,
    output_path: &str,
    page_num: usize,
    new_width: f32,
    new_height: f32,
) -> Result<()> {
    use super::page_resize::resize_page;
    resize_page(input_path, output_path, page_num, new_width, new_height)
}
