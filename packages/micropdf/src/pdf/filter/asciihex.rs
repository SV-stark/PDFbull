//! ASCIIHexDecode Filter Implementation

use crate::fitz::error::{Error, Result};

/// Decode ASCIIHex encoded data
pub fn decode_ascii_hex(data: &[u8]) -> Result<Vec<u8>> {
    let mut result = Vec::with_capacity(data.len() / 2);
    let mut high_nibble: Option<u8> = None;

    for &byte in data {
        // Skip whitespace
        if byte.is_ascii_whitespace() {
            continue;
        }

        // End of data marker
        if byte == b'>' {
            break;
        }

        let nibble = match byte {
            b'0'..=b'9' => byte - b'0',
            b'A'..=b'F' => byte - b'A' + 10,
            b'a'..=b'f' => byte - b'a' + 10,
            _ => return Err(Error::Generic(format!("Invalid hex character: {}", byte))),
        };

        match high_nibble {
            None => high_nibble = Some(nibble),
            Some(high) => {
                result.push((high << 4) | nibble);
                high_nibble = None;
            }
        }
    }

    // Handle odd number of hex digits
    if let Some(high) = high_nibble {
        result.push(high << 4);
    }

    Ok(result)
}

/// Encode data with ASCIIHex
pub fn encode_ascii_hex(data: &[u8]) -> Result<Vec<u8>> {
    let mut result = Vec::with_capacity(data.len() * 2 + 1);

    for &byte in data {
        let high = (byte >> 4) & 0x0F;
        let low = byte & 0x0F;

        result.push(if high < 10 {
            b'0' + high
        } else {
            b'A' + high - 10
        });
        result.push(if low < 10 {
            b'0' + low
        } else {
            b'A' + low - 10
        });
    }

    result.push(b'>');

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_asciihex_encode_decode() {
        let original = b"Hello, Hex!";

        // Encode
        let encoded = encode_ascii_hex(original).unwrap();

        // Decode
        let decoded = decode_ascii_hex(&encoded).unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn test_asciihex_empty() {
        let empty: &[u8] = &[];
        let encoded = encode_ascii_hex(empty).unwrap();
        let decoded = decode_ascii_hex(&encoded).unwrap();
        assert_eq!(decoded, empty);
    }

    #[test]
    fn test_asciihex_odd_digits() {
        // "F" becomes "F0" when padding
        let encoded = b"F>";
        let decoded = decode_ascii_hex(encoded).unwrap();
        assert_eq!(decoded, &[0xF0]);
    }
}
