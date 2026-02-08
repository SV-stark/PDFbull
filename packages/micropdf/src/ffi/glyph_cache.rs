//! FFI bindings for fz_glyph_cache (Glyph Cache Management)
//!
//! This module provides glyph caching with statistics, eviction policies,
//! and subpixel positioning support.

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};
use std::time::Instant;

use crate::ffi::colorspace::FZ_COLORSPACE_GRAY;
use crate::ffi::glyph::{GLYPHS, Glyph};
use crate::ffi::{Handle, PIXMAPS};
use crate::fitz::geometry::{IRect, Matrix};

// ============================================================================
// Cache Configuration
// ============================================================================

/// Default maximum cache size in bytes
pub const DEFAULT_CACHE_SIZE: usize = 8 * 1024 * 1024; // 8 MB

/// Default maximum number of cached glyphs
pub const DEFAULT_MAX_GLYPHS: usize = 4096;

/// Subpixel quantization levels
pub const SUBPIXEL_LEVELS: u8 = 4;

/// Cache eviction policy
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CacheEvictionPolicy {
    /// Least Recently Used
    #[default]
    Lru = 0,
    /// Least Frequently Used
    Lfu = 1,
    /// First In First Out
    Fifo = 2,
    /// Random eviction
    Random = 3,
}

impl CacheEvictionPolicy {
    pub fn from_i32(value: i32) -> Self {
        match value {
            0 => CacheEvictionPolicy::Lru,
            1 => CacheEvictionPolicy::Lfu,
            2 => CacheEvictionPolicy::Fifo,
            3 => CacheEvictionPolicy::Random,
            _ => CacheEvictionPolicy::Lru,
        }
    }
}

// ============================================================================
// Cache Entry
// ============================================================================

/// Cache key for a glyph
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct GlyphCacheKey {
    /// Font handle
    pub font: Handle,
    /// Glyph ID
    pub glyph_id: u32,
    /// Quantized matrix components (scaled to integers for hashing)
    pub matrix_key: [i32; 6],
    /// Subpixel position X (0-3)
    pub subpix_x: u8,
    /// Subpixel position Y (0-3)
    pub subpix_y: u8,
    /// Antialiasing level
    pub aa_level: u8,
}

impl GlyphCacheKey {
    pub fn new(
        font: Handle,
        glyph_id: u32,
        matrix: &Matrix,
        subpix_x: u8,
        subpix_y: u8,
        aa_level: u8,
    ) -> Self {
        // Quantize matrix to integer components for reliable hashing
        let scale = 256.0;
        Self {
            font,
            glyph_id,
            matrix_key: [
                (matrix.a * scale) as i32,
                (matrix.b * scale) as i32,
                (matrix.c * scale) as i32,
                (matrix.d * scale) as i32,
                (matrix.e * scale) as i32,
                (matrix.f * scale) as i32,
            ],
            subpix_x: subpix_x % SUBPIXEL_LEVELS,
            subpix_y: subpix_y % SUBPIXEL_LEVELS,
            aa_level,
        }
    }
}

/// Cached glyph entry
#[derive(Debug, Clone)]
pub struct GlyphCacheEntry {
    /// Handle to the cached pixmap
    pub pixmap: Handle,
    /// Size in bytes
    pub size: usize,
    /// Creation time
    pub created: Instant,
    /// Last access time
    pub last_access: Instant,
    /// Access count
    pub access_count: u64,
    /// Original glyph handle (if stored)
    pub glyph: Option<Handle>,
}

// ============================================================================
// Cache Statistics
// ============================================================================

/// Cache statistics
#[repr(C)]
#[derive(Debug, Clone, Default)]
pub struct GlyphCacheStats {
    /// Total cache hits
    pub hits: u64,
    /// Total cache misses
    pub misses: u64,
    /// Number of evictions
    pub evictions: u64,
    /// Current number of cached glyphs
    pub glyph_count: usize,
    /// Current memory usage in bytes
    pub memory_usage: usize,
    /// Maximum memory limit
    pub max_memory: usize,
    /// Maximum glyph count limit
    pub max_glyphs: usize,
    /// Number of cache purges
    pub purge_count: u64,
}

