// MicroPDF - MuPDF API Compatible C Header
// Auto-generated from Rust FFI - DO NOT EDIT MANUALLY
// Module: story

#ifndef MUPDF_FITZ_STORY_H
#define MUPDF_FITZ_STORY_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================================
// Story Functions (15 total)
// ============================================================================

void fz_draw_story(int32_t _ctx, int32_t story, int32_t dev, float ctm_a, float ctm_b, float ctm_c, float ctm_d, float ctm_e, float ctm_f);
void fz_drop_story(int32_t _ctx, int32_t story);
int32_t fz_new_story(int32_t _ctx, int32_t buf, const char * user_css, float em, int32_t archive);
int32_t fz_place_story(int32_t _ctx, int32_t story, float where_x0, float where_y0, float where_x1, float where_y1, Rect * filled);
int32_t fz_place_story_flags(int32_t _ctx, int32_t story, float where_x0, float where_y0, float where_x1, float where_y1, Rect * filled, int32_t flags);
void fz_reset_story(int32_t _ctx, int32_t story);
int32_t fz_story_document(int32_t _ctx, int32_t story);
float fz_story_em(int32_t story);
int32_t fz_story_is_complete(int32_t story);
int32_t fz_story_placed_regions_count(int32_t story);
void fz_story_positions(int32_t ctx, int32_t story, StoryPositionCallback callback, c_void * arg);
int32_t fz_story_rectangle_num(int32_t story);
void fz_story_set_em(int32_t story, float em);
int32_t fz_story_state(int32_t story);
const char * fz_story_warnings(int32_t _ctx, int32_t story);

#ifdef __cplusplus
}
#endif

#endif /* MUPDF_FITZ_STORY_H */
