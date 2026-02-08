// MicroPDF - MuPDF API Compatible C Header
// Auto-generated from Rust FFI - DO NOT EDIT MANUALLY
// Module: colorspace

#ifndef MUPDF_FITZ_COLORSPACE_H
#define MUPDF_FITZ_COLORSPACE_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================================
// Colorspace Functions (42 total)
// ============================================================================

void fz_clamp_color(int32_t _ctx, int32_t cs, float const * color_in, float * color_out);
int32_t fz_clone_colorspace(int32_t _ctx, int32_t cs);
int32_t fz_colorspace_base(int32_t _ctx, int32_t cs);
int32_t fz_colorspace_base_n(int32_t _ctx, int32_t cs);
const char * fz_colorspace_colorant(int32_t _ctx, int32_t _cs, int32_t _idx);
int32_t fz_colorspace_device_n_has_cmyk(int32_t _ctx, int32_t _cs);
int32_t fz_colorspace_device_n_has_only_cmyk(int32_t _ctx, int32_t _cs);
int32_t fz_colorspace_eq(int32_t _ctx, int32_t a, int32_t b);
int32_t fz_colorspace_has_spots(int32_t _ctx, int32_t cs);
int32_t fz_colorspace_high(int32_t _ctx, int32_t cs);
int32_t fz_colorspace_is_cmyk(int32_t _ctx, int32_t cs);
int32_t fz_colorspace_is_device(int32_t _ctx, int32_t cs);
int32_t fz_colorspace_is_device_n(int32_t _ctx, int32_t cs);
int32_t fz_colorspace_is_gray(int32_t _ctx, int32_t cs);
int32_t fz_colorspace_is_icc(int32_t _ctx, int32_t cs);
int32_t fz_colorspace_is_indexed(int32_t _ctx, int32_t cs);
int32_t fz_colorspace_is_lab(int32_t _ctx, int32_t cs);
int32_t fz_colorspace_is_rgb(int32_t _ctx, int32_t cs);
int32_t fz_colorspace_is_subtractive(int32_t _ctx, int32_t cs);
int32_t fz_colorspace_is_valid(int32_t _ctx, int32_t cs);
u8 const * fz_colorspace_lookup(int32_t _ctx, int32_t _cs);
float fz_colorspace_max(int32_t _ctx, int32_t cs);
int32_t fz_colorspace_n(int32_t _ctx, int32_t cs);
int32_t fz_colorspace_n_spots(int32_t _ctx, int32_t cs);
const char * fz_colorspace_name(int32_t _ctx, int32_t cs);
const char * fz_colorspace_name_string(int32_t _ctx, int32_t cs);
int32_t fz_colorspace_num_colorants(int32_t _ctx, int32_t cs);
int32_t fz_colorspace_type(int32_t _ctx, int32_t cs);
void fz_convert_color(int32_t _ctx, int32_t src_cs, float const * src, int32_t dst_cs, float * dst, int32_t _proof_cs);
void fz_convert_pixel(int32_t _ctx, int32_t src_cs, float const * src, int32_t dst_cs, float * dst);
int32_t fz_device_bgr(int32_t _ctx);
int32_t fz_device_cmyk(int32_t _ctx);
int32_t fz_device_gray(int32_t _ctx);
int32_t fz_device_grayscale(int32_t _ctx);
int32_t fz_device_lab(int32_t _ctx);
int32_t fz_device_rgb(int32_t _ctx);
int32_t fz_device_srgb(int32_t _ctx);
void fz_drop_colorspace(int32_t _ctx, int32_t _cs);
int32_t fz_keep_colorspace(int32_t _ctx, int32_t cs);
int32_t fz_new_device_n_colorspace(int32_t _ctx, int32_t base, int32_t n, const char * const * _colorants);
int32_t fz_new_icc_colorspace(int32_t _ctx, int32_t _type_hint, int32_t // Hint about what type of colorspace (gray, rgb, cmyk) _flags, const char * name, u8 const * _data, size_t _size);
int32_t fz_new_indexed_colorspace(int32_t _ctx, int32_t base, int32_t high, u8 const * lookup);

#ifdef __cplusplus
}
#endif

#endif /* MUPDF_FITZ_COLORSPACE_H */
