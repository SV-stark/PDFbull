// MicroPDF - MuPDF API Compatible C Header
// Auto-generated from Rust FFI - DO NOT EDIT MANUALLY
// Module: struct_layout

#ifndef MUPDF_FITZ_STRUCT_LAYOUT_H
#define MUPDF_FITZ_STRUCT_LAYOUT_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================================
// Struct_layout Functions (12 total)
// ============================================================================

size_t fz_cache_line_size(void);
size_t fz_cache_padding(size_t size);
int fz_fits_in_cache_lines(size_t size, size_t lines);
int fz_is_cache_aligned(void const * ptr);
int fz_is_page_aligned(void const * ptr);
FfiLayoutInfo fz_layout_matrix(void);
FfiLayoutInfo fz_layout_point(void);
FfiLayoutInfo fz_layout_quad(void);
FfiLayoutInfo fz_layout_rect(void);
size_t fz_page_size(void);
size_t fz_round_to_cache_line(size_t size);
size_t fz_round_to_page(size_t size);

#ifdef __cplusplus
}
#endif

#endif /* MUPDF_FITZ_STRUCT_LAYOUT_H */
