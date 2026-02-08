//! Geometry primitives - Point, Rect, Matrix, Quad

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    pub const ORIGIN: Point = Point { x: 0.0, y: 0.0 };

    #[inline]
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    #[inline]
    pub fn transform(&self, m: &Matrix) -> Self {
        Self {
            x: self.x * m.a + self.y * m.c + m.e,
            y: self.x * m.b + self.y * m.d + m.f,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Rect {
    pub x0: f32,
    pub y0: f32,
    pub x1: f32,
    pub y1: f32,
}

impl Rect {
    pub const EMPTY: Rect = Rect {
        x0: f32::INFINITY,
        y0: f32::INFINITY,
        x1: f32::NEG_INFINITY,
        y1: f32::NEG_INFINITY,
    };
    pub const INFINITE: Rect = Rect {
        x0: f32::NEG_INFINITY,
        y0: f32::NEG_INFINITY,
        x1: f32::INFINITY,
        y1: f32::INFINITY,
    };
    pub const UNIT: Rect = Rect {
        x0: 0.0,
        y0: 0.0,
        x1: 1.0,
        y1: 1.0,
    };

    #[inline]
    pub fn new(x0: f32, y0: f32, x1: f32, y1: f32) -> Self {
        Self { x0, y0, x1, y1 }
    }

    #[inline]
    pub fn width(&self) -> f32 {
        self.x1 - self.x0
    }

    #[inline]
    pub fn height(&self) -> f32 {
        self.y1 - self.y0
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.x0 >= self.x1 || self.y0 >= self.y1
    }

    #[inline]
    pub fn is_infinite(&self) -> bool {
        self.x0 == f32::NEG_INFINITY
    }

    #[inline]
    pub fn contains(&self, x: f32, y: f32) -> bool {
        x >= self.x0 && x < self.x1 && y >= self.y0 && y < self.y1
    }

    #[inline]
    pub fn union(&self, other: &Rect) -> Rect {
        Rect {
            x0: self.x0.min(other.x0),
            y0: self.y0.min(other.y0),
            x1: self.x1.max(other.x1),
            y1: self.y1.max(other.y1),
        }
    }

    #[inline]
    pub fn intersect(&self, other: &Rect) -> Rect {
        Rect {
            x0: self.x0.max(other.x0),
            y0: self.y0.max(other.y0),
            x1: self.x1.min(other.x1),
            y1: self.y1.min(other.y1),
        }
    }

    /// Check if two rectangles intersect
    #[inline]
    pub fn intersects(&self, other: &Rect) -> bool {
        self.x0 < other.x1 && self.x1 > other.x0 && self.y0 < other.y1 && self.y1 > other.y0
    }

    #[inline]
    pub fn include_point(&mut self, p: Point) {
        self.x0 = self.x0.min(p.x);
        self.y0 = self.y0.min(p.y);
        self.x1 = self.x1.max(p.x);
        self.y1 = self.y1.max(p.y);
    }

    /// Expand rectangle by a given amount in all directions
    pub fn expand(&self, amount: f32) -> Rect {
        Rect {
            x0: self.x0 - amount,
            y0: self.y0 - amount,
            x1: self.x1 + amount,
            y1: self.y1 + amount,
        }
    }

    /// Transform rectangle by a matrix
    pub fn transform(&self, m: &Matrix) -> Rect {
        if self.is_empty() {
            return *self;
        }

        // Transform all four corners
        let p0 = Point::new(self.x0, self.y0).transform(m);
        let p1 = Point::new(self.x1, self.y0).transform(m);
        let p2 = Point::new(self.x0, self.y1).transform(m);
        let p3 = Point::new(self.x1, self.y1).transform(m);

        // Find bounding box
        let mut result = Rect::EMPTY;
        result.include_point(p0);
        result.include_point(p1);
        result.include_point(p2);
        result.include_point(p3);
        result
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct IRect {
    pub x0: i32,
    pub y0: i32,
    pub x1: i32,
    pub y1: i32,
}

impl IRect {
    #[inline]
    pub fn new(x0: i32, y0: i32, x1: i32, y1: i32) -> Self {
        Self { x0, y0, x1, y1 }
    }

    #[inline]
    pub fn width(&self) -> i32 {
        self.x1 - self.x0
    }

    #[inline]
    pub fn height(&self) -> i32 {
        self.y1 - self.y0
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.x0 >= self.x1 || self.y0 >= self.y1
    }
}

impl From<Rect> for IRect {
    fn from(r: Rect) -> Self {
        IRect {
            x0: r.x0.floor() as i32,
            y0: r.y0.floor() as i32,
            x1: r.x1.ceil() as i32,
            y1: r.y1.ceil() as i32,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Matrix {
    pub a: f32,
    pub b: f32,
    pub c: f32,
    pub d: f32,
    pub e: f32,
    pub f: f32,
}

impl Default for Matrix {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Matrix {
    pub const IDENTITY: Matrix = Matrix {
        a: 1.0,
        b: 0.0,
        c: 0.0,
        d: 1.0,
        e: 0.0,
        f: 0.0,
    };

    #[inline]
    pub fn new(a: f32, b: f32, c: f32, d: f32, e: f32, f: f32) -> Self {
        Self { a, b, c, d, e, f }
    }

    #[inline]
    pub fn translate(tx: f32, ty: f32) -> Self {
        Self {
            a: 1.0,
            b: 0.0,
            c: 0.0,
            d: 1.0,
            e: tx,
            f: ty,
        }
    }

    #[inline]
    pub fn scale(sx: f32, sy: f32) -> Self {
        Self {
            a: sx,
            b: 0.0,
            c: 0.0,
            d: sy,
            e: 0.0,
            f: 0.0,
        }
    }

    #[inline]
    pub fn rotate(degrees: f32) -> Self {
        let rad = degrees * std::f32::consts::PI / 180.0;
        let (s, c) = (rad.sin(), rad.cos());
        Self {
            a: c,
            b: s,
            c: -s,
            d: c,
            e: 0.0,
            f: 0.0,
        }
    }

    #[inline]
    pub fn concat(&self, m: &Matrix) -> Self {
        Self {
            a: self.a * m.a + self.b * m.c,
            b: self.a * m.b + self.b * m.d,
            c: self.c * m.a + self.d * m.c,
            d: self.c * m.b + self.d * m.d,
            e: self.e * m.a + self.f * m.c + m.e,
            f: self.e * m.b + self.f * m.d + m.f,
        }
    }

    /// Transform a point by this matrix
    #[inline]
    pub fn transform_point(&self, p: Point) -> Point {
        Point {
            x: p.x * self.a + p.y * self.c + self.e,
            y: p.x * self.b + p.y * self.d + self.f,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Quad {
    pub ul: Point,
    pub ur: Point,
    pub ll: Point,
    pub lr: Point,
}

impl Quad {
    #[inline]
    pub fn from_rect(r: &Rect) -> Self {
        Self {
            ul: Point::new(r.x0, r.y0),
            ur: Point::new(r.x1, r.y0),
            ll: Point::new(r.x0, r.y1),
            lr: Point::new(r.x1, r.y1),
        }
    }

    #[inline]
    pub fn transform(&self, m: &Matrix) -> Self {
        Self {
            ul: self.ul.transform(m),
            ur: self.ur.transform(m),
            ll: self.ll.transform(m),
            lr: self.lr.transform(m),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Point tests
    #[test]
    fn test_point_origin() {
        assert_eq!(Point::ORIGIN.x, 0.0);
        assert_eq!(Point::ORIGIN.y, 0.0);
    }

    #[test]
    fn test_point_new() {
        let p = Point::new(3.0, 4.0);
        assert_eq!(p.x, 3.0);
        assert_eq!(p.y, 4.0);
    }

    #[test]
    fn test_point_transform_identity() {
        let p = Point::new(5.0, 10.0);
        let transformed = p.transform(&Matrix::IDENTITY);
        assert_eq!(transformed.x, 5.0);
        assert_eq!(transformed.y, 10.0);
    }

    #[test]
    fn test_point_transform_translate() {
        let p = Point::new(5.0, 10.0);
        let m = Matrix::translate(2.0, 3.0);
        let transformed = p.transform(&m);
        assert!((transformed.x - 7.0).abs() < 0.001);
        assert!((transformed.y - 13.0).abs() < 0.001);
    }

    #[test]
    fn test_point_transform_scale() {
        let p = Point::new(5.0, 10.0);
        let m = Matrix::scale(2.0, 3.0);
        let transformed = p.transform(&m);
        assert!((transformed.x - 10.0).abs() < 0.001);
        assert!((transformed.y - 30.0).abs() < 0.001);
    }

    #[test]
    fn test_point_default() {
        let p: Point = Default::default();
        assert_eq!(p.x, 0.0);
        assert_eq!(p.y, 0.0);
    }

    // Rect tests
    #[test]
    fn test_rect_new() {
        let r = Rect::new(1.0, 2.0, 10.0, 20.0);
        assert_eq!(r.x0, 1.0);
        assert_eq!(r.y0, 2.0);
        assert_eq!(r.x1, 10.0);
        assert_eq!(r.y1, 20.0);
    }

    #[test]
    fn test_rect_width_height() {
        let r = Rect::new(0.0, 0.0, 10.0, 20.0);
        assert_eq!(r.width(), 10.0);
        assert_eq!(r.height(), 20.0);
    }

    #[test]
    fn test_rect_is_empty() {
        assert!(Rect::EMPTY.is_empty());
        assert!(!Rect::UNIT.is_empty());
        assert!(Rect::new(10.0, 10.0, 5.0, 5.0).is_empty()); // inverted
    }

    #[test]
    fn test_rect_is_infinite() {
        assert!(Rect::INFINITE.is_infinite());
        assert!(!Rect::UNIT.is_infinite());
    }

    #[test]
    fn test_rect_contains() {
        let r = Rect::new(0.0, 0.0, 10.0, 10.0);
        assert!(r.contains(5.0, 5.0));
        assert!(r.contains(0.0, 0.0));
        assert!(!r.contains(10.0, 10.0)); // exclusive upper bound
        assert!(!r.contains(-1.0, 5.0));
        assert!(!r.contains(5.0, -1.0));
        assert!(!r.contains(11.0, 5.0));
        assert!(!r.contains(5.0, 11.0));
    }

    #[test]
    fn test_rect_union() {
        let r1 = Rect::new(0.0, 0.0, 5.0, 5.0);
        let r2 = Rect::new(3.0, 3.0, 10.0, 10.0);
        let u = r1.union(&r2);
        assert_eq!(u.x0, 0.0);
        assert_eq!(u.y0, 0.0);
        assert_eq!(u.x1, 10.0);
        assert_eq!(u.y1, 10.0);
    }

    #[test]
    fn test_rect_intersect() {
        let r1 = Rect::new(0.0, 0.0, 10.0, 10.0);
        let r2 = Rect::new(5.0, 5.0, 15.0, 15.0);
        let i = r1.intersect(&r2);
        assert_eq!(i.x0, 5.0);
        assert_eq!(i.y0, 5.0);
        assert_eq!(i.x1, 10.0);
        assert_eq!(i.y1, 10.0);
    }

    #[test]
    fn test_rect_include_point() {
        let mut r = Rect::EMPTY;
        r.include_point(Point::new(5.0, 5.0));
        r.include_point(Point::new(0.0, 0.0));
        r.include_point(Point::new(10.0, 10.0));
        assert_eq!(r.x0, 0.0);
        assert_eq!(r.y0, 0.0);
        assert_eq!(r.x1, 10.0);
        assert_eq!(r.y1, 10.0);
    }

    #[test]
    fn test_rect_constants() {
        assert!(Rect::EMPTY.is_empty());
        assert!(Rect::INFINITE.is_infinite());
        assert_eq!(Rect::UNIT.width(), 1.0);
        assert_eq!(Rect::UNIT.height(), 1.0);
    }

    // IRect tests
    #[test]
    fn test_irect_new() {
        let r = IRect::new(1, 2, 10, 20);
        assert_eq!(r.x0, 1);
        assert_eq!(r.y0, 2);
        assert_eq!(r.x1, 10);
        assert_eq!(r.y1, 20);
    }

    #[test]
    fn test_irect_width_height() {
        let r = IRect::new(0, 0, 10, 20);
        assert_eq!(r.width(), 10);
        assert_eq!(r.height(), 20);
    }

    #[test]
    fn test_irect_is_empty() {
        assert!(IRect::new(5, 5, 5, 5).is_empty());
        assert!(IRect::new(10, 10, 5, 5).is_empty());
        assert!(!IRect::new(0, 0, 10, 10).is_empty());
    }

    #[test]
    fn test_irect_from_rect() {
        let r = Rect::new(0.5, 1.5, 9.5, 19.5);
        let ir: IRect = r.into();
        assert_eq!(ir.x0, 0);
        assert_eq!(ir.y0, 1);
        assert_eq!(ir.x1, 10);
        assert_eq!(ir.y1, 20);
    }

    // Matrix tests
    #[test]
    fn test_matrix_identity() {
        let m = Matrix::IDENTITY;
        assert_eq!(m.a, 1.0);
        assert_eq!(m.b, 0.0);
        assert_eq!(m.c, 0.0);
        assert_eq!(m.d, 1.0);
        assert_eq!(m.e, 0.0);
        assert_eq!(m.f, 0.0);
    }

    #[test]
    fn test_matrix_new() {
        let m = Matrix::new(1.0, 2.0, 3.0, 4.0, 5.0, 6.0);
        assert_eq!(m.a, 1.0);
        assert_eq!(m.b, 2.0);
        assert_eq!(m.c, 3.0);
        assert_eq!(m.d, 4.0);
        assert_eq!(m.e, 5.0);
        assert_eq!(m.f, 6.0);
    }

    #[test]
    fn test_matrix_translate() {
        let m = Matrix::translate(10.0, 20.0);
        assert_eq!(m.e, 10.0);
        assert_eq!(m.f, 20.0);
        // Should be identity otherwise
        assert_eq!(m.a, 1.0);
        assert_eq!(m.d, 1.0);
    }

    #[test]
    fn test_matrix_scale() {
        let m = Matrix::scale(2.0, 3.0);
        assert_eq!(m.a, 2.0);
        assert_eq!(m.d, 3.0);
    }

    #[test]
    fn test_matrix_rotate() {
        let m = Matrix::rotate(90.0);
        // cos(90) ≈ 0, sin(90) = 1
        assert!((m.a - 0.0).abs() < 0.001);
        assert!((m.b - 1.0).abs() < 0.001);
        assert!((m.c - (-1.0)).abs() < 0.001);
        assert!((m.d - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_matrix_concat_identity() {
        let m1 = Matrix::scale(2.0, 3.0);
        let m2 = Matrix::IDENTITY;
        let result = m1.concat(&m2);
        assert_eq!(result.a, 2.0);
        assert_eq!(result.d, 3.0);
    }

    #[test]
    fn test_matrix_concat_scale_translate() {
        let scale = Matrix::scale(2.0, 2.0);
        let translate = Matrix::translate(10.0, 10.0);
        let result = scale.concat(&translate);
        // Scaling first, then translating
        assert_eq!(result.e, 10.0);
        assert_eq!(result.f, 10.0);
    }

    #[test]
    fn test_matrix_default() {
        let m: Matrix = Default::default();
        assert_eq!(m, Matrix::IDENTITY);
    }

    // Quad tests
    #[test]
    fn test_quad_from_rect() {
        let r = Rect::new(0.0, 0.0, 10.0, 20.0);
        let q = Quad::from_rect(&r);
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
    fn test_quad_transform() {
        let r = Rect::new(0.0, 0.0, 10.0, 10.0);
        let q = Quad::from_rect(&r);
        let m = Matrix::translate(5.0, 5.0);
        let transformed = q.transform(&m);
        assert_eq!(transformed.ul.x, 5.0);
        assert_eq!(transformed.ul.y, 5.0);
        assert_eq!(transformed.lr.x, 15.0);
        assert_eq!(transformed.lr.y, 15.0);
    }

    #[test]
    fn test_quad_default() {
        let q: Quad = Default::default();
        assert_eq!(q.ul, Point::default());
        assert_eq!(q.ur, Point::default());
        assert_eq!(q.ll, Point::default());
        assert_eq!(q.lr, Point::default());
    }
}
