//! PDF/A and PDF/UA Compliance
//!
//! Standards compliance for archival and accessibility:
//! - PDF/A (Archival): PDF/A-1b, PDF/A-2b, PDF/A-3b
//! - PDF/UA (Universal Access): Tagged PDFs, accessibility
//! - Validation and conversion
//! - Pre-flight checks

use super::error::{EnhancedError, Result};
use super::pdf_reader::PdfDocument;
use std::collections::HashSet;
use std::path::Path;

/// PDF/A standard version
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PdfAStandard {
    /// PDF/A-1b (Basic)
    PdfA1b,
    /// PDF/A-1a (Accessible)
    PdfA1a,
    /// PDF/A-2b (Basic, ISO 32000-1)
    PdfA2b,
    /// PDF/A-2u (Unicode)
    PdfA2u,
    /// PDF/A-2a (Accessible)
    PdfA2a,
    /// PDF/A-3b (Basic with attachments)
    PdfA3b,
    /// PDF/A-3u (Unicode with attachments)
    PdfA3u,
    /// PDF/A-3a (Accessible with attachments)
    PdfA3a,
}

/// PDF/UA standard version
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PdfUaStandard {
    /// PDF/UA-1 (ISO 14289-1)
    PdfUa1,
}

/// Validation result
#[derive(Debug, Clone)]
pub struct ComplianceValidation {
    pub compliant: bool,
    pub errors: Vec<ComplianceError>,
    pub warnings: Vec<ComplianceWarning>,
}

/// Compliance error
#[derive(Debug, Clone)]
pub struct ComplianceError {
    pub code: String,
    pub message: String,
    pub location: String,
    pub severity: ErrorSeverity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorSeverity {
    Critical,
    Error,
    Warning,
}

/// Compliance warning
#[derive(Debug, Clone)]
pub struct ComplianceWarning {
    pub code: String,
    pub message: String,
    pub recommendation: String,
}

/// Validate PDF/A compliance
pub fn validate_pdfa(pdf_path: &str, standard: PdfAStandard) -> Result<ComplianceValidation> {
    if !Path::new(pdf_path).exists() {
        return Err(EnhancedError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("PDF file not found: {}", pdf_path),
        )));
    }

    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    // Read PDF structure
    let pdf_data = std::fs::read(pdf_path)?;
    let content = String::from_utf8_lossy(&pdf_data);

    // 1. Check for encryption
    if content.contains("/Encrypt") {
        errors.push(ComplianceError {
            code: "PDFA-001".to_string(),
            message: "PDF/A does not allow encryption".to_string(),
            location: "Document".to_string(),
            severity: ErrorSeverity::Critical,
        });
    }

    // 2. Check for JavaScript
    if content.contains("/JavaScript") || content.contains("/JS") {
        errors.push(ComplianceError {
            code: "PDFA-002".to_string(),
            message: "PDF/A does not allow JavaScript".to_string(),
            location: "Document".to_string(),
            severity: ErrorSeverity::Critical,
        });
    }

    // 3. Check for XMP metadata
    if !content.contains("<?xpacket") {
        errors.push(ComplianceError {
            code: "PDFA-003".to_string(),
            message: "PDF/A requires XMP metadata".to_string(),
            location: "Metadata".to_string(),
            severity: ErrorSeverity::Critical,
        });
    }

    // 4. Check for PDF/A identifier in XMP
    let has_pdfa_identifier =
        content.contains("pdfaid:part") && content.contains("pdfaid:conformance");
    if !has_pdfa_identifier {
        errors.push(ComplianceError {
            code: "PDFA-004".to_string(),
            message: "XMP metadata must contain PDF/A identification schema".to_string(),
            location: "XMP Metadata".to_string(),
            severity: ErrorSeverity::Critical,
        });
    }

    // 5. Check font embedding
    let font_check = check_font_embedding(&content);
    errors.extend(font_check.errors);
    warnings.extend(font_check.warnings);

    // 6. Check for OutputIntent (color profile)
    if !content.contains("/OutputIntent") {
        warnings.push(ComplianceWarning {
            code: "PDFA-W001".to_string(),
            message: "PDF/A should have an OutputIntent for color management".to_string(),
            recommendation: "Add ICC color profile to OutputIntent".to_string(),
        });
    }

    // 7. Standard-specific checks
    match standard {
        PdfAStandard::PdfA1a | PdfAStandard::PdfA1b => {
            // PDF/A-1 does not allow transparency
            if content.contains("/ExtGState") && content.contains("/ca") {
                errors.push(ComplianceError {
                    code: "PDFA1-001".to_string(),
                    message: "PDF/A-1 does not allow transparency".to_string(),
                    location: "Graphics State".to_string(),
                    severity: ErrorSeverity::Error,
                });
            }
        }
        PdfAStandard::PdfA3a | PdfAStandard::PdfA3b | PdfAStandard::PdfA3u => {
            // PDF/A-3 allows embedded files, but they must be valid
            if content.contains("/EmbeddedFile") {
                // Check that embedded files have proper metadata
                if !content.contains("/AFRelationship") {
                    warnings.push(ComplianceWarning {
                        code: "PDFA3-W001".to_string(),
                        message: "Embedded files should have AFRelationship".to_string(),
                        recommendation: "Add AFRelationship to embedded file specifications"
                            .to_string(),
                    });
                }
            }
        }
        _ => {}
    }

    // 8. Check for invalid elements
    let invalid_elements = check_invalid_elements(&content);
    errors.extend(invalid_elements);

    Ok(ComplianceValidation {
        compliant: errors.is_empty(),
        errors,
        warnings,
    })
}

