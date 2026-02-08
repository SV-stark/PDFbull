//! Enhanced Metadata Management

use super::error::{EnhancedError, Result};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// PDF metadata
#[derive(Debug, Clone, Default)]
pub struct Metadata {
    /// Title
    pub title: Option<String>,
    /// Author
    pub author: Option<String>,
    /// Subject
    pub subject: Option<String>,
    /// Keywords
    pub keywords: Option<String>,
    /// Creator
    pub creator: Option<String>,
    /// Producer
    pub producer: Option<String>,
    /// Creation date
    pub creation_date: Option<String>,
    /// Modification date
    pub mod_date: Option<String>,
    /// Custom metadata
    pub custom: HashMap<String, String>,
}

impl Metadata {
    /// Create new metadata
    pub fn new() -> Self {
        Self::default()
    }

    /// Set title
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set author
    pub fn with_author(mut self, author: impl Into<String>) -> Self {
        self.author = Some(author.into());
        self
    }

    /// Set subject
    pub fn with_subject(mut self, subject: impl Into<String>) -> Self {
        self.subject = Some(subject.into());
        self
    }

    /// Set keywords
    pub fn with_keywords(mut self, keywords: impl Into<String>) -> Self {
        self.keywords = Some(keywords.into());
        self
    }

    /// Add custom metadata field
    pub fn add_custom(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.custom.insert(key.into(), value.into());
    }
}

/// Read metadata from PDF
pub fn read_metadata(pdf_path: &str) -> Result<Metadata> {
    // Verify file exists
    if !Path::new(pdf_path).exists() {
        return Err(EnhancedError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("PDF file not found: {}", pdf_path),
        )));
    }

    // For now, return empty metadata with creator set
    // Full implementation would parse PDF Info dictionary
    let mut metadata = Metadata::new();
    metadata.producer = Some("MicroPDF".to_string());

    Ok(metadata)
}

/// Update metadata in PDF
pub fn update_metadata(pdf_path: &str, metadata: &Metadata) -> Result<()> {
    // Verify file exists
    if !Path::new(pdf_path).exists() {
        return Err(EnhancedError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("PDF file not found: {}", pdf_path),
        )));
    }

    // Validate metadata
    if let Some(ref title) = metadata.title {
        if title.len() > 1000 {
            return Err(EnhancedError::InvalidParameter(
                "Title too long (max 1000 characters)".into(),
            ));
        }
    }

    // Read PDF data
    let mut pdf_data = fs::read(pdf_path).map_err(EnhancedError::Io)?;
    let content_string = String::from_utf8_lossy(&pdf_data).to_string();

    // Find Info dictionary in trailer
    if let Some(trailer_pos) = content_string.rfind("trailer") {
        let trailer_section = &content_string[trailer_pos..];

        // Look for /Info reference
        if let Some(info_pos) = trailer_section.find("/Info") {
            let after_info = &trailer_section[info_pos + 5..];
            let parts: Vec<&str> = after_info.split_whitespace().take(3).collect();

            if parts.len() >= 3 && parts[2] == "R" {
                if let Ok(info_obj_num) = parts[0].parse::<i32>() {
                    // Update the Info dictionary
                    update_info_dict(&mut pdf_data, info_obj_num, metadata)?;
                    fs::write(pdf_path, pdf_data).map_err(EnhancedError::Io)?;
                    return Ok(());
                }
            }
        }

        // No Info dictionary - create one
        return create_info_dictionary(&mut pdf_data, pdf_path, metadata);
    }

    Err(EnhancedError::Generic("Could not find trailer".into()))
}

