// MicroPDF - MuPDF API Compatible C Header
// Auto-generated from Rust FFI - DO NOT EDIT MANUALLY
// Module: glyph_cache

#ifndef MUPDF_FITZ_GLYPH_CACHE_H
#define MUPDF_FITZ_GLYPH_CACHE_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================================
// Glyph_cache Functions (24 total)
// ============================================================================

void fz_dump_glyph_cache_stats(int32_t _ctx, int32_t _out);
size_t fz_glyph_cache_count(int32_t _ctx);
uint64_t fz_glyph_cache_evictions(int32_t _ctx);
float fz_glyph_cache_hit_rate(int32_t _ctx);
uint64_t fz_glyph_cache_hits(int32_t _ctx);
size_t fz_glyph_cache_max_count(int32_t _ctx);
size_t fz_glyph_cache_max_size(int32_t _ctx);
size_t fz_glyph_cache_memory_size(int32_t _ctx);
uint64_t fz_glyph_cache_misses(int32_t _ctx);
int32_t fz_glyph_cache_policy(int32_t _ctx);
uint64_t fz_glyph_cache_purges(int32_t _ctx);
void fz_glyph_cache_reset_stats(int32_t _ctx);
float fz_glyph_cache_utilization(int32_t _ctx);
void fz_prepare_t3_glyph(int32_t _ctx, int32_t _font, int32_t _gid);
void fz_purge_glyph_cache(int32_t _ctx);
int32_t fz_render_glyph_pixmap(int32_t _ctx, int32_t font, int32_t gid, Matrix const * ctm, IRect const * scissor, int32_t aa);
void fz_render_t3_glyph_direct(int32_t _ctx, int32_t _dev, int32_t _font, int32_t _gid, Matrix _trm, c_void * _gstate, int32_t _def_cs, c_void * _fill_gstate, c_void * _stroke_gstate);
void fz_rendered_glyph_cache_insert(int32_t _ctx, int32_t font, int32_t gid, Matrix const * ctm, u8 subpix_x, u8 subpix_y, int32_t aa, int32_t pixmap, size_t size);
int32_t fz_rendered_glyph_cache_lookup(int32_t _ctx, int32_t font, int32_t gid, Matrix const * ctm, u8 subpix_x, u8 subpix_y, int32_t aa);
size_t fz_rendered_glyph_cache_purge_font(int32_t _ctx, int32_t font);
int32_t fz_rendered_glyph_cache_remove(int32_t _ctx, int32_t font, int32_t gid, Matrix const * ctm, u8 subpix_x, u8 subpix_y, int32_t aa);
void fz_set_glyph_cache_max_count(int32_t _ctx, size_t max);
void fz_set_glyph_cache_max_size(int32_t _ctx, size_t max);
void fz_set_glyph_cache_policy(int32_t _ctx, int32_t policy);

#ifdef __cplusplus
}
#endif

#endif /* MUPDF_FITZ_GLYPH_CACHE_H */
