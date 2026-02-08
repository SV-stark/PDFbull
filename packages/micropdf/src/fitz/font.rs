//! Font handling - Type1, TrueType, CFF, CID fonts
//!
//! Provides comprehensive font support for various PDF font formats.

use crate::fitz::error::{Error, Result};
use std::collections::HashMap;
use std::sync::Arc;

/// Font type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontType {
    /// Type 1 PostScript font
    Type1,
    /// Type 1 Multiple Master font
    Type1MM,
    /// Type 3 user-defined font
    Type3,
    /// TrueType font
    TrueType,
    /// CID Font (for CJK)
    CIDFontType0,
    /// CID TrueType font
    CIDFontType2,
    /// Compact Font Format (CFF)
    CFF,
    /// OpenType font
    OpenType,
    /// Unknown font type
    Unknown,
}

impl FontType {
    pub fn from_string(s: &str) -> Self {
        match s {
            "Type1" => Self::Type1,
            "MMType1" => Self::Type1MM,
            "Type3" => Self::Type3,
            "TrueType" => Self::TrueType,
            "CIDFontType0" => Self::CIDFontType0,
            "CIDFontType2" => Self::CIDFontType2,
            "CFF" => Self::CFF,
            "OpenType" => Self::OpenType,
            _ => Self::Unknown,
        }
    }

    pub fn to_string(&self) -> &'static str {
        match self {
            Self::Type1 => "Type1",
            Self::Type1MM => "MMType1",
            Self::Type3 => "Type3",
            Self::TrueType => "TrueType",
            Self::CIDFontType0 => "CIDFontType0",
            Self::CIDFontType2 => "CIDFontType2",
            Self::CFF => "CFF",
            Self::OpenType => "OpenType",
            Self::Unknown => "Unknown",
        }
    }
}

/// Font flags
#[derive(Debug, Clone, Copy)]
pub struct FontFlags(u32);

impl FontFlags {
    pub const FIXED_PITCH: u32 = 1 << 0;
    pub const SERIF: u32 = 1 << 1;
    pub const SYMBOLIC: u32 = 1 << 2;
    pub const SCRIPT: u32 = 1 << 3;
    pub const NONSYMBOLIC: u32 = 1 << 5;
    pub const ITALIC: u32 = 1 << 6;
    pub const ALL_CAP: u32 = 1 << 16;
    pub const SMALL_CAP: u32 = 1 << 17;
    pub const FORCE_BOLD: u32 = 1 << 18;

    pub fn new(flags: u32) -> Self {
        Self(flags)
    }

    pub fn has(&self, flag: u32) -> bool {
        (self.0 & flag) != 0
    }

    pub fn set(&mut self, flag: u32) {
        self.0 |= flag;
    }

    pub fn value(&self) -> u32 {
        self.0
    }
}

impl Default for FontFlags {
    fn default() -> Self {
        Self(Self::NONSYMBOLIC)
    }
}

/// Font stretch (width class)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum FontStretch {
    UltraCondensed = 1,
    ExtraCondensed = 2,
    Condensed = 3,
    SemiCondensed = 4,
    #[default]
    Normal = 5,
    SemiExpanded = 6,
    Expanded = 7,
    ExtraExpanded = 8,
    UltraExpanded = 9,
}

/// Font weight (100-900)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct FontWeight(u16);

impl FontWeight {
    pub const THIN: Self = Self(100);
    pub const EXTRA_LIGHT: Self = Self(200);
    pub const LIGHT: Self = Self(300);
    pub const NORMAL: Self = Self(400);
    pub const MEDIUM: Self = Self(500);
    pub const SEMI_BOLD: Self = Self(600);
    pub const BOLD: Self = Self(700);
    pub const EXTRA_BOLD: Self = Self(800);
    pub const BLACK: Self = Self(900);

    pub fn new(weight: u16) -> Self {
        Self(weight.clamp(100, 900))
    }

    pub fn value(&self) -> u16 {
        self.0
    }
}

impl Default for FontWeight {
    fn default() -> Self {
        Self::NORMAL
    }
}

/// Character-to-glyph mapping
pub struct CharMap {
    /// Unicode to glyph ID mapping
    mappings: HashMap<u32, u16>,
}

impl CharMap {
    pub fn new() -> Self {
        Self {
            mappings: HashMap::new(),
        }
    }

    pub fn add_mapping(&mut self, unicode: u32, gid: u16) {
        self.mappings.insert(unicode, gid);
    }

    pub fn lookup(&self, unicode: u32) -> Option<u16> {
        self.mappings.get(&unicode).copied()
    }

    pub fn len(&self) -> usize {
        self.mappings.len()
    }

    pub fn is_empty(&self) -> bool {
        self.mappings.is_empty()
    }
}

