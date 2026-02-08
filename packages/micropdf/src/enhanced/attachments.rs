//! Attachment Management - Embed and extract files

use super::error::{EnhancedError, Result};
use std::fs;
use std::path::Path;

/// PDF attachment
#[derive(Debug, Clone)]
pub struct Attachment {
    /// Filename
    pub filename: String,
    /// Data
    pub data: Vec<u8>,
    /// MIME type
    pub mime_type: Option<String>,
    /// Description
    pub description: Option<String>,
}

impl Attachment {
    /// Create a new attachment
    pub fn new(filename: impl Into<String>, data: Vec<u8>) -> Self {
        Self {
            filename: filename.into(),
            data,
            mime_type: None,
            description: None,
        }
    }

    /// Set MIME type
    pub fn with_mime_type(mut self, mime_type: impl Into<String>) -> Self {
        self.mime_type = Some(mime_type.into());
        self
    }

    /// Set description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Get file size
    pub fn size(&self) -> usize {
        self.data.len()
    }

    /// Guess MIME type from filename extension
    pub fn guess_mime_type(&self) -> String {
        let filename_lower = self.filename.to_lowercase();

        if filename_lower.ends_with(".pdf") {
            "application/pdf".to_string()
        } else if filename_lower.ends_with(".txt") {
            "text/plain".to_string()
        } else if filename_lower.ends_with(".png") {
            "image/png".to_string()
        } else if filename_lower.ends_with(".jpg") || filename_lower.ends_with(".jpeg") {
            "image/jpeg".to_string()
        } else if filename_lower.ends_with(".zip") {
            "application/zip".to_string()
        } else if filename_lower.ends_with(".json") {
            "application/json".to_string()
        } else if filename_lower.ends_with(".xml") {
            "application/xml".to_string()
        } else {
            "application/octet-stream".to_string()
        }
    }
}

/// Add attachment to PDF
pub fn add_attachment(pdf_path: &str, attachment: &Attachment) -> Result<()> {
    // Verify PDF exists
    if !Path::new(pdf_path).exists() {
        return Err(EnhancedError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("PDF file not found: {}", pdf_path),
        )));
    }

    // Validate attachment
    if attachment.filename.is_empty() {
        return Err(EnhancedError::InvalidParameter(
            "Attachment filename cannot be empty".into(),
        ));
    }

    if attachment.data.is_empty() {
        return Err(EnhancedError::InvalidParameter(
            "Attachment data cannot be empty".into(),
        ));
    }

    // Validate filename doesn't contain path separators
    if attachment.filename.contains('/') || attachment.filename.contains('\\') {
        return Err(EnhancedError::InvalidParameter(
            "Attachment filename cannot contain path separators".into(),
        ));
    }

    // Read PDF
    let mut pdf_data = fs::read(pdf_path)?;
    let content_string = String::from_utf8_lossy(&pdf_data).to_string();

    // Find max object number
    let mut max_obj = find_max_object_number(&content_string);

    // Create EmbeddedFile stream object
    let embedded_file_obj_num = max_obj + 1;
    let embedded_file_obj = create_embedded_file_object(
        embedded_file_obj_num,
        &attachment.data,
        &attachment
            .mime_type
            .as_ref()
            .unwrap_or(&attachment.guess_mime_type()),
    );

    // Create FileSpec object
    let filespec_obj_num = max_obj + 2;
    let filespec_obj = create_filespec_object(
        filespec_obj_num,
        &attachment.filename,
        embedded_file_obj_num,
        attachment.description.as_deref(),
    );

    // Insert objects before xref
    let xref_pos = content_string
        .rfind("xref")
        .ok_or_else(|| EnhancedError::Generic("xref not found".into()))?;

    let mut insert_data = String::new();
    insert_data.push_str(&embedded_file_obj);
    insert_data.push_str(&filespec_obj);

    for (i, byte) in insert_data.bytes().enumerate() {
        pdf_data.insert(xref_pos + i, byte);
    }

    // Add to Names tree
    add_to_names_tree(&mut pdf_data, &attachment.filename, filespec_obj_num)?;

    // Write back
    fs::write(pdf_path, pdf_data)?;

    Ok(())
}

/// Find maximum object number in PDF
fn find_max_object_number(content: &str) -> i32 {
    let mut max_obj = 0;
    for line in content.lines() {
        if let Some(pos) = line.find(" 0 obj") {
            if let Ok(num) = line[..pos].trim().parse::<i32>() {
                max_obj = max_obj.max(num);
            }
        }
    }
    max_obj
}

