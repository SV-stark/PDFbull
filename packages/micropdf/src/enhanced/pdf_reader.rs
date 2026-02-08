//! PDF Reader - Parse and read PDF files
//!
//! This module provides functionality to parse PDF files and extract their structure.

use super::error::{EnhancedError, Result};
use crate::pdf::object::{Dict, Name, Object};
use crate::pdf::xref::{XrefEntry, XrefTable};
use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};
use std::path::Path;

/// PDF document structure
pub struct PdfDocument {
    /// PDF file data
    data: Vec<u8>,
    /// Cross-reference table
    xref: XrefTable,
    /// Root object (Catalog)
    root: Option<i32>,
    /// Pages object
    pages: Option<i32>,
    /// Page objects (ordered list)
    page_objects: Vec<i32>,
}

impl PdfDocument {
    /// Open and parse a PDF file
    pub fn open(path: &str) -> Result<Self> {
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

        let mut doc = Self {
            data,
            xref: XrefTable::new(),
            root: None,
            pages: None,
            page_objects: Vec::new(),
        };

        doc.parse()?;
        Ok(doc)
    }

    /// Parse the PDF structure
    fn parse(&mut self) -> Result<()> {
        // Find xref table
        self.parse_xref()?;

        // Find root (Catalog)
        self.find_root()?;

        // Find pages
        self.find_pages()?;

        Ok(())
    }

    /// Parse xref table
    fn parse_xref(&mut self) -> Result<()> {
        // Look for "xref" keyword
        let content = String::from_utf8_lossy(&self.data);
        let xref_pos = content
            .rfind("xref")
            .ok_or_else(|| EnhancedError::InvalidParameter("No xref table found".into()))?;

        // Parse xref entries
        // Simplified: look for object numbers and offsets
        // Full implementation would parse the xref table properly
        let lines: Vec<&str> = content[xref_pos..].lines().collect();
        let mut i = 1; // Skip "xref" line

        while i < lines.len() {
            let line = lines[i].trim();
            if line.is_empty() || line == "trailer" {
                break;
            }

            // Parse xref subsection: "start count"
            if let Some(space) = line.find(' ') {
                let start_str = &line[..space];
                let count_str = line[space..].trim();

                if let (Ok(start), Ok(count)) =
                    (start_str.parse::<i32>(), count_str.parse::<usize>())
                {
                    i += 1;
                    for j in 0..count {
                        if i + j < lines.len() {
                            let entry_line = lines[i + j].trim();
                            // Format: "offset generation n" or "offset generation f"
                            let parts: Vec<&str> = entry_line.split_whitespace().collect();
                            if parts.len() >= 3 {
                                if let (Ok(offset), Ok(generation)) =
                                    (parts[0].parse::<i64>(), parts[1].parse::<u16>())
                                {
                                    let entry_type = if parts[2] == "f" {
                                        XrefEntry::free(start + j as i32, generation)
                                    } else {
                                        XrefEntry::in_use(start + j as i32, generation, offset)
                                    };
                                    self.xref.add_entry(entry_type);
                                }
                            }
                        }
                    }
                    i += count;
                } else {
                    i += 1;
                }
            } else {
                i += 1;
            }
        }

        Ok(())
    }

