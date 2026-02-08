// MicroPDF - MuPDF API Compatible C Header
// Auto-generated from Rust FFI - DO NOT EDIT MANUALLY
// Module: glyph

#ifndef MUPDF_FITZ_GLYPH_H
#define MUPDF_FITZ_GLYPH_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================================
// Glyph Functions (36 total)
// ============================================================================

void fz_drop_glyph(int32_t _ctx, int32_t glyph);
int32_t fz_glyph_add_color_layer(int32_t _ctx, int32_t glyph, uint32_t layer_glyph_id, u16 palette_index);
float fz_glyph_advance(int32_t _ctx, int32_t glyph, int32_t horizontal);
void fz_glyph_bbox(int32_t _ctx, int32_t glyph, float * bbox);
void fz_glyph_cache_clear(int32_t _ctx);
void fz_glyph_cache_insert(int32_t _ctx, int32_t font, uint32_t glyph_id, uint32_t scale_key, int32_t glyph);
uint32_t fz_glyph_cache_key(int32_t font, uint32_t glyph_id, float scale);
int32_t fz_glyph_cache_lookup(int32_t _ctx, int32_t font, uint32_t glyph_id, uint32_t scale_key);
void fz_glyph_cache_set_size(int32_t _ctx, int32_t max_entries);
int32_t fz_glyph_cache_size(int32_t _ctx);
int32_t fz_glyph_color_layer(int32_t _ctx, int32_t glyph, int32_t idx, uint32_t * layer_glyph_id, u16 * palette_index);
int32_t fz_glyph_color_layer_count(int32_t _ctx, int32_t glyph);
int32_t fz_glyph_font(int32_t _ctx, int32_t glyph);
int32_t fz_glyph_hinting(int32_t _ctx, int32_t glyph);
uint32_t fz_glyph_id(int32_t _ctx, int32_t glyph);
int32_t fz_glyph_is_color(int32_t _ctx, int32_t glyph);
void fz_glyph_matrix(int32_t _ctx, int32_t glyph, float * matrix);
void fz_glyph_metrics(int32_t _ctx, int32_t glyph, GlyphMetrics * metrics);
void fz_glyph_origin(int32_t _ctx, int32_t glyph, float * x, float * y);
void fz_glyph_subpixel(int32_t _ctx, int32_t glyph, u8 * x, u8 * y);
uint32_t fz_glyph_unicode(int32_t _ctx, int32_t glyph);
float fz_glyph_variation(int32_t _ctx, int32_t glyph, int32_t axis_index);
int32_t fz_glyph_variation_count(int32_t _ctx, int32_t glyph);
int32_t fz_keep_glyph(int32_t _ctx, int32_t glyph);
int32_t fz_new_glyph(int32_t _ctx, int32_t font, uint32_t glyph_id, uint32_t unicode);
int32_t fz_new_glyph_at(int32_t _ctx, int32_t font, uint32_t glyph_id, float x, float y);
int32_t fz_new_glyph_with_matrix(int32_t _ctx, int32_t font, uint32_t glyph_id, float const * matrix);
void fz_set_glyph_advance(int32_t _ctx, int32_t glyph, float advance_width, float advance_height);
void fz_set_glyph_bbox(int32_t _ctx, int32_t glyph, float x0, float y0, float x1, float y1);
void fz_set_glyph_color(int32_t _ctx, int32_t glyph, int32_t is_color);
void fz_set_glyph_hinting(int32_t _ctx, int32_t glyph, int32_t hinting);
void fz_set_glyph_matrix(int32_t _ctx, int32_t glyph, float const * matrix);
void fz_set_glyph_origin(int32_t _ctx, int32_t glyph, float x, float y);
void fz_set_glyph_subpixel(int32_t _ctx, int32_t glyph, u8 x, u8 y);
void fz_set_glyph_variation(int32_t _ctx, int32_t glyph, int32_t axis_index, float value);
void fz_subpixel_adjust(int32_t _ctx, float * x, float * y, u8 * subpixel_x, u8 * subpixel_y, int32_t mode);

#ifdef __cplusplus
}
#endif

#endif /* MUPDF_FITZ_GLYPH_H */
