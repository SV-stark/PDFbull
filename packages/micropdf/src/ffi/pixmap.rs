//! C FFI for pixmap - MuPDF compatible
//! Safe Rust implementation using handle-based resource management

use super::colorspace::{ColorspaceHandle, FZ_COLORSPACE_RGB};
use super::geometry::fz_irect;
use super::{Handle, PIXMAPS};

/// Internal pixmap state
pub struct Pixmap {
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    n: i32, // Number of components
    alpha: bool,
    stride: i32,
    samples: Vec<u8>,
    colorspace: ColorspaceHandle,
}

impl Pixmap {
    pub fn new(cs: ColorspaceHandle, width: i32, height: i32, alpha: bool) -> Self {
        let n = super::colorspace::fz_colorspace_n(0, cs) + i32::from(alpha);
        let stride = width * n;
        let size = (stride * height) as usize;

        Self {
            x: 0,
            y: 0,
            width,
            height,
            n,
            alpha,
            stride,
            samples: vec![0u8; size],
            colorspace: cs,
        }
    }

    pub fn with_bbox(cs: ColorspaceHandle, bbox: fz_irect, alpha: bool) -> Self {
        let width = bbox.x1 - bbox.x0;
        let height = bbox.y1 - bbox.y0;
        let n = super::colorspace::fz_colorspace_n(0, cs) + i32::from(alpha);
        let stride = width * n;
        let size = (stride * height).max(0) as usize;

        Self {
            x: bbox.x0,
            y: bbox.y0,
            width,
            height,
            n,
            alpha,
            stride,
            samples: vec![0u8; size],
            colorspace: cs,
        }
    }

    pub fn clear(&mut self) {
        self.samples.fill(0);
    }

    pub fn clear_with_value(&mut self, value: u8) {
        self.samples.fill(value);
    }

    pub fn get_sample(&self, x: i32, y: i32, component: i32) -> Option<u8> {
        if x < self.x
            || x >= self.x + self.width
            || y < self.y
            || y >= self.y + self.height
            || component < 0
            || component >= self.n
        {
            return None;
        }
        let local_x = x - self.x;
        let local_y = y - self.y;
        let offset = (local_y * self.stride + local_x * self.n + component) as usize;
        self.samples.get(offset).copied()
    }

    pub fn set_sample(&mut self, x: i32, y: i32, component: i32, value: u8) {
        if x < self.x
            || x >= self.x + self.width
            || y < self.y
            || y >= self.y + self.height
            || component < 0
            || component >= self.n
        {
            return;
        }
        let local_x = x - self.x;
        let local_y = y - self.y;
        let offset = (local_y * self.stride + local_x * self.n + component) as usize;
        if let Some(sample) = self.samples.get_mut(offset) {
            *sample = value;
        }
    }

    /// Get width
    pub fn w(&self) -> i32 {
        self.width
    }

    /// Get height
    pub fn h(&self) -> i32 {
        self.height
    }

    /// Get x origin
    pub fn x(&self) -> i32 {
        self.x
    }

    /// Get y origin
    pub fn y(&self) -> i32 {
        self.y
    }

    /// Get number of components (including alpha)
    pub fn n(&self) -> i32 {
        self.n
    }

    /// Get stride (bytes per row)
    pub fn stride(&self) -> i32 {
        self.stride
    }

    /// Check if pixmap has alpha channel
    pub fn has_alpha(&self) -> bool {
        self.alpha
    }

    /// Get colorspace handle
    pub fn colorspace(&self) -> ColorspaceHandle {
        self.colorspace
    }

    /// Get immutable reference to samples
    pub fn samples(&self) -> &[u8] {
        &self.samples
    }

    /// Get mutable reference to samples
    pub fn samples_mut(&mut self) -> &mut [u8] {
        &mut self.samples
    }
}

/// Create a new pixmap
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_pixmap(
    _ctx: Handle,
    cs: ColorspaceHandle,
    w: i32,
    h: i32,
    _seps: Handle, // Separations not implemented yet
    alpha: i32,
) -> Handle {
    let cs = if cs == 0 { FZ_COLORSPACE_RGB } else { cs };
    PIXMAPS.insert(Pixmap::new(cs, w, h, alpha != 0))
}

/// Create a new pixmap with bounding box
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_pixmap_with_bbox(
    _ctx: Handle,
    cs: ColorspaceHandle,
    bbox: fz_irect,
    _seps: Handle,
    alpha: i32,
) -> Handle {
    let cs = if cs == 0 { FZ_COLORSPACE_RGB } else { cs };
    PIXMAPS.insert(Pixmap::with_bbox(cs, bbox, alpha != 0))
}

/// Keep (increment ref) pixmap
#[unsafe(no_mangle)]
pub extern "C" fn fz_keep_pixmap(_ctx: Handle, pix: Handle) -> Handle {
    PIXMAPS.keep(pix)
}

/// Drop pixmap reference
#[unsafe(no_mangle)]
pub extern "C" fn fz_drop_pixmap(_ctx: Handle, pix: Handle) {
    let _ = PIXMAPS.remove(pix);
}

/// Get pixmap X origin
#[unsafe(no_mangle)]
pub extern "C" fn fz_pixmap_x(_ctx: Handle, pix: Handle) -> i32 {
    if let Some(p) = PIXMAPS.get(pix) {
        if let Ok(guard) = p.lock() {
            return guard.x;
        }
    }
    0
}

