//! C FFI for 1-bit bitmap - MuPDF compatible
//! Safe Rust implementation of fz_bitmap

use super::{Handle, HandleStore};
use std::sync::LazyLock;

/// Halftone algorithm
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HalftoneType {
    /// No halftoning (simple threshold)
    None = 0,
    /// Floyd-Steinberg error diffusion
    FloydSteinberg = 1,
    /// Ordered dithering (Bayer matrix)
    Ordered = 2,
    /// Clustered dot halftone
    ClusteredDot = 3,
    /// Stochastic/blue noise dithering
    Stochastic = 4,
}

/// Compression type for bitmap output
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BitmapCompression {
    /// No compression (raw)
    None = 0,
    /// Run-length encoding
    RLE = 1,
    /// CCITT Group 3 (fax)
    CCITTGroup3 = 2,
    /// CCITT Group 4 (fax)
    CCITTGroup4 = 3,
    /// PackBits (TIFF)
    PackBits = 4,
}

/// 1-bit bitmap structure
#[derive(Debug, Clone)]
pub struct Bitmap {
    /// Width in pixels
    pub width: i32,
    /// Height in pixels
    pub height: i32,
    /// Stride (bytes per row, including padding)
    pub stride: i32,
    /// X resolution (DPI)
    pub x_res: i32,
    /// Y resolution (DPI)
    pub y_res: i32,
    /// Bitmap data (1 bit per pixel, MSB first)
    pub data: Vec<u8>,
    /// Whether 1 = black (default) or 1 = white
    pub invert: bool,
}

impl Default for Bitmap {
    fn default() -> Self {
        Self {
            width: 0,
            height: 0,
            stride: 0,
            x_res: 72,
            y_res: 72,
            data: Vec::new(),
            invert: false,
        }
    }
}

/// Halftone screen parameters
#[derive(Debug, Clone)]
pub struct HalftoneScreen {
    /// Screen frequency (lines per inch)
    pub frequency: f32,
    /// Screen angle (degrees)
    pub angle: f32,
    /// Spot function type
    pub spot_type: i32,
}

impl Default for HalftoneScreen {
    fn default() -> Self {
        Self {
            frequency: 150.0,
            angle: 45.0,
            spot_type: 0,
        }
    }
}

/// Global bitmap storage
pub static BITMAPS: LazyLock<HandleStore<Bitmap>> = LazyLock::new(HandleStore::new);

// ============================================================================
// Bitmap Creation
// ============================================================================

/// Create a new empty bitmap
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_bitmap(
    _ctx: Handle,
    width: i32,
    height: i32,
    x_res: i32,
    y_res: i32,
) -> Handle {
    if width <= 0 || height <= 0 {
        return 0;
    }

    // Calculate stride (bytes per row, rounded up to nearest byte)
    let stride = (width + 7) / 8;
    let data_size = (stride * height) as usize;

    let bitmap = Bitmap {
        width,
        height,
        stride,
        x_res: if x_res > 0 { x_res } else { 72 },
        y_res: if y_res > 0 { y_res } else { 72 },
        data: vec![0u8; data_size],
        invert: false,
    };

    BITMAPS.insert(bitmap)
}

/// Create bitmap from pixmap using threshold
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_bitmap_from_pixmap(
    _ctx: Handle,
    pixmap: Handle,
    threshold: i32,
) -> Handle {
    // Get pixmap dimensions from pixmap module
    let (width, height) = if let Some(pix) = super::PIXMAPS.get(pixmap) {
        if let Ok(guard) = pix.lock() {
            (guard.w(), guard.h())
        } else {
            return 0;
        }
    } else {
        return 0;
    };

    let stride = (width + 7) / 8;
    let data_size = (stride * height) as usize;
    let mut data = vec![0u8; data_size];

    // Get pixmap data and convert using threshold
    if let Some(pix) = super::PIXMAPS.get(pixmap) {
        if let Ok(guard) = pix.lock() {
            let n = guard.n() as usize;
            let pix_stride = guard.stride() as usize;
            let samples = guard.samples();
            let thresh = threshold.clamp(0, 255) as u8;

            for y in 0..height as usize {
                for x in 0..width as usize {
                    let pix_offset = y * pix_stride + x * n;

                    // Calculate luminance (simple average for grayscale)
                    let lum = if n >= 3 && pix_offset + 2 < samples.len() {
                        let r = samples[pix_offset] as u32;
                        let g = samples[pix_offset + 1] as u32;
                        let b = samples[pix_offset + 2] as u32;
                        ((r * 299 + g * 587 + b * 114) / 1000) as u8
                    } else if pix_offset < samples.len() {
                        samples[pix_offset]
                    } else {
                        0
                    };

                    // Set bit if below threshold (black)
                    if lum < thresh {
                        let byte_idx = y * stride as usize + x / 8;
                        let bit_idx = 7 - (x % 8);
                        if byte_idx < data.len() {
                            data[byte_idx] |= 1 << bit_idx;
                        }
                    }
                }
            }
        }
    }

    let bitmap = Bitmap {
        width,
        height,
        stride,
        x_res: 72,
        y_res: 72,
        data,
        invert: false,
    };

    BITMAPS.insert(bitmap)
}

