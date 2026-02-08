// MicroPDF - MuPDF API Compatible C Header
// Auto-generated from Rust FFI - DO NOT EDIT MANUALLY
// Module: hyphen

#ifndef MUPDF_FITZ_HYPHEN_H
#define MUPDF_FITZ_HYPHEN_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================================
// Hyphen Functions (17 total)
// ============================================================================

void fz_drop_hyphenator(int32_t _ctx, int32_t hyph);
int32_t fz_hyphenate_word(int32_t _ctx, int32_t hyph, const char * input, int32_t input_size, char * output, int32_t output_size);
size_t fz_hyphenation_points(int32_t _ctx, int32_t hyph, const char * word, u8 * points, size_t points_len);
int32_t fz_hyphenator_add_pattern(int32_t _ctx, int32_t hyph, const char * pattern);
int32_t fz_hyphenator_language(int32_t _ctx, int32_t hyph);
size_t fz_hyphenator_left_min(int32_t _ctx, int32_t hyph);
int32_t fz_hyphenator_load_patterns(int32_t _ctx, int32_t hyph, const char * data);
size_t fz_hyphenator_pattern_count(int32_t _ctx, int32_t hyph);
size_t fz_hyphenator_right_min(int32_t _ctx, int32_t hyph);
void fz_hyphenator_set_left_min(int32_t _ctx, int32_t hyph, size_t min);
void fz_hyphenator_set_right_min(int32_t _ctx, int32_t hyph, size_t min);
int32_t fz_is_unicode_hyphen(uint32_t c);
int32_t fz_lookup_hyphenator(int32_t _ctx, int32_t language);
int32_t fz_new_empty_hyphenator(int32_t _ctx);
int32_t fz_new_hyphenator(int32_t _ctx, int32_t language);
void fz_register_hyphenator(int32_t _ctx, int32_t language, int32_t hyph);
const char * fz_text_language_code(int32_t language);

#ifdef __cplusplus
}
#endif

#endif /* MUPDF_FITZ_HYPHEN_H */