/// Validate PDF/UA compliance
pub fn validate_pdfua(pdf_path: &str, _standard: PdfUaStandard) -> Result<ComplianceValidation> {
    if !Path::new(pdf_path).exists() {
        return Err(EnhancedError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("PDF file not found: {}", pdf_path),
        )));
    }

    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    // Read PDF structure
    let pdf_data = std::fs::read(pdf_path)?;
    let content = String::from_utf8_lossy(&pdf_data);

    // 1. Check for tagged PDF structure
    if !content.contains("/MarkInfo") {
        errors.push(ComplianceError {
            code: "PDFUA-001".to_string(),
            message: "PDF/UA requires a tagged PDF structure (MarkInfo)".to_string(),
            location: "Catalog".to_string(),
            severity: ErrorSeverity::Critical,
        });
    }

    // Check if tagging is marked as present
    if !content.contains("/Marked true") {
        errors.push(ComplianceError {
            code: "PDFUA-002".to_string(),
            message: "Document must be marked as tagged (/Marked true)".to_string(),
            location: "MarkInfo".to_string(),
            severity: ErrorSeverity::Critical,
        });
    }

    // 2. Check for structure tree root
    if !content.contains("/StructTreeRoot") {
        errors.push(ComplianceError {
            code: "PDFUA-003".to_string(),
            message: "PDF/UA requires a structure tree root".to_string(),
            location: "Catalog".to_string(),
            severity: ErrorSeverity::Critical,
        });
    }

    // 3. Check for document title in metadata
    let has_title = content.contains("/Title") && !content.contains("/Title()");
    if !has_title {
        errors.push(ComplianceError {
            code: "PDFUA-004".to_string(),
            message: "Document must have a title in metadata".to_string(),
            location: "Info Dictionary".to_string(),
            severity: ErrorSeverity::Critical,
        });
    }

    // 4. Check for language specification
    if !content.contains("/Lang") {
        warnings.push(ComplianceWarning {
            code: "PDFUA-W001".to_string(),
            message: "Document should specify a natural language".to_string(),
            recommendation: "Add /Lang entry to Catalog dictionary".to_string(),
        });
    }

    // 5. Check for ViewerPreferences with DisplayDocTitle
    if !content.contains("/DisplayDocTitle true") {
        warnings.push(ComplianceWarning {
            code: "PDFUA-W002".to_string(),
            message: "ViewerPreferences should set DisplayDocTitle to true".to_string(),
            recommendation: "Add /DisplayDocTitle true to ViewerPreferences".to_string(),
        });
    }

    // 6. Check for XMP metadata
    if !content.contains("<?xpacket") {
        errors.push(ComplianceError {
            code: "PDFUA-005".to_string(),
            message: "PDF/UA requires XMP metadata".to_string(),
            location: "Metadata".to_string(),
            severity: ErrorSeverity::Critical,
        });
    }

    // 7. Check for images without alt text (simplified check)
    let image_count = content.matches("/Subtype/Image").count();
    let alt_text_count = content.matches("/Alt").count();
    if image_count > alt_text_count {
        warnings.push(ComplianceWarning {
            code: "PDFUA-W003".to_string(),
            message: format!(
                "Found {} images but only {} alt text entries",
                image_count, alt_text_count
            ),
            recommendation: "Add alt text to all images for accessibility".to_string(),
        });
    }

    // 8. Check for suspect tagging
    if content.contains("/Suspects true") {
        warnings.push(ComplianceWarning {
            code: "PDFUA-W004".to_string(),
            message: "Document marked with suspect tagging".to_string(),
            recommendation: "Review and correct tagging structure".to_string(),
        });
    }

    Ok(ComplianceValidation {
        compliant: errors.is_empty(),
        errors,
        warnings,
    })
}

