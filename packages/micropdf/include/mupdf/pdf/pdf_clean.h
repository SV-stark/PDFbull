// MicroPDF - MuPDF API Compatible C Header
// Auto-generated from Rust FFI - DO NOT EDIT MANUALLY
// Module: pdf_clean

#ifndef MUPDF_PDF_PDF_CLEAN_H
#define MUPDF_PDF_PDF_CLEAN_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================================
// Pdf_clean Functions (29 total)
// ============================================================================

int32_t pdf_can_be_saved_incrementally(int32_t _ctx, int32_t _doc);
void pdf_clean_file(int32_t _ctx, const char * infile, const char * outfile, const char * _password, CleanOptions const * _opts, int32_t _retainlen, const char * const * _retainlist);
void pdf_clean_free_string(int32_t _ctx, char * s);
void pdf_clean_object_entries(int32_t _ctx, int32_t _obj);
void pdf_compress_streams(int32_t _ctx, int32_t _doc, int32_t method);
void pdf_create_object_streams(int32_t _ctx, int32_t _doc);
void pdf_decompress_streams(int32_t _ctx, int32_t _doc);
void pdf_deduplicate_objects(int32_t _ctx, int32_t _doc);
CleanOptions pdf_default_clean_options(void);
WriteOptions pdf_default_write_options(void);
char * pdf_format_write_options(int32_t _ctx, char * buffer, size_t buffer_len, WriteOptions const * opts);
void pdf_garbage_collect(int32_t _ctx, int32_t _doc, int32_t level);
int32_t pdf_has_unsaved_sigs(int32_t _ctx, int32_t _doc);
void pdf_linearize(int32_t ctx, int32_t doc, const char * filename);
void pdf_optimize(int32_t ctx, int32_t doc, const char * filename);
WriteOptions * pdf_parse_write_options(int32_t _ctx, WriteOptions * opts, const char * args);
void pdf_rearrange_pages(int32_t _ctx, int32_t _doc, int32_t count, int32_t const * pages, CleanStructureOption _structure);
void pdf_remove_encryption(int32_t _ctx, WriteOptions * opts);
void pdf_remove_object_streams(int32_t _ctx, int32_t _doc);
void pdf_remove_unused_resources(int32_t _ctx, int32_t _doc);
void pdf_renumber_objects(int32_t _ctx, int32_t _doc);
void pdf_save_document(int32_t _ctx, int32_t doc, const char * filename, WriteOptions const * _opts);
void pdf_save_journal(int32_t _ctx, int32_t _doc, const char * filename);
void pdf_save_snapshot(int32_t _ctx, int32_t _doc, const char * filename);
void pdf_set_encryption(int32_t _ctx, WriteOptions * opts, int32_t method, int32_t permissions, const char * owner_pwd, const char * user_pwd);
void pdf_vectorize_pages(int32_t _ctx, int32_t _doc, int32_t _count, int32_t const * _pages, CleanVectorizeOption _vectorize);
void pdf_write_document(int32_t _ctx, int32_t _doc, int32_t _out, WriteOptions const * _opts);
void pdf_write_journal(int32_t _ctx, int32_t _doc, int32_t _out);
void pdf_write_snapshot(int32_t _ctx, int32_t _doc, int32_t _out);

#ifdef __cplusplus
}
#endif

#endif /* MUPDF_PDF_PDF_CLEAN_H */
