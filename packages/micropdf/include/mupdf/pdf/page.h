/*
 * PDF Page FFI
 *
 * Provides page loading, manipulation, and rendering capabilities for PDF documents.
 */

#ifndef MICROPDF_PDF_PAGE_H
#define MICROPDF_PDF_PAGE_H

#include <stdint.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Handle types */
typedef uint64_t fz_context;
typedef uint64_t pdf_document;
typedef uint64_t pdf_page;
typedef uint64_t pdf_obj;
typedef uint64_t fz_device;
typedef uint64_t fz_cookie;
typedef uint64_t fz_colorspace;
typedef uint64_t fz_separations;
typedef uint64_t fz_link;
typedef uint64_t pdf_annot;
typedef uint64_t fz_pixmap;
typedef uint64_t fz_transition;
typedef uint64_t fz_default_colorspaces;

/* Geometry types */
typedef struct {
    float x0, y0, x1, y1;
} fz_rect;

typedef struct {
    float a, b, c, d, e, f;
} fz_matrix;

/* Box types */
typedef enum {
    FZ_MEDIA_BOX = 0,
    FZ_CROP_BOX = 1,
    FZ_BLEED_BOX = 2,
    FZ_TRIM_BOX = 3,
    FZ_ART_BOX = 4,
    FZ_UNKNOWN_BOX = 5
} fz_box_type;

/* Redaction options */
typedef enum {
    PDF_REDACT_IMAGE_NONE = 0,
    PDF_REDACT_IMAGE_REMOVE = 1,
    PDF_REDACT_IMAGE_PIXELS = 2,
    PDF_REDACT_IMAGE_REMOVE_UNLESS_INVISIBLE = 3
} pdf_redact_image_method;

typedef enum {
    PDF_REDACT_LINE_ART_NONE = 0,
    PDF_REDACT_LINE_ART_REMOVE_IF_COVERED = 1,
    PDF_REDACT_LINE_ART_REMOVE_IF_TOUCHED = 2
} pdf_redact_line_art_method;

typedef enum {
    PDF_REDACT_TEXT_REMOVE = 0,
    PDF_REDACT_TEXT_NONE = 1,
    PDF_REDACT_TEXT_REMOVE_INVISIBLE = 2
} pdf_redact_text_method;

typedef struct {
    int black_boxes;
    int image_method;
    int line_art;
    int text;
} pdf_redact_options;

/* ============================================================================
 * Page Lifecycle
 * ============================================================================ */

/**
 * Load a page from a PDF document.
 * @param ctx Context handle
 * @param doc Document handle
 * @param number Page number (0-based)
 * @return Page handle, or 0 on failure
 */
pdf_page *pdf_load_page(fz_context *ctx, pdf_document *doc, int number);

/**
 * Keep (increment reference count) a page.
 */
pdf_page *pdf_keep_page(fz_context *ctx, pdf_page *page);

/**
 * Drop (decrement reference count) a page.
 */
void pdf_drop_page(fz_context *ctx, pdf_page *page);

/* ============================================================================
 * Page Count and Lookup
 * ============================================================================ */

/**
 * Count the number of pages in a document.
 */
int pdf_count_pages(fz_context *ctx, pdf_document *doc);

/**
 * Lookup the page number for a page object.
 * @return Page number (0-based), or -1 if not found
 */
int pdf_lookup_page_number(fz_context *ctx, pdf_document *doc, pdf_obj *pageobj);

/**
 * Lookup a page object by page number.
 */
pdf_obj *pdf_lookup_page_obj(fz_context *ctx, pdf_document *doc, int number);

/* ============================================================================
 * Page Properties
 * ============================================================================ */

/** Get the page's PDF object */
pdf_obj *pdf_page_obj(fz_context *ctx, pdf_page *page);

/** Get the page's resources dictionary */
pdf_obj *pdf_page_resources(fz_context *ctx, pdf_page *page);

/** Get the page's content stream */
pdf_obj *pdf_page_contents(fz_context *ctx, pdf_page *page);

/** Get the page's transparency group */
pdf_obj *pdf_page_group(fz_context *ctx, pdf_page *page);

/** Check if page has transparency */
int pdf_page_has_transparency(fz_context *ctx, pdf_page *page);

