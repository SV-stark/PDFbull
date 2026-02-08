//! C FFI for draw device (rendering) - MuPDF compatible
//! Safe Rust implementation of fz_draw_device

use super::{Handle, HandleStore};
use std::sync::LazyLock;

/// Blend mode enumeration (PDF/SVG standard)
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlendMode {
    /// Normal/source-over compositing
    Normal = 0,
    /// Multiply blend
    Multiply = 1,
    /// Screen blend
    Screen = 2,
    /// Overlay blend
    Overlay = 3,
    /// Darken blend
    Darken = 4,
    /// Lighten blend
    Lighten = 5,
    /// Color dodge blend
    ColorDodge = 6,
    /// Color burn blend
    ColorBurn = 7,
    /// Hard light blend
    HardLight = 8,
    /// Soft light blend
    SoftLight = 9,
    /// Difference blend
    Difference = 10,
    /// Exclusion blend
    Exclusion = 11,
    /// Hue blend (HSL)
    Hue = 12,
    /// Saturation blend (HSL)
    Saturation = 13,
    /// Color blend (HSL)
    Color = 14,
    /// Luminosity blend (HSL)
    Luminosity = 15,
}

/// Anti-aliasing level
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AntiAliasLevel {
    /// No anti-aliasing
    None = 0,
    /// 2x2 supersampling
    Low = 1,
    /// 4x4 supersampling
    Medium = 2,
    /// 8x8 supersampling
    High = 3,
}

/// Clip rule
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipRule {
    /// Non-zero winding rule
    NonZero = 0,
    /// Even-odd fill rule
    EvenOdd = 1,
}

/// Overprint mode
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverprintMode {
    /// No overprint simulation
    Off = 0,
    /// Overprint for stroke operations
    Stroke = 1,
    /// Overprint for fill operations
    Fill = 2,
    /// Overprint for both stroke and fill
    Both = 3,
}

/// Graphics state for draw device
#[derive(Debug, Clone)]
pub struct GraphicsState {
    /// Current transformation matrix [a, b, c, d, e, f]
    pub ctm: [f32; 6],
    /// Stroke color (RGBA)
    pub stroke_color: [f32; 4],
    /// Fill color (RGBA)
    pub fill_color: [f32; 4],
    /// Line width
    pub line_width: f32,
    /// Miter limit
    pub miter_limit: f32,
    /// Line cap style (0=butt, 1=round, 2=square)
    pub line_cap: i32,
    /// Line join style (0=miter, 1=round, 2=bevel)
    pub line_join: i32,
    /// Dash array
    pub dash_array: Vec<f32>,
    /// Dash phase
    pub dash_phase: f32,
    /// Current blend mode
    pub blend_mode: BlendMode,
    /// Global alpha (opacity)
    pub alpha: f32,
    /// Stroke alpha
    pub stroke_alpha: f32,
    /// Fill alpha
    pub fill_alpha: f32,
    /// Overprint mode
    pub overprint: OverprintMode,
}

impl Default for GraphicsState {
    fn default() -> Self {
        Self {
            ctm: [1.0, 0.0, 0.0, 1.0, 0.0, 0.0], // Identity
            stroke_color: [0.0, 0.0, 0.0, 1.0],  // Black
            fill_color: [0.0, 0.0, 0.0, 1.0],    // Black
            line_width: 1.0,
            miter_limit: 10.0,
            line_cap: 0,
            line_join: 0,
            dash_array: Vec::new(),
            dash_phase: 0.0,
            blend_mode: BlendMode::Normal,
            alpha: 1.0,
            stroke_alpha: 1.0,
            fill_alpha: 1.0,
            overprint: OverprintMode::Off,
        }
    }
}

/// Clip region entry
#[derive(Debug, Clone)]
pub struct ClipRegion {
    /// Path data for clip (simplified as points)
    pub path: Vec<(f32, f32)>,
    /// Clip rule
    pub rule: ClipRule,
    /// Bounding box [x0, y0, x1, y1]
    pub bbox: [f32; 4],
}

