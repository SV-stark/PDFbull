//! Bookmark Writer - Create and modify PDF outline structures
//!
//! This module provides functionality to create and modify bookmark (outline)
//! structures in PDF files by generating the necessary PDF objects.

use super::bookmarks::Bookmark;
use super::error::{EnhancedError, Result};
use std::collections::HashMap;

/// Find a byte pattern in data, starting from a given position
fn find_bytes_pattern(data: &[u8], pattern: &[u8], start: usize) -> Option<usize> {
    if start >= data.len() || pattern.is_empty() {
        return None;
    }
    data[start..]
        .windows(pattern.len())
        .position(|window| window == pattern)
        .map(|idx| start + idx)
}

/// Find a byte pattern in data, searching from the end
fn find_bytes_pattern_reverse(data: &[u8], pattern: &[u8]) -> Option<usize> {
    if pattern.is_empty() || data.len() < pattern.len() {
        return None;
    }
    for i in (0..=data.len() - pattern.len()).rev() {
        if &data[i..i + pattern.len()] == pattern {
            return Some(i);
        }
    }
    None
}

/// Bookmark outline item in PDF format
#[derive(Debug, Clone)]
pub struct OutlineItem {
    /// Object number for this outline item
    pub obj_num: i32,
    /// Title of the bookmark
    pub title: String,
    /// Destination page (0-indexed)
    pub page: usize,
    /// Parent outline object number
    pub parent: Option<i32>,
    /// First child outline object number
    pub first: Option<i32>,
    /// Last child outline object number
    pub last: Option<i32>,
    /// Next sibling outline object number
    pub next: Option<i32>,
    /// Previous sibling outline object number
    pub prev: Option<i32>,
    /// Number of descendants (for Count)
    pub count: i32,
}

impl OutlineItem {
    /// Create a new outline item
    pub fn new(obj_num: i32, title: String, page: usize) -> Self {
        Self {
            obj_num,
            title,
            page,
            parent: None,
            first: None,
            last: None,
            next: None,
            prev: None,
            count: 0,
        }
    }

    /// Generate PDF object for this outline item
    pub fn to_pdf_object(&self, page_obj_num: i32) -> String {
        let mut obj = format!("{} 0 obj\n<<\n", self.obj_num);
        obj.push_str(&format!("/Title ({})\n", escape_pdf_string(&self.title)));

        // Destination: [page_obj /XYZ null null null]
        obj.push_str(&format!(
            "/Dest [{} 0 R /XYZ null null null]\n",
            page_obj_num
        ));

        if let Some(parent) = self.parent {
            obj.push_str(&format!("/Parent {} 0 R\n", parent));
        }

        if let Some(first) = self.first {
            obj.push_str(&format!("/First {} 0 R\n", first));
        }

        if let Some(last) = self.last {
            obj.push_str(&format!("/Last {} 0 R\n", last));
        }

        if let Some(next) = self.next {
            obj.push_str(&format!("/Next {} 0 R\n", next));
        }

        if let Some(prev) = self.prev {
            obj.push_str(&format!("/Prev {} 0 R\n", prev));
        }

        if self.count != 0 {
            obj.push_str(&format!("/Count {}\n", self.count));
        }

        obj.push_str(">>\nendobj\n");
        obj
    }
}

/// Outline (bookmarks) root object
#[derive(Debug, Clone)]
pub struct Outlines {
    /// Object number for outlines root
    pub obj_num: i32,
    /// First outline item
    pub first: Option<i32>,
    /// Last outline item
    pub last: Option<i32>,
    /// Total count of visible outline items
    pub count: i32,
}

impl Outlines {
    /// Create new outlines root
    pub fn new(obj_num: i32) -> Self {
        Self {
            obj_num,
            first: None,
            last: None,
            count: 0,
        }
    }

    /// Generate PDF object for outlines root
    pub fn to_pdf_object(&self) -> String {
        let mut obj = format!("{} 0 obj\n<<\n", self.obj_num);
        obj.push_str("/Type /Outlines\n");

        if let Some(first) = self.first {
            obj.push_str(&format!("/First {} 0 R\n", first));
        }

        if let Some(last) = self.last {
            obj.push_str(&format!("/Last {} 0 R\n", last));
        }

        if self.count != 0 {
            obj.push_str(&format!("/Count {}\n", self.count));
        }

        obj.push_str(">>\nendobj\n");
        obj
    }
}

