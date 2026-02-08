//! PDF Image Rewriter FFI Module
//!
//! Provides PDF image optimization including resampling, recompression,
//! and resolution changes for color, grayscale, and bitonal images.

use crate::ffi::Handle;
use std::ffi::{CStr, CString, c_char};
use std::ptr;

// ============================================================================
// Type Aliases
// ============================================================================

type ContextHandle = Handle;
type DocumentHandle = Handle;

// ============================================================================
// Subsample Methods
// ============================================================================

/// Average subsampling method
pub const FZ_SUBSAMPLE_AVERAGE: i32 = 0;
/// Bicubic subsampling method (higher quality)
pub const FZ_SUBSAMPLE_BICUBIC: i32 = 1;

// ============================================================================
// Recompress Methods
// ============================================================================

/// Never recompress images
pub const FZ_RECOMPRESS_NEVER: i32 = 0;
/// Recompress using same method as original
pub const FZ_RECOMPRESS_SAME: i32 = 1;
/// Recompress losslessly (PNG/Flate)
pub const FZ_RECOMPRESS_LOSSLESS: i32 = 2;
/// Recompress as JPEG
pub const FZ_RECOMPRESS_JPEG: i32 = 3;
/// Recompress as JPEG 2000
pub const FZ_RECOMPRESS_J2K: i32 = 4;
/// Recompress as CCITT Fax (bitonal only)
pub const FZ_RECOMPRESS_FAX: i32 = 5;

// ============================================================================
// Image Rewriter Options
// ============================================================================

/// Image rewriter options for color, grayscale, and bitonal images
#[derive(Debug, Clone)]
#[repr(C)]
pub struct ImageRewriterOptions {
    // Color lossless images
    /// Subsample method for lossless color images
    pub color_lossless_image_subsample_method: i32,
    /// Subsample method for lossy color images
    pub color_lossy_image_subsample_method: i32,
    /// DPI threshold for subsampling lossless color images (0 = never)
    pub color_lossless_image_subsample_threshold: i32,
    /// Target DPI for subsampling lossless color images
    pub color_lossless_image_subsample_to: i32,
    /// DPI threshold for subsampling lossy color images (0 = never)
    pub color_lossy_image_subsample_threshold: i32,
    /// Target DPI for subsampling lossy color images
    pub color_lossy_image_subsample_to: i32,
    /// Recompress method for lossless color images
    pub color_lossless_image_recompress_method: i32,
    /// Recompress method for lossy color images
    pub color_lossy_image_recompress_method: i32,
    /// Quality string for lossy color image recompression
    color_lossy_image_recompress_quality: *mut c_char,
    /// Quality string for lossless color image recompression
    color_lossless_image_recompress_quality: *mut c_char,

    // Grayscale images
    /// Subsample method for lossless gray images
    pub gray_lossless_image_subsample_method: i32,
    /// Subsample method for lossy gray images
    pub gray_lossy_image_subsample_method: i32,
    /// DPI threshold for subsampling lossless gray images
    pub gray_lossless_image_subsample_threshold: i32,
    /// Target DPI for subsampling lossless gray images
    pub gray_lossless_image_subsample_to: i32,
    /// DPI threshold for subsampling lossy gray images
    pub gray_lossy_image_subsample_threshold: i32,
    /// Target DPI for subsampling lossy gray images
    pub gray_lossy_image_subsample_to: i32,
    /// Recompress method for lossless gray images
    pub gray_lossless_image_recompress_method: i32,
    /// Recompress method for lossy gray images
    pub gray_lossy_image_recompress_method: i32,
    /// Quality string for lossy gray image recompression
    gray_lossy_image_recompress_quality: *mut c_char,
    /// Quality string for lossless gray image recompression
    gray_lossless_image_recompress_quality: *mut c_char,

