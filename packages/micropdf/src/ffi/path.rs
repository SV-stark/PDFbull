//! C FFI for fz_path - MuPDF compatible vector paths
//!
//! Provides FFI bindings for path construction and manipulation.

use super::{Handle, HandleStore};
use crate::fitz::geometry::{Point, Rect};
use crate::fitz::path::{LineCap, LineJoin, Path, StrokeState};
use std::sync::LazyLock;

/// Path storage
pub static PATHS: LazyLock<HandleStore<Path>> = LazyLock::new(HandleStore::default);

/// StrokeState storage
pub static STROKE_STATES: LazyLock<HandleStore<StrokeState>> = LazyLock::new(HandleStore::default);

/// Create a new empty path
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_path(_ctx: Handle) -> Handle {
    let path = Path::new();
    PATHS.insert(path)
}

/// Keep (increment ref) path
#[unsafe(no_mangle)]
pub extern "C" fn fz_keep_path(_ctx: Handle, path: Handle) -> Handle {
    PATHS.keep(path)
}

/// Drop path reference
#[unsafe(no_mangle)]
pub extern "C" fn fz_drop_path(_ctx: Handle, path: Handle) {
    let _ = PATHS.remove(path);
}

/// Get current point from path
#[unsafe(no_mangle)]
pub extern "C" fn fz_currentpoint(_ctx: Handle, path: Handle) -> super::geometry::fz_point {
    if let Some(p) = PATHS.get(path) {
        if let Ok(guard) = p.lock() {
            if let Some(point) = guard.current_point() {
                return super::geometry::fz_point {
                    x: point.x,
                    y: point.y,
                };
            }
        }
    }
    super::geometry::fz_point { x: 0.0, y: 0.0 }
}

/// Move to a point (start new subpath)
#[unsafe(no_mangle)]
pub extern "C" fn fz_moveto(_ctx: Handle, path: Handle, x: f32, y: f32) {
    if let Some(p) = PATHS.get(path) {
        if let Ok(mut guard) = p.lock() {
            guard.move_to(Point::new(x, y));
        }
    }
}

/// Draw line to a point
#[unsafe(no_mangle)]
pub extern "C" fn fz_lineto(_ctx: Handle, path: Handle, x: f32, y: f32) {
    if let Some(p) = PATHS.get(path) {
        if let Ok(mut guard) = p.lock() {
            guard.line_to(Point::new(x, y));
        }
    }
}

/// Draw quadratic Bezier curve
#[unsafe(no_mangle)]
pub extern "C" fn fz_quadto(_ctx: Handle, path: Handle, x1: f32, y1: f32, x2: f32, y2: f32) {
    if let Some(p) = PATHS.get(path) {
        if let Ok(mut guard) = p.lock() {
            guard.quad_to(Point::new(x1, y1), Point::new(x2, y2));
        }
    }
}

/// Draw cubic Bezier curve
#[unsafe(no_mangle)]
pub extern "C" fn fz_curveto(
    _ctx: Handle,
    path: Handle,
    x1: f32,
    y1: f32,
    x2: f32,
    y2: f32,
    x3: f32,
    y3: f32,
) {
    if let Some(p) = PATHS.get(path) {
        if let Ok(mut guard) = p.lock() {
            guard.curve_to(Point::new(x1, y1), Point::new(x2, y2), Point::new(x3, y3));
        }
    }
}

/// Close current subpath
#[unsafe(no_mangle)]
pub extern "C" fn fz_closepath(_ctx: Handle, path: Handle) {
    if let Some(p) = PATHS.get(path) {
        if let Ok(mut guard) = p.lock() {
            guard.close();
        }
    }
}

/// Add a rectangle to path
#[unsafe(no_mangle)]
pub extern "C" fn fz_rectto(_ctx: Handle, path: Handle, x0: f32, y0: f32, x1: f32, y1: f32) {
    if let Some(p) = PATHS.get(path) {
        if let Ok(mut guard) = p.lock() {
            let rect = Rect::new(x0, y0, x1, y1);
            guard.rect(rect);
        }
    }
}

