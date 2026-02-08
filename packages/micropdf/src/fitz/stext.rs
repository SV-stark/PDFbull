//! Structured Text Extraction
//!
//! Provides layout-aware text extraction from PDF pages.
//! Organizes text into a hierarchy: Page → Block → Line → Char
//!
//! This enables:
//! - Reading order detection
//! - Column layout analysis
//! - Paragraph detection
//! - Word boundaries
//! - Bounding box tracking

use crate::fitz::geometry::{Matrix, Point, Rect};
use crate::fitz::text::{BidiDirection, TextItem, TextLanguage, TextSpan};
use std::fmt;

/// Structured text page - top-level container
#[derive(Debug, Clone)]
pub struct STextPage {
    /// Page bounding box
    pub media_box: Rect,
    /// Text blocks on this page
    pub blocks: Vec<STextBlock>,
}

impl STextPage {
    /// Create a new structured text page
    pub fn new(media_box: Rect) -> Self {
        Self {
            media_box,
            blocks: Vec::new(),
        }
    }

    /// Add a block to the page
    pub fn add_block(&mut self, block: STextBlock) {
        self.blocks.push(block);
    }

    /// Get all text as a single string
    pub fn get_text(&self) -> String {
        let mut result = String::new();
        for block in &self.blocks {
            result.push_str(&block.get_text());
            result.push('\n');
        }
        result
    }

    /// Get text within a specific rectangle
    pub fn get_text_in_rect(&self, rect: &Rect) -> String {
        let mut result = String::new();
        for block in &self.blocks {
            if block.bbox.intersects(rect) {
                result.push_str(&block.get_text_in_rect(rect));
                result.push('\n');
            }
        }
        result
    }

    /// Search for text on the page
    pub fn search(&self, needle: &str, case_sensitive: bool) -> Vec<Rect> {
        let mut results = Vec::new();
        let search_text = if case_sensitive {
            needle.to_string()
        } else {
            needle.to_lowercase()
        };

        for block in &self.blocks {
            let block_text = if case_sensitive {
                block.get_text()
            } else {
                block.get_text().to_lowercase()
            };

            if block_text.contains(&search_text) {
                results.push(block.bbox);
            }
        }

        results
    }

    /// Get the number of characters on the page
    pub fn char_count(&self) -> usize {
        self.blocks.iter().map(|b| b.char_count()).sum()
    }

    /// Get all blocks of a specific type
    pub fn get_blocks_of_type(&self, block_type: STextBlockType) -> Vec<&STextBlock> {
        self.blocks
            .iter()
            .filter(|b| b.block_type == block_type)
            .collect()
    }
}

/// Text block type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum STextBlockType {
    /// Regular text block
    Text,
    /// Image block
    Image,
    /// List item
    List,
    /// Table cell
    Table,
}

/// Structured text block - a contiguous region of text
#[derive(Debug, Clone)]
pub struct STextBlock {
    /// Block type
    pub block_type: STextBlockType,
    /// Bounding box
    pub bbox: Rect,
    /// Text lines in this block
    pub lines: Vec<STextLine>,
}

impl STextBlock {
    /// Create a new text block
    pub fn new(block_type: STextBlockType, bbox: Rect) -> Self {
        Self {
            block_type,
            bbox,
            lines: Vec::new(),
        }
    }

    /// Add a line to the block
    pub fn add_line(&mut self, line: STextLine) {
        // Update bbox to include this line
        self.bbox = self.bbox.union(&line.bbox);
        self.lines.push(line);
    }

    /// Get all text in the block
    pub fn get_text(&self) -> String {
        let mut result = String::new();
        for line in &self.lines {
            result.push_str(&line.get_text());
            result.push('\n');
        }
        result
    }

    /// Get text within a specific rectangle
    pub fn get_text_in_rect(&self, rect: &Rect) -> String {
        let mut result = String::new();
        for line in &self.lines {
            if line.bbox.intersects(rect) {
                result.push_str(&line.get_text_in_rect(rect));
                result.push('\n');
            }
        }
        result
    }

    /// Get the number of lines
    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    /// Get the number of characters
    pub fn char_count(&self) -> usize {
        self.lines.iter().map(|l| l.char_count()).sum()
    }
}

/// Writing mode for text lines
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WritingMode {
    /// Horizontal left-to-right
    HorizontalLtr,
    /// Horizontal right-to-left
    HorizontalRtl,
    /// Vertical top-to-bottom
    VerticalTtb,
    /// Vertical bottom-to-top
    VerticalBtt,
}

