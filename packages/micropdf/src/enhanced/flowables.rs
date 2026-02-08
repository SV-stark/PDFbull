//! Flowables System
//!
//! Comprehensive set of flowable elements for document composition.
//!
//! ## Core Flowables
//!
//! - `Paragraph` - Rich text with inline styling
//! - `Table` - Complex tables with 40+ style commands (see table.rs)
//! - `Image` - Images with auto-sizing and lazy loading
//! - `Spacer` - Vertical spacing
//! - `PageBreak` - Force new page
//! - `CondPageBreak` - Conditional page break
//! - `KeepTogether` - Keep elements on same page
//! - `KeepWithNext` - Keep with following element
//!
//! ## Example
//!
//! ```rust,ignore
//! use micropdf::enhanced::flowables::*;
//!
//! let story = Story::new()
//!     .add(Paragraph::new("Introduction"))
//!     .add(Spacer::new(10.0))
//!     .add(Image::from_file("chart.png").with_width(400.0))
//!     .add(PageBreak::new());
//! ```

use super::error::{EnhancedError, Result};
use super::typography::{ParagraphStyle, RichText, StyleSheet, TextAlign};
use std::any::Any;
use std::collections::HashMap;
use std::path::PathBuf;

// ============================================================================
// Flow Context
// ============================================================================

/// Context passed during wrap phase
#[derive(Debug, Clone, Default)]
pub struct FlowContext {
    /// Current page number
    pub page_number: usize,
    /// Total page count (if known)
    pub total_pages: Option<usize>,
    /// Current frame name
    pub frame_name: String,
    /// Available width
    pub available_width: f32,
    /// Available height
    pub available_height: f32,
    /// Style sheet
    pub styles: StyleSheet,
    /// Document variables
    pub variables: HashMap<String, String>,
}

impl FlowContext {
    /// Create new flow context
    pub fn new(available_width: f32, available_height: f32) -> Self {
        Self {
            page_number: 1,
            total_pages: None,
            frame_name: "main".to_string(),
            available_width,
            available_height,
            styles: StyleSheet::default(),
            variables: HashMap::new(),
        }
    }

    /// Set page number
    pub fn with_page_number(mut self, page: usize) -> Self {
        self.page_number = page;
        self
    }

    /// Set style sheet
    pub fn with_styles(mut self, styles: StyleSheet) -> Self {
        self.styles = styles;
        self
    }
}

// ============================================================================
// Draw Context
// ============================================================================

/// Context passed during draw phase
#[derive(Debug)]
pub struct DrawContext {
    /// PDF content stream
    pub content_stream: Vec<u8>,
    /// Current X position
    pub x: f32,
    /// Current Y position
    pub y: f32,
    /// Page width
    pub page_width: f32,
    /// Page height
    pub page_height: f32,
    /// Registered fonts
    pub fonts: HashMap<String, String>,
    /// Image resources
    pub images: HashMap<String, ImageResource>,
    /// Current page number
    pub page_number: usize,
}

impl DrawContext {
    /// Create new draw context
    pub fn new(page_width: f32, page_height: f32) -> Self {
        Self {
            content_stream: Vec::new(),
            x: 0.0,
            y: page_height,
            page_width,
            page_height,
            fonts: HashMap::new(),
            images: HashMap::new(),
            page_number: 1,
        }
    }

    /// Add to content stream
    pub fn write(&mut self, data: &[u8]) {
        self.content_stream.extend_from_slice(data);
    }

    /// Add line to content stream
    pub fn writeln(&mut self, line: &str) {
        self.content_stream.extend_from_slice(line.as_bytes());
        self.content_stream.push(b'\n');
    }

    /// Register a font
    pub fn register_font(&mut self, name: &str, pdf_name: &str) {
        self.fonts.insert(name.to_string(), pdf_name.to_string());
    }

    /// Get PDF font name
    pub fn get_font(&self, name: &str) -> Option<&String> {
        self.fonts.get(name)
    }
}

/// Image resource
#[derive(Debug, Clone)]
pub struct ImageResource {
    pub name: String,
    pub width: f32,
    pub height: f32,
    pub data: Vec<u8>,
}

// ============================================================================
// Wrap Result
// ============================================================================

