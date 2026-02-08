//! FFI bindings for fz_color (Color Management)
//!
//! This module provides color handling parameters including rendering intent,
//! black point compensation, overprint settings, and default colorspace management.

use std::ffi::{CStr, c_char};
use std::sync::LazyLock;

use crate::ffi::colorspace::{
    ColorspaceHandle, FZ_COLORSPACE_CMYK, FZ_COLORSPACE_GRAY, FZ_COLORSPACE_RGB,
};
use crate::ffi::{Handle, HandleStore};

// ============================================================================
// Rendering Intent
// ============================================================================

/// Rendering intent enumeration
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RenderingIntent {
    /// Perceptual - maintains visual relationship between colors
    #[default]
    Perceptual = 0,
    /// Relative colorimetric - maps white point, clips out-of-gamut
    RelativeColorimetric = 1,
    /// Saturation - maintains saturation at expense of hue/lightness
    Saturation = 2,
    /// Absolute colorimetric - no white point mapping
    AbsoluteColorimetric = 3,
}

impl RenderingIntent {
    /// Get rendering intent from integer
    pub fn from_i32(value: i32) -> Self {
        match value & 0x7F {
            // Mask off the softmask bit
            0 => RenderingIntent::Perceptual,
            1 => RenderingIntent::RelativeColorimetric,
            2 => RenderingIntent::Saturation,
            3 => RenderingIntent::AbsoluteColorimetric,
            _ => RenderingIntent::Perceptual,
        }
    }

    /// Get rendering intent name
    pub fn name(&self) -> &'static str {
        match self {
            RenderingIntent::Perceptual => "Perceptual",
            RenderingIntent::RelativeColorimetric => "RelativeColorimetric",
            RenderingIntent::Saturation => "Saturation",
            RenderingIntent::AbsoluteColorimetric => "AbsoluteColorimetric",
        }
    }

    /// Get rendering intent from name
    pub fn from_name(name: &str) -> Self {
        match name.to_lowercase().as_str() {
            "perceptual" => RenderingIntent::Perceptual,
            "relativecolorimetric" | "relative" => RenderingIntent::RelativeColorimetric,
            "saturation" => RenderingIntent::Saturation,
            "absolutecolorimetric" | "absolute" => RenderingIntent::AbsoluteColorimetric,
            _ => RenderingIntent::Perceptual,
        }
    }
}

// ============================================================================
// Color Parameters
// ============================================================================

/// Color handling parameters
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct ColorParams {
    /// Rendering intent (0-3)
    pub ri: u8,
    /// Black point compensation (0 or 1)
    pub bp: u8,
    /// Overprinting (0 or 1)
    pub op: u8,
    /// Overprint mode (0 or 1)
    pub opm: u8,
}

impl ColorParams {
    /// Create new color parameters
    pub fn new(ri: RenderingIntent, bp: bool, op: bool, opm: bool) -> Self {
        Self {
            ri: ri as u8,
            bp: if bp { 1 } else { 0 },
            op: if op { 1 } else { 0 },
            opm: if opm { 1 } else { 0 },
        }
    }

    /// Get rendering intent
    pub fn rendering_intent(&self) -> RenderingIntent {
        RenderingIntent::from_i32(self.ri as i32)
    }

    /// Check if black point compensation is enabled
    pub fn black_point_compensation(&self) -> bool {
        self.bp != 0
    }

    /// Check if overprinting is enabled
    pub fn overprint(&self) -> bool {
        self.op != 0
    }

    /// Check if overprint mode is enabled
    pub fn overprint_mode(&self) -> bool {
        self.opm != 0
    }
}

/// Default color parameters (global constant)
pub static DEFAULT_COLOR_PARAMS: ColorParams = ColorParams {
    ri: 0,  // Perceptual
    bp: 1,  // Black point compensation enabled
    op: 0,  // Overprinting disabled
    opm: 0, // Overprint mode 0
};

// ============================================================================
// Default Colorspaces
// ============================================================================

/// Default colorspaces structure
#[derive(Debug, Clone)]
pub struct DefaultColorspaces {
    /// Reference count
    pub refs: i32,
    /// Default gray colorspace
    pub gray: ColorspaceHandle,
    /// Default RGB colorspace
    pub rgb: ColorspaceHandle,
    /// Default CMYK colorspace
    pub cmyk: ColorspaceHandle,
    /// Output intent colorspace
    pub oi: ColorspaceHandle,
}

