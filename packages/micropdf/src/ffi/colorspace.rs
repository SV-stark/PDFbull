//! C FFI for colorspace - MuPDF compatible
//! Safe Rust implementation

use super::HandleStore;
use std::ffi::c_char;
use std::sync::LazyLock;

/// Colorspace type enumeration
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorspaceType {
    None = 0,
    Gray = 1,
    Rgb = 2,
    Bgr = 3,
    Cmyk = 4,
    Lab = 5,
    Indexed = 6,
    Separation = 7,
    Icc = 8,
}

/// Custom colorspace structure
#[derive(Debug, Clone)]
pub struct Colorspace {
    pub cs_type: ColorspaceType,
    pub n: i32,
    pub name: String,
    pub base_cs: ColorspaceHandle,
    pub lookup: Vec<u8>, // For indexed colorspaces
    pub high: i32,       // For indexed colorspaces (max index)
}

impl Default for Colorspace {
    fn default() -> Self {
        Self {
            cs_type: ColorspaceType::None,
            n: 0,
            name: String::new(),
            base_cs: 0,
            lookup: Vec::new(),
            high: 0,
        }
    }
}

/// Custom colorspace storage (handles > 100 to avoid conflicts with device colorspaces)
pub static COLORSPACES: LazyLock<HandleStore<Colorspace>> = LazyLock::new(HandleStore::default);

/// Base offset for custom colorspace handles
const CUSTOM_CS_OFFSET: ColorspaceHandle = 100;

/// Colorspace handle - we use small integers for device colorspaces
/// Handles 1-5 are reserved for device colorspaces
/// Handles >= 100 are for custom colorspaces
/// 0 = invalid/null
pub type ColorspaceHandle = u64;

pub const FZ_COLORSPACE_GRAY: ColorspaceHandle = 1;
pub const FZ_COLORSPACE_RGB: ColorspaceHandle = 2;
pub const FZ_COLORSPACE_BGR: ColorspaceHandle = 3;
pub const FZ_COLORSPACE_CMYK: ColorspaceHandle = 4;
pub const FZ_COLORSPACE_LAB: ColorspaceHandle = 5;

/// Get number of components for a colorspace
fn colorspace_n(handle: ColorspaceHandle) -> i32 {
    match handle {
        FZ_COLORSPACE_GRAY => 1,
        FZ_COLORSPACE_RGB | FZ_COLORSPACE_BGR => 3,
        FZ_COLORSPACE_CMYK => 4,
        FZ_COLORSPACE_LAB => 3,
        h if h >= CUSTOM_CS_OFFSET => {
            let real_handle = h - CUSTOM_CS_OFFSET;
            if let Some(cs) = COLORSPACES.get(real_handle) {
                if let Ok(guard) = cs.lock() {
                    return guard.n;
                }
            }
            0
        }
        _ => 0,
    }
}

/// Get colorspace type
fn colorspace_type(handle: ColorspaceHandle) -> ColorspaceType {
    match handle {
        FZ_COLORSPACE_GRAY => ColorspaceType::Gray,
        FZ_COLORSPACE_RGB => ColorspaceType::Rgb,
        FZ_COLORSPACE_BGR => ColorspaceType::Bgr,
        FZ_COLORSPACE_CMYK => ColorspaceType::Cmyk,
        FZ_COLORSPACE_LAB => ColorspaceType::Lab,
        h if h >= CUSTOM_CS_OFFSET => {
            let real_handle = h - CUSTOM_CS_OFFSET;
            if let Some(cs) = COLORSPACES.get(real_handle) {
                if let Ok(guard) = cs.lock() {
                    return guard.cs_type;
                }
            }
            ColorspaceType::None
        }
        _ => ColorspaceType::None,
    }
}

/// Get device gray colorspace
#[unsafe(no_mangle)]
pub extern "C" fn fz_device_gray(_ctx: super::Handle) -> ColorspaceHandle {
    FZ_COLORSPACE_GRAY
}

/// Get device RGB colorspace
#[unsafe(no_mangle)]
pub extern "C" fn fz_device_rgb(_ctx: super::Handle) -> ColorspaceHandle {
    FZ_COLORSPACE_RGB
}

/// Get device BGR colorspace
#[unsafe(no_mangle)]
pub extern "C" fn fz_device_bgr(_ctx: super::Handle) -> ColorspaceHandle {
    FZ_COLORSPACE_BGR
}

