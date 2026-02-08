//! Pixmap - Pixel buffer for rendering

use crate::fitz::colorspace::Colorspace;
use crate::fitz::error::{Error, Result};
use std::sync::Arc;

#[derive(Clone)]
pub struct Pixmap {
    inner: Arc<PixmapInner>,
}

#[derive(Clone)]
#[allow(dead_code)]
struct PixmapInner {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    n: u8,
    alpha: u8,
    stride: usize,
    colorspace: Option<Colorspace>,
    samples: Vec<u8>,
}

impl Pixmap {
    pub fn new(colorspace: Option<Colorspace>, w: i32, h: i32, alpha: bool) -> Result<Self> {
        if w <= 0 || h <= 0 {
            return Err(Error::argument("Invalid dimensions"));
        }
        let n = match &colorspace {
            Some(cs) => cs.n() + if alpha { 1 } else { 0 },
            None if alpha => 1,
            None => return Err(Error::argument("Pixmap must have colorspace or alpha")),
        };
        let stride = (w as usize) * (n as usize);
        Ok(Self {
            inner: Arc::new(PixmapInner {
                x: 0,
                y: 0,
                w,
                h,
                n,
                alpha: if alpha { 1 } else { 0 },
                stride,
                colorspace,
                samples: vec![0; stride * (h as usize)],
            }),
        })
    }
    pub fn width(&self) -> i32 {
        self.inner.w
    }
    pub fn height(&self) -> i32 {
        self.inner.h
    }
    /// Alias for width()
    pub fn w(&self) -> i32 {
        self.inner.w
    }
    /// Alias for height()
    pub fn h(&self) -> i32 {
        self.inner.h
    }
    pub fn n(&self) -> u8 {
        self.inner.n
    }
    pub fn has_alpha(&self) -> bool {
        self.inner.alpha > 0
    }
    pub fn stride(&self) -> usize {
        self.inner.stride
    }
    pub fn colorspace(&self) -> Option<&Colorspace> {
        self.inner.colorspace.as_ref()
    }
    pub fn samples(&self) -> &[u8] {
        &self.inner.samples
    }
    pub fn samples_mut(&mut self) -> &mut [u8] {
        &mut Arc::make_mut(&mut self.inner).samples
    }
    pub fn clear(&mut self, value: u8) {
        let inner = Arc::make_mut(&mut self.inner);
        inner.samples.fill(value);
    }
    pub fn get_pixel(&self, x: i32, y: i32) -> Option<&[u8]> {
        if x < 0 || x >= self.inner.w || y < 0 || y >= self.inner.h {
            return None;
        }
        let offset = (y as usize) * self.inner.stride + (x as usize) * (self.inner.n as usize);
        Some(&self.inner.samples[offset..offset + self.inner.n as usize])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pixmap_new_rgb() {
        let cs = Colorspace::device_rgb();
        let pm = Pixmap::new(Some(cs), 100, 50, false).unwrap();
        assert_eq!(pm.width(), 100);
        assert_eq!(pm.height(), 50);
        assert_eq!(pm.n(), 3); // RGB
        assert!(!pm.has_alpha());
        assert_eq!(pm.stride(), 300); // 100 * 3
    }

    #[test]
    fn test_pixmap_new_rgb_with_alpha() {
        let cs = Colorspace::device_rgb();
        let pm = Pixmap::new(Some(cs), 100, 50, true).unwrap();
        assert_eq!(pm.n(), 4); // RGBA
        assert!(pm.has_alpha());
        assert_eq!(pm.stride(), 400); // 100 * 4
    }

    #[test]
    fn test_pixmap_new_gray() {
        let cs = Colorspace::device_gray();
        let pm = Pixmap::new(Some(cs), 100, 100, false).unwrap();
        assert_eq!(pm.n(), 1);
        assert!(!pm.has_alpha());
    }

    #[test]
    fn test_pixmap_new_cmyk() {
        let cs = Colorspace::device_cmyk();
        let pm = Pixmap::new(Some(cs), 50, 50, false).unwrap();
        assert_eq!(pm.n(), 4);
    }

    #[test]
    fn test_pixmap_new_alpha_only() {
        let pm = Pixmap::new(None, 100, 100, true).unwrap();
        assert_eq!(pm.n(), 1);
        assert!(pm.has_alpha());
        assert!(pm.colorspace().is_none());
    }

    #[test]
    fn test_pixmap_new_invalid_dimensions() {
        let cs = Colorspace::device_rgb();
        assert!(Pixmap::new(Some(cs.clone()), 0, 100, false).is_err());
        assert!(Pixmap::new(Some(cs.clone()), 100, 0, false).is_err());
        assert!(Pixmap::new(Some(cs), -1, 100, false).is_err());
    }

    #[test]
    fn test_pixmap_new_no_colorspace_no_alpha() {
        // Must have either colorspace or alpha
        assert!(Pixmap::new(None, 100, 100, false).is_err());
    }

    #[test]
    fn test_pixmap_samples() {
        let cs = Colorspace::device_rgb();
        let pm = Pixmap::new(Some(cs), 10, 10, false).unwrap();
        let samples = pm.samples();
        assert_eq!(samples.len(), 10 * 10 * 3);
        // Should be initialized to 0
        assert!(samples.iter().all(|&b| b == 0));
    }

    #[test]
    fn test_pixmap_samples_mut() {
        let cs = Colorspace::device_rgb();
        let mut pm = Pixmap::new(Some(cs), 10, 10, false).unwrap();
        let samples = pm.samples_mut();
        samples[0] = 255;
        samples[1] = 128;
        samples[2] = 64;

        let samples_read = pm.samples();
        assert_eq!(samples_read[0], 255);
        assert_eq!(samples_read[1], 128);
        assert_eq!(samples_read[2], 64);
    }

    #[test]
    fn test_pixmap_clear() {
        let cs = Colorspace::device_rgb();
        let mut pm = Pixmap::new(Some(cs), 10, 10, false).unwrap();
        pm.clear(128);

        assert!(pm.samples().iter().all(|&b| b == 128));
    }

    #[test]
    fn test_pixmap_get_pixel() {
        let cs = Colorspace::device_rgb();
        let mut pm = Pixmap::new(Some(cs), 10, 10, false).unwrap();

        // Set pixel at (2, 3)
        let offset = 3 * 30 + 2 * 3; // y * stride + x * n
        pm.samples_mut()[offset] = 255;
        pm.samples_mut()[offset + 1] = 128;
        pm.samples_mut()[offset + 2] = 64;

        let pixel = pm.get_pixel(2, 3).unwrap();
        assert_eq!(pixel, &[255, 128, 64]);
    }

    #[test]
    fn test_pixmap_get_pixel_out_of_bounds() {
        let cs = Colorspace::device_rgb();
        let pm = Pixmap::new(Some(cs), 10, 10, false).unwrap();

        assert!(pm.get_pixel(-1, 0).is_none());
        assert!(pm.get_pixel(0, -1).is_none());
        assert!(pm.get_pixel(10, 0).is_none());
        assert!(pm.get_pixel(0, 10).is_none());
    }

    #[test]
    fn test_pixmap_colorspace() {
        let cs = Colorspace::device_rgb();
        let pm = Pixmap::new(Some(cs), 10, 10, false).unwrap();

        let cs_ref = pm.colorspace().unwrap();
        assert_eq!(cs_ref.name(), "DeviceRGB");
    }

    #[test]
    fn test_pixmap_clone() {
        let cs = Colorspace::device_rgb();
        let pm1 = Pixmap::new(Some(cs), 10, 10, false).unwrap();
        let pm2 = pm1.clone();

        assert_eq!(pm1.width(), pm2.width());
        assert_eq!(pm1.height(), pm2.height());
        assert_eq!(pm1.n(), pm2.n());
    }
}
