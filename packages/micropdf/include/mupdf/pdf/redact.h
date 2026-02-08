/*
 * PDF Redaction FFI
 *
 * Provides PDF redaction functionality including redaction annotations,
 * content removal, image handling, and metadata sanitization.
 */

#ifndef MICROPDF_PDF_REDACT_H
#define MICROPDF_PDF_REDACT_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Handle types */
typedef uint64_t fz_context;
typedef uint64_t pdf_document;
typedef uint64_t pdf_page;
typedef uint64_t pdf_annot;
typedef uint64_t pdf_redact_context;

/* ============================================================================
 * Image Redaction Methods
 * ============================================================================ */

#define PDF_REDACT_IMAGE_NONE                   0   /* No image changes */
#define PDF_REDACT_IMAGE_REMOVE                 1   /* Remove intruding images */
#define PDF_REDACT_IMAGE_PIXELS                 2   /* Black out intruding pixels */
#define PDF_REDACT_IMAGE_REMOVE_UNLESS_INVISIBLE 3  /* Remove unless invisible */

/* ============================================================================
 * Line Art Redaction Methods
 * ============================================================================ */

#define PDF_REDACT_LINE_ART_NONE                0   /* No line art changes */
#define PDF_REDACT_LINE_ART_REMOVE_IF_COVERED   1   /* Remove if fully covered */
#define PDF_REDACT_LINE_ART_REMOVE_IF_TOUCHED   2   /* Remove if touched */

/* ============================================================================
 * Text Redaction Methods
 * ============================================================================ */

#define PDF_REDACT_TEXT_REMOVE                  0   /* Remove overlapping text (secure) */
#define PDF_REDACT_TEXT_NONE                    1   /* No text removal (INSECURE) */
#define PDF_REDACT_TEXT_REMOVE_INVISIBLE        2   /* Remove invisible text only */

/* ============================================================================
 * Redaction Options
 * ============================================================================ */

/** Redaction options */
typedef struct {
    int black_boxes;        /* Draw black boxes over redacted areas */
    int image_method;       /* Image handling method */
    int line_art;           /* Line art handling method */
    int text;               /* Text handling method */
} pdf_redact_options;

/** Redaction statistics */
typedef struct {
    int regions_applied;    /* Number of regions applied */
    int text_removed;       /* Number of text objects removed */
    int images_removed;     /* Number of images removed */
    int images_modified;    /* Number of images modified */
    int line_art_removed;   /* Number of line art objects removed */
    int annotations_removed;/* Number of annotations removed */
} pdf_redact_stats;

/* ============================================================================
 * Default Options
 * ============================================================================ */

/**
 * Get default redaction options.
 */
pdf_redact_options pdf_default_redact_options(void);

/**
 * Get secure redaction options (most aggressive).
 */
pdf_redact_options pdf_secure_redact_options(void);

/**
 * Get OCR-only redaction options (invisible text only).
 */
pdf_redact_options pdf_ocr_redact_options(void);

/* ============================================================================
 * Redaction Context
 * ============================================================================ */

/**
 * Create a new redaction context.
 * @return Redaction context handle
 */
pdf_redact_context *pdf_new_redact_context(fz_context *ctx, pdf_document *doc, pdf_page *page);

/**
 * Drop a redaction context.
 */
void pdf_drop_redact_context(fz_context *ctx, pdf_redact_context *redact_ctx);

/**
 * Set redaction options.
 */
void pdf_set_redact_options(fz_context *ctx, pdf_redact_context *redact_ctx, pdf_redact_options opts);

/* ============================================================================
 * Redaction Regions
 * ============================================================================ */

/**
 * Add a redaction region.
 */
void pdf_add_redact_region(fz_context *ctx, pdf_redact_context *redact_ctx, float x0, float y0, float x1, float y1);

/**
 * Add a redaction region with color.
 * @param r, g, b Color components (0.0-1.0)
 */
