//! DCTDecode (JPEG) Filter Implementation

use super::params::DCTDecodeParams;
use crate::fitz::error::{Error, Result};

/// Decode JPEG compressed data
pub fn decode_dct(data: &[u8], _params: Option<&DCTDecodeParams>) -> Result<Vec<u8>> {
    use image::ImageReader;
    use std::io::Cursor;

    let reader = ImageReader::with_format(Cursor::new(data), image::ImageFormat::Jpeg);

    let img = reader
        .decode()
        .map_err(|e| Error::Generic(format!("DCTDecode failed: {}", e)))?;

    Ok(img.into_bytes())
}

/// Encode data with JPEG compression
pub fn encode_dct(data: &[u8], width: u32, height: u32, quality: u8) -> Result<Vec<u8>> {
    use image::codecs::jpeg::JpegEncoder;
    use image::{ImageBuffer, Rgb};
    use std::io::Cursor;

    // Assume RGB data
    let img: ImageBuffer<Rgb<u8>, _> = ImageBuffer::from_raw(width, height, data.to_vec())
        .ok_or_else(|| Error::Generic("Invalid image dimensions".into()))?;

    let mut output = Cursor::new(Vec::new());

    // Use JpegEncoder to specify quality (1-100)
    let mut encoder = JpegEncoder::new_with_quality(&mut output, quality);
    encoder
        .encode(img.as_raw(), width, height, image::ExtendedColorType::Rgb8)
        .map_err(|e| Error::Generic(format!("DCTEncode failed: {}", e)))?;

    Ok(output.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode_dct() {
        // Create a simple 2x2 RGB image
        let width = 2u32;
        let height = 2u32;
        let data: Vec<u8> = vec![
            255, 0, 0, // Red pixel
            0, 255, 0, // Green pixel
            0, 0, 255, // Blue pixel
            255, 255, 0, // Yellow pixel
        ];

        // Encode to JPEG
        let encoded = encode_dct(&data, width, height, 85).unwrap();

        // Should start with JPEG magic bytes
        assert_eq!(&encoded[0..2], &[0xFF, 0xD8]);

        // Decode back
        let decoded = decode_dct(&encoded, None).unwrap();

        // Decoded data should be same dimensions (may have slight differences due to JPEG compression)
        assert_eq!(decoded.len(), data.len());
    }

    #[test]
    fn test_decode_dct_invalid_data() {
        let invalid_data = vec![1, 2, 3, 4, 5];
        let result = decode_dct(&invalid_data, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_encode_dct_invalid_dimensions() {
        let data = vec![255, 0, 0]; // 3 bytes (1 pixel)

        // Try to encode as 10x10 (should fail, not enough data)
        let result = encode_dct(&data, 10, 10, 85);
        assert!(result.is_err());
    }

    #[test]
    fn test_encode_dct_empty_data() {
        let data: Vec<u8> = vec![];
        let result = encode_dct(&data, 0, 0, 85);
        assert!(result.is_err());
    }

    #[test]
    fn test_decode_dct_with_params() {
        // Create a small valid JPEG
        let width = 2u32;
        let height = 2u32;
        let data: Vec<u8> = vec![255, 0, 0, 0, 255, 0, 0, 0, 255, 255, 255, 255];

        let encoded = encode_dct(&data, width, height, 85).unwrap();

        // Decode with parameters (parameters are ignored but function should still work)
        let params = DCTDecodeParams::default();
        let decoded = decode_dct(&encoded, Some(&params)).unwrap();

        assert_eq!(decoded.len(), data.len());
    }

    #[test]
    fn test_encode_dct_different_quality() {
        let width = 4u32;
        let height = 4u32;
        let data: Vec<u8> = vec![255; (width * height * 3) as usize];

        // Note: quality parameter is currently ignored, but test should pass
        let encoded_low = encode_dct(&data, width, height, 10).unwrap();
        let encoded_high = encode_dct(&data, width, height, 95).unwrap();

        // Both should be valid JPEG
        assert_eq!(&encoded_low[0..2], &[0xFF, 0xD8]);
        assert_eq!(&encoded_high[0..2], &[0xFF, 0xD8]);
    }
}
