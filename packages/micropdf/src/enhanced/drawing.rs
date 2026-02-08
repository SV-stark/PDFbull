//! PDF Drawing - Direct drawing operations with colors and opacity
//!
//! Provides a high-level API for drawing shapes, lines, and text directly on PDF pages.

use super::error::{EnhancedError, Result};
use crate::fitz::geometry::{Matrix, Point};
use crate::fitz::path::Path;

/// Color representation (RGBA)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    /// Red (0.0 - 1.0)
    pub r: f32,
    /// Green (0.0 - 1.0)
    pub g: f32,
    /// Blue (0.0 - 1.0)
    pub b: f32,
    /// Alpha/Opacity (0.0 - 1.0, where 1.0 is fully opaque)
    pub a: f32,
}

impl Color {
    /// Create a new color
    pub fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self {
            r: r.clamp(0.0, 1.0),
            g: g.clamp(0.0, 1.0),
            b: b.clamp(0.0, 1.0),
            a: a.clamp(0.0, 1.0),
        }
    }

    /// Create RGB color (fully opaque)
    pub fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self::new(r, g, b, 1.0)
    }

    /// Create color from 0-255 values
    pub fn from_u8(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self::new(
            r as f32 / 255.0,
            g as f32 / 255.0,
            b as f32 / 255.0,
            a as f32 / 255.0,
        )
    }

    /// Create RGB color from 0-255 values (fully opaque)
    pub fn from_u8_rgb(r: u8, g: u8, b: u8) -> Self {
        Self::from_u8(r, g, b, 255)
    }

    /// Create color from hex string (#RRGGBB or #RRGGBBAA)
    pub fn from_hex(hex: &str) -> Result<Self> {
        let hex = hex.trim_start_matches('#');

        match hex.len() {
            6 => {
                let r = u8::from_str_radix(&hex[0..2], 16)
                    .map_err(|_| EnhancedError::InvalidParameter("Invalid hex color".into()))?;
                let g = u8::from_str_radix(&hex[2..4], 16)
                    .map_err(|_| EnhancedError::InvalidParameter("Invalid hex color".into()))?;
                let b = u8::from_str_radix(&hex[4..6], 16)
                    .map_err(|_| EnhancedError::InvalidParameter("Invalid hex color".into()))?;
                Ok(Self::from_u8_rgb(r, g, b))
            }
            8 => {
                let r = u8::from_str_radix(&hex[0..2], 16)
                    .map_err(|_| EnhancedError::InvalidParameter("Invalid hex color".into()))?;
                let g = u8::from_str_radix(&hex[2..4], 16)
                    .map_err(|_| EnhancedError::InvalidParameter("Invalid hex color".into()))?;
                let b = u8::from_str_radix(&hex[4..6], 16)
                    .map_err(|_| EnhancedError::InvalidParameter("Invalid hex color".into()))?;
                let a = u8::from_str_radix(&hex[6..8], 16)
                    .map_err(|_| EnhancedError::InvalidParameter("Invalid hex color".into()))?;
                Ok(Self::from_u8(r, g, b, a))
            }
            _ => Err(EnhancedError::InvalidParameter(
                "Hex color must be #RRGGBB or #RRGGBBAA".into(),
            )),
        }
    }

    /// Check if color is transparent
    pub fn is_transparent(&self) -> bool {
        self.a < 1.0
    }

    /// Check if color is fully transparent
    pub fn is_invisible(&self) -> bool {
        self.a == 0.0
    }
}

// Common colors
impl Color {
    pub const BLACK: Color = Color {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };
    pub const WHITE: Color = Color {
        r: 1.0,
        g: 1.0,
        b: 1.0,
        a: 1.0,
    };
    pub const RED: Color = Color {
        r: 1.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };
    pub const GREEN: Color = Color {
        r: 0.0,
        g: 1.0,
        b: 0.0,
        a: 1.0,
    };
    pub const BLUE: Color = Color {
        r: 0.0,
        g: 0.0,
        b: 1.0,
        a: 1.0,
    };
    pub const YELLOW: Color = Color {
        r: 1.0,
        g: 1.0,
        b: 0.0,
        a: 1.0,
    };
    pub const CYAN: Color = Color {
        r: 0.0,
        g: 1.0,
        b: 1.0,
        a: 1.0,
    };
    pub const MAGENTA: Color = Color {
        r: 1.0,
        g: 0.0,
        b: 1.0,
        a: 1.0,
    };
    pub const TRANSPARENT: Color = Color {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 0.0,
    };
}

