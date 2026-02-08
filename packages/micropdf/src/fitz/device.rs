//! Rendering device trait and implementations
//!
//! Devices receive and process drawing operations from content stream interpretation.

use crate::fitz::colorspace::Colorspace;
use crate::fitz::geometry::{Matrix, Rect};
use crate::fitz::image::Image;
use crate::fitz::path::{Path, StrokeState};
use crate::fitz::text::Text;

/// Blend modes for transparency groups
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum BlendMode {
    // PDF 1.4 - standard separable
    #[default]
    Normal = 0,
    Multiply = 1,
    Screen = 2,
    Overlay = 3,
    Darken = 4,
    Lighten = 5,
    ColorDodge = 6,
    ColorBurn = 7,
    HardLight = 8,
    SoftLight = 9,
    Difference = 10,
    Exclusion = 11,

    // PDF 1.4 - standard non-separable
    Hue = 12,
    Saturation = 13,
    Color = 14,
    Luminosity = 15,
}

impl BlendMode {
    /// Parse blend mode from string
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "Normal" => Some(Self::Normal),
            "Multiply" => Some(Self::Multiply),
            "Screen" => Some(Self::Screen),
            "Overlay" => Some(Self::Overlay),
            "Darken" => Some(Self::Darken),
            "Lighten" => Some(Self::Lighten),
            "ColorDodge" => Some(Self::ColorDodge),
            "ColorBurn" => Some(Self::ColorBurn),
            "HardLight" => Some(Self::HardLight),
            "SoftLight" => Some(Self::SoftLight),
            "Difference" => Some(Self::Difference),
            "Exclusion" => Some(Self::Exclusion),
            "Hue" => Some(Self::Hue),
            "Saturation" => Some(Self::Saturation),
            "Color" => Some(Self::Color),
            "Luminosity" => Some(Self::Luminosity),
            _ => None,
        }
    }

    /// Get blend mode name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Normal => "Normal",
            Self::Multiply => "Multiply",
            Self::Screen => "Screen",
            Self::Overlay => "Overlay",
            Self::Darken => "Darken",
            Self::Lighten => "Lighten",
            Self::ColorDodge => "ColorDodge",
            Self::ColorBurn => "ColorBurn",
            Self::HardLight => "HardLight",
            Self::SoftLight => "SoftLight",
            Self::Difference => "Difference",
            Self::Exclusion => "Exclusion",
            Self::Hue => "Hue",
            Self::Saturation => "Saturation",
            Self::Color => "Color",
            Self::Luminosity => "Luminosity",
        }
    }
}

/// Container stack type for tracking clips/masks/groups
#[derive(Debug, Clone)]
pub enum ContainerType {
    Clip,
    Mask { luminosity: bool },
    Group { isolated: bool, knockout: bool },
    Tile,
}

/// Container stack entry
#[derive(Debug, Clone)]
pub struct Container {
    pub scissor: Rect,
    pub container_type: ContainerType,
}

/// Core device trait - receives drawing operations
pub trait Device {
    // Path operations
    fn fill_path(
        &mut self,
        path: &Path,
        even_odd: bool,
        ctm: &Matrix,
        colorspace: &Colorspace,
        color: &[f32],
        alpha: f32,
    );

    fn stroke_path(
        &mut self,
        path: &Path,
        stroke: &StrokeState,
        ctm: &Matrix,
        colorspace: &Colorspace,
        color: &[f32],
        alpha: f32,
    );

    fn clip_path(&mut self, path: &Path, even_odd: bool, ctm: &Matrix, scissor: Rect);

    fn clip_stroke_path(&mut self, path: &Path, stroke: &StrokeState, ctm: &Matrix, scissor: Rect);

    // Text operations
    fn fill_text(
        &mut self,
        text: &Text,
        ctm: &Matrix,
        colorspace: &Colorspace,
        color: &[f32],
        alpha: f32,
    );

    fn stroke_text(
        &mut self,
        text: &Text,
        stroke: &StrokeState,
        ctm: &Matrix,
        colorspace: &Colorspace,
        color: &[f32],
        alpha: f32,
    );

    fn clip_text(&mut self, text: &Text, ctm: &Matrix, scissor: Rect);

    fn clip_stroke_text(&mut self, text: &Text, stroke: &StrokeState, ctm: &Matrix, scissor: Rect);

    fn ignore_text(&mut self, text: &Text, ctm: &Matrix);

    // Image operations
    fn fill_image(&mut self, image: &Image, ctm: &Matrix, alpha: f32);