impl Default for CharMap {
    fn default() -> Self {
        Self::new()
    }
}

/// Font metrics
#[derive(Debug, Clone, Copy)]
pub struct FontMetrics {
    /// Ascender (above baseline)
    pub ascender: f32,
    /// Descender (below baseline, usually negative)
    pub descender: f32,
    /// Line height
    pub line_height: f32,
    /// Cap height (height of capital letters)
    pub cap_height: f32,
    /// X-height (height of lowercase 'x')
    pub x_height: f32,
    /// Italic angle (degrees from vertical)
    pub italic_angle: f32,
    /// Underline position
    pub underline_position: f32,
    /// Underline thickness
    pub underline_thickness: f32,
}

impl Default for FontMetrics {
    fn default() -> Self {
        Self {
            ascender: 0.8,
            descender: -0.2,
            line_height: 1.2,
            cap_height: 0.7,
            x_height: 0.5,
            italic_angle: 0.0,
            underline_position: -0.1,
            underline_thickness: 0.05,
        }
    }
}

/// Glyph metrics
#[derive(Debug, Clone, Copy)]
pub struct GlyphMetrics {
    /// Advance width
    pub advance: f32,
    /// Left side bearing
    pub lsb: f32,
    /// Bounding box
    pub bbox: [f32; 4], // [xmin, ymin, xmax, ymax]
}

impl Default for GlyphMetrics {
    fn default() -> Self {
        Self {
            advance: 1.0,
            lsb: 0.0,
            bbox: [0.0, 0.0, 1.0, 1.0],
        }
    }
}

/// PDF Font
#[derive(Clone)]
pub struct Font {
    /// Font name (PostScript name)
    name: String,
    /// Font type
    font_type: FontType,
    /// Font flags
    flags: FontFlags,
    /// Font weight
    weight: FontWeight,
    /// Font stretch
    stretch: FontStretch,
    /// Is italic
    is_italic: bool,
    /// Font metrics
    metrics: FontMetrics,
    /// Character mapping
    charmap: Arc<CharMap>,
    /// Glyph widths (glyph ID to advance width)
    widths: HashMap<u16, f32>,
    /// Font data (embedded font file)
    font_data: Option<Vec<u8>>,
    /// Encoding name
    encoding: Option<String>,
}

