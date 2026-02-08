// MicroPDF - MuPDF API Compatible C Header
// Auto-generated from Rust FFI - DO NOT EDIT MANUALLY
// Module: hints

#ifndef MUPDF_FITZ_HINTS_H
#define MUPDF_FITZ_HINTS_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================================
// Hints Functions (11 total)
// ============================================================================

void fz_assume(int condition);
int fz_black_box_int(int value);
float fz_clamp_f32(float value, float min, float max);
int32_t fz_clamp_i32(int32_t value, int32_t min, int32_t max);
int fz_likely(int condition);
float fz_max_f32(float a, float b);
int32_t fz_max_i32(int32_t a, int32_t b);
float fz_min_f32(float a, float b);
int32_t fz_min_i32(int32_t a, int32_t b);
void fz_spin_loop_hint(void);
int fz_unlikely(int condition);

#ifdef __cplusplus
}
#endif

#endif /* MUPDF_FITZ_HINTS_H */
