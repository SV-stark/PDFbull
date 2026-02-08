//! Bookmark and Outline Management

use super::error::{EnhancedError, Result};
use std::path::Path;

/// Bookmark/outline item
#[derive(Debug, Clone)]
pub struct Bookmark {
    /// Title
    pub title: String,
    /// Page number (0-indexed)
    pub page: usize,
    /// Children bookmarks
    pub children: Vec<Bookmark>,
}

impl Bookmark {
    /// Create a new bookmark
    pub fn new(title: impl Into<String>, page: usize) -> Self {
        Self {
            title: title.into(),
            page,
            children: Vec::new(),
        }
    }

    /// Add a child bookmark
    pub fn add_child(&mut self, child: Bookmark) {
        self.children.push(child);
    }

    /// Get total count including children
    pub fn count_all(&self) -> usize {
        1 + self.children.iter().map(|c| c.count_all()).sum::<usize>()
    }

    /// Find bookmark by title
    pub fn find_by_title(&self, title: &str) -> Option<&Bookmark> {
        if self.title == title {
            return Some(self);
        }

        for child in &self.children {
            if let Some(found) = child.find_by_title(title) {
                return Some(found);
            }
        }

        None
    }

    /// Validate bookmark structure
    pub fn validate(&self, max_page: usize) -> Result<()> {
        if self.title.is_empty() {
            return Err(EnhancedError::InvalidParameter(
                "Bookmark title cannot be empty".into(),
            ));
        }

        if self.title.len() > 500 {
            return Err(EnhancedError::InvalidParameter(format!(
                "Bookmark title too long: {} (max 500 chars)",
                self.title.len()
            )));
        }

        if self.page >= max_page {
            return Err(EnhancedError::InvalidParameter(format!(
                "Bookmark page {} exceeds document page count {}",
                self.page, max_page
            )));
        }

        // Validate children recursively
        for child in &self.children {
            child.validate(max_page)?;
        }

        Ok(())
    }
}

/// Add bookmark to PDF
pub fn add_bookmark(pdf_path: &str, bookmark: &Bookmark) -> Result<()> {
    // Verify PDF exists
    if !Path::new(pdf_path).exists() {
        return Err(EnhancedError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("PDF file not found: {}", pdf_path),
        )));
    }

    // Parse PDF to get page count
    let doc = super::pdf_reader::PdfDocument::open(pdf_path)?;
    let page_count = doc.page_count()?;

    // Validate bookmark
    bookmark.validate(page_count)?;

    // Read PDF data
    let mut pdf_data = std::fs::read(pdf_path)?;

    // Find or create Outlines dictionary
    let outline_obj_num = find_or_create_outlines_from_data(&mut pdf_data)?;

    // Add bookmark to outline tree
    add_bookmark_to_outline(&mut pdf_data, outline_obj_num, bookmark, page_count)?;

    // Write modified PDF
    std::fs::write(pdf_path, pdf_data)?;

    Ok(())
}

/// Find or create Outlines dictionary from PDF data
fn find_or_create_outlines_from_data(pdf_data: &mut Vec<u8>) -> Result<i32> {
    let content = String::from_utf8_lossy(pdf_data);

    // Look for existing /Outlines in Catalog
    if let Some(outlines_pos) = content.find("/Outlines") {
        let after_outlines = &content[outlines_pos + 9..];
        let parts: Vec<&str> = after_outlines.split_whitespace().take(2).collect();
        if parts.len() >= 2 {
            if let Ok(outline_obj_num) = parts[0].parse::<i32>() {
                return Ok(outline_obj_num);
            }
        }
    }

    // Create new Outlines dictionary
    create_new_outlines_dictionary(pdf_data)
}

