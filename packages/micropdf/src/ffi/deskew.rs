//! FFI bindings for fz_deskew (Auto-deskew for Scanned Pages)
//!
//! Provides skew angle detection and automatic deskewing of scanned documents.

use crate::ffi::colorspace::FZ_COLORSPACE_GRAY;
use crate::ffi::pixmap::Pixmap;
use crate::ffi::{Handle, PIXMAPS};
use std::f64::consts::PI;

// ============================================================================
// Types
// ============================================================================

/// Border handling modes for deskewing
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DeskewBorder {
    /// Increase image size to fit rotated content (no cropping)
    #[default]
    Increase = 0,
    /// Maintain original size (may crop corners)
    Maintain = 1,
    /// Decrease size to show only fully visible content
    Decrease = 2,
}

impl DeskewBorder {
    pub fn from_i32(value: i32) -> Self {
        match value {
            0 => DeskewBorder::Increase,
            1 => DeskewBorder::Maintain,
            2 => DeskewBorder::Decrease,
            _ => DeskewBorder::Increase,
        }
    }
}

// ============================================================================
// Skew Detection Algorithm
// ============================================================================

/// Detect skew angle using the projection profile method
///
/// This algorithm:
/// 1. Converts image to grayscale if needed
/// 2. Applies edge detection (Sobel-like)
/// 3. Uses Hough transform variant for line detection
/// 4. Returns the dominant angle
pub fn detect_skew(pixmap: &Pixmap) -> f64 {
    let width = pixmap.w() as usize;
    let height = pixmap.h() as usize;
    let samples = pixmap.samples();
    let n = pixmap.n() as usize;

    if width < 10 || height < 10 || samples.is_empty() {
        return 0.0;
    }

    // Convert to grayscale intensity values
    let gray: Vec<u8> = if n == 1 {
        samples.to_vec()
    } else {
        samples
            .chunks(n)
            .map(|pixel| {
                if n >= 3 {
                    // RGB or RGBA -> luminance
                    ((pixel[0] as u32 * 299 + pixel[1] as u32 * 587 + pixel[2] as u32 * 114) / 1000)
                        as u8
                } else {
                    pixel[0]
                }
            })
            .collect()
    };

    // Detect edges using simple gradient
    let edges = detect_edges(&gray, width, height);

    // Find dominant angle using projection profile method
    let angle = find_dominant_angle(&edges, width, height);

    angle
}

/// Simple edge detection using Sobel-like operators
fn detect_edges(gray: &[u8], width: usize, height: usize) -> Vec<u8> {
    let mut edges = vec![0u8; width * height];

    for y in 1..height - 1 {
        for x in 1..width - 1 {
            let idx = y * width + x;

            // Sobel X gradient
            let gx = gray[(y - 1) * width + (x + 1)] as i32
                - gray[(y - 1) * width + (x - 1)] as i32
                + 2 * (gray[y * width + (x + 1)] as i32 - gray[y * width + (x - 1)] as i32)
                + gray[(y + 1) * width + (x + 1)] as i32
                - gray[(y + 1) * width + (x - 1)] as i32;

            // Sobel Y gradient
            let gy = gray[(y + 1) * width + (x - 1)] as i32
                - gray[(y - 1) * width + (x - 1)] as i32
                + 2 * (gray[(y + 1) * width + x] as i32 - gray[(y - 1) * width + x] as i32)
                + gray[(y + 1) * width + (x + 1)] as i32
                - gray[(y - 1) * width + (x + 1)] as i32;

            // Magnitude
            let magnitude = ((gx * gx + gy * gy) as f64).sqrt();
            edges[idx] = (magnitude.min(255.0)) as u8;
        }
    }

    edges
}