impl GlyphCacheStats {
    /// Calculate hit rate (0.0 - 1.0)
    pub fn hit_rate(&self) -> f32 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f32 / total as f32
        }
    }

    /// Calculate memory utilization (0.0 - 1.0)
    pub fn memory_utilization(&self) -> f32 {
        if self.max_memory == 0 {
            0.0
        } else {
            self.memory_usage as f32 / self.max_memory as f32
        }
    }
}

// ============================================================================
// Glyph Cache
// ============================================================================

/// Global glyph cache
pub struct GlyphCache {
    /// Cached entries
    entries: HashMap<GlyphCacheKey, GlyphCacheEntry>,
    /// Statistics
    stats: GlyphCacheStats,
    /// Eviction policy
    policy: CacheEvictionPolicy,
    /// Insertion order for FIFO
    insertion_order: Vec<GlyphCacheKey>,
}

impl Default for GlyphCache {
    fn default() -> Self {
        Self {
            entries: HashMap::new(),
            stats: GlyphCacheStats {
                max_memory: DEFAULT_CACHE_SIZE,
                max_glyphs: DEFAULT_MAX_GLYPHS,
                ..Default::default()
            },
            policy: CacheEvictionPolicy::Lru,
            insertion_order: Vec::new(),
        }
    }
}

impl GlyphCache {
    /// Look up a glyph in the cache
    pub fn get(&mut self, key: &GlyphCacheKey) -> Option<Handle> {
        if let Some(entry) = self.entries.get_mut(key) {
            entry.last_access = Instant::now();
            entry.access_count += 1;
            self.stats.hits += 1;
            Some(entry.pixmap)
        } else {
            self.stats.misses += 1;
            None
        }
    }

    /// Insert a glyph into the cache
    pub fn insert(
        &mut self,
        key: GlyphCacheKey,
        pixmap: Handle,
        size: usize,
        glyph: Option<Handle>,
    ) {
        // Evict if necessary
        while self.stats.memory_usage + size > self.stats.max_memory
            || self.stats.glyph_count >= self.stats.max_glyphs
        {
            if !self.evict_one() {
                break; // No more entries to evict
            }
        }

        let entry = GlyphCacheEntry {
            pixmap,
            size,
            created: Instant::now(),
            last_access: Instant::now(),
            access_count: 1,
            glyph,
        };

        self.stats.memory_usage += size;
        self.stats.glyph_count += 1;
        self.insertion_order.push(key.clone());
        self.entries.insert(key, entry);
    }

    /// Evict one entry based on policy
    fn evict_one(&mut self) -> bool {
        let key_to_evict = match self.policy {
            CacheEvictionPolicy::Lru => self.find_lru_key(),
            CacheEvictionPolicy::Lfu => self.find_lfu_key(),
            CacheEvictionPolicy::Fifo => self.find_fifo_key(),
            CacheEvictionPolicy::Random => self.find_random_key(),
        };

        if let Some(key) = key_to_evict {
            self.remove(&key);
            self.stats.evictions += 1;
            true
        } else {
            false
        }
    }

    fn find_lru_key(&self) -> Option<GlyphCacheKey> {
        self.entries
            .iter()
            .min_by_key(|(_, v)| v.last_access)
            .map(|(k, _)| k.clone())
    }

    fn find_lfu_key(&self) -> Option<GlyphCacheKey> {
        self.entries
            .iter()
            .min_by_key(|(_, v)| v.access_count)
            .map(|(k, _)| k.clone())
    }

    fn find_fifo_key(&mut self) -> Option<GlyphCacheKey> {
        while let Some(key) = self.insertion_order.first().cloned() {
            if self.entries.contains_key(&key) {
                return Some(key);
            }
            self.insertion_order.remove(0);
        }
        None
    }

    fn find_random_key(&self) -> Option<GlyphCacheKey> {
        self.entries.keys().next().cloned()
    }

    /// Remove an entry
    pub fn remove(&mut self, key: &GlyphCacheKey) -> Option<GlyphCacheEntry> {
        if let Some(entry) = self.entries.remove(key) {
            self.stats.memory_usage = self.stats.memory_usage.saturating_sub(entry.size);
            self.stats.glyph_count = self.stats.glyph_count.saturating_sub(1);
            self.insertion_order.retain(|k| k != key);
            Some(entry)
        } else {
            None
        }
    }

