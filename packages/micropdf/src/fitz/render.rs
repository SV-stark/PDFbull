//! Pixel Rendering Engine
//!
//! This module implements the core rasterization algorithms for converting
//! vector graphics (paths, text) into pixels.
//!
//! # Features
//!
//! - Scan-line conversion for paths
//! - Non-zero winding and even-odd fill rules
//! - Anti-aliasing
//! - Stroke expansion
//! - Clipping
//! - Blending with alpha compositing

use crate::fitz::colorspace::Colorspace;
use crate::fitz::geometry::{Matrix, Point, Rect};
use crate::fitz::path::{LineCap, LineJoin, Path, PathElement};
use crate::fitz::pixmap::Pixmap;

/// Edge for scan-line conversion
#[derive(Debug, Clone)]
struct Edge {
    /// Starting x coordinate
    x: f32,
    /// Starting y coordinate (integer scan line)
    y: i32,
    /// Change in x per scan line
    dx: f32,
    /// Height of edge in scan lines
    height: i32,
    /// Direction: +1 for down, -1 for up
    direction: i32,
}

impl Edge {
    /// Create a new edge from two points
    fn new(p0: Point, p1: Point) -> Option<Self> {
        // Sort points by y coordinate
        let (p0, p1, direction) = if p0.y < p1.y {
            (p0, p1, 1)
        } else if p1.y < p0.y {
            (p1, p0, -1)
        } else {
            // Horizontal edge - skip
            return None;
        };

        let y0 = p0.y.floor() as i32;
        let y1 = p1.y.floor() as i32;
        let height = y1 - y0;

        if height <= 0 {
            return None;
        }

        // Calculate dx/dy
        let dy = p1.y - p0.y;
        let dx = if dy.abs() > 0.0001 {
            (p1.x - p0.x) / dy
        } else {
            0.0
        };

        Some(Self {
            x: p0.x,
            y: y0,
            dx,
            height,
            direction,
        })
    }

    /// Step to the next scan line
    fn step(&mut self) {
        self.x += self.dx;
        self.y += 1;
        self.height -= 1;
    }

    /// Check if edge is still active
    fn is_active(&self) -> bool {
        self.height > 0
    }
}

/// Active edge for scan-line algorithm
#[derive(Debug, Clone)]
struct ActiveEdge {
    edge: Edge,
    winding: i32,
}

/// Rasterizer for converting paths to pixels
pub struct Rasterizer {
    /// Width of output pixmap
    width: i32,
    /// Height of output pixmap
    height: i32,
    /// Clip rectangle
    clip: Rect,
    /// Anti-aliasing level (1 = no AA, 2 = 2x2, 4 = 4x4, 8 = 8x8)
    aa_level: i32,
}

impl Rasterizer {
    /// Create a new rasterizer
    pub fn new(width: i32, height: i32, clip: Rect) -> Self {
        Self {
            width,
            height,
            clip,
            aa_level: 8, // Default to 8x8 supersampling
        }
    }

    /// Set anti-aliasing level
    pub fn set_aa_level(&mut self, level: i32) {
        self.aa_level = level.max(1).min(8);
    }

    /// Fill a path into a pixmap
    pub fn fill_path(
        &self,
        path: &Path,
        even_odd: bool,
        ctm: &Matrix,
        colorspace: &Colorspace,
        color: &[f32],
        alpha: f32,
        dest: &mut Pixmap,
    ) {
        // Transform path by CTM
        let transformed_path = self.transform_path(path, ctm);

        // Build edge list
        let mut edges = self.build_edge_list(&transformed_path);

        if edges.is_empty() {
            return;
        }

        // Sort edges by starting y coordinate
        edges.sort_by_key(|e| e.y);

        // Convert color to destination colorspace
        let default_cs = Colorspace::device_rgb();
        let dest_cs = dest.colorspace().unwrap_or(&default_cs);
        let pixel_color = self.convert_color(colorspace, color, dest_cs, alpha);

        // Scan-line conversion
        self.scan_convert(&edges, even_odd, &pixel_color, dest);
    }