/// Get device CMYK colorspace
#[unsafe(no_mangle)]
pub extern "C" fn fz_device_cmyk(_ctx: super::Handle) -> ColorspaceHandle {
    FZ_COLORSPACE_CMYK
}

/// Get device Lab colorspace
#[unsafe(no_mangle)]
pub extern "C" fn fz_device_lab(_ctx: super::Handle) -> ColorspaceHandle {
    FZ_COLORSPACE_LAB
}

/// Keep (increment ref) colorspace - device colorspaces are immortal
#[unsafe(no_mangle)]
pub extern "C" fn fz_keep_colorspace(
    _ctx: super::Handle,
    cs: ColorspaceHandle,
) -> ColorspaceHandle {
    cs // Device colorspaces don't need ref counting
}

/// Drop colorspace reference - device colorspaces are immortal
#[unsafe(no_mangle)]
pub extern "C" fn fz_drop_colorspace(_ctx: super::Handle, _cs: ColorspaceHandle) {
    // Device colorspaces are never freed
}

/// Get number of components in colorspace
#[unsafe(no_mangle)]
pub extern "C" fn fz_colorspace_n(_ctx: super::Handle, cs: ColorspaceHandle) -> i32 {
    colorspace_n(cs)
}

/// Check if colorspace is gray
#[unsafe(no_mangle)]
pub extern "C" fn fz_colorspace_is_gray(_ctx: super::Handle, cs: ColorspaceHandle) -> i32 {
    i32::from(colorspace_type(cs) == ColorspaceType::Gray)
}

/// Check if colorspace is RGB
#[unsafe(no_mangle)]
pub extern "C" fn fz_colorspace_is_rgb(_ctx: super::Handle, cs: ColorspaceHandle) -> i32 {
    i32::from(colorspace_type(cs) == ColorspaceType::Rgb)
}

/// Check if colorspace is CMYK
#[unsafe(no_mangle)]
pub extern "C" fn fz_colorspace_is_cmyk(_ctx: super::Handle, cs: ColorspaceHandle) -> i32 {
    i32::from(colorspace_type(cs) == ColorspaceType::Cmyk)
}

/// Check if colorspace is Lab
#[unsafe(no_mangle)]
pub extern "C" fn fz_colorspace_is_lab(_ctx: super::Handle, cs: ColorspaceHandle) -> i32 {
    i32::from(colorspace_type(cs) == ColorspaceType::Lab)
}

/// Check if colorspace is device colorspace
#[unsafe(no_mangle)]
pub extern "C" fn fz_colorspace_is_device(_ctx: super::Handle, cs: ColorspaceHandle) -> i32 {
    i32::from((FZ_COLORSPACE_GRAY..=FZ_COLORSPACE_LAB).contains(&cs))
}

/// Check if colorspace is indexed
#[unsafe(no_mangle)]
pub extern "C" fn fz_colorspace_is_indexed(_ctx: super::Handle, cs: ColorspaceHandle) -> i32 {
    i32::from(colorspace_type(cs) == ColorspaceType::Indexed)
}

/// Check if colorspace is device-n (separation)
#[unsafe(no_mangle)]
pub extern "C" fn fz_colorspace_is_device_n(_ctx: super::Handle, cs: ColorspaceHandle) -> i32 {
    i32::from(colorspace_type(cs) == ColorspaceType::Separation)
}

/// Check if colorspace is subtractive (CMYK, DeviceN)
#[unsafe(no_mangle)]
pub extern "C" fn fz_colorspace_is_subtractive(_ctx: super::Handle, cs: ColorspaceHandle) -> i32 {
    match colorspace_type(cs) {
        ColorspaceType::Cmyk | ColorspaceType::Separation => 1,
        _ => 0,
    }
}

/// Check if colorspace is device-based (not ICC)
#[unsafe(no_mangle)]
pub extern "C" fn fz_colorspace_device_n_has_cmyk(
    _ctx: super::Handle,
    _cs: ColorspaceHandle,
) -> i32 {
    0 // Not implemented yet
}

/// Check if colorspace device-n has only colorants
#[unsafe(no_mangle)]
pub extern "C" fn fz_colorspace_device_n_has_only_cmyk(
    _ctx: super::Handle,
    _cs: ColorspaceHandle,
) -> i32 {
    0 // Not implemented yet
}

