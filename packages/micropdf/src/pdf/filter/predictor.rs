//! Predictor Functions for PDF Filters

use super::params::FlateDecodeParams;
use crate::fitz::error::{Error, Result};

/// Apply PNG/TIFF predictor for decoding
pub fn apply_predictor_decode(data: &[u8], params: &FlateDecodeParams) -> Result<Vec<u8>> {
    let predictor = params.predictor;
    let colors = params.colors.max(1) as usize;
    let bits = params.bits_per_component.max(8) as usize;
    let columns = params.columns.max(1) as usize;

    // Calculate bytes per pixel and bytes per row
    let bytes_per_pixel = (colors * bits).div_ceil(8);
    let bytes_per_row = (colors * bits * columns).div_ceil(8);

    match predictor {
        1 => Ok(data.to_vec()), // No predictor
        2 => apply_tiff_predictor_decode(data, bytes_per_row, bytes_per_pixel),
        10..=15 => apply_png_predictor_decode(data, bytes_per_row, bytes_per_pixel),
        _ => Err(Error::Generic(format!(
            "Unsupported predictor: {}",
            predictor
        ))),
    }
}

/// Apply TIFF predictor (horizontal differencing)
pub fn apply_tiff_predictor_decode(
    data: &[u8],
    bytes_per_row: usize,
    bytes_per_pixel: usize,
) -> Result<Vec<u8>> {
    let mut result = Vec::with_capacity(data.len());

    for row in data.chunks(bytes_per_row) {
        let mut prev = vec![0u8; bytes_per_pixel];

        for pixel in row.chunks(bytes_per_pixel) {
            for (i, &byte) in pixel.iter().enumerate() {
                let decoded = byte.wrapping_add(prev[i]);
                result.push(decoded);
                prev[i] = decoded;
            }
        }
    }

    Ok(result)
}

/// Apply PNG predictor
pub fn apply_png_predictor_decode(
    data: &[u8],
    bytes_per_row: usize,
    bytes_per_pixel: usize,
) -> Result<Vec<u8>> {
    // PNG predictor includes a filter type byte at the start of each row
    let row_size = bytes_per_row + 1;
    let mut result = Vec::with_capacity(data.len());
    let mut prev_row = vec![0u8; bytes_per_row];

    for row_data in data.chunks(row_size) {
        if row_data.is_empty() {
            continue;
        }

        let filter_type = row_data[0];
        let row = &row_data[1..];

        if row.len() < bytes_per_row {
            // Incomplete row, pad with zeros
            let mut padded = row.to_vec();
            padded.resize(bytes_per_row, 0);
            decode_png_filter(
                filter_type,
                &padded,
                &prev_row,
                bytes_per_pixel,
                &mut result,
            )?;
        } else {
            decode_png_filter(
                filter_type,
                &row[..bytes_per_row],
                &prev_row,
                bytes_per_pixel,
                &mut result,
            )?;
        }

        // Update previous row
        let start = result.len().saturating_sub(bytes_per_row);
        prev_row.copy_from_slice(&result[start..]);
    }

    Ok(result)
}

/// Decode a single PNG filter row
pub fn decode_png_filter(
    filter_type: u8,
    row: &[u8],
    prev_row: &[u8],
    bytes_per_pixel: usize,
    output: &mut Vec<u8>,
) -> Result<()> {
    match filter_type {
        0 => {
            // None
            output.extend_from_slice(row);
        }
        1 => {
            // Sub
            for (i, &byte) in row.iter().enumerate() {
                let left = if i >= bytes_per_pixel {
                    output[output.len() - bytes_per_pixel]
                } else {
                    0
                };
                output.push(byte.wrapping_add(left));
            }
        }
        2 => {
            // Up
            for (i, &byte) in row.iter().enumerate() {
                let up = prev_row.get(i).copied().unwrap_or(0);
                output.push(byte.wrapping_add(up));
            }
        }
        3 => {
            // Average
            for (i, &byte) in row.iter().enumerate() {
                let left = if i >= bytes_per_pixel {
                    output[output.len() - bytes_per_pixel] as u32
                } else {
                    0
                };
                let up = prev_row.get(i).copied().unwrap_or(0) as u32;
                let avg = ((left + up) / 2) as u8;
                output.push(byte.wrapping_add(avg));
            }
        }
        4 => {
            // Paeth
            for (i, &byte) in row.iter().enumerate() {
                let left = if i >= bytes_per_pixel {
                    output[output.len() - bytes_per_pixel]
                } else {
                    0
                };
                let up = prev_row.get(i).copied().unwrap_or(0);
                let up_left = if i >= bytes_per_pixel {
                    prev_row.get(i - bytes_per_pixel).copied().unwrap_or(0)
                } else {
                    0
                };
                let paeth = paeth_predictor(left, up, up_left);
                output.push(byte.wrapping_add(paeth));
            }
        }
        _ => {
            return Err(Error::Generic(format!(
                "Unknown PNG filter type: {}",
                filter_type
            )));
        }
    }

    Ok(())
}

