//! C FFI for glyph handling - MuPDF compatible
//! Safe Rust implementation of fz_glyph

use super::{Handle, HandleStore};
use std::sync::LazyLock;

/// Glyph origin type
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct GlyphOrigin {
    pub x: f32,
    pub y: f32,
}

impl Default for GlyphOrigin {
    fn default() -> Self {
        Self { x: 0.0, y: 0.0 }
    }
}

/// Subpixel positioning mode
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubpixelMode {
    /// No subpixel positioning
    None = 0,
    /// Horizontal subpixel positioning only
    HorizontalOnly = 1,
    /// Full subpixel positioning (H+V)
    Full = 2,
}

/// Glyph hints for rendering
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlyphHints {
    /// No hinting
    NoHint = 0,
    /// Light hinting
    LightHint = 1,
    /// Normal hinting
    NormalHint = 2,
    /// Strong hinting
    StrongHint = 3,
}

/// Glyph metrics
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct GlyphMetrics {
    /// Horizontal advance width
    pub advance_width: f32,
    /// Vertical advance height (for vertical text)
    pub advance_height: f32,
    /// Left side bearing
    pub left_bearing: f32,
    /// Top bearing (for vertical text)
    pub top_bearing: f32,
    /// Glyph bounding box
    pub bbox: [f32; 4],
}

/// Color layer for color fonts (COLR/CPAL)
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ColorLayer {
    /// Glyph ID for this layer
    pub glyph_id: u32,
    /// Palette index for color
    pub palette_index: u16,
    /// Reserved for alignment
    pub _reserved: u16,
}

impl Default for ColorLayer {
    fn default() -> Self {
        Self {
            glyph_id: 0,
            palette_index: 0,
            _reserved: 0,
        }
    }
}

/// Glyph structure representing a single rendered glyph
#[derive(Debug, Clone)]
pub struct Glyph {
    /// Font handle this glyph belongs to
    pub font: Handle,
    /// Glyph ID within the font
    pub glyph_id: u32,
    /// Unicode codepoint (if known)
    pub unicode: u32,
    /// Position/origin
    pub origin: GlyphOrigin,
    /// Transformation matrix [a, b, c, d, e, f]
    pub matrix: [f32; 6],
    /// Metrics
    pub metrics: GlyphMetrics,
    /// Subpixel position (0-255 for fractional pixel)
    pub subpixel_x: u8,
    pub subpixel_y: u8,
    /// Hinting mode used
    pub hinting: GlyphHints,
    /// Is this a color glyph?
    pub is_color: bool,
    /// Color layers (for COLR fonts)
    pub color_layers: Vec<ColorLayer>,
    /// Cached rasterized bitmap (if any)
    pub bitmap: Option<GlyphBitmap>,
    /// Variation axis values (for variable fonts)
    pub variations: Vec<f32>,
}

impl Default for Glyph {
    fn default() -> Self {
        Self {
            font: 0,
            glyph_id: 0,
            unicode: 0,
            origin: GlyphOrigin::default(),
            matrix: [1.0, 0.0, 0.0, 1.0, 0.0, 0.0], // Identity
            metrics: GlyphMetrics::default(),
            subpixel_x: 0,
            subpixel_y: 0,
            hinting: GlyphHints::NormalHint,
            is_color: false,
            color_layers: Vec::new(),
            bitmap: None,
            variations: Vec::new(),
        }
    }
}

/// Rasterized glyph bitmap
#[derive(Debug, Clone)]
pub struct GlyphBitmap {
    /// Width in pixels
    pub width: i32,
    /// Height in pixels
    pub height: i32,
    /// X offset from origin
    pub x_offset: i32,
    /// Y offset from origin
    pub y_offset: i32,
    /// Bytes per row (stride)
    pub stride: i32,
    /// Number of components (1 for grayscale, 4 for BGRA color)
    pub n: i32,
    /// Pixel data
    pub data: Vec<u8>,
}