/// Result of wrap operation
#[derive(Debug, Clone)]
pub struct WrapResult {
    /// Required width
    pub width: f32,
    /// Required height
    pub height: f32,
    /// Whether element fits
    pub fits: bool,
    /// Extra data
    pub data: Option<HashMap<String, String>>,
}

impl WrapResult {
    /// Create wrap result
    pub fn new(width: f32, height: f32) -> Self {
        Self {
            width,
            height,
            fits: true,
            data: None,
        }
    }

    /// Mark as not fitting
    pub fn not_fitting(mut self) -> Self {
        self.fits = false;
        self
    }
}

// ============================================================================
// Flowable Trait
// ============================================================================

/// Trait for elements that flow in the document
pub trait Flowable: Send + Sync {
    /// Calculate required dimensions
    fn wrap(&self, available_width: f32, available_height: f32, ctx: &FlowContext) -> WrapResult;

    /// Draw the element
    fn draw(&self, x: f32, y: f32, ctx: &mut DrawContext) -> Result<Vec<String>>;

    /// Split the element if it doesn't fit
    fn split(
        &self,
        available_height: f32,
        ctx: &FlowContext,
    ) -> Option<(Box<dyn Flowable>, Box<dyn Flowable>)>;

    /// Whether this element can be split
    fn is_splittable(&self) -> bool {
        false
    }

    /// Space before element
    fn space_before(&self) -> f32 {
        0.0
    }

    /// Space after element
    fn space_after(&self) -> f32 {
        0.0
    }

    /// Clone as boxed trait object
    fn clone_box(&self) -> Box<dyn Flowable>;

    /// As Any for downcasting
    fn as_any(&self) -> &dyn Any;
}

// ============================================================================
// Paragraph Flowable
// ============================================================================

/// Rich text paragraph
#[derive(Debug, Clone)]
pub struct Paragraph {
    /// Text content (plain or rich)
    pub text: String,
    /// Paragraph style
    pub style: ParagraphStyle,
    /// Pre-computed lines
    lines: Vec<String>,
    /// Computed height
    computed_height: f32,
    /// Computed width
    computed_width: f32,
}

impl Paragraph {
    /// Create new paragraph
    pub fn new(text: &str) -> Self {
        Self {
            text: text.to_string(),
            style: ParagraphStyle::default(),
            lines: Vec::new(),
            computed_height: 0.0,
            computed_width: 0.0,
        }
    }

    /// Create with style
    pub fn with_style(mut self, style: ParagraphStyle) -> Self {
        self.style = style;
        self
    }

    /// Create with style name from stylesheet
    pub fn with_style_name(mut self, name: &str, styles: &StyleSheet) -> Self {
        if let Some(style) = styles.get_style(name) {
            self.style = style.clone();
        }
        self
    }

    /// Set text alignment
    pub fn align(mut self, align: TextAlign) -> Self {
        self.style.alignment = align;
        self
    }

    /// Set font size
    pub fn font_size(mut self, size: f32) -> Self {
        self.style.font_size = size;
        self
    }

    /// Set leading
    pub fn leading(mut self, leading: f32) -> Self {
        self.style.leading = leading;
        self
    }

    /// Set space before
    pub fn space_before(mut self, space: f32) -> Self {
        self.style.space_before = space;
        self
    }

    /// Set space after
    pub fn space_after_value(mut self, space: f32) -> Self {
        self.style.space_after = space;
        self
    }

    /// Set first line indent
    pub fn first_line_indent(mut self, indent: f32) -> Self {
        self.style.first_line_indent = indent;
        self
    }

    /// Set left indent
    pub fn left_indent(mut self, indent: f32) -> Self {
        self.style.left_indent = indent;
        self
    }

    /// Set right indent
    pub fn right_indent(mut self, indent: f32) -> Self {
        self.style.right_indent = indent;
        self
    }

