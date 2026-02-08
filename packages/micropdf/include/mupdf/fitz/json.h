// MicroPDF - MuPDF API Compatible C Header
// Auto-generated from Rust FFI - DO NOT EDIT MANUALLY
// Module: json

#ifndef MUPDF_FITZ_JSON_H
#define MUPDF_FITZ_JSON_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================================
// Json Functions (29 total)
// ============================================================================

void fz_drop_json(int32_t _ctx, int32_t json);
int32_t fz_json_array_get(int32_t _ctx, int32_t json, int32_t index);
int32_t fz_json_array_length(int32_t _ctx, int32_t json);
int32_t fz_json_array_push(int32_t _ctx, int32_t _pool, int32_t array, int32_t item);
size_t fz_json_escape_string(const char * input, char * output, size_t output_size);
int32_t fz_json_is_array(int32_t _ctx, int32_t json);
int32_t fz_json_is_boolean(int32_t _ctx, int32_t json);
int32_t fz_json_is_null(int32_t _ctx, int32_t json);
int32_t fz_json_is_number(int32_t _ctx, int32_t json);
int32_t fz_json_is_object(int32_t _ctx, int32_t json);
int32_t fz_json_is_string(int32_t _ctx, int32_t json);
int32_t fz_json_new_array(int32_t _ctx, int32_t _pool);
int32_t fz_json_new_boolean(int32_t _ctx, int32_t _pool, int32_t value);
int32_t fz_json_new_null(int32_t _ctx, int32_t _pool);
int32_t fz_json_new_number(int32_t _ctx, int32_t _pool, double value);
int32_t fz_json_new_object(int32_t _ctx, int32_t _pool);
int32_t fz_json_new_string(int32_t _ctx, int32_t _pool, const char * value);
int32_t fz_json_object_get(int32_t _ctx, int32_t json, const char * key);
int32_t fz_json_object_length(int32_t _ctx, int32_t json);
int32_t fz_json_object_set(int32_t _ctx, int32_t _pool, int32_t object, const char * key, int32_t item);
size_t fz_json_string_length(int32_t _ctx, int32_t json);
int32_t fz_json_to_boolean(int32_t _ctx, int32_t json);
double fz_json_to_number(int32_t _ctx, int32_t json);
size_t fz_json_to_string(int32_t _ctx, int32_t json, char * output, size_t output_size);
int32_t fz_json_type(int32_t _ctx, int32_t json);
size_t fz_json_unescape_string(const char * input, char * output, size_t output_size);
int32_t fz_parse_json(int32_t _ctx, int32_t _pool, const char * input);
size_t fz_write_json(int32_t _ctx, int32_t json, char * output, size_t output_size);
size_t fz_write_json_pretty(int32_t _ctx, int32_t json, size_t indent, char * output, size_t output_size);

#ifdef __cplusplus
}
#endif

#endif /* MUPDF_FITZ_JSON_H */