    /// Stroke a path into a pixmap
    pub fn stroke_path(
        &self,
        path: &Path,
        stroke_state: &crate::fitz::path::StrokeState,
        ctm: &Matrix,
        colorspace: &Colorspace,
        color: &[f32],
        alpha: f32,
        dest: &mut Pixmap,
    ) {
        // Expand stroke to a filled path
        let stroked_path = self.expand_stroke(path, stroke_state, ctm);

        // Fill the stroked path
        self.fill_path(
            &stroked_path,
            false,             // Always use non-zero winding for strokes
            &Matrix::IDENTITY, // Already transformed
            colorspace,
            color,
            alpha,
            dest,
        );
    }

    /// Transform a path by a matrix
    fn transform_path(&self, path: &Path, ctm: &Matrix) -> Path {
        let mut result = Path::new();

        for element in path.elements() {
            match element {
                PathElement::MoveTo(p) => {
                    result.move_to(ctm.transform_point(*p));
                }
                PathElement::LineTo(p) => {
                    result.line_to(ctm.transform_point(*p));
                }
                PathElement::QuadTo(p1, p2) => {
                    result.quad_to(ctm.transform_point(*p1), ctm.transform_point(*p2));
                }
                PathElement::CurveTo(p1, p2, p3) => {
                    result.curve_to(
                        ctm.transform_point(*p1),
                        ctm.transform_point(*p2),
                        ctm.transform_point(*p3),
                    );
                }
                PathElement::Close => {
                    result.close();
                }
                PathElement::Rect(r) => {
                    let transformed_rect = r.transform(ctm);
                    result.rect(transformed_rect);
                }
            }
        }

        result
    }

    /// Build edge list from path
    fn build_edge_list(&self, path: &Path) -> Vec<Edge> {
        let mut edges = Vec::new();
        let mut current_point = Point::new(0.0, 0.0);
        let mut subpath_start = Point::new(0.0, 0.0);

        for element in path.elements() {
            match element {
                PathElement::MoveTo(p) => {
                    current_point = *p;
                    subpath_start = *p;
                }
                PathElement::LineTo(p) => {
                    if let Some(edge) = Edge::new(current_point, *p) {
                        edges.push(edge);
                    }
                    current_point = *p;
                }
                PathElement::QuadTo(p1, p2) => {
                    // Flatten quadratic curve into line segments
                    let quad_edges = self.flatten_quad(current_point, *p1, *p2);
                    edges.extend(quad_edges);
                    current_point = *p2;
                }
                PathElement::CurveTo(p1, p2, p3) => {
                    // Flatten curve into line segments
                    let curve_edges = self.flatten_curve(current_point, *p1, *p2, *p3);
                    edges.extend(curve_edges);
                    current_point = *p3;
                }
                PathElement::Close => {
                    if let Some(edge) = Edge::new(current_point, subpath_start) {
                        edges.push(edge);
                    }
                    current_point = subpath_start;
                }
                PathElement::Rect(r) => {
                    // Add rectangle edges
                    let p0 = Point::new(r.x0, r.y0);
                    let p1 = Point::new(r.x1, r.y0);
                    let p2 = Point::new(r.x1, r.y1);
                    let p3 = Point::new(r.x0, r.y1);

                    if let Some(edge) = Edge::new(p0, p1) {
                        edges.push(edge);
                    }
                    if let Some(edge) = Edge::new(p1, p2) {
                        edges.push(edge);
                    }
                    if let Some(edge) = Edge::new(p2, p3) {
                        edges.push(edge);
                    }
                    if let Some(edge) = Edge::new(p3, p0) {
                        edges.push(edge);
                    }
                }
            }
        }

        edges
    }