/// Get path bounding box
#[unsafe(no_mangle)]
pub extern "C" fn fz_bound_path(
    _ctx: Handle,
    path: Handle,
    _stroke: Handle,
    _transform: super::geometry::fz_matrix,
) -> super::geometry::fz_rect {
    if let Some(p) = PATHS.get(path) {
        if let Ok(guard) = p.lock() {
            let bounds = guard.bounds();
            return super::geometry::fz_rect {
                x0: bounds.x0,
                y0: bounds.y0,
                x1: bounds.x1,
                y1: bounds.y1,
            };
        }
    }
    super::geometry::fz_rect {
        x0: 0.0,
        y0: 0.0,
        x1: 0.0,
        y1: 0.0,
    }
}

/// Transform path by matrix
#[unsafe(no_mangle)]
pub extern "C" fn fz_transform_path(
    _ctx: Handle,
    path: Handle,
    transform: super::geometry::fz_matrix,
) {
    if let Some(p) = PATHS.get(path) {
        if let Ok(mut guard) = p.lock() {
            let matrix = crate::fitz::geometry::Matrix::new(
                transform.a,
                transform.b,
                transform.c,
                transform.d,
                transform.e,
                transform.f,
            );
            guard.transform(|p| p.transform(&matrix));
        }
    }
}

// ============================================================================
// Stroke State Functions
// ============================================================================

/// Create a new stroke state
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_stroke_state(_ctx: Handle) -> Handle {
    let stroke = StrokeState::new();
    STROKE_STATES.insert(stroke)
}

/// Create a stroke state with line width
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_stroke_state_with_len(_ctx: Handle, _len: i32, linewidth: f32) -> Handle {
    let mut stroke = StrokeState::new();
    stroke.linewidth = linewidth;
    STROKE_STATES.insert(stroke)
}

/// Keep (increment ref) stroke state
#[unsafe(no_mangle)]
pub extern "C" fn fz_keep_stroke_state(_ctx: Handle, stroke: Handle) -> Handle {
    STROKE_STATES.keep(stroke)
}

/// Drop stroke state reference
#[unsafe(no_mangle)]
pub extern "C" fn fz_drop_stroke_state(_ctx: Handle, stroke: Handle) {
    let _ = STROKE_STATES.remove(stroke);
}

/// Clone stroke state
#[unsafe(no_mangle)]
pub extern "C" fn fz_clone_stroke_state(_ctx: Handle, stroke: Handle) -> Handle {
    if let Some(s) = STROKE_STATES.get(stroke) {
        if let Ok(guard) = s.lock() {
            let cloned = guard.clone();
            return STROKE_STATES.insert(cloned);
        }
    }
    0
}

/// Unshare stroke state (make writable copy if shared)
#[unsafe(no_mangle)]
pub extern "C" fn fz_unshare_stroke_state(_ctx: Handle, stroke: Handle) -> Handle {
    // Just clone it for simplicity
    fz_clone_stroke_state(_ctx, stroke)
}

/// Get line width from stroke state
#[unsafe(no_mangle)]
pub extern "C" fn fz_stroke_state_linewidth(_ctx: Handle, stroke: Handle) -> f32 {
    if let Some(s) = STROKE_STATES.get(stroke) {
        if let Ok(guard) = s.lock() {
            return guard.linewidth;
        }
    }
    1.0
}

/// Set line width in stroke state
#[unsafe(no_mangle)]
pub extern "C" fn fz_stroke_state_set_linewidth(_ctx: Handle, stroke: Handle, linewidth: f32) {
    if let Some(s) = STROKE_STATES.get(stroke) {
        if let Ok(mut guard) = s.lock() {
            guard.linewidth = linewidth;
        }
    }
}

