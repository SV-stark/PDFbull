// MicroPDF - MuPDF API Compatible C Header
// Auto-generated from Rust FFI - DO NOT EDIT MANUALLY
// Module: xps

#ifndef MUPDF_FITZ_XPS_H
#define MUPDF_FITZ_XPS_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================================
// Xps Functions (22 total)
// ============================================================================

int32_t xps_add_part(int32_t _ctx, int32_t doc, const char * name, u8 const * data, size_t len, const char * content_type);
int32_t xps_add_target(int32_t _ctx, int32_t doc, const char * name, int32_t page);
char * xps_content_type_string(int32_t _ctx, int32_t content_type);
int32_t xps_count_documents(int32_t _ctx, int32_t doc);
int32_t xps_count_pages(int32_t _ctx, int32_t doc);
int32_t xps_count_pages_in_document(int32_t _ctx, int32_t doc, int32_t doc_num);
void xps_drop_document(int32_t _ctx, int32_t doc);
int32_t xps_font_count(int32_t _ctx, int32_t doc);
void xps_free_string(char * s);
char * xps_get_document_name(int32_t _ctx, int32_t doc, int32_t doc_num);
char * xps_get_page_name(int32_t _ctx, int32_t doc, int32_t page_num);
int32_t xps_get_page_size(int32_t _ctx, int32_t doc, int32_t page_num, float * width, float * height);
char * xps_get_part_content_type(int32_t _ctx, int32_t doc, const char * name);
u8 const * xps_get_part_data(int32_t _ctx, int32_t doc, const char * name, size_t * len_out);
int32_t xps_has_part(int32_t _ctx, int32_t doc, const char * name);
int32_t xps_lookup_font(int32_t _ctx, int32_t doc, const char * uri);
int32_t xps_lookup_target(int32_t _ctx, int32_t doc, const char * name);
int32_t xps_new_document(int32_t ctx);
int32_t xps_open_document(int32_t ctx, const char * filename);
int32_t xps_open_document_with_directory(int32_t ctx, int32_t _archive);
int32_t xps_open_document_with_stream(int32_t ctx, int32_t _stream);
int32_t xps_resolve_url(int32_t _ctx, const char * base_uri, const char * path, char * output, int32_t output_size);

#ifdef __cplusplus
}
#endif

#endif /* MUPDF_FITZ_XPS_H */