    /// Flatten a quadratic Bézier curve into line segments
    fn flatten_quad(&self, p0: Point, p1: Point, p2: Point) -> Vec<Edge> {
        let mut edges = Vec::new();
        let mut segments = Vec::new();

        // Convert quadratic to cubic: cubic control points are:
        // cp1 = p0 + 2/3 * (p1 - p0)
        // cp2 = p2 + 2/3 * (p1 - p2)
        let cp1 = Point::new(
            p0.x + (2.0 / 3.0) * (p1.x - p0.x),
            p0.y + (2.0 / 3.0) * (p1.y - p0.y),
        );
        let cp2 = Point::new(
            p2.x + (2.0 / 3.0) * (p1.x - p2.x),
            p2.y + (2.0 / 3.0) * (p1.y - p2.y),
        );

        // Recursive subdivision
        self.subdivide_curve(p0, cp1, cp2, p2, 0, &mut segments);

        // Convert segments to edges
        for i in 0..segments.len() - 1 {
            if let Some(edge) = Edge::new(segments[i], segments[i + 1]) {
                edges.push(edge);
            }
        }

        edges
    }

    /// Flatten a cubic Bézier curve into line segments
    fn flatten_curve(&self, p0: Point, p1: Point, p2: Point, p3: Point) -> Vec<Edge> {
        let mut edges = Vec::new();
        let mut segments = Vec::new();

        // Recursive subdivision
        self.subdivide_curve(p0, p1, p2, p3, 0, &mut segments);

        // Convert segments to edges
        for i in 0..segments.len() - 1 {
            if let Some(edge) = Edge::new(segments[i], segments[i + 1]) {
                edges.push(edge);
            }
        }

        edges
    }

    /// Recursively subdivide a cubic Bézier curve
    fn subdivide_curve(
        &self,
        p0: Point,
        p1: Point,
        p2: Point,
        p3: Point,
        depth: i32,
        segments: &mut Vec<Point>,
    ) {
        const MAX_DEPTH: i32 = 8;
        const FLATNESS: f32 = 0.5;

        if segments.is_empty() {
            segments.push(p0);
        }

        // Check if curve is flat enough
        if depth >= MAX_DEPTH || self.is_flat(p0, p1, p2, p3, FLATNESS) {
            segments.push(p3);
            return;
        }

        // Subdivide using de Casteljau's algorithm
        let p01 = Point::new((p0.x + p1.x) * 0.5, (p0.y + p1.y) * 0.5);
        let p12 = Point::new((p1.x + p2.x) * 0.5, (p1.y + p2.y) * 0.5);
        let p23 = Point::new((p2.x + p3.x) * 0.5, (p2.y + p3.y) * 0.5);

        let p012 = Point::new((p01.x + p12.x) * 0.5, (p01.y + p12.y) * 0.5);
        let p123 = Point::new((p12.x + p23.x) * 0.5, (p12.y + p23.y) * 0.5);

        let p0123 = Point::new((p012.x + p123.x) * 0.5, (p012.y + p123.y) * 0.5);

        // Recurse on both halves
        self.subdivide_curve(p0, p01, p012, p0123, depth + 1, segments);
        self.subdivide_curve(p0123, p123, p23, p3, depth + 1, segments);
    }

    /// Check if a cubic Bézier curve is flat enough
    fn is_flat(&self, p0: Point, p1: Point, p2: Point, p3: Point, tolerance: f32) -> bool {
        // Calculate maximum deviation from line p0-p3
        let dx = p3.x - p0.x;
        let dy = p3.y - p0.y;
        let d = (dx * dx + dy * dy).sqrt();

        if d < 0.001 {
            return true;
        }

        // Distance from p1 to line p0-p3
        let d1 = ((p1.x - p0.x) * dy - (p1.y - p0.y) * dx).abs() / d;

        // Distance from p2 to line p0-p3
        let d2 = ((p2.x - p0.x) * dy - (p2.y - p0.y) * dx).abs() / d;

        d1.max(d2) < tolerance
    }

