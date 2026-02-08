// MicroPDF - MuPDF API Compatible C Header
// Auto-generated from Rust FFI - DO NOT EDIT MANUALLY
// Module: pixmap

#ifndef MUPDF_FITZ_PIXMAP_H
#define MUPDF_FITZ_PIXMAP_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================================
// Pixmap Functions (33 total)
// ============================================================================

void fz_clear_pixmap(int32_t _ctx, int32_t pix);
void fz_clear_pixmap_with_value(int32_t _ctx, int32_t pix, int32_t value);
int32_t fz_clone_pixmap(int32_t _ctx, int32_t pix);
int32_t fz_convert_pixmap(int32_t _ctx, int32_t pix, int32_t cs, int32_t _prf, int32_t // Color profile (not implemented) _default_cs, int32_t _color_params, int32_t keep_alpha);
void fz_drop_pixmap(int32_t _ctx, int32_t pix);
void fz_gamma_pixmap(int32_t _ctx, int32_t pix, float gamma);
u8 fz_get_pixmap_sample(int32_t _ctx, int32_t pix, int32_t x, int32_t y, int32_t n);
void fz_invert_pixmap(int32_t _ctx, int32_t pix);
int32_t fz_keep_pixmap(int32_t _ctx, int32_t pix);
int32_t fz_new_pixmap(int32_t _ctx, int32_t cs, int32_t w, int32_t h, int32_t _seps, int32_t // Separations not implemented yet alpha);
int32_t fz_new_pixmap_with_bbox(int32_t _ctx, int32_t cs, fz_irect bbox, int32_t _seps, int32_t alpha);
int32_t fz_pixmap_alpha(int32_t _ctx, int32_t pix);
fz_irect fz_pixmap_bbox(int32_t _ctx, int32_t pix);
int32_t fz_pixmap_colorants(int32_t _ctx, int32_t pix);
int32_t fz_pixmap_colorspace(int32_t _ctx, int32_t pix);
int32_t fz_pixmap_components(int32_t _ctx, int32_t pix);
int32_t fz_pixmap_height(int32_t _ctx, int32_t pix);
int32_t fz_pixmap_is_valid(int32_t _ctx, int32_t pix);
void fz_pixmap_resolution(int32_t _ctx, int32_t _pix, int32_t * xres, int32_t * yres);
u8 * fz_pixmap_samples(int32_t _ctx, int32_t pix);
size_t fz_pixmap_samples_size(int32_t _ctx, int32_t pix);
int32_t fz_pixmap_stride(int32_t _ctx, int32_t pix);
int32_t fz_pixmap_width(int32_t _ctx, int32_t pix);
int32_t fz_pixmap_x(int32_t _ctx, int32_t pix);
int32_t fz_pixmap_xres(int32_t _ctx, int32_t _pix);
int32_t fz_pixmap_y(int32_t _ctx, int32_t pix);
int32_t fz_pixmap_yres(int32_t _ctx, int32_t _pix);
int32_t fz_scale_pixmap(int32_t _ctx, int32_t pix, float xscale, float yscale);
void fz_set_pixmap_resolution(int32_t _ctx, int32_t _pix, int32_t _xres, int32_t _yres);
void fz_set_pixmap_sample(int32_t _ctx, int32_t pix, int32_t x, int32_t y, int32_t n, u8 v);
void fz_set_pixmap_xres(int32_t _ctx, int32_t _pix, int32_t _xres);
void fz_set_pixmap_yres(int32_t _ctx, int32_t _pix, int32_t _yres);
void fz_tint_pixmap(int32_t _ctx, int32_t pix, int32_t r, int32_t g, int32_t b);

#ifdef __cplusplus
}
#endif

#endif /* MUPDF_FITZ_PIXMAP_H */
