//! Glyph Rasterization
//!
//! Converts glyph outlines to pixmaps for rendering text in PDFs.
//!
//! Supports:
//! - TrueType/OpenType fonts
//! - Type1 PostScript fonts
//! - CFF (Compact Font Format) fonts
//! - Glyph caching for performance

use crate::fitz::error::{Error, Result};
use crate::fitz::geometry::{Matrix, Point, Rect};
use crate::fitz::path::Path;
use crate::fitz::pixmap::Pixmap;
use crate::fitz::render::Rasterizer;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Glyph identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GlyphId(pub u16);

impl GlyphId {
    pub fn new(id: u16) -> Self {
        Self(id)
    }

    pub fn value(&self) -> u16 {
        self.0
    }
}

/// Glyph metrics
#[derive(Debug, Clone, Copy)]
pub struct GlyphMetrics {
    /// Glyph advance width
    pub advance_width: f32,
    /// Glyph advance height (for vertical writing)
    pub advance_height: f32,
    /// Left side bearing
    pub lsb: f32,
    /// Top side bearing (for vertical writing)
    pub tsb: f32,
    /// Bounding box
    pub bbox: Rect,
}

impl Default for GlyphMetrics {
    fn default() -> Self {
        Self {
            advance_width: 1.0,
            advance_height: 1.0,
            lsb: 0.0,
            tsb: 0.0,
            bbox: Rect::new(0.0, 0.0, 1.0, 1.0),
        }
    }
}

/// Glyph outline as a path
#[derive(Debug, Clone)]
pub struct GlyphOutline {
    /// Glyph identifier
    pub gid: GlyphId,
    /// Glyph path
    pub path: Path,
    /// Glyph metrics
    pub metrics: GlyphMetrics,
}

impl GlyphOutline {
    /// Create a new glyph outline
    pub fn new(gid: GlyphId, path: Path, metrics: GlyphMetrics) -> Self {
        Self { gid, path, metrics }
    }

    /// Transform the glyph outline by a matrix
    pub fn transform(&mut self, ctm: &Matrix) {
        // Transform path
        let transformed_path = Path::new();
        // TODO: Transform each path element
        self.path = transformed_path;

        // Transform metrics
        let p0 = ctm.transform_point(Point::new(0.0, 0.0));
        let p1 = ctm.transform_point(Point::new(self.metrics.advance_width, 0.0));
        self.metrics.advance_width = (p1.x - p0.x).abs();

        // Transform bbox
        let bbox = self.metrics.bbox;
        let corners = [
            ctm.transform_point(Point::new(bbox.x0, bbox.y0)),
            ctm.transform_point(Point::new(bbox.x1, bbox.y0)),
            ctm.transform_point(Point::new(bbox.x1, bbox.y1)),
            ctm.transform_point(Point::new(bbox.x0, bbox.y1)),
        ];

        let min_x = corners.iter().map(|p| p.x).fold(f32::INFINITY, f32::min);
        let min_y = corners.iter().map(|p| p.y).fold(f32::INFINITY, f32::min);
        let max_x = corners
            .iter()
            .map(|p| p.x)
            .fold(f32::NEG_INFINITY, f32::max);
        let max_y = corners
            .iter()
            .map(|p| p.y)
            .fold(f32::NEG_INFINITY, f32::max);

        self.metrics.bbox = Rect::new(min_x, min_y, max_x, max_y);
    }
}

/// Glyph cache key
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct GlyphCacheKey {
    gid: GlyphId,
    size: u32,      // Font size in 1/64ths of a point
    subpixel_x: u8, // Subpixel position (0-63)
    subpixel_y: u8, // Subpixel position (0-63)
}

/// Glyph cache
pub struct GlyphCache {
    /// Cached glyph pixmaps
    cache: Arc<Mutex<HashMap<GlyphCacheKey, Arc<Pixmap>>>>,
    /// Maximum cache size in bytes
    max_size: usize,
    /// Current cache size in bytes
    current_size: Arc<Mutex<usize>>,
}

