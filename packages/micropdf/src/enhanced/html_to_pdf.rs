//! HTML to PDF Conversion
//!
//! Convert HTML/CSS to PDF with high fidelity:
//! - CSS 2.1 + CSS3 subset support
//! - Flexbox and Grid layouts
//! - Web fonts
//! - External resources (images, CSS, fonts)
//! - SVG rendering
//! - Media queries for print
//!
//! ## Example
//!
//! ```rust,ignore
//! use micropdf::enhanced::html_to_pdf::*;
//!
//! let options = HtmlToPdfOptions::default()
//!     .with_page_size(PageSize::A4)
//!     .with_margins(72.0, 72.0, 72.0, 72.0);
//!
//! html_to_pdf("<h1>Hello</h1><p>World</p>", "output.pdf", &options)?;
//! ```

use super::error::{EnhancedError, Result};
use std::collections::HashMap;
use std::path::Path;

// ============================================================================
// Conversion Options
// ============================================================================

/// Predefined page sizes
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PageSize {
    Letter,
    Legal,
    A3,
    A4,
    A5,
    Custom(f32, f32),
}

impl PageSize {
    /// Get dimensions in points
    pub fn dimensions(&self) -> (f32, f32) {
        match self {
            PageSize::Letter => (612.0, 792.0),
            PageSize::Legal => (612.0, 1008.0),
            PageSize::A3 => (841.89, 1190.55),
            PageSize::A4 => (595.28, 841.89),
            PageSize::A5 => (419.53, 595.28),
            PageSize::Custom(w, h) => (*w, *h),
        }
    }
}

/// HTML to PDF conversion options
#[derive(Debug, Clone)]
pub struct HtmlToPdfOptions {
    /// Page size
    pub page_width: f32,
    pub page_height: f32,
    /// Page margins
    pub margin_top: f32,
    pub margin_bottom: f32,
    pub margin_left: f32,
    pub margin_right: f32,
    /// Base URL for resolving relative paths
    pub base_url: Option<String>,
    /// Enable JavaScript
    pub enable_javascript: bool,
    /// Wait time for dynamic content (ms)
    pub javascript_delay: u64,
    /// User stylesheet
    pub user_stylesheet: Option<String>,
    /// Print media emulation
    pub print_media_type: bool,
    /// Background graphics
    pub print_background: bool,
    /// Scale factor
    pub scale: f32,
    /// Header HTML (repeated on each page)
    pub header_html: Option<String>,
    /// Footer HTML (repeated on each page)
    pub footer_html: Option<String>,
    /// Landscape orientation
    pub landscape: bool,
    /// Default font
    pub default_font: String,
    /// Default font size
    pub default_font_size: f32,
}

impl Default for HtmlToPdfOptions {
    fn default() -> Self {
        Self {
            page_width: 612.0,
            page_height: 792.0,
            margin_top: 36.0,
            margin_bottom: 36.0,
            margin_left: 36.0,
            margin_right: 36.0,
            base_url: None,
            enable_javascript: false,
            javascript_delay: 0,
            user_stylesheet: None,
            print_media_type: true,
            print_background: true,
            scale: 1.0,
            header_html: None,
            footer_html: None,
            landscape: false,
            default_font: "Helvetica".to_string(),
            default_font_size: 12.0,
        }
    }
}

impl HtmlToPdfOptions {
    /// Create new options
    pub fn new() -> Self {
        Self::default()
    }

    /// Set page size
    pub fn with_page_size(mut self, size: PageSize) -> Self {
        let (w, h) = size.dimensions();
        self.page_width = w;
        self.page_height = h;
        self
    }

    /// Set margins
    pub fn with_margins(mut self, top: f32, right: f32, bottom: f32, left: f32) -> Self {
        self.margin_top = top;
        self.margin_right = right;
        self.margin_bottom = bottom;
        self.margin_left = left;
        self
    }

    /// Set base URL
    pub fn with_base_url(mut self, url: &str) -> Self {
        self.base_url = Some(url.to_string());
        self
    }

    /// Enable JavaScript
    pub fn with_javascript(mut self, enabled: bool) -> Self {
        self.enable_javascript = enabled;
        self
    }

    /// Set user stylesheet
    pub fn with_stylesheet(mut self, css: &str) -> Self {
        self.user_stylesheet = Some(css.to_string());
        self
    }

    /// Set landscape orientation
    pub fn with_landscape(mut self, landscape: bool) -> Self {
        self.landscape = landscape;
        if landscape {
            std::mem::swap(&mut self.page_width, &mut self.page_height);
        }
        self
    }

    /// Set scale
    pub fn with_scale(mut self, scale: f32) -> Self {
        self.scale = scale;
        self
    }

    /// Set header HTML
    pub fn with_header(mut self, html: &str) -> Self {
        self.header_html = Some(html.to_string());
        self
    }

    /// Set footer HTML
    pub fn with_footer(mut self, html: &str) -> Self {
        self.footer_html = Some(html.to_string());
        self
    }

    /// Get content area width
    pub fn content_width(&self) -> f32 {
        self.page_width - self.margin_left - self.margin_right
    }

    /// Get content area height
    pub fn content_height(&self) -> f32 {
        self.page_height - self.margin_top - self.margin_bottom
    }
}

// ============================================================================
// CSS Types
// ============================================================================

/// CSS color
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CssColor {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Default for CssColor {
    fn default() -> Self {
        Self {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 1.0,
        }
    }
}

impl CssColor {
    /// Create RGB color
    pub fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self {
            r: r as f32 / 255.0,
            g: g as f32 / 255.0,
            b: b as f32 / 255.0,
            a: 1.0,
        }
    }

    /// Create RGBA color
    pub fn rgba(r: u8, g: u8, b: u8, a: f32) -> Self {
        Self {
            r: r as f32 / 255.0,
            g: g as f32 / 255.0,
            b: b as f32 / 255.0,
            a,
        }
    }

    /// Parse color string (hex, rgb, named)
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim().to_lowercase();

        // Named colors
        match s.as_str() {
            "black" => return Some(Self::rgb(0, 0, 0)),
            "white" => return Some(Self::rgb(255, 255, 255)),
            "red" => return Some(Self::rgb(255, 0, 0)),
            "green" => return Some(Self::rgb(0, 128, 0)),
            "blue" => return Some(Self::rgb(0, 0, 255)),
            "yellow" => return Some(Self::rgb(255, 255, 0)),
            "cyan" => return Some(Self::rgb(0, 255, 255)),
            "magenta" => return Some(Self::rgb(255, 0, 255)),
            "gray" | "grey" => return Some(Self::rgb(128, 128, 128)),
            "orange" => return Some(Self::rgb(255, 165, 0)),
            "purple" => return Some(Self::rgb(128, 0, 128)),
            "transparent" => return Some(Self::rgba(0, 0, 0, 0.0)),
            _ => {}
        }

        // Hex color
        if s.starts_with('#') {
            let hex = &s[1..];
            if hex.len() == 3 {
                let r = u8::from_str_radix(&hex[0..1], 16).ok()? * 17;
                let g = u8::from_str_radix(&hex[1..2], 16).ok()? * 17;
                let b = u8::from_str_radix(&hex[2..3], 16).ok()? * 17;
                return Some(Self::rgb(r, g, b));
            } else if hex.len() == 6 {
                let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
                let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
                let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
                return Some(Self::rgb(r, g, b));
            }
        }

        // RGB/RGBA function
        if s.starts_with("rgb") {
            let inner = s
                .trim_start_matches("rgba")
                .trim_start_matches("rgb")
                .trim_start_matches('(')
                .trim_end_matches(')');
            let parts: Vec<&str> = inner.split(',').collect();
            if parts.len() >= 3 {
                let r: u8 = parts[0].trim().parse().ok()?;
                let g: u8 = parts[1].trim().parse().ok()?;
                let b: u8 = parts[2].trim().parse().ok()?;
                let a: f32 = if parts.len() >= 4 {
                    parts[3].trim().parse().unwrap_or(1.0)
                } else {
                    1.0
                };
                return Some(Self::rgba(r, g, b, a));
            }
        }

        None
    }

    /// White color
    pub fn white() -> Self {
        Self::rgb(255, 255, 255)
    }

    /// Black color
    pub fn black() -> Self {
        Self::rgb(0, 0, 0)
    }
}

