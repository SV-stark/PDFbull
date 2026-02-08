//! Overlay Merging - Merge overlay PDFs onto target PDF pages
//!
//! This module provides functionality to merge overlay PDFs (like redaction or highlight overlays)
//! onto specific pages of a target PDF.

use super::error::{EnhancedError, Result};
use super::pdf_reader::PdfDocument;
use super::writer::PdfWriter;
use crate::pdf::object::{Array, Dict, Name, Object};
use std::fs;
use std::path::Path;

/// Merge an overlay PDF onto specific pages of a target PDF
///
/// This merges the overlay PDF's content streams onto the target PDF pages.
///
/// # Arguments
/// * `target_path` - Path to target PDF
/// * `overlay_path` - Path to overlay PDF (single page or multi-page)
/// * `output_path` - Path to output PDF
/// * `pages` - Page numbers to apply overlay to (0-indexed, empty = all pages)
pub fn merge_overlay(
    target_path: &str,
    overlay_path: &str,
    output_path: &str,
    pages: &[usize],
) -> Result<()> {
    if !Path::new(target_path).exists() {
        return Err(EnhancedError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Target PDF not found: {}", target_path),
        )));
    }

    if !Path::new(overlay_path).exists() {
        return Err(EnhancedError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Overlay PDF not found: {}", overlay_path),
        )));
    }

    // Parse both PDFs
    let target_doc = PdfDocument::open(target_path)?;
    let overlay_doc = PdfDocument::open(overlay_path)?;

    let target_page_count = target_doc.page_count()?;
    let overlay_page_count = overlay_doc.page_count()?;

    // Determine which pages to process
    let pages_to_process: Vec<usize> = if pages.is_empty() {
        (0..target_page_count).collect()
    } else {
        pages.to_vec()
    };

    // Validate page numbers
    for &page_num in &pages_to_process {
        if page_num >= target_page_count {
            return Err(EnhancedError::InvalidParameter(format!(
                "Page {} does not exist in target PDF (has {} pages)",
                page_num, target_page_count
            )));
        }
    }

    // Merge overlay content onto target pages
    // This requires:
    // 1. Extracting content streams from overlay PDF
    // 2. Appending them to target PDF page content streams
    // 3. Updating page resources if needed
    // 4. Writing the modified PDF

    // Read overlay PDF data and extract streams
    let overlay_data = fs::read(overlay_path)?;
    let overlay_streams: Vec<(usize, Vec<u8>)> = {
        let mut streams = Vec::new();
        for &page_num in &pages_to_process {
            let overlay_page_num = if overlay_page_count == 1 {
                0
            } else {
                page_num.min(overlay_page_count - 1)
            };
            let stream = extract_page_content_stream(&overlay_data, overlay_page_num)?;
            streams.push((page_num, stream));
        }
        streams
    };

    // Read target PDF data
    let mut target_data = fs::read(target_path)?;

    // First, find and remove xref/trailer/EOF - we'll rebuild it at the end
    remove_xref_trailer(&mut target_data);

    // Track the max object number as we add overlay objects
    let mut max_obj_num = find_max_object_number(&target_data);

    // Merge each overlay into target (this only adds objects, doesn't rebuild xref)
    for (page_num, overlay_content_stream) in overlay_streams {
        // Continue with other pages if one fails
        if let Ok(new_obj_num) = add_overlay_object(
            &mut target_data,
            page_num,
            &overlay_content_stream,
            max_obj_num,
        ) {
            max_obj_num = new_obj_num;
        }
    }

    // Now rebuild the xref table and trailer ONCE for all objects
    rebuild_xref_and_trailer(&mut target_data, max_obj_num + 1)?;

    // Write modified PDF
    fs::write(output_path, target_data)?;

    Ok(())
}

/// Remove xref, trailer, and EOF from PDF data
fn remove_xref_trailer(data: &mut Vec<u8>) {
    // Find xref position - keep the newline before it
    let xref_pos = if let Some(pos) = find_bytes_pattern(data, b"\nxref") {
        pos + 1 // Keep the \n, truncate at 'x'
    } else if let Some(pos) = find_bytes_pattern(data, b"\r\nxref") {
        pos + 2 // Keep the \r\n, truncate at 'x'
    } else if let Some(pos) = find_bytes_pattern(data, b"xref\n") {
        pos // No leading newline
    } else {
        return; // No xref found
    };

    data.truncate(xref_pos);
}

