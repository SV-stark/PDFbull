//! Image handling - loading, decoding, and rendering images
//!
//! Provides image representation with support for various formats and color spaces.

use crate::fitz::colorspace::Colorspace;
use crate::fitz::error::{Error, Result};
use crate::fitz::geometry::{IRect, Matrix};
use crate::fitz::pixmap::Pixmap;

/// Image format/compression type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFormat {
    /// Raw uncompressed image data
    Raw,
    /// JPEG (DCT) compressed
    Jpeg,
    /// JPEG2000 (JPX) compressed
    Jpeg2000,
    /// JBIG2 compressed (monochrome)
    Jbig2,
    /// CCITT Fax compressed
    Ccitt,
    /// Flate/ZIP compressed
    Flate,
    /// LZW compressed
    Lzw,
    /// Run-length encoded
    RunLength,
}

/// Image mask type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaskType {
    /// No mask
    None,
    /// Hard mask (binary transparency)
    ImageMask,
    /// Soft mask (alpha channel)
    SoftMask,
    /// Stencil mask (pixels are either on or off)
    Stencil,
}

/// Image interpolation mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Interpolation {
    /// Nearest neighbor (fast, pixelated)
    Nearest,
    /// Linear interpolation (balanced)
    #[default]
    Linear,
    /// Bicubic interpolation (smooth, slow)
    Bicubic,
}

/// Complete image representation
#[derive(Clone)]
pub struct Image {
    /// Image width in pixels
    width: i32,
    /// Image height in pixels
    height: i32,
    /// Bits per component (1, 2, 4, 8, 16)
    bpc: u8,
    /// Number of color components
    n: u8,
    /// Colorspace
    colorspace: Option<Colorspace>,
    /// Image data (raw or compressed)
    data: Vec<u8>,
    /// Image format/compression
    format: ImageFormat,
    /// Mask type
    mask_type: MaskType,
    /// Mask image (for ImageMask or SoftMask)
    mask: Option<Box<Image>>,
    /// Horizontal resolution (DPI)
    xres: i32,
    /// Vertical resolution (DPI)
    yres: i32,
    /// Interpolation flag
    interpolate: bool,
    /// Decoded pixmap cache
    pixmap: Option<Pixmap>,
}

impl Image {
    /// Create a new image with raw data
    pub fn new(width: i32, height: i32, pixmap: Option<Pixmap>) -> Self {
        Self {
            width,
            height,
            bpc: 8,
            n: 3,
            colorspace: Some(Colorspace::device_rgb()),
            data: Vec::new(),
            format: ImageFormat::Raw,
            mask_type: MaskType::None,
            mask: None,
            xres: 96,
            yres: 96,
            interpolate: true,
            pixmap,
        }
    }

    /// Create image from raw pixel data
    pub fn from_raw(
        width: i32,
        height: i32,
        bpc: u8,
        colorspace: Colorspace,
        data: Vec<u8>,
    ) -> Result<Self> {
        if width <= 0 || height <= 0 {
            return Err(Error::Argument("Image dimensions must be positive".into()));
        }

        let n = colorspace.n();
        let expected_size = ((width * height * n as i32 * bpc as i32) / 8) as usize;

        if data.len() < expected_size {
            return Err(Error::Argument(format!(
                "Insufficient image data: expected {} bytes, got {}",
                expected_size,
                data.len()
            )));
        }

        Ok(Self {
            width,
            height,
            bpc,
            n,
            colorspace: Some(colorspace),
            data,
            format: ImageFormat::Raw,
            mask_type: MaskType::None,
            mask: None,
            xres: 96,
            yres: 96,
            interpolate: true,
            pixmap: None,
        })
    }

    /// Create compressed image
    pub fn from_compressed(
        width: i32,
        height: i32,
        bpc: u8,
        colorspace: Option<Colorspace>,
        format: ImageFormat,
        data: Vec<u8>,
    ) -> Result<Self> {
        if width <= 0 || height <= 0 {
            return Err(Error::Argument("Image dimensions must be positive".into()));
        }

        let n = colorspace.as_ref().map(|cs| cs.n()).unwrap_or(1);

        Ok(Self {
            width,
            height,
            bpc,
            n,
            colorspace,
            data,
            format,
            mask_type: MaskType::None,
            mask: None,
            xres: 96,
            yres: 96,
            interpolate: true,
            pixmap: None,
        })
    }

