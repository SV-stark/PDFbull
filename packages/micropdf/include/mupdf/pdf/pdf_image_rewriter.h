// MicroPDF - MuPDF API Compatible C Header
// Auto-generated from Rust FFI - DO NOT EDIT MANUALLY
// Module: pdf_image_rewriter

#ifndef MUPDF_PDF_PDF_IMAGE_REWRITER_H
#define MUPDF_PDF_PDF_IMAGE_REWRITER_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================================
// Pdf_image_rewriter Functions (19 total)
// ============================================================================

ImageRewriteStats pdf_analyze_images(int32_t _ctx, int32_t _doc);
int32_t pdf_count_images(int32_t _ctx, int32_t _doc);
ImageRewriterOptions pdf_default_image_rewriter_options(void);
void pdf_drop_image_rewriter_options(ImageRewriterOptions * opts);
ImageRewriterOptions pdf_ebook_image_rewriter_options(void);
uint64_t pdf_get_total_image_size(int32_t _ctx, int32_t _doc);
ImageRewriterOptions pdf_max_compression_image_rewriter_options(void);
ImageRewriterOptions pdf_print_image_rewriter_options(void);
void pdf_rewrite_images(int32_t _ctx, int32_t _doc, ImageRewriterOptions * _opts);
ImageRewriteStats pdf_rewrite_images_with_stats(int32_t _ctx, int32_t _doc, ImageRewriterOptions * _opts);
void pdf_set_bitonal_recompress(ImageRewriterOptions * opts, int32_t method);
void pdf_set_bitonal_subsample(ImageRewriterOptions * opts, int32_t threshold_dpi, int32_t target_dpi, int32_t method);
void pdf_set_color_jpeg_quality(ImageRewriterOptions * opts, const char * quality);
void pdf_set_color_recompress(ImageRewriterOptions * opts, int32_t method);
void pdf_set_color_subsample(ImageRewriterOptions * opts, int32_t threshold_dpi, int32_t target_dpi, int32_t method);
void pdf_set_gray_jpeg_quality(ImageRewriterOptions * opts, const char * quality);
void pdf_set_gray_recompress(ImageRewriterOptions * opts, int32_t method);
void pdf_set_gray_subsample(ImageRewriterOptions * opts, int32_t threshold_dpi, int32_t target_dpi, int32_t method);
ImageRewriterOptions pdf_web_image_rewriter_options(void);

#ifdef __cplusplus
}
#endif

#endif /* MUPDF_PDF_PDF_IMAGE_REWRITER_H */