/// Get start cap style
#[unsafe(no_mangle)]
pub extern "C" fn fz_stroke_state_start_cap(_ctx: Handle, stroke: Handle) -> i32 {
    if let Some(s) = STROKE_STATES.get(stroke) {
        if let Ok(guard) = s.lock() {
            return guard.start_cap as i32;
        }
    }
    0
}

/// Set start cap style
#[unsafe(no_mangle)]
pub extern "C" fn fz_stroke_state_set_start_cap(_ctx: Handle, stroke: Handle, cap: i32) {
    if let Some(s) = STROKE_STATES.get(stroke) {
        if let Ok(mut guard) = s.lock() {
            guard.start_cap = match cap {
                0 => LineCap::Butt,
                1 => LineCap::Round,
                2 => LineCap::Square,
                3 => LineCap::Triangle,
                _ => LineCap::Butt,
            };
        }
    }
}

/// Get line join style
#[unsafe(no_mangle)]
pub extern "C" fn fz_stroke_state_linejoin(_ctx: Handle, stroke: Handle) -> i32 {
    if let Some(s) = STROKE_STATES.get(stroke) {
        if let Ok(guard) = s.lock() {
            return guard.linejoin as i32;
        }
    }
    0
}

/// Set line join style
#[unsafe(no_mangle)]
pub extern "C" fn fz_stroke_state_set_linejoin(_ctx: Handle, stroke: Handle, join: i32) {
    if let Some(s) = STROKE_STATES.get(stroke) {
        if let Ok(mut guard) = s.lock() {
            guard.linejoin = match join {
                0 => LineJoin::Miter,
                1 => LineJoin::Round,
                2 => LineJoin::Bevel,
                3 => LineJoin::MiterXPS,
                _ => LineJoin::Miter,
            };
        }
    }
}

/// Get miter limit
#[unsafe(no_mangle)]
pub extern "C" fn fz_stroke_state_miterlimit(_ctx: Handle, stroke: Handle) -> f32 {
    if let Some(s) = STROKE_STATES.get(stroke) {
        if let Ok(guard) = s.lock() {
            return guard.miterlimit;
        }
    }
    10.0
}

/// Set miter limit
#[unsafe(no_mangle)]
pub extern "C" fn fz_stroke_state_set_miterlimit(_ctx: Handle, stroke: Handle, limit: f32) {
    if let Some(s) = STROKE_STATES.get(stroke) {
        if let Ok(mut guard) = s.lock() {
            guard.miterlimit = limit;
        }
    }
}

/// Set dash pattern
///
/// # Safety
/// Caller must ensure `dashes` points to readable memory of at least `len` floats.
#[unsafe(no_mangle)]
pub extern "C" fn fz_stroke_state_set_dash(
    _ctx: Handle,
    stroke: Handle,
    phase: f32,
    dashes: *const f32,
    len: i32,
) {
    if dashes.is_null() || len < 0 {
        return;
    }

    if let Some(s) = STROKE_STATES.get(stroke) {
        if let Ok(mut guard) = s.lock() {
            guard.dash_phase = phase;
            guard.dash_pattern.clear();

            unsafe {
                for i in 0..len as usize {
                    guard.dash_pattern.push(*dashes.add(i));
                }
            }
        }
    }
}

/// Check if a path is valid
#[unsafe(no_mangle)]
pub extern "C" fn fz_path_is_valid(_ctx: Handle, path: Handle) -> i32 {
    if PATHS.get(path).is_some() { 1 } else { 0 }
}

/// Clone a path (create deep copy)
#[unsafe(no_mangle)]
pub extern "C" fn fz_clone_path(_ctx: Handle, path: Handle) -> Handle {
    if let Some(p) = PATHS.get(path) {
        if let Ok(guard) = p.lock() {
            let cloned = guard.clone();
            return PATHS.insert(cloned);
        }
    }
    0
}

/// Check if a stroke state is valid
#[unsafe(no_mangle)]
pub extern "C" fn fz_stroke_state_is_valid(_ctx: Handle, stroke: Handle) -> i32 {
    if STROKE_STATES.get(stroke).is_some() {
        1
    } else {
        0
    }
}