    /// Create an image mask (stencil)
    pub fn from_mask(width: i32, height: i32, data: Vec<u8>) -> Result<Self> {
        if width <= 0 || height <= 0 {
            return Err(Error::Argument("Image dimensions must be positive".into()));
        }

        Ok(Self {
            width,
            height,
            bpc: 1,
            n: 1,
            colorspace: None,
            data,
            format: ImageFormat::Raw,
            mask_type: MaskType::Stencil,
            mask: None,
            xres: 96,
            yres: 96,
            interpolate: false,
            pixmap: None,
        })
    }

    /// Get image width
    pub fn width(&self) -> i32 {
        self.width
    }

    /// Get image height
    pub fn height(&self) -> i32 {
        self.height
    }

    /// Get bits per component
    pub fn bpc(&self) -> u8 {
        self.bpc
    }

    /// Get number of components
    pub fn n(&self) -> u8 {
        self.n
    }

    /// Get colorspace
    pub fn colorspace(&self) -> Option<&Colorspace> {
        self.colorspace.as_ref()
    }

    /// Get image format
    pub fn format(&self) -> ImageFormat {
        self.format
    }

    /// Get mask type
    pub fn mask_type(&self) -> MaskType {
        self.mask_type
    }

    /// Get resolution
    pub fn resolution(&self) -> (i32, i32) {
        (self.xres, self.yres)
    }

    /// Set resolution
    pub fn set_resolution(&mut self, xres: i32, yres: i32) {
        self.xres = xres;
        self.yres = yres;
    }

    /// Get interpolation flag
    pub fn interpolate(&self) -> bool {
        self.interpolate
    }

    /// Set interpolation flag
    pub fn set_interpolate(&mut self, interpolate: bool) {
        self.interpolate = interpolate;
    }

    /// Set mask
    pub fn set_mask(&mut self, mask: Option<Image>) {
        if let Some(m) = mask {
            self.mask_type = if m.mask_type == MaskType::Stencil {
                MaskType::ImageMask
            } else {
                MaskType::SoftMask
            };
            self.mask = Some(Box::new(m));
        } else {
            self.mask_type = MaskType::None;
            self.mask = None;
        }
    }

    /// Get mask
    pub fn mask(&self) -> Option<&Image> {
        self.mask.as_deref()
    }

    /// Get raw image data
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Check if image needs decoding
    pub fn is_compressed(&self) -> bool {
        self.format != ImageFormat::Raw
    }

    /// Decode image to pixmap (if compressed)
    pub fn decode(&mut self) -> Result<()> {
        if !self.is_compressed() {
            return Ok(());
        }

        // Decode based on format
        let decoded_data = match self.format {
            ImageFormat::Flate => {
                // Use flate2 to decompress
                use flate2::read::ZlibDecoder;
                use std::io::Read;

                let mut decoder = ZlibDecoder::new(&self.data[..]);
                let mut decoded = Vec::new();
                decoder
                    .read_to_end(&mut decoded)
                    .map_err(|e| Error::Generic(format!("Flate decode failed: {}", e)))?;
                decoded
            }
            ImageFormat::Jpeg => {
                use crate::pdf::filter::decode_dct;
                decode_dct(&self.data, None)?
            }
            ImageFormat::Jpeg2000 => {
                use crate::pdf::filter::decode_jpx;
                decode_jpx(&self.data)?
            }
            ImageFormat::Jbig2 => {
                use crate::pdf::filter::decode_jbig2;
                decode_jbig2(&self.data, None)?
            }
            ImageFormat::Ccitt => {
                use crate::pdf::filter::{CCITTFaxDecodeParams, decode_ccitt_fax};
                // Use default CCITT parameters - caller should provide proper params
                let params = CCITTFaxDecodeParams::default();
                decode_ccitt_fax(&self.data, &params)?
            }
            ImageFormat::Lzw => {
                use crate::pdf::filter::decode_lzw;
                decode_lzw(&self.data, None)?
            }
            ImageFormat::RunLength => {
                use crate::pdf::filter::decode_run_length;
                decode_run_length(&self.data)?
            }
            ImageFormat::Raw => {
                return Ok(());
            }
        };

        // Update to raw format
        self.data = decoded_data;
        self.format = ImageFormat::Raw;
        Ok(())
    }