    /// Clear all entries
    pub fn purge(&mut self) {
        self.entries.clear();
        self.insertion_order.clear();
        self.stats.memory_usage = 0;
        self.stats.glyph_count = 0;
        self.stats.purge_count += 1;
    }

    /// Get statistics
    pub fn stats(&self) -> &GlyphCacheStats {
        &self.stats
    }

    /// Set maximum memory
    pub fn set_max_memory(&mut self, max: usize) {
        self.stats.max_memory = max;
    }

    /// Set maximum glyphs
    pub fn set_max_glyphs(&mut self, max: usize) {
        self.stats.max_glyphs = max;
    }

    /// Set eviction policy
    pub fn set_policy(&mut self, policy: CacheEvictionPolicy) {
        self.policy = policy;
    }
}

/// Global glyph cache instance
pub static GLYPH_CACHE: LazyLock<Mutex<GlyphCache>> =
    LazyLock::new(|| Mutex::new(GlyphCache::default()));

// ============================================================================
// FFI Functions - Cache Management
// ============================================================================

/// Purge all glyphs from the cache
#[unsafe(no_mangle)]
pub extern "C" fn fz_purge_glyph_cache(_ctx: Handle) {
    if let Ok(mut cache) = GLYPH_CACHE.lock() {
        cache.purge();
    }
}

/// Get cache hit count
#[unsafe(no_mangle)]
pub extern "C" fn fz_glyph_cache_hits(_ctx: Handle) -> u64 {
    if let Ok(cache) = GLYPH_CACHE.lock() {
        cache.stats.hits
    } else {
        0
    }
}

/// Get cache miss count
#[unsafe(no_mangle)]
pub extern "C" fn fz_glyph_cache_misses(_ctx: Handle) -> u64 {
    if let Ok(cache) = GLYPH_CACHE.lock() {
        cache.stats.misses
    } else {
        0
    }
}

/// Get cache hit rate (0.0 - 1.0)
#[unsafe(no_mangle)]
pub extern "C" fn fz_glyph_cache_hit_rate(_ctx: Handle) -> f32 {
    if let Ok(cache) = GLYPH_CACHE.lock() {
        cache.stats.hit_rate()
    } else {
        0.0
    }
}

/// Get number of cached glyphs
#[unsafe(no_mangle)]
pub extern "C" fn fz_glyph_cache_count(_ctx: Handle) -> usize {
    if let Ok(cache) = GLYPH_CACHE.lock() {
        cache.stats.glyph_count
    } else {
        0
    }
}

/// Get cache memory usage in bytes
#[unsafe(no_mangle)]
pub extern "C" fn fz_glyph_cache_memory_size(_ctx: Handle) -> usize {
    if let Ok(cache) = GLYPH_CACHE.lock() {
        cache.stats.memory_usage
    } else {
        0
    }
}

/// Get maximum cache memory limit
#[unsafe(no_mangle)]
pub extern "C" fn fz_glyph_cache_max_size(_ctx: Handle) -> usize {
    if let Ok(cache) = GLYPH_CACHE.lock() {
        cache.stats.max_memory
    } else {
        0
    }
}

/// Set maximum cache memory limit
#[unsafe(no_mangle)]
pub extern "C" fn fz_set_glyph_cache_max_size(_ctx: Handle, max: usize) {
    if let Ok(mut cache) = GLYPH_CACHE.lock() {
        cache.set_max_memory(max);
    }
}

/// Get maximum cached glyph count
#[unsafe(no_mangle)]
pub extern "C" fn fz_glyph_cache_max_count(_ctx: Handle) -> usize {
    if let Ok(cache) = GLYPH_CACHE.lock() {
        cache.stats.max_glyphs
    } else {
        0
    }
}

/// Set maximum cached glyph count
#[unsafe(no_mangle)]
pub extern "C" fn fz_set_glyph_cache_max_count(_ctx: Handle, max: usize) {
    if let Ok(mut cache) = GLYPH_CACHE.lock() {
        cache.set_max_glyphs(max);
    }
}

/// Get number of evictions
#[unsafe(no_mangle)]
pub extern "C" fn fz_glyph_cache_evictions(_ctx: Handle) -> u64 {
    if let Ok(cache) = GLYPH_CACHE.lock() {
        cache.stats.evictions
    } else {
        0
    }
}

