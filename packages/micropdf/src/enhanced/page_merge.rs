//! PDF Merge with Complete Resource Preservation
//!
//! This module provides PDF merging that properly preserves all page resources
//! including fonts, images, graphics states, and other objects.
//! All operations work at the byte level to avoid corrupting binary streams.

use super::error::{EnhancedError, Result};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Merge multiple PDFs while preserving all resources
///
/// This function extracts each page with all its resources using the comprehensive
/// page copy approach, then combines them into a single document.
pub fn merge_pdfs_comprehensive(input_paths: &[String], output_path: &str) -> Result<usize> {
    if input_paths.is_empty() {
        return Err(EnhancedError::InvalidParameter(
            "At least one input PDF required".into(),
        ));
    }

    // Collect all pages with their data
    let mut page_pdfs: Vec<Vec<u8>> = Vec::new();

    // Extract each page from each input PDF
    for input_path in input_paths {
        if !Path::new(input_path).exists() {
            return Err(EnhancedError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("PDF file not found: {}", input_path),
            )));
        }

        let source_data = fs::read(input_path)?;
        if !source_data.starts_with(b"%PDF-") {
            return Err(EnhancedError::InvalidParameter(format!(
                "Not a valid PDF file: {}",
                input_path
            )));
        }

        let page_count = count_pages(&source_data)?;

        for page_idx in 0..page_count {
            // Extract page using comprehensive copy
            let temp_path = format!(
                "/tmp/micropdf_merge_{}_{}_{}.pdf",
                std::process::id(),
                input_paths
                    .iter()
                    .position(|p| p == input_path)
                    .unwrap_or(0),
                page_idx
            );

            super::page_copy::copy_page_complete(input_path, page_idx, &temp_path)?;

            let page_data = fs::read(&temp_path)?;
            page_pdfs.push(page_data);

            let _ = fs::remove_file(&temp_path);
        }
    }

    let total_pages = page_pdfs.len();
    if total_pages == 0 {
        return Err(EnhancedError::InvalidParameter("No pages to merge".into()));
    }

    // Now combine all pages into one PDF using byte-level operations
    let merged_data = combine_page_pdfs_bytes(&page_pdfs)?;
    fs::write(output_path, merged_data)?;

    Ok(total_pages)
}

/// Count pages in a PDF by looking at the /Count entry
fn count_pages(data: &[u8]) -> Result<usize> {
    // Find /Type /Pages
    if let Some(pages_pos) = find_pattern(data, b"/Type /Pages") {
        // Look for /Count in the next 200 bytes
        let end = std::cmp::min(pages_pos + 200, data.len());
        if let Some(count_pos) = find_pattern(&data[pages_pos..end], b"/Count") {
            let abs_count_pos = pages_pos + count_pos + 6;
            // Skip whitespace and parse number
            let mut i = abs_count_pos;
            while i < data.len() && data[i].is_ascii_whitespace() {
                i += 1;
            }
            let num_start = i;
            while i < data.len() && data[i].is_ascii_digit() {
                i += 1;
            }
            if i > num_start {
                if let Ok(count) = std::str::from_utf8(&data[num_start..i])
                    .unwrap_or("0")
                    .parse::<usize>()
                {
                    if count > 0 {
                        return Ok(count);
                    }
                }
            }
        }
    }

    // Fallback: count /Type /Page occurrences (not /Type /Pages)
    let mut count = 0;
    let mut pos = 0;
    while pos < data.len() {
        if let Some(found) = find_pattern(&data[pos..], b"/Type /Page") {
            let abs_pos = pos + found;
            // Check next char isn't 's' (to distinguish from /Type /Pages)
            if abs_pos + 11 < data.len() && data[abs_pos + 11] != b's' && data[abs_pos + 11] != b'S'
            {
                count += 1;
            }
            pos = abs_pos + 11;
        } else {
            break;
        }
    }

    if count > 0 { Ok(count) } else { Ok(1) }
}

/// Find a byte pattern in data
fn find_pattern(data: &[u8], pattern: &[u8]) -> Option<usize> {
    data.windows(pattern.len())
        .position(|window| window == pattern)
}