/// Create bitmap using halftone algorithm
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_bitmap_from_pixmap_halftone(
    _ctx: Handle,
    pixmap: Handle,
    halftone_type: i32,
) -> Handle {
    let (width, height) = if let Some(pix) = super::PIXMAPS.get(pixmap) {
        if let Ok(guard) = pix.lock() {
            (guard.w(), guard.h())
        } else {
            return 0;
        }
    } else {
        return 0;
    };

    let stride = (width + 7) / 8;
    let data_size = (stride * height) as usize;
    let mut data = vec![0u8; data_size];

    let ht = match halftone_type {
        1 => HalftoneType::FloydSteinberg,
        2 => HalftoneType::Ordered,
        3 => HalftoneType::ClusteredDot,
        4 => HalftoneType::Stochastic,
        _ => HalftoneType::None,
    };

    // Get pixmap data and apply halftoning
    if let Some(pix) = super::PIXMAPS.get(pixmap) {
        if let Ok(guard) = pix.lock() {
            let n = guard.n() as usize;
            let pix_stride = guard.stride() as usize;
            let samples = guard.samples();

            // Create grayscale buffer for error diffusion
            let mut gray_buffer: Vec<i32> = Vec::with_capacity((width * height) as usize);

            for y in 0..height as usize {
                for x in 0..width as usize {
                    let pix_offset = y * pix_stride + x * n;
                    let lum = if n >= 3 && pix_offset + 2 < samples.len() {
                        let r = samples[pix_offset] as i32;
                        let g = samples[pix_offset + 1] as i32;
                        let b = samples[pix_offset + 2] as i32;
                        (r * 299 + g * 587 + b * 114) / 1000
                    } else if pix_offset < samples.len() {
                        samples[pix_offset] as i32
                    } else {
                        0
                    };
                    gray_buffer.push(lum);
                }
            }

            match ht {
                HalftoneType::FloydSteinberg => {
                    floyd_steinberg_dither(&mut gray_buffer, width, height, &mut data, stride);
                }
                HalftoneType::Ordered => {
                    ordered_dither(&gray_buffer, width, height, &mut data, stride);
                }
                _ => {
                    // Simple threshold for other types
                    threshold_convert(&gray_buffer, width, height, &mut data, stride, 128);
                }
            }
        }
    }

    let bitmap = Bitmap {
        width,
        height,
        stride,
        x_res: 72,
        y_res: 72,
        data,
        invert: false,
    };

    BITMAPS.insert(bitmap)
}

/// Floyd-Steinberg error diffusion dithering
fn floyd_steinberg_dither(gray: &mut [i32], width: i32, height: i32, out: &mut [u8], stride: i32) {
    let w = width as usize;
    let h = height as usize;

    for y in 0..h {
        for x in 0..w {
            let idx = y * w + x;
            let old_val = gray[idx].clamp(0, 255);
            let new_val = if old_val < 128 { 0 } else { 255 };
            let error = old_val - new_val;

            // Set output bit
            if new_val == 0 {
                let byte_idx = y * stride as usize + x / 8;
                let bit_idx = 7 - (x % 8);
                if byte_idx < out.len() {
                    out[byte_idx] |= 1 << bit_idx;
                }
            }

            // Distribute error to neighbors
            if x + 1 < w {
                gray[idx + 1] += error * 7 / 16;
            }
            if y + 1 < h {
                if x > 0 {
                    gray[idx + w - 1] += error * 3 / 16;
                }
                gray[idx + w] += error * 5 / 16;
                if x + 1 < w {
                    gray[idx + w + 1] += error * 1 / 16;
                }
            }
        }
    }
}

