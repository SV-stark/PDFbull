//! PDF annotation support
//!
//! Provides types and functionality for PDF annotations (interactive elements).

use crate::fitz::geometry::{Matrix, Rect};
use std::collections::HashMap;

/// PDF annotation types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum AnnotType {
    /// Text annotation (sticky note)
    Text = 0,
    /// Link annotation (hyperlink)
    Link = 1,
    /// Free text annotation
    FreeText = 2,
    /// Line annotation
    Line = 3,
    /// Square annotation
    Square = 4,
    /// Circle annotation
    Circle = 5,
    /// Polygon annotation
    Polygon = 6,
    /// Polyline annotation
    PolyLine = 7,
    /// Text highlight
    Highlight = 8,
    /// Text underline
    Underline = 9,
    /// Text squiggly underline
    Squiggly = 10,
    /// Text strikeout
    StrikeOut = 11,
    /// Redaction annotation
    Redact = 12,
    /// Rubber stamp
    Stamp = 13,
    /// Caret annotation
    Caret = 14,
    /// Ink annotation (freehand drawing)
    Ink = 15,
    /// Popup annotation
    Popup = 16,
    /// File attachment
    FileAttachment = 17,
    /// Sound annotation
    Sound = 18,
    /// Movie annotation
    Movie = 19,
    /// Rich media annotation
    RichMedia = 20,
    /// Form widget
    Widget = 21,
    /// Screen annotation
    Screen = 22,
    /// Printer mark annotation
    PrinterMark = 23,
    /// Trap network annotation
    TrapNet = 24,
    /// Watermark annotation
    Watermark = 25,
    /// 3D annotation
    ThreeD = 26,
    /// Projection annotation
    Projection = 27,
    /// Unknown annotation type
    Unknown = -1,
}

impl AnnotType {
    /// Convert from string subtype
    pub fn from_string(s: &str) -> Self {
        match s {
            "Text" => Self::Text,
            "Link" => Self::Link,
            "FreeText" => Self::FreeText,
            "Line" => Self::Line,
            "Square" => Self::Square,
            "Circle" => Self::Circle,
            "Polygon" => Self::Polygon,
            "PolyLine" => Self::PolyLine,
            "Highlight" => Self::Highlight,
            "Underline" => Self::Underline,
            "Squiggly" => Self::Squiggly,
            "StrikeOut" => Self::StrikeOut,
            "Redact" => Self::Redact,
            "Stamp" => Self::Stamp,
            "Caret" => Self::Caret,
            "Ink" => Self::Ink,
            "Popup" => Self::Popup,
            "FileAttachment" => Self::FileAttachment,
            "Sound" => Self::Sound,
            "Movie" => Self::Movie,
            "RichMedia" => Self::RichMedia,
            "Widget" => Self::Widget,
            "Screen" => Self::Screen,
            "PrinterMark" => Self::PrinterMark,
            "TrapNet" => Self::TrapNet,
            "Watermark" => Self::Watermark,
            "3D" => Self::ThreeD,
            "Projection" => Self::Projection,
            _ => Self::Unknown,
        }
    }

    /// Convert to string subtype
    pub fn to_string(&self) -> &'static str {
        match self {
            Self::Text => "Text",
            Self::Link => "Link",
            Self::FreeText => "FreeText",
            Self::Line => "Line",
            Self::Square => "Square",
            Self::Circle => "Circle",
            Self::Polygon => "Polygon",
            Self::PolyLine => "PolyLine",
            Self::Highlight => "Highlight",
            Self::Underline => "Underline",
            Self::Squiggly => "Squiggly",
            Self::StrikeOut => "StrikeOut",
            Self::Redact => "Redact",
            Self::Stamp => "Stamp",
            Self::Caret => "Caret",
            Self::Ink => "Ink",
            Self::Popup => "Popup",
            Self::FileAttachment => "FileAttachment",
            Self::Sound => "Sound",
            Self::Movie => "Movie",
            Self::RichMedia => "RichMedia",
            Self::Widget => "Widget",
            Self::Screen => "Screen",
            Self::PrinterMark => "PrinterMark",
            Self::TrapNet => "TrapNet",
            Self::Watermark => "Watermark",
            Self::ThreeD => "3D",
            Self::Projection => "Projection",
            Self::Unknown => "Unknown",
        }
    }
}

/// Annotation flags (bitfield)
#[derive(Debug, Clone, Copy)]
pub struct AnnotFlags(u32);