impl Default for GlyphBitmap {
    fn default() -> Self {
        Self {
            width: 0,
            height: 0,
            x_offset: 0,
            y_offset: 0,
            stride: 0,
            n: 1,
            data: Vec::new(),
        }
    }
}

/// Glyph cache for storing rendered glyphs
#[derive(Debug, Default)]
pub struct GlyphCache {
    /// Maximum number of cached glyphs
    pub max_entries: usize,
    /// Current entries (font_handle, glyph_id, scale_key) -> Glyph
    pub entries: std::collections::HashMap<(Handle, u32, u32), Handle>,
    /// LRU order for eviction
    pub lru: Vec<(Handle, u32, u32)>,
}

/// Global glyph storage
pub static GLYPHS: LazyLock<HandleStore<Glyph>> = LazyLock::new(HandleStore::new);

/// Global glyph cache
pub static GLYPH_CACHE: LazyLock<std::sync::Mutex<GlyphCache>> = LazyLock::new(|| {
    std::sync::Mutex::new(GlyphCache {
        max_entries: 1024,
        entries: std::collections::HashMap::new(),
        lru: Vec::new(),
    })
});

// ============================================================================
// Glyph Creation
// ============================================================================

/// Create a new glyph
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_glyph(_ctx: Handle, font: Handle, glyph_id: u32, unicode: u32) -> Handle {
    let glyph = Glyph {
        font,
        glyph_id,
        unicode,
        ..Default::default()
    };
    GLYPHS.insert(glyph)
}

/// Create glyph from font at specific position
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_glyph_at(
    _ctx: Handle,
    font: Handle,
    glyph_id: u32,
    x: f32,
    y: f32,
) -> Handle {
    let glyph = Glyph {
        font,
        glyph_id,
        origin: GlyphOrigin { x, y },
        ..Default::default()
    };
    GLYPHS.insert(glyph)
}

/// Create glyph with transformation matrix
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_glyph_with_matrix(
    _ctx: Handle,
    font: Handle,
    glyph_id: u32,
    matrix: *const f32,
) -> Handle {
    let mut glyph = Glyph {
        font,
        glyph_id,
        ..Default::default()
    };

    if !matrix.is_null() {
        let m = unsafe { std::slice::from_raw_parts(matrix, 6) };
        glyph.matrix.copy_from_slice(m);
    }

    GLYPHS.insert(glyph)
}

// ============================================================================
// Glyph Properties
// ============================================================================

/// Get glyph ID
#[unsafe(no_mangle)]
pub extern "C" fn fz_glyph_id(_ctx: Handle, glyph: Handle) -> u32 {
    if let Some(g) = GLYPHS.get(glyph) {
        if let Ok(guard) = g.lock() {
            return guard.glyph_id;
        }
    }
    0
}

/// Get glyph unicode
#[unsafe(no_mangle)]
pub extern "C" fn fz_glyph_unicode(_ctx: Handle, glyph: Handle) -> u32 {
    if let Some(g) = GLYPHS.get(glyph) {
        if let Ok(guard) = g.lock() {
            return guard.unicode;
        }
    }
    0
}

/// Get glyph font handle
#[unsafe(no_mangle)]
pub extern "C" fn fz_glyph_font(_ctx: Handle, glyph: Handle) -> Handle {
    if let Some(g) = GLYPHS.get(glyph) {
        if let Ok(guard) = g.lock() {
            return guard.font;
        }
    }
    0
}

/// Get glyph origin
#[unsafe(no_mangle)]
pub extern "C" fn fz_glyph_origin(_ctx: Handle, glyph: Handle, x: *mut f32, y: *mut f32) {
    if let Some(g) = GLYPHS.get(glyph) {
        if let Ok(guard) = g.lock() {
            if !x.is_null() {
                unsafe { *x = guard.origin.x };
            }
            if !y.is_null() {
                unsafe { *y = guard.origin.y };
            }
        }
    }
}

