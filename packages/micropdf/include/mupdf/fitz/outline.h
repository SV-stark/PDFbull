// MicroPDF - MuPDF API Compatible C Header
// Auto-generated from Rust FFI - DO NOT EDIT MANUALLY
// Module: outline

#ifndef MUPDF_FITZ_OUTLINE_H
#define MUPDF_FITZ_OUTLINE_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================================
// Outline Functions (38 total)
// ============================================================================

void fz_drop_outline(int32_t _ctx, int32_t outline);
void fz_drop_outline_iterator(int32_t _ctx, int32_t iter);
int32_t fz_keep_outline(int32_t _ctx, int32_t outline);
int32_t fz_load_outline_from_iterator(int32_t _ctx, int32_t iter);
int32_t fz_new_outline(int32_t _ctx);
int32_t fz_new_outline_iterator(int32_t _ctx);
float fz_outline_color_b(int32_t _ctx, int32_t outline);
float fz_outline_color_g(int32_t _ctx, int32_t outline);
float fz_outline_color_r(int32_t _ctx, int32_t outline);
int32_t fz_outline_count(int32_t _ctx, int32_t outline);
int32_t fz_outline_depth(int32_t _ctx, int32_t iter);
int32_t fz_outline_down(int32_t _ctx, int32_t outline);
int32_t fz_outline_flags(int32_t _ctx, int32_t outline);
int32_t fz_outline_is_open(int32_t _ctx, int32_t outline);
int32_t fz_outline_iterator_delete(int32_t _ctx, int32_t iter);
int32_t fz_outline_iterator_down(int32_t _ctx, int32_t iter);
int32_t fz_outline_iterator_from_outline(int32_t _ctx, int32_t outline);
int32_t fz_outline_iterator_insert(int32_t _ctx, int32_t iter, FzOutlineItem const * item);
FzOutlineItem const * fz_outline_iterator_item(int32_t _ctx, int32_t iter);
int32_t fz_outline_iterator_next(int32_t _ctx, int32_t iter);
int32_t fz_outline_iterator_prev(int32_t _ctx, int32_t iter);
int32_t fz_outline_iterator_up(int32_t _ctx, int32_t iter);
void fz_outline_iterator_update(int32_t _ctx, int32_t iter, FzOutlineItem const * item);
int32_t fz_outline_next(int32_t _ctx, int32_t outline);
int32_t fz_outline_page(int32_t _ctx, int32_t outline);
const char * fz_outline_title(int32_t _ctx, int32_t outline);
const char * fz_outline_uri(int32_t _ctx, int32_t outline);
float fz_outline_x(int32_t _ctx, int32_t outline);
float fz_outline_y(int32_t _ctx, int32_t outline);
void fz_set_outline_color(int32_t _ctx, int32_t outline, float r, float g, float b);
void fz_set_outline_down(int32_t _ctx, int32_t outline, int32_t down);
void fz_set_outline_flags(int32_t _ctx, int32_t outline, int32_t flags);
void fz_set_outline_is_open(int32_t _ctx, int32_t outline, int32_t is_open);
void fz_set_outline_next(int32_t _ctx, int32_t outline, int32_t next);
void fz_set_outline_page(int32_t _ctx, int32_t outline, int32_t chapter, int32_t page);
void fz_set_outline_title(int32_t _ctx, int32_t outline, const char * title);
void fz_set_outline_uri(int32_t _ctx, int32_t outline, const char * uri);
void fz_set_outline_xy(int32_t _ctx, int32_t outline, float x, float y);

#ifdef __cplusplus
}
#endif

#endif /* MUPDF_FITZ_OUTLINE_H */
