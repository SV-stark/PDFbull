// MicroPDF - MuPDF API Compatible C Header
// Auto-generated from Rust FFI - DO NOT EDIT MANUALLY
// Module: string_util

#ifndef MUPDF_FITZ_STRING_UTIL_H
#define MUPDF_FITZ_STRING_UTIL_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================================
// String_util Functions (13 total)
// ============================================================================

size_t fz_bidi_reorder(int32_t _ctx, const char * input, char * output, size_t output_size, int32_t base_dir);
int32_t fz_byte_to_char_offset(int32_t _ctx, const char * s, size_t byte_offset);
size_t fz_casefold(int32_t _ctx, const char * input, char * output, size_t output_size);
int32_t fz_char_to_byte_offset(int32_t _ctx, const char * s, size_t char_index);
int32_t fz_detect_script(int32_t _ctx, const char * text);
size_t fz_find_line_breaks(int32_t _ctx, const char * text, int32_t * breaks, size_t max_breaks);
size_t fz_find_word_breaks(int32_t _ctx, const char * text, int32_t * breaks, size_t max_breaks);
int32_t fz_get_bidi_direction(int32_t _ctx, const char * text);
int32_t fz_get_word_at(int32_t _ctx, const char * text, size_t position, size_t * word_start, size_t * word_end);
size_t fz_normalize_string(int32_t _ctx, const char * input, char * output, size_t output_size, int32_t form);
int32_t fz_strcoll(int32_t _ctx, const char * s1, const char * s2, const char * _locale);
size_t fz_string_char_count(int32_t _ctx, const char * s);
int32_t fz_string_is_normalized(int32_t _ctx, const char * input, int32_t form);

#ifdef __cplusplus
}
#endif

#endif /* MUPDF_FITZ_STRING_UTIL_H */
