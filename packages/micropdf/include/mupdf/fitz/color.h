// MicroPDF - MuPDF API Compatible C Header
// Auto-generated from Rust FFI - DO NOT EDIT MANUALLY
// Module: color

#ifndef MUPDF_FITZ_COLOR_H
#define MUPDF_FITZ_COLOR_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================================
// Color Functions (26 total)
// ============================================================================

int32_t fz_clone_default_colorspaces(int32_t _ctx, int32_t base);
int32_t fz_color_params_bp(ColorParams params);
int32_t fz_color_params_op(ColorParams params);
int32_t fz_color_params_opm(ColorParams params);
int32_t fz_color_params_ri(ColorParams params);
void fz_colorspace_digest(int32_t _ctx, int32_t _cs, u8 * digest);
void fz_convert_color_with_params(int32_t _ctx, int32_t src_cs, float const * src, int32_t dst_cs, float * dst, int32_t proof_cs, ColorParams _params);
int32_t fz_default_cmyk(int32_t _ctx, int32_t default_cs);
ColorParams fz_default_color_params(void);
int32_t fz_default_gray(int32_t _ctx, int32_t default_cs);
int32_t fz_default_output_intent(int32_t _ctx, int32_t default_cs);
int32_t fz_default_rgb(int32_t _ctx, int32_t default_cs);
void fz_drop_default_colorspaces(int32_t _ctx, int32_t default_cs);
int32_t fz_is_valid_blend_colorspace(int32_t _ctx, int32_t cs);
int32_t fz_keep_default_colorspaces(int32_t _ctx, int32_t default_cs);
int32_t fz_lookup_rendering_intent(const char * name);
int32_t fz_max_colors(void);
int32_t fz_new_cal_gray_colorspace(int32_t _ctx, float const * wp, float const * bp, float gamma);
int32_t fz_new_cal_rgb_colorspace(int32_t _ctx, float const * wp, float const * bp, float const * gamma, float const * matrix);
ColorParams fz_new_color_params(int32_t ri, int32_t bp, int32_t op, int32_t opm);
int32_t fz_new_default_colorspaces(int32_t _ctx);
const char * fz_rendering_intent_name(int32_t ri);
void fz_set_default_cmyk(int32_t _ctx, int32_t default_cs, int32_t cs);
void fz_set_default_gray(int32_t _ctx, int32_t default_cs, int32_t cs);
void fz_set_default_output_intent(int32_t _ctx, int32_t default_cs, int32_t cs);
void fz_set_default_rgb(int32_t _ctx, int32_t default_cs, int32_t cs);

#ifdef __cplusplus
}
#endif

#endif /* MUPDF_FITZ_COLOR_H */