    // Bitonal images
    /// Subsample method for bitonal images
    pub bitonal_image_subsample_method: i32,
    /// DPI threshold for subsampling bitonal images
    pub bitonal_image_subsample_threshold: i32,
    /// Target DPI for subsampling bitonal images
    pub bitonal_image_subsample_to: i32,
    /// Recompress method for bitonal images
    pub bitonal_image_recompress_method: i32,
    /// Quality string for bitonal image recompression
    bitonal_image_recompress_quality: *mut c_char,
}

impl Default for ImageRewriterOptions {
    fn default() -> Self {
        Self::new()
    }
}

impl ImageRewriterOptions {
    pub fn new() -> Self {
        Self {
            color_lossless_image_subsample_method: FZ_SUBSAMPLE_BICUBIC,
            color_lossy_image_subsample_method: FZ_SUBSAMPLE_BICUBIC,
            color_lossless_image_subsample_threshold: 0,
            color_lossless_image_subsample_to: 0,
            color_lossy_image_subsample_threshold: 0,
            color_lossy_image_subsample_to: 0,
            color_lossless_image_recompress_method: FZ_RECOMPRESS_SAME,
            color_lossy_image_recompress_method: FZ_RECOMPRESS_SAME,
            color_lossy_image_recompress_quality: ptr::null_mut(),
            color_lossless_image_recompress_quality: ptr::null_mut(),

            gray_lossless_image_subsample_method: FZ_SUBSAMPLE_BICUBIC,
            gray_lossy_image_subsample_method: FZ_SUBSAMPLE_BICUBIC,
            gray_lossless_image_subsample_threshold: 0,
            gray_lossless_image_subsample_to: 0,
            gray_lossy_image_subsample_threshold: 0,
            gray_lossy_image_subsample_to: 0,
            gray_lossless_image_recompress_method: FZ_RECOMPRESS_SAME,
            gray_lossy_image_recompress_method: FZ_RECOMPRESS_SAME,
            gray_lossy_image_recompress_quality: ptr::null_mut(),
            gray_lossless_image_recompress_quality: ptr::null_mut(),

            bitonal_image_subsample_method: FZ_SUBSAMPLE_AVERAGE,
            bitonal_image_subsample_threshold: 0,
            bitonal_image_subsample_to: 0,
            bitonal_image_recompress_method: FZ_RECOMPRESS_SAME,
            bitonal_image_recompress_quality: ptr::null_mut(),
        }
    }

    /// Create options for web optimization (72 DPI, JPEG)
    pub fn web_optimized() -> Self {
        let mut opts = Self::new();
        opts.color_lossless_image_subsample_threshold = 150;
        opts.color_lossless_image_subsample_to = 72;
        opts.color_lossy_image_subsample_threshold = 150;
        opts.color_lossy_image_subsample_to = 72;
        opts.color_lossless_image_recompress_method = FZ_RECOMPRESS_JPEG;
        opts.color_lossy_image_recompress_method = FZ_RECOMPRESS_JPEG;

        opts.gray_lossless_image_subsample_threshold = 150;
        opts.gray_lossless_image_subsample_to = 72;
        opts.gray_lossy_image_subsample_threshold = 150;
        opts.gray_lossy_image_subsample_to = 72;
        opts.gray_lossless_image_recompress_method = FZ_RECOMPRESS_JPEG;
        opts.gray_lossy_image_recompress_method = FZ_RECOMPRESS_JPEG;

        opts.bitonal_image_subsample_threshold = 300;
        opts.bitonal_image_subsample_to = 150;
        opts.bitonal_image_recompress_method = FZ_RECOMPRESS_FAX;
        opts
    }