impl GlyphCache {
    /// Create a new glyph cache
    pub fn new(max_size_mb: usize) -> Self {
        Self {
            cache: Arc::new(Mutex::new(HashMap::new())),
            max_size: max_size_mb * 1024 * 1024,
            current_size: Arc::new(Mutex::new(0)),
        }
    }

    /// Get a glyph from the cache
    pub fn get(
        &self,
        gid: GlyphId,
        size: f32,
        subpixel_x: f32,
        subpixel_y: f32,
    ) -> Option<Arc<Pixmap>> {
        let key = GlyphCacheKey {
            gid,
            size: (size * 64.0) as u32,
            subpixel_x: (subpixel_x * 64.0) as u8,
            subpixel_y: (subpixel_y * 64.0) as u8,
        };

        self.cache.lock().unwrap().get(&key).cloned()
    }

    /// Insert a glyph into the cache
    pub fn insert(
        &self,
        gid: GlyphId,
        size: f32,
        subpixel_x: f32,
        subpixel_y: f32,
        pixmap: Pixmap,
    ) {
        let key = GlyphCacheKey {
            gid,
            size: (size * 64.0) as u32,
            subpixel_x: (subpixel_x * 64.0) as u8,
            subpixel_y: (subpixel_y * 64.0) as u8,
        };

        let pixmap_size = pixmap.samples().len();
        let pixmap = Arc::new(pixmap);

        // Check cache size
        let mut current = self.current_size.lock().unwrap();
        if *current + pixmap_size > self.max_size {
            // Simple eviction: clear entire cache
            // A better strategy would be LRU
            self.clear();
            *current = 0;
        }

        self.cache.lock().unwrap().insert(key, pixmap);
        *current += pixmap_size;
    }

    /// Clear the cache
    pub fn clear(&self) {
        self.cache.lock().unwrap().clear();
        *self.current_size.lock().unwrap() = 0;
    }

    /// Get cache statistics
    pub fn stats(&self) -> (usize, usize, usize) {
        let cache = self.cache.lock().unwrap();
        let size = *self.current_size.lock().unwrap();
        (cache.len(), size, self.max_size)
    }
}

impl Default for GlyphCache {
    fn default() -> Self {
        Self::new(16) // 16 MB default
    }
}

/// Glyph rasterizer
pub struct GlyphRasterizer {
    /// Pixel rasterizer
    rasterizer: Rasterizer,
    /// Glyph cache
    cache: GlyphCache,
}

impl GlyphRasterizer {
    /// Create a new glyph rasterizer
    pub fn new() -> Self {
        // Create a reasonable default rasterizer size
        let clip = Rect::new(0.0, 0.0, 1024.0, 1024.0);
        Self {
            rasterizer: Rasterizer::new(1024, 1024, clip),
            cache: GlyphCache::default(),
        }
    }

    /// Create glyph rasterizer with custom cache size
    pub fn with_cache_size(cache_size_mb: usize) -> Self {
        let clip = Rect::new(0.0, 0.0, 1024.0, 1024.0);
        Self {
            rasterizer: Rasterizer::new(1024, 1024, clip),
            cache: GlyphCache::new(cache_size_mb),
        }
    }