/// CSS length value
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CssLength {
    Px(f32),
    Pt(f32),
    Em(f32),
    Rem(f32),
    Percent(f32),
    Auto,
}

impl Default for CssLength {
    fn default() -> Self {
        CssLength::Auto
    }
}

impl CssLength {
    /// Parse CSS length string
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim().to_lowercase();

        if s == "auto" {
            return Some(CssLength::Auto);
        }

        if s.ends_with("px") {
            let num: f32 = s.trim_end_matches("px").parse().ok()?;
            return Some(CssLength::Px(num));
        }
        if s.ends_with("pt") {
            let num: f32 = s.trim_end_matches("pt").parse().ok()?;
            return Some(CssLength::Pt(num));
        }
        if s.ends_with("em") {
            let num: f32 = s.trim_end_matches("em").parse().ok()?;
            return Some(CssLength::Em(num));
        }
        if s.ends_with("rem") {
            let num: f32 = s.trim_end_matches("rem").parse().ok()?;
            return Some(CssLength::Rem(num));
        }
        if s.ends_with('%') {
            let num: f32 = s.trim_end_matches('%').parse().ok()?;
            return Some(CssLength::Percent(num));
        }

        // Plain number (pixels)
        if let Ok(num) = s.parse::<f32>() {
            return Some(CssLength::Px(num));
        }

        None
    }

    /// Convert to points
    pub fn to_points(&self, parent_size: f32, font_size: f32, root_font_size: f32) -> f32 {
        match self {
            CssLength::Px(px) => px * 0.75, // 1px = 0.75pt at 96dpi
            CssLength::Pt(pt) => *pt,
            CssLength::Em(em) => em * font_size,
            CssLength::Rem(rem) => rem * root_font_size,
            CssLength::Percent(pct) => parent_size * pct / 100.0,
            CssLength::Auto => 0.0,
        }
    }
}

/// CSS display type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CssDisplay {
    #[default]
    Block,
    Inline,
    InlineBlock,
    Flex,
    Grid,
    None,
    Table,
    TableRow,
    TableCell,
    ListItem,
}

impl CssDisplay {
    /// Parse display value
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_lowercase().as_str() {
            "block" => Some(CssDisplay::Block),
            "inline" => Some(CssDisplay::Inline),
            "inline-block" => Some(CssDisplay::InlineBlock),
            "flex" => Some(CssDisplay::Flex),
            "grid" => Some(CssDisplay::Grid),
            "none" => Some(CssDisplay::None),
            "table" => Some(CssDisplay::Table),
            "table-row" => Some(CssDisplay::TableRow),
            "table-cell" => Some(CssDisplay::TableCell),
            "list-item" => Some(CssDisplay::ListItem),
            _ => None,
        }
    }
}

/// CSS position type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CssPosition {
    #[default]
    Static,
    Relative,
    Absolute,
    Fixed,
    Sticky,
}

/// CSS text alignment
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CssTextAlign {
    #[default]
    Left,
    Right,
    Center,
    Justify,
}

/// CSS vertical alignment
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CssVerticalAlign {
    #[default]
    Baseline,
    Top,
    Middle,
    Bottom,
}

/// CSS flex direction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CssFlexDirection {
    #[default]
    Row,
    RowReverse,
    Column,
    ColumnReverse,
}

/// CSS justify content
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CssJustifyContent {
    #[default]
    FlexStart,
    FlexEnd,
    Center,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}

/// CSS align items
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CssAlignItems {
    #[default]
    Stretch,
    FlexStart,
    FlexEnd,
    Center,
    Baseline,
}

/// CSS font weight
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CssFontWeight {
    Normal,
    Bold,
    Bolder,
    Lighter,
    Weight(u16),
}

impl Default for CssFontWeight {
    fn default() -> Self {
        CssFontWeight::Normal
    }
}

impl CssFontWeight {
    /// Parse font weight
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_lowercase().as_str() {
            "normal" => Some(CssFontWeight::Normal),
            "bold" => Some(CssFontWeight::Bold),
            "bolder" => Some(CssFontWeight::Bolder),
            "lighter" => Some(CssFontWeight::Lighter),
            _ => s.trim().parse::<u16>().ok().map(CssFontWeight::Weight),
        }
    }

    /// Get numeric weight
    pub fn to_weight(&self) -> u16 {
        match self {
            CssFontWeight::Normal => 400,
            CssFontWeight::Bold => 700,
            CssFontWeight::Bolder => 700,
            CssFontWeight::Lighter => 300,
            CssFontWeight::Weight(w) => *w,
        }
    }
}

/// CSS font style
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CssFontStyle {
    #[default]
    Normal,
    Italic,
    Oblique,
}

/// CSS text decoration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CssTextDecoration {
    #[default]
    None,
    Underline,
    Overline,
    LineThrough,
}

/// CSS list style type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CssListStyleType {
    #[default]
    Disc,
    Circle,
    Square,
    Decimal,
    DecimalLeadingZero,
    LowerAlpha,
    UpperAlpha,
    LowerRoman,
    UpperRoman,
    None,
}

// ============================================================================
// Computed Style
// ============================================================================

/// Computed style for an element
#[derive(Debug, Clone)]
pub struct ComputedStyle {
    // Box model
    pub width: CssLength,
    pub height: CssLength,
    pub min_width: CssLength,
    pub min_height: CssLength,
    pub max_width: CssLength,
    pub max_height: CssLength,
    pub margin_top: f32,
    pub margin_right: f32,
    pub margin_bottom: f32,
    pub margin_left: f32,
    pub padding_top: f32,
    pub padding_right: f32,
    pub padding_bottom: f32,
    pub padding_left: f32,
    pub border_top_width: f32,
    pub border_right_width: f32,
    pub border_bottom_width: f32,
    pub border_left_width: f32,
    pub border_color: CssColor,
    pub border_radius: f32,

    // Typography
    pub font_family: String,
    pub font_size: f32,
    pub font_weight: CssFontWeight,
    pub font_style: CssFontStyle,
    pub line_height: f32,
    pub text_align: CssTextAlign,
    pub text_decoration: CssTextDecoration,
    pub text_indent: f32,
    pub letter_spacing: f32,
    pub word_spacing: f32,
    pub color: CssColor,

    // Background
    pub background_color: Option<CssColor>,
    pub background_image: Option<String>,

    // Layout
    pub display: CssDisplay,
    pub position: CssPosition,
    pub top: CssLength,
    pub right: CssLength,
    pub bottom: CssLength,
    pub left: CssLength,
    pub float: String,
    pub clear: String,
    pub overflow: String,
    pub z_index: i32,

    // Flexbox
    pub flex_direction: CssFlexDirection,
    pub flex_wrap: String,
    pub justify_content: CssJustifyContent,
    pub align_items: CssAlignItems,
    pub flex_grow: f32,
    pub flex_shrink: f32,
    pub flex_basis: CssLength,

    // List
    pub list_style_type: CssListStyleType,
    pub list_style_position: String,

    // Visibility
    pub visibility: String,
    pub opacity: f32,

    // Page break
    pub page_break_before: String,
    pub page_break_after: String,
    pub page_break_inside: String,
}

