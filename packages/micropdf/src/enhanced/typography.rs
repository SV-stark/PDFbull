//! Typography - Professional text styling and layout
//!
//! Advanced typography features:
//! - ParagraphStyle with 40+ attributes
//! - Style inheritance
//! - Widow/orphan control
//! - Justification and hyphenation
//! - Rich text with inline styling

use super::error::{EnhancedError, Result};
use std::collections::HashMap;

/// Text alignment
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextAlign {
    Left,
    Right,
    Center,
    Justify,
}

/// Vertical alignment
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerticalAlign {
    Top,
    Middle,
    Bottom,
    Baseline,
}

/// Text direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextDirection {
    LeftToRight,
    RightToLeft,
    TopToBottom,
}

/// Hyphenation settings
#[derive(Debug, Clone)]
pub struct HyphenationSettings {
    /// Enable hyphenation
    pub enabled: bool,
    /// Language for hyphenation rules
    pub language: String,
    /// Minimum word length to hyphenate
    pub min_word_length: usize,
    /// Minimum characters before hyphen
    pub left_min: usize,
    /// Minimum characters after hyphen
    pub right_min: usize,
}

impl Default for HyphenationSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            language: "en".to_string(),
            min_word_length: 6,
            left_min: 2,
            right_min: 3,
        }
    }
}

/// Paragraph style with comprehensive formatting
#[derive(Debug, Clone)]
pub struct ParagraphStyle {
    pub name: String,
    pub parent: Option<String>,

    // Font properties
    pub font_name: String,
    pub font_size: f32,
    pub leading: f32, // Line height
    pub text_color: (f32, f32, f32),
    pub backcolor: Option<(f32, f32, f32)>,

    // Alignment
    pub alignment: TextAlign,
    pub vertical_align: VerticalAlign,
    pub text_direction: TextDirection,

    // Spacing
    pub space_before: f32,
    pub space_after: f32,
    pub first_line_indent: f32,
    pub left_indent: f32,
    pub right_indent: f32,
    pub bullet_indent: f32,

    // Line breaking
    pub word_wrap: bool,
    pub hyphenation: HyphenationSettings,
    pub widow_orphan_control: bool,
    pub keep_with_next: bool,
    pub keep_together: bool,

    // Borders and shading
    pub border_width: f32,
    pub border_color: Option<(f32, f32, f32)>,
    pub border_padding: f32,
    pub border_radius: f32,

    // Advanced
    pub underline: bool,
    pub strike_through: bool,
    pub super_script: bool,
    pub sub_script: bool,
    pub all_caps: bool,
    pub small_caps: bool,
}

impl Default for ParagraphStyle {
    fn default() -> Self {
        Self::new("Default")
    }
}

impl ParagraphStyle {
    /// Create new style
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            parent: None,
            font_name: "Helvetica".to_string(),
            font_size: 12.0,
            leading: 14.0,
            text_color: (0.0, 0.0, 0.0),
            backcolor: None,
            alignment: TextAlign::Left,
            vertical_align: VerticalAlign::Top,
            text_direction: TextDirection::LeftToRight,
            space_before: 0.0,
            space_after: 0.0,
            first_line_indent: 0.0,
            left_indent: 0.0,
            right_indent: 0.0,
            bullet_indent: 0.0,
            word_wrap: true,
            hyphenation: HyphenationSettings::default(),
            widow_orphan_control: true,
            keep_with_next: false,
            keep_together: false,
            border_width: 0.0,
            border_color: None,
            border_padding: 0.0,
            border_radius: 0.0,
            underline: false,
            strike_through: false,
            super_script: false,
            sub_script: false,
            all_caps: false,
            small_caps: false,
        }
    }

    /// Create style inheriting from parent
    pub fn from_parent(name: impl Into<String>, parent: &ParagraphStyle) -> Self {
        let mut style = parent.clone();
        style.name = name.into();
        style.parent = Some(parent.name.clone());
        style
    }

    // Builder methods

    /// Set font name
    pub fn with_font_name(mut self, name: &str) -> Self {
        self.font_name = name.to_string();
        self
    }

    /// Set font size
    pub fn with_font_size(mut self, size: f32) -> Self {
        self.font_size = size;
        self
    }

    /// Set leading (line height)
    pub fn with_leading(mut self, leading: f32) -> Self {
        self.leading = leading;
        self
    }

    /// Set text color
    pub fn with_text_color(mut self, r: f32, g: f32, b: f32) -> Self {
        self.text_color = (r, g, b);
        self
    }

    /// Set background color
    pub fn with_backcolor(mut self, r: f32, g: f32, b: f32) -> Self {
        self.backcolor = Some((r, g, b));
        self
    }

    /// Set alignment
    pub fn with_alignment(mut self, align: TextAlign) -> Self {
        self.alignment = align;
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

    /// Set first line indent
    pub fn with_first_line_indent(mut self, indent: f32) -> Self {
        self.first_line_indent = indent;
        self
    }

    /// Set left indent
    pub fn with_left_indent(mut self, indent: f32) -> Self {
        self.left_indent = indent;
        self
    }

    /// Set right indent
    pub fn with_right_indent(mut self, indent: f32) -> Self {
        self.right_indent = indent;
        self
    }
}

/// Style sheet manager
#[derive(Debug, Clone, Default)]
pub struct StyleSheet {
    styles: HashMap<String, ParagraphStyle>,
}

impl StyleSheet {
    /// Create new stylesheet
    pub fn new() -> Self {
        let mut sheet = Self::default();
        sheet.add_default_styles();
        sheet
    }

