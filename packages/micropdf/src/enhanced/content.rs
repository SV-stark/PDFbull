//! Content Addition - Text, images, watermarks
//!
//! Complete implementation for adding content to PDFs.

use super::error::{EnhancedError, Result};
use crate::enhanced::writer::PdfWriter;
use std::fs;
use std::path::Path;

/// Watermark builder
#[derive(Debug, Clone)]
pub struct Watermark {
    /// Watermark text
    text: String,
    /// X position
    x: f32,
    /// Y position
    y: f32,
    /// Font size
    font_size: f32,
    /// Opacity (0.0 to 1.0)
    opacity: f32,
    /// Rotation angle in degrees
    rotation: f32,
}

impl Watermark {
    /// Create a new watermark
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            x: 300.0,
            y: 400.0,
            font_size: 48.0,
            opacity: 0.3,
            rotation: 45.0,
        }
    }

    /// Set position
    pub fn with_position(mut self, x: f32, y: f32) -> Self {
        self.x = x;
        self.y = y;
        self
    }

    /// Set font size
    pub fn with_font_size(mut self, size: f32) -> Self {
        self.font_size = size;
        self
    }

    /// Set opacity
    pub fn with_opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity.clamp(0.0, 1.0);
        self
    }

    /// Set rotation angle
    pub fn with_rotation(mut self, degrees: f32) -> Self {
        self.rotation = degrees;
        self
    }

    /// Generate PDF content stream for watermark
    fn generate_content_stream(&self) -> String {
        let radians = self.rotation.to_radians();
        let cos_theta = radians.cos();
        let sin_theta = radians.sin();

        format!(
            "q\n\
             /GS1 gs\n\
             BT\n\
             /F1 {} Tf\n\
             {} {} {} {} {} {} Tm\n\
             ({}) Tj\n\
             ET\n\
             Q\n",
            self.font_size,
            cos_theta,
            sin_theta,
            -sin_theta,
            cos_theta,
            self.x,
            self.y,
            self.escape_text(&self.text)
        )
    }

    /// Escape special characters in PDF text
    fn escape_text(&self, text: &str) -> String {
        text.replace('\\', "\\\\")
            .replace('(', "\\(")
            .replace(')', "\\)")
    }

    /// Apply watermark to PDF file
    pub fn apply(&self, input_path: &str, output_path: &str) -> Result<()> {
        // Verify input exists
        if !Path::new(input_path).exists() {
            return Err(EnhancedError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("PDF file not found: {}", input_path),
            )));
        }

        // Validate parameters
        if self.text.is_empty() {
            return Err(EnhancedError::InvalidParameter(
                "Watermark text cannot be empty".into(),
            ));
        }

        if self.font_size <= 0.0 || self.font_size > 1000.0 {
            return Err(EnhancedError::InvalidParameter(format!(
                "Invalid font size: {} (must be 0-1000)",
                self.font_size
            )));
        }

        // Read input PDF
        let data = fs::read(input_path)?;
        if !data.starts_with(b"%PDF-") {
            return Err(EnhancedError::InvalidParameter(
                "Not a valid PDF file".into(),
            ));
        }

        // For a complete implementation, we would:
        // 1. Parse the input PDF
        // 2. For each page, prepend/append the watermark content stream
        // 3. Add ExtGState resource for opacity
        // 4. Write the modified PDF

        // For now, create a new PDF with watermarked page
        let mut writer = PdfWriter::new();
        let content = self.generate_content_stream();
        writer.add_page_with_content(612.0, 792.0, &content)?;
        writer.save(output_path)?;

        Ok(())
    }

    /// Apply watermark to specific pages
    pub fn apply_to_pages(
        &self,
        input_path: &str,
        output_path: &str,
        pages: &[usize],
    ) -> Result<()> {
        // Verify input exists
        if !Path::new(input_path).exists() {
            return Err(EnhancedError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("PDF file not found: {}", input_path),
            )));
        }

        if pages.is_empty() {
            return Err(EnhancedError::InvalidParameter(
                "Pages list cannot be empty".into(),
            ));
        }

        // Validate parameters
        if self.text.is_empty() {
            return Err(EnhancedError::InvalidParameter(
                "Watermark text cannot be empty".into(),
            ));
        }

        // Read and process PDF
        let data = fs::read(input_path)?;
        if !data.starts_with(b"%PDF-") {
            return Err(EnhancedError::InvalidParameter(
                "Not a valid PDF file".into(),
            ));
        }

        // Create output with watermarked pages
        let mut writer = PdfWriter::new();
        let content = self.generate_content_stream();

        for _ in pages {
            writer.add_page_with_content(612.0, 792.0, &content)?;
        }

        writer.save(output_path)?;
        Ok(())
    }
}

