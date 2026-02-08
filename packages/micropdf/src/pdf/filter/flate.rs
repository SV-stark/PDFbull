//! FlateDecode (zlib/deflate) Filter Implementation

use super::params::FlateDecodeParams;
use super::predictor::apply_predictor_decode;
use crate::fitz::error::{Error, Result};
use flate2::Compression;
use flate2::read::{ZlibDecoder, ZlibEncoder};
use std::io::Read;

/// Decode FlateDecode (zlib/deflate) compressed data
pub fn decode_flate(data: &[u8], params: Option<&FlateDecodeParams>) -> Result<Vec<u8>> {
    let mut decoder = ZlibDecoder::new(data);
    let mut decompressed = Vec::new();
    decoder
        .read_to_end(&mut decompressed)
        .map_err(|e| Error::Generic(format!("FlateDecode failed: {}", e)))?;

    // Apply predictor if specified
    if let Some(params) = params {
        if params.predictor > 1 {
            decompressed = apply_predictor_decode(&decompressed, params)?;
        }
    }

    Ok(decompressed)
}

/// Encode data with FlateDecode (zlib/deflate)
pub fn encode_flate(data: &[u8], level: u32) -> Result<Vec<u8>> {
    let compression = match level {
        0 => Compression::none(),
        1..=3 => Compression::fast(),
        4..=6 => Compression::default(),
        _ => Compression::best(),
    };

    let mut encoder = ZlibEncoder::new(data, compression);
    let mut compressed = Vec::new();
    encoder
        .read_to_end(&mut compressed)
        .map_err(|e| Error::Generic(format!("FlateDecode encode failed: {}", e)))?;

    Ok(compressed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flate_encode_decode() {
        // Use longer text with repetition for better compression
        let original = b"Hello, FlateDecode! This is a test of zlib compression. \
                         Hello, FlateDecode! This is a test of zlib compression. \
                         Hello, FlateDecode! This is a test of zlib compression. \
                         Hello, FlateDecode! This is a test of zlib compression.";

        // Encode
        let compressed = encode_flate(original, 6).unwrap();
        assert!(compressed.len() < original.len()); // Should be smaller with repetition

        // Decode
        let decompressed = decode_flate(&compressed, None).unwrap();
        assert_eq!(decompressed, original.as_slice());
    }

    #[test]
    fn test_flate_empty_data() {
        let empty: &[u8] = &[];
        let compressed = encode_flate(empty, 6).unwrap();
        let decompressed = decode_flate(&compressed, None).unwrap();
        assert_eq!(decompressed, empty);
    }

    #[test]
    fn test_flate_compression_levels() {
        let data = b"Test data for compression level testing";

        // Test different compression levels
        for level in [0, 3, 6, 9] {
            let compressed = encode_flate(data, level).unwrap();
            let decompressed = decode_flate(&compressed, None).unwrap();
            assert_eq!(decompressed, data);
        }
    }
}
