// MicroPDF - MuPDF API Compatible C Header
// Auto-generated from Rust FFI - DO NOT EDIT MANUALLY
// Module: draw_device

#ifndef MUPDF_FITZ_DRAW_DEVICE_H
#define MUPDF_FITZ_DRAW_DEVICE_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================================
// Draw_device Functions (40 total)
// ============================================================================

void fz_draw_device_begin_mask(int32_t _ctx, int32_t device, float const * _mask_area, int32_t _luminosity);
void fz_draw_device_begin_path(int32_t _ctx, int32_t device);
void fz_draw_device_begin_pattern(int32_t _ctx, int32_t device, int32_t _pattern_handle, float const * _area);
void fz_draw_device_begin_text(int32_t _ctx, int32_t device);
void fz_draw_device_clip(int32_t _ctx, int32_t device, int32_t rule);
int32_t fz_draw_device_clip_depth(int32_t _ctx, int32_t device);
void fz_draw_device_close_path(int32_t _ctx, int32_t device);
void fz_draw_device_concat_ctm(int32_t _ctx, int32_t device, float const * matrix);
void fz_draw_device_curve_to(int32_t _ctx, int32_t device, float x1, float y1, float x2, float y2, float x3, float y3);
int32_t fz_draw_device_draw_glyph(int32_t _ctx, int32_t device, int32_t _font, uint32_t _glyph_id, float _x, float _y, float _size);
void fz_draw_device_enable_overprint(int32_t _ctx, int32_t device, int32_t enable);
void fz_draw_device_enable_subpixel_text(int32_t _ctx, int32_t device, int32_t enable);
void fz_draw_device_end_mask(int32_t _ctx, int32_t device);
void fz_draw_device_end_pattern(int32_t _ctx, int32_t device);
void fz_draw_device_end_text(int32_t _ctx, int32_t device);
int32_t fz_draw_device_fill(int32_t _ctx, int32_t device, int32_t rule);
void fz_draw_device_line_to(int32_t _ctx, int32_t device, float x, float y);
void fz_draw_device_move_to(int32_t _ctx, int32_t device, float x, float y);
void fz_draw_device_pop_clip(int32_t _ctx, int32_t device);
void fz_draw_device_restore(int32_t _ctx, int32_t device);
void fz_draw_device_save(int32_t _ctx, int32_t device);
void fz_draw_device_set_aa_level(int32_t _ctx, int32_t device, int32_t level);
void fz_draw_device_set_alpha(int32_t _ctx, int32_t device, float alpha);
void fz_draw_device_set_blend_mode(int32_t _ctx, int32_t device, int32_t mode);
void fz_draw_device_set_ctm(int32_t _ctx, int32_t device, float const * matrix);
void fz_draw_device_set_dash(int32_t _ctx, int32_t device, float const * dash_array, int32_t dash_count, float dash_phase);
void fz_draw_device_set_fill_color(int32_t _ctx, int32_t device, float r, float g, float b, float a);
void fz_draw_device_set_line_cap(int32_t _ctx, int32_t device, int32_t cap);
void fz_draw_device_set_line_join(int32_t _ctx, int32_t device, int32_t join);
void fz_draw_device_set_line_width(int32_t _ctx, int32_t device, float width);
void fz_draw_device_set_miter_limit(int32_t _ctx, int32_t device, float limit);
void fz_draw_device_set_overprint(int32_t _ctx, int32_t device, int32_t mode);
void fz_draw_device_set_stroke_color(int32_t _ctx, int32_t device, float r, float g, float b, float a);
int32_t fz_draw_device_stroke(int32_t _ctx, int32_t device);
int32_t fz_draw_device_target(int32_t _ctx, int32_t device);
void fz_drop_draw_device(int32_t _ctx, int32_t device);
int32_t fz_keep_draw_device(int32_t _ctx, int32_t device);
int32_t fz_new_draw_device_with_matrix(int32_t _ctx, int32_t target_pixmap, float const * matrix);
int32_t fz_new_draw_device_with_options(int32_t _ctx, int32_t target_pixmap, int32_t aa_level, int32_t subpixel_text);
int32_t fz_new_draw_device_with_size(int32_t _ctx, int32_t target_pixmap, int32_t width, int32_t height);

#ifdef __cplusplus
}
#endif

#endif /* MUPDF_FITZ_DRAW_DEVICE_H */