/// Set glyph origin
#[unsafe(no_mangle)]
pub extern "C" fn fz_set_glyph_origin(_ctx: Handle, glyph: Handle, x: f32, y: f32) {
    if let Some(g) = GLYPHS.get(glyph) {
        if let Ok(mut guard) = g.lock() {
            guard.origin.x = x;
            guard.origin.y = y;
        }
    }
}

/// Get glyph matrix
///
/// # Safety
/// `matrix` must point to at least 6 floats.
#[unsafe(no_mangle)]
pub extern "C" fn fz_glyph_matrix(_ctx: Handle, glyph: Handle, matrix: *mut f32) {
    if matrix.is_null() {
        return;
    }

    if let Some(g) = GLYPHS.get(glyph) {
        if let Ok(guard) = g.lock() {
            let m = unsafe { std::slice::from_raw_parts_mut(matrix, 6) };
            m.copy_from_slice(&guard.matrix);
        }
    }
}

/// Set glyph matrix
///
/// # Safety
/// `matrix` must point to at least 6 floats.
#[unsafe(no_mangle)]
pub extern "C" fn fz_set_glyph_matrix(_ctx: Handle, glyph: Handle, matrix: *const f32) {
    if matrix.is_null() {
        return;
    }

    if let Some(g) = GLYPHS.get(glyph) {
        if let Ok(mut guard) = g.lock() {
            let m = unsafe { std::slice::from_raw_parts(matrix, 6) };
            guard.matrix.copy_from_slice(m);
        }
    }
}

// ============================================================================
// Glyph Metrics
// ============================================================================

/// Get glyph advance width
#[unsafe(no_mangle)]
pub extern "C" fn fz_glyph_advance(_ctx: Handle, glyph: Handle, horizontal: i32) -> f32 {
    if let Some(g) = GLYPHS.get(glyph) {
        if let Ok(guard) = g.lock() {
            return if horizontal != 0 {
                guard.metrics.advance_width
            } else {
                guard.metrics.advance_height
            };
        }
    }
    0.0
}

/// Set glyph advance
#[unsafe(no_mangle)]
pub extern "C" fn fz_set_glyph_advance(
    _ctx: Handle,
    glyph: Handle,
    advance_width: f32,
    advance_height: f32,
) {
    if let Some(g) = GLYPHS.get(glyph) {
        if let Ok(mut guard) = g.lock() {
            guard.metrics.advance_width = advance_width;
            guard.metrics.advance_height = advance_height;
        }
    }
}

/// Get glyph bounding box
///
/// # Safety
/// `bbox` must point to at least 4 floats.
#[unsafe(no_mangle)]
pub extern "C" fn fz_glyph_bbox(_ctx: Handle, glyph: Handle, bbox: *mut f32) {
    if bbox.is_null() {
        return;
    }

    if let Some(g) = GLYPHS.get(glyph) {
        if let Ok(guard) = g.lock() {
            let b = unsafe { std::slice::from_raw_parts_mut(bbox, 4) };
            b.copy_from_slice(&guard.metrics.bbox);
        }
    }
}

/// Set glyph bounding box
#[unsafe(no_mangle)]
pub extern "C" fn fz_set_glyph_bbox(
    _ctx: Handle,
    glyph: Handle,
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
) {
    if let Some(g) = GLYPHS.get(glyph) {
        if let Ok(mut guard) = g.lock() {
            guard.metrics.bbox = [x0, y0, x1, y1];
        }
    }
}

/// Get full glyph metrics
///
/// # Safety
/// `metrics` must point to a valid GlyphMetrics struct.
#[unsafe(no_mangle)]
pub extern "C" fn fz_glyph_metrics(_ctx: Handle, glyph: Handle, metrics: *mut GlyphMetrics) {
    if metrics.is_null() {
        return;
    }

    if let Some(g) = GLYPHS.get(glyph) {
        if let Ok(guard) = g.lock() {
            unsafe { *metrics = guard.metrics };
        }
    }
}

// ============================================================================
// Subpixel Positioning
// ============================================================================

