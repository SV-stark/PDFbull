/*
 * PDF Parse FFI
 *
 * Provides PDF lexer and parsing capabilities for PDF documents.
 */

#ifndef MICROPDF_PDF_PARSE_H
#define MICROPDF_PDF_PARSE_H

#include <stdint.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Handle types */
typedef uint64_t fz_context;
typedef uint64_t pdf_document;
typedef uint64_t pdf_lexbuf;
typedef uint64_t pdf_parser;
typedef uint64_t pdf_obj;
typedef uint64_t fz_stream;
typedef uint64_t fz_buffer;

/* Lexer buffer sizes */
#define PDF_LEXBUF_SMALL 256
#define PDF_LEXBUF_LARGE 65536

/* Token types */
typedef enum {
    PDF_TOK_ERROR = 0,
    PDF_TOK_EOF = 1,
    PDF_TOK_OPEN_ARRAY = 2,
    PDF_TOK_CLOSE_ARRAY = 3,
    PDF_TOK_OPEN_DICT = 4,
    PDF_TOK_CLOSE_DICT = 5,
    PDF_TOK_OPEN_BRACE = 6,
    PDF_TOK_CLOSE_BRACE = 7,
    PDF_TOK_NAME = 8,
    PDF_TOK_INT = 9,
    PDF_TOK_REAL = 10,
    PDF_TOK_STRING = 11,
    PDF_TOK_KEYWORD = 12,
    PDF_TOK_R = 13,
    PDF_TOK_TRUE = 14,
    PDF_TOK_FALSE = 15,
    PDF_TOK_NULL = 16,
    PDF_TOK_OBJ = 17,
    PDF_TOK_ENDOBJ = 18,
    PDF_TOK_STREAM = 19,
    PDF_TOK_ENDSTREAM = 20,
    PDF_TOK_XREF = 21,
    PDF_TOK_TRAILER = 22,
    PDF_TOK_STARTXREF = 23,
    PDF_TOK_NEWOBJ = 24,
    PDF_NUM_TOKENS = 25
} pdf_token;

/* ============================================================================
 * Lexer Buffer Functions
 * ============================================================================ */

/**
 * Initialize a lexer buffer.
 * @param ctx Context handle
 * @param size Initial buffer size
 * @return Lexer buffer handle, or 0 on failure
 */
pdf_lexbuf *pdf_lexbuf_init(fz_context *ctx, int size);

/**
 * Finalize (drop) a lexer buffer.
 */
void pdf_lexbuf_fin(fz_context *ctx, pdf_lexbuf *lexbuf);

/**
 * Grow the lexer buffer.
 * @return Amount grown, or 0 if growth failed
 */
ptrdiff_t pdf_lexbuf_grow(fz_context *ctx, pdf_lexbuf *lexbuf);

/**
 * Get the integer value from lexer buffer.
 */
int64_t pdf_lexbuf_get_int(fz_context *ctx, pdf_lexbuf *lexbuf);

/**
 * Get the float value from lexer buffer.
 */
float pdf_lexbuf_get_float(fz_context *ctx, pdf_lexbuf *lexbuf);

/**
 * Get the string length from lexer buffer.
 */
size_t pdf_lexbuf_get_len(fz_context *ctx, pdf_lexbuf *lexbuf);

/**
 * Get the string value from lexer buffer.
 * Caller must free the returned string with pdf_lexbuf_free_string.
 */
const char *pdf_lexbuf_get_string(fz_context *ctx, pdf_lexbuf *lexbuf);

/**
 * Free a string returned by pdf_lexbuf_get_string.
 */
void pdf_lexbuf_free_string(fz_context *ctx, char *s);

/* ============================================================================
 * Parser Functions
 * ============================================================================ */

/**
 * Create a new parser from data.
 * @param ctx Context handle
 * @param data Pointer to PDF data
 * @param len Length of data
 * @return Parser handle, or 0 on failure
 */
pdf_parser *pdf_parser_new(fz_context *ctx, const unsigned char *data, size_t len);

/**
 * Drop a parser.
 */
void pdf_parser_drop(fz_context *ctx, pdf_parser *parser);