    fn fill_image_mask(
        &mut self,
        image: &Image,
        ctm: &Matrix,
        colorspace: &Colorspace,
        color: &[f32],
        alpha: f32,
    );

    fn clip_image_mask(&mut self, image: &Image, ctm: &Matrix, scissor: Rect);

    // Clipping
    fn pop_clip(&mut self);

    // Masking
    fn begin_mask(&mut self, area: Rect, luminosity: bool, colorspace: &Colorspace, color: &[f32]);

    fn end_mask(&mut self);

    // Transparency groups
    fn begin_group(
        &mut self,
        area: Rect,
        colorspace: Option<&Colorspace>,
        isolated: bool,
        knockout: bool,
        blendmode: BlendMode,
        alpha: f32,
    );

    fn end_group(&mut self);

    // Tiling
    fn begin_tile(&mut self, area: Rect, view: Rect, xstep: f32, ystep: f32, ctm: &Matrix) -> i32;

    fn end_tile(&mut self);

    // Control
    fn close(&mut self) {}
}

/// Null device - discards all operations
pub struct NullDevice;

impl Device for NullDevice {
    fn fill_path(&mut self, _: &Path, _: bool, _: &Matrix, _: &Colorspace, _: &[f32], _: f32) {}
    fn stroke_path(
        &mut self,
        _: &Path,
        _: &StrokeState,
        _: &Matrix,
        _: &Colorspace,
        _: &[f32],
        _: f32,
    ) {
    }
    fn clip_path(&mut self, _: &Path, _: bool, _: &Matrix, _: Rect) {}
    fn clip_stroke_path(&mut self, _: &Path, _: &StrokeState, _: &Matrix, _: Rect) {}
    fn fill_text(&mut self, _: &Text, _: &Matrix, _: &Colorspace, _: &[f32], _: f32) {}
    fn stroke_text(
        &mut self,
        _: &Text,
        _: &StrokeState,
        _: &Matrix,
        _: &Colorspace,
        _: &[f32],
        _: f32,
    ) {
    }
    fn clip_text(&mut self, _: &Text, _: &Matrix, _: Rect) {}
    fn clip_stroke_text(&mut self, _: &Text, _: &StrokeState, _: &Matrix, _: Rect) {}
    fn ignore_text(&mut self, _: &Text, _: &Matrix) {}
    fn fill_image(&mut self, _: &Image, _: &Matrix, _: f32) {}
    fn fill_image_mask(&mut self, _: &Image, _: &Matrix, _: &Colorspace, _: &[f32], _: f32) {}
    fn clip_image_mask(&mut self, _: &Image, _: &Matrix, _: Rect) {}
    fn pop_clip(&mut self) {}
    fn begin_mask(&mut self, _: Rect, _: bool, _: &Colorspace, _: &[f32]) {}
    fn end_mask(&mut self) {}
    fn begin_group(
        &mut self,
        _: Rect,
        _: Option<&Colorspace>,
        _: bool,
        _: bool,
        _: BlendMode,
        _: f32,
    ) {
    }
    fn end_group(&mut self) {}
    fn begin_tile(&mut self, _: Rect, _: Rect, _: f32, _: f32, _: &Matrix) -> i32 {
        0
    }
    fn end_tile(&mut self) {}
}

/// Bounding box device - calculates bounding box of all operations
pub struct BBoxDevice {
    bbox: Rect,
}

impl BBoxDevice {
    pub fn new() -> Self {
        Self { bbox: Rect::EMPTY }
    }

    pub fn bbox(&self) -> Rect {
        self.bbox
    }

    fn expand_bbox(&mut self, rect: Rect) {
        self.bbox = self.bbox.union(&rect);
    }
}

impl Default for BBoxDevice {
    fn default() -> Self {
        Self::new()
    }
}

impl Device for BBoxDevice {
    fn fill_path(&mut self, path: &Path, _: bool, ctm: &Matrix, _: &Colorspace, _: &[f32], _: f32) {
        let bbox = path.bounds().transform(ctm);
        self.expand_bbox(bbox);
    }

    fn stroke_path(
        &mut self,
        path: &Path,
        _stroke: &StrokeState,
        ctm: &Matrix,
        _: &Colorspace,
        _: &[f32],
        _: f32,
    ) {
        // For now, just use path bounds (should expand by stroke width)
        let bbox = path.bounds().transform(ctm);
        self.expand_bbox(bbox);
    }

    fn clip_path(&mut self, _: &Path, _: bool, _: &Matrix, _: Rect) {}

    fn clip_stroke_path(&mut self, _: &Path, _: &StrokeState, _: &Matrix, _: Rect) {}