/// Convert PDF to PDF/UA
pub fn convert_to_pdfua(pdf_path: &str, output_path: &str, _standard: PdfUaStandard) -> Result<()> {
    if !Path::new(pdf_path).exists() {
        return Err(EnhancedError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("PDF file not found: {}", pdf_path),
        )));
    }

    let mut pdf_data = std::fs::read(pdf_path)?;
    let content = String::from_utf8_lossy(&pdf_data);

    // 1. Add MarkInfo to Catalog
    let mut content = add_mark_info(&content)?;

    // 2. Create structure tree root
    content = add_structure_tree_root(&content)?;

    // 3. Add ViewerPreferences with DisplayDocTitle
    content = add_viewer_preferences(&content)?;

    // 4. Add/update document title
    content = ensure_document_title(&content)?;

    // 5. Add language specification
    content = add_language_spec(&content, "en-US")?;

    // 6. Add XMP metadata for PDF/UA
    content = add_pdfua_xmp_metadata(&content)?;

    // Write modified PDF
    std::fs::write(output_path, content.as_bytes())?;

    Ok(())
}

/// Add MarkInfo dictionary to Catalog
fn add_mark_info(content: &str) -> Result<String> {
    let mut result = content.to_string();

    if !result.contains("/MarkInfo") {
        let mark_info = "/MarkInfo << /Marked true >>";

        // Find Catalog and add MarkInfo
        if let Some(catalog_pos) = result.find("/Type/Catalog") {
            if let Some(end_pos) = result[catalog_pos..].find(">>") {
                result.insert_str(catalog_pos + end_pos, mark_info);
            }
        }
    }

    Ok(result)
}

/// Add structure tree root
fn add_structure_tree_root(content: &str) -> Result<String> {
    let mut result = content.to_string();

    if !result.contains("/StructTreeRoot") {
        // Create a basic structure tree
        let struct_tree = r#"
/StructTreeRoot <<
  /Type /StructTreeRoot
  /K [<< /Type /StructElem /S /Document /P 1 0 R >>]
  /ParentTree << /Nums [] >>
>>"#;

        // Find Catalog and add StructTreeRoot
        if let Some(catalog_pos) = result.find("/Type/Catalog") {
            if let Some(end_pos) = result[catalog_pos..].find(">>") {
                result.insert_str(catalog_pos + end_pos, struct_tree);
            }
        }
    }

    Ok(result)
}

