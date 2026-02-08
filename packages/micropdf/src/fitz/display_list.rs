//! Display list - record and replay drawing operations
//!
//! Display lists capture drawing operations for caching and multi-threaded rendering.

use crate::fitz::colorspace::Colorspace;
use crate::fitz::device::{BlendMode, Device};
use crate::fitz::geometry::{Matrix, Rect};
use crate::fitz::image::Image;
use crate::fitz::path::{Path, StrokeState};
use crate::fitz::text::Text;

/// Display list command
#[derive(Clone)]
enum Command {
    FillPath {
        path: Path,
        even_odd: bool,
        ctm: Matrix,
        colorspace: Colorspace,
        color: Vec<f32>,
        alpha: f32,
    },
    StrokePath {
        path: Path,
        stroke: StrokeState,
        ctm: Matrix,
        colorspace: Colorspace,
        color: Vec<f32>,
        alpha: f32,
    },
    ClipPath {
        path: Path,
        even_odd: bool,
        ctm: Matrix,
        scissor: Rect,
    },
    ClipStrokePath {
        path: Path,
        stroke: StrokeState,
        ctm: Matrix,
        scissor: Rect,
    },
    FillText {
        text: Text,
        ctm: Matrix,
        colorspace: Colorspace,
        color: Vec<f32>,
        alpha: f32,
    },
    StrokeText {
        text: Text,
        stroke: StrokeState,
        ctm: Matrix,
        colorspace: Colorspace,
        color: Vec<f32>,
        alpha: f32,
    },
    ClipText {
        text: Text,
        ctm: Matrix,
        scissor: Rect,
    },
    ClipStrokeText {
        text: Text,
        stroke: StrokeState,
        ctm: Matrix,
        scissor: Rect,
    },
    IgnoreText {
        text: Text,
        ctm: Matrix,
    },
    FillImage {
        image: Image,
        ctm: Matrix,
        alpha: f32,
    },
    FillImageMask {
        image: Image,
        ctm: Matrix,
        colorspace: Colorspace,
        color: Vec<f32>,
        alpha: f32,
    },
    ClipImageMask {
        image: Image,
        ctm: Matrix,
        scissor: Rect,
    },
    PopClip,
    BeginMask {
        area: Rect,
        luminosity: bool,
        colorspace: Colorspace,
        color: Vec<f32>,
    },
    EndMask,
    BeginGroup {
        area: Rect,
        colorspace: Option<Colorspace>,
        isolated: bool,
        knockout: bool,
        blendmode: BlendMode,
        alpha: f32,
    },
    EndGroup,
    BeginTile {
        area: Rect,
        view: Rect,
        xstep: f32,
        ystep: f32,
        ctm: Matrix,
    },
    EndTile,
}

/// Display list - records drawing operations for playback
#[derive(Clone)]
pub struct DisplayList {
    mediabox: Rect,
    commands: Vec<Command>,
}

impl DisplayList {
    /// Create a new empty display list
    pub fn new(mediabox: Rect) -> Self {
        Self {
            mediabox,
            commands: Vec::new(),
        }
    }

    /// Get the media box
    pub fn mediabox(&self) -> Rect {
        self.mediabox
    }