    /// Find root (Catalog) object
    fn find_root(&mut self) -> Result<()> {
        let content = String::from_utf8_lossy(&self.data);

        // Method 1: Look for /Root in trailer (traditional PDF)
        if let Some(trailer_pos) = content.rfind("trailer") {
            let trailer_section = &content[trailer_pos..];
            if let Some(root_num) = Self::extract_root_from_section(&trailer_section) {
                self.root = Some(root_num);
                return Ok(());
            }
        }

        // Method 2: Look for /Root anywhere in the PDF (handles xref streams, linearized PDFs)
        // Some PDFs embed the trailer dictionary in an xref stream object
        if let Some(root_num) = Self::extract_root_from_section(&content) {
            self.root = Some(root_num);
            return Ok(());
        }

        // Method 3: Look for /Type /Catalog to find the Root object directly
        for (i, _) in content.match_indices("/Type") {
            let section = &content[i..];
            if section.len() > 20 {
                let check = &section[..section.len().min(50)];
                if check.contains("/Catalog") {
                    // Look backwards for object number
                    if let Some(obj_num) = Self::find_object_number_before(&content, i) {
                        self.root = Some(obj_num);
                        return Ok(());
                    }
                }
            }
        }

        Err(EnhancedError::InvalidParameter(
            "Could not find Root object".into(),
        ))
    }

    /// Extract root object number from a section containing /Root
    fn extract_root_from_section(section: &str) -> Option<i32> {
        if let Some(root_pos) = section.find("/Root") {
            let after_root = &section[root_pos + 5..];
            // Skip whitespace and look for object reference
            let trimmed = after_root.trim_start();

            // Handle both "/Root N 0 R" and "/Root N gen R" formats
            let mut chars = trimmed.chars().peekable();
            let mut num_str = String::new();

            // Skip potential leading whitespace or special chars
            while let Some(&c) = chars.peek() {
                if c.is_ascii_digit() || c == '-' {
                    break;
                }
                chars.next();
            }

            // Collect digits
            while let Some(&c) = chars.peek() {
                if c.is_ascii_digit() {
                    num_str.push(c);
                    chars.next();
                } else {
                    break;
                }
            }

            if let Ok(num) = num_str.parse::<i32>() {
                return Some(num);
            }
        }
        None
    }

    /// Find object number before a given position (for /Type /Catalog matching)
    fn find_object_number_before(content: &str, pos: usize) -> Option<i32> {
        // Look backwards for "N 0 obj" pattern
        let before = &content[..pos];

        // Find the last "obj" keyword before pos
        if let Some(obj_pos) = before.rfind(" obj") {
            let section_before_obj = &before[..obj_pos];
            // Extract the number before "obj"
            let mut num_end = section_before_obj.len();
            let mut num_start;

            // Find end of number (skip whitespace backwards)
            while num_end > 0 && section_before_obj.as_bytes()[num_end - 1].is_ascii_whitespace() {
                num_end -= 1;
            }

            // Skip generation number
            while num_end > 0 && section_before_obj.as_bytes()[num_end - 1].is_ascii_digit() {
                num_end -= 1;
            }

            // Skip whitespace
            while num_end > 0 && section_before_obj.as_bytes()[num_end - 1].is_ascii_whitespace() {
                num_end -= 1;
            }

            // Find start of object number
            num_start = num_end;
            while num_start > 0 && section_before_obj.as_bytes()[num_start - 1].is_ascii_digit() {
                num_start -= 1;
            }

            if num_start < num_end {
                if let Ok(num) = section_before_obj[num_start..num_end].parse::<i32>() {
                    return Some(num);
                }
            }
        }
        None
    }

    /// Find pages object and page list
    fn find_pages(&mut self) -> Result<()> {
        // Search for Pages object using byte-level searching to avoid UTF-8 boundary issues
        let pages_type_pattern = b"/Type/Pages";
        let pages_type_pattern_space = b"/Type /Pages";

        let mut pages_obj_num = None;

        // Search for /Type/Pages or /Type /Pages pattern in raw bytes
        for (i, window) in self.data.windows(pages_type_pattern.len()).enumerate() {
            if window == pages_type_pattern {
                pages_obj_num = Self::find_obj_number_before_byte_pos(&self.data, i);
                if pages_obj_num.is_some() {
                    break;
                }
            }
        }

        // If not found, try with space
        if pages_obj_num.is_none() {
            for (i, window) in self
                .data
                .windows(pages_type_pattern_space.len())
                .enumerate()
            {
                if window == pages_type_pattern_space {
                    pages_obj_num = Self::find_obj_number_before_byte_pos(&self.data, i);
                    if pages_obj_num.is_some() {
                        break;
                    }
                }
            }
        }

        if let Some(pages_num) = pages_obj_num {
            self.pages = Some(pages_num);
        }

        // Find page count
        let page_count = self.get_page_count()?;

        // Find individual page objects by looking for /Type /Page (singular, not /Pages)
        self.find_page_objects(page_count);

        Ok(())
    }

