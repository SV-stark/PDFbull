// MicroPDF - MuPDF API Compatible C Header
// Auto-generated from Rust FFI - DO NOT EDIT MANUALLY
// Module: stext

#ifndef MUPDF_FITZ_STEXT_H
#define MUPDF_FITZ_STEXT_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================================
// Stext Functions (37 total)
// ============================================================================

int32_t fz_add_stext_block(int32_t _ctx, int32_t page, float x0, float y0, float x1, float y1);
int32_t fz_add_stext_char(int32_t _ctx, int32_t page, int32_t block_idx, int32_t line_idx, int32_t c, float x, float y, float size);
int32_t fz_add_stext_line(int32_t _ctx, int32_t page, int32_t block_idx, float x0, float y0, float x1, float y1);
const char * fz_copy_rectangle(int32_t ctx, int32_t page, float x0, float y0, float x1, float y1, int32_t crlf);
const char * fz_copy_selection(int32_t _ctx, int32_t page, float a_x, float a_y, float b_x, float b_y, int32_t crlf);
StextOptions * fz_default_stext_options(int32_t _ctx, StextOptions * opts);
void fz_drop_stext_page(int32_t _ctx, int32_t page);
int32_t fz_highlight_selection(int32_t _ctx, int32_t page, float a_x, float a_y, float b_x, float b_y, FzQuad * quads, int32_t max_quads);
int32_t fz_keep_stext_page(int32_t _ctx, int32_t page);
int32_t fz_new_stext_page(int32_t _ctx, float x0, float y0, float x1, float y1);
void fz_paragraph_break(int32_t _ctx, int32_t page);
StextOptions * fz_parse_stext_options(int32_t _ctx, StextOptions * opts, const char * string);
const char * fz_print_stext_page_as_html(int32_t _ctx, int32_t _output, int32_t page, int32_t _id);
const char * fz_print_stext_page_as_json(int32_t _ctx, int32_t _output, int32_t page, float _scale);
const char * fz_print_stext_page_as_xml(int32_t _ctx, int32_t _output, int32_t page, int32_t _id);
int32_t fz_search_stext_page(int32_t _ctx, int32_t page, const char * needle, int32_t * hit_mark, FzQuad * hit_bbox, int32_t hit_max);
int32_t fz_segment_stext_page(int32_t _ctx, int32_t page);
void fz_stext_block_bbox(int32_t _ctx, int32_t page, int32_t block_idx, float * x0, float * y0, float * x1, float * y1);
int32_t fz_stext_block_count(int32_t _ctx, int32_t page);
int32_t fz_stext_block_type(int32_t _ctx, int32_t page, int32_t block_idx);
int32_t fz_stext_char_count(int32_t _ctx, int32_t page, int32_t block_idx, int32_t line_idx);
void fz_stext_char_origin(int32_t _ctx, int32_t page, int32_t block_idx, int32_t line_idx, int32_t char_idx, float * x, float * y);
void fz_stext_char_quad(int32_t _ctx, int32_t page, int32_t block_idx, int32_t line_idx, int32_t char_idx, FzQuad * quad);
float fz_stext_char_size(int32_t _ctx, int32_t page, int32_t block_idx, int32_t line_idx, int32_t char_idx);
int32_t fz_stext_char_value(int32_t _ctx, int32_t page, int32_t block_idx, int32_t line_idx, int32_t char_idx);
int32_t fz_stext_first_block(int32_t _ctx, int32_t page);
int32_t fz_stext_first_char(int32_t _ctx, int32_t page, int32_t block_idx, int32_t line_idx);
int32_t fz_stext_first_line(int32_t _ctx, int32_t page, int32_t block_idx);
void fz_stext_line_bbox(int32_t _ctx, int32_t page, int32_t block_idx, int32_t line_idx, float * x0, float * y0, float * x1, float * y1);
int32_t fz_stext_line_count(int32_t _ctx, int32_t page, int32_t block_idx);
int32_t fz_stext_line_wmode(int32_t _ctx, int32_t page, int32_t block_idx, int32_t line_idx);
int32_t fz_stext_next_block(int32_t _ctx, int32_t page, int32_t block_idx);
int32_t fz_stext_next_char(int32_t _ctx, int32_t page, int32_t block_idx, int32_t line_idx, int32_t char_idx);
int32_t fz_stext_next_line(int32_t _ctx, int32_t page, int32_t block_idx, int32_t line_idx);
const char * fz_stext_page_as_text(int32_t _ctx, int32_t page);
void fz_stext_page_mediabox(int32_t _ctx, int32_t page, float * x0, float * y0, float * x1, float * y1);
void fz_table_hunt(int32_t _ctx, int32_t page);

#ifdef __cplusplus
}
#endif

#endif /* MUPDF_FITZ_STEXT_H */
