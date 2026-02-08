//! RunLengthDecode Filter Implementation

use crate::fitz::error::{Error, Result};

/// Decode RunLength encoded data
pub fn decode_run_length(data: &[u8]) -> Result<Vec<u8>> {
    let mut result = Vec::new();
    let mut i = 0;

    while i < data.len() {
        let length_byte = data[i];
        i += 1;

        if length_byte == 128 {
            // End of data
            break;
        } else if length_byte < 128 {
            // Copy next (length_byte + 1) bytes literally
            let count = length_byte as usize + 1;
            if i + count > data.len() {
                return Err(Error::Generic(
                    "RunLengthDecode: unexpected end of data".into(),
                ));
            }
            result.extend_from_slice(&data[i..i + count]);
            i += count;
        } else {
            // Repeat next byte (257 - length_byte) times
            let count = 257 - length_byte as usize;
            if i >= data.len() {
                return Err(Error::Generic(
                    "RunLengthDecode: unexpected end of data".into(),
                ));
            }
            let byte = data[i];
            i += 1;
            result.resize(result.len() + count, byte);
        }
    }

    Ok(result)
}

/// Encode data with RunLength
pub fn encode_run_length(data: &[u8]) -> Result<Vec<u8>> {
    let mut result = Vec::new();
    let mut i = 0;

    while i < data.len() {
        // Look for a run of identical bytes
        let start = i;
        let byte = data[i];
        while i < data.len() && data[i] == byte && i - start < 128 {
            i += 1;
        }
        let run_length = i - start;

        if run_length >= 2 {
            // Encode as a run
            result.push((257 - run_length) as u8);
            result.push(byte);
        } else {
            // Look for literal bytes
            i = start;
            let literal_start = i;

            while i < data.len() {
                // Check for a run of 3+ identical bytes
                if i + 2 < data.len() && data[i] == data[i + 1] && data[i] == data[i + 2] {
                    break;
                }
                i += 1;
                if i - literal_start >= 128 {
                    break;
                }
            }

            let literal_length = i - literal_start;
            if literal_length > 0 {
                result.push((literal_length - 1) as u8);
                result.extend_from_slice(&data[literal_start..i]);
            }
        }
    }

    // End of data marker
    result.push(128);

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runlength_encode_decode() {
        let original = b"AAAAAABBBCCCCCCCCCCDDDDDD";

        // Encode
        let encoded = encode_run_length(original).unwrap();

        // Decode
        let decoded = decode_run_length(&encoded).unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn test_runlength_no_runs() {
        let original = b"ABCDEFGH";

        let encoded = encode_run_length(original).unwrap();
        let decoded = decode_run_length(&encoded).unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn test_runlength_all_same() {
        let original = &[b'X'; 50];

        let encoded = encode_run_length(original).unwrap();
        let decoded = decode_run_length(&encoded).unwrap();
        assert_eq!(decoded, original);
    }
}