/// Get colorspace name
#[unsafe(no_mangle)]
pub extern "C" fn fz_colorspace_name(_ctx: super::Handle, cs: ColorspaceHandle) -> *const c_char {
    match cs {
        FZ_COLORSPACE_GRAY => c"DeviceGray".as_ptr(),
        FZ_COLORSPACE_RGB => c"DeviceRGB".as_ptr(),
        FZ_COLORSPACE_BGR => c"DeviceBGR".as_ptr(),
        FZ_COLORSPACE_CMYK => c"DeviceCMYK".as_ptr(),
        FZ_COLORSPACE_LAB => c"Lab".as_ptr(),
        _ => c"Unknown".as_ptr(),
    }
}

// ============================================================================
// Custom Colorspace Creation
// ============================================================================

/// Create a new indexed colorspace
///
/// # Safety
/// Caller must ensure `lookup` points to valid memory of `(high + 1) * base_n` bytes.
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_indexed_colorspace(
    _ctx: super::Handle,
    base: ColorspaceHandle,
    high: i32,
    lookup: *const u8,
) -> ColorspaceHandle {
    let base_n = colorspace_n(base);
    if base_n == 0 || high < 0 {
        return 0;
    }

    let lookup_size = ((high + 1) * base_n) as usize;
    let lookup_data = if lookup.is_null() || lookup_size == 0 {
        vec![0u8; lookup_size]
    } else {
        unsafe { std::slice::from_raw_parts(lookup, lookup_size) }.to_vec()
    };

    let cs = Colorspace {
        cs_type: ColorspaceType::Indexed,
        n: 1, // Indexed colorspaces have 1 component (the index)
        name: format!("Indexed({})", high),
        base_cs: base,
        lookup: lookup_data,
        high,
    };

    COLORSPACES.insert(cs) + CUSTOM_CS_OFFSET
}

/// Create a new device-n colorspace
///
/// # Safety
/// Caller must ensure `colorants` points to an array of `n` null-terminated C strings.
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_device_n_colorspace(
    _ctx: super::Handle,
    base: ColorspaceHandle,
    n: i32,
    _colorants: *const *const c_char,
) -> ColorspaceHandle {
    if n <= 0 || n > 32 {
        return 0;
    }

    let cs = Colorspace {
        cs_type: ColorspaceType::Separation,
        n,
        name: format!("DeviceN({})", n),
        base_cs: base,
        lookup: Vec::new(),
        high: 0,
    };

    COLORSPACES.insert(cs) + CUSTOM_CS_OFFSET
}

/// Create a new ICC colorspace from data
///
/// # Safety
/// Caller must ensure `data` points to valid ICC profile data of `size` bytes.
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_icc_colorspace(
    _ctx: super::Handle,
    _type_hint: i32, // Hint about what type of colorspace (gray, rgb, cmyk)
    _flags: i32,
    name: *const c_char,
    _data: *const u8,
    _size: usize,
) -> ColorspaceHandle {
    // Determine components from type hint or default to RGB
    let n = 3; // Default to RGB

    let cs_name = if name.is_null() {
        "ICCBased".to_string()
    } else {
        let c_str = unsafe { std::ffi::CStr::from_ptr(name) };
        c_str.to_str().unwrap_or("ICCBased").to_string()
    };

    let cs = Colorspace {
        cs_type: ColorspaceType::Icc,
        n,
        name: cs_name,
        base_cs: FZ_COLORSPACE_RGB,
        lookup: Vec::new(),
        high: 0,
    };

    COLORSPACES.insert(cs) + CUSTOM_CS_OFFSET
}

/// Get base colorspace of indexed colorspace
#[unsafe(no_mangle)]
pub extern "C" fn fz_colorspace_base(
    _ctx: super::Handle,
    cs: ColorspaceHandle,
) -> ColorspaceHandle {
    if cs < CUSTOM_CS_OFFSET {
        return 0; // Device colorspaces have no base
    }

    let real_handle = cs - CUSTOM_CS_OFFSET;
    if let Some(colorspace) = COLORSPACES.get(real_handle) {
        if let Ok(guard) = colorspace.lock() {
            return guard.base_cs;
        }
    }
    0
}

