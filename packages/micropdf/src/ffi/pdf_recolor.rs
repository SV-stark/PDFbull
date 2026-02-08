//! PDF Recolor FFI Module
//!
//! Provides PDF color conversion functionality including page recoloring,
//! shade recoloring, and output intent management.

use crate::ffi::{Handle, HandleStore};
use std::ffi::c_void;
use std::sync::LazyLock;

// ============================================================================
// Type Aliases
// ============================================================================

type ContextHandle = Handle;
type DocumentHandle = Handle;
type ColorspaceHandle = Handle;
type ShadeHandle = Handle;

// ============================================================================
// Color Space Types
// ============================================================================

/// Grayscale color space
pub const RECOLOR_GRAY: i32 = 1;
/// RGB color space
pub const RECOLOR_RGB: i32 = 3;
/// CMYK color space
pub const RECOLOR_CMYK: i32 = 4;

// ============================================================================
// Recolor Options
// ============================================================================

/// Recolor options for page conversion
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct RecolorOptions {
    /// Number of components in target color space
    /// 1 = Gray, 3 = RGB, 4 = CMYK
    pub num_comp: i32,
}

impl Default for RecolorOptions {
    fn default() -> Self {
        Self::rgb()
    }
}

impl RecolorOptions {
    /// Create options for grayscale conversion
    pub fn gray() -> Self {
        Self {
            num_comp: RECOLOR_GRAY,
        }
    }

    /// Create options for RGB conversion
    pub fn rgb() -> Self {
        Self {
            num_comp: RECOLOR_RGB,
        }
    }

    /// Create options for CMYK conversion
    pub fn cmyk() -> Self {
        Self {
            num_comp: RECOLOR_CMYK,
        }
    }

    /// Check if options are valid
    pub fn is_valid(&self) -> bool {
        matches!(self.num_comp, 1 | 3 | 4)
    }
}

// ============================================================================
// Recolor Vertex (single color point)
// ============================================================================

/// A single vertex color for shade recoloring
#[derive(Debug, Clone)]
pub struct RecolorVertex {
    /// Source color components
    pub src_color: Vec<f32>,
    /// Destination color components
    pub dst_color: Vec<f32>,
    /// Source colorspace handle
    pub src_cs: ColorspaceHandle,
    /// Destination colorspace handle
    pub dst_cs: ColorspaceHandle,
}

impl RecolorVertex {
    pub fn new(src_components: usize, dst_components: usize) -> Self {
        Self {
            src_color: vec![0.0; src_components],
            dst_color: vec![0.0; dst_components],
            src_cs: 0,
            dst_cs: 0,
        }
    }
}

// ============================================================================
// Shade Recolorer Context
// ============================================================================

/// Callback type for recoloring a single vertex
pub type RecolorVertexFn = extern "C" fn(
    ctx: ContextHandle,
    opaque: *mut c_void,
    dst_cs: ColorspaceHandle,
    dst: *mut f32,
    src_cs: ColorspaceHandle,
    src: *const f32,
);

/// Callback type for shade recoloring decision
pub type ShadeRecolorerFn = extern "C" fn(
    ctx: ContextHandle,
    opaque: *mut c_void,
    src_cs: ColorspaceHandle,
    dst_cs: *mut ColorspaceHandle,
) -> *const RecolorVertexFn;

/// Context for shade recoloring operations
#[derive(Debug)]
pub struct ShadeRecolorContext {
    /// Source colorspace
    pub src_colorspace: ColorspaceHandle,
    /// Destination colorspace
    pub dst_colorspace: ColorspaceHandle,
    /// User-provided opaque data
    pub opaque: *mut c_void,
    /// Vertex recolor callback
    pub vertex_fn: Option<RecolorVertexFn>,
    /// Shade recolorer callback
    pub shade_fn: Option<ShadeRecolorerFn>,
}

unsafe impl Send for ShadeRecolorContext {}
unsafe impl Sync for ShadeRecolorContext {}

impl Default for ShadeRecolorContext {
    fn default() -> Self {
        Self::new()
    }
}