    fn fill_text(&mut self, text: &Text, ctm: &Matrix, _: &Colorspace, _: &[f32], _: f32) {
        let bbox = text.bounds(None, ctm);
        self.expand_bbox(bbox);
    }

    fn stroke_text(
        &mut self,
        text: &Text,
        stroke: &StrokeState,
        ctm: &Matrix,
        _: &Colorspace,
        _: &[f32],
        _: f32,
    ) {
        let bbox = text.bounds(Some(stroke), ctm);
        self.expand_bbox(bbox);
    }

    fn clip_text(&mut self, _: &Text, _: &Matrix, _: Rect) {}

    fn clip_stroke_text(&mut self, _: &Text, _: &StrokeState, _: &Matrix, _: Rect) {}

    fn ignore_text(&mut self, _: &Text, _: &Matrix) {}

    fn fill_image(&mut self, _: &Image, ctm: &Matrix, _: f32) {
        // Image fills unit square
        let bbox = Rect::UNIT.transform(ctm);
        self.expand_bbox(bbox);
    }

    fn fill_image_mask(&mut self, _: &Image, ctm: &Matrix, _: &Colorspace, _: &[f32], _: f32) {
        let bbox = Rect::UNIT.transform(ctm);
        self.expand_bbox(bbox);
    }

    fn clip_image_mask(&mut self, _: &Image, _: &Matrix, _: Rect) {}

    fn pop_clip(&mut self) {}

    fn begin_mask(&mut self, _: Rect, _: bool, _: &Colorspace, _: &[f32]) {}

    fn end_mask(&mut self) {}

    fn begin_group(
        &mut self,
        _: Rect,
        _: Option<&Colorspace>,
        _: bool,
        _: bool,
        _: BlendMode,
        _: f32,
    ) {
    }

    fn end_group(&mut self) {}

    fn begin_tile(&mut self, _: Rect, _: Rect, _: f32, _: f32, _: &Matrix) -> i32 {
        0
    }

    fn end_tile(&mut self) {}
}

/// Trace device - logs all operations for debugging
pub struct TraceDevice {
    indent: usize,
}

impl TraceDevice {
    pub fn new() -> Self {
        Self { indent: 0 }
    }

    fn log(&self, msg: &str) {
        println!("{}{}", "  ".repeat(self.indent), msg);
    }
}

impl Default for TraceDevice {
    fn default() -> Self {
        Self::new()
    }
}

impl Device for TraceDevice {
    fn fill_path(
        &mut self,
        _: &Path,
        even_odd: bool,
        ctm: &Matrix,
        _: &Colorspace,
        color: &[f32],
        alpha: f32,
    ) {
        self.log(&format!(
            "fill_path even_odd={} ctm={:?} color={:?} alpha={}",
            even_odd, ctm, color, alpha
        ));
    }

    fn stroke_path(
        &mut self,
        _: &Path,
        stroke: &StrokeState,
        ctm: &Matrix,
        _: &Colorspace,
        color: &[f32],
        alpha: f32,
    ) {
        self.log(&format!(
            "stroke_path width={} ctm={:?} color={:?} alpha={}",
            stroke.linewidth, ctm, color, alpha
        ));
    }

    fn clip_path(&mut self, _: &Path, even_odd: bool, ctm: &Matrix, scissor: Rect) {
        self.log(&format!(
            "clip_path even_odd={} ctm={:?} scissor={:?}",
            even_odd, ctm, scissor
        ));
        self.indent += 1;
    }

    fn clip_stroke_path(&mut self, _: &Path, _: &StrokeState, ctm: &Matrix, scissor: Rect) {
        self.log(&format!(
            "clip_stroke_path ctm={:?} scissor={:?}",
            ctm, scissor
        ));
        self.indent += 1;
    }

    fn fill_text(&mut self, text: &Text, ctm: &Matrix, _: &Colorspace, color: &[f32], alpha: f32) {
        let content = text.text_content();
        self.log(&format!(
            "fill_text '{}' ctm={:?} color={:?} alpha={}",
            content, ctm, color, alpha
        ));
    }

    fn stroke_text(
        &mut self,
        text: &Text,
        stroke: &StrokeState,
        ctm: &Matrix,
        _: &Colorspace,
        color: &[f32],
        alpha: f32,
    ) {
        let content = text.text_content();
        self.log(&format!(
            "stroke_text '{}' width={} ctm={:?} color={:?} alpha={}",
            content, stroke.linewidth, ctm, color, alpha
        ));
    }

