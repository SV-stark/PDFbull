//! PDF Form Field Rendering
//!
//! Renders PDF AcroForm fields (text boxes, checkboxes, buttons, etc.) to devices.

use crate::fitz::colorspace::Colorspace;
use crate::fitz::device::Device;
use crate::fitz::geometry::{Matrix, Point, Rect};
use crate::fitz::path::{LineCap, LineJoin, Path, StrokeState};
use crate::pdf::form::{FieldFlags, FormField, WidgetType};

/// Form field rendering options
#[derive(Debug, Clone)]
pub struct FormRenderOptions {
    /// Render read-only fields differently
    pub highlight_readonly: bool,
    /// Render required fields differently
    pub highlight_required: bool,
    /// Show field borders
    pub show_borders: bool,
    /// Show field backgrounds
    pub show_backgrounds: bool,
    /// Border color for fields [R, G, B]
    pub border_color: [f32; 3],
    /// Background color for fields [R, G, B]
    pub background_color: [f32; 3],
    /// Text color for field values [R, G, B]
    pub text_color: [f32; 3],
    /// Highlight color for required fields [R, G, B]
    pub required_color: [f32; 3],
}

impl Default for FormRenderOptions {
    fn default() -> Self {
        Self {
            highlight_readonly: true,
            highlight_required: true,
            show_borders: true,
            show_backgrounds: true,
            border_color: [0.0, 0.0, 0.0],     // Black
            background_color: [1.0, 1.0, 1.0], // White
            text_color: [0.0, 0.0, 0.0],       // Black
            required_color: [1.0, 0.9, 0.9],   // Light red
        }
    }
}

/// Form field renderer
pub struct FormRenderer {
    options: FormRenderOptions,
}

impl FormRenderer {
    /// Create a new form renderer
    pub fn new(options: FormRenderOptions) -> Self {
        Self { options }
    }

    /// Create with default options
    pub fn with_defaults() -> Self {
        Self::new(FormRenderOptions::default())
    }

    /// Render a form field to a device
    pub fn render<D: Device>(
        &self,
        field: &FormField,
        ctm: &Matrix,
        device: &mut D,
    ) -> Result<(), String> {
        match field.field_type {
            WidgetType::Text => self.render_text_field(field, ctm, device),
            WidgetType::Button => self.render_button_field(field, ctm, device),
            WidgetType::Checkbox => self.render_checkbox_field(field, ctm, device),
            WidgetType::RadioButton => self.render_radio_field(field, ctm, device),
            WidgetType::ComboBox => self.render_combo_field(field, ctm, device),
            WidgetType::ListBox => self.render_list_field(field, ctm, device),
            WidgetType::Signature => self.render_signature_field(field, ctm, device),
            WidgetType::Unknown => Ok(()), // Skip unknown types
        }
    }

    /// Render multiple form fields
    pub fn render_all<D: Device>(
        &self,
        fields: &[FormField],
        ctm: &Matrix,
        device: &mut D,
    ) -> Result<(), String> {
        for field in fields {
            self.render(field, ctm, device)?;
        }
        Ok(())
    }

    // ========================================================================
    // Field Type Renderers
    // ========================================================================