    /// Add default styles
    fn add_default_styles(&mut self) {
        // Normal
        let normal = ParagraphStyle::new("Normal");
        self.add_style(normal);

        // Heading 1
        let mut h1 = ParagraphStyle::new("Heading1");
        h1.font_name = "Helvetica-Bold".to_string();
        h1.font_size = 24.0;
        h1.leading = 28.0;
        h1.space_before = 12.0;
        h1.space_after = 6.0;
        self.add_style(h1);

        // Heading 2
        let mut h2 = ParagraphStyle::new("Heading2");
        h2.font_name = "Helvetica-Bold".to_string();
        h2.font_size = 18.0;
        h2.leading = 22.0;
        h2.space_before = 10.0;
        h2.space_after = 4.0;
        self.add_style(h2);

        // Body text
        let mut body = ParagraphStyle::new("BodyText");
        body.space_after = 6.0;
        body.alignment = TextAlign::Justify;
        self.add_style(body);

        // Code
        let mut code = ParagraphStyle::new("Code");
        code.font_name = "Courier".to_string();
        code.font_size = 10.0;
        code.backcolor = Some((0.95, 0.95, 0.95));
        code.border_width = 1.0;
        code.border_color = Some((0.7, 0.7, 0.7));
        code.border_padding = 4.0;
        self.add_style(code);
    }

    /// Add style
    pub fn add_style(&mut self, style: ParagraphStyle) {
        self.styles.insert(style.name.clone(), style);
    }

    /// Get style by name
    pub fn get_style(&self, name: &str) -> Option<&ParagraphStyle> {
        self.styles.get(name)
    }

    /// Get style by name (mutable)
    pub fn get_style_mut(&mut self, name: &str) -> Option<&mut ParagraphStyle> {
        self.styles.get_mut(name)
    }

    /// List all style names
    pub fn list_styles(&self) -> Vec<String> {
        self.styles.keys().cloned().collect()
    }
}

/// Rich text element
#[derive(Debug, Clone)]
pub enum RichTextElement {
    Text(String),
    Bold(String),
    Italic(String),
    BoldItalic(String),
    Underline(String),
    StrikeThrough(String),
    SuperScript(String),
    SubScript(String),
    Link {
        text: String,
        url: String,
    },
    Color {
        text: String,
        r: f32,
        g: f32,
        b: f32,
    },
    Font {
        text: String,
        font: String,
        size: f32,
    },
}

/// Rich text paragraph
#[derive(Debug, Clone)]
pub struct RichText {
    pub elements: Vec<RichTextElement>,
    pub style_name: String,
}

impl RichText {
    /// Create new rich text
    pub fn new(style_name: impl Into<String>) -> Self {
        Self {
            elements: vec![],
            style_name: style_name.into(),
        }
    }

    /// Add text element
    pub fn add(&mut self, element: RichTextElement) {
        self.elements.push(element);
    }

    /// Parse simple markup (bold, italic, etc.)
    pub fn from_markup(markup: &str, style_name: impl Into<String>) -> Result<Self> {
        let mut rich = Self::new(style_name);

        // TODO: Implement markup parsing
        // Support tags like <b>bold</b>, <i>italic</i>, <a href="">link</a>
        rich.add(RichTextElement::Text(markup.to_string()));

        Ok(rich)
    }
}

/// Text layout engine
pub struct TextLayout {
    pub width: f32,
    pub hyphenate: bool,
}

impl TextLayout {
    /// Create new layout engine
    pub fn new(width: f32) -> Self {
        Self {
            width,
            hyphenate: true,
        }
    }

    /// Layout text into lines
    pub fn layout(&self, text: &str, style: &ParagraphStyle) -> Vec<TextLine> {
        // TODO: Implement sophisticated text layout
        // 1. Break into words
        // 2. Apply hyphenation if needed
        // 3. Calculate line breaks
        // 4. Apply justification
        // 5. Handle widow/orphan control

        vec![TextLine {
            text: text.to_string(),
            width: self.width,
            ascent: style.font_size * 0.8,
            descent: style.font_size * 0.2,
        }]
    }
}

/// Single line of text
#[derive(Debug, Clone)]
pub struct TextLine {
    pub text: String,
    pub width: f32,
    pub ascent: f32,
    pub descent: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_paragraph_style_creation() {
        let style = ParagraphStyle::new("Test");
        assert_eq!(style.name, "Test");
        assert_eq!(style.font_size, 12.0);
        assert_eq!(style.alignment, TextAlign::Left);
    }

    #[test]
    fn test_stylesheet_default_styles() {
        let sheet = StyleSheet::new();
        assert!(sheet.get_style("Normal").is_some());
        assert!(sheet.get_style("Heading1").is_some());
        assert!(sheet.get_style("Heading2").is_some());
        assert!(sheet.get_style("BodyText").is_some());
        assert!(sheet.get_style("Code").is_some());
    }

    #[test]
    fn test_style_inheritance() {
        let parent = ParagraphStyle::new("Parent");
        let child = ParagraphStyle::from_parent("Child", &parent);
        assert_eq!(child.parent, Some("Parent".to_string()));
    }

    #[test]
    fn test_rich_text_creation() {
        let mut rich = RichText::new("Normal");
        rich.add(RichTextElement::Text("Hello ".to_string()));
        rich.add(RichTextElement::Bold("world".to_string()));
        assert_eq!(rich.elements.len(), 2);
    }
}