    /// Calculate lines (word wrapping)
    fn calculate_lines(&mut self, available_width: f32) {
        self.lines.clear();
        let text_width = available_width - self.style.left_indent - self.style.right_indent;

        // Simple word wrapping
        let words: Vec<&str> = self.text.split_whitespace().collect();
        if words.is_empty() {
            return;
        }

        // Approximate character width based on font size
        let char_width = self.style.font_size * 0.5;
        let chars_per_line = (text_width / char_width) as usize;

        if chars_per_line == 0 {
            self.lines = vec![self.text.clone()];
            return;
        }

        let mut current_line = String::new();
        let mut is_first_line = true;

        for word in words {
            let first_indent = if is_first_line {
                self.style.first_line_indent
            } else {
                0.0
            };
            let effective_chars = ((text_width - first_indent) / char_width) as usize;

            let test_line = if current_line.is_empty() {
                word.to_string()
            } else {
                format!("{} {}", current_line, word)
            };

            if test_line.len() > effective_chars && !current_line.is_empty() {
                self.lines.push(current_line);
                current_line = word.to_string();
                is_first_line = false;
            } else {
                current_line = test_line;
            }
        }

        if !current_line.is_empty() {
            self.lines.push(current_line);
        }
    }
}

impl Flowable for Paragraph {
    fn wrap(&self, available_width: f32, _available_height: f32, _ctx: &FlowContext) -> WrapResult {
        let mut para = self.clone();
        para.calculate_lines(available_width);

        let height = para.style.space_before
            + (para.lines.len() as f32 * para.style.leading)
            + para.style.space_after;

        WrapResult::new(available_width, height)
    }

    fn draw(&self, x: f32, y: f32, ctx: &mut DrawContext) -> Result<Vec<String>> {
        let mut commands = Vec::new();
        let mut para = self.clone();
        para.calculate_lines(400.0); // Default width

        commands.push("BT".to_string());

        // Set font
        let font_name = if self.style.font_name.contains("Bold") {
            "F2"
        } else {
            "F1"
        };
        commands.push(format!("/{} {} Tf", font_name, self.style.font_size));

        // Set color
        commands.push(format!(
            "{} {} {} rg",
            self.style.text_color.0, self.style.text_color.1, self.style.text_color.2
        ));

        let mut current_y = y - self.style.space_before - self.style.font_size;

        for (idx, line) in para.lines.iter().enumerate() {
            let line_x = if idx == 0 {
                x + self.style.left_indent + self.style.first_line_indent
            } else {
                x + self.style.left_indent
            };

            commands.push(format!("{} {} Td", line_x, current_y));
            commands.push(format!("({}) Tj", escape_pdf_string(line)));

            current_y -= self.style.leading;
        }

        commands.push("ET".to_string());

        Ok(commands)
    }

    fn split(
        &self,
        available_height: f32,
        _ctx: &FlowContext,
    ) -> Option<(Box<dyn Flowable>, Box<dyn Flowable>)> {
        let mut para = self.clone();
        para.calculate_lines(400.0);

        let lines_per_height =
            ((available_height - self.style.space_before) / self.style.leading).floor() as usize;

        if lines_per_height == 0 || lines_per_height >= para.lines.len() {
            return None;
        }

        let first_text: String = para.lines[..lines_per_height].join(" ");
        let second_text: String = para.lines[lines_per_height..].join(" ");

        let first = Paragraph::new(&first_text).with_style(self.style.clone());
        let mut second = Paragraph::new(&second_text).with_style(self.style.clone());
        second.style.space_before = 0.0; // Remove space before continuation

        Some((Box::new(first), Box::new(second)))
    }

    fn is_splittable(&self) -> bool {
        true
    }

    fn space_before(&self) -> f32 {
        self.style.space_before
    }

    fn space_after(&self) -> f32 {
        self.style.space_after
    }

    fn clone_box(&self) -> Box<dyn Flowable> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

// ============================================================================
// Image Flowable
// ============================================================================

/// Image with auto-sizing and lazy loading
#[derive(Debug, Clone)]
pub struct Image {
    /// Image path
    path: Option<PathBuf>,
    /// Image data
    data: Option<Vec<u8>>,
    /// Original width
    original_width: f32,
    /// Original height
    original_height: f32,
    /// Target width (None for auto)
    width: Option<f32>,
    /// Target height (None for auto)
    height: Option<f32>,
    /// Horizontal alignment
    h_align: TextAlign,
    /// Space before
    space_before: f32,
    /// Space after
    space_after: f32,
    /// Caption
    caption: Option<String>,
    /// Lazy load
    lazy: bool,
}

impl Image {
    /// Create from file path
    pub fn from_file(path: &str) -> Self {
        Self {
            path: Some(PathBuf::from(path)),
            data: None,
            original_width: 0.0,
            original_height: 0.0,
            width: None,
            height: None,
            h_align: TextAlign::Center,
            space_before: 6.0,
            space_after: 6.0,
            caption: None,
            lazy: true,
        }
    }

