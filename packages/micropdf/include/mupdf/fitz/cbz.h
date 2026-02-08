// MicroPDF - MuPDF API Compatible C Header
// Auto-generated from Rust FFI - DO NOT EDIT MANUALLY
// Module: cbz

#ifndef MUPDF_FITZ_CBZ_H
#define MUPDF_FITZ_CBZ_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================================
// Cbz Functions (35 total)
// ============================================================================

int32_t cbz_add_entry(int32_t _ctx, int32_t doc, const char * name);
void cbz_drop_document(int32_t _ctx, int32_t doc);
char * cbz_format_name(int32_t _ctx, int32_t format);
void cbz_free_string(char * s);
int32_t cbz_get_format(int32_t _ctx, int32_t doc);
int32_t cbz_get_manga(int32_t _ctx, int32_t doc);
char * cbz_get_number(int32_t _ctx, int32_t doc);
char * cbz_get_page_filename(int32_t _ctx, int32_t doc, int32_t page_num);
int32_t cbz_get_page_format(int32_t _ctx, int32_t doc, int32_t page_num);
int32_t cbz_get_page_size(int32_t _ctx, int32_t doc, int32_t page_num, int32_t * width, int32_t * height);
char * cbz_get_publisher(int32_t _ctx, int32_t doc);
char * cbz_get_series(int32_t _ctx, int32_t doc);
char * cbz_get_summary(int32_t _ctx, int32_t doc);
char * cbz_get_title(int32_t _ctx, int32_t doc);
char * cbz_get_writer(int32_t _ctx, int32_t doc);
int32_t cbz_get_year(int32_t _ctx, int32_t doc);
char * cbz_image_format_name(int32_t _ctx, int32_t format);
int32_t cbz_is_image_file(int32_t _ctx, const char * filename);
int32_t cbz_new_document(int32_t ctx);
int32_t cbz_open_document(int32_t ctx, const char * filename);
int32_t cbz_open_document_with_archive(int32_t ctx, int32_t _archive);
int32_t cbz_open_document_with_stream(int32_t ctx, int32_t _stream);
int32_t cbz_page_count(int32_t _ctx, int32_t doc);
int32_t cbz_page_is_double(int32_t _ctx, int32_t doc, int32_t page_num);
int32_t cbz_set_manga(int32_t _ctx, int32_t doc, int32_t manga);
int32_t cbz_set_number(int32_t _ctx, int32_t doc, const char * number);
int32_t cbz_set_page_double(int32_t _ctx, int32_t doc, int32_t page_num, int32_t double);
int32_t cbz_set_page_size(int32_t _ctx, int32_t doc, int32_t page_num, int32_t width, int32_t height);
int32_t cbz_set_publisher(int32_t _ctx, int32_t doc, const char * publisher);
int32_t cbz_set_series(int32_t _ctx, int32_t doc, const char * series);
int32_t cbz_set_summary(int32_t _ctx, int32_t doc, const char * summary);
int32_t cbz_set_title(int32_t _ctx, int32_t doc, const char * title);
int32_t cbz_set_writer(int32_t _ctx, int32_t doc, const char * writer);
int32_t cbz_set_year(int32_t _ctx, int32_t doc, int32_t year);
int32_t cbz_sort_pages(int32_t _ctx, int32_t doc);

#ifdef __cplusplus
}
#endif

#endif /* MUPDF_FITZ_CBZ_H */