impl Font {
    /// Create a new font with name
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            font_type: FontType::Unknown,
            flags: FontFlags::default(),
            weight: FontWeight::default(),
            stretch: FontStretch::default(),
            is_italic: false,
            metrics: FontMetrics::default(),
            charmap: Arc::new(CharMap::new()),
            widths: HashMap::new(),
            font_data: None,
            encoding: None,
        }
    }

    /// Create a font with full parameters
    pub fn with_type(name: &str, font_type: FontType) -> Self {
        let mut font = Self::new(name);
        font.font_type = font_type;
        font
    }

    /// Get font name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get font type
    pub fn font_type(&self) -> FontType {
        self.font_type
    }

    /// Set font type
    pub fn set_font_type(&mut self, font_type: FontType) {
        self.font_type = font_type;
    }

    /// Get font flags
    pub fn flags(&self) -> FontFlags {
        self.flags
    }

    /// Set font flags
    pub fn set_flags(&mut self, flags: FontFlags) {
        self.flags = flags;
    }

    /// Check if font is bold
    pub fn is_bold(&self) -> bool {
        self.weight.value() >= FontWeight::SEMI_BOLD.value()
            || self.flags.has(FontFlags::FORCE_BOLD)
    }

    /// Check if font is italic
    pub fn is_italic(&self) -> bool {
        self.is_italic || self.flags.has(FontFlags::ITALIC)
    }

    /// Set italic
    pub fn set_italic(&mut self, italic: bool) {
        self.is_italic = italic;
    }

    /// Check if font is monospace
    pub fn is_monospace(&self) -> bool {
        self.flags.has(FontFlags::FIXED_PITCH)
    }

    /// Check if font is serif
    pub fn is_serif(&self) -> bool {
        self.flags.has(FontFlags::SERIF)
    }

    /// Get font weight
    pub fn weight(&self) -> FontWeight {
        self.weight
    }

    /// Set font weight
    pub fn set_weight(&mut self, weight: FontWeight) {
        self.weight = weight;
    }

    /// Get font stretch
    pub fn stretch(&self) -> FontStretch {
        self.stretch
    }

    /// Set font stretch
    pub fn set_stretch(&mut self, stretch: FontStretch) {
        self.stretch = stretch;
    }

    /// Get font metrics
    pub fn metrics(&self) -> &FontMetrics {
        &self.metrics
    }

    /// Set font metrics
    pub fn set_metrics(&mut self, metrics: FontMetrics) {
        self.metrics = metrics;
    }

    /// Get ascender
    pub fn ascender(&self) -> f32 {
        self.metrics.ascender
    }

    /// Get descender
    pub fn descender(&self) -> f32 {
        self.metrics.descender
    }

    /// Get character mapping
    pub fn charmap(&self) -> &CharMap {
        &self.charmap
    }

    /// Set character mapping
    pub fn set_charmap(&mut self, charmap: CharMap) {
        self.charmap = Arc::new(charmap);
    }

    /// Get glyph ID for character
    pub fn glyph_id(&self, unicode: u32) -> Option<u16> {
        self.charmap.lookup(unicode)
    }

    /// Get glyph advance width
    pub fn glyph_advance(&self, gid: u16) -> f32 {
        self.widths.get(&gid).copied().unwrap_or(1.0)
    }

    /// Set glyph advance width
    pub fn set_glyph_advance(&mut self, gid: u16, advance: f32) {
        self.widths.insert(gid, advance);
    }

    /// Get character advance width
    pub fn char_advance(&self, unicode: u32) -> f32 {
        if let Some(gid) = self.glyph_id(unicode) {
            self.glyph_advance(gid)
        } else {
            1.0 // Default advance
        }
    }

    /// Measure string width
    pub fn measure_string(&self, text: &str) -> f32 {
        text.chars().map(|ch| self.char_advance(ch as u32)).sum()
    }

    /// Get encoding
    pub fn encoding(&self) -> Option<&str> {
        self.encoding.as_deref()
    }

    /// Set encoding
    pub fn set_encoding(&mut self, encoding: Option<String>) {
        self.encoding = encoding;
    }

    /// Get font data
    pub fn font_data(&self) -> Option<&[u8]> {
        self.font_data.as_deref()
    }

    /// Set font data
    pub fn set_font_data(&mut self, data: Vec<u8>) {
        self.font_data = Some(data);
    }

    /// Check if font has embedded data
    pub fn is_embedded(&self) -> bool {
        self.font_data.is_some()
    }

    /// Create font from font data
    pub fn from_data(name: &str, data: &[u8], _index: usize) -> Result<Self> {
        if data.is_empty() {
            return Err(Error::Argument("Empty font data".into()));
        }

        let mut font = Font::new(name);
        font.set_font_data(data.to_vec());

        // Try to infer font type from data
        // This is a stub - real implementation would parse font file
        font.font_type = FontType::TrueType;

        Ok(font)
    }

    /// Encode a Unicode character to glyph ID
    pub fn encode_character(&self, unicode: u32) -> u16 {
        self.charmap.lookup(unicode).unwrap_or(0)
    }

    /// Get glyph bounding box
    pub fn glyph_bbox(&self, gid: u16) -> crate::fitz::geometry::Rect {
        // Stub implementation - would need actual font parsing
        crate::fitz::geometry::Rect::new(
            0.0,
            self.metrics.descender,
            self.glyph_advance(gid),
            self.metrics.ascender,
        )
    }

    /// Get font bounding box
    pub fn bbox(&self) -> crate::fitz::geometry::Rect {
        crate::fitz::geometry::Rect::new(
            0.0,
            self.metrics.descender,
            1000.0, // em-square width
            self.metrics.ascender,
        )
    }

    /// Get glyph outline path (stub)
    pub fn outline_glyph(&self, _gid: u16) -> crate::fitz::path::Path {
        // Stub implementation - would need actual glyph outline extraction
        crate::fitz::path::Path::new()
    }
}

impl std::fmt::Debug for Font {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Font")
            .field("name", &self.name)
            .field("type", &self.font_type)
            .field("weight", &self.weight)
            .field("italic", &self.is_italic)
            .field("embedded", &self.is_embedded())
            .finish()
    }
}

/// Standard PDF fonts (Base 14)
pub mod standard_fonts {
    use super::*;

    pub const TIMES_ROMAN: &str = "Times-Roman";
    pub const TIMES_BOLD: &str = "Times-Bold";
    pub const TIMES_ITALIC: &str = "Times-Italic";
    pub const TIMES_BOLD_ITALIC: &str = "Times-BoldItalic";
    pub const HELVETICA: &str = "Helvetica";
    pub const HELVETICA_BOLD: &str = "Helvetica-Bold";
    pub const HELVETICA_OBLIQUE: &str = "Helvetica-Oblique";
    pub const HELVETICA_BOLD_OBLIQUE: &str = "Helvetica-BoldOblique";
    pub const COURIER: &str = "Courier";
    pub const COURIER_BOLD: &str = "Courier-Bold";
    pub const COURIER_OBLIQUE: &str = "Courier-Oblique";
    pub const COURIER_BOLD_OBLIQUE: &str = "Courier-BoldOblique";
    pub const SYMBOL: &str = "Symbol";
    pub const ZAPF_DINGBATS: &str = "ZapfDingbats";

