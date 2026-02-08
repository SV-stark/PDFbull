// MicroPDF - MuPDF API Compatible C Header
// Auto-generated from Rust FFI - DO NOT EDIT MANUALLY
// Module: shade

#ifndef MUPDF_FITZ_SHADE_H
#define MUPDF_FITZ_SHADE_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================================
// Shade Functions (22 total)
// ============================================================================

void fz_drop_shade(int32_t _ctx, int32_t shade);
int32_t fz_keep_shade(int32_t _ctx, int32_t shade);
int32_t fz_new_function_shade(int32_t _ctx, uint64_t colorspace, float const * domain, float const * matrix);
int32_t fz_new_linear_shade(int32_t _ctx, uint64_t colorspace, float x0, float y0, float x1, float y1, int32_t extend_start, int32_t extend_end);
int32_t fz_new_mesh_shade(int32_t _ctx, uint64_t colorspace, int32_t shade_type, int32_t bits_per_coord, int32_t bits_per_comp, int32_t bits_per_flag);
int32_t fz_new_radial_shade(int32_t _ctx, uint64_t colorspace, float x0, float y0, float r0, float x1, float y1, float r1, int32_t extend_start, int32_t extend_end);
int32_t fz_shade_add_color_stop(int32_t _ctx, int32_t shade, float offset, float const * color, int32_t n);
int32_t fz_shade_add_patch(int32_t _ctx, int32_t shade, ShadePoint const * points, [f32; 4] const * colors);
int32_t fz_shade_add_vertex(int32_t _ctx, int32_t shade, float x, float y, float const * color, int32_t n);
void fz_shade_bbox(int32_t _ctx, int32_t shade, float * bbox);
int32_t fz_shade_color_stop_count(int32_t _ctx, int32_t shade);
uint64_t fz_shade_colorspace(int32_t _ctx, int32_t shade);
int32_t fz_shade_extend_end(int32_t _ctx, int32_t shade);
int32_t fz_shade_extend_start(int32_t _ctx, int32_t shade);
int32_t fz_shade_get_color_stop(int32_t _ctx, int32_t shade, int32_t index, float * offset, float * color);
int32_t fz_shade_has_background(int32_t _ctx, int32_t shade);
int32_t fz_shade_patch_count(int32_t _ctx, int32_t shade);
void fz_shade_sample(int32_t _ctx, int32_t shade, float t, float * color);
void fz_shade_set_background(int32_t _ctx, int32_t shade, float const * color);
void fz_shade_set_bbox(int32_t _ctx, int32_t shade, float x0, float y0, float x1, float y1);
int32_t fz_shade_type(int32_t _ctx, int32_t shade);
int32_t fz_shade_vertex_count(int32_t _ctx, int32_t shade);

#ifdef __cplusplus
}
#endif

#endif /* MUPDF_FITZ_SHADE_H */