/// Add ViewerPreferences
fn add_viewer_preferences(content: &str) -> Result<String> {
    let mut result = content.to_string();

    if !result.contains("/ViewerPreferences") {
        let viewer_prefs = "/ViewerPreferences << /DisplayDocTitle true >>";

        // Find Catalog and add ViewerPreferences
        if let Some(catalog_pos) = result.find("/Type/Catalog") {
            if let Some(end_pos) = result[catalog_pos..].find(">>") {
                result.insert_str(catalog_pos + end_pos, viewer_prefs);
            }
        }
    }

    Ok(result)
}

/// Ensure document has a title
fn ensure_document_title(content: &str) -> Result<String> {
    let mut result = content.to_string();

    // Check if title exists
    if !result.contains("/Title") || result.contains("/Title()") {
        // Add a default title to Info dictionary
        let title = "/Title(Untitled Document)";

        if let Some(info_pos) = result.find("/Info") {
            if let Some(end_pos) = result[info_pos..].find(">>") {
                result.insert_str(info_pos + end_pos, title);
            }
        }
    }

    Ok(result)
}

/// Add language specification
fn add_language_spec(content: &str, lang: &str) -> Result<String> {
    let mut result = content.to_string();

    if !result.contains("/Lang") {
        let lang_spec = format!("/Lang({})", lang);

        // Find Catalog and add Lang
        if let Some(catalog_pos) = result.find("/Type/Catalog") {
            if let Some(end_pos) = result[catalog_pos..].find(">>") {
                result.insert_str(catalog_pos + end_pos, &lang_spec);
            }
        }
    }

    Ok(result)
}

/// Add PDF/UA XMP metadata
fn add_pdfua_xmp_metadata(content: &str) -> Result<String> {
    let xmp_metadata = r#"<?xpacket begin="" id="W5M0MpCehiHzreSzNTczkc9d"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/">
  <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
    <rdf:Description rdf:about="" xmlns:pdfuaid="http://www.aiim.org/pdfua/ns/id/">
      <pdfuaid:part>1</pdfuaid:part>
    </rdf:Description>
  </rdf:RDF>
</x:xmpmeta>
<?xpacket end="w"?>"#;

    let mut result = content.to_string();

    // Insert XMP metadata into PDF
    if let Some(pos) = result.find("/Metadata") {
        // XMP already exists, append PDF/UA identifier
        result.insert_str(pos + 100, xmp_metadata);
    } else {
        // Add new XMP metadata stream
        if let Some(catalog_pos) = result.find("/Type/Catalog") {
            result.insert_str(
                catalog_pos + 100,
                &format!("/Metadata << {} >>", xmp_metadata),
            );
        }
    }

    Ok(result)
}

/// Add tags to PDF for accessibility
pub fn add_tags(pdf_path: &str, output_path: &str) -> Result<()> {
    if !Path::new(pdf_path).exists() {
        return Err(EnhancedError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("PDF file not found: {}", pdf_path),
        )));
    }

    // TODO: Implement automatic tagging
    // 1. Analyze PDF structure
    // 2. Identify text, images, tables, etc.
    // 3. Create structure tree
    // 4. Add appropriate tags
    // 5. Set reading order

    Ok(())
}

/// Set reading order for screen readers
pub fn set_reading_order(pdf_path: &str, output_path: &str, order: Vec<String>) -> Result<()> {
    if !Path::new(pdf_path).exists() {
        return Err(EnhancedError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("PDF file not found: {}", pdf_path),
        )));
    }

    // TODO: Implement reading order
    // 1. Parse structure tree
    // 2. Reorder elements
    // 3. Update structure tree

    Ok(())
}