/// Ordered dithering with Bayer matrix
fn ordered_dither(gray: &[i32], width: i32, height: i32, out: &mut [u8], stride: i32) {
    // 4x4 Bayer matrix (normalized to 0-255 range)
    const BAYER_4X4: [[i32; 4]; 4] = [
        [0, 128, 32, 160],
        [192, 64, 224, 96],
        [48, 176, 16, 144],
        [240, 112, 208, 80],
    ];

    let w = width as usize;
    let h = height as usize;

    for y in 0..h {
        for x in 0..w {
            let idx = y * w + x;
            let threshold = BAYER_4X4[y % 4][x % 4];

            if gray[idx] < threshold {
                let byte_idx = y * stride as usize + x / 8;
                let bit_idx = 7 - (x % 8);
                if byte_idx < out.len() {
                    out[byte_idx] |= 1 << bit_idx;
                }
            }
        }
    }
}

/// Simple threshold conversion
fn threshold_convert(
    gray: &[i32],
    width: i32,
    height: i32,
    out: &mut [u8],
    stride: i32,
    threshold: i32,
) {
    let w = width as usize;
    let h = height as usize;

    for y in 0..h {
        for x in 0..w {
            let idx = y * w + x;
            if gray[idx] < threshold {
                let byte_idx = y * stride as usize + x / 8;
                let bit_idx = 7 - (x % 8);
                if byte_idx < out.len() {
                    out[byte_idx] |= 1 << bit_idx;
                }
            }
        }
    }
}

// ============================================================================
// Bitmap Properties
// ============================================================================

/// Get bitmap width
#[unsafe(no_mangle)]
pub extern "C" fn fz_bitmap_width(_ctx: Handle, bitmap: Handle) -> i32 {
    if let Some(bm) = BITMAPS.get(bitmap) {
        if let Ok(guard) = bm.lock() {
            return guard.width;
        }
    }
    0
}

/// Get bitmap height
#[unsafe(no_mangle)]
pub extern "C" fn fz_bitmap_height(_ctx: Handle, bitmap: Handle) -> i32 {
    if let Some(bm) = BITMAPS.get(bitmap) {
        if let Ok(guard) = bm.lock() {
            return guard.height;
        }
    }
    0
}

/// Get bitmap stride
#[unsafe(no_mangle)]
pub extern "C" fn fz_bitmap_stride(_ctx: Handle, bitmap: Handle) -> i32 {
    if let Some(bm) = BITMAPS.get(bitmap) {
        if let Ok(guard) = bm.lock() {
            return guard.stride;
        }
    }
    0
}

/// Get X resolution (DPI)
#[unsafe(no_mangle)]
pub extern "C" fn fz_bitmap_x_res(_ctx: Handle, bitmap: Handle) -> i32 {
    if let Some(bm) = BITMAPS.get(bitmap) {
        if let Ok(guard) = bm.lock() {
            return guard.x_res;
        }
    }
    72
}

/// Get Y resolution (DPI)
#[unsafe(no_mangle)]
pub extern "C" fn fz_bitmap_y_res(_ctx: Handle, bitmap: Handle) -> i32 {
    if let Some(bm) = BITMAPS.get(bitmap) {
        if let Ok(guard) = bm.lock() {
            return guard.y_res;
        }
    }
    72
}

/// Set resolution
#[unsafe(no_mangle)]
pub extern "C" fn fz_bitmap_set_res(_ctx: Handle, bitmap: Handle, x_res: i32, y_res: i32) {
    if let Some(bm) = BITMAPS.get(bitmap) {
        if let Ok(mut guard) = bm.lock() {
            guard.x_res = x_res.max(1);
            guard.y_res = y_res.max(1);
        }
    }
}