    /// Get or create pixmap from image
    pub fn pixmap(&mut self) -> Result<&Pixmap> {
        if self.pixmap.is_none() {
            self.to_pixmap()?;
        }
        Ok(self.pixmap.as_ref().unwrap())
    }

    /// Convert image to pixmap
    pub fn to_pixmap(&mut self) -> Result<Pixmap> {
        // Decode if compressed
        if self.is_compressed() {
            self.decode()?;
        }

        // Create pixmap
        let colorspace = self.colorspace.clone();
        let has_alpha = self.mask.is_some();

        let mut pixmap = Pixmap::new(colorspace, self.width, self.height, has_alpha)?;

        // Copy image data to pixmap
        if !self.data.is_empty() {
            // Simplified: just copy data
            // In reality, we'd need to handle different bpc values, stride, etc.
            let samples = pixmap.samples_mut();
            let copy_len = samples.len().min(self.data.len());
            samples[..copy_len].copy_from_slice(&self.data[..copy_len]);
        }

        // Apply mask if present
        if self.mask.is_some() {
            // Take mask temporarily to avoid borrow issues
            let mut mask = self.mask.take().unwrap();
            Self::apply_mask_static(&mut pixmap, &mut mask)?;
            self.mask = Some(mask);
        }

        self.pixmap = Some(pixmap.clone());
        Ok(pixmap)
    }

