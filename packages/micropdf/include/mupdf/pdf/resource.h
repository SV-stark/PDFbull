/*
 * PDF Resource FFI
 *
 * Provides PDF resource management including fonts, images, colorspaces,
 * patterns, shadings, functions, and XObjects.
 */

#ifndef MICROPDF_PDF_RESOURCE_H
#define MICROPDF_PDF_RESOURCE_H

#include <stdint.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Handle types */
typedef uint64_t fz_context;
typedef uint64_t pdf_document;
typedef uint64_t pdf_obj;
typedef uint64_t fz_font;
typedef uint64_t fz_image;
typedef uint64_t fz_colorspace;
typedef uint64_t fz_shade;
typedef uint64_t fz_stream;
typedef uint64_t fz_buffer;
typedef uint64_t pdf_resource_stack;
typedef uint64_t pdf_pattern;
typedef uint64_t pdf_function;

/* ============================================================================
 * Font Resource Constants
 * ============================================================================ */

#define PDF_SIMPLE_FONT_RESOURCE    1
#define PDF_CID_FONT_RESOURCE       2
#define PDF_CJK_FONT_RESOURCE       3

#define PDF_SIMPLE_ENCODING_LATIN       0
#define PDF_SIMPLE_ENCODING_GREEK       1
#define PDF_SIMPLE_ENCODING_CYRILLIC    2

/* ============================================================================
 * Resource Key Structures
 * ============================================================================ */

/** Font resource key for lookup/caching */
typedef struct {
    unsigned char digest[16];   /* MD5 digest */
    int type;                   /* Font type */
    int encoding;               /* Encoding type */
    int local_xref;             /* Local xref flag */
} pdf_font_resource_key;

/** Colorspace resource key for lookup/caching */
typedef struct {
    unsigned char digest[16];   /* MD5 digest */
    int local_xref;             /* Local xref flag */
} pdf_colorspace_resource_key;

/* ============================================================================
 * Store Operations
 * ============================================================================ */

/**
 * Store an item in the PDF store.
 */
void pdf_store_item(fz_context *ctx, pdf_obj *key, void *val, size_t itemsize);

/**
 * Find an item in the PDF store.
 * @return Item pointer or NULL if not found
 */
void *pdf_find_item(fz_context *ctx, void *drop, pdf_obj *key);

/**
 * Remove an item from the PDF store.
 */
void pdf_remove_item(fz_context *ctx, void *drop, pdf_obj *key);

/**
 * Empty the document's store.
 */
void pdf_empty_store(fz_context *ctx, pdf_document *doc);

/**
 * Purge locals from the store.
 */
void pdf_purge_locals_from_store(fz_context *ctx, pdf_document *doc);

/**
 * Purge specific object from the store.
 */
void pdf_purge_object_from_store(fz_context *ctx, pdf_document *doc, int num);

/* ============================================================================
 * Font Resource Functions
 * ============================================================================ */

/**
 * Find a font resource by digest.
 * @param type Font type (PDF_*_FONT_RESOURCE)
 * @param encoding Encoding type
 * @param item Font handle
 * @param key Key structure to fill
 * @return PDF object handle or 0 if not found
 */
pdf_obj *pdf_find_font_resource(fz_context *ctx, pdf_document *doc, int type, int encoding, fz_font *item, pdf_font_resource_key *key);

/**
 * Insert a font resource.
 * @return The inserted object handle
 */
pdf_obj *pdf_insert_font_resource(fz_context *ctx, pdf_document *doc, pdf_font_resource_key *key, pdf_obj *obj);

/* ============================================================================
 * Colorspace Resource Functions
 * ============================================================================ */

/**
 * Find a colorspace resource by digest.
 * @return PDF object handle or 0 if not found
 */
pdf_obj *pdf_find_colorspace_resource(fz_context *ctx, pdf_document *doc, fz_colorspace *item, pdf_colorspace_resource_key *key);

/**
 * Insert a colorspace resource.
 * @return The inserted object handle
 */
pdf_obj *pdf_insert_colorspace_resource(fz_context *ctx, pdf_document *doc, pdf_colorspace_resource_key *key, pdf_obj *obj);

/**
 * Drop resource tables for a document.
 */
void pdf_drop_resource_tables(fz_context *ctx, pdf_document *doc);

/**
 * Purge local resources.
 */
void pdf_purge_local_resources(fz_context *ctx, pdf_document *doc);

/* ============================================================================
 * Resource Stack Functions
 * ============================================================================ */

/**
 * Create a new resource stack.
 * @param resources Resources dictionary handle
 * @return Stack handle
 */
pdf_resource_stack *pdf_new_resource_stack(fz_context *ctx, pdf_obj *resources);

/**
 * Push a resource stack entry.
 * @return New stack handle
 */
pdf_resource_stack *pdf_push_resource_stack(fz_context *ctx, pdf_resource_stack *stack, pdf_obj *resources);

/**
 * Pop a resource stack entry.
 * @return Parent stack handle
 */
pdf_resource_stack *pdf_pop_resource_stack(fz_context *ctx, pdf_resource_stack *stack);

/**
 * Drop a resource stack.
 */
void pdf_drop_resource_stack(fz_context *ctx, pdf_resource_stack *stack);

/**
 * Lookup a resource in the stack.
 * @return Resource object handle or 0 if not found
 */
pdf_obj *pdf_lookup_resource(fz_context *ctx, pdf_resource_stack *stack, pdf_obj *type, const char *name);

/* ============================================================================
 * Function Types and Functions
 * ============================================================================ */

/**
 * Load a PDF function.
 * @param ref Function reference object
 * @param in Number of input values
 * @param out Number of output values
 * @return Function handle
 */