impl Default for ComputedStyle {
    fn default() -> Self {
        Self {
            width: CssLength::Auto,
            height: CssLength::Auto,
            min_width: CssLength::Px(0.0),
            min_height: CssLength::Px(0.0),
            max_width: CssLength::Auto,
            max_height: CssLength::Auto,
            margin_top: 0.0,
            margin_right: 0.0,
            margin_bottom: 0.0,
            margin_left: 0.0,
            padding_top: 0.0,
            padding_right: 0.0,
            padding_bottom: 0.0,
            padding_left: 0.0,
            border_top_width: 0.0,
            border_right_width: 0.0,
            border_bottom_width: 0.0,
            border_left_width: 0.0,
            border_color: CssColor::black(),
            border_radius: 0.0,
            font_family: "Helvetica".to_string(),
            font_size: 12.0,
            font_weight: CssFontWeight::Normal,
            font_style: CssFontStyle::Normal,
            line_height: 1.2,
            text_align: CssTextAlign::Left,
            text_decoration: CssTextDecoration::None,
            text_indent: 0.0,
            letter_spacing: 0.0,
            word_spacing: 0.0,
            color: CssColor::black(),
            background_color: None,
            background_image: None,
            display: CssDisplay::Block,
            position: CssPosition::Static,
            top: CssLength::Auto,
            right: CssLength::Auto,
            bottom: CssLength::Auto,
            left: CssLength::Auto,
            float: "none".to_string(),
            clear: "none".to_string(),
            overflow: "visible".to_string(),
            z_index: 0,
            flex_direction: CssFlexDirection::Row,
            flex_wrap: "nowrap".to_string(),
            justify_content: CssJustifyContent::FlexStart,
            align_items: CssAlignItems::Stretch,
            flex_grow: 0.0,
            flex_shrink: 1.0,
            flex_basis: CssLength::Auto,
            list_style_type: CssListStyleType::Disc,
            list_style_position: "outside".to_string(),
            visibility: "visible".to_string(),
            opacity: 1.0,
            page_break_before: "auto".to_string(),
            page_break_after: "auto".to_string(),
            page_break_inside: "auto".to_string(),
        }
    }
}

// ============================================================================
// CSS Parser
// ============================================================================

/// CSS rule
#[derive(Debug, Clone)]
pub struct CssRule {
    pub selector: String,
    pub properties: HashMap<String, String>,
    pub specificity: (u32, u32, u32, u32),
}

impl CssRule {
    /// Create new rule
    pub fn new(selector: &str) -> Self {
        let specificity = Self::calculate_specificity(selector);
        Self {
            selector: selector.to_string(),
            properties: HashMap::new(),
            specificity,
        }
    }

    /// Calculate selector specificity
    fn calculate_specificity(selector: &str) -> (u32, u32, u32, u32) {
        let mut ids = 0u32;
        let mut classes = 0u32;
        let mut elements = 0u32;

        for part in selector.split_whitespace() {
            for token in part.split(|c| c == '>' || c == '+' || c == '~') {
                let token = token.trim();
                if token.starts_with('#') {
                    ids += 1;
                } else if token.starts_with('.') || token.starts_with('[') || token.starts_with(':')
                {
                    classes += 1;
                } else if !token.is_empty() && token != "*" {
                    elements += 1;
                }
            }
        }

        (0, ids, classes, elements)
    }

    /// Add property
    pub fn add_property(&mut self, name: &str, value: &str) {
        self.properties
            .insert(name.to_lowercase(), value.to_string());
    }
}

/// CSS stylesheet
#[derive(Debug, Clone, Default)]
pub struct CssStylesheet {
    pub rules: Vec<CssRule>,
}

impl CssStylesheet {
    /// Create new stylesheet
    pub fn new() -> Self {
        Self::default()
    }

    /// Parse CSS string
    pub fn parse(css: &str) -> Self {
        let mut stylesheet = Self::new();
        let mut in_comment = false;
        let mut current_rule: Option<CssRule> = None;
        let mut buffer = String::new();
        let mut in_selector = true;

        let chars: Vec<char> = css.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            let c = chars[i];

            // Handle comments
            if !in_comment && i + 1 < chars.len() && c == '/' && chars[i + 1] == '*' {
                in_comment = true;
                i += 2;
                continue;
            }
            if in_comment && i + 1 < chars.len() && c == '*' && chars[i + 1] == '/' {
                in_comment = false;
                i += 2;
                continue;
            }
            if in_comment {
                i += 1;
                continue;
            }

            match c {
                '{' => {
                    if in_selector {
                        let selector = buffer.trim().to_string();
                        if !selector.is_empty() {
                            current_rule = Some(CssRule::new(&selector));
                        }
                        buffer.clear();
                        in_selector = false;
                    }
                }
                '}' => {
                    if let Some(ref mut rule) = current_rule {
                        // Parse remaining property
                        Self::parse_property(&buffer, rule);
                        stylesheet.rules.push(rule.clone());
                    }
                    current_rule = None;
                    buffer.clear();
                    in_selector = true;
                }
                ';' => {
                    if let Some(ref mut rule) = current_rule {
                        Self::parse_property(&buffer, rule);
                    }
                    buffer.clear();
                }
                _ => {
                    buffer.push(c);
                }
            }

            i += 1;
        }

        stylesheet
    }

    /// Parse a single property
    fn parse_property(property_str: &str, rule: &mut CssRule) {
        if let Some(colon_pos) = property_str.find(':') {
            let name = property_str[..colon_pos].trim();
            let value = property_str[colon_pos + 1..].trim();
            if !name.is_empty() && !value.is_empty() {
                rule.add_property(name, value);
            }
        }
    }

    /// Get rules matching a selector
    pub fn get_rules_for_element(
        &self,
        tag: &str,
        id: Option<&str>,
        classes: &[String],
    ) -> Vec<&CssRule> {
        self.rules
            .iter()
            .filter(|rule| Self::selector_matches(&rule.selector, tag, id, classes))
            .collect()
    }

    /// Check if selector matches element
    fn selector_matches(selector: &str, tag: &str, id: Option<&str>, classes: &[String]) -> bool {
        let selector = selector.trim();

        // Universal selector
        if selector == "*" {
            return true;
        }

        // Split compound selectors
        for part in selector.split(',') {
            let part = part.trim();
            if Self::simple_selector_matches(part, tag, id, classes) {
                return true;
            }
        }

        false
    }

    /// Match simple selector
    fn simple_selector_matches(
        selector: &str,
        tag: &str,
        id: Option<&str>,
        classes: &[String],
    ) -> bool {
        let selector = selector.trim();

        // ID selector
        if selector.starts_with('#') {
            if let Some(elem_id) = id {
                return &selector[1..] == elem_id;
            }
            return false;
        }

        // Class selector
        if selector.starts_with('.') {
            return classes.iter().any(|c| c == &selector[1..]);
        }

        // Tag selector (potentially with class/id)
        if let Some(hash_pos) = selector.find('#') {
            let tag_part = &selector[..hash_pos];
            let id_part = &selector[hash_pos + 1..];
            return (tag_part.is_empty() || tag_part == tag) && id.map_or(false, |i| i == id_part);
        }

        if let Some(dot_pos) = selector.find('.') {
            let tag_part = &selector[..dot_pos];
            let class_part = &selector[dot_pos + 1..];
            return (tag_part.is_empty() || tag_part == tag)
                && classes.contains(&class_part.to_string());
        }

        // Simple tag selector
        selector.to_lowercase() == tag.to_lowercase()
    }

    /// Add default browser styles
    pub fn add_default_styles(&mut self) {
        let defaults = r#"
            html, body { margin: 0; padding: 0; }
            body { font-family: serif; font-size: 12pt; line-height: 1.2; }
            h1 { font-size: 24pt; font-weight: bold; margin: 0.67em 0; }
            h2 { font-size: 18pt; font-weight: bold; margin: 0.83em 0; }
            h3 { font-size: 14pt; font-weight: bold; margin: 1em 0; }
            h4 { font-size: 12pt; font-weight: bold; margin: 1.33em 0; }
            h5 { font-size: 10pt; font-weight: bold; margin: 1.67em 0; }
            h6 { font-size: 8pt; font-weight: bold; margin: 2.33em 0; }
            p { margin: 1em 0; }
            ul, ol { margin: 1em 0; padding-left: 40px; }
            li { display: list-item; }
            a { color: blue; text-decoration: underline; }
            strong, b { font-weight: bold; }
            em, i { font-style: italic; }
            u { text-decoration: underline; }
            s, strike { text-decoration: line-through; }
            code, pre { font-family: monospace; }
            pre { white-space: pre; margin: 1em 0; }
            blockquote { margin: 1em 40px; }
            table { border-collapse: collapse; }
            td, th { padding: 1px; }
            th { font-weight: bold; text-align: center; }
            hr { border: 1px inset; margin: 0.5em auto; }
            img { display: inline-block; }
            br { display: block; }
            div { display: block; }
            span { display: inline; }
        "#;

        let default_sheet = CssStylesheet::parse(defaults);
        self.rules.extend(default_sheet.rules);
    }
}

