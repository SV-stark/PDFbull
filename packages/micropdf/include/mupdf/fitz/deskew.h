// MicroPDF - MuPDF API Compatible C Header
// Auto-generated from Rust FFI - DO NOT EDIT MANUALLY
// Module: deskew

#ifndef MUPDF_FITZ_DESKEW_H
#define MUPDF_FITZ_DESKEW_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================================
// Deskew Functions (9 total)
// ============================================================================

int32_t fz_auto_deskew_pixmap(int32_t _ctx, int32_t src, int32_t border);
int32_t fz_deskew_pixmap(int32_t _ctx, int32_t src, double degrees, int32_t border);
double fz_detect_skew(int32_t _ctx, int32_t pixmap);
int32_t fz_detect_skew_angle(int32_t _ctx, int32_t pixmap, double * angle);
int32_t fz_flip_pixmap_horizontal(int32_t _ctx, int32_t src);
int32_t fz_flip_pixmap_vertical(int32_t _ctx, int32_t src);
int32_t fz_is_skewed(int32_t _ctx, int32_t pixmap, double threshold);
int32_t fz_rotate_pixmap(int32_t _ctx, int32_t src, double degrees, int32_t border);
int32_t fz_rotate_pixmap_90(int32_t _ctx, int32_t src, int32_t quarters);

#ifdef __cplusplus
}
#endif

#endif /* MUPDF_FITZ_DESKEW_H */