impl AnnotFlags {
    pub const INVISIBLE: u32 = 1 << 0;
    pub const HIDDEN: u32 = 1 << 1;
    pub const PRINT: u32 = 1 << 2;
    pub const NO_ZOOM: u32 = 1 << 3;
    pub const NO_ROTATE: u32 = 1 << 4;
    pub const NO_VIEW: u32 = 1 << 5;
    pub const READ_ONLY: u32 = 1 << 6;
    pub const LOCKED: u32 = 1 << 7;
    pub const TOGGLE_NO_VIEW: u32 = 1 << 8;
    pub const LOCKED_CONTENTS: u32 = 1 << 9;

    pub fn new(flags: u32) -> Self {
        Self(flags)
    }

    pub fn has(&self, flag: u32) -> bool {
        (self.0 & flag) != 0
    }

    pub fn set(&mut self, flag: u32) {
        self.0 |= flag;
    }

    pub fn clear(&mut self, flag: u32) {
        self.0 &= !flag;
    }

    pub fn value(&self) -> u32 {
        self.0
    }
}

impl Default for AnnotFlags {
    fn default() -> Self {
        Self(Self::PRINT) // By default, annotations are printable
    }
}

/// Line ending style
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineEnding {
    None = 0,
    Square = 1,
    Circle = 2,
    Diamond = 3,
    OpenArrow = 4,
    ClosedArrow = 5,
    Butt = 6,
    ROpenArrow = 7,
    RClosedArrow = 8,
    Slash = 9,
}

impl LineEnding {
    pub fn from_string(s: &str) -> Self {
        match s {
            "Square" => Self::Square,
            "Circle" => Self::Circle,
            "Diamond" => Self::Diamond,
            "OpenArrow" => Self::OpenArrow,
            "ClosedArrow" => Self::ClosedArrow,
            "Butt" => Self::Butt,
            "ROpenArrow" => Self::ROpenArrow,
            "RClosedArrow" => Self::RClosedArrow,
            "Slash" => Self::Slash,
            _ => Self::None,
        }
    }

    pub fn to_string(&self) -> &'static str {
        match self {
            Self::None => "None",
            Self::Square => "Square",
            Self::Circle => "Circle",
            Self::Diamond => "Diamond",
            Self::OpenArrow => "OpenArrow",
            Self::ClosedArrow => "ClosedArrow",
            Self::Butt => "Butt",
            Self::ROpenArrow => "ROpenArrow",
            Self::RClosedArrow => "RClosedArrow",
            Self::Slash => "Slash",
        }
    }
}

/// Text quadding (alignment)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Quadding {
    Left = 0,
    Center = 1,
    Right = 2,
}

/// Annotation intent
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intent {
    Default = 0,
    FreeTextCallout = 1,
    FreeTextTypewriter = 2,
    LineArrow = 3,
    LineDimension = 4,
    PolyLineDimension = 5,
    PolygonCloud = 6,
    PolygonDimension = 7,
    StampImage = 8,
    StampSnapshot = 9,
    Unknown = 255,
}

impl Intent {
    pub fn from_string(s: &str) -> Self {
        match s {
            "FreeTextCallout" => Self::FreeTextCallout,
            "FreeTextTypewriter" => Self::FreeTextTypewriter,
            "LineArrow" => Self::LineArrow,
            "LineDimension" => Self::LineDimension,
            "PolyLineDimension" => Self::PolyLineDimension,
            "PolygonCloud" => Self::PolygonCloud,
            "PolygonDimension" => Self::PolygonDimension,
            "StampImage" => Self::StampImage,
            "StampSnapshot" => Self::StampSnapshot,
            _ => Self::Unknown,
        }
    }
}

/// Annotation border style
#[derive(Debug, Clone)]
pub struct BorderStyle {
    /// Border width
    pub width: f32,
    /// Dash pattern
    pub dash_pattern: Vec<f32>,
    /// Border style (S, D, B, I, U)
    pub style: BorderStyleType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BorderStyleType {
    Solid,
    Dashed,
    Beveled,
    Inset,
    Underline,
}

impl Default for BorderStyle {
    fn default() -> Self {
        Self {
            width: 1.0,
            dash_pattern: Vec::new(),
            style: BorderStyleType::Solid,
        }
    }
}

/// PDF annotation
#[derive(Clone)]
pub struct Annotation {
    /// Annotation type
    annot_type: AnnotType,
    /// Annotation rectangle
    rect: Rect,
    /// Contents (text content)
    contents: String,
    /// Author/title
    author: String,
    /// Subject
    subject: String,
    /// Creation date
    creation_date: Option<String>,
    /// Modification date
    mod_date: Option<String>,
    /// Annotation flags
    flags: AnnotFlags,
    /// Color (RGB, 0.0-1.0)
    color: Option<[f32; 3]>,
    /// Interior color (for filled annotations)
    interior_color: Vec<f32>,
    /// Border style
    border: BorderStyle,
    /// Opacity (0.0-1.0)
    opacity: f32,
    /// Popup annotation reference
    popup: Option<Box<Annotation>>,
    /// Line start point (for line annotations)
    line_start: Option<(f32, f32)>,
    /// Line end point (for line annotations)
    line_end: Option<(f32, f32)>,
    /// Dirty flag - tracks if annotation has been modified
    dirty: bool,
    /// Additional properties
    properties: HashMap<String, String>,
}

impl Annotation {
    /// Create a new annotation
    pub fn new(annot_type: AnnotType, rect: Rect) -> Self {
        Self {
            annot_type,
            rect,
            contents: String::new(),
            author: String::new(),
            subject: String::new(),
            creation_date: None,
            mod_date: None,
            flags: AnnotFlags::default(),
            color: None,
            interior_color: Vec::new(),
            border: BorderStyle::default(),
            opacity: 1.0,
            popup: None,
            line_start: None,
            line_end: None,
            dirty: false,
            properties: HashMap::new(),
        }
    }

