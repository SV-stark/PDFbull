// MicroPDF - MuPDF API Compatible C Header
// Auto-generated from Rust FFI - DO NOT EDIT MANUALLY
// Module: barcode

#ifndef MUPDF_FITZ_BARCODE_H
#define MUPDF_FITZ_BARCODE_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================================
// Barcode Functions (11 total)
// ============================================================================

int32_t fz_barcode_check_digit(int32_t barcode_type, const char * value);
int32_t fz_barcode_default_size(int32_t barcode_type);
int32_t fz_barcode_is_1d(int32_t barcode_type);
int32_t fz_barcode_is_2d(int32_t barcode_type);
int32_t fz_barcode_type_count(void);
int32_t fz_barcode_type_from_string(const char * str_ptr);
int32_t fz_barcode_validate(int32_t barcode_type, const char * value);
char * fz_decode_barcode_from_pixmap(int32_t _ctx, int32_t * type_out, int32_t _pix, int32_t _rotate);
int32_t fz_new_barcode_image(int32_t ctx, int32_t barcode_type, const char * value, int32_t size, int32_t ec_level, int32_t quiet, int32_t hrt);
int32_t fz_new_barcode_pixmap(int32_t _ctx, int32_t barcode_type, const char * value, int32_t size, int32_t ec_level, int32_t quiet, int32_t _hrt);
const char * fz_string_from_barcode_type(int32_t barcode_type);

#ifdef __cplusplus
}
#endif

#endif /* MUPDF_FITZ_BARCODE_H */