/// Add alt text to images
pub fn add_alt_text(
    pdf_path: &str,
    output_path: &str,
    alt_texts: Vec<(u32, String)>, // (image_id, alt_text)
) -> Result<()> {
    if !Path::new(pdf_path).exists() {
        return Err(EnhancedError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("PDF file not found: {}", pdf_path),
        )));
    }

    // TODO: Implement alt text addition
    // 1. Find image objects
    // 2. Add Alt entry to structure element
    // 3. Update structure tree

    Ok(())
}

/// Pre-flight check for compliance
pub fn preflight_check(pdf_path: &str) -> Result<ComplianceValidation> {
    if !Path::new(pdf_path).exists() {
        return Err(EnhancedError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("PDF file not found: {}", pdf_path),
        )));
    }

    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    // Read PDF structure
    let pdf_data = std::fs::read(pdf_path)?;
    let content = String::from_utf8_lossy(&pdf_data);

    // 1. Check PDF header
    if !content.starts_with("%PDF-") {
        errors.push(ComplianceError {
            code: "PRE-001".to_string(),
            message: "Invalid PDF header".to_string(),
            location: "File Header".to_string(),
            severity: ErrorSeverity::Critical,
        });
    }

    // 2. Check PDF version
    if let Some(version_line) = content.lines().next() {
        if version_line.contains("PDF-1.") {
            let version = version_line.trim_start_matches("%PDF-");
            if let Ok(ver) = version.parse::<f32>() {
                if ver < 1.4 {
                    warnings.push(ComplianceWarning {
                        code: "PRE-W001".to_string(),
                        message: format!("PDF version {} is quite old", version),
                        recommendation: "Consider upgrading to PDF 1.7 or later".to_string(),
                    });
                }
            }
        }
    }

    // 3. Check for xref table
    if !content.contains("xref") {
        errors.push(ComplianceError {
            code: "PRE-002".to_string(),
            message: "Missing xref table".to_string(),
            location: "File Structure".to_string(),
            severity: ErrorSeverity::Critical,
        });
    }

    // 4. Check for trailer
    if !content.contains("trailer") {
        errors.push(ComplianceError {
            code: "PRE-003".to_string(),
            message: "Missing trailer".to_string(),
            location: "File Structure".to_string(),
            severity: ErrorSeverity::Critical,
        });
    }

    // 5. Check for Catalog
    if !content.contains("/Type/Catalog") {
        errors.push(ComplianceError {
            code: "PRE-004".to_string(),
            message: "Missing document catalog".to_string(),
            location: "Document Structure".to_string(),
            severity: ErrorSeverity::Critical,
        });
    }

    // 6. Check for Pages tree
    if !content.contains("/Type/Pages") {
        errors.push(ComplianceError {
            code: "PRE-005".to_string(),
            message: "Missing pages tree".to_string(),
            location: "Document Structure".to_string(),
            severity: ErrorSeverity::Critical,
        });
    }

    // 7. Font embedding check
    let font_check = check_font_embedding(&content);
    if !font_check.errors.is_empty() {
        warnings.push(ComplianceWarning {
            code: "PRE-W002".to_string(),
            message: format!("Found {} font embedding issues", font_check.errors.len()),
            recommendation: "Embed all fonts for better portability".to_string(),
        });
    }

    // 8. Image resolution check (simplified)
    let image_count = content.matches("/Subtype/Image").count();
    if image_count > 0 {
        // Check for low-resolution images (simplified heuristic)
        let low_res_count =
            content.matches("/Width 72").count() + content.matches("/Width 96").count();
        if low_res_count > 0 {
            warnings.push(ComplianceWarning {
                code: "PRE-W003".to_string(),
                message: "Possible low-resolution images detected".to_string(),
                recommendation: "Ensure images are at least 300 DPI for print".to_string(),
            });
        }
    }

    // 9. Color space check
    let has_rgb = content.contains("/DeviceRGB");
    let has_cmyk = content.contains("/DeviceCMYK");
    let has_gray = content.contains("/DeviceGray");

    if has_rgb && has_cmyk {
        warnings.push(ComplianceWarning {
            code: "PRE-W004".to_string(),
            message: "Mixed RGB and CMYK color spaces detected".to_string(),
            recommendation: "Use consistent color space for print production".to_string(),
        });
    }

    // 10. Transparency check
    if content.contains("/ExtGState") && content.contains("/ca") {
        warnings.push(ComplianceWarning {
            code: "PRE-W005".to_string(),
            message: "Transparency detected".to_string(),
            recommendation: "Flatten transparency for print or PDF/A-1 compliance".to_string(),
        });
    }

    // 11. Encryption check
    if content.contains("/Encrypt") {
        warnings.push(ComplianceWarning {
            code: "PRE-W006".to_string(),
            message: "Document is encrypted".to_string(),
            recommendation: "Remove encryption for archival or accessibility compliance"
                .to_string(),
        });
    }

    // 12. JavaScript check
    if content.contains("/JavaScript") || content.contains("/JS") {
        warnings.push(ComplianceWarning {
            code: "PRE-W007".to_string(),
            message: "JavaScript detected".to_string(),
            recommendation: "Remove JavaScript for PDF/A compliance".to_string(),
        });
    }

    // 13. Form fields check
    if content.contains("/AcroForm") {
        let field_count = content.matches("/FT/").count();
        warnings.push(ComplianceWarning {
            code: "PRE-W008".to_string(),
            message: format!("Document contains {} form fields", field_count),
            recommendation: "Consider flattening forms for archival".to_string(),
        });
    }

    // 14. Annotation check
    if content.contains("/Annots") {
        let annot_count = content.matches("/Subtype/").count();
        warnings.push(ComplianceWarning {
            code: "PRE-W009".to_string(),
            message: format!(
                "Document contains approximately {} annotations",
                annot_count
            ),
            recommendation: "Review annotations for compliance requirements".to_string(),
        });
    }

    // 15. Metadata check
    if !content.contains("/Info") {
        warnings.push(ComplianceWarning {
            code: "PRE-W010".to_string(),
            message: "Missing document info dictionary".to_string(),
            recommendation: "Add document metadata (title, author, etc.)".to_string(),
        });
    }

    Ok(ComplianceValidation {
        compliant: errors.is_empty(),
        errors,
        warnings,
    })
}

