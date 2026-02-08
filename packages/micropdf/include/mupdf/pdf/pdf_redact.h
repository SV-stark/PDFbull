// MicroPDF - MuPDF API Compatible C Header
// Auto-generated from Rust FFI - DO NOT EDIT MANUALLY
// Module: pdf_redact

#ifndef MUPDF_PDF_PDF_REDACT_H
#define MUPDF_PDF_PDF_REDACT_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================================
// Pdf_redact Functions (26 total)
// ============================================================================

void pdf_add_redact_annot_quad(int32_t _ctx, int32_t _annot, float const * _quad, x0 // 8 floats);
void pdf_add_redact_region(int32_t _ctx, int32_t redact_ctx, float x0, float y0, float x1, float y1);
void pdf_add_redact_region_with_color(int32_t _ctx, int32_t redact_ctx, float x0, float y0, float x1, float y1, float r, float g, float b);
int32_t pdf_apply_all_redactions(int32_t _ctx, int32_t _doc, RedactOptions const * _opts);
int32_t pdf_apply_redaction(int32_t _ctx, int32_t _annot, RedactOptions const * _opts);
int32_t pdf_apply_redactions(int32_t _ctx, int32_t redact_ctx);
void pdf_clear_redact_regions(int32_t _ctx, int32_t redact_ctx);
int32_t pdf_count_redact_regions(int32_t _ctx, int32_t redact_ctx);
int32_t pdf_create_redact_annot(int32_t _ctx, int32_t _page, float x0, float y0, float x1, float y1);
RedactOptions pdf_default_redact_options(void);
void pdf_drop_redact_context(int32_t _ctx, int32_t redact_ctx);
RedactStats pdf_get_redact_stats(int32_t _ctx, int32_t redact_ctx);
int32_t pdf_new_redact_context(int32_t _ctx, int32_t doc, int32_t page);
RedactOptions pdf_ocr_redact_options(void);
int32_t pdf_redact_document(int32_t _ctx, int32_t _doc, RedactOptions const * _opts);
int32_t pdf_redact_page_annotations(int32_t _ctx, int32_t _doc, int32_t _page, RedactOptions const * _opts);
void pdf_remove_attachments(int32_t _ctx, int32_t _doc);
void pdf_remove_comments(int32_t _ctx, int32_t _doc);
void pdf_remove_hidden_content(int32_t _ctx, int32_t _doc);
void pdf_remove_javascript(int32_t _ctx, int32_t _doc);
void pdf_remove_metadata_field(int32_t _ctx, int32_t _doc, const char * _field);
void pdf_sanitize_metadata(int32_t _ctx, int32_t _doc);
RedactOptions pdf_secure_redact_options(void);
void pdf_set_redact_annot_color(int32_t _ctx, int32_t _annot, float r, float g, float b);
void pdf_set_redact_annot_text(int32_t _ctx, int32_t _annot, const char * _text);
void pdf_set_redact_options(int32_t _ctx, int32_t redact_ctx, RedactOptions opts);

#ifdef __cplusplus
}
#endif

#endif /* MUPDF_PDF_PDF_REDACT_H */