impl ShadeRecolorContext {
    pub fn new() -> Self {
        Self {
            src_colorspace: 0,
            dst_colorspace: 0,
            opaque: std::ptr::null_mut(),
            vertex_fn: None,
            shade_fn: None,
        }
    }

    pub fn with_colorspaces(src: ColorspaceHandle, dst: ColorspaceHandle) -> Self {
        Self {
            src_colorspace: src,
            dst_colorspace: dst,
            opaque: std::ptr::null_mut(),
            vertex_fn: None,
            shade_fn: None,
        }
    }
}

// ============================================================================
// Recolor Statistics
// ============================================================================

/// Statistics from recoloring operations
#[derive(Debug, Default, Clone)]
#[repr(C)]
pub struct RecolorStats {
    /// Pages processed
    pub pages_processed: i32,
    /// Colors converted
    pub colors_converted: i32,
    /// Shades recolored
    pub shades_recolored: i32,
    /// Images processed
    pub images_processed: i32,
    /// Output intents removed
    pub output_intents_removed: i32,
}

// ============================================================================
// Global Handle Store
// ============================================================================

pub static SHADE_RECOLOR_CONTEXTS: LazyLock<HandleStore<ShadeRecolorContext>> =
    LazyLock::new(HandleStore::new);

// ============================================================================
// FFI Functions - Recolor Options
// ============================================================================

/// Get grayscale recolor options.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_recolor_options_gray() -> RecolorOptions {
    RecolorOptions::gray()
}

/// Get RGB recolor options.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_recolor_options_rgb() -> RecolorOptions {
    RecolorOptions::rgb()
}

/// Get CMYK recolor options.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_recolor_options_cmyk() -> RecolorOptions {
    RecolorOptions::cmyk()
}

/// Create custom recolor options.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_recolor_options_new(num_comp: i32) -> RecolorOptions {
    RecolorOptions { num_comp }
}

/// Check if recolor options are valid.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_recolor_options_is_valid(opts: *const RecolorOptions) -> i32 {
    if opts.is_null() {
        return 0;
    }
    unsafe { if (*opts).is_valid() { 1 } else { 0 } }
}

// ============================================================================
// FFI Functions - Page Recoloring
// ============================================================================

/// Recolor a given document page.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_recolor_page(
    _ctx: ContextHandle,
    _doc: DocumentHandle,
    _pagenum: i32,
    _opts: *const RecolorOptions,
) {
    // In a full implementation, this would:
    // 1. Load the page content stream
    // 2. Process all color operators (g, rg, k, cs, scn, etc.)
    // 3. Convert colors to target colorspace
    // 4. Rewrite the content stream with new colors
}

/// Recolor all pages in a document.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_recolor_document(
    _ctx: ContextHandle,
    _doc: DocumentHandle,
    _opts: *const RecolorOptions,
) -> RecolorStats {
    // In a full implementation, this would recolor all pages
    RecolorStats::default()
}

/// Recolor a range of pages.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_recolor_pages(
    _ctx: ContextHandle,
    _doc: DocumentHandle,
    _start_page: i32,
    _end_page: i32,
    _opts: *const RecolorOptions,
) -> RecolorStats {
    // In a full implementation, this would recolor specified pages
    RecolorStats::default()
}

// ============================================================================
// FFI Functions - Output Intents
// ============================================================================

/// Remove output intents from a document.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_remove_output_intents(_ctx: ContextHandle, _doc: DocumentHandle) {
    // In a full implementation, this would:
    // 1. Access the document catalog
    // 2. Remove the OutputIntents array
    // 3. Update any ICC profile references
}

/// Count output intents in a document.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_count_output_intents(_ctx: ContextHandle, _doc: DocumentHandle) -> i32 {
    // In a full implementation, this would count OutputIntents
    0
}

// ============================================================================
// FFI Functions - Shade Recoloring
// ============================================================================