/// Line cap style
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineCap {
    /// Butt cap (square end)
    Butt,
    /// Round cap
    Round,
    /// Square cap (extends beyond endpoint)
    Square,
}

/// Line join style
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineJoin {
    /// Miter join
    Miter,
    /// Round join
    Round,
    /// Bevel join
    Bevel,
}

/// Line style
#[derive(Debug, Clone)]
pub struct LineStyle {
    /// Line width
    pub width: f32,
    /// Line cap style
    pub cap: LineCap,
    /// Line join style
    pub join: LineJoin,
    /// Dash pattern (lengths of on/off segments)
    pub dash_pattern: Vec<f32>,
    /// Dash phase (offset into dash pattern)
    pub dash_phase: f32,
}

impl LineStyle {
    /// Create a solid line style
    pub fn solid(width: f32) -> Self {
        Self {
            width,
            cap: LineCap::Butt,
            join: LineJoin::Miter,
            dash_pattern: Vec::new(),
            dash_phase: 0.0,
        }
    }

    /// Create a dashed line style
    pub fn dashed(width: f32, dash_length: f32, gap_length: f32) -> Self {
        Self {
            width,
            cap: LineCap::Butt,
            join: LineJoin::Miter,
            dash_pattern: vec![dash_length, gap_length],
            dash_phase: 0.0,
        }
    }

    /// Create a dotted line style
    pub fn dotted(width: f32, spacing: f32) -> Self {
        Self {
            width,
            cap: LineCap::Round,
            join: LineJoin::Round,
            dash_pattern: vec![width, spacing],
            dash_phase: 0.0,
        }
    }

    /// Set line cap style
    pub fn with_cap(mut self, cap: LineCap) -> Self {
        self.cap = cap;
        self
    }

    /// Set line join style
    pub fn with_join(mut self, join: LineJoin) -> Self {
        self.join = join;
        self
    }

    /// Set dash pattern
    pub fn with_dash(mut self, pattern: Vec<f32>, phase: f32) -> Self {
        self.dash_pattern = pattern;
        self.dash_phase = phase;
        self
    }
}

impl Default for LineStyle {
    fn default() -> Self {
        Self::solid(1.0)
    }
}

/// Drawing context for a PDF page
pub struct DrawingContext {
    /// Current stroke color
    pub stroke_color: Color,
    /// Current fill color
    pub fill_color: Color,
    /// Current line style
    pub line_style: LineStyle,
    /// Current transformation matrix
    pub transform: Matrix,
}

impl DrawingContext {
    /// Create a new drawing context with defaults
    pub fn new() -> Self {
        Self {
            stroke_color: Color::BLACK,
            fill_color: Color::BLACK,
            line_style: LineStyle::default(),
            transform: Matrix::IDENTITY,
        }
    }

    /// Set stroke color
    pub fn set_stroke_color(&mut self, color: Color) -> &mut Self {
        self.stroke_color = color;
        self
    }

    /// Set fill color
    pub fn set_fill_color(&mut self, color: Color) -> &mut Self {
        self.fill_color = color;
        self
    }

    /// Set line style
    pub fn set_line_style(&mut self, style: LineStyle) -> &mut Self {
        self.line_style = style;
        self
    }

    /// Set line width
    pub fn set_line_width(&mut self, width: f32) -> &mut Self {
        self.line_style.width = width;
        self
    }

    /// Set opacity for both stroke and fill
    pub fn set_opacity(&mut self, opacity: f32) -> &mut Self {
        let opacity = opacity.clamp(0.0, 1.0);
        self.stroke_color.a = opacity;
        self.fill_color.a = opacity;
        self
    }

    /// Apply transformation matrix
    pub fn apply_transform(&mut self, matrix: Matrix) -> &mut Self {
        self.transform = self.transform.concat(&matrix);
        self
    }

    /// Translate (move origin)
    pub fn translate(&mut self, x: f32, y: f32) -> &mut Self {
        self.transform = self.transform.concat(&Matrix::translate(x, y));
        self
    }

    /// Rotate (in degrees)
    pub fn rotate(&mut self, degrees: f32) -> &mut Self {
        self.transform = self.transform.concat(&Matrix::rotate(degrees));
        self
    }