/// Find the maximum object number in the PDF
fn find_max_object_number(data: &[u8]) -> i32 {
    let obj_pattern = b" 0 obj";
    let mut max_obj_num = 0;
    let mut search_pos = 0;

    while let Some(obj_pos) = find_bytes_pattern_from(data, obj_pattern, search_pos) {
        let before_obj = &data[..obj_pos];
        let obj_num = extract_number_backwards(before_obj);
        max_obj_num = max_obj_num.max(obj_num);
        search_pos = obj_pos + obj_pattern.len();
    }

    max_obj_num
}

/// Add an overlay object for a specific page (doesn't rebuild xref)
fn add_overlay_object(
    target_data: &mut Vec<u8>,
    page_num: usize,
    overlay_content: &[u8],
    current_max_obj: i32,
) -> Result<i32> {
    let type_page_pattern = b"/Type /Page";
    let type_pages_pattern = b"/Type /Pages";
    let obj_pattern = b" 0 obj";

    // Find page objects
    let mut page_objects: Vec<(usize, i32)> = Vec::new();
    let mut search_pos = 0;

    while let Some(type_pos) = find_bytes_pattern_from(target_data, type_page_pattern, search_pos) {
        if target_data[type_pos..].starts_with(type_pages_pattern) {
            search_pos = type_pos + type_pages_pattern.len();
            continue;
        }

        if let Some(obj_pos) = rfind_bytes_pattern(&target_data[..type_pos], obj_pattern) {
            let before_obj = &target_data[..obj_pos];
            let obj_num = extract_number_backwards(before_obj);
            if obj_num > 0 {
                page_objects.push((type_pos, obj_num));
            }
        }

        search_pos = type_pos + type_page_pattern.len();
    }

    if page_num >= page_objects.len() {
        return Err(EnhancedError::Generic(format!(
            "Page {} doesn't exist in target (only {} pages)",
            page_num,
            page_objects.len()
        )));
    }

    let (_page_type_pos, page_obj_num) = page_objects[page_num];
    let new_obj_num = current_max_obj + 1;

    // Find the page object
    let obj_marker = format!("{} 0 obj", page_obj_num);
    let obj_marker_bytes = obj_marker.as_bytes();
    let obj_pos = find_bytes_pattern_from(target_data, obj_marker_bytes, 0)
        .ok_or_else(|| EnhancedError::Generic(format!("Page object {} not found", page_obj_num)))?;

    // Find /Contents in the page object section
    let page_section_end = (obj_pos + 2000).min(target_data.len());
    let page_section = &target_data[obj_pos..page_section_end];

    let contents_pattern = b"/Contents";
    let contents_pos = find_bytes_pattern_from(page_section, contents_pattern, 0)
        .ok_or_else(|| EnhancedError::Generic("No /Contents in page object".into()))?;

    // Parse the original content object number(s)
    let after_contents = &page_section[contents_pos + 9..];

    // Check if /Contents is already an array
    let trimmed = after_contents
        .iter()
        .skip_while(|&&b| b == b' ')
        .cloned()
        .collect::<Vec<u8>>();

    let (original_content_refs, is_array) = if trimmed.starts_with(b"[") {
        // Already an array - extract all references
        let mut refs = Vec::new();
        let mut i = 1; // Skip '['
        while i < trimmed.len() {
            if trimmed[i] == b']' {
                break;
            }
            if trimmed[i].is_ascii_digit() {
                let num = extract_first_number(&trimmed[i..]);
                if let Some(n) = num {
                    refs.push(n);
                    // Skip past this reference
                    while i < trimmed.len()
                        && (trimmed[i].is_ascii_digit()
                            || trimmed[i] == b' '
                            || trimmed[i] == b'0'
                            || trimmed[i] == b'R')
                    {
                        i += 1;
                    }
                    continue;
                }
            }
            i += 1;
        }
        (refs, true)
    } else {
        // Single reference
        let original_content_obj = extract_first_number(after_contents)
            .ok_or_else(|| EnhancedError::Generic("Invalid /Contents reference".into()))?;
        (vec![original_content_obj], false)
    };

    // Create the new content stream object
    let overlay_str = String::from_utf8_lossy(overlay_content);
    let new_stream_obj = format!(
        "{} 0 obj\n<</Length {}>>\nstream\n{}\nendstream\nendobj\n",
        new_obj_num,
        overlay_content.len(),
        overlay_str
    );

    // Update the page's /Contents to include the new overlay reference
    if is_array {
        // Contents is already an array - find it and add to it
        let abs_contents_pos = obj_pos + contents_pos;
        let after_abs = &target_data[abs_contents_pos + 9..];
        if let Some(bracket_pos) = after_abs.iter().position(|&b| b == b'[') {
            if let Some(close_pos) = after_abs.iter().position(|&b| b == b']') {
                let insert_pos = abs_contents_pos + 9 + close_pos;
                let new_ref = format!(" {} 0 R", new_obj_num);
                let new_ref_bytes = new_ref.as_bytes();
                target_data.splice(insert_pos..insert_pos, new_ref_bytes.iter().copied());
            }
        }
    } else {
        // Single reference - convert to array
        let base_pattern = format!("/Contents {} 0 R", original_content_refs[0]);
        let base_bytes = base_pattern.as_bytes();

        if let Some(contents_pos) = find_bytes_pattern(target_data, base_bytes) {
            let pattern_end = contents_pos + base_bytes.len();
            let new_contents = format!(
                "/Contents [{} 0 R {} 0 R]",
                original_content_refs[0], new_obj_num
            );
            let new_bytes = new_contents.as_bytes();
            target_data.splice(contents_pos..pattern_end, new_bytes.iter().copied());
        }
    }

    // Append the new object to the end of the PDF
    target_data.extend_from_slice(new_stream_obj.as_bytes());

    Ok(new_obj_num)
}