    /// Render text field
    fn render_text_field<D: Device>(
        &self,
        field: &FormField,
        ctm: &Matrix,
        device: &mut D,
    ) -> Result<(), String> {
        let rect = field.rect();

        // Draw background
        if self.options.show_backgrounds {
            let bg_color = if field.is_required() && self.options.highlight_required {
                self.options.required_color.to_vec()
            } else {
                self.options.background_color.to_vec()
            };

            let mut path = self.create_rect_path(&rect);
            let colorspace = Colorspace::device_rgb();
            device.fill_path(&path, false, ctm, &colorspace, &bg_color, 1.0);
        }

        // Draw border
        if self.options.show_borders {
            let mut path = self.create_rect_path(&rect);
            let mut stroke_state = StrokeState::new();
            stroke_state.linewidth = 1.0;

            let colorspace = Colorspace::device_rgb();
            let border_color = self.options.border_color.to_vec();
            device.stroke_path(&path, &stroke_state, ctm, &colorspace, &border_color, 1.0);
        }

        // TODO: Render text value (requires text rendering integration)

        // Draw cursor if multiline
        if field.is_multiline() {
            // Visual indicator for multiline text field
            let cursor_x = rect.x1 - 5.0;
            let cursor_y = rect.y0 + 3.0;
            let cursor_h = (rect.height() - 6.0).min(10.0);

            let mut path = Path::new();
            path.move_to(Point::new(cursor_x, cursor_y));
            path.line_to(Point::new(cursor_x, cursor_y + cursor_h));

            let mut stroke_state = StrokeState::new();
            stroke_state.linewidth = 1.0;

            let colorspace = Colorspace::device_rgb();
            let text_color = self.options.text_color.to_vec();
            device.stroke_path(&path, &stroke_state, ctm, &colorspace, &text_color, 0.5);
        }

        Ok(())
    }

    /// Render button field
    fn render_button_field<D: Device>(
        &self,
        field: &FormField,
        ctm: &Matrix,
        device: &mut D,
    ) -> Result<(), String> {
        let rect = field.rect();

        // Draw button background
        let mut path = self.create_rect_path(&rect);

        let bg_color = if field.is_read_only() {
            vec![0.9, 0.9, 0.9] // Gray for read-only
        } else {
            vec![0.95, 0.95, 0.95] // Light gray
        };

        let colorspace = Colorspace::device_rgb();
        device.fill_path(&path, false, ctm, &colorspace, &bg_color, 1.0);

        // Draw button border (3D effect)
        let mut stroke_state = StrokeState::new();
        stroke_state.linewidth = 2.0;

        let border_color = vec![0.6, 0.6, 0.6];
        device.stroke_path(&path, &stroke_state, ctm, &colorspace, &border_color, 1.0);

        // TODO: Render button label (requires text rendering)

        Ok(())
    }

    /// Render checkbox field
    fn render_checkbox_field<D: Device>(
        &self,
        field: &FormField,
        ctm: &Matrix,
        device: &mut D,
    ) -> Result<(), String> {
        let rect = field.rect();

        // Make checkbox square
        let size = rect.width().min(rect.height());
        let checkbox_rect = Rect::new(rect.x0, rect.y0, rect.x0 + size, rect.y0 + size);

        // Draw checkbox background
        let mut path = self.create_rect_path(&checkbox_rect);
        let bg_color = self.options.background_color.to_vec();
        let colorspace = Colorspace::device_rgb();
        device.fill_path(&path, false, ctm, &colorspace, &bg_color, 1.0);

        // Draw checkbox border
        let mut stroke_state = StrokeState::new();
        stroke_state.linewidth = 1.0;

        let border_color = self.options.border_color.to_vec();
        device.stroke_path(&path, &stroke_state, ctm, &colorspace, &border_color, 1.0);

        // Draw checkmark if checked
        if field.is_checked() {
            let inset = size * 0.2;
            let mut check_path = Path::new();

            // Draw checkmark
            check_path.move_to(Point::new(
                checkbox_rect.x0 + inset,
                checkbox_rect.y0 + size * 0.5,
            ));
            check_path.line_to(Point::new(
                checkbox_rect.x0 + size * 0.4,
                checkbox_rect.y0 + size - inset,
            ));
            check_path.line_to(Point::new(
                checkbox_rect.x0 + size - inset,
                checkbox_rect.y0 + inset,
            ));

            let mut check_stroke = StrokeState::new();
            check_stroke.linewidth = 2.0;
            check_stroke.start_cap = LineCap::Round;
            check_stroke.linejoin = LineJoin::Round;

            let check_color = vec![0.0, 0.5, 0.0]; // Dark green
            device.stroke_path(
                &check_path,
                &check_stroke,
                ctm,
                &colorspace,
                &check_color,
                1.0,
            );
        }

        Ok(())
    }

