/*
 * PDF Image Rewriter FFI
 *
 * Provides PDF image optimization including resampling, recompression,
 * and resolution changes for color, grayscale, and bitonal images.
 */

#ifndef MICROPDF_PDF_IMAGE_REWRITER_H
#define MICROPDF_PDF_IMAGE_REWRITER_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Handle types */
typedef uint64_t fz_context;
typedef uint64_t pdf_document;

/* ============================================================================
 * Subsample Methods
 * ============================================================================ */

#define FZ_SUBSAMPLE_AVERAGE    0   /* Average subsampling */
#define FZ_SUBSAMPLE_BICUBIC    1   /* Bicubic subsampling (higher quality) */

/* ============================================================================
 * Recompress Methods
 * ============================================================================ */

#define FZ_RECOMPRESS_NEVER     0   /* Never recompress */
#define FZ_RECOMPRESS_SAME      1   /* Use same method as original */
#define FZ_RECOMPRESS_LOSSLESS  2   /* Lossless compression (PNG/Flate) */
#define FZ_RECOMPRESS_JPEG      3   /* JPEG compression */
#define FZ_RECOMPRESS_J2K       4   /* JPEG 2000 compression */
#define FZ_RECOMPRESS_FAX       5   /* CCITT Fax compression (bitonal only) */

/* ============================================================================
 * Image Rewriter Options
 * ============================================================================ */

/**
 * Image rewriter options for color, grayscale, and bitonal images.
 */
typedef struct {
    /* Color lossless images */
    int color_lossless_image_subsample_method;      /* Subsample method */
    int color_lossy_image_subsample_method;         /* Subsample method */
    int color_lossless_image_subsample_threshold;   /* DPI threshold (0=never) */
    int color_lossless_image_subsample_to;          /* Target DPI */
    int color_lossy_image_subsample_threshold;      /* DPI threshold */
    int color_lossy_image_subsample_to;             /* Target DPI */
    int color_lossless_image_recompress_method;     /* Recompress method */
    int color_lossy_image_recompress_method;        /* Recompress method */
    char *color_lossy_image_recompress_quality;     /* Quality string */
    char *color_lossless_image_recompress_quality;  /* Quality string */

    /* Grayscale images */
    int gray_lossless_image_subsample_method;
    int gray_lossy_image_subsample_method;
    int gray_lossless_image_subsample_threshold;
    int gray_lossless_image_subsample_to;
    int gray_lossy_image_subsample_threshold;
    int gray_lossy_image_subsample_to;
    int gray_lossless_image_recompress_method;
    int gray_lossy_image_recompress_method;
    char *gray_lossy_image_recompress_quality;
    char *gray_lossless_image_recompress_quality;

    /* Bitonal images */
    int bitonal_image_subsample_method;
    int bitonal_image_subsample_threshold;
    int bitonal_image_subsample_to;
    int bitonal_image_recompress_method;
    char *bitonal_image_recompress_quality;
} pdf_image_rewriter_options;

/**
 * Image rewrite statistics.
 */
typedef struct {
    int images_processed;       /* Total images processed */
    int images_subsampled;      /* Images that were subsampled */
    int images_recompressed;    /* Images that were recompressed */
    int images_unchanged;       /* Images left unchanged */
    uint64_t original_size;     /* Original total size in bytes */
    uint64_t new_size;          /* New total size in bytes */
    int color_images;           /* Color images processed */
    int gray_images;            /* Grayscale images processed */
    int bitonal_images;         /* Bitonal images processed */
} pdf_image_rewrite_stats;

/* ============================================================================
 * Default Options
 * ============================================================================ */

/**
 * Get default image rewriter options.
 */
pdf_image_rewriter_options pdf_default_image_rewriter_options(void);

/**
 * Get web-optimized options (72 DPI, JPEG).
 */
pdf_image_rewriter_options pdf_web_image_rewriter_options(void);

/**
 * Get print quality options (300 DPI).
 */
pdf_image_rewriter_options pdf_print_image_rewriter_options(void);

/**
 * Get ebook quality options (150 DPI).
 */
pdf_image_rewriter_options pdf_ebook_image_rewriter_options(void);

/**
 * Get maximum compression options.
 */
pdf_image_rewriter_options pdf_max_compression_image_rewriter_options(void);

/* ============================================================================
 * Option Setters
 * ============================================================================ */

/**
 * Set color image subsample threshold.
 * @param threshold_dpi DPI threshold (subsample if image DPI > threshold)
 * @param target_dpi Target DPI after subsampling
 * @param method Subsample method (FZ_SUBSAMPLE_*)
 */
void pdf_set_color_subsample(pdf_image_rewriter_options *opts, int threshold_dpi, int target_dpi, int method);

/**
 * Set grayscale image subsample threshold.
 */
void pdf_set_gray_subsample(pdf_image_rewriter_options *opts, int threshold_dpi, int target_dpi, int method);

/**
 * Set bitonal image subsample threshold.
 */
void pdf_set_bitonal_subsample(pdf_image_rewriter_options *opts, int threshold_dpi, int target_dpi, int method);

/**
 * Set color image recompression method.
 */
void pdf_set_color_recompress(pdf_image_rewriter_options *opts, int method);

/**
 * Set grayscale image recompression method.
 */
void pdf_set_gray_recompress(pdf_image_rewriter_options *opts, int method);

/**
 * Set bitonal image recompression method.
 */
void pdf_set_bitonal_recompress(pdf_image_rewriter_options *opts, int method);

/**
 * Set JPEG quality for color images.
 * @param quality Quality string (e.g., "75" or "high")
 */
void pdf_set_color_jpeg_quality(pdf_image_rewriter_options *opts, const char *quality);

/**
 * Set JPEG quality for grayscale images.
 */
void pdf_set_gray_jpeg_quality(pdf_image_rewriter_options *opts, const char *quality);

/* ============================================================================
 * Main Rewrite Functions
 * ============================================================================ */

/**
 * Rewrite images within the given document.
 */
void pdf_rewrite_images(fz_context *ctx, pdf_document *doc, pdf_image_rewriter_options *opts);

/**
 * Rewrite images and return statistics.
 */
pdf_image_rewrite_stats pdf_rewrite_images_with_stats(fz_context *ctx, pdf_document *doc, pdf_image_rewriter_options *opts);

/* ============================================================================
 * Image Analysis
 * ============================================================================ */

/**
 * Count images in document.
 */
int pdf_count_images(fz_context *ctx, pdf_document *doc);

/**
 * Get total image size in bytes.
 */
uint64_t pdf_get_total_image_size(fz_context *ctx, pdf_document *doc);

/**
 * Analyze images and return statistics without modifying.
 */
pdf_image_rewrite_stats pdf_analyze_images(fz_context *ctx, pdf_document *doc);

/* ============================================================================
 * Cleanup
 * ============================================================================ */

/**
 * Free resources in image rewriter options.
 */
void pdf_drop_image_rewriter_options(pdf_image_rewriter_options *opts);

#ifdef __cplusplus
}
#endif

#endif /* MICROPDF_PDF_IMAGE_REWRITER_H */