    /// Apply mask to pixmap (static version to avoid borrow issues)
    fn apply_mask_static(pixmap: &mut Pixmap, mask: &mut Image) -> Result<()> {
        // Ensure mask is decoded
        if mask.is_compressed() {
            mask.decode()?;
        }

        // For simplicity, just handle stencil masks
        if mask.mask_type == MaskType::Stencil {
            // Apply binary mask
            let mask_data = mask.data();
            let mask_width = mask.width;
            let mask_height = mask.height;
            let pixmap_width = pixmap.width();
            let pixmap_height = pixmap.height();
            let pixmap_n = pixmap.n();
            let has_alpha = pixmap.has_alpha();
            let stride = pixmap.stride() as i32;
            let samples = pixmap.samples_mut();

            for y in 0..pixmap_height.min(mask_height) {
                for x in 0..pixmap_width.min(mask_width) {
                    let mask_byte = mask_data[(y * mask_width + x) as usize / 8];
                    let mask_bit = (mask_byte >> (7 - (x % 8))) & 1;

                    if mask_bit == 0 {
                        // Make pixel transparent
                        let offset = (y * stride + x * pixmap_n as i32) as usize;
                        if offset + pixmap_n as usize <= samples.len() {
                            // Set alpha to 0
                            if has_alpha {
                                samples[offset + pixmap_n as usize - 1] = 0;
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Get scaled pixmap
    pub fn get_scaled_pixmap(&mut self, ctm: &Matrix, subarea: Option<IRect>) -> Result<Pixmap> {
        // Calculate target dimensions from CTM
        let scale_x = (ctm.a * ctm.a + ctm.b * ctm.b).sqrt();
        let scale_y = (ctm.c * ctm.c + ctm.d * ctm.d).sqrt();

        let width = self.width;
        let height = self.height;
        let target_width = (width as f32 * scale_x) as i32;
        let target_height = (height as f32 * scale_y) as i32;

        // Get base pixmap
        let base_pixmap = self.pixmap()?.clone();

        // If scaling is close to 1:1 and no subarea, return base pixmap
        if subarea.is_none()
            && (target_width - width).abs() < 2
            && (target_height - height).abs() < 2
        {
            return Ok(base_pixmap);
        }

        // Perform actual image scaling
        use image::{ImageBuffer, RgbaImage, imageops::FilterType};

        // Convert pixmap to image buffer (assuming RGBA format)
        let img: RgbaImage =
            ImageBuffer::from_raw(width as u32, height as u32, base_pixmap.samples().to_vec())
                .ok_or_else(|| Error::Generic("Failed to create image buffer".into()))?;

        // Resize image
        let scaled_img = image::imageops::resize(
            &img,
            target_width as u32,
            target_height as u32,
            FilterType::Lanczos3, // High-quality scaling
        );

        // Convert back to pixmap
        let mut scaled_pixmap = Pixmap::new(
            base_pixmap.colorspace().cloned(),
            target_width,
            target_height,
            base_pixmap.has_alpha(),
        )?;

        // Copy scaled image data
        let scaled_data = scaled_img.into_raw();
        scaled_pixmap.samples_mut().copy_from_slice(&scaled_data);

        Ok(scaled_pixmap)
    }

    /// Get memory usage
    pub fn size(&self) -> usize {
        let mut size = std::mem::size_of::<Self>();
        size += self.data.len();
        if let Some(ref pixmap) = self.pixmap {
            size += pixmap.samples().len();
        }
        if let Some(ref mask) = self.mask {
            size += mask.size();
        }
        size
    }

    /// Get X resolution (DPI)
    pub fn xres(&self) -> i32 {
        self.xres
    }

    /// Get Y resolution (DPI)
    pub fn yres(&self) -> i32 {
        self.yres
    }

    /// Check if this image is a mask
    pub fn is_mask(&self) -> bool {
        self.mask_type != MaskType::None || self.n == 1 && self.bpc == 1
    }

    /// Create image from image file data (auto-detects format)
    pub fn from_data(data: &[u8]) -> Result<Self> {
        if data.is_empty() {
            return Err(Error::Argument("Empty image data".into()));
        }

        // Use image crate to decode the image format
        use image::ImageReader;
        use std::io::Cursor;

        let reader = ImageReader::new(Cursor::new(data))
            .with_guessed_format()
            .map_err(|e| Error::Generic(format!("Failed to detect image format: {}", e)))?;

        let img = reader
            .decode()
            .map_err(|e| Error::Generic(format!("Failed to decode image: {}", e)))?;

        // Convert to RGBA
        let rgba = img.to_rgba8();
        let (width, height) = rgba.dimensions();

        Ok(Self {
            width: width as i32,
            height: height as i32,
            bpc: 8,
            n: 4, // RGBA
            colorspace: Some(Colorspace::device_rgb()),
            data: rgba.into_raw(),
            format: ImageFormat::Raw,
            mask_type: MaskType::None,
            mask: None,
            xres: 96,
            yres: 96,
            interpolate: true,
            pixmap: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_image_new() {
        let img = Image::new(100, 50, None);
        assert_eq!(img.width(), 100);
        assert_eq!(img.height(), 50);
        assert_eq!(img.bpc(), 8);
        assert_eq!(img.n(), 3);
    }

    #[test]
    fn test_image_from_raw() {
        let width = 10;
        let height = 10;
        let bpc = 8;
        let cs = Colorspace::device_rgb();
        let data = vec![0u8; width as usize * height as usize * 3];

        let img = Image::from_raw(width, height, bpc, cs, data).unwrap();
        assert_eq!(img.width(), width);
        assert_eq!(img.height(), height);
        assert_eq!(img.bpc(), bpc);
        assert_eq!(img.format(), ImageFormat::Raw);
    }

    #[test]
    fn test_image_from_raw_invalid_dimensions() {
        let cs = Colorspace::device_rgb();
        let data = vec![0u8; 100];

        assert!(Image::from_raw(0, 10, 8, cs.clone(), data.clone()).is_err());
        assert!(Image::from_raw(10, 0, 8, cs.clone(), data.clone()).is_err());
        assert!(Image::from_raw(-10, 10, 8, cs, data).is_err());
    }

    #[test]
    fn test_image_from_raw_insufficient_data() {
        let cs = Colorspace::device_rgb();
        let data = vec![0u8; 10]; // Not enough for 10x10 RGB

        assert!(Image::from_raw(10, 10, 8, cs, data).is_err());
    }

    #[test]
    fn test_image_from_compressed() {
        let width = 10;
        let height = 10;
        let cs = Colorspace::device_rgb();
        let data = vec![1, 2, 3, 4, 5]; // Compressed data

        let img =
            Image::from_compressed(width, height, 8, Some(cs), ImageFormat::Flate, data).unwrap();

        assert_eq!(img.width(), width);
        assert_eq!(img.height(), height);
        assert_eq!(img.format(), ImageFormat::Flate);
        assert!(img.is_compressed());
    }

    #[test]
    fn test_image_from_mask() {
        let width = 8;
        let height = 8;
        let data = vec![0xFFu8; 8]; // 8x8 = 64 bits = 8 bytes

        let img = Image::from_mask(width, height, data).unwrap();
        assert_eq!(img.width(), width);
        assert_eq!(img.height(), height);
        assert_eq!(img.bpc(), 1);
        assert_eq!(img.mask_type(), MaskType::Stencil);
        assert!(!img.interpolate());
    }

    #[test]
    fn test_image_resolution() {
        let mut img = Image::new(100, 100, None);
        assert_eq!(img.resolution(), (96, 96));

        img.set_resolution(300, 300);
        assert_eq!(img.resolution(), (300, 300));
    }

    #[test]
    fn test_image_interpolation() {
        let mut img = Image::new(100, 100, None);
        assert!(img.interpolate());

        img.set_interpolate(false);
        assert!(!img.interpolate());
    }

    #[test]
    fn test_image_set_mask() {
        let mut img = Image::new(100, 100, None);
        assert_eq!(img.mask_type(), MaskType::None);

        let mask = Image::from_mask(100, 100, vec![0xFF; 1250]).unwrap();
        img.set_mask(Some(mask));
        assert_eq!(img.mask_type(), MaskType::ImageMask);
        assert!(img.mask().is_some());

        img.set_mask(None);
        assert_eq!(img.mask_type(), MaskType::None);
        assert!(img.mask().is_none());
    }

    #[test]
    fn test_image_format_types() {
        assert_eq!(ImageFormat::Raw, ImageFormat::Raw);
        assert_ne!(ImageFormat::Raw, ImageFormat::Jpeg);
    }

    #[test]
    fn test_mask_types() {
        assert_eq!(MaskType::None, MaskType::None);
        assert_ne!(MaskType::None, MaskType::Stencil);
    }

    #[test]
    fn test_interpolation_default() {
        assert_eq!(Interpolation::default(), Interpolation::Linear);
    }

    #[test]
    fn test_image_data() {
        let data = vec![1, 2, 3, 4, 5];
        let img = Image::from_compressed(10, 10, 8, None, ImageFormat::Raw, data.clone()).unwrap();

        assert_eq!(img.data(), &data[..]);
    }

    #[test]
    fn test_image_size() {
        let img = Image::new(100, 100, None);
        let size = img.size();
        assert!(size > 0);
    }

    #[test]
    fn test_image_colorspace() {
        let cs = Colorspace::device_rgb();
        let data = vec![0u8; 300];
        let img = Image::from_raw(10, 10, 8, cs.clone(), data).unwrap();

        assert!(img.colorspace().is_some());
        assert_eq!(img.n(), 3);
    }

    #[test]
    fn test_image_compressed_flag() {
        let raw_img = Image::new(10, 10, None);
        assert!(!raw_img.is_compressed());

        let comp_img =
            Image::from_compressed(10, 10, 8, None, ImageFormat::Flate, vec![1, 2, 3]).unwrap();
        assert!(comp_img.is_compressed());
    }
}