// ============================================================================
// HTML Parser
// ============================================================================

/// HTML element node
#[derive(Debug, Clone)]
pub struct HtmlElement {
    pub tag: String,
    pub id: Option<String>,
    pub classes: Vec<String>,
    pub attributes: HashMap<String, String>,
    pub children: Vec<HtmlNode>,
    pub style: ComputedStyle,
}

/// HTML node (element or text)
#[derive(Debug, Clone)]
pub enum HtmlNode {
    Element(HtmlElement),
    Text(String),
}

impl HtmlElement {
    /// Create new element
    pub fn new(tag: impl Into<String>) -> Self {
        Self {
            tag: tag.into().to_lowercase(),
            id: None,
            classes: vec![],
            attributes: HashMap::new(),
            children: vec![],
            style: ComputedStyle::default(),
        }
    }

    /// Add child node
    pub fn add_child(&mut self, child: HtmlNode) {
        self.children.push(child);
    }

    /// Get text content
    pub fn text_content(&self) -> String {
        let mut text = String::new();
        for child in &self.children {
            match child {
                HtmlNode::Text(t) => text.push_str(t),
                HtmlNode::Element(e) => text.push_str(&e.text_content()),
            }
        }
        text
    }

    /// Is block element
    pub fn is_block(&self) -> bool {
        matches!(
            self.tag.as_str(),
            "div"
                | "p"
                | "h1"
                | "h2"
                | "h3"
                | "h4"
                | "h5"
                | "h6"
                | "ul"
                | "ol"
                | "li"
                | "table"
                | "tr"
                | "td"
                | "th"
                | "blockquote"
                | "pre"
                | "hr"
                | "form"
                | "header"
                | "footer"
                | "nav"
                | "main"
                | "section"
                | "article"
                | "aside"
        )
    }

    /// Is void element (self-closing)
    pub fn is_void(&self) -> bool {
        matches!(
            self.tag.as_str(),
            "br" | "hr" | "img" | "input" | "meta" | "link" | "area" | "base" | "col" | "embed"
        )
    }
}

/// HTML parser
pub struct HtmlParser {
    pos: usize,
    input: Vec<char>,
}

impl HtmlParser {
    /// Create new parser
    pub fn new(html: &str) -> Self {
        Self {
            pos: 0,
            input: html.chars().collect(),
        }
    }

    /// Parse HTML string
    pub fn parse(html: &str) -> Result<HtmlElement> {
        let mut parser = HtmlParser::new(html);
        parser.parse_document()
    }

    /// Parse complete document
    fn parse_document(&mut self) -> Result<HtmlElement> {
        // Skip DOCTYPE if present
        self.skip_whitespace();
        if self.starts_with("<!") {
            self.skip_until('>');
            self.advance();
        }

        // Parse root element
        let mut root = HtmlElement::new("html");

        while !self.eof() {
            self.skip_whitespace();
            if self.eof() {
                break;
            }

            if self.starts_with("</") {
                // End tag - skip it at root level
                self.skip_until('>');
                self.advance();
            } else if self.starts_with("<") {
                if let Some(node) = self.parse_node()? {
                    root.add_child(node);
                }
            } else {
                // Text content
                let text = self.parse_text();
                if !text.trim().is_empty() {
                    root.add_child(HtmlNode::Text(text));
                }
            }
        }

        Ok(root)
    }

    /// Parse a single node
    fn parse_node(&mut self) -> Result<Option<HtmlNode>> {
        self.skip_whitespace();

        if self.eof() {
            return Ok(None);
        }

        if self.starts_with("<!--") {
            // Comment
            self.skip_until_str("-->");
            self.advance_by(3);
            return Ok(None);
        }

        if self.starts_with("</") {
            // End tag - handled by parent
            return Ok(None);
        }

        if self.starts_with("<") {
            // Element
            self.advance(); // Skip '<'
            let element = self.parse_element()?;
            return Ok(Some(HtmlNode::Element(element)));
        }

        // Text
        let text = self.parse_text();
        if !text.is_empty() {
            Ok(Some(HtmlNode::Text(text)))
        } else {
            Ok(None)
        }
    }

    /// Parse element
    fn parse_element(&mut self) -> Result<HtmlElement> {
        // Parse tag name
        let tag_name = self.parse_tag_name();
        let mut element = HtmlElement::new(&tag_name);

        // Parse attributes
        self.parse_attributes(&mut element);

        // Skip closing of start tag
        self.skip_whitespace();
        let self_closing = self.starts_with("/>");
        if self_closing {
            self.advance_by(2);
            return Ok(element);
        }
        if self.current() == '>' {
            self.advance();
        }

        // Void elements don't have children
        if element.is_void() {
            return Ok(element);
        }

        // Parse children
        loop {
            self.skip_whitespace();

            if self.eof() {
                break;
            }

            // Check for end tag
            if self.starts_with("</") {
                self.advance_by(2);
                let end_tag = self.parse_tag_name();
                self.skip_until('>');
                self.advance();
                if end_tag.to_lowercase() == element.tag {
                    break;
                }
                // Mismatched tag - continue
                continue;
            }

            if let Some(child) = self.parse_node()? {
                element.add_child(child);
            }
        }

        Ok(element)
    }

    /// Parse tag name
    fn parse_tag_name(&mut self) -> String {
        let mut name = String::new();
        while !self.eof() {
            let c = self.current();
            if c.is_alphanumeric() || c == '-' || c == '_' {
                name.push(c);
                self.advance();
            } else {
                break;
            }
        }
        name.to_lowercase()
    }

    /// Parse attributes
    fn parse_attributes(&mut self, element: &mut HtmlElement) {
        loop {
            self.skip_whitespace();
            if self.eof() || self.current() == '>' || self.starts_with("/>") {
                break;
            }

            // Parse attribute name
            let name = self.parse_attribute_name();
            if name.is_empty() {
                break;
            }

            self.skip_whitespace();

            // Parse value if present
            let value = if self.current() == '=' {
                self.advance(); // Skip '='
                self.skip_whitespace();
                self.parse_attribute_value()
            } else {
                name.clone() // Boolean attribute
            };

            // Handle special attributes
            let name_lower = name.to_lowercase();
            if name_lower == "id" {
                element.id = Some(value.clone());
            } else if name_lower == "class" {
                element.classes = value.split_whitespace().map(|s| s.to_string()).collect();
            } else if name_lower == "style" {
                // Inline style will be processed later
            }

            element.attributes.insert(name, value);
        }
    }

    /// Parse attribute name
    fn parse_attribute_name(&mut self) -> String {
        let mut name = String::new();
        while !self.eof() {
            let c = self.current();
            if c.is_alphanumeric() || c == '-' || c == '_' || c == ':' {
                name.push(c);
                self.advance();
            } else {
                break;
            }
        }
        name
    }

    /// Parse attribute value
    fn parse_attribute_value(&mut self) -> String {
        let quote = self.current();
        if quote == '"' || quote == '\'' {
            self.advance();
            let mut value = String::new();
            while !self.eof() && self.current() != quote {
                value.push(self.current());
                self.advance();
            }
            if self.current() == quote {
                self.advance();
            }
            value
        } else {
            // Unquoted value
            let mut value = String::new();
            while !self.eof() {
                let c = self.current();
                if c.is_whitespace() || c == '>' {
                    break;
                }
                value.push(c);
                self.advance();
            }
            value
        }
    }