/// Get subpixel position
#[unsafe(no_mangle)]
pub extern "C" fn fz_glyph_subpixel(_ctx: Handle, glyph: Handle, x: *mut u8, y: *mut u8) {
    if let Some(g) = GLYPHS.get(glyph) {
        if let Ok(guard) = g.lock() {
            if !x.is_null() {
                unsafe { *x = guard.subpixel_x };
            }
            if !y.is_null() {
                unsafe { *y = guard.subpixel_y };
            }
        }
    }
}

/// Set subpixel position
#[unsafe(no_mangle)]
pub extern "C" fn fz_set_glyph_subpixel(_ctx: Handle, glyph: Handle, x: u8, y: u8) {
    if let Some(g) = GLYPHS.get(glyph) {
        if let Ok(mut guard) = g.lock() {
            guard.subpixel_x = x;
            guard.subpixel_y = y;
        }
    }
}

/// Calculate subpixel position from float coordinates
#[unsafe(no_mangle)]
pub extern "C" fn fz_subpixel_adjust(
    _ctx: Handle,
    x: *mut f32,
    y: *mut f32,
    subpixel_x: *mut u8,
    subpixel_y: *mut u8,
    mode: i32,
) {
    if x.is_null() || y.is_null() {
        return;
    }

    let px = unsafe { *x };
    let py = unsafe { *y };

    // Get integer part
    let ix = px.floor();
    let iy = py.floor();

    // Get fractional part (0-255)
    let fx = ((px - ix) * 256.0) as u8;
    let fy = ((py - iy) * 256.0) as u8;

    // Store adjusted integer position
    unsafe {
        *x = ix;
        *y = iy;
    }

    // Store subpixel based on mode
    match mode {
        0 => {
            // No subpixel
            if !subpixel_x.is_null() {
                unsafe { *subpixel_x = 0 };
            }
            if !subpixel_y.is_null() {
                unsafe { *subpixel_y = 0 };
            }
        }
        1 => {
            // Horizontal only
            if !subpixel_x.is_null() {
                unsafe { *subpixel_x = fx };
            }
            if !subpixel_y.is_null() {
                unsafe { *subpixel_y = 0 };
            }
        }
        _ => {
            // Full subpixel
            if !subpixel_x.is_null() {
                unsafe { *subpixel_x = fx };
            }
            if !subpixel_y.is_null() {
                unsafe { *subpixel_y = fy };
            }
        }
    }
}

// ============================================================================
// Color Font Support
// ============================================================================

/// Check if glyph is a color glyph
#[unsafe(no_mangle)]
pub extern "C" fn fz_glyph_is_color(_ctx: Handle, glyph: Handle) -> i32 {
    if let Some(g) = GLYPHS.get(glyph) {
        if let Ok(guard) = g.lock() {
            return i32::from(guard.is_color);
        }
    }
    0
}

/// Set glyph as color glyph
#[unsafe(no_mangle)]
pub extern "C" fn fz_set_glyph_color(_ctx: Handle, glyph: Handle, is_color: i32) {
    if let Some(g) = GLYPHS.get(glyph) {
        if let Ok(mut guard) = g.lock() {
            guard.is_color = is_color != 0;
        }
    }
}

/// Get number of color layers
#[unsafe(no_mangle)]
pub extern "C" fn fz_glyph_color_layer_count(_ctx: Handle, glyph: Handle) -> i32 {
    if let Some(g) = GLYPHS.get(glyph) {
        if let Ok(guard) = g.lock() {
            return guard.color_layers.len() as i32;
        }
    }
    0
}

/// Add a color layer to glyph
#[unsafe(no_mangle)]
pub extern "C" fn fz_glyph_add_color_layer(
    _ctx: Handle,
    glyph: Handle,
    layer_glyph_id: u32,
    palette_index: u16,
) -> i32 {
    if let Some(g) = GLYPHS.get(glyph) {
        if let Ok(mut guard) = g.lock() {
            guard.is_color = true;
            guard.color_layers.push(ColorLayer {
                glyph_id: layer_glyph_id,
                palette_index,
                _reserved: 0,
            });
            return (guard.color_layers.len() - 1) as i32;
        }
    }
    -1
}

