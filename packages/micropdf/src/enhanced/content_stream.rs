//! Content Stream Extraction and Manipulation
//!
//! This module provides functionality to extract, parse, and manipulate
//! PDF content streams for page content copying and merging.

use super::error::{EnhancedError, Result};
use std::collections::HashMap;

/// Extracted page content including content streams and resources
#[derive(Debug, Clone)]
pub struct PageContent {
    /// Content stream data (may be multiple streams)
    pub content_streams: Vec<Vec<u8>>,
    /// Resources dictionary data
    pub resources: Option<Vec<u8>>,
    /// Font references used in this page
    pub fonts: HashMap<String, i32>,
    /// XObject references (images, forms)
    pub xobjects: HashMap<String, i32>,
    /// Graphics state references
    pub graphics_states: HashMap<String, i32>,
    /// Other resource object references
    pub other_resources: HashMap<String, i32>,
}

impl PageContent {
    /// Create new empty page content
    pub fn new() -> Self {
        Self {
            content_streams: Vec::new(),
            resources: None,
            fonts: HashMap::new(),
            xobjects: HashMap::new(),
            graphics_states: HashMap::new(),
            other_resources: HashMap::new(),
        }
    }

    /// Check if page has content
    pub fn has_content(&self) -> bool {
        !self.content_streams.is_empty()
    }

    /// Get combined content stream
    pub fn combined_content(&self) -> Vec<u8> {
        if self.content_streams.is_empty() {
            return Vec::new();
        }

        if self.content_streams.len() == 1 {
            return self.content_streams[0].clone();
        }

        // Combine multiple content streams
        let mut combined = Vec::new();
        for (i, stream) in self.content_streams.iter().enumerate() {
            if i > 0 {
                combined.push(b'\n');
            }
            combined.extend_from_slice(stream);
        }
        combined
    }
}

impl Default for PageContent {
    fn default() -> Self {
        Self::new()
    }
}

/// Extract content stream from a PDF page object using byte-level operations
pub fn extract_content_stream(pdf_data: &[u8], page_obj_num: i32) -> Result<Vec<u8>> {
    // Find the page object
    let obj_pattern = format!("{} 0 obj", page_obj_num);
    let obj_pattern_bytes = obj_pattern.as_bytes();

    let obj_pos = find_bytes_pattern(pdf_data, obj_pattern_bytes)
        .ok_or_else(|| EnhancedError::Generic(format!("Page object {} not found", page_obj_num)))?;

    // Find endobj to limit our search
    let after_obj = &pdf_data[obj_pos..];
    let endobj_pos = find_bytes_pattern(after_obj, b"endobj")
        .ok_or_else(|| EnhancedError::Generic("Page object end not found".into()))?;
    let page_section = &after_obj[..endobj_pos];

    // Look for /Contents entry
    let contents_pos = find_bytes_pattern(page_section, b"/Contents")
        .ok_or_else(|| EnhancedError::Generic("No /Contents in page object".into()))?;

    let after_contents = &page_section[contents_pos + 9..];

    // Skip whitespace
    let mut start = 0;
    while start < after_contents.len() && after_contents[start].is_ascii_whitespace() {
        start += 1;
    }

    if start >= after_contents.len() {
        return Err(EnhancedError::Generic("No content after /Contents".into()));
    }

    // Convert to string only for parsing the reference/array portion (ASCII-safe)
    let after_contents_str = String::from_utf8_lossy(&after_contents[start..]);

    // Contents can be:
    // 1. Direct array: /Contents [obj1 obj2 ...]
    // 2. Reference: /Contents 123 0 R
    // 3. Direct stream (rare): /Contents << ... >> stream...

    if after_contents_str.starts_with('[') {
        // Array of content stream references
        extract_content_stream_array(pdf_data, &after_contents_str)
    } else {
        // Single reference
        extract_single_content_stream_ref(pdf_data, &after_contents_str)
    }
}