    /// Create from bytes
    pub fn from_bytes(data: Vec<u8>, width: f32, height: f32) -> Self {
        Self {
            path: None,
            data: Some(data),
            original_width: width,
            original_height: height,
            width: None,
            height: None,
            h_align: TextAlign::Center,
            space_before: 6.0,
            space_after: 6.0,
            caption: None,
            lazy: false,
        }
    }

    /// Set width
    pub fn with_width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    /// Set height
    pub fn with_height(mut self, height: f32) -> Self {
        self.height = Some(height);
        self
    }

    /// Set both dimensions
    pub fn with_size(mut self, width: f32, height: f32) -> Self {
        self.width = Some(width);
        self.height = Some(height);
        self
    }

    /// Set horizontal alignment
    pub fn with_align(mut self, align: TextAlign) -> Self {
        self.h_align = align;
        self
    }

    /// Set caption
    pub fn with_caption(mut self, caption: &str) -> Self {
        self.caption = Some(caption.to_string());
        self
    }

    /// Set space before
    pub fn with_space_before(mut self, space: f32) -> Self {
        self.space_before = space;
        self
    }

    /// Set space after
    pub fn with_space_after(mut self, space: f32) -> Self {
        self.space_after = space;
        self
    }

    /// Disable lazy loading
    pub fn preload(mut self) -> Self {
        self.lazy = false;
        self
    }

    /// Calculate actual dimensions
    fn calculate_dimensions(&self, available_width: f32) -> (f32, f32) {
        let orig_w = if self.original_width > 0.0 {
            self.original_width
        } else {
            200.0
        };
        let orig_h = if self.original_height > 0.0 {
            self.original_height
        } else {
            200.0
        };

        match (self.width, self.height) {
            (Some(w), Some(h)) => (w, h),
            (Some(w), None) => {
                let scale = w / orig_w;
                (w, orig_h * scale)
            }
            (None, Some(h)) => {
                let scale = h / orig_h;
                (orig_w * scale, h)
            }
            (None, None) => {
                // Auto-fit to available width
                if orig_w > available_width {
                    let scale = available_width / orig_w;
                    (available_width, orig_h * scale)
                } else {
                    (orig_w, orig_h)
                }
            }
        }
    }
}

impl Flowable for Image {
    fn wrap(&self, available_width: f32, _available_height: f32, _ctx: &FlowContext) -> WrapResult {
        let (w, h) = self.calculate_dimensions(available_width);
        let total_height = h + self.space_before + self.space_after;

        // Add caption height if present
        let caption_height = if self.caption.is_some() { 14.0 } else { 0.0 };

        WrapResult::new(w, total_height + caption_height)
    }

    fn draw(&self, x: f32, y: f32, _ctx: &mut DrawContext) -> Result<Vec<String>> {
        let mut commands = Vec::new();
        let (w, h) = self.calculate_dimensions(400.0);

        // Calculate X position based on alignment
        let img_x = match self.h_align {
            TextAlign::Left => x,
            TextAlign::Center => x + (400.0 - w) / 2.0,
            TextAlign::Right => x + 400.0 - w,
            _ => x,
        };

        let img_y = y - self.space_before - h;

        commands.push("q".to_string());
        commands.push(format!("{} 0 0 {} {} {} cm", w, h, img_x, img_y));
        commands.push("/Im1 Do".to_string()); // Reference to image XObject
        commands.push("Q".to_string());

        // Draw caption if present
        if let Some(ref caption) = self.caption {
            let caption_y = img_y - 12.0;
            commands.push("BT".to_string());
            commands.push("/F1 10 Tf".to_string());
            commands.push(format!(
                "{} {} Td",
                img_x + w / 2.0 - caption.len() as f32 * 2.5,
                caption_y
            ));
            commands.push(format!("({}) Tj", escape_pdf_string(caption)));
            commands.push("ET".to_string());
        }

        Ok(commands)
    }

    fn split(
        &self,
        _available_height: f32,
        _ctx: &FlowContext,
    ) -> Option<(Box<dyn Flowable>, Box<dyn Flowable>)> {
        None // Images can't be split
    }