    /// Create options for print quality (300 DPI)
    pub fn print_quality() -> Self {
        let mut opts = Self::new();
        opts.color_lossless_image_subsample_threshold = 450;
        opts.color_lossless_image_subsample_to = 300;
        opts.color_lossy_image_subsample_threshold = 450;
        opts.color_lossy_image_subsample_to = 300;
        opts.color_lossless_image_recompress_method = FZ_RECOMPRESS_LOSSLESS;
        opts.color_lossy_image_recompress_method = FZ_RECOMPRESS_JPEG;

        opts.gray_lossless_image_subsample_threshold = 450;
        opts.gray_lossless_image_subsample_to = 300;
        opts.gray_lossy_image_subsample_threshold = 450;
        opts.gray_lossy_image_subsample_to = 300;
        opts.gray_lossless_image_recompress_method = FZ_RECOMPRESS_LOSSLESS;
        opts.gray_lossy_image_recompress_method = FZ_RECOMPRESS_JPEG;

        opts.bitonal_image_subsample_threshold = 600;
        opts.bitonal_image_subsample_to = 300;
        opts.bitonal_image_recompress_method = FZ_RECOMPRESS_FAX;
        opts
    }

    /// Create options for ebook (150 DPI)
    pub fn ebook_quality() -> Self {
        let mut opts = Self::new();
        opts.color_lossless_image_subsample_threshold = 300;
        opts.color_lossless_image_subsample_to = 150;
        opts.color_lossy_image_subsample_threshold = 300;
        opts.color_lossy_image_subsample_to = 150;
        opts.color_lossless_image_recompress_method = FZ_RECOMPRESS_JPEG;
        opts.color_lossy_image_recompress_method = FZ_RECOMPRESS_JPEG;

        opts.gray_lossless_image_subsample_threshold = 300;
        opts.gray_lossless_image_subsample_to = 150;
        opts.gray_lossy_image_subsample_threshold = 300;
        opts.gray_lossy_image_subsample_to = 150;
        opts.gray_lossless_image_recompress_method = FZ_RECOMPRESS_JPEG;
        opts.gray_lossy_image_recompress_method = FZ_RECOMPRESS_JPEG;

        opts.bitonal_image_subsample_threshold = 300;
        opts.bitonal_image_subsample_to = 150;
        opts.bitonal_image_recompress_method = FZ_RECOMPRESS_FAX;
        opts
    }

    /// Create options for maximum compression
    pub fn max_compression() -> Self {
        let mut opts = Self::new();
        opts.color_lossless_image_subsample_threshold = 100;
        opts.color_lossless_image_subsample_to = 72;
        opts.color_lossy_image_subsample_threshold = 100;
        opts.color_lossy_image_subsample_to = 72;
        opts.color_lossless_image_recompress_method = FZ_RECOMPRESS_JPEG;
        opts.color_lossy_image_recompress_method = FZ_RECOMPRESS_JPEG;

        opts.gray_lossless_image_subsample_threshold = 100;
        opts.gray_lossless_image_subsample_to = 72;
        opts.gray_lossy_image_subsample_threshold = 100;
        opts.gray_lossy_image_subsample_to = 72;
        opts.gray_lossless_image_recompress_method = FZ_RECOMPRESS_JPEG;
        opts.gray_lossy_image_recompress_method = FZ_RECOMPRESS_JPEG;

        opts.bitonal_image_subsample_threshold = 200;
        opts.bitonal_image_subsample_to = 100;
        opts.bitonal_image_recompress_method = FZ_RECOMPRESS_FAX;
        opts
    }
}

// ============================================================================
// Image Statistics
// ============================================================================

/// Statistics from image rewriting operation
#[derive(Debug, Default, Clone)]
#[repr(C)]
pub struct ImageRewriteStats {
    /// Total images processed
    pub images_processed: i32,
    /// Images that were subsampled
    pub images_subsampled: i32,
    /// Images that were recompressed
    pub images_recompressed: i32,
    /// Images left unchanged
    pub images_unchanged: i32,
    /// Original total size in bytes
    pub original_size: u64,
    /// New total size in bytes
    pub new_size: u64,
    /// Color images processed
    pub color_images: i32,
    /// Grayscale images processed
    pub gray_images: i32,
    /// Bitonal images processed
    pub bitonal_images: i32,
}

