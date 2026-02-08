// MicroPDF - MuPDF API Compatible C Header
// Auto-generated from Rust FFI - DO NOT EDIT MANUALLY
// Module: font

#ifndef MUPDF_FITZ_FONT_H
#define MUPDF_FITZ_FONT_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================================
// Font Functions (22 total)
// ============================================================================

float fz_advance_glyph(int32_t _ctx, int32_t font, int32_t glyph, int32_t _wmode);
fz_rect fz_bound_glyph(int32_t _ctx, int32_t font, int32_t glyph, fz_matrix _transform);
int32_t fz_clone_font(int32_t _ctx, int32_t font);
void fz_drop_font(int32_t _ctx, int32_t font);
int32_t fz_encode_character(int32_t _ctx, int32_t font, int32_t unicode);
int32_t fz_encode_character_with_fallback(int32_t _ctx, int32_t font, int32_t unicode, int32_t _script, int32_t _language, int32_t * out_font);
float fz_font_ascender(int32_t _ctx, int32_t font);
fz_rect fz_font_bbox(int32_t _ctx, int32_t font);
float fz_font_descender(int32_t _ctx, int32_t font);
int32_t fz_font_is_bold(int32_t _ctx, int32_t font);
int32_t fz_font_is_embedded(int32_t _ctx, int32_t _font);
int32_t fz_font_is_italic(int32_t _ctx, int32_t font);
int32_t fz_font_is_monospaced(int32_t _ctx, int32_t font);
int32_t fz_font_is_serif(int32_t _ctx, int32_t font);
int32_t fz_font_is_valid(int32_t _ctx, int32_t font);
void fz_font_name(int32_t _ctx, int32_t font, c_char * buf, int32_t size);
void fz_glyph_name(int32_t _ctx, int32_t _font, int32_t glyph, c_char * buf, int32_t size);
int32_t fz_keep_font(int32_t _ctx, int32_t font);
int32_t fz_new_font(int32_t _ctx, const char * name, int32_t _is_bold, int32_t _is_italic, int32_t _font_file);
int32_t fz_new_font_from_file(int32_t _ctx, const char * name, const char * path, int32_t index, int32_t _use_glyph_bbox);
int32_t fz_new_font_from_memory(int32_t _ctx, const char * name, u8 const * data, int32_t len, int32_t index, int32_t _use_glyph_bbox);
int32_t fz_outline_glyph(int32_t _ctx, int32_t font, int32_t glyph, fz_matrix _transform);

#ifdef __cplusplus
}
#endif

#endif /* MUPDF_FITZ_FONT_H */
