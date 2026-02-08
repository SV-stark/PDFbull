//! C FFI for shading/gradients - MuPDF compatible
//! Safe Rust implementation of fz_shade

use super::{Handle, HandleStore};
use std::sync::LazyLock;

/// Shading type enumeration (PDF spec types)
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShadeType {
    /// No shading
    None = 0,
    /// Function-based shading (Type 1)
    Function = 1,
    /// Axial/Linear gradient (Type 2)
    Linear = 2,
    /// Radial gradient (Type 3)
    Radial = 3,
    /// Free-form Gouraud-shaded triangle mesh (Type 4)
    FreeFormTriangle = 4,
    /// Lattice-form Gouraud-shaded triangle mesh (Type 5)
    LatticeTriangle = 5,
    /// Coons patch mesh (Type 6)
    CoonsPatch = 6,
    /// Tensor-product patch mesh (Type 7)
    TensorPatch = 7,
}

/// Color stop for gradients
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ColorStop {
    /// Position along gradient (0.0 to 1.0)
    pub offset: f32,
    /// Color components (up to 4 for CMYK)
    pub color: [f32; 4],
    /// Number of color components used
    pub n: i32,
}

impl Default for ColorStop {
    fn default() -> Self {
        Self {
            offset: 0.0,
            color: [0.0; 4],
            n: 3,
        }
    }
}

/// Point in 2D space
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct ShadePoint {
    pub x: f32,
    pub y: f32,
}

/// Vertex with position and color
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ShadeVertex {
    pub point: ShadePoint,
    pub color: [f32; 4],
}

impl Default for ShadeVertex {
    fn default() -> Self {
        Self {
            point: ShadePoint::default(),
            color: [0.0; 4],
        }
    }
}

/// Coons/Tensor patch control points (16 points)
#[repr(C)]
#[derive(Debug, Clone)]
pub struct ShadePatch {
    /// Control points for patch (16 for tensor, 12 used for Coons)
    pub points: [ShadePoint; 16],
    /// Corner colors (4 corners)
    pub colors: [[f32; 4]; 4],
}

impl Default for ShadePatch {
    fn default() -> Self {
        Self {
            points: [ShadePoint::default(); 16],
            colors: [[0.0; 4]; 4],
        }
    }
}

/// Shade/gradient structure
#[derive(Debug, Clone)]
pub struct Shade {
    /// Type of shading
    pub shade_type: ShadeType,
    /// Colorspace handle
    pub colorspace: u64,
    /// Bounding box
    pub bbox: [f32; 4],
    /// Background color (if any)
    pub background: Option<[f32; 4]>,
    /// Whether to use function (vs stitching)
    pub use_function: bool,
    /// Whether to extend before start
    pub extend_start: bool,
    /// Whether to extend after end
    pub extend_end: bool,
    /// For linear: start point
    pub linear_start: ShadePoint,
    /// For linear: end point
    pub linear_end: ShadePoint,
    /// For radial: start center
    pub radial_start: ShadePoint,
    /// For radial: start radius
    pub radial_r0: f32,
    /// For radial: end center
    pub radial_end: ShadePoint,
    /// For radial: end radius
    pub radial_r1: f32,
    /// Color stops for gradients
    pub color_stops: Vec<ColorStop>,
    /// Mesh vertices (for types 4-5)
    pub vertices: Vec<ShadeVertex>,
    /// Mesh patches (for types 6-7)
    pub patches: Vec<ShadePatch>,
    /// Bits per coordinate (mesh)
    pub bits_per_coord: i32,
    /// Bits per component (mesh)
    pub bits_per_comp: i32,
    /// Bits per flag (mesh)
    pub bits_per_flag: i32,
    /// Domain for function-based
    pub domain: [f32; 4],
    /// Matrix for function-based
    pub matrix: [f32; 6],
}

impl Default for Shade {
    fn default() -> Self {
        Self {
            shade_type: ShadeType::None,
            colorspace: 0,
            bbox: [0.0, 0.0, 1.0, 1.0],
            background: None,
            use_function: false,
            extend_start: false,
            extend_end: false,
            linear_start: ShadePoint::default(),
            linear_end: ShadePoint { x: 1.0, y: 0.0 },
            radial_start: ShadePoint::default(),
            radial_r0: 0.0,
            radial_end: ShadePoint::default(),
            radial_r1: 1.0,
            color_stops: Vec::new(),
            vertices: Vec::new(),
            patches: Vec::new(),
            bits_per_coord: 8,
            bits_per_comp: 8,
            bits_per_flag: 2,
            domain: [0.0, 1.0, 0.0, 1.0],
            matrix: [1.0, 0.0, 0.0, 1.0, 0.0, 0.0], // Identity
        }
    }
}