/** Get page rotation (0, 90, 180, 270) */
int pdf_page_rotation(fz_context *ctx, pdf_page *page);

/** Get page user unit */
float pdf_page_user_unit(fz_context *ctx, pdf_page *page);

/* ============================================================================
 * Page Bounds and Transform
 * ============================================================================ */

/**
 * Get the bounds of a page.
 * @param box Box type (FZ_MEDIA_BOX, FZ_CROP_BOX, etc.)
 */
fz_rect pdf_bound_page(fz_context *ctx, pdf_page *page, fz_box_type box);

/**
 * Get the page transformation matrix.
 * @param mediabox Output for the media box (may be NULL)
 * @param ctm Output for the transformation matrix (may be NULL)
 */
void pdf_page_transform(fz_context *ctx, pdf_page *page, fz_rect *mediabox, fz_matrix *ctm);

/**
 * Get the page transformation matrix for a specific box type.
 */
void pdf_page_transform_box(fz_context *ctx, pdf_page *page, fz_rect *outbox, fz_matrix *outctm, fz_box_type box);

/** Get page object transformation */
void pdf_page_obj_transform(fz_context *ctx, pdf_obj *pageobj, fz_rect *outbox, fz_matrix *outctm);

/** Get page object transformation for a specific box type */
void pdf_page_obj_transform_box(fz_context *ctx, pdf_obj *pageobj, fz_rect *outbox, fz_matrix *outctm, fz_box_type box);

/* ============================================================================
 * Page Box Manipulation
 * ============================================================================ */

/**
 * Set a page box.
 * @param box Box type
 * @param rect New box rectangle
 */
void pdf_set_page_box(fz_context *ctx, pdf_page *page, fz_box_type box, fz_rect rect);

/** Get box type from string name */
fz_box_type fz_box_type_from_string(const char *name);

/** Get string name from box type */
const char *fz_string_from_box_type(fz_box_type box);

/* ============================================================================
 * Page Rendering
 * ============================================================================ */

/**
 * Run page contents on a device.
 * Renders the complete page including annotations and widgets.
 */
void pdf_run_page(fz_context *ctx, pdf_page *page, fz_device *dev, fz_matrix ctm, fz_cookie *cookie);

/** Run page with usage (View, Print, Export) */
void pdf_run_page_with_usage(fz_context *ctx, pdf_page *page, fz_device *dev, fz_matrix ctm, const char *usage, fz_cookie *cookie);

/** Run only page contents (no annotations) */
void pdf_run_page_contents(fz_context *ctx, pdf_page *page, fz_device *dev, fz_matrix ctm, fz_cookie *cookie);

/** Run page annotations */
void pdf_run_page_annots(fz_context *ctx, pdf_page *page, fz_device *dev, fz_matrix ctm, fz_cookie *cookie);

/** Run page widgets (form fields) */
void pdf_run_page_widgets(fz_context *ctx, pdf_page *page, fz_device *dev, fz_matrix ctm, fz_cookie *cookie);

/** Run page contents with usage */
void pdf_run_page_contents_with_usage(fz_context *ctx, pdf_page *page, fz_device *dev, fz_matrix ctm, const char *usage, fz_cookie *cookie);

/** Run page annotations with usage */
void pdf_run_page_annots_with_usage(fz_context *ctx, pdf_page *page, fz_device *dev, fz_matrix ctm, const char *usage, fz_cookie *cookie);

/** Run page widgets with usage */
void pdf_run_page_widgets_with_usage(fz_context *ctx, pdf_page *page, fz_device *dev, fz_matrix ctm, const char *usage, fz_cookie *cookie);

/* ============================================================================
 * Links
 * ============================================================================ */

/** Load links from a page */
fz_link *pdf_load_links(fz_context *ctx, pdf_page *page);

/* ============================================================================
 * Separations
 * ============================================================================ */

/** Get page separations (spot colors) */
fz_separations *pdf_page_separations(fz_context *ctx, pdf_page *page);

/* ============================================================================
 * Page Tree
 * ============================================================================ */

/** Enable or disable page tree cache */
void pdf_set_page_tree_cache(fz_context *ctx, pdf_document *doc, int enabled);