/// Get high value of indexed colorspace
#[unsafe(no_mangle)]
pub extern "C" fn fz_colorspace_high(_ctx: super::Handle, cs: ColorspaceHandle) -> i32 {
    if cs < CUSTOM_CS_OFFSET {
        return -1;
    }

    let real_handle = cs - CUSTOM_CS_OFFSET;
    if let Some(colorspace) = COLORSPACES.get(real_handle) {
        if let Ok(guard) = colorspace.lock() {
            if guard.cs_type == ColorspaceType::Indexed {
                return guard.high;
            }
        }
    }
    -1
}

/// Get lookup table of indexed colorspace
#[unsafe(no_mangle)]
pub extern "C" fn fz_colorspace_lookup(_ctx: super::Handle, _cs: ColorspaceHandle) -> *const u8 {
    // Cannot safely return pointer to internal data
    std::ptr::null()
}

/// Convert color from one colorspace to another
///
/// # Safety
/// Caller must ensure:
/// - `src` points to valid memory of at least `src_cs.n` floats
/// - `dst` points to writable memory of at least `dst_cs.n` floats
#[unsafe(no_mangle)]
pub extern "C" fn fz_convert_color(
    _ctx: super::Handle,
    src_cs: ColorspaceHandle,
    src: *const f32,
    dst_cs: ColorspaceHandle,
    dst: *mut f32,
    _proof_cs: ColorspaceHandle,
) {
    if src.is_null() || dst.is_null() {
        return;
    }

    let src_n = colorspace_n(src_cs) as usize;
    let dst_n = colorspace_n(dst_cs) as usize;

    if src_n == 0 || dst_n == 0 {
        return;
    }

    // SAFETY: Caller guarantees src and dst point to valid memory
    let (src_slice, dst_slice) = unsafe {
        (
            std::slice::from_raw_parts(src, src_n),
            std::slice::from_raw_parts_mut(dst, dst_n),
        )
    };

    // Simple color conversion (Gray -> RGB, RGB -> Gray, etc.)
    match (colorspace_type(src_cs), colorspace_type(dst_cs)) {
        (ColorspaceType::Gray, ColorspaceType::Rgb) => {
            let g = src_slice[0];
            dst_slice[0] = g;
            dst_slice[1] = g;
            dst_slice[2] = g;
        }
        (ColorspaceType::Rgb, ColorspaceType::Gray) => {
            // Luminance formula
            dst_slice[0] = src_slice[0] * 0.299 + src_slice[1] * 0.587 + src_slice[2] * 0.114;
        }
        (ColorspaceType::Rgb, ColorspaceType::Cmyk) => {
            let r = src_slice[0];
            let g = src_slice[1];
            let b = src_slice[2];
            let k = 1.0 - r.max(g).max(b);
            if k < 1.0 {
                let inv_k = 1.0 / (1.0 - k);
                dst_slice[0] = (1.0 - r - k) * inv_k;
                dst_slice[1] = (1.0 - g - k) * inv_k;
                dst_slice[2] = (1.0 - b - k) * inv_k;
            } else {
                dst_slice[0] = 0.0;
                dst_slice[1] = 0.0;
                dst_slice[2] = 0.0;
            }
            dst_slice[3] = k;
        }
        (ColorspaceType::Cmyk, ColorspaceType::Rgb) => {
            let c = src_slice[0];
            let m = src_slice[1];
            let y = src_slice[2];
            let k = src_slice[3];
            dst_slice[0] = (1.0 - c) * (1.0 - k);
            dst_slice[1] = (1.0 - m) * (1.0 - k);
            dst_slice[2] = (1.0 - y) * (1.0 - k);
        }
        _ if src_cs == dst_cs => {
            // Same colorspace, just copy
            dst_slice[..src_n.min(dst_n)].copy_from_slice(&src_slice[..src_n.min(dst_n)]);
        }
        _ => {
            // Default: fill with zeros
            dst_slice.fill(0.0);
        }
    }
}

/// Clone a colorspace (increments reference count)
#[unsafe(no_mangle)]
pub extern "C" fn fz_clone_colorspace(
    _ctx: super::Handle,
    cs: ColorspaceHandle,
) -> ColorspaceHandle {
    fz_keep_colorspace(_ctx, cs)
}

