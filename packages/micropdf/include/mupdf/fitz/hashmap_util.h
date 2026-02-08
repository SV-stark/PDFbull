// MicroPDF - MuPDF API Compatible C Header
// Auto-generated from Rust FFI - DO NOT EDIT MANUALLY
// Module: hashmap_util

#ifndef MUPDF_FITZ_HASHMAP_UTIL_H
#define MUPDF_FITZ_HASHMAP_UTIL_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================================
// Hashmap_util Functions (10 total)
// ============================================================================

uint64_t fz_hash_pdf_name(const char * name);
int fz_is_standard_name(const char * name);
int fz_lookup_standard_name(const char * name);
size_t fz_new_font_dict_capacity(void);
size_t fz_new_image_dict_capacity(void);
size_t fz_new_page_dict_capacity(void);
size_t fz_new_resources_dict_capacity(void);
size_t fz_new_stream_dict_capacity(void);
int fz_standard_name_count(void);
const char * fz_standard_name_str(int index);

#ifdef __cplusplus
}
#endif

#endif /* MUPDF_FITZ_HASHMAP_UTIL_H */