    fn clip_text(&mut self, text: &Text, ctm: &Matrix, scissor: Rect) {
        let content = text.text_content();
        self.log(&format!(
            "clip_text '{}' ctm={:?} scissor={:?}",
            content, ctm, scissor
        ));
        self.indent += 1;
    }

    fn clip_stroke_text(&mut self, text: &Text, _: &StrokeState, ctm: &Matrix, scissor: Rect) {
        let content = text.text_content();
        self.log(&format!(
            "clip_stroke_text '{}' ctm={:?} scissor={:?}",
            content, ctm, scissor
        ));
        self.indent += 1;
    }

    fn ignore_text(&mut self, text: &Text, ctm: &Matrix) {
        let content = text.text_content();
        self.log(&format!("ignore_text '{}' ctm={:?}", content, ctm));
    }

    fn fill_image(&mut self, _: &Image, ctm: &Matrix, alpha: f32) {
        self.log(&format!("fill_image ctm={:?} alpha={}", ctm, alpha));
    }

    fn fill_image_mask(
        &mut self,
        _: &Image,
        ctm: &Matrix,
        _: &Colorspace,
        color: &[f32],
        alpha: f32,
    ) {
        self.log(&format!(
            "fill_image_mask ctm={:?} color={:?} alpha={}",
            ctm, color, alpha
        ));
    }

    fn clip_image_mask(&mut self, _: &Image, ctm: &Matrix, scissor: Rect) {
        self.log(&format!(
            "clip_image_mask ctm={:?} scissor={:?}",
            ctm, scissor
        ));
        self.indent += 1;
    }

    fn pop_clip(&mut self) {
        if self.indent > 0 {
            self.indent -= 1;
        }
        self.log("pop_clip");
    }

    fn begin_mask(&mut self, area: Rect, luminosity: bool, _: &Colorspace, _: &[f32]) {
        self.log(&format!(
            "begin_mask area={:?} luminosity={}",
            area, luminosity
        ));
        self.indent += 1;
    }

    fn end_mask(&mut self) {
        if self.indent > 0 {
            self.indent -= 1;
        }
        self.log("end_mask");
    }

    fn begin_group(
        &mut self,
        area: Rect,
        _: Option<&Colorspace>,
        isolated: bool,
        knockout: bool,
        blendmode: BlendMode,
        alpha: f32,
    ) {
        self.log(&format!(
            "begin_group area={:?} isolated={} knockout={} blend={:?} alpha={}",
            area, isolated, knockout, blendmode, alpha
        ));
        self.indent += 1;
    }

    fn end_group(&mut self) {
        if self.indent > 0 {
            self.indent -= 1;
        }
        self.log("end_group");
    }

    fn begin_tile(&mut self, area: Rect, view: Rect, xstep: f32, ystep: f32, ctm: &Matrix) -> i32 {
        self.log(&format!(
            "begin_tile area={:?} view={:?} step=({},{}) ctm={:?}",
            area, view, xstep, ystep, ctm
        ));
        self.indent += 1;
        0
    }

    fn end_tile(&mut self) {
        if self.indent > 0 {
            self.indent -= 1;
        }
        self.log("end_tile");
    }