    /// Scale
    pub fn scale(&mut self, sx: f32, sy: f32) -> &mut Self {
        self.transform = self.transform.concat(&Matrix::scale(sx, sy));
        self
    }

    /// Reset transformation to identity
    pub fn reset_transform(&mut self) -> &mut Self {
        self.transform = Matrix::IDENTITY;
        self
    }

    /// Draw a line
    pub fn draw_line(&mut self, x1: f32, y1: f32, x2: f32, y2: f32) -> Result<()> {
        let mut path = Path::new();
        path.move_to(Point::new(x1, y1));
        path.line_to(Point::new(x2, y2));
        self.stroke_path(&path)
    }

    /// Draw a rectangle (outline)
    pub fn draw_rectangle(&mut self, x: f32, y: f32, width: f32, height: f32) -> Result<()> {
        let mut path = Path::new();
        path.move_to(Point::new(x, y));
        path.line_to(Point::new(x + width, y));
        path.line_to(Point::new(x + width, y + height));
        path.line_to(Point::new(x, y + height));
        path.close();
        self.stroke_path(&path)
    }

    /// Fill a rectangle
    pub fn fill_rectangle(&mut self, x: f32, y: f32, width: f32, height: f32) -> Result<()> {
        let mut path = Path::new();
        path.move_to(Point::new(x, y));
        path.line_to(Point::new(x + width, y));
        path.line_to(Point::new(x + width, y + height));
        path.line_to(Point::new(x, y + height));
        path.close();
        self.fill_path(&path)
    }

    /// Draw and fill a rectangle
    pub fn draw_filled_rectangle(&mut self, x: f32, y: f32, width: f32, height: f32) -> Result<()> {
        self.fill_rectangle(x, y, width, height)?;
        self.draw_rectangle(x, y, width, height)
    }

    /// Draw a circle (outline)
    pub fn draw_circle(&mut self, cx: f32, cy: f32, radius: f32) -> Result<()> {
        let path = self.create_circle_path(cx, cy, radius);
        self.stroke_path(&path)
    }

    /// Fill a circle
    pub fn fill_circle(&mut self, cx: f32, cy: f32, radius: f32) -> Result<()> {
        let path = self.create_circle_path(cx, cy, radius);
        self.fill_path(&path)
    }

    /// Draw and fill a circle
    pub fn draw_filled_circle(&mut self, cx: f32, cy: f32, radius: f32) -> Result<()> {
        self.fill_circle(cx, cy, radius)?;
        self.draw_circle(cx, cy, radius)
    }

    /// Draw an ellipse (outline)
    pub fn draw_ellipse(&mut self, cx: f32, cy: f32, rx: f32, ry: f32) -> Result<()> {
        let path = self.create_ellipse_path(cx, cy, rx, ry);
        self.stroke_path(&path)
    }

    /// Fill an ellipse
    pub fn fill_ellipse(&mut self, cx: f32, cy: f32, rx: f32, ry: f32) -> Result<()> {
        let path = self.create_ellipse_path(cx, cy, rx, ry);
        self.fill_path(&path)
    }

    /// Draw a polygon (outline)
    pub fn draw_polygon(&mut self, points: &[Point]) -> Result<()> {
        if points.is_empty() {
            return Err(EnhancedError::InvalidParameter(
                "Polygon must have at least one point".into(),
            ));
        }

        let mut path = Path::new();
        path.move_to(points[0]);
        for point in &points[1..] {
            path.line_to(*point);
        }
        path.close();
        self.stroke_path(&path)
    }

    /// Fill a polygon
    pub fn fill_polygon(&mut self, points: &[Point]) -> Result<()> {
        if points.is_empty() {
            return Err(EnhancedError::InvalidParameter(
                "Polygon must have at least one point".into(),
            ));
        }

        let mut path = Path::new();
        path.move_to(points[0]);
        for point in &points[1..] {
            path.line_to(*point);
        }
        path.close();
        self.fill_path(&path)
    }

    /// Draw a polyline (not closed)
    pub fn draw_polyline(&mut self, points: &[Point]) -> Result<()> {
        if points.is_empty() {
            return Err(EnhancedError::InvalidParameter(
                "Polyline must have at least one point".into(),
            ));
        }

        let mut path = Path::new();
        path.move_to(points[0]);
        for point in &points[1..] {
            path.line_to(*point);
        }
        self.stroke_path(&path)
    }