/// Get color layer info
#[unsafe(no_mangle)]
pub extern "C" fn fz_glyph_color_layer(
    _ctx: Handle,
    glyph: Handle,
    idx: i32,
    layer_glyph_id: *mut u32,
    palette_index: *mut u16,
) -> i32 {
    if idx < 0 {
        return 0;
    }

    if let Some(g) = GLYPHS.get(glyph) {
        if let Ok(guard) = g.lock() {
            if let Some(layer) = guard.color_layers.get(idx as usize) {
                if !layer_glyph_id.is_null() {
                    unsafe { *layer_glyph_id = layer.glyph_id };
                }
                if !palette_index.is_null() {
                    unsafe { *palette_index = layer.palette_index };
                }
                return 1;
            }
        }
    }
    0
}

// ============================================================================
// Variable Font Support
// ============================================================================

/// Get number of variation axes
#[unsafe(no_mangle)]
pub extern "C" fn fz_glyph_variation_count(_ctx: Handle, glyph: Handle) -> i32 {
    if let Some(g) = GLYPHS.get(glyph) {
        if let Ok(guard) = g.lock() {
            return guard.variations.len() as i32;
        }
    }
    0
}

/// Set variation axis value
#[unsafe(no_mangle)]
pub extern "C" fn fz_set_glyph_variation(_ctx: Handle, glyph: Handle, axis_index: i32, value: f32) {
    if axis_index < 0 {
        return;
    }

    if let Some(g) = GLYPHS.get(glyph) {
        if let Ok(mut guard) = g.lock() {
            let idx = axis_index as usize;
            if idx >= guard.variations.len() {
                guard.variations.resize(idx + 1, 0.0);
            }
            guard.variations[idx] = value;
        }
    }
}

/// Get variation axis value
#[unsafe(no_mangle)]
pub extern "C" fn fz_glyph_variation(_ctx: Handle, glyph: Handle, axis_index: i32) -> f32 {
    if axis_index < 0 {
        return 0.0;
    }

    if let Some(g) = GLYPHS.get(glyph) {
        if let Ok(guard) = g.lock() {
            if let Some(v) = guard.variations.get(axis_index as usize) {
                return *v;
            }
        }
    }
    0.0
}

// ============================================================================
// Glyph Caching
// ============================================================================

/// Compute cache key for a glyph (combines font, glyph_id, and scale)
#[unsafe(no_mangle)]
pub extern "C" fn fz_glyph_cache_key(font: Handle, glyph_id: u32, scale: f32) -> u32 {
    // Simple key combining font handle, glyph id, and quantized scale
    let scale_key = (scale * 16.0) as u32 & 0xFFFF;
    let font_key = (font as u32) & 0xFFFF;
    (font_key << 16) | ((glyph_id & 0xFF) << 8) | (scale_key & 0xFF)
}

/// Look up glyph in cache
#[unsafe(no_mangle)]
pub extern "C" fn fz_glyph_cache_lookup(
    _ctx: Handle,
    font: Handle,
    glyph_id: u32,
    scale_key: u32,
) -> Handle {
    if let Ok(cache) = GLYPH_CACHE.lock() {
        if let Some(&handle) = cache.entries.get(&(font, glyph_id, scale_key)) {
            return handle;
        }
    }
    0
}

/// Insert glyph into cache
#[unsafe(no_mangle)]
pub extern "C" fn fz_glyph_cache_insert(
    _ctx: Handle,
    font: Handle,
    glyph_id: u32,
    scale_key: u32,
    glyph: Handle,
) {
    if let Ok(mut cache) = GLYPH_CACHE.lock() {
        let key = (font, glyph_id, scale_key);

        // Evict if at capacity
        while cache.entries.len() >= cache.max_entries && !cache.lru.is_empty() {
            let oldest = cache.lru.remove(0);
            if let Some(old_handle) = cache.entries.remove(&oldest) {
                GLYPHS.remove(old_handle);
            }
        }

        // Insert new entry
        cache.entries.insert(key, glyph);
        cache.lru.push(key);
    }
}