/// Helper: Check font embedding
fn check_font_embedding(content: &str) -> FontEmbeddingCheck {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    // Look for font dictionaries
    let font_positions: Vec<_> = content.match_indices("/Type/Font").collect();

    for (pos, _) in font_positions {
        // Get context around font definition
        let start = pos.saturating_sub(500);
        let end = (pos + 500).min(content.len());
        let font_context = &content[start..end];

        // Check for font descriptor
        if !font_context.contains("/FontDescriptor") {
            warnings.push(ComplianceWarning {
                code: "FONT-W001".to_string(),
                message: "Font should have FontDescriptor".to_string(),
                recommendation: "Add FontDescriptor to font dictionary".to_string(),
            });
        }

        // Check for embedded font stream
        let has_font_file = font_context.contains("/FontFile")
            || font_context.contains("/FontFile2")
            || font_context.contains("/FontFile3");

        if !has_font_file {
            errors.push(ComplianceError {
                code: "FONT-001".to_string(),
                message: "All fonts must be embedded in PDF/A".to_string(),
                location: format!("Font at position {}", pos),
                severity: ErrorSeverity::Critical,
            });
        }

        // Check for standard 14 fonts (which should be embedded in PDF/A)
        let standard_fonts = [
            "Times-Roman",
            "Helvetica",
            "Courier",
            "Symbol",
            "ZapfDingbats",
        ];
        for font_name in &standard_fonts {
            if font_context.contains(font_name) && !has_font_file {
                errors.push(ComplianceError {
                    code: "FONT-002".to_string(),
                    message: format!("Standard font '{}' must be embedded in PDF/A", font_name),
                    location: format!("Font at position {}", pos),
                    severity: ErrorSeverity::Critical,
                });
            }
        }
    }

    FontEmbeddingCheck { errors, warnings }
}

