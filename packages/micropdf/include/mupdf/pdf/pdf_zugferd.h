// MicroPDF - MuPDF API Compatible C Header
// Auto-generated from Rust FFI - DO NOT EDIT MANUALLY
// Module: pdf_zugferd

#ifndef MUPDF_PDF_PDF_ZUGFERD_H
#define MUPDF_PDF_PDF_ZUGFERD_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================================
// Pdf_zugferd Functions (16 total)
// ============================================================================

void pdf_drop_zugferd_context(int32_t _ctx, int32_t zugferd);
int32_t pdf_is_zugferd(int32_t _ctx, int32_t zugferd);
int32_t pdf_new_zugferd_context(int32_t _ctx, int32_t doc);
char * pdf_zugferd_af_relationship(int32_t _ctx);
ZugferdEmbedParams pdf_zugferd_default_embed_params(void);
int32_t pdf_zugferd_embed(int32_t _ctx, int32_t zugferd, u8 const * xml, size_t xml_len, ZugferdEmbedParams const * params);
int32_t pdf_zugferd_error_count(int32_t _ctx, int32_t _zugferd);
void pdf_zugferd_free_string(char * s);
char * pdf_zugferd_mime_type(int32_t _ctx);
int32_t pdf_zugferd_profile(int32_t _ctx, int32_t zugferd, float * version_out);
char * pdf_zugferd_profile_to_string(int32_t _ctx, int32_t profile);
int32_t pdf_zugferd_set_xml(int32_t _ctx, int32_t zugferd, u8 const * xml, size_t len);
char * pdf_zugferd_standard_filename(int32_t _ctx, int32_t profile);
int32_t pdf_zugferd_validate(int32_t _ctx, int32_t zugferd);
float pdf_zugferd_version(int32_t _ctx, int32_t zugferd);
u8 const * pdf_zugferd_xml(int32_t _ctx, int32_t zugferd, size_t * len_out);

#ifdef __cplusplus
}
#endif

#endif /* MUPDF_PDF_PDF_ZUGFERD_H */