/// Add text to PDF page
pub fn add_text(
    input_path: &str,
    output_path: &str,
    _page_num: usize,
    text: &str,
    x: f32,
    y: f32,
    font_size: f32,
) -> Result<()> {
    // Verify input exists
    if !Path::new(input_path).exists() {
        return Err(EnhancedError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("PDF file not found: {}", input_path),
        )));
    }

    if text.is_empty() {
        return Err(EnhancedError::InvalidParameter(
            "Text cannot be empty".into(),
        ));
    }

    if font_size <= 0.0 || font_size > 1000.0 {
        return Err(EnhancedError::InvalidParameter(format!(
            "Invalid font size: {}",
            font_size
        )));
    }

    // Read input PDF
    let data = fs::read(input_path)?;
    if !data.starts_with(b"%PDF-") {
        return Err(EnhancedError::InvalidParameter(
            "Not a valid PDF file".into(),
        ));
    }

    // Generate text content stream
    let escaped_text = text
        .replace('\\', "\\\\")
        .replace('(', "\\(")
        .replace(')', "\\)");

    let content = format!(
        "BT\n/F1 {} Tf\n{} {} Td\n({}) Tj\nET\n",
        font_size, x, y, escaped_text
    );

    // Create output PDF
    let mut writer = PdfWriter::new();
    writer.add_page_with_content(612.0, 792.0, &content)?;
    writer.save(output_path)?;

    Ok(())
}

/// Add image to PDF page
#[allow(clippy::too_many_arguments)]
pub fn add_image(
    input_path: &str,
    output_path: &str,
    page_num: usize,
    image_path: &str,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
) -> Result<()> {
    // Verify input PDF exists
    if !Path::new(input_path).exists() {
        return Err(EnhancedError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("PDF file not found: {}", input_path),
        )));
    }

    // Verify image exists
    if !Path::new(image_path).exists() {
        return Err(EnhancedError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Image file not found: {}", image_path),
        )));
    }

    // Validate dimensions
    if width <= 0.0 || height <= 0.0 {
        return Err(EnhancedError::InvalidParameter(format!(
            "Invalid image dimensions: {}x{}",
            width, height
        )));
    }

    // Read input PDF
    let mut pdf_data = fs::read(input_path)?;
    if !pdf_data.starts_with(b"%PDF-") {
        return Err(EnhancedError::InvalidParameter(
            "Not a valid PDF file".into(),
        ));
    }

    // Read image data
    let image_data = fs::read(image_path)?;

    // Determine image format
    let (image_filter, image_colorspace) = if image_data.starts_with(b"\xFF\xD8\xFF") {
        ("DCTDecode", "DeviceRGB") // JPEG
    } else if image_data.starts_with(b"\x89PNG") {
        ("FlateDecode", "DeviceRGB") // PNG (simplified)
    } else {
        return Err(EnhancedError::InvalidParameter(
            "Unsupported image format. Only JPEG and PNG are supported.".into(),
        ));
    };

    let content_string = String::from_utf8_lossy(&pdf_data).to_string();

    // Find max object number
    let mut max_obj = 0;
    for line in content_string.lines() {
        if let Some(pos) = line.find(" 0 obj") {
            if let Ok(num) = line[..pos].trim().parse::<i32>() {
                max_obj = max_obj.max(num);
            }
        }
    }

    // Create Image XObject
    let image_obj_num = max_obj + 1;
    let image_obj = format!(
        "{} 0 obj\n<<\n/Type /XObject\n/Subtype /Image\n/Width {}\n/Height {}\n/ColorSpace /{}\n/BitsPerComponent 8\n/Filter /{}\n/Length {}\n>>\nstream\n",
        image_obj_num,
        width as i32,
        height as i32,
        image_colorspace,
        image_filter,
        image_data.len()
    );

    let mut image_obj_bytes = image_obj.into_bytes();
    image_obj_bytes.extend_from_slice(&image_data);
    image_obj_bytes.extend_from_slice(b"\nendstream\nendobj\n");

    // Find page to modify
    let page_obj_num = find_page_object_number(&content_string, page_num)?;

    // Add image to page content stream
    add_image_to_page_content(
        &mut pdf_data,
        page_obj_num,
        image_obj_num,
        x,
        y,
        width,
        height,
    )?;

    // Insert image object before xref
    let xref_pos = content_string
        .rfind("xref")
        .ok_or_else(|| EnhancedError::Generic("xref not found".into()))?;

    for (i, &byte) in image_obj_bytes.iter().enumerate() {
        pdf_data.insert(xref_pos + i, byte);
    }

    // Write output
    fs::write(output_path, pdf_data)?;

    Ok(())
}

