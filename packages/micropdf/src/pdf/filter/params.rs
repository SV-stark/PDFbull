//! Parameter structures for PDF filters

/// Parameters for FlateDecode filter
#[derive(Debug, Clone, Default)]
pub struct FlateDecodeParams {
    /// PNG predictor algorithm (1 = None, 2 = TIFF, 10-15 = PNG)
    pub predictor: i32,
    /// Number of color components per sample
    pub colors: i32,
    /// Number of bits per color component
    pub bits_per_component: i32,
    /// Number of samples per row
    pub columns: i32,
}

/// Parameters for LZWDecode filter
#[derive(Debug, Clone, Default)]
pub struct LZWDecodeParams {
    /// PNG predictor algorithm
    pub predictor: i32,
    /// Number of color components per sample
    pub colors: i32,
    /// Number of bits per color component
    pub bits_per_component: i32,
    /// Number of samples per row
    pub columns: i32,
    /// Early change parameter (0 or 1)
    pub early_change: i32,
}

/// Parameters for CCITTFaxDecode filter
#[derive(Debug, Clone)]
pub struct CCITTFaxDecodeParams {
    /// Encoding scheme: 0 = Group 3 1D, <0 = Group 3 2D, >0 = Group 4
    pub k: i32,
    /// If true, end-of-line bit patterns are required
    pub end_of_line: bool,
    /// If true, byte-aligned encoding is expected
    pub encoded_byte_align: bool,
    /// Width of the image in pixels
    pub columns: i32,
    /// Height of the image in pixels
    pub rows: i32,
    /// If true, uncompressed data should be end-of-block
    pub end_of_block: bool,
    /// If true, 0 means white, 1 means black (default: false)
    pub black_is_1: bool,
    /// Number of damaged rows allowed
    pub damaged_rows_before_error: i32,
}

impl Default for CCITTFaxDecodeParams {
    fn default() -> Self {
        Self {
            k: 0,
            end_of_line: false,
            encoded_byte_align: false,
            columns: 1728,
            rows: 0,
            end_of_block: true,
            black_is_1: false,
            damaged_rows_before_error: 0,
        }
    }
}

/// Parameters for DCTDecode filter (JPEG)
#[derive(Debug, Clone, Default)]
pub struct DCTDecodeParams {
    /// Color transform: 0 = no transform, 1 = YCbCr to RGB
    pub color_transform: i32,
}

/// Parameters for JBIG2Decode filter
#[derive(Debug, Clone, Default)]
pub struct JBIG2DecodeParams {
    /// Global segment data
    pub jbig2_globals: Option<Vec<u8>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flate_decode_params_default() {
        let params = FlateDecodeParams::default();
        assert_eq!(params.predictor, 0);
        assert_eq!(params.colors, 0);
    }

    #[test]
    fn test_ccitt_fax_decode_params_default() {
        let params = CCITTFaxDecodeParams::default();
        assert_eq!(params.k, 0);
        assert_eq!(params.columns, 1728);
        assert!(params.end_of_block);
        assert!(!params.black_is_1);
    }

    #[test]
    fn test_dct_decode_params_default() {
        let params = DCTDecodeParams::default();
        assert_eq!(params.color_transform, 0);
    }

    #[test]
    fn test_jbig2_decode_params_default() {
        let params = JBIG2DecodeParams::default();
        assert!(params.jbig2_globals.is_none());
    }
}