/// Create a shade recolor context.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_new_shade_recolor_context(
    _ctx: ContextHandle,
    src_cs: ColorspaceHandle,
    dst_cs: ColorspaceHandle,
) -> Handle {
    let context = ShadeRecolorContext::with_colorspaces(src_cs, dst_cs);
    SHADE_RECOLOR_CONTEXTS.insert(context)
}

/// Drop a shade recolor context.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_drop_shade_recolor_context(_ctx: ContextHandle, recolor_ctx: Handle) {
    SHADE_RECOLOR_CONTEXTS.remove(recolor_ctx);
}

/// Set opaque data for shade recolor context.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_shade_recolor_set_opaque(
    _ctx: ContextHandle,
    recolor_ctx: Handle,
    opaque: *mut c_void,
) {
    if let Some(ctx_arc) = SHADE_RECOLOR_CONTEXTS.get(recolor_ctx) {
        let mut c = ctx_arc.lock().unwrap();
        c.opaque = opaque;
    }
}

/// Recolor a shade object.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_recolor_shade(
    _ctx: ContextHandle,
    _shade: ShadeHandle,
    _recolor_ctx: Handle,
) -> ShadeHandle {
    // In a full implementation, this would:
    // 1. Get the shade's colorspace and function
    // 2. Convert colors using the recolor context
    // 3. Create a new shade with converted colors
    0
}

// ============================================================================
// FFI Functions - Color Conversion Utilities
// ============================================================================

/// Convert a single color from one colorspace to another.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_convert_color(
    _ctx: ContextHandle,
    _src_cs: ColorspaceHandle,
    src: *const f32,
    src_n: i32,
    _dst_cs: ColorspaceHandle,
    dst: *mut f32,
    dst_n: i32,
) {
    if src.is_null() || dst.is_null() {
        return;
    }

    // Basic conversion - in a full implementation this would use
    // proper color management
    unsafe {
        match (src_n, dst_n) {
            // Gray to RGB
            (1, 3) => {
                let gray = *src;
                *dst = gray;
                *dst.add(1) = gray;
                *dst.add(2) = gray;
            }
            // RGB to Gray
            (3, 1) => {
                let r = *src;
                let g = *src.add(1);
                let b = *src.add(2);
                // Standard luminance formula
                *dst = 0.299 * r + 0.587 * g + 0.114 * b;
            }
            // CMYK to RGB
            (4, 3) => {
                let c = *src;
                let m = *src.add(1);
                let y = *src.add(2);
                let k = *src.add(3);
                *dst = (1.0 - c) * (1.0 - k);
                *dst.add(1) = (1.0 - m) * (1.0 - k);
                *dst.add(2) = (1.0 - y) * (1.0 - k);
            }
            // RGB to CMYK
            (3, 4) => {
                let r = *src;
                let g = *src.add(1);
                let b = *src.add(2);
                let k = 1.0 - r.max(g).max(b);
                if k < 1.0 {
                    *dst = (1.0 - r - k) / (1.0 - k);
                    *dst.add(1) = (1.0 - g - k) / (1.0 - k);
                    *dst.add(2) = (1.0 - b - k) / (1.0 - k);
                } else {
                    *dst = 0.0;
                    *dst.add(1) = 0.0;
                    *dst.add(2) = 0.0;
                }
                *dst.add(3) = k;
            }
            // Same number of components - direct copy
            _ if src_n == dst_n => {
                for i in 0..src_n as usize {
                    *dst.add(i) = *src.add(i);
                }
            }
            // Unsupported conversion
            _ => {
                for i in 0..dst_n as usize {
                    *dst.add(i) = 0.0;
                }
            }
        }
    }
}

/// Convert gray to RGB.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_gray_to_rgb(gray: f32, r: *mut f32, g: *mut f32, b: *mut f32) {
    if !r.is_null() {
        unsafe { *r = gray };
    }
    if !g.is_null() {
        unsafe { *g = gray };
    }
    if !b.is_null() {
        unsafe { *b = gray };
    }
}

/// Convert RGB to gray.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_rgb_to_gray(r: f32, g: f32, b: f32) -> f32 {
    0.299 * r + 0.587 * g + 0.114 * b
}

