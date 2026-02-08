// MicroPDF - MuPDF API Compatible C Header
// Auto-generated from Rust FFI - DO NOT EDIT MANUALLY
// Module: band_writer

#ifndef MUPDF_FITZ_BAND_WRITER_H
#define MUPDF_FITZ_BAND_WRITER_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================================
// Band_writer Functions (21 total)
// ============================================================================

size_t fz_band_writer_bytes_written(int32_t _ctx, int32_t writer);
int32_t fz_band_writer_current_band(int32_t _ctx, int32_t writer);
u8 const * fz_band_writer_get_output(int32_t _ctx, int32_t writer, size_t * size);
float fz_band_writer_progress(int32_t _ctx, int32_t writer);
void fz_band_writer_set_components(int32_t _ctx, int32_t writer, int32_t n, int32_t alpha);
void fz_band_writer_set_compression(int32_t _ctx, int32_t writer, int32_t level);
void fz_band_writer_set_dimensions(int32_t _ctx, int32_t writer, int32_t width, int32_t height);
void fz_band_writer_set_jpeg_quality(int32_t _ctx, int32_t writer, int32_t quality);
void fz_band_writer_set_page_info(int32_t _ctx, int32_t writer, int32_t page_num, int32_t total_pages);
void fz_band_writer_set_progress(int32_t _ctx, int32_t writer, ProgressCallback callback, c_void * user_data);
void fz_band_writer_set_res(int32_t _ctx, int32_t writer, int32_t x_res, int32_t y_res);
void fz_band_writer_set_rows_per_band(int32_t _ctx, int32_t writer, int32_t rows);
int32_t fz_band_writer_state(int32_t _ctx, int32_t writer);
int32_t fz_band_writer_total_bands(int32_t _ctx, int32_t writer);
int32_t fz_band_writer_write_band(int32_t _ctx, int32_t writer, int32_t band_rows, u8 const * data);
int32_t fz_band_writer_write_header(int32_t _ctx, int32_t writer);
int32_t fz_band_writer_write_trailer(int32_t _ctx, int32_t writer);
void fz_drop_band_writer(int32_t _ctx, int32_t writer);
int32_t fz_keep_band_writer(int32_t _ctx, int32_t writer);
int32_t fz_new_band_writer(int32_t _ctx, int32_t output, int32_t format);
int32_t fz_new_band_writer_with_config(int32_t _ctx, int32_t output, int32_t format, int32_t width, int32_t height, int32_t n, int32_t alpha);

#ifdef __cplusplus
}
#endif

#endif /* MUPDF_FITZ_BAND_WRITER_H */
