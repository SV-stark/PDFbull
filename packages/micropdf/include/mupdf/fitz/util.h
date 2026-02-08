// MicroPDF - MuPDF API Compatible C Header
// Auto-generated from Rust FFI - DO NOT EDIT MANUALLY
// Module: util

#ifndef MUPDF_FITZ_UTIL_H
#define MUPDF_FITZ_UTIL_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================================
// Util Functions (36 total)
// ============================================================================

float fz_atof(const char * s);
int32_t fz_atoi(const char * s);
int64_t fz_atoi64(const char * s);
const char * fz_basename(const char * path);
int32_t fz_chartorune(int32_t * rune, const char * str);
int32_t fz_chartorunen(int32_t * rune, const char * str, size_t n);
char * fz_cleanname(char * name);
char * fz_decode_uri(int32_t _ctx, const char * s);
char * fz_decode_uri_component(int32_t _ctx, const char * s);
void fz_dirname(char * dir, const char * path, size_t dirsize);
char * fz_encode_uri(int32_t _ctx, const char * s);
char * fz_encode_uri_component(int32_t _ctx, const char * s);
char * fz_encode_uri_pathname(int32_t _ctx, const char * s);
void fz_format_output_path(int32_t _ctx, char * path, size_t size, const char * fmt, int32_t page);
void fz_free_string(int32_t _ctx, char * s);
int32_t fz_is_page_range(int32_t _ctx, const char * s);
void const * fz_memmem(void const * haystack, size_t haystacklen, void const * needle, size_t needlelen);
int32_t fz_runeidx(const char * str, const char * p);
int32_t fz_runelen(int32_t rune);
const char * fz_runeptr(const char * str, int32_t idx);
int32_t fz_runetochar(char * str, int32_t rune);
int32_t fz_strcasecmp(const char * a, const char * b);
char * fz_strdup(int32_t _ctx, const char * s);
size_t fz_strlcat(char * dst, const char * src, size_t n);
size_t fz_strlcpy(char * dst, const char * src, size_t n);
int32_t fz_strncasecmp(const char * a, const char * b, size_t n);
size_t fz_strnlen(const char * s, size_t maxlen);
char * fz_strsep(char * * stringp, const char * delim);
const char * fz_strstr(const char * haystack, const char * needle);
const char * fz_strstrcase(const char * haystack, const char * needle);
float fz_strtof(const char * s, char * * es);
int32_t fz_strverscmp(const char * s1, const char * s2);
int32_t fz_tolower(int32_t c);
int32_t fz_toupper(int32_t c);
char * fz_urldecode(char * url);
int32_t fz_utflen(const char * s);

#ifdef __cplusplus
}
#endif

#endif /* MUPDF_FITZ_UTIL_H */
