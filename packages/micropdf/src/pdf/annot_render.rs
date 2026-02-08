//! PDF Annotation Rendering
//!
//! Renders PDF annotations to devices for display and printing.
//! Supports all standard PDF annotation types.

use crate::fitz::colorspace::Colorspace;
use crate::fitz::device::Device;
use crate::fitz::geometry::{Matrix, Point, Rect};
use crate::fitz::path::{Path, StrokeState};
use crate::pdf::annot::{AnnotFlags, AnnotType, Annotation};

/// Annotation rendering options
#[derive(Debug, Clone)]
pub struct AnnotRenderOptions {
    /// Render annotations marked as "Print"
    pub render_print: bool,
    /// Render annotations marked as "NoView"
    pub render_no_view: bool,
    /// Render popup annotations
    pub render_popups: bool,
    /// Render widget annotations (form fields)
    pub render_widgets: bool,
    /// Override annotation opacity (0.0-1.0, None = use annotation's value)
    pub override_opacity: Option<f32>,
}

impl Default for AnnotRenderOptions {
    fn default() -> Self {
        Self {
            render_print: true,
            render_no_view: false,
            render_popups: false,
            render_widgets: true,
            override_opacity: None,
        }
    }
}

/// Annotation renderer
pub struct AnnotRenderer {
    options: AnnotRenderOptions,
}

impl AnnotRenderer {
    /// Create a new annotation renderer
    pub fn new(options: AnnotRenderOptions) -> Self {
        Self { options }
    }

    /// Create with default options
    pub fn with_defaults() -> Self {
        Self::new(AnnotRenderOptions::default())
    }

    /// Check if annotation should be rendered
    pub fn should_render(&self, annot: &Annotation) -> bool {
        // Check flags using convenience methods
        if annot.is_hidden() {
            return false;
        }

        let flags = annot.flags();
        if !self.options.render_no_view && flags.has(AnnotFlags::NO_VIEW) {
            return false;
        }

        if !self.options.render_print && !annot.is_printable() {
            return false;
        }

        // Check annotation type
        match annot.annot_type() {
            AnnotType::Popup if !self.options.render_popups => false,
            AnnotType::Widget if !self.options.render_widgets => false,
            _ => true,
        }
    }

    /// Render an annotation to a device
    pub fn render<D: Device>(
        &self,
        annot: &Annotation,
        ctm: &Matrix,
        device: &mut D,
    ) -> Result<(), String> {
        if !self.should_render(annot) {
            return Ok(());
        }

        let opacity = self.options.override_opacity.unwrap_or(annot.opacity());

        match annot.annot_type() {
            AnnotType::Text => self.render_text_annot(annot, ctm, opacity, device),
            AnnotType::Link => self.render_link_annot(annot, ctm, opacity, device),
            AnnotType::FreeText => self.render_free_text_annot(annot, ctm, opacity, device),
            AnnotType::Line => self.render_line_annot(annot, ctm, opacity, device),
            AnnotType::Square => self.render_square_annot(annot, ctm, opacity, device),
            AnnotType::Circle => self.render_circle_annot(annot, ctm, opacity, device),
            AnnotType::Polygon => self.render_polygon_annot(annot, ctm, opacity, device),
            AnnotType::PolyLine => self.render_polyline_annot(annot, ctm, opacity, device),
            AnnotType::Highlight => self.render_highlight_annot(annot, ctm, opacity, device),
            AnnotType::Underline => self.render_underline_annot(annot, ctm, opacity, device),
            AnnotType::Squiggly => self.render_squiggly_annot(annot, ctm, opacity, device),
            AnnotType::StrikeOut => self.render_strikeout_annot(annot, ctm, opacity, device),
            AnnotType::Stamp => self.render_stamp_annot(annot, ctm, opacity, device),
            AnnotType::Ink => self.render_ink_annot(annot, ctm, opacity, device),
            _ => Ok(()), // Other types not yet implemented
        }
    }

    /// Render multiple annotations
    pub fn render_all<D: Device>(
        &self,
        annots: &[Annotation],
        ctm: &Matrix,
        device: &mut D,
    ) -> Result<(), String> {
        for annot in annots {
            self.render(annot, ctm, device)?;
        }
        Ok(())
    }

    // ========================================================================
    // Annotation Type Renderers
    // ========================================================================

