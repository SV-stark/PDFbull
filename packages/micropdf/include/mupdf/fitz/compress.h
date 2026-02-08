// MicroPDF - MuPDF API Compatible C Header
// Auto-generated from Rust FFI - DO NOT EDIT MANUALLY
// Module: compress

#ifndef MUPDF_FITZ_COMPRESS_H
#define MUPDF_FITZ_COMPRESS_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================================
// Compress Functions (25 total)
// ============================================================================

size_t fz_brotli_bound(int32_t _ctx, size_t size);
int32_t fz_brotli_to_buffer(int32_t _ctx, u8 const * source, size_t source_length, int32_t level);
void fz_compress_brotli(int32_t _ctx, u8 * dest, size_t * compressed_length, u8 const * source, size_t source_length, int32_t level);
int32_t fz_compress_ccitt_fax_g3(int32_t _ctx, u8 const * data, int32_t columns, int32_t rows, intptr_t stride);
int32_t fz_compress_ccitt_fax_g4(int32_t _ctx, u8 const * data, int32_t columns, int32_t rows, intptr_t stride);
int32_t fz_compressed_buffer_get_data(int32_t _ctx, int32_t cbuf);
int32_t fz_compressed_buffer_get_type(int32_t _ctx, int32_t cbuf);
void fz_compressed_buffer_set_data(int32_t _ctx, int32_t cbuf, int32_t buffer);
void fz_compressed_buffer_set_type(int32_t _ctx, int32_t cbuf, int32_t image_type);
size_t fz_compressed_buffer_size(int32_t cbuf);
int32_t fz_decompress_brotli(int32_t _ctx, u8 * dest, size_t * dest_length, u8 const * source, size_t source_length);
void fz_deflate(int32_t _ctx, u8 * dest, size_t * compressed_length, u8 const * source, size_t source_length, int32_t level);
size_t fz_deflate_bound(int32_t _ctx, size_t size);
int32_t fz_deflate_to_buffer(int32_t _ctx, u8 const * source, size_t source_length, int32_t level);
void fz_drop_compressed_buffer(int32_t _ctx, int32_t cbuf);
const char * fz_image_type_name(int32_t image_type);
int32_t fz_inflate(int32_t _ctx, u8 * dest, size_t * dest_length, u8 const * source, size_t source_length);
int32_t fz_keep_compressed_buffer(int32_t _ctx, int32_t cbuf);
int32_t fz_lookup_image_type(const char * name);
u8 * fz_new_brotli_data(int32_t _ctx, size_t * compressed_length, u8 const * source, size_t source_length, int32_t level);
u8 * fz_new_brotli_data_from_buffer(int32_t _ctx, size_t * compressed_length, int32_t buffer, int32_t level);
int32_t fz_new_compressed_buffer(int32_t _ctx);
u8 * fz_new_deflated_data(int32_t _ctx, size_t * compressed_length, u8 const * source, size_t source_length, int32_t level);
u8 * fz_new_deflated_data_from_buffer(int32_t _ctx, size_t * compressed_length, int32_t buffer, int32_t level);
int32_t fz_recognize_image_format(int32_t _ctx, u8 const * data);

#ifdef __cplusplus
}
#endif

#endif /* MUPDF_FITZ_COMPRESS_H */