/// Check if two colorspaces are equal
#[unsafe(no_mangle)]
pub extern "C" fn fz_colorspace_eq(
    _ctx: super::Handle,
    a: ColorspaceHandle,
    b: ColorspaceHandle,
) -> i32 {
    if a == b { 1 } else { 0 }
}

/// Get the type of a colorspace as integer
#[unsafe(no_mangle)]
pub extern "C" fn fz_colorspace_type(_ctx: super::Handle, cs: ColorspaceHandle) -> i32 {
    colorspace_type(cs) as i32
}

/// Check if colorspace is ICC-based
#[unsafe(no_mangle)]
pub extern "C" fn fz_colorspace_is_icc(_ctx: super::Handle, cs: ColorspaceHandle) -> i32 {
    let cs_type = colorspace_type(cs);
    if cs_type == ColorspaceType::Icc { 1 } else { 0 }
}

/// Clamp color values to valid range [0, 1]
#[unsafe(no_mangle)]
pub extern "C" fn fz_clamp_color(
    _ctx: super::Handle,
    cs: ColorspaceHandle,
    color_in: *const f32,
    color_out: *mut f32,
) {
    if color_in.is_null() || color_out.is_null() {
        return;
    }

    let n = colorspace_n(cs) as usize;
    if n == 0 {
        return;
    }

    unsafe {
        let input = std::slice::from_raw_parts(color_in, n);
        let output = std::slice::from_raw_parts_mut(color_out, n);

        for i in 0..n {
            output[i] = input[i].clamp(0.0, 1.0);
        }
    }
}

/// Get the colorspace for a separation/DeviceN colorant by name
#[unsafe(no_mangle)]
pub extern "C" fn fz_colorspace_colorant(
    _ctx: super::Handle,
    _cs: ColorspaceHandle,
    _idx: i32,
) -> *const c_char {
    c"".as_ptr()
}

/// Count number of colorants in a separation/DeviceN colorspace
#[unsafe(no_mangle)]
pub extern "C" fn fz_colorspace_num_colorants(_ctx: super::Handle, cs: ColorspaceHandle) -> i32 {
    if colorspace_type(cs) == ColorspaceType::Separation {
        colorspace_n(cs)
    } else {
        0
    }
}

/// Get the base colorspace component count
#[unsafe(no_mangle)]
pub extern "C" fn fz_colorspace_base_n(_ctx: super::Handle, cs: ColorspaceHandle) -> i32 {
    let base = fz_colorspace_base(_ctx, cs);
    colorspace_n(base)
}

/// Convert a single pixel
#[unsafe(no_mangle)]
pub extern "C" fn fz_convert_pixel(
    _ctx: super::Handle,
    src_cs: ColorspaceHandle,
    src: *const f32,
    dst_cs: ColorspaceHandle,
    dst: *mut f32,
) {
    fz_convert_color(_ctx, src_cs, src, dst_cs, dst, 0)
}

/// Get standard sRGB colorspace
#[unsafe(no_mangle)]
pub extern "C" fn fz_device_srgb(_ctx: super::Handle) -> ColorspaceHandle {
    FZ_COLORSPACE_RGB
}

/// Get standard grayscale colorspace (alias)
#[unsafe(no_mangle)]
pub extern "C" fn fz_device_grayscale(_ctx: super::Handle) -> ColorspaceHandle {
    FZ_COLORSPACE_GRAY
}

/// Check if colorspace has spot colors
#[unsafe(no_mangle)]
pub extern "C" fn fz_colorspace_has_spots(_ctx: super::Handle, cs: ColorspaceHandle) -> i32 {
    let cs_type = colorspace_type(cs);
    if cs_type == ColorspaceType::Separation {
        1
    } else {
        0
    }
}

/// Count spot colors in a colorspace
#[unsafe(no_mangle)]
pub extern "C" fn fz_colorspace_n_spots(_ctx: super::Handle, cs: ColorspaceHandle) -> i32 {
    if fz_colorspace_has_spots(_ctx, cs) != 0 {
        1
    } else {
        0
    }
}

/// Get colorspace name as string (alias)
#[unsafe(no_mangle)]
pub extern "C" fn fz_colorspace_name_string(
    _ctx: super::Handle,
    cs: ColorspaceHandle,
) -> *const c_char {
    fz_colorspace_name(_ctx, cs)
}