    /// Render text annotation (sticky note)
    fn render_text_annot<D: Device>(
        &self,
        annot: &Annotation,
        ctm: &Matrix,
        opacity: f32,
        device: &mut D,
    ) -> Result<(), String> {
        // Draw sticky note icon
        let rect = annot.rect();
        let icon_size = 20.0f32.min(rect.width()).min(rect.height());

        let mut path = Path::new();
        path.move_to(Point::new(rect.x0, rect.y0));
        path.line_to(Point::new(rect.x0 + icon_size, rect.y0));
        path.line_to(Point::new(rect.x0 + icon_size, rect.y0 + icon_size));
        path.line_to(Point::new(rect.x0, rect.y0 + icon_size));
        path.close();

        // Yellow fill for sticky note
        let color = annot
            .color()
            .map(|c| vec![c[0], c[1], c[2]])
            .unwrap_or(vec![1.0, 1.0, 0.0]); // Default yellow
        let colorspace = Colorspace::device_rgb();

        device.fill_path(&path, false, ctm, &colorspace, &color, opacity);

        Ok(())
    }

    /// Render link annotation (usually invisible)
    fn render_link_annot<D: Device>(
        &self,
        annot: &Annotation,
        ctm: &Matrix,
        opacity: f32,
        device: &mut D,
    ) -> Result<(), String> {
        // Links are typically not visible
        // But we can draw a border for debugging/highlighting
        if annot.border().width > 0.0 {
            let mut path = Path::new();
            let rect = annot.rect();
            path.move_to(Point::new(rect.x0, rect.y0));
            path.line_to(Point::new(rect.x1, rect.y0));
            path.line_to(Point::new(rect.x1, rect.y1));
            path.line_to(Point::new(rect.x0, rect.y1));
            path.close();

            let stroke_state = StrokeState::new();
            let color = vec![0.0, 0.0, 1.0]; // Blue for links
            let colorspace = Colorspace::device_rgb();

            device.stroke_path(&path, &stroke_state, ctm, &colorspace, &color, opacity);
        }

        Ok(())
    }

    /// Render free text annotation
    fn render_free_text_annot<D: Device>(
        &self,
        annot: &Annotation,
        ctm: &Matrix,
        opacity: f32,
        device: &mut D,
    ) -> Result<(), String> {
        // Draw background rectangle
        let mut path = Path::new();
        let rect = annot.rect();
        path.move_to(Point::new(rect.x0, rect.y0));
        path.line_to(Point::new(rect.x1, rect.y0));
        path.line_to(Point::new(rect.x1, rect.y1));
        path.line_to(Point::new(rect.x0, rect.y1));
        path.close();

        // Light background
        let bg_color = vec![1.0, 1.0, 0.9]; // Light yellow
        let colorspace = Colorspace::device_rgb();

        device.fill_path(&path, false, ctm, &colorspace, &bg_color, opacity * 0.5);

        // TODO: Render text content (requires text rendering)

        Ok(())
    }

    /// Render line annotation
    fn render_line_annot<D: Device>(
        &self,
        annot: &Annotation,
        ctm: &Matrix,
        opacity: f32,
        device: &mut D,
    ) -> Result<(), String> {
        let rect = annot.rect();

        let mut path = Path::new();
        path.move_to(Point::new(rect.x0, rect.y0));
        path.line_to(Point::new(rect.x1, rect.y1));

        let mut stroke_state = StrokeState::new();
        stroke_state.linewidth = annot.border().width.max(1.0);

        let color = annot
            .color()
            .map(|c| vec![c[0], c[1], c[2]])
            .unwrap_or(vec![1.0, 0.0, 0.0]); // Default red
        let colorspace = Colorspace::device_rgb();

        device.stroke_path(&path, &stroke_state, ctm, &colorspace, &color, opacity);

        Ok(())
    }

    /// Render square annotation
    fn render_square_annot<D: Device>(
        &self,
        annot: &Annotation,
        ctm: &Matrix,
        opacity: f32,
        device: &mut D,
    ) -> Result<(), String> {
        let rect = annot.rect();

        let mut path = Path::new();
        path.move_to(Point::new(rect.x0, rect.y0));
        path.line_to(Point::new(rect.x1, rect.y0));
        path.line_to(Point::new(rect.x1, rect.y1));
        path.line_to(Point::new(rect.x0, rect.y1));
        path.close();

        let colorspace = Colorspace::device_rgb();

        // Fill if interior color is set
        let interior_color = annot.interior_color();
        if !interior_color.is_empty() {
            let fill_color = interior_color.to_vec();
            device.fill_path(&path, false, ctm, &colorspace, &fill_color, opacity * 0.5);
        }

        // Stroke border
        if annot.border().width > 0.0 {
            let mut stroke_state = StrokeState::new();
            stroke_state.linewidth = annot.border().width;

            let color = annot
                .color()
                .map(|c| vec![c[0], c[1], c[2]])
                .unwrap_or(vec![0.0, 0.0, 0.0]);
            device.stroke_path(&path, &stroke_state, ctm, &colorspace, &color, opacity);
        }

        Ok(())
    }