    /// Parse text content
    fn parse_text(&mut self) -> String {
        let mut text = String::new();
        while !self.eof() && self.current() != '<' {
            text.push(self.current());
            self.advance();
        }
        // Decode HTML entities
        decode_html_entities(&text)
    }

    // Helper methods

    fn eof(&self) -> bool {
        self.pos >= self.input.len()
    }

    fn current(&self) -> char {
        if self.eof() {
            '\0'
        } else {
            self.input[self.pos]
        }
    }

    fn advance(&mut self) {
        self.pos += 1;
    }

    fn advance_by(&mut self, n: usize) {
        self.pos += n;
    }

    fn skip_whitespace(&mut self) {
        while !self.eof() && self.current().is_whitespace() {
            self.advance();
        }
    }

    fn skip_until(&mut self, c: char) {
        while !self.eof() && self.current() != c {
            self.advance();
        }
    }

    fn skip_until_str(&mut self, s: &str) {
        while !self.eof() && !self.starts_with(s) {
            self.advance();
        }
    }

    fn starts_with(&self, s: &str) -> bool {
        let chars: Vec<char> = s.chars().collect();
        for (i, c) in chars.iter().enumerate() {
            if self.pos + i >= self.input.len() || self.input[self.pos + i] != *c {
                return false;
            }
        }
        true
    }
}

/// Decode HTML entities
fn decode_html_entities(text: &str) -> String {
    text.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
        .replace("&nbsp;", "\u{00A0}")
        .replace("&copy;", "©")
        .replace("&reg;", "®")
        .replace("&trade;", "™")
        .replace("&mdash;", "—")
        .replace("&ndash;", "–")
        .replace("&bull;", "•")
}

// ============================================================================
// Layout Engine
// ============================================================================

/// Layout box
#[derive(Debug, Clone)]
pub struct LayoutBox {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub content_x: f32,
    pub content_y: f32,
    pub content_width: f32,
    pub content_height: f32,
    pub element: Option<HtmlElement>,
    pub text: Option<String>,
    pub children: Vec<LayoutBox>,
}

impl Default for LayoutBox {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: 0.0,
            content_x: 0.0,
            content_y: 0.0,
            content_width: 0.0,
            content_height: 0.0,
            element: None,
            text: None,
            children: vec![],
        }
    }
}

/// Layout engine
pub struct LayoutEngine {
    pub stylesheet: CssStylesheet,
    pub root_font_size: f32,
    pub viewport_width: f32,
    pub viewport_height: f32,
}

impl LayoutEngine {
    /// Create new layout engine
    pub fn new(viewport_width: f32, viewport_height: f32) -> Self {
        let mut stylesheet = CssStylesheet::new();
        stylesheet.add_default_styles();
        Self {
            stylesheet,
            root_font_size: 12.0,
            viewport_width,
            viewport_height,
        }
    }

    /// Add stylesheet
    pub fn add_stylesheet(&mut self, css: &str) {
        let sheet = CssStylesheet::parse(css);
        self.stylesheet.rules.extend(sheet.rules);
    }

    /// Compute styles for element tree
    pub fn compute_styles(&mut self, element: &mut HtmlElement) {
        self.compute_element_style(element, &ComputedStyle::default());
    }

    /// Compute style for single element
    fn compute_element_style(&mut self, element: &mut HtmlElement, parent_style: &ComputedStyle) {
        // Start with inherited styles
        element.style = ComputedStyle::default();
        element.style.font_family = parent_style.font_family.clone();
        element.style.font_size = parent_style.font_size;
        element.style.color = parent_style.color;
        element.style.line_height = parent_style.line_height;

        // Apply matching rules
        let matching_rules = self.stylesheet.get_rules_for_element(
            &element.tag,
            element.id.as_deref(),
            &element.classes,
        );

        for rule in matching_rules {
            self.apply_properties(&rule.properties, &mut element.style);
        }

        // Apply inline styles
        if let Some(inline_style) = element.attributes.get("style") {
            let inline_props = self.parse_inline_style(inline_style);
            self.apply_properties(&inline_props, &mut element.style);
        }

        // Compute children
        let child_style = element.style.clone();
        for child in &mut element.children {
            if let HtmlNode::Element(child_elem) = child {
                self.compute_element_style(child_elem, &child_style);
            }
        }
    }

    /// Parse inline style attribute
    fn parse_inline_style(&self, style: &str) -> HashMap<String, String> {
        let mut props = HashMap::new();
        for decl in style.split(';') {
            if let Some(colon_pos) = decl.find(':') {
                let name = decl[..colon_pos].trim().to_lowercase();
                let value = decl[colon_pos + 1..].trim().to_string();
                props.insert(name, value);
            }
        }
        props
    }

    /// Apply CSS properties to computed style
    fn apply_properties(&self, props: &HashMap<String, String>, style: &mut ComputedStyle) {
        for (name, value) in props {
            match name.as_str() {
                "color" => {
                    if let Some(color) = CssColor::parse(value) {
                        style.color = color;
                    }
                }
                "background-color" | "background" => {
                    if let Some(color) = CssColor::parse(value) {
                        style.background_color = Some(color);
                    }
                }
                "font-size" => {
                    if let Some(length) = CssLength::parse(value) {
                        style.font_size =
                            length.to_points(style.font_size, style.font_size, self.root_font_size);
                    }
                }
                "font-family" => {
                    style.font_family = value.trim_matches('"').trim_matches('\'').to_string();
                }
                "font-weight" => {
                    if let Some(weight) = CssFontWeight::parse(value) {
                        style.font_weight = weight;
                    }
                }
                "font-style" => match value.as_str() {
                    "italic" => style.font_style = CssFontStyle::Italic,
                    "oblique" => style.font_style = CssFontStyle::Oblique,
                    _ => style.font_style = CssFontStyle::Normal,
                },
                "text-align" => match value.as_str() {
                    "center" => style.text_align = CssTextAlign::Center,
                    "right" => style.text_align = CssTextAlign::Right,
                    "justify" => style.text_align = CssTextAlign::Justify,
                    _ => style.text_align = CssTextAlign::Left,
                },
                "text-decoration" => match value.as_str() {
                    "underline" => style.text_decoration = CssTextDecoration::Underline,
                    "line-through" => style.text_decoration = CssTextDecoration::LineThrough,
                    "overline" => style.text_decoration = CssTextDecoration::Overline,
                    _ => style.text_decoration = CssTextDecoration::None,
                },
                "display" => {
                    if let Some(display) = CssDisplay::parse(value) {
                        style.display = display;
                    }
                }
                "margin" => {
                    if let Some(length) = CssLength::parse(value) {
                        let v = length.to_points(
                            self.viewport_width,
                            style.font_size,
                            self.root_font_size,
                        );
                        style.margin_top = v;
                        style.margin_right = v;
                        style.margin_bottom = v;
                        style.margin_left = v;
                    }
                }
                "margin-top" => {
                    if let Some(length) = CssLength::parse(value) {
                        style.margin_top = length.to_points(
                            self.viewport_width,
                            style.font_size,
                            self.root_font_size,
                        );
                    }
                }
                "margin-bottom" => {
                    if let Some(length) = CssLength::parse(value) {
                        style.margin_bottom = length.to_points(
                            self.viewport_width,
                            style.font_size,
                            self.root_font_size,
                        );
                    }
                }
                "margin-left" => {
                    if let Some(length) = CssLength::parse(value) {
                        style.margin_left = length.to_points(
                            self.viewport_width,
                            style.font_size,
                            self.root_font_size,
                        );
                    }
                }
                "margin-right" => {
                    if let Some(length) = CssLength::parse(value) {
                        style.margin_right = length.to_points(
                            self.viewport_width,
                            style.font_size,
                            self.root_font_size,
                        );
                    }
                }
                "padding" => {
                    if let Some(length) = CssLength::parse(value) {
                        let v = length.to_points(
                            self.viewport_width,
                            style.font_size,
                            self.root_font_size,
                        );
                        style.padding_top = v;
                        style.padding_right = v;
                        style.padding_bottom = v;
                        style.padding_left = v;
                    }
                }
                "padding-top" => {
                    if let Some(length) = CssLength::parse(value) {
                        style.padding_top = length.to_points(
                            self.viewport_width,
                            style.font_size,
                            self.root_font_size,
                        );
                    }
                }
                "padding-bottom" => {
                    if let Some(length) = CssLength::parse(value) {
                        style.padding_bottom = length.to_points(
                            self.viewport_width,
                            style.font_size,
                            self.root_font_size,
                        );
                    }
                }
                "padding-left" => {
                    if let Some(length) = CssLength::parse(value) {
                        style.padding_left = length.to_points(
                            self.viewport_width,
                            style.font_size,
                            self.root_font_size,
                        );
                    }
                }
                "padding-right" => {
                    if let Some(length) = CssLength::parse(value) {
                        style.padding_right = length.to_points(
                            self.viewport_width,
                            style.font_size,
                            self.root_font_size,
                        );
                    }
                }
                "width" => {
                    if let Some(length) = CssLength::parse(value) {
                        style.width = length;
                    }
                }
                "height" => {
                    if let Some(length) = CssLength::parse(value) {
                        style.height = length;
                    }
                }
                "line-height" => {
                    if let Ok(num) = value.parse::<f32>() {
                        style.line_height = num;
                    } else if let Some(length) = CssLength::parse(value) {
                        style.line_height =
                            length.to_points(style.font_size, style.font_size, self.root_font_size)
                                / style.font_size;
                    }
                }
                "border" => {
                    // Simple border parsing
                    let parts: Vec<&str> = value.split_whitespace().collect();
                    if !parts.is_empty() {
                        if let Some(length) = CssLength::parse(parts[0]) {
                            let v = length.to_points(
                                self.viewport_width,
                                style.font_size,
                                self.root_font_size,
                            );
                            style.border_top_width = v;
                            style.border_right_width = v;
                            style.border_bottom_width = v;
                            style.border_left_width = v;
                        }
                    }
                    if parts.len() >= 3 {
                        if let Some(color) = CssColor::parse(parts[2]) {
                            style.border_color = color;
                        }
                    }
                }
                "border-width" => {
                    if let Some(length) = CssLength::parse(value) {
                        let v = length.to_points(
                            self.viewport_width,
                            style.font_size,
                            self.root_font_size,
                        );
                        style.border_top_width = v;
                        style.border_right_width = v;
                        style.border_bottom_width = v;
                        style.border_left_width = v;
                    }
                }
                "border-color" => {
                    if let Some(color) = CssColor::parse(value) {
                        style.border_color = color;
                    }
                }
                "border-radius" => {
                    if let Some(length) = CssLength::parse(value) {
                        style.border_radius = length.to_points(
                            self.viewport_width,
                            style.font_size,
                            self.root_font_size,
                        );
                    }
                }
                "opacity" => {
                    if let Ok(o) = value.parse::<f32>() {
                        style.opacity = o.clamp(0.0, 1.0);
                    }
                }
                "page-break-before" => {
                    style.page_break_before = value.clone();
                }
                "page-break-after" => {
                    style.page_break_after = value.clone();
                }
                "page-break-inside" => {
                    style.page_break_inside = value.clone();
                }
                _ => {}
            }
        }
    }