/// Global shade storage
pub static SHADES: LazyLock<HandleStore<Shade>> = LazyLock::new(HandleStore::new);

// ============================================================================
// Shade Creation Functions
// ============================================================================

/// Create a new linear (axial) gradient
///
/// Creates a gradient that transitions colors along a line from (x0,y0) to (x1,y1).
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_linear_shade(
    _ctx: Handle,
    colorspace: u64,
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
    extend_start: i32,
    extend_end: i32,
) -> Handle {
    let shade = Shade {
        shade_type: ShadeType::Linear,
        colorspace,
        linear_start: ShadePoint { x: x0, y: y0 },
        linear_end: ShadePoint { x: x1, y: y1 },
        extend_start: extend_start != 0,
        extend_end: extend_end != 0,
        ..Default::default()
    };

    SHADES.insert(shade)
}

/// Create a new radial gradient
///
/// Creates a gradient that transitions colors between two circles.
/// The inner circle is at (x0,y0) with radius r0, outer at (x1,y1) with radius r1.
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_radial_shade(
    _ctx: Handle,
    colorspace: u64,
    x0: f32,
    y0: f32,
    r0: f32,
    x1: f32,
    y1: f32,
    r1: f32,
    extend_start: i32,
    extend_end: i32,
) -> Handle {
    let shade = Shade {
        shade_type: ShadeType::Radial,
        colorspace,
        radial_start: ShadePoint { x: x0, y: y0 },
        radial_r0: r0,
        radial_end: ShadePoint { x: x1, y: y1 },
        radial_r1: r1,
        extend_start: extend_start != 0,
        extend_end: extend_end != 0,
        ..Default::default()
    };

    SHADES.insert(shade)
}

/// Create a new function-based shade (Type 1)
///
/// Creates a shading that uses a function to determine colors.
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_function_shade(
    _ctx: Handle,
    colorspace: u64,
    domain: *const f32,
    matrix: *const f32,
) -> Handle {
    let mut shade = Shade {
        shade_type: ShadeType::Function,
        colorspace,
        ..Default::default()
    };

    if !domain.is_null() {
        let domain_slice = unsafe { std::slice::from_raw_parts(domain, 4) };
        shade.domain.copy_from_slice(domain_slice);
    }

    if !matrix.is_null() {
        let matrix_slice = unsafe { std::slice::from_raw_parts(matrix, 6) };
        shade.matrix.copy_from_slice(matrix_slice);
    }

    SHADES.insert(shade)
}

/// Create a new mesh shade (Types 4-7)
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_mesh_shade(
    _ctx: Handle,
    colorspace: u64,
    shade_type: i32,
    bits_per_coord: i32,
    bits_per_comp: i32,
    bits_per_flag: i32,
) -> Handle {
    let st = match shade_type {
        4 => ShadeType::FreeFormTriangle,
        5 => ShadeType::LatticeTriangle,
        6 => ShadeType::CoonsPatch,
        7 => ShadeType::TensorPatch,
        _ => return 0, // Invalid type
    };

    let shade = Shade {
        shade_type: st,
        colorspace,
        bits_per_coord,
        bits_per_comp,
        bits_per_flag,
        ..Default::default()
    };

    SHADES.insert(shade)
}

// ============================================================================
// Shade Manipulation Functions
// ============================================================================

/// Add a color stop to a gradient
///
/// # Safety
/// `color` must point to at least `n` floats.
#[unsafe(no_mangle)]
pub extern "C" fn fz_shade_add_color_stop(
    _ctx: Handle,
    shade: Handle,
    offset: f32,
    color: *const f32,
    n: i32,
) -> i32 {
    if color.is_null() || n <= 0 || n > 4 {
        return 0;
    }

    if let Some(shade_arc) = SHADES.get(shade) {
        if let Ok(mut guard) = shade_arc.lock() {
            let color_slice = unsafe { std::slice::from_raw_parts(color, n as usize) };
            let mut stop = ColorStop {
                offset: offset.clamp(0.0, 1.0),
                color: [0.0; 4],
                n,
            };
            stop.color[..n as usize].copy_from_slice(color_slice);
            guard.color_stops.push(stop);

            // Keep stops sorted by offset
            guard
                .color_stops
                .sort_by(|a, b| a.offset.partial_cmp(&b.offset).unwrap());
            return 1;
        }
    }
    0
}