/// Paeth predictor function
pub fn paeth_predictor(a: u8, b: u8, c: u8) -> u8 {
    let a = a as i32;
    let b = b as i32;
    let c = c as i32;

    let p = a + b - c;
    let pa = (p - a).abs();
    let pb = (p - b).abs();
    let pc = (p - c).abs();

    if pa <= pb && pa <= pc {
        a as u8
    } else if pb <= pc {
        b as u8
    } else {
        c as u8
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_paeth_predictor() {
        // Test cases for Paeth predictor
        // a=10, b=20, c=15: p=15, pa=5, pb=5, pc=0 -> c=15
        assert_eq!(paeth_predictor(10, 20, 15), 15);
        // a=20, b=10, c=15: p=15, pa=5, pb=5, pc=0 -> c=15
        assert_eq!(paeth_predictor(20, 10, 15), 15);
        // All equal
        assert_eq!(paeth_predictor(10, 10, 10), 10);
        // Zeros
        assert_eq!(paeth_predictor(0, 0, 0), 0);
        // Max values
        assert_eq!(paeth_predictor(255, 255, 255), 255);
        // a is closest: a=10, b=5, c=0: p=15, pa=5, pb=10, pc=15 -> a=10
        assert_eq!(paeth_predictor(10, 5, 0), 10);
        // b is closest: a=5, b=10, c=0: p=15, pa=10, pb=5, pc=15 -> b=10
        assert_eq!(paeth_predictor(5, 10, 0), 10);
    }

    #[test]
    fn test_apply_predictor_decode_no_predictor() {
        let data = vec![1, 2, 3, 4, 5];
        let params = FlateDecodeParams {
            predictor: 1,
            colors: 1,
            bits_per_component: 8,
            columns: 5,
        };
        let result = apply_predictor_decode(&data, &params).unwrap();
        assert_eq!(result, data);
    }

    #[test]
    fn test_apply_predictor_decode_tiff() {
        let data = vec![10, 5, 3, 2]; // TIFF predictor with differences
        let params = FlateDecodeParams {
            predictor: 2,
            colors: 1,
            bits_per_component: 8,
            columns: 4,
        };
        let result = apply_predictor_decode(&data, &params).unwrap();
        // TIFF horizontal differencing: each byte is added to previous
        // 10, 10+5=15, 15+3=18, 18+2=20
        assert_eq!(result, vec![10, 15, 18, 20]);
    }

    #[test]
    fn test_apply_predictor_decode_unsupported() {
        let data = vec![1, 2, 3];
        let params = FlateDecodeParams {
            predictor: 99, // Unsupported
            colors: 1,
            bits_per_component: 8,
            columns: 3,
        };
        let result = apply_predictor_decode(&data, &params);
        assert!(result.is_err());
    }

    #[test]
    fn test_apply_tiff_predictor_decode() {
        // Simple TIFF predictor test with 1 byte per pixel
        let data = vec![10, 5, 3];
        let result = apply_tiff_predictor_decode(&data, 3, 1).unwrap();
        assert_eq!(result, vec![10, 15, 18]);
    }

    #[test]
    fn test_apply_tiff_predictor_decode_multi_pixel() {
        // TIFF predictor with 2 bytes per pixel (e.g., RGB color)
        let data = vec![10, 20, 5, 10, 3, 5];
        let result = apply_tiff_predictor_decode(&data, 6, 2).unwrap();
        // First pixel: [10, 20]
        // Second pixel: [10+5=15, 20+10=30]
        // Third pixel: [15+3=18, 30+5=35]
        assert_eq!(result, vec![10, 20, 15, 30, 18, 35]);
    }

    #[test]
    fn test_decode_png_filter_none() {
        let row = vec![1, 2, 3, 4];
        let prev_row = vec![0, 0, 0, 0];
        let mut output = Vec::new();
        decode_png_filter(0, &row, &prev_row, 1, &mut output).unwrap();
        assert_eq!(output, vec![1, 2, 3, 4]);
    }

    #[test]
    fn test_decode_png_filter_sub() {
        let row = vec![10, 5, 3, 2];
        let prev_row = vec![0, 0, 0, 0];
        let mut output = Vec::new();
        decode_png_filter(1, &row, &prev_row, 1, &mut output).unwrap();
        // Sub filter: each byte is added to the byte to its left
        // 10, 10+5=15, 15+3=18, 18+2=20
        assert_eq!(output, vec![10, 15, 18, 20]);
    }

    #[test]
    fn test_decode_png_filter_up() {
        let row = vec![10, 5, 3, 2];
        let prev_row = vec![5, 10, 15, 20];
        let mut output = Vec::new();
        decode_png_filter(2, &row, &prev_row, 1, &mut output).unwrap();
        // Up filter: each byte is added to the byte above it
        // 10+5=15, 5+10=15, 3+15=18, 2+20=22
        assert_eq!(output, vec![15, 15, 18, 22]);
    }

    #[test]
    fn test_decode_png_filter_average() {
        let row = vec![10, 5, 3, 2];
        let prev_row = vec![4, 8, 12, 16];
        let mut output = Vec::new();
        decode_png_filter(3, &row, &prev_row, 1, &mut output).unwrap();
        // Average filter: each byte is added to the average of left and up
        // 10+(0+4)/2=12, 5+(12+8)/2=15, 3+(15+12)/2=16, 2+(16+16)/2=18
        assert_eq!(output, vec![12, 15, 16, 18]);
    }

    #[test]
    fn test_decode_png_filter_paeth() {
        let row = vec![10, 5, 3, 2];
        let prev_row = vec![5, 10, 15, 20];
        let mut output = Vec::new();
        decode_png_filter(4, &row, &prev_row, 1, &mut output).unwrap();
        // Paeth filter uses the Paeth predictor function
        assert_eq!(output.len(), 4);
    }

    #[test]
    fn test_decode_png_filter_unknown() {
        let row = vec![1, 2, 3];
        let prev_row = vec![0, 0, 0];
        let mut output = Vec::new();
        let result = decode_png_filter(99, &row, &prev_row, 1, &mut output);
        assert!(result.is_err());
    }

    #[test]
    fn test_apply_png_predictor_decode() {
        // PNG predictor with filter type byte at start of each row
        // Filter type 0 (None) for a simple row
        let data = vec![0, 10, 20, 30]; // Filter type 0, then 3 bytes
        let result = apply_png_predictor_decode(&data, 3, 1).unwrap();
        assert_eq!(result, vec![10, 20, 30]);
    }

    #[test]
    fn test_apply_png_predictor_decode_sub_filter() {
        // PNG predictor with Sub filter (type 1)
        let data = vec![1, 10, 5, 3]; // Filter type 1, then differences
        let result = apply_png_predictor_decode(&data, 3, 1).unwrap();
        assert_eq!(result, vec![10, 15, 18]);
    }

    #[test]
    fn test_apply_png_predictor_decode_multiple_rows() {
        // Two rows with None filter
        let data = vec![
            0, 10, 20, 30, // Row 1: filter type 0, data
            0, 40, 50, 60, // Row 2: filter type 0, data
        ];
        let result = apply_png_predictor_decode(&data, 3, 1).unwrap();
        assert_eq!(result, vec![10, 20, 30, 40, 50, 60]);
    }

    #[test]
    fn test_apply_png_predictor_decode_incomplete_row() {
        // Incomplete last row (should be padded with zeros)
        let data = vec![0, 10, 20]; // Filter type 0, but only 2 bytes (expecting 3)
        let result = apply_png_predictor_decode(&data, 3, 1).unwrap();
        assert_eq!(result, vec![10, 20, 0]); // Padded with zero
    }

    #[test]
    fn test_apply_png_predictor_decode_empty_row() {
        // Empty row should be skipped
        let data = vec![];
        let result = apply_png_predictor_decode(&data, 3, 1).unwrap();
        assert_eq!(result, Vec::<u8>::new());
    }

    #[test]
    fn test_apply_predictor_decode_png_range() {
        // Test PNG predictor range (10-15 all map to PNG predictor)
        for predictor in 10..=15 {
            let data = vec![0, 1, 2, 3]; // Filter type 0 (None)
            let params = FlateDecodeParams {
                predictor,
                colors: 1,
                bits_per_component: 8,
                columns: 3,
            };
            let result = apply_predictor_decode(&data, &params).unwrap();
            assert_eq!(result, vec![1, 2, 3]);
        }
    }
}