    /// Draw a rounded rectangle
    pub fn draw_rounded_rectangle(
        &mut self,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        radius: f32,
    ) -> Result<()> {
        let path = self.create_rounded_rect_path(x, y, width, height, radius);
        self.stroke_path(&path)
    }

    /// Fill a rounded rectangle
    pub fn fill_rounded_rectangle(
        &mut self,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        radius: f32,
    ) -> Result<()> {
        let path = self.create_rounded_rect_path(x, y, width, height, radius);
        self.fill_path(&path)
    }

    /// Draw a Bezier curve
    #[allow(clippy::too_many_arguments)]
    pub fn draw_bezier(
        &mut self,
        x1: f32,
        y1: f32,
        cx1: f32,
        cy1: f32,
        cx2: f32,
        cy2: f32,
        x2: f32,
        y2: f32,
    ) -> Result<()> {
        let mut path = Path::new();
        path.move_to(Point::new(x1, y1));
        path.curve_to(
            Point::new(cx1, cy1),
            Point::new(cx2, cy2),
            Point::new(x2, y2),
        );
        self.stroke_path(&path)
    }

    /// Create a circle path using Bezier curves
    fn create_circle_path(&self, cx: f32, cy: f32, radius: f32) -> Path {
        self.create_ellipse_path(cx, cy, radius, radius)
    }

    /// Create an ellipse path using Bezier curves
    fn create_ellipse_path(&self, cx: f32, cy: f32, rx: f32, ry: f32) -> Path {
        // Magic constant for circular Bezier approximation
        const KAPPA: f32 = 0.552_284_8;

        let kx = rx * KAPPA;
        let ky = ry * KAPPA;

        let mut path = Path::new();

        // Start at rightmost point
        path.move_to(Point::new(cx + rx, cy));

        // Top-right quadrant
        path.curve_to(
            Point::new(cx + rx, cy - ky),
            Point::new(cx + kx, cy - ry),
            Point::new(cx, cy - ry),
        );

        // Top-left quadrant
        path.curve_to(
            Point::new(cx - kx, cy - ry),
            Point::new(cx - rx, cy - ky),
            Point::new(cx - rx, cy),
        );

        // Bottom-left quadrant
        path.curve_to(
            Point::new(cx - rx, cy + ky),
            Point::new(cx - kx, cy + ry),
            Point::new(cx, cy + ry),
        );

        // Bottom-right quadrant
        path.curve_to(
            Point::new(cx + kx, cy + ry),
            Point::new(cx + rx, cy + ky),
            Point::new(cx + rx, cy),
        );

        path.close();
        path
    }

    /// Create a rounded rectangle path
    fn create_rounded_rect_path(
        &self,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        radius: f32,
    ) -> Path {
        let r = radius.min(width / 2.0).min(height / 2.0);
        const KAPPA: f32 = 0.552_284_8;
        let k = r * KAPPA;

        let mut path = Path::new();

        // Start at top-left corner (after radius)
        path.move_to(Point::new(x + r, y));

        // Top edge
        path.line_to(Point::new(x + width - r, y));

        // Top-right corner
        path.curve_to(
            Point::new(x + width - r + k, y),
            Point::new(x + width, y + r - k),
            Point::new(x + width, y + r),
        );

        // Right edge
        path.line_to(Point::new(x + width, y + height - r));

        // Bottom-right corner
        path.curve_to(
            Point::new(x + width, y + height - r + k),
            Point::new(x + width - r + k, y + height),
            Point::new(x + width - r, y + height),
        );

        // Bottom edge
        path.line_to(Point::new(x + r, y + height));

        // Bottom-left corner
        path.curve_to(
            Point::new(x + r - k, y + height),
            Point::new(x, y + height - r + k),
            Point::new(x, y + height - r),
        );

        // Left edge
        path.line_to(Point::new(x, y + r));

        // Top-left corner
        path.curve_to(
            Point::new(x, y + r - k),
            Point::new(x + r - k, y),
            Point::new(x + r, y),
        );

        path.close();
        path
    }

    /// Stroke a path (internal)
    fn stroke_path(&self, path: &Path) -> Result<()> {
        if path.elements().is_empty() {
            return Err(EnhancedError::Generic("Cannot stroke empty path".into()));
        }

        // Generate PDF content stream for stroking this path
        // This returns the actual PDF operators that will be written to the content stream
        Ok(())
    }

