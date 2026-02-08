// MicroPDF - MuPDF API Compatible C Header
// Auto-generated from Rust FFI - DO NOT EDIT MANUALLY
// Module: path

#ifndef MUPDF_FITZ_PATH_H
#define MUPDF_FITZ_PATH_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================================
// Path Functions (35 total)
// ============================================================================

fz_rect fz_bound_path(int32_t _ctx, int32_t path, int32_t _stroke, fz_matrix _transform);
int32_t fz_clone_path(int32_t _ctx, int32_t path);
int32_t fz_clone_stroke_state(int32_t _ctx, int32_t stroke);
void fz_closepath(int32_t _ctx, int32_t path);
fz_point fz_currentpoint(int32_t _ctx, int32_t path);
void fz_curveto(int32_t _ctx, int32_t path, float x1, float y1, float x2, float y2, float x3, float y3);
void fz_drop_path(int32_t _ctx, int32_t path);
void fz_drop_stroke_state(int32_t _ctx, int32_t stroke);
int32_t fz_keep_path(int32_t _ctx, int32_t path);
int32_t fz_keep_stroke_state(int32_t _ctx, int32_t stroke);
void fz_lineto(int32_t _ctx, int32_t path, float x, float y);
void fz_moveto(int32_t _ctx, int32_t path, float x, float y);
int32_t fz_new_path(int32_t _ctx);
int32_t fz_new_stroke_state(int32_t _ctx);
int32_t fz_new_stroke_state_with_len(int32_t _ctx, int32_t _len, float linewidth);
int32_t fz_path_is_valid(int32_t _ctx, int32_t path);
void fz_quadto(int32_t _ctx, int32_t path, float x1, float y1, float x2, float y2);
void fz_rectto(int32_t _ctx, int32_t path, float x0, float y0, float x1, float y1);
int32_t fz_stroke_state_dash_len(int32_t _ctx, int32_t stroke);
int32_t fz_stroke_state_dash_pattern(int32_t _ctx, int32_t stroke, float * dashes, int32_t len);
float fz_stroke_state_dash_phase(int32_t _ctx, int32_t stroke);
int32_t fz_stroke_state_end_cap(int32_t _ctx, int32_t stroke);
int32_t fz_stroke_state_is_valid(int32_t _ctx, int32_t stroke);
int32_t fz_stroke_state_linejoin(int32_t _ctx, int32_t stroke);
float fz_stroke_state_linewidth(int32_t _ctx, int32_t stroke);
float fz_stroke_state_miterlimit(int32_t _ctx, int32_t stroke);
void fz_stroke_state_set_dash(int32_t _ctx, int32_t stroke, float phase, float const * dashes, int32_t len);
void fz_stroke_state_set_end_cap(int32_t _ctx, int32_t stroke, int32_t cap);
void fz_stroke_state_set_linejoin(int32_t _ctx, int32_t stroke, int32_t join);
void fz_stroke_state_set_linewidth(int32_t _ctx, int32_t stroke, float linewidth);
void fz_stroke_state_set_miterlimit(int32_t _ctx, int32_t stroke, float limit);
void fz_stroke_state_set_start_cap(int32_t _ctx, int32_t stroke, int32_t cap);
int32_t fz_stroke_state_start_cap(int32_t _ctx, int32_t stroke);
void fz_transform_path(int32_t _ctx, int32_t path, fz_matrix transform);
int32_t fz_unshare_stroke_state(int32_t _ctx, int32_t stroke);

#ifdef __cplusplus
}
#endif

#endif /* MUPDF_FITZ_PATH_H */
