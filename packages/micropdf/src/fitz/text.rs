//! Text handling - glyph positioning and text spans
//!
//! Provides structured text representation with font, position, and layout information.

use crate::fitz::font::Font;
use crate::fitz::geometry::{Matrix, Rect};
use crate::fitz::path::StrokeState;
use std::sync::Arc;

/// Language codes for text (ISO 639)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum TextLanguage {
    Unset = 0,
    Ur = 572,       // FZ_LANG_TAG2('u','r')
    Urd = 15444,    // FZ_LANG_TAG3('u','r','d')
    Ko = 299,       // FZ_LANG_TAG2('k','o')
    Ja = 271,       // FZ_LANG_TAG2('j','a')
    Zh = 703,       // FZ_LANG_TAG2('z','h')
    ZhHans = 18982, // FZ_LANG_TAG3('z','h','s')
    ZhHant = 19009, // FZ_LANG_TAG3('z','h','t')
}

impl TextLanguage {
    /// Parse language from ISO 639 string
    pub fn from_string(s: &str) -> Self {
        match s {
            "ur" => Self::Ur,
            "urd" => Self::Urd,
            "ko" => Self::Ko,
            "ja" => Self::Ja,
            "zh" => Self::Zh,
            "zh-Hans" | "zhs" => Self::ZhHans,
            "zh-Hant" | "zht" => Self::ZhHant,
            _ => Self::Unset,
        }
    }

    /// Convert to ISO 639 string
    pub fn to_string(&self) -> &'static str {
        match self {
            Self::Unset => "",
            Self::Ur => "ur",
            Self::Urd => "urd",
            Self::Ko => "ko",
            Self::Ja => "ja",
            Self::Zh => "zh",
            Self::ZhHans => "zh-Hans",
            Self::ZhHant => "zh-Hant",
        }
    }
}

/// Bidirectional text direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BidiDirection {
    /// Left-to-right
    Ltr = 0,
    /// Right-to-left
    Rtl = 1,
    /// Neutral (inherits from context)
    Neutral = 2,
}

/// Single text item (glyph) with position and metrics
#[derive(Debug, Clone)]
pub struct TextItem {
    /// X position
    pub x: f32,
    /// Y position
    pub y: f32,
    /// Advance width
    pub advance: f32,
    /// Glyph ID (-1 for one gid to many ucs mappings)
    pub gid: i32,
    /// Unicode codepoint (-1 for one ucs to many gid mappings)
    pub ucs: i32,
    /// CID for CJK fonts, raw character code for other fonts
    pub cid: i32,
}

impl TextItem {
    /// Create a new text item
    pub fn new(x: f32, y: f32, gid: i32, ucs: i32) -> Self {
        Self {
            x,
            y,
            advance: 0.0,
            gid,
            ucs,
            cid: ucs,
        }
    }

    /// Create with full parameters
    pub fn with_advance(x: f32, y: f32, advance: f32, gid: i32, ucs: i32, cid: i32) -> Self {
        Self {
            x,
            y,
            advance,
            gid,
            ucs,
            cid,
        }
    }
}

/// Text span - sequence of glyphs with same font and formatting
#[derive(Clone)]
pub struct TextSpan {
    /// Font used for this span
    pub font: Arc<Font>,
    /// Text rendering matrix (a, b, c, d components)
    pub trm: Matrix,
    /// Writing mode (0=horizontal, 1=vertical)
    pub wmode: bool,
    /// Bidirectional level (0-127)
    pub bidi_level: u8,
    /// Markup direction
    pub markup_dir: BidiDirection,
    /// Language
    pub language: TextLanguage,
    /// Glyphs in this span
    items: Vec<TextItem>,
}

impl TextSpan {
    /// Create a new text span
    pub fn new(font: Arc<Font>, trm: Matrix) -> Self {
        Self {
            font,
            trm,
            wmode: false,
            bidi_level: 0,
            markup_dir: BidiDirection::Ltr,
            language: TextLanguage::Unset,
            items: Vec::new(),
        }
    }

    /// Create with capacity
    pub fn with_capacity(font: Arc<Font>, trm: Matrix, capacity: usize) -> Self {
        Self {
            font,
            trm,
            wmode: false,
            bidi_level: 0,
            markup_dir: BidiDirection::Ltr,
            language: TextLanguage::Unset,
            items: Vec::with_capacity(capacity),
        }
    }

    /// Add a glyph to this span
    pub fn add_glyph(&mut self, item: TextItem) {
        self.items.push(item);
    }

