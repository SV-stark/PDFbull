// MicroPDF - MuPDF API Compatible C Header
// Auto-generated from Rust FFI - DO NOT EDIT MANUALLY
// Module: document

#ifndef MUPDF_PDF_DOCUMENT_H
#define MUPDF_PDF_DOCUMENT_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================================
// Document Functions (30 total)
// ============================================================================

int32_t fz_authenticate_password(int32_t _ctx, int32_t doc, const char * password);
fz_rect fz_bound_page(int32_t _ctx, int32_t page);
fz_rect fz_bound_page_box(int32_t _ctx, int32_t page, int32_t _box_type);
int32_t fz_clone_document(int32_t _ctx, int32_t doc);
int32_t fz_count_chapter_pages(int32_t _ctx, int32_t doc, int32_t _chapter);
int32_t fz_count_chapters(int32_t _ctx, int32_t _doc);
int32_t fz_count_pages(int32_t _ctx, int32_t doc);
int32_t fz_document_format(int32_t _ctx, int32_t doc, char * buf, int32_t size);
int32_t fz_document_is_valid(int32_t _ctx, int32_t doc);
void fz_drop_document(int32_t _ctx, int32_t doc);
void fz_drop_page(int32_t _ctx, int32_t page);
int32_t fz_has_permission(int32_t _ctx, int32_t doc, int32_t _permission);
int32_t fz_is_document_reflowable(int32_t _ctx, int32_t doc);
int32_t fz_keep_document(int32_t _ctx, int32_t doc);
int32_t fz_keep_page(int32_t _ctx, int32_t page);
void fz_layout_document(int32_t _ctx, int32_t doc, float _w, float _h, float _em);
int32_t fz_load_chapter_page(int32_t _ctx, int32_t doc, int32_t chapter, int32_t page);
int32_t fz_load_outline(int32_t _ctx, int32_t doc);
int32_t fz_load_page(int32_t _ctx, int32_t doc, int32_t page_num);
int32_t fz_lookup_metadata(int32_t _ctx, int32_t _doc, const char * _key, char * buf, int32_t size);
char * fz_make_location_uri(int32_t _ctx, int32_t _doc, int32_t page, char * buf, int32_t size);
int32_t fz_needs_password(int32_t _ctx, int32_t doc);
int32_t fz_open_document(int32_t _ctx, const char * filename);
int32_t fz_open_document_with_stream(int32_t _ctx, const char * _magic, int32_t stm);
int32_t fz_page_label(int32_t _ctx, int32_t doc, int32_t page_num, char * buf, int32_t size);
int32_t fz_page_number_from_location(int32_t _ctx, int32_t _doc, int32_t chapter, int32_t page);
int32_t fz_resolve_link(int32_t _ctx, int32_t doc, const char * uri, float * xp, float * yp);
void fz_run_page(int32_t _ctx, int32_t page, int32_t device, fz_matrix transform, c_void * cookie);
void fz_run_page_annots(int32_t _ctx, int32_t page, int32_t device, fz_matrix transform, c_void * cookie);
void fz_run_page_contents(int32_t _ctx, int32_t page, int32_t device, fz_matrix transform, c_void * cookie);

#ifdef __cplusplus
}
#endif

#endif /* MUPDF_PDF_DOCUMENT_H */
