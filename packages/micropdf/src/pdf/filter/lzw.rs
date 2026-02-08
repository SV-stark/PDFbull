//! LZWDecode Filter Implementation

use super::params::{FlateDecodeParams, LZWDecodeParams};
use super::predictor::apply_predictor_decode;
use crate::fitz::error::{Error, Result};

/// Decode LZW compressed data
pub fn decode_lzw(data: &[u8], params: Option<&LZWDecodeParams>) -> Result<Vec<u8>> {
    let early_change = params.map(|p| p.early_change != 0).unwrap_or(true);

    let mut decoder = weezl::decode::Decoder::with_tiff_size_switch(
        weezl::BitOrder::Msb,
        if early_change { 8 } else { 9 },
    );

    let decompressed = decoder
        .decode(data)
        .map_err(|e| Error::Generic(format!("LZWDecode failed: {:?}", e)))?;

    // Apply predictor if specified
    let mut result = decompressed;
    if let Some(params) = params {
        if params.predictor > 1 {
            let flate_params = FlateDecodeParams {
                predictor: params.predictor,
                colors: params.colors,
                bits_per_component: params.bits_per_component,
                columns: params.columns,
            };
            result = apply_predictor_decode(&result, &flate_params)?;
        }
    }

    Ok(result)
}

/// Encode data with LZW compression
pub fn encode_lzw(data: &[u8]) -> Result<Vec<u8>> {
    let mut encoder = weezl::encode::Encoder::with_tiff_size_switch(weezl::BitOrder::Msb, 8);
    encoder
        .encode(data)
        .map_err(|e| Error::Generic(format!("LZWEncode failed: {:?}", e)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lzw_encode_decode() {
        let original = b"ABCABCABCABCABC"; // Repetitive data compresses well

        // Encode
        let compressed = encode_lzw(original).unwrap();

        // Decode
        let decompressed = decode_lzw(&compressed, None).unwrap();
        assert_eq!(decompressed, original);
    }

    #[test]
    fn test_lzw_empty_data() {
        let empty: &[u8] = &[];
        let compressed = encode_lzw(empty).unwrap();
        let decompressed = decode_lzw(&compressed, None).unwrap();
        assert_eq!(decompressed, empty);
    }
}