/// Add a vertex to a mesh shade
#[unsafe(no_mangle)]
pub extern "C" fn fz_shade_add_vertex(
    _ctx: Handle,
    shade: Handle,
    x: f32,
    y: f32,
    color: *const f32,
    n: i32,
) -> i32 {
    if color.is_null() || n <= 0 || n > 4 {
        return 0;
    }

    if let Some(shade_arc) = SHADES.get(shade) {
        if let Ok(mut guard) = shade_arc.lock() {
            let color_slice = unsafe { std::slice::from_raw_parts(color, n as usize) };
            let mut vertex = ShadeVertex {
                point: ShadePoint { x, y },
                color: [0.0; 4],
            };
            vertex.color[..n as usize].copy_from_slice(color_slice);
            guard.vertices.push(vertex);
            return 1;
        }
    }
    0
}

/// Add a patch to a mesh shade (Coons or Tensor)
///
/// # Safety
/// `points` must point to 16 ShadePoints, `colors` must point to 4 arrays of 4 floats.
#[unsafe(no_mangle)]
pub extern "C" fn fz_shade_add_patch(
    _ctx: Handle,
    shade: Handle,
    points: *const ShadePoint,
    colors: *const [f32; 4],
) -> i32 {
    if points.is_null() || colors.is_null() {
        return 0;
    }

    if let Some(shade_arc) = SHADES.get(shade) {
        if let Ok(mut guard) = shade_arc.lock() {
            let points_slice = unsafe { std::slice::from_raw_parts(points, 16) };
            let colors_slice = unsafe { std::slice::from_raw_parts(colors, 4) };

            let mut patch = ShadePatch::default();
            patch.points.copy_from_slice(points_slice);
            patch.colors.copy_from_slice(colors_slice);

            guard.patches.push(patch);
            return 1;
        }
    }
    0
}

/// Set bounding box for shade
#[unsafe(no_mangle)]
pub extern "C" fn fz_shade_set_bbox(
    _ctx: Handle,
    shade: Handle,
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
) {
    if let Some(shade_arc) = SHADES.get(shade) {
        if let Ok(mut guard) = shade_arc.lock() {
            guard.bbox = [x0, y0, x1, y1];
        }
    }
}

/// Set background color for shade
///
/// # Safety
/// `color` must point to at least 4 floats.
#[unsafe(no_mangle)]
pub extern "C" fn fz_shade_set_background(_ctx: Handle, shade: Handle, color: *const f32) {
    if color.is_null() {
        return;
    }

    if let Some(shade_arc) = SHADES.get(shade) {
        if let Ok(mut guard) = shade_arc.lock() {
            let color_slice = unsafe { std::slice::from_raw_parts(color, 4) };
            let mut bg = [0.0f32; 4];
            bg.copy_from_slice(color_slice);
            guard.background = Some(bg);
        }
    }
}

// ============================================================================
// Shade Query Functions
// ============================================================================

/// Get shade type
#[unsafe(no_mangle)]
pub extern "C" fn fz_shade_type(_ctx: Handle, shade: Handle) -> i32 {
    if let Some(shade_arc) = SHADES.get(shade) {
        if let Ok(guard) = shade_arc.lock() {
            return guard.shade_type as i32;
        }
    }
    0
}

/// Get shade colorspace
#[unsafe(no_mangle)]
pub extern "C" fn fz_shade_colorspace(_ctx: Handle, shade: Handle) -> u64 {
    if let Some(shade_arc) = SHADES.get(shade) {
        if let Ok(guard) = shade_arc.lock() {
            return guard.colorspace;
        }
    }
    0
}

/// Get bounding box
///
/// # Safety
/// `bbox` must point to an array of 4 floats.
#[unsafe(no_mangle)]
pub extern "C" fn fz_shade_bbox(_ctx: Handle, shade: Handle, bbox: *mut f32) {
    if bbox.is_null() {
        return;
    }

    if let Some(shade_arc) = SHADES.get(shade) {
        if let Ok(guard) = shade_arc.lock() {
            let bbox_slice = unsafe { std::slice::from_raw_parts_mut(bbox, 4) };
            bbox_slice.copy_from_slice(&guard.bbox);
        }
    }
}