/// Update existing Info dictionary
fn update_info_dict(pdf_data: &mut Vec<u8>, info_obj_num: i32, metadata: &Metadata) -> Result<()> {
    let content_string = String::from_utf8_lossy(pdf_data).to_string();

    let obj_pattern = format!("{} 0 obj", info_obj_num);
    let obj_pos = content_string
        .find(&obj_pattern)
        .ok_or_else(|| EnhancedError::Generic(format!("Info object {} not found", info_obj_num)))?;

    let after_obj = &content_string[obj_pos..];
    let dict_start = after_obj
        .find("<<")
        .ok_or_else(|| EnhancedError::Generic("Info dictionary start not found".into()))?;
    let dict_end = after_obj
        .find(">>")
        .ok_or_else(|| EnhancedError::Generic("Info dictionary end not found".into()))?;

    // Build new dictionary
    let mut new_dict = String::from("<<\n");
    if let Some(ref title) = metadata.title {
        new_dict.push_str(&format!("/Title ({})\n", escape_string(title)));
    }
    if let Some(ref author) = metadata.author {
        new_dict.push_str(&format!("/Author ({})\n", escape_string(author)));
    }
    if let Some(ref subject) = metadata.subject {
        new_dict.push_str(&format!("/Subject ({})\n", escape_string(subject)));
    }
    if let Some(ref keywords) = metadata.keywords {
        new_dict.push_str(&format!("/Keywords ({})\n", escape_string(keywords)));
    }
    if let Some(ref creator) = metadata.creator {
        new_dict.push_str(&format!("/Creator ({})\n", escape_string(creator)));
    }
    if let Some(ref producer) = metadata.producer {
        new_dict.push_str(&format!("/Producer ({})\n", escape_string(producer)));
    }
    new_dict.push_str(">>");

    // Replace dictionary
    let replace_start = obj_pos + dict_start;
    let replace_end = obj_pos + dict_end + 2;
    pdf_data.splice(replace_start..replace_end, new_dict.bytes());

    Ok(())
}

/// Create new Info dictionary
fn create_info_dictionary(
    pdf_data: &mut Vec<u8>,
    pdf_path: &str,
    metadata: &Metadata,
) -> Result<()> {
    let content_string = String::from_utf8_lossy(pdf_data).to_string();

    // Find max object number
    let mut max_obj = 0;
    for line in content_string.lines() {
        if let Some(pos) = line.find(" 0 obj") {
            if let Ok(num) = line[..pos].trim().parse::<i32>() {
                max_obj = max_obj.max(num);
            }
        }
    }

    let info_obj_num = max_obj + 1;

    // Create Info object
    let mut info_obj = format!("{} 0 obj\n<<\n", info_obj_num);
    if let Some(ref title) = metadata.title {
        info_obj.push_str(&format!("/Title ({})\n", escape_string(title)));
    }
    if let Some(ref author) = metadata.author {
        info_obj.push_str(&format!("/Author ({})\n", escape_string(author)));
    }
    if let Some(ref subject) = metadata.subject {
        info_obj.push_str(&format!("/Subject ({})\n", escape_string(subject)));
    }
    if let Some(ref keywords) = metadata.keywords {
        info_obj.push_str(&format!("/Keywords ({})\n", escape_string(keywords)));
    }
    if let Some(ref creator) = metadata.creator {
        info_obj.push_str(&format!("/Creator ({})\n", escape_string(creator)));
    }
    if let Some(ref producer) = metadata.producer {
        info_obj.push_str(&format!("/Producer ({})\n", escape_string(producer)));
    }
    info_obj.push_str(">>\nendobj\n");

    // Insert before xref
    let xref_pos = content_string
        .rfind("xref")
        .ok_or_else(|| EnhancedError::Generic("xref not found".into()))?;

    for (i, byte) in info_obj.bytes().enumerate() {
        pdf_data.insert(xref_pos + i, byte);
    }

    // Add /Info to trailer
    add_info_to_trailer(pdf_data, info_obj_num)?;
    fs::write(pdf_path, pdf_data).map_err(EnhancedError::Io)?;

    Ok(())
}

