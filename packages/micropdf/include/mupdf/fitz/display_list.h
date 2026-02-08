// MicroPDF - MuPDF API Compatible C Header
// Auto-generated from Rust FFI - DO NOT EDIT MANUALLY
// Module: display_list

#ifndef MUPDF_FITZ_DISPLAY_LIST_H
#define MUPDF_FITZ_DISPLAY_LIST_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================================
// Display_list Functions (10 total)
// ============================================================================

fz_rect fz_bound_display_list(int32_t _ctx, int32_t list);
int32_t fz_clone_display_list(int32_t _ctx, int32_t list);
void fz_display_list_clear(int32_t _ctx, int32_t list);
int32_t fz_display_list_count_commands(int32_t _ctx, int32_t list);
int32_t fz_display_list_is_empty(int32_t _ctx, int32_t list);
int32_t fz_display_list_is_valid(int32_t _ctx, int32_t list);
void fz_drop_display_list(int32_t _ctx, int32_t list);
int32_t fz_keep_display_list(int32_t _ctx, int32_t list);
int32_t fz_new_display_list(int32_t _ctx, float x0, float y0, float x1, float y1);
void fz_run_display_list(int32_t _ctx, int32_t list, int32_t dev, fz_matrix ctm, fz_rect scissor);

#ifdef __cplusplus
}
#endif

#endif /* MUPDF_FITZ_DISPLAY_LIST_H */