    /// Get items
    pub fn items(&self) -> &[TextItem] {
        &self.items
    }

    /// Get mutable items
    pub fn items_mut(&mut self) -> &mut Vec<TextItem> {
        &mut self.items
    }

    /// Get number of items
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Get bounding box of this span
    pub fn bounds(&self, stroke: Option<&StrokeState>) -> Rect {
        if self.items.is_empty() {
            return Rect::EMPTY;
        }

        // Get font metrics
        let font_size = self.trm.a.hypot(self.trm.b);
        let ascender = font_size * 0.8; // Approximate
        let descender = font_size * 0.2; // Approximate

        let mut bbox = Rect::EMPTY;
        for item in &self.items {
            let mut item_bbox = Rect::new(
                item.x,
                item.y - descender,
                item.x + item.advance,
                item.y + ascender,
            );

            // Expand for stroke
            if let Some(stroke) = stroke {
                let expand = stroke.linewidth / 2.0;
                item_bbox = item_bbox.expand(expand);
            }

            bbox = bbox.union(&item_bbox);
        }

        bbox.transform(&self.trm)
    }

    /// Extract text content as string
    pub fn text_content(&self) -> String {
        self.items
            .iter()
            .filter(|item| item.ucs >= 0)
            .filter_map(|item| char::from_u32(item.ucs as u32))
            .collect()
    }
}

impl std::fmt::Debug for TextSpan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TextSpan")
            .field("font", &"<Font>")
            .field("trm", &self.trm)
            .field("wmode", &self.wmode)
            .field("bidi_level", &self.bidi_level)
            .field("items", &self.items.len())
            .finish()
    }
}

/// Text object - collection of text spans
#[derive(Clone)]
pub struct Text {
    /// Text spans
    spans: Vec<TextSpan>,
}

impl Text {
    /// Create a new empty text object
    pub fn new() -> Self {
        Self { spans: Vec::new() }
    }