/// Draw device for rendering operations
#[derive(Debug)]
pub struct DrawDevice {
    /// Target pixmap handle
    pub target: Handle,
    /// Width in pixels
    pub width: i32,
    /// Height in pixels
    pub height: i32,
    /// Anti-aliasing level
    pub aa_level: AntiAliasLevel,
    /// Graphics state stack
    pub state_stack: Vec<GraphicsState>,
    /// Current graphics state
    pub current_state: GraphicsState,
    /// Clip stack
    pub clip_stack: Vec<ClipRegion>,
    /// Whether device is in a text object
    pub in_text: bool,
    /// Accumulated path for current stroke/fill
    pub current_path: Vec<PathOp>,
    /// Rendering hints
    pub hints: RenderHints,
}

/// Path operation for accumulating paths
#[derive(Debug, Clone)]
pub enum PathOp {
    MoveTo(f32, f32),
    LineTo(f32, f32),
    CurveTo(f32, f32, f32, f32, f32, f32),
    ClosePath,
}

/// Rendering hints
#[derive(Debug, Clone, Default)]
pub struct RenderHints {
    /// Use subpixel text rendering
    pub subpixel_text: bool,
    /// Text gamma correction
    pub text_gamma: f32,
    /// Use color management
    pub color_management: bool,
    /// Simulate overprint
    pub overprint_simulation: bool,
    /// Use spot color rendering
    pub spot_colors: bool,
}

impl Default for DrawDevice {
    fn default() -> Self {
        Self {
            target: 0,
            width: 0,
            height: 0,
            aa_level: AntiAliasLevel::Medium,
            state_stack: Vec::new(),
            current_state: GraphicsState::default(),
            clip_stack: Vec::new(),
            in_text: false,
            current_path: Vec::new(),
            hints: RenderHints::default(),
        }
    }
}

/// Global draw device storage
pub static DRAW_DEVICES: LazyLock<HandleStore<DrawDevice>> = LazyLock::new(HandleStore::new);

// ============================================================================
// Device Creation
// ============================================================================

/// Create a new advanced draw device targeting a pixmap with explicit dimensions
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_draw_device_with_size(
    _ctx: Handle,
    target_pixmap: Handle,
    width: i32,
    height: i32,
) -> Handle {
    let device = DrawDevice {
        target: target_pixmap,
        width,
        height,
        ..Default::default()
    };
    DRAW_DEVICES.insert(device)
}

/// Create draw device with transformation matrix
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_draw_device_with_matrix(
    _ctx: Handle,
    target_pixmap: Handle,
    matrix: *const f32,
) -> Handle {
    let mut device = DrawDevice {
        target: target_pixmap,
        ..Default::default()
    };

    if !matrix.is_null() {
        let m = unsafe { std::slice::from_raw_parts(matrix, 6) };
        device.current_state.ctm.copy_from_slice(m);
    }

    DRAW_DEVICES.insert(device)
}

/// Create draw device with specific options
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_draw_device_with_options(
    _ctx: Handle,
    target_pixmap: Handle,
    aa_level: i32,
    subpixel_text: i32,
) -> Handle {
    let aa = match aa_level {
        0 => AntiAliasLevel::None,
        1 => AntiAliasLevel::Low,
        3 => AntiAliasLevel::High,
        _ => AntiAliasLevel::Medium,
    };

    let device = DrawDevice {
        target: target_pixmap,
        aa_level: aa,
        hints: RenderHints {
            subpixel_text: subpixel_text != 0,
            ..Default::default()
        },
        ..Default::default()
    };
    DRAW_DEVICES.insert(device)
}

// ============================================================================
// Graphics State
// ============================================================================

/// Save graphics state (push)
#[unsafe(no_mangle)]
pub extern "C" fn fz_draw_device_save(_ctx: Handle, device: Handle) {
    if let Some(dev) = DRAW_DEVICES.get(device) {
        if let Ok(mut guard) = dev.lock() {
            let state_clone = guard.current_state.clone();
            guard.state_stack.push(state_clone);
        }
    }
}

