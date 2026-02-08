// MicroPDF - MuPDF API Compatible C Header
// Auto-generated from Rust FFI - DO NOT EDIT MANUALLY
// Module: image

#ifndef MUPDF_FITZ_IMAGE_H
#define MUPDF_FITZ_IMAGE_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================================
// Image Functions (23 total)
// ============================================================================

int32_t fz_clone_image(int32_t _ctx, int32_t image);
int32_t fz_decode_image(int32_t _ctx, int32_t image, int32_t _l2factor, fz_irect const * _subarea);
int32_t fz_decode_image_scaled(int32_t _ctx, int32_t image, int32_t w, int32_t h, int32_t _l2factor, fz_irect const * _subarea);
void fz_drop_image(int32_t _ctx, int32_t image);
int32_t fz_get_pixmap_from_image(int32_t _ctx, int32_t image, fz_irect const * _subarea, fz_matrix * _ctm, int32_t * w, int32_t * h);
int32_t fz_image_bpp(int32_t _ctx, int32_t _image);
int32_t fz_image_colorspace(int32_t _ctx, int32_t image);
int32_t fz_image_h(int32_t _ctx, int32_t image);
int32_t fz_image_has_alpha(int32_t _ctx, int32_t _image);
int32_t fz_image_height(int32_t _ctx, int32_t image);
int32_t fz_image_is_mask(int32_t _ctx, int32_t image);
int32_t fz_image_is_valid(int32_t _ctx, int32_t image);
int32_t fz_image_orientation(int32_t _ctx, int32_t _image);
int32_t fz_image_w(int32_t _ctx, int32_t image);
int32_t fz_image_width(int32_t _ctx, int32_t image);
int32_t fz_image_xres(int32_t _ctx, int32_t image);
int32_t fz_image_yres(int32_t _ctx, int32_t image);
int32_t fz_keep_image(int32_t _ctx, int32_t image);
int32_t fz_new_image_from_buffer(int32_t _ctx, int32_t buffer);
int32_t fz_new_image_from_buffer_data(int32_t _ctx, u8 const * data, size_t len);
int32_t fz_new_image_from_data(int32_t _ctx, int32_t w, int32_t h, int32_t _bpc, int32_t _colorspace, int32_t _xres, int32_t _yres, int32_t _interpolate, int32_t _imagemask, float const * _decode, u8 const * _mask, u8 const * data, int32_t len);
int32_t fz_new_image_from_file(int32_t _ctx, const char * filename);
int32_t fz_new_image_from_pixmap(int32_t _ctx, int32_t pixmap, int32_t _mask);

#ifdef __cplusplus
}
#endif

#endif /* MUPDF_FITZ_IMAGE_H */