/// Convert CMYK to RGB.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_cmyk_to_rgb(
    c: f32,
    m: f32,
    y: f32,
    k: f32,
    r: *mut f32,
    g: *mut f32,
    b: *mut f32,
) {
    if !r.is_null() {
        unsafe { *r = (1.0 - c) * (1.0 - k) };
    }
    if !g.is_null() {
        unsafe { *g = (1.0 - m) * (1.0 - k) };
    }
    if !b.is_null() {
        unsafe { *b = (1.0 - y) * (1.0 - k) };
    }
}

/// Convert RGB to CMYK.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_rgb_to_cmyk(
    r: f32,
    g: f32,
    b: f32,
    c: *mut f32,
    m: *mut f32,
    y: *mut f32,
    k: *mut f32,
) {
    let k_val = 1.0 - r.max(g).max(b);
    if !k.is_null() {
        unsafe { *k = k_val };
    }
    if k_val < 1.0 {
        if !c.is_null() {
            unsafe { *c = (1.0 - r - k_val) / (1.0 - k_val) };
        }
        if !m.is_null() {
            unsafe { *m = (1.0 - g - k_val) / (1.0 - k_val) };
        }
        if !y.is_null() {
            unsafe { *y = (1.0 - b - k_val) / (1.0 - k_val) };
        }
    } else {
        if !c.is_null() {
            unsafe { *c = 0.0 };
        }
        if !m.is_null() {
            unsafe { *m = 0.0 };
        }
        if !y.is_null() {
            unsafe { *y = 0.0 };
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
    fn test_recolor_constants() {
        assert_eq!(RECOLOR_GRAY, 1);
        assert_eq!(RECOLOR_RGB, 3);
        assert_eq!(RECOLOR_CMYK, 4);
    }

    #[test]
    fn test_recolor_options_gray() {
        let opts = RecolorOptions::gray();
        assert_eq!(opts.num_comp, 1);
        assert!(opts.is_valid());
    }

    #[test]
    fn test_recolor_options_rgb() {
        let opts = RecolorOptions::rgb();
        assert_eq!(opts.num_comp, 3);
        assert!(opts.is_valid());
    }

    #[test]
    fn test_recolor_options_cmyk() {
        let opts = RecolorOptions::cmyk();
        assert_eq!(opts.num_comp, 4);
        assert!(opts.is_valid());
    }

    #[test]
    fn test_recolor_options_invalid() {
        let opts = RecolorOptions { num_comp: 5 };
        assert!(!opts.is_valid());

        let opts = RecolorOptions { num_comp: 0 };
        assert!(!opts.is_valid());
    }

    #[test]
    fn test_recolor_vertex() {
        let v = RecolorVertex::new(3, 4);
        assert_eq!(v.src_color.len(), 3);
        assert_eq!(v.dst_color.len(), 4);
    }

    #[test]
    fn test_shade_recolor_context() {
        let ctx = ShadeRecolorContext::new();
        assert_eq!(ctx.src_colorspace, 0);
        assert_eq!(ctx.dst_colorspace, 0);
        assert!(ctx.opaque.is_null());

        let ctx2 = ShadeRecolorContext::with_colorspaces(1, 2);
        assert_eq!(ctx2.src_colorspace, 1);
        assert_eq!(ctx2.dst_colorspace, 2);
    }

    #[test]
    fn test_recolor_stats() {
        let stats = RecolorStats::default();
        assert_eq!(stats.pages_processed, 0);
        assert_eq!(stats.colors_converted, 0);
        assert_eq!(stats.shades_recolored, 0);
    }

    #[test]
    fn test_ffi_options() {
        let gray = pdf_recolor_options_gray();
        assert_eq!(gray.num_comp, 1);

        let rgb = pdf_recolor_options_rgb();
        assert_eq!(rgb.num_comp, 3);

        let cmyk = pdf_recolor_options_cmyk();
        assert_eq!(cmyk.num_comp, 4);

        let custom = pdf_recolor_options_new(3);
        assert_eq!(custom.num_comp, 3);
    }

    #[test]
    fn test_ffi_options_valid() {
        let valid = pdf_recolor_options_rgb();
        assert_eq!(pdf_recolor_options_is_valid(&valid), 1);

        let invalid = RecolorOptions { num_comp: 7 };
        assert_eq!(pdf_recolor_options_is_valid(&invalid), 0);
    }

    #[test]
    fn test_ffi_shade_context() {
        let ctx = 0;
        let handle = pdf_new_shade_recolor_context(ctx, 1, 2);
        assert!(handle > 0);

        pdf_drop_shade_recolor_context(ctx, handle);
    }

    #[test]
    fn test_gray_to_rgb() {
        let mut r = 0.0f32;
        let mut g = 0.0f32;
        let mut b = 0.0f32;

        pdf_gray_to_rgb(0.5, &mut r, &mut g, &mut b);
        assert!((r - 0.5).abs() < 0.001);
        assert!((g - 0.5).abs() < 0.001);
        assert!((b - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_rgb_to_gray() {
        // Pure white
        let gray = pdf_rgb_to_gray(1.0, 1.0, 1.0);
        assert!((gray - 1.0).abs() < 0.001);

        // Pure black
        let gray = pdf_rgb_to_gray(0.0, 0.0, 0.0);
        assert!((gray - 0.0).abs() < 0.001);

        // Mid-gray
        let gray = pdf_rgb_to_gray(0.5, 0.5, 0.5);
        assert!((gray - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_cmyk_to_rgb() {
        let mut r = 0.0f32;
        let mut g = 0.0f32;
        let mut b = 0.0f32;

        // Pure black (K=1)
        pdf_cmyk_to_rgb(0.0, 0.0, 0.0, 1.0, &mut r, &mut g, &mut b);
        assert!((r - 0.0).abs() < 0.001);
        assert!((g - 0.0).abs() < 0.001);
        assert!((b - 0.0).abs() < 0.001);

        // Pure white (no ink)
        pdf_cmyk_to_rgb(0.0, 0.0, 0.0, 0.0, &mut r, &mut g, &mut b);
        assert!((r - 1.0).abs() < 0.001);
        assert!((g - 1.0).abs() < 0.001);
        assert!((b - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_rgb_to_cmyk() {
        let mut c = 0.0f32;
        let mut m = 0.0f32;
        let mut y = 0.0f32;
        let mut k = 0.0f32;

        // Pure black
        pdf_rgb_to_cmyk(0.0, 0.0, 0.0, &mut c, &mut m, &mut y, &mut k);
        assert!((k - 1.0).abs() < 0.001);

        // Pure white
        pdf_rgb_to_cmyk(1.0, 1.0, 1.0, &mut c, &mut m, &mut y, &mut k);
        assert!((k - 0.0).abs() < 0.001);
        assert!((c - 0.0).abs() < 0.001);
        assert!((m - 0.0).abs() < 0.001);
        assert!((y - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_convert_color_gray_to_rgb() {
        let src = [0.5f32];
        let mut dst = [0.0f32; 3];

        pdf_convert_color(0, 0, src.as_ptr(), 1, 0, dst.as_mut_ptr(), 3);
        assert!((dst[0] - 0.5).abs() < 0.001);
        assert!((dst[1] - 0.5).abs() < 0.001);
        assert!((dst[2] - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_convert_color_rgb_to_gray() {
        let src = [0.5f32, 0.5, 0.5];
        let mut dst = [0.0f32];

        pdf_convert_color(0, 0, src.as_ptr(), 3, 0, dst.as_mut_ptr(), 1);
        assert!((dst[0] - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_convert_color_same() {
        let src = [0.1f32, 0.2, 0.3];
        let mut dst = [0.0f32; 3];

        pdf_convert_color(0, 0, src.as_ptr(), 3, 0, dst.as_mut_ptr(), 3);
        assert!((dst[0] - 0.1).abs() < 0.001);
        assert!((dst[1] - 0.2).abs() < 0.001);
        assert!((dst[2] - 0.3).abs() < 0.001);
    }
}