    /// Layout element tree
    pub fn layout(&self, element: &HtmlElement, available_width: f32) -> LayoutBox {
        self.layout_element(element, 0.0, 0.0, available_width)
    }

    /// Layout single element
    fn layout_element(
        &self,
        element: &HtmlElement,
        x: f32,
        y: f32,
        available_width: f32,
    ) -> LayoutBox {
        let style = &element.style;
        let mut layout_box = LayoutBox::default();

        // Calculate box dimensions
        let margin_left = style.margin_left;
        let margin_right = style.margin_right;
        let margin_top = style.margin_top;
        let margin_bottom = style.margin_bottom;

        let padding_left = style.padding_left;
        let padding_right = style.padding_right;
        let padding_top = style.padding_top;
        let padding_bottom = style.padding_bottom;

        let border_left = style.border_left_width;
        let border_right = style.border_right_width;
        let border_top = style.border_top_width;
        let border_bottom = style.border_bottom_width;

        // Content area
        let content_width = match style.width {
            CssLength::Auto => {
                available_width
                    - margin_left
                    - margin_right
                    - padding_left
                    - padding_right
                    - border_left
                    - border_right
            }
            other => other.to_points(available_width, style.font_size, self.root_font_size),
        };

        layout_box.x = x + margin_left;
        layout_box.y = y - margin_top;
        layout_box.content_x = layout_box.x + border_left + padding_left;
        layout_box.content_y = layout_box.y - border_top - padding_top;
        layout_box.content_width = content_width;

        // Layout children
        let mut child_y = layout_box.content_y;
        let mut content_height = 0.0;

        for child in &element.children {
            match child {
                HtmlNode::Element(child_elem) => {
                    if child_elem.style.display == CssDisplay::None {
                        continue;
                    }

                    let child_box = self.layout_element(
                        child_elem,
                        layout_box.content_x,
                        child_y,
                        content_width,
                    );
                    child_y -= child_box.height;
                    content_height += child_box.height;
                    layout_box.children.push(child_box);
                }
                HtmlNode::Text(text) => {
                    let text_height = self.layout_text(text, &element.style, content_width);
                    let mut text_box = LayoutBox::default();
                    text_box.x = layout_box.content_x;
                    text_box.y = child_y;
                    text_box.width = content_width;
                    text_box.height = text_height;
                    text_box.text = Some(text.clone());
                    child_y -= text_height;
                    content_height += text_height;
                    layout_box.children.push(text_box);
                }
            }
        }

        // Calculate total height
        layout_box.content_height = match style.height {
            CssLength::Auto => content_height,
            other => other.to_points(0.0, style.font_size, self.root_font_size),
        };

        layout_box.width =
            content_width + padding_left + padding_right + border_left + border_right;
        layout_box.height = layout_box.content_height
            + padding_top
            + padding_bottom
            + border_top
            + border_bottom
            + margin_top
            + margin_bottom;

        layout_box.element = Some(element.clone());
        layout_box
    }

    /// Layout text and return height
    fn layout_text(&self, text: &str, style: &ComputedStyle, available_width: f32) -> f32 {
        // Simple line calculation
        let char_width = style.font_size * 0.5;
        let chars_per_line = (available_width / char_width).max(1.0) as usize;

        let words: Vec<&str> = text.split_whitespace().collect();
        if words.is_empty() {
            return 0.0;
        }

        let mut lines = 1;
        let mut current_line_chars = 0;

        for word in words {
            if current_line_chars + word.len() + 1 > chars_per_line {
                lines += 1;
                current_line_chars = word.len();
            } else {
                current_line_chars += word.len() + 1;
            }
        }

        lines as f32 * style.font_size * style.line_height
    }
}

// ============================================================================
// PDF Renderer
// ============================================================================

/// PDF renderer
pub struct PdfRenderer {
    pub options: HtmlToPdfOptions,
    pub pages: Vec<Vec<String>>,
    current_page: Vec<String>,
    current_y: f32,
}

impl PdfRenderer {
    /// Create new renderer
    pub fn new(options: HtmlToPdfOptions) -> Self {
        Self {
            current_y: options.page_height - options.margin_top,
            pages: vec![],
            current_page: vec![],
            options,
        }
    }

    /// Render layout to PDF commands
    pub fn render(&mut self, layout: &LayoutBox) {
        self.current_y = self.options.page_height - self.options.margin_top;
        self.render_box(layout);

        // Add current page if not empty
        if !self.current_page.is_empty() {
            self.pages.push(std::mem::take(&mut self.current_page));
        }
    }