/// Get pixmap Y origin
#[unsafe(no_mangle)]
pub extern "C" fn fz_pixmap_y(_ctx: Handle, pix: Handle) -> i32 {
    if let Some(p) = PIXMAPS.get(pix) {
        if let Ok(guard) = p.lock() {
            return guard.y;
        }
    }
    0
}

/// Get pixmap width
#[unsafe(no_mangle)]
pub extern "C" fn fz_pixmap_width(_ctx: Handle, pix: Handle) -> i32 {
    if let Some(p) = PIXMAPS.get(pix) {
        if let Ok(guard) = p.lock() {
            return guard.width;
        }
    }
    0
}

/// Get pixmap height
#[unsafe(no_mangle)]
pub extern "C" fn fz_pixmap_height(_ctx: Handle, pix: Handle) -> i32 {
    if let Some(p) = PIXMAPS.get(pix) {
        if let Ok(guard) = p.lock() {
            return guard.height;
        }
    }
    0
}

/// Get number of components (including alpha)
#[unsafe(no_mangle)]
pub extern "C" fn fz_pixmap_components(_ctx: Handle, pix: Handle) -> i32 {
    if let Some(p) = PIXMAPS.get(pix) {
        if let Ok(guard) = p.lock() {
            return guard.n;
        }
    }
    0
}

/// Get number of colorants (excluding alpha)
#[unsafe(no_mangle)]
pub extern "C" fn fz_pixmap_colorants(_ctx: Handle, pix: Handle) -> i32 {
    if let Some(p) = PIXMAPS.get(pix) {
        if let Ok(guard) = p.lock() {
            return guard.n - i32::from(guard.alpha);
        }
    }
    0
}

/// Check if pixmap has alpha
#[unsafe(no_mangle)]
pub extern "C" fn fz_pixmap_alpha(_ctx: Handle, pix: Handle) -> i32 {
    if let Some(p) = PIXMAPS.get(pix) {
        if let Ok(guard) = p.lock() {
            return i32::from(guard.alpha);
        }
    }
    0
}

/// Get pixmap stride (bytes per row)
#[unsafe(no_mangle)]
pub extern "C" fn fz_pixmap_stride(_ctx: Handle, pix: Handle) -> i32 {
    if let Some(p) = PIXMAPS.get(pix) {
        if let Ok(guard) = p.lock() {
            return guard.stride;
        }
    }
    0
}

/// Get pixmap bounding box
#[unsafe(no_mangle)]
pub extern "C" fn fz_pixmap_bbox(_ctx: Handle, pix: Handle) -> fz_irect {
    if let Some(p) = PIXMAPS.get(pix) {
        if let Ok(guard) = p.lock() {
            return fz_irect {
                x0: guard.x,
                y0: guard.y,
                x1: guard.x + guard.width,
                y1: guard.y + guard.height,
            };
        }
    }
    fz_irect {
        x0: 0,
        y0: 0,
        x1: 0,
        y1: 0,
    }
}

/// Get pixmap colorspace
#[unsafe(no_mangle)]
pub extern "C" fn fz_pixmap_colorspace(_ctx: Handle, pix: Handle) -> ColorspaceHandle {
    if let Some(p) = PIXMAPS.get(pix) {
        if let Ok(guard) = p.lock() {
            return guard.colorspace;
        }
    }
    0
}

/// Clear pixmap to transparent black
#[unsafe(no_mangle)]
pub extern "C" fn fz_clear_pixmap(_ctx: Handle, pix: Handle) {
    if let Some(pixmap) = PIXMAPS.get(pix) {
        if let Ok(mut p) = pixmap.lock() {
            p.clear();
        }
    }
}

/// Clear pixmap to specific value
#[unsafe(no_mangle)]
pub extern "C" fn fz_clear_pixmap_with_value(_ctx: Handle, pix: Handle, value: i32) {
    if let Some(pixmap) = PIXMAPS.get(pix) {
        if let Ok(mut p) = pixmap.lock() {
            p.clear_with_value(value as u8);
        }
    }
}

/// Invert pixmap colors
#[unsafe(no_mangle)]
pub extern "C" fn fz_invert_pixmap(_ctx: Handle, pix: Handle) {
    if let Some(pixmap) = PIXMAPS.get(pix) {
        if let Ok(mut p) = pixmap.lock() {
            let colorants = (p.n - i32::from(p.alpha)) as usize;
            for y in 0..p.height {
                for x in 0..p.width {
                    let offset = (y * p.stride + x * p.n) as usize;
                    for c in 0..colorants {
                        if let Some(sample) = p.samples.get_mut(offset + c) {
                            *sample = 255 - *sample;
                        }
                    }
                }
            }
        }
    }
}

/// Apply gamma correction to pixmap
#[unsafe(no_mangle)]
pub extern "C" fn fz_gamma_pixmap(_ctx: Handle, pix: Handle, gamma: f32) {
    if gamma <= 0.0 {
        return;
    }

    // Pre-compute gamma lookup table
    let mut gamma_table = [0u8; 256];
    for (i, entry) in gamma_table.iter_mut().enumerate() {
        let normalized = (i as f32) / 255.0;
        let corrected = normalized.powf(1.0 / gamma);
        *entry = (corrected * 255.0).round().clamp(0.0, 255.0) as u8;
    }

    if let Some(pixmap) = PIXMAPS.get(pix) {
        if let Ok(mut p) = pixmap.lock() {
            let colorants = (p.n - i32::from(p.alpha)) as usize;
            for y in 0..p.height {
                for x in 0..p.width {
                    let offset = (y * p.stride + x * p.n) as usize;
                    for c in 0..colorants {
                        if let Some(sample) = p.samples.get_mut(offset + c) {
                            *sample = gamma_table[*sample as usize];
                        }
                    }
                }
            }
        }
    }
}