/// Extract content stream from array reference
fn extract_content_stream_array(pdf_data: &[u8], after_contents: &str) -> Result<Vec<u8>> {
    let bracket_start = after_contents
        .find('[')
        .ok_or_else(|| EnhancedError::Generic("Array start not found".into()))?;
    let bracket_end = after_contents[bracket_start + 1..]
        .find(']')
        .ok_or_else(|| EnhancedError::Generic("Array end not found".into()))?;

    let array_content = &after_contents[bracket_start + 1..bracket_start + 1 + bracket_end];
    let parts: Vec<&str> = array_content.split_whitespace().collect();

    let mut combined = Vec::new();
    let mut i = 0;

    while i < parts.len() {
        // Check for object reference: num gen R (parts[i+2] might be "R" or "R/" or "R>")
        if i + 2 < parts.len()
            && (parts[i + 2] == "R"
                || parts[i + 2].starts_with("R/")
                || parts[i + 2].starts_with("R>"))
        {
            if let Ok(obj_num) = parts[i].parse::<i32>() {
                match extract_stream_object(pdf_data, obj_num) {
                    Ok(stream_data) => {
                        if !combined.is_empty() {
                            combined.push(b'\n');
                        }
                        combined.extend_from_slice(&stream_data);
                    }
                    Err(_) => {
                        // Skip objects that don't have stream data (might be metadata)
                    }
                }
            }
            i += 3;
        } else {
            i += 1;
        }
    }

    Ok(combined)
}

/// Extract content stream from single reference
fn extract_single_content_stream_ref(pdf_data: &[u8], after_contents: &str) -> Result<Vec<u8>> {
    let parts: Vec<&str> = after_contents.split_whitespace().take(3).collect();

    // Check if this looks like an indirect reference: "num gen R"
    // parts[2] might be "R" or "R/something" (no space before next entry)
    if parts.len() >= 3
        && (parts[2] == "R" || parts[2].starts_with("R/") || parts[2].starts_with("R>"))
    {
        if let Ok(obj_num) = parts[0].parse::<i32>() {
            return extract_stream_object(pdf_data, obj_num);
        }
    }

    Err(EnhancedError::Generic(
        "Could not parse content stream reference".into(),
    ))
}

/// Extract stream object data using byte-level operations
/// Handles FlateDecode decompression automatically
pub fn extract_stream_object(pdf_data: &[u8], obj_num: i32) -> Result<Vec<u8>> {
    use flate2::read::ZlibDecoder;
    use std::io::Read;

    // Build pattern for "N 0 obj" where N is object number
    let obj_pattern = format!("{} 0 obj", obj_num);
    let obj_pattern_bytes = obj_pattern.as_bytes();

    // Find the object in raw bytes
    let obj_pos = find_bytes_pattern(pdf_data, obj_pattern_bytes)
        .ok_or_else(|| EnhancedError::Generic(format!("Stream object {} not found", obj_num)))?;

    // Find "stream" keyword after object
    let after_obj = &pdf_data[obj_pos..];
    let stream_pos = find_bytes_pattern(after_obj, b"stream")
        .ok_or_else(|| EnhancedError::Generic("Stream keyword not found".into()))?;

    // Check if stream has FlateDecode filter (check object dictionary before "stream")
    let obj_dict = &after_obj[..stream_pos];
    let is_flate_decode = find_bytes_pattern(obj_dict, b"/FlateDecode").is_some()
        || find_bytes_pattern(obj_dict, b"/Fl").is_some();

    // Calculate absolute position
    let stream_keyword_pos = obj_pos + stream_pos;

    // Skip "stream" (6 bytes) and newline
    let after_stream_keyword = stream_keyword_pos + 6;

    if after_stream_keyword >= pdf_data.len() {
        return Err(EnhancedError::Generic("Stream data truncated".into()));
    }

    let stream_data_start = if pdf_data[after_stream_keyword] == b'\r'
        && pdf_data.get(after_stream_keyword + 1) == Some(&b'\n')
    {
        after_stream_keyword + 2
    } else if pdf_data[after_stream_keyword] == b'\n' || pdf_data[after_stream_keyword] == b'\r' {
        after_stream_keyword + 1
    } else {
        after_stream_keyword
    };

    if stream_data_start >= pdf_data.len() {
        return Err(EnhancedError::Generic(
            "Stream data start out of bounds".into(),
        ));
    }

    // Find "endstream" keyword
    let after_stream = &pdf_data[stream_data_start..];
    let endstream_pos = find_bytes_pattern(after_stream, b"endstream")
        .ok_or_else(|| EnhancedError::Generic("endstream not found".into()))?;

    let stream_data_end = stream_data_start + endstream_pos;

    // Handle trailing newlines before endstream
    let mut actual_end = stream_data_end;
    while actual_end > stream_data_start
        && (pdf_data[actual_end - 1] == b'\n' || pdf_data[actual_end - 1] == b'\r')
    {
        actual_end -= 1;
    }

    let raw_data = &pdf_data[stream_data_start..actual_end];

    // Decompress if FlateDecode
    if is_flate_decode && !raw_data.is_empty() {
        let mut decoder = ZlibDecoder::new(raw_data);
        let mut decompressed = Vec::new();
        match decoder.read_to_end(&mut decompressed) {
            Ok(_) => Ok(decompressed),
            Err(_) => {
                // If decompression fails, return raw data (might not be compressed)
                Ok(raw_data.to_vec())
            }
        }
    } else {
        Ok(raw_data.to_vec())
    }
}