/// Get stroke state dash phase
#[unsafe(no_mangle)]
pub extern "C" fn fz_stroke_state_dash_phase(_ctx: Handle, stroke: Handle) -> f32 {
    if let Some(s) = STROKE_STATES.get(stroke) {
        if let Ok(guard) = s.lock() {
            return guard.dash_phase;
        }
    }
    0.0
}

/// Get stroke state dash pattern length
#[unsafe(no_mangle)]
pub extern "C" fn fz_stroke_state_dash_len(_ctx: Handle, stroke: Handle) -> i32 {
    if let Some(s) = STROKE_STATES.get(stroke) {
        if let Ok(guard) = s.lock() {
            return guard.dash_pattern.len() as i32;
        }
    }
    0
}

/// Get stroke state dash pattern
///
/// # Safety
/// Caller must ensure `dashes` points to writable memory of at least `len` floats
#[unsafe(no_mangle)]
pub extern "C" fn fz_stroke_state_dash_pattern(
    _ctx: Handle,
    stroke: Handle,
    dashes: *mut f32,
    len: i32,
) -> i32 {
    if dashes.is_null() || len <= 0 {
        return 0;
    }

    if let Some(s) = STROKE_STATES.get(stroke) {
        if let Ok(guard) = s.lock() {
            let pattern = &guard.dash_pattern;
            let copy_len = pattern.len().min(len as usize);

            unsafe {
                for (i, &dash) in pattern.iter().enumerate().take(copy_len) {
                    *dashes.add(i) = dash;
                }
            }

            return copy_len as i32;
        }
    }
    0
}

/// Get stroke end cap style
#[unsafe(no_mangle)]
pub extern "C" fn fz_stroke_state_end_cap(_ctx: Handle, stroke: Handle) -> i32 {
    if let Some(s) = STROKE_STATES.get(stroke) {
        if let Ok(guard) = s.lock() {
            return guard.end_cap as i32;
        }
    }
    0
}

