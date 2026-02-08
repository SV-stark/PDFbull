//! Page Resizing - Resize PDF page dimensions
//!
//! This module provides functionality to resize PDF page MediaBox dimensions
//! by modifying the PDF structure directly.

use super::error::{EnhancedError, Result};
use super::pdf_reader::PdfDocument;
use super::writer::PdfWriter;
use crate::pdf::object::{Array, Dict, Name, Object};
use std::fs;
use std::io::Write;
use std::path::Path;

/// Resize a page in a PDF to new dimensions
///
/// Modifies the MediaBox directly in the PDF structure.
///
/// # Arguments
/// * `input_path` - Path to input PDF
/// * `output_path` - Path to output PDF
/// * `page_num` - Page number (0-indexed)
/// * `new_width` - New width in points
/// * `new_height` - New height in points
pub fn resize_page(
    input_path: &str,
    output_path: &str,
    page_num: usize,
    new_width: f32,
    new_height: f32,
) -> Result<()> {
    if !Path::new(input_path).exists() {
        return Err(EnhancedError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("PDF file not found: {}", input_path),
        )));
    }

    if new_width <= 0.0 || new_height <= 0.0 {
        return Err(EnhancedError::InvalidParameter(format!(
            "Invalid dimensions: {}x{}",
            new_width, new_height
        )));
    }

    // Parse the PDF
    let doc = PdfDocument::open(input_path)?;
    let page_count = doc.page_count()?;

    if page_num >= page_count {
        return Err(EnhancedError::InvalidParameter(format!(
            "Page {} does not exist (document has {} pages)",
            page_num, page_count
        )));
    }

    // Get page object number
    let page_obj_num = doc
        .get_page_object(page_num)
        .ok_or_else(|| EnhancedError::Generic("Could not find page object".into()))?;

    // Read PDF data
    let mut pdf_data = fs::read(input_path)?;

    // Find and modify MediaBox for this page
    modify_page_mediabox(&mut pdf_data, page_obj_num, new_width, new_height)?;

    // Write modified PDF
    fs::write(output_path, pdf_data)?;

    Ok(())
}

/// Modify MediaBox in PDF data
fn modify_page_mediabox(
    pdf_data: &mut Vec<u8>,
    page_obj_num: i32,
    new_width: f32,
    new_height: f32,
) -> Result<()> {
    let content = String::from_utf8_lossy(pdf_data);

    // Find the page object
    let obj_pattern = format!("{} 0 obj", page_obj_num);
    if let Some(obj_pos) = content.find(&obj_pattern) {
        // Find MediaBox in this object
        let page_section_start = obj_pos;
        let page_section_end = content[obj_pos..]
            .find("endobj")
            .map(|i| obj_pos + i)
            .unwrap_or(pdf_data.len());

        let page_section = &content[page_section_start..page_section_end];

        if let Some(mb_pos) = page_section.find("/MediaBox") {
            // Find the array: [x0 y0 x1 y1]
            let after_mb = &page_section[mb_pos + 9..];
            if let Some(bracket_start) = after_mb.find('[') {
                if let Some(bracket_end) = after_mb[bracket_start + 1..].find(']') {
                    let array_start = obj_pos + mb_pos + 9 + bracket_start;
                    let array_end = array_start + bracket_end + 1;

                    // Replace MediaBox array with new dimensions
                    let new_mediabox = format!("[0 0 {} {}]", new_width, new_height);

                    // Convert to bytes for replacement
                    let new_bytes = new_mediabox.as_bytes();
                    let old_bytes = &pdf_data[array_start..=array_end];

                    // Replace in the vector
                    let start_idx = array_start;
                    let end_idx = array_end + 1;

                    pdf_data.splice(start_idx..end_idx, new_bytes.iter().copied());

                    return Ok(());
                }
            }
        }
    }

    Err(EnhancedError::Generic(
        "Could not find MediaBox in page object".into(),
    ))
}