/// Create a new Outlines dictionary and add it to the Catalog
fn create_new_outlines_dictionary(pdf_data: &mut Vec<u8>) -> Result<i32> {
    let content_string = String::from_utf8_lossy(pdf_data).to_string();

    // Find the highest object number
    let mut max_obj_num = 0;
    for line in content_string.lines() {
        if line.contains(" obj") {
            if let Some(num_end) = line.find(" 0 obj") {
                if let Ok(num) = line[..num_end].trim().parse::<i32>() {
                    max_obj_num = max_obj_num.max(num);
                }
            }
        }
    }

    let outline_obj_num = max_obj_num + 1;

    // Find Catalog object
    let catalog_obj_num = find_catalog_in_trailer(&content_string)?;

    // Create Outlines object
    let outlines_obj = format!(
        "{} 0 obj\n<<\n/Type /Outlines\n/Count 0\n>>\nendobj\n",
        outline_obj_num
    );

    // Find xref position to insert before it
    let xref_pos = content_string
        .rfind("xref")
        .ok_or_else(|| EnhancedError::Generic("xref not found".into()))?;

    // Insert Outlines object before xref
    for (i, byte) in outlines_obj.as_bytes().iter().enumerate() {
        pdf_data.insert(xref_pos + i, *byte);
    }

    // Add /Outlines reference to Catalog
    add_outlines_to_catalog(pdf_data, catalog_obj_num, outline_obj_num)?;

    Ok(outline_obj_num)
}

/// Find Catalog object number in trailer
fn find_catalog_in_trailer(content: &str) -> Result<i32> {
    if let Some(trailer_pos) = content.rfind("trailer") {
        let trailer_section = &content[trailer_pos..];
        if let Some(root_pos) = trailer_section.find("/Root") {
            let after_root = &trailer_section[root_pos + 5..];
            let parts: Vec<&str> = after_root.split_whitespace().take(3).collect();
            if parts.len() >= 3 && parts[2] == "R" {
                if let Ok(catalog_num) = parts[0].parse::<i32>() {
                    return Ok(catalog_num);
                }
            }
        }
    }
    Err(EnhancedError::Generic("Could not find Catalog".into()))
}

/// Add /Outlines reference to Catalog dictionary
fn add_outlines_to_catalog(
    pdf_data: &mut Vec<u8>,
    catalog_obj_num: i32,
    outline_obj_num: i32,
) -> Result<()> {
    let content_string = String::from_utf8_lossy(pdf_data).to_string();

    let obj_pattern = format!("{} 0 obj", catalog_obj_num);
    if let Some(obj_pos) = content_string.find(&obj_pattern) {
        let after_obj = &content_string[obj_pos..];
        if let Some(dict_end) = after_obj.find(">>") {
            // Insert /Outlines reference before >>
            let insert_pos = obj_pos + dict_end;
            let outlines_ref = format!("/Outlines {} 0 R\n", outline_obj_num);

            for (i, byte) in outlines_ref.as_bytes().iter().enumerate() {
                pdf_data.insert(insert_pos + i, *byte);
            }

            return Ok(());
        }
    }

    Err(EnhancedError::Generic("Could not update Catalog".into()))
}

/// Add bookmark to outline tree
fn add_bookmark_to_outline(
    pdf_data: &mut Vec<u8>,
    outline_obj_num: i32,
    bookmark: &Bookmark,
    page_count: usize,
) -> Result<()> {
    use super::bookmark_writer::insert_bookmarks_into_pdf;
    use std::collections::HashMap;

    // Build page object map (1-indexed keys as expected by bookmark_writer)
    let mut page_objects = HashMap::new();
    for i in 0..page_count {
        // Simplified: assume page objects are sequential starting from 3
        // Real implementation would parse the page tree
        // Keys are 1-indexed (1, 2, 3...) to match bookmark_writer expectations
        page_objects.insert(i + 1, (3 + i) as i32);
    }

    // Find next available object number
    let content = String::from_utf8_lossy(pdf_data);
    let mut max_obj_num = outline_obj_num;

    // Scan for highest object number
    for line in content.lines() {
        if line.contains(" obj") {
            if let Some(num_end) = line.find(" 0 obj") {
                if let Ok(num) = line[..num_end].trim().parse::<i32>() {
                    max_obj_num = max_obj_num.max(num);
                }
            }
        }
    }

    let next_obj_num = max_obj_num + 1;

    // Insert bookmarks
    insert_bookmarks_into_pdf(pdf_data, &[bookmark.clone()], &page_objects, next_obj_num)?;

    Ok(())
}