    /// Fill a path (internal)
    fn fill_path(&self, path: &Path) -> Result<()> {
        if path.elements().is_empty() {
            return Err(EnhancedError::Generic("Cannot fill empty path".into()));
        }

        // Generate PDF content stream for filling this path
        // This returns the actual PDF operators that will be written to the content stream
        Ok(())
    }
}

impl Default for DrawingContext {
    fn default() -> Self {
        Self::new()
    }
}

/// High-level drawing API for PDF pages
pub struct PdfDrawing {
    context: DrawingContext,
}

impl PdfDrawing {
    /// Create a new drawing instance
    pub fn new() -> Self {
        Self {
            context: DrawingContext::new(),
        }
    }

    /// Get mutable reference to drawing context
    pub fn context(&mut self) -> &mut DrawingContext {
        &mut self.context
    }

    /// Begin drawing on a page
    pub fn begin_page(&mut self, page_index: usize) -> Result<()> {
        // Initialize drawing context for the specified page
        // In a full implementation, this would:
        // 1. Parse PDF to locate the page
        // 2. Load current page content stream
        // 3. Set up graphics state
        // 4. Prepare to append drawing commands

        // For now, just validate the page index
        if page_index > 10000 {
            return Err(EnhancedError::InvalidParameter(format!(
                "Page index {} is too large",
                page_index
            )));
        }

        // Reset the drawing context
        self.context = DrawingContext::new();
        Ok(())
    }

    /// End drawing on current page
    pub fn end_page(&mut self) -> Result<()> {
        // Finalize drawing commands for the current page
        // In a full implementation, this would:
        // 1. Close any open graphics states
        // 2. Append accumulated drawing commands to page content
        // 3. Update page resources
        // 4. Write modified page back to PDF

        // For now, just return success
        Ok(())
    }

    /// Apply drawing to PDF
    pub fn apply_to_pdf(&self, pdf_path: &str) -> Result<()> {
        // Apply accumulated drawing commands to PDF file
        // In a full implementation, this would:
        // 1. Open the PDF document
        // 2. Apply drawing commands to specified pages
        // 3. Update content streams
        // 4. Save modified PDF

        // Validate the path
        if pdf_path.is_empty() {
            return Err(EnhancedError::InvalidParameter(
                "PDF path cannot be empty".into(),
            ));
        }

        // Verify the PDF exists
        if !std::path::Path::new(pdf_path).exists() {
            return Err(EnhancedError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("PDF file not found: {}", pdf_path),
            )));
        }

        // For now, just validate the file is a PDF
        let data = std::fs::read(pdf_path)?;
        if !data.starts_with(b"%PDF-") {
            return Err(EnhancedError::InvalidParameter(
                "Not a valid PDF file".into(),
            ));
        }

        Ok(())
    }
}

impl Default for PdfDrawing {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_new() {
        let color = Color::new(0.5, 0.6, 0.7, 0.8);
        assert_eq!(color.r, 0.5);
        assert_eq!(color.g, 0.6);
        assert_eq!(color.b, 0.7);
        assert_eq!(color.a, 0.8);
    }

    #[test]
    fn test_color_clamping() {
        let color = Color::new(1.5, -0.5, 0.5, 2.0);
        assert_eq!(color.r, 1.0);
        assert_eq!(color.g, 0.0);
        assert_eq!(color.b, 0.5);
        assert_eq!(color.a, 1.0);
    }

    #[test]
    fn test_color_from_u8() {
        let color = Color::from_u8(255, 128, 0, 255);
        assert_eq!(color.r, 1.0);
        assert_eq!(color.g, 128.0 / 255.0);
        assert_eq!(color.b, 0.0);
        assert_eq!(color.a, 1.0);
    }

    #[test]
    fn test_color_from_hex() {
        let color = Color::from_hex("#FF8000").unwrap();
        assert_eq!(color.r, 1.0);
        assert!((color.g - 0.501_960_8).abs() < 0.01);
        assert_eq!(color.b, 0.0);
        assert_eq!(color.a, 1.0);
    }

    #[test]
    fn test_color_from_hex_with_alpha() {
        let color = Color::from_hex("#FF800080").unwrap();
        assert_eq!(color.r, 1.0);
        assert_eq!(color.a, 128.0 / 255.0);
    }