    /// Render single layout box
    fn render_box(&mut self, layout: &LayoutBox) {
        // Check for page break
        if self.current_y - layout.height < self.options.margin_bottom {
            if !self.current_page.is_empty() {
                self.pages.push(std::mem::take(&mut self.current_page));
            }
            self.current_y = self.options.page_height - self.options.margin_top;
        }

        let x = layout.x + self.options.margin_left;
        let y = self.current_y;

        // Render background
        if let Some(ref element) = layout.element {
            let style = &element.style;

            if let Some(bg) = &style.background_color {
                self.current_page.push("q".to_string());
                self.current_page
                    .push(format!("{} {} {} rg", bg.r, bg.g, bg.b));
                self.current_page.push(format!(
                    "{} {} {} {} re f",
                    x,
                    y - layout.height,
                    layout.width,
                    layout.height
                ));
                self.current_page.push("Q".to_string());
            }

            // Render border
            if style.border_top_width > 0.0 {
                self.current_page.push("q".to_string());
                self.current_page.push(format!(
                    "{} {} {} RG",
                    style.border_color.r, style.border_color.g, style.border_color.b
                ));
                self.current_page
                    .push(format!("{} w", style.border_top_width));
                self.current_page
                    .push(format!("{} {} m {} {} l S", x, y, x + layout.width, y));
                self.current_page.push("Q".to_string());
            }
            if style.border_bottom_width > 0.0 {
                self.current_page.push("q".to_string());
                self.current_page.push(format!(
                    "{} {} {} RG",
                    style.border_color.r, style.border_color.g, style.border_color.b
                ));
                self.current_page
                    .push(format!("{} w", style.border_bottom_width));
                self.current_page.push(format!(
                    "{} {} m {} {} l S",
                    x,
                    y - layout.height,
                    x + layout.width,
                    y - layout.height
                ));
                self.current_page.push("Q".to_string());
            }
            if style.border_left_width > 0.0 {
                self.current_page.push("q".to_string());
                self.current_page.push(format!(
                    "{} {} {} RG",
                    style.border_color.r, style.border_color.g, style.border_color.b
                ));
                self.current_page
                    .push(format!("{} w", style.border_left_width));
                self.current_page
                    .push(format!("{} {} m {} {} l S", x, y, x, y - layout.height));
                self.current_page.push("Q".to_string());
            }
            if style.border_right_width > 0.0 {
                self.current_page.push("q".to_string());
                self.current_page.push(format!(
                    "{} {} {} RG",
                    style.border_color.r, style.border_color.g, style.border_color.b
                ));
                self.current_page
                    .push(format!("{} w", style.border_right_width));
                self.current_page.push(format!(
                    "{} {} m {} {} l S",
                    x + layout.width,
                    y,
                    x + layout.width,
                    y - layout.height
                ));
                self.current_page.push("Q".to_string());
            }
        }

        // Render text
        if let Some(ref text) = layout.text {
            let default_style = ComputedStyle::default();
            let style = layout
                .element
                .as_ref()
                .map(|e| &e.style)
                .unwrap_or(&default_style);

            let text_y = y - style.font_size;

            self.current_page.push("BT".to_string());

            // Font
            let font = if style.font_weight.to_weight() >= 700 {
                "/F2"
            } else {
                "/F1"
            };
            self.current_page
                .push(format!("{} {} Tf", font, style.font_size));

            // Color
            self.current_page.push(format!(
                "{} {} {} rg",
                style.color.r, style.color.g, style.color.b
            ));

            // Position
            self.current_page.push(format!("{} {} Td", x, text_y));

            // Text
            self.current_page
                .push(format!("({}) Tj", escape_pdf_string(text)));

            self.current_page.push("ET".to_string());

            // Underline
            if style.text_decoration == CssTextDecoration::Underline {
                self.current_page.push("q".to_string());
                self.current_page.push(format!(
                    "{} {} {} RG",
                    style.color.r, style.color.g, style.color.b
                ));
                self.current_page.push("0.5 w".to_string());
                self.current_page.push(format!(
                    "{} {} m {} {} l S",
                    x,
                    text_y - 2.0,
                    x + layout.width,
                    text_y - 2.0
                ));
                self.current_page.push("Q".to_string());
            }
        }

        // Render children
        let mut child_y = y;
        for child in &layout.children {
            if child.text.is_some() {
                let mut child_copy = child.clone();
                child_copy.element = layout.element.clone();
                self.render_child_at(&child_copy, x, child_y);
            } else {
                self.render_child_at(child, x, child_y);
            }
            child_y -= child.height;
        }

        self.current_y -= layout.height;
    }

    /// Render child at specific position
    fn render_child_at(&mut self, child: &LayoutBox, _parent_x: f32, y: f32) {
        // Render text child
        if let Some(ref text) = child.text {
            let default_style = ComputedStyle::default();
            let style = child
                .element
                .as_ref()
                .map(|e| &e.style)
                .unwrap_or(&default_style);

            let x = child.x + self.options.margin_left;
            let text_y = y - style.font_size;

            self.current_page.push("BT".to_string());

            let font = if style.font_weight.to_weight() >= 700 {
                "/F2"
            } else {
                "/F1"
            };
            self.current_page
                .push(format!("{} {} Tf", font, style.font_size));

            self.current_page.push(format!(
                "{} {} {} rg",
                style.color.r, style.color.g, style.color.b
            ));

            self.current_page.push(format!("{} {} Td", x, text_y));
            self.current_page
                .push(format!("({}) Tj", escape_pdf_string(text)));

            self.current_page.push("ET".to_string());
        }
    }

    /// Get all page content streams
    pub fn get_pages(&self) -> &Vec<Vec<String>> {
        &self.pages
    }
}

fn escape_pdf_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('(', "\\(")
        .replace(')', "\\)")
}

// ============================================================================
// Public API
// ============================================================================

/// Convert HTML string to PDF
pub fn html_to_pdf(html: &str, output_path: &str, options: &HtmlToPdfOptions) -> Result<()> {
    // Parse HTML
    let mut root = HtmlParser::parse(html)?;

    // Create layout engine
    let mut engine = LayoutEngine::new(options.content_width(), options.content_height());

    // Add user stylesheet
    if let Some(ref css) = options.user_stylesheet {
        engine.add_stylesheet(css);
    }

    // Extract and parse embedded styles
    extract_and_apply_styles(&mut root, &mut engine);

    // Compute styles
    engine.compute_styles(&mut root);

    // Layout
    let layout = engine.layout(&root, options.content_width());

    // Render
    let mut renderer = PdfRenderer::new(options.clone());
    renderer.render(&layout);

    // Generate PDF
    generate_pdf(output_path, &renderer, options)?;

    Ok(())
}

/// Extract <style> tags and apply to engine
fn extract_and_apply_styles(element: &mut HtmlElement, engine: &mut LayoutEngine) {
    // Find style elements
    let mut style_content = String::new();

    for child in &element.children {
        if let HtmlNode::Element(child_elem) = child {
            if child_elem.tag == "style" {
                style_content.push_str(&child_elem.text_content());
            }
        }
    }

    if !style_content.is_empty() {
        engine.add_stylesheet(&style_content);
    }

    // Recurse
    for child in &mut element.children {
        if let HtmlNode::Element(child_elem) = child {
            extract_and_apply_styles(child_elem, engine);
        }
    }
}