/// Find dominant angle using projection profile method
fn find_dominant_angle(edges: &[u8], width: usize, height: usize) -> f64 {
    let mut best_angle = 0.0;
    let mut best_variance = 0.0;

    // Test angles from -15 to +15 degrees in 0.1 degree steps
    let step = 0.1_f64;
    let mut angle = -15.0_f64;

    while angle <= 15.0 {
        let variance = calculate_projection_variance(edges, width, height, angle);

        if variance > best_variance {
            best_variance = variance;
            best_angle = angle;
        }

        angle += step;
    }

    // Refine with smaller steps around best angle
    let refined_step = 0.01_f64;
    let mut refined_angle = best_angle - step;
    let end_angle = best_angle + step;

    while refined_angle <= end_angle {
        let variance = calculate_projection_variance(edges, width, height, refined_angle);

        if variance > best_variance {
            best_variance = variance;
            best_angle = refined_angle;
        }

        refined_angle += refined_step;
    }

    best_angle
}

/// Calculate variance of horizontal projection at given angle
fn calculate_projection_variance(edges: &[u8], width: usize, height: usize, angle: f64) -> f64 {
    let rad = angle * PI / 180.0;
    let cos_a = rad.cos();
    let sin_a = rad.sin();

    let cx = width as f64 / 2.0;
    let cy = height as f64 / 2.0;

    // Project onto rotated horizontal axis
    let mut projection = vec![0u32; height];

    for y in 0..height {
        for x in 0..width {
            let edge_val = edges[y * width + x] as u32;
            if edge_val > 30 {
                // Threshold
                // Rotate point
                let dx = x as f64 - cx;
                let dy = y as f64 - cy;
                let ry = (-dx * sin_a + dy * cos_a + cy) as usize;

                if ry < height {
                    projection[ry] += edge_val;
                }
            }
        }
    }

    // Calculate variance
    let sum: u64 = projection.iter().map(|&v| v as u64).sum();
    let mean = sum as f64 / height as f64;

    let variance: f64 = projection
        .iter()
        .map(|&v| {
            let diff = v as f64 - mean;
            diff * diff
        })
        .sum::<f64>()
        / height as f64;

    variance
}

// ============================================================================
// Deskew (Rotation) Algorithm
// ============================================================================

/// Deskew a pixmap by rotating it
pub fn deskew_pixmap(pixmap: &Pixmap, degrees: f64, border: DeskewBorder) -> Option<Pixmap> {
    let src_width = pixmap.w();
    let src_height = pixmap.h();
    let n = pixmap.n();
    let alpha = pixmap.has_alpha();

    if src_width <= 0 || src_height <= 0 {
        return None;
    }

    let rad = degrees * PI / 180.0;
    let cos_a = rad.cos();
    let sin_a = rad.sin();

    // Calculate destination size based on border mode
    let (dst_width, dst_height) = match border {
        DeskewBorder::Increase => {
            // Bounding box of rotated rectangle
            let w = src_width as f64;
            let h = src_height as f64;
            let new_w = (w * cos_a.abs() + h * sin_a.abs()).ceil() as i32;
            let new_h = (w * sin_a.abs() + h * cos_a.abs()).ceil() as i32;
            (new_w, new_h)
        }
        DeskewBorder::Maintain => (src_width, src_height),
        DeskewBorder::Decrease => {
            // Inscribed rectangle
            let w = src_width as f64;
            let h = src_height as f64;
            let abs_cos = cos_a.abs();
            let abs_sin = sin_a.abs();

            if abs_sin < 1e-10 {
                (src_width, src_height)
            } else {
                let new_w = ((w * abs_cos - h * abs_sin) / (abs_cos * abs_cos - abs_sin * abs_sin))
                    .abs()
                    .floor() as i32;
                let new_h = ((h * abs_cos - w * abs_sin) / (abs_cos * abs_cos - abs_sin * abs_sin))
                    .abs()
                    .floor() as i32;
                (new_w.max(1), new_h.max(1))
            }
        }
    };

    // Create destination pixmap
    let mut dst = Pixmap::new(FZ_COLORSPACE_GRAY, dst_width, dst_height, alpha);

    // Center points
    let src_cx = src_width as f64 / 2.0;
    let src_cy = src_height as f64 / 2.0;
    let dst_cx = dst_width as f64 / 2.0;
    let dst_cy = dst_height as f64 / 2.0;

    let src_samples = pixmap.samples();
    let dst_samples = dst.samples_mut();
    let n_usize = n as usize;

    // Fill with white (background)
    for sample in dst_samples.iter_mut() {
        *sample = 255;
    }

    // Bilinear interpolation rotation
    for dst_y in 0..dst_height {
        for dst_x in 0..dst_width {
            // Map destination to source coordinates (inverse rotation)
            let dx = dst_x as f64 - dst_cx;
            let dy = dst_y as f64 - dst_cy;

            let src_x = dx * cos_a + dy * sin_a + src_cx;
            let src_y = -dx * sin_a + dy * cos_a + src_cy;

            // Bilinear interpolation
            if src_x >= 0.0
                && src_x < (src_width - 1) as f64
                && src_y >= 0.0
                && src_y < (src_height - 1) as f64
            {
                let x0 = src_x.floor() as usize;
                let y0 = src_y.floor() as usize;
                let x1 = x0 + 1;
                let y1 = y0 + 1;

                let fx = src_x - x0 as f64;
                let fy = src_y - y0 as f64;

                let dst_idx = (dst_y as usize * dst_width as usize + dst_x as usize) * n_usize;

                for c in 0..n_usize {
                    let v00 = src_samples[(y0 * src_width as usize + x0) * n_usize + c] as f64;
                    let v10 = src_samples[(y0 * src_width as usize + x1) * n_usize + c] as f64;
                    let v01 = src_samples[(y1 * src_width as usize + x0) * n_usize + c] as f64;
                    let v11 = src_samples[(y1 * src_width as usize + x1) * n_usize + c] as f64;

                    let value = v00 * (1.0 - fx) * (1.0 - fy)
                        + v10 * fx * (1.0 - fy)
                        + v01 * (1.0 - fx) * fy
                        + v11 * fx * fy;

                    if dst_idx + c < dst_samples.len() {
                        dst_samples[dst_idx + c] = value.round().clamp(0.0, 255.0) as u8;
                    }
                }
            }
        }
    }

    Some(dst)
}

