// MicroPDF - MuPDF compatible glyph cache API
// Auto-generated header file

#ifndef MICROPDF_GLYPH_CACHE_H
#define MICROPDF_GLYPH_CACHE_H

#include <stdint.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================================
// Types
// ============================================================================

/// Handle type (opaque pointer)
typedef uint64_t fz_handle;
typedef fz_handle fz_context;
typedef fz_handle fz_font;
typedef fz_handle fz_pixmap;
typedef fz_handle fz_output;

/// Matrix structure
typedef struct {
    float a, b, c, d, e, f;
} fz_matrix;

/// Integer rectangle
typedef struct {
    int32_t x0, y0, x1, y1;
} fz_irect;

/// Cache eviction policy
typedef enum {
    FZ_CACHE_LRU = 0,      // Least Recently Used (default)
    FZ_CACHE_LFU = 1,      // Least Frequently Used
    FZ_CACHE_FIFO = 2,     // First In First Out
    FZ_CACHE_RANDOM = 3    // Random eviction
} fz_cache_eviction_policy;

// ============================================================================
// Cache Management Functions
// ============================================================================

/// Purge all glyphs from the cache
void fz_purge_glyph_cache(fz_context *ctx);

/// Get cache hit count
uint64_t fz_glyph_cache_hits(fz_context *ctx);

/// Get cache miss count
uint64_t fz_glyph_cache_misses(fz_context *ctx);

/// Get cache hit rate (0.0 - 1.0)
float fz_glyph_cache_hit_rate(fz_context *ctx);

/// Get number of cached glyphs
size_t fz_glyph_cache_count(fz_context *ctx);

/// Get cache memory usage in bytes
size_t fz_glyph_cache_memory_size(fz_context *ctx);

/// Get maximum cache memory limit
size_t fz_glyph_cache_max_size(fz_context *ctx);

/// Set maximum cache memory limit
void fz_set_glyph_cache_max_size(fz_context *ctx, size_t max);

/// Get maximum cached glyph count
size_t fz_glyph_cache_max_count(fz_context *ctx);

/// Set maximum cached glyph count
void fz_set_glyph_cache_max_count(fz_context *ctx, size_t max);

/// Get number of cache evictions
uint64_t fz_glyph_cache_evictions(fz_context *ctx);

/// Get number of cache purges
uint64_t fz_glyph_cache_purges(fz_context *ctx);

/// Get cache eviction policy
int32_t fz_glyph_cache_policy(fz_context *ctx);

/// Set cache eviction policy
void fz_set_glyph_cache_policy(fz_context *ctx, int32_t policy);

/// Get memory utilization (0.0 - 1.0)
float fz_glyph_cache_utilization(fz_context *ctx);

/// Reset cache statistics (hits, misses, evictions)
void fz_glyph_cache_reset_stats(fz_context *ctx);

// ============================================================================
// Glyph Rendering Functions
// ============================================================================

/// Render a glyph to a pixmap (cached)
///
/// font: Font handle
/// gid: Glyph ID
/// ctm: Transformation matrix
/// scissor: Clipping rectangle (can be NULL)
/// aa: Antialiasing level (0-8)
///
/// Returns: Handle to rendered pixmap, or 0 on error
fz_pixmap *fz_render_glyph_pixmap(fz_context *ctx, fz_font *font, int32_t gid,
                                   fz_matrix *ctm, const fz_irect *scissor, int32_t aa);

/// Prepare a Type 3 glyph for caching
void fz_prepare_t3_glyph(fz_context *ctx, fz_font *font, int32_t gid);

/// Render Type 3 glyph directly to device
void fz_render_t3_glyph_direct(fz_context *ctx, fz_handle dev, fz_font *font,
                                int32_t gid, fz_matrix trm, void *gstate,
                                fz_handle def_cs, void *fill_gstate, void *stroke_gstate);

/// Dump glyph cache statistics to output
void fz_dump_glyph_cache_stats(fz_context *ctx, fz_output *out);

// ============================================================================
// Rendered Glyph Cache Functions
// ============================================================================

/// Look up a cached rendered glyph by key components
///
/// Returns: Pixmap handle if found, 0 if not cached
fz_handle fz_rendered_glyph_cache_lookup(fz_context *ctx, fz_handle font, int32_t gid,
                                          const fz_matrix *ctm, unsigned char subpix_x,
                                          unsigned char subpix_y, int32_t aa);

/// Insert a rendered glyph pixmap into cache
void fz_rendered_glyph_cache_insert(fz_context *ctx, fz_handle font, int32_t gid,
                                     const fz_matrix *ctm, unsigned char subpix_x,
                                     unsigned char subpix_y, int32_t aa,
                                     fz_handle pixmap, size_t size);

/// Remove a specific rendered glyph from cache
///
/// Returns: 1 if removed, 0 if not found
int32_t fz_rendered_glyph_cache_remove(fz_context *ctx, fz_handle font, int32_t gid,
                                        const fz_matrix *ctm, unsigned char subpix_x,
                                        unsigned char subpix_y, int32_t aa);

/// Remove all cached rendered glyphs for a specific font
///
/// Returns: Number of glyphs removed
size_t fz_rendered_glyph_cache_purge_font(fz_context *ctx, fz_handle font);

#ifdef __cplusplus
}
#endif

#endif // MICROPDF_GLYPH_CACHE_H

