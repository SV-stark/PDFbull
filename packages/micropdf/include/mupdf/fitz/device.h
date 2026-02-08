// MicroPDF - MuPDF API Compatible C Header
// Auto-generated from Rust FFI - DO NOT EDIT MANUALLY
// Module: device

#ifndef MUPDF_FITZ_DEVICE_H
#define MUPDF_FITZ_DEVICE_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================================
// Device Functions (30 total)
// ============================================================================

void fz_begin_group(int32_t _ctx, int32_t dev, fz_rect area, int32_t colorspace, int32_t isolated, int32_t knockout, int32_t blendmode, float alpha);
void fz_begin_mask(int32_t _ctx, int32_t dev, fz_rect area, int32_t luminosity, int32_t colorspace, float const * color);
int32_t fz_begin_tile(int32_t _ctx, int32_t dev, fz_rect area, fz_rect view, float xstep, float ystep, fz_matrix transform);
void fz_clip_image_mask(int32_t _ctx, int32_t dev, int32_t image, fz_matrix transform);
void fz_clip_path(int32_t _ctx, int32_t dev, int32_t path, int32_t even_odd, fz_matrix transform);
void fz_clip_stroke_path(int32_t _ctx, int32_t dev, int32_t path, int32_t stroke, fz_matrix transform);
void fz_clip_stroke_text(int32_t _ctx, int32_t dev, int32_t text, int32_t stroke, fz_matrix transform);
void fz_clip_text(int32_t _ctx, int32_t dev, int32_t text, fz_matrix transform);
void fz_close_device(int32_t _ctx, int32_t dev);
int32_t fz_device_is_valid(int32_t _ctx, int32_t dev);
const char * fz_device_type(int32_t _ctx, int32_t dev);
void fz_disable_device_hints(int32_t _ctx, int32_t dev, int32_t hints);
void fz_drop_device(int32_t _ctx, int32_t dev);
void fz_enable_device_hints(int32_t _ctx, int32_t dev, int32_t hints);
void fz_end_group(int32_t _ctx, int32_t dev);
void fz_end_mask(int32_t _ctx, int32_t dev);
void fz_end_tile(int32_t _ctx, int32_t dev);
void fz_fill_image(int32_t _ctx, int32_t dev, int32_t image, fz_matrix transform, float alpha);
void fz_fill_image_mask(int32_t _ctx, int32_t dev, int32_t image, fz_matrix transform, int32_t colorspace, float const * color, float alpha);
void fz_fill_path(int32_t _ctx, int32_t dev, int32_t path, int32_t even_odd, fz_matrix transform, int32_t colorspace, float const * color, float alpha);
void fz_fill_text(int32_t _ctx, int32_t dev, int32_t text, fz_matrix transform, int32_t colorspace, float const * color, float alpha);
void fz_ignore_text(int32_t _ctx, int32_t dev, int32_t text, fz_matrix transform);
int32_t fz_keep_device(int32_t _ctx, int32_t dev);
int32_t fz_new_bbox_device(int32_t _ctx, fz_rect * rect);
int32_t fz_new_draw_device(int32_t _ctx, fz_matrix _transform, int32_t pixmap);
int32_t fz_new_list_device(int32_t _ctx, int32_t _list);
int32_t fz_new_trace_device(int32_t _ctx);
void fz_pop_clip(int32_t _ctx, int32_t dev);
void fz_stroke_path(int32_t _ctx, int32_t dev, int32_t path, int32_t stroke, fz_matrix transform, int32_t colorspace, float const * color, float alpha);
void fz_stroke_text(int32_t _ctx, int32_t dev, int32_t text, int32_t stroke, fz_matrix transform, int32_t colorspace, float const * color, float alpha);

#ifdef __cplusplus
}
#endif

#endif /* MUPDF_FITZ_DEVICE_H */