/// Get sample at specific position
#[unsafe(no_mangle)]
pub extern "C" fn fz_get_pixmap_sample(_ctx: Handle, pix: Handle, x: i32, y: i32, n: i32) -> u8 {
    if let Some(p) = PIXMAPS.get(pix) {
        if let Ok(guard) = p.lock() {
            if let Some(sample) = guard.get_sample(x, y, n) {
                return sample;
            }
        }
    }
    0
}

/// Set sample at specific position
#[unsafe(no_mangle)]
pub extern "C" fn fz_set_pixmap_sample(_ctx: Handle, pix: Handle, x: i32, y: i32, n: i32, v: u8) {
    if let Some(pixmap) = PIXMAPS.get(pix) {
        if let Ok(mut p) = pixmap.lock() {
            p.set_sample(x, y, n, v);
        }
    }
}

/// Get pointer to pixmap samples
///
/// Returns a pointer to the internal pixel data. The pointer is valid only while
/// the pixmap handle is valid. Caller should copy the data immediately.
/// Use fz_pixmap_samples_size() to get the size of the buffer.
#[unsafe(no_mangle)]
pub extern "C" fn fz_pixmap_samples(_ctx: Handle, pix: Handle) -> *mut u8 {
    if let Some(pixmap) = PIXMAPS.get(pix) {
        if let Ok(mut guard) = pixmap.lock() {
            if !guard.samples.is_empty() {
                return guard.samples.as_mut_ptr();
            }
        }
    }
    std::ptr::null_mut()
}

/// Get size of pixmap samples buffer in bytes
#[unsafe(no_mangle)]
pub extern "C" fn fz_pixmap_samples_size(_ctx: Handle, pix: Handle) -> usize {
    if let Some(pixmap) = PIXMAPS.get(pix) {
        if let Ok(guard) = pixmap.lock() {
            return guard.samples.len();
        }
    }
    0
}

/// Clone a pixmap
#[unsafe(no_mangle)]
pub extern "C" fn fz_clone_pixmap(_ctx: Handle, pix: Handle) -> Handle {
    if let Some(pixmap) = PIXMAPS.get(pix) {
        if let Ok(guard) = pixmap.lock() {
            let cloned = Pixmap {
                x: guard.x,
                y: guard.y,
                width: guard.width,
                height: guard.height,
                n: guard.n,
                alpha: guard.alpha,
                stride: guard.stride,
                samples: guard.samples.clone(),
                colorspace: guard.colorspace,
            };
            return PIXMAPS.insert(cloned);
        }
    }
    0
}

/// Convert pixmap to different colorspace
#[unsafe(no_mangle)]
pub extern "C" fn fz_convert_pixmap(
    _ctx: Handle,
    pix: Handle,
    cs: ColorspaceHandle,
    _prf: Handle, // Color profile (not implemented)
    _default_cs: Handle,
    _color_params: Handle,
    keep_alpha: i32,
) -> Handle {
    let cs = if cs == 0 { FZ_COLORSPACE_RGB } else { cs };
    let target_n = super::colorspace::fz_colorspace_n(0, cs);

    if let Some(pixmap) = PIXMAPS.get(pix) {
        if let Ok(guard) = pixmap.lock() {
            let alpha = if keep_alpha != 0 { guard.alpha } else { false };
            let new_n = target_n + i32::from(alpha);
            let new_stride = guard.width * new_n;
            let new_size = (new_stride * guard.height).max(0) as usize;

            let mut new_samples = vec![0u8; new_size];

            // Simple conversion: copy/convert each pixel
            let src_colorants = (guard.n - i32::from(guard.alpha)) as usize;
            let dst_colorants = target_n as usize;

            for y in 0..guard.height {
                for x in 0..guard.width {
                    let src_offset = (y * guard.stride + x * guard.n) as usize;
                    let dst_offset = (y * new_stride + x * new_n) as usize;

                    // Get source color values
                    let mut src_colors = [0u8; 4];
                    for (c, color) in src_colors.iter_mut().enumerate().take(src_colorants.min(4)) {
                        *color = guard.samples.get(src_offset + c).copied().unwrap_or(0);
                    }

                    // Convert colors (simple mapping)
                    let dst_colors = convert_color(
                        guard.colorspace,
                        cs,
                        &src_colors[..src_colorants.min(4)],
                        dst_colorants,
                    );

                    // Write destination colors
                    for c in 0..dst_colorants {
                        if let Some(sample) = new_samples.get_mut(dst_offset + c) {
                            *sample = dst_colors.get(c).copied().unwrap_or(0);
                        }
                    }

                    // Copy alpha if requested
                    if alpha && guard.alpha {
                        let src_alpha_offset = src_offset + src_colorants;
                        let dst_alpha_offset = dst_offset + dst_colorants;
                        if let (Some(&src_alpha), Some(dst_alpha)) = (
                            guard.samples.get(src_alpha_offset),
                            new_samples.get_mut(dst_alpha_offset),
                        ) {
                            *dst_alpha = src_alpha;
                        }
                    } else if alpha {
                        // Set opaque alpha if no source alpha
                        let dst_alpha_offset = dst_offset + dst_colorants;
                        if let Some(dst_alpha) = new_samples.get_mut(dst_alpha_offset) {
                            *dst_alpha = 255;
                        }
                    }
                }
            }

            let converted = Pixmap {
                x: guard.x,
                y: guard.y,
                width: guard.width,
                height: guard.height,
                n: new_n,
                alpha,
                stride: new_stride,
                samples: new_samples,
                colorspace: cs,
            };
            return PIXMAPS.insert(converted);
        }
    }
    0
}

