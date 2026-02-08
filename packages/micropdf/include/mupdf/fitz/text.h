// MicroPDF - MuPDF API Compatible C Header
// Auto-generated from Rust FFI - DO NOT EDIT MANUALLY
// Module: text

#ifndef MUPDF_FITZ_TEXT_H
#define MUPDF_FITZ_TEXT_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================================
// Text Functions (15 total)
// ============================================================================

fz_rect fz_bound_text(int32_t _ctx, int32_t text, int32_t stroke, fz_matrix transform);
void fz_clear_text(int32_t _ctx, int32_t text);
int32_t fz_clone_text(int32_t _ctx, int32_t text);
void fz_drop_text(int32_t _ctx, int32_t text);
int32_t fz_keep_text(int32_t _ctx, int32_t text);
int32_t fz_new_text(int32_t _ctx);
void fz_set_text_language(int32_t _ctx, int32_t text, const char * lang);
void fz_show_glyph(int32_t _ctx, int32_t text, int32_t font, fz_matrix transform, int32_t glyph, int32_t unicode, int32_t wmode);
void fz_show_string(int32_t _ctx, int32_t text, int32_t font, fz_matrix transform, const char * string, int32_t wmode);
int32_t fz_text_count_items(int32_t _ctx, int32_t text);
int32_t fz_text_count_spans(int32_t _ctx, int32_t text);
int32_t fz_text_is_empty(int32_t _ctx, int32_t text);
int32_t fz_text_is_valid(int32_t _ctx, int32_t text);
int32_t fz_text_language(int32_t _ctx, int32_t text, c_char * buf, int32_t len);
int32_t fz_text_walk(int32_t _ctx, int32_t text, void const * callback, c_void * arg);

#ifdef __cplusplus
}
#endif

#endif /* MUPDF_FITZ_TEXT_H */