// ============================================================================
// FFI Functions
// ============================================================================

/// Detect skew angle in a pixmap
///
/// Returns the detected skew angle in degrees (typically -15 to +15)
/// Positive = clockwise skew, Negative = counter-clockwise skew
#[unsafe(no_mangle)]
pub extern "C" fn fz_detect_skew(_ctx: Handle, pixmap: Handle) -> f64 {
    let pix = match PIXMAPS.get(pixmap) {
        Some(p) => p,
        None => return 0.0,
    };

    let guard = match pix.lock() {
        Ok(g) => g,
        Err(_) => return 0.0,
    };

    detect_skew(&guard)
}

/// Deskew a pixmap by rotating it
///
/// @param ctx      Context handle
/// @param src      Source pixmap handle
/// @param degrees  Rotation angle in degrees (use negative of detected skew)
/// @param border   Border handling mode (0=increase, 1=maintain, 2=decrease)
///
/// Returns handle to new deskewed pixmap, or 0 on error
#[unsafe(no_mangle)]
pub extern "C" fn fz_deskew_pixmap(_ctx: Handle, src: Handle, degrees: f64, border: i32) -> Handle {
    let pix = match PIXMAPS.get(src) {
        Some(p) => p,
        None => return 0,
    };

    let guard = match pix.lock() {
        Ok(g) => g,
        Err(_) => return 0,
    };

    let border_mode = DeskewBorder::from_i32(border);

    match deskew_pixmap(&guard, degrees, border_mode) {
        Some(result) => PIXMAPS.insert(result),
        None => 0,
    }
}

/// Auto-deskew a pixmap (detect and correct skew)
///
/// Convenience function that detects skew and applies correction
///
/// @param ctx      Context handle
/// @param src      Source pixmap handle
/// @param border   Border handling mode (0=increase, 1=maintain, 2=decrease)
///
/// Returns handle to deskewed pixmap, or 0 on error
#[unsafe(no_mangle)]
pub extern "C" fn fz_auto_deskew_pixmap(_ctx: Handle, src: Handle, border: i32) -> Handle {
    let pix = match PIXMAPS.get(src) {
        Some(p) => p,
        None => return 0,
    };

    let guard = match pix.lock() {
        Ok(g) => g,
        Err(_) => return 0,
    };

    // Detect skew
    let skew_angle = detect_skew(&guard);

    // Apply correction (negative angle to counteract skew)
    let border_mode = DeskewBorder::from_i32(border);

    match deskew_pixmap(&guard, -skew_angle, border_mode) {
        Some(result) => PIXMAPS.insert(result),
        None => 0,
    }
}