    /// Create with capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            spans: Vec::with_capacity(capacity),
        }
    }

    /// Add a text span
    pub fn add_span(&mut self, span: TextSpan) {
        self.spans.push(span);
    }

    /// Show a single glyph
    #[allow(clippy::too_many_arguments)]
    pub fn show_glyph(
        &mut self,
        font: Arc<Font>,
        trm: Matrix,
        gid: i32,
        unicode: i32,
        wmode: bool,
        bidi_level: u8,
        markup_dir: BidiDirection,
        language: TextLanguage,
    ) {
        // Try to append to existing span if compatible
        if let Some(last_span) = self.spans.last_mut() {
            if Arc::ptr_eq(&last_span.font, &font)
                && last_span.trm == trm
                && last_span.wmode == wmode
                && last_span.bidi_level == bidi_level
                && last_span.markup_dir == markup_dir
                && last_span.language == language
            {
                // Append to existing span
                let x = trm.e;
                let y = trm.f;
                last_span.add_glyph(TextItem::new(x, y, gid, unicode));
                return;
            }
        }

        // Create new span
        let mut span = TextSpan::new(font, trm);
        span.wmode = wmode;
        span.bidi_level = bidi_level;
        span.markup_dir = markup_dir;
        span.language = language;

        let x = trm.e;
        let y = trm.f;
        span.add_glyph(TextItem::new(x, y, gid, unicode));
        self.spans.push(span);
    }

    /// Show a single glyph with full parameters
    #[allow(clippy::too_many_arguments)]
    pub fn show_glyph_with_advance(
        &mut self,
        font: Arc<Font>,
        trm: Matrix,
        advance: f32,
        gid: i32,
        unicode: i32,
        cid: i32,
        wmode: bool,
        bidi_level: u8,
        markup_dir: BidiDirection,
        language: TextLanguage,
    ) {
        // Try to append to existing span if compatible
        if let Some(last_span) = self.spans.last_mut() {
            if Arc::ptr_eq(&last_span.font, &font)
                && last_span.trm == trm
                && last_span.wmode == wmode
                && last_span.bidi_level == bidi_level
                && last_span.markup_dir == markup_dir
                && last_span.language == language
            {
                let x = trm.e;
                let y = trm.f;
                last_span.add_glyph(TextItem::with_advance(x, y, advance, gid, unicode, cid));
                return;
            }
        }

        // Create new span
        let mut span = TextSpan::new(font, trm);
        span.wmode = wmode;
        span.bidi_level = bidi_level;
        span.markup_dir = markup_dir;
        span.language = language;

        let x = trm.e;
        let y = trm.f;
        span.add_glyph(TextItem::with_advance(x, y, advance, gid, unicode, cid));
        self.spans.push(span);
    }

    /// Show a UTF-8 string
    #[allow(clippy::too_many_arguments)]
    pub fn show_string(
        &mut self,
        font: Arc<Font>,
        mut trm: Matrix,
        s: &str,
        wmode: bool,
        bidi_level: u8,
        markup_dir: BidiDirection,
        language: TextLanguage,
    ) -> Matrix {
        for ch in s.chars() {
            // Simple implementation: each char is a glyph
            // In a real implementation, we'd do font shaping
            let gid = ch as i32;
            let advance = 10.0; // Simplified: should come from font metrics

            self.show_glyph(
                Arc::clone(&font),
                trm,
                gid,
                ch as i32,
                wmode,
                bidi_level,
                markup_dir,
                language,
            );

            // Advance position
            if wmode {
                trm.f += advance; // Vertical
            } else {
                trm.e += advance; // Horizontal
            }
        }

        trm
    }

    /// Measure a UTF-8 string without adding it
    pub fn measure_string(_font: &Font, trm: Matrix, s: &str, wmode: bool) -> Matrix {
        let mut result_trm = trm;

        for _ch in s.chars() {
            let advance = 10.0; // Simplified: should use font metrics

            if wmode {
                result_trm.f += advance;
            } else {
                result_trm.e += advance;
            }
        }

        result_trm
    }

    /// Get spans
    pub fn spans(&self) -> &[TextSpan] {
        &self.spans
    }

    /// Get mutable spans
    pub fn spans_mut(&mut self) -> &mut Vec<TextSpan> {
        &mut self.spans
    }

    /// Get number of spans
    pub fn len(&self) -> usize {
        self.spans.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.spans.is_empty()
    }

    /// Get bounding box of all text
    pub fn bounds(&self, stroke: Option<&StrokeState>, ctm: &Matrix) -> Rect {
        if self.spans.is_empty() {
            return Rect::EMPTY;
        }

        let mut bbox = Rect::EMPTY;
        for span in &self.spans {
            let span_bbox = span.bounds(stroke).transform(ctm);
            bbox = bbox.union(&span_bbox);
        }

        bbox
    }

    /// Extract all text content as a single string
    pub fn text_content(&self) -> String {
        self.spans
            .iter()
            .map(|span| span.text_content())
            .collect::<Vec<_>>()
            .join("")
    }

    /// Clear all spans
    pub fn clear(&mut self) {
        self.spans.clear();
    }

    /// Get the number of spans (alias for len())
    pub fn span_count(&self) -> usize {
        self.spans.len()
    }

    /// Get the total number of items across all spans
    pub fn item_count(&self) -> usize {
        self.spans.iter().map(|span| span.len()).sum()
    }

    /// Set language for all spans
    pub fn set_language(&mut self, language: TextLanguage) {
        for span in &mut self.spans {
            span.language = language;
        }
    }
}