/// Combine multiple single-page PDFs into one using byte-level operations
/// This preserves all binary data including fonts and images
fn combine_page_pdfs_bytes(page_pdfs: &[Vec<u8>]) -> Result<Vec<u8>> {
    if page_pdfs.is_empty() {
        return Err(EnhancedError::InvalidParameter(
            "No pages to combine".into(),
        ));
    }

    if page_pdfs.len() == 1 {
        return Ok(page_pdfs[0].clone());
    }

    // Parse each page PDF and collect objects at byte level
    let mut all_objects: Vec<ObjectData> = Vec::new();
    let mut page_obj_nums: Vec<i32> = Vec::new();
    let mut next_obj_num: i32 = 3; // 1=Catalog, 2=Pages

    for page_data in page_pdfs {
        let (objects, page_num, next_num) = extract_objects_bytes(page_data, next_obj_num)?;
        page_obj_nums.push(page_num);
        all_objects.extend(objects);
        next_obj_num = next_num;
    }

    // Build output PDF
    let mut output = Vec::new();
    let mut offsets: HashMap<i32, usize> = HashMap::new();

    // PDF header
    output.extend_from_slice(b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n");

    // Object 1: Catalog
    offsets.insert(1, output.len());
    output.extend_from_slice(b"1 0 obj\n<</Type/Catalog/Pages 2 0 R>>\nendobj\n");

    // Object 2: Pages
    offsets.insert(2, output.len());
    let kids_str: String = page_obj_nums
        .iter()
        .map(|r| format!("{} 0 R", r))
        .collect::<Vec<_>>()
        .join(" ");
    output.extend_from_slice(
        format!(
            "2 0 obj\n<</Type/Pages/Count {}/Kids[{}]>>\nendobj\n",
            page_obj_nums.len(),
            kids_str
        )
        .as_bytes(),
    );

    // Write all objects
    for obj in &all_objects {
        offsets.insert(obj.new_num, output.len());
        output.extend_from_slice(&obj.data);
    }

    // Write xref table
    let xref_offset = output.len();
    output.extend_from_slice(format!("xref\n0 {}\n", next_obj_num).as_bytes());
    output.extend_from_slice(b"0000000000 65535 f \n");

    for i in 1..next_obj_num {
        let offset = offsets.get(&i).unwrap_or(&0);
        output.extend_from_slice(format!("{:010} 00000 n \n", offset).as_bytes());
    }

    // Write trailer
    output
        .extend_from_slice(format!("trailer\n<</Size {}/Root 1 0 R>>\n", next_obj_num).as_bytes());
    output.extend_from_slice(format!("startxref\n{}\n%%EOF\n", xref_offset).as_bytes());

    Ok(output)
}

/// Object data extracted from a single-page PDF
struct ObjectData {
    data: Vec<u8>,
    new_num: i32,
    is_page: bool,
}

/// Extract objects from a single-page PDF using byte-level operations
/// Returns (objects, page_object_number, next_available_object_number)
fn extract_objects_bytes(
    pdf_data: &[u8],
    start_obj_num: i32,
) -> Result<(Vec<ObjectData>, i32, i32)> {
    let mut objects: Vec<ObjectData> = Vec::new();
    let mut obj_mapping: HashMap<i32, i32> = HashMap::new();
    let mut page_new_num = start_obj_num;
    let mut next_new = start_obj_num;

    // Find all object declarations
    let obj_positions = find_all_object_positions(pdf_data);

    // First pass: create mapping for objects > 2
    for &(_, old_num) in &obj_positions {
        if old_num > 2 {
            obj_mapping.insert(old_num, next_new);
            next_new += 1;
        }
    }

    // Second pass: extract and renumber objects at byte level
    for i in 0..obj_positions.len() {
        let (start_pos, old_num) = obj_positions[i];

        // Skip catalog and pages (objects 1 and 2)
        if old_num <= 2 {
            continue;
        }

        // Find object boundaries
        let obj_start = find_line_start(pdf_data, start_pos);
        let obj_end = find_object_end_bytes(pdf_data, start_pos)?;

        // Extract the object data
        let obj_data = &pdf_data[obj_start..obj_end];

        // Check if this is a page object
        let is_page =
            contains_pattern(obj_data, b"/Type /Page") || contains_pattern(obj_data, b"/Type/Page");
        let is_pages = contains_pattern(obj_data, b"/Type /Pages")
            || contains_pattern(obj_data, b"/Type/Pages");

        if is_pages {
            continue; // Skip /Pages objects
        }

        let new_num = obj_mapping.get(&old_num).copied().unwrap_or(old_num);

        if is_page {
            page_new_num = new_num;
        }

        // Renumber the object at byte level
        let renumbered_data =
            renumber_object_bytes(obj_data, old_num, new_num, &obj_mapping, is_page)?;

        objects.push(ObjectData {
            data: renumbered_data,
            new_num,
            is_page,
        });
    }

    Ok((objects, page_new_num, next_new))
}

/// Find all "N 0 obj" patterns and their positions in the PDF
fn find_all_object_positions(data: &[u8]) -> Vec<(usize, i32)> {
    let mut results = Vec::new();
    let mut pos = 0;

    while pos < data.len() {
        // Look for digits
        if pos == 0 || data[pos - 1].is_ascii_whitespace() || data[pos - 1] == b'\n' {
            let num_start = pos;
            let mut num_end = pos;

            while num_end < data.len() && data[num_end].is_ascii_digit() {
                num_end += 1;
            }

            if num_end > num_start {
                // Parse the object number
                if let Ok(obj_num) = std::str::from_utf8(&data[num_start..num_end])
                    .unwrap_or("")
                    .parse::<i32>()
                {
                    // Check for " 0 obj"
                    let mut check_pos = num_end;

                    // Skip whitespace
                    while check_pos < data.len() && data[check_pos].is_ascii_whitespace() {
                        check_pos += 1;
                    }

                    // Check for "0"
                    if check_pos < data.len() && data[check_pos] == b'0' {
                        check_pos += 1;

                        // Skip whitespace
                        while check_pos < data.len() && data[check_pos].is_ascii_whitespace() {
                            check_pos += 1;
                        }

                        // Check for "obj"
                        if check_pos + 3 <= data.len() && &data[check_pos..check_pos + 3] == b"obj"
                        {
                            // Make sure it's not "object" or similar
                            if check_pos + 3 >= data.len()
                                || !data[check_pos + 3].is_ascii_alphanumeric()
                            {
                                results.push((num_start, obj_num));
                                pos = check_pos + 3;
                                continue;
                            }
                        }
                    }
                }
            }
        }
        pos += 1;
    }

    results
}

/// Find the start of the line containing a position
fn find_line_start(data: &[u8], pos: usize) -> usize {
    let mut start = pos;
    while start > 0 && data[start - 1] != b'\n' {
        start -= 1;
    }
    start
}

/// Find where an object ends (finds the endobj or endstream followed by endobj)
fn find_object_end_bytes(data: &[u8], start: usize) -> Result<usize> {
    let search_region = &data[start..];

    // First find the first "endobj" to limit our search for stream keywords
    // This ensures we don't accidentally pick up content from a subsequent object
    let first_endobj = find_pattern(search_region, b"endobj")
        .ok_or_else(|| EnhancedError::InvalidParameter("Could not find endobj".into()))?;

    let object_region = &search_region[..first_endobj];

    // Look for "stream" keyword within this object only
    // It must be followed by \r\n or \n to be a valid stream marker
    if let Some(stream_pos) = find_stream_keyword_in_region(object_region) {
        // This is a stream object - need to find endstream then endobj
        let abs_stream = start + stream_pos;

        if let Some(endstream_pos) = find_pattern(&data[abs_stream..], b"endstream") {
            let after_endstream = abs_stream + endstream_pos + 9;

            // Skip whitespace after endstream
            let mut end_pos = after_endstream;
            while end_pos < data.len()
                && (data[end_pos] == b' '
                    || data[end_pos] == b'\t'
                    || data[end_pos] == b'\r'
                    || data[end_pos] == b'\n')
            {
                end_pos += 1;
            }

            // Now look for endobj
            if let Some(endobj_pos) = find_pattern(&data[end_pos..], b"endobj") {
                // Include endobj and any trailing newline
                let mut result = end_pos + endobj_pos + 6;
                while result < data.len() && (data[result] == b'\r' || data[result] == b'\n') {
                    result += 1;
                }
                return Ok(result);
            }
        }
    }

    // Non-stream object: use the first endobj we found
    // Include any trailing newline
    let mut result = start + first_endobj + 6;
    while result < data.len() && (data[result] == b'\r' || data[result] == b'\n') {
        result += 1;
    }
    Ok(result)
}

/// Find "stream" keyword that starts actual stream data in a limited region
fn find_stream_keyword_in_region(data: &[u8]) -> Option<usize> {
    let mut pos = 0;
    while pos < data.len() {
        if let Some(found) = find_pattern(&data[pos..], b"stream") {
            let abs_pos = pos + found;
            let after_stream = abs_pos + 6;

            // Check if followed by \r\n or \n (required by PDF spec for stream)
            if after_stream < data.len() {
                let next_byte = data[after_stream];
                if next_byte == b'\r' || next_byte == b'\n' {
                    // Verify there's a << before this (stream dictionary)
                    if find_pattern(&data[..abs_pos], b"<<").is_some() {
                        return Some(abs_pos);
                    }
                }
            }
            pos = abs_pos + 1;
        } else {
            break;
        }
    }
    None
}

/// Renumber an object and its references at the byte level
/// This preserves binary stream content while only modifying text parts
fn renumber_object_bytes(
    data: &[u8],
    old_num: i32,
    new_num: i32,
    mapping: &HashMap<i32, i32>,
    is_page: bool,
) -> Result<Vec<u8>> {
    // Find where the stream keyword starts (if any)
    // Use the proper stream detection that verifies it's a real stream marker
    let stream_keyword_pos = find_stream_keyword_in_region(data);

    if let Some(stream_pos) = stream_keyword_pos {
        // This object has a stream - we need to be very careful
        // Only modify the dictionary part BEFORE "stream", copy the rest byte-for-byte

        // The dictionary is from the start to just before "stream"
        let dict_part = &data[..stream_pos];
        let stream_and_rest = &data[stream_pos..];

        // Process only the dictionary part
        let result_dict = renumber_dict_content(dict_part, old_num, new_num, mapping, is_page);

        // Build result: modified dictionary + unchanged stream
        let mut result = Vec::new();
        result.extend_from_slice(&result_dict);
        result.extend_from_slice(stream_and_rest); // Copy stream part byte-for-byte

        Ok(result)
    } else {
        // No stream - safe to process the whole object
        let result = renumber_dict_content(data, old_num, new_num, mapping, is_page);
        Ok(result)
    }
}

/// Renumber references in dictionary content, being careful to match whole numbers only
fn renumber_dict_content(
    data: &[u8],
    old_num: i32,
    new_num: i32,
    mapping: &HashMap<i32, i32>,
    is_page: bool,
) -> Vec<u8> {
    let mut result = Vec::with_capacity(data.len() + 100);
    let mut i = 0;

    // First, replace the object header "old_num 0 obj" with "new_num 0 obj"
    let old_header = format!("{} 0 obj", old_num);
    let data_str = String::from_utf8_lossy(data);
    let header_replaced = data_str.replacen(&old_header, &format!("{} 0 obj", new_num), 1);
    let data = header_replaced.as_bytes();

    while i < data.len() {
        // Look for potential object reference pattern: digit followed by space
        if data[i].is_ascii_digit() {
            let num_start = i;

            // Check if preceded by whitespace, '[', '<', '/', or start of data
            let preceded_ok = num_start == 0
                || data[num_start - 1].is_ascii_whitespace()
                || data[num_start - 1] == b'['
                || data[num_start - 1] == b'<'
                || data[num_start - 1] == b'/';

            if preceded_ok {
                // Parse the number
                let mut num_end = i;
                while num_end < data.len() && data[num_end].is_ascii_digit() {
                    num_end += 1;
                }

                if num_end > num_start {
                    // Check for " 0 R" pattern after the number
                    let after_num = num_end;
                    let mut check_pos = after_num;

                    // Skip optional whitespace
                    while check_pos < data.len() && data[check_pos] == b' ' {
                        check_pos += 1;
                    }

                    // Check for "0"
                    if check_pos < data.len() && data[check_pos] == b'0' {
                        let zero_pos = check_pos;
                        check_pos += 1;

                        // Skip optional whitespace
                        while check_pos < data.len() && data[check_pos] == b' ' {
                            check_pos += 1;
                        }

                        // Check for "R"
                        if check_pos < data.len() && data[check_pos] == b'R' {
                            // Make sure R is followed by non-alphanumeric
                            let r_pos = check_pos;
                            check_pos += 1;

                            let followed_ok =
                                check_pos >= data.len() || !data[check_pos].is_ascii_alphanumeric();

                            if followed_ok {
                                // Parse the object number
                                if let Ok(ref_num) = std::str::from_utf8(&data[num_start..num_end])
                                    .unwrap_or("")
                                    .parse::<i32>()
                                {
                                    // Look up in mapping
                                    if let Some(&new_ref_num) = mapping.get(&ref_num) {
                                        // Write the remapped reference
                                        result.extend_from_slice(
                                            format!("{} 0 R", new_ref_num).as_bytes(),
                                        );
                                        i = r_pos + 1;
                                        continue;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        result.push(data[i]);
        i += 1;
    }

    // If this is a page, update the /Parent reference to point to object 2
    if is_page {
        let result_str = String::from_utf8_lossy(&result);
        let updated = replace_parent_ref(&result_str, 2);
        return updated.into_bytes();
    }

    result
}

/// Replace /Parent reference with new parent number
fn replace_parent_ref(content: &str, new_parent: i32) -> String {
    // Find /Parent and replace the reference
    let mut result = String::new();
    let mut chars = content.char_indices().peekable();

    while let Some((i, c)) = chars.next() {
        result.push(c);

        // Check for "/Parent"
        if c == '/' && content[i..].starts_with("/Parent") {
            result.push_str("Parent");
            // Skip "Parent"
            for _ in 0..6 {
                chars.next();
            }

            // Skip whitespace
            while let Some(&(_, wc)) = chars.peek() {
                if wc.is_whitespace() {
                    result.push(wc);
                    chars.next();
                } else {
                    break;
                }
            }

            // Skip old object number
            while let Some(&(_, dc)) = chars.peek() {
                if dc.is_ascii_digit() {
                    chars.next();
                } else {
                    break;
                }
            }

            // Skip whitespace
            while let Some(&(_, wc)) = chars.peek() {
                if wc.is_whitespace() {
                    chars.next();
                } else {
                    break;
                }
            }

            // Skip "0"
            if let Some(&(_, '0')) = chars.peek() {
                chars.next();
            }

            // Skip whitespace
            while let Some(&(_, wc)) = chars.peek() {
                if wc.is_whitespace() {
                    chars.next();
                } else {
                    break;
                }
            }

            // Skip "R"
            if let Some(&(_, 'R')) = chars.peek() {
                chars.next();
            }

            // Write new reference
            result.push_str(&format!("{} 0 R", new_parent));
        }
    }

    result
}

/// Check if data contains a pattern
fn contains_pattern(data: &[u8], pattern: &[u8]) -> bool {
    find_pattern(data, pattern).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn create_test_pdf() -> NamedTempFile {
        let mut temp = NamedTempFile::new().unwrap();
        let pdf_content = b"%PDF-1.4\n\
            1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
            2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
            3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] >>\nendobj\n\
            xref\n0 4\n0000000000 65535 f \n0000000009 00000 n \n\
            0000000058 00000 n \n0000000115 00000 n \n\
            trailer\n<< /Size 4 /Root 1 0 R >>\nstartxref\n190\n%%EOF\n";
        temp.write_all(pdf_content).unwrap();
        temp.flush().unwrap();
        temp
    }

    #[test]
    fn test_count_pages() {
        let temp = create_test_pdf();
        let data = fs::read(temp.path()).unwrap();
        let count = count_pages(&data).unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_merge_empty() {
        let result = merge_pdfs_comprehensive(&[], "/tmp/test_out.pdf");
        assert!(result.is_err());
    }

    #[test]
    fn test_merge_single() {
        let temp = create_test_pdf();
        let output = NamedTempFile::new().unwrap();

        let result = merge_pdfs_comprehensive(
            &[temp.path().to_str().unwrap().to_string()],
            output.path().to_str().unwrap(),
        );

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1);
    }

    #[test]
    fn test_find_pattern() {
        let data = b"hello world test";
        assert_eq!(find_pattern(data, b"world"), Some(6));
        assert_eq!(find_pattern(data, b"xyz"), None);
    }

    #[test]
    fn test_find_all_object_positions() {
        let data = b"1 0 obj\n<<test>>\nendobj\n3 0 obj\n<<data>>\nendobj\n";
        let positions = find_all_object_positions(data);
        assert_eq!(positions.len(), 2);
        assert_eq!(positions[0].1, 1);
        assert_eq!(positions[1].1, 3);
    }
}