    /// Get number of commands
    pub fn len(&self) -> usize {
        self.commands.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    /// Run the display list through a device
    pub fn run(&self, device: &mut dyn Device, ctm: &Matrix, scissor: Rect) {
        for cmd in &self.commands {
            // Check if command is within scissor bounds (simplified)
            match cmd {
                Command::FillPath {
                    path,
                    even_odd,
                    ctm: cmd_ctm,
                    colorspace,
                    color,
                    alpha,
                } => {
                    let final_ctm = cmd_ctm.concat(ctm);
                    device.fill_path(path, *even_odd, &final_ctm, colorspace, color, *alpha);
                }
                Command::StrokePath {
                    path,
                    stroke,
                    ctm: cmd_ctm,
                    colorspace,
                    color,
                    alpha,
                } => {
                    let final_ctm = cmd_ctm.concat(ctm);
                    device.stroke_path(path, stroke, &final_ctm, colorspace, color, *alpha);
                }
                Command::ClipPath {
                    path,
                    even_odd,
                    ctm: cmd_ctm,
                    scissor: cmd_scissor,
                } => {
                    let final_ctm = cmd_ctm.concat(ctm);
                    let final_scissor = scissor.intersect(cmd_scissor);
                    device.clip_path(path, *even_odd, &final_ctm, final_scissor);
                }
                Command::ClipStrokePath {
                    path,
                    stroke,
                    ctm: cmd_ctm,
                    scissor: cmd_scissor,
                } => {
                    let final_ctm = cmd_ctm.concat(ctm);
                    let final_scissor = scissor.intersect(cmd_scissor);
                    device.clip_stroke_path(path, stroke, &final_ctm, final_scissor);
                }
                Command::FillText {
                    text,
                    ctm: cmd_ctm,
                    colorspace,
                    color,
                    alpha,
                } => {
                    let final_ctm = cmd_ctm.concat(ctm);
                    device.fill_text(text, &final_ctm, colorspace, color, *alpha);
                }
                Command::StrokeText {
                    text,
                    stroke,
                    ctm: cmd_ctm,
                    colorspace,
                    color,
                    alpha,
                } => {
                    let final_ctm = cmd_ctm.concat(ctm);
                    device.stroke_text(text, stroke, &final_ctm, colorspace, color, *alpha);
                }
                Command::ClipText {
                    text,
                    ctm: cmd_ctm,
                    scissor: cmd_scissor,
                } => {
                    let final_ctm = cmd_ctm.concat(ctm);
                    let final_scissor = scissor.intersect(cmd_scissor);
                    device.clip_text(text, &final_ctm, final_scissor);
                }
                Command::ClipStrokeText {
                    text,
                    stroke,
                    ctm: cmd_ctm,
                    scissor: cmd_scissor,
                } => {
                    let final_ctm = cmd_ctm.concat(ctm);
                    let final_scissor = scissor.intersect(cmd_scissor);
                    device.clip_stroke_text(text, stroke, &final_ctm, final_scissor);
                }
                Command::IgnoreText { text, ctm: cmd_ctm } => {
                    let final_ctm = cmd_ctm.concat(ctm);
                    device.ignore_text(text, &final_ctm);
                }
                Command::FillImage {
                    image,
                    ctm: cmd_ctm,
                    alpha,
                } => {
                    let final_ctm = cmd_ctm.concat(ctm);
                    device.fill_image(image, &final_ctm, *alpha);
                }
                Command::FillImageMask {
                    image,
                    ctm: cmd_ctm,
                    colorspace,
                    color,
                    alpha,
                } => {
                    let final_ctm = cmd_ctm.concat(ctm);
                    device.fill_image_mask(image, &final_ctm, colorspace, color, *alpha);
                }
                Command::ClipImageMask {
                    image,
                    ctm: cmd_ctm,
                    scissor: cmd_scissor,
                } => {
                    let final_ctm = cmd_ctm.concat(ctm);
                    let final_scissor = scissor.intersect(cmd_scissor);
                    device.clip_image_mask(image, &final_ctm, final_scissor);
                }
                Command::PopClip => {
                    device.pop_clip();
                }
                Command::BeginMask {
                    area,
                    luminosity,
                    colorspace,
                    color,
                } => {
                    device.begin_mask(*area, *luminosity, colorspace, color);
                }
                Command::EndMask => {
                    device.end_mask();
                }
                Command::BeginGroup {
                    area,
                    colorspace,
                    isolated,
                    knockout,
                    blendmode,
                    alpha,
                } => {
                    device.begin_group(
                        *area,
                        colorspace.as_ref(),
                        *isolated,
                        *knockout,
                        *blendmode,
                        *alpha,
                    );
                }
                Command::EndGroup => {
                    device.end_group();
                }
                Command::BeginTile {
                    area,
                    view,
                    xstep,
                    ystep,
                    ctm: cmd_ctm,
                } => {
                    let final_ctm = cmd_ctm.concat(ctm);
                    device.begin_tile(*area, *view, *xstep, *ystep, &final_ctm);
                }
                Command::EndTile => {
                    device.end_tile();
                }
            }
        }
    }

    /// Clear all commands
    pub fn clear(&mut self) {
        self.commands.clear();
    }
}

/// List device - records drawing operations to a display list
pub struct ListDevice {
    list: DisplayList,
}

impl ListDevice {
    /// Create a new list device
    pub fn new(mediabox: Rect) -> Self {
        Self {
            list: DisplayList::new(mediabox),
        }
    }