/// Rebuild xref table and trailer (called once after all modifications)
fn rebuild_xref_and_trailer(data: &mut Vec<u8>, obj_count: i32) -> Result<()> {
    // Collect all object offsets
    let mut offsets = vec![0usize; obj_count as usize];

    // Find all "N 0 obj" patterns
    let mut search_pos = 0;
    while search_pos < data.len() {
        // Look for digit at line start
        if search_pos == 0 || data[search_pos - 1] == b'\n' || data[search_pos - 1] == b'\r' {
            if data[search_pos].is_ascii_digit() {
                let num_start = search_pos;
                let mut num_end = search_pos;
                while num_end < data.len() && data[num_end].is_ascii_digit() {
                    num_end += 1;
                }

                // Check for " 0 obj"
                if num_end + 6 <= data.len() {
                    let after_num = &data[num_end..num_end + 6];
                    if after_num == b" 0 obj" {
                        if let Ok(obj_num_str) = std::str::from_utf8(&data[num_start..num_end]) {
                            if let Ok(obj_num) = obj_num_str.parse::<i32>() {
                                if obj_num > 0 && (obj_num as usize) < offsets.len() {
                                    offsets[obj_num as usize] = search_pos;
                                }
                            }
                        }
                        search_pos = num_end + 6;
                        continue;
                    }
                }
            }
        }
        search_pos += 1;
    }

    // Write new xref table
    let xref_offset = data.len();
    data.extend_from_slice(format!("xref\n0 {}\n", obj_count).as_bytes());
    data.extend_from_slice(b"0000000000 65535 f \n");

    for i in 1..obj_count as usize {
        let offset = offsets.get(i).copied().unwrap_or(0);
        data.extend_from_slice(format!("{:010} 00000 n \n", offset).as_bytes());
    }

    // Write trailer
    data.extend_from_slice(format!("trailer\n<</Size {}/Root 1 0 R>>\n", obj_count).as_bytes());
    data.extend_from_slice(format!("startxref\n{}\n%%EOF\n", xref_offset).as_bytes());

    Ok(())
}