/// Escape PDF string
fn escape_pdf_string(s: &str) -> String {
    let mut result = String::new();
    for ch in s.chars() {
        match ch {
            '(' => result.push_str("\\("),
            ')' => result.push_str("\\)"),
            '\\' => result.push_str("\\\\"),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            _ if ch.is_ascii() => result.push(ch),
            _ => {
                // Unicode characters - use hex notation
                for byte in ch.to_string().as_bytes() {
                    result.push_str(&format!("\\{:03o}", byte));
                }
            }
        }
    }
    result
}

/// Build outline tree from flat bookmark list
pub fn build_outline_tree(
    bookmarks: &[Bookmark],
    start_obj_num: i32,
    page_objects: &HashMap<usize, i32>,
) -> Result<(Outlines, Vec<OutlineItem>)> {
    if bookmarks.is_empty() {
        return Err(EnhancedError::InvalidParameter(
            "No bookmarks provided".into(),
        ));
    }

    let mut outline_items = Vec::new();
    let mut current_obj_num = start_obj_num + 1; // Reserve start_obj_num for Outlines root

    // Create outline items
    for bookmark in bookmarks {
        // Convert 0-indexed page to 1-indexed for page_objects lookup
        let page_key = bookmark.page + 1;
        let page_obj_num = *page_objects.get(&page_key).ok_or_else(|| {
            EnhancedError::InvalidParameter(format!(
                "Page {} not found (1-indexed: {})",
                bookmark.page, page_key
            ))
        })?;

        let mut item = OutlineItem::new(current_obj_num, bookmark.title.clone(), bookmark.page);

        // Handle children recursively
        if !bookmark.children.is_empty() {
            let child_start = current_obj_num + 1;
            item.first = Some(child_start);

            let (child_items, last_child_num) = build_child_items(
                &bookmark.children,
                child_start,
                current_obj_num, // Parent
                page_objects,
            )?;

            item.last = Some(last_child_num);
            item.count = child_items.len() as i32;
            current_obj_num += child_items.len() as i32 + 1;

            outline_items.push(item);
            outline_items.extend(child_items);
        } else {
            current_obj_num += 1;
            outline_items.push(item);
        }
    }

    // Link siblings
    for i in 0..outline_items.len() - 1 {
        if outline_items[i].parent == outline_items[i + 1].parent {
            outline_items[i].next = Some(outline_items[i + 1].obj_num);
            outline_items[i + 1].prev = Some(outline_items[i].obj_num);
        }
    }

    // Create Outlines root
    let mut outlines = Outlines::new(start_obj_num);
    outlines.first = Some(outline_items[0].obj_num);
    outlines.last = Some(outline_items[outline_items.len() - 1].obj_num);
    outlines.count = bookmarks.len() as i32;

    // Set parent for top-level items
    for item in &mut outline_items {
        if item.parent.is_none() {
            item.parent = Some(start_obj_num);
        }
    }

    Ok((outlines, outline_items))
}

/// Build child outline items recursively
fn build_child_items(
    children: &[Bookmark],
    start_obj_num: i32,
    parent_obj_num: i32,
    page_objects: &HashMap<usize, i32>,
) -> Result<(Vec<OutlineItem>, i32)> {
    let mut items = Vec::new();
    let mut current_obj_num = start_obj_num;

    for child in children {
        // Convert 0-indexed page to 1-indexed for page_objects lookup
        let page_key = child.page + 1;
        let page_obj_num = *page_objects.get(&page_key).ok_or_else(|| {
            EnhancedError::InvalidParameter(format!(
                "Page {} not found (1-indexed: {})",
                child.page, page_key
            ))
        })?;

        let mut item = OutlineItem::new(current_obj_num, child.title.clone(), child.page);
        item.parent = Some(parent_obj_num);

        if !child.children.is_empty() {
            let child_start = current_obj_num + 1;
            item.first = Some(child_start);

            let (child_items, last_child_num) =
                build_child_items(&child.children, child_start, current_obj_num, page_objects)?;

            item.last = Some(last_child_num);
            item.count = child_items.len() as i32;
            current_obj_num += child_items.len() as i32 + 1;

            items.push(item);
            items.extend(child_items);
        } else {
            current_obj_num += 1;
            items.push(item);
        }
    }

    let last_obj_num = if items.is_empty() {
        start_obj_num - 1
    } else {
        items[items.len() - 1].obj_num
    };

    Ok((items, last_obj_num))
}

