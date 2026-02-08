//! Filter Chain Implementation

use super::FilterType;
use super::params::CCITTFaxDecodeParams;
use super::*;
use crate::fitz::error::{Error, Result};

/// A chain of filters to apply
#[derive(Debug, Clone)]
pub struct FilterChain {
    filters: Vec<FilterType>,
}

impl FilterChain {
    pub fn new() -> Self {
        Self {
            filters: Vec::new(),
        }
    }

    pub fn add(&mut self, filter: FilterType) {
        self.filters.push(filter);
    }

    /// Decode data through the filter chain (in order)
    pub fn decode(&self, mut data: Vec<u8>) -> Result<Vec<u8>> {
        for filter in &self.filters {
            data = match filter {
                FilterType::FlateDecode => decode_flate(&data, None)?,
                FilterType::LZWDecode => decode_lzw(&data, None)?,
                FilterType::ASCII85Decode => decode_ascii85(&data)?,
                FilterType::ASCIIHexDecode => decode_ascii_hex(&data)?,
                FilterType::RunLengthDecode => decode_run_length(&data)?,
                FilterType::CCITTFaxDecode => {
                    decode_ccitt_fax(&data, &CCITTFaxDecodeParams::default())?
                }
                FilterType::DCTDecode => decode_dct(&data, None)?,
                FilterType::JPXDecode => decode_jpx(&data)?,
                FilterType::JBIG2Decode => decode_jbig2(&data, None)?,
                FilterType::Crypt => data, // Encryption handled separately
            };
        }
        Ok(data)
    }

    /// Encode data through the filter chain (in reverse order)
    pub fn encode(&self, mut data: Vec<u8>) -> Result<Vec<u8>> {
        for filter in self.filters.iter().rev() {
            data = match filter {
                FilterType::FlateDecode => encode_flate(&data, 6)?,
                FilterType::LZWDecode => encode_lzw(&data)?,
                FilterType::ASCII85Decode => encode_ascii85(&data)?,
                FilterType::ASCIIHexDecode => encode_ascii_hex(&data)?,
                FilterType::RunLengthDecode => encode_run_length(&data)?,
                FilterType::CCITTFaxDecode => {
                    return Err(Error::Generic("CCITTFaxEncode not supported".into()));
                }
                FilterType::DCTDecode => {
                    return Err(Error::Generic("DCTEncode requires image dimensions".into()));
                }
                FilterType::JPXDecode => {
                    return Err(Error::Generic("JPXEncode not supported".into()));
                }
                FilterType::JBIG2Decode => {
                    return Err(Error::Generic("JBIG2Encode not supported".into()));
                }
                FilterType::Crypt => data,
            };
        }
        Ok(data)
    }
}

impl Default for FilterChain {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_chain_flate() {
        let mut chain = FilterChain::new();
        chain.add(FilterType::FlateDecode);

        let original = b"Hello, FilterChain!";
        let compressed = encode_flate(original, 6).unwrap();
        let decoded = chain.decode(compressed).unwrap();

        assert_eq!(decoded, original);
    }

    #[test]
    fn test_filter_chain_multiple() {
        let mut chain = FilterChain::new();
        chain.add(FilterType::ASCII85Decode);
        chain.add(FilterType::FlateDecode);

        let original = b"Test data";

        // Encode manually (reverse order)
        let compressed = encode_flate(original, 6).unwrap();
        let ascii85 = encode_ascii85(&compressed).unwrap();

        // Decode with chain (forward order)
        let decoded = chain.decode(ascii85).unwrap();

        assert_eq!(decoded, original);
    }

    #[test]
    fn test_filter_chain_encode() {
        let mut chain = FilterChain::new();
        chain.add(FilterType::FlateDecode);

        let original = b"Encode test data";
        let encoded = chain.encode(original.to_vec()).unwrap();

        // Should be compressed
        assert!(!encoded.is_empty());

        // Decode it back
        let decoded = chain.decode(encoded).unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn test_filter_chain_encode_multiple() {
        let mut chain = FilterChain::new();
        chain.add(FilterType::ASCII85Decode);
        chain.add(FilterType::FlateDecode);

        let original = b"Multiple encode test";
        let encoded = chain.encode(original.to_vec()).unwrap();

        // Decode it back
        let decoded = chain.decode(encoded).unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn test_filter_chain_lzw() {
        let mut chain = FilterChain::new();
        chain.add(FilterType::LZWDecode);

        let original = b"LZW test data";
        let encoded = encode_lzw(original).unwrap();
        let decoded = chain.decode(encoded).unwrap();

        assert_eq!(decoded, original);
    }

    #[test]
    fn test_filter_chain_asciihex() {
        let mut chain = FilterChain::new();
        chain.add(FilterType::ASCIIHexDecode);

        let original = b"ASCIIHex test";
        let encoded = encode_ascii_hex(original).unwrap();
        let decoded = chain.decode(encoded).unwrap();

        assert_eq!(decoded, original);
    }

    #[test]
    fn test_filter_chain_runlength() {
        let mut chain = FilterChain::new();
        chain.add(FilterType::RunLengthDecode);

        let original = b"RunLength test";
        let encoded = encode_run_length(original).unwrap();
        let decoded = chain.decode(encoded).unwrap();

        assert_eq!(decoded, original);
    }

    #[test]
    fn test_filter_chain_crypt() {
        let mut chain = FilterChain::new();
        chain.add(FilterType::Crypt);

        let original = b"Crypt pass-through";
        let decoded = chain.decode(original.to_vec()).unwrap();

        // Crypt is a no-op in the filter chain
        assert_eq!(decoded, original);
    }

    #[test]
    fn test_filter_chain_encode_ccitt_unsupported() {
        let mut chain = FilterChain::new();
        chain.add(FilterType::CCITTFaxDecode);

        let original = b"Test";
        let result = chain.encode(original.to_vec());

        assert!(result.is_err());
    }

    #[test]
    fn test_filter_chain_encode_dct_unsupported() {
        let mut chain = FilterChain::new();
        chain.add(FilterType::DCTDecode);

        let original = b"Test";
        let result = chain.encode(original.to_vec());

        assert!(result.is_err());
    }

    #[test]
    fn test_filter_chain_encode_jpx_unsupported() {
        let mut chain = FilterChain::new();
        chain.add(FilterType::JPXDecode);

        let original = b"Test";
        let result = chain.encode(original.to_vec());

        assert!(result.is_err());
    }

    #[test]
    fn test_filter_chain_encode_jbig2_unsupported() {
        let mut chain = FilterChain::new();
        chain.add(FilterType::JBIG2Decode);

        let original = b"Test";
        let result = chain.encode(original.to_vec());

        assert!(result.is_err());
    }

    #[test]
    fn test_filter_chain_default() {
        let chain = FilterChain::default();
        assert_eq!(chain.filters.len(), 0);
    }

    #[test]
    fn test_filter_chain_clone() {
        let mut chain = FilterChain::new();
        chain.add(FilterType::FlateDecode);
        chain.add(FilterType::ASCII85Decode);

        let cloned = chain.clone();
        assert_eq!(cloned.filters.len(), chain.filters.len());
    }
}