pdf_function *pdf_load_function(fz_context *ctx, pdf_obj *ref, int in, int out);

/**
 * Keep a function.
 */
pdf_function *pdf_keep_function(fz_context *ctx, pdf_function *func);

/**
 * Drop a function.
 */
void pdf_drop_function(fz_context *ctx, pdf_function *func);

/**
 * Get function size in memory.
 */
size_t pdf_function_size(fz_context *ctx, pdf_function *func);

/**
 * Evaluate a function.
 * @param in Input values array
 * @param inlen Number of input values
 * @param out Output values array
 * @param outlen Number of output values
 */
void pdf_eval_function(fz_context *ctx, pdf_function *func, const float *in, int inlen, float *out, int outlen);

/* ============================================================================
 * Pattern Functions
 * ============================================================================ */

/**
 * Load a pattern.
 * @return Pattern handle
 */
pdf_pattern *pdf_load_pattern(fz_context *ctx, pdf_document *doc, pdf_obj *obj);

/**
 * Keep a pattern.
 */
pdf_pattern *pdf_keep_pattern(fz_context *ctx, pdf_pattern *pat);

/**
 * Drop a pattern.
 */
void pdf_drop_pattern(fz_context *ctx, pdf_pattern *pat);

/**
 * Check if pattern is a mask.
 */
int pdf_pattern_is_mask(fz_context *ctx, pdf_pattern *pat);

/**
 * Get pattern X step.
 */
float pdf_pattern_xstep(fz_context *ctx, pdf_pattern *pat);

/**
 * Get pattern Y step.
 */
float pdf_pattern_ystep(fz_context *ctx, pdf_pattern *pat);

/* ============================================================================
 * Colorspace Functions
 * ============================================================================ */

/**
 * Load a colorspace from PDF object.
 */
fz_colorspace *pdf_load_colorspace(fz_context *ctx, pdf_obj *obj);

/**
 * Get document output intent colorspace.
 */
fz_colorspace *pdf_document_output_intent(fz_context *ctx, pdf_document *doc);

/**
 * Check if colorspace is a tint colorspace.
 */
int pdf_is_tint_colorspace(fz_context *ctx, fz_colorspace *cs);

/**
 * Guess number of colorspace components.
 */
int pdf_guess_colorspace_components(fz_context *ctx, pdf_obj *obj);

/* ============================================================================
 * Shading Functions
 * ============================================================================ */

/**
 * Load a shading from PDF object.
 */
fz_shade *pdf_load_shading(fz_context *ctx, pdf_document *doc, pdf_obj *obj);

/**
 * Sample shade function values.
 */
void pdf_sample_shade_function(fz_context *ctx, float *samples, int n, int funcs, pdf_function **func, float t0, float t1);

/* ============================================================================
 * Image Functions
 * ============================================================================ */

/**
 * Load an image from PDF object.
 */
fz_image *pdf_load_image(fz_context *ctx, pdf_document *doc, pdf_obj *obj);

/**
 * Load an inline image.
 */
fz_image *pdf_load_inline_image(fz_context *ctx, pdf_document *doc, pdf_resource_stack *rdb, pdf_obj *dict, fz_stream *file);

/**
 * Check if image is JPX format.
 */
int pdf_is_jpx_image(fz_context *ctx, pdf_obj *dict);

/**
 * Add an image to document.
 */
pdf_obj *pdf_add_image(fz_context *ctx, pdf_document *doc, fz_image *image);

/**
 * Add a colorspace to document.
 */
pdf_obj *pdf_add_colorspace(fz_context *ctx, pdf_document *doc, fz_colorspace *cs);

/* ============================================================================
 * XObject Functions
 * ============================================================================ */

/**
 * Create a new XObject.
 * @param bbox Bounding box [x0, y0, x1, y1]
 * @param matrix Transformation matrix [a, b, c, d, e, f]
 * @param res Resources dictionary
 * @param buffer Content stream buffer
 * @return XObject reference
 */
pdf_obj *pdf_new_xobject(fz_context *ctx, pdf_document *doc, const float *bbox, const float *matrix, pdf_obj *res, fz_buffer *buffer);

/**
 * Update an XObject.
 */
void pdf_update_xobject(fz_context *ctx, pdf_document *doc, pdf_obj *xobj, const float *bbox, const float *matrix, pdf_obj *res, fz_buffer *buffer);

/**
 * Get XObject resources dictionary.
 */
pdf_obj *pdf_xobject_resources(fz_context *ctx, pdf_obj *xobj);

/**
 * Get XObject bounding box.
 * @param bbox Output array for [x0, y0, x1, y1]
 */
void pdf_xobject_bbox(fz_context *ctx, pdf_obj *xobj, float *bbox);

/**
 * Get XObject transformation matrix.
 * @param matrix Output array for [a, b, c, d, e, f]
 */
void pdf_xobject_matrix(fz_context *ctx, pdf_obj *xobj, float *matrix);

/**
 * Check if XObject is isolated.
 */
int pdf_xobject_isolated(fz_context *ctx, pdf_obj *xobj);

/**
 * Check if XObject has knockout.
 */
int pdf_xobject_knockout(fz_context *ctx, pdf_obj *xobj);

/**
 * Check if XObject has transparency.
 */
int pdf_xobject_transparency(fz_context *ctx, pdf_obj *xobj);

/**
 * Get XObject colorspace.
 */
fz_colorspace *pdf_xobject_colorspace(fz_context *ctx, pdf_obj *xobj);

#ifdef __cplusplus
}
#endif

#endif /* MICROPDF_PDF_RESOURCE_H */