/// Add bookmarks to PDF data
pub fn insert_bookmarks_into_pdf(
    pdf_data: &mut Vec<u8>,
    bookmarks: &[Bookmark],
    page_objects: &HashMap<usize, i32>,
    next_obj_num: i32,
) -> Result<i32> {
    // Build outline tree
    let (outlines, items) = build_outline_tree(bookmarks, next_obj_num, page_objects)?;

    // Find positions using BYTE-based searching (not character-based)
    // This is critical because String::from_utf8_lossy can change positions if there's invalid UTF-8
    let (catalog_insert_pos, existing_outlines_pos, xref_pos) = {
        let mut catalog_pos_opt = None;
        let mut existing_outlines = None;

        // Find Catalog object using byte search
        let catalog_start = find_bytes_pattern(pdf_data, b"/Type /Catalog", 0)
            .or_else(|| find_bytes_pattern(pdf_data, b"/Type/Catalog", 0));

        if let Some(catalog_pos) = catalog_start {
            // Find the end of the catalog dictionary
            if let Some(dict_end_offset) = find_bytes_pattern(pdf_data, b">>", catalog_pos) {
                // Check if /Outlines already exists in this catalog
                let catalog_section = &pdf_data[catalog_pos..dict_end_offset];
                if let Some(outlines_pos) = find_bytes_pattern(catalog_section, b"/Outlines", 0) {
                    // Found existing /Outlines - we need to replace it
                    existing_outlines = Some(catalog_pos + outlines_pos);
                } else {
                    // No existing /Outlines - insert before >>
                    catalog_pos_opt = Some(dict_end_offset);
                }
            }
        }

        // Find xref using byte search (search from end)
        // We need to keep the newline before xref, so look for \nxref or \r\nxref
        let xref_pos = if let Some(pos) = find_bytes_pattern_reverse(pdf_data, b"\nxref") {
            pos + 1 // Keep the \n, position at 'x'
        } else if let Some(pos) = find_bytes_pattern_reverse(pdf_data, b"\r\nxref") {
            pos + 2 // Keep the \r\n, position at 'x'
        } else {
            find_bytes_pattern_reverse(pdf_data, b"xref")
                .ok_or_else(|| EnhancedError::Generic("xref not found".into()))?
        };

        (catalog_pos_opt, existing_outlines, xref_pos)
    };

    // Handle /Outlines reference in Catalog
    let mut bytes_inserted: i64 = 0;
    if let Some(existing_pos) = existing_outlines_pos {
        // Replace existing /Outlines reference
        // Find the end of the existing reference (N 0 R pattern)
        let search_start = existing_pos + 9; // After "/Outlines"
        let mut ref_end = search_start;

        // Skip whitespace
        while ref_end < pdf_data.len() && pdf_data[ref_end].is_ascii_whitespace() {
            ref_end += 1;
        }
        // Skip number
        while ref_end < pdf_data.len() && pdf_data[ref_end].is_ascii_digit() {
            ref_end += 1;
        }
        // Skip " 0 R"
        while ref_end < pdf_data.len()
            && (pdf_data[ref_end].is_ascii_whitespace()
                || pdf_data[ref_end] == b'0'
                || pdf_data[ref_end] == b'R')
        {
            ref_end += 1;
            // Stop at R
            if ref_end > 0 && pdf_data[ref_end - 1] == b'R' {
                break;
            }
        }

        // Replace the old reference with the new one
        let new_ref = format!("/Outlines {} 0 R", outlines.obj_num);
        let old_len = ref_end - existing_pos;
        let new_bytes = new_ref.as_bytes();
        bytes_inserted = new_bytes.len() as i64 - old_len as i64;
        pdf_data.splice(existing_pos..ref_end, new_bytes.iter().cloned());
    } else if let Some(insert_pos) = catalog_insert_pos {
        // Insert new /Outlines reference
        let outlines_ref = format!("/Outlines {} 0 R\n", outlines.obj_num);
        bytes_inserted = outlines_ref.len() as i64;
        if insert_pos > pdf_data.len() {
            return Err(EnhancedError::Generic(format!(
                "Catalog insert position {} exceeds pdf_data length {}",
                insert_pos,
                pdf_data.len()
            )));
        }
        let outlines_bytes = outlines_ref.as_bytes();
        pdf_data.splice(insert_pos..insert_pos, outlines_bytes.iter().cloned());
    }

    // Generate outline objects
    let mut outline_data = String::new();
    outline_data.push_str(&outlines.to_pdf_object());
    outline_data.push('\n');

    for item in &items {
        // Convert 0-indexed page to 1-indexed for page_objects lookup
        let page_key = item.page + 1;
        let page_obj = *page_objects.get(&page_key).ok_or_else(|| {
            EnhancedError::InvalidParameter(format!(
                "Page {} not found in page_objects (1-indexed: {})",
                item.page, page_key
            ))
        })?;
        outline_data.push_str(&item.to_pdf_object(page_obj));
        outline_data.push('\n');
    }

    // Remove everything from xref to end - we'll rebuild it
    // Account for any bytes we inserted before xref
    let catalog_change_pos = existing_outlines_pos
        .or(catalog_insert_pos)
        .unwrap_or(usize::MAX);
    let adjusted_xref_pos = if catalog_change_pos < xref_pos {
        ((xref_pos as i64) + bytes_inserted) as usize
    } else {
        xref_pos
    };

    // Find the actual xref keyword position in the adjusted range
    // The xref_pos we found earlier was using byte search, keep the newline before xref
    let truncate_pos = adjusted_xref_pos;

    if truncate_pos < pdf_data.len() {
        pdf_data.truncate(truncate_pos);
    }

    // Append outline objects
    pdf_data.extend_from_slice(outline_data.as_bytes());

    // Calculate new object count
    let new_obj_count = next_obj_num + 1 + items.len() as i32;

    // Rebuild xref table by scanning for all objects
    let mut offsets = vec![0usize; new_obj_count as usize];

    // Find all "N 0 obj" patterns
    let mut search_pos = 0;
    while search_pos < pdf_data.len() {
        // Look for digit at line start or after whitespace
        if search_pos == 0 || pdf_data[search_pos - 1] == b'\n' || pdf_data[search_pos - 1] == b'\r'
        {
            if pdf_data[search_pos].is_ascii_digit() {
                let num_start = search_pos;
                let mut num_end = search_pos;
                while num_end < pdf_data.len() && pdf_data[num_end].is_ascii_digit() {
                    num_end += 1;
                }

                // Check for " 0 obj"
                if num_end + 6 <= pdf_data.len() {
                    let after_num = &pdf_data[num_end..num_end + 6];
                    if after_num == b" 0 obj" {
                        if let Ok(obj_num_str) = std::str::from_utf8(&pdf_data[num_start..num_end])
                        {
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
    let xref_offset = pdf_data.len();
    pdf_data.extend_from_slice(format!("xref\n0 {}\n", new_obj_count).as_bytes());
    pdf_data.extend_from_slice(b"0000000000 65535 f \n");

    for i in 1..new_obj_count as usize {
        let offset = offsets.get(i).copied().unwrap_or(0);
        pdf_data.extend_from_slice(format!("{:010} 00000 n \n", offset).as_bytes());
    }

    // Write trailer
    pdf_data
        .extend_from_slice(format!("trailer\n<</Size {}/Root 1 0 R>>\n", new_obj_count).as_bytes());
    pdf_data.extend_from_slice(format!("startxref\n{}\n%%EOF\n", xref_offset).as_bytes());

    // Return next available object number
    Ok(new_obj_count)
}