/// Extract content stream from a page (using byte-level operations for safety)
fn extract_page_content_stream(data: &[u8], page_num: usize) -> Result<Vec<u8>> {
    // Find page object by looking for /Type /Page (not /Type /Pages)
    // Use byte-level operations to avoid UTF-8 boundary issues
    let type_page_pattern = b"/Type /Page";
    let type_pages_pattern = b"/Type /Pages";
    let obj_pattern = b" 0 obj";

    let mut page_objects: Vec<(usize, i32)> = Vec::new();
    let mut search_pos = 0;

    while let Some(type_pos) = find_bytes_pattern_from(data, type_page_pattern, search_pos) {
        // Check this isn't /Type /Pages
        if data[type_pos..].starts_with(type_pages_pattern) {
            search_pos = type_pos + type_pages_pattern.len();
            continue;
        }

        // Find the object number by looking back for " 0 obj"
        if let Some(obj_pos) = rfind_bytes_pattern(&data[..type_pos], obj_pattern) {
            // Find the number before " 0 obj"
            let before_obj = &data[..obj_pos];
            let obj_num = extract_number_backwards(before_obj);

            if obj_num > 0 {
                page_objects.push((type_pos, obj_num));
            }
        }

        search_pos = type_pos + type_page_pattern.len();
    }

    if page_num >= page_objects.len() {
        return Ok(Vec::new());
    }

    let (page_type_pos, _page_obj_num) = page_objects[page_num];

    // Find /Contents in this page object
    // Look in a window around the /Type /Page
    let search_start = page_type_pos.saturating_sub(500);
    let search_end = (page_type_pos + 500).min(data.len());
    let page_section = &data[search_start..search_end];

    let contents_pattern = b"/Contents";
    let content_ref: Option<i32> =
        if let Some(contents_pos) = find_bytes_pattern_from(page_section, contents_pattern, 0) {
            // Parse the content reference number after /Contents
            let after_contents = &page_section[contents_pos + 9..];
            extract_first_number(after_contents)
        } else {
            None
        };

    if let Some(content_obj_num) = content_ref {
        // Find and extract the content stream
        let obj_marker = format!("{} 0 obj", content_obj_num);
        let obj_marker_bytes = obj_marker.as_bytes();

        if let Some(obj_pos) = find_bytes_pattern_from(data, obj_marker_bytes, 0) {
            // Find the end of this object
            let search_area = &data[obj_pos..];
            let obj_end = find_bytes_pattern_from(search_area, b"endobj", 0).unwrap_or(5000);
            let obj_section = &data[obj_pos..obj_pos + obj_end.min(search_area.len())];

            // Check if stream is compressed
            let is_flate = find_bytes_pattern_from(obj_section, b"/FlateDecode", 0).is_some()
                || find_bytes_pattern_from(obj_section, b"/Filter", 0).is_some();

            // Look for stream data
            if let Some(stream_pos) = find_bytes_pattern_from(search_area, b"stream", 0) {
                let mut stream_data_start = obj_pos + stream_pos + 6; // After "stream"

                // Skip newline(s)
                while stream_data_start < data.len()
                    && (data[stream_data_start] == b'\r' || data[stream_data_start] == b'\n')
                {
                    stream_data_start += 1;
                }

                if let Some(endstream_pos) =
                    find_bytes_pattern_from(&data[stream_data_start..], b"endstream", 0)
                {
                    let mut actual_end = stream_data_start + endstream_pos;

                    // Remove trailing newlines before endstream
                    while actual_end > stream_data_start
                        && (data[actual_end - 1] == b'\r' || data[actual_end - 1] == b'\n')
                    {
                        actual_end -= 1;
                    }

                    let stream_data = &data[stream_data_start..actual_end];

                    // Decompress if needed
                    if is_flate && !stream_data.is_empty() {
                        use flate2::read::ZlibDecoder;
                        use std::io::Read;

                        let mut decoder = ZlibDecoder::new(stream_data);
                        let mut decompressed = Vec::new();
                        match decoder.read_to_end(&mut decompressed) {
                            Ok(_) => {
                                return Ok(decompressed);
                            }
                            Err(_) => {
                                // Decompression failed, fall through to return raw data
                            }
                        }
                    }

                    return Ok(stream_data.to_vec());
                }
            }
        }
    }

    // Return empty content if not found
    Ok(Vec::new())
}

/// Find bytes pattern starting from a position
fn find_bytes_pattern_from(haystack: &[u8], needle: &[u8], start: usize) -> Option<usize> {
    if needle.is_empty() || start >= haystack.len() || needle.len() > haystack.len() - start {
        return None;
    }
    haystack[start..]
        .windows(needle.len())
        .position(|window| window == needle)
        .map(|pos| start + pos)
}

/// Reverse find bytes pattern
fn rfind_bytes_pattern(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || needle.len() > haystack.len() {
        return None;
    }
    haystack
        .windows(needle.len())
        .rposition(|window| window == needle)
}

/// Extract a number from the end of a byte slice (reading backwards)
fn extract_number_backwards(data: &[u8]) -> i32 {
    let mut digits = Vec::new();
    for &b in data.iter().rev() {
        if b.is_ascii_digit() {
            digits.push(b);
        } else if !digits.is_empty() {
            break;
        }
    }
    digits.reverse();
    String::from_utf8_lossy(&digits).parse().unwrap_or(0)
}

/// Extract first number from a byte slice
fn extract_first_number(data: &[u8]) -> Option<i32> {
    let mut start = 0;

    // Skip whitespace
    while start < data.len() && data[start].is_ascii_whitespace() {
        start += 1;
    }

    // Collect digits
    let mut end = start;
    while end < data.len() && data[end].is_ascii_digit() {
        end += 1;
    }

    if end > start {
        String::from_utf8_lossy(&data[start..end]).parse().ok()
    } else {
        None
    }
}

/// Find a byte pattern in a byte slice
fn find_bytes_pattern(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || needle.len() > haystack.len() {
        return None;
    }
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}