/**
 * Lex the next token.
 * @return Token type (pdf_token)
 */
pdf_token pdf_lex(fz_context *ctx, pdf_parser *parser);

/**
 * Lex without processing string escapes (for faster scanning).
 */
pdf_token pdf_lex_no_string(fz_context *ctx, pdf_parser *parser);

/**
 * Get the current token type.
 */
pdf_token pdf_parser_get_token(fz_context *ctx, pdf_parser *parser);

/**
 * Get the integer value from parser.
 */
int64_t pdf_parser_get_int(fz_context *ctx, pdf_parser *parser);

/**
 * Get the float value from parser.
 */
float pdf_parser_get_float(fz_context *ctx, pdf_parser *parser);

/**
 * Get the string value from parser.
 * Caller must free the returned string.
 */
const char *pdf_parser_get_string(fz_context *ctx, pdf_parser *parser);

/**
 * Get the current position in the input.
 */
size_t pdf_parser_get_pos(fz_context *ctx, pdf_parser *parser);

/**
 * Set the current position in the input.
 */
void pdf_parser_set_pos(fz_context *ctx, pdf_parser *parser, size_t pos);

/**
 * Check if parser has error.
 * @return 1 if error, 0 otherwise
 */
int pdf_parser_has_error(fz_context *ctx, pdf_parser *parser);

/**
 * Get parser error message.
 */
const char *pdf_parser_get_error(fz_context *ctx, pdf_parser *parser);

/* ============================================================================
 * Object Parsing Functions
 * ============================================================================ */

/**
 * Parse a PDF array [...].
 * Must be called after lexing the opening bracket.
 * @return Parsed object handle, or 0 on failure
 */
pdf_obj *pdf_parse_array(fz_context *ctx, pdf_document *doc, pdf_parser *parser);

/**
 * Parse a PDF dictionary <<...>>.
 * Must be called after lexing the opening angle brackets.
 * @return Parsed object handle, or 0 on failure
 */
pdf_obj *pdf_parse_dict(fz_context *ctx, pdf_document *doc, pdf_parser *parser);

/**
 * Parse a stream object.
 * @return Parsed object handle, or 0 on failure
 */
pdf_obj *pdf_parse_stm_obj(fz_context *ctx, pdf_document *doc, pdf_parser *parser);

/**
 * Parse an indirect object (num gen obj ... endobj).
 * @param num Output for object number (may be NULL)
 * @param generation Output for generation number (may be NULL)
 * @param stm_ofs Output for stream offset (may be NULL)
 * @param try_repair Output for repair flag (may be NULL)
 * @return Parsed object handle, or 0 on failure
 */
pdf_obj *pdf_parse_ind_obj(
    fz_context *ctx,
    pdf_document *doc,
    pdf_parser *parser,
    int *num,
    int *generation,
    int64_t *stm_ofs,
    int *try_repair
);

/* ============================================================================
 * Parsed Object Access
 * ============================================================================ */

/**
 * Drop a parsed object.
 */
void pdf_parsed_obj_drop(fz_context *ctx, pdf_obj *obj);

/**
 * Get the type of a parsed object.
 * @return Type code (0=null, 1=bool, 2=int, 3=real, 4=string, 5=name, 6=array, 7=dict, 8=ref, 9=indirect)
 */
int pdf_parsed_obj_type(fz_context *ctx, pdf_obj *obj);

/**
 * Get array length.
 */
int pdf_parsed_array_len(fz_context *ctx, pdf_obj *obj);

/**
 * Get dict length (number of key-value pairs).
 */
int pdf_parsed_dict_len(fz_context *ctx, pdf_obj *obj);

/* ============================================================================
 * Token Utilities
 * ============================================================================ */

/**
 * Append a token representation to a buffer.
 */
void pdf_append_token(fz_context *ctx, fz_buffer *buf, pdf_token tok, pdf_lexbuf *lex);

/**
 * Get token name string.
 */
const char *pdf_token_name(pdf_token tok);

#ifdef __cplusplus
}
#endif

#endif /* MICROPDF_PDF_PARSE_H */