impl Default for Text {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for Text {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Text")
            .field("spans", &self.spans.len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_language_from_string() {
        assert_eq!(TextLanguage::from_string("ja"), TextLanguage::Ja);
        assert_eq!(TextLanguage::from_string("ko"), TextLanguage::Ko);
        assert_eq!(TextLanguage::from_string("zh"), TextLanguage::Zh);
        assert_eq!(TextLanguage::from_string("zh-Hans"), TextLanguage::ZhHans);
        assert_eq!(TextLanguage::from_string("unknown"), TextLanguage::Unset);
    }

    #[test]
    fn test_text_language_to_string() {
        assert_eq!(TextLanguage::Ja.to_string(), "ja");
        assert_eq!(TextLanguage::Ko.to_string(), "ko");
        assert_eq!(TextLanguage::Zh.to_string(), "zh");
        assert_eq!(TextLanguage::ZhHans.to_string(), "zh-Hans");
        assert_eq!(TextLanguage::Unset.to_string(), "");
    }

    #[test]
    fn test_text_item_new() {
        let item = TextItem::new(10.0, 20.0, 42, 65); // 'A'
        assert_eq!(item.x, 10.0);
        assert_eq!(item.y, 20.0);
        assert_eq!(item.gid, 42);
        assert_eq!(item.ucs, 65);
        assert_eq!(item.advance, 0.0);
    }

    #[test]
    fn test_text_item_with_advance() {
        let item = TextItem::with_advance(10.0, 20.0, 15.0, 42, 65, 65);
        assert_eq!(item.advance, 15.0);
        assert_eq!(item.cid, 65);
    }

    #[test]
    fn test_text_span_new() {
        let font = Arc::new(Font::new("TestFont"));
        let trm = Matrix::IDENTITY;
        let span = TextSpan::new(font, trm);

        assert!(span.is_empty());
        assert_eq!(span.len(), 0);
        assert!(!span.wmode);
    }

    #[test]
    fn test_text_span_add_glyph() {
        let font = Arc::new(Font::new("TestFont"));
        let trm = Matrix::IDENTITY;
        let mut span = TextSpan::new(font, trm);

        span.add_glyph(TextItem::new(0.0, 0.0, 1, 65)); // 'A'
        span.add_glyph(TextItem::new(10.0, 0.0, 2, 66)); // 'B'

        assert_eq!(span.len(), 2);
        assert!(!span.is_empty());
    }

    #[test]
    fn test_text_span_to_string() {
        let font = Arc::new(Font::new("TestFont"));
        let trm = Matrix::IDENTITY;
        let mut span = TextSpan::new(font, trm);

        span.add_glyph(TextItem::new(0.0, 0.0, 1, 72)); // 'H'
        span.add_glyph(TextItem::new(10.0, 0.0, 2, 105)); // 'i'

        assert_eq!(span.text_content(), "Hi");
    }

    #[test]
    fn test_text_new() {
        let text = Text::new();
        assert!(text.is_empty());
        assert_eq!(text.len(), 0);
    }

    #[test]
    fn test_text_show_glyph() {
        let mut text = Text::new();
        let font = Arc::new(Font::new("TestFont"));
        let trm = Matrix::IDENTITY;

        text.show_glyph(
            font,
            trm,
            1,
            65, // 'A'
            false,
            0,
            BidiDirection::Ltr,
            TextLanguage::Unset,
        );

        assert_eq!(text.len(), 1);
        assert_eq!(text.spans()[0].len(), 1);
    }

    #[test]
    fn test_text_show_string() {
        let mut text = Text::new();
        let font = Arc::new(Font::new("TestFont"));
        let trm = Matrix::IDENTITY;

        let result_trm = text.show_string(
            font,
            trm,
            "Hello",
            false,
            0,
            BidiDirection::Ltr,
            TextLanguage::Unset,
        );

        // Should have advanced horizontally
        assert!(result_trm.e > trm.e);
        assert!(!text.is_empty());
    }

    #[test]
    fn test_text_to_string() {
        let mut text = Text::new();
        let font = Arc::new(Font::new("TestFont"));
        let trm = Matrix::IDENTITY;

        text.show_string(
            Arc::clone(&font),
            trm,
            "Hello",
            false,
            0,
            BidiDirection::Ltr,
            TextLanguage::Unset,
        );

        let content = text.text_content();
        assert_eq!(content, "Hello");
    }

    #[test]
    fn test_text_measure_string() {
        let font = Font::new("TestFont");
        let trm = Matrix::IDENTITY;

        let result_trm = Text::measure_string(&font, trm, "Test", false);

        // Should have advanced
        assert!(result_trm.e > trm.e);
        assert_eq!(result_trm.f, trm.f); // No vertical movement
    }

    #[test]
    fn test_text_clear() {
        let mut text = Text::new();
        let font = Arc::new(Font::new("TestFont"));
        let trm = Matrix::IDENTITY;

        text.show_string(
            font,
            trm,
            "Test",
            false,
            0,
            BidiDirection::Ltr,
            TextLanguage::Unset,
        );

        assert!(!text.is_empty());
        text.clear();
        assert!(text.is_empty());
    }

    #[test]
    fn test_text_span_coalescing() {
        let mut text = Text::new();
        let font = Arc::new(Font::new("TestFont"));
        let trm = Matrix::IDENTITY;

        // Add two glyphs with same formatting - should coalesce into one span
        text.show_glyph(
            Arc::clone(&font),
            trm,
            1,
            65,
            false,
            0,
            BidiDirection::Ltr,
            TextLanguage::Unset,
        );

        text.show_glyph(
            Arc::clone(&font),
            trm,
            2,
            66,
            false,
            0,
            BidiDirection::Ltr,
            TextLanguage::Unset,
        );

        // Should have 1 span with 2 glyphs
        assert_eq!(text.len(), 1);
        assert_eq!(text.spans()[0].len(), 2);
    }

    #[test]
    fn test_bidi_direction() {
        assert_eq!(BidiDirection::Ltr as u8, 0);
        assert_eq!(BidiDirection::Rtl as u8, 1);
        assert_eq!(BidiDirection::Neutral as u8, 2);
    }
}