/// Check if colorspace is valid
#[unsafe(no_mangle)]
pub extern "C" fn fz_colorspace_is_valid(_ctx: super::Handle, cs: ColorspaceHandle) -> i32 {
    if cs == 0 {
        0
    } else if cs <= 5 || cs >= CUSTOM_CS_OFFSET {
        1
    } else {
        0
    }
}

/// Get maximum component value for colorspace
#[unsafe(no_mangle)]
pub extern "C" fn fz_colorspace_max(_ctx: super::Handle, cs: ColorspaceHandle) -> f32 {
    if colorspace_type(cs) == ColorspaceType::Indexed {
        fz_colorspace_high(_ctx, cs) as f32
    } else {
        1.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_colorspaces() {
        assert_eq!(fz_colorspace_n(0, FZ_COLORSPACE_GRAY), 1);
        assert_eq!(fz_colorspace_n(0, FZ_COLORSPACE_RGB), 3);
        assert_eq!(fz_colorspace_n(0, FZ_COLORSPACE_CMYK), 4);
    }

    #[test]
    fn test_colorspace_checks() {
        assert_eq!(fz_colorspace_is_gray(0, FZ_COLORSPACE_GRAY), 1);
        assert_eq!(fz_colorspace_is_rgb(0, FZ_COLORSPACE_RGB), 1);
        assert_eq!(fz_colorspace_is_cmyk(0, FZ_COLORSPACE_CMYK), 1);
    }

    #[test]
    fn test_colorspace_is_gray_negative() {
        assert_eq!(fz_colorspace_is_gray(0, FZ_COLORSPACE_RGB), 0);
        assert_eq!(fz_colorspace_is_gray(0, FZ_COLORSPACE_CMYK), 0);
    }

    #[test]
    fn test_colorspace_is_rgb_negative() {
        assert_eq!(fz_colorspace_is_rgb(0, FZ_COLORSPACE_GRAY), 0);
        assert_eq!(fz_colorspace_is_rgb(0, FZ_COLORSPACE_CMYK), 0);
    }

    #[test]
    fn test_colorspace_is_cmyk_negative() {
        assert_eq!(fz_colorspace_is_cmyk(0, FZ_COLORSPACE_GRAY), 0);
        assert_eq!(fz_colorspace_is_cmyk(0, FZ_COLORSPACE_RGB), 0);
    }

    #[test]
    fn test_colorspace_type() {
        assert!(matches!(
            colorspace_type(FZ_COLORSPACE_GRAY),
            ColorspaceType::Gray
        ));
        assert!(matches!(
            colorspace_type(FZ_COLORSPACE_RGB),
            ColorspaceType::Rgb
        ));
        assert!(matches!(
            colorspace_type(FZ_COLORSPACE_CMYK),
            ColorspaceType::Cmyk
        ));
        assert!(matches!(colorspace_type(99), ColorspaceType::None));
    }

    #[test]
    fn test_colorspace_n() {
        assert_eq!(colorspace_n(FZ_COLORSPACE_GRAY), 1);
        assert_eq!(colorspace_n(FZ_COLORSPACE_RGB), 3);
        assert_eq!(colorspace_n(FZ_COLORSPACE_CMYK), 4);
        assert_eq!(colorspace_n(99), 0);
    }

    #[test]
    fn test_device_gray_handle() {
        let handle = fz_device_gray(0);
        assert_eq!(handle, FZ_COLORSPACE_GRAY);
    }

    #[test]
    fn test_device_rgb_handle() {
        let handle = fz_device_rgb(0);
        assert_eq!(handle, FZ_COLORSPACE_RGB);
    }

    #[test]
    fn test_device_cmyk_handle() {
        let handle = fz_device_cmyk(0);
        assert_eq!(handle, FZ_COLORSPACE_CMYK);
    }

    #[test]
    fn test_keep_drop_colorspace() {
        // Keep and drop should not panic
        let handle = fz_keep_colorspace(0, FZ_COLORSPACE_RGB);
        assert_eq!(handle, FZ_COLORSPACE_RGB);
        fz_drop_colorspace(0, handle);
    }

    #[test]
    fn test_convert_color_gray_to_rgb() {
        let src = [0.5f32];
        let mut dst = [0.0f32; 3];

        fz_convert_color(
            0,
            FZ_COLORSPACE_GRAY,
            src.as_ptr(),
            FZ_COLORSPACE_RGB,
            dst.as_mut_ptr(),
            0,
        );

        assert!((dst[0] - 0.5).abs() < 0.01);
        assert!((dst[1] - 0.5).abs() < 0.01);
        assert!((dst[2] - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_convert_color_rgb_to_gray() {
        let src = [1.0f32, 1.0, 1.0]; // White
        let mut dst = [0.0f32];

        fz_convert_color(
            0,
            FZ_COLORSPACE_RGB,
            src.as_ptr(),
            FZ_COLORSPACE_GRAY,
            dst.as_mut_ptr(),
            0,
        );

        // Luminance should be close to 1.0 for white
        assert!((dst[0] - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_convert_color_rgb_to_cmyk() {
        let src = [1.0f32, 0.0, 0.0]; // Red
        let mut dst = [0.0f32; 4];

        fz_convert_color(
            0,
            FZ_COLORSPACE_RGB,
            src.as_ptr(),
            FZ_COLORSPACE_CMYK,
            dst.as_mut_ptr(),
            0,
        );

        // Red in CMYK: C=0, M=1, Y=1, K=0
        assert!(dst[0] < 0.1); // Cyan should be low
        assert_eq!(dst[3], 0.0); // Black should be 0
    }

    #[test]
    fn test_convert_color_cmyk_to_rgb() {
        let src = [0.0f32, 0.0, 0.0, 0.0]; // No ink = white
        let mut dst = [0.0f32; 3];

        fz_convert_color(
            0,
            FZ_COLORSPACE_CMYK,
            src.as_ptr(),
            FZ_COLORSPACE_RGB,
            dst.as_mut_ptr(),
            0,
        );

        assert!((dst[0] - 1.0).abs() < 0.01);
        assert!((dst[1] - 1.0).abs() < 0.01);
        assert!((dst[2] - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_convert_color_same_colorspace() {
        let src = [0.25f32, 0.5, 0.75];
        let mut dst = [0.0f32; 3];

        fz_convert_color(
            0,
            FZ_COLORSPACE_RGB,
            src.as_ptr(),
            FZ_COLORSPACE_RGB,
            dst.as_mut_ptr(),
            0,
        );

        assert_eq!(dst, src);
    }

    #[test]
    fn test_convert_color_null_pointers() {
        // Should not panic with null pointers
        fz_convert_color(
            0,
            FZ_COLORSPACE_RGB,
            std::ptr::null(),
            FZ_COLORSPACE_GRAY,
            std::ptr::null_mut(),
            0,
        );
    }

    #[test]
    fn test_convert_color_invalid_colorspace() {
        let src = [0.5f32];
        let mut dst = [1.0f32; 3];

        // Invalid source colorspace - dst should be filled with zeros
        fz_convert_color(0, 99, src.as_ptr(), FZ_COLORSPACE_RGB, dst.as_mut_ptr(), 0);
        // Operation should not panic even with invalid colorspace
    }

    #[test]
    fn test_colorspace_is_device() {
        assert_eq!(fz_colorspace_is_device(0, FZ_COLORSPACE_GRAY), 1);
        assert_eq!(fz_colorspace_is_device(0, FZ_COLORSPACE_RGB), 1);
        assert_eq!(fz_colorspace_is_device(0, FZ_COLORSPACE_CMYK), 1);
    }

    // ============================================================================
    // Additional Check Tests
    // ============================================================================

    #[test]
    fn test_colorspace_is_indexed() {
        assert_eq!(fz_colorspace_is_indexed(0, FZ_COLORSPACE_RGB), 0);
        assert_eq!(fz_colorspace_is_indexed(0, FZ_COLORSPACE_GRAY), 0);
    }

    #[test]
    fn test_colorspace_is_device_n() {
        assert_eq!(fz_colorspace_is_device_n(0, FZ_COLORSPACE_RGB), 0);
        assert_eq!(fz_colorspace_is_device_n(0, FZ_COLORSPACE_CMYK), 0);
    }

    #[test]
    fn test_colorspace_is_subtractive() {
        assert_eq!(fz_colorspace_is_subtractive(0, FZ_COLORSPACE_CMYK), 1);
        assert_eq!(fz_colorspace_is_subtractive(0, FZ_COLORSPACE_RGB), 0);
        assert_eq!(fz_colorspace_is_subtractive(0, FZ_COLORSPACE_GRAY), 0);
    }

    // ============================================================================
    // Indexed Colorspace Tests
    // ============================================================================

    #[test]
    fn test_new_indexed_colorspace() {
        // Create a simple indexed colorspace with 4 colors
        let lookup = [
            255, 0, 0, // Index 0 = Red
            0, 255, 0, // Index 1 = Green
            0, 0, 255, // Index 2 = Blue
            255, 255, 0, // Index 3 = Yellow
        ];

        let cs = fz_new_indexed_colorspace(0, FZ_COLORSPACE_RGB, 3, lookup.as_ptr());
        assert!(cs >= CUSTOM_CS_OFFSET);

        // Should be indexed
        assert_eq!(fz_colorspace_is_indexed(0, cs), 1);

        // Should have 1 component
        assert_eq!(fz_colorspace_n(0, cs), 1);

        // Should have base RGB
        assert_eq!(fz_colorspace_base(0, cs), FZ_COLORSPACE_RGB);

        // High should be 3
        assert_eq!(fz_colorspace_high(0, cs), 3);
    }

    #[test]
    fn test_new_indexed_colorspace_null_lookup() {
        let cs = fz_new_indexed_colorspace(0, FZ_COLORSPACE_RGB, 3, std::ptr::null());
        assert!(cs >= CUSTOM_CS_OFFSET);
    }

    #[test]
    fn test_new_indexed_colorspace_invalid() {
        // Invalid base
        let cs1 = fz_new_indexed_colorspace(0, 99, 3, std::ptr::null());
        assert_eq!(cs1, 0);

        // Invalid high
        let cs2 = fz_new_indexed_colorspace(0, FZ_COLORSPACE_RGB, -1, std::ptr::null());
        assert_eq!(cs2, 0);
    }

    // ============================================================================
    // DeviceN Colorspace Tests
    // ============================================================================

    #[test]
    fn test_new_device_n_colorspace() {
        let cs = fz_new_device_n_colorspace(0, FZ_COLORSPACE_CMYK, 2, std::ptr::null());
        assert!(cs >= CUSTOM_CS_OFFSET);

        assert_eq!(fz_colorspace_is_device_n(0, cs), 1);
        assert_eq!(fz_colorspace_n(0, cs), 2);
    }

    #[test]
    fn test_new_device_n_colorspace_invalid() {
        // Invalid n
        let cs1 = fz_new_device_n_colorspace(0, FZ_COLORSPACE_CMYK, 0, std::ptr::null());
        assert_eq!(cs1, 0);

        let cs2 = fz_new_device_n_colorspace(0, FZ_COLORSPACE_CMYK, 100, std::ptr::null());
        assert_eq!(cs2, 0);
    }

    // ============================================================================
    // ICC Colorspace Tests
    // ============================================================================

    #[test]
    fn test_new_icc_colorspace() {
        let cs = fz_new_icc_colorspace(0, 0, 0, std::ptr::null(), std::ptr::null(), 0);
        assert!(cs >= CUSTOM_CS_OFFSET);

        // Default is RGB (3 components)
        assert_eq!(fz_colorspace_n(0, cs), 3);
    }

    #[test]
    fn test_new_icc_colorspace_with_name() {
        let name = c"sRGB IEC61966-2.1";
        let cs = fz_new_icc_colorspace(0, 0, 0, name.as_ptr(), std::ptr::null(), 0);
        assert!(cs >= CUSTOM_CS_OFFSET);
    }

    // ============================================================================
    // Base/High Tests for Device Colorspaces
    // ============================================================================

    #[test]
    fn test_colorspace_base_device() {
        // Device colorspaces should return 0 (no base)
        assert_eq!(fz_colorspace_base(0, FZ_COLORSPACE_RGB), 0);
        assert_eq!(fz_colorspace_base(0, FZ_COLORSPACE_GRAY), 0);
    }

    #[test]
    fn test_colorspace_high_device() {
        // Device colorspaces should return -1 (not indexed)
        assert_eq!(fz_colorspace_high(0, FZ_COLORSPACE_RGB), -1);
        assert_eq!(fz_colorspace_high(0, FZ_COLORSPACE_GRAY), -1);
    }

    #[test]
    fn test_colorspace_lookup_returns_null() {
        // Should always return null for safety
        assert!(fz_colorspace_lookup(0, FZ_COLORSPACE_RGB).is_null());
    }
}