impl ImageRewriteStats {
    /// Calculate compression ratio
    pub fn compression_ratio(&self) -> f64 {
        if self.new_size == 0 {
            return 0.0;
        }
        self.original_size as f64 / self.new_size as f64
    }

    /// Calculate size reduction percentage
    pub fn size_reduction_percent(&self) -> f64 {
        if self.original_size == 0 {
            return 0.0;
        }
        (1.0 - (self.new_size as f64 / self.original_size as f64)) * 100.0
    }
}

// ============================================================================
// FFI Functions - Default Options
// ============================================================================

/// Get default image rewriter options.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_default_image_rewriter_options() -> ImageRewriterOptions {
    ImageRewriterOptions::new()
}

/// Get web-optimized options (72 DPI, JPEG).
#[unsafe(no_mangle)]
pub extern "C" fn pdf_web_image_rewriter_options() -> ImageRewriterOptions {
    ImageRewriterOptions::web_optimized()
}

/// Get print quality options (300 DPI).
#[unsafe(no_mangle)]
pub extern "C" fn pdf_print_image_rewriter_options() -> ImageRewriterOptions {
    ImageRewriterOptions::print_quality()
}

/// Get ebook quality options (150 DPI).
#[unsafe(no_mangle)]
pub extern "C" fn pdf_ebook_image_rewriter_options() -> ImageRewriterOptions {
    ImageRewriterOptions::ebook_quality()
}

/// Get maximum compression options.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_max_compression_image_rewriter_options() -> ImageRewriterOptions {
    ImageRewriterOptions::max_compression()
}

// ============================================================================
// FFI Functions - Option Setters
// ============================================================================

/// Set color image subsample threshold.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_set_color_subsample(
    opts: *mut ImageRewriterOptions,
    threshold_dpi: i32,
    target_dpi: i32,
    method: i32,
) {
    if opts.is_null() {
        return;
    }
    unsafe {
        (*opts).color_lossless_image_subsample_threshold = threshold_dpi;
        (*opts).color_lossless_image_subsample_to = target_dpi;
        (*opts).color_lossless_image_subsample_method = method;
        (*opts).color_lossy_image_subsample_threshold = threshold_dpi;
        (*opts).color_lossy_image_subsample_to = target_dpi;
        (*opts).color_lossy_image_subsample_method = method;
    }
}

/// Set grayscale image subsample threshold.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_set_gray_subsample(
    opts: *mut ImageRewriterOptions,
    threshold_dpi: i32,
    target_dpi: i32,
    method: i32,
) {
    if opts.is_null() {
        return;
    }
    unsafe {
        (*opts).gray_lossless_image_subsample_threshold = threshold_dpi;
        (*opts).gray_lossless_image_subsample_to = target_dpi;
        (*opts).gray_lossless_image_subsample_method = method;
        (*opts).gray_lossy_image_subsample_threshold = threshold_dpi;
        (*opts).gray_lossy_image_subsample_to = target_dpi;
        (*opts).gray_lossy_image_subsample_method = method;
    }
}

/// Set bitonal image subsample threshold.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_set_bitonal_subsample(
    opts: *mut ImageRewriterOptions,
    threshold_dpi: i32,
    target_dpi: i32,
    method: i32,
) {
    if opts.is_null() {
        return;
    }
    unsafe {
        (*opts).bitonal_image_subsample_threshold = threshold_dpi;
        (*opts).bitonal_image_subsample_to = target_dpi;
        (*opts).bitonal_image_subsample_method = method;
    }
}

/// Set color image recompression method.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_set_color_recompress(opts: *mut ImageRewriterOptions, method: i32) {
    if opts.is_null() {
        return;
    }
    unsafe {
        (*opts).color_lossless_image_recompress_method = method;
        (*opts).color_lossy_image_recompress_method = method;
    }
}