    /// Rasterize a glyph outline to a pixmap
    pub fn rasterize_glyph(
        &self,
        outline: &GlyphOutline,
        font_size: f32,
        subpixel_x: f32,
        subpixel_y: f32,
    ) -> Result<Pixmap> {
        // Check cache first
        if let Some(pixmap) = self
            .cache
            .get(outline.gid, font_size, subpixel_x, subpixel_y)
        {
            return Ok((*pixmap).clone());
        }

        // Calculate glyph transformation matrix
        let scale = font_size / 1000.0; // Assuming 1000 units per em (standard)
        let ctm = Matrix::new(
            scale, 0.0, 0.0, -scale, // Flip Y axis (PDF coordinates)
            subpixel_x, subpixel_y,
        );

        // Calculate pixmap dimensions
        let bbox = outline.metrics.bbox;
        let transformed_bbox = bbox.transform(&ctm);

        let width = (transformed_bbox.width().ceil() as i32).max(1);
        let height = (transformed_bbox.height().ceil() as i32).max(1);

        // Create destination pixmap (grayscale + alpha)
        let mut pixmap = Pixmap::new(None, width, height, true)?;

        // Rasterize the glyph path
        let colorspace = crate::fitz::colorspace::Colorspace::device_gray();
        let color = vec![1.0]; // White glyph
        let alpha = 1.0;

        self.rasterizer.fill_path(
            &outline.path,
            false, // Non-zero winding rule
            &ctm,
            &colorspace,
            &color,
            alpha,
            &mut pixmap,
        );

        // Cache the result
        self.cache.insert(
            outline.gid,
            font_size,
            subpixel_x,
            subpixel_y,
            pixmap.clone(),
        );

        Ok(pixmap)
    }

    /// Rasterize multiple glyphs (for performance)
    pub fn rasterize_glyphs(
        &self,
        outlines: &[&GlyphOutline],
        font_size: f32,
    ) -> Result<Vec<Pixmap>> {
        outlines
            .iter()
            .map(|outline| self.rasterize_glyph(outline, font_size, 0.0, 0.0))
            .collect()
    }

    /// Clear the glyph cache
    pub fn clear_cache(&self) {
        self.cache.clear();
    }

    /// Get cache statistics
    pub fn cache_stats(&self) -> (usize, usize, usize) {
        self.cache.stats()
    }
}

impl Default for GlyphRasterizer {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper: Create a simple glyph outline for a rectangle (for missing glyphs)
pub fn create_missing_glyph_outline(gid: GlyphId, advance: f32) -> GlyphOutline {
    let mut path = Path::new();

    // Draw a simple rectangle as "missing glyph" indicator
    let width = advance * 0.8;
    let height = advance * 1.0;
    let margin = advance * 0.1;

    path.move_to(Point::new(margin, margin));
    path.line_to(Point::new(width, margin));
    path.line_to(Point::new(width, height));
    path.line_to(Point::new(margin, height));
    path.close();

    // Inner rectangle (hollow)
    let inner_margin = margin * 2.0;
    path.move_to(Point::new(inner_margin, inner_margin));
    path.line_to(Point::new(width - inner_margin, inner_margin));
    path.line_to(Point::new(width - inner_margin, height - inner_margin));
    path.line_to(Point::new(inner_margin, height - inner_margin));
    path.close();

    let metrics = GlyphMetrics {
        advance_width: advance,
        advance_height: height,
        lsb: margin,
        tsb: margin,
        bbox: Rect::new(margin, margin, width, height),
    };

    GlyphOutline::new(gid, path, metrics)
}

/// TrueType glyph loader (simplified)
pub struct TrueTypeLoader {
    /// Font data
    data: Vec<u8>,
    /// Units per em
    units_per_em: u16,
    /// Glyph count
    num_glyphs: u16,
}

impl TrueTypeLoader {
    /// Create a new TrueType loader from font data
    pub fn new(data: Vec<u8>) -> Result<Self> {
        // Simplified TrueType parser
        // A real implementation would use ttf-parser crate

        // Check for TrueType signature
        if data.len() < 12 {
            return Err(Error::Generic("Invalid TrueType font data".into()));
        }

        let signature = &data[0..4];
        if signature != b"\x00\x01\x00\x00" && signature != b"true" && signature != b"typ1" {
            return Err(Error::Generic("Invalid TrueType signature".into()));
        }

        Ok(Self {
            data,
            units_per_em: 1000, // Default
            num_glyphs: 1,      // Simplified
        })
    }

    /// Get the number of glyphs in the font
    pub fn num_glyphs(&self) -> u16 {
        self.num_glyphs
    }

