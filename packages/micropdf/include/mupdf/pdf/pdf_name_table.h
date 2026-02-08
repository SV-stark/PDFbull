// MicroPDF - MuPDF API Compatible C Header
// Auto-generated from Rust FFI - DO NOT EDIT MANUALLY
// Module: pdf_name_table

#ifndef MUPDF_PDF_PDF_NAME_TABLE_H
#define MUPDF_PDF_PDF_NAME_TABLE_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================================
// Pdf_name_table Functions (17 total)
// ============================================================================

void pdf_free_name_string(char * name);
char * pdf_get_interned_name(int32_t idx);
int32_t pdf_intern_name(const char * name);
int32_t pdf_lookup_name(const char * name);
int32_t pdf_name_eq_str(int32_t idx, const char * name);
int32_t pdf_name_index_eq(int32_t a, int32_t b);
int32_t pdf_name_table_count(void);
double pdf_name_table_hit_rate(void);
uint64_t pdf_name_table_hits(void);
uint64_t pdf_name_table_lookups(void);
void pdf_release_name(int32_t idx);
int32_t pdf_std_name_filter(void);
int32_t pdf_std_name_font(void);
int32_t pdf_std_name_image(void);
int32_t pdf_std_name_length(void);
int32_t pdf_std_name_subtype(void);
int32_t pdf_std_name_type(void);

#ifdef __cplusplus
}
#endif

#endif /* MUPDF_PDF_PDF_NAME_TABLE_H */
