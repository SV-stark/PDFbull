//! C FFI for geometry types - MuPDF compatible
//!
//! # Safety Note
//! This module provides C FFI exports which require `unsafe` annotations
//! in Rust 2024 edition. The internal implementation is 100% safe Rust.
//! The `#[unsafe(no_mangle)]` attribute is required for C symbol visibility.

use std::ffi::c_float;

/// fz_point - 2D point
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct fz_point {
    pub x: c_float,
    pub y: c_float,
}

/// fz_rect - Rectangle
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct fz_rect {
    pub x0: c_float,
    pub y0: c_float,
    pub x1: c_float,
    pub y1: c_float,
}

/// fz_irect - Integer rectangle
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct fz_irect {
    pub x0: i32,
    pub y0: i32,
    pub x1: i32,
    pub y1: i32,
}

/// fz_matrix - Transformation matrix
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct fz_matrix {
    pub a: c_float,
    pub b: c_float,
    pub c: c_float,
    pub d: c_float,
    pub e: c_float,
    pub f: c_float,
}

impl Default for fz_matrix {
    fn default() -> Self {
        Self::identity()
    }
}

impl fz_matrix {
    pub const fn identity() -> Self {
        Self {
            a: 1.0,
            b: 0.0,
            c: 0.0,
            d: 1.0,
            e: 0.0,
            f: 0.0,
        }
    }
}

/// fz_quad - Quadrilateral
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct fz_quad {
    pub ul: fz_point,
    pub ur: fz_point,
    pub ll: fz_point,
    pub lr: fz_point,
}

// Constants
pub const FZ_MIN_INF_RECT: i32 = i32::MIN;
pub const FZ_MAX_INF_RECT: i32 = 0x7fffff80;

// Static constants exposed to C
// SAFETY: These are constant data with no mutable access, safe for FFI export
#[unsafe(no_mangle)]
pub static fz_identity: fz_matrix = fz_matrix {
    a: 1.0,
    b: 0.0,
    c: 0.0,
    d: 1.0,
    e: 0.0,
    f: 0.0,
};

#[unsafe(no_mangle)]
pub static fz_empty_rect: fz_rect = fz_rect {
    x0: f32::INFINITY,
    y0: f32::INFINITY,
    x1: f32::NEG_INFINITY,
    y1: f32::NEG_INFINITY,
};

#[unsafe(no_mangle)]
pub static fz_infinite_rect: fz_rect = fz_rect {
    x0: FZ_MIN_INF_RECT as f32,
    y0: FZ_MIN_INF_RECT as f32,
    x1: FZ_MAX_INF_RECT as f32,
    y1: FZ_MAX_INF_RECT as f32,
};

#[unsafe(no_mangle)]
pub static fz_unit_rect: fz_rect = fz_rect {
    x0: 0.0,
    y0: 0.0,
    x1: 1.0,
    y1: 1.0,
};

#[unsafe(no_mangle)]
pub static fz_empty_irect: fz_irect = fz_irect {
    x0: 0,
    y0: 0,
    x1: 0,
    y1: 0,
};

#[unsafe(no_mangle)]
pub static fz_infinite_irect: fz_irect = fz_irect {
    x0: FZ_MIN_INF_RECT,
    y0: FZ_MIN_INF_RECT,
    x1: FZ_MAX_INF_RECT,
    y1: FZ_MAX_INF_RECT,
};

// ============================================================================
// Matrix functions - Pure safe Rust implementations with FFI export
// ============================================================================

