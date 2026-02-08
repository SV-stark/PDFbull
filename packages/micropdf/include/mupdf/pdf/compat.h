// MicroPDF - MuPDF API Compatible C Header
// Auto-generated from Rust FFI - DO NOT EDIT MANUALLY
// Module: compat

#ifndef MUPDF_PDF_COMPAT_H
#define MUPDF_PDF_COMPAT_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================================
// Compat Functions (9 total)
// ============================================================================

void fz_abort_cookie(int32_t ctx, int32_t cookie);
int32_t fz_cookie_is_aborted(int32_t ctx, int32_t cookie);
int32_t fz_cookie_progress(int32_t ctx, int32_t cookie);
int32_t fz_new_buffer_from_stext_page(int32_t ctx, int32_t stext);
int32_t fz_new_pixmap_from_page(int32_t _ctx, int32_t page, fz_matrix ctm, int32_t cs, int32_t alpha);
int32_t fz_new_stext_page_from_page(int32_t _ctx, int32_t page, void const * _options);
int32_t fz_open_document_with_buffer(int32_t _ctx, const char * _magic, u8 const * data, size_t len);
void fz_reset_cookie(int32_t ctx, int32_t cookie);
int32_t pdf_lookup_named_dest(int32_t _ctx, int32_t _doc, const char * name);

#ifdef __cplusplus
}
#endif

#endif /* MUPDF_PDF_COMPAT_H */