/// Simple color conversion helper
fn convert_color(
    src_cs: ColorspaceHandle,
    dst_cs: ColorspaceHandle,
    src: &[u8],
    dst_n: usize,
) -> Vec<u8> {
    use super::colorspace::{FZ_COLORSPACE_CMYK, FZ_COLORSPACE_GRAY, FZ_COLORSPACE_RGB};

    let mut dst = vec![0u8; dst_n];

    match (src_cs, dst_cs) {
        // Same colorspace: copy
        (s, d) if s == d => {
            for (i, &v) in src.iter().enumerate().take(dst_n) {
                dst[i] = v;
            }
        }
        // Gray to RGB
        (FZ_COLORSPACE_GRAY, FZ_COLORSPACE_RGB) => {
            let gray = src.first().copied().unwrap_or(0);
            dst[0] = gray;
            dst[1] = gray;
            dst[2] = gray;
        }
        // RGB to Gray
        (FZ_COLORSPACE_RGB, FZ_COLORSPACE_GRAY) => {
            let r = src.first().copied().unwrap_or(0) as u32;
            let g = src.get(1).copied().unwrap_or(0) as u32;
            let b = src.get(2).copied().unwrap_or(0) as u32;
            // Standard luminance formula
            let gray = ((r * 77 + g * 150 + b * 29) >> 8) as u8;
            dst[0] = gray;
        }
        // CMYK to RGB
        (FZ_COLORSPACE_CMYK, FZ_COLORSPACE_RGB) => {
            let c = src.first().copied().unwrap_or(0) as f32 / 255.0;
            let m = src.get(1).copied().unwrap_or(0) as f32 / 255.0;
            let y = src.get(2).copied().unwrap_or(0) as f32 / 255.0;
            let k = src.get(3).copied().unwrap_or(0) as f32 / 255.0;
            dst[0] = ((1.0 - c) * (1.0 - k) * 255.0) as u8;
            dst[1] = ((1.0 - m) * (1.0 - k) * 255.0) as u8;
            dst[2] = ((1.0 - y) * (1.0 - k) * 255.0) as u8;
        }
        // RGB to CMYK
        (FZ_COLORSPACE_RGB, FZ_COLORSPACE_CMYK) => {
            let r = src.first().copied().unwrap_or(0) as f32 / 255.0;
            let g = src.get(1).copied().unwrap_or(0) as f32 / 255.0;
            let b = src.get(2).copied().unwrap_or(0) as f32 / 255.0;
            let k = 1.0 - r.max(g).max(b);
            if k < 1.0 {
                dst[0] = ((1.0 - r - k) / (1.0 - k) * 255.0) as u8;
                dst[1] = ((1.0 - g - k) / (1.0 - k) * 255.0) as u8;
                dst[2] = ((1.0 - b - k) / (1.0 - k) * 255.0) as u8;
            }
            dst[3] = (k * 255.0) as u8;
        }
        // Gray to CMYK
        (FZ_COLORSPACE_GRAY, FZ_COLORSPACE_CMYK) => {
            let gray = src.first().copied().unwrap_or(0);
            dst[0] = 0;
            dst[1] = 0;
            dst[2] = 0;
            dst[3] = 255 - gray;
        }
        // CMYK to Gray
        (FZ_COLORSPACE_CMYK, FZ_COLORSPACE_GRAY) => {
            // Convert via RGB
            let c = src.first().copied().unwrap_or(0) as f32 / 255.0;
            let m = src.get(1).copied().unwrap_or(0) as f32 / 255.0;
            let y = src.get(2).copied().unwrap_or(0) as f32 / 255.0;
            let k = src.get(3).copied().unwrap_or(0) as f32 / 255.0;
            let r = (1.0 - c) * (1.0 - k);
            let g = (1.0 - m) * (1.0 - k);
            let b = (1.0 - y) * (1.0 - k);
            dst[0] = ((r * 0.3 + g * 0.59 + b * 0.11) * 255.0) as u8;
        }
        // Default: just copy what we can
        _ => {
            for (i, &v) in src.iter().enumerate().take(dst_n) {
                dst[i] = v;
            }
        }
    }

    dst
}