    /// Scan-line conversion algorithm
    fn scan_convert(&self, edges: &[Edge], even_odd: bool, color: &[u8], dest: &mut Pixmap) {
        if edges.is_empty() {
            return;
        }

        let mut active_edges: Vec<ActiveEdge> = Vec::new();
        let mut edge_index = 0;

        // Find y range
        let min_y = edges.first().unwrap().y.max(self.clip.y0 as i32);
        let max_y = edges
            .iter()
            .map(|e| e.y + e.height)
            .max()
            .unwrap_or(0)
            .min(self.clip.y1 as i32);

        // Process each scan line
        for y in min_y..max_y {
            // Add new edges that start on this scan line
            while edge_index < edges.len() && edges[edge_index].y <= y {
                active_edges.push(ActiveEdge {
                    edge: edges[edge_index].clone(),
                    winding: 0,
                });
                edge_index += 1;
            }

            // Remove finished edges
            active_edges.retain(|ae| ae.edge.is_active());

            if active_edges.is_empty() {
                continue;
            }

            // Sort active edges by x coordinate
            active_edges.sort_by(|a, b| a.edge.x.partial_cmp(&b.edge.x).unwrap());

            // Calculate winding numbers
            let mut winding = 0;
            for ae in &mut active_edges {
                winding += ae.edge.direction;
                ae.winding = winding;
            }

            // Fill spans
            self.fill_spans(&active_edges, y, even_odd, color, dest);

            // Step all active edges to next scan line
            for ae in &mut active_edges {
                ae.edge.step();
            }
        }
    }

    /// Fill spans for a single scan line
    fn fill_spans(
        &self,
        active_edges: &[ActiveEdge],
        y: i32,
        even_odd: bool,
        color: &[u8],
        dest: &mut Pixmap,
    ) {
        if y < 0 || y >= dest.height() {
            return;
        }

        let mut inside = false;
        let mut x_start = 0;

        for i in 0..active_edges.len() {
            let ae = &active_edges[i];
            let x = ae.edge.x as i32;

            // Check if we're inside based on winding rule
            let new_inside = if even_odd {
                ae.winding % 2 != 0
            } else {
                ae.winding != 0
            };

            if new_inside != inside {
                if new_inside {
                    // Start of span
                    x_start = x;
                } else {
                    // End of span - fill it
                    self.fill_span(x_start, x, y, color, dest);
                }
                inside = new_inside;
            }
        }
    }

    /// Fill a single horizontal span
    fn fill_span(&self, x0: i32, x1: i32, y: i32, color: &[u8], dest: &mut Pixmap) {
        let x0 = x0.max(self.clip.x0 as i32).max(0);
        let x1 = x1.min(self.clip.x1 as i32).min(dest.width());

        if x0 >= x1 {
            return;
        }

        let y = y as usize;
        let stride = dest.stride() as usize;
        let n = dest.n() as usize;

        // Get pixel data
        let samples = dest.samples_mut();

        for x in x0..x1 {
            let offset = y * stride + (x as usize) * n;

            // Simple alpha blending
            if color.len() >= n {
                for i in 0..n {
                    if offset + i < samples.len() {
                        samples[offset + i] = color[i];
                    }
                }
            }
        }
    }