/// Set stroke end cap style
#[unsafe(no_mangle)]
pub extern "C" fn fz_stroke_state_set_end_cap(_ctx: Handle, stroke: Handle, cap: i32) {
    if let Some(s) = STROKE_STATES.get(stroke) {
        if let Ok(mut guard) = s.lock() {
            guard.end_cap = match cap {
                1 => crate::fitz::path::LineCap::Round,
                2 => crate::fitz::path::LineCap::Square,
                _ => crate::fitz::path::LineCap::Butt,
            };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_path() {
        let path_handle = fz_new_path(0);
        assert_ne!(path_handle, 0);
        fz_drop_path(0, path_handle);
    }

    #[test]
    fn test_keep_path() {
        let path_handle = fz_new_path(0);
        let kept = fz_keep_path(0, path_handle);
        assert_eq!(kept, path_handle);
        fz_drop_path(0, path_handle);
    }

    #[test]
    fn test_path_operations() {
        let path_handle = fz_new_path(0);

        fz_moveto(0, path_handle, 10.0, 20.0);
        fz_lineto(0, path_handle, 30.0, 40.0);
        fz_closepath(0, path_handle);

        // Current point returns the last path element's endpoint (30, 40)
        let point = fz_currentpoint(0, path_handle);
        assert!((point.x - 30.0).abs() < 0.1);
        assert!((point.y - 40.0).abs() < 0.1);

        fz_drop_path(0, path_handle);
    }

    #[test]
    fn test_path_curves() {
        let path_handle = fz_new_path(0);

        fz_moveto(0, path_handle, 0.0, 0.0);
        fz_quadto(0, path_handle, 50.0, 100.0, 100.0, 0.0);
        fz_curveto(0, path_handle, 150.0, 50.0, 150.0, 150.0, 100.0, 200.0);

        fz_drop_path(0, path_handle);
    }

    #[test]
    fn test_path_rect() {
        let path_handle = fz_new_path(0);

        fz_rectto(0, path_handle, 0.0, 0.0, 100.0, 200.0);

        let bounds = fz_bound_path(
            0,
            path_handle,
            0,
            super::super::geometry::fz_matrix::identity(),
        );
        assert!((bounds.x0 - 0.0).abs() < 0.1);
        assert!((bounds.y0 - 0.0).abs() < 0.1);
        assert!((bounds.x1 - 100.0).abs() < 0.1);
        assert!((bounds.y1 - 200.0).abs() < 0.1);

        fz_drop_path(0, path_handle);
    }

    #[test]
    fn test_new_stroke_state() {
        let stroke_handle = fz_new_stroke_state(0);
        assert_ne!(stroke_handle, 0);
        fz_drop_stroke_state(0, stroke_handle);
    }

    #[test]
    fn test_stroke_state_linewidth() {
        let stroke_handle = fz_new_stroke_state_with_len(0, 0, 2.5);
        let width = fz_stroke_state_linewidth(0, stroke_handle);
        assert!((width - 2.5).abs() < 0.01);

        fz_stroke_state_set_linewidth(0, stroke_handle, 5.0);
        let new_width = fz_stroke_state_linewidth(0, stroke_handle);
        assert!((new_width - 5.0).abs() < 0.01);

        fz_drop_stroke_state(0, stroke_handle);
    }

    #[test]
    fn test_stroke_state_cap() {
        let stroke_handle = fz_new_stroke_state(0);

        fz_stroke_state_set_start_cap(0, stroke_handle, 1); // Round
        assert_eq!(fz_stroke_state_start_cap(0, stroke_handle), 1);

        fz_stroke_state_set_start_cap(0, stroke_handle, 2); // Square
        assert_eq!(fz_stroke_state_start_cap(0, stroke_handle), 2);

        fz_drop_stroke_state(0, stroke_handle);
    }

    #[test]
    fn test_stroke_state_join() {
        let stroke_handle = fz_new_stroke_state(0);

        fz_stroke_state_set_linejoin(0, stroke_handle, 1); // Round
        assert_eq!(fz_stroke_state_linejoin(0, stroke_handle), 1);

        fz_stroke_state_set_linejoin(0, stroke_handle, 2); // Bevel
        assert_eq!(fz_stroke_state_linejoin(0, stroke_handle), 2);

        fz_drop_stroke_state(0, stroke_handle);
    }

    #[test]
    fn test_stroke_state_miterlimit() {
        let stroke_handle = fz_new_stroke_state(0);

        fz_stroke_state_set_miterlimit(0, stroke_handle, 5.0);
        let limit = fz_stroke_state_miterlimit(0, stroke_handle);
        assert!((limit - 5.0).abs() < 0.01);

        fz_drop_stroke_state(0, stroke_handle);
    }

    #[test]
    fn test_stroke_state_dash() {
        let stroke_handle = fz_new_stroke_state(0);

        let dashes = [5.0, 3.0, 2.0, 3.0];
        fz_stroke_state_set_dash(0, stroke_handle, 2.5, dashes.as_ptr(), 4);

        // Verify dash was set (check internally)
        if let Some(s) = STROKE_STATES.get(stroke_handle) {
            if let Ok(guard) = s.lock() {
                assert!((guard.dash_phase - 2.5).abs() < 0.01);
                assert_eq!(guard.dash_pattern.len(), 4);
            }
        }

        fz_drop_stroke_state(0, stroke_handle);
    }

    #[test]
    fn test_clone_stroke_state() {
        let stroke_handle = fz_new_stroke_state(0);
        fz_stroke_state_set_linewidth(0, stroke_handle, 3.0);

        let cloned = fz_clone_stroke_state(0, stroke_handle);
        assert_ne!(cloned, 0);
        assert_ne!(cloned, stroke_handle);

        let width = fz_stroke_state_linewidth(0, cloned);
        assert!((width - 3.0).abs() < 0.01);

        fz_drop_stroke_state(0, cloned);
        fz_drop_stroke_state(0, stroke_handle);
    }
}
