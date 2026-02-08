// MicroPDF - MuPDF API Compatible C Header
// Auto-generated from Rust FFI - DO NOT EDIT MANUALLY
// Module: ocr

#ifndef MUPDF_FITZ_OCR_H
#define MUPDF_FITZ_OCR_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================================
// Ocr Functions (15 total)
// ============================================================================

void fz_drop_ocr_engine(int32_t _ctx, int32_t engine);
void fz_drop_ocr_result(int32_t _ctx, int32_t result);
void fz_free_ocr_string(int32_t _ctx, char * s);
int32_t fz_new_ocr_engine(int32_t _ctx, int engine_type);
int32_t fz_new_ocr_result(int32_t _ctx, uint32_t width, uint32_t height);
char * fz_ocr_engine_get_language(int32_t _ctx, int32_t engine);
int fz_ocr_engine_init(int32_t _ctx, int32_t engine);
int fz_ocr_engine_is_initialized(int32_t _ctx, int32_t engine);
int fz_ocr_engine_set_language(int32_t _ctx, int32_t engine, const char * lang);
void fz_ocr_engine_set_psm(int32_t _ctx, int32_t engine, int psm);
int fz_ocr_is_available(int32_t _ctx, int engine_type);
int fz_ocr_result_confidence(int32_t _ctx, int32_t result);
int fz_ocr_result_line_count(int32_t _ctx, int32_t result);
char * fz_ocr_result_text(int32_t _ctx, int32_t result);
int fz_ocr_result_word_count(int32_t _ctx, int32_t result);

#ifdef __cplusplus
}
#endif

#endif /* MUPDF_FITZ_OCR_H */