/// Clear glyph cache
#[unsafe(no_mangle)]
pub extern "C" fn fz_glyph_cache_clear(_ctx: Handle) {
    if let Ok(mut cache) = GLYPH_CACHE.lock() {
        for (_, handle) in cache.entries.drain() {
            GLYPHS.remove(handle);
        }
        cache.lru.clear();
    }
}

/// Set glyph cache size
#[unsafe(no_mangle)]
pub extern "C" fn fz_glyph_cache_set_size(_ctx: Handle, max_entries: i32) {
    if max_entries <= 0 {
        return;
    }

    if let Ok(mut cache) = GLYPH_CACHE.lock() {
        cache.max_entries = max_entries as usize;

        // Evict excess entries
        while cache.entries.len() > cache.max_entries && !cache.lru.is_empty() {
            let oldest = cache.lru.remove(0);
            if let Some(old_handle) = cache.entries.remove(&oldest) {
                GLYPHS.remove(old_handle);
            }
        }
    }
}

/// Get current cache size
#[unsafe(no_mangle)]
pub extern "C" fn fz_glyph_cache_size(_ctx: Handle) -> i32 {
    if let Ok(cache) = GLYPH_CACHE.lock() {
        return cache.entries.len() as i32;
    }
    0
}

// ============================================================================
// Hinting
// ============================================================================

/// Get hinting mode
#[unsafe(no_mangle)]
pub extern "C" fn fz_glyph_hinting(_ctx: Handle, glyph: Handle) -> i32 {
    if let Some(g) = GLYPHS.get(glyph) {
        if let Ok(guard) = g.lock() {
            return guard.hinting as i32;
        }
    }
    GlyphHints::NormalHint as i32
}

/// Set hinting mode
#[unsafe(no_mangle)]
pub extern "C" fn fz_set_glyph_hinting(_ctx: Handle, glyph: Handle, hinting: i32) {
    let h = match hinting {
        0 => GlyphHints::NoHint,
        1 => GlyphHints::LightHint,
        3 => GlyphHints::StrongHint,
        _ => GlyphHints::NormalHint,
    };

    if let Some(g) = GLYPHS.get(glyph) {
        if let Ok(mut guard) = g.lock() {
            guard.hinting = h;
        }
    }
}

// ============================================================================
// Reference Counting
// ============================================================================

/// Keep glyph reference
#[unsafe(no_mangle)]
pub extern "C" fn fz_keep_glyph(_ctx: Handle, glyph: Handle) -> Handle {
    GLYPHS.keep(glyph)
}