/** Load page tree (no-op, loaded on demand) */
void pdf_load_page_tree(fz_context *ctx, pdf_document *doc);

/** Drop page tree (no-op) */
void pdf_drop_page_tree(fz_context *ctx, pdf_document *doc);

/** Internal: Drop page tree */
void pdf_drop_page_tree_internal(fz_context *ctx, pdf_document *doc);

/** Flatten inheritable page items */
void pdf_flatten_inheritable_page_items(fz_context *ctx, pdf_obj *page);

/* ============================================================================
 * Page Presentation
 * ============================================================================ */

/** Get page presentation (transition) info */
fz_transition *pdf_page_presentation(fz_context *ctx, pdf_page *page, fz_transition *transition, float *duration);

/* ============================================================================
 * Default Colorspaces
 * ============================================================================ */

/** Load default colorspaces for a page */
fz_default_colorspaces *pdf_load_default_colorspaces(fz_context *ctx, pdf_document *doc, pdf_page *page);

/** Update default colorspaces from resources */
fz_default_colorspaces *pdf_update_default_colorspaces(fz_context *ctx, fz_default_colorspaces *old_cs, pdf_obj *res);

/* ============================================================================
 * Page Filtering
 * ============================================================================ */

/** Filter page contents */
void pdf_filter_page_contents(fz_context *ctx, pdf_document *doc, pdf_page *page, void *options);

/** Filter annotation contents */
void pdf_filter_annot_contents(fz_context *ctx, pdf_document *doc, pdf_annot *annot, void *options);

/* ============================================================================
 * Pixmap Creation
 * ============================================================================ */

/** Create pixmap from page contents */
fz_pixmap *pdf_new_pixmap_from_page_contents_with_usage(
    fz_context *ctx, pdf_page *page, fz_matrix ctm,
    fz_colorspace *cs, int alpha, const char *usage, fz_box_type box);

/** Create pixmap from page (including annotations) */
fz_pixmap *pdf_new_pixmap_from_page_with_usage(
    fz_context *ctx, pdf_page *page, fz_matrix ctm,
    fz_colorspace *cs, int alpha, const char *usage, fz_box_type box);

/** Create pixmap from page contents with separations */
fz_pixmap *pdf_new_pixmap_from_page_contents_with_separations_and_usage(
    fz_context *ctx, pdf_page *page, fz_matrix ctm,
    fz_colorspace *cs, fz_separations *seps, int alpha, const char *usage, fz_box_type box);

/** Create pixmap from page with separations */
fz_pixmap *pdf_new_pixmap_from_page_with_separations_and_usage(
    fz_context *ctx, pdf_page *page, fz_matrix ctm,
    fz_colorspace *cs, fz_separations *seps, int alpha, const char *usage, fz_box_type box);

/* ============================================================================
 * Redaction
 * ============================================================================ */

/**
 * Redact page content.
 * @return 1 if content was changed, 0 otherwise
 */
int pdf_redact_page(fz_context *ctx, pdf_document *doc, pdf_page *page, pdf_redact_options *opts);

/* ============================================================================
 * Page Clipping and Vectorization
 * ============================================================================ */

/** Clip page content to a rectangle */
void pdf_clip_page(fz_context *ctx, pdf_page *page, fz_rect *clip);

/** Vectorize page content (convert text to paths) */
void pdf_vectorize_page(fz_context *ctx, pdf_page *page);

/* ============================================================================
 * Page Synchronization
 * ============================================================================ */

/** Sync all open pages with document */
void pdf_sync_open_pages(fz_context *ctx, pdf_document *doc);

/** Sync a single page */
void pdf_sync_page(fz_context *ctx, pdf_page *page);

/** Sync page links */
void pdf_sync_links(fz_context *ctx, pdf_page *page);

/** Sync page annotations */
void pdf_sync_annots(fz_context *ctx, pdf_page *page);

/** Nuke (invalidate) a page */
void pdf_nuke_page(fz_context *ctx, pdf_page *page);

/** Nuke page links */
void pdf_nuke_links(fz_context *ctx, pdf_page *page);

/** Nuke page annotations */
void pdf_nuke_annots(fz_context *ctx, pdf_page *page);

#ifdef __cplusplus
}
#endif

#endif /* MICROPDF_PDF_PAGE_H */