/// Set grayscale image recompression method.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_set_gray_recompress(opts: *mut ImageRewriterOptions, method: i32) {
    if opts.is_null() {
        return;
    }
    unsafe {
        (*opts).gray_lossless_image_recompress_method = method;
        (*opts).gray_lossy_image_recompress_method = method;
    }
}

/// Set bitonal image recompression method.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_set_bitonal_recompress(opts: *mut ImageRewriterOptions, method: i32) {
    if opts.is_null() {
        return;
    }
    unsafe {
        (*opts).bitonal_image_recompress_method = method;
    }
}

/// Set JPEG quality for color images.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_set_color_jpeg_quality(
    opts: *mut ImageRewriterOptions,
    quality: *const c_char,
) {
    if opts.is_null() || quality.is_null() {
        return;
    }
    unsafe {
        // Free old quality string if present
        if !(*opts).color_lossy_image_recompress_quality.is_null() {
            drop(CString::from_raw(
                (*opts).color_lossy_image_recompress_quality,
            ));
        }
        // Copy new quality string
        let q = CStr::from_ptr(quality);
        if let Ok(cstr) = CString::new(q.to_bytes()) {
            (*opts).color_lossy_image_recompress_quality = cstr.into_raw();
        }
    }
}

/// Set JPEG quality for grayscale images.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_set_gray_jpeg_quality(
    opts: *mut ImageRewriterOptions,
    quality: *const c_char,
) {
    if opts.is_null() || quality.is_null() {
        return;
    }
    unsafe {
        // Free old quality string if present
        if !(*opts).gray_lossy_image_recompress_quality.is_null() {
            drop(CString::from_raw(
                (*opts).gray_lossy_image_recompress_quality,
            ));
        }
        // Copy new quality string
        let q = CStr::from_ptr(quality);
        if let Ok(cstr) = CString::new(q.to_bytes()) {
            (*opts).gray_lossy_image_recompress_quality = cstr.into_raw();
        }
    }
}

// ============================================================================
// FFI Functions - Main Rewrite Function
// ============================================================================

/// Rewrite images within the given document.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_rewrite_images(
    _ctx: ContextHandle,
    _doc: DocumentHandle,
    _opts: *mut ImageRewriterOptions,
) {
    // In a full implementation, this would:
    // 1. Iterate through all images in the document
    // 2. For each image, determine if it should be subsampled based on DPI thresholds
    // 3. Resample the image if needed using the specified method
    // 4. Recompress the image using the specified method
    // 5. Replace the original image data in the PDF
}

/// Rewrite images and return statistics.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_rewrite_images_with_stats(
    _ctx: ContextHandle,
    _doc: DocumentHandle,
    _opts: *mut ImageRewriterOptions,
) -> ImageRewriteStats {
    // In a full implementation, this would rewrite images and collect stats
    ImageRewriteStats::default()
}

// ============================================================================
// FFI Functions - Image Analysis
// ============================================================================

/// Count images in document.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_count_images(_ctx: ContextHandle, _doc: DocumentHandle) -> i32 {
    // In a full implementation, this would count all images
    0
}

/// Get total image size in bytes.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_get_total_image_size(_ctx: ContextHandle, _doc: DocumentHandle) -> u64 {
    // In a full implementation, this would sum all image sizes
    0
}

/// Analyze images and return statistics without modifying.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_analyze_images(
    _ctx: ContextHandle,
    _doc: DocumentHandle,
) -> ImageRewriteStats {
    // In a full implementation, this would analyze images without modifying them
    ImageRewriteStats::default()
}

// ============================================================================
// FFI Functions - Options Cleanup
// ============================================================================