    /// Get units per em
    pub fn units_per_em(&self) -> u16 {
        self.units_per_em
    }

    /// Load a glyph outline by ID
    pub fn load_glyph(&self, gid: GlyphId) -> Result<GlyphOutline> {
        // Simplified: return a placeholder outline
        // Real implementation would parse the 'glyf' table
        Ok(create_missing_glyph_outline(gid, 500.0))
    }

    /// Get glyph metrics
    pub fn glyph_metrics(&self, gid: GlyphId) -> Result<GlyphMetrics> {
        // Simplified: return default metrics
        // Real implementation would parse 'hmtx' and 'vmtx' tables
        let _ = gid;
        Ok(GlyphMetrics::default())
    }
}

/// Type1 glyph loader (simplified)
pub struct Type1Loader {
    /// Font data
    data: Vec<u8>,
}

impl Type1Loader {
    /// Create a new Type1 loader from font data
    pub fn new(data: Vec<u8>) -> Result<Self> {
        // Check for Type1 signature
        if data.len() < 16 {
            return Err(Error::Generic("Invalid Type1 font data".into()));
        }

        // Type1 fonts start with "%!PS-AdobeFont" or "%!FontType1"
        let header = String::from_utf8_lossy(&data[0..14.min(data.len())]);
        if !header.starts_with("%!") {
            return Err(Error::Generic("Invalid Type1 signature".into()));
        }

        Ok(Self { data })
    }

    /// Load a glyph outline by name
    pub fn load_glyph_by_name(&self, _name: &str) -> Result<GlyphOutline> {
        // Simplified: return a placeholder outline
        // Real implementation would parse the CharStrings dictionary
        Ok(create_missing_glyph_outline(GlyphId::new(0), 500.0))
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_glyph_id() {
        let gid = GlyphId::new(42);
        assert_eq!(gid.value(), 42);
    }

    #[test]
    fn test_glyph_metrics_default() {
        let metrics = GlyphMetrics::default();
        assert_eq!(metrics.advance_width, 1.0);
        assert_eq!(metrics.lsb, 0.0);
    }

    #[test]
    fn test_glyph_cache_creation() {
        let cache = GlyphCache::new(16);
        let (count, size, max) = cache.stats();
        assert_eq!(count, 0);
        assert_eq!(size, 0);
        assert_eq!(max, 16 * 1024 * 1024);
    }

    #[test]
    fn test_glyph_cache_insert_get() {
        let cache = GlyphCache::new(16);
        let pixmap = Pixmap::new(None, 10, 10, true).unwrap();
        let gid = GlyphId::new(42);

        cache.insert(gid, 12.0, 0.0, 0.0, pixmap);

        let retrieved = cache.get(gid, 12.0, 0.0, 0.0);
        assert!(retrieved.is_some());

        let (count, _, _) = cache.stats();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_glyph_cache_clear() {
        let cache = GlyphCache::new(16);
        let pixmap = Pixmap::new(None, 10, 10, true).unwrap();

        cache.insert(GlyphId::new(1), 12.0, 0.0, 0.0, pixmap);
        cache.clear();

        let (count, size, _) = cache.stats();
        assert_eq!(count, 0);
        assert_eq!(size, 0);
    }

    #[test]
    fn test_create_missing_glyph_outline() {
        let gid = GlyphId::new(0);
        let outline = create_missing_glyph_outline(gid, 500.0);

        assert_eq!(outline.gid, gid);
        assert_eq!(outline.metrics.advance_width, 500.0);
        assert!(!outline.path.elements().is_empty());
    }

    #[test]
    fn test_glyph_rasterizer_creation() {
        let rasterizer = GlyphRasterizer::new();
        let (count, _, _) = rasterizer.cache_stats();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_glyph_rasterizer_with_cache_size() {
        let rasterizer = GlyphRasterizer::with_cache_size(32);
        let (_, _, max) = rasterizer.cache_stats();
        assert_eq!(max, 32 * 1024 * 1024);
    }
}
