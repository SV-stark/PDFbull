// MicroPDF - MuPDF API Compatible C Header
// Auto-generated from Rust FFI - DO NOT EDIT MANUALLY
// Module: bitmap

#ifndef MUPDF_FITZ_BITMAP_H
#define MUPDF_FITZ_BITMAP_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================================
// Bitmap Functions (20 total)
// ============================================================================

void fz_bitmap_clear(int32_t _ctx, int32_t bitmap);
size_t fz_bitmap_compress_rle(int32_t _ctx, int32_t bitmap, u8 * output, size_t max_size);
size_t fz_bitmap_compressed_size(int32_t _ctx, int32_t bitmap, int32_t _compression);
u8 const * fz_bitmap_data(int32_t _ctx, int32_t bitmap);
size_t fz_bitmap_data_size(int32_t _ctx, int32_t bitmap);
void fz_bitmap_fill(int32_t _ctx, int32_t bitmap);
int32_t fz_bitmap_get_pixel(int32_t _ctx, int32_t bitmap, int32_t x, int32_t y);
int32_t fz_bitmap_height(int32_t _ctx, int32_t bitmap);
void fz_bitmap_invert(int32_t _ctx, int32_t bitmap);
void fz_bitmap_set_pixel(int32_t _ctx, int32_t bitmap, int32_t x, int32_t y, int32_t value);
void fz_bitmap_set_res(int32_t _ctx, int32_t bitmap, int32_t x_res, int32_t y_res);
int32_t fz_bitmap_stride(int32_t _ctx, int32_t bitmap);
int32_t fz_bitmap_width(int32_t _ctx, int32_t bitmap);
int32_t fz_bitmap_x_res(int32_t _ctx, int32_t bitmap);
int32_t fz_bitmap_y_res(int32_t _ctx, int32_t bitmap);
void fz_drop_bitmap(int32_t _ctx, int32_t bitmap);
int32_t fz_keep_bitmap(int32_t _ctx, int32_t bitmap);
int32_t fz_new_bitmap(int32_t _ctx, int32_t width, int32_t height, int32_t x_res, int32_t y_res);
int32_t fz_new_bitmap_from_pixmap(int32_t _ctx, int32_t pixmap, int32_t threshold);
int32_t fz_new_bitmap_from_pixmap_halftone(int32_t _ctx, int32_t pixmap, int32_t halftone_type);

#ifdef __cplusplus
}
#endif

#endif /* MUPDF_FITZ_BITMAP_H */