/// Restore graphics state (pop)
#[unsafe(no_mangle)]
pub extern "C" fn fz_draw_device_restore(_ctx: Handle, device: Handle) {
    if let Some(dev) = DRAW_DEVICES.get(device) {
        if let Ok(mut guard) = dev.lock() {
            if let Some(state) = guard.state_stack.pop() {
                guard.current_state = state;
            }
        }
    }
}

/// Set transformation matrix
#[unsafe(no_mangle)]
pub extern "C" fn fz_draw_device_set_ctm(_ctx: Handle, device: Handle, matrix: *const f32) {
    if matrix.is_null() {
        return;
    }

    if let Some(dev) = DRAW_DEVICES.get(device) {
        if let Ok(mut guard) = dev.lock() {
            let m = unsafe { std::slice::from_raw_parts(matrix, 6) };
            guard.current_state.ctm.copy_from_slice(m);
        }
    }
}

/// Concatenate transformation matrix
#[unsafe(no_mangle)]
pub extern "C" fn fz_draw_device_concat_ctm(_ctx: Handle, device: Handle, matrix: *const f32) {
    if matrix.is_null() {
        return;
    }

    if let Some(dev) = DRAW_DEVICES.get(device) {
        if let Ok(mut guard) = dev.lock() {
            let m = unsafe { std::slice::from_raw_parts(matrix, 6) };
            let ctm = &guard.current_state.ctm;

            // Matrix multiplication: result = ctm * m
            let new_ctm = [
                ctm[0] * m[0] + ctm[1] * m[2],
                ctm[0] * m[1] + ctm[1] * m[3],
                ctm[2] * m[0] + ctm[3] * m[2],
                ctm[2] * m[1] + ctm[3] * m[3],
                ctm[4] * m[0] + ctm[5] * m[2] + m[4],
                ctm[4] * m[1] + ctm[5] * m[3] + m[5],
            ];
            guard.current_state.ctm = new_ctm;
        }
    }
}

/// Set stroke color
#[unsafe(no_mangle)]
pub extern "C" fn fz_draw_device_set_stroke_color(
    _ctx: Handle,
    device: Handle,
    r: f32,
    g: f32,
    b: f32,
    a: f32,
) {
    if let Some(dev) = DRAW_DEVICES.get(device) {
        if let Ok(mut guard) = dev.lock() {
            guard.current_state.stroke_color = [
                r.clamp(0.0, 1.0),
                g.clamp(0.0, 1.0),
                b.clamp(0.0, 1.0),
                a.clamp(0.0, 1.0),
            ];
        }
    }
}

/// Set fill color
#[unsafe(no_mangle)]
pub extern "C" fn fz_draw_device_set_fill_color(
    _ctx: Handle,
    device: Handle,
    r: f32,
    g: f32,
    b: f32,
    a: f32,
) {
    if let Some(dev) = DRAW_DEVICES.get(device) {
        if let Ok(mut guard) = dev.lock() {
            guard.current_state.fill_color = [
                r.clamp(0.0, 1.0),
                g.clamp(0.0, 1.0),
                b.clamp(0.0, 1.0),
                a.clamp(0.0, 1.0),
            ];
        }
    }
}

/// Set line width
#[unsafe(no_mangle)]
pub extern "C" fn fz_draw_device_set_line_width(_ctx: Handle, device: Handle, width: f32) {
    if let Some(dev) = DRAW_DEVICES.get(device) {
        if let Ok(mut guard) = dev.lock() {
            guard.current_state.line_width = width.max(0.0);
        }
    }
}

/// Set line cap style
#[unsafe(no_mangle)]
pub extern "C" fn fz_draw_device_set_line_cap(_ctx: Handle, device: Handle, cap: i32) {
    if let Some(dev) = DRAW_DEVICES.get(device) {
        if let Ok(mut guard) = dev.lock() {
            guard.current_state.line_cap = cap.clamp(0, 2);
        }
    }
}