/// Add /Info reference to trailer
fn add_info_to_trailer(pdf_data: &mut Vec<u8>, info_obj_num: i32) -> Result<()> {
    let content_string = String::from_utf8_lossy(pdf_data).to_string();

    if let Some(trailer_pos) = content_string.rfind("trailer") {
        let after_trailer = &content_string[trailer_pos..];
        if let Some(dict_start) = after_trailer.find("<<") {
            let insert_pos = trailer_pos + dict_start + 2; // After <<
            let info_ref = format!("\n/Info {} 0 R", info_obj_num);

            for (i, byte) in info_ref.bytes().enumerate() {
                pdf_data.insert(insert_pos + i, byte);
            }

            return Ok(());
        }
    }

    Err(EnhancedError::Generic("Could not update trailer".into()))
}

/// Escape PDF string
fn escape_string(s: &str) -> String {
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
                for byte in ch.to_string().bytes() {
                    result.push_str(&format!("\\{:03o}", byte));
                }
            }
        }
    }
    result
}

/// Read XMP metadata
pub fn read_xmp_metadata(pdf_path: &str) -> Result<String> {
    // Verify file exists
    if !Path::new(pdf_path).exists() {
        return Err(EnhancedError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("PDF file not found: {}", pdf_path),
        )));
    }

    // XMP is XML-based metadata stored in PDF Metadata stream
    // Full implementation would:
    // 1. Open PDF
    // 2. Find Metadata stream in Catalog
    // 3. Extract and decode XML

    // Return empty XMP for now
    Ok(String::from(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/">
    <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
    </rdf:RDF>
</x:xmpmeta>"#,
    ))
}

/// Update XMP metadata
pub fn update_xmp_metadata(pdf_path: &str, xmp: &str) -> Result<()> {
    // Verify file exists
    if !Path::new(pdf_path).exists() {
        return Err(EnhancedError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("PDF file not found: {}", pdf_path),
        )));
    }

    // Validate XMP
    if !xmp.contains("<?xml") {
        return Err(EnhancedError::InvalidParameter(
            "XMP must be valid XML starting with <?xml declaration".into(),
        ));
    }

    if !xmp.contains("xmpmeta") {
        return Err(EnhancedError::InvalidParameter(
            "XMP must contain xmpmeta element".into(),
        ));
    }

    // Read PDF
    let mut pdf_data = fs::read(pdf_path).map_err(EnhancedError::Io)?;
    let content_string = String::from_utf8_lossy(&pdf_data).to_string();

    // Find Catalog
    let catalog_obj_num = find_catalog(&content_string)?;

    // Check if Metadata stream exists
    let obj_pattern = format!("{} 0 obj", catalog_obj_num);
    if let Some(obj_pos) = content_string.find(&obj_pattern) {
        let catalog_section =
            &content_string[obj_pos..obj_pos + 5000.min(content_string.len() - obj_pos)];

        if let Some(metadata_pos) = catalog_section.find("/Metadata") {
            // Update existing Metadata stream
            let after_metadata = &catalog_section[metadata_pos + 9..];
            let parts: Vec<&str> = after_metadata.split_whitespace().take(3).collect();

            if parts.len() >= 3 && parts[2] == "R" {
                if let Ok(metadata_obj_num) = parts[0].parse::<i32>() {
                    update_xmp_stream(&mut pdf_data, metadata_obj_num, xmp)?;
                    fs::write(pdf_path, pdf_data).map_err(EnhancedError::Io)?;
                    return Ok(());
                }
            }
        }

        // No Metadata stream - create one
        return create_xmp_stream(&mut pdf_data, pdf_path, catalog_obj_num, xmp);
    }

    Err(EnhancedError::Generic("Could not find Catalog".into()))
}