/// Get number of purges
#[unsafe(no_mangle)]
pub extern "C" fn fz_glyph_cache_purges(_ctx: Handle) -> u64 {
    if let Ok(cache) = GLYPH_CACHE.lock() {
        cache.stats.purge_count
    } else {
        0
    }
}

/// Get cache eviction policy
#[unsafe(no_mangle)]
pub extern "C" fn fz_glyph_cache_policy(_ctx: Handle) -> i32 {
    if let Ok(cache) = GLYPH_CACHE.lock() {
        cache.policy as i32
    } else {
        0
    }
}

/// Set cache eviction policy
#[unsafe(no_mangle)]
pub extern "C" fn fz_set_glyph_cache_policy(_ctx: Handle, policy: i32) {
    if let Ok(mut cache) = GLYPH_CACHE.lock() {
        cache.set_policy(CacheEvictionPolicy::from_i32(policy));
    }
}

/// Get memory utilization (0.0 - 1.0)
#[unsafe(no_mangle)]
pub extern "C" fn fz_glyph_cache_utilization(_ctx: Handle) -> f32 {
    if let Ok(cache) = GLYPH_CACHE.lock() {
        cache.stats.memory_utilization()
    } else {
        0.0
    }
}

// ============================================================================
// FFI Functions - Glyph Rendering
// ============================================================================

/// Render a glyph to a pixmap (cached)
#[unsafe(no_mangle)]
pub extern "C" fn fz_render_glyph_pixmap(
    _ctx: Handle,
    font: Handle,
    gid: i32,
    ctm: *const Matrix,
    scissor: *const IRect,
    aa: i32,
) -> Handle {
    if ctm.is_null() {
        return 0;
    }

    let matrix = unsafe { &*ctm };
    let aa_level = aa.clamp(0, 8) as u8;

    // Create cache key
    let key = GlyphCacheKey::new(font, gid as u32, matrix, 0, 0, aa_level);

    // Check cache
    if let Ok(mut cache) = GLYPH_CACHE.lock() {
        if let Some(pixmap) = cache.get(&key) {
            return pixmap;
        }
    }

    // Render glyph (create a simple pixmap for the glyph)
    let pixmap = render_glyph_internal(font, gid as u32, matrix, scissor, aa_level);
    if pixmap == 0 {
        return 0;
    }

    // Get pixmap size for cache
    let size = if let Some(pix) = PIXMAPS.get(pixmap) {
        let guard = pix.lock().unwrap();
        (guard.w() * guard.h() * guard.n()) as usize
    } else {
        1024 // Default estimate
    };

    // Insert into cache
    if let Ok(mut cache) = GLYPH_CACHE.lock() {
        cache.insert(key, pixmap, size, None);
    }

    pixmap
}

/// Internal glyph rendering function
fn render_glyph_internal(
    font: Handle,
    _gid: u32,
    matrix: &Matrix,
    scissor: *const IRect,
    aa: u8,
) -> Handle {
    // Get glyph metrics
    let glyph: Glyph = if let Some(g) = GLYPHS.get(font) {
        g.lock().unwrap().clone()
    } else {
        // Create a default glyph for rendering
        Glyph::default()
    };

    // Calculate bounding box
    let scale = (matrix.a * matrix.a + matrix.b * matrix.b).sqrt();
    let width = ((glyph.metrics.bbox[2] - glyph.metrics.bbox[0]) * scale).ceil() as i32;
    let height = ((glyph.metrics.bbox[3] - glyph.metrics.bbox[1]) * scale).ceil() as i32;

    let width = width.max(1).min(4096);
    let height = height.max(1).min(4096);

    // Apply scissor if provided
    let (final_width, final_height) = if !scissor.is_null() {
        let s = unsafe { &*scissor };
        let sw = (s.x1 - s.x0).min(width);
        let sh = (s.y1 - s.y0).min(height);
        (sw.max(1), sh.max(1))
    } else {
        (width, height)
    };

    // Create pixmap for glyph (grayscale with alpha for AA)
    let alpha = aa > 0;
    let pixmap =
        crate::ffi::pixmap::Pixmap::new(FZ_COLORSPACE_GRAY, final_width, final_height, alpha);
    let handle = PIXMAPS.insert(pixmap);

    // Fill with rendered glyph data (simplified - real implementation would rasterize)
    if let Some(pix) = PIXMAPS.get(handle) {
        let mut guard = pix.lock().unwrap();
        // Simple placeholder rendering - in reality would call font rasterizer
        let n = guard.n() as usize;
        let data = guard.samples_mut();
        for pixel in data.chunks_mut(n) {
            pixel[0] = 255; // Full coverage
            if n > 1 {
                pixel[1] = 255; // Alpha
            }
        }
    }

    handle
}