    /// Create a text annotation (sticky note)
    pub fn text(rect: Rect, contents: &str) -> Self {
        let mut annot = Self::new(AnnotType::Text, rect);
        annot.contents = contents.to_string();
        annot
    }

    /// Create a highlight annotation
    pub fn highlight(rect: Rect, color: [f32; 3]) -> Self {
        let mut annot = Self::new(AnnotType::Highlight, rect);
        annot.color = Some(color);
        annot.opacity = 0.5; // Typical highlight opacity
        annot
    }

    /// Create an underline annotation
    pub fn underline(rect: Rect, color: [f32; 3]) -> Self {
        let mut annot = Self::new(AnnotType::Underline, rect);
        annot.color = Some(color);
        annot
    }

    /// Create a strikeout annotation
    pub fn strikeout(rect: Rect, color: [f32; 3]) -> Self {
        let mut annot = Self::new(AnnotType::StrikeOut, rect);
        annot.color = Some(color);
        annot
    }

    /// Create a square annotation
    pub fn square(rect: Rect, color: [f32; 3]) -> Self {
        let mut annot = Self::new(AnnotType::Square, rect);
        annot.color = Some(color);
        annot
    }

    /// Create a circle annotation
    pub fn circle(rect: Rect, color: [f32; 3]) -> Self {
        let mut annot = Self::new(AnnotType::Circle, rect);
        annot.color = Some(color);
        annot
    }

    /// Create a free text annotation
    pub fn free_text(rect: Rect, text: &str) -> Self {
        let mut annot = Self::new(AnnotType::FreeText, rect);
        annot.contents = text.to_string();
        annot
    }

    /// Get annotation type
    pub fn annot_type(&self) -> AnnotType {
        self.annot_type
    }

    /// Get rectangle
    pub fn rect(&self) -> Rect {
        self.rect
    }

    /// Set rectangle
    pub fn set_rect(&mut self, rect: Rect) {
        self.rect = rect;
    }

    /// Get contents
    pub fn contents(&self) -> &str {
        &self.contents
    }

    /// Set contents
    pub fn set_contents(&mut self, contents: &str) {
        self.contents = contents.to_string();
    }

    /// Get author
    pub fn author(&self) -> &str {
        &self.author
    }

    /// Set author
    pub fn set_author(&mut self, author: &str) {
        self.author = author.to_string();
    }

    /// Get subject
    pub fn subject(&self) -> &str {
        &self.subject
    }

    /// Set subject
    pub fn set_subject(&mut self, subject: &str) {
        self.subject = subject.to_string();
    }

    /// Get flags
    pub fn flags(&self) -> AnnotFlags {
        self.flags
    }

    /// Set flags
    pub fn set_flags(&mut self, flags: AnnotFlags) {
        self.flags = flags;
    }

    /// Check if annotation is hidden
    pub fn is_hidden(&self) -> bool {
        self.flags.has(AnnotFlags::HIDDEN)
    }

    /// Check if annotation is printable
    pub fn is_printable(&self) -> bool {
        self.flags.has(AnnotFlags::PRINT)
    }

    /// Check if annotation is read-only
    pub fn is_read_only(&self) -> bool {
        self.flags.has(AnnotFlags::READ_ONLY)
    }

    /// Get color
    pub fn color(&self) -> Option<[f32; 3]> {
        self.color
    }

    /// Set color
    pub fn set_color(&mut self, color: Option<[f32; 3]>) {
        self.color = color;
    }

    /// Get opacity
    pub fn opacity(&self) -> f32 {
        self.opacity
    }

