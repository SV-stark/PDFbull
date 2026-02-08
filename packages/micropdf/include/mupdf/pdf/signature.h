/*
 * PDF Signature FFI
 *
 * Provides support for PDF digital signatures, including signature
 * verification, signing, and certificate handling.
 */

#ifndef MICROPDF_PDF_SIGNATURE_H
#define MICROPDF_PDF_SIGNATURE_H

#include <stdint.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Handle types */
typedef uint64_t fz_context;
typedef uint64_t pdf_document;
typedef uint64_t pdf_obj;
typedef uint64_t pdf_annot;
typedef uint64_t fz_stream;
typedef uint64_t pdf_pkcs7_signer;
typedef uint64_t pdf_pkcs7_verifier;
typedef uint64_t pdf_pkcs7_distinguished_name;
typedef uint64_t pdf_signature_info;

/* ============================================================================
 * Signature Error Types
 * ============================================================================ */

typedef enum {
    PDF_SIGNATURE_ERROR_OKAY = 0,
    PDF_SIGNATURE_ERROR_NO_SIGNATURES = 1,
    PDF_SIGNATURE_ERROR_NO_CERTIFICATE = 2,
    PDF_SIGNATURE_ERROR_DIGEST_FAILURE = 3,
    PDF_SIGNATURE_ERROR_SELF_SIGNED = 4,
    PDF_SIGNATURE_ERROR_SELF_SIGNED_IN_CHAIN = 5,
    PDF_SIGNATURE_ERROR_NOT_TRUSTED = 6,
    PDF_SIGNATURE_ERROR_NOT_SIGNED = 7,
    PDF_SIGNATURE_ERROR_UNKNOWN = 8
} pdf_signature_error;

/* ============================================================================
 * Signature Appearance Flags
 * ============================================================================ */

#define PDF_SIGNATURE_SHOW_LABELS 1
#define PDF_SIGNATURE_SHOW_DN 2
#define PDF_SIGNATURE_SHOW_DATE 4
#define PDF_SIGNATURE_SHOW_TEXT_NAME 8
#define PDF_SIGNATURE_SHOW_GRAPHIC_NAME 16
#define PDF_SIGNATURE_SHOW_LOGO 32

#define PDF_SIGNATURE_DEFAULT_APPEARANCE ( \
    PDF_SIGNATURE_SHOW_LABELS | \
    PDF_SIGNATURE_SHOW_DN | \
    PDF_SIGNATURE_SHOW_DATE | \
    PDF_SIGNATURE_SHOW_TEXT_NAME | \
    PDF_SIGNATURE_SHOW_GRAPHIC_NAME | \
    PDF_SIGNATURE_SHOW_LOGO)

/* ============================================================================
 * Byte Range Structure
 * ============================================================================ */

typedef struct {
    int64_t offset;
    int64_t length;
} fz_range;

/* ============================================================================
 * Distinguished Name Structure (for FFI)
 * ============================================================================ */

typedef struct {
    const char *cn;     /* Common Name */
    const char *o;      /* Organization */
    const char *ou;     /* Organizational Unit */
    const char *email;  /* Email address */
    const char *c;      /* Country */
} ffi_distinguished_name;

/* ============================================================================
 * Signature Query Functions
 * ============================================================================ */

/**
 * Check if a signature field is signed.
 * @return 1 if signed, 0 if not signed
 */
int pdf_signature_is_signed(fz_context *ctx, pdf_document *doc, pdf_obj *field);

/**
 * Count signatures in document.
 * @return Number of signatures
 */
int pdf_count_signatures(fz_context *ctx, pdf_document *doc);

/**
 * Get signature byte range.
 * @param byte_range Pointer to structure to fill
 * @return Number of ranges (typically 2)
 */
int pdf_signature_byte_range(fz_context *ctx, pdf_document *doc, pdf_obj *signature, fz_range *byte_range);

/**
 * Get signature contents (PKCS#7 data).
 * @param contents Pointer to receive allocated buffer (caller must free)
 * @return Size of contents
 */
size_t pdf_signature_contents(fz_context *ctx, pdf_document *doc, pdf_obj *signature, char **contents);

/**
 * Check if document has incremental changes since signing.
 * @return 1 if changed, 0 if not changed
 */
int pdf_signature_incremental_change_since_signing(fz_context *ctx, pdf_document *doc, pdf_obj *signature);

/* ============================================================================
 * Signature Verification Functions
 * ============================================================================ */

/**
 * Check signature digest.
 * @return pdf_signature_error code
 */
pdf_signature_error pdf_check_digest(fz_context *ctx, pdf_pkcs7_verifier *verifier, pdf_document *doc, pdf_obj *signature);

/**
 * Check signature certificate.
 * @return pdf_signature_error code
 */
pdf_signature_error pdf_check_certificate(fz_context *ctx, pdf_pkcs7_verifier *verifier, pdf_document *doc, pdf_obj *signature);

/**
 * Get signature error description.
 * @return Static string describing the error
 */
const char *pdf_signature_error_description(pdf_signature_error err);

/* ============================================================================
 * Distinguished Name Functions
 * ============================================================================ */

/**
 * Get signatory information from signature.
 * @return Distinguished name handle, or 0 on failure
 */
pdf_pkcs7_distinguished_name *pdf_signature_get_signatory(fz_context *ctx, pdf_pkcs7_verifier *verifier, pdf_document *doc, pdf_obj *signature);

