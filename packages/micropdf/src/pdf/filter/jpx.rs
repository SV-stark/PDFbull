//! JPXDecode (JPEG 2000) Filter Implementation

use crate::fitz::error::{Error, Result};

/// Decode JPEG 2000 compressed data
#[cfg(feature = "jpeg2000")]
pub fn decode_jpx(data: &[u8]) -> Result<Vec<u8>> {
    use jpeg2k::Image;

    let image = Image::from_bytes(data)
        .map_err(|e| Error::Generic(format!("JPXDecode failed: {:?}", e)))?;

    // Get the decoded image data
    // The jpeg2k crate provides access to image data through its API
    let mut result = Vec::new();

    // Get dimensions
    let width = image.width() as usize;
    let height = image.height() as usize;
    let num_components = image.components().len();

    // Reserve space for the output
    result.reserve(width * height * num_components);

    // Extract data component by component
    // JPEG2000 stores components separately, we need to interleave them
    for y in 0..height {
        for x in 0..width {
            for comp in image.components() {
                let comp_width = comp.width() as usize;
                let idx = y * comp_width + x;
                if let Some(&val) = comp.data().get(idx) {
                    // jpeg2k returns i32 values, convert to u8
                    result.push(val.clamp(0, 255) as u8);
                }
            }
        }
    }

    Ok(result)
}

#[cfg(not(feature = "jpeg2000"))]
pub fn decode_jpx(_data: &[u8]) -> Result<Vec<u8>> {
    Err(Error::Generic(
        "JPEG 2000 support not enabled. Enable 'jpeg2000' feature.".into(),
    ))
}

#[cfg(test)]
mod tests {
    #[test]
    #[cfg(not(feature = "jpeg2000"))]
    fn test_jpx_disabled() {
        use super::*;
        let data = &[0u8; 100];
        let result = decode_jpx(data);
        assert!(result.is_err());
    }
}