/// Perform subpixel quantization and adjustment for cache key generation
/// Note: Use fz_subpixel_adjust in glyph.rs for the main API
pub fn subpixel_adjust_internal(matrix: &mut Matrix) -> (u8, u8) {
    // Quantize translation components for subpixel positioning
    let e_frac = matrix.e.fract();
    let f_frac = matrix.f.fract();

    // Quantize to subpixel levels (0-3 typically)
    let qe_val = ((e_frac * SUBPIXEL_LEVELS as f32) as u8) % SUBPIXEL_LEVELS;
    let qf_val = ((f_frac * SUBPIXEL_LEVELS as f32) as u8) % SUBPIXEL_LEVELS;

    // Adjust matrix to quantized position
    matrix.e = matrix.e.floor() + (qe_val as f32 / SUBPIXEL_LEVELS as f32);
    matrix.f = matrix.f.floor() + (qf_val as f32 / SUBPIXEL_LEVELS as f32);

    (qe_val, qf_val)
}

/// Prepare a Type 3 glyph for caching
#[unsafe(no_mangle)]
pub extern "C" fn fz_prepare_t3_glyph(_ctx: Handle, _font: Handle, _gid: i32) {
    // Type 3 glyphs are rendered from display lists
    // This would pre-cache the display list for the glyph
    // For now, this is a no-op placeholder
}

/// Render Type 3 glyph directly to device
#[unsafe(no_mangle)]
pub extern "C" fn fz_render_t3_glyph_direct(
    _ctx: Handle,
    _dev: Handle,
    _font: Handle,
    _gid: i32,
    _trm: Matrix,
    _gstate: *mut std::ffi::c_void,
    _def_cs: Handle,
    _fill_gstate: *mut std::ffi::c_void,
    _stroke_gstate: *mut std::ffi::c_void,
) {
    // Type 3 glyphs contain their own drawing operations
    // This would execute the glyph's display list on the device
    // Placeholder for now
}

/// Dump glyph cache statistics
#[unsafe(no_mangle)]
pub extern "C" fn fz_dump_glyph_cache_stats(_ctx: Handle, _out: Handle) {
    if let Ok(cache) = GLYPH_CACHE.lock() {
        let stats = cache.stats();
        eprintln!("Glyph Cache Statistics:");
        eprintln!("  Hits: {}", stats.hits);
        eprintln!("  Misses: {}", stats.misses);
        eprintln!("  Hit Rate: {:.2}%", stats.hit_rate() * 100.0);
        eprintln!("  Glyphs: {} / {}", stats.glyph_count, stats.max_glyphs);
        eprintln!(
            "  Memory: {} / {} bytes ({:.1}%)",
            stats.memory_usage,
            stats.max_memory,
            stats.memory_utilization() * 100.0
        );
        eprintln!("  Evictions: {}", stats.evictions);
        eprintln!("  Purges: {}", stats.purge_count);
    }
}

// ============================================================================
// FFI Functions - Cache Lookup
// ============================================================================

/// Look up a cached rendered glyph by key components
#[unsafe(no_mangle)]
pub extern "C" fn fz_rendered_glyph_cache_lookup(
    _ctx: Handle,
    font: Handle,
    gid: i32,
    ctm: *const Matrix,
    subpix_x: u8,
    subpix_y: u8,
    aa: i32,
) -> Handle {
    if ctm.is_null() {
        return 0;
    }

    let matrix = unsafe { &*ctm };
    let key = GlyphCacheKey::new(font, gid as u32, matrix, subpix_x, subpix_y, aa as u8);

    if let Ok(mut cache) = GLYPH_CACHE.lock() {
        cache.get(&key).unwrap_or(0)
    } else {
        0
    }
}