void pdf_add_redact_region_with_color(fz_context *ctx, pdf_redact_context *redact_ctx, float x0, float y0, float x1, float y1, float r, float g, float b);

/**
 * Get number of redaction regions.
 */
int pdf_count_redact_regions(fz_context *ctx, pdf_redact_context *redact_ctx);

/**
 * Clear all redaction regions.
 */
void pdf_clear_redact_regions(fz_context *ctx, pdf_redact_context *redact_ctx);

/* ============================================================================
 * Apply Redactions
 * ============================================================================ */

/**
 * Apply all redactions in the context.
 * @return Number of regions applied
 */
int pdf_apply_redactions(fz_context *ctx, pdf_redact_context *redact_ctx);

/**
 * Redact a page with options (applies all redaction annotations).
 * @return Number of redactions applied
 */
int pdf_redact_page_annotations(fz_context *ctx, pdf_document *doc, pdf_page *page, const pdf_redact_options *opts);

/**
 * Apply a single redaction annotation.
 * @return 1 on success, 0 on failure
 */
int pdf_apply_redaction(fz_context *ctx, pdf_annot *annot, const pdf_redact_options *opts);

/* ============================================================================
 * Statistics
 * ============================================================================ */

/**
 * Get redaction statistics.
 */
pdf_redact_stats pdf_get_redact_stats(fz_context *ctx, pdf_redact_context *redact_ctx);

/* ============================================================================
 * Metadata Sanitization
 * ============================================================================ */

/**
 * Remove all metadata from document.
 * Removes: Info dict, XMP, IDs, dates, author, producer, etc.
 */
void pdf_sanitize_metadata(fz_context *ctx, pdf_document *doc);

/**
 * Remove specific metadata field.
 * @param field Field name (e.g., "Author", "Title")
 */
void pdf_remove_metadata_field(fz_context *ctx, pdf_document *doc, const char *field);

/**
 * Remove hidden content from document.
 * Removes: hidden layers, invisible text, comments, attachments, JS
 */
void pdf_remove_hidden_content(fz_context *ctx, pdf_document *doc);

/**
 * Remove document attachments.
 */
void pdf_remove_attachments(fz_context *ctx, pdf_document *doc);

/**
 * Remove document JavaScript.
 */
void pdf_remove_javascript(fz_context *ctx, pdf_document *doc);

/**
 * Remove document comments.
 */
void pdf_remove_comments(fz_context *ctx, pdf_document *doc);

/* ============================================================================
 * Redaction Annotation Creation
 * ============================================================================ */

/**
 * Create a redaction annotation on a page.
 * @return Annotation handle
 */
pdf_annot *pdf_create_redact_annot(fz_context *ctx, pdf_page *page, float x0, float y0, float x1, float y1);

/**
 * Set redaction annotation overlay color.
 */
void pdf_set_redact_annot_color(fz_context *ctx, pdf_annot *annot, float r, float g, float b);

/**
 * Set redaction annotation overlay text.
 */
void pdf_set_redact_annot_text(fz_context *ctx, pdf_annot *annot, const char *text);

/**
 * Add quad point to redaction annotation.
 * @param quad Array of 8 floats: x0,y0,x1,y1,x2,y2,x3,y3
 */
void pdf_add_redact_annot_quad(fz_context *ctx, pdf_annot *annot, const float *quad);

/* ============================================================================
 * Batch Operations
 * ============================================================================ */

/**
 * Redact all pages in document.
 * @return Total number of redactions applied
 */
int pdf_redact_document(fz_context *ctx, pdf_document *doc, const pdf_redact_options *opts);

/**
 * Apply all redaction annotations in document.
 * @return Total number of annotations applied
 */
int pdf_apply_all_redactions(fz_context *ctx, pdf_document *doc, const pdf_redact_options *opts);

#ifdef __cplusplus
}
#endif

#endif /* MICROPDF_PDF_REDACT_H */