    /// Render radio button field
    fn render_radio_field<D: Device>(
        &self,
        field: &FormField,
        ctm: &Matrix,
        device: &mut D,
    ) -> Result<(), String> {
        let rect = field.rect();

        // Make radio button circular
        let size = rect.width().min(rect.height());
        let cx = rect.x0 + size * 0.5;
        let cy = rect.y0 + size * 0.5;
        let radius = size * 0.4;

        // Draw radio circle
        let circle_path = self.create_circle_path(cx, cy, radius);
        let bg_color = self.options.background_color.to_vec();
        let colorspace = Colorspace::device_rgb();
        device.fill_path(&circle_path, false, ctm, &colorspace, &bg_color, 1.0);

        // Draw radio border
        let mut stroke_state = StrokeState::new();
        stroke_state.linewidth = 1.0;

        let border_color = self.options.border_color.to_vec();
        device.stroke_path(
            &circle_path,
            &stroke_state,
            ctm,
            &colorspace,
            &border_color,
            1.0,
        );

        // Draw inner circle if selected
        if field.is_checked() {
            let inner_radius = radius * 0.5;
            let inner_circle = self.create_circle_path(cx, cy, inner_radius);

            let select_color = vec![0.0, 0.0, 0.8]; // Blue
            device.fill_path(&inner_circle, false, ctm, &colorspace, &select_color, 1.0);
        }

        Ok(())
    }

    /// Render combo box (dropdown) field
    fn render_combo_field<D: Device>(
        &self,
        field: &FormField,
        ctm: &Matrix,
        device: &mut D,
    ) -> Result<(), String> {
        let rect = field.rect();

        // Draw background
        let mut path = self.create_rect_path(&rect);
        let bg_color = self.options.background_color.to_vec();
        let colorspace = Colorspace::device_rgb();
        device.fill_path(&path, false, ctm, &colorspace, &bg_color, 1.0);

        // Draw border
        let mut stroke_state = StrokeState::new();
        stroke_state.linewidth = 1.0;

        let border_color = self.options.border_color.to_vec();
        device.stroke_path(&path, &stroke_state, ctm, &colorspace, &border_color, 1.0);

        // Draw dropdown arrow
        let arrow_size = 8.0;
        let arrow_x = rect.x1 - arrow_size - 5.0;
        let arrow_y = rect.y0 + (rect.height() - arrow_size) * 0.5;

        let mut arrow_path = Path::new();
        arrow_path.move_to(Point::new(arrow_x, arrow_y));
        arrow_path.line_to(Point::new(arrow_x + arrow_size, arrow_y));
        arrow_path.line_to(Point::new(arrow_x + arrow_size * 0.5, arrow_y + arrow_size));
        arrow_path.close();

        let arrow_color = vec![0.3, 0.3, 0.3];
        device.fill_path(&arrow_path, false, ctm, &colorspace, &arrow_color, 1.0);

        // TODO: Render selected value text

        Ok(())
    }

    /// Render list box field
    fn render_list_field<D: Device>(
        &self,
        field: &FormField,
        ctm: &Matrix,
        device: &mut D,
    ) -> Result<(), String> {
        let rect = field.rect();

        // Draw background
        let mut path = self.create_rect_path(&rect);
        let bg_color = self.options.background_color.to_vec();
        let colorspace = Colorspace::device_rgb();
        device.fill_path(&path, false, ctm, &colorspace, &bg_color, 1.0);

        // Draw border
        let mut stroke_state = StrokeState::new();
        stroke_state.linewidth = 1.0;

        let border_color = self.options.border_color.to_vec();
        device.stroke_path(&path, &stroke_state, ctm, &colorspace, &border_color, 1.0);

        // TODO: Render list items (requires text rendering)
        // Draw scrollbar indicator if needed
        if field.options.len() > 5 {
            let scrollbar_width = 10.0;
            let scrollbar_rect = Rect::new(
                rect.x1 - scrollbar_width - 2.0,
                rect.y0 + 2.0,
                rect.x1 - 2.0,
                rect.y1 - 2.0,
            );

            let mut scrollbar_path = self.create_rect_path(&scrollbar_rect);
            let scrollbar_color = vec![0.8, 0.8, 0.8];
            device.fill_path(
                &scrollbar_path,
                false,
                ctm,
                &colorspace,
                &scrollbar_color,
                1.0,
            );
        }

        Ok(())
    }