/// Free resources in image rewriter options.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_drop_image_rewriter_options(opts: *mut ImageRewriterOptions) {
    if opts.is_null() {
        return;
    }
    unsafe {
        // Free quality strings
        if !(*opts).color_lossy_image_recompress_quality.is_null() {
            drop(CString::from_raw(
                (*opts).color_lossy_image_recompress_quality,
            ));
            (*opts).color_lossy_image_recompress_quality = ptr::null_mut();
        }
        if !(*opts).color_lossless_image_recompress_quality.is_null() {
            drop(CString::from_raw(
                (*opts).color_lossless_image_recompress_quality,
            ));
            (*opts).color_lossless_image_recompress_quality = ptr::null_mut();
        }
        if !(*opts).gray_lossy_image_recompress_quality.is_null() {
            drop(CString::from_raw(
                (*opts).gray_lossy_image_recompress_quality,
            ));
            (*opts).gray_lossy_image_recompress_quality = ptr::null_mut();
        }
        if !(*opts).gray_lossless_image_recompress_quality.is_null() {
            drop(CString::from_raw(
                (*opts).gray_lossless_image_recompress_quality,
            ));
            (*opts).gray_lossless_image_recompress_quality = ptr::null_mut();
        }
        if !(*opts).bitonal_image_recompress_quality.is_null() {
            drop(CString::from_raw((*opts).bitonal_image_recompress_quality));
            (*opts).bitonal_image_recompress_quality = ptr::null_mut();
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_subsample_constants() {
        assert_eq!(FZ_SUBSAMPLE_AVERAGE, 0);
        assert_eq!(FZ_SUBSAMPLE_BICUBIC, 1);
    }

    #[test]
    fn test_recompress_constants() {
        assert_eq!(FZ_RECOMPRESS_NEVER, 0);
        assert_eq!(FZ_RECOMPRESS_SAME, 1);
        assert_eq!(FZ_RECOMPRESS_LOSSLESS, 2);
        assert_eq!(FZ_RECOMPRESS_JPEG, 3);
        assert_eq!(FZ_RECOMPRESS_J2K, 4);
        assert_eq!(FZ_RECOMPRESS_FAX, 5);
    }

    #[test]
    fn test_default_options() {
        let opts = ImageRewriterOptions::new();
        assert_eq!(
            opts.color_lossless_image_subsample_method,
            FZ_SUBSAMPLE_BICUBIC
        );
        assert_eq!(opts.color_lossless_image_subsample_threshold, 0);
        assert_eq!(
            opts.color_lossless_image_recompress_method,
            FZ_RECOMPRESS_SAME
        );
        assert_eq!(opts.bitonal_image_subsample_method, FZ_SUBSAMPLE_AVERAGE);
    }

    #[test]
    fn test_web_optimized() {
        let opts = ImageRewriterOptions::web_optimized();
        assert_eq!(opts.color_lossless_image_subsample_threshold, 150);
        assert_eq!(opts.color_lossless_image_subsample_to, 72);
        assert_eq!(
            opts.color_lossless_image_recompress_method,
            FZ_RECOMPRESS_JPEG
        );
    }

    #[test]
    fn test_print_quality() {
        let opts = ImageRewriterOptions::print_quality();
        assert_eq!(opts.color_lossless_image_subsample_threshold, 450);
        assert_eq!(opts.color_lossless_image_subsample_to, 300);
        assert_eq!(
            opts.color_lossless_image_recompress_method,
            FZ_RECOMPRESS_LOSSLESS
        );
    }

    #[test]
    fn test_ebook_quality() {
        let opts = ImageRewriterOptions::ebook_quality();
        assert_eq!(opts.color_lossless_image_subsample_threshold, 300);
        assert_eq!(opts.color_lossless_image_subsample_to, 150);
    }

    #[test]
    fn test_max_compression() {
        let opts = ImageRewriterOptions::max_compression();
        assert_eq!(opts.color_lossless_image_subsample_threshold, 100);
        assert_eq!(opts.color_lossless_image_subsample_to, 72);
    }

    #[test]
    fn test_stats_compression_ratio() {
        let mut stats = ImageRewriteStats::default();
        stats.original_size = 1000;
        stats.new_size = 500;
        assert!((stats.compression_ratio() - 2.0).abs() < 0.001);
    }

    #[test]
    fn test_stats_size_reduction() {
        let mut stats = ImageRewriteStats::default();
        stats.original_size = 1000;
        stats.new_size = 400;
        assert!((stats.size_reduction_percent() - 60.0).abs() < 0.001);
    }

    #[test]
    fn test_stats_zero_handling() {
        let stats = ImageRewriteStats::default();
        assert_eq!(stats.compression_ratio(), 0.0);
        assert_eq!(stats.size_reduction_percent(), 0.0);
    }

    #[test]
    fn test_ffi_default_options() {
        let opts = pdf_default_image_rewriter_options();
        assert_eq!(opts.color_lossless_image_subsample_threshold, 0);
    }

    #[test]
    fn test_ffi_preset_options() {
        let web = pdf_web_image_rewriter_options();
        assert_eq!(web.color_lossless_image_subsample_to, 72);

        let print = pdf_print_image_rewriter_options();
        assert_eq!(print.color_lossless_image_subsample_to, 300);

        let ebook = pdf_ebook_image_rewriter_options();
        assert_eq!(ebook.color_lossless_image_subsample_to, 150);

        let max = pdf_max_compression_image_rewriter_options();
        assert_eq!(max.color_lossless_image_subsample_to, 72);
    }

    #[test]
    fn test_ffi_set_color_subsample() {
        let mut opts = ImageRewriterOptions::new();
        pdf_set_color_subsample(&mut opts, 200, 100, FZ_SUBSAMPLE_AVERAGE);
        assert_eq!(opts.color_lossless_image_subsample_threshold, 200);
        assert_eq!(opts.color_lossless_image_subsample_to, 100);
        assert_eq!(
            opts.color_lossless_image_subsample_method,
            FZ_SUBSAMPLE_AVERAGE
        );
    }

    #[test]
    fn test_ffi_set_gray_subsample() {
        let mut opts = ImageRewriterOptions::new();
        pdf_set_gray_subsample(&mut opts, 300, 150, FZ_SUBSAMPLE_BICUBIC);
        assert_eq!(opts.gray_lossless_image_subsample_threshold, 300);
        assert_eq!(opts.gray_lossless_image_subsample_to, 150);
    }

    #[test]
    fn test_ffi_set_bitonal_subsample() {
        let mut opts = ImageRewriterOptions::new();
        pdf_set_bitonal_subsample(&mut opts, 600, 300, FZ_SUBSAMPLE_AVERAGE);
        assert_eq!(opts.bitonal_image_subsample_threshold, 600);
        assert_eq!(opts.bitonal_image_subsample_to, 300);
    }

    #[test]
    fn test_ffi_set_recompress() {
        let mut opts = ImageRewriterOptions::new();

        pdf_set_color_recompress(&mut opts, FZ_RECOMPRESS_JPEG);
        assert_eq!(
            opts.color_lossless_image_recompress_method,
            FZ_RECOMPRESS_JPEG
        );
        assert_eq!(opts.color_lossy_image_recompress_method, FZ_RECOMPRESS_JPEG);

        pdf_set_gray_recompress(&mut opts, FZ_RECOMPRESS_LOSSLESS);
        assert_eq!(
            opts.gray_lossless_image_recompress_method,
            FZ_RECOMPRESS_LOSSLESS
        );

        pdf_set_bitonal_recompress(&mut opts, FZ_RECOMPRESS_FAX);
        assert_eq!(opts.bitonal_image_recompress_method, FZ_RECOMPRESS_FAX);
    }

    #[test]
    fn test_ffi_analyze_images() {
        let stats = pdf_analyze_images(0, 0);
        assert_eq!(stats.images_processed, 0);
    }

    #[test]
    fn test_ffi_count_images() {
        let count = pdf_count_images(0, 0);
        assert_eq!(count, 0);
    }
}