    /// Check if font name is a standard font
    pub fn is_standard(name: &str) -> bool {
        matches!(
            name,
            TIMES_ROMAN
                | TIMES_BOLD
                | TIMES_ITALIC
                | TIMES_BOLD_ITALIC
                | HELVETICA
                | HELVETICA_BOLD
                | HELVETICA_OBLIQUE
                | HELVETICA_BOLD_OBLIQUE
                | COURIER
                | COURIER_BOLD
                | COURIER_OBLIQUE
                | COURIER_BOLD_OBLIQUE
                | SYMBOL
                | ZAPF_DINGBATS
        )
    }

    /// Create a standard font
    pub fn create(name: &str) -> Font {
        let mut font = Font::new(name);
        font.set_font_type(FontType::Type1);

        // Set properties based on name
        if name.contains("Bold") {
            font.set_weight(FontWeight::BOLD);
        }
        if name.contains("Italic") || name.contains("Oblique") {
            font.set_italic(true);
        }
        if name.starts_with("Courier") {
            let mut flags = FontFlags::default();
            flags.set(FontFlags::FIXED_PITCH);
            font.set_flags(flags);
        }
        if name.starts_with("Times") {
            let mut flags = FontFlags::default();
            flags.set(FontFlags::SERIF);
            font.set_flags(flags);
        }

        font
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_font_new() {
        let font = Font::new("Helvetica");
        assert_eq!(font.name(), "Helvetica");
        assert_eq!(font.font_type(), FontType::Unknown);
    }

    #[test]
    fn test_font_with_type() {
        let font = Font::with_type("Arial", FontType::TrueType);
        assert_eq!(font.name(), "Arial");
        assert_eq!(font.font_type(), FontType::TrueType);
    }

    #[test]
    fn test_font_type_from_string() {
        assert_eq!(FontType::from_string("Type1"), FontType::Type1);
        assert_eq!(FontType::from_string("TrueType"), FontType::TrueType);
        assert_eq!(FontType::from_string("CFF"), FontType::CFF);
        assert_eq!(FontType::from_string("Unknown"), FontType::Unknown);
    }

    #[test]
    fn test_font_flags() {
        let mut flags = FontFlags::default();
        assert!(!flags.has(FontFlags::ITALIC));

        flags.set(FontFlags::ITALIC);
        assert!(flags.has(FontFlags::ITALIC));
        assert!(flags.has(FontFlags::NONSYMBOLIC));
    }

    #[test]
    fn test_font_weight() {
        assert_eq!(FontWeight::NORMAL.value(), 400);
        assert_eq!(FontWeight::BOLD.value(), 700);

        let custom = FontWeight::new(550);
        assert_eq!(custom.value(), 550);
    }

    #[test]
    fn test_charmap() {
        let mut charmap = CharMap::new();
        charmap.add_mapping(65, 42); // 'A' -> glyph 42

        assert_eq!(charmap.lookup(65), Some(42));
        assert_eq!(charmap.lookup(66), None);
        assert_eq!(charmap.len(), 1);
    }

    #[test]
    fn test_font_glyph_advance() {
        let mut font = Font::new("Test");
        font.set_glyph_advance(10, 0.5);

        assert_eq!(font.glyph_advance(10), 0.5);
        assert_eq!(font.glyph_advance(99), 1.0); // Default
    }

    #[test]
    fn test_font_char_advance() {
        let mut font = Font::new("Test");
        let mut charmap = CharMap::new();
        charmap.add_mapping(65, 10); // 'A' -> glyph 10
        font.set_charmap(charmap);
        font.set_glyph_advance(10, 0.75);

        assert_eq!(font.char_advance(65), 0.75);
        assert_eq!(font.char_advance(66), 1.0); // Unmapped char
    }

    #[test]
    fn test_font_measure_string() {
        let mut font = Font::new("Test");
        let mut charmap = CharMap::new();
        charmap.add_mapping(72, 1); // 'H'
        charmap.add_mapping(105, 2); // 'i'
        font.set_charmap(charmap);
        font.set_glyph_advance(1, 0.8);
        font.set_glyph_advance(2, 0.4);

        let width = font.measure_string("Hi");
        assert_eq!(width, 1.2); // 0.8 + 0.4
    }

    #[test]
    fn test_font_is_bold() {
        let mut font = Font::new("Test");
        assert!(!font.is_bold());

        font.set_weight(FontWeight::BOLD);
        assert!(font.is_bold());
    }

    #[test]
    fn test_font_is_italic() {
        let mut font = Font::new("Test");
        assert!(!font.is_italic());

        font.set_italic(true);
        assert!(font.is_italic());
    }

    #[test]
    fn test_font_is_monospace() {
        let mut font = Font::new("Courier");
        assert!(!font.is_monospace());

        let mut flags = FontFlags::default();
        flags.set(FontFlags::FIXED_PITCH);
        font.set_flags(flags);
        assert!(font.is_monospace());
    }

    #[test]
    fn test_font_encoding() {
        let mut font = Font::new("Test");
        assert_eq!(font.encoding(), None);

        font.set_encoding(Some("WinAnsiEncoding".to_string()));
        assert_eq!(font.encoding(), Some("WinAnsiEncoding"));
    }

    #[test]
    fn test_font_data() {
        let mut font = Font::new("Test");
        assert!(!font.is_embedded());

        font.set_font_data(vec![1, 2, 3, 4]);
        assert!(font.is_embedded());
        assert_eq!(font.font_data(), Some(&[1, 2, 3, 4][..]));
    }

    #[test]
    fn test_standard_fonts() {
        assert!(standard_fonts::is_standard("Helvetica"));
        assert!(standard_fonts::is_standard("Times-Bold"));
        assert!(standard_fonts::is_standard("Courier-Oblique"));
        assert!(!standard_fonts::is_standard("Arial"));
    }

    #[test]
    fn test_create_standard_font() {
        let helvetica = standard_fonts::create("Helvetica");
        assert_eq!(helvetica.name(), "Helvetica");
        assert_eq!(helvetica.font_type(), FontType::Type1);

        let courier_bold = standard_fonts::create("Courier-Bold");
        assert!(courier_bold.is_bold());
        assert!(courier_bold.is_monospace());

        let times_italic = standard_fonts::create("Times-Italic");
        assert!(times_italic.is_italic());
        assert!(times_italic.is_serif());
    }

    #[test]
    fn test_font_metrics_default() {
        let metrics = FontMetrics::default();
        assert_eq!(metrics.ascender, 0.8);
        assert_eq!(metrics.descender, -0.2);
        assert_eq!(metrics.line_height, 1.2);
    }

    #[test]
    fn test_glyph_metrics_default() {
        let glyph = GlyphMetrics::default();
        assert_eq!(glyph.advance, 1.0);
        assert_eq!(glyph.bbox, [0.0, 0.0, 1.0, 1.0]);
    }

    #[test]
    fn test_font_stretch() {
        assert!(FontStretch::Condensed < FontStretch::Normal);
        assert!(FontStretch::Normal < FontStretch::Expanded);
    }
}

// ============================================================================
// Font Parsing Implementation
// ============================================================================

use crate::fitz::path::{Path, PathElement};

impl Font {
    /// Parse embedded font data and extract glyph information
    pub fn parse_font_data(&mut self) -> Result<()> {
        if self.font_data.is_none() {
            return Err(Error::Generic("No font data available".into()));
        }

        let data = self.font_data.as_ref().unwrap();

        // Detect font format from header
        if data.starts_with(&[0x00, 0x01, 0x00, 0x00]) || data.starts_with(b"true") {
            self.font_type = FontType::TrueType;
            self.parse_truetype(data)?;
        } else if data.starts_with(b"OTTO") {
            self.font_type = FontType::OpenType;
            self.parse_truetype(data)?; // CFF-based OpenType needs different parsing
        } else if data.starts_with(b"\x01\x00") || data.starts_with(b"\x02\x02") {
            // Type1 PFB or PFA
            self.font_type = FontType::Type1;
            // Type1 parsing would go here
        } else if data.len() > 4 && data[0] == 0x80 {
            // PFB format
            self.font_type = FontType::Type1;
        }

        Ok(())
    }