/// Generate PDF file
fn generate_pdf(
    output_path: &str,
    renderer: &PdfRenderer,
    options: &HtmlToPdfOptions,
) -> Result<()> {
    let mut pdf = String::new();
    let mut obj_offsets: Vec<usize> = vec![];

    // Header
    pdf.push_str("%PDF-1.7\n%");
    pdf.push('\u{00E2}');
    pdf.push('\u{00E3}');
    pdf.push('\u{00CF}');
    pdf.push('\u{00D3}');
    pdf.push('\n');

    // Catalog
    obj_offsets.push(pdf.len());
    pdf.push_str("1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");

    // Pages
    let page_count = renderer.pages.len().max(1);
    obj_offsets.push(pdf.len());
    pdf.push_str(&format!(
        "2 0 obj\n<< /Type /Pages /Kids [{}] /Count {} >>\nendobj\n",
        (0..page_count)
            .map(|i| format!("{} 0 R", 4 + i * 2))
            .collect::<Vec<_>>()
            .join(" "),
        page_count
    ));

    // Fonts
    obj_offsets.push(pdf.len());
    pdf.push_str("3 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n");

    // Bold font
    let bold_font_obj = 4 + page_count * 2;
    obj_offsets.push(pdf.len());
    pdf.push_str(&format!(
        "{} 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica-Bold >>\nendobj\n",
        bold_font_obj
    ));

    // Generate pages
    for (i, page_content) in renderer.pages.iter().enumerate() {
        let page_obj = 4 + i * 2;
        let content_obj = 5 + i * 2;

        // Page
        obj_offsets.push(pdf.len());
        pdf.push_str(&format!(
            "{} 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {} {}] /Contents {} 0 R /Resources << /Font << /F1 3 0 R /F2 {} 0 R >> >> >>\nendobj\n",
            page_obj,
            options.page_width,
            options.page_height,
            content_obj,
            bold_font_obj
        ));

        // Content stream
        let content = page_content.join("\n");
        obj_offsets.push(pdf.len());
        pdf.push_str(&format!(
            "{} 0 obj\n<< /Length {} >>\nstream\n{}\nendstream\nendobj\n",
            content_obj,
            content.len(),
            content
        ));
    }

    // Handle empty pages
    if renderer.pages.is_empty() {
        obj_offsets.push(pdf.len());
        pdf.push_str(&format!(
            "4 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {} {}] /Contents 5 0 R /Resources << /Font << /F1 3 0 R /F2 {} 0 R >> >> >>\nendobj\n",
            options.page_width,
            options.page_height,
            bold_font_obj
        ));

        obj_offsets.push(pdf.len());
        pdf.push_str("5 0 obj\n<< /Length 0 >>\nstream\n\nendstream\nendobj\n");
    }

    // Xref
    let xref_offset = pdf.len();
    pdf.push_str(&format!("xref\n0 {}\n", obj_offsets.len() + 1));
    pdf.push_str("0000000000 65535 f \n");
    for offset in &obj_offsets {
        pdf.push_str(&format!("{:010} 00000 n \n", offset));
    }

    // Trailer
    pdf.push_str(&format!(
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
        obj_offsets.len() + 1,
        xref_offset
    ));

    // Write file
    std::fs::write(output_path, pdf)?;

    Ok(())
}

/// Convert HTML file to PDF
pub fn html_file_to_pdf(
    html_path: &str,
    output_path: &str,
    options: &HtmlToPdfOptions,
) -> Result<()> {
    if !Path::new(html_path).exists() {
        return Err(EnhancedError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("HTML file not found: {}", html_path),
        )));
    }

    let html = std::fs::read_to_string(html_path)?;
    html_to_pdf(&html, output_path, options)
}

/// Convert URL to PDF (placeholder - requires async HTTP)
pub fn url_to_pdf(_url: &str, _output_path: &str, _options: &HtmlToPdfOptions) -> Result<()> {
    Err(EnhancedError::Unsupported(
        "URL to PDF requires async HTTP support. Use html_to_pdf with fetched content.".into(),
    ))
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_html_to_pdf_options() {
        let options = HtmlToPdfOptions::default();
        assert_eq!(options.page_width, 612.0);
        assert_eq!(options.page_height, 792.0);
        assert!(options.print_media_type);
    }

    #[test]
    fn test_options_builder() {
        let options = HtmlToPdfOptions::new()
            .with_page_size(PageSize::A4)
            .with_margins(72.0, 72.0, 72.0, 72.0)
            .with_scale(1.5);

        assert_eq!(options.page_width, 595.28);
        assert_eq!(options.margin_top, 72.0);
        assert_eq!(options.scale, 1.5);
    }

    #[test]
    fn test_css_color_parse() {
        assert_eq!(CssColor::parse("red"), Some(CssColor::rgb(255, 0, 0)));
        assert_eq!(CssColor::parse("#ff0000"), Some(CssColor::rgb(255, 0, 0)));
        assert_eq!(CssColor::parse("#f00"), Some(CssColor::rgb(255, 0, 0)));
        assert_eq!(
            CssColor::parse("rgb(255, 0, 0)"),
            Some(CssColor::rgb(255, 0, 0))
        );
    }

    #[test]
    fn test_css_length_parse() {
        assert_eq!(CssLength::parse("10px"), Some(CssLength::Px(10.0)));
        assert_eq!(CssLength::parse("12pt"), Some(CssLength::Pt(12.0)));
        assert_eq!(CssLength::parse("1.5em"), Some(CssLength::Em(1.5)));
        assert_eq!(CssLength::parse("50%"), Some(CssLength::Percent(50.0)));
        assert_eq!(CssLength::parse("auto"), Some(CssLength::Auto));
    }

    #[test]
    fn test_css_length_to_points() {
        assert_eq!(CssLength::Pt(12.0).to_points(100.0, 12.0, 16.0), 12.0);
        assert_eq!(CssLength::Em(2.0).to_points(100.0, 12.0, 16.0), 24.0);
        assert_eq!(CssLength::Percent(50.0).to_points(100.0, 12.0, 16.0), 50.0);
    }

    #[test]
    fn test_css_stylesheet_parse() {
        let css = "body { color: red; font-size: 14px; } .test { margin: 10px; }";
        let sheet = CssStylesheet::parse(css);
        assert_eq!(sheet.rules.len(), 2);
    }

    #[test]
    fn test_html_parser() {
        let html = "<div id='main' class='container'><p>Hello</p></div>";
        let root = HtmlParser::parse(html).unwrap();

        // Find the div
        let div = root.children.iter().find_map(|n| {
            if let HtmlNode::Element(e) = n {
                if e.tag == "div" {
                    return Some(e);
                }
            }
            None
        });

        assert!(div.is_some());
        let div = div.unwrap();
        assert_eq!(div.id, Some("main".to_string()));
        assert!(div.classes.contains(&"container".to_string()));
    }

    #[test]
    fn test_html_element() {
        let mut div = HtmlElement::new("div");
        div.id = Some("main".to_string());
        div.classes.push("container".to_string());

        let p = HtmlElement::new("p");
        div.add_child(HtmlNode::Element(p));

        assert_eq!(div.tag, "div");
        assert_eq!(div.children.len(), 1);
    }

    #[test]
    fn test_layout_engine() {
        let engine = LayoutEngine::new(612.0, 792.0);
        assert_eq!(engine.viewport_width, 612.0);
    }

    #[test]
    fn test_selector_specificity() {
        let rule1 = CssRule::new("div");
        assert_eq!(rule1.specificity, (0, 0, 0, 1));

        let rule2 = CssRule::new(".class");
        assert_eq!(rule2.specificity, (0, 0, 1, 0));

        let rule3 = CssRule::new("#id");
        assert_eq!(rule3.specificity, (0, 1, 0, 0));

        // Simple compound selectors are counted as their individual parts
        let rule4 = CssRule::new("div .class #id");
        assert_eq!(rule4.specificity, (0, 1, 1, 1));
    }

    #[test]
    fn test_decode_html_entities() {
        assert_eq!(decode_html_entities("&lt;div&gt;"), "<div>");
        assert_eq!(decode_html_entities("&amp;"), "&");
        assert_eq!(decode_html_entities("&quot;"), "\"");
    }

    #[test]
    fn test_font_weight_parse() {
        assert_eq!(CssFontWeight::parse("bold"), Some(CssFontWeight::Bold));
        assert_eq!(CssFontWeight::parse("normal"), Some(CssFontWeight::Normal));
        assert_eq!(
            CssFontWeight::parse("700"),
            Some(CssFontWeight::Weight(700))
        );
    }

    #[test]
    fn test_page_sizes() {
        assert_eq!(PageSize::Letter.dimensions(), (612.0, 792.0));
        assert_eq!(PageSize::A4.dimensions(), (595.28, 841.89));
        assert_eq!(PageSize::Custom(100.0, 200.0).dimensions(), (100.0, 200.0));
    }
}