/// Set line join style
#[unsafe(no_mangle)]
pub extern "C" fn fz_draw_device_set_line_join(_ctx: Handle, device: Handle, join: i32) {
    if let Some(dev) = DRAW_DEVICES.get(device) {
        if let Ok(mut guard) = dev.lock() {
            guard.current_state.line_join = join.clamp(0, 2);
        }
    }
}

/// Set miter limit
#[unsafe(no_mangle)]
pub extern "C" fn fz_draw_device_set_miter_limit(_ctx: Handle, device: Handle, limit: f32) {
    if let Some(dev) = DRAW_DEVICES.get(device) {
        if let Ok(mut guard) = dev.lock() {
            guard.current_state.miter_limit = limit.max(1.0);
        }
    }
}

/// Set dash pattern
#[unsafe(no_mangle)]
pub extern "C" fn fz_draw_device_set_dash(
    _ctx: Handle,
    device: Handle,
    dash_array: *const f32,
    dash_count: i32,
    dash_phase: f32,
) {
    if let Some(dev) = DRAW_DEVICES.get(device) {
        if let Ok(mut guard) = dev.lock() {
            if dash_array.is_null() || dash_count <= 0 {
                guard.current_state.dash_array.clear();
            } else {
                let arr = unsafe { std::slice::from_raw_parts(dash_array, dash_count as usize) };
                guard.current_state.dash_array = arr.to_vec();
            }
            guard.current_state.dash_phase = dash_phase;
        }
    }
}

/// Set blend mode
#[unsafe(no_mangle)]
pub extern "C" fn fz_draw_device_set_blend_mode(_ctx: Handle, device: Handle, mode: i32) {
    let blend = match mode {
        1 => BlendMode::Multiply,
        2 => BlendMode::Screen,
        3 => BlendMode::Overlay,
        4 => BlendMode::Darken,
        5 => BlendMode::Lighten,
        6 => BlendMode::ColorDodge,
        7 => BlendMode::ColorBurn,
        8 => BlendMode::HardLight,
        9 => BlendMode::SoftLight,
        10 => BlendMode::Difference,
        11 => BlendMode::Exclusion,
        12 => BlendMode::Hue,
        13 => BlendMode::Saturation,
        14 => BlendMode::Color,
        15 => BlendMode::Luminosity,
        _ => BlendMode::Normal,
    };

    if let Some(dev) = DRAW_DEVICES.get(device) {
        if let Ok(mut guard) = dev.lock() {
            guard.current_state.blend_mode = blend;
        }
    }
}

/// Set global alpha
#[unsafe(no_mangle)]
pub extern "C" fn fz_draw_device_set_alpha(_ctx: Handle, device: Handle, alpha: f32) {
    if let Some(dev) = DRAW_DEVICES.get(device) {
        if let Ok(mut guard) = dev.lock() {
            guard.current_state.alpha = alpha.clamp(0.0, 1.0);
        }
    }
}

/// Set overprint mode
#[unsafe(no_mangle)]
pub extern "C" fn fz_draw_device_set_overprint(_ctx: Handle, device: Handle, mode: i32) {
    let op = match mode {
        1 => OverprintMode::Stroke,
        2 => OverprintMode::Fill,
        3 => OverprintMode::Both,
        _ => OverprintMode::Off,
    };

    if let Some(dev) = DRAW_DEVICES.get(device) {
        if let Ok(mut guard) = dev.lock() {
            guard.current_state.overprint = op;
        }
    }
}

// ============================================================================
// Path Operations
// ============================================================================

/// Begin a new path
#[unsafe(no_mangle)]
pub extern "C" fn fz_draw_device_begin_path(_ctx: Handle, device: Handle) {
    if let Some(dev) = DRAW_DEVICES.get(device) {
        if let Ok(mut guard) = dev.lock() {
            guard.current_path.clear();
        }
    }
}