    fn space_before(&self) -> f32 {
        self.space_before
    }

    fn space_after(&self) -> f32 {
        self.space_after
    }

    fn clone_box(&self) -> Box<dyn Flowable> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

// ============================================================================
// Spacer Flowable
// ============================================================================

/// Vertical spacer
#[derive(Debug, Clone)]
pub struct Spacer {
    height: f32,
}

impl Spacer {
    /// Create spacer with height
    pub fn new(height: f32) -> Self {
        Self { height }
    }
}

impl Flowable for Spacer {
    fn wrap(
        &self,
        _available_width: f32,
        _available_height: f32,
        _ctx: &FlowContext,
    ) -> WrapResult {
        WrapResult::new(0.0, self.height)
    }

    fn draw(&self, _x: f32, _y: f32, _ctx: &mut DrawContext) -> Result<Vec<String>> {
        Ok(Vec::new()) // Spacer doesn't draw anything
    }

    fn split(
        &self,
        available_height: f32,
        _ctx: &FlowContext,
    ) -> Option<(Box<dyn Flowable>, Box<dyn Flowable>)> {
        if available_height < self.height && available_height > 0.0 {
            Some((
                Box::new(Spacer::new(available_height)),
                Box::new(Spacer::new(self.height - available_height)),
            ))
        } else {
            None
        }
    }

    fn is_splittable(&self) -> bool {
        true
    }

    fn clone_box(&self) -> Box<dyn Flowable> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

// ============================================================================
// Page Break Flowable
// ============================================================================

/// Force a page break
#[derive(Debug, Clone)]
pub struct PageBreak;

impl PageBreak {
    /// Create page break
    pub fn new() -> Self {
        Self
    }
}

impl Default for PageBreak {
    fn default() -> Self {
        Self::new()
    }
}

impl Flowable for PageBreak {
    fn wrap(&self, _available_width: f32, available_height: f32, _ctx: &FlowContext) -> WrapResult {
        // Request all remaining space to force page break
        WrapResult::new(0.0, available_height + 1.0).not_fitting()
    }

    fn draw(&self, _x: f32, _y: f32, _ctx: &mut DrawContext) -> Result<Vec<String>> {
        Ok(Vec::new())
    }

    fn split(
        &self,
        _available_height: f32,
        _ctx: &FlowContext,
    ) -> Option<(Box<dyn Flowable>, Box<dyn Flowable>)> {
        None
    }

    fn clone_box(&self) -> Box<dyn Flowable> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

// ============================================================================
// Conditional Page Break
// ============================================================================

/// Page break if not enough space
#[derive(Debug, Clone)]
pub struct CondPageBreak {
    min_height: f32,
}

impl CondPageBreak {
    /// Create conditional page break
    pub fn new(min_height: f32) -> Self {
        Self { min_height }
    }
}

impl Flowable for CondPageBreak {
    fn wrap(&self, _available_width: f32, available_height: f32, _ctx: &FlowContext) -> WrapResult {
        if available_height < self.min_height {
            // Not enough space, request page break
            WrapResult::new(0.0, available_height + 1.0).not_fitting()
        } else {
            WrapResult::new(0.0, 0.0)
        }
    }

    fn draw(&self, _x: f32, _y: f32, _ctx: &mut DrawContext) -> Result<Vec<String>> {
        Ok(Vec::new())
    }

    fn split(
        &self,
        _available_height: f32,
        _ctx: &FlowContext,
    ) -> Option<(Box<dyn Flowable>, Box<dyn Flowable>)> {
        None
    }

    fn clone_box(&self) -> Box<dyn Flowable> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

// ============================================================================
// Keep Together
// ============================================================================

/// Keep elements together on same page
pub struct KeepTogether {
    elements: Vec<Box<dyn Flowable>>,
}

impl std::fmt::Debug for KeepTogether {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KeepTogether")
            .field("elements_count", &self.elements.len())
            .finish()
    }
}

impl Clone for KeepTogether {
    fn clone(&self) -> Self {
        Self {
            elements: self.elements.iter().map(|e| e.clone_box()).collect(),
        }
    }
}

impl KeepTogether {
    /// Create keep together container
    pub fn new(elements: Vec<Box<dyn Flowable>>) -> Self {
        Self { elements }
    }

