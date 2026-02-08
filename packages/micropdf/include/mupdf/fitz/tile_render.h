// MicroPDF - MuPDF API Compatible C Header
// Auto-generated from Rust FFI - DO NOT EDIT MANUALLY
// Module: tile_render

#ifndef MUPDF_FITZ_TILE_RENDER_H
#define MUPDF_FITZ_TILE_RENDER_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================================
// Tile_render Functions (12 total)
// ============================================================================

void fz_cancel_render_task(int32_t _ctx, int32_t task);
void fz_drop_render_task(int32_t _ctx, int32_t task);
void fz_drop_tile_renderer(int32_t _ctx, int32_t renderer);
int32_t fz_new_render_task(int32_t _ctx, int32_t renderer);
int32_t fz_new_tile_renderer(int32_t _ctx, float x0, float y0, float x1, float y1, uint32_t tile_width, uint32_t tile_height, float scale, int alpha);
int fz_render_task_is_cancelled(int32_t _ctx, int32_t task);
int fz_tile_renderer_count(int32_t _ctx, int32_t renderer);
void fz_tile_renderer_dimensions(int32_t _ctx, int32_t renderer, uint32_t * width, uint32_t * height);
int fz_tile_renderer_get_bounds(int32_t _ctx, int32_t renderer, int index, float * x0, float * y0, float * x1, float * y1);
void fz_tile_renderer_grid(int32_t _ctx, int32_t renderer, uint32_t * cols, uint32_t * rows);
int fz_tile_renderer_is_complete(int32_t _ctx, int32_t renderer);
float fz_tile_renderer_progress(int32_t _ctx, int32_t renderer);

#ifdef __cplusplus
}
#endif

#endif /* MUPDF_FITZ_TILE_RENDER_H */