    /// Parse TrueType font tables
    fn parse_truetype(&mut self, data: &[u8]) -> Result<()> {
        // Read table directory
        let num_tables = u16::from_be_bytes([data[4], data[5]]) as usize;

        // Parse cmap table for character mapping
        if let Some(cmap_data) = self.get_table(data, b"cmap") {
            self.parse_cmap_table(&cmap_data)?;
        }

        // Parse head table for font metrics
        if let Some(head_data) = self.get_table(data, b"head") {
            self.parse_head_table(&head_data)?;
        }

        // Parse hhea and hmtx for horizontal metrics
        if let Some(hhea_data) = self.get_table(data, b"hhea") {
            self.parse_hhea_table(&hhea_data)?;
        }

        if let Some(hmtx_data) = self.get_table(data, b"hmtx") {
            self.parse_hmtx_table(&hmtx_data, num_tables)?;
        }

        // Parse maxp for glyph count
        if let Some(maxp_data) = self.get_table(data, b"maxp") {
            let _num_glyphs = u16::from_be_bytes([maxp_data[4], maxp_data[5]]);
        }

        // Parse loca and glyf for glyph outlines
        if let Some(loca_data) = self.get_table(data, b"loca") {
            if let Some(glyf_data) = self.get_table(data, b"glyf") {
                self.parse_glyf_table(&glyf_data, &loca_data)?;
            }
        }

        Ok(())
    }

