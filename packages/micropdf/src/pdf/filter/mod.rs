//! PDF Stream Filter/Compression Module
//!
//! This module implements all PDF stream filters for decompression and compression.
//! Supports the complete set of PDF filters as defined in PDF 1.7 specification.

// Module declarations
pub mod ascii85;
pub mod asciihex;
pub mod ccitt;
pub mod chain;
pub mod dct;
pub mod flate;
pub mod jbig2;
pub mod jpx;
pub mod lzw;
pub mod params;
pub mod predictor;
pub mod runlength;

// Re-exports
pub use ascii85::*;
pub use asciihex::*;
pub use ccitt::*;
pub use chain::*;
pub use dct::*;
pub use flate::*;
pub use jbig2::*;
pub use jpx::*;
pub use lzw::*;
pub use params::*;
pub use predictor::*;
pub use runlength::*;

/// PDF Filter types as defined in PDF specification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterType {
    /// FlateDecode - zlib/deflate compression (most common)
    FlateDecode,
    /// LZWDecode - Lempel-Ziv-Welch compression
    LZWDecode,
    /// ASCII85Decode - ASCII base-85 encoding
    ASCII85Decode,
    /// ASCIIHexDecode - Hexadecimal encoding
    ASCIIHexDecode,
    /// RunLengthDecode - Run-length encoding
    RunLengthDecode,
    /// CCITTFaxDecode - CCITT Group 3 and Group 4 fax encoding
    CCITTFaxDecode,
    /// DCTDecode - JPEG compression
    DCTDecode,
    /// JPXDecode - JPEG 2000 compression
    JPXDecode,
    /// JBIG2Decode - JBIG2 compression
    JBIG2Decode,
    /// Crypt - Encryption filter
    Crypt,
}

impl FilterType {
    /// Parse filter type from PDF name
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "FlateDecode" | "Fl" => Some(FilterType::FlateDecode),
            "LZWDecode" | "LZW" => Some(FilterType::LZWDecode),
            "ASCII85Decode" | "A85" => Some(FilterType::ASCII85Decode),
            "ASCIIHexDecode" | "AHx" => Some(FilterType::ASCIIHexDecode),
            "RunLengthDecode" | "RL" => Some(FilterType::RunLengthDecode),
            "CCITTFaxDecode" | "CCF" => Some(FilterType::CCITTFaxDecode),
            "DCTDecode" | "DCT" => Some(FilterType::DCTDecode),
            "JPXDecode" => Some(FilterType::JPXDecode),
            "JBIG2Decode" => Some(FilterType::JBIG2Decode),
            "Crypt" => Some(FilterType::Crypt),
            _ => None,
        }
    }

    /// Get the PDF name for this filter
    pub fn to_name(&self) -> &'static str {
        match self {
            FilterType::FlateDecode => "FlateDecode",
            FilterType::LZWDecode => "LZWDecode",
            FilterType::ASCII85Decode => "ASCII85Decode",
            FilterType::ASCIIHexDecode => "ASCIIHexDecode",
            FilterType::RunLengthDecode => "RunLengthDecode",
            FilterType::CCITTFaxDecode => "CCITTFaxDecode",
            FilterType::DCTDecode => "DCTDecode",
            FilterType::JPXDecode => "JPXDecode",
            FilterType::JBIG2Decode => "JBIG2Decode",
            FilterType::Crypt => "Crypt",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_type_from_name() {
        assert_eq!(
            FilterType::from_name("FlateDecode"),
            Some(FilterType::FlateDecode)
        );
        assert_eq!(FilterType::from_name("Fl"), Some(FilterType::FlateDecode));
        assert_eq!(FilterType::from_name("LZW"), Some(FilterType::LZWDecode));
        assert_eq!(FilterType::from_name("Invalid"), None);
    }

    #[test]
    fn test_filter_type_to_name() {
        assert_eq!(FilterType::FlateDecode.to_name(), "FlateDecode");
        assert_eq!(FilterType::LZWDecode.to_name(), "LZWDecode");
        assert_eq!(FilterType::ASCII85Decode.to_name(), "ASCII85Decode");
    }
}
