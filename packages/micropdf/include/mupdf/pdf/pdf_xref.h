// MicroPDF - MuPDF API Compatible C Header
// Auto-generated from Rust FFI - DO NOT EDIT MANUALLY
// Module: pdf_xref

#ifndef MUPDF_PDF_PDF_XREF_H
#define MUPDF_PDF_PDF_XREF_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================================
// Pdf_xref Functions (26 total)
// ============================================================================

int32_t pdf_cache_object(int32_t _ctx, int32_t xref, int32_t num);
void pdf_clear_xref_marks(int32_t _ctx, int32_t xref);
int32_t pdf_count_objects(int32_t _ctx, int32_t xref);
int32_t pdf_create_object(int32_t _ctx, int32_t xref);
void pdf_delete_object(int32_t _ctx, int32_t xref, int32_t num);
void pdf_drop_xref(int32_t _ctx, int32_t xref);
int32_t pdf_get_cached_object(int32_t _ctx, int32_t xref, int32_t num);
int32_t pdf_get_stream_buffer(int32_t _ctx, int32_t xref, int32_t num);
int32_t pdf_get_xref_entry(int32_t _ctx, int32_t xref, int32_t num, XrefEntry * entry_out);
int32_t pdf_is_local_object(int32_t _ctx, int32_t xref, int32_t num);
int32_t pdf_mark_xref(int32_t _ctx, int32_t xref, int32_t num);
int32_t pdf_new_xref(int32_t _ctx, int32_t doc);
int32_t pdf_object_exists(int32_t _ctx, int32_t xref, int32_t num);
int32_t pdf_set_trailer(int32_t _ctx, int32_t xref, int32_t trailer);
int32_t pdf_set_version(int32_t _ctx, int32_t xref, int32_t version);
int32_t pdf_trailer(int32_t _ctx, int32_t xref);
int32_t pdf_update_object(int32_t _ctx, int32_t xref, int32_t num, int32_t obj);
int32_t pdf_update_stream(int32_t _ctx, int32_t xref, int32_t num, int32_t buffer, int32_t compressed);
int32_t pdf_version(int32_t _ctx, int32_t xref);
int32_t pdf_xref_add_subsection(int32_t _ctx, int32_t xref, int32_t start, int32_t count);
int64_t pdf_xref_end_offset(int32_t _ctx, int32_t xref);
char * pdf_xref_entry_type_string(int32_t _ctx, int32_t entry_type);
void pdf_xref_free_string(char * s);
int32_t pdf_xref_len(int32_t _ctx, int32_t xref);
int32_t pdf_xref_set_end_offset(int32_t _ctx, int32_t xref, int64_t offset);
int32_t pdf_xref_set_entry(int32_t _ctx, int32_t xref, int32_t num, int32_t entry_type, u16 generation, int64_t offset);

#ifdef __cplusplus
}
#endif

#endif /* MUPDF_PDF_PDF_XREF_H */