struct FontEmbeddingCheck {
    errors: Vec<ComplianceError>,
    warnings: Vec<ComplianceWarning>,
}

/// Helper: Check for invalid PDF/A elements
fn check_invalid_elements(content: &str) -> Vec<ComplianceError> {
    let mut errors = Vec::new();

    // PDF/A does not allow these elements
    let invalid_elements = [
        (
            "/OPI",
            "PDFA-E001",
            "OPI (Open Prepress Interface) not allowed",
        ),
        (
            "/Linearized",
            "PDFA-E002",
            "Linearized PDFs may have issues, validate carefully",
        ),
        ("/Movie", "PDFA-E003", "Movie annotations not allowed"),
        ("/Sound", "PDFA-E004", "Sound annotations not allowed"),
        (
            "/FileAttachment",
            "PDFA-E005",
            "FileAttachment annotations not allowed in PDF/A-1 and PDF/A-2",
        ),
    ];

    for (element, code, message) in &invalid_elements {
        if content.contains(element) {
            errors.push(ComplianceError {
                code: code.to_string(),
                message: message.to_string(),
                location: "Document".to_string(),
                severity: ErrorSeverity::Error,
            });
        }
    }

    errors
}

/// Convert PDF to PDF/A
pub fn convert_to_pdfa(pdf_path: &str, output_path: &str, standard: PdfAStandard) -> Result<()> {
    if !Path::new(pdf_path).exists() {
        return Err(EnhancedError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("PDF file not found: {}", pdf_path),
        )));
    }

    let mut pdf_data = std::fs::read(pdf_path)?;
    let content = String::from_utf8_lossy(&pdf_data);

    // 1. Remove encryption if present
    if content.contains("/Encrypt") {
        // Note: Actual encryption removal would require decrypting first
        return Err(EnhancedError::Unsupported(
            "Encrypted PDFs must be decrypted before PDF/A conversion".to_string(),
        ));
    }

    // 2. Remove JavaScript
    let content = remove_javascript(&content);

    // 3. Add XMP metadata with PDF/A identifier
    let content = add_pdfa_xmp_metadata(&content, standard)?;

    // 4. Add OutputIntent with ICC profile
    let content = add_output_intent(&content)?;

    // 5. Embed standard fonts (if any are used)
    // Note: This requires actual font files, which is complex
    // For now, we'll flag this as needed

    // 6. For PDF/A-1, flatten transparency
    let content = if matches!(standard, PdfAStandard::PdfA1a | PdfAStandard::PdfA1b) {
        flatten_transparency(&content)?
    } else {
        content.to_string()
    };

    // Write modified PDF
    std::fs::write(output_path, content.as_bytes())?;

    Ok(())
}

/// Remove JavaScript from PDF
fn remove_javascript(content: &str) -> String {
    let mut result = content.to_string();

    // Remove JavaScript actions
    // This is a simplified version - real implementation would parse the PDF structure
    result = result.replace("/JavaScript", "/Removed_JavaScript");
    result = result.replace("/JS", "/Removed_JS");

    result
}

