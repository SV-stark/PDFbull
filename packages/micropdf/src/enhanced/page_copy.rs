//! Page Copying with Complete Resource Handling
//!
//! This module properly copies PDF pages including all referenced resources
//! (fonts, XObjects, graphics states, etc.) to create standalone PDF files.

use super::error::{EnhancedError, Result};
use std::collections::{HashMap, HashSet};
use std::fs;

/// Copies a single page from source PDF, preserving all resources
pub fn copy_page_complete(input_path: &str, page_num: usize, output_path: &str) -> Result<()> {
    let source_data = fs::read(input_path)?;

    // Find the page object
    let page_obj_num = find_page_object(&source_data, page_num)?;

    // Extract page dictionary boundaries
    let (page_start, page_end) = find_object_boundaries(&source_data, page_obj_num)?;
    let page_dict_data = &source_data[page_start..page_end];

    // Get MediaBox
    let media_box = extract_media_box(page_dict_data)?;

    // Extract /Contents reference and get content stream object numbers
    let content_obj_nums = extract_content_references(page_dict_data)?;

    // Extract /Resources reference
    let resources_ref = extract_resources_reference(page_dict_data, &source_data)?;

    // Collect all objects we need to copy
    let mut objects_to_copy: HashSet<i32> = HashSet::new();

    // Add content stream objects
    for obj_num in &content_obj_nums {
        objects_to_copy.insert(*obj_num);
    }

    // Add all resources and their dependencies
    if let Some(ref res) = resources_ref {
        collect_resource_objects(&source_data, res, &mut objects_to_copy)?;
    }

    // Remove object 0 if present (it's the free object entry, not a real object)
    objects_to_copy.remove(&0);

    // Build object mapping: old_num -> new_num
    let mut obj_mapping: HashMap<i32, i32> = HashMap::new();
    let mut next_obj = 4; // Start at 4 (1=Catalog, 2=Pages, 3=Page)

    // Sort objects for deterministic output
    let mut sorted_objects: Vec<i32> = objects_to_copy.iter().copied().collect();
    sorted_objects.sort();

    for old_num in &sorted_objects {
        obj_mapping.insert(*old_num, next_obj);
        next_obj += 1;
    }

    // Debug: write mapping to file for page 2
    if page_num == 1 {
        use std::io::Write;
        let mut log = std::fs::File::create("/tmp/page_copy_mapping.txt").unwrap();
        writeln!(log, "Objects count: {}", sorted_objects.len()).unwrap();
        writeln!(log, "Object mapping (old -> new):").unwrap();
        for old_num in &sorted_objects {
            writeln!(log, "  {} -> {}", old_num, obj_mapping[old_num]).unwrap();
        }
    }

    // Now generate the new PDF
    let mut output = Vec::new();

    // PDF header
    output.extend_from_slice(b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n");

    // Track object offsets for xref
    let mut offsets: Vec<usize> = vec![0; next_obj as usize];

    // Object 1: Catalog
    offsets[1] = output.len();
    output.extend_from_slice(b"1 0 obj\n<</Type/Catalog/Pages 2 0 R>>\nendobj\n");

    // Object 2: Pages
    offsets[2] = output.len();
    output.extend_from_slice(b"2 0 obj\n<</Type/Pages/Count 1/Kids[3 0 R]>>\nendobj\n");

    // Object 3: Page with updated references
    offsets[3] = output.len();
    write_page_object(
        &mut output,
        &media_box,
        &content_obj_nums,
        &resources_ref,
        &obj_mapping,
    )?;

    // Copy all dependent objects with updated references
    for old_obj_num in &sorted_objects {
        let new_obj_num = obj_mapping[old_obj_num];
        offsets[new_obj_num as usize] = output.len();
        copy_object_with_remapping(
            &source_data,
            *old_obj_num,
            new_obj_num,
            &obj_mapping,
            &mut output,
        )?;
    }

    // Xref table
    let xref_offset = output.len();
    write_xref_table(&mut output, &offsets)?;

    // Trailer
    output.extend_from_slice(
        format!(
            "trailer\n<</Size {}/Root 1 0 R>>\nstartxref\n{}\n%%EOF\n",
            next_obj, xref_offset
        )
        .as_bytes(),
    );

    fs::write(output_path, &output)?;

    Ok(())
}

/// Copies a single page from source data, preserving all resources
///
/// This variant accepts raw PDF data and allows specifying the starting object number.
/// Used by the merge function to control object numbering.
pub fn copy_page_complete_with_start_obj(
    source_data: &[u8],
    page_num: usize,
    output_path: &str,
    _start_obj_num: i32, // Not used yet - always generates from object 1
) -> Result<()> {
    // For now, delegate to the standard copy function
    // The start_obj_num parameter is reserved for future use when
    // we implement proper multi-document merging with object renumbering

    // Find the page object
    let page_obj_num = find_page_object(source_data, page_num)?;

    // Extract page dictionary boundaries
    let (page_start, page_end) = find_object_boundaries(source_data, page_obj_num)?;
    let page_dict_data = &source_data[page_start..page_end];

    // Get MediaBox
    let media_box = extract_media_box(page_dict_data)?;

    // Extract /Contents reference and get content stream object numbers
    let content_obj_nums = extract_content_references(page_dict_data)?;

    // Extract /Resources reference
    let resources_ref = extract_resources_reference(page_dict_data, source_data)?;

    // Collect all objects we need to copy
    let mut objects_to_copy: HashSet<i32> = HashSet::new();

    // Add content stream objects
    for obj_num in &content_obj_nums {
        objects_to_copy.insert(*obj_num);
    }

    // Add all resources and their dependencies
    if let Some(ref res) = resources_ref {
        collect_resource_objects(source_data, res, &mut objects_to_copy)?;
    }

    // Remove object 0 if present
    objects_to_copy.remove(&0);

    // Build object mapping (starting from 4)
    let mut obj_mapping: HashMap<i32, i32> = HashMap::new();
    let mut next_obj = 4; // 1=Catalog, 2=Pages, 3=Page

    // Sort objects for deterministic output
    let mut sorted_objects: Vec<i32> = objects_to_copy.iter().copied().collect();
    sorted_objects.sort();

    for old_num in &sorted_objects {
        obj_mapping.insert(*old_num, next_obj);
        next_obj += 1;
    }

    // Generate the new PDF
    let mut output = Vec::new();

    // PDF header
    output.extend_from_slice(b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n");

    // Track object offsets for xref
    let mut offsets: Vec<usize> = vec![0; next_obj as usize];

    // Object 1: Catalog
    offsets[1] = output.len();
    output.extend_from_slice(b"1 0 obj\n<</Type/Catalog/Pages 2 0 R>>\nendobj\n");

    // Object 2: Pages
    offsets[2] = output.len();
    output.extend_from_slice(b"2 0 obj\n<</Type/Pages/Count 1/Kids[3 0 R]>>\nendobj\n");

    // Object 3: Page with updated references
    offsets[3] = output.len();
    write_page_object_with_parent(
        &mut output,
        3,
        2,
        &media_box,
        &content_obj_nums,
        &resources_ref,
        &obj_mapping,
    )?;

    // Copy all dependent objects with updated references
    for old_obj_num in &sorted_objects {
        let new_obj_num = obj_mapping[old_obj_num];
        if (new_obj_num as usize) < offsets.len() {
            offsets[new_obj_num as usize] = output.len();
        }
        copy_object_with_remapping(
            source_data,
            *old_obj_num,
            new_obj_num,
            &obj_mapping,
            &mut output,
        )?;
    }

    // Write xref table
    write_xref_table(&mut output, &offsets)?;

    // Write trailer
    output.extend_from_slice(format!("trailer\n<</Size {}/Root 1 0 R>>\n", next_obj).as_bytes());

    // Write startxref
    let xref_offset = output.windows(5).rposition(|w| w == b"xref\n").unwrap_or(0);
    output.extend_from_slice(format!("startxref\n{}\n%%EOF\n", xref_offset).as_bytes());

    fs::write(output_path, &output)?;

    Ok(())
}

/// Write page object with explicit parent reference
fn write_page_object_with_parent(
    output: &mut Vec<u8>,
    obj_num: i32,
    parent_num: i32,
    media_box: &[f32; 4],
    content_obj_nums: &[i32],
    resources_ref: &Option<ResourcesRef>,
    obj_mapping: &HashMap<i32, i32>,
) -> Result<()> {
    output.extend_from_slice(format!("{} 0 obj\n<<", obj_num).as_bytes());
    output.extend_from_slice(b"/Type/Page");
    output.extend_from_slice(format!("/Parent {} 0 R", parent_num).as_bytes());
    output.extend_from_slice(
        format!(
            "/MediaBox[{} {} {} {}]",
            media_box[0], media_box[1], media_box[2], media_box[3]
        )
        .as_bytes(),
    );

    // Add Contents reference
    if !content_obj_nums.is_empty() {
        if content_obj_nums.len() == 1 {
            let new_ref = obj_mapping
                .get(&content_obj_nums[0])
                .unwrap_or(&content_obj_nums[0]);
            output.extend_from_slice(format!("/Contents {} 0 R", new_ref).as_bytes());
        } else {
            output.extend_from_slice(b"/Contents[");
            for (i, obj_num) in content_obj_nums.iter().enumerate() {
                let new_ref = obj_mapping.get(obj_num).unwrap_or(obj_num);
                if i > 0 {
                    output.push(b' ');
                }
                output.extend_from_slice(format!("{} 0 R", new_ref).as_bytes());
            }
            output.push(b']');
        }
    }

    // Add Resources
    match resources_ref {
        Some(ResourcesRef::Direct(data)) => {
            output.extend_from_slice(b"/Resources");
            let data_str = String::from_utf8_lossy(data);
            let remapped = remap_references_in_dict(&data_str, obj_mapping);
            output.extend_from_slice(remapped.as_bytes());
        }
        Some(ResourcesRef::Indirect(old_ref)) => {
            let new_ref = obj_mapping.get(old_ref).unwrap_or(old_ref);
            output.extend_from_slice(format!("/Resources {} 0 R", new_ref).as_bytes());
        }
        None => {
            // Minimal empty resources
            output.extend_from_slice(b"/Resources<<>>");
        }
    }

    output.extend_from_slice(b">>\nendobj\n");
    Ok(())
}

/// Find page object number by page index
fn find_page_object(data: &[u8], page_num: usize) -> Result<i32> {
    // Find /Type /Pages object first
    let pages_pattern = b"/Type/Pages";
    let pages_pattern_alt = b"/Type /Pages";

    let pages_pos = find_pattern(data, pages_pattern)
        .or_else(|| find_pattern(data, pages_pattern_alt))
        .ok_or_else(|| EnhancedError::Generic("Pages object not found".into()))?;

    // Find /Kids array
    let search_region =
        &data[pages_pos.saturating_sub(100)..pages_pos.saturating_add(2000).min(data.len())];

    if let Some(kids_pos) = find_pattern(search_region, b"/Kids") {
        let after_kids = &search_region[kids_pos + 5..];

        // Skip whitespace
        let mut start = 0;
        while start < after_kids.len() && after_kids[start].is_ascii_whitespace() {
            start += 1;
        }

        if start < after_kids.len() && after_kids[start] == b'[' {
            // Parse array to get page object numbers
            let array_start = start + 1;
            if let Some(array_end) = find_byte(&after_kids[array_start..], b']') {
                let array_content = &after_kids[array_start..array_start + array_end];
                let array_str = String::from_utf8_lossy(array_content);

                // Parse page references
                let parts: Vec<&str> = array_str.split_whitespace().collect();
                let mut page_refs = Vec::new();
                let mut i = 0;

                while i + 2 < parts.len() {
                    if parts[i + 2] == "R" || parts[i + 2].starts_with("R") {
                        if let Ok(obj_num) = parts[i].parse::<i32>() {
                            page_refs.push(obj_num);
                        }
                        i += 3;
                    } else {
                        i += 1;
                    }
                }

                if page_num < page_refs.len() {
                    return Ok(page_refs[page_num]);
                }
            }
        }
    }

    // Fallback: search for /Type /Page objects sequentially
    let mut count = 0;
    let page_pattern = b"/Type/Page";
    let page_pattern_alt = b"/Type /Page";

    let mut search_start = 0;
    while search_start < data.len() {
        let region = &data[search_start..];

        let found_pos =
            find_pattern(region, page_pattern).or_else(|| find_pattern(region, page_pattern_alt));

        if let Some(pos) = found_pos {
            let abs_pos = search_start + pos;

            // Verify this is /Type /Page not /Type /Pages
            let check_region = &data[abs_pos..abs_pos.saturating_add(20).min(data.len())];
            if !find_pattern(check_region, b"/Pages").is_some() {
                if count == page_num {
                    // Find object number by searching backwards for "N 0 obj"
                    let obj_num = find_object_num_backwards(data, abs_pos)?;
                    return Ok(obj_num);
                }
                count += 1;
            }

            search_start = abs_pos + 10;
        } else {
            break;
        }
    }

    Err(EnhancedError::InvalidParameter(format!(
        "Page {} not found (document has {} pages)",
        page_num, count
    )))
}

/// Find object number by searching backwards from a position
fn find_object_num_backwards(data: &[u8], pos: usize) -> Result<i32> {
    // Search backwards for "N 0 obj" pattern
    let search_start = pos.saturating_sub(200);
    let region = &data[search_start..pos];

    // Find last occurrence of " 0 obj"
    let obj_marker = b" 0 obj";
    let mut last_pos = None;

    for i in 0..region.len().saturating_sub(obj_marker.len()) {
        if &region[i..i + obj_marker.len()] == obj_marker {
            last_pos = Some(i);
        }
    }

    if let Some(marker_pos) = last_pos {
        // Parse backwards to get the object number
        let before_marker = &region[..marker_pos];
        let before_str = String::from_utf8_lossy(before_marker);

        // Find the last number before the marker
        if let Some(last_word) = before_str.split_whitespace().last() {
            if let Ok(obj_num) = last_word.parse::<i32>() {
                return Ok(obj_num);
            }
        }
    }

    Err(EnhancedError::Generic(
        "Could not find object number".into(),
    ))
}

/// Find object boundaries (start of obj to endobj)
fn find_object_boundaries(data: &[u8], obj_num: i32) -> Result<(usize, usize)> {
    let pattern = format!("{} 0 obj", obj_num);
    let pattern_bytes = pattern.as_bytes();

    let start = find_pattern(data, pattern_bytes)
        .ok_or_else(|| EnhancedError::Generic(format!("Object {} not found", obj_num)))?;

    let after_start = &data[start..];

    // First, find the first "endobj" to limit our search for stream keywords
    // This ensures we don't find a "stream" keyword from a subsequent object
    let first_endobj = find_pattern(after_start, b"endobj")
        .ok_or_else(|| EnhancedError::Generic(format!("Object {} end not found", obj_num)))?;

    // Only check for stream within this object (before the first endobj)
    let object_region = &after_start[..first_endobj];

    // Check if this is a stream object (stream keyword must be within this object)
    if let Some(stream_pos) = find_pattern(object_region, b"stream") {
        // Verify this is actually a stream keyword and not just the word "stream" in some content
        // The pattern should be: dictionary >> stream\n or >>stream\n
        let before_stream = &object_region[..stream_pos];
        if find_pattern(before_stream, b"<<").is_some() {
            // This is a stream object - we need to find endstream and endobj
            // The stream content starts after "stream" + newline
            let dict_str = String::from_utf8_lossy(before_stream);
            let stream_length = extract_stream_length(&dict_str);

            let stream_data_start = stream_pos + 6; // "stream"
            // Skip whitespace after stream keyword (PDF requires newline)
            let mut actual_start = stream_data_start;
            while actual_start < after_start.len() {
                let byte = after_start[actual_start];
                if byte == b'\r' || byte == b'\n' {
                    actual_start += 1;
                } else {
                    break;
                }
            }

            // Find endstream - if we have length, use it; otherwise search
            let endstream_search_start = if let Some(len) = stream_length {
                // Start searching after the expected stream data
                actual_start + len
            } else {
                actual_start
            };

            // Search for endstream
            if endstream_search_start < after_start.len() {
                let search_region = &after_start[endstream_search_start..];
                if let Some(endstream_pos) = find_pattern(search_region, b"endstream") {
                    // Find endobj after endstream
                    let after_endstream = &search_region[endstream_pos..];
                    if let Some(endobj_pos) = find_pattern(after_endstream, b"endobj") {
                        let total_end = endstream_search_start + endstream_pos + endobj_pos + 6;
                        return Ok((start, start + total_end));
                    }
                }
            }
        }
    }

    // Non-stream object: use the first endobj we found
    Ok((start, start + first_endobj + 6)) // +6 for "endobj"
}

/// Extract /Length value from stream dictionary
fn extract_stream_length(dict_str: &str) -> Option<usize> {
    // Look for /Length N pattern
    if let Some(len_pos) = dict_str.find("/Length") {
        let after_len = &dict_str[len_pos + 7..];
        let trimmed = after_len.trim_start();

        // Collect digits
        let num_str: String = trimmed.chars().take_while(|c| c.is_ascii_digit()).collect();

        if !num_str.is_empty() {
            return num_str.parse().ok();
        }
    }
    None
}

/// Extract MediaBox from page dictionary
fn extract_media_box(page_data: &[u8]) -> Result<[f32; 4]> {
    let page_str = String::from_utf8_lossy(page_data);

    // Look for /MediaBox
    if let Some(mb_pos) = page_str.find("/MediaBox") {
        let after_mb = &page_str[mb_pos + 9..];

        // Skip whitespace
        let trimmed = after_mb.trim_start();

        if trimmed.starts_with('[') {
            if let Some(end_bracket) = trimmed.find(']') {
                let box_content = &trimmed[1..end_bracket];
                let parts: Vec<f32> = box_content
                    .split_whitespace()
                    .filter_map(|s| s.parse().ok())
                    .collect();

                if parts.len() >= 4 {
                    return Ok([parts[0], parts[1], parts[2], parts[3]]);
                }
            }
        }
    }

    // Default to US Letter
    Ok([0.0, 0.0, 612.0, 792.0])
}

/// Extract content stream object references
fn extract_content_references(page_data: &[u8]) -> Result<Vec<i32>> {
    let page_str = String::from_utf8_lossy(page_data);
    let mut refs = Vec::new();

    if let Some(contents_pos) = page_str.find("/Contents") {
        let after_contents = &page_str[contents_pos + 9..];
        let trimmed = after_contents.trim_start();

        if trimmed.starts_with('[') {
            // Array of references
            if let Some(end_bracket) = trimmed.find(']') {
                let array_content = &trimmed[1..end_bracket];
                let parts: Vec<&str> = array_content.split_whitespace().collect();

                let mut i = 0;
                while i + 2 < parts.len() {
                    if parts[i + 2] == "R" || parts[i + 2].starts_with("R") {
                        if let Ok(obj_num) = parts[i].parse::<i32>() {
                            refs.push(obj_num);
                        }
                        i += 3;
                    } else {
                        i += 1;
                    }
                }
            }
        } else {
            // Single reference
            let parts: Vec<&str> = trimmed.split_whitespace().take(3).collect();
            if parts.len() >= 3 && (parts[2] == "R" || parts[2].starts_with("R")) {
                if let Ok(obj_num) = parts[0].parse::<i32>() {
                    refs.push(obj_num);
                }
            }
        }
    }

    Ok(refs)
}

/// Extracted resources info
#[derive(Debug, Clone)]
enum ResourcesRef {
    Direct(Vec<u8>),
    Indirect(i32),
}

/// Extract resources reference from page
fn extract_resources_reference(page_data: &[u8], full_pdf: &[u8]) -> Result<Option<ResourcesRef>> {
    let page_str = String::from_utf8_lossy(page_data);

    if let Some(resources_pos) = page_str.find("/Resources") {
        let after_resources = &page_str[resources_pos + 10..];
        let trimmed = after_resources.trim_start();

        if trimmed.starts_with("<<") {
            // Direct dictionary - extract the whole thing
            let mut depth = 0;
            let mut end = 0;
            let chars: Vec<char> = trimmed.chars().collect();

            for i in 0..chars.len().saturating_sub(1) {
                if chars[i] == '<' && chars.get(i + 1) == Some(&'<') {
                    depth += 1;
                } else if chars[i] == '>' && chars.get(i + 1) == Some(&'>') {
                    depth -= 1;
                    if depth == 0 {
                        end = i + 2;
                        break;
                    }
                }
            }

            if end > 0 {
                let dict_str: String = chars[..end].iter().collect();
                return Ok(Some(ResourcesRef::Direct(dict_str.into_bytes())));
            }
        } else {
            // Indirect reference
            let parts: Vec<&str> = trimmed.split_whitespace().take(3).collect();
            if parts.len() >= 3 && (parts[2] == "R" || parts[2].starts_with("R")) {
                if let Ok(obj_num) = parts[0].parse::<i32>() {
                    return Ok(Some(ResourcesRef::Indirect(obj_num)));
                }
            }
        }
    }

    Ok(None)
}

/// Collect all objects referenced by resources
fn collect_resource_objects(
    pdf_data: &[u8],
    resources: &ResourcesRef,
    objects: &mut HashSet<i32>,
) -> Result<()> {
    let resources_data = match resources {
        ResourcesRef::Direct(data) => data.clone(),
        ResourcesRef::Indirect(obj_num) => {
            objects.insert(*obj_num);
            // Get the object data
            let (start, end) = find_object_boundaries(pdf_data, *obj_num)?;
            pdf_data[start..end].to_vec()
        }
    };

    let resources_str = String::from_utf8_lossy(&resources_data);

    // Extract all indirect references from resources
    collect_references_from_dict(&resources_str, pdf_data, objects)?;

    Ok(())
}

/// Recursively collect all references from a dictionary
/// Handles compact PDF syntax like "5 0 R/NextEntry" without whitespace
/// Skips /Parent references to avoid collecting the entire page tree
fn collect_references_from_dict(
    dict_str: &str,
    pdf_data: &[u8],
    objects: &mut HashSet<i32>,
) -> Result<()> {
    // Use character-based scanning to find "N 0 R" patterns
    // This handles compact syntax like "5 0 R/Font" (no space before /)
    let chars: Vec<char> = dict_str.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        // Look for digit sequences that could be object numbers
        if chars[i].is_ascii_digit() {
            let num_start = i;

            // Check if this reference is preceded by /Parent - if so, skip it
            // Look back up to 10 characters for "/Parent"
            let before_start = num_start.saturating_sub(10);
            let before: String = chars[before_start..num_start].iter().collect();
            let is_parent_ref = before.contains("/Parent");

            // Collect the number
            while i < chars.len() && chars[i].is_ascii_digit() {
                i += 1;
            }

            let num_str: String = chars[num_start..i].iter().collect();
            let after_number = i; // Save position after collecting digits

            // Try to match " 0 R" pattern
            let mut check_i = i;

            // Skip whitespace
            while check_i < chars.len() && chars[check_i].is_ascii_whitespace() {
                check_i += 1;
            }

            // Check for "0"
            if check_i < chars.len() && chars[check_i] == '0' {
                check_i += 1;

                // Skip whitespace
                while check_i < chars.len() && chars[check_i].is_ascii_whitespace() {
                    check_i += 1;
                }

                // Check for "R" (can be followed by / > ] or whitespace)
                if check_i < chars.len() && chars[check_i] == 'R' {
                    let next_char = chars.get(check_i + 1);
                    let is_valid_ref = next_char.is_none()
                        || next_char == Some(&'/')
                        || next_char == Some(&'>')
                        || next_char == Some(&']')
                        || next_char == Some(&' ')
                        || next_char == Some(&'\n')
                        || next_char == Some(&'\r');

                    if is_valid_ref && !is_parent_ref {
                        // Valid reference found - update i to after 'R'
                        i = check_i;
                        if let Ok(obj_num) = num_str.parse::<i32>() {
                            if !objects.contains(&obj_num) {
                                // Check if this object is a Page or Pages object - skip those
                                match find_object_boundaries(pdf_data, obj_num) {
                                    Ok((start, end)) => {
                                        let obj_data = &pdf_data[start..end];
                                        let obj_str = String::from_utf8_lossy(obj_data);

                                        // Skip Page, Pages, and Catalog objects
                                        // Be more specific to avoid matching /Subtype/Type0
                                        let is_structural = obj_str.contains("/Type/Page>")
                                            || obj_str.contains("/Type/Page/")
                                            || obj_str.contains("/Type /Page>")
                                            || obj_str.contains("/Type /Page/")
                                            || obj_str.contains("/Type/Pages>")
                                            || obj_str.contains("/Type/Pages/")
                                            || obj_str.contains("/Type /Pages>")
                                            || obj_str.contains("/Type /Pages/")
                                            || obj_str.contains("/Type/Catalog>")
                                            || obj_str.contains("/Type/Catalog/")
                                            || obj_str.contains("/Type /Catalog>")
                                            || obj_str.contains("/Type /Catalog/");

                                        if !is_structural {
                                            objects.insert(obj_num);
                                            // Recursively collect references from this object
                                            collect_references_from_dict(
                                                &obj_str, pdf_data, objects,
                                            )?;
                                        }
                                    }
                                    Err(_) => {
                                        // Object not found - skip it
                                    }
                                }
                            }
                        }
                        i += 1; // Skip 'R'
                        continue;
                    }
                }
            }
            // Reference check failed - restore i to just after the digits
            // so we don't skip over other potential references
            i = after_number;
        }
        i += 1;
    }

    Ok(())
}

/// Write the page object with updated references
fn write_page_object(
    output: &mut Vec<u8>,
    media_box: &[f32; 4],
    content_refs: &[i32],
    resources: &Option<ResourcesRef>,
    mapping: &HashMap<i32, i32>,
) -> Result<()> {
    output.extend_from_slice(b"3 0 obj\n<<");
    output.extend_from_slice(b"/Type/Page");
    output.extend_from_slice(b"/Parent 2 0 R");

    // MediaBox
    output.extend_from_slice(
        format!(
            "/MediaBox[{} {} {} {}]",
            media_box[0], media_box[1], media_box[2], media_box[3]
        )
        .as_bytes(),
    );

    // Contents
    if content_refs.len() == 1 {
        let new_ref = mapping.get(&content_refs[0]).unwrap_or(&content_refs[0]);
        output.extend_from_slice(format!("/Contents {} 0 R", new_ref).as_bytes());
    } else if content_refs.len() > 1 {
        output.extend_from_slice(b"/Contents[");
        for (i, old_ref) in content_refs.iter().enumerate() {
            let new_ref = mapping.get(old_ref).unwrap_or(old_ref);
            if i > 0 {
                output.push(b' ');
            }
            output.extend_from_slice(format!("{} 0 R", new_ref).as_bytes());
        }
        output.push(b']');
    }

    // Resources
    match resources {
        Some(ResourcesRef::Direct(data)) => {
            output.extend_from_slice(b"/Resources");
            // Rewrite references in the direct dictionary
            let data_str = String::from_utf8_lossy(data);
            let remapped = remap_references_in_dict(&data_str, mapping);
            output.extend_from_slice(remapped.as_bytes());
        }
        Some(ResourcesRef::Indirect(old_ref)) => {
            let new_ref = mapping.get(old_ref).unwrap_or(old_ref);
            output.extend_from_slice(format!("/Resources {} 0 R", new_ref).as_bytes());
        }
        None => {
            // Minimal resources
            output.extend_from_slice(b"/Resources<</ProcSet[/PDF/Text]>>");
        }
    }

    output.extend_from_slice(b">>\nendobj\n");

    Ok(())
}

/// Remap object references in a dictionary string
/// Handles compact syntax like "5 0 R/NextEntry"
fn remap_references_in_dict(dict: &str, mapping: &HashMap<i32, i32>) -> String {
    let mut result = String::with_capacity(dict.len() + 100);
    let chars: Vec<char> = dict.chars().collect();
    let mut last_end = 0;
    let mut j = 0;

    while j < chars.len() {
        // Look for digit sequences
        if chars[j].is_ascii_digit() {
            let num_start = j;

            // Collect the number
            while j < chars.len() && chars[j].is_ascii_digit() {
                j += 1;
            }

            // Skip optional whitespace
            let mut k = j;
            while k < chars.len() && chars[k].is_ascii_whitespace() {
                k += 1;
            }

            // Check for "0"
            if k < chars.len() && chars[k] == '0' {
                k += 1;

                // Skip optional whitespace
                while k < chars.len() && chars[k].is_ascii_whitespace() {
                    k += 1;
                }

                // Check for "R" followed by valid terminator
                if k < chars.len() && chars[k] == 'R' {
                    let next_char = chars.get(k + 1);
                    let is_valid_ref = next_char.is_none()
                        || next_char == Some(&'/')
                        || next_char == Some(&'>')
                        || next_char == Some(&']')
                        || next_char == Some(&' ')
                        || next_char == Some(&'\n')
                        || next_char == Some(&'\r');

                    if is_valid_ref {
                        let num_str: String = chars[num_start..j].iter().collect();
                        if let Ok(old_num) = num_str.parse::<i32>() {
                            let new_num = mapping.get(&old_num).unwrap_or(&old_num);
                            // Add everything before this number
                            let prefix: String = chars[last_end..num_start].iter().collect();
                            result.push_str(&prefix);
                            result.push_str(&new_num.to_string());
                            // Add " 0 R" (normalized with spaces)
                            result.push_str(" 0 R");
                            last_end = k + 1; // Skip past the 'R'
                            j = k + 1;
                            continue;
                        }
                    }
                }
            }
        }
        j += 1;
    }

    // Add remaining content
    let suffix: String = chars[last_end..].iter().collect();
    result.push_str(&suffix);

    result
}

/// Copy an object with remapped references
fn copy_object_with_remapping(
    source_data: &[u8],
    old_obj_num: i32,
    new_obj_num: i32,
    mapping: &HashMap<i32, i32>,
    output: &mut Vec<u8>,
) -> Result<()> {
    let (start, end) = find_object_boundaries(source_data, old_obj_num)?;
    let obj_data = &source_data[start..end];

    // Check if this is a stream object (must be "stream" followed by newline, not just any "stream" text)
    let is_stream = find_stream_keyword(obj_data).is_some();

    // Start new object
    output.extend_from_slice(format!("{} 0 obj\n", new_obj_num).as_bytes());

    if is_stream {
        // For streams, we need to preserve the binary data
        let stream_keyword_pos = find_stream_keyword(obj_data)
            .ok_or_else(|| EnhancedError::Generic("Stream keyword not found".into()))?;

        // Get dictionary part (before stream)
        let dict_part = &obj_data[..stream_keyword_pos];
        let dict_str = String::from_utf8_lossy(dict_part);

        // Skip "N 0 obj" prefix - find the dictionary start
        let dict_content_start = dict_str.find("<<").unwrap_or(0);
        let dict_content = &dict_str[dict_content_start..];

        // Remap references in dictionary
        let remapped_dict = remap_references_in_dict(dict_content, mapping);
        output.extend_from_slice(remapped_dict.as_bytes());

        // Copy stream keyword and data as-is (binary safe)
        let stream_data = &obj_data[stream_keyword_pos..];

        // Find endstream - copy up to and including it
        if let Some(endstream_pos) = find_pattern(stream_data, b"endstream") {
            output.extend_from_slice(&stream_data[..endstream_pos + 9]); // Include "endstream"
            output.extend_from_slice(b"\nendobj\n");
        } else {
            // No endstream found - write stream data and close properly
            output.extend_from_slice(stream_data);
            output.extend_from_slice(b"\nendstream\nendobj\n");
        }
    } else {
        // Non-stream object - remap all references
        let obj_str = String::from_utf8_lossy(obj_data);

        // Skip "N 0 obj" prefix - find the actual content start
        let content_start = obj_str
            .find("<<")
            .or_else(|| obj_str.find('['))
            .or_else(|| obj_str.find('('))
            .unwrap_or(0);

        // Find content end - exclude "endobj" and any trailing whitespace
        let content_end = if let Some(pos) = obj_str.rfind("endobj") {
            // Trim trailing whitespace before endobj
            let mut end = pos;
            while end > content_start && obj_str.as_bytes()[end - 1].is_ascii_whitespace() {
                end -= 1;
            }
            end
        } else {
            obj_str.len()
        };

        let content = &obj_str[content_start..content_end];
        let remapped = remap_references_in_dict(content, mapping);
        output.extend_from_slice(remapped.trim_end().as_bytes());
        output.extend_from_slice(b"\nendobj\n");
    }

    Ok(())
}

/// Find the "stream" keyword that starts actual stream data
/// Returns the position of "stream" only if it's a proper stream marker (followed by newline)
fn find_stream_keyword(data: &[u8]) -> Option<usize> {
    let mut pos = 0;
    while pos < data.len() {
        if let Some(found) = find_pattern(&data[pos..], b"stream") {
            let abs_pos = pos + found;
            let after_stream = abs_pos + 6;

            // Check if this is followed by \r, \n, or \r\n (required by PDF spec)
            if after_stream < data.len() {
                let next_byte = data[after_stream];
                if next_byte == b'\r' || next_byte == b'\n' {
                    // Also verify there's a << before this (stream dictionary)
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

/// Write xref table
fn write_xref_table(output: &mut Vec<u8>, offsets: &[usize]) -> Result<()> {
    output.extend_from_slice(format!("xref\n0 {}\n", offsets.len()).as_bytes());

    // Object 0 is free
    output.extend_from_slice(b"0000000000 65535 f \n");

    // Other objects
    for offset in offsets.iter().skip(1) {
        output.extend_from_slice(format!("{:010} 00000 n \n", offset).as_bytes());
    }

    Ok(())
}

/// Find byte pattern in data
fn find_pattern(data: &[u8], pattern: &[u8]) -> Option<usize> {
    if pattern.is_empty() || pattern.len() > data.len() {
        return None;
    }
    for i in 0..=data.len() - pattern.len() {
        if &data[i..i + pattern.len()] == pattern {
            return Some(i);
        }
    }
    None
}

/// Find single byte in data
fn find_byte(data: &[u8], byte: u8) -> Option<usize> {
    data.iter().position(|&b| b == byte)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remap_references() {
        let mut mapping = HashMap::new();
        mapping.insert(5, 10);
        mapping.insert(6, 11);

        let input = "<</Font 5 0 R/XObject 6 0 R>>";
        let result = remap_references_in_dict(input, &mapping);

        assert!(result.contains("10 0 R"));
        assert!(result.contains("11 0 R"));
    }

    #[test]
    fn test_extract_media_box() {
        let data = b"/Type/Page/MediaBox[0 0 612 792]/Contents 4 0 R";
        let result = extract_media_box(data).unwrap();
        assert_eq!(result, [0.0, 0.0, 612.0, 792.0]);
    }
}