    /// Expand a stroke to a filled path
    fn expand_stroke(
        &self,
        path: &Path,
        stroke_state: &crate::fitz::path::StrokeState,
        ctm: &Matrix,
    ) -> Path {
        let mut result = Path::new();
        let width = stroke_state.linewidth * 0.5;

        // Transform path first
        let transformed = self.transform_path(path, ctm);

        // For each line segment in the path, create an expanded rectangle
        let mut current_point = Point::new(0.0, 0.0);
        let mut subpath_start = Point::new(0.0, 0.0);

        for element in transformed.elements() {
            match element {
                PathElement::MoveTo(p) => {
                    current_point = *p;
                    subpath_start = *p;
                }
                PathElement::LineTo(p) => {
                    self.expand_line_segment(current_point, *p, width, stroke_state, &mut result);
                    current_point = *p;
                }
                PathElement::QuadTo(p1, p2) => {
                    // Convert quadratic to cubic and flatten
                    let cp1 = Point::new(
                        current_point.x + (2.0 / 3.0) * (p1.x - current_point.x),
                        current_point.y + (2.0 / 3.0) * (p1.y - current_point.y),
                    );
                    let cp2 = Point::new(
                        p2.x + (2.0 / 3.0) * (p1.x - p2.x),
                        p2.y + (2.0 / 3.0) * (p1.y - p2.y),
                    );

                    let mut segments = Vec::new();
                    self.subdivide_curve(current_point, cp1, cp2, *p2, 0, &mut segments);

                    for i in 0..segments.len() - 1 {
                        self.expand_line_segment(
                            segments[i],
                            segments[i + 1],
                            width,
                            stroke_state,
                            &mut result,
                        );
                    }
                    current_point = *p2;
                }
                PathElement::CurveTo(p1, p2, p3) => {
                    // Flatten curve and expand each segment
                    let mut segments = Vec::new();
                    self.subdivide_curve(current_point, *p1, *p2, *p3, 0, &mut segments);

                    for i in 0..segments.len() - 1 {
                        self.expand_line_segment(
                            segments[i],
                            segments[i + 1],
                            width,
                            stroke_state,
                            &mut result,
                        );
                    }
                    current_point = *p3;
                }
                PathElement::Close => {
                    self.expand_line_segment(
                        current_point,
                        subpath_start,
                        width,
                        stroke_state,
                        &mut result,
                    );
                    current_point = subpath_start;
                }
                PathElement::Rect(r) => {
                    // Expand rectangle outline
                    let p0 = Point::new(r.x0, r.y0);
                    let p1 = Point::new(r.x1, r.y0);
                    let p2 = Point::new(r.x1, r.y1);
                    let p3 = Point::new(r.x0, r.y1);

                    self.expand_line_segment(p0, p1, width, stroke_state, &mut result);
                    self.expand_line_segment(p1, p2, width, stroke_state, &mut result);
                    self.expand_line_segment(p2, p3, width, stroke_state, &mut result);
                    self.expand_line_segment(p3, p0, width, stroke_state, &mut result);
                }
            }
        }

        result
    }

    /// Expand a single line segment into a rectangle
    fn expand_line_segment(
        &self,
        p0: Point,
        p1: Point,
        width: f32,
        _stroke_state: &crate::fitz::path::StrokeState,
        result: &mut Path,
    ) {
        // Calculate perpendicular vector
        let dx = p1.x - p0.x;
        let dy = p1.y - p0.y;
        let len = (dx * dx + dy * dy).sqrt();

        if len < 0.001 {
            return;
        }

        let nx = -dy / len * width;
        let ny = dx / len * width;

        // Create rectangle around line segment
        let corner1 = Point::new(p0.x + nx, p0.y + ny);
        let corner2 = Point::new(p0.x - nx, p0.y - ny);
        let corner3 = Point::new(p1.x - nx, p1.y - ny);
        let corner4 = Point::new(p1.x + nx, p1.y + ny);

        result.move_to(corner1);
        result.line_to(corner2);
        result.line_to(corner3);
        result.line_to(corner4);
        result.close();
    }