/// Get number of color stops
#[unsafe(no_mangle)]
pub extern "C" fn fz_shade_color_stop_count(_ctx: Handle, shade: Handle) -> i32 {
    if let Some(shade_arc) = SHADES.get(shade) {
        if let Ok(guard) = shade_arc.lock() {
            return guard.color_stops.len() as i32;
        }
    }
    0
}

/// Get color stop at index
///
/// # Safety
/// `offset` and `color` must be valid pointers.
#[unsafe(no_mangle)]
pub extern "C" fn fz_shade_get_color_stop(
    _ctx: Handle,
    shade: Handle,
    index: i32,
    offset: *mut f32,
    color: *mut f32,
) -> i32 {
    if offset.is_null() || color.is_null() || index < 0 {
        return 0;
    }

    if let Some(shade_arc) = SHADES.get(shade) {
        if let Ok(guard) = shade_arc.lock() {
            if let Some(stop) = guard.color_stops.get(index as usize) {
                unsafe {
                    *offset = stop.offset;
                    let color_slice = std::slice::from_raw_parts_mut(color, 4);
                    color_slice.copy_from_slice(&stop.color);
                }
                return stop.n;
            }
        }
    }
    0
}

/// Get number of vertices (mesh shades)
#[unsafe(no_mangle)]
pub extern "C" fn fz_shade_vertex_count(_ctx: Handle, shade: Handle) -> i32 {
    if let Some(shade_arc) = SHADES.get(shade) {
        if let Ok(guard) = shade_arc.lock() {
            return guard.vertices.len() as i32;
        }
    }
    0
}

/// Get number of patches (Coons/Tensor shades)
#[unsafe(no_mangle)]
pub extern "C" fn fz_shade_patch_count(_ctx: Handle, shade: Handle) -> i32 {
    if let Some(shade_arc) = SHADES.get(shade) {
        if let Ok(guard) = shade_arc.lock() {
            return guard.patches.len() as i32;
        }
    }
    0
}

/// Check if shade uses extend at start
#[unsafe(no_mangle)]
pub extern "C" fn fz_shade_extend_start(_ctx: Handle, shade: Handle) -> i32 {
    if let Some(shade_arc) = SHADES.get(shade) {
        if let Ok(guard) = shade_arc.lock() {
            return i32::from(guard.extend_start);
        }
    }
    0
}

/// Check if shade uses extend at end
#[unsafe(no_mangle)]
pub extern "C" fn fz_shade_extend_end(_ctx: Handle, shade: Handle) -> i32 {
    if let Some(shade_arc) = SHADES.get(shade) {
        if let Ok(guard) = shade_arc.lock() {
            return i32::from(guard.extend_end);
        }
    }
    0
}

/// Check if shade has background
#[unsafe(no_mangle)]
pub extern "C" fn fz_shade_has_background(_ctx: Handle, shade: Handle) -> i32 {
    if let Some(shade_arc) = SHADES.get(shade) {
        if let Ok(guard) = shade_arc.lock() {
            return i32::from(guard.background.is_some());
        }
    }
    0
}

// ============================================================================
// Shade Color Sampling
// ============================================================================

/// Sample color from gradient at position t (0.0 to 1.0)
///
/// # Safety
/// `color` must point to at least 4 floats.
#[unsafe(no_mangle)]
pub extern "C" fn fz_shade_sample(_ctx: Handle, shade: Handle, t: f32, color: *mut f32) {
    if color.is_null() {
        return;
    }

    if let Some(shade_arc) = SHADES.get(shade) {
        if let Ok(guard) = shade_arc.lock() {
            let color_slice = unsafe { std::slice::from_raw_parts_mut(color, 4) };

            // Default to black
            color_slice.fill(0.0);

            if guard.color_stops.is_empty() {
                return;
            }

            let t = t.clamp(0.0, 1.0);

            // Find bounding stops
            let mut prev_stop = &guard.color_stops[0];
            let mut next_stop = &guard.color_stops[guard.color_stops.len() - 1];

            for stop in &guard.color_stops {
                if stop.offset <= t {
                    prev_stop = stop;
                }
                if stop.offset >= t && stop.offset <= next_stop.offset {
                    next_stop = stop;
                    break;
                }
            }

            // Interpolate between stops
            if (next_stop.offset - prev_stop.offset).abs() < f32::EPSILON {
                color_slice.copy_from_slice(&prev_stop.color);
            } else {
                let blend = (t - prev_stop.offset) / (next_stop.offset - prev_stop.offset);
                for i in 0..4 {
                    color_slice[i] =
                        prev_stop.color[i] + blend * (next_stop.color[i] - prev_stop.color[i]);
                }
            }
        }
    }
}

