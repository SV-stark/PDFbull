/*
 * PDF ZUGFeRD/Factur-X FFI
 *
 * Provides support for ZUGFeRD and Factur-X electronic invoice formats,
 * enabling extraction and embedding of XML invoice data in PDF documents.
 */

#ifndef MICROPDF_PDF_ZUGFERD_H
#define MICROPDF_PDF_ZUGFERD_H

#include <stdint.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Handle types */
typedef uint64_t fz_context;
typedef uint64_t pdf_document;
typedef uint64_t pdf_zugferd_context;

/* ============================================================================
 * ZUGFeRD Profile Constants
 * ============================================================================ */

/** Not a ZUGFeRD document */
#define PDF_NOT_ZUGFERD         0
/** ZUGFeRD 1.0 Comfort profile */
#define PDF_ZUGFERD_COMFORT     1
/** ZUGFeRD 1.0 Basic profile */
#define PDF_ZUGFERD_BASIC       2
/** ZUGFeRD 1.0 Extended profile */
#define PDF_ZUGFERD_EXTENDED    3
/** ZUGFeRD 2.01 Basic WL profile */
#define PDF_ZUGFERD_BASIC_WL    4
/** ZUGFeRD 2.01 Minimum profile */
#define PDF_ZUGFERD_MINIMUM     5
/** ZUGFeRD 2.2 XRechnung profile */
#define PDF_ZUGFERD_XRECHNUNG   6
/** Unknown ZUGFeRD profile */
#define PDF_ZUGFERD_UNKNOWN     7

/* Factur-X aliases */
#define PDF_FACTURX_MINIMUM     PDF_ZUGFERD_MINIMUM
#define PDF_FACTURX_BASIC_WL    PDF_ZUGFERD_BASIC_WL
#define PDF_FACTURX_BASIC       PDF_ZUGFERD_BASIC
#define PDF_FACTURX_EN16931     PDF_ZUGFERD_COMFORT
#define PDF_FACTURX_EXTENDED    PDF_ZUGFERD_EXTENDED

/* ============================================================================
 * Structures
 * ============================================================================ */

/** Parameters for embedding a ZUGFeRD invoice */
typedef struct {
    int profile;            /**< Profile to use */
    float version;          /**< Version (e.g., 2.2) */
    const char *filename;   /**< Filename (default: "factur-x.xml") */
    int add_checksum;       /**< Add checksum to embedded file */
} pdf_zugferd_embed_params;

/* ============================================================================
 * Context Management
 * ============================================================================ */

/**
 * Create a new ZUGFeRD context for a document.
 */
pdf_zugferd_context *pdf_new_zugferd_context(fz_context *ctx, pdf_document *doc);

/**
 * Drop a ZUGFeRD context.
 */
void pdf_drop_zugferd_context(fz_context *ctx, pdf_zugferd_context *zugferd);

/* ============================================================================
 * Profile Detection
 * ============================================================================ */

/**
 * Detect the ZUGFeRD profile of a document.
 * @param ctx Context
 * @param zugferd ZUGFeRD context
 * @param version_out Pointer to receive version (may be NULL)
 * @return Profile constant
 */
int pdf_zugferd_profile(fz_context *ctx, pdf_zugferd_context *zugferd, float *version_out);

/**
 * Check if a document is a ZUGFeRD invoice.
 * @return 1 if ZUGFeRD, 0 if not
 */
int pdf_is_zugferd(fz_context *ctx, pdf_zugferd_context *zugferd);

/**
 * Get the ZUGFeRD version.
 */
float pdf_zugferd_version(fz_context *ctx, pdf_zugferd_context *zugferd);

/* ============================================================================
 * XML Extraction
 * ============================================================================ */

/**
 * Extract the embedded XML invoice data.
 * @param ctx Context
 * @param zugferd ZUGFeRD context
 * @param len_out Pointer to receive length (may be NULL)
 * @return Pointer to XML data (owned by context) or NULL
 */
const uint8_t *pdf_zugferd_xml(fz_context *ctx, pdf_zugferd_context *zugferd, size_t *len_out);

/**
 * Set XML data for the ZUGFeRD context.
 * @return 1 on success, 0 on failure
 */
int pdf_zugferd_set_xml(fz_context *ctx, pdf_zugferd_context *zugferd, const uint8_t *xml, size_t len);

/* ============================================================================
 * Profile String Conversion
 * ============================================================================ */

/**
 * Convert a profile constant to a human-readable string.
 * @return String (caller must free with pdf_zugferd_free_string)
 */
char *pdf_zugferd_profile_to_string(fz_context *ctx, int profile);

/**
 * Free a string returned by ZUGFeRD functions.
 */
void pdf_zugferd_free_string(char *s);

/* ============================================================================
 * Invoice Embedding
 * ============================================================================ */

/**
 * Create default embed parameters.
 */
pdf_zugferd_embed_params pdf_zugferd_default_embed_params(void);

/**
 * Embed an XML invoice into a document.
 * @param ctx Context
 * @param zugferd ZUGFeRD context
 * @param xml XML data
 * @param xml_len XML length
 * @param params Embedding parameters (may be NULL for defaults)
 * @return 1 on success, 0 on failure
 */
int pdf_zugferd_embed(fz_context *ctx, pdf_zugferd_context *zugferd, const uint8_t *xml, size_t xml_len, const pdf_zugferd_embed_params *params);

/* ============================================================================
 * Validation
 * ============================================================================ */

/**
 * Validate ZUGFeRD compliance.
 * @return 1 if valid, 0 if invalid
 */
int pdf_zugferd_validate(fz_context *ctx, pdf_zugferd_context *zugferd);

/**
 * Get validation error count.
 */
int pdf_zugferd_error_count(fz_context *ctx, pdf_zugferd_context *zugferd);

/* ============================================================================
 * Utility Functions
 * ============================================================================ */

/**
 * Get the standard filename for a ZUGFeRD profile.
 * @return Filename string (caller must free with pdf_zugferd_free_string)
 */
char *pdf_zugferd_standard_filename(fz_context *ctx, int profile);

/**
 * Get the MIME type for ZUGFeRD XML.
 * @return MIME type string (caller must free with pdf_zugferd_free_string)
 */
char *pdf_zugferd_mime_type(fz_context *ctx);

/**
 * Get AF relationship for ZUGFeRD.
 * @return Relationship string (caller must free with pdf_zugferd_free_string)
 */
char *pdf_zugferd_af_relationship(fz_context *ctx);

#ifdef __cplusplus
}
#endif

#endif /* MICROPDF_PDF_ZUGFERD_H */