/// Find Catalog object number
fn find_catalog(content: &str) -> Result<i32> {
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

/// Update existing XMP Metadata stream
fn update_xmp_stream(pdf_data: &mut Vec<u8>, metadata_obj_num: i32, xmp: &str) -> Result<()> {
    let content_string = String::from_utf8_lossy(pdf_data).to_string();

    let obj_pattern = format!("{} 0 obj", metadata_obj_num);
    let obj_pos = content_string.find(&obj_pattern).ok_or_else(|| {
        EnhancedError::Generic(format!("Metadata object {} not found", metadata_obj_num))
    })?;

    let after_obj = &content_string[obj_pos..];
    let stream_start = after_obj
        .find("stream")
        .ok_or_else(|| EnhancedError::Generic("stream not found".into()))?;
    let stream_data_start = obj_pos + stream_start + 7;

    let endstream_pos = after_obj
        .find("endstream")
        .ok_or_else(|| EnhancedError::Generic("endstream not found".into()))?;
    let stream_data_end = obj_pos + endstream_pos;

    // Replace stream data
    pdf_data.splice(stream_data_start..stream_data_end, xmp.bytes());

    // Update Length
    let dict_start = after_obj
        .find("<<")
        .ok_or_else(|| EnhancedError::Generic("dictionary not found".into()))?;
    let dict_section = &after_obj[dict_start..stream_start];

    if let Some(length_pos) = dict_section.find("/Length") {
        let after_length = &dict_section[length_pos + 7..];
        let parts: Vec<&str> = after_length.split_whitespace().take(1).collect();

        if let Some(old_len_str) = parts.first() {
            let new_len = xmp.len().to_string();
            let len_abs_pos = obj_pos + dict_start + length_pos + 7;
            let len_end = len_abs_pos + old_len_str.len();

            pdf_data.splice(len_abs_pos..len_end, new_len.bytes());
        }
    }

    Ok(())
}

/// Create new XMP Metadata stream
fn create_xmp_stream(
    pdf_data: &mut Vec<u8>,
    pdf_path: &str,
    catalog_obj_num: i32,
    xmp: &str,
) -> Result<()> {
    let content_string = String::from_utf8_lossy(pdf_data).to_string();

    // Find max object number
    let mut max_obj = 0;
    for line in content_string.lines() {
        if let Some(pos) = line.find(" 0 obj") {
            if let Ok(num) = line[..pos].trim().parse::<i32>() {
                max_obj = max_obj.max(num);
            }
        }
    }

    let metadata_obj_num = max_obj + 1;

    // Create Metadata stream object
    let metadata_obj = format!(
        "{} 0 obj\n<<\n/Type /Metadata\n/Subtype /XML\n/Length {}\n>>\nstream\n{}endstream\nendobj\n",
        metadata_obj_num,
        xmp.len(),
        xmp
    );

    // Insert before xref
    let xref_pos = content_string
        .rfind("xref")
        .ok_or_else(|| EnhancedError::Generic("xref not found".into()))?;

    for (i, byte) in metadata_obj.bytes().enumerate() {
        pdf_data.insert(xref_pos + i, byte);
    }

    // Add /Metadata to Catalog
    add_metadata_to_catalog(pdf_data, catalog_obj_num, metadata_obj_num)?;
    fs::write(pdf_path, pdf_data).map_err(EnhancedError::Io)?;

    Ok(())
}

/// Add /Metadata reference to Catalog
fn add_metadata_to_catalog(
    pdf_data: &mut Vec<u8>,
    catalog_obj_num: i32,
    metadata_obj_num: i32,
) -> Result<()> {
    let content_string = String::from_utf8_lossy(pdf_data).to_string();

    let obj_pattern = format!("{} 0 obj", catalog_obj_num);
    if let Some(obj_pos) = content_string.find(&obj_pattern) {
        let after_obj = &content_string[obj_pos..];
        if let Some(dict_end) = after_obj.find(">>") {
            let insert_pos = obj_pos + dict_end;
            let metadata_ref = format!("/Metadata {} 0 R\n", metadata_obj_num);

            for (i, byte) in metadata_ref.bytes().enumerate() {
                pdf_data.insert(insert_pos + i, byte);
            }

            return Ok(());
        }
    }

    Err(EnhancedError::Generic("Could not update Catalog".into()))
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
    fn test_metadata_new() {
        let metadata = Metadata::new();
        assert!(metadata.title.is_none());
        assert!(metadata.author.is_none());
        assert!(metadata.custom.is_empty());
    }

    #[test]
    fn test_metadata_with_title() {
        let metadata = Metadata::new().with_title("Test Document");
        assert_eq!(metadata.title, Some("Test Document".to_string()));
    }

    #[test]
    fn test_metadata_with_author() {
        let metadata = Metadata::new().with_author("John Doe");
        assert_eq!(metadata.author, Some("John Doe".to_string()));
    }

    #[test]
    fn test_metadata_add_custom() {
        let mut metadata = Metadata::new();
        metadata.add_custom("Department", "Engineering");
        assert_eq!(
            metadata.custom.get("Department"),
            Some(&"Engineering".to_string())
        );
    }

    #[test]
    fn test_metadata_builder() {
        let metadata = Metadata::new()
            .with_title("Title")
            .with_author("Author")
            .with_subject("Subject")
            .with_keywords("rust, pdf");

        assert!(metadata.title.is_some());
        assert!(metadata.author.is_some());
        assert!(metadata.subject.is_some());
        assert!(metadata.keywords.is_some());
    }

    #[test]
    fn test_read_metadata_nonexistent() {
        let result = read_metadata("/nonexistent/file.pdf");
        assert!(result.is_err());
    }

    #[test]
    fn test_read_metadata_empty_file() -> Result<()> {
        let mut temp = NamedTempFile::new().map_err(|e| EnhancedError::Generic(e.to_string()))?;
        temp.write_all(b"%PDF-1.4\n")
            .map_err(|e| EnhancedError::Generic(e.to_string()))?;

        let path = temp.path().to_str().unwrap();
        let metadata = read_metadata(path)?;

        // Should have default producer
        assert!(metadata.producer.is_some());
        Ok(())
    }

    #[test]
    fn test_update_metadata_nonexistent() {
        let metadata = Metadata::new();
        let result = update_metadata("/nonexistent/file.pdf", &metadata);
        assert!(result.is_err());
    }

    #[test]
    fn test_update_metadata_title_too_long() -> Result<()> {
        let mut temp = NamedTempFile::new().map_err(|e| EnhancedError::Generic(e.to_string()))?;
        temp.write_all(b"%PDF-1.4\n")
            .map_err(|e| EnhancedError::Generic(e.to_string()))?;

        let path = temp.path().to_str().unwrap();
        let metadata = Metadata::new().with_title("x".repeat(1001));

        let result = update_metadata(path, &metadata);
        assert!(result.is_err());
        Ok(())
    }

    #[test]
    fn test_read_xmp_metadata() -> Result<()> {
        let mut temp = NamedTempFile::new().map_err(|e| EnhancedError::Generic(e.to_string()))?;
        temp.write_all(b"%PDF-1.4\n")
            .map_err(|e| EnhancedError::Generic(e.to_string()))?;

        let path = temp.path().to_str().unwrap();
        let xmp = read_xmp_metadata(path)?;

        assert!(xmp.contains("<?xml"));
        assert!(xmp.contains("xmpmeta"));
        Ok(())
    }

    #[test]
    fn test_update_xmp_invalid_xml() -> Result<()> {
        let mut temp = NamedTempFile::new().map_err(|e| EnhancedError::Generic(e.to_string()))?;
        temp.write_all(b"%PDF-1.4\n")
            .map_err(|e| EnhancedError::Generic(e.to_string()))?;

        let path = temp.path().to_str().unwrap();
        let result = update_xmp_metadata(path, "not xml");

        assert!(result.is_err());
        Ok(())
    }

    #[test]
    fn test_update_xmp_valid() -> Result<()> {
        let mut temp = NamedTempFile::new().map_err(|e| EnhancedError::Generic(e.to_string()))?;
        temp.write_all(minimal_valid_pdf())
            .map_err(|e| EnhancedError::Generic(e.to_string()))?;

        let path = temp.path().to_str().unwrap();
        let xmp = r#"<?xml version="1.0"?><x:xmpmeta/>"#;

        let result = update_xmp_metadata(path, xmp);
        // Should succeed (even though not fully implemented yet)
        assert!(result.is_ok());
        Ok(())
    }
}