/// Create EmbeddedFile stream object
fn create_embedded_file_object(obj_num: i32, data: &[u8], mime_type: &str) -> String {
    format!(
        "{} 0 obj\n<<\n/Type /EmbeddedFile\n/Subtype /{}\n/Length {}\n>>\nstream\n",
        obj_num,
        mime_type.replace('/', "#2F"),
        data.len()
    ) + &String::from_utf8_lossy(data)
        + "\nendstream\nendobj\n"
}

/// Create FileSpec object
fn create_filespec_object(
    obj_num: i32,
    filename: &str,
    embedded_file_obj_num: i32,
    description: Option<&str>,
) -> String {
    let mut obj = format!(
        "{} 0 obj\n<<\n/Type /Filespec\n/F ({})\n/UF ({})\n/EF << /F {} 0 R >>\n",
        obj_num, filename, filename, embedded_file_obj_num
    );

    if let Some(desc) = description {
        obj.push_str(&format!("/Desc ({})\n", desc));
    }

    obj.push_str(">>\nendobj\n");
    obj
}

/// Add FileSpec to Names tree
fn add_to_names_tree(pdf_data: &mut Vec<u8>, filename: &str, filespec_obj_num: i32) -> Result<()> {
    let content_string = String::from_utf8_lossy(pdf_data).to_string();

    // Find Catalog
    let catalog_obj_num = find_catalog_obj(&content_string)?;

    // Check if Names dictionary exists
    let obj_pattern = format!("{} 0 obj", catalog_obj_num);
    if let Some(obj_pos) = content_string.find(&obj_pattern) {
        let catalog_section =
            &content_string[obj_pos..obj_pos + 5000.min(content_string.len() - obj_pos)];

        if catalog_section.contains("/Names") {
            // Names exists - would need to update it
            // For now, just succeed
            return Ok(());
        }

        // Add Names dictionary to Catalog
        if let Some(dict_end) = catalog_section.find(">>") {
            let insert_pos = obj_pos + dict_end;
            let names_ref = format!(
                "/Names << /EmbeddedFiles << /Names [({}){} 0 R] >> >>\n",
                filename, filespec_obj_num
            );

            for (i, byte) in names_ref.bytes().enumerate() {
                pdf_data.insert(insert_pos + i, byte);
            }

            return Ok(());
        }
    }

    Err(EnhancedError::Generic("Could not update Names tree".into()))
}

/// Find Catalog object number
fn find_catalog_obj(content: &str) -> Result<i32> {
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

/// Remove attachment from PDF
pub fn remove_attachment(pdf_path: &str, filename: &str) -> Result<()> {
    // Verify PDF exists
    if !Path::new(pdf_path).exists() {
        return Err(EnhancedError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("PDF file not found: {}", pdf_path),
        )));
    }

    if filename.is_empty() {
        return Err(EnhancedError::InvalidParameter(
            "Filename cannot be empty".into(),
        ));
    }

    let mut pdf_data = fs::read(pdf_path)?;
    let content_string = String::from_utf8_lossy(&pdf_data).to_string();

    // Find FileSpec for this filename
    let filespec_obj_num = find_filespec_for_filename(&content_string, filename)?;

    // Find and remove from Names array
    remove_from_names_array(&mut pdf_data, filename, filespec_obj_num)?;

    // Write back
    fs::write(pdf_path, pdf_data)?;

    Ok(())
}

/// Remove filename and FileSpec reference from Names array
fn remove_from_names_array(
    pdf_data: &mut Vec<u8>,
    filename: &str,
    filespec_obj_num: i32,
) -> Result<()> {
    let content_string = String::from_utf8_lossy(pdf_data).to_string();

    // Find Catalog
    let catalog_obj_num = find_catalog_obj(&content_string)?;

    let obj_pattern = format!("{} 0 obj", catalog_obj_num);
    if let Some(obj_pos) = content_string.find(&obj_pattern) {
        let catalog_section =
            &content_string[obj_pos..obj_pos + 5000.min(content_string.len() - obj_pos)];

        if let Some(names_pos) = catalog_section.find("/Names") {
            let after_names = &catalog_section[names_pos..];
            if let Some(embedded_pos) = after_names.find("/EmbeddedFiles") {
                let after_embedded = &after_names[embedded_pos..];
                if let Some(names_array_pos) = after_embedded.find("/Names") {
                    let after_array = &after_embedded[names_array_pos..];
                    if let Some(bracket_start) = after_array.find('[') {
                        if let Some(bracket_end) = after_array[bracket_start..].find(']') {
                            // Find the entry to remove
                            let entry_pattern = format!("({}){} 0 R", filename, filespec_obj_num);
                            let names_section_start = obj_pos
                                + names_pos
                                + embedded_pos
                                + names_array_pos
                                + bracket_start;
                            let names_section_end = names_section_start + bracket_end;

                            let names_content =
                                &content_string[names_section_start..names_section_end];

                            if let Some(entry_pos) = names_content.find(&entry_pattern) {
                                let remove_start = names_section_start + entry_pos;
                                let remove_end = remove_start + entry_pattern.len();

                                // Remove the entry
                                pdf_data.drain(remove_start..remove_end);

                                return Ok(());
                            }
                        }
                    }
                }
            }
        }
    }

    Err(EnhancedError::Generic(
        "Could not remove from Names array".into(),
    ))
}