/**
 * Drop a distinguished name.
 */
void pdf_signature_drop_distinguished_name(fz_context *ctx, pdf_pkcs7_distinguished_name *dn);

/**
 * Format distinguished name as string.
 * @return Formatted string (caller must free with pdf_signature_free_string)
 */
const char *pdf_signature_format_distinguished_name(fz_context *ctx, pdf_pkcs7_distinguished_name *dn);

/**
 * Get DN component - Common Name (CN).
 */
const char *pdf_dn_cn(fz_context *ctx, pdf_pkcs7_distinguished_name *dn);

/**
 * Get DN component - Organization (O).
 */
const char *pdf_dn_o(fz_context *ctx, pdf_pkcs7_distinguished_name *dn);

/**
 * Get DN component - Organizational Unit (OU).
 */
const char *pdf_dn_ou(fz_context *ctx, pdf_pkcs7_distinguished_name *dn);

/**
 * Get DN component - Email.
 */
const char *pdf_dn_email(fz_context *ctx, pdf_pkcs7_distinguished_name *dn);

/**
 * Get DN component - Country (C).
 */
const char *pdf_dn_c(fz_context *ctx, pdf_pkcs7_distinguished_name *dn);

/* ============================================================================
 * Signer Functions
 * ============================================================================ */

/**
 * Create a new PKCS#7 signer.
 * @param cn Common Name for the signer
 * @return Signer handle
 */
pdf_pkcs7_signer *pdf_pkcs7_signer_new(fz_context *ctx, const char *cn);

/**
 * Keep (increment reference to) a signer.
 */
pdf_pkcs7_signer *pdf_pkcs7_keep_signer(fz_context *ctx, pdf_pkcs7_signer *signer);

/**
 * Drop a signer.
 */
void pdf_drop_signer(fz_context *ctx, pdf_pkcs7_signer *signer);

/**
 * Get signer's distinguished name.
 * @return Distinguished name handle
 */
pdf_pkcs7_distinguished_name *pdf_pkcs7_signer_get_name(fz_context *ctx, pdf_pkcs7_signer *signer);

/**
 * Get signer's max digest size.
 * @return Maximum size of generated digest
 */
size_t pdf_pkcs7_signer_max_digest_size(fz_context *ctx, pdf_pkcs7_signer *signer);

/* ============================================================================
 * Verifier Functions
 * ============================================================================ */

/**
 * Create a new PKCS#7 verifier.
 * @return Verifier handle
 */
pdf_pkcs7_verifier *pdf_pkcs7_verifier_new(fz_context *ctx);

/**
 * Drop a verifier.
 */
void pdf_drop_verifier(fz_context *ctx, pdf_pkcs7_verifier *verifier);

/**
 * Add a trusted certificate to verifier.
 * @param cert Certificate data
 * @param len Length of certificate data
 */
void pdf_pkcs7_verifier_add_cert(fz_context *ctx, pdf_pkcs7_verifier *verifier, const unsigned char *cert, size_t len);

/* ============================================================================
 * Signing Functions
 * ============================================================================ */

/**
 * Sign a signature field.
 * @param widget Annotation widget handle
 * @param signer Signer handle
 * @param date Signing date (Unix timestamp)
 * @param reason Signing reason (may be NULL)
 * @param location Signing location (may be NULL)
 */
void pdf_sign_signature(fz_context *ctx, pdf_annot *widget, pdf_pkcs7_signer *signer, int64_t date, const char *reason, const char *location);

/**
 * Clear a signature from a widget.
 */
void pdf_clear_signature(fz_context *ctx, pdf_annot *widget);

/**
 * Set signature value on a field.
 * @param field Field object handle
 * @param signer Signer handle
 * @param stime Signing time (Unix timestamp)
 */
void pdf_signature_set_value(fz_context *ctx, pdf_document *doc, pdf_obj *field, pdf_pkcs7_signer *signer, int64_t stime);

/* ============================================================================
 * Signature Info Formatting
 * ============================================================================ */

/**
 * Format signature info as string.
 * @param name Signer name
 * @param dn Distinguished name handle
 * @param reason Signing reason (may be NULL)
 * @param location Signing location (may be NULL)
 * @param date Signing date
 * @param include_labels Whether to include labels
 * @return Formatted string (caller must free with pdf_signature_free_string)
 */
const char *pdf_signature_info(fz_context *ctx, const char *name, pdf_pkcs7_distinguished_name *dn, const char *reason, const char *location, int64_t date, int include_labels);

/**
 * Free a string allocated by signature functions.
 */
void pdf_signature_free_string(fz_context *ctx, char *s);

/* ============================================================================
 * Additional Signature Management
 * ============================================================================ */

/**
 * Add a signature to document (for testing/simulation).
 * @param cn Signer common name
 * @param date Signing date
 * @return Signature index
 */
int pdf_add_signature(fz_context *ctx, pdf_document *doc, const char *cn, int64_t date);

/**
 * Get signature at index.
 * @return Signature info handle
 */
pdf_signature_info *pdf_get_signature(fz_context *ctx, pdf_document *doc, int index);

/**
 * Drop a signature info handle.
 */
void pdf_drop_signature_info(fz_context *ctx, pdf_signature_info *sig);

/**
 * Clear all signatures from document.
 */
void pdf_clear_all_signatures(fz_context *ctx, pdf_document *doc);

#ifdef __cplusplus
}
#endif

#endif /* MICROPDF_PDF_SIGNATURE_H */