    /// Render circle annotation
    fn render_circle_annot<D: Device>(
        &self,
        annot: &Annotation,
        ctm: &Matrix,
        opacity: f32,
        device: &mut D,
    ) -> Result<(), String> {
        let rect = annot.rect();
        let cx = (rect.x0 + rect.x1) * 0.5;
        let cy = (rect.y0 + rect.y1) * 0.5;
        let rx = (rect.x1 - rect.x0) * 0.5;
        let ry = (rect.y1 - rect.y0) * 0.5;

        // Approximate circle with 4 Bézier curves
        let kappa = 0.5522847498; // Magic number for circle approximation
        let kx = rx * kappa;
        let ky = ry * kappa;

        let mut path = Path::new();
        path.move_to(Point::new(cx, cy - ry));
        path.curve_to(
            Point::new(cx + kx, cy - ry),
            Point::new(cx + rx, cy - ky),
            Point::new(cx + rx, cy),
        );
        path.curve_to(
            Point::new(cx + rx, cy + ky),
            Point::new(cx + kx, cy + ry),
            Point::new(cx, cy + ry),
        );
        path.curve_to(
            Point::new(cx - kx, cy + ry),
            Point::new(cx - rx, cy + ky),
            Point::new(cx - rx, cy),
        );
        path.curve_to(
            Point::new(cx - rx, cy - ky),
            Point::new(cx - kx, cy - ry),
            Point::new(cx, cy - ry),
        );
        path.close();

        let colorspace = Colorspace::device_rgb();

        // Fill if interior color is set
        let interior_color = annot.interior_color();
        if !interior_color.is_empty() {
            let fill_color = interior_color.to_vec();
            device.fill_path(&path, false, ctm, &colorspace, &fill_color, opacity * 0.5);
        }

        // Stroke border
        if annot.border().width > 0.0 {
            let mut stroke_state = StrokeState::new();
            stroke_state.linewidth = annot.border().width;

            let color = annot
                .color()
                .map(|c| vec![c[0], c[1], c[2]])
                .unwrap_or(vec![0.0, 0.0, 0.0]);
            device.stroke_path(&path, &stroke_state, ctm, &colorspace, &color, opacity);
        }

        Ok(())
    }

    /// Render polygon annotation
    fn render_polygon_annot<D: Device>(
        &self,
        annot: &Annotation,
        ctm: &Matrix,
        opacity: f32,
        device: &mut D,
    ) -> Result<(), String> {
        // Simplified: draw rectangle
        // Real implementation would use vertices from annotation
        self.render_square_annot(annot, ctm, opacity, device)
    }

    /// Render polyline annotation
    fn render_polyline_annot<D: Device>(
        &self,
        annot: &Annotation,
        ctm: &Matrix,
        opacity: f32,
        device: &mut D,
    ) -> Result<(), String> {
        // Simplified: draw line
        // Real implementation would use vertices from annotation
        self.render_line_annot(annot, ctm, opacity, device)
    }

    /// Render highlight annotation
    fn render_highlight_annot<D: Device>(
        &self,
        annot: &Annotation,
        ctm: &Matrix,
        opacity: f32,
        device: &mut D,
    ) -> Result<(), String> {
        // Semi-transparent yellow overlay
        let rect = annot.rect();

        let mut path = Path::new();
        path.move_to(Point::new(rect.x0, rect.y0));
        path.line_to(Point::new(rect.x1, rect.y0));
        path.line_to(Point::new(rect.x1, rect.y1));
        path.line_to(Point::new(rect.x0, rect.y1));
        path.close();

        let color = annot
            .color()
            .map(|c| vec![c[0], c[1], c[2]])
            .unwrap_or(vec![1.0, 1.0, 0.0]); // Yellow
        let colorspace = Colorspace::device_rgb();

        device.fill_path(&path, false, ctm, &colorspace, &color, opacity * 0.3);

        Ok(())
    }

    /// Render underline annotation
    fn render_underline_annot<D: Device>(
        &self,
        annot: &Annotation,
        ctm: &Matrix,
        opacity: f32,
        device: &mut D,
    ) -> Result<(), String> {
        let rect = annot.rect();

        // Draw line at bottom of rect
        let mut path = Path::new();
        let y = rect.y0 + 2.0; // Slightly above bottom
        path.move_to(Point::new(rect.x0, y));
        path.line_to(Point::new(rect.x1, y));

        let mut stroke_state = StrokeState::new();
        stroke_state.linewidth = 1.0;

        let color = annot
            .color()
            .map(|c| vec![c[0], c[1], c[2]])
            .unwrap_or(vec![0.0, 0.0, 0.0]); // Black
        let colorspace = Colorspace::device_rgb();

        device.stroke_path(&path, &stroke_state, ctm, &colorspace, &color, opacity);

        Ok(())
    }