// ============================================================================
// Reference Counting
// ============================================================================

/// Keep (increment reference to) shade
#[unsafe(no_mangle)]
pub extern "C" fn fz_keep_shade(_ctx: Handle, shade: Handle) -> Handle {
    SHADES.keep(shade)
}

/// Drop shade reference
#[unsafe(no_mangle)]
pub extern "C" fn fz_drop_shade(_ctx: Handle, shade: Handle) {
    SHADES.remove(shade);
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_linear_gradient_creation() {
        let shade = fz_new_linear_shade(0, 2, 0.0, 0.0, 100.0, 0.0, 1, 1);
        assert!(shade > 0);

        assert_eq!(fz_shade_type(0, shade), ShadeType::Linear as i32);
        assert_eq!(fz_shade_extend_start(0, shade), 1);
        assert_eq!(fz_shade_extend_end(0, shade), 1);

        fz_drop_shade(0, shade);
    }

    #[test]
    fn test_radial_gradient_creation() {
        let shade = fz_new_radial_shade(0, 2, 50.0, 50.0, 0.0, 50.0, 50.0, 100.0, 0, 1);
        assert!(shade > 0);

        assert_eq!(fz_shade_type(0, shade), ShadeType::Radial as i32);

        fz_drop_shade(0, shade);
    }

    #[test]
    fn test_color_stops() {
        let shade = fz_new_linear_shade(0, 2, 0.0, 0.0, 100.0, 0.0, 0, 0);

        let red = [1.0f32, 0.0, 0.0, 1.0];
        let blue = [0.0f32, 0.0, 1.0, 1.0];

        fz_shade_add_color_stop(0, shade, 0.0, red.as_ptr(), 4);
        fz_shade_add_color_stop(0, shade, 1.0, blue.as_ptr(), 4);

        assert_eq!(fz_shade_color_stop_count(0, shade), 2);

        // Sample at midpoint
        let mut color = [0.0f32; 4];
        fz_shade_sample(0, shade, 0.5, color.as_mut_ptr());

        // Should be purple-ish (mix of red and blue)
        assert!((color[0] - 0.5).abs() < 0.1);
        assert!((color[2] - 0.5).abs() < 0.1);

        fz_drop_shade(0, shade);
    }

    #[test]
    fn test_mesh_shade() {
        let shade = fz_new_mesh_shade(0, 2, 6, 8, 8, 2); // Coons patch
        assert!(shade > 0);

        assert_eq!(fz_shade_type(0, shade), ShadeType::CoonsPatch as i32);

        fz_drop_shade(0, shade);
    }

    #[test]
    fn test_invalid_mesh_type() {
        let shade = fz_new_mesh_shade(0, 2, 99, 8, 8, 2);
        assert_eq!(shade, 0);
    }

    #[test]
    fn test_bounding_box() {
        let shade = fz_new_linear_shade(0, 2, 0.0, 0.0, 100.0, 100.0, 0, 0);

        fz_shade_set_bbox(0, shade, 10.0, 20.0, 90.0, 80.0);

        let mut bbox = [0.0f32; 4];
        fz_shade_bbox(0, shade, bbox.as_mut_ptr());

        assert_eq!(bbox, [10.0, 20.0, 90.0, 80.0]);

        fz_drop_shade(0, shade);
    }

    #[test]
    fn test_background() {
        let shade = fz_new_linear_shade(0, 2, 0.0, 0.0, 100.0, 0.0, 0, 0);

        assert_eq!(fz_shade_has_background(0, shade), 0);

        let bg = [1.0f32, 1.0, 1.0, 1.0];
        fz_shade_set_background(0, shade, bg.as_ptr());

        assert_eq!(fz_shade_has_background(0, shade), 1);

        fz_drop_shade(0, shade);
    }
}
