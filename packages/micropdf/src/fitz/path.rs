//! Vector paths - MuPDF Compatible
//!
//! Vector path operations for stroking and filling.

use crate::fitz::geometry::{Point, Rect};

/// Path element types
#[derive(Debug, Clone, PartialEq)]
pub enum PathElement {
    /// Move to a point (starts a new subpath)
    MoveTo(Point),
    /// Line to a point
    LineTo(Point),
    /// Quadratic Bezier curve
    QuadTo(Point, Point),
    /// Cubic Bezier curve
    CurveTo(Point, Point, Point),
    /// Close the current subpath
    Close,
    /// Rectangle (optimization for axis-aligned rectangles)
    Rect(Rect),
}

/// Line cap styles
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineCap {
    /// Butt cap (square at endpoint)
    Butt = 0,
    /// Round cap (semicircle at endpoint)
    Round = 1,
    /// Square cap (extends past endpoint)
    Square = 2,
    /// Triangle cap
    Triangle = 3,
}

/// Line join styles
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineJoin {
    /// Miter join (extends to point)
    Miter = 0,
    /// Round join (arc between lines)
    Round = 1,
    /// Bevel join (straight line)
    Bevel = 2,
    /// Miter join with XPS behavior
    MiterXPS = 3,
}

/// Stroke state for path rendering
#[derive(Debug, Clone)]
pub struct StrokeState {
    /// Line width
    pub linewidth: f32,
    /// Miter limit (for miter joins)
    pub miterlimit: f32,
    /// Start line cap
    pub start_cap: LineCap,
    /// Dash line cap
    pub dash_cap: LineCap,
    /// End line cap
    pub end_cap: LineCap,
    /// Line join style
    pub linejoin: LineJoin,
    /// Dash pattern phase offset
    pub dash_phase: f32,
    /// Dash pattern (alternating on/off lengths)
    pub dash_pattern: Vec<f32>,
}

impl StrokeState {
    /// Create a new stroke state with default values
    pub fn new() -> Self {
        Self {
            linewidth: 1.0,
            miterlimit: 10.0,
            start_cap: LineCap::Butt,
            dash_cap: LineCap::Butt,
            end_cap: LineCap::Butt,
            linejoin: LineJoin::Miter,
            dash_phase: 0.0,
            dash_pattern: Vec::new(),
        }
    }

    /// Check if this is a dashed stroke
    pub fn is_dashed(&self) -> bool {
        !self.dash_pattern.is_empty()
    }
}

impl Default for StrokeState {
    fn default() -> Self {
        Self::new()
    }
}

/// Vector path
#[derive(Debug)]
pub struct Path {
    elements: Vec<PathElement>,
}

impl Path {
    /// Create a new empty path
    pub fn new() -> Self {
        Self {
            elements: Vec::new(),
        }
    }

