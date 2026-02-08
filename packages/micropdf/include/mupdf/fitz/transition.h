// MicroPDF - MuPDF API Compatible C Header
// Auto-generated from Rust FFI - DO NOT EDIT MANUALLY
// Module: transition

#ifndef MUPDF_FITZ_TRANSITION_H
#define MUPDF_FITZ_TRANSITION_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================================
// Transition Functions (23 total)
// ============================================================================

void fz_drop_transition(int32_t _ctx, int32_t trans);
int32_t fz_generate_transition(int32_t _ctx, int32_t tpix, int32_t opix, int32_t npix, int32_t time, int32_t trans);
int32_t fz_new_blinds_transition(float duration, int32_t vertical);
int32_t fz_new_box_transition(float duration, int32_t outwards);
int32_t fz_new_cover_transition(float duration, int32_t direction);
int32_t fz_new_dissolve_transition(float duration);
int32_t fz_new_fade_transition(float duration);
int32_t fz_new_fly_transition(float duration, int32_t direction, int32_t outwards);
int32_t fz_new_glitter_transition(float duration, int32_t direction);
int32_t fz_new_push_transition(float duration, int32_t direction);
int32_t fz_new_split_transition(float duration, int32_t vertical, int32_t outwards);
int32_t fz_new_transition(int32_t transition_type, float duration);
int32_t fz_new_uncover_transition(float duration, int32_t direction);
int32_t fz_new_wipe_transition(float duration, int32_t direction);
int32_t fz_transition_direction(int32_t trans);
float fz_transition_duration(int32_t trans);
int32_t fz_transition_outwards(int32_t trans);
void fz_transition_set_direction(int32_t trans, int32_t direction);
void fz_transition_set_duration(int32_t trans, float duration);
void fz_transition_set_outwards(int32_t trans, int32_t outwards);
void fz_transition_set_vertical(int32_t trans, int32_t vertical);
int32_t fz_transition_type(int32_t trans);
int32_t fz_transition_vertical(int32_t trans);

#ifdef __cplusplus
}
#endif

#endif /* MUPDF_FITZ_TRANSITION_H */