/// Drop glyph reference
#[unsafe(no_mangle)]
pub extern "C" fn fz_drop_glyph(_ctx: Handle, glyph: Handle) {
    GLYPHS.remove(glyph);
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_glyph() {
        let glyph = fz_new_glyph(0, 1, 65, 0x41); // Font 1, glyph 65, 'A'
        assert!(glyph > 0);

        assert_eq!(fz_glyph_id(0, glyph), 65);
        assert_eq!(fz_glyph_unicode(0, glyph), 0x41);
        assert_eq!(fz_glyph_font(0, glyph), 1);

        fz_drop_glyph(0, glyph);
    }

    #[test]
    fn test_glyph_origin() {
        let glyph = fz_new_glyph_at(0, 1, 65, 10.0, 20.0);

        let mut x = 0.0f32;
        let mut y = 0.0f32;
        fz_glyph_origin(0, glyph, &mut x, &mut y);

        assert_eq!(x, 10.0);
        assert_eq!(y, 20.0);

        fz_set_glyph_origin(0, glyph, 30.0, 40.0);
        fz_glyph_origin(0, glyph, &mut x, &mut y);

        assert_eq!(x, 30.0);
        assert_eq!(y, 40.0);

        fz_drop_glyph(0, glyph);
    }

    #[test]
    fn test_glyph_matrix() {
        let matrix = [2.0f32, 0.0, 0.0, 2.0, 100.0, 100.0];
        let glyph = fz_new_glyph_with_matrix(0, 1, 65, matrix.as_ptr());

        let mut out = [0.0f32; 6];
        fz_glyph_matrix(0, glyph, out.as_mut_ptr());

        assert_eq!(out, matrix);

        fz_drop_glyph(0, glyph);
    }

    #[test]
    fn test_glyph_advance() {
        let glyph = fz_new_glyph(0, 1, 65, 0x41);

        fz_set_glyph_advance(0, glyph, 600.0, 1000.0);

        assert_eq!(fz_glyph_advance(0, glyph, 1), 600.0); // Horizontal
        assert_eq!(fz_glyph_advance(0, glyph, 0), 1000.0); // Vertical

        fz_drop_glyph(0, glyph);
    }

    #[test]
    fn test_subpixel_adjust() {
        let mut x = 10.75f32;
        let mut y = 20.25f32;
        let mut sx = 0u8;
        let mut sy = 0u8;

        fz_subpixel_adjust(0, &mut x, &mut y, &mut sx, &mut sy, 2); // Full mode

        assert_eq!(x, 10.0);
        assert_eq!(y, 20.0);
        assert!(sx > 0); // Should have subpixel info
    }

    #[test]
    fn test_color_layers() {
        let glyph = fz_new_glyph(0, 1, 65, 0x41);

        assert_eq!(fz_glyph_is_color(0, glyph), 0);

        fz_glyph_add_color_layer(0, glyph, 100, 0);
        fz_glyph_add_color_layer(0, glyph, 101, 1);

        assert_eq!(fz_glyph_is_color(0, glyph), 1);
        assert_eq!(fz_glyph_color_layer_count(0, glyph), 2);

        let mut gid = 0u32;
        let mut pal = 0u16;
        fz_glyph_color_layer(0, glyph, 1, &mut gid, &mut pal);

        assert_eq!(gid, 101);
        assert_eq!(pal, 1);

        fz_drop_glyph(0, glyph);
    }

    #[test]
    fn test_variations() {
        let glyph = fz_new_glyph(0, 1, 65, 0x41);

        fz_set_glyph_variation(0, glyph, 0, 400.0); // Weight
        fz_set_glyph_variation(0, glyph, 1, 100.0); // Width

        assert_eq!(fz_glyph_variation_count(0, glyph), 2);
        assert_eq!(fz_glyph_variation(0, glyph, 0), 400.0);
        assert_eq!(fz_glyph_variation(0, glyph, 1), 100.0);

        fz_drop_glyph(0, glyph);
    }

    #[test]
    fn test_glyph_cache() {
        fz_glyph_cache_clear(0);

        let glyph1 = fz_new_glyph(0, 1, 65, 0x41);
        let glyph2 = fz_new_glyph(0, 1, 66, 0x42);

        fz_glyph_cache_insert(0, 1, 65, 100, glyph1);
        fz_glyph_cache_insert(0, 1, 66, 100, glyph2);

        assert_eq!(fz_glyph_cache_size(0), 2);

        let found = fz_glyph_cache_lookup(0, 1, 65, 100);
        assert_eq!(found, glyph1);

        fz_glyph_cache_clear(0);
        assert_eq!(fz_glyph_cache_size(0), 0);
    }

    #[test]
    fn test_hinting() {
        let glyph = fz_new_glyph(0, 1, 65, 0x41);

        // Default is normal
        assert_eq!(fz_glyph_hinting(0, glyph), GlyphHints::NormalHint as i32);

        fz_set_glyph_hinting(0, glyph, GlyphHints::NoHint as i32);
        assert_eq!(fz_glyph_hinting(0, glyph), GlyphHints::NoHint as i32);

        fz_drop_glyph(0, glyph);
    }
}