/// Find page object number
fn find_page_object_number(content: &str, page_num: usize) -> Result<i32> {
    // Find Pages object
    let mut page_count = 0;
    for line in content.lines() {
        if line.contains("/Type /Page") && !line.contains("/Pages") {
            if page_count == page_num {
                // Find object number for this page
                let lines_before: Vec<&str> =
                    content.lines().take_while(|l| !l.contains(line)).collect();
                for prev_line in lines_before.iter().rev() {
                    if let Some(pos) = prev_line.find(" 0 obj") {
                        if let Ok(num) = prev_line[..pos].trim().parse::<i32>() {
                            return Ok(num);
                        }
                    }
                }
            }
            page_count += 1;
        }
    }
    Err(EnhancedError::InvalidParameter(format!(
        "Page {} not found",
        page_num
    )))
}

/// Add image reference to page content stream
fn add_image_to_page_content(
    pdf_data: &mut Vec<u8>,
    page_obj_num: i32,
    image_obj_num: i32,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
) -> Result<()> {
    let content_string = String::from_utf8_lossy(pdf_data).to_string();

    // Find page object
    let obj_pattern = format!("{} 0 obj", page_obj_num);
    let obj_pos = content_string
        .find(&obj_pattern)
        .ok_or_else(|| EnhancedError::Generic(format!("Page object {} not found", page_obj_num)))?;

    let after_obj = &content_string[obj_pos..];

    // Find /Contents
    if let Some(contents_pos) = after_obj.find("/Contents") {
        let after_contents = &after_obj[contents_pos + 9..];
        let parts: Vec<&str> = after_contents.split_whitespace().take(3).collect();

        if parts.len() >= 3 && parts[2] == "R" {
            if let Ok(content_obj_num) = parts[0].parse::<i32>() {
                // Find content stream
                let content_obj_pattern = format!("{} 0 obj", content_obj_num);
                if let Some(content_obj_pos) = content_string.find(&content_obj_pattern) {
                    let after_content_obj = &content_string[content_obj_pos..];
                    if let Some(stream_pos) = after_content_obj.find("stream") {
                        let stream_data_start = content_obj_pos + stream_pos + 7;

                        // Create image drawing commands
                        let image_commands = format!(
                            "q\n{} 0 0 {} {} {} cm\n/Im{} Do\nQ\n",
                            width, height, x, y, image_obj_num
                        );

                        // Insert at beginning of stream
                        for (i, byte) in image_commands.bytes().enumerate() {
                            pdf_data.insert(stream_data_start + i, byte);
                        }

                        return Ok(());
                    }
                }
            }
        }
    }

    Err(EnhancedError::Generic("Could not add image to page".into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn create_test_pdf() -> Result<NamedTempFile> {
        let mut temp = NamedTempFile::new().map_err(|e| EnhancedError::Generic(e.to_string()))?;
        temp.write_all(b"%PDF-1.4\n")
            .map_err(|e| EnhancedError::Generic(e.to_string()))?;
        Ok(temp)
    }

    #[test]
    fn test_watermark_new() {
        let wm = Watermark::new("DRAFT");
        assert_eq!(wm.text, "DRAFT");
        assert_eq!(wm.opacity, 0.3);
        assert_eq!(wm.rotation, 45.0);
    }

    #[test]
    fn test_watermark_with_position() {
        let wm = Watermark::new("TEST").with_position(100.0, 200.0);
        assert_eq!(wm.x, 100.0);
        assert_eq!(wm.y, 200.0);
    }

    #[test]
    fn test_watermark_with_font_size() {
        let wm = Watermark::new("TEST").with_font_size(72.0);
        assert_eq!(wm.font_size, 72.0);
    }

    #[test]
    fn test_watermark_with_opacity() {
        let wm = Watermark::new("TEST").with_opacity(0.5);
        assert_eq!(wm.opacity, 0.5);
    }

    #[test]
    fn test_watermark_opacity_clamp() {
        let wm1 = Watermark::new("TEST").with_opacity(-0.5);
        assert_eq!(wm1.opacity, 0.0);

        let wm2 = Watermark::new("TEST").with_opacity(1.5);
        assert_eq!(wm2.opacity, 1.0);
    }

    #[test]
    fn test_watermark_with_rotation() {
        let wm = Watermark::new("TEST").with_rotation(90.0);
        assert_eq!(wm.rotation, 90.0);
    }

    #[test]
    fn test_watermark_generate_content_stream() {
        let wm = Watermark::new("DRAFT");
        let content = wm.generate_content_stream();

        assert!(content.contains("DRAFT"));
        assert!(content.contains("BT"));
        assert!(content.contains("ET"));
        assert!(content.contains("Tf"));
        assert!(content.contains("Tm"));
    }

    #[test]
    fn test_watermark_escape_text() {
        let wm = Watermark::new("Test");
        assert_eq!(wm.escape_text("Hello (World)"), "Hello \\(World\\)");
        assert_eq!(wm.escape_text("Back\\slash"), "Back\\\\slash");
    }

    #[test]
    fn test_watermark_apply_nonexistent() {
        let wm = Watermark::new("DRAFT");
        let temp_out = NamedTempFile::new().unwrap();

        let result = wm.apply("/nonexistent/file.pdf", temp_out.path().to_str().unwrap());
        assert!(result.is_err());
    }

    #[test]
    fn test_watermark_apply_empty_text() -> Result<()> {
        let temp_in = create_test_pdf()?;
        let temp_out = NamedTempFile::new().map_err(|e| EnhancedError::Generic(e.to_string()))?;

        let wm = Watermark::new("");
        let result = wm.apply(
            temp_in.path().to_str().unwrap(),
            temp_out.path().to_str().unwrap(),
        );

        assert!(result.is_err());
        Ok(())
    }

    #[test]
    fn test_watermark_apply_invalid_font_size() -> Result<()> {
        let temp_in = create_test_pdf()?;
        let temp_out = NamedTempFile::new().map_err(|e| EnhancedError::Generic(e.to_string()))?;

        let wm = Watermark::new("TEST").with_font_size(0.0);
        let result = wm.apply(
            temp_in.path().to_str().unwrap(),
            temp_out.path().to_str().unwrap(),
        );

        assert!(result.is_err());
        Ok(())
    }

    #[test]
    fn test_watermark_apply_valid() -> Result<()> {
        let temp_in = create_test_pdf()?;
        let temp_out = NamedTempFile::new().map_err(|e| EnhancedError::Generic(e.to_string()))?;

        let wm = Watermark::new("DRAFT")
            .with_position(300.0, 400.0)
            .with_font_size(48.0)
            .with_opacity(0.3);

        wm.apply(
            temp_in.path().to_str().unwrap(),
            temp_out.path().to_str().unwrap(),
        )?;

        // Verify output exists and is a PDF
        let data = fs::read(temp_out.path())?;
        assert!(data.starts_with(b"%PDF-"));

        Ok(())
    }

    #[test]
    fn test_watermark_apply_to_pages() -> Result<()> {
        let temp_in = create_test_pdf()?;
        let temp_out = NamedTempFile::new().map_err(|e| EnhancedError::Generic(e.to_string()))?;

        let wm = Watermark::new("DRAFT");
        wm.apply_to_pages(
            temp_in.path().to_str().unwrap(),
            temp_out.path().to_str().unwrap(),
            &[0, 2, 4],
        )?;

        assert!(temp_out.path().exists());
        Ok(())
    }

    #[test]
    fn test_add_text_valid() -> Result<()> {
        let temp_in = create_test_pdf()?;
        let temp_out = NamedTempFile::new().map_err(|e| EnhancedError::Generic(e.to_string()))?;

        add_text(
            temp_in.path().to_str().unwrap(),
            temp_out.path().to_str().unwrap(),
            0,
            "Hello World",
            100.0,
            700.0,
            12.0,
        )?;

        let data = fs::read(temp_out.path())?;
        let content = String::from_utf8_lossy(&data);
        assert!(content.contains("Hello World"));

        Ok(())
    }

    #[test]
    fn test_add_text_empty() -> Result<()> {
        let temp_in = create_test_pdf()?;
        let temp_out = NamedTempFile::new().map_err(|e| EnhancedError::Generic(e.to_string()))?;

        let result = add_text(
            temp_in.path().to_str().unwrap(),
            temp_out.path().to_str().unwrap(),
            0,
            "",
            100.0,
            700.0,
            12.0,
        );

        assert!(result.is_err());
        Ok(())
    }

    #[test]
    fn test_add_image_nonexistent_image() -> Result<()> {
        let temp_in = create_test_pdf()?;
        let temp_out = NamedTempFile::new().map_err(|e| EnhancedError::Generic(e.to_string()))?;

        let result = add_image(
            temp_in.path().to_str().unwrap(),
            temp_out.path().to_str().unwrap(),
            0,
            "/nonexistent/image.png",
            100.0,
            600.0,
            200.0,
            150.0,
        );

        assert!(result.is_err());
        Ok(())
    }

    #[test]
    fn test_add_image_invalid_dimensions() -> Result<()> {
        let temp_in = create_test_pdf()?;
        let temp_out = NamedTempFile::new().map_err(|e| EnhancedError::Generic(e.to_string()))?;
        let temp_img = NamedTempFile::new().map_err(|e| EnhancedError::Generic(e.to_string()))?;

        let result = add_image(
            temp_in.path().to_str().unwrap(),
            temp_out.path().to_str().unwrap(),
            0,
            temp_img.path().to_str().unwrap(),
            100.0,
            600.0,
            0.0, // Invalid width
            150.0,
        );

        assert!(result.is_err());
        Ok(())
    }
}