/// Concatenate two matrices
#[unsafe(no_mangle)]
pub extern "C" fn fz_concat(left: fz_matrix, right: fz_matrix) -> fz_matrix {
    fz_matrix {
        a: left.a * right.a + left.b * right.c,
        b: left.a * right.b + left.b * right.d,
        c: left.c * right.a + left.d * right.c,
        d: left.c * right.b + left.d * right.d,
        e: left.e * right.a + left.f * right.c + right.e,
        f: left.e * right.b + left.f * right.d + right.f,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn fz_scale(sx: c_float, sy: c_float) -> fz_matrix {
    fz_matrix {
        a: sx,
        b: 0.0,
        c: 0.0,
        d: sy,
        e: 0.0,
        f: 0.0,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn fz_pre_scale(m: fz_matrix, sx: c_float, sy: c_float) -> fz_matrix {
    fz_concat(fz_scale(sx, sy), m)
}

#[unsafe(no_mangle)]
pub extern "C" fn fz_post_scale(m: fz_matrix, sx: c_float, sy: c_float) -> fz_matrix {
    fz_concat(m, fz_scale(sx, sy))
}

#[unsafe(no_mangle)]
pub extern "C" fn fz_shear(sx: c_float, sy: c_float) -> fz_matrix {
    fz_matrix {
        a: 1.0,
        b: sy,
        c: sx,
        d: 1.0,
        e: 0.0,
        f: 0.0,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn fz_pre_shear(m: fz_matrix, sx: c_float, sy: c_float) -> fz_matrix {
    fz_concat(fz_shear(sx, sy), m)
}

#[unsafe(no_mangle)]
pub extern "C" fn fz_rotate(degrees: c_float) -> fz_matrix {
    let rad = degrees * std::f32::consts::PI / 180.0;
    let (s, c) = rad.sin_cos();
    fz_matrix {
        a: c,
        b: s,
        c: -s,
        d: c,
        e: 0.0,
        f: 0.0,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn fz_pre_rotate(m: fz_matrix, degrees: c_float) -> fz_matrix {
    fz_concat(fz_rotate(degrees), m)
}

#[unsafe(no_mangle)]
pub extern "C" fn fz_translate(tx: c_float, ty: c_float) -> fz_matrix {
    fz_matrix {
        a: 1.0,
        b: 0.0,
        c: 0.0,
        d: 1.0,
        e: tx,
        f: ty,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn fz_pre_translate(m: fz_matrix, tx: c_float, ty: c_float) -> fz_matrix {
    fz_concat(fz_translate(tx, ty), m)
}

#[unsafe(no_mangle)]
pub extern "C" fn fz_invert_matrix(m: fz_matrix) -> fz_matrix {
    let det = m.a * m.d - m.b * m.c;
    if det.abs() < 1e-6 {
        return m; // Singular matrix, return original
    }
    let rdet = 1.0 / det;
    fz_matrix {
        a: m.d * rdet,
        b: -m.b * rdet,
        c: -m.c * rdet,
        d: m.a * rdet,
        e: (m.c * m.f - m.d * m.e) * rdet,
        f: (m.b * m.e - m.a * m.f) * rdet,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn fz_matrix_expansion(m: fz_matrix) -> c_float {
    (m.a.abs() * m.d.abs() - m.b.abs() * m.c.abs()).abs().sqrt()
}

#[unsafe(no_mangle)]
pub extern "C" fn fz_matrix_max_expansion(m: fz_matrix) -> c_float {
    m.a.abs().max(m.b.abs()).max(m.c.abs()).max(m.d.abs())
}

#[unsafe(no_mangle)]
pub extern "C" fn fz_is_rectilinear(m: fz_matrix) -> i32 {
    if (m.b.abs() < 1e-6 && m.c.abs() < 1e-6) || (m.a.abs() < 1e-6 && m.d.abs() < 1e-6) {
        1
    } else {
        0
    }
}

// ============================================================================
// Rect functions
// ============================================================================

#[unsafe(no_mangle)]
pub extern "C" fn fz_intersect_rect(a: fz_rect, b: fz_rect) -> fz_rect {
    fz_rect {
        x0: a.x0.max(b.x0),
        y0: a.y0.max(b.y0),
        x1: a.x1.min(b.x1),
        y1: a.y1.min(b.y1),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn fz_intersect_irect(a: fz_irect, b: fz_irect) -> fz_irect {
    fz_irect {
        x0: a.x0.max(b.x0),
        y0: a.y0.max(b.y0),
        x1: a.x1.min(b.x1),
        y1: a.y1.min(b.y1),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn fz_union_rect(a: fz_rect, b: fz_rect) -> fz_rect {
    fz_rect {
        x0: a.x0.min(b.x0),
        y0: a.y0.min(b.y0),
        x1: a.x1.max(b.x1),
        y1: a.y1.max(b.y1),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn fz_irect_from_rect(rect: fz_rect) -> fz_irect {
    fz_irect {
        x0: rect.x0.floor() as i32,
        y0: rect.y0.floor() as i32,
        x1: rect.x1.ceil() as i32,
        y1: rect.y1.ceil() as i32,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn fz_round_rect(rect: fz_rect) -> fz_irect {
    fz_irect {
        x0: (rect.x0 + 0.001).floor() as i32,
        y0: (rect.y0 + 0.001).floor() as i32,
        x1: (rect.x1 - 0.001).ceil() as i32,
        y1: (rect.y1 - 0.001).ceil() as i32,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn fz_rect_from_irect(bbox: fz_irect) -> fz_rect {
    fz_rect {
        x0: bbox.x0 as f32,
        y0: bbox.y0 as f32,
        x1: bbox.x1 as f32,
        y1: bbox.y1 as f32,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn fz_expand_rect(r: fz_rect, expand: c_float) -> fz_rect {
    fz_rect {
        x0: r.x0 - expand,
        y0: r.y0 - expand,
        x1: r.x1 + expand,
        y1: r.y1 + expand,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn fz_expand_irect(r: fz_irect, expand: i32) -> fz_irect {
    fz_irect {
        x0: r.x0.saturating_sub(expand),
        y0: r.y0.saturating_sub(expand),
        x1: r.x1.saturating_add(expand),
        y1: r.y1.saturating_add(expand),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn fz_include_point_in_rect(r: fz_rect, p: fz_point) -> fz_rect {
    fz_rect {
        x0: r.x0.min(p.x),
        y0: r.y0.min(p.y),
        x1: r.x1.max(p.x),
        y1: r.y1.max(p.y),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn fz_translate_rect(r: fz_rect, xoff: c_float, yoff: c_float) -> fz_rect {
    fz_rect {
        x0: r.x0 + xoff,
        y0: r.y0 + yoff,
        x1: r.x1 + xoff,
        y1: r.y1 + yoff,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn fz_translate_irect(r: fz_irect, xoff: i32, yoff: i32) -> fz_irect {
    fz_irect {
        x0: r.x0.saturating_add(xoff),
        y0: r.y0.saturating_add(yoff),
        x1: r.x1.saturating_add(xoff),
        y1: r.y1.saturating_add(yoff),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn fz_contains_rect(a: fz_rect, b: fz_rect) -> i32 {
    i32::from(a.x0 <= b.x0 && a.y0 <= b.y0 && a.x1 >= b.x1 && a.y1 >= b.y1)
}

#[unsafe(no_mangle)]
pub extern "C" fn fz_overlaps_rect(a: fz_rect, b: fz_rect) -> i32 {
    i32::from(a.x0 < b.x1 && b.x0 < a.x1 && a.y0 < b.y1 && b.y0 < a.y1)
}

// ============================================================================
// Point/Transform functions
// ============================================================================

#[unsafe(no_mangle)]
pub extern "C" fn fz_transform_point(p: fz_point, m: fz_matrix) -> fz_point {
    fz_point {
        x: p.x * m.a + p.y * m.c + m.e,
        y: p.x * m.b + p.y * m.d + m.f,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn fz_transform_point_xy(x: c_float, y: c_float, m: fz_matrix) -> fz_point {
    fz_point {
        x: x * m.a + y * m.c + m.e,
        y: x * m.b + y * m.d + m.f,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn fz_transform_vector(v: fz_point, m: fz_matrix) -> fz_point {
    fz_point {
        x: v.x * m.a + v.y * m.c,
        y: v.x * m.b + v.y * m.d,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn fz_transform_rect(r: fz_rect, m: fz_matrix) -> fz_rect {
    if fz_is_rectilinear(m) != 0 {
        let p1 = fz_transform_point(fz_point { x: r.x0, y: r.y0 }, m);
        let p2 = fz_transform_point(fz_point { x: r.x1, y: r.y1 }, m);
        fz_rect {
            x0: p1.x.min(p2.x),
            y0: p1.y.min(p2.y),
            x1: p1.x.max(p2.x),
            y1: p1.y.max(p2.y),
        }
    } else {
        let p1 = fz_transform_point(fz_point { x: r.x0, y: r.y0 }, m);
        let p2 = fz_transform_point(fz_point { x: r.x1, y: r.y0 }, m);
        let p3 = fz_transform_point(fz_point { x: r.x0, y: r.y1 }, m);
        let p4 = fz_transform_point(fz_point { x: r.x1, y: r.y1 }, m);
        fz_rect {
            x0: p1.x.min(p2.x).min(p3.x).min(p4.x),
            y0: p1.y.min(p2.y).min(p3.y).min(p4.y),
            x1: p1.x.max(p2.x).max(p3.x).max(p4.x),
            y1: p1.y.max(p2.y).max(p3.y).max(p4.y),
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn fz_normalize_vector(p: fz_point) -> fz_point {
    let len = (p.x * p.x + p.y * p.y).sqrt();
    if len < 1e-6 {
        return fz_point { x: 0.0, y: 0.0 };
    }
    fz_point {
        x: p.x / len,
        y: p.y / len,
    }
}

// ============================================================================
// Quad functions
// ============================================================================

#[unsafe(no_mangle)]
pub extern "C" fn fz_quad_from_rect(r: fz_rect) -> fz_quad {
    fz_quad {
        ul: fz_point { x: r.x0, y: r.y0 },
        ur: fz_point { x: r.x1, y: r.y0 },
        ll: fz_point { x: r.x0, y: r.y1 },
        lr: fz_point { x: r.x1, y: r.y1 },
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn fz_rect_from_quad(q: fz_quad) -> fz_rect {
    fz_rect {
        x0: q.ul.x.min(q.ur.x).min(q.ll.x).min(q.lr.x),
        y0: q.ul.y.min(q.ur.y).min(q.ll.y).min(q.lr.y),
        x1: q.ul.x.max(q.ur.x).max(q.ll.x).max(q.lr.x),
        y1: q.ul.y.max(q.ur.y).max(q.ll.y).max(q.lr.y),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn fz_transform_quad(q: fz_quad, m: fz_matrix) -> fz_quad {
    fz_quad {
        ul: fz_transform_point(q.ul, m),
        ur: fz_transform_point(q.ur, m),
        ll: fz_transform_point(q.ll, m),
        lr: fz_transform_point(q.lr, m),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn fz_is_point_inside_rect(p: fz_point, r: fz_rect) -> i32 {
    i32::from(p.x >= r.x0 && p.x < r.x1 && p.y >= r.y0 && p.y < r.y1)
}

#[unsafe(no_mangle)]
pub extern "C" fn fz_is_point_inside_irect(x: i32, y: i32, r: fz_irect) -> i32 {
    i32::from(x >= r.x0 && x < r.x1 && y >= r.y0 && y < r.y1)
}

/// Check if a rect is empty
#[unsafe(no_mangle)]
pub extern "C" fn fz_is_empty_rect(r: fz_rect) -> i32 {
    i32::from(r.x0 >= r.x1 || r.y0 >= r.y1)
}

/// Check if an irect is empty
#[unsafe(no_mangle)]
pub extern "C" fn fz_is_empty_irect(r: fz_irect) -> i32 {
    i32::from(r.x0 >= r.x1 || r.y0 >= r.y1)
}

/// Check if a rect is infinite
#[unsafe(no_mangle)]
pub extern "C" fn fz_is_infinite_rect(r: fz_rect) -> i32 {
    i32::from(r.x0 == FZ_MIN_INF_RECT as f32 && r.x1 == FZ_MAX_INF_RECT as f32)
}

/// Check if an irect is infinite
#[unsafe(no_mangle)]
pub extern "C" fn fz_is_infinite_irect(r: fz_irect) -> i32 {
    i32::from(r.x0 == FZ_MIN_INF_RECT && r.x1 == FZ_MAX_INF_RECT)
}

/// Get width of a rect
#[unsafe(no_mangle)]
pub extern "C" fn fz_rect_width(r: fz_rect) -> c_float {
    if r.x1 > r.x0 { r.x1 - r.x0 } else { 0.0 }
}

/// Get height of a rect
#[unsafe(no_mangle)]
pub extern "C" fn fz_rect_height(r: fz_rect) -> c_float {
    if r.y1 > r.y0 { r.y1 - r.y0 } else { 0.0 }
}

/// Get area of a rect
#[unsafe(no_mangle)]
pub extern "C" fn fz_rect_area(r: fz_rect) -> c_float {
    fz_rect_width(r) * fz_rect_height(r)
}

/// Get width of an irect
#[unsafe(no_mangle)]
pub extern "C" fn fz_irect_width(r: fz_irect) -> i32 {
    if r.x1 > r.x0 { r.x1 - r.x0 } else { 0 }
}

/// Get height of an irect
#[unsafe(no_mangle)]
pub extern "C" fn fz_irect_height(r: fz_irect) -> i32 {
    if r.y1 > r.y0 { r.y1 - r.y0 } else { 0 }
}

/// Get area of an irect
#[unsafe(no_mangle)]
pub extern "C" fn fz_irect_area(r: fz_irect) -> i32 {
    fz_irect_width(r) * fz_irect_height(r)
}

/// Get center point of a rect
#[unsafe(no_mangle)]
pub extern "C" fn fz_rect_center(r: fz_rect) -> fz_point {
    fz_point {
        x: (r.x0 + r.x1) / 2.0,
        y: (r.y0 + r.y1) / 2.0,
    }
}

/// Check if two rects are equal
#[unsafe(no_mangle)]
pub extern "C" fn fz_rect_eq(a: fz_rect, b: fz_rect) -> i32 {
    i32::from(a == b)
}

/// Check if two irect are equal
#[unsafe(no_mangle)]
pub extern "C" fn fz_irect_eq(a: fz_irect, b: fz_irect) -> i32 {
    i32::from(a == b)
}

/// Post-translate a matrix
#[unsafe(no_mangle)]
pub extern "C" fn fz_post_translate(m: fz_matrix, tx: c_float, ty: c_float) -> fz_matrix {
    fz_concat(m, fz_translate(tx, ty))
}

/// Post-rotate a matrix
#[unsafe(no_mangle)]
pub extern "C" fn fz_post_rotate(m: fz_matrix, degrees: c_float) -> fz_matrix {
    fz_concat(m, fz_rotate(degrees))
}

/// Calculate matrix determinant
#[unsafe(no_mangle)]
pub extern "C" fn fz_matrix_determinant(m: fz_matrix) -> c_float {
    m.a * m.d - m.b * m.c
}

/// Check if matrix is singular (non-invertible)
#[unsafe(no_mangle)]
pub extern "C" fn fz_matrix_is_singular(m: fz_matrix) -> i32 {
    let det = fz_matrix_determinant(m);
    i32::from(det.abs() < 1e-10)
}

/// Create a point
#[unsafe(no_mangle)]
pub extern "C" fn fz_make_point(x: c_float, y: c_float) -> fz_point {
    fz_point { x, y }
}

/// Create a rect
#[unsafe(no_mangle)]
pub extern "C" fn fz_make_rect(x0: c_float, y0: c_float, x1: c_float, y1: c_float) -> fz_rect {
    fz_rect { x0, y0, x1, y1 }
}

/// Create an irect
#[unsafe(no_mangle)]
pub extern "C" fn fz_make_irect(x0: i32, y0: i32, x1: i32, y1: i32) -> fz_irect {
    fz_irect { x0, y0, x1, y1 }
}

// ============================================================================
// Version
// ============================================================================

#[unsafe(no_mangle)]
pub extern "C" fn fz_version() -> *const std::ffi::c_char {
    c"0.1.0".as_ptr()
}

#[cfg(test)]
mod tests {
    use super::*;

    // Struct tests
    #[test]
    fn test_point_struct() {
        let p = fz_point { x: 3.5, y: 4.5 };
        assert_eq!(p.x, 3.5);
        assert_eq!(p.y, 4.5);
    }

    #[test]
    fn test_rect_struct() {
        let r = fz_rect {
            x0: 1.0,
            y0: 2.0,
            x1: 10.0,
            y1: 20.0,
        };
        assert_eq!(r.x0, 1.0);
        assert_eq!(r.y0, 2.0);
        assert_eq!(r.x1, 10.0);
        assert_eq!(r.y1, 20.0);
    }

    #[test]
    fn test_irect_struct() {
        let r = fz_irect {
            x0: 1,
            y0: 2,
            x1: 10,
            y1: 20,
        };
        assert_eq!(r.x0, 1);
        assert_eq!(r.y0, 2);
        assert_eq!(r.x1, 10);
        assert_eq!(r.y1, 20);
    }

    #[test]
    fn test_irect_from_rect() {
        let r = fz_rect {
            x0: 0.5,
            y0: 1.5,
            x1: 9.5,
            y1: 19.5,
        };
        let ir = fz_irect_from_rect(r);
        assert_eq!(ir.x0, 0);
        assert_eq!(ir.y0, 1);
        assert_eq!(ir.x1, 10);
        assert_eq!(ir.y1, 20);
    }

    #[test]
    fn test_round_rect() {
        let r = fz_rect {
            x0: 0.2,
            y0: 1.8,
            x1: 9.3,
            y1: 19.7,
        };
        let ir = fz_round_rect(r);
        // round_rect should round correctly
        assert!(ir.x0 >= 0);
        assert!(ir.x1 >= 9);
    }

    #[test]
    fn test_rect_from_irect() {
        let ir = fz_irect {
            x0: 1,
            y0: 2,
            x1: 10,
            y1: 20,
        };
        let r = fz_rect_from_irect(ir);
        assert_eq!(r.x0, 1.0);
        assert_eq!(r.y0, 2.0);
        assert_eq!(r.x1, 10.0);
        assert_eq!(r.y1, 20.0);
    }

    // Matrix tests
    #[test]
    fn test_matrix_identity() {
        let m = fz_matrix::identity();
        assert_eq!(m.a, 1.0);
        assert_eq!(m.d, 1.0);
        assert_eq!(m.b, 0.0);
        assert_eq!(m.c, 0.0);
        assert_eq!(m.e, 0.0);
        assert_eq!(m.f, 0.0);
    }

    #[test]
    fn test_matrix_struct() {
        let m = fz_matrix {
            a: 1.0,
            b: 2.0,
            c: 3.0,
            d: 4.0,
            e: 5.0,
            f: 6.0,
        };
        assert_eq!(m.a, 1.0);
        assert_eq!(m.b, 2.0);
        assert_eq!(m.c, 3.0);
        assert_eq!(m.d, 4.0);
        assert_eq!(m.e, 5.0);
        assert_eq!(m.f, 6.0);
    }

    #[test]
    fn test_translate() {
        let m = fz_translate(10.0, 20.0);
        assert_eq!(m.a, 1.0);
        assert_eq!(m.d, 1.0);
        assert_eq!(m.e, 10.0);
        assert_eq!(m.f, 20.0);
    }

    #[test]
    fn test_scale() {
        let m = fz_scale(2.0, 3.0);
        assert_eq!(m.a, 2.0);
        assert_eq!(m.d, 3.0);
        assert_eq!(m.e, 0.0);
        assert_eq!(m.f, 0.0);
    }

    #[test]
    fn test_rotate() {
        let m = fz_rotate(90.0);
        assert!((m.a - 0.0).abs() < 0.001);
        assert!((m.b - 1.0).abs() < 0.001);
        assert!((m.c - (-1.0)).abs() < 0.001);
        assert!((m.d - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_shear() {
        let m = fz_shear(0.5, 0.5);
        assert_eq!(m.a, 1.0);
        assert_eq!(m.b, 0.5);
        assert_eq!(m.c, 0.5);
        assert_eq!(m.d, 1.0);
    }

    #[test]
    fn test_pre_translate() {
        let m = fz_scale(2.0, 2.0);
        let result = fz_pre_translate(m, 10.0, 10.0);
        // Pre-translate: scale first, then translate
        assert_eq!(result.e, 20.0);
        assert_eq!(result.f, 20.0);
    }

    #[test]
    fn test_pre_scale() {
        let m = fz_translate(10.0, 10.0);
        let result = fz_pre_scale(m, 2.0, 2.0);
        assert_eq!(result.a, 2.0);
        assert_eq!(result.d, 2.0);
    }

    #[test]
    fn test_post_scale() {
        let m = fz_translate(10.0, 10.0);
        let result = fz_post_scale(m, 2.0, 2.0);
        assert_eq!(result.a, 2.0);
        assert_eq!(result.d, 2.0);
    }

    #[test]
    fn test_pre_rotate() {
        let m = fz_matrix::identity();
        let result = fz_pre_rotate(m, 90.0);
        assert!((result.b - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_pre_shear() {
        let m = fz_matrix::identity();
        let result = fz_pre_shear(m, 0.5, 0.5);
        assert_eq!(result.b, 0.5);
        assert_eq!(result.c, 0.5);
    }

    #[test]
    fn test_matrix_concat() {
        let m1 = fz_translate(10.0, 20.0);
        let m2 = fz_scale(2.0, 2.0);
        let m3 = fz_concat(m1, m2);

        let p = fz_point { x: 0.0, y: 0.0 };
        let result = fz_transform_point(p, m3);
        assert_eq!(result.x, 20.0);
        assert_eq!(result.y, 40.0);
    }

    #[test]
    fn test_invert_matrix() {
        let m = fz_scale(2.0, 2.0);
        let inv = fz_invert_matrix(m);
        // Inverse of scale(2,2) should be scale(0.5, 0.5)
        assert!((inv.a - 0.5).abs() < 0.001);
        assert!((inv.d - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_matrix_expansion() {
        let m = fz_scale(2.0, 3.0);
        let exp = fz_matrix_expansion(m);
        // Geometric mean of scale factors
        assert!(exp > 2.0);
        assert!(exp < 3.0);
    }

    #[test]
    fn test_matrix_max_expansion() {
        let m = fz_scale(2.0, 3.0);
        let max_exp = fz_matrix_max_expansion(m);
        assert_eq!(max_exp, 3.0);
    }

    #[test]
    fn test_is_rectilinear() {
        let identity = fz_matrix::identity();
        let rotated = fz_rotate(45.0);
        assert_eq!(fz_is_rectilinear(identity), 1);
        assert_eq!(fz_is_rectilinear(rotated), 0);
    }

    // Transform tests
    #[test]
    fn test_transform_point() {
        let m = fz_translate(10.0, 20.0);
        let p = fz_point { x: 5.0, y: 5.0 };
        let result = fz_transform_point(p, m);
        assert_eq!(result.x, 15.0);
        assert_eq!(result.y, 25.0);
    }

    #[test]
    fn test_transform_point_xy() {
        let m = fz_translate(10.0, 20.0);
        let result = fz_transform_point_xy(5.0, 5.0, m);
        assert_eq!(result.x, 15.0);
        assert_eq!(result.y, 25.0);
    }

    #[test]
    fn test_transform_vector() {
        let m = fz_translate(10.0, 20.0);
        let v = fz_point { x: 5.0, y: 5.0 };
        // Vector transform ignores translation
        let result = fz_transform_vector(v, m);
        assert_eq!(result.x, 5.0);
        assert_eq!(result.y, 5.0);
    }

    #[test]
    fn test_transform_rect() {
        let m = fz_scale(2.0, 2.0);
        let r = fz_rect {
            x0: 0.0,
            y0: 0.0,
            x1: 10.0,
            y1: 10.0,
        };
        let result = fz_transform_rect(r, m);
        assert_eq!(result.x0, 0.0);
        assert_eq!(result.y0, 0.0);
        assert_eq!(result.x1, 20.0);
        assert_eq!(result.y1, 20.0);
    }

    #[test]
    fn test_normalize_vector() {
        let p = fz_point { x: 3.0, y: 4.0 };
        let result = fz_normalize_vector(p);
        // Length should be 1
        let len = (result.x * result.x + result.y * result.y).sqrt();
        assert!((len - 1.0).abs() < 0.001);
    }

    // Rect operations tests
    #[test]
    fn test_rect_operations() {
        let r1 = fz_rect {
            x0: 0.0,
            y0: 0.0,
            x1: 100.0,
            y1: 100.0,
        };
        let r2 = fz_rect {
            x0: 50.0,
            y0: 50.0,
            x1: 150.0,
            y1: 150.0,
        };

        let intersection = fz_intersect_rect(r1, r2);
        assert_eq!(intersection.x0, 50.0);
        assert_eq!(intersection.y0, 50.0);
        assert_eq!(intersection.x1, 100.0);
        assert_eq!(intersection.y1, 100.0);

        let union = fz_union_rect(r1, r2);
        assert_eq!(union.x0, 0.0);
        assert_eq!(union.y0, 0.0);
        assert_eq!(union.x1, 150.0);
        assert_eq!(union.y1, 150.0);
    }

    #[test]
    fn test_intersect_irect() {
        let r1 = fz_irect {
            x0: 0,
            y0: 0,
            x1: 100,
            y1: 100,
        };
        let r2 = fz_irect {
            x0: 50,
            y0: 50,
            x1: 150,
            y1: 150,
        };
        let result = fz_intersect_irect(r1, r2);
        assert_eq!(result.x0, 50);
        assert_eq!(result.y0, 50);
        assert_eq!(result.x1, 100);
        assert_eq!(result.y1, 100);
    }

    #[test]
    fn test_expand_rect() {
        let r = fz_rect {
            x0: 10.0,
            y0: 10.0,
            x1: 20.0,
            y1: 20.0,
        };
        let expanded = fz_expand_rect(r, 5.0);
        assert_eq!(expanded.x0, 5.0);
        assert_eq!(expanded.y0, 5.0);
        assert_eq!(expanded.x1, 25.0);
        assert_eq!(expanded.y1, 25.0);
    }

    #[test]
    fn test_expand_irect() {
        let r = fz_irect {
            x0: 10,
            y0: 10,
            x1: 20,
            y1: 20,
        };
        let expanded = fz_expand_irect(r, 5);
        assert_eq!(expanded.x0, 5);
        assert_eq!(expanded.y0, 5);
        assert_eq!(expanded.x1, 25);
        assert_eq!(expanded.y1, 25);
    }

    #[test]
    fn test_include_point_in_rect() {
        let r = fz_rect {
            x0: 10.0,
            y0: 10.0,
            x1: 20.0,
            y1: 20.0,
        };
        let p = fz_point { x: 0.0, y: 30.0 };
        let result = fz_include_point_in_rect(r, p);
        assert_eq!(result.x0, 0.0);
        assert_eq!(result.y0, 10.0);
        assert_eq!(result.x1, 20.0);
        assert_eq!(result.y1, 30.0);
    }

    #[test]
    fn test_translate_rect() {
        let r = fz_rect {
            x0: 0.0,
            y0: 0.0,
            x1: 10.0,
            y1: 10.0,
        };
        let result = fz_translate_rect(r, 5.0, 5.0);
        assert_eq!(result.x0, 5.0);
        assert_eq!(result.y0, 5.0);
        assert_eq!(result.x1, 15.0);
        assert_eq!(result.y1, 15.0);
    }

    #[test]
    fn test_translate_irect() {
        let r = fz_irect {
            x0: 0,
            y0: 0,
            x1: 10,
            y1: 10,
        };
        let result = fz_translate_irect(r, 5, 5);
        assert_eq!(result.x0, 5);
        assert_eq!(result.y0, 5);
        assert_eq!(result.x1, 15);
        assert_eq!(result.y1, 15);
    }

    #[test]
    fn test_contains_rect() {
        let outer = fz_rect {
            x0: 0.0,
            y0: 0.0,
            x1: 100.0,
            y1: 100.0,
        };
        let inner = fz_rect {
            x0: 10.0,
            y0: 10.0,
            x1: 50.0,
            y1: 50.0,
        };
        let outside = fz_rect {
            x0: 200.0,
            y0: 200.0,
            x1: 300.0,
            y1: 300.0,
        };

        assert_eq!(fz_contains_rect(outer, inner), 1);
        assert_eq!(fz_contains_rect(outer, outside), 0);
    }

    #[test]
    fn test_overlaps_rect() {
        let r1 = fz_rect {
            x0: 0.0,
            y0: 0.0,
            x1: 50.0,
            y1: 50.0,
        };
        let r2 = fz_rect {
            x0: 25.0,
            y0: 25.0,
            x1: 75.0,
            y1: 75.0,
        };
        let r3 = fz_rect {
            x0: 100.0,
            y0: 100.0,
            x1: 150.0,
            y1: 150.0,
        };

        assert_eq!(fz_overlaps_rect(r1, r2), 1);
        assert_eq!(fz_overlaps_rect(r1, r3), 0);
    }

    // Quad tests
    #[test]
    fn test_quad_from_rect() {
        let r = fz_rect {
            x0: 0.0,
            y0: 0.0,
            x1: 10.0,
            y1: 20.0,
        };
        let q = fz_quad_from_rect(r);
        assert_eq!(q.ul.x, 0.0);
        assert_eq!(q.ul.y, 0.0);
        assert_eq!(q.ur.x, 10.0);
        assert_eq!(q.ur.y, 0.0);
        assert_eq!(q.ll.x, 0.0);
        assert_eq!(q.ll.y, 20.0);
        assert_eq!(q.lr.x, 10.0);
        assert_eq!(q.lr.y, 20.0);
    }

    #[test]
    fn test_rect_from_quad() {
        let q = fz_quad {
            ul: fz_point { x: 0.0, y: 0.0 },
            ur: fz_point { x: 10.0, y: 0.0 },
            ll: fz_point { x: 0.0, y: 20.0 },
            lr: fz_point { x: 10.0, y: 20.0 },
        };
        let r = fz_rect_from_quad(q);
        assert_eq!(r.x0, 0.0);
        assert_eq!(r.y0, 0.0);
        assert_eq!(r.x1, 10.0);
        assert_eq!(r.y1, 20.0);
    }

    #[test]
    fn test_transform_quad() {
        let q = fz_quad {
            ul: fz_point { x: 0.0, y: 0.0 },
            ur: fz_point { x: 10.0, y: 0.0 },
            ll: fz_point { x: 0.0, y: 10.0 },
            lr: fz_point { x: 10.0, y: 10.0 },
        };
        let m = fz_translate(5.0, 5.0);
        let result = fz_transform_quad(q, m);
        assert_eq!(result.ul.x, 5.0);
        assert_eq!(result.ul.y, 5.0);
        assert_eq!(result.lr.x, 15.0);
        assert_eq!(result.lr.y, 15.0);
    }

    // Point inside tests
    #[test]
    fn test_is_point_inside_rect() {
        let r = fz_rect {
            x0: 0.0,
            y0: 0.0,
            x1: 10.0,
            y1: 10.0,
        };
        let inside = fz_point { x: 5.0, y: 5.0 };
        let outside = fz_point { x: 15.0, y: 15.0 };
        assert_eq!(fz_is_point_inside_rect(inside, r), 1);
        assert_eq!(fz_is_point_inside_rect(outside, r), 0);
    }

    #[test]
    fn test_is_point_inside_irect() {
        let r = fz_irect {
            x0: 0,
            y0: 0,
            x1: 10,
            y1: 10,
        };
        assert_eq!(fz_is_point_inside_irect(5, 5, r), 1);
        assert_eq!(fz_is_point_inside_irect(15, 15, r), 0);
    }

    // Static values tests
    #[test]
    fn test_static_values() {
        assert_eq!(fz_identity.a, 1.0);
        assert_eq!(fz_identity.d, 1.0);

        // Empty rect uses infinity
        assert!(fz_empty_rect.x0.is_infinite());
        assert_eq!(fz_unit_rect.x1, 1.0);
    }

    // Version test
    #[test]
    fn test_version() {
        let version = fz_version();
        assert!(!version.is_null());
    }
}