/// Find a byte pattern in data
fn find_bytes_pattern(data: &[u8], pattern: &[u8]) -> Option<usize> {
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

/// Extract resources dictionary from page object using byte-level operations
pub fn extract_resources(pdf_data: &[u8], page_obj_num: i32) -> Result<Option<Vec<u8>>> {
    // Find the page object
    let obj_pattern = format!("{} 0 obj", page_obj_num);
    let obj_pattern_bytes = obj_pattern.as_bytes();

    let obj_pos = find_bytes_pattern(pdf_data, obj_pattern_bytes)
        .ok_or_else(|| EnhancedError::Generic(format!("Page object {} not found", page_obj_num)))?;

    let after_obj = &pdf_data[obj_pos..];
    let endobj_pos = find_bytes_pattern(after_obj, b"endobj")
        .ok_or_else(|| EnhancedError::Generic("Page object end not found".into()))?;
    let page_section = &after_obj[..endobj_pos];

    // Look for /Resources entry
    if let Some(resources_pos) = find_bytes_pattern(page_section, b"/Resources") {
        let after_resources = &page_section[resources_pos + 10..];

        // Skip whitespace
        let mut start = 0;
        while start < after_resources.len() && after_resources[start].is_ascii_whitespace() {
            start += 1;
        }

        if start >= after_resources.len() {
            return Ok(None);
        }

        // Convert to string for parsing (the part after /Resources should be ASCII)
        let after_resources_str = String::from_utf8_lossy(&after_resources[start..]);

        // Resources can be:
        // 1. Direct dictionary: /Resources << ... >>
        // 2. Reference: /Resources 123 0 R

        if after_resources_str.starts_with("<<") {
            // Direct dictionary - extract it
            extract_dictionary_data(&after_resources_str)
        } else {
            // Reference - need to follow it
            let parts: Vec<&str> = after_resources_str.split_whitespace().take(3).collect();
            if parts.len() >= 3 && parts[2] == "R" {
                if let Ok(obj_num) = parts[0].parse::<i32>() {
                    return extract_resource_object(pdf_data, obj_num);
                }
            }
            Ok(None)
        }
    } else {
        Ok(None)
    }
}

/// Extract dictionary data as bytes
fn extract_dictionary_data(text: &str) -> Result<Option<Vec<u8>>> {
    let dict_start = text
        .find("<<")
        .ok_or_else(|| EnhancedError::Generic("Dictionary start not found".into()))?;

    // Count nested dictionaries
    let mut depth = 0;
    let mut dict_end = dict_start + 2;
    let chars: Vec<char> = text.chars().collect();

    for i in dict_start..chars.len() - 1 {
        if chars[i] == '<' && chars.get(i + 1) == Some(&'<') {
            depth += 1;
        } else if chars[i] == '>' && chars.get(i + 1) == Some(&'>') {
            depth -= 1;
            if depth == 0 {
                dict_end = i + 2;
                break;
            }
        }
    }

    if depth != 0 {
        return Err(EnhancedError::Generic(
            "Unbalanced dictionary brackets".into(),
        ));
    }

    let dict_text = &text[dict_start..dict_end];
    Ok(Some(dict_text.as_bytes().to_vec()))
}

/// Extract resource object (indirect reference) using byte-level operations
fn extract_resource_object(pdf_data: &[u8], obj_num: i32) -> Result<Option<Vec<u8>>> {
    let obj_pattern = format!("{} 0 obj", obj_num);
    let obj_pattern_bytes = obj_pattern.as_bytes();

    let obj_pos = find_bytes_pattern(pdf_data, obj_pattern_bytes)
        .ok_or_else(|| EnhancedError::Generic(format!("Resource object {} not found", obj_num)))?;

    // Get the section after the object definition
    let after_obj_start = obj_pos + obj_pattern_bytes.len();
    if after_obj_start >= pdf_data.len() {
        return Ok(None);
    }

    // Limit search to a reasonable section (up to next endobj or 10KB)
    let search_end = (after_obj_start + 10240).min(pdf_data.len());
    let after_obj = &pdf_data[after_obj_start..search_end];

    // Convert to string only for dictionary extraction (should be ASCII structure)
    let after_obj_str = String::from_utf8_lossy(after_obj);

    // Extract dictionary
    extract_dictionary_data(&after_obj_str)
}

/// Parse resource dictionary to extract references
fn parse_resource_references(page_content: &mut PageContent, resources_data: &[u8]) -> Result<()> {
    let resources_str = String::from_utf8_lossy(resources_data);

    // Extract Font references
    if let Some(font_pos) = resources_str.find("/Font") {
        let after_font = &resources_str[font_pos..];
        if let Some(dict_start) = after_font.find("<<") {
            if let Some(dict_end) = after_font[dict_start..].find(">>") {
                let font_dict = &after_font[dict_start..dict_start + dict_end];
                // Parse font references like /F1 123 0 R
                for line in font_dict.lines() {
                    if let Some(slash_pos) = line.find('/') {
                        let after_slash = &line[slash_pos + 1..];
                        let parts: Vec<&str> = after_slash.split_whitespace().collect();
                        if parts.len() >= 3 && parts[2] == "R" {
                            if let Ok(obj_num) = parts[1].parse::<i32>() {
                                page_content.fonts.insert(parts[0].to_string(), obj_num);
                            }
                        }
                    }
                }
            }
        }
    }

    // Extract XObject references (images, forms)
    if let Some(xobject_pos) = resources_str.find("/XObject") {
        let after_xobject = &resources_str[xobject_pos..];
        if let Some(dict_start) = after_xobject.find("<<") {
            if let Some(dict_end) = after_xobject[dict_start..].find(">>") {
                let xobject_dict = &after_xobject[dict_start..dict_start + dict_end];
                for line in xobject_dict.lines() {
                    if let Some(slash_pos) = line.find('/') {
                        let after_slash = &line[slash_pos + 1..];
                        let parts: Vec<&str> = after_slash.split_whitespace().collect();
                        if parts.len() >= 3 && parts[2] == "R" {
                            if let Ok(obj_num) = parts[1].parse::<i32>() {
                                page_content.xobjects.insert(parts[0].to_string(), obj_num);
                            }
                        }
                    }
                }
            }
        }
    }

    // Extract ExtGState (graphics state) references
    if let Some(extgstate_pos) = resources_str.find("/ExtGState") {
        let after_extgstate = &resources_str[extgstate_pos..];
        if let Some(dict_start) = after_extgstate.find("<<") {
            if let Some(dict_end) = after_extgstate[dict_start..].find(">>") {
                let extgstate_dict = &after_extgstate[dict_start..dict_start + dict_end];
                for line in extgstate_dict.lines() {
                    if let Some(slash_pos) = line.find('/') {
                        let after_slash = &line[slash_pos + 1..];
                        let parts: Vec<&str> = after_slash.split_whitespace().collect();
                        if parts.len() >= 3 && parts[2] == "R" {
                            if let Ok(obj_num) = parts[1].parse::<i32>() {
                                page_content
                                    .graphics_states
                                    .insert(parts[0].to_string(), obj_num);
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

/// Extract complete page content including resources
pub fn extract_page_content(pdf_data: &[u8], page_obj_num: i32) -> Result<PageContent> {
    let mut page_content = PageContent::new();

    // Extract content stream
    match extract_content_stream(pdf_data, page_obj_num) {
        Ok(content) => page_content.content_streams.push(content),
        Err(_) => {
            // No content stream - blank page
        }
    }

    // Extract resources
    page_content.resources = extract_resources(pdf_data, page_obj_num)?;

    // Parse resources to extract references
    if let Some(resources_data) = page_content.resources.clone() {
        parse_resource_references(&mut page_content, &resources_data)?;
    }

    Ok(page_content)
}