    /// Find object number before a byte position (for binary PDF data)
    fn find_obj_number_before_byte_pos(data: &[u8], pos: usize) -> Option<i32> {
        // Look backwards for " obj" pattern
        let obj_pattern = b" obj";
        let search_start = pos.saturating_sub(500); // Look back up to 500 bytes
        let search_data = &data[search_start..pos];

        // Find the last " obj" in the search region
        let mut last_obj_pos = None;
        for i in 0..search_data.len().saturating_sub(obj_pattern.len()) {
            if &search_data[i..i + obj_pattern.len()] == obj_pattern {
                last_obj_pos = Some(search_start + i);
            }
        }

        if let Some(obj_pos) = last_obj_pos {
            // Look backwards from obj_pos to find the object number
            // Format: "N M obj" where N is object number, M is generation
            let before_obj = &data[obj_pos.saturating_sub(30)..obj_pos];

            // Find the digits working backwards
            let mut digits_end = before_obj.len();

            // Skip any whitespace at the end (before "obj")
            while digits_end > 0 && before_obj[digits_end - 1].is_ascii_whitespace() {
                digits_end -= 1;
            }

            // Skip generation number
            while digits_end > 0 && before_obj[digits_end - 1].is_ascii_digit() {
                digits_end -= 1;
            }

            // Skip whitespace between object number and generation
            while digits_end > 0 && before_obj[digits_end - 1].is_ascii_whitespace() {
                digits_end -= 1;
            }

            // Now read the object number
            let mut digits_start = digits_end;
            while digits_start > 0 && before_obj[digits_start - 1].is_ascii_digit() {
                digits_start -= 1;
            }

            if digits_start < digits_end {
                let num_str = String::from_utf8_lossy(&before_obj[digits_start..digits_end]);
                if let Ok(num) = num_str.parse::<i32>() {
                    return Some(num);
                }
            }
        }

        None
    }

    /// Find individual page objects
    fn find_page_objects(&mut self, expected_count: usize) {
        // Look for /Type /Page (not /Pages) patterns
        let page_type_pattern = b"/Type/Page";
        let page_type_pattern_space = b"/Type /Page";

        let mut found_pages = Vec::new();

        // Search for page objects
        for (i, _) in self.data.windows(page_type_pattern.len()).enumerate() {
            let window = &self.data[i..i + page_type_pattern.len()];
            if window == page_type_pattern {
                // Make sure it's not /Type/Pages (check next byte isn't 's')
                if i + page_type_pattern.len() < self.data.len()
                    && self.data[i + page_type_pattern.len()] != b's'
                {
                    if let Some(obj_num) = Self::find_obj_number_before_byte_pos(&self.data, i) {
                        if !found_pages.contains(&obj_num) {
                            found_pages.push(obj_num);
                        }
                    }
                }
            }
        }

        // Try with space
        for (i, _) in self.data.windows(page_type_pattern_space.len()).enumerate() {
            let window = &self.data[i..i + page_type_pattern_space.len()];
            if window == page_type_pattern_space {
                // Make sure it's not /Type /Pages (check next byte isn't 's')
                if i + page_type_pattern_space.len() < self.data.len()
                    && self.data[i + page_type_pattern_space.len()] != b's'
                {
                    if let Some(obj_num) = Self::find_obj_number_before_byte_pos(&self.data, i) {
                        if !found_pages.contains(&obj_num) {
                            found_pages.push(obj_num);
                        }
                    }
                }
            }
        }

        // If we found pages, use them
        if !found_pages.is_empty() {
            self.page_objects = found_pages;
        } else if let Some(pages_num) = self.pages {
            // Fallback: assume pages are sequential after Pages object
            for i in 1..=expected_count {
                self.page_objects.push(pages_num + i as i32);
            }
        }
    }