impl Default for DefaultColorspaces {
    fn default() -> Self {
        Self {
            refs: 1,
            gray: FZ_COLORSPACE_GRAY,
            rgb: FZ_COLORSPACE_RGB,
            cmyk: FZ_COLORSPACE_CMYK,
            oi: 0,
        }
    }
}

/// Global store for default colorspaces
pub static DEFAULT_COLORSPACES: LazyLock<HandleStore<DefaultColorspaces>> =
    LazyLock::new(HandleStore::new);

// ============================================================================
// Calibrated Colorspaces
// ============================================================================

/// Calibrated gray colorspace parameters
#[derive(Debug, Clone)]
pub struct CalGray {
    /// White point (X, Y, Z)
    pub white_point: [f32; 3],
    /// Black point (X, Y, Z)
    pub black_point: [f32; 3],
    /// Gamma value
    pub gamma: f32,
}

impl Default for CalGray {
    fn default() -> Self {
        Self {
            white_point: [0.9505, 1.0, 1.089], // D65
            black_point: [0.0, 0.0, 0.0],
            gamma: 2.2,
        }
    }
}

/// Calibrated RGB colorspace parameters
#[derive(Debug, Clone)]
pub struct CalRgb {
    /// White point (X, Y, Z)
    pub white_point: [f32; 3],
    /// Black point (X, Y, Z)
    pub black_point: [f32; 3],
    /// Gamma values (R, G, B)
    pub gamma: [f32; 3],
    /// Transformation matrix (3x3)
    pub matrix: [f32; 9],
}

impl Default for CalRgb {
    fn default() -> Self {
        Self {
            white_point: [0.9505, 1.0, 1.089], // D65
            black_point: [0.0, 0.0, 0.0],
            gamma: [2.2, 2.2, 2.2],
            // sRGB matrix
            matrix: [
                0.4124564, 0.3575761, 0.1804375, 0.2126729, 0.7151522, 0.0721750, 0.0193339,
                0.1191920, 0.9503041,
            ],
        }
    }
}

// ============================================================================
// FFI Functions - Rendering Intent
// ============================================================================

/// Lookup rendering intent by name
#[unsafe(no_mangle)]
pub extern "C" fn fz_lookup_rendering_intent(name: *const c_char) -> i32 {
    if name.is_null() {
        return RenderingIntent::Perceptual as i32;
    }

    let name_str = unsafe { CStr::from_ptr(name).to_str().unwrap_or("") };
    RenderingIntent::from_name(name_str) as i32
}

/// Get rendering intent name
#[unsafe(no_mangle)]
pub extern "C" fn fz_rendering_intent_name(ri: i32) -> *const c_char {
    match RenderingIntent::from_i32(ri) {
        RenderingIntent::Perceptual => c"Perceptual".as_ptr(),
        RenderingIntent::RelativeColorimetric => c"RelativeColorimetric".as_ptr(),
        RenderingIntent::Saturation => c"Saturation".as_ptr(),
        RenderingIntent::AbsoluteColorimetric => c"AbsoluteColorimetric".as_ptr(),
    }
}

// ============================================================================
// FFI Functions - Color Parameters
// ============================================================================

/// Get default color parameters
#[unsafe(no_mangle)]
pub extern "C" fn fz_default_color_params() -> ColorParams {
    DEFAULT_COLOR_PARAMS
}

/// Create new color parameters
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_color_params(ri: i32, bp: i32, op: i32, opm: i32) -> ColorParams {
    ColorParams {
        ri: ri as u8,
        bp: bp as u8,
        op: op as u8,
        opm: opm as u8,
    }
}

/// Get rendering intent from color params
#[unsafe(no_mangle)]
pub extern "C" fn fz_color_params_ri(params: ColorParams) -> i32 {
    params.ri as i32
}

/// Get black point compensation from color params
#[unsafe(no_mangle)]
pub extern "C" fn fz_color_params_bp(params: ColorParams) -> i32 {
    params.bp as i32
}

/// Get overprint from color params
#[unsafe(no_mangle)]
pub extern "C" fn fz_color_params_op(params: ColorParams) -> i32 {
    params.op as i32
}

/// Get overprint mode from color params
#[unsafe(no_mangle)]
pub extern "C" fn fz_color_params_opm(params: ColorParams) -> i32 {
    params.opm as i32
}

// ============================================================================
// FFI Functions - Default Colorspaces
// ============================================================================