    /// Get the display list
    pub fn into_display_list(self) -> DisplayList {
        self.list
    }

    /// Get a reference to the display list
    pub fn display_list(&self) -> &DisplayList {
        &self.list
    }

    /// Get a mutable reference to the display list
    pub fn display_list_mut(&mut self) -> &mut DisplayList {
        &mut self.list
    }
}

impl Device for ListDevice {
    fn fill_path(
        &mut self,
        path: &Path,
        even_odd: bool,
        ctm: &Matrix,
        colorspace: &Colorspace,
        color: &[f32],
        alpha: f32,
    ) {
        self.list.commands.push(Command::FillPath {
            path: path.clone(),
            even_odd,
            ctm: *ctm,
            colorspace: colorspace.clone(),
            color: color.to_vec(),
            alpha,
        });
    }

    fn stroke_path(
        &mut self,
        path: &Path,
        stroke: &StrokeState,
        ctm: &Matrix,
        colorspace: &Colorspace,
        color: &[f32],
        alpha: f32,
    ) {
        self.list.commands.push(Command::StrokePath {
            path: path.clone(),
            stroke: stroke.clone(),
            ctm: *ctm,
            colorspace: colorspace.clone(),
            color: color.to_vec(),
            alpha,
        });
    }

    fn clip_path(&mut self, path: &Path, even_odd: bool, ctm: &Matrix, scissor: Rect) {
        self.list.commands.push(Command::ClipPath {
            path: path.clone(),
            even_odd,
            ctm: *ctm,
            scissor,
        });
    }

    fn clip_stroke_path(&mut self, path: &Path, stroke: &StrokeState, ctm: &Matrix, scissor: Rect) {
        self.list.commands.push(Command::ClipStrokePath {
            path: path.clone(),
            stroke: stroke.clone(),
            ctm: *ctm,
            scissor,
        });
    }

    fn fill_text(
        &mut self,
        text: &Text,
        ctm: &Matrix,
        colorspace: &Colorspace,
        color: &[f32],
        alpha: f32,
    ) {
        self.list.commands.push(Command::FillText {
            text: text.clone(),
            ctm: *ctm,
            colorspace: colorspace.clone(),
            color: color.to_vec(),
            alpha,
        });
    }

    fn stroke_text(
        &mut self,
        text: &Text,
        stroke: &StrokeState,
        ctm: &Matrix,
        colorspace: &Colorspace,
        color: &[f32],
        alpha: f32,
    ) {
        self.list.commands.push(Command::StrokeText {
            text: text.clone(),
            stroke: stroke.clone(),
            ctm: *ctm,
            colorspace: colorspace.clone(),
            color: color.to_vec(),
            alpha,
        });
    }

    fn clip_text(&mut self, text: &Text, ctm: &Matrix, scissor: Rect) {
        self.list.commands.push(Command::ClipText {
            text: text.clone(),
            ctm: *ctm,
            scissor,
        });
    }

    fn clip_stroke_text(&mut self, text: &Text, stroke: &StrokeState, ctm: &Matrix, scissor: Rect) {
        self.list.commands.push(Command::ClipStrokeText {
            text: text.clone(),
            stroke: stroke.clone(),
            ctm: *ctm,
            scissor,
        });
    }

    fn ignore_text(&mut self, text: &Text, ctm: &Matrix) {
        self.list.commands.push(Command::IgnoreText {
            text: text.clone(),
            ctm: *ctm,
        });
    }

    fn fill_image(&mut self, image: &Image, ctm: &Matrix, alpha: f32) {
        self.list.commands.push(Command::FillImage {
            image: image.clone(),
            ctm: *ctm,
            alpha,
        });
    }

    fn fill_image_mask(
        &mut self,
        image: &Image,
        ctm: &Matrix,
        colorspace: &Colorspace,
        color: &[f32],
        alpha: f32,
    ) {
        self.list.commands.push(Command::FillImageMask {
            image: image.clone(),
            ctm: *ctm,
            colorspace: colorspace.clone(),
            color: color.to_vec(),
            alpha,
        });
    }

    fn clip_image_mask(&mut self, image: &Image, ctm: &Matrix, scissor: Rect) {
        self.list.commands.push(Command::ClipImageMask {
            image: image.clone(),
            ctm: *ctm,
            scissor,
        });
    }