/// Insert a rendered glyph pixmap into cache
#[unsafe(no_mangle)]
pub extern "C" fn fz_rendered_glyph_cache_insert(
    _ctx: Handle,
    font: Handle,
    gid: i32,
    ctm: *const Matrix,
    subpix_x: u8,
    subpix_y: u8,
    aa: i32,
    pixmap: Handle,
    size: usize,
) {
    if ctm.is_null() || pixmap == 0 {
        return;
    }

    let matrix = unsafe { &*ctm };
    let key = GlyphCacheKey::new(font, gid as u32, matrix, subpix_x, subpix_y, aa as u8);

    if let Ok(mut cache) = GLYPH_CACHE.lock() {
        cache.insert(key, pixmap, size, None);
    }
}

/// Remove a specific rendered glyph from cache
#[unsafe(no_mangle)]
pub extern "C" fn fz_rendered_glyph_cache_remove(
    _ctx: Handle,
    font: Handle,
    gid: i32,
    ctm: *const Matrix,
    subpix_x: u8,
    subpix_y: u8,
    aa: i32,
) -> i32 {
    if ctm.is_null() {
        return 0;
    }

    let matrix = unsafe { &*ctm };
    let key = GlyphCacheKey::new(font, gid as u32, matrix, subpix_x, subpix_y, aa as u8);

    if let Ok(mut cache) = GLYPH_CACHE.lock() {
        if cache.remove(&key).is_some() {
            return 1;
        }
    }
    0
}

/// Remove all cached rendered glyphs for a specific font
#[unsafe(no_mangle)]
pub extern "C" fn fz_rendered_glyph_cache_purge_font(_ctx: Handle, font: Handle) -> usize {
    let mut removed = 0;

    if let Ok(mut cache) = GLYPH_CACHE.lock() {
        let keys_to_remove: Vec<_> = cache
            .entries
            .keys()
            .filter(|k| k.font == font)
            .cloned()
            .collect();

        for key in keys_to_remove {
            cache.remove(&key);
            removed += 1;
        }
    }

    removed
}

