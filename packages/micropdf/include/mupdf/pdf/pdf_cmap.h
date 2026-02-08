// MicroPDF - MuPDF API Compatible C Header
// Auto-generated from Rust FFI - DO NOT EDIT MANUALLY
// Module: pdf_cmap

#ifndef MUPDF_PDF_PDF_CMAP_H
#define MUPDF_PDF_PDF_CMAP_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================================
// Pdf_cmap Functions (27 total)
// ============================================================================

void pdf_add_codespace(int32_t _ctx, int32_t cmap, uint32_t low, uint32_t high, size_t n);
int32_t pdf_cmap_codespace_len(int32_t _ctx, int32_t cmap);
void pdf_cmap_free_string(int32_t _ctx, char * s);
int32_t pdf_cmap_has_usecmap(int32_t _ctx, int32_t cmap);
int32_t pdf_cmap_mrange_count(int32_t _ctx, int32_t cmap);
const char * pdf_cmap_name(int32_t _ctx, int32_t cmap);
int32_t pdf_cmap_range_count(int32_t _ctx, int32_t cmap);
size_t pdf_cmap_size(int32_t _ctx, int32_t cmap);
int32_t pdf_cmap_wmode(int32_t _ctx, int32_t cmap);
int32_t pdf_cmap_xrange_count(int32_t _ctx, int32_t cmap);
int32_t pdf_decode_cmap(int32_t cmap, u8 const * s, u8 const * e, uint32_t * cpt);
void pdf_drop_cmap(int32_t _ctx, int32_t cmap);
int32_t pdf_keep_cmap(int32_t _ctx, int32_t cmap);
int32_t pdf_load_builtin_cmap(int32_t _ctx, const char * name);
int32_t pdf_load_cmap(int32_t _ctx, int32_t _file);
int32_t pdf_load_embedded_cmap(int32_t _ctx, int32_t _doc, int32_t _ref);
int32_t pdf_load_system_cmap(int32_t _ctx, const char * name);
int32_t pdf_lookup_cmap(int32_t cmap, uint32_t cpt);
int32_t pdf_lookup_cmap_full(int32_t cmap, uint32_t cpt, int32_t * out);
void pdf_map_one_to_many(int32_t _ctx, int32_t cmap, uint32_t one, int32_t const * many, size_t len);
void pdf_map_range_to_range(int32_t _ctx, int32_t cmap, uint32_t srclo, uint32_t srchi, int32_t dstlo);
int32_t pdf_new_cmap(int32_t _ctx);
int32_t pdf_new_identity_cmap(int32_t _ctx, int32_t wmode, int32_t bytes);
void pdf_set_cmap_name(int32_t _ctx, int32_t cmap, const char * name);
void pdf_set_cmap_wmode(int32_t _ctx, int32_t cmap, int32_t wmode);
void pdf_set_usecmap(int32_t _ctx, int32_t cmap, int32_t usecmap);
void pdf_sort_cmap(int32_t _ctx, int32_t cmap);

#ifdef __cplusplus
}
#endif

#endif /* MUPDF_PDF_PDF_CMAP_H */
