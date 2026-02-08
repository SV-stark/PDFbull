// MicroPDF - MuPDF API Compatible C Header
// Auto-generated from Rust FFI - DO NOT EDIT MANUALLY
// Module: geometry

#ifndef MUPDF_FITZ_GEOMETRY_H
#define MUPDF_FITZ_GEOMETRY_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================================
// Geometry Functions (58 total)
// ============================================================================

fz_matrix fz_concat(fz_matrix left, fz_matrix right);
int32_t fz_contains_rect(fz_rect a, fz_rect b);
fz_irect fz_expand_irect(fz_irect r, int32_t expand);
fz_rect fz_expand_rect(fz_rect r, float expand);
fz_rect fz_include_point_in_rect(fz_rect r, fz_point p);
fz_irect fz_intersect_irect(fz_irect a, fz_irect b);
fz_rect fz_intersect_rect(fz_rect a, fz_rect b);
fz_matrix fz_invert_matrix(fz_matrix m);
int32_t fz_irect_area(fz_irect r);
int32_t fz_irect_eq(fz_irect a, fz_irect b);
fz_irect fz_irect_from_rect(fz_rect rect);
int32_t fz_irect_height(fz_irect r);
int32_t fz_irect_width(fz_irect r);
int32_t fz_is_empty_irect(fz_irect r);
int32_t fz_is_empty_rect(fz_rect r);
int32_t fz_is_infinite_irect(fz_irect r);
int32_t fz_is_infinite_rect(fz_rect r);
int32_t fz_is_point_inside_irect(int32_t x, int32_t y, fz_irect r);
int32_t fz_is_point_inside_rect(fz_point p, fz_rect r);
int32_t fz_is_rectilinear(fz_matrix m);
fz_irect fz_make_irect(int32_t x0, int32_t y0, int32_t x1, int32_t y1);
fz_point fz_make_point(float x, float y);
fz_rect fz_make_rect(float x0, float y0, float x1, float y1);
float fz_matrix_determinant(fz_matrix m);
float fz_matrix_expansion(fz_matrix m);
int32_t fz_matrix_is_singular(fz_matrix m);
float fz_matrix_max_expansion(fz_matrix m);
fz_point fz_normalize_vector(fz_point p);
int32_t fz_overlaps_rect(fz_rect a, fz_rect b);
fz_matrix fz_post_rotate(fz_matrix m, float degrees);
fz_matrix fz_post_scale(fz_matrix m, float sx, float sy);
fz_matrix fz_post_translate(fz_matrix m, float tx, float ty);
fz_matrix fz_pre_rotate(fz_matrix m, float degrees);
fz_matrix fz_pre_scale(fz_matrix m, float sx, float sy);
fz_matrix fz_pre_shear(fz_matrix m, float sx, float sy);
fz_matrix fz_pre_translate(fz_matrix m, float tx, float ty);
fz_quad fz_quad_from_rect(fz_rect r);
float fz_rect_area(fz_rect r);
fz_point fz_rect_center(fz_rect r);
int32_t fz_rect_eq(fz_rect a, fz_rect b);
fz_rect fz_rect_from_irect(fz_irect bbox);
fz_rect fz_rect_from_quad(fz_quad q);
float fz_rect_height(fz_rect r);
float fz_rect_width(fz_rect r);
fz_matrix fz_rotate(float degrees);
fz_irect fz_round_rect(fz_rect rect);
fz_matrix fz_scale(float sx, float sy);
fz_matrix fz_shear(float sx, float sy);
fz_point fz_transform_point(fz_point p, fz_matrix m);
fz_point fz_transform_point_xy(float x, float y, fz_matrix m);
fz_quad fz_transform_quad(fz_quad q, fz_matrix m);
fz_rect fz_transform_rect(fz_rect r, fz_matrix m);
fz_point fz_transform_vector(fz_point v, fz_matrix m);
fz_matrix fz_translate(float tx, float ty);
fz_irect fz_translate_irect(fz_irect r, int32_t xoff, int32_t yoff);
fz_rect fz_translate_rect(fz_rect r, float xoff, float yoff);
fz_rect fz_union_rect(fz_rect a, fz_rect b);
const char * fz_version(void);

#ifdef __cplusplus
}
#endif

#endif /* MUPDF_FITZ_GEOMETRY_H */