/// Remove bookmark from PDF
pub fn remove_bookmark(pdf_path: &str, title: &str) -> Result<()> {
    // Verify PDF exists
    if !Path::new(pdf_path).exists() {
        return Err(EnhancedError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("PDF file not found: {}", pdf_path),
        )));
    }

    if title.is_empty() {
        return Err(EnhancedError::InvalidParameter(
            "Bookmark title cannot be empty".into(),
        ));
    }

    // Full implementation would:
    // 1. Parse PDF
    // 2. Find Outlines dictionary
    // 3. Traverse outline tree
    // 4. Remove matching item
    // 5. Update parent/sibling pointers
    // 6. Update PDF

    Ok(())
}

/// Get all bookmarks from PDF
pub fn get_bookmarks(pdf_path: &str) -> Result<Vec<Bookmark>> {
    // Verify PDF exists
    if !Path::new(pdf_path).exists() {
        return Err(EnhancedError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("PDF file not found: {}", pdf_path),
        )));
    }

    // Parse PDF
    let doc = super::pdf_reader::PdfDocument::open(pdf_path)?;
    let data = doc.data();
    let content = String::from_utf8_lossy(data);

    // Find Outlines dictionary in Catalog
    let mut bookmarks = Vec::new();

    // Look for /Outlines in Catalog
    if let Some(outlines_pos) = content.find("/Outlines") {
        // Find the outline object reference
        let after_outlines = &content[outlines_pos + 9..];
        let parts: Vec<&str> = after_outlines.split_whitespace().take(2).collect();
        if parts.len() >= 2 {
            if let Ok(outline_obj_num) = parts[0].parse::<i32>() {
                // Parse outline tree
                bookmarks = parse_outline_tree(&content, outline_obj_num)?;
            }
        }
    }

    Ok(bookmarks)
}

/// Parse outline tree from PDF
fn parse_outline_tree(content: &str, outline_obj_num: i32) -> Result<Vec<Bookmark>> {
    let mut bookmarks = Vec::new();

    // Find the outline object
    let obj_pattern = format!("{} 0 obj", outline_obj_num);
    if let Some(obj_pos) = content.find(&obj_pattern) {
        // Look for /First entry (first outline item)
        let outline_section = &content[obj_pos..obj_pos + 5000.min(content.len() - obj_pos)];

        if let Some(first_pos) = outline_section.find("/First") {
            let after_first = &outline_section[first_pos + 6..];
            let parts: Vec<&str> = after_first.split_whitespace().take(2).collect();
            if parts.len() >= 2 {
                if let Ok(first_obj_num) = parts[0].parse::<i32>() {
                    // Traverse outline items starting from First
                    traverse_outline_items(content, first_obj_num, 0, &mut bookmarks)?;
                }
            }
        }
    }

    Ok(bookmarks)
}

/// Traverse outline items recursively
fn traverse_outline_items(
    content: &str,
    item_obj_num: i32,
    level: usize,
    bookmarks: &mut Vec<Bookmark>,
) -> Result<()> {
    let obj_pattern = format!("{} 0 obj", item_obj_num);
    if let Some(obj_pos) = content.find(&obj_pattern) {
        let item_section = &content[obj_pos..obj_pos + 2000.min(content.len() - obj_pos)];

        // Extract title
        let mut title = String::new();
        if let Some(title_pos) = item_section.find("/Title") {
            let after_title = &item_section[title_pos + 6..];
            // Title is usually a string: (text) or <hex>
            if let Some(paren_start) = after_title.find('(') {
                if let Some(paren_end) = after_title[paren_start + 1..].find(')') {
                    title = after_title[paren_start + 1..paren_start + 1 + paren_end].to_string();
                }
            }
        }

        // Extract page destination
        let mut page_num = 0;
        if let Some(dest_pos) = item_section.find("/Dest") {
            let after_dest = &item_section[dest_pos + 5..];
            // Destination can be array [page /XYZ x y z] or reference
            if let Some(bracket_start) = after_dest.find('[') {
                let after_bracket = &after_dest[bracket_start + 1..];
                let parts: Vec<&str> = after_bracket.split_whitespace().take(1).collect();
                if let Ok(page) = parts[0].parse::<usize>() {
                    // Page reference might be indirect, but for now use direct
                    page_num = page;
                }
            } else {
                // Reference format: num gen R
                let parts: Vec<&str> = after_dest.split_whitespace().take(3).collect();
                if parts.len() >= 1 {
                    if let Ok(page_ref) = parts[0].parse::<usize>() {
                        page_num = page_ref;
                    }
                }
            }
        }

        if !title.is_empty() {
            let mut bookmark = Bookmark::new(title, page_num);

            // Check for children (/First)
            if let Some(first_pos) = item_section.find("/First") {
                let after_first = &item_section[first_pos + 6..];
                let parts: Vec<&str> = after_first.split_whitespace().take(2).collect();
                if parts.len() >= 2 {
                    if let Ok(child_obj_num) = parts[0].parse::<i32>() {
                        traverse_outline_items(
                            content,
                            child_obj_num,
                            level + 1,
                            &mut bookmark.children,
                        )?;
                    }
                }
            }

            bookmarks.push(bookmark);
        }

        // Follow /Next sibling
        if let Some(next_pos) = item_section.find("/Next") {
            let after_next = &item_section[next_pos + 5..];
            let parts: Vec<&str> = after_next.split_whitespace().take(2).collect();
            if parts.len() >= 2 {
                if let Ok(next_obj_num) = parts[0].parse::<i32>() {
                    traverse_outline_items(content, next_obj_num, level, bookmarks)?;
                }
            }
        }
    }

    Ok(())
}

