// MicroPDF - MuPDF API Compatible C Header
// Auto-generated from Rust FFI - DO NOT EDIT MANUALLY
// Module: log

#ifndef MUPDF_FITZ_LOG_H
#define MUPDF_FITZ_LOG_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================================
// Log Functions (27 total)
// ============================================================================

void fz_clear_module_log_level(int32_t _ctx, const char * module);
size_t fz_get_log_buffer_size(int32_t _ctx);
int32_t fz_get_log_level(int32_t _ctx);
int32_t fz_get_module_log_level(int32_t _ctx, const char * module);
void fz_log(int32_t _ctx, const char * message);
void fz_log_buffer_clear(int32_t _ctx);
size_t fz_log_buffer_count(int32_t _ctx);
size_t fz_log_buffer_get(int32_t _ctx, size_t index, char * output, size_t output_size);
void fz_log_debug(int32_t _ctx, const char * message);
void fz_log_error(int32_t _ctx, const char * message);
void fz_log_fl(int32_t _ctx, int32_t level, const char * file, int32_t line, const char * message);
void fz_log_include_location(int32_t _ctx, int32_t include);
void fz_log_include_timestamp(int32_t _ctx, int32_t include);
const char * fz_log_last_warning(int32_t _ctx);
void fz_log_level(int32_t _ctx, int32_t level, const char * message);
const char * fz_log_level_name(int32_t level);
void fz_log_module(int32_t _ctx, const char * module, const char * message);
void fz_log_set_warning_callback(int32_t _ctx, WarningCallback callback, void * user);
void fz_log_trace(int32_t _ctx, const char * message);
void fz_log_warn(int32_t _ctx, const char * message);
WarningCallback fz_log_warning_callback(int32_t _ctx, void * * user);
int32_t fz_parse_log_level(const char * name);
void fz_set_log_buffer_size(int32_t _ctx, size_t size);
void fz_set_log_callback(int32_t _ctx, LogCallback callback, void * user);
void fz_set_log_file(int32_t _ctx, const char * path);
void fz_set_log_level(int32_t _ctx, int32_t level);
void fz_set_module_log_level(int32_t _ctx, const char * module, int32_t level);

#ifdef __cplusplus
}
#endif

#endif /* MUPDF_FITZ_LOG_H */
