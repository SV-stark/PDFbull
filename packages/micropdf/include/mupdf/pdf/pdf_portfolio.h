// MicroPDF - MuPDF API Compatible C Header
// Auto-generated from Rust FFI - DO NOT EDIT MANUALLY
// Module: pdf_portfolio

#ifndef MUPDF_PDF_PDF_PORTFOLIO_H
#define MUPDF_PDF_PDF_PORTFOLIO_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================================
// Pdf_portfolio Functions (21 total)
// ============================================================================

char * pdf_af_relationship_to_string(int32_t _ctx, int32_t relationship);
char * pdf_collection_view_to_string(int32_t _ctx, int32_t view);
void pdf_drop_portfolio(int32_t _ctx, int32_t portfolio);
int32_t pdf_is_portfolio(int32_t _ctx, int32_t portfolio);
int32_t pdf_new_portfolio(int32_t _ctx, int32_t doc);
int32_t pdf_portfolio_add_file(int32_t _ctx, int32_t portfolio, const char * name, u8 const * data, size_t len, const char * mime_type);
int32_t pdf_portfolio_add_schema_field(int32_t _ctx, int32_t portfolio, const char * key, const char * name, char field_type);
int32_t pdf_portfolio_count(int32_t _ctx, int32_t portfolio);
void pdf_portfolio_free_string(char * s);
u8 const * pdf_portfolio_get_file(int32_t _ctx, int32_t portfolio, const char * name, size_t * len_out);
int64_t pdf_portfolio_get_file_size(int32_t _ctx, int32_t portfolio, const char * name);
char * pdf_portfolio_get_mime_type(int32_t _ctx, int32_t portfolio, const char * name);
char * pdf_portfolio_get_name(int32_t _ctx, int32_t portfolio, int32_t index);
int32_t pdf_portfolio_get_view(int32_t _ctx, int32_t portfolio);
int32_t pdf_portfolio_remove_file(int32_t _ctx, int32_t portfolio, const char * name);
int32_t pdf_portfolio_schema_field_count(int32_t _ctx, int32_t portfolio);
char * pdf_portfolio_schema_field_key(int32_t _ctx, int32_t portfolio, int32_t index);
int32_t pdf_portfolio_set_description(int32_t _ctx, int32_t portfolio, const char * name, const char * description);
int32_t pdf_portfolio_set_initial_document(int32_t _ctx, int32_t portfolio, const char * name);
int32_t pdf_portfolio_set_sort(int32_t _ctx, int32_t portfolio, const char * field, int32_t ascending);
int32_t pdf_portfolio_set_view(int32_t _ctx, int32_t portfolio, int32_t view);

#ifdef __cplusplus
}
#endif

#endif /* MUPDF_PDF_PDF_PORTFOLIO_H */