    /// Add element
    pub fn add<F: Flowable + 'static>(mut self, element: F) -> Self {
        self.elements.push(Box::new(element));
        self
    }
}

impl Flowable for KeepTogether {
    fn wrap(&self, available_width: f32, available_height: f32, ctx: &FlowContext) -> WrapResult {
        let mut total_height: f32 = 0.0;
        let mut max_width: f32 = 0.0;

        for element in &self.elements {
            let result = element.wrap(available_width, available_height - total_height, ctx);
            total_height += result.height;
            max_width = max_width.max(result.width);
        }

        WrapResult::new(max_width, total_height)
    }

    fn draw(&self, x: f32, y: f32, ctx: &mut DrawContext) -> Result<Vec<String>> {
        let mut commands = Vec::new();
        let mut current_y = y;

        for element in &self.elements {
            let result = element.wrap(400.0, 1000.0, &FlowContext::default());
            let drawn = element.draw(x, current_y, ctx)?;
            commands.extend(drawn);
            current_y -= result.height;
        }

        Ok(commands)
    }

    fn split(
        &self,
        _available_height: f32,
        _ctx: &FlowContext,
    ) -> Option<(Box<dyn Flowable>, Box<dyn Flowable>)> {
        // Keep together should not be split
        None
    }

    fn clone_box(&self) -> Box<dyn Flowable> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

// ============================================================================
// Keep With Next
// ============================================================================

/// Keep this element with the next one
pub struct KeepWithNext {
    element: Box<dyn Flowable>,
}

impl std::fmt::Debug for KeepWithNext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KeepWithNext").finish()
    }
}

impl Clone for KeepWithNext {
    fn clone(&self) -> Self {
        Self {
            element: self.element.clone_box(),
        }
    }
}

impl KeepWithNext {
    /// Create keep with next wrapper
    pub fn new<F: Flowable + 'static>(element: F) -> Self {
        Self {
            element: Box::new(element),
        }
    }

    fn new_box(element: Box<dyn Flowable>) -> Self {
        Self { element }
    }
}

impl Flowable for KeepWithNext {
    fn wrap(&self, available_width: f32, available_height: f32, ctx: &FlowContext) -> WrapResult {
        self.element.wrap(available_width, available_height, ctx)
    }

    fn draw(&self, x: f32, y: f32, ctx: &mut DrawContext) -> Result<Vec<String>> {
        self.element.draw(x, y, ctx)
    }

    fn split(
        &self,
        _available_height: f32,
        _ctx: &FlowContext,
    ) -> Option<(Box<dyn Flowable>, Box<dyn Flowable>)> {
        None
    }

    fn space_before(&self) -> f32 {
        self.element.space_before()
    }

    fn space_after(&self) -> f32 {
        self.element.space_after()
    }

