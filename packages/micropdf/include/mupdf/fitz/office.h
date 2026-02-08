// MicroPDF - MuPDF API Compatible C Header
// Auto-generated from Rust FFI - DO NOT EDIT MANUALLY
// Module: office

#ifndef MUPDF_FITZ_OFFICE_H
#define MUPDF_FITZ_OFFICE_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================================
// Office Functions (30 total)
// ============================================================================

int32_t office_add_heading(int32_t _ctx, int32_t doc, const char * text, int32_t level);
int32_t office_add_paragraph(int32_t _ctx, int32_t doc, const char * text);
int32_t office_add_sheet(int32_t _ctx, int32_t doc, const char * name);
int32_t office_add_slide(int32_t _ctx, int32_t doc);
int32_t office_content_count(int32_t _ctx, int32_t doc);
void office_drop_document(int32_t _ctx, int32_t doc);
void office_free_string(char * s);
char * office_get_cell_string(int32_t _ctx, int32_t doc, int32_t sheet_idx, int32_t row, int32_t col);
char * office_get_creator(int32_t _ctx, int32_t doc);
int32_t office_get_page_size(int32_t _ctx, int32_t doc, float * width, float * height);
char * office_get_sheet_name(int32_t _ctx, int32_t doc, int32_t sheet_idx);
char * office_get_slide_title(int32_t _ctx, int32_t doc, int32_t slide_num);
char * office_get_title(int32_t _ctx, int32_t doc);
int32_t office_get_type(int32_t _ctx, int32_t doc);
int32_t office_new_document(int32_t ctx, int32_t doc_type);
int32_t office_new_docx(int32_t ctx);
int32_t office_new_pptx(int32_t ctx);
int32_t office_new_xlsx(int32_t ctx);
int32_t office_open_document(int32_t ctx, const char * filename);
int32_t office_page_count(int32_t _ctx, int32_t doc);
int32_t office_set_cell_number(int32_t _ctx, int32_t doc, int32_t sheet_idx, int32_t row, int32_t col, double value);
int32_t office_set_cell_string(int32_t _ctx, int32_t doc, int32_t sheet_idx, int32_t row, int32_t col, const char * value);
int32_t office_set_creator(int32_t _ctx, int32_t doc, const char * creator);
int32_t office_set_page_size(int32_t _ctx, int32_t doc, float width, float height);
int32_t office_set_slide_title(int32_t _ctx, int32_t doc, int32_t slide_num, const char * title);
int32_t office_set_title(int32_t _ctx, int32_t doc, const char * title);
int32_t office_sheet_count(int32_t _ctx, int32_t doc);
int32_t office_slide_count(int32_t _ctx, int32_t doc);
char * office_type_extension(int32_t _ctx, int32_t doc_type);
char * office_type_name(int32_t _ctx, int32_t doc_type);

#ifdef __cplusplus
}
#endif

#endif /* MUPDF_FITZ_OFFICE_H */
