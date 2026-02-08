/*
 * PDF Recolor FFI
 *
 * Provides PDF color conversion functionality including page recoloring,
 * shade recoloring, and output intent management.
 */

#ifndef MICROPDF_PDF_RECOLOR_H
#define MICROPDF_PDF_RECOLOR_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Handle types */
typedef uint64_t fz_context;
typedef uint64_t pdf_document;
typedef uint64_t fz_colorspace;
typedef uint64_t fz_shade;
typedef uint64_t pdf_shade_recolor_context;

/* ============================================================================
 * Color Space Types
 * ============================================================================ */

#define RECOLOR_GRAY    1   /* Grayscale (1 component) */
#define RECOLOR_RGB     3   /* RGB (3 components) */
#define RECOLOR_CMYK    4   /* CMYK (4 components) */

/* ============================================================================
 * Recolor Options
 * ============================================================================ */

/**
 * Recolor options for page conversion.
 * num_comp: 1 = Gray, 3 = RGB, 4 = CMYK
 */
typedef struct {
    int num_comp;
} pdf_recolor_options;

/**
 * Recolor statistics.
 */
typedef struct {
    int pages_processed;
    int colors_converted;
    int shades_recolored;
    int images_processed;
    int output_intents_removed;
} pdf_recolor_stats;

/* ============================================================================
 * Recolor Options Functions
 * ============================================================================ */

/**
 * Get grayscale recolor options.
 */
pdf_recolor_options pdf_recolor_options_gray(void);

/**
 * Get RGB recolor options.
 */
pdf_recolor_options pdf_recolor_options_rgb(void);

/**
 * Get CMYK recolor options.
 */
pdf_recolor_options pdf_recolor_options_cmyk(void);

/**
 * Create custom recolor options.
 */
pdf_recolor_options pdf_recolor_options_new(int num_comp);

/**
 * Check if recolor options are valid.
 * @return 1 if valid, 0 if invalid
 */
int pdf_recolor_options_is_valid(const pdf_recolor_options *opts);

/* ============================================================================
 * Page Recoloring
 * ============================================================================ */

/**
 * Recolor a given document page.
 * Converts all colors on the page to the target colorspace.
 */
void pdf_recolor_page(fz_context *ctx, pdf_document *doc, int pagenum, const pdf_recolor_options *opts);

/**
 * Recolor all pages in a document.
 */
pdf_recolor_stats pdf_recolor_document(fz_context *ctx, pdf_document *doc, const pdf_recolor_options *opts);

/**
 * Recolor a range of pages.
 */
pdf_recolor_stats pdf_recolor_pages(fz_context *ctx, pdf_document *doc, int start_page, int end_page, const pdf_recolor_options *opts);

/* ============================================================================
 * Output Intents
 * ============================================================================ */

/**
 * Remove output intents from a document.
 */
void pdf_remove_output_intents(fz_context *ctx, pdf_document *doc);

/**
 * Count output intents in a document.
 */
int pdf_count_output_intents(fz_context *ctx, pdf_document *doc);

/* ============================================================================
 * Shade Recoloring
 * ============================================================================ */

/**
 * Create a shade recolor context.
 */
pdf_shade_recolor_context *pdf_new_shade_recolor_context(fz_context *ctx, fz_colorspace *src_cs, fz_colorspace *dst_cs);

/**
 * Drop a shade recolor context.
 */
void pdf_drop_shade_recolor_context(fz_context *ctx, pdf_shade_recolor_context *recolor_ctx);

/**
 * Set opaque data for shade recolor context.
 */
void pdf_shade_recolor_set_opaque(fz_context *ctx, pdf_shade_recolor_context *recolor_ctx, void *opaque);

/**
 * Recolor a shade object.
 * @return New shade handle (0 on error)
 */
fz_shade *pdf_recolor_shade(fz_context *ctx, fz_shade *shade, pdf_shade_recolor_context *recolor_ctx);

/* ============================================================================
 * Color Conversion Utilities
 * ============================================================================ */

/**
 * Convert a single color from one colorspace to another.
 */
void pdf_convert_color(fz_context *ctx, fz_colorspace *src_cs, const float *src, int src_n, fz_colorspace *dst_cs, float *dst, int dst_n);

/**
 * Convert gray to RGB.
 */
void pdf_gray_to_rgb(float gray, float *r, float *g, float *b);

/**
 * Convert RGB to gray.
 */
float pdf_rgb_to_gray(float r, float g, float b);

/**
 * Convert CMYK to RGB.
 */
void pdf_cmyk_to_rgb(float c, float m, float y, float k, float *r, float *g, float *b);

/**
 * Convert RGB to CMYK.
 */
void pdf_rgb_to_cmyk(float r, float g, float b, float *c, float *m, float *y, float *k);

#ifdef __cplusplus
}
#endif

#endif /* MICROPDF_PDF_RECOLOR_H */