/// Create new default colorspaces structure
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_default_colorspaces(_ctx: Handle) -> Handle {
    DEFAULT_COLORSPACES.insert(DefaultColorspaces::default())
}

/// Keep (increment ref) default colorspaces
#[unsafe(no_mangle)]
pub extern "C" fn fz_keep_default_colorspaces(_ctx: Handle, default_cs: Handle) -> Handle {
    if let Some(cs) = DEFAULT_COLORSPACES.get(default_cs) {
        let mut guard = cs.lock().unwrap();
        guard.refs += 1;
    }
    default_cs
}

/// Drop default colorspaces reference
#[unsafe(no_mangle)]
pub extern "C" fn fz_drop_default_colorspaces(_ctx: Handle, default_cs: Handle) {
    if let Some(cs) = DEFAULT_COLORSPACES.get(default_cs) {
        let mut guard = cs.lock().unwrap();
        guard.refs -= 1;
        if guard.refs <= 0 {
            drop(guard);
            DEFAULT_COLORSPACES.remove(default_cs);
        }
    }
}

/// Clone default colorspaces
#[unsafe(no_mangle)]
pub extern "C" fn fz_clone_default_colorspaces(_ctx: Handle, base: Handle) -> Handle {
    if let Some(base_cs) = DEFAULT_COLORSPACES.get(base) {
        let guard = base_cs.lock().unwrap();
        let new_cs = DefaultColorspaces {
            refs: 1,
            gray: guard.gray,
            rgb: guard.rgb,
            cmyk: guard.cmyk,
            oi: guard.oi,
        };
        DEFAULT_COLORSPACES.insert(new_cs)
    } else {
        fz_new_default_colorspaces(_ctx)
    }
}

/// Get default gray colorspace
#[unsafe(no_mangle)]
pub extern "C" fn fz_default_gray(_ctx: Handle, default_cs: Handle) -> ColorspaceHandle {
    if default_cs == 0 {
        return FZ_COLORSPACE_GRAY;
    }
    if let Some(cs) = DEFAULT_COLORSPACES.get(default_cs) {
        cs.lock().unwrap().gray
    } else {
        FZ_COLORSPACE_GRAY
    }
}

/// Get default RGB colorspace
#[unsafe(no_mangle)]
pub extern "C" fn fz_default_rgb(_ctx: Handle, default_cs: Handle) -> ColorspaceHandle {
    if default_cs == 0 {
        return FZ_COLORSPACE_RGB;
    }
    if let Some(cs) = DEFAULT_COLORSPACES.get(default_cs) {
        cs.lock().unwrap().rgb
    } else {
        FZ_COLORSPACE_RGB
    }
}

/// Get default CMYK colorspace
#[unsafe(no_mangle)]
pub extern "C" fn fz_default_cmyk(_ctx: Handle, default_cs: Handle) -> ColorspaceHandle {
    if default_cs == 0 {
        return FZ_COLORSPACE_CMYK;
    }
    if let Some(cs) = DEFAULT_COLORSPACES.get(default_cs) {
        cs.lock().unwrap().cmyk
    } else {
        FZ_COLORSPACE_CMYK
    }
}

/// Get default output intent colorspace
#[unsafe(no_mangle)]
pub extern "C" fn fz_default_output_intent(_ctx: Handle, default_cs: Handle) -> ColorspaceHandle {
    if default_cs == 0 {
        return 0;
    }
    if let Some(cs) = DEFAULT_COLORSPACES.get(default_cs) {
        cs.lock().unwrap().oi
    } else {
        0
    }
}

/// Set default gray colorspace
#[unsafe(no_mangle)]
pub extern "C" fn fz_set_default_gray(_ctx: Handle, default_cs: Handle, cs: ColorspaceHandle) {
    if let Some(dcs) = DEFAULT_COLORSPACES.get(default_cs) {
        dcs.lock().unwrap().gray = cs;
    }
}

/// Set default RGB colorspace
#[unsafe(no_mangle)]
pub extern "C" fn fz_set_default_rgb(_ctx: Handle, default_cs: Handle, cs: ColorspaceHandle) {
    if let Some(dcs) = DEFAULT_COLORSPACES.get(default_cs) {
        dcs.lock().unwrap().rgb = cs;
    }
}

/// Set default CMYK colorspace
#[unsafe(no_mangle)]
pub extern "C" fn fz_set_default_cmyk(_ctx: Handle, default_cs: Handle, cs: ColorspaceHandle) {
    if let Some(dcs) = DEFAULT_COLORSPACES.get(default_cs) {
        dcs.lock().unwrap().cmyk = cs;
    }
}

