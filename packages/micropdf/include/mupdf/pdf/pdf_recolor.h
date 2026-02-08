// MicroPDF - MuPDF API Compatible C Header
// Auto-generated from Rust FFI - DO NOT EDIT MANUALLY
// Module: pdf_recolor

#ifndef MUPDF_PDF_PDF_RECOLOR_H
#define MUPDF_PDF_PDF_RECOLOR_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================================
// Pdf_recolor Functions (19 total)
// ============================================================================

void pdf_cmyk_to_rgb(float c, float m, float y, float k, float * r, float * g, float * b);
void pdf_convert_color(int32_t _ctx, int32_t _src_cs, float const * src, int32_t src_n, int32_t _dst_cs, float * dst, int32_t dst_n);
int32_t pdf_count_output_intents(int32_t _ctx, int32_t _doc);
void pdf_drop_shade_recolor_context(int32_t _ctx, int32_t recolor_ctx);
void pdf_gray_to_rgb(float gray, float * r, float * g, float * b);
int32_t pdf_new_shade_recolor_context(int32_t _ctx, int32_t src_cs, int32_t dst_cs);
RecolorStats pdf_recolor_document(int32_t _ctx, int32_t _doc, RecolorOptions const * _opts);
RecolorOptions pdf_recolor_options_cmyk(void);
RecolorOptions pdf_recolor_options_gray(void);
int32_t pdf_recolor_options_is_valid(RecolorOptions const * opts);
RecolorOptions pdf_recolor_options_new(int32_t num_comp);
RecolorOptions pdf_recolor_options_rgb(void);
void pdf_recolor_page(int32_t _ctx, int32_t _doc, int32_t _pagenum, RecolorOptions const * _opts);
RecolorStats pdf_recolor_pages(int32_t _ctx, int32_t _doc, int32_t _start_page, int32_t _end_page, RecolorOptions const * _opts);
int32_t pdf_recolor_shade(int32_t _ctx, int32_t _shade, int32_t _recolor_ctx);
void pdf_remove_output_intents(int32_t _ctx, int32_t _doc);
void pdf_rgb_to_cmyk(float r, float g, float b, float * c, float * m, float * y, float * k);
float pdf_rgb_to_gray(float r, float g, float b);
void pdf_shade_recolor_set_opaque(int32_t _ctx, int32_t recolor_ctx, void * opaque);

#ifdef __cplusplus
}
#endif

#endif /* MUPDF_PDF_PDF_RECOLOR_H */
