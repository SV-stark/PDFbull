// MicroPDF - MuPDF API Compatible C Header
// Auto-generated from Rust FFI - DO NOT EDIT MANUALLY
// Module: simd_util

#ifndef MUPDF_FITZ_SIMD_UTIL_H
#define MUPDF_FITZ_SIMD_UTIL_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================================
// Simd_util Functions (12 total)
// ============================================================================

int fz_has_simd(void);
int fz_simd_buffer_copy(u8 * dst, u8 const * src, size_t len);
int fz_simd_buffer_equal(u8 const * a, u8 const * b, size_t len);
int fz_simd_buffer_fill(u8 * dst, size_t len, u8 value);
[c_float fz_simd_cmyk_to_rgb(float c, float m, float y, float k);
SimdFeatures fz_simd_features(void);
int fz_simd_level(void);
SimdMatrix fz_simd_matrix_concat(SimdMatrix left, SimdMatrix right);
[c_float fz_simd_rgb_to_cmyk(float r, float g, float b);
float fz_simd_rgb_to_gray(float r, float g, float b);
[c_float fz_simd_transform_point(float x, float y, SimdMatrix m);
int fz_simd_transform_points(float * points, size_t count, SimdMatrix m);

#ifdef __cplusplus
}
#endif

#endif /* MUPDF_FITZ_SIMD_UTIL_H */