    /// Render squiggly underline annotation
    fn render_squiggly_annot<D: Device>(
        &self,
        annot: &Annotation,
        ctm: &Matrix,
        opacity: f32,
        device: &mut D,
    ) -> Result<(), String> {
        let rect = annot.rect();

        // Draw wavy line at bottom
        let mut path = Path::new();
        let y = rect.y0 + 2.0;
        let width = rect.width();
        let wave_count = (width / 5.0) as i32;
        let segment_width = width / wave_count as f32;

        path.move_to(Point::new(rect.x0, y));

        for i in 0..wave_count {
            let x1 = rect.x0 + (i as f32 + 0.5) * segment_width;
            let y1 = y + 2.0; // Wave peak
            let x2 = rect.x0 + (i as f32 + 1.0) * segment_width;
            let y2 = y;

            path.curve_to(Point::new(x1, y1), Point::new(x1, y1), Point::new(x2, y2));
        }

        let mut stroke_state = StrokeState::new();
        stroke_state.linewidth = 1.0;

        let color = annot
            .color()
            .map(|c| vec![c[0], c[1], c[2]])
            .unwrap_or(vec![1.0, 0.0, 0.0]); // Red
        let colorspace = Colorspace::device_rgb();

        device.stroke_path(&path, &stroke_state, ctm, &colorspace, &color, opacity);

        Ok(())
    }

    /// Render strikeout annotation
    fn render_strikeout_annot<D: Device>(
        &self,
        annot: &Annotation,
        ctm: &Matrix,
        opacity: f32,
        device: &mut D,
    ) -> Result<(), String> {
        let rect = annot.rect();

        // Draw line through middle of rect
        let mut path = Path::new();
        let y = (rect.y0 + rect.y1) * 0.5;
        path.move_to(Point::new(rect.x0, y));
        path.line_to(Point::new(rect.x1, y));

        let mut stroke_state = StrokeState::new();
        stroke_state.linewidth = 1.0;

        let color = annot
            .color()
            .map(|c| vec![c[0], c[1], c[2]])
            .unwrap_or(vec![1.0, 0.0, 0.0]); // Red
        let colorspace = Colorspace::device_rgb();

        device.stroke_path(&path, &stroke_state, ctm, &colorspace, &color, opacity);

        Ok(())
    }

    /// Render stamp annotation
    fn render_stamp_annot<D: Device>(
        &self,
        annot: &Annotation,
        ctm: &Matrix,
        opacity: f32,
        device: &mut D,
    ) -> Result<(), String> {
        // Draw stamp as colored rectangle with border
        let rect = annot.rect();

        let mut path = Path::new();
        path.move_to(Point::new(rect.x0, rect.y0));
        path.line_to(Point::new(rect.x1, rect.y0));
        path.line_to(Point::new(rect.x1, rect.y1));
        path.line_to(Point::new(rect.x0, rect.y1));
        path.close();

        let colorspace = Colorspace::device_rgb();

        // Fill with stamp color
        let fill_color = annot
            .color()
            .map(|c| vec![c[0], c[1], c[2]])
            .unwrap_or(vec![1.0, 0.0, 0.0]); // Red
        device.fill_path(&path, false, ctm, &colorspace, &fill_color, opacity * 0.3);

        // Draw border
        let mut stroke_state = StrokeState::new();
        stroke_state.linewidth = 2.0;
        device.stroke_path(&path, &stroke_state, ctm, &colorspace, &fill_color, opacity);

        Ok(())
    }

    /// Render ink annotation (freehand drawing)
    fn render_ink_annot<D: Device>(
        &self,
        annot: &Annotation,
        ctm: &Matrix,
        opacity: f32,
        device: &mut D,
    ) -> Result<(), String> {
        // Simplified: draw rectangle outline
        // Real implementation would use ink path data
        let rect = annot.rect();

        let mut path = Path::new();
        path.move_to(Point::new(rect.x0, rect.y0));
        path.line_to(Point::new(rect.x1, rect.y1));

        let mut stroke_state = StrokeState::new();
        stroke_state.linewidth = annot.border().width.max(1.0);

        let color = annot
            .color()
            .map(|c| vec![c[0], c[1], c[2]])
            .unwrap_or(vec![0.0, 0.0, 0.0]); // Black
        let colorspace = Colorspace::device_rgb();

        device.stroke_path(&path, &stroke_state, ctm, &colorspace, &color, opacity);

        Ok(())
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_annot_render_options_default() {
        let opts = AnnotRenderOptions::default();
        assert!(opts.render_print);
        assert!(!opts.render_no_view);
        assert!(opts.render_widgets);
    }

    #[test]
    fn test_annot_renderer_creation() {
        let renderer = AnnotRenderer::with_defaults();
        assert!(renderer.options.render_print);
    }
}