    fn close(&mut self) {
        self.log("close");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fitz::font::Font;
    use std::sync::Arc;

    #[test]
    fn test_blend_mode_from_name() {
        assert_eq!(BlendMode::from_name("Normal"), Some(BlendMode::Normal));
        assert_eq!(BlendMode::from_name("Multiply"), Some(BlendMode::Multiply));
        assert_eq!(BlendMode::from_name("Invalid"), None);
    }

    #[test]
    fn test_blend_mode_name() {
        assert_eq!(BlendMode::Normal.name(), "Normal");
        assert_eq!(BlendMode::Multiply.name(), "Multiply");
        assert_eq!(BlendMode::Overlay.name(), "Overlay");
    }

    #[test]
    fn test_blend_mode_default() {
        assert_eq!(BlendMode::default(), BlendMode::Normal);
    }

    #[test]
    fn test_null_device_fill_path() {
        let mut device = NullDevice;
        let path = Path::new();
        let cs = Colorspace::device_rgb();
        let color = [1.0, 0.0, 0.0];

        // Should not panic
        device.fill_path(&path, false, &Matrix::IDENTITY, &cs, &color, 1.0);
    }

    #[test]
    fn test_null_device_stroke_path() {
        let mut device = NullDevice;
        let path = Path::new();
        let cs = Colorspace::device_rgb();
        let color = [0.0, 1.0, 0.0];
        let stroke = StrokeState::default();

        // Should not panic
        device.stroke_path(&path, &stroke, &Matrix::IDENTITY, &cs, &color, 1.0);
    }

    #[test]
    fn test_null_device_fill_text() {
        let mut device = NullDevice;
        let text = Text::new();
        let cs = Colorspace::device_rgb();
        let color = [0.0, 0.0, 0.0];

        // Should not panic
        device.fill_text(&text, &Matrix::IDENTITY, &cs, &color, 1.0);
    }

    #[test]
    fn test_null_device_fill_image() {
        let mut device = NullDevice;
        let image = Image::new(100, 100, None);

        // Should not panic
        device.fill_image(&image, &Matrix::IDENTITY, 1.0);
    }

    #[test]
    fn test_bbox_device_empty() {
        let device = BBoxDevice::new();
        assert!(device.bbox().is_empty());
    }

    #[test]
    fn test_bbox_device_fill_path() {
        use crate::fitz::geometry::Point;

        let mut device = BBoxDevice::new();
        let mut path = Path::new();
        path.move_to(Point::new(10.0, 10.0));
        path.line_to(Point::new(100.0, 100.0));

        let cs = Colorspace::device_rgb();
        let color = [1.0, 0.0, 0.0];

        device.fill_path(&path, false, &Matrix::IDENTITY, &cs, &color, 1.0);

        let bbox = device.bbox();
        assert!(!bbox.is_empty());
    }

    #[test]
    fn test_bbox_device_fill_text() {
        let mut device = BBoxDevice::new();
        let mut text = Text::new();
        let font = Arc::new(Font::new("TestFont"));

        text.show_string(
            font,
            Matrix::IDENTITY,
            "Hello",
            false,
            0,
            crate::fitz::text::BidiDirection::Ltr,
            crate::fitz::text::TextLanguage::Unset,
        );

        let cs = Colorspace::device_rgb();
        let color = [0.0, 0.0, 0.0];

        device.fill_text(&text, &Matrix::IDENTITY, &cs, &color, 1.0);

        let bbox = device.bbox();
        assert!(!bbox.is_empty());
    }

    #[test]
    fn test_bbox_device_fill_image() {
        let mut device = BBoxDevice::new();
        let image = Image::new(100, 100, None);

        device.fill_image(&image, &Matrix::IDENTITY, 1.0);

        let bbox = device.bbox();
        assert!(!bbox.is_empty());
        assert_eq!(bbox, Rect::UNIT);
    }

    #[test]
    fn test_bbox_device_multiple_operations() {
        use crate::fitz::geometry::Point;

        let mut device = BBoxDevice::new();

        // Add path
        let mut path = Path::new();
        path.move_to(Point::new(0.0, 0.0));
        path.line_to(Point::new(50.0, 50.0));
        let cs = Colorspace::device_rgb();
        device.fill_path(&path, false, &Matrix::IDENTITY, &cs, &[1.0, 0.0, 0.0], 1.0);

        // Add image at different location
        let transform = Matrix::translate(100.0, 100.0);
        device.fill_image(&Image::new(10, 10, None), &transform, 1.0);

        let bbox = device.bbox();
        assert!(!bbox.is_empty());
        // Should encompass both operations
        assert!(bbox.x1 > 50.0);
        assert!(bbox.y1 > 50.0);
    }

    #[test]
    fn test_trace_device() {
        let mut device = TraceDevice::new();
        let path = Path::new();
        let cs = Colorspace::device_rgb();
        let color = [1.0, 0.0, 0.0];

        // Should not panic, logs to stdout
        device.fill_path(&path, false, &Matrix::IDENTITY, &cs, &color, 1.0);
        device.close();
    }

    #[test]
    fn test_trace_device_indentation() {
        let mut device = TraceDevice::new();
        assert_eq!(device.indent, 0);

        let path = Path::new();
        device.clip_path(&path, false, &Matrix::IDENTITY, Rect::EMPTY);
        assert_eq!(device.indent, 1);

        device.pop_clip();
        assert_eq!(device.indent, 0);
    }

    #[test]
    fn test_container_type() {
        let clip = ContainerType::Clip;
        let mask = ContainerType::Mask { luminosity: true };
        let group = ContainerType::Group {
            isolated: true,
            knockout: false,
        };
        let tile = ContainerType::Tile;

        // Just ensure they can be created
        assert!(matches!(clip, ContainerType::Clip));
        assert!(matches!(mask, ContainerType::Mask { .. }));
        assert!(matches!(group, ContainerType::Group { .. }));
        assert!(matches!(tile, ContainerType::Tile));
    }
}
