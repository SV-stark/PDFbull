// MicroPDF - MuPDF API Compatible C Header
// Auto-generated from Rust FFI - DO NOT EDIT MANUALLY
// Module: epub

#ifndef MUPDF_FITZ_EPUB_H
#define MUPDF_FITZ_EPUB_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================================
// Epub Functions (35 total)
// ============================================================================

int32_t epub_add_creator(int32_t _ctx, int32_t doc, const char * creator);
int32_t epub_add_file(int32_t _ctx, int32_t doc, const char * path, u8 const * data, size_t len);
int32_t epub_add_manifest_item(int32_t _ctx, int32_t doc, const char * id, const char * href, const char * media_type);
int32_t epub_add_spine_item(int32_t _ctx, int32_t doc, const char * idref, int32_t linear);
int32_t epub_add_toc_entry(int32_t _ctx, int32_t doc, const char * id, const char * label, const char * content);
void epub_drop_document(int32_t _ctx, int32_t doc);
void epub_free_string(char * s);
char * epub_get_creator(int32_t _ctx, int32_t doc, int32_t index);
int32_t epub_get_creator_count(int32_t _ctx, int32_t doc);
int32_t epub_get_direction(int32_t _ctx, int32_t doc);
u8 const * epub_get_file_data(int32_t _ctx, int32_t doc, const char * path, size_t * len_out);
char * epub_get_identifier(int32_t _ctx, int32_t doc);
char * epub_get_language(int32_t _ctx, int32_t doc);
char * epub_get_manifest_href(int32_t _ctx, int32_t doc, const char * id);
int32_t epub_get_manifest_media_type(int32_t _ctx, int32_t doc, const char * id);
char * epub_get_spine_idref(int32_t _ctx, int32_t doc, int32_t index);
char * epub_get_title(int32_t _ctx, int32_t doc);
char * epub_get_toc_content(int32_t _ctx, int32_t doc, int32_t index);
char * epub_get_toc_label(int32_t _ctx, int32_t doc, int32_t index);
int32_t epub_get_version(int32_t _ctx, int32_t doc);
int32_t epub_has_file(int32_t _ctx, int32_t doc, const char * path);
int32_t epub_manifest_count(int32_t _ctx, int32_t doc);
char * epub_media_type_string(int32_t _ctx, int32_t media_type);
int32_t epub_new_document(int32_t ctx);
int32_t epub_open_document(int32_t ctx, const char * filename);
int32_t epub_open_document_with_archive(int32_t ctx, int32_t _archive);
int32_t epub_open_document_with_stream(int32_t ctx, int32_t _stream);
int32_t epub_set_direction(int32_t _ctx, int32_t doc, int32_t direction);
int32_t epub_set_identifier(int32_t _ctx, int32_t doc, const char * id);
int32_t epub_set_language(int32_t _ctx, int32_t doc, const char * lang);
int32_t epub_set_title(int32_t _ctx, int32_t doc, const char * title);
int32_t epub_set_version(int32_t _ctx, int32_t doc, int32_t version);
int32_t epub_spine_count(int32_t _ctx, int32_t doc);
int32_t epub_spine_item_is_linear(int32_t _ctx, int32_t doc, int32_t index);
int32_t epub_toc_count(int32_t _ctx, int32_t doc);

#ifdef __cplusplus
}
#endif

#endif /* MUPDF_FITZ_EPUB_H */