impl WritingMode {
    pub fn is_horizontal(&self) -> bool {
        matches!(
            self,
            WritingMode::HorizontalLtr | WritingMode::HorizontalRtl
        )
    }

    pub fn is_vertical(&self) -> bool {
        !self.is_horizontal()
    }

    pub fn is_rtl(&self) -> bool {
        matches!(self, WritingMode::HorizontalRtl)
    }
}

/// Structured text line - a single line of text
#[derive(Debug, Clone)]
pub struct STextLine {
    /// Writing mode
    pub wmode: WritingMode,
    /// Bounding box
    pub bbox: Rect,
    /// Baseline Y coordinate (for horizontal) or X (for vertical)
    pub baseline: f32,
    /// Text direction
    pub dir: Point,
    /// Characters in this line
    pub chars: Vec<STextChar>,
}

impl STextLine {
    /// Create a new text line
    pub fn new(wmode: WritingMode, baseline: f32) -> Self {
        Self {
            wmode,
            bbox: Rect::EMPTY,
            baseline,
            dir: Point::new(1.0, 0.0), // Default: left-to-right
            chars: Vec::new(),
        }
    }

    /// Add a character to the line
    pub fn add_char(&mut self, ch: STextChar) {
        // Update bbox to include this char
        if self.chars.is_empty() {
            self.bbox = ch.quad.to_rect();
        } else {
            self.bbox = self.bbox.union(&ch.quad.to_rect());
        }
        self.chars.push(ch);
    }

    /// Get all text in the line
    pub fn get_text(&self) -> String {
        self.chars.iter().map(|c| c.c).collect()
    }

    /// Get text within a specific rectangle
    pub fn get_text_in_rect(&self, rect: &Rect) -> String {
        self.chars
            .iter()
            .filter(|c| c.quad.to_rect().intersects(rect))
            .map(|c| c.c)
            .collect()
    }

    /// Get the number of characters
    pub fn char_count(&self) -> usize {
        self.chars.len()
    }

    /// Split line into words
    pub fn get_words(&self) -> Vec<String> {
        let text = self.get_text();
        text.split_whitespace().map(|s| s.to_string()).collect()
    }

    /// Get line height
    pub fn height(&self) -> f32 {
        self.bbox.height()
    }

    /// Get line width
    pub fn width(&self) -> f32 {
        self.bbox.width()
    }
}

/// Quadrilateral for character bounding box
#[derive(Debug, Clone, Copy)]
pub struct Quad {
    /// Lower-left corner
    pub ll: Point,
    /// Lower-right corner
    pub lr: Point,
    /// Upper-left corner
    pub ul: Point,
    /// Upper-right corner
    pub ur: Point,
}

impl Quad {
    /// Create a new quad
    pub fn new(ll: Point, lr: Point, ul: Point, ur: Point) -> Self {
        Self { ll, lr, ul, ur }
    }

    /// Create a quad from a rectangle
    pub fn from_rect(rect: &Rect) -> Self {
        Self {
            ll: Point::new(rect.x0, rect.y0),
            lr: Point::new(rect.x1, rect.y0),
            ul: Point::new(rect.x0, rect.y1),
            ur: Point::new(rect.x1, rect.y1),
        }
    }

    /// Convert quad to axis-aligned bounding box
    pub fn to_rect(&self) -> Rect {
        let min_x = self.ll.x.min(self.lr.x).min(self.ul.x).min(self.ur.x);
        let min_y = self.ll.y.min(self.lr.y).min(self.ul.y).min(self.ur.y);
        let max_x = self.ll.x.max(self.lr.x).max(self.ul.x).max(self.ur.x);
        let max_y = self.ll.y.max(self.lr.y).max(self.ul.y).max(self.ur.y);
        Rect::new(min_x, min_y, max_x, max_y)
    }

    /// Check if quad contains a point
    pub fn contains_point(&self, p: Point) -> bool {
        self.to_rect().contains(p.x, p.y)
    }

    /// Transform quad by matrix
    pub fn transform(&self, ctm: &Matrix) -> Self {
        Self {
            ll: ctm.transform_point(self.ll),
            lr: ctm.transform_point(self.lr),
            ul: ctm.transform_point(self.ul),
            ur: ctm.transform_point(self.ur),
        }
    }
}

