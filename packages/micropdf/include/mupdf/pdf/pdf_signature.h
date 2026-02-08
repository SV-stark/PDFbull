// MicroPDF - MuPDF API Compatible C Header
// Auto-generated from Rust FFI - DO NOT EDIT MANUALLY
// Module: pdf_signature

#ifndef MUPDF_PDF_PDF_SIGNATURE_H
#define MUPDF_PDF_PDF_SIGNATURE_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================================
// Pdf_signature Functions (33 total)
// ============================================================================

int32_t pdf_add_signature(int32_t _ctx, int32_t doc, const char * cn, int64_t date);
int32_t pdf_check_certificate(int32_t _ctx, int32_t verifier, int32_t doc, int32_t _signature);
int32_t pdf_check_digest(int32_t _ctx, int32_t verifier, int32_t doc, int32_t _signature);
void pdf_clear_all_signatures(int32_t _ctx, int32_t doc);
void pdf_clear_signature(int32_t _ctx, int32_t widget);
int32_t pdf_count_signatures(int32_t _ctx, int32_t doc);
const char * pdf_dn_c(int32_t _ctx, int32_t dn);
const char * pdf_dn_cn(int32_t _ctx, int32_t dn);
const char * pdf_dn_email(int32_t _ctx, int32_t dn);
const char * pdf_dn_o(int32_t _ctx, int32_t dn);
const char * pdf_dn_ou(int32_t _ctx, int32_t dn);
void pdf_drop_signature_info(int32_t _ctx, int32_t sig);
void pdf_drop_signer(int32_t _ctx, int32_t signer);
void pdf_drop_verifier(int32_t _ctx, int32_t verifier);
int32_t pdf_get_signature(int32_t _ctx, int32_t doc, int32_t index);
int32_t pdf_pkcs7_keep_signer(int32_t _ctx, int32_t signer);
int32_t pdf_pkcs7_signer_get_name(int32_t _ctx, int32_t signer);
size_t pdf_pkcs7_signer_max_digest_size(int32_t _ctx, int32_t signer);
int32_t pdf_pkcs7_signer_new(int32_t _ctx, const char * cn);
void pdf_pkcs7_verifier_add_cert(int32_t _ctx, int32_t verifier, u8 const * cert, size_t len);
int32_t pdf_pkcs7_verifier_new(int32_t _ctx);
void pdf_sign_signature(int32_t _ctx, int32_t _widget, int32_t signer, int64_t date, const char * _reason, const char * _location);
int32_t pdf_signature_byte_range(int32_t _ctx, int32_t doc, int32_t _signature, ByteRange * byte_range);
size_t pdf_signature_contents(int32_t _ctx, int32_t doc, int32_t _signature, char * * contents);
void pdf_signature_drop_distinguished_name(int32_t _ctx, int32_t dn);
const char * pdf_signature_error_description(int32_t err);
const char * pdf_signature_format_distinguished_name(int32_t _ctx, int32_t dn);
void pdf_signature_free_string(int32_t _ctx, char * s);
int32_t pdf_signature_get_signatory(int32_t _ctx, int32_t verifier, int32_t doc, int32_t _signature);
int32_t pdf_signature_incremental_change_since_signing(int32_t _ctx, int32_t doc, int32_t _signature);
const char * pdf_signature_info(int32_t _ctx, const char * name, int32_t dn, const char * reason, const char * location, int64_t date, int32_t include_labels);
int32_t pdf_signature_is_signed(int32_t _ctx, int32_t doc, int32_t _field);
void pdf_signature_set_value(int32_t _ctx, int32_t doc, int32_t _field, int32_t signer, int64_t stime);

#ifdef __cplusplus
}
#endif

#endif /* MUPDF_PDF_PDF_SIGNATURE_H */