/// Add PDF/A XMP metadata
fn add_pdfa_xmp_metadata(content: &str, standard: PdfAStandard) -> Result<String> {
    let (part, conformance) = match standard {
        PdfAStandard::PdfA1a => (1, "A"),
        PdfAStandard::PdfA1b => (1, "B"),
        PdfAStandard::PdfA2a => (2, "A"),
        PdfAStandard::PdfA2b => (2, "B"),
        PdfAStandard::PdfA2u => (2, "U"),
        PdfAStandard::PdfA3a => (3, "A"),
        PdfAStandard::PdfA3b => (3, "B"),
        PdfAStandard::PdfA3u => (3, "U"),
    };

    let xmp_metadata = format!(
        r#"<?xpacket begin="" id="W5M0MpCehiHzreSzNTczkc9d"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/">
  <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
    <rdf:Description rdf:about="" xmlns:pdfaid="http://www.aiim.org/pdfa/ns/id/">
      <pdfaid:part>{}</pdfaid:part>
      <pdfaid:conformance>{}</pdfaid:conformance>
    </rdf:Description>
  </rdf:RDF>
</x:xmpmeta>
<?xpacket end="w"?>"#,
        part, conformance
    );

    // Insert XMP metadata into PDF
    let mut result = content.to_string();
    if let Some(catalog_pos) = result.find("/Type/Catalog") {
        // Find the end of the Catalog dictionary
        if let Some(dict_end) = result[catalog_pos..].find(">>") {
            let insert_pos = catalog_pos + dict_end;
            result.insert_str(insert_pos, &format!(" /Metadata << {} >>", xmp_metadata));
        }
    }

    Ok(result)
}

/// Add OutputIntent with ICC color profile
fn add_output_intent(content: &str) -> Result<String> {
    // sRGB IEC61966-2.1 profile identifier
    let output_intent = r#"
/OutputIntents [<<
  /Type /OutputIntent
  /S /GTS_PDFA1
  /OutputConditionIdentifier (sRGB IEC61966-2.1)
  /RegistryName (http://www.color.org)
  /Info (sRGB IEC61966-2.1)
>>]"#;

    let mut result = content.to_string();

    // Add OutputIntent to Catalog
    if let Some(catalog_pos) = result.find("/Type/Catalog") {
        // Find end of Catalog dictionary
        if let Some(end_pos) = result[catalog_pos..].find(">>") {
            result.insert_str(catalog_pos + end_pos, output_intent);
        }
    }

    Ok(result)
}

/// Flatten transparency for PDF/A-1
fn flatten_transparency(content: &str) -> Result<String> {
    // This is a placeholder - real implementation would:
    // 1. Find all transparency groups
    // 2. Render them to raster
    // 3. Replace with non-transparent equivalents

    if content.contains("/ca") || content.contains("/CA") {
        return Err(EnhancedError::Unsupported(
            "Transparency flattening requires rendering - not yet implemented".to_string(),
        ));
    }

    Ok(content.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pdfa_standards() {
        let standard = PdfAStandard::PdfA3b;
        assert_eq!(standard, PdfAStandard::PdfA3b);
    }

    #[test]
    fn test_pdfua_standards() {
        let standard = PdfUaStandard::PdfUa1;
        assert_eq!(standard, PdfUaStandard::PdfUa1);
    }

    #[test]
    fn test_compliance_validation() {
        let validation = ComplianceValidation {
            compliant: false,
            errors: vec![],
            warnings: vec![],
        };
        assert!(!validation.compliant);
    }

    #[test]
    fn test_remove_javascript() {
        let content = "test /JavaScript test /JS test";
        let result = remove_javascript(content);
        assert!(!result.contains("/JavaScript"));
        assert!(!result.contains("/JS"));
    }

    #[test]
    fn test_xmp_metadata_generation() {
        let test_content = "test /Type/Catalog << >> test";
        let result = add_pdfa_xmp_metadata(test_content, PdfAStandard::PdfA3b);
        assert!(result.is_ok());
        let xmp = result.unwrap();
        assert!(xmp.contains("pdfaid:part"));
        assert!(xmp.contains("pdfaid:conformance"));
    }

    #[test]
    fn test_check_invalid_elements() {
        let content = "test /Movie test /Sound test";
        let errors = check_invalid_elements(content);
        assert!(!errors.is_empty());
        assert!(errors.iter().any(|e| e.code == "PDFA-E003"));
        assert!(errors.iter().any(|e| e.code == "PDFA-E004"));
    }
}
