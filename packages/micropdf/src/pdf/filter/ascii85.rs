//! ASCII85Decode Filter Implementation

use crate::fitz::error::{Error, Result};

/// Decode ASCII85 encoded data
pub fn decode_ascii85(data: &[u8]) -> Result<Vec<u8>> {
    let mut result = Vec::with_capacity(data.len() * 4 / 5);
    let mut group: u32 = 0;
    let mut count = 0;

    for &byte in data {
        // Skip whitespace
        if byte.is_ascii_whitespace() {
            continue;
        }

        // End of data marker
        if byte == b'~' {
            break;
        }

        // Special 'z' character represents 4 zero bytes
        if byte == b'z' {
            if count != 0 {
                return Err(Error::Generic("Invalid 'z' in ASCII85 stream".into()));
            }
            result.extend_from_slice(&[0, 0, 0, 0]);
            continue;
        }

        // Regular ASCII85 character
        if !(b'!'..=b'u').contains(&byte) {
            return Err(Error::Generic(format!(
                "Invalid ASCII85 character: {}",
                byte
            )));
        }

        group = group * 85 + (byte - b'!') as u32;
        count += 1;

        if count == 5 {
            result.push((group >> 24) as u8);
            result.push((group >> 16) as u8);
            result.push((group >> 8) as u8);
            result.push(group as u8);
            group = 0;
            count = 0;
        }
    }

    // Handle remaining bytes
    if count > 0 {
        // Pad with 'u' characters
        for _ in count..5 {
            group = group * 85 + 84;
        }

        for i in 0..(count - 1) {
            result.push((group >> (24 - i * 8)) as u8);
        }
    }

    Ok(result)
}

/// Encode data with ASCII85
pub fn encode_ascii85(data: &[u8]) -> Result<Vec<u8>> {
    let mut result = Vec::with_capacity(data.len() * 5 / 4 + 10);

    let mut i = 0;
    while i < data.len() {
        let chunk_len = (data.len() - i).min(4);
        let chunk = &data[i..i + chunk_len];

        let mut group: u32 = 0;
        for (j, &byte) in chunk.iter().enumerate() {
            group |= (byte as u32) << (24 - j * 8);
        }

        // Special case: all zeros (only for complete 4-byte chunks)
        if group == 0 && chunk_len == 4 {
            result.push(b'z');
            i += 4;
            continue;
        }

        let mut encoded = [0u8; 5];
        let mut temp = group;
        for j in (0..5).rev() {
            encoded[j] = (temp % 85) as u8 + b'!';
            temp /= 85;
        }

        // Output all 5 bytes for complete groups, or chunk_len + 1 for partial
        let output_len = if chunk_len == 4 { 5 } else { chunk_len + 1 };
        result.extend_from_slice(&encoded[..output_len]);

        i += chunk_len;
    }

    // Add end marker
    result.extend_from_slice(b"~>");

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ascii85_encode_decode() {
        let original = b"Hello, ASCII85!";

        // Encode
        let encoded = encode_ascii85(original).unwrap();

        // Decode
        let decoded = decode_ascii85(&encoded).unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn test_ascii85_zeros() {
        let zeros = &[0u8; 4];
        let encoded = encode_ascii85(zeros).unwrap();
        assert!(encoded.contains(&b'z')); // Should contain 'z' for zeros

        let decoded = decode_ascii85(&encoded).unwrap();
        assert_eq!(decoded, zeros);
    }

    #[test]
    fn test_ascii85_empty() {
        let empty: &[u8] = &[];
        let encoded = encode_ascii85(empty).unwrap();
        let decoded = decode_ascii85(&encoded).unwrap();
        assert_eq!(decoded, empty);
    }
}
