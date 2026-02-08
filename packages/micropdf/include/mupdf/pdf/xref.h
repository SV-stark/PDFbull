/*
 * PDF Cross-Reference Table FFI
 *
 * Provides support for PDF cross-reference table operations, including
 * object management, stream handling, and document structure.
 */

#ifndef MICROPDF_PDF_XREF_H
#define MICROPDF_PDF_XREF_H

#include <stdint.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Handle types */
typedef uint64_t fz_context;
typedef uint64_t pdf_document;
typedef uint64_t pdf_xref;
typedef uint64_t pdf_obj;
typedef uint64_t fz_buffer;

/* ============================================================================
 * Xref Entry Type Constants
 * ============================================================================ */

#define PDF_XREF_FREE       0
#define PDF_XREF_INUSE      1
#define PDF_XREF_OBJSTM     2
#define PDF_XREF_COMPRESSED 3

/* ============================================================================
 * Xref Entry Structure
 * ============================================================================ */

/**
 * A cross-reference table entry.
 */
typedef struct {
    int entry_type;     /* Entry type (free, in-use, objstm) */
    int marked;         /* Marked flag (for garbage collection) */
    uint16_t generation; /* Generation number or object stream index */
    int num;            /* Object number */
    int64_t offset;     /* File offset or object stream number */
    int64_t stm_offset; /* Stream offset (on-disk) */
    int has_stm_buf;    /* Has in-memory stream buffer */
    int has_obj;        /* Has cached object */
} pdf_xref_entry;

/* ============================================================================
 * Xref Management
 * ============================================================================ */

/**
 * Create a new xref table for a document.
 */
pdf_xref *pdf_new_xref(fz_context *ctx, pdf_document *doc);

/**
 * Drop an xref table.
 */
void pdf_drop_xref(fz_context *ctx, pdf_xref *xref);

/**
 * Get the number of objects in the xref.
 */
int pdf_xref_len(fz_context *ctx, pdf_xref *xref);

/**
 * Count objects in a document.
 */
int pdf_count_objects(fz_context *ctx, pdf_xref *xref);

/**
 * Get the PDF version.
 * @return Version (e.g., 17 for 1.7, 20 for 2.0)
 */
int pdf_version(fz_context *ctx, pdf_xref *xref);

/**
 * Set the PDF version.
 * @return 1 on success, 0 on failure
 */
int pdf_set_version(fz_context *ctx, pdf_xref *xref, int version);

/* ============================================================================
 * Object Management
 * ============================================================================ */

/**
 * Create a new object and return its number.
 * @return Object number or -1 on error
 */
int pdf_create_object(fz_context *ctx, pdf_xref *xref);

/**
 * Delete an object.
 */
void pdf_delete_object(fz_context *ctx, pdf_xref *xref, int num);

/**
 * Check if an object exists.
 * @return 1 if exists, 0 if not
 */
int pdf_object_exists(fz_context *ctx, pdf_xref *xref, int num);

/**
 * Update an object in the xref.
 * @return 1 on success, 0 on failure
 */
int pdf_update_object(fz_context *ctx, pdf_xref *xref, int num, pdf_obj *obj);

/**
 * Cache an object.
 * @return 1 if cached, 0 if not
 */
int pdf_cache_object(fz_context *ctx, pdf_xref *xref, int num);

/**
 * Get a cached object.
 * @return Object handle or 0 if not cached
 */
pdf_obj *pdf_get_cached_object(fz_context *ctx, pdf_xref *xref, int num);

/* ============================================================================
 * Xref Entry Access
 * ============================================================================ */

/**
 * Get xref entry info.
 * @param entry_out Pointer to receive entry data
 * @return 1 on success, 0 on failure
 */
int pdf_get_xref_entry(fz_context *ctx, pdf_xref *xref, int num, pdf_xref_entry *entry_out);

/**
 * Add a subsection to the xref.
 * @return 1 on success, 0 on failure
 */
int pdf_xref_add_subsection(fz_context *ctx, pdf_xref *xref, int start, int count);

/**
 * Set an xref entry.
 * @return 1 on success, 0 on failure
 */
int pdf_xref_set_entry(fz_context *ctx, pdf_xref *xref, int num, int entry_type, uint16_t generation, int64_t offset);

/**
 * Mark an entry for garbage collection.
 * @return 1 on success, 0 on failure
 */
int pdf_mark_xref(fz_context *ctx, pdf_xref *xref, int num);

/**
 * Clear all marks.
 */
void pdf_clear_xref_marks(fz_context *ctx, pdf_xref *xref);

/* ============================================================================
 * Trailer
 * ============================================================================ */

/**
 * Get the trailer dictionary.
 */
pdf_obj *pdf_trailer(fz_context *ctx, pdf_xref *xref);

/**
 * Set the trailer dictionary.
 * @return 1 on success, 0 on failure
 */
int pdf_set_trailer(fz_context *ctx, pdf_xref *xref, pdf_obj *trailer);

/* ============================================================================
 * Stream Operations
 * ============================================================================ */

/**
 * Update stream contents.
 * @param compressed 0 for uncompressed, non-zero for compressed
 * @return 1 on success, 0 on failure
 */
int pdf_update_stream(fz_context *ctx, pdf_xref *xref, int num, fz_buffer *buffer, int compressed);

/**
 * Get cached stream buffer.
 * @return Buffer handle or 0 if not cached
 */
fz_buffer *pdf_get_stream_buffer(fz_context *ctx, pdf_xref *xref, int num);

/**
 * Check if object is a local object.
 * @return 1 if local, 0 if not
 */
int pdf_is_local_object(fz_context *ctx, pdf_xref *xref, int num);

/* ============================================================================
 * Utility Functions
 * ============================================================================ */

/**
 * Get entry type as string.
 * @return Type string ("f", "n", "o", "c") (caller must free) or NULL
 */
char *pdf_xref_entry_type_string(fz_context *ctx, int entry_type);

/**
 * Free a string.
 */
void pdf_xref_free_string(char *s);

/**
 * Get end offset.
 */
int64_t pdf_xref_end_offset(fz_context *ctx, pdf_xref *xref);

/**
 * Set end offset.
 * @return 1 on success, 0 on failure
 */
int pdf_xref_set_end_offset(fz_context *ctx, pdf_xref *xref, int64_t offset);

#ifdef __cplusplus
}
#endif

#endif /* MICROPDF_PDF_XREF_H */

