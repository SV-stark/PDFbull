// MicroPDF - MuPDF API Compatible C Header
// Auto-generated from Rust FFI - DO NOT EDIT MANUALLY
// Module: separation

#ifndef MUPDF_FITZ_SEPARATION_H
#define MUPDF_FITZ_SEPARATION_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================================
// Separation Functions (23 total)
// ============================================================================

int32_t fz_add_separation(int32_t _ctx, int32_t seps, const char * name, uint64_t _colorspace, float cmyk_c, float cmyk_m, float cmyk_y, float cmyk_k);
int32_t fz_add_separation_all(int32_t _ctx, int32_t seps);
int32_t fz_add_separation_none(int32_t _ctx, int32_t seps);
int32_t fz_clone_separations(int32_t _ctx, int32_t seps);
void fz_convert_separation_colors(int32_t _ctx, int32_t seps, float const * src, int32_t src_n, float * dst, int32_t dst_n);
int32_t fz_count_active_separations(int32_t _ctx, int32_t seps);
int32_t fz_count_separations(int32_t _ctx, int32_t seps);
void fz_disable_all_separations(int32_t _ctx, int32_t seps);
void fz_drop_separations(int32_t _ctx, int32_t seps);
int32_t fz_keep_separations(int32_t _ctx, int32_t seps);
int32_t fz_new_separations(int32_t _ctx, int32_t controllable);
int32_t fz_separation_current_behavior(int32_t _ctx, int32_t seps, int32_t idx);
void fz_separation_equivalent(int32_t _ctx, int32_t seps, int32_t idx, float * cmyk);
int32_t fz_separation_is_all(int32_t _ctx, int32_t seps, int32_t idx);
int32_t fz_separation_is_none(int32_t _ctx, int32_t seps, int32_t idx);
const char * fz_separation_name(int32_t _ctx, int32_t seps, int32_t idx);
int32_t fz_separations_controllable(int32_t _ctx, int32_t seps);
int32_t fz_separations_equal(int32_t _ctx, int32_t seps1, int32_t seps2);
int32_t fz_separations_have_spots(int32_t _ctx, int32_t seps);
void fz_set_all_separations_to_composite(int32_t _ctx, int32_t seps);
void fz_set_all_separations_to_spot(int32_t _ctx, int32_t seps);
void fz_set_separation_behavior(int32_t _ctx, int32_t seps, int32_t idx, int32_t behavior);
void fz_set_separation_equivalent(int32_t _ctx, int32_t seps, int32_t idx, float const * cmyk);

#ifdef __cplusplus
}
#endif

#endif /* MUPDF_FITZ_SEPARATION_H */