    fn clone_box(&self) -> Box<dyn Flowable> {
        Box::new(KeepWithNext::new_box(self.element.clone_box()))
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

// ============================================================================
// Horizontal Rule
// ============================================================================

/// Horizontal line
#[derive(Debug, Clone)]
pub struct HorizontalRule {
    width: Option<f32>,
    thickness: f32,
    color: (f32, f32, f32),
    space_before: f32,
    space_after: f32,
}

impl HorizontalRule {
    /// Create horizontal rule
    pub fn new() -> Self {
        Self {
            width: None, // Full width
            thickness: 1.0,
            color: (0.0, 0.0, 0.0),
            space_before: 6.0,
            space_after: 6.0,
        }
    }

    /// Set width
    pub fn with_width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    /// Set thickness
    pub fn with_thickness(mut self, thickness: f32) -> Self {
        self.thickness = thickness;
        self
    }

    /// Set color
    pub fn with_color(mut self, r: f32, g: f32, b: f32) -> Self {
        self.color = (r, g, b);
        self
    }
}

impl Default for HorizontalRule {
    fn default() -> Self {
        Self::new()
    }
}

impl Flowable for HorizontalRule {
    fn wrap(&self, available_width: f32, _available_height: f32, _ctx: &FlowContext) -> WrapResult {
        let w = self.width.unwrap_or(available_width);
        WrapResult::new(w, self.thickness + self.space_before + self.space_after)
    }

    fn draw(&self, x: f32, y: f32, _ctx: &mut DrawContext) -> Result<Vec<String>> {
        let mut commands = Vec::new();
        let w = self.width.unwrap_or(400.0);
        let line_y = y - self.space_before - self.thickness / 2.0;

        commands.push("q".to_string());
        commands.push(format!(
            "{} {} {} RG",
            self.color.0, self.color.1, self.color.2
        ));
        commands.push(format!("{} w", self.thickness));
        commands.push(format!("{} {} m {} {} l S", x, line_y, x + w, line_y));
        commands.push("Q".to_string());

        Ok(commands)
    }

    fn split(
        &self,
        _available_height: f32,
        _ctx: &FlowContext,
    ) -> Option<(Box<dyn Flowable>, Box<dyn Flowable>)> {
        None
    }

    fn space_before(&self) -> f32 {
        self.space_before
    }

    fn space_after(&self) -> f32 {
        self.space_after
    }

    fn clone_box(&self) -> Box<dyn Flowable> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

// ============================================================================
// List Item
// ============================================================================

/// List item (bullet or numbered)
#[derive(Debug, Clone)]
pub struct ListItem {
    content: Paragraph,
    bullet: String,
    indent: f32,
}

impl ListItem {
    /// Create bullet list item
    pub fn bullet(text: &str) -> Self {
        Self {
            content: Paragraph::new(text),
            bullet: "•".to_string(),
            indent: 20.0,
        }
    }

    /// Create numbered list item
    pub fn numbered(number: usize, text: &str) -> Self {
        Self {
            content: Paragraph::new(text),
            bullet: format!("{}.", number),
            indent: 20.0,
        }
    }

    /// Set custom bullet
    pub fn with_bullet(mut self, bullet: &str) -> Self {
        self.bullet = bullet.to_string();
        self
    }

    /// Set indent
    pub fn with_indent(mut self, indent: f32) -> Self {
        self.indent = indent;
        self
    }

    /// Set paragraph style
    pub fn with_style(mut self, style: ParagraphStyle) -> Self {
        self.content = self.content.with_style(style);
        self
    }
}

impl Flowable for ListItem {
    fn wrap(&self, available_width: f32, available_height: f32, ctx: &FlowContext) -> WrapResult {
        self.content
            .wrap(available_width - self.indent, available_height, ctx)
    }

    fn draw(&self, x: f32, y: f32, ctx: &mut DrawContext) -> Result<Vec<String>> {
        let mut commands = Vec::new();

        // Draw bullet
        commands.push("BT".to_string());
        commands.push("/F1 10 Tf".to_string());
        commands.push(format!("{} {} Td", x, y - 10.0));
        commands.push(format!("({}) Tj", escape_pdf_string(&self.bullet)));
        commands.push("ET".to_string());

        // Draw content
        let content_cmds = self.content.draw(x + self.indent, y, ctx)?;
        commands.extend(content_cmds);

        Ok(commands)
    }

    fn split(
        &self,
        available_height: f32,
        ctx: &FlowContext,
    ) -> Option<(Box<dyn Flowable>, Box<dyn Flowable>)> {
        self.content
            .split(available_height, ctx)
            .map(|(first, second)| {
                let first_content = if let Some(para) = first.as_any().downcast_ref::<Paragraph>() {
                    para.clone()
                } else {
                    Paragraph::new("")
                };
                let second_content = if let Some(para) = second.as_any().downcast_ref::<Paragraph>()
                {
                    para.clone()
                } else {
                    Paragraph::new("")
                };

                (
                    Box::new(ListItem {
                        content: first_content,
                        bullet: self.bullet.clone(),
                        indent: self.indent,
                    }) as Box<dyn Flowable>,
                    Box::new(ListItem {
                        content: second_content,
                        bullet: "".to_string(), // No bullet for continuation
                        indent: self.indent,
                    }) as Box<dyn Flowable>,
                )
            })
    }

    fn is_splittable(&self) -> bool {
        true
    }

    fn clone_box(&self) -> Box<dyn Flowable> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

// ============================================================================
// Story
// ============================================================================

/// Collection of flowables
#[derive(Default)]
pub struct Story {
    elements: Vec<Box<dyn Flowable>>,
}

impl std::fmt::Debug for Story {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Story")
            .field("elements_count", &self.elements.len())
            .finish()
    }
}

impl Story {
    /// Create empty story
    pub fn new() -> Self {
        Self::default()
    }

    /// Add flowable
    pub fn add<F: Flowable + 'static>(mut self, element: F) -> Self {
        self.elements.push(Box::new(element));
        self
    }

    /// Add boxed flowable
    pub fn add_box(mut self, element: Box<dyn Flowable>) -> Self {
        self.elements.push(element);
        self
    }

    /// Get elements
    pub fn elements(&self) -> &[Box<dyn Flowable>] {
        &self.elements
    }

    /// Take elements
    pub fn take_elements(self) -> Vec<Box<dyn Flowable>> {
        self.elements
    }

    /// Number of elements
    pub fn len(&self) -> usize {
        self.elements.len()
    }

    /// Is empty
    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

fn escape_pdf_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('(', "\\(")
        .replace(')', "\\)")
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_paragraph_creation() {
        let para = Paragraph::new("Hello, World!");
        let ctx = FlowContext::new(400.0, 600.0);
        let result = para.wrap(400.0, 600.0, &ctx);
        assert!(result.height > 0.0);
    }

    #[test]
    fn test_paragraph_with_style() {
        let style = ParagraphStyle::new("TestStyle")
            .with_font_size(14.0)
            .with_leading(18.0);
        let para = Paragraph::new("Styled text").with_style(style);
        assert_eq!(para.style.font_size, 14.0);
    }

    #[test]
    fn test_spacer() {
        let spacer = Spacer::new(20.0);
        let ctx = FlowContext::new(400.0, 600.0);
        let result = spacer.wrap(400.0, 600.0, &ctx);
        assert_eq!(result.height, 20.0);
    }

    #[test]
    fn test_page_break() {
        let pb = PageBreak::new();
        let ctx = FlowContext::new(400.0, 100.0);
        let result = pb.wrap(400.0, 100.0, &ctx);
        assert!(!result.fits);
    }

    #[test]
    fn test_cond_page_break_with_space() {
        let cpb = CondPageBreak::new(50.0);
        let ctx = FlowContext::new(400.0, 100.0);
        let result = cpb.wrap(400.0, 100.0, &ctx);
        assert!(result.fits);
        assert_eq!(result.height, 0.0);
    }

    #[test]
    fn test_cond_page_break_without_space() {
        let cpb = CondPageBreak::new(150.0);
        let ctx = FlowContext::new(400.0, 100.0);
        let result = cpb.wrap(400.0, 100.0, &ctx);
        assert!(!result.fits);
    }

    #[test]
    fn test_image() {
        let img = Image::from_bytes(vec![], 200.0, 150.0).with_width(400.0);
        let ctx = FlowContext::new(500.0, 600.0);
        let result = img.wrap(500.0, 600.0, &ctx);
        assert!(result.width <= 500.0);
    }

    #[test]
    fn test_horizontal_rule() {
        let hr = HorizontalRule::new().with_thickness(2.0);
        let ctx = FlowContext::new(400.0, 600.0);
        let result = hr.wrap(400.0, 600.0, &ctx);
        assert!(result.height > 0.0);
    }

    #[test]
    fn test_list_item() {
        let item = ListItem::bullet("First item");
        let ctx = FlowContext::new(400.0, 600.0);
        let result = item.wrap(400.0, 600.0, &ctx);
        assert!(result.height > 0.0);
    }

    #[test]
    fn test_story() {
        let story = Story::new()
            .add(Paragraph::new("Introduction"))
            .add(Spacer::new(10.0))
            .add(Paragraph::new("Body text"));

        assert_eq!(story.len(), 3);
    }

    #[test]
    fn test_keep_together() {
        let kt = KeepTogether::new(vec![
            Box::new(Paragraph::new("Line 1")),
            Box::new(Paragraph::new("Line 2")),
        ]);

        let ctx = FlowContext::new(400.0, 600.0);
        let result = kt.wrap(400.0, 600.0, &ctx);
        assert!(result.height > 0.0);
    }

    #[test]
    fn test_keep_with_next() {
        let kwn = KeepWithNext::new(Paragraph::new("Heading"));
        let ctx = FlowContext::new(400.0, 600.0);
        let result = kwn.wrap(400.0, 600.0, &ctx);
        assert!(result.height > 0.0);
    }
}