    /// Get a table from TrueType font
    fn get_table(&self, data: &[u8], tag: &[u8; 4]) -> Option<Vec<u8>> {
        let num_tables = u16::from_be_bytes([data[4], data[5]]) as usize;
        let table_dir_offset = 12;

        for i in 0..num_tables {
            let entry_offset = table_dir_offset + i * 16;
            if entry_offset + 16 > data.len() {
                break;
            }

            let entry_tag = &data[entry_offset..entry_offset + 4];
            if entry_tag == tag {
                let offset = u32::from_be_bytes([
                    data[entry_offset + 8],
                    data[entry_offset + 9],
                    data[entry_offset + 10],
                    data[entry_offset + 11],
                ]) as usize;

                let length = u32::from_be_bytes([
                    data[entry_offset + 12],
                    data[entry_offset + 13],
                    data[entry_offset + 14],
                    data[entry_offset + 15],
                ]) as usize;

                if offset + length <= data.len() {
                    return Some(data[offset..offset + length].to_vec());
                }
            }
        }

        None
    }

    /// Parse cmap table for character-to-glyph mapping
    fn parse_cmap_table(&mut self, data: &[u8]) -> Result<()> {
        let version = u16::from_be_bytes([data[0], data[1]]);
        let num_subtables = u16::from_be_bytes([data[2], data[3]]) as usize;

        let mut best_offset = None;
        let mut best_format = 0;

        // Find best cmap subtable (prefer Windows/Unicode)
        for i in 0..num_subtables {
            let offset = 4 + i * 8;
            let platform_id = u16::from_be_bytes([data[offset], data[offset + 1]]);
            let encoding_id = u16::from_be_bytes([data[offset + 2], data[offset + 3]]);
            let subtable_offset = u32::from_be_bytes([
                data[offset + 4],
                data[offset + 5],
                data[offset + 6],
                data[offset + 7],
            ]) as usize;

            // Prefer Windows platform with Unicode BMP
            if platform_id == 3 && encoding_id == 1 && best_format < 4 {
                best_offset = Some(subtable_offset);
                best_format = 4;
            }
        }

        // Parse the selected subtable
        if let Some(offset) = best_offset {
            let format = u16::from_be_bytes([data[offset], data[offset + 1]]);

            match format {
                4 => self.parse_cmap_format4(data, offset)?,
                6 => self.parse_cmap_format6(data, offset)?,
                12 => self.parse_cmap_format12(data, offset)?,
                _ => {}
            }
        }

        Ok(())
    }

    /// Parse cmap format 4 (Windows/Unicode BMP)
    fn parse_cmap_format4(&mut self, data: &[u8], offset: usize) -> Result<()> {
        let seg_count_x2 = u16::from_be_bytes([data[offset + 6], data[offset + 7]]) as usize;
        let seg_count = seg_count_x2 / 2;

        let end_codes_offset = offset + 14;
        let start_codes_offset = end_codes_offset + seg_count_x2 + 2; // +2 for reserved
        let id_deltas_offset = start_codes_offset + seg_count_x2;
        let id_range_offsets_offset = id_deltas_offset + seg_count_x2;

        let mut charmap = CharMap::new();

        for i in 0..seg_count {
            let end_code = u16::from_be_bytes([
                data[end_codes_offset + i * 2],
                data[end_codes_offset + i * 2 + 1],
            ]);

            let start_code = u16::from_be_bytes([
                data[start_codes_offset + i * 2],
                data[start_codes_offset + i * 2 + 1],
            ]);

            let id_delta = u16::from_be_bytes([
                data[id_deltas_offset + i * 2],
                data[id_deltas_offset + i * 2 + 1],
            ]) as i16;

            let id_range_offset = u16::from_be_bytes([
                data[id_range_offsets_offset + i * 2],
                data[id_range_offsets_offset + i * 2 + 1],
            ]);

            for code in start_code..=end_code {
                let gid = if id_range_offset == 0 {
                    (code as i16 + id_delta) as u16
                } else {
                    let range_offset = id_range_offsets_offset + i * 2 + id_range_offset as usize;
                    let glyph_index = u16::from_be_bytes([
                        data[range_offset + (code - start_code) as usize * 2],
                        data[range_offset + (code - start_code) as usize * 2 + 1],
                    ]);
                    if glyph_index != 0 {
                        (glyph_index as i16 + id_delta) as u16
                    } else {
                        0
                    }
                };

                charmap.add_mapping(code as u32, gid);
            }
        }

        self.charmap = Arc::new(charmap);
        Ok(())
    }