    /// Create a path with a specific capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            elements: Vec::with_capacity(capacity),
        }
    }

    /// Move to a point (starts a new subpath)
    pub fn move_to(&mut self, p: Point) {
        self.elements.push(PathElement::MoveTo(p));
    }

    /// Line to a point
    pub fn line_to(&mut self, p: Point) {
        self.elements.push(PathElement::LineTo(p));
    }

    /// Quadratic Bezier curve
    pub fn quad_to(&mut self, p1: Point, p2: Point) {
        self.elements.push(PathElement::QuadTo(p1, p2));
    }

    /// Cubic Bezier curve
    pub fn curve_to(&mut self, p1: Point, p2: Point, p3: Point) {
        self.elements.push(PathElement::CurveTo(p1, p2, p3));
    }

    /// Close the current subpath
    pub fn close(&mut self) {
        self.elements.push(PathElement::Close);
    }

    /// Add a rectangle
    pub fn rect(&mut self, r: Rect) {
        self.elements.push(PathElement::Rect(r));
    }

    /// Add a rectangle by coordinates
    pub fn rect_coords(&mut self, x0: f32, y0: f32, x1: f32, y1: f32) {
        self.rect(Rect::new(x0, y0, x1, y1));
    }

    /// Get the bounding box of the path
    pub fn bounds(&self) -> Rect {
        let mut bbox = Rect::EMPTY;
        for el in &self.elements {
            match el {
                PathElement::MoveTo(p) | PathElement::LineTo(p) => bbox.include_point(*p),
                PathElement::QuadTo(p1, p2) => {
                    bbox.include_point(*p1);
                    bbox.include_point(*p2);
                }
                PathElement::CurveTo(p1, p2, p3) => {
                    bbox.include_point(*p1);
                    bbox.include_point(*p2);
                    bbox.include_point(*p3);
                }
                PathElement::Rect(r) => {
                    bbox = bbox.union(r);
                }
                PathElement::Close => {}
            }
        }
        bbox
    }

    /// Get the number of path elements
    pub fn len(&self) -> usize {
        self.elements.len()
    }

    /// Check if the path is empty
    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }

    /// Get the path elements
    pub fn elements(&self) -> &[PathElement] {
        &self.elements
    }

    /// Clear all elements from the path
    pub fn clear(&mut self) {
        self.elements.clear();
    }

    /// Get the current point (last point in path)
    pub fn current_point(&self) -> Option<Point> {
        // Iterate backwards to find the last point
        for element in self.elements.iter().rev() {
            match element {
                PathElement::MoveTo(p) | PathElement::LineTo(p) => return Some(*p),
                PathElement::QuadTo(_, p2) => return Some(*p2),
                PathElement::CurveTo(_, _, p3) => return Some(*p3),
                PathElement::Rect(r) => return Some(Point::new(r.x1, r.y1)),
                PathElement::Close => continue,
            }
        }
        None
    }

    /// Clone the path
    pub fn clone_path(&self) -> Self {
        Self {
            elements: self.elements.clone(),
        }
    }

    /// Walk the path, calling callbacks for each element
    pub fn walk<F>(&self, mut walker: F)
    where
        F: FnMut(&PathElement),
    {
        for element in &self.elements {
            walker(element);
        }
    }

    /// Transform the path by applying a function to each point
    pub fn transform<F>(&mut self, mut transform: F)
    where
        F: FnMut(Point) -> Point,
    {
        for element in &mut self.elements {
            match element {
                PathElement::MoveTo(p) | PathElement::LineTo(p) => {
                    *p = transform(*p);
                }
                PathElement::QuadTo(p1, p2) => {
                    *p1 = transform(*p1);
                    *p2 = transform(*p2);
                }
                PathElement::CurveTo(p1, p2, p3) => {
                    *p1 = transform(*p1);
                    *p2 = transform(*p2);
                    *p3 = transform(*p3);
                }
                PathElement::Rect(r) => {
                    let p0 = transform(Point::new(r.x0, r.y0));
                    let p1 = transform(Point::new(r.x1, r.y1));
                    *r = Rect::new(p0.x, p0.y, p1.x, p1.y);
                }
                PathElement::Close => {}
            }
        }
    }

    /// Check if the path contains only rectangles
    pub fn is_rect_only(&self) -> bool {
        self.elements
            .iter()
            .all(|e| matches!(e, PathElement::Rect(_)))
    }
}
impl Default for Path {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for Path {
    fn clone(&self) -> Self {
        self.clone_path()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_new() {
        let path = Path::new();
        assert!(path.is_empty());
        assert_eq!(path.len(), 0);
    }

    #[test]
    fn test_path_with_capacity() {
        let path = Path::with_capacity(100);
        assert!(path.is_empty());
    }

    #[test]
    fn test_path_default() {
        let path: Path = Default::default();
        assert!(path.is_empty());
    }

    #[test]
    fn test_path_move_to() {
        let mut path = Path::new();
        path.move_to(Point::new(10.0, 20.0));
        assert_eq!(path.len(), 1);
        assert!(!path.is_empty());
    }

    #[test]
    fn test_path_line_to() {
        let mut path = Path::new();
        path.move_to(Point::new(0.0, 0.0));
        path.line_to(Point::new(10.0, 10.0));
        assert_eq!(path.len(), 2);
    }

    #[test]
    fn test_path_quad_to() {
        let mut path = Path::new();
        path.move_to(Point::new(0.0, 0.0));
        path.quad_to(Point::new(5.0, 10.0), Point::new(10.0, 0.0));
        assert_eq!(path.len(), 2);
    }

    #[test]
    fn test_path_curve_to() {
        let mut path = Path::new();
        path.move_to(Point::new(0.0, 0.0));
        path.curve_to(
            Point::new(5.0, 10.0),
            Point::new(15.0, 10.0),
            Point::new(20.0, 0.0),
        );
        assert_eq!(path.len(), 2);
    }

    #[test]
    fn test_path_close() {
        let mut path = Path::new();
        path.move_to(Point::new(0.0, 0.0));
        path.line_to(Point::new(10.0, 0.0));
        path.line_to(Point::new(10.0, 10.0));
        path.close();
        assert_eq!(path.len(), 4);
    }

    #[test]
    fn test_path_rect() {
        let mut path = Path::new();
        path.rect(Rect::new(0.0, 0.0, 100.0, 50.0));
        assert_eq!(path.len(), 1);
    }

    #[test]
    fn test_path_rect_coords() {
        let mut path = Path::new();
        path.rect_coords(0.0, 0.0, 100.0, 50.0);
        assert_eq!(path.len(), 1);
    }

    #[test]
    fn test_path_bounds_simple() {
        let mut path = Path::new();
        path.move_to(Point::new(0.0, 0.0));
        path.line_to(Point::new(100.0, 50.0));

        let bounds = path.bounds();
        assert_eq!(bounds.x0, 0.0);
        assert_eq!(bounds.y0, 0.0);
        assert_eq!(bounds.x1, 100.0);
        assert_eq!(bounds.y1, 50.0);
    }

    #[test]
    fn test_path_bounds_with_rect() {
        let mut path = Path::new();
        path.rect(Rect::new(10.0, 10.0, 50.0, 50.0));

        let bounds = path.bounds();
        assert_eq!(bounds, Rect::new(10.0, 10.0, 50.0, 50.0));
    }

    #[test]
    fn test_path_clear() {
        let mut path = Path::new();
        path.move_to(Point::new(0.0, 0.0));
        path.line_to(Point::new(10.0, 10.0));
        assert_eq!(path.len(), 2);

        path.clear();
        assert!(path.is_empty());
    }

    #[test]
    fn test_path_clone() {
        let mut path = Path::new();
        path.move_to(Point::new(0.0, 0.0));
        path.line_to(Point::new(10.0, 10.0));

        let cloned = path.clone();
        assert_eq!(cloned.len(), path.len());
    }

    #[test]
    fn test_path_walk() {
        let mut path = Path::new();
        path.move_to(Point::new(0.0, 0.0));
        path.line_to(Point::new(10.0, 10.0));
        path.close();

        let mut count = 0;
        path.walk(|_| count += 1);
        assert_eq!(count, 3);
    }

    #[test]
    fn test_path_transform() {
        let mut path = Path::new();
        path.move_to(Point::new(0.0, 0.0));
        path.line_to(Point::new(10.0, 10.0));

        // Scale by 2
        path.transform(|p| Point::new(p.x * 2.0, p.y * 2.0));

        let bounds = path.bounds();
        assert_eq!(bounds.x1, 20.0);
        assert_eq!(bounds.y1, 20.0);
    }

    #[test]
    fn test_path_is_rect_only() {
        let mut path1 = Path::new();
        path1.rect(Rect::new(0.0, 0.0, 10.0, 10.0));
        assert!(path1.is_rect_only());

        let mut path2 = Path::new();
        path2.move_to(Point::new(0.0, 0.0));
        path2.line_to(Point::new(10.0, 10.0));
        assert!(!path2.is_rect_only());
    }

    #[test]
    fn test_stroke_state_new() {
        let stroke = StrokeState::new();
        assert_eq!(stroke.linewidth, 1.0);
        assert_eq!(stroke.miterlimit, 10.0);
        assert_eq!(stroke.start_cap, LineCap::Butt);
        assert_eq!(stroke.linejoin, LineJoin::Miter);
        assert!(!stroke.is_dashed());
    }

    #[test]
    fn test_stroke_state_default() {
        let stroke: StrokeState = Default::default();
        assert_eq!(stroke.linewidth, 1.0);
    }

    #[test]
    fn test_stroke_state_dashed() {
        let mut stroke = StrokeState::new();
        assert!(!stroke.is_dashed());

        stroke.dash_pattern = vec![5.0, 3.0];
        assert!(stroke.is_dashed());
    }

    #[test]
    fn test_line_cap_values() {
        assert_eq!(LineCap::Butt as i32, 0);
        assert_eq!(LineCap::Round as i32, 1);
        assert_eq!(LineCap::Square as i32, 2);
        assert_eq!(LineCap::Triangle as i32, 3);
    }

    #[test]
    fn test_line_join_values() {
        assert_eq!(LineJoin::Miter as i32, 0);
        assert_eq!(LineJoin::Round as i32, 1);
        assert_eq!(LineJoin::Bevel as i32, 2);
        assert_eq!(LineJoin::MiterXPS as i32, 3);
    }

    #[test]
    fn test_path_element_equality() {
        let e1 = PathElement::MoveTo(Point::new(0.0, 0.0));
        let e2 = PathElement::MoveTo(Point::new(0.0, 0.0));
        let e3 = PathElement::LineTo(Point::new(0.0, 0.0));

        assert_eq!(e1, e2);
        assert_ne!(e1, e3);
    }

    #[test]
    fn test_path_elements_access() {
        let mut path = Path::new();
        path.move_to(Point::new(0.0, 0.0));
        path.line_to(Point::new(10.0, 10.0));

        let elements = path.elements();
        assert_eq!(elements.len(), 2);
    }

    #[test]
    fn test_path_bounds_with_curve() {
        let mut path = Path::new();
        path.move_to(Point::new(0.0, 0.0));
        path.curve_to(
            Point::new(50.0, 100.0),
            Point::new(100.0, 100.0),
            Point::new(150.0, 0.0),
        );

        let bounds = path.bounds();
        assert_eq!(bounds.x0, 0.0);
        assert_eq!(bounds.y0, 0.0);
        assert_eq!(bounds.x1, 150.0);
        assert_eq!(bounds.y1, 100.0);
    }

    #[test]
    fn test_path_bounds_empty() {
        let path = Path::new();
        let bounds = path.bounds();
        assert!(bounds.is_empty());
    }

    #[test]
    fn test_path_bounds_with_close() {
        let mut path = Path::new();
        path.move_to(Point::new(0.0, 0.0));
        path.line_to(Point::new(10.0, 10.0));
        path.close();

        let bounds = path.bounds();
        assert_eq!(bounds.x0, 0.0);
        assert_eq!(bounds.y0, 0.0);
        assert_eq!(bounds.x1, 10.0);
        assert_eq!(bounds.y1, 10.0);
    }

    #[test]
    fn test_path_rectangle() {
        let mut path = Path::new();
        path.move_to(Point::new(0.0, 0.0));
        path.line_to(Point::new(100.0, 0.0));
        path.line_to(Point::new(100.0, 50.0));
        path.line_to(Point::new(0.0, 50.0));
        path.close();

        let bounds = path.bounds();
        assert_eq!(bounds.width(), 100.0);
        assert_eq!(bounds.height(), 50.0);
    }
}