/// Set default output intent colorspace
#[unsafe(no_mangle)]
pub extern "C" fn fz_set_default_output_intent(
    _ctx: Handle,
    default_cs: Handle,
    cs: ColorspaceHandle,
) {
    if let Some(dcs) = DEFAULT_COLORSPACES.get(default_cs) {
        dcs.lock().unwrap().oi = cs;
    }
}

// ============================================================================
// FFI Functions - Calibrated Colorspaces
// ============================================================================

/// Create a calibrated gray colorspace
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_cal_gray_colorspace(
    _ctx: Handle,
    wp: *const f32,
    bp: *const f32,
    gamma: f32,
) -> ColorspaceHandle {
    let mut cal_gray = CalGray::default();
    cal_gray.gamma = gamma;

    if !wp.is_null() {
        let wp_slice = unsafe { std::slice::from_raw_parts(wp, 3) };
        cal_gray.white_point.copy_from_slice(wp_slice);
    }

    if !bp.is_null() {
        let bp_slice = unsafe { std::slice::from_raw_parts(bp, 3) };
        cal_gray.black_point.copy_from_slice(bp_slice);
    }

    // For now, return device gray (full ICC/Cal support would require more work)
    FZ_COLORSPACE_GRAY
}

/// Create a calibrated RGB colorspace
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_cal_rgb_colorspace(
    _ctx: Handle,
    wp: *const f32,
    bp: *const f32,
    gamma: *const f32,
    matrix: *const f32,
) -> ColorspaceHandle {
    let mut cal_rgb = CalRgb::default();

    if !wp.is_null() {
        let wp_slice = unsafe { std::slice::from_raw_parts(wp, 3) };
        cal_rgb.white_point.copy_from_slice(wp_slice);
    }

    if !bp.is_null() {
        let bp_slice = unsafe { std::slice::from_raw_parts(bp, 3) };
        cal_rgb.black_point.copy_from_slice(bp_slice);
    }

    if !gamma.is_null() {
        let gamma_slice = unsafe { std::slice::from_raw_parts(gamma, 3) };
        cal_rgb.gamma.copy_from_slice(gamma_slice);
    }

    if !matrix.is_null() {
        let matrix_slice = unsafe { std::slice::from_raw_parts(matrix, 9) };
        cal_rgb.matrix.copy_from_slice(matrix_slice);
    }

    // For now, return device RGB (full ICC/Cal support would require more work)
    FZ_COLORSPACE_RGB
}

// ============================================================================
// FFI Functions - Colorspace Validation
// ============================================================================

/// Check if colorspace is valid for blending
#[unsafe(no_mangle)]
pub extern "C" fn fz_is_valid_blend_colorspace(_ctx: Handle, cs: ColorspaceHandle) -> i32 {
    match cs {
        FZ_COLORSPACE_GRAY | FZ_COLORSPACE_RGB | FZ_COLORSPACE_CMYK => 1,
        _ => 0,
    }
}

/// Get colorspace digest (MD5 of ICC profile if applicable)
#[unsafe(no_mangle)]
pub extern "C" fn fz_colorspace_digest(_ctx: Handle, _cs: ColorspaceHandle, digest: *mut u8) {
    if digest.is_null() {
        return;
    }

    // For non-ICC colorspaces, fill with zeros
    unsafe {
        std::ptr::write_bytes(digest, 0, 16);
    }
}

// ============================================================================
// FFI Functions - Color Conversion with Params
// ============================================================================

/// Convert color with color parameters
#[unsafe(no_mangle)]
pub extern "C" fn fz_convert_color_with_params(
    _ctx: Handle,
    src_cs: ColorspaceHandle,
    src: *const f32,
    dst_cs: ColorspaceHandle,
    dst: *mut f32,
    proof_cs: ColorspaceHandle,
    _params: ColorParams,
) {
    // Delegate to existing convert function (params would affect ICC conversions)
    crate::ffi::colorspace::fz_convert_color(_ctx, src_cs, src, dst_cs, dst, proof_cs);
}

// ============================================================================
// Maximum Colors Constant
// ============================================================================

/// Maximum number of colorants in any colorspace
pub const FZ_MAX_COLORS: i32 = 32;

