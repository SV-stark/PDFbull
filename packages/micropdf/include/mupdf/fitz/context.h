// MicroPDF - MuPDF API Compatible C Header
// Auto-generated from Rust FFI - DO NOT EDIT MANUALLY
// Module: context

#ifndef MUPDF_FITZ_CONTEXT_H
#define MUPDF_FITZ_CONTEXT_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================================
// Context Functions (28 total)
// ============================================================================

int fz_aa_level(int32_t ctx);
int fz_caught(int32_t ctx);
const char * fz_caught_message(int32_t ctx);
int32_t fz_clone_context(int32_t ctx);
int fz_context_is_valid(int32_t ctx);
const char * fz_convert_error(int32_t ctx, int * code);
void fz_disable_icc(int32_t ctx);
void fz_drop_context(int32_t ctx);
void fz_empty_store(int32_t _ctx);
void fz_enable_icc(int32_t ctx);
void fz_flush_warnings(int32_t _ctx);
int fz_has_error(int32_t ctx);
void fz_ignore_error(int32_t ctx);
int32_t fz_keep_context(int32_t ctx);
int32_t fz_new_context(void const * _alloc, void const * we use Rust allocator _locks, size_t we use Rust sync max_store);
int32_t fz_new_default_context(void);
void fz_report_error(int32_t ctx);
void fz_rethrow(int32_t ctx);
void fz_set_aa_level(int32_t ctx, int bits);
void fz_set_error_callback(int32_t ctx, Option<unsafe extern "C" fn(*mut c_void, c_int, *const c_char)> callback, void * user);
void fz_set_user_context(int32_t ctx, void * user);
void fz_set_warning_callback(int32_t ctx, Option<unsafe extern "C" fn(*mut c_void, *const c_char)> callback, void * user);
void fz_shrink_store(int32_t _ctx, int percent);
int fz_store_scavenge(int32_t _ctx, size_t size, int * phase);
size_t fz_store_size(int32_t ctx);
void fz_throw(int32_t ctx, int errcode, const char * fmt);
void * fz_user_context(int32_t ctx);
void fz_warn(int32_t ctx, const char * fmt);

#ifdef __cplusplus
}
#endif

#endif /* MUPDF_FITZ_CONTEXT_H */