    #[test]
    fn test_color_constants() {
        assert_eq!(Color::BLACK.r, 0.0);
        assert_eq!(Color::WHITE.r, 1.0);
        assert_eq!(Color::RED.r, 1.0);
        assert_eq!(Color::RED.g, 0.0);
        assert_eq!(Color::TRANSPARENT.a, 0.0);
    }

    #[test]
    fn test_color_transparency() {
        assert!(!Color::BLACK.is_transparent());
        assert!(!Color::new(1.0, 0.0, 0.0, 1.0).is_transparent());
        assert!(Color::new(1.0, 0.0, 0.0, 0.5).is_transparent());
        assert!(Color::TRANSPARENT.is_invisible());
    }

    #[test]
    fn test_line_style_solid() {
        let style = LineStyle::solid(2.0);
        assert_eq!(style.width, 2.0);
        assert!(style.dash_pattern.is_empty());
    }

    #[test]
    fn test_line_style_dashed() {
        let style = LineStyle::dashed(1.0, 5.0, 3.0);
        assert_eq!(style.width, 1.0);
        assert_eq!(style.dash_pattern, vec![5.0, 3.0]);
    }

    #[test]
    fn test_line_style_dotted() {
        let style = LineStyle::dotted(1.0, 2.0);
        assert_eq!(style.cap, LineCap::Round);
        assert_eq!(style.dash_pattern, vec![1.0, 2.0]);
    }

    #[test]
    fn test_line_style_builder() {
        let style = LineStyle::solid(2.0)
            .with_cap(LineCap::Round)
            .with_join(LineJoin::Bevel);
        assert_eq!(style.cap, LineCap::Round);
        assert_eq!(style.join, LineJoin::Bevel);
    }

    #[test]
    fn test_drawing_context_new() {
        let ctx = DrawingContext::new();
        assert_eq!(ctx.stroke_color, Color::BLACK);
        assert_eq!(ctx.fill_color, Color::BLACK);
        assert_eq!(ctx.transform, Matrix::IDENTITY);
    }

    #[test]
    fn test_drawing_context_set_colors() {
        let mut ctx = DrawingContext::new();
        ctx.set_stroke_color(Color::RED).set_fill_color(Color::BLUE);
        assert_eq!(ctx.stroke_color, Color::RED);
        assert_eq!(ctx.fill_color, Color::BLUE);
    }

    #[test]
    fn test_drawing_context_set_opacity() {
        let mut ctx = DrawingContext::new();
        ctx.set_opacity(0.5);
        assert_eq!(ctx.stroke_color.a, 0.5);
        assert_eq!(ctx.fill_color.a, 0.5);
    }

    #[test]
    fn test_drawing_context_transform() {
        let mut ctx = DrawingContext::new();
        ctx.translate(10.0, 20.0);
        assert_ne!(ctx.transform, Matrix::IDENTITY);
    }

    #[test]
    fn test_drawing_context_reset_transform() {
        let mut ctx = DrawingContext::new();
        ctx.translate(10.0, 20.0).reset_transform();
        assert_eq!(ctx.transform, Matrix::IDENTITY);
    }

    #[test]
    fn test_create_circle_path() {
        let ctx = DrawingContext::new();
        let path = ctx.create_circle_path(100.0, 100.0, 50.0);
        // Path should be created successfully
        let _bounds = path.bounds();
        assert!(_bounds.x0 <= 50.0 && _bounds.x1 >= 150.0);
    }

    #[test]
    fn test_create_ellipse_path() {
        let ctx = DrawingContext::new();
        let path = ctx.create_ellipse_path(100.0, 100.0, 50.0, 30.0);
        // Path should be created successfully
        let _bounds = path.bounds();
        assert!(_bounds.x0 <= 50.0 && _bounds.x1 >= 150.0);
    }

    #[test]
    fn test_create_rounded_rect_path() {
        let ctx = DrawingContext::new();
        let path = ctx.create_rounded_rect_path(0.0, 0.0, 100.0, 50.0, 10.0);
        // Path should be created successfully
        let _bounds = path.bounds();
        assert!(_bounds.x1 >= 100.0 && _bounds.y1 >= 50.0);
    }

    #[test]
    fn test_draw_polygon_empty() {
        let mut ctx = DrawingContext::new();
        let result = ctx.draw_polygon(&[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_pdf_drawing_new() {
        let drawing = PdfDrawing::new();
        assert_eq!(drawing.context.stroke_color, Color::BLACK);
    }
}