/// Get maximum colors constant
#[unsafe(no_mangle)]
pub extern "C" fn fz_max_colors() -> i32 {
    FZ_MAX_COLORS
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rendering_intent_from_i32() {
        assert_eq!(RenderingIntent::from_i32(0), RenderingIntent::Perceptual);
        assert_eq!(
            RenderingIntent::from_i32(1),
            RenderingIntent::RelativeColorimetric
        );
        assert_eq!(RenderingIntent::from_i32(2), RenderingIntent::Saturation);
        assert_eq!(
            RenderingIntent::from_i32(3),
            RenderingIntent::AbsoluteColorimetric
        );
        assert_eq!(RenderingIntent::from_i32(99), RenderingIntent::Perceptual);
    }

    #[test]
    fn test_rendering_intent_name() {
        assert_eq!(RenderingIntent::Perceptual.name(), "Perceptual");
        assert_eq!(
            RenderingIntent::RelativeColorimetric.name(),
            "RelativeColorimetric"
        );
    }

    #[test]
    fn test_rendering_intent_from_name() {
        assert_eq!(
            RenderingIntent::from_name("Perceptual"),
            RenderingIntent::Perceptual
        );
        assert_eq!(
            RenderingIntent::from_name("perceptual"),
            RenderingIntent::Perceptual
        );
        assert_eq!(
            RenderingIntent::from_name("RelativeColorimetric"),
            RenderingIntent::RelativeColorimetric
        );
        assert_eq!(
            RenderingIntent::from_name("Relative"),
            RenderingIntent::RelativeColorimetric
        );
        assert_eq!(
            RenderingIntent::from_name("unknown"),
            RenderingIntent::Perceptual
        );
    }

    #[test]
    fn test_color_params_new() {
        let params = ColorParams::new(RenderingIntent::Saturation, true, false, true);
        assert_eq!(params.ri, 2);
        assert_eq!(params.bp, 1);
        assert_eq!(params.op, 0);
        assert_eq!(params.opm, 1);
    }

    #[test]
    fn test_color_params_accessors() {
        let params = ColorParams::new(RenderingIntent::RelativeColorimetric, true, true, false);
        assert_eq!(
            params.rendering_intent(),
            RenderingIntent::RelativeColorimetric
        );
        assert!(params.black_point_compensation());
        assert!(params.overprint());
        assert!(!params.overprint_mode());
    }

    #[test]
    fn test_default_color_params() {
        let params = fz_default_color_params();
        assert_eq!(params.ri, 0);
        assert_eq!(params.bp, 1);
    }

    #[test]
    fn test_lookup_rendering_intent() {
        let name = c"Perceptual";
        assert_eq!(fz_lookup_rendering_intent(name.as_ptr()), 0);

        let name = c"Saturation";
        assert_eq!(fz_lookup_rendering_intent(name.as_ptr()), 2);

        assert_eq!(fz_lookup_rendering_intent(std::ptr::null()), 0);
    }

    #[test]
    fn test_rendering_intent_name_ffi() {
        let name = fz_rendering_intent_name(0);
        assert!(!name.is_null());

        let name = fz_rendering_intent_name(2);
        assert!(!name.is_null());
    }

    #[test]
    fn test_new_color_params() {
        let params = fz_new_color_params(1, 1, 0, 1);
        assert_eq!(fz_color_params_ri(params), 1);
        assert_eq!(fz_color_params_bp(params), 1);
        assert_eq!(fz_color_params_op(params), 0);
        assert_eq!(fz_color_params_opm(params), 1);
    }

    #[test]
    fn test_default_colorspaces() {
        let ctx = 1;
        let dcs = fz_new_default_colorspaces(ctx);
        assert!(dcs > 0);

        assert_eq!(fz_default_gray(ctx, dcs), FZ_COLORSPACE_GRAY);
        assert_eq!(fz_default_rgb(ctx, dcs), FZ_COLORSPACE_RGB);
        assert_eq!(fz_default_cmyk(ctx, dcs), FZ_COLORSPACE_CMYK);
        assert_eq!(fz_default_output_intent(ctx, dcs), 0);

        fz_drop_default_colorspaces(ctx, dcs);
    }

    #[test]
    fn test_default_colorspaces_null() {
        let ctx = 1;
        assert_eq!(fz_default_gray(ctx, 0), FZ_COLORSPACE_GRAY);
        assert_eq!(fz_default_rgb(ctx, 0), FZ_COLORSPACE_RGB);
        assert_eq!(fz_default_cmyk(ctx, 0), FZ_COLORSPACE_CMYK);
    }

    #[test]
    fn test_set_default_colorspaces() {
        let ctx = 1;
        let dcs = fz_new_default_colorspaces(ctx);

        // Set a different gray colorspace
        fz_set_default_gray(ctx, dcs, FZ_COLORSPACE_RGB);
        assert_eq!(fz_default_gray(ctx, dcs), FZ_COLORSPACE_RGB);

        fz_drop_default_colorspaces(ctx, dcs);
    }

    #[test]
    fn test_clone_default_colorspaces() {
        let ctx = 1;
        let dcs1 = fz_new_default_colorspaces(ctx);
        fz_set_default_gray(ctx, dcs1, FZ_COLORSPACE_RGB);

        let dcs2 = fz_clone_default_colorspaces(ctx, dcs1);
        assert!(dcs2 > 0);
        assert_ne!(dcs1, dcs2);
        assert_eq!(fz_default_gray(ctx, dcs2), FZ_COLORSPACE_RGB);

        fz_drop_default_colorspaces(ctx, dcs1);
        fz_drop_default_colorspaces(ctx, dcs2);
    }

    #[test]
    fn test_keep_drop_default_colorspaces() {
        let ctx = 1;
        let dcs = fz_new_default_colorspaces(ctx);

        let dcs2 = fz_keep_default_colorspaces(ctx, dcs);
        assert_eq!(dcs, dcs2);

        // Should be able to drop twice now
        fz_drop_default_colorspaces(ctx, dcs);
        fz_drop_default_colorspaces(ctx, dcs);
    }

    #[test]
    fn test_cal_gray_colorspace() {
        let ctx = 1;
        let wp = [0.9505f32, 1.0, 1.089];
        let bp = [0.0f32, 0.0, 0.0];

        let cs = fz_new_cal_gray_colorspace(ctx, wp.as_ptr(), bp.as_ptr(), 2.2);
        assert_eq!(cs, FZ_COLORSPACE_GRAY);
    }

    #[test]
    fn test_cal_rgb_colorspace() {
        let ctx = 1;
        let wp = [0.9505f32, 1.0, 1.089];
        let bp = [0.0f32, 0.0, 0.0];
        let gamma = [2.2f32, 2.2, 2.2];
        let matrix = [
            0.4124564f32,
            0.3575761,
            0.1804375,
            0.2126729,
            0.7151522,
            0.0721750,
            0.0193339,
            0.1191920,
            0.9503041,
        ];

        let cs = fz_new_cal_rgb_colorspace(
            ctx,
            wp.as_ptr(),
            bp.as_ptr(),
            gamma.as_ptr(),
            matrix.as_ptr(),
        );
        assert_eq!(cs, FZ_COLORSPACE_RGB);
    }

    #[test]
    fn test_is_valid_blend_colorspace() {
        let ctx = 1;
        assert_eq!(fz_is_valid_blend_colorspace(ctx, FZ_COLORSPACE_GRAY), 1);
        assert_eq!(fz_is_valid_blend_colorspace(ctx, FZ_COLORSPACE_RGB), 1);
        assert_eq!(fz_is_valid_blend_colorspace(ctx, FZ_COLORSPACE_CMYK), 1);
        assert_eq!(fz_is_valid_blend_colorspace(ctx, 99), 0);
    }

    #[test]
    fn test_colorspace_digest() {
        let mut digest = [0xFFu8; 16];
        fz_colorspace_digest(1, FZ_COLORSPACE_RGB, digest.as_mut_ptr());
        // Non-ICC colorspaces should have zero digest
        assert_eq!(digest, [0u8; 16]);
    }

    #[test]
    fn test_max_colors() {
        assert_eq!(fz_max_colors(), 32);
    }

    #[test]
    fn test_convert_color_with_params() {
        let src = [0.5f32];
        let mut dst = [0.0f32; 3];
        let params = fz_default_color_params();

        fz_convert_color_with_params(
            1,
            FZ_COLORSPACE_GRAY,
            src.as_ptr(),
            FZ_COLORSPACE_RGB,
            dst.as_mut_ptr(),
            0,
            params,
        );

        // Gray 0.5 should become RGB (0.5, 0.5, 0.5)
        assert!((dst[0] - 0.5).abs() < 0.01);
        assert!((dst[1] - 0.5).abs() < 0.01);
        assert!((dst[2] - 0.5).abs() < 0.01);
    }
}