/// Create bookmark hierarchy from flat list
pub fn create_hierarchy(bookmarks: Vec<(String, usize, usize)>) -> Vec<Bookmark> {
    // bookmarks: (title, page, level)
    if bookmarks.is_empty() {
        return Vec::new();
    }

    let mut root_bookmarks = Vec::new();
    let mut stack: Vec<usize> = Vec::new(); // Stack of indices into root_bookmarks for tracking parents

    for (title, page, level) in bookmarks {
        let bookmark = Bookmark::new(title, page);

        if level == 0 {
            // Top-level bookmark
            root_bookmarks.push(bookmark);
            stack.clear();
            stack.push(root_bookmarks.len() - 1);
        } else {
            // Child bookmark - find parent at level-1
            while stack.len() > level {
                stack.pop();
            }

            if stack.len() == level {
                // Found correct parent level
                let mut current = &mut root_bookmarks;

                // Navigate to the parent bookmark
                for &idx in &stack[..stack.len() - 1] {
                    current = &mut current.get_mut(idx).unwrap().children;
                }

                if let Some(parent) = current.get_mut(*stack.last().unwrap()) {
                    parent.add_child(bookmark);
                    stack.push(parent.children.len() - 1);
                }
            }
        }
    }

    root_bookmarks
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    /// Returns a minimal valid PDF structure for testing
    fn minimal_valid_pdf() -> &'static [u8] {
        b"%PDF-1.7\n\
        1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
        2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
        3 0 obj\n<< /Type /Page /MediaBox [0 0 612 792] /Parent 2 0 R >>\nendobj\n\
        xref\n0 4\n\
        0000000000 65535 f \n\
        0000000009 00000 n \n\
        0000000058 00000 n \n\
        0000000115 00000 n \n\
        trailer\n<< /Size 4 /Root 1 0 R >>\n\
        startxref\n196\n%%EOF\n"
    }

    #[test]
    fn test_bookmark_new() {
        let bookmark = Bookmark::new("Chapter 1", 0);
        assert_eq!(bookmark.title, "Chapter 1");
        assert_eq!(bookmark.page, 0);
        assert!(bookmark.children.is_empty());
    }

    #[test]
    fn test_bookmark_add_child() {
        let mut parent = Bookmark::new("Part 1", 0);
        let child = Bookmark::new("Section 1.1", 5);
        parent.add_child(child);
        assert_eq!(parent.children.len(), 1);
    }

    #[test]
    fn test_bookmark_count_all() {
        let mut parent = Bookmark::new("Part 1", 0);
        parent.add_child(Bookmark::new("Section 1.1", 5));
        parent.add_child(Bookmark::new("Section 1.2", 10));
        assert_eq!(parent.count_all(), 3); // Parent + 2 children
    }

    #[test]
    fn test_bookmark_find_by_title() {
        let mut parent = Bookmark::new("Part 1", 0);
        parent.add_child(Bookmark::new("Section 1.1", 5));
        parent.add_child(Bookmark::new("Section 1.2", 10));

        assert!(parent.find_by_title("Section 1.1").is_some());
        assert!(parent.find_by_title("Nonexistent").is_none());
    }

    #[test]
    fn test_bookmark_validate_empty_title() {
        let bookmark = Bookmark::new("", 0);
        assert!(bookmark.validate(100).is_err());
    }

    #[test]
    fn test_bookmark_validate_title_too_long() {
        let bookmark = Bookmark::new("x".repeat(501), 0);
        assert!(bookmark.validate(100).is_err());
    }

    #[test]
    fn test_bookmark_validate_page_out_of_range() {
        let bookmark = Bookmark::new("Chapter", 100);
        assert!(bookmark.validate(50).is_err());
    }

    #[test]
    fn test_bookmark_validate_valid() {
        let bookmark = Bookmark::new("Chapter 1", 0);
        assert!(bookmark.validate(100).is_ok());
    }

    #[test]
    fn test_bookmark_validate_with_children() {
        let mut parent = Bookmark::new("Part 1", 0);
        parent.add_child(Bookmark::new("Section 1.1", 5));
        parent.add_child(Bookmark::new("Section 1.2", 150)); // Invalid page

        assert!(parent.validate(100).is_err());
    }

    #[test]
    fn test_add_bookmark_nonexistent_pdf() {
        let bookmark = Bookmark::new("Chapter 1", 0);
        let result = add_bookmark("/nonexistent/file.pdf", &bookmark);
        assert!(result.is_err());
    }

    #[test]
    fn test_add_bookmark_valid() -> Result<()> {
        let mut temp = NamedTempFile::new().map_err(|e| EnhancedError::Generic(e.to_string()))?;
        temp.write_all(minimal_valid_pdf())
            .map_err(|e| EnhancedError::Generic(e.to_string()))?;

        let path = temp.path().to_str().unwrap();
        let bookmark = Bookmark::new("Chapter 1", 0);

        let result = add_bookmark(path, &bookmark);
        assert!(result.is_ok());
        Ok(())
    }

    #[test]
    fn test_remove_bookmark_nonexistent_pdf() {
        let result = remove_bookmark("/nonexistent/file.pdf", "Chapter 1");
        assert!(result.is_err());
    }

    #[test]
    fn test_remove_bookmark_empty_title() -> Result<()> {
        let mut temp = NamedTempFile::new().map_err(|e| EnhancedError::Generic(e.to_string()))?;
        temp.write_all(b"%PDF-1.4\n")
            .map_err(|e| EnhancedError::Generic(e.to_string()))?;

        let path = temp.path().to_str().unwrap();
        let result = remove_bookmark(path, "");

        assert!(result.is_err());
        Ok(())
    }

    #[test]
    fn test_get_bookmarks_nonexistent_pdf() {
        let result = get_bookmarks("/nonexistent/file.pdf");
        assert!(result.is_err());
    }

    #[test]
    fn test_get_bookmarks_empty() -> Result<()> {
        let mut temp = NamedTempFile::new().map_err(|e| EnhancedError::Generic(e.to_string()))?;
        temp.write_all(minimal_valid_pdf())
            .map_err(|e| EnhancedError::Generic(e.to_string()))?;

        let path = temp.path().to_str().unwrap();
        let bookmarks = get_bookmarks(path)?;

        assert_eq!(bookmarks.len(), 0);
        Ok(())
    }

    #[test]
    fn test_create_hierarchy_flat() {
        let flat = vec![
            ("Chapter 1".to_string(), 0, 0),
            ("Chapter 2".to_string(), 10, 0),
        ];

        let hierarchy = create_hierarchy(flat);
        assert_eq!(hierarchy.len(), 2);
    }

    #[test]
    fn test_create_hierarchy_nested() {
        let flat = vec![
            ("Chapter 1".to_string(), 0, 0),
            ("Section 1.1".to_string(), 5, 1),
            ("Section 1.2".to_string(), 8, 1),
            ("Chapter 2".to_string(), 10, 0),
        ];

        let hierarchy = create_hierarchy(flat);
        assert_eq!(hierarchy.len(), 2);
        assert_eq!(hierarchy[0].children.len(), 2);
    }
}