/// Move to point
#[unsafe(no_mangle)]
pub extern "C" fn fz_draw_device_move_to(_ctx: Handle, device: Handle, x: f32, y: f32) {
    if let Some(dev) = DRAW_DEVICES.get(device) {
        if let Ok(mut guard) = dev.lock() {
            guard.current_path.push(PathOp::MoveTo(x, y));
        }
    }
}

/// Line to point
#[unsafe(no_mangle)]
pub extern "C" fn fz_draw_device_line_to(_ctx: Handle, device: Handle, x: f32, y: f32) {
    if let Some(dev) = DRAW_DEVICES.get(device) {
        if let Ok(mut guard) = dev.lock() {
            guard.current_path.push(PathOp::LineTo(x, y));
        }
    }
}

/// Cubic bezier curve
#[unsafe(no_mangle)]
pub extern "C" fn fz_draw_device_curve_to(
    _ctx: Handle,
    device: Handle,
    x1: f32,
    y1: f32,
    x2: f32,
    y2: f32,
    x3: f32,
    y3: f32,
) {
    if let Some(dev) = DRAW_DEVICES.get(device) {
        if let Ok(mut guard) = dev.lock() {
            guard
                .current_path
                .push(PathOp::CurveTo(x1, y1, x2, y2, x3, y3));
        }
    }
}

/// Close current subpath
#[unsafe(no_mangle)]
pub extern "C" fn fz_draw_device_close_path(_ctx: Handle, device: Handle) {
    if let Some(dev) = DRAW_DEVICES.get(device) {
        if let Ok(mut guard) = dev.lock() {
            guard.current_path.push(PathOp::ClosePath);
        }
    }
}

/// Stroke the current path
#[unsafe(no_mangle)]
pub extern "C" fn fz_draw_device_stroke(_ctx: Handle, device: Handle) -> i32 {
    if let Some(dev) = DRAW_DEVICES.get(device) {
        if let Ok(mut guard) = dev.lock() {
            // In a real implementation, this would rasterize the path
            // For now, just clear the path after "drawing"
            let path_len = guard.current_path.len();
            guard.current_path.clear();
            return path_len as i32;
        }
    }
    0
}

/// Fill the current path
#[unsafe(no_mangle)]
pub extern "C" fn fz_draw_device_fill(_ctx: Handle, device: Handle, rule: i32) -> i32 {
    if let Some(dev) = DRAW_DEVICES.get(device) {
        if let Ok(mut guard) = dev.lock() {
            let _rule = if rule == 1 {
                ClipRule::EvenOdd
            } else {
                ClipRule::NonZero
            };

            let path_len = guard.current_path.len();
            guard.current_path.clear();
            return path_len as i32;
        }
    }
    0
}

// ============================================================================
// Clipping
// ============================================================================

/// Push clip path
#[unsafe(no_mangle)]
pub extern "C" fn fz_draw_device_clip(_ctx: Handle, device: Handle, rule: i32) {
    if let Some(dev) = DRAW_DEVICES.get(device) {
        if let Ok(mut guard) = dev.lock() {
            // Convert path to points for clip region
            let mut points = Vec::new();
            for op in &guard.current_path {
                match op {
                    PathOp::MoveTo(x, y) | PathOp::LineTo(x, y) => points.push((*x, *y)),
                    PathOp::CurveTo(_, _, _, _, x, y) => points.push((*x, *y)),
                    PathOp::ClosePath => {}
                }
            }

            let clip = ClipRegion {
                path: points,
                rule: if rule == 1 {
                    ClipRule::EvenOdd
                } else {
                    ClipRule::NonZero
                },
                bbox: [0.0, 0.0, guard.width as f32, guard.height as f32],
            };

            guard.clip_stack.push(clip);
            guard.current_path.clear();
        }
    }
}

/// Pop clip path
#[unsafe(no_mangle)]
pub extern "C" fn fz_draw_device_pop_clip(_ctx: Handle, device: Handle) {
    if let Some(dev) = DRAW_DEVICES.get(device) {
        if let Ok(mut guard) = dev.lock() {
            guard.clip_stack.pop();
        }
    }
}