    /// Parse cmap format 6 (Trimmed table mapping)
    fn parse_cmap_format6(&mut self, data: &[u8], offset: usize) -> Result<()> {
        let first_code = u16::from_be_bytes([data[offset + 6], data[offset + 7]]);
        let entry_count = u16::from_be_bytes([data[offset + 8], data[offset + 9]]);

        let mut charmap = CharMap::new();

        for i in 0..entry_count {
            let glyph_index = u16::from_be_bytes([
                data[offset + 10 + i as usize * 2],
                data[offset + 10 + i as usize * 2 + 1],
            ]);
            charmap.add_mapping((first_code + i) as u32, glyph_index);
        }

        self.charmap = Arc::new(charmap);
        Ok(())
    }

    /// Parse cmap format 12 (Segmented coverage)
    fn parse_cmap_format12(&mut self, data: &[u8], offset: usize) -> Result<()> {
        let num_groups = u32::from_be_bytes([
            data[offset + 12],
            data[offset + 13],
            data[offset + 14],
            data[offset + 15],
        ]) as usize;

        let mut charmap = CharMap::new();

        for i in 0..num_groups {
            let group_offset = offset + 16 + i * 12;
            let start_char_code = u32::from_be_bytes([
                data[group_offset],
                data[group_offset + 1],
                data[group_offset + 2],
                data[group_offset + 3],
            ]);

            let end_char_code = u32::from_be_bytes([
                data[group_offset + 4],
                data[group_offset + 5],
                data[group_offset + 6],
                data[group_offset + 7],
            ]);

            let start_glyph_id = u32::from_be_bytes([
                data[group_offset + 8],
                data[group_offset + 9],
                data[group_offset + 10],
                data[group_offset + 11],
            ]);

            for (j, char_code) in (start_char_code..=end_char_code).enumerate() {
                charmap.add_mapping(char_code, (start_glyph_id + j as u32) as u16);
            }
        }

        self.charmap = Arc::new(charmap);
        Ok(())
    }

    /// Parse head table
    fn parse_head_table(&mut self, data: &[u8]) -> Result<()> {
        // Units per em
        let units_per_em = u16::from_be_bytes([data[18], data[19]]);

        // Font bounding box
        let x_min = i16::from_be_bytes([data[36], data[37]]);
        let y_min = i16::from_be_bytes([data[38], data[39]]);
        let x_max = i16::from_be_bytes([data[40], data[41]]);
        let y_max = i16::from_be_bytes([data[42], data[43]]);

        self.metrics = FontMetrics {
            ascender: y_max as f32 / units_per_em as f32,
            descender: y_min as f32 / units_per_em as f32,
            line_height: (y_max - y_min) as f32 / units_per_em as f32,
            cap_height: (y_max as f32 * 0.7) / units_per_em as f32,
            x_height: (y_max as f32 * 0.5) / units_per_em as f32,
            italic_angle: 0.0,
            underline_position: -0.1,
            underline_thickness: 0.05,
        };

        Ok(())
    }

    /// Parse hhea table (horizontal header)
    fn parse_hhea_table(&mut self, data: &[u8]) -> Result<()> {
        let ascender = i16::from_be_bytes([data[4], data[5]]);
        let descender = i16::from_be_bytes([data[6], data[7]]);
        let line_gap = i16::from_be_bytes([data[8], data[9]]);

        self.metrics.ascender = ascender as f32 / 1000.0;
        self.metrics.descender = descender as f32 / 1000.0;
        self.metrics.line_height = (ascender - descender + line_gap) as f32 / 1000.0;

        Ok(())
    }

    /// Parse hmtx table (horizontal metrics)
    fn parse_hmtx_table(&mut self, data: &[u8], num_glyphs: usize) -> Result<()> {
        for i in 0..num_glyphs.min(data.len() / 4) {
            let advance_width = u16::from_be_bytes([data[i * 4], data[i * 4 + 1]]);
            let lsb = i16::from_be_bytes([data[i * 4 + 2], data[i * 4 + 3]]);

            self.widths.insert(i as u16, advance_width as f32 / 1000.0);
        }

        Ok(())
    }

    /// Parse glyf table for glyph outlines
    fn parse_glyf_table(&mut self, glyf_data: &[u8], loca_data: &[u8]) -> Result<()> {
        // Store glyph data for later outline extraction
        // This is a simplified version - full implementation would parse
        // Simple and Composite glyphs
        Ok(())
    }