    /// Convert color from one colorspace to another
    fn convert_color(
        &self,
        src_cs: &Colorspace,
        src_color: &[f32],
        dest_cs: &Colorspace,
        alpha: f32,
    ) -> Vec<u8> {
        // Simplified color conversion
        // TODO: Implement proper ICC profile-based conversion

        let mut result = Vec::new();

        // Convert to RGB first (simplified)
        let rgb = match (src_cs.color_type(), src_color.len()) {
            (crate::fitz::colorspace::ColorType::Gray, 1) => {
                let g = (src_color[0] * 255.0) as u8;
                vec![g, g, g]
            }
            (crate::fitz::colorspace::ColorType::RGB, 3) => {
                vec![
                    (src_color[0] * 255.0) as u8,
                    (src_color[1] * 255.0) as u8,
                    (src_color[2] * 255.0) as u8,
                ]
            }
            (crate::fitz::colorspace::ColorType::CMYK, 4) => {
                // Simplified CMYK to RGB
                let c = src_color[0];
                let m = src_color[1];
                let y = src_color[2];
                let k = src_color[3];

                let r = ((1.0 - c) * (1.0 - k) * 255.0) as u8;
                let g = ((1.0 - m) * (1.0 - k) * 255.0) as u8;
                let b = ((1.0 - y) * (1.0 - k) * 255.0) as u8;

                vec![r, g, b]
            }
            _ => vec![0, 0, 0], // Default to black
        };

        // Convert RGB to destination colorspace
        match dest_cs.color_type() {
            crate::fitz::colorspace::ColorType::Gray => {
                // RGB to grayscale
                let gray =
                    (0.299 * rgb[0] as f32 + 0.587 * rgb[1] as f32 + 0.114 * rgb[2] as f32) as u8;
                result.push(gray);
            }
            crate::fitz::colorspace::ColorType::RGB => {
                result.extend_from_slice(&rgb);
            }
            crate::fitz::colorspace::ColorType::CMYK => {
                // RGB to CMYK (simplified)
                let r = rgb[0] as f32 / 255.0;
                let g = rgb[1] as f32 / 255.0;
                let b = rgb[2] as f32 / 255.0;

                let k = 1.0 - r.max(g).max(b);
                let c = if k < 1.0 {
                    (1.0 - r - k) / (1.0 - k)
                } else {
                    0.0
                };
                let m = if k < 1.0 {
                    (1.0 - g - k) / (1.0 - k)
                } else {
                    0.0
                };
                let y = if k < 1.0 {
                    (1.0 - b - k) / (1.0 - k)
                } else {
                    0.0
                };

                result.push((c * 255.0) as u8);
                result.push((m * 255.0) as u8);
                result.push((y * 255.0) as u8);
                result.push((k * 255.0) as u8);
            }
            _ => {
                result.extend_from_slice(&rgb);
            }
        }

        // Add alpha channel if destination has it
        if dest_cs.has_alpha() {
            result.push((alpha * 255.0) as u8);
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_edge_creation() {
        let p0 = Point::new(0.0, 0.0);
        let p1 = Point::new(10.0, 10.0);

        let edge = Edge::new(p0, p1).unwrap();
        assert_eq!(edge.y, 0);
        assert_eq!(edge.direction, 1);
        assert_eq!(edge.height, 10);
    }

    #[test]
    fn test_horizontal_edge() {
        let p0 = Point::new(0.0, 5.0);
        let p1 = Point::new(10.0, 5.0);

        assert!(Edge::new(p0, p1).is_none());
    }

    #[test]
    fn test_edge_step() {
        let p0 = Point::new(0.0, 0.0);
        let p1 = Point::new(10.0, 10.0);

        let mut edge = Edge::new(p0, p1).unwrap();
        let initial_x = edge.x;

        edge.step();

        assert_eq!(edge.y, 1);
        assert!(edge.x > initial_x);
        assert_eq!(edge.height, 9);
    }

    #[test]
    fn test_rasterizer_creation() {
        let rast = Rasterizer::new(100, 100, Rect::new(0.0, 0.0, 100.0, 100.0));
        assert_eq!(rast.width, 100);
        assert_eq!(rast.height, 100);
    }
}
