// MicroPDF - MuPDF API Compatible C Header
// Auto-generated from Rust FFI - DO NOT EDIT MANUALLY
// Module: bidi

#ifndef MUPDF_FITZ_BIDI_H
#define MUPDF_FITZ_BIDI_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================================
// Bidi Functions (14 total)
// ============================================================================

int32_t fz_bidi_char_type(uint32_t ch);
int32_t fz_bidi_detect_direction(int32_t _ctx, uint32_t const * text, size_t textlen);
int32_t fz_bidi_direction_from_char(uint32_t ch);
void fz_bidi_fragment_text(int32_t _ctx, uint32_t const * text, size_t textlen, int32_t * base_dir, BidiFragmentFn callback, void * arg, int32_t flags);
int32_t fz_bidi_get_level(int32_t _ctx, uint32_t const * text, size_t textlen, int32_t base_dir, size_t position);
size_t fz_bidi_get_levels(int32_t _ctx, uint32_t const * text, size_t textlen, int32_t base_dir, int32_t * levels_out, size_t levels_len);
uint32_t fz_bidi_get_mirror(uint32_t ch);
int32_t fz_bidi_has_mirror(uint32_t ch);
int32_t fz_bidi_has_rtl(int32_t _ctx, uint32_t const * text, size_t textlen);
int32_t fz_bidi_is_control(uint32_t ch);
int32_t fz_bidi_is_ltr_only(int32_t _ctx, uint32_t const * text, size_t textlen);
int32_t fz_bidi_is_rtl_only(int32_t _ctx, uint32_t const * text, size_t textlen);
size_t fz_bidi_reorder_run(int32_t _ctx, uint32_t const * text, size_t textlen, int32_t base_dir, uint32_t * output, size_t output_len);
size_t fz_bidi_strip_controls(int32_t _ctx, uint32_t const * text, size_t textlen, uint32_t * output, size_t output_len);

#ifdef __cplusplus
}
#endif

#endif /* MUPDF_FITZ_BIDI_H */