/// Tint pixmap with specified color
#[unsafe(no_mangle)]
pub extern "C" fn fz_tint_pixmap(_ctx: Handle, pix: Handle, r: i32, g: i32, b: i32) {
    if let Some(pixmap) = PIXMAPS.get(pix) {
        if let Ok(mut p) = pixmap.lock() {
            // Only works for grayscale pixmaps
            if p.n - i32::from(p.alpha) != 1 {
                return;
            }

            let r_factor = (r as f32) / 255.0;
            let g_factor = (g as f32) / 255.0;
            let b_factor = (b as f32) / 255.0;

            // We need to convert to RGB to apply tint
            // For now, just modulate the gray value
            for y in 0..p.height {
                for x in 0..p.width {
                    let offset = (y * p.stride + x * p.n) as usize;
                    if let Some(sample) = p.samples.get_mut(offset) {
                        let gray = *sample as f32;
                        // Apply average tint factor
                        let factor = (r_factor + g_factor + b_factor) / 3.0;
                        *sample = (gray * factor).clamp(0.0, 255.0) as u8;
                    }
                }
            }
        }
    }
}

/// Set pixmap resolution
#[unsafe(no_mangle)]
pub extern "C" fn fz_set_pixmap_resolution(_ctx: Handle, _pix: Handle, _xres: i32, _yres: i32) {
    // Resolution is not stored in our simple implementation
    // This is a no-op for now
}

/// Get pixmap resolution
#[unsafe(no_mangle)]
pub extern "C" fn fz_pixmap_resolution(_ctx: Handle, _pix: Handle, xres: *mut i32, yres: *mut i32) {
    // Return default 72 dpi
    if !xres.is_null() {
        unsafe {
            *xres = 72;
        }
    }
    if !yres.is_null() {
        unsafe {
            *yres = 72;
        }
    }
}

/// Check if pixmap is valid
#[unsafe(no_mangle)]
pub extern "C" fn fz_pixmap_is_valid(_ctx: Handle, pix: Handle) -> i32 {
    if PIXMAPS.get(pix).is_some() { 1 } else { 0 }
}

/// Scale pixmap to new dimensions
#[unsafe(no_mangle)]
pub extern "C" fn fz_scale_pixmap(_ctx: Handle, pix: Handle, xscale: f32, yscale: f32) -> Handle {
    if let Some(p) = PIXMAPS.get(pix) {
        if let Ok(guard) = p.lock() {
            let new_width = ((guard.width as f32) * xscale) as i32;
            let new_height = ((guard.height as f32) * yscale) as i32;

            if new_width <= 0 || new_height <= 0 {
                return 0;
            }

            let mut scaled = Pixmap::new(guard.colorspace, new_width, new_height, guard.alpha);

            // Simple nearest-neighbor scaling
            for y in 0..new_height {
                for x in 0..new_width {
                    let src_x = ((x as f32) / xscale) as i32;
                    let src_y = ((y as f32) / yscale) as i32;

                    for c in 0..guard.n {
                        if let Some(value) = guard.get_sample(src_x, src_y, c) {
                            scaled.set_sample(x, y, c, value);
                        }
                    }
                }
            }

            return PIXMAPS.insert(scaled);
        }
    }
    0
}

/// Get X resolution
#[unsafe(no_mangle)]
pub extern "C" fn fz_pixmap_xres(_ctx: Handle, _pix: Handle) -> i32 {
    72 // Default DPI
}

/// Get Y resolution
#[unsafe(no_mangle)]
pub extern "C" fn fz_pixmap_yres(_ctx: Handle, _pix: Handle) -> i32 {
    72 // Default DPI
}

/// Set X resolution
#[unsafe(no_mangle)]
pub extern "C" fn fz_set_pixmap_xres(_ctx: Handle, _pix: Handle, _xres: i32) {
    // No-op in our implementation
}

/// Set Y resolution
#[unsafe(no_mangle)]
pub extern "C" fn fz_set_pixmap_yres(_ctx: Handle, _pix: Handle, _yres: i32) {
    // No-op in our implementation
}

#[cfg(test)]
mod tests {
    use super::super::colorspace::FZ_COLORSPACE_GRAY;
    use super::*;

    #[test]
    fn test_pixmap_create() {
        let handle = fz_new_pixmap(0, FZ_COLORSPACE_RGB, 100, 100, 0, 1);
        assert_ne!(handle, 0);

        assert_eq!(fz_pixmap_width(0, handle), 100);
        assert_eq!(fz_pixmap_height(0, handle), 100);
        assert_eq!(fz_pixmap_components(0, handle), 4); // RGB + alpha
        assert_eq!(fz_pixmap_alpha(0, handle), 1);

        fz_drop_pixmap(0, handle);
    }

    #[test]
    fn test_pixmap_create_gray() {
        let handle = fz_new_pixmap(0, FZ_COLORSPACE_GRAY, 50, 50, 0, 0);
        assert_ne!(handle, 0);

        assert_eq!(fz_pixmap_components(0, handle), 1); // Gray only
        assert_eq!(fz_pixmap_colorants(0, handle), 1);
        assert_eq!(fz_pixmap_alpha(0, handle), 0);

        fz_drop_pixmap(0, handle);
    }

    #[test]
    fn test_pixmap_clear() {
        let handle = fz_new_pixmap(0, FZ_COLORSPACE_RGB, 10, 10, 0, 0);

        fz_clear_pixmap_with_value(0, handle, 128);
        assert_eq!(fz_get_pixmap_sample(0, handle, 0, 0, 0), 128);

        fz_clear_pixmap(0, handle);
        assert_eq!(fz_get_pixmap_sample(0, handle, 0, 0, 0), 0);

        fz_drop_pixmap(0, handle);
    }