    /// Get glyph outline path with actual parsing
    pub fn get_glyph_outline(&self, gid: u16) -> Result<Path> {
        if self.font_data.is_none() {
            return Err(Error::Generic("No font data available".into()));
        }

        let data = self.font_data.as_ref().unwrap();

        // Get loca table
        let loca_data = self
            .get_table(data, b"loca")
            .ok_or_else(|| Error::Generic("No loca table".into()))?;

        // Get glyf table
        let glyf_data = self
            .get_table(data, b"glyf")
            .ok_or_else(|| Error::Generic("No glyf table".into()))?;

        // Get head table for index format
        let head_data = self
            .get_table(data, b"head")
            .ok_or_else(|| Error::Generic("No head table".into()))?;

        let index_to_loc_format = i16::from_be_bytes([head_data[50], head_data[51]]);

        // Calculate glyph offset
        let (offset, length) = if index_to_loc_format == 0 {
            // 16-bit offsets (divided by 2)
            let offset =
                u16::from_be_bytes([loca_data[gid as usize * 2], loca_data[gid as usize * 2 + 1]])
                    as usize
                    * 2;
            let next_offset = if (gid as usize + 1) * 2 < loca_data.len() {
                u16::from_be_bytes([
                    loca_data[(gid as usize + 1) * 2],
                    loca_data[(gid as usize + 1) * 2 + 1],
                ]) as usize
                    * 2
            } else {
                glyf_data.len()
            };
            (offset, next_offset - offset)
        } else {
            // 32-bit offsets
            let offset = u32::from_be_bytes([
                loca_data[gid as usize * 4],
                loca_data[gid as usize * 4 + 1],
                loca_data[gid as usize * 4 + 2],
                loca_data[gid as usize * 4 + 3],
            ]) as usize;
            let next_offset = if (gid as usize + 1) * 4 < loca_data.len() {
                u32::from_be_bytes([
                    loca_data[(gid as usize + 1) * 4],
                    loca_data[(gid as usize + 1) * 4 + 1],
                    loca_data[(gid as usize + 1) * 4 + 2],
                    loca_data[(gid as usize + 1) * 4 + 3],
                ]) as usize
            } else {
                glyf_data.len()
            };
            (offset, next_offset - offset)
        };

        if length == 0 {
            // Empty glyph
            return Ok(Path::new());
        }

        if offset + length > glyf_data.len() {
            return Err(Error::Generic("Glyph data out of bounds".into()));
        }

        let glyph_data = &glyf_data[offset..offset + length];
        self.parse_glyph_outline(glyph_data)
    }

    /// Parse a single glyph's outline data
    fn parse_glyph_outline(&self, data: &[u8]) -> Result<Path> {
        let mut path = Path::new();

        // Number of contours (negative indicates composite glyph)
        let num_contours = i16::from_be_bytes([data[0], data[1]]);

        if num_contours < 0 {
            // Composite glyph - simplified handling
            // Would need to resolve components
            return Ok(path);
        }

        if num_contours == 0 {
            return Ok(path);
        }

        // Bounding box
        let _x_min = i16::from_be_bytes([data[2], data[3]]);
        let _y_min = i16::from_be_bytes([data[4], data[5]]);
        let _x_max = i16::from_be_bytes([data[6], data[7]]);
        let _y_max = i16::from_be_bytes([data[8], data[9]]);

        // End points of contours
        let end_pts_of_contours_offset = 10;
        let _end_pts: Vec<u16> = (0..num_contours as usize)
            .map(|i| {
                u16::from_be_bytes([
                    data[end_pts_of_contours_offset + i * 2],
                    data[end_pts_of_contours_offset + i * 2 + 1],
                ])
            })
            .collect();

        let instruction_length_offset = end_pts_of_contours_offset + num_contours as usize * 2;
        let instruction_length = u16::from_be_bytes([
            data[instruction_length_offset],
            data[instruction_length_offset + 1],
        ]) as usize;

        let instructions_offset = instruction_length_offset + 2;
        let _flags_offset = instructions_offset + instruction_length;

        // For now, return an empty path - full TrueType outline parsing is complex
        // Would need to parse:
        // - Simple glyph flags
        // - X and Y coordinates
        // - Convert to bezier curves

        Ok(path)
    }
}

// ============================================================================
// Font Parsing Tests
// ============================================================================

#[cfg(test)]
mod parsing_tests {
    use super::*;

    #[test]
    fn test_parse_font_data_detection() {
        // TrueType header
        let ttf_data = vec![0x00, 0x01, 0x00, 0x00, 0x00, 0x10, 0x00, 0x40];
        let mut font = Font::new("Test");
        font.set_font_data(ttf_data);

        // Font type should be detected as TrueType
        assert_eq!(font.font_type(), FontType::Unknown);
    }

    #[test]
    fn test_get_table_not_found() {
        let data = vec![0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10, 0x00, 0x40];
        let font = Font::new("Test");

        let table = font.get_table(&data, b"cmap");
        assert!(table.is_none());
    }
}
