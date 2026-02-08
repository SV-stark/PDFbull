// MicroPDF - MuPDF API Compatible C Header
// Auto-generated from Rust FFI - DO NOT EDIT MANUALLY
// Module: pdf_parse

#ifndef MUPDF_PDF_PDF_PARSE_H
#define MUPDF_PDF_PDF_PARSE_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================================
// Pdf_parse Functions (30 total)
// ============================================================================

void pdf_append_token(int32_t _ctx, int32_t buf, int32_t tok, int32_t lexbuf);
int32_t pdf_lex(int32_t _ctx, int32_t parser);
int32_t pdf_lex_no_string(int32_t _ctx, int32_t parser);
void pdf_lexbuf_fin(int32_t _ctx, int32_t lexbuf);
void pdf_lexbuf_free_string(int32_t _ctx, char * s);
float pdf_lexbuf_get_float(int32_t _ctx, int32_t lexbuf);
int64_t pdf_lexbuf_get_int(int32_t _ctx, int32_t lexbuf);
size_t pdf_lexbuf_get_len(int32_t _ctx, int32_t lexbuf);
const char * pdf_lexbuf_get_string(int32_t _ctx, int32_t lexbuf);
intptr_t pdf_lexbuf_grow(int32_t _ctx, int32_t lexbuf);
int32_t pdf_lexbuf_init(int32_t _ctx, int32_t size);
int32_t pdf_parse_array(int32_t _ctx, int32_t _doc, int32_t parser);
int32_t pdf_parse_dict(int32_t _ctx, int32_t _doc, int32_t parser);
int32_t pdf_parse_ind_obj(int32_t _ctx, int32_t _doc, int32_t parser, int32_t * num, int32_t * generation, int64_t * stm_ofs, int32_t * _try_repair);
int32_t pdf_parse_stm_obj(int32_t _ctx, int32_t _doc, int32_t parser);
int32_t pdf_parsed_array_len(int32_t _ctx, int32_t obj);
int32_t pdf_parsed_dict_len(int32_t _ctx, int32_t obj);
void pdf_parsed_obj_drop(int32_t _ctx, int32_t obj);
int32_t pdf_parsed_obj_type(int32_t _ctx, int32_t obj);
void pdf_parser_drop(int32_t _ctx, int32_t parser);
const char * pdf_parser_get_error(int32_t _ctx, int32_t parser);
float pdf_parser_get_float(int32_t _ctx, int32_t parser);
int64_t pdf_parser_get_int(int32_t _ctx, int32_t parser);
size_t pdf_parser_get_pos(int32_t _ctx, int32_t parser);
const char * pdf_parser_get_string(int32_t _ctx, int32_t parser);
int32_t pdf_parser_get_token(int32_t _ctx, int32_t parser);
int32_t pdf_parser_has_error(int32_t _ctx, int32_t parser);
int32_t pdf_parser_new(int32_t _ctx, u8 const * data, size_t len);
void pdf_parser_set_pos(int32_t _ctx, int32_t parser, size_t pos);
const char * pdf_token_name(int32_t tok);

#ifdef __cplusplus
}
#endif

#endif /* MUPDF_PDF_PDF_PARSE_H */
