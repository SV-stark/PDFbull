// MicroPDF - MuPDF API Compatible C Header
// Auto-generated from Rust FFI - DO NOT EDIT MANUALLY
// Module: cookie

#ifndef MUPDF_FITZ_COOKIE_H
#define MUPDF_FITZ_COOKIE_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================================
// Cookie Functions (24 total)
// ============================================================================

int32_t fz_clone_cookie(int32_t _ctx, int32_t cookie);
void fz_cookie_abort(int32_t _ctx, int32_t cookie);
int32_t fz_cookie_get_errors(int32_t _ctx, int32_t cookie);
int32_t fz_cookie_get_progress(int32_t _ctx, int32_t cookie);
int32_t fz_cookie_get_progress_max(int32_t _ctx, int32_t cookie);
int32_t fz_cookie_has_errors(int32_t _ctx, int32_t cookie);
void fz_cookie_inc_errors(int32_t _ctx, int32_t cookie);
void fz_cookie_inc_progress(int32_t _ctx, int32_t cookie);
int32_t fz_cookie_is_complete(int32_t _ctx, int32_t cookie);
int32_t fz_cookie_is_incomplete(int32_t _ctx, int32_t cookie);
int32_t fz_cookie_is_valid(int32_t _ctx, int32_t cookie);
float fz_cookie_progress_float(int32_t _ctx, int32_t cookie);
int32_t fz_cookie_progress_percent(int32_t _ctx, int32_t cookie);
int32_t fz_cookie_progress_remaining(int32_t _ctx, int32_t cookie);
void fz_cookie_reset(int32_t _ctx, int32_t cookie);
void fz_cookie_reset_abort(int32_t _ctx, int32_t cookie);
void fz_cookie_set_errors(int32_t _ctx, int32_t cookie, int32_t count);
void fz_cookie_set_incomplete(int32_t _ctx, int32_t cookie, int32_t value);
void fz_cookie_set_progress(int32_t _ctx, int32_t cookie, int32_t value);
void fz_cookie_set_progress_max(int32_t _ctx, int32_t cookie, int32_t value);
int32_t fz_cookie_should_abort(int32_t _ctx, int32_t cookie);
void fz_drop_cookie(int32_t _ctx, int32_t cookie);
int32_t fz_keep_cookie(int32_t _ctx, int32_t cookie);
int32_t fz_new_cookie(int32_t _ctx);

#ifdef __cplusplus
}
#endif

#endif /* MUPDF_FITZ_COOKIE_H */