    fn pop_clip(&mut self) {
        self.list.commands.push(Command::PopClip);
    }

    fn begin_mask(&mut self, area: Rect, luminosity: bool, colorspace: &Colorspace, color: &[f32]) {
        self.list.commands.push(Command::BeginMask {
            area,
            luminosity,
            colorspace: colorspace.clone(),
            color: color.to_vec(),
        });
    }

    fn end_mask(&mut self) {
        self.list.commands.push(Command::EndMask);
    }

    fn begin_group(
        &mut self,
        area: Rect,
        colorspace: Option<&Colorspace>,
        isolated: bool,
        knockout: bool,
        blendmode: BlendMode,
        alpha: f32,
    ) {
        self.list.commands.push(Command::BeginGroup {
            area,
            colorspace: colorspace.cloned(),
            isolated,
            knockout,
            blendmode,
            alpha,
        });
    }

    fn end_group(&mut self) {
        self.list.commands.push(Command::EndGroup);
    }

    fn begin_tile(&mut self, area: Rect, view: Rect, xstep: f32, ystep: f32, ctm: &Matrix) -> i32 {
        self.list.commands.push(Command::BeginTile {
            area,
            view,
            xstep,
            ystep,
            ctm: *ctm,
        });
        0
    }

    fn end_tile(&mut self) {
        self.list.commands.push(Command::EndTile);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fitz::device::{BBoxDevice, NullDevice};
    use crate::fitz::font::Font;
    use std::sync::Arc;

    #[test]
    fn test_display_list_new() {
        let mediabox = Rect::new(0.0, 0.0, 612.0, 792.0);
        let list = DisplayList::new(mediabox);

        assert_eq!(list.mediabox(), mediabox);
        assert!(list.is_empty());
        assert_eq!(list.len(), 0);
    }

    #[test]
    fn test_list_device_record_fill_path() {
        use crate::fitz::geometry::Point;

        let mediabox = Rect::new(0.0, 0.0, 100.0, 100.0);
        let mut device = ListDevice::new(mediabox);

        let mut path = Path::new();
        path.move_to(Point::new(10.0, 10.0));
        path.line_to(Point::new(90.0, 90.0));

        let cs = Colorspace::device_rgb();
        let color = [1.0, 0.0, 0.0];

        device.fill_path(&path, false, &Matrix::IDENTITY, &cs, &color, 1.0);

        assert_eq!(device.display_list().len(), 1);
    }

    #[test]
    fn test_list_device_record_fill_text() {
        let mediabox = Rect::new(0.0, 0.0, 100.0, 100.0);
        let mut device = ListDevice::new(mediabox);

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

        assert_eq!(device.display_list().len(), 1);
    }

    #[test]
    fn test_list_device_record_fill_image() {
        let mediabox = Rect::new(0.0, 0.0, 100.0, 100.0);
        let mut device = ListDevice::new(mediabox);

        let image = Image::new(50, 50, None);
        device.fill_image(&image, &Matrix::IDENTITY, 1.0);

        assert_eq!(device.display_list().len(), 1);
    }

    #[test]
    fn test_list_device_record_multiple_operations() {
        let mediabox = Rect::new(0.0, 0.0, 100.0, 100.0);
        let mut device = ListDevice::new(mediabox);

        // Fill path
        let path = Path::new();
        let cs = Colorspace::device_rgb();
        device.fill_path(&path, false, &Matrix::IDENTITY, &cs, &[1.0, 0.0, 0.0], 1.0);

        // Fill image
        let image = Image::new(10, 10, None);
        device.fill_image(&image, &Matrix::IDENTITY, 0.5);

        // Fill text
        let text = Text::new();
        device.fill_text(&text, &Matrix::IDENTITY, &cs, &[0.0, 0.0, 0.0], 1.0);

        assert_eq!(device.display_list().len(), 3);
    }

    #[test]
    fn test_display_list_run_on_null_device() {
        let mediabox = Rect::new(0.0, 0.0, 100.0, 100.0);
        let mut list_device = ListDevice::new(mediabox);

        // Record some operations
        let path = Path::new();
        let cs = Colorspace::device_rgb();
        list_device.fill_path(&path, false, &Matrix::IDENTITY, &cs, &[1.0, 0.0, 0.0], 1.0);

        let list = list_device.into_display_list();

        // Run on null device (should not panic)
        let mut null_device = NullDevice;
        list.run(&mut null_device, &Matrix::IDENTITY, Rect::INFINITE);
    }

    #[test]
    fn test_display_list_run_on_bbox_device() {
        use crate::fitz::geometry::Point;

        let mediabox = Rect::new(0.0, 0.0, 100.0, 100.0);
        let mut list_device = ListDevice::new(mediabox);

        // Record a path operation
        let mut path = Path::new();
        path.move_to(Point::new(10.0, 10.0));
        path.line_to(Point::new(50.0, 50.0));

        let cs = Colorspace::device_rgb();
        list_device.fill_path(&path, false, &Matrix::IDENTITY, &cs, &[1.0, 0.0, 0.0], 1.0);

        let list = list_device.into_display_list();

        // Run on bbox device
        let mut bbox_device = BBoxDevice::new();
        list.run(&mut bbox_device, &Matrix::IDENTITY, Rect::INFINITE);

        let bbox = bbox_device.bbox();
        assert!(!bbox.is_empty());
    }

    #[test]
    fn test_display_list_run_with_transform() {
        let mediabox = Rect::new(0.0, 0.0, 100.0, 100.0);
        let mut list_device = ListDevice::new(mediabox);

        // Record an image
        let image = Image::new(10, 10, None);
        list_device.fill_image(&image, &Matrix::IDENTITY, 1.0);

        let list = list_device.into_display_list();

        // Run with scaling transform
        let mut bbox_device = BBoxDevice::new();
        let scale_ctm = Matrix::scale(2.0, 2.0);
        list.run(&mut bbox_device, &scale_ctm, Rect::INFINITE);

        let bbox = bbox_device.bbox();
        assert!(!bbox.is_empty());
        // Should be scaled
        assert!(bbox.width() >= 2.0);
        assert!(bbox.height() >= 2.0);
    }

    #[test]
    fn test_display_list_clear() {
        let mediabox = Rect::new(0.0, 0.0, 100.0, 100.0);
        let mut device = ListDevice::new(mediabox);

        let path = Path::new();
        let cs = Colorspace::device_rgb();
        device.fill_path(&path, false, &Matrix::IDENTITY, &cs, &[1.0, 0.0, 0.0], 1.0);

        let mut list = device.into_display_list();
        assert!(!list.is_empty());

        list.clear();
        assert!(list.is_empty());
    }

    #[test]
    fn test_display_list_clip_operations() {
        let mediabox = Rect::new(0.0, 0.0, 100.0, 100.0);
        let mut device = ListDevice::new(mediabox);

        let path = Path::new();
        device.clip_path(&path, false, &Matrix::IDENTITY, Rect::EMPTY);
        device.pop_clip();

        assert_eq!(device.display_list().len(), 2);
    }

    #[test]
    fn test_display_list_group_operations() {
        let mediabox = Rect::new(0.0, 0.0, 100.0, 100.0);
        let mut device = ListDevice::new(mediabox);

        let area = Rect::new(10.0, 10.0, 90.0, 90.0);
        let cs = Colorspace::device_rgb();

        device.begin_group(area, Some(&cs), true, false, BlendMode::Normal, 1.0);
        device.end_group();

        assert_eq!(device.display_list().len(), 2);
    }

    #[test]
    fn test_display_list_mask_operations() {
        let mediabox = Rect::new(0.0, 0.0, 100.0, 100.0);
        let mut device = ListDevice::new(mediabox);

        let area = Rect::new(10.0, 10.0, 90.0, 90.0);
        let cs = Colorspace::device_rgb();
        let color = [0.0, 0.0, 0.0];

        device.begin_mask(area, false, &cs, &color);
        device.end_mask();

        assert_eq!(device.display_list().len(), 2);
    }

    #[test]
    fn test_display_list_tile_operations() {
        let mediabox = Rect::new(0.0, 0.0, 100.0, 100.0);
        let mut device = ListDevice::new(mediabox);

        let area = Rect::new(0.0, 0.0, 10.0, 10.0);
        let view = Rect::new(0.0, 0.0, 10.0, 10.0);

        device.begin_tile(area, view, 10.0, 10.0, &Matrix::IDENTITY);
        device.end_tile();

        assert_eq!(device.display_list().len(), 2);
    }
}
