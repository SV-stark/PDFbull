// MicroPDF - MuPDF API Compatible C Header
// Auto-generated from Rust FFI - DO NOT EDIT MANUALLY
// Module: data_locality

#ifndef MUPDF_FITZ_DATA_LOCALITY_H
#define MUPDF_FITZ_DATA_LOCALITY_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================================
// Data_locality Functions (19 total)
// ============================================================================

void fz_drop_page_aligned_buffer(int32_t _ctx, int32_t buf);
void fz_drop_point_soa(int32_t _ctx, int32_t soa);
void fz_drop_rect_soa(int32_t _ctx, int32_t soa);
LocalityStatsSnapshot fz_locality_stats(int32_t _ctx);
void fz_locality_stats_reset(int32_t _ctx);
int32_t fz_new_page_aligned_buffer(int32_t _ctx, size_t capacity);
int32_t fz_new_point_soa(int32_t _ctx, size_t capacity);
int32_t fz_new_rect_soa(int32_t _ctx, size_t capacity);
size_t fz_page_buffer_capacity(int32_t _ctx, int32_t buf);
size_t fz_page_buffer_len(int32_t _ctx, int32_t buf);
void fz_page_buffer_prefetch_read(int32_t _ctx, int32_t buf, int locality);
void fz_page_buffer_prefetch_write(int32_t _ctx, int32_t buf, int locality);
int fz_page_buffer_read(int32_t _ctx, int32_t buf, size_t offset, u8 * dst, size_t len);
int fz_page_buffer_write(int32_t _ctx, int32_t buf, u8 const * data, size_t len);
size_t fz_point_soa_len(int32_t _ctx, int32_t soa);
void fz_point_soa_push(int32_t _ctx, int32_t soa, float x, float y);
void fz_point_soa_transform(int32_t _ctx, int32_t soa, float a, float b, float c, float d, float e, float f);
size_t fz_rect_soa_len(int32_t _ctx, int32_t soa);
void fz_rect_soa_push(int32_t _ctx, int32_t soa, float x0, float y0, float x1, float y1);

#ifdef __cplusplus
}
#endif

#endif /* MUPDF_FITZ_DATA_LOCALITY_H */