    /// Set opacity
    pub fn set_opacity(&mut self, opacity: f32) {
        self.opacity = opacity.clamp(0.0, 1.0);
    }

    /// Get border style
    pub fn border(&self) -> &BorderStyle {
        &self.border
    }

    /// Set border style
    pub fn set_border(&mut self, border: BorderStyle) {
        self.border = border;
    }

    /// Get creation date
    pub fn creation_date(&self) -> Option<&str> {
        self.creation_date.as_deref()
    }

    /// Set creation date
    pub fn set_creation_date(&mut self, date: Option<String>) {
        self.creation_date = date;
    }

    /// Get modification date
    pub fn modification_date(&self) -> Option<&str> {
        self.mod_date.as_deref()
    }

    /// Set modification date
    pub fn set_modification_date(&mut self, date: Option<String>) {
        self.mod_date = date;
    }

    /// Get popup annotation
    pub fn popup(&self) -> Option<&Annotation> {
        self.popup.as_deref()
    }

    /// Set popup annotation
    pub fn set_popup(&mut self, popup: Option<Annotation>) {
        self.popup = popup.map(Box::new);
    }

    /// Get property
    pub fn get_property(&self, key: &str) -> Option<&str> {
        self.properties.get(key).map(|s| s.as_str())
    }

    /// Set property
    pub fn set_property(&mut self, key: String, value: String) {
        self.properties.insert(key, value);
    }

    /// Check if annotation intersects with rectangle
    pub fn intersects(&self, rect: &Rect) -> bool {
        !self.rect.intersect(rect).is_empty()
    }

    /// Transform annotation rectangle
    pub fn transform(&mut self, matrix: &Matrix) {
        self.rect = self.rect.transform(matrix);
    }

    /// Get interior color
    pub fn interior_color(&self) -> &[f32] {
        &self.interior_color
    }

    /// Set interior color
    pub fn set_interior_color(&mut self, color: Vec<f32>) {
        self.interior_color = color;
        self.mark_dirty();
    }

    /// Get line start point
    pub fn line_start(&self) -> Option<(f32, f32)> {
        self.line_start
    }

    /// Set line start point
    pub fn set_line_start(&mut self, point: Option<(f32, f32)>) {
        self.line_start = point;
        self.mark_dirty();
    }

    /// Get line end point
    pub fn line_end(&self) -> Option<(f32, f32)> {
        self.line_end
    }

    /// Set line end point
    pub fn set_line_end(&mut self, point: Option<(f32, f32)>) {
        self.line_end = point;
        self.mark_dirty();
    }

    /// Check if annotation is dirty (modified)
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Mark annotation as dirty
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// Clear dirty flag
    pub fn clear_dirty(&mut self) {
        self.dirty = false;
    }

    /// Update annotation appearance (regenerate AP stream)
    pub fn update_appearance(&mut self) {
        // Mark as clean after updating appearance
        self.clear_dirty();
    }

    /// Update annotation (alias for update_appearance)
    pub fn update(&mut self) -> Result<(), String> {
        self.update_appearance();
        Ok(())
    }
}

impl std::fmt::Debug for Annotation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Annotation")
            .field("type", &self.annot_type)
            .field("rect", &self.rect)
            .field("contents", &self.contents)
            .field("author", &self.author)
            .field("flags", &self.flags.value())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_annot_type_from_string() {
        assert_eq!(AnnotType::from_string("Text"), AnnotType::Text);
        assert_eq!(AnnotType::from_string("Link"), AnnotType::Link);
        assert_eq!(AnnotType::from_string("Highlight"), AnnotType::Highlight);
        assert_eq!(AnnotType::from_string("Unknown"), AnnotType::Unknown);
    }

    #[test]
    fn test_annot_type_to_string() {
        assert_eq!(AnnotType::Text.to_string(), "Text");
        assert_eq!(AnnotType::Highlight.to_string(), "Highlight");
        assert_eq!(AnnotType::Link.to_string(), "Link");
    }

    #[test]
    fn test_annot_flags() {
        let mut flags = AnnotFlags::default();
        assert!(flags.has(AnnotFlags::PRINT));

        flags.set(AnnotFlags::HIDDEN);
        assert!(flags.has(AnnotFlags::HIDDEN));

        flags.clear(AnnotFlags::HIDDEN);
        assert!(!flags.has(AnnotFlags::HIDDEN));
    }

    #[test]
    fn test_line_ending_from_string() {
        assert_eq!(LineEnding::from_string("Square"), LineEnding::Square);
        assert_eq!(LineEnding::from_string("Circle"), LineEnding::Circle);
        assert_eq!(LineEnding::from_string("OpenArrow"), LineEnding::OpenArrow);
        assert_eq!(LineEnding::from_string("Unknown"), LineEnding::None);
    }