/// Get pointer to bitmap data
#[unsafe(no_mangle)]
pub extern "C" fn fz_bitmap_data(_ctx: Handle, bitmap: Handle) -> *const u8 {
    if let Some(bm) = BITMAPS.get(bitmap) {
        if let Ok(guard) = bm.lock() {
            return guard.data.as_ptr();
        }
    }
    std::ptr::null()
}

/// Get bitmap data size
#[unsafe(no_mangle)]
pub extern "C" fn fz_bitmap_data_size(_ctx: Handle, bitmap: Handle) -> usize {
    if let Some(bm) = BITMAPS.get(bitmap) {
        if let Ok(guard) = bm.lock() {
            return guard.data.len();
        }
    }
    0
}

// ============================================================================
// Pixel Operations
// ============================================================================

/// Get pixel value at (x, y)
#[unsafe(no_mangle)]
pub extern "C" fn fz_bitmap_get_pixel(_ctx: Handle, bitmap: Handle, x: i32, y: i32) -> i32 {
    if let Some(bm) = BITMAPS.get(bitmap) {
        if let Ok(guard) = bm.lock() {
            if x < 0 || x >= guard.width || y < 0 || y >= guard.height {
                return 0;
            }

            let byte_idx = (y * guard.stride + x / 8) as usize;
            let bit_idx = 7 - (x % 8) as usize;

            if byte_idx < guard.data.len() {
                let bit = (guard.data[byte_idx] >> bit_idx) & 1;
                return if guard.invert {
                    1 - bit as i32
                } else {
                    bit as i32
                };
            }
        }
    }
    0
}

/// Set pixel value at (x, y)
#[unsafe(no_mangle)]
pub extern "C" fn fz_bitmap_set_pixel(_ctx: Handle, bitmap: Handle, x: i32, y: i32, value: i32) {
    if let Some(bm) = BITMAPS.get(bitmap) {
        if let Ok(mut guard) = bm.lock() {
            if x < 0 || x >= guard.width || y < 0 || y >= guard.height {
                return;
            }

            let byte_idx = (y * guard.stride + x / 8) as usize;
            let bit_idx = 7 - (x % 8) as usize;

            if byte_idx < guard.data.len() {
                let bit_val = if guard.invert { 1 - value } else { value };
                if bit_val != 0 {
                    guard.data[byte_idx] |= 1 << bit_idx;
                } else {
                    guard.data[byte_idx] &= !(1 << bit_idx);
                }
            }
        }
    }
}

/// Invert all pixels
#[unsafe(no_mangle)]
pub extern "C" fn fz_bitmap_invert(_ctx: Handle, bitmap: Handle) {
    if let Some(bm) = BITMAPS.get(bitmap) {
        if let Ok(mut guard) = bm.lock() {
            for byte in &mut guard.data {
                *byte = !*byte;
            }
        }
    }
}

/// Clear bitmap (set all pixels to white/0)
#[unsafe(no_mangle)]
pub extern "C" fn fz_bitmap_clear(_ctx: Handle, bitmap: Handle) {
    if let Some(bm) = BITMAPS.get(bitmap) {
        if let Ok(mut guard) = bm.lock() {
            guard.data.fill(0);
        }
    }
}

/// Fill bitmap (set all pixels to black/1)
#[unsafe(no_mangle)]
pub extern "C" fn fz_bitmap_fill(_ctx: Handle, bitmap: Handle) {
    if let Some(bm) = BITMAPS.get(bitmap) {
        if let Ok(mut guard) = bm.lock() {
            guard.data.fill(0xFF);
        }
    }
}

// ============================================================================
// Compression
// ============================================================================

/// Compress bitmap using RLE
#[unsafe(no_mangle)]
pub extern "C" fn fz_bitmap_compress_rle(
    _ctx: Handle,
    bitmap: Handle,
    output: *mut u8,
    max_size: usize,
) -> usize {
    if output.is_null() || max_size == 0 {
        return 0;
    }

    if let Some(bm) = BITMAPS.get(bitmap) {
        if let Ok(guard) = bm.lock() {
            let out_slice = unsafe { std::slice::from_raw_parts_mut(output, max_size) };
            let mut out_idx = 0;

            // Simple RLE: repeat count + byte value
            let mut i = 0;
            while i < guard.data.len() && out_idx + 2 <= max_size {
                let byte = guard.data[i];
                let mut count = 1u8;

                while i + (count as usize) < guard.data.len()
                    && count < 255
                    && guard.data[i + (count as usize)] == byte
                {
                    count += 1;
                }

                out_slice[out_idx] = count;
                out_slice[out_idx + 1] = byte;
                out_idx += 2;
                i += count as usize;
            }

            return out_idx;
        }
    }
    0
}