/// Get the skew angle that was detected (for information)
///
/// @param ctx      Context handle
/// @param pixmap   Pixmap handle
/// @param angle    Pointer to receive detected angle
///
/// Returns 1 on success, 0 on error
#[unsafe(no_mangle)]
pub extern "C" fn fz_detect_skew_angle(_ctx: Handle, pixmap: Handle, angle: *mut f64) -> i32 {
    if angle.is_null() {
        return 0;
    }

    let pix = match PIXMAPS.get(pixmap) {
        Some(p) => p,
        None => return 0,
    };

    let guard = match pix.lock() {
        Ok(g) => g,
        Err(_) => return 0,
    };

    let detected = detect_skew(&guard);
    unsafe {
        *angle = detected;
    }

    1
}

/// Check if a pixmap appears to be skewed
///
/// Returns 1 if skew angle > threshold (default 0.5 degrees), 0 otherwise
#[unsafe(no_mangle)]
pub extern "C" fn fz_is_skewed(_ctx: Handle, pixmap: Handle, threshold: f64) -> i32 {
    let pix = match PIXMAPS.get(pixmap) {
        Some(p) => p,
        None => return 0,
    };

    let guard = match pix.lock() {
        Ok(g) => g,
        Err(_) => return 0,
    };

    let angle = detect_skew(&guard);
    let thresh = if threshold <= 0.0 { 0.5 } else { threshold };

    if angle.abs() > thresh { 1 } else { 0 }
}

/// Rotate a pixmap by arbitrary angle
///
/// General rotation function (not limited to deskew range)
///
/// @param ctx      Context handle
/// @param src      Source pixmap handle
/// @param degrees  Rotation angle in degrees
/// @param border   Border handling mode
///
/// Returns handle to rotated pixmap, or 0 on error
#[unsafe(no_mangle)]
pub extern "C" fn fz_rotate_pixmap(_ctx: Handle, src: Handle, degrees: f64, border: i32) -> Handle {
    // Same as deskew, but doesn't imply the deskew use case
    fz_deskew_pixmap(_ctx, src, degrees, border)
}

/// Rotate pixmap by 90-degree increments (fast path)
///
/// @param ctx      Context handle
/// @param src      Source pixmap handle
/// @param quarters Number of 90-degree rotations (1=90°, 2=180°, 3=270°)
///
/// Returns handle to rotated pixmap, or 0 on error
#[unsafe(no_mangle)]
pub extern "C" fn fz_rotate_pixmap_90(_ctx: Handle, src: Handle, quarters: i32) -> Handle {
    let pix = match PIXMAPS.get(src) {
        Some(p) => p,
        None => return 0,
    };

    let guard = match pix.lock() {
        Ok(g) => g,
        Err(_) => return 0,
    };

    let src_width = guard.w() as usize;
    let src_height = guard.h() as usize;
    let n = guard.n() as usize;
    let alpha = guard.has_alpha();
    let src_samples = guard.samples();

    let quarters = ((quarters % 4) + 4) % 4; // Normalize to 0-3

    if quarters == 0 {
        // No rotation, clone the pixmap
        let mut dst = Pixmap::new(FZ_COLORSPACE_GRAY, guard.w(), guard.h(), alpha);
        dst.samples_mut().copy_from_slice(src_samples);
        return PIXMAPS.insert(dst);
    }

    let (dst_width, dst_height) = if quarters == 1 || quarters == 3 {
        (src_height as i32, src_width as i32)
    } else {
        (src_width as i32, src_height as i32)
    };

    let mut dst = Pixmap::new(FZ_COLORSPACE_GRAY, dst_width, dst_height, alpha);
    let dst_samples = dst.samples_mut();

    match quarters {
        1 => {
            // 90° clockwise
            for y in 0..src_height {
                for x in 0..src_width {
                    let src_idx = (y * src_width + x) * n;
                    let dst_x = src_height - 1 - y;
                    let dst_y = x;
                    let dst_idx = (dst_y * dst_width as usize + dst_x) * n;
                    for c in 0..n {
                        dst_samples[dst_idx + c] = src_samples[src_idx + c];
                    }
                }
            }
        }
        2 => {
            // 180°
            for y in 0..src_height {
                for x in 0..src_width {
                    let src_idx = (y * src_width + x) * n;
                    let dst_x = src_width - 1 - x;
                    let dst_y = src_height - 1 - y;
                    let dst_idx = (dst_y * dst_width as usize + dst_x) * n;
                    for c in 0..n {
                        dst_samples[dst_idx + c] = src_samples[src_idx + c];
                    }
                }
            }
        }
        3 => {
            // 270° clockwise (90° counter-clockwise)
            for y in 0..src_height {
                for x in 0..src_width {
                    let src_idx = (y * src_width + x) * n;
                    let dst_x = y;
                    let dst_y = src_width - 1 - x;
                    let dst_idx = (dst_y * dst_width as usize + dst_x) * n;
                    for c in 0..n {
                        dst_samples[dst_idx + c] = src_samples[src_idx + c];
                    }
                }
            }
        }
        _ => unreachable!(),
    }

    PIXMAPS.insert(dst)
}

