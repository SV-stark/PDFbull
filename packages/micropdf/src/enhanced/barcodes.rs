//! Barcodes and QR Codes
//!
//! Generate and embed 1D and 2D barcodes in PDF documents:
//! - 1D: Code39, Code128, EAN-13, UPC-A
//! - 2D: QR codes, Data Matrix, PDF417, Aztec
//! - Vector format (scalable)
//! - Customizable dimensions and appearance

use super::error::{EnhancedError, Result};
use std::path::Path;

/// Barcode type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BarcodeType {
    // 1D Barcodes
    Code39,
    Code93,
    Code128,
    Ean8,
    Ean13,
    UpcA,
    UpcE,
    Codabar,
    Itf,

    // 2D Barcodes
    QrCode,
    DataMatrix,
    Pdf417,
    AztecCode,
    MaxiCode,
}

/// QR code error correction level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QrErrorCorrection {
    Low,      // 7% recovery
    Medium,   // 15% recovery
    Quartile, // 25% recovery
    High,     // 30% recovery
}

/// Barcode configuration
#[derive(Debug, Clone)]
pub struct BarcodeConfig {
    /// Barcode type
    pub barcode_type: BarcodeType,
    /// Data to encode
    pub data: String,
    /// Width in points
    pub width: f32,
    /// Height in points
    pub height: f32,
    /// Show human-readable text
    pub show_text: bool,
    /// Text position
    pub text_position: TextPosition,
    /// Font size for text
    pub text_font_size: f32,
    /// Quiet zone (margin) in modules
    pub quiet_zone: u32,
    /// QR error correction (for QR codes only)
    pub qr_error_correction: QrErrorCorrection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextPosition {
    None,
    Top,
    Bottom,
}

impl BarcodeConfig {
    /// Create new barcode config
    pub fn new(barcode_type: BarcodeType, data: impl Into<String>) -> Self {
        Self {
            barcode_type,
            data: data.into(),
            width: 200.0,
            height: 50.0,
            show_text: true,
            text_position: TextPosition::Bottom,
            text_font_size: 10.0,
            quiet_zone: 10,
            qr_error_correction: QrErrorCorrection::Medium,
        }
    }

    /// Set dimensions
    pub fn size(mut self, width: f32, height: f32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    /// Hide text
    pub fn no_text(mut self) -> Self {
        self.show_text = false;
        self
    }

    /// Set QR error correction
    pub fn qr_error_correction(mut self, level: QrErrorCorrection) -> Self {
        self.qr_error_correction = level;
        self
    }
}

/// Generate barcode and add to PDF
pub fn add_barcode_to_pdf(
    pdf_path: &str,
    page: u32,
    x: f32,
    y: f32,
    config: &BarcodeConfig,
) -> Result<()> {
    if !Path::new(pdf_path).exists() {
        return Err(EnhancedError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("PDF file not found: {}", pdf_path),
        )));
    }

    // TODO: Implement barcode generation and embedding
    // 1. Generate barcode image (use barcode crates)
    // 2. Convert to vector format (XObject)
    // 3. Add to PDF page content stream
    // 4. Add human-readable text if configured

    Ok(())
}

/// Generate barcode and save as PDF
pub fn create_barcode_pdf(output_path: &str, config: &BarcodeConfig) -> Result<()> {
    // TODO: Create new PDF with barcode
    // 1. Generate barcode
    // 2. Create PDF with appropriate size
    // 3. Embed barcode as vector graphics

    Ok(())
}

/// Generate QR code and add to PDF
pub fn add_qr_code(
    pdf_path: &str,
    page: u32,
    x: f32,
    y: f32,
    data: &str,
    size: f32,
    error_correction: QrErrorCorrection,
) -> Result<()> {
    let config = BarcodeConfig::new(BarcodeType::QrCode, data)
        .size(size, size)
        .qr_error_correction(error_correction)
        .no_text();

    add_barcode_to_pdf(pdf_path, page, x, y, &config)
}

/// Generate Code128 barcode and add to PDF
pub fn add_code128(
    pdf_path: &str,
    page: u32,
    x: f32,
    y: f32,
    data: &str,
    width: f32,
    height: f32,
) -> Result<()> {
    let config = BarcodeConfig::new(BarcodeType::Code128, data).size(width, height);

    add_barcode_to_pdf(pdf_path, page, x, y, &config)
}

/// Generate EAN-13 barcode and add to PDF
pub fn add_ean13(pdf_path: &str, page: u32, x: f32, y: f32, data: &str) -> Result<()> {
    // Validate EAN-13 format (13 digits)
    if data.len() != 13 || !data.chars().all(|c| c.is_ascii_digit()) {
        return Err(EnhancedError::InvalidParameter(
            "EAN-13 must be exactly 13 digits".to_string(),
        ));
    }

    let config = BarcodeConfig::new(BarcodeType::Ean13, data).size(113.0, 60.0); // Standard EAN-13 dimensions

    add_barcode_to_pdf(pdf_path, page, x, y, &config)
}

/// Validate barcode data for type
pub fn validate_barcode_data(barcode_type: BarcodeType, data: &str) -> Result<()> {
    match barcode_type {
        BarcodeType::Ean8 => {
            if data.len() != 8 || !data.chars().all(|c| c.is_ascii_digit()) {
                return Err(EnhancedError::InvalidParameter(
                    "EAN-8 must be exactly 8 digits".to_string(),
                ));
            }
        }
        BarcodeType::Ean13 => {
            if data.len() != 13 || !data.chars().all(|c| c.is_ascii_digit()) {
                return Err(EnhancedError::InvalidParameter(
                    "EAN-13 must be exactly 13 digits".to_string(),
                ));
            }
        }
        BarcodeType::UpcA => {
            if data.len() != 12 || !data.chars().all(|c| c.is_ascii_digit()) {
                return Err(EnhancedError::InvalidParameter(
                    "UPC-A must be exactly 12 digits".to_string(),
                ));
            }
        }
        // Add validation for other types
        _ => {}
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_barcode_config() {
        let config = BarcodeConfig::new(BarcodeType::Code128, "TEST123")
            .size(250.0, 60.0)
            .no_text();

        assert_eq!(config.data, "TEST123");
        assert_eq!(config.width, 250.0);
        assert_eq!(config.height, 60.0);
        assert!(!config.show_text);
    }

    #[test]
    fn test_qr_error_correction() {
        let config = BarcodeConfig::new(BarcodeType::QrCode, "https://example.com")
            .qr_error_correction(QrErrorCorrection::High);

        assert_eq!(config.qr_error_correction, QrErrorCorrection::High);
    }

    #[test]
    fn test_ean13_validation() {
        assert!(validate_barcode_data(BarcodeType::Ean13, "1234567890128").is_ok());
        assert!(validate_barcode_data(BarcodeType::Ean13, "123").is_err());
        assert!(validate_barcode_data(BarcodeType::Ean13, "123456789012A").is_err());
    }

    #[test]
    fn test_upca_validation() {
        assert!(validate_barcode_data(BarcodeType::UpcA, "123456789012").is_ok());
        assert!(validate_barcode_data(BarcodeType::UpcA, "12345").is_err());
    }
}