    #[test]
    fn test_pixmap_set_get_sample() {
        let handle = fz_new_pixmap(0, FZ_COLORSPACE_RGB, 10, 10, 0, 0);

        fz_set_pixmap_sample(0, handle, 5, 5, 0, 255);
        assert_eq!(fz_get_pixmap_sample(0, handle, 5, 5, 0), 255);
        assert_eq!(fz_get_pixmap_sample(0, handle, 5, 5, 1), 0);

        fz_drop_pixmap(0, handle);
    }

    #[test]
    fn test_pixmap_keep() {
        let handle = fz_new_pixmap(0, FZ_COLORSPACE_RGB, 10, 10, 0, 0);
        let kept = fz_keep_pixmap(0, handle);
        assert_eq!(kept, handle);
        fz_drop_pixmap(0, handle);
    }

    #[test]
    fn test_pixmap_x_y() {
        let handle = fz_new_pixmap(0, FZ_COLORSPACE_RGB, 10, 10, 0, 0);

        // Default origin is 0,0
        assert_eq!(fz_pixmap_x(0, handle), 0);
        assert_eq!(fz_pixmap_y(0, handle), 0);

        fz_drop_pixmap(0, handle);
    }

    #[test]
    fn test_pixmap_stride() {
        let handle = fz_new_pixmap(0, FZ_COLORSPACE_RGB, 10, 10, 0, 0);
        let stride = fz_pixmap_stride(0, handle);
        // RGB = 3 components, width = 10, so stride = 30
        assert_eq!(stride, 30);
        fz_drop_pixmap(0, handle);
    }

    #[test]
    fn test_pixmap_stride_with_alpha() {
        let handle = fz_new_pixmap(0, FZ_COLORSPACE_RGB, 10, 10, 0, 1);
        let stride = fz_pixmap_stride(0, handle);
        // RGBA = 4 components, width = 10, so stride = 40
        assert_eq!(stride, 40);
        fz_drop_pixmap(0, handle);
    }

    #[test]
    fn test_pixmap_bbox() {
        let handle = fz_new_pixmap(0, FZ_COLORSPACE_RGB, 100, 50, 0, 0);
        let bbox = fz_pixmap_bbox(0, handle);
        assert_eq!(bbox.x0, 0);
        assert_eq!(bbox.y0, 0);
        assert_eq!(bbox.x1, 100);
        assert_eq!(bbox.y1, 50);
        fz_drop_pixmap(0, handle);
    }

    #[test]
    fn test_pixmap_colorspace() {
        let handle = fz_new_pixmap(0, FZ_COLORSPACE_RGB, 10, 10, 0, 0);
        let cs = fz_pixmap_colorspace(0, handle);
        assert_eq!(cs, FZ_COLORSPACE_RGB);
        fz_drop_pixmap(0, handle);
    }

    #[test]
    fn test_pixmap_invalid_handle() {
        assert_eq!(fz_pixmap_width(0, 0), 0);
        assert_eq!(fz_pixmap_height(0, 0), 0);
        assert_eq!(fz_pixmap_components(0, 0), 0);
        assert_eq!(fz_pixmap_x(0, 0), 0);
        assert_eq!(fz_pixmap_y(0, 0), 0);
        assert_eq!(fz_pixmap_alpha(0, 0), 0);
        assert_eq!(fz_pixmap_stride(0, 0), 0);
        assert_eq!(fz_pixmap_colorants(0, 0), 0);
        assert_eq!(fz_get_pixmap_sample(0, 0, 0, 0, 0), 0);
    }

    #[test]
    fn test_pixmap_invert() {
        let handle = fz_new_pixmap(0, FZ_COLORSPACE_GRAY, 2, 2, 0, 0);

        // Set to known values
        fz_set_pixmap_sample(0, handle, 0, 0, 0, 100);
        fz_set_pixmap_sample(0, handle, 1, 0, 0, 200);

        fz_invert_pixmap(0, handle);

        // Values should be inverted (255 - x)
        assert_eq!(fz_get_pixmap_sample(0, handle, 0, 0, 0), 155);
        assert_eq!(fz_get_pixmap_sample(0, handle, 1, 0, 0), 55);

        fz_drop_pixmap(0, handle);
    }

    #[test]
    fn test_pixmap_gamma() {
        let handle = fz_new_pixmap(0, FZ_COLORSPACE_GRAY, 2, 2, 0, 0);

        // Set to mid-gray
        fz_clear_pixmap_with_value(0, handle, 128);

        // Apply gamma (1.0 should not change values significantly)
        fz_gamma_pixmap(0, handle, 1.0);

        // Value should be roughly the same
        let sample = fz_get_pixmap_sample(0, handle, 0, 0, 0);
        assert!((125..=131).contains(&sample));

        fz_drop_pixmap(0, handle);
    }

    #[test]
    fn test_pixmap_sample_bounds() {
        let handle = fz_new_pixmap(0, FZ_COLORSPACE_RGB, 10, 10, 0, 0);

        // Out of bounds access should return 0
        assert_eq!(fz_get_pixmap_sample(0, handle, -1, 0, 0), 0);
        assert_eq!(fz_get_pixmap_sample(0, handle, 0, -1, 0), 0);
        assert_eq!(fz_get_pixmap_sample(0, handle, 10, 0, 0), 0);
        assert_eq!(fz_get_pixmap_sample(0, handle, 0, 10, 0), 0);
        assert_eq!(fz_get_pixmap_sample(0, handle, 0, 0, 3), 0); // Component out of bounds

        fz_drop_pixmap(0, handle);
    }