/// Get estimated compressed size
#[unsafe(no_mangle)]
pub extern "C" fn fz_bitmap_compressed_size(
    _ctx: Handle,
    bitmap: Handle,
    _compression: i32,
) -> usize {
    if let Some(bm) = BITMAPS.get(bitmap) {
        if let Ok(guard) = bm.lock() {
            // Estimate: worst case is 2x for RLE (no compression)
            // Best case is much smaller for repetitive data
            return guard.data.len() * 2;
        }
    }
    0
}

// ============================================================================
// Reference Counting
// ============================================================================

/// Keep bitmap reference
#[unsafe(no_mangle)]
pub extern "C" fn fz_keep_bitmap(_ctx: Handle, bitmap: Handle) -> Handle {
    BITMAPS.keep(bitmap)
}

/// Drop bitmap reference
#[unsafe(no_mangle)]
pub extern "C" fn fz_drop_bitmap(_ctx: Handle, bitmap: Handle) {
    BITMAPS.remove(bitmap);
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_bitmap() {
        let bm = fz_new_bitmap(0, 100, 50, 300, 300);
        assert!(bm > 0);

        assert_eq!(fz_bitmap_width(0, bm), 100);
        assert_eq!(fz_bitmap_height(0, bm), 50);
        assert_eq!(fz_bitmap_stride(0, bm), 13); // (100 + 7) / 8 = 13
        assert_eq!(fz_bitmap_x_res(0, bm), 300);

        fz_drop_bitmap(0, bm);
    }

    #[test]
    fn test_pixel_operations() {
        let bm = fz_new_bitmap(0, 16, 16, 72, 72);

        // Initially all white (0)
        assert_eq!(fz_bitmap_get_pixel(0, bm, 0, 0), 0);

        // Set pixel to black
        fz_bitmap_set_pixel(0, bm, 5, 5, 1);
        assert_eq!(fz_bitmap_get_pixel(0, bm, 5, 5), 1);

        // Clear and verify
        fz_bitmap_clear(0, bm);
        assert_eq!(fz_bitmap_get_pixel(0, bm, 5, 5), 0);

        // Fill and verify
        fz_bitmap_fill(0, bm);
        assert_eq!(fz_bitmap_get_pixel(0, bm, 0, 0), 1);

        fz_drop_bitmap(0, bm);
    }

    #[test]
    fn test_invert() {
        let bm = fz_new_bitmap(0, 8, 8, 72, 72);

        // Set a pattern
        fz_bitmap_set_pixel(0, bm, 0, 0, 1);
        fz_bitmap_set_pixel(0, bm, 1, 1, 1);

        // Invert
        fz_bitmap_invert(0, bm);

        // Check inverted values
        assert_eq!(fz_bitmap_get_pixel(0, bm, 0, 0), 0);
        assert_eq!(fz_bitmap_get_pixel(0, bm, 2, 2), 1);

        fz_drop_bitmap(0, bm);
    }

    #[test]
    fn test_invalid_dimensions() {
        let bm = fz_new_bitmap(0, -1, 100, 72, 72);
        assert_eq!(bm, 0);

        let bm = fz_new_bitmap(0, 100, 0, 72, 72);
        assert_eq!(bm, 0);
    }

    #[test]
    fn test_rle_compression() {
        let bm = fz_new_bitmap(0, 64, 1, 72, 72);

        // Fill with repeating pattern (should compress well)
        fz_bitmap_fill(0, bm);

        let mut buffer = vec![0u8; 256];
        let compressed_size = fz_bitmap_compress_rle(0, bm, buffer.as_mut_ptr(), buffer.len());

        // Should be smaller than uncompressed (8 bytes = 64 bits)
        assert!(compressed_size > 0);

        fz_drop_bitmap(0, bm);
    }
}