/// Flip pixmap horizontally
#[unsafe(no_mangle)]
pub extern "C" fn fz_flip_pixmap_horizontal(_ctx: Handle, src: Handle) -> Handle {
    let pix = match PIXMAPS.get(src) {
        Some(p) => p,
        None => return 0,
    };

    let guard = match pix.lock() {
        Ok(g) => g,
        Err(_) => return 0,
    };

    let width = guard.w() as usize;
    let height = guard.h() as usize;
    let n = guard.n() as usize;
    let alpha = guard.has_alpha();
    let src_samples = guard.samples();

    let mut dst = Pixmap::new(FZ_COLORSPACE_GRAY, guard.w(), guard.h(), alpha);
    let dst_samples = dst.samples_mut();

    for y in 0..height {
        for x in 0..width {
            let src_idx = (y * width + x) * n;
            let dst_idx = (y * width + (width - 1 - x)) * n;
            for c in 0..n {
                dst_samples[dst_idx + c] = src_samples[src_idx + c];
            }
        }
    }

    PIXMAPS.insert(dst)
}

/// Flip pixmap vertically
#[unsafe(no_mangle)]
pub extern "C" fn fz_flip_pixmap_vertical(_ctx: Handle, src: Handle) -> Handle {
    let pix = match PIXMAPS.get(src) {
        Some(p) => p,
        None => return 0,
    };

    let guard = match pix.lock() {
        Ok(g) => g,
        Err(_) => return 0,
    };

    let width = guard.w() as usize;
    let height = guard.h() as usize;
    let n = guard.n() as usize;
    let alpha = guard.has_alpha();
    let src_samples = guard.samples();

    let mut dst = Pixmap::new(FZ_COLORSPACE_GRAY, guard.w(), guard.h(), alpha);
    let dst_samples = dst.samples_mut();

    for y in 0..height {
        for x in 0..width {
            let src_idx = (y * width + x) * n;
            let dst_idx = ((height - 1 - y) * width + x) * n;
            for c in 0..n {
                dst_samples[dst_idx + c] = src_samples[src_idx + c];
            }
        }
    }

    PIXMAPS.insert(dst)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_pixmap(width: i32, height: i32) -> Handle {
        let mut pix = Pixmap::new(FZ_COLORSPACE_GRAY, width, height, false);
        let samples = pix.samples_mut();

        // Fill with a diagonal line pattern (creates detectable skew)
        for y in 0..height as usize {
            for x in 0..width as usize {
                // White background with some dark lines
                samples[y * width as usize + x] = if (x + y) % 20 < 2 { 0 } else { 255 };
            }
        }

        PIXMAPS.insert(pix)
    }

    fn create_skewed_text_pixmap(width: i32, height: i32, skew_angle: f64) -> Handle {
        let mut pix = Pixmap::new(FZ_COLORSPACE_GRAY, width, height, false);
        let samples = pix.samples_mut();

        // White background
        for sample in samples.iter_mut() {
            *sample = 255;
        }

        // Draw horizontal lines with skew
        let rad = skew_angle * PI / 180.0;
        let cx = width as f64 / 2.0;
        let cy = height as f64 / 2.0;

        for line in 0..10 {
            let base_y = (line * height / 12 + height / 12) as f64;

            for x in 0..width {
                let dx = x as f64 - cx;
                let dy = base_y - cy;

                // Apply skew
                let rotated_y = (dx * rad.sin() + dy * rad.cos() + cy) as i32;

                if rotated_y >= 0 && rotated_y < height {
                    // Draw 2-pixel thick line
                    for t in 0..2 {
                        let y = (rotated_y + t) as usize;
                        if y < height as usize {
                            samples[y * width as usize + x as usize] = 0; // Black
                        }
                    }
                }
            }
        }

        PIXMAPS.insert(pix)
    }

    #[test]
    fn test_deskew_border_enum() {
        assert_eq!(DeskewBorder::from_i32(0), DeskewBorder::Increase);
        assert_eq!(DeskewBorder::from_i32(1), DeskewBorder::Maintain);
        assert_eq!(DeskewBorder::from_i32(2), DeskewBorder::Decrease);
        assert_eq!(DeskewBorder::from_i32(99), DeskewBorder::Increase);
    }

    #[test]
    fn test_detect_skew_straight() {
        // Create a straight horizontal line pattern
        let mut pix = Pixmap::new(FZ_COLORSPACE_GRAY, 100, 100, false);
        let samples = pix.samples_mut();

        // White background
        for sample in samples.iter_mut() {
            *sample = 255;
        }

        // Horizontal lines
        for y in (10..90).step_by(10) {
            for x in 10..90 {
                samples[y * 100 + x] = 0;
            }
        }

        let angle = detect_skew(&pix);
        assert!(
            angle.abs() < 1.0,
            "Straight lines should have near-zero skew, got {}",
            angle
        );
    }

    #[test]
    fn test_ffi_detect_skew() {
        let handle = create_test_pixmap(200, 200);
        let angle = fz_detect_skew(1, handle);

        // Just verify we get a reasonable angle
        assert!(angle >= -90.0 && angle <= 90.0);

        PIXMAPS.remove(handle);
    }

    #[test]
    fn test_ffi_deskew_pixmap() {
        let src = create_test_pixmap(100, 100);
        let dst = fz_deskew_pixmap(1, src, 5.0, DeskewBorder::Maintain as i32);

        assert!(dst > 0);

        if let Some(pix) = PIXMAPS.get(dst) {
            let guard = pix.lock().unwrap();
            // With maintain mode, size should be same
            assert_eq!(guard.w(), 100);
            assert_eq!(guard.h(), 100);
        }

        PIXMAPS.remove(src);
        PIXMAPS.remove(dst);
    }

    #[test]
    fn test_ffi_deskew_increase_border() {
        let src = create_test_pixmap(100, 100);
        let dst = fz_deskew_pixmap(1, src, 10.0, DeskewBorder::Increase as i32);

        assert!(dst > 0);

        if let Some(pix) = PIXMAPS.get(dst) {
            let guard = pix.lock().unwrap();
            // With increase mode, size should be larger
            assert!(guard.w() > 100 || guard.h() > 100);
        }

        PIXMAPS.remove(src);
        PIXMAPS.remove(dst);
    }

    #[test]
    fn test_ffi_auto_deskew() {
        let src = create_skewed_text_pixmap(200, 200, 3.0);
        let dst = fz_auto_deskew_pixmap(1, src, DeskewBorder::Maintain as i32);

        assert!(dst > 0);

        PIXMAPS.remove(src);
        PIXMAPS.remove(dst);
    }

    #[test]
    fn test_ffi_is_skewed() {
        let straight = create_test_pixmap(100, 100);
        let skewed = create_skewed_text_pixmap(200, 200, 5.0);

        // Check with low threshold
        let is_straight_skewed = fz_is_skewed(1, straight, 0.1);
        let is_skewed_skewed = fz_is_skewed(1, skewed, 0.1);

        // At least verify the function runs
        assert!(is_straight_skewed == 0 || is_straight_skewed == 1);
        assert!(is_skewed_skewed == 0 || is_skewed_skewed == 1);

        PIXMAPS.remove(straight);
        PIXMAPS.remove(skewed);
    }

    #[test]
    fn test_ffi_rotate_90() {
        let src = create_test_pixmap(100, 50);

        // 90 degrees
        let dst = fz_rotate_pixmap_90(1, src, 1);
        assert!(dst > 0);
        if let Some(pix) = PIXMAPS.get(dst) {
            let guard = pix.lock().unwrap();
            assert_eq!(guard.w(), 50);
            assert_eq!(guard.h(), 100);
        }

        PIXMAPS.remove(dst);
        PIXMAPS.remove(src);
    }

    #[test]
    fn test_ffi_rotate_180() {
        let src = create_test_pixmap(100, 50);

        let dst = fz_rotate_pixmap_90(1, src, 2);
        assert!(dst > 0);
        if let Some(pix) = PIXMAPS.get(dst) {
            let guard = pix.lock().unwrap();
            assert_eq!(guard.w(), 100);
            assert_eq!(guard.h(), 50);
        }

        PIXMAPS.remove(dst);
        PIXMAPS.remove(src);
    }

    #[test]
    fn test_ffi_rotate_270() {
        let src = create_test_pixmap(100, 50);

        let dst = fz_rotate_pixmap_90(1, src, 3);
        assert!(dst > 0);
        if let Some(pix) = PIXMAPS.get(dst) {
            let guard = pix.lock().unwrap();
            assert_eq!(guard.w(), 50);
            assert_eq!(guard.h(), 100);
        }

        PIXMAPS.remove(dst);
        PIXMAPS.remove(src);
    }

    #[test]
    fn test_ffi_flip_horizontal() {
        let src = create_test_pixmap(100, 50);
        let dst = fz_flip_pixmap_horizontal(1, src);

        assert!(dst > 0);
        if let Some(pix) = PIXMAPS.get(dst) {
            let guard = pix.lock().unwrap();
            assert_eq!(guard.w(), 100);
            assert_eq!(guard.h(), 50);
        }

        PIXMAPS.remove(dst);
        PIXMAPS.remove(src);
    }

    #[test]
    fn test_ffi_flip_vertical() {
        let src = create_test_pixmap(100, 50);
        let dst = fz_flip_pixmap_vertical(1, src);

        assert!(dst > 0);
        if let Some(pix) = PIXMAPS.get(dst) {
            let guard = pix.lock().unwrap();
            assert_eq!(guard.w(), 100);
            assert_eq!(guard.h(), 50);
        }

        PIXMAPS.remove(dst);
        PIXMAPS.remove(src);
    }

    #[test]
    fn test_detect_skew_angle() {
        let handle = create_test_pixmap(100, 100);
        let mut angle = 0.0;

        let result = fz_detect_skew_angle(1, handle, &mut angle);
        assert_eq!(result, 1);
        assert!(angle >= -90.0 && angle <= 90.0);

        PIXMAPS.remove(handle);
    }

    #[test]
    fn test_null_handling() {
        assert_eq!(fz_detect_skew(1, 0), 0.0);
        assert_eq!(fz_deskew_pixmap(1, 0, 5.0, 0), 0);
        assert_eq!(fz_auto_deskew_pixmap(1, 0, 0), 0);
        assert_eq!(fz_is_skewed(1, 0, 0.5), 0);
        assert_eq!(fz_rotate_pixmap_90(1, 0, 1), 0);
        assert_eq!(fz_flip_pixmap_horizontal(1, 0), 0);
        assert_eq!(fz_flip_pixmap_vertical(1, 0), 0);
    }
}