    #[test]
    fn test_pixmap_internal_new() {
        let pixmap = Pixmap::new(FZ_COLORSPACE_RGB, 100, 50, true);
        assert_eq!(pixmap.width, 100);
        assert_eq!(pixmap.height, 50);
        assert_eq!(pixmap.n, 4); // RGB + alpha
        assert!(pixmap.alpha);
        assert_eq!(pixmap.stride, 400); // width * n
        assert_eq!(pixmap.samples.len(), 400 * 50);
    }

    #[test]
    fn test_pixmap_internal_get_set_sample() {
        let mut pixmap = Pixmap::new(FZ_COLORSPACE_RGB, 10, 10, false);

        pixmap.set_sample(5, 5, 0, 123);
        assert_eq!(pixmap.get_sample(5, 5, 0), Some(123));

        // Out of bounds
        assert_eq!(pixmap.get_sample(-1, 0, 0), None);
        assert_eq!(pixmap.get_sample(100, 0, 0), None);
    }

    #[test]
    fn test_pixmap_internal_clear() {
        let mut pixmap = Pixmap::new(FZ_COLORSPACE_GRAY, 5, 5, false);
        pixmap.clear_with_value(255);

        assert_eq!(pixmap.get_sample(0, 0, 0), Some(255));
        assert_eq!(pixmap.get_sample(4, 4, 0), Some(255));
    }

    // ============================================================================
    // Clone Tests
    // ============================================================================

    #[test]
    fn test_clone_pixmap() {
        let handle = fz_new_pixmap(0, FZ_COLORSPACE_RGB, 10, 10, 0, 0);
        fz_set_pixmap_sample(0, handle, 5, 5, 0, 123);

        let cloned = fz_clone_pixmap(0, handle);
        assert_ne!(cloned, 0);
        assert_ne!(cloned, handle);

        // Cloned should have same values
        assert_eq!(fz_pixmap_width(0, cloned), 10);
        assert_eq!(fz_get_pixmap_sample(0, cloned, 5, 5, 0), 123);

        // Modify original, clone should be unchanged
        fz_set_pixmap_sample(0, handle, 5, 5, 0, 200);
        assert_eq!(fz_get_pixmap_sample(0, cloned, 5, 5, 0), 123);

        fz_drop_pixmap(0, handle);
        fz_drop_pixmap(0, cloned);
    }

    #[test]
    fn test_clone_pixmap_invalid() {
        let result = fz_clone_pixmap(0, 99999);
        assert_eq!(result, 0);
    }

    // ============================================================================
    // Convert Tests
    // ============================================================================

    #[test]
    fn test_convert_pixmap_gray_to_rgb() {
        let handle = fz_new_pixmap(0, FZ_COLORSPACE_GRAY, 10, 10, 0, 0);
        fz_set_pixmap_sample(0, handle, 5, 5, 0, 128);

        let converted = fz_convert_pixmap(0, handle, FZ_COLORSPACE_RGB, 0, 0, 0, 0);
        assert_ne!(converted, 0);

        // Check colorspace changed
        assert_eq!(fz_pixmap_colorspace(0, converted), FZ_COLORSPACE_RGB);
        assert_eq!(fz_pixmap_components(0, converted), 3);

        // Gray 128 should convert to RGB (128, 128, 128)
        assert_eq!(fz_get_pixmap_sample(0, converted, 5, 5, 0), 128);
        assert_eq!(fz_get_pixmap_sample(0, converted, 5, 5, 1), 128);
        assert_eq!(fz_get_pixmap_sample(0, converted, 5, 5, 2), 128);

        fz_drop_pixmap(0, handle);
        fz_drop_pixmap(0, converted);
    }

    #[test]
    fn test_convert_pixmap_rgb_to_gray() {
        let handle = fz_new_pixmap(0, FZ_COLORSPACE_RGB, 10, 10, 0, 0);
        // Set to pure red
        fz_set_pixmap_sample(0, handle, 5, 5, 0, 255);
        fz_set_pixmap_sample(0, handle, 5, 5, 1, 0);
        fz_set_pixmap_sample(0, handle, 5, 5, 2, 0);

        let converted = fz_convert_pixmap(0, handle, FZ_COLORSPACE_GRAY, 0, 0, 0, 0);
        assert_ne!(converted, 0);

        assert_eq!(fz_pixmap_colorspace(0, converted), FZ_COLORSPACE_GRAY);
        assert_eq!(fz_pixmap_components(0, converted), 1);

        // Red should convert to a gray value (luminance)
        let gray = fz_get_pixmap_sample(0, converted, 5, 5, 0);
        assert!(gray > 0 && gray < 128); // Should be around 77 based on luminance

        fz_drop_pixmap(0, handle);
        fz_drop_pixmap(0, converted);
    }