    /// Get page count
    pub fn page_count(&self) -> Result<usize> {
        self.get_page_count()
    }

    fn get_page_count(&self) -> Result<usize> {
        // Search for /Count in binary data to avoid UTF-8 issues
        let count_pattern = b"/Count";
        let pages_type_pattern = b"/Type/Pages";
        let pages_type_pattern_space = b"/Type /Pages";

        // Find /Type/Pages or /Type /Pages position
        let mut pages_pos = None;
        for (i, _) in self.data.windows(pages_type_pattern.len()).enumerate() {
            if &self.data[i..i + pages_type_pattern.len()] == pages_type_pattern {
                pages_pos = Some(i);
                break;
            }
        }

        if pages_pos.is_none() {
            for (i, _) in self
                .data
                .windows(pages_type_pattern_space.len())
                .enumerate()
            {
                if &self.data[i..i + pages_type_pattern_space.len()] == pages_type_pattern_space {
                    pages_pos = Some(i);
                    break;
                }
            }
        }

        // Look for /Count near the Pages object
        if let Some(pos) = pages_pos {
            // Search in a window around the Pages object (both before and after)
            let search_start = pos.saturating_sub(200);
            let search_end = (pos + 500).min(self.data.len());
            let search_region = &self.data[search_start..search_end];

            for (i, _) in search_region.windows(count_pattern.len()).enumerate() {
                if &search_region[i..i + count_pattern.len()] == count_pattern {
                    // Found /Count, extract the number
                    let after_count = &search_region[i + count_pattern.len()..];

                    // Skip whitespace and extract digits
                    let mut start = 0;
                    while start < after_count.len() && after_count[start].is_ascii_whitespace() {
                        start += 1;
                    }

                    let mut end = start;
                    while end < after_count.len() && after_count[end].is_ascii_digit() {
                        end += 1;
                    }

                    if start < end {
                        let num_str = String::from_utf8_lossy(&after_count[start..end]);
                        if let Ok(count) = num_str.parse::<usize>() {
                            return Ok(count);
                        }
                    }
                }
            }
        }

        // Fallback: count the number of /Type /Page (not /Pages) occurrences
        let page_type_pattern = b"/Type /Page";
        let page_type_pattern_no_space = b"/Type/Page";
        let mut count = 0;

        for (i, _) in self.data.windows(page_type_pattern.len()).enumerate() {
            if &self.data[i..i + page_type_pattern.len()] == page_type_pattern {
                // Make sure it's not /Type /Pages
                if i + page_type_pattern.len() < self.data.len()
                    && self.data[i + page_type_pattern.len()] != b's'
                {
                    count += 1;
                }
            }
        }

        for (i, _) in self
            .data
            .windows(page_type_pattern_no_space.len())
            .enumerate()
        {
            if &self.data[i..i + page_type_pattern_no_space.len()] == page_type_pattern_no_space {
                // Make sure it's not /Type/Pages
                if i + page_type_pattern_no_space.len() < self.data.len()
                    && self.data[i + page_type_pattern_no_space.len()] != b's'
                {
                    count += 1;
                }
            }
        }

        if count > 0 {
            return Ok(count);
        }

        Ok(1) // Default to 1 page
    }

    /// Get page object number
    pub fn get_page_object(&self, page_num: usize) -> Option<i32> {
        self.page_objects.get(page_num).copied()
    }

    /// Get raw PDF data
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Get xref table
    pub fn xref(&self) -> &XrefTable {
        &self.xref
    }
}