/// Reset cache statistics
#[unsafe(no_mangle)]
pub extern "C" fn fz_glyph_cache_reset_stats(_ctx: Handle) {
    if let Ok(mut cache) = GLYPH_CACHE.lock() {
        cache.stats.hits = 0;
        cache.stats.misses = 0;
        cache.stats.evictions = 0;
        // Don't reset purge_count as it's historical
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_eviction_policy() {
        assert_eq!(CacheEvictionPolicy::from_i32(0), CacheEvictionPolicy::Lru);
        assert_eq!(CacheEvictionPolicy::from_i32(1), CacheEvictionPolicy::Lfu);
        assert_eq!(CacheEvictionPolicy::from_i32(2), CacheEvictionPolicy::Fifo);
        assert_eq!(
            CacheEvictionPolicy::from_i32(3),
            CacheEvictionPolicy::Random
        );
        assert_eq!(CacheEvictionPolicy::from_i32(99), CacheEvictionPolicy::Lru);
    }

    #[test]
    fn test_glyph_cache_key() {
        let m1 = Matrix {
            a: 1.0,
            b: 0.0,
            c: 0.0,
            d: 1.0,
            e: 10.0,
            f: 20.0,
        };
        let m2 = Matrix {
            a: 1.0,
            b: 0.0,
            c: 0.0,
            d: 1.0,
            e: 10.0,
            f: 20.0,
        };

        let key1 = GlyphCacheKey::new(1, 65, &m1, 0, 0, 4);
        let key2 = GlyphCacheKey::new(1, 65, &m2, 0, 0, 4);

        assert_eq!(key1, key2);
    }

    #[test]
    fn test_glyph_cache_key_different_subpixel() {
        let m = Matrix {
            a: 1.0,
            b: 0.0,
            c: 0.0,
            d: 1.0,
            e: 10.0,
            f: 20.0,
        };

        let key1 = GlyphCacheKey::new(1, 65, &m, 0, 0, 4);
        let key2 = GlyphCacheKey::new(1, 65, &m, 1, 0, 4);

        assert_ne!(key1, key2);
    }

    #[test]
    fn test_cache_stats() {
        let stats = GlyphCacheStats {
            hits: 80,
            misses: 20,
            max_memory: 1000,
            memory_usage: 500,
            ..Default::default()
        };

        assert!((stats.hit_rate() - 0.8).abs() < 0.01);
        assert!((stats.memory_utilization() - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_cache_stats_empty() {
        let stats = GlyphCacheStats::default();
        assert_eq!(stats.hit_rate(), 0.0);
        assert_eq!(stats.memory_utilization(), 0.0);
    }

    #[test]
    fn test_cache_basic_operations() {
        let mut cache = GlyphCache::default();
        let matrix = Matrix {
            a: 1.0,
            b: 0.0,
            c: 0.0,
            d: 1.0,
            e: 0.0,
            f: 0.0,
        };

        let key = GlyphCacheKey::new(1, 65, &matrix, 0, 0, 4);

        // Initial lookup should miss
        assert!(cache.get(&key).is_none());
        assert_eq!(cache.stats.misses, 1);

        // Insert
        cache.insert(key.clone(), 100, 1024, None);
        assert_eq!(cache.stats.glyph_count, 1);
        assert_eq!(cache.stats.memory_usage, 1024);

        // Lookup should hit
        assert_eq!(cache.get(&key), Some(100));
        assert_eq!(cache.stats.hits, 1);
    }

    #[test]
    fn test_cache_eviction() {
        let mut cache = GlyphCache::default();
        cache.set_max_memory(2000);
        cache.set_max_glyphs(2);

        let matrix = Matrix {
            a: 1.0,
            b: 0.0,
            c: 0.0,
            d: 1.0,
            e: 0.0,
            f: 0.0,
        };

        let key1 = GlyphCacheKey::new(1, 65, &matrix, 0, 0, 4);
        let key2 = GlyphCacheKey::new(1, 66, &matrix, 0, 0, 4);
        let key3 = GlyphCacheKey::new(1, 67, &matrix, 0, 0, 4);

        cache.insert(key1.clone(), 100, 1000, None);
        cache.insert(key2.clone(), 200, 1000, None);

        assert_eq!(cache.stats.glyph_count, 2);

        // This should trigger eviction
        cache.insert(key3.clone(), 300, 1000, None);

        assert_eq!(cache.stats.glyph_count, 2);
        assert!(cache.stats.evictions >= 1);
    }

    #[test]
    fn test_cache_purge() {
        let mut cache = GlyphCache::default();
        let matrix = Matrix {
            a: 1.0,
            b: 0.0,
            c: 0.0,
            d: 1.0,
            e: 0.0,
            f: 0.0,
        };

        for i in 0..10 {
            let key = GlyphCacheKey::new(1, i, &matrix, 0, 0, 4);
            cache.insert(key, i as Handle, 100, None);
        }

        assert_eq!(cache.stats.glyph_count, 10);

        cache.purge();

        assert_eq!(cache.stats.glyph_count, 0);
        assert_eq!(cache.stats.memory_usage, 0);
        assert_eq!(cache.stats.purge_count, 1);
    }

    #[test]
    #[cfg_attr(tarpaulin, ignore)] // Global state conflicts under coverage instrumentation
    fn test_ffi_cache_stats() {
        let ctx = 1;

        // Purge first to get clean state
        fz_purge_glyph_cache(ctx);
        fz_glyph_cache_reset_stats(ctx);

        assert_eq!(fz_glyph_cache_count(ctx), 0);
        assert_eq!(fz_glyph_cache_hits(ctx), 0);
        assert_eq!(fz_glyph_cache_misses(ctx), 0);
    }

    #[test]
    fn test_ffi_cache_limits() {
        let ctx = 1;

        let original_size = fz_glyph_cache_max_size(ctx);
        let original_count = fz_glyph_cache_max_count(ctx);

        fz_set_glyph_cache_max_size(ctx, 1024 * 1024);
        fz_set_glyph_cache_max_count(ctx, 100);

        assert_eq!(fz_glyph_cache_max_size(ctx), 1024 * 1024);
        assert_eq!(fz_glyph_cache_max_count(ctx), 100);

        // Restore
        fz_set_glyph_cache_max_size(ctx, original_size);
        fz_set_glyph_cache_max_count(ctx, original_count);
    }

    #[test]
    fn test_ffi_cache_policy() {
        let ctx = 1;

        let original = fz_glyph_cache_policy(ctx);

        fz_set_glyph_cache_policy(ctx, CacheEvictionPolicy::Lfu as i32);
        assert_eq!(fz_glyph_cache_policy(ctx), CacheEvictionPolicy::Lfu as i32);

        fz_set_glyph_cache_policy(ctx, CacheEvictionPolicy::Fifo as i32);
        assert_eq!(fz_glyph_cache_policy(ctx), CacheEvictionPolicy::Fifo as i32);

        // Restore
        fz_set_glyph_cache_policy(ctx, original);
    }

    #[test]
    fn test_subpixel_adjust() {
        let mut ctm = Matrix {
            a: 12.0,
            b: 0.0,
            c: 0.0,
            d: 12.0,
            e: 100.25,
            f: 200.75,
        };

        let (qe, qf) = subpixel_adjust_internal(&mut ctm);

        assert!(qe < SUBPIXEL_LEVELS);
        assert!(qf < SUBPIXEL_LEVELS);
    }

    #[test]
    fn test_cache_lookup_insert() {
        let ctx = 1;

        fz_purge_glyph_cache(ctx);

        let ctm = Matrix {
            a: 12.0,
            b: 0.0,
            c: 0.0,
            d: 12.0,
            e: 0.0,
            f: 0.0,
        };

        // Lookup should miss
        let result = fz_rendered_glyph_cache_lookup(ctx, 1, 65, &ctm, 0, 0, 4);
        assert_eq!(result, 0);

        // Insert
        fz_rendered_glyph_cache_insert(ctx, 1, 65, &ctm, 0, 0, 4, 999, 1024);

        // Lookup should hit
        let result = fz_rendered_glyph_cache_lookup(ctx, 1, 65, &ctm, 0, 0, 4);
        assert_eq!(result, 999);

        fz_purge_glyph_cache(ctx);
    }

    #[test]
    #[cfg_attr(tarpaulin, ignore)] // Global state conflicts under coverage instrumentation
    fn test_cache_remove() {
        let ctx = 1;

        fz_purge_glyph_cache(ctx);

        let ctm = Matrix {
            a: 12.0,
            b: 0.0,
            c: 0.0,
            d: 12.0,
            e: 0.0,
            f: 0.0,
        };

        fz_rendered_glyph_cache_insert(ctx, 1, 65, &ctm, 0, 0, 4, 888, 1024);

        let removed = fz_rendered_glyph_cache_remove(ctx, 1, 65, &ctm, 0, 0, 4);
        assert_eq!(removed, 1);

        let result = fz_rendered_glyph_cache_lookup(ctx, 1, 65, &ctm, 0, 0, 4);
        assert_eq!(result, 0);

        fz_purge_glyph_cache(ctx);
    }

    #[test]
    #[cfg_attr(tarpaulin, ignore)] // Global state conflicts under coverage instrumentation
    fn test_purge_font() {
        let ctx = 1;

        fz_purge_glyph_cache(ctx);

        let ctm = Matrix {
            a: 12.0,
            b: 0.0,
            c: 0.0,
            d: 12.0,
            e: 0.0,
            f: 0.0,
        };

        // Insert glyphs for font 1
        fz_rendered_glyph_cache_insert(ctx, 1, 65, &ctm, 0, 0, 4, 100, 100);
        fz_rendered_glyph_cache_insert(ctx, 1, 66, &ctm, 0, 0, 4, 101, 100);

        // Insert glyph for font 2
        fz_rendered_glyph_cache_insert(ctx, 2, 65, &ctm, 0, 0, 4, 200, 100);

        assert_eq!(fz_glyph_cache_count(ctx), 3);

        // Purge font 1
        let removed = fz_rendered_glyph_cache_purge_font(ctx, 1);
        assert_eq!(removed, 2);
        assert_eq!(fz_glyph_cache_count(ctx), 1);

        // Font 2 glyph should still be there
        let result = fz_rendered_glyph_cache_lookup(ctx, 2, 65, &ctm, 0, 0, 4);
        assert_eq!(result, 200);

        fz_purge_glyph_cache(ctx);
    }
}
