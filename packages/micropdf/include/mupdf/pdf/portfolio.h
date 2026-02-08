/*
 * PDF Portfolio/Collection FFI
 *
 * Provides support for PDF portfolios (packages/collections), including
 * embedded file management, collection structure, and navigator schema.
 */

#ifndef MICROPDF_PDF_PORTFOLIO_H
#define MICROPDF_PDF_PORTFOLIO_H

#include <stdint.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Handle types */
typedef uint64_t fz_context;
typedef uint64_t pdf_document;
typedef uint64_t pdf_portfolio;

/* ============================================================================
 * AF Relationship Constants
 * ============================================================================ */

#define PDF_AF_RELATIONSHIP_SOURCE           0
#define PDF_AF_RELATIONSHIP_DATA             1
#define PDF_AF_RELATIONSHIP_ALTERNATIVE      2
#define PDF_AF_RELATIONSHIP_SUPPLEMENT       3
#define PDF_AF_RELATIONSHIP_ENCRYPTED_PAYLOAD 4
#define PDF_AF_RELATIONSHIP_FORM_DATA        5
#define PDF_AF_RELATIONSHIP_SCHEMA           6
#define PDF_AF_RELATIONSHIP_UNSPECIFIED      7

/* ============================================================================
 * Collection Sort Constants
 * ============================================================================ */

#define PDF_COLLECTION_SORT_NAME        0
#define PDF_COLLECTION_SORT_MODIFIED    1
#define PDF_COLLECTION_SORT_CREATED     2
#define PDF_COLLECTION_SORT_SIZE        3
#define PDF_COLLECTION_SORT_DESCRIPTION 4

/* ============================================================================
 * Collection View Constants
 * ============================================================================ */

#define PDF_COLLECTION_VIEW_DETAILS     0
#define PDF_COLLECTION_VIEW_TILE        1
#define PDF_COLLECTION_VIEW_HIDDEN      2
#define PDF_COLLECTION_VIEW_CUSTOM      3

/* ============================================================================
 * Portfolio Management
 * ============================================================================ */

/**
 * Create a new portfolio context for a document.
 */
pdf_portfolio *pdf_new_portfolio(fz_context *ctx, pdf_document *doc);

/**
 * Drop a portfolio context.
 */
void pdf_drop_portfolio(fz_context *ctx, pdf_portfolio *portfolio);

/**
 * Check if a document is a portfolio.
 * @return 1 if portfolio, 0 if not
 */
int pdf_is_portfolio(fz_context *ctx, pdf_portfolio *portfolio);

/* ============================================================================
 * Embedded File Management
 * ============================================================================ */

/**
 * Add an embedded file to the portfolio.
 * @param ctx Context
 * @param portfolio Portfolio handle
 * @param name Filename
 * @param data File contents
 * @param len Data length
 * @param mime_type MIME type (may be NULL for default)
 * @return 1 on success, 0 on failure
 */
int pdf_portfolio_add_file(fz_context *ctx, pdf_portfolio *portfolio, const char *name, const uint8_t *data, size_t len, const char *mime_type);

/**
 * Get the number of embedded files.
 */
int pdf_portfolio_count(fz_context *ctx, pdf_portfolio *portfolio);

/**
 * Get the name of an embedded file by index.
 * @return Filename (caller must free with pdf_portfolio_free_string) or NULL
 */
char *pdf_portfolio_get_name(fz_context *ctx, pdf_portfolio *portfolio, int index);

/**
 * Get embedded file contents.
 * @param ctx Context
 * @param portfolio Portfolio handle
 * @param name Filename
 * @param len_out Pointer to receive length
 * @return Pointer to data (owned by portfolio) or NULL
 */
const uint8_t *pdf_portfolio_get_file(fz_context *ctx, pdf_portfolio *portfolio, const char *name, size_t *len_out);

/**
 * Remove an embedded file.
 * @return 1 on success, 0 if not found
 */
int pdf_portfolio_remove_file(fz_context *ctx, pdf_portfolio *portfolio, const char *name);

/* ============================================================================
 * File Parameters
 * ============================================================================ */

/**
 * Get file size.
 * @return Size in bytes or -1 on error
 */
int64_t pdf_portfolio_get_file_size(fz_context *ctx, pdf_portfolio *portfolio, const char *name);

/**
 * Get file MIME type.
 * @return MIME type string (caller must free) or NULL
 */
char *pdf_portfolio_get_mime_type(fz_context *ctx, pdf_portfolio *portfolio, const char *name);

/**
 * Set file description.
 * @return 1 on success, 0 on failure
 */
int pdf_portfolio_set_description(fz_context *ctx, pdf_portfolio *portfolio, const char *name, const char *description);

/* ============================================================================
 * Collection Schema
 * ============================================================================ */

/**
 * Add a schema field.
 * @param ctx Context
 * @param portfolio Portfolio handle
 * @param key Field key
 * @param name Display name
 * @param field_type Type: 'S'=string, 'D'=date, 'N'=number, 'F'=filename
 * @return 1 on success, 0 on failure
 */
int pdf_portfolio_add_schema_field(fz_context *ctx, pdf_portfolio *portfolio, const char *key, const char *name, char field_type);

/**
 * Get schema field count.
 */
int pdf_portfolio_schema_field_count(fz_context *ctx, pdf_portfolio *portfolio);

/**
 * Get schema field key by index.
 * @return Key string (caller must free) or NULL
 */
char *pdf_portfolio_schema_field_key(fz_context *ctx, pdf_portfolio *portfolio, int index);

/* ============================================================================
 * Collection Settings
 * ============================================================================ */

/**
 * Set the initial view mode.
 * @return 1 on success, 0 on failure
 */
int pdf_portfolio_set_view(fz_context *ctx, pdf_portfolio *portfolio, int view);

/**
 * Get the initial view mode.
 */
int pdf_portfolio_get_view(fz_context *ctx, pdf_portfolio *portfolio);

/**
 * Set the initial document (cover sheet).
 * @return 1 on success, 0 on failure
 */
int pdf_portfolio_set_initial_document(fz_context *ctx, pdf_portfolio *portfolio, const char *name);

/**
 * Set sort order.
 * @param field Sort field name (may be NULL to clear)
 * @param ascending 1 for ascending, 0 for descending
 * @return 1 on success, 0 on failure
 */
int pdf_portfolio_set_sort(fz_context *ctx, pdf_portfolio *portfolio, const char *field, int ascending);

/* ============================================================================
 * Utility Functions
 * ============================================================================ */

/**
 * Free a string returned by portfolio functions.
 */
void pdf_portfolio_free_string(char *s);

/**
 * Get AF relationship string.
 * @return Relationship name (caller must free) or NULL
 */
char *pdf_af_relationship_to_string(fz_context *ctx, int relationship);

/**
 * Get view mode string.
 * @return View mode string ("D", "T", "H", or "C") (caller must free) or NULL
 */
char *pdf_collection_view_to_string(fz_context *ctx, int view);

#ifdef __cplusplus
}
#endif

#endif /* MICROPDF_PDF_PORTFOLIO_H */