    /// Render signature field
    fn render_signature_field<D: Device>(
        &self,
        field: &FormField,
        ctm: &Matrix,
        device: &mut D,
    ) -> Result<(), String> {
        let rect = field.rect();

        // Draw background
        let mut path = self.create_rect_path(&rect);
        let bg_color = if field.value().is_empty() {
            vec![1.0, 1.0, 0.9] // Light yellow (unsigned)
        } else {
            vec![0.9, 1.0, 0.9] // Light green (signed)
        };

        let colorspace = Colorspace::device_rgb();
        device.fill_path(&path, false, ctm, &colorspace, &bg_color, 1.0);

        // Draw border
        let mut stroke_state = StrokeState::new();
        stroke_state.linewidth = 1.5;
        stroke_state.dash_pattern = vec![3.0, 3.0]; // Dashed for signature

        let border_color = if field.value().is_empty() {
            vec![0.8, 0.6, 0.0] // Orange (unsigned)
        } else {
            vec![0.0, 0.6, 0.0] // Green (signed)
        };

        device.stroke_path(&path, &stroke_state, ctm, &colorspace, &border_color, 1.0);

        // Draw signature icon
        if field.value().is_empty() {
            let icon_size = 16.0;
            let icon_x = rect.x0 + 5.0;
            let icon_y = rect.y0 + (rect.height() - icon_size) * 0.5;

            // Draw pen icon
            let mut pen_path = Path::new();
            pen_path.move_to(Point::new(icon_x, icon_y + icon_size));
            pen_path.line_to(Point::new(icon_x + icon_size, icon_y));

            let mut pen_stroke = StrokeState::new();
            pen_stroke.linewidth = 2.0;

            let pen_color = vec![0.5, 0.5, 0.5];
            device.stroke_path(&pen_path, &pen_stroke, ctm, &colorspace, &pen_color, 1.0);
        }

        // TODO: Render signature image/text

        Ok(())
    }

    // ========================================================================
    // Helper Functions
    // ========================================================================

    /// Create a rectangle path
    fn create_rect_path(&self, rect: &Rect) -> Path {
        let mut path = Path::new();
        path.move_to(Point::new(rect.x0, rect.y0));
        path.line_to(Point::new(rect.x1, rect.y0));
        path.line_to(Point::new(rect.x1, rect.y1));
        path.line_to(Point::new(rect.x0, rect.y1));
        path.close();
        path
    }

    /// Create a circle path using Bézier curves
    fn create_circle_path(&self, cx: f32, cy: f32, radius: f32) -> Path {
        let kappa = 0.5522847498; // Magic number for circle approximation
        let k = radius * kappa;

        let mut path = Path::new();
        path.move_to(Point::new(cx, cy - radius));
        path.curve_to(
            Point::new(cx + k, cy - radius),
            Point::new(cx + radius, cy - k),
            Point::new(cx + radius, cy),
        );
        path.curve_to(
            Point::new(cx + radius, cy + k),
            Point::new(cx + k, cy + radius),
            Point::new(cx, cy + radius),
        );
        path.curve_to(
            Point::new(cx - k, cy + radius),
            Point::new(cx - radius, cy + k),
            Point::new(cx - radius, cy),
        );
        path.curve_to(
            Point::new(cx - radius, cy - k),
            Point::new(cx - k, cy - radius),
            Point::new(cx, cy - radius),
        );
        path.close();
        path
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_form_render_options_default() {
        let opts = FormRenderOptions::default();
        assert!(opts.show_borders);
        assert!(opts.show_backgrounds);
    }

    #[test]
    fn test_form_renderer_creation() {
        let renderer = FormRenderer::with_defaults();
        assert!(renderer.options.show_borders);
    }
}