/// Get clip depth
#[unsafe(no_mangle)]
pub extern "C" fn fz_draw_device_clip_depth(_ctx: Handle, device: Handle) -> i32 {
    if let Some(dev) = DRAW_DEVICES.get(device) {
        if let Ok(guard) = dev.lock() {
            return guard.clip_stack.len() as i32;
        }
    }
    0
}

// ============================================================================
// Pattern and Mask
// ============================================================================

/// Begin pattern fill
#[unsafe(no_mangle)]
pub extern "C" fn fz_draw_device_begin_pattern(
    _ctx: Handle,
    device: Handle,
    _pattern_handle: Handle,
    _area: *const f32,
) {
    if let Some(dev) = DRAW_DEVICES.get(device) {
        if let Ok(mut guard) = dev.lock() {
            let state_clone = guard.current_state.clone();
            guard.state_stack.push(state_clone);
        }
    }
}

/// End pattern fill
#[unsafe(no_mangle)]
pub extern "C" fn fz_draw_device_end_pattern(_ctx: Handle, device: Handle) {
    if let Some(dev) = DRAW_DEVICES.get(device) {
        if let Ok(mut guard) = dev.lock() {
            if let Some(state) = guard.state_stack.pop() {
                guard.current_state = state;
            }
        }
    }
}

/// Begin soft mask
#[unsafe(no_mangle)]
pub extern "C" fn fz_draw_device_begin_mask(
    _ctx: Handle,
    device: Handle,
    _mask_area: *const f32,
    _luminosity: i32,
) {
    if let Some(dev) = DRAW_DEVICES.get(device) {
        if let Ok(mut guard) = dev.lock() {
            let state_clone = guard.current_state.clone();
            guard.state_stack.push(state_clone);
        }
    }
}

/// End soft mask
#[unsafe(no_mangle)]
pub extern "C" fn fz_draw_device_end_mask(_ctx: Handle, device: Handle) {
    if let Some(dev) = DRAW_DEVICES.get(device) {
        if let Ok(mut guard) = dev.lock() {
            if let Some(state) = guard.state_stack.pop() {
                guard.current_state = state;
            }
        }
    }
}

// ============================================================================
// Text Operations
// ============================================================================

/// Begin text object
#[unsafe(no_mangle)]
pub extern "C" fn fz_draw_device_begin_text(_ctx: Handle, device: Handle) {
    if let Some(dev) = DRAW_DEVICES.get(device) {
        if let Ok(mut guard) = dev.lock() {
            guard.in_text = true;
        }
    }
}

/// End text object
#[unsafe(no_mangle)]
pub extern "C" fn fz_draw_device_end_text(_ctx: Handle, device: Handle) {
    if let Some(dev) = DRAW_DEVICES.get(device) {
        if let Ok(mut guard) = dev.lock() {
            guard.in_text = false;
        }
    }
}

/// Draw glyph
#[unsafe(no_mangle)]
pub extern "C" fn fz_draw_device_draw_glyph(
    _ctx: Handle,
    device: Handle,
    _font: Handle,
    _glyph_id: u32,
    _x: f32,
    _y: f32,
    _size: f32,
) -> i32 {
    if let Some(dev) = DRAW_DEVICES.get(device) {
        if let Ok(guard) = dev.lock() {
            if guard.in_text {
                return 1;
            }
        }
    }
    0
}

// ============================================================================
// Rendering Options
// ============================================================================

/// Set anti-aliasing level
#[unsafe(no_mangle)]
pub extern "C" fn fz_draw_device_set_aa_level(_ctx: Handle, device: Handle, level: i32) {
    let aa = match level {
        0 => AntiAliasLevel::None,
        1 => AntiAliasLevel::Low,
        3 => AntiAliasLevel::High,
        _ => AntiAliasLevel::Medium,
    };

    if let Some(dev) = DRAW_DEVICES.get(device) {
        if let Ok(mut guard) = dev.lock() {
            guard.aa_level = aa;
        }
    }
}