/// Structured text character - a single character with position
#[derive(Debug, Clone)]
pub struct STextChar {
    /// Unicode character
    pub c: char,
    /// Character quad (4 corners for rotated text)
    pub quad: Quad,
    /// Font size
    pub size: f32,
    /// Font name
    pub font_name: String,
    /// Glyph ID in font
    pub gid: u16,
    /// Text color (RGB)
    pub color: [u8; 3],
    /// Origin point
    pub origin: Point,
}

impl STextChar {
    /// Create a new structured text character
    pub fn new(c: char, quad: Quad, size: f32, font_name: String) -> Self {
        Self {
            c,
            quad,
            size,
            font_name,
            gid: 0,
            color: [0, 0, 0], // Black
            origin: quad.ll,
        }
    }

    /// Create with full parameters
    pub fn with_details(
        c: char,
        quad: Quad,
        size: f32,
        font_name: String,
        gid: u16,
        color: [u8; 3],
        origin: Point,
    ) -> Self {
        Self {
            c,
            quad,
            size,
            font_name,
            gid,
            color,
            origin,
        }
    }

    /// Check if character is whitespace
    pub fn is_whitespace(&self) -> bool {
        self.c.is_whitespace()
    }

    /// Get character bounding box
    pub fn bbox(&self) -> Rect {
        self.quad.to_rect()
    }
}

/// Structured text options
#[derive(Debug, Clone, Copy)]
pub struct STextOptions {
    /// Preserve ligatures (e.g., "fi" as single char)
    pub preserve_ligatures: bool,
    /// Preserve whitespace exactly as in PDF
    pub preserve_whitespace: bool,
    /// Preserve images as special blocks
    pub preserve_images: bool,
    /// Use heuristics for paragraph detection
    pub detect_paragraphs: bool,
    /// Dehyphenate words split across lines
    pub dehyphenate: bool,
}

impl Default for STextOptions {
    fn default() -> Self {
        Self {
            preserve_ligatures: true,
            preserve_whitespace: false,
            preserve_images: false,
            detect_paragraphs: true,
            dehyphenate: true,
        }
    }
}

/// Structured text builder - converts text spans to structured layout
pub struct STextBuilder {
    /// Current page being built
    page: STextPage,
    /// Current block being built
    current_block: Option<STextBlock>,
    /// Current line being built
    current_line: Option<STextLine>,
    /// Options
    options: STextOptions,
}

impl STextBuilder {
    /// Create a new structured text builder
    pub fn new(media_box: Rect, options: STextOptions) -> Self {
        Self {
            page: STextPage::new(media_box),
            current_block: None,
            current_line: None,
            options,
        }
    }

    /// Create with default options
    pub fn with_defaults(media_box: Rect) -> Self {
        Self::new(media_box, STextOptions::default())
    }

    /// Add a text span to the structured layout
    pub fn add_span(&mut self, span: &TextSpan) {
        // Determine writing mode
        let wmode = if span.wmode {
            WritingMode::VerticalTtb
        } else if span.markup_dir == BidiDirection::Rtl {
            WritingMode::HorizontalRtl
        } else {
            WritingMode::HorizontalLtr
        };

        // Process each glyph in the span
        for item in span.items() {
            self.add_text_item(item, span, wmode);
        }
    }

    /// Add a single text item
    fn add_text_item(&mut self, item: &TextItem, span: &TextSpan, wmode: WritingMode) {
        // Convert glyph to character
        let c = if item.ucs >= 0 {
            char::from_u32(item.ucs as u32).unwrap_or('?')
        } else {
            '?'
        };

        // Calculate character quad
        let size = (span.trm.a.abs() + span.trm.b.abs()).max(span.trm.c.abs() + span.trm.d.abs());
        let origin = Point::new(item.x, item.y);
        let advance = item.advance;

        // Simplified quad calculation
        let quad = Quad::from_rect(&Rect::new(
            origin.x,
            origin.y - size,
            origin.x + advance,
            origin.y,
        ));

        let ch = STextChar::new(c, quad, size, span.font.name().to_string());

        // Add to current line or create new line
        if let Some(ref mut line) = self.current_line {
            // Check if we need a new line (vertical spacing)
            let baseline_diff = (item.y - line.baseline).abs();
            if baseline_diff > size * 0.3 {
                // New line needed
                self.finish_line();
                self.start_line(wmode, item.y);
            }
        } else {
            // Start first line
            self.start_line(wmode, item.y);
        }

        if let Some(ref mut line) = self.current_line {
            line.add_char(ch);
        }
    }

    /// Start a new line
    fn start_line(&mut self, wmode: WritingMode, baseline: f32) {
        let line = STextLine::new(wmode, baseline);
        self.current_line = Some(line);
    }