    #[test]
    fn test_convert_pixmap_with_alpha() {
        let handle = fz_new_pixmap(0, FZ_COLORSPACE_GRAY, 10, 10, 0, 1);
        fz_set_pixmap_sample(0, handle, 5, 5, 0, 128); // Gray
        fz_set_pixmap_sample(0, handle, 5, 5, 1, 200); // Alpha

        let converted = fz_convert_pixmap(0, handle, FZ_COLORSPACE_RGB, 0, 0, 0, 1);
        assert_ne!(converted, 0);

        // Should have 4 components (RGB + alpha)
        assert_eq!(fz_pixmap_components(0, converted), 4);
        assert_eq!(fz_pixmap_alpha(0, converted), 1);

        // Alpha should be preserved
        assert_eq!(fz_get_pixmap_sample(0, converted, 5, 5, 3), 200);

        fz_drop_pixmap(0, handle);
        fz_drop_pixmap(0, converted);
    }

    #[test]
    fn test_convert_pixmap_invalid() {
        let result = fz_convert_pixmap(0, 99999, FZ_COLORSPACE_RGB, 0, 0, 0, 0);
        assert_eq!(result, 0);
    }

    // ============================================================================
    // Samples Pointer Test
    // ============================================================================

    #[test]
    fn test_pixmap_samples_returns_pointer() {
        let handle = fz_new_pixmap(0, FZ_COLORSPACE_RGB, 10, 10, 0, 0);
        let samples = fz_pixmap_samples(0, handle);
        // Should return valid pointer to samples data
        assert!(!samples.is_null());

        // Verify samples size matches expected dimensions
        let size = fz_pixmap_samples_size(0, handle);
        assert_eq!(size, 10 * 10 * 3); // width * height * 3 components (RGB)

        fz_drop_pixmap(0, handle);
    }

    #[test]
    fn test_pixmap_samples_invalid_handle() {
        let samples = fz_pixmap_samples(0, 99999);
        assert!(samples.is_null());

        let size = fz_pixmap_samples_size(0, 99999);
        assert_eq!(size, 0);
    }

    // ============================================================================
    // Tint Tests
    // ============================================================================

    #[test]
    fn test_tint_pixmap() {
        let handle = fz_new_pixmap(0, FZ_COLORSPACE_GRAY, 10, 10, 0, 0);
        fz_set_pixmap_sample(0, handle, 5, 5, 0, 200);

        // Tint with red (should reduce brightness due to averaging)
        fz_tint_pixmap(0, handle, 255, 0, 0);

        let sample = fz_get_pixmap_sample(0, handle, 5, 5, 0);
        assert!(sample < 200); // Should be reduced

        fz_drop_pixmap(0, handle);
    }

    #[test]
    fn test_tint_pixmap_non_gray() {
        let handle = fz_new_pixmap(0, FZ_COLORSPACE_RGB, 10, 10, 0, 0);
        fz_set_pixmap_sample(0, handle, 5, 5, 0, 100);

        // Tinting RGB should be a no-op (not supported)
        fz_tint_pixmap(0, handle, 255, 0, 0);

        // Value should be unchanged
        assert_eq!(fz_get_pixmap_sample(0, handle, 5, 5, 0), 100);

        fz_drop_pixmap(0, handle);
    }

    // ============================================================================
    // Resolution Tests
    // ============================================================================

    #[test]
    fn test_pixmap_resolution() {
        let handle = fz_new_pixmap(0, FZ_COLORSPACE_RGB, 10, 10, 0, 0);

        let mut xres: i32 = 0;
        let mut yres: i32 = 0;
        fz_pixmap_resolution(0, handle, &mut xres, &mut yres);

        // Default is 72 dpi
        assert_eq!(xres, 72);
        assert_eq!(yres, 72);

        fz_drop_pixmap(0, handle);
    }

    #[test]
    fn test_set_pixmap_resolution() {
        let handle = fz_new_pixmap(0, FZ_COLORSPACE_RGB, 10, 10, 0, 0);

        // This is a no-op in our implementation
        fz_set_pixmap_resolution(0, handle, 300, 300);

        // Still returns default
        let mut xres: i32 = 0;
        let mut yres: i32 = 0;
        fz_pixmap_resolution(0, handle, &mut xres, &mut yres);
        assert_eq!(xres, 72);

        fz_drop_pixmap(0, handle);
    }

    // ============================================================================
    // Color Conversion Helper Tests
    // ============================================================================

    #[test]
    fn test_convert_color_gray_to_rgb() {
        use super::super::colorspace::{FZ_COLORSPACE_GRAY, FZ_COLORSPACE_RGB};
        let result = convert_color(FZ_COLORSPACE_GRAY, FZ_COLORSPACE_RGB, &[128], 3);
        assert_eq!(result, vec![128, 128, 128]);
    }

    #[test]
    fn test_convert_color_rgb_to_gray() {
        use super::super::colorspace::{FZ_COLORSPACE_GRAY, FZ_COLORSPACE_RGB};
        // Pure white should convert to white
        let result = convert_color(FZ_COLORSPACE_RGB, FZ_COLORSPACE_GRAY, &[255, 255, 255], 1);
        assert!(result[0] >= 250); // Allow some rounding

        // Pure black should convert to black
        let result2 = convert_color(FZ_COLORSPACE_RGB, FZ_COLORSPACE_GRAY, &[0, 0, 0], 1);
        assert_eq!(result2[0], 0);
    }

    #[test]
    fn test_convert_color_same_space() {
        use super::super::colorspace::FZ_COLORSPACE_RGB;
        let result = convert_color(FZ_COLORSPACE_RGB, FZ_COLORSPACE_RGB, &[100, 150, 200], 3);
        assert_eq!(result, vec![100, 150, 200]);
    }
}