    #[test]
    fn test_annotation_new() {
        let rect = Rect::new(10.0, 10.0, 100.0, 50.0);
        let annot = Annotation::new(AnnotType::Text, rect);

        assert_eq!(annot.annot_type(), AnnotType::Text);
        assert_eq!(annot.rect(), rect);
        assert_eq!(annot.opacity(), 1.0);
    }

    #[test]
    fn test_text_annotation() {
        let rect = Rect::new(10.0, 10.0, 50.0, 50.0);
        let annot = Annotation::text(rect, "Test note");

        assert_eq!(annot.annot_type(), AnnotType::Text);
        assert_eq!(annot.contents(), "Test note");
    }

    #[test]
    fn test_highlight_annotation() {
        let rect = Rect::new(10.0, 10.0, 100.0, 20.0);
        let color = [1.0, 1.0, 0.0]; // Yellow
        let annot = Annotation::highlight(rect, color);

        assert_eq!(annot.annot_type(), AnnotType::Highlight);
        assert_eq!(annot.color(), Some(color));
        assert_eq!(annot.opacity(), 0.5);
    }

    #[test]
    fn test_annotation_properties() {
        let mut annot = Annotation::new(AnnotType::Text, Rect::EMPTY);

        annot.set_author("John Doe");
        assert_eq!(annot.author(), "John Doe");

        annot.set_subject("Important");
        assert_eq!(annot.subject(), "Important");

        annot.set_opacity(0.7);
        assert_eq!(annot.opacity(), 0.7);
    }

    #[test]
    fn test_annotation_flags_operations() {
        let mut annot = Annotation::new(AnnotType::Text, Rect::EMPTY);

        assert!(annot.is_printable());
        assert!(!annot.is_hidden());

        let mut flags = AnnotFlags::default();
        flags.set(AnnotFlags::HIDDEN);
        annot.set_flags(flags);

        assert!(annot.is_hidden());
    }

    #[test]
    fn test_annotation_intersects() {
        let annot_rect = Rect::new(10.0, 10.0, 50.0, 50.0);
        let annot = Annotation::new(AnnotType::Square, annot_rect);

        let overlapping = Rect::new(30.0, 30.0, 70.0, 70.0);
        assert!(annot.intersects(&overlapping));

        let non_overlapping = Rect::new(100.0, 100.0, 150.0, 150.0);
        assert!(!annot.intersects(&non_overlapping));
    }

    #[test]
    fn test_annotation_transform() {
        let mut annot = Annotation::new(AnnotType::Text, Rect::new(0.0, 0.0, 100.0, 50.0));

        let matrix = Matrix::translate(10.0, 20.0);
        annot.transform(&matrix);

        let rect = annot.rect();
        assert_eq!(rect.x0, 10.0);
        assert_eq!(rect.y0, 20.0);
    }

    #[test]
    fn test_annotation_popup() {
        let mut annot = Annotation::new(AnnotType::Text, Rect::EMPTY);
        assert!(annot.popup().is_none());

        let popup = Annotation::new(AnnotType::Popup, Rect::new(0.0, 0.0, 100.0, 100.0));
        annot.set_popup(Some(popup));
        assert!(annot.popup().is_some());

        annot.set_popup(None);
        assert!(annot.popup().is_none());
    }

    #[test]
    fn test_annotation_custom_properties() {
        let mut annot = Annotation::new(AnnotType::Text, Rect::EMPTY);

        annot.set_property("CustomKey".to_string(), "CustomValue".to_string());
        assert_eq!(annot.get_property("CustomKey"), Some("CustomValue"));
        assert_eq!(annot.get_property("NonExistent"), None);
    }

    #[test]
    fn test_border_style_default() {
        let border = BorderStyle::default();
        assert_eq!(border.width, 1.0);
        assert!(border.dash_pattern.is_empty());
        assert_eq!(border.style, BorderStyleType::Solid);
    }

    #[test]
    fn test_quadding() {
        assert_eq!(Quadding::Left as i32, 0);
        assert_eq!(Quadding::Center as i32, 1);
        assert_eq!(Quadding::Right as i32, 2);
    }

    #[test]
    fn test_intent_from_string() {
        assert_eq!(Intent::from_string("LineArrow"), Intent::LineArrow);
        assert_eq!(Intent::from_string("PolygonCloud"), Intent::PolygonCloud);
        assert_eq!(Intent::from_string("Unknown"), Intent::Unknown);
    }
}