    /// Finish the current line
    fn finish_line(&mut self) {
        if let Some(line) = self.current_line.take() {
            // Add to current block or create new block
            if let Some(ref mut block) = self.current_block {
                // Check if line belongs to this block (vertical spacing)
                let last_line_bbox = block.lines.last().map(|l| l.bbox).unwrap_or(Rect::EMPTY);
                let spacing = (line.bbox.y0 - last_line_bbox.y1).abs();

                if spacing < line.height() * 1.5 {
                    block.add_line(line);
                } else {
                    // New block needed
                    self.finish_block();
                    self.start_block();
                    if let Some(ref mut block) = self.current_block {
                        block.add_line(line);
                    }
                }
            } else {
                // Start first block
                self.start_block();
                if let Some(ref mut block) = self.current_block {
                    block.add_line(line);
                }
            }
        }
    }

    /// Start a new block
    fn start_block(&mut self) {
        let block = STextBlock::new(STextBlockType::Text, Rect::EMPTY);
        self.current_block = Some(block);
    }

    /// Finish the current block
    fn finish_block(&mut self) {
        if let Some(block) = self.current_block.take() {
            if !block.lines.is_empty() {
                self.page.add_block(block);
            }
        }
    }

    /// Finish building and return the structured text page
    pub fn finish(mut self) -> STextPage {
        self.finish_line();
        self.finish_block();
        self.page
    }
}

impl fmt::Display for STextPage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.get_text())
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stext_page_creation() {
        let page = STextPage::new(Rect::new(0.0, 0.0, 612.0, 792.0));
        assert_eq!(page.blocks.len(), 0);
        assert_eq!(page.char_count(), 0);
    }

    #[test]
    fn test_stext_block_creation() {
        let block = STextBlock::new(STextBlockType::Text, Rect::new(0.0, 0.0, 100.0, 50.0));
        assert_eq!(block.line_count(), 0);
        assert_eq!(block.char_count(), 0);
    }

    #[test]
    fn test_stext_line_creation() {
        let line = STextLine::new(WritingMode::HorizontalLtr, 100.0);
        assert_eq!(line.baseline, 100.0);
        assert_eq!(line.char_count(), 0);
    }

    #[test]
    fn test_stext_char_creation() {
        let quad = Quad::from_rect(&Rect::new(0.0, 0.0, 10.0, 12.0));
        let ch = STextChar::new('A', quad, 12.0, "Times".to_string());

        assert_eq!(ch.c, 'A');
        assert_eq!(ch.size, 12.0);
        assert!(!ch.is_whitespace());
    }

    #[test]
    fn test_quad_to_rect() {
        let quad = Quad::from_rect(&Rect::new(10.0, 20.0, 30.0, 40.0));
        let rect = quad.to_rect();

        assert_eq!(rect.x0, 10.0);
        assert_eq!(rect.y0, 20.0);
        assert_eq!(rect.x1, 30.0);
        assert_eq!(rect.y1, 40.0);
    }

    #[test]
    fn test_writing_mode() {
        assert!(WritingMode::HorizontalLtr.is_horizontal());
        assert!(!WritingMode::VerticalTtb.is_horizontal());
        assert!(WritingMode::VerticalTtb.is_vertical());
        assert!(WritingMode::HorizontalRtl.is_rtl());
    }

    #[test]
    fn test_stext_line_add_char() {
        let mut line = STextLine::new(WritingMode::HorizontalLtr, 100.0);
        let quad = Quad::from_rect(&Rect::new(0.0, 90.0, 10.0, 100.0));
        let ch = STextChar::new('H', quad, 10.0, "Arial".to_string());

        line.add_char(ch);
        assert_eq!(line.char_count(), 1);
        assert_eq!(line.get_text(), "H");
    }

    #[test]
    fn test_stext_line_get_words() {
        let mut line = STextLine::new(WritingMode::HorizontalLtr, 100.0);

        // Add "Hello World"
        for (i, c) in "Hello World".chars().enumerate() {
            let x = i as f32 * 10.0;
            let quad = Quad::from_rect(&Rect::new(x, 90.0, x + 10.0, 100.0));
            let ch = STextChar::new(c, quad, 10.0, "Arial".to_string());
            line.add_char(ch);
        }

        let words = line.get_words();
        assert_eq!(words.len(), 2);
        assert_eq!(words[0], "Hello");
        assert_eq!(words[1], "World");
    }
}