/// Enable subpixel text
#[unsafe(no_mangle)]
pub extern "C" fn fz_draw_device_enable_subpixel_text(_ctx: Handle, device: Handle, enable: i32) {
    if let Some(dev) = DRAW_DEVICES.get(device) {
        if let Ok(mut guard) = dev.lock() {
            guard.hints.subpixel_text = enable != 0;
        }
    }
}

/// Enable overprint simulation
#[unsafe(no_mangle)]
pub extern "C" fn fz_draw_device_enable_overprint(_ctx: Handle, device: Handle, enable: i32) {
    if let Some(dev) = DRAW_DEVICES.get(device) {
        if let Ok(mut guard) = dev.lock() {
            guard.hints.overprint_simulation = enable != 0;
        }
    }
}

/// Get target pixmap
#[unsafe(no_mangle)]
pub extern "C" fn fz_draw_device_target(_ctx: Handle, device: Handle) -> Handle {
    if let Some(dev) = DRAW_DEVICES.get(device) {
        if let Ok(guard) = dev.lock() {
            return guard.target;
        }
    }
    0
}

// ============================================================================
// Reference Counting
// ============================================================================

/// Keep draw device
#[unsafe(no_mangle)]
pub extern "C" fn fz_keep_draw_device(_ctx: Handle, device: Handle) -> Handle {
    DRAW_DEVICES.keep(device)
}

/// Drop draw device
#[unsafe(no_mangle)]
pub extern "C" fn fz_drop_draw_device(_ctx: Handle, device: Handle) {
    DRAW_DEVICES.remove(device);
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_draw_device() {
        let dev = fz_new_draw_device_with_size(0, 1, 800, 600);
        assert!(dev > 0);
        fz_drop_draw_device(0, dev);
    }

    #[test]
    fn test_graphics_state() {
        let dev = fz_new_draw_device_with_size(0, 1, 100, 100);

        fz_draw_device_set_fill_color(0, dev, 1.0, 0.0, 0.0, 1.0);
        fz_draw_device_set_line_width(0, dev, 2.5);

        fz_draw_device_save(0, dev);
        fz_draw_device_set_fill_color(0, dev, 0.0, 1.0, 0.0, 1.0);
        fz_draw_device_restore(0, dev);

        fz_drop_draw_device(0, dev);
    }

    #[test]
    fn test_path_operations() {
        let dev = fz_new_draw_device_with_size(0, 1, 100, 100);

        fz_draw_device_begin_path(0, dev);
        fz_draw_device_move_to(0, dev, 10.0, 10.0);
        fz_draw_device_line_to(0, dev, 90.0, 10.0);
        fz_draw_device_line_to(0, dev, 90.0, 90.0);
        fz_draw_device_line_to(0, dev, 10.0, 90.0);
        fz_draw_device_close_path(0, dev);

        let ops = fz_draw_device_fill(0, dev, 0);
        assert_eq!(ops, 5); // 1 move + 3 lines + 1 close

        fz_drop_draw_device(0, dev);
    }

    #[test]
    fn test_clipping() {
        let dev = fz_new_draw_device_with_size(0, 1, 100, 100);

        assert_eq!(fz_draw_device_clip_depth(0, dev), 0);

        fz_draw_device_begin_path(0, dev);
        fz_draw_device_move_to(0, dev, 0.0, 0.0);
        fz_draw_device_line_to(0, dev, 100.0, 100.0);
        fz_draw_device_clip(0, dev, 0);

        assert_eq!(fz_draw_device_clip_depth(0, dev), 1);

        fz_draw_device_pop_clip(0, dev);
        assert_eq!(fz_draw_device_clip_depth(0, dev), 0);

        fz_drop_draw_device(0, dev);
    }

    #[test]
    fn test_blend_modes() {
        let dev = fz_new_draw_device_with_size(0, 1, 100, 100);

        for mode in 0..16 {
            fz_draw_device_set_blend_mode(0, dev, mode);
        }

        fz_drop_draw_device(0, dev);
    }
}