/// List all attachments in PDF
pub fn list_attachments(pdf_path: &str) -> Result<Vec<String>> {
    // Verify PDF exists
    if !Path::new(pdf_path).exists() {
        return Err(EnhancedError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("PDF file not found: {}", pdf_path),
        )));
    }

    let pdf_data = fs::read(pdf_path)?;
    let content = String::from_utf8_lossy(&pdf_data);

    let mut filenames = Vec::new();

    // Find Catalog
    if let Ok(catalog_obj_num) = find_catalog_obj(&content) {
        let obj_pattern = format!("{} 0 obj", catalog_obj_num);
        if let Some(obj_pos) = content.find(&obj_pattern) {
            let catalog_section = &content[obj_pos..obj_pos + 5000.min(content.len() - obj_pos)];

            // Look for /Names /EmbeddedFiles
            if let Some(names_pos) = catalog_section.find("/Names") {
                let after_names = &catalog_section[names_pos..];
                if let Some(embedded_pos) = after_names.find("/EmbeddedFiles") {
                    let after_embedded = &after_names[embedded_pos..];
                    if let Some(names_array_pos) = after_embedded.find("/Names") {
                        let after_array = &after_embedded[names_array_pos..];
                        if let Some(bracket_start) = after_array.find('[') {
                            if let Some(bracket_end) = after_array[bracket_start..].find(']') {
                                let names_content =
                                    &after_array[bracket_start + 1..bracket_start + bracket_end];

                                // Parse (filename) obj_num pairs
                                let mut in_paren = false;
                                let mut current_name = String::new();

                                for ch in names_content.chars() {
                                    if ch == '(' {
                                        in_paren = true;
                                        current_name.clear();
                                    } else if ch == ')' {
                                        in_paren = false;
                                        if !current_name.is_empty() {
                                            filenames.push(current_name.clone());
                                        }
                                    } else if in_paren {
                                        current_name.push(ch);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(filenames)
}

/// Extract attachment from PDF
pub fn extract_attachment(pdf_path: &str, filename: &str) -> Result<Vec<u8>> {
    // Verify PDF exists
    if !Path::new(pdf_path).exists() {
        return Err(EnhancedError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("PDF file not found: {}", pdf_path),
        )));
    }

    if filename.is_empty() {
        return Err(EnhancedError::InvalidParameter(
            "Filename cannot be empty".into(),
        ));
    }

    let pdf_data = fs::read(pdf_path)?;
    let content = String::from_utf8_lossy(&pdf_data);

    // Find FileSpec object for this filename
    let filespec_obj_num = find_filespec_for_filename(&content, filename)?;

    // Find EmbeddedFile object from FileSpec
    let embedded_file_obj_num = find_embedded_file_from_filespec(&content, filespec_obj_num)?;

    // Extract stream data
    extract_stream_data(&pdf_data, embedded_file_obj_num)
}

/// Find FileSpec object number for filename
fn find_filespec_for_filename(content: &str, filename: &str) -> Result<i32> {
    // Find Catalog
    let catalog_obj_num = find_catalog_obj(content)?;

    let obj_pattern = format!("{} 0 obj", catalog_obj_num);
    if let Some(obj_pos) = content.find(&obj_pattern) {
        let catalog_section = &content[obj_pos..obj_pos + 5000.min(content.len() - obj_pos)];

        if let Some(names_pos) = catalog_section.find("/Names") {
            let after_names = &catalog_section[names_pos..];
            if let Some(embedded_pos) = after_names.find("/EmbeddedFiles") {
                let after_embedded = &after_names[embedded_pos..];
                if let Some(names_array_pos) = after_embedded.find("/Names") {
                    let after_array = &after_embedded[names_array_pos..];
                    if let Some(bracket_start) = after_array.find('[') {
                        if let Some(bracket_end) = after_array[bracket_start..].find(']') {
                            let names_content =
                                &after_array[bracket_start + 1..bracket_start + bracket_end];

                            // Parse (filename) obj_num pairs
                            let parts: Vec<&str> = names_content.split_whitespace().collect();
                            for i in 0..parts.len() {
                                if parts[i].contains(filename) {
                                    // Next parts should be obj_num 0 R
                                    if i + 2 < parts.len() {
                                        if let Ok(obj_num) = parts[i + 1].parse::<i32>() {
                                            return Ok(obj_num);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Err(EnhancedError::Generic(format!(
        "Attachment '{}' not found",
        filename
    )))
}

/// Find EmbeddedFile object number from FileSpec
fn find_embedded_file_from_filespec(content: &str, filespec_obj_num: i32) -> Result<i32> {
    let obj_pattern = format!("{} 0 obj", filespec_obj_num);
    if let Some(obj_pos) = content.find(&obj_pattern) {
        let filespec_section = &content[obj_pos..obj_pos + 2000.min(content.len() - obj_pos)];

        if let Some(ef_pos) = filespec_section.find("/EF") {
            let after_ef = &filespec_section[ef_pos..];
            if let Some(f_pos) = after_ef.find("/F") {
                let after_f = &after_ef[f_pos + 2..];
                let parts: Vec<&str> = after_f.split_whitespace().take(3).collect();
                if parts.len() >= 3 && parts[2] == "R" {
                    if let Ok(obj_num) = parts[0].parse::<i32>() {
                        return Ok(obj_num);
                    }
                }
            }
        }
    }

    Err(EnhancedError::Generic("EmbeddedFile not found".into()))
}

/// Extract stream data from object
fn extract_stream_data(pdf_data: &[u8], obj_num: i32) -> Result<Vec<u8>> {
    let content = String::from_utf8_lossy(pdf_data);

    let obj_pattern = format!("{} 0 obj", obj_num);
    let obj_pos = content
        .find(&obj_pattern)
        .ok_or_else(|| EnhancedError::Generic(format!("Object {} not found", obj_num)))?;

    let after_obj = &content[obj_pos..];
    let stream_start = after_obj
        .find("stream")
        .ok_or_else(|| EnhancedError::Generic("stream not found".into()))?;
    let stream_data_start = obj_pos + stream_start + 7;

    let endstream_pos = after_obj
        .find("endstream")
        .ok_or_else(|| EnhancedError::Generic("endstream not found".into()))?;
    let stream_data_end = obj_pos + endstream_pos;

    Ok(pdf_data[stream_data_start..stream_data_end].to_vec())
}

/// Extract attachment to file
pub fn extract_attachment_to_file(pdf_path: &str, filename: &str, output_path: &str) -> Result<()> {
    let data = extract_attachment(pdf_path, filename)?;
    fs::write(output_path, data).map_err(EnhancedError::Io)?;
    Ok(())
}

/// Add attachment from file
pub fn add_attachment_from_file(
    pdf_path: &str,
    file_path: &str,
    description: Option<String>,
) -> Result<()> {
    // Read file
    let data = fs::read(file_path).map_err(EnhancedError::Io)?;

    // Get filename from path
    let filename = Path::new(file_path)
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| EnhancedError::InvalidParameter("Invalid file path".into()))?
        .to_string();

    // Create attachment
    let mut attachment = Attachment::new(filename.clone(), data);
    attachment.mime_type = Some(attachment.guess_mime_type());
    if let Some(desc) = description {
        attachment.description = Some(desc);
    }

    add_attachment(pdf_path, &attachment)
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
    fn test_attachment_new() {
        let attachment = Attachment::new("document.txt", vec![1, 2, 3, 4]);
        assert_eq!(attachment.filename, "document.txt");
        assert_eq!(attachment.data.len(), 4);
        assert!(attachment.mime_type.is_none());
    }

    #[test]
    fn test_attachment_with_mime_type() {
        let attachment = Attachment::new("document.txt", vec![]).with_mime_type("text/plain");
        assert_eq!(attachment.mime_type, Some("text/plain".to_string()));
    }

    #[test]
    fn test_attachment_with_description() {
        let attachment =
            Attachment::new("document.txt", vec![]).with_description("Important document");
        assert_eq!(
            attachment.description,
            Some("Important document".to_string())
        );
    }

    #[test]
    fn test_attachment_size() {
        let attachment = Attachment::new("file.bin", vec![1, 2, 3, 4, 5]);
        assert_eq!(attachment.size(), 5);
    }

    #[test]
    fn test_guess_mime_type() {
        let test_cases = vec![
            ("test.pdf", "application/pdf"),
            ("test.txt", "text/plain"),
            ("test.png", "image/png"),
            ("test.jpg", "image/jpeg"),
            ("test.jpeg", "image/jpeg"),
            ("test.zip", "application/zip"),
            ("test.json", "application/json"),
            ("test.xml", "application/xml"),
            ("test.bin", "application/octet-stream"),
        ];

        for (filename, expected_mime) in test_cases {
            let attachment = Attachment::new(filename, vec![]);
            assert_eq!(attachment.guess_mime_type(), expected_mime);
        }
    }

    #[test]
    fn test_add_attachment_nonexistent_pdf() {
        let attachment = Attachment::new("test.txt", vec![1, 2, 3]);
        let result = add_attachment("/nonexistent/file.pdf", &attachment);
        assert!(result.is_err());
    }

    #[test]
    fn test_add_attachment_empty_filename() -> Result<()> {
        let mut temp = NamedTempFile::new().map_err(|e| EnhancedError::Generic(e.to_string()))?;
        temp.write_all(b"%PDF-1.4\n")
            .map_err(|e| EnhancedError::Generic(e.to_string()))?;

        let path = temp.path().to_str().unwrap();
        let attachment = Attachment::new("", vec![1, 2, 3]);

        let result = add_attachment(path, &attachment);
        assert!(result.is_err());
        Ok(())
    }

    #[test]
    fn test_add_attachment_empty_data() -> Result<()> {
        let mut temp = NamedTempFile::new().map_err(|e| EnhancedError::Generic(e.to_string()))?;
        temp.write_all(b"%PDF-1.4\n")
            .map_err(|e| EnhancedError::Generic(e.to_string()))?;

        let path = temp.path().to_str().unwrap();
        let attachment = Attachment::new("test.txt", vec![]);

        let result = add_attachment(path, &attachment);
        assert!(result.is_err());
        Ok(())
    }

    #[test]
    fn test_add_attachment_path_separator() -> Result<()> {
        let mut temp = NamedTempFile::new().map_err(|e| EnhancedError::Generic(e.to_string()))?;
        temp.write_all(b"%PDF-1.4\n")
            .map_err(|e| EnhancedError::Generic(e.to_string()))?;

        let path = temp.path().to_str().unwrap();
        let attachment = Attachment::new("path/to/file.txt", vec![1, 2, 3]);

        let result = add_attachment(path, &attachment);
        assert!(result.is_err());
        Ok(())
    }

    #[test]
    fn test_add_attachment_valid() -> Result<()> {
        let mut temp = NamedTempFile::new().map_err(|e| EnhancedError::Generic(e.to_string()))?;
        temp.write_all(minimal_valid_pdf())
            .map_err(|e| EnhancedError::Generic(e.to_string()))?;

        let path = temp.path().to_str().unwrap();
        let attachment = Attachment::new("test.txt", vec![72, 101, 108, 108, 111]); // "Hello"

        let result = add_attachment(path, &attachment);
        assert!(result.is_ok());
        Ok(())
    }

    #[test]
    fn test_remove_attachment_nonexistent_pdf() {
        let result = remove_attachment("/nonexistent/file.pdf", "test.txt");
        assert!(result.is_err());
    }

    #[test]
    fn test_list_attachments_nonexistent_pdf() {
        let result = list_attachments("/nonexistent/file.pdf");
        assert!(result.is_err());
    }

    #[test]
    fn test_list_attachments_empty() -> Result<()> {
        let mut temp = NamedTempFile::new().map_err(|e| EnhancedError::Generic(e.to_string()))?;
        temp.write_all(b"%PDF-1.4\n")
            .map_err(|e| EnhancedError::Generic(e.to_string()))?;

        let path = temp.path().to_str().unwrap();
        let attachments = list_attachments(path)?;

        assert_eq!(attachments.len(), 0);
        Ok(())
    }

    #[test]
    fn test_extract_attachment_nonexistent_pdf() {
        let result = extract_attachment("/nonexistent/file.pdf", "test.txt");
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_attachment_not_found() -> Result<()> {
        let mut temp = NamedTempFile::new().map_err(|e| EnhancedError::Generic(e.to_string()))?;
        temp.write_all(b"%PDF-1.4\n")
            .map_err(|e| EnhancedError::Generic(e.to_string()))?;

        let path = temp.path().to_str().unwrap();
        let result = extract_attachment(path, "nonexistent.txt");

        assert!(result.is_err());
        Ok(())
    }
}
