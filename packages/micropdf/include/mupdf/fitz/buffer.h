// MicroPDF - MuPDF API Compatible C Header
// Auto-generated from Rust FFI - DO NOT EDIT MANUALLY
// Module: buffer

#ifndef MUPDF_FITZ_BUFFER_H
#define MUPDF_FITZ_BUFFER_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================================
// Buffer Functions (44 total)
// ============================================================================

void fz_append_base64(int32_t _ctx, int32_t buf, u8 const * data, size_t size, int32_t newline);
void fz_append_bits(int32_t _ctx, int32_t buf, int32_t value, int32_t count);
void fz_append_bits_pad(int32_t _ctx, int32_t buf);
void fz_append_buffer(int32_t _ctx, int32_t buf, int32_t src);
void fz_append_byte(int32_t _ctx, int32_t buf, int c);
void fz_append_data(int32_t _ctx, int32_t buf, void const * data, size_t len);
void fz_append_float(int32_t _ctx, int32_t buf, float value, int32_t digits);
void fz_append_hex(int32_t _ctx, int32_t buf, u8 const * data, size_t size);
void fz_append_int(int32_t _ctx, int32_t buf, int64_t value);
void fz_append_int16_be(int32_t _ctx, int32_t buf, i16 x);
void fz_append_int16_le(int32_t _ctx, int32_t buf, i16 x);
void fz_append_int32_be(int32_t _ctx, int32_t buf, int32_t x);
void fz_append_int32_le(int32_t _ctx, int32_t buf, int32_t x);
void fz_append_pdf_string(int32_t _ctx, int32_t buf, const char * str);
void fz_append_rune(int32_t _ctx, int32_t buf, int32_t rune);
void fz_append_string(int32_t _ctx, int32_t buf, const char * data);
size_t fz_buffer_capacity(int32_t _ctx, int32_t buf);
u8 const * fz_buffer_data(int32_t _ctx, int32_t buf, size_t * len);
int32_t fz_buffer_eq(int32_t _ctx, int32_t buf1, int32_t buf2);
int32_t fz_buffer_extract(int32_t _ctx, int32_t buf);
int fz_buffer_is_pooled(int32_t _ctx, int32_t buf);
size_t fz_buffer_len(int32_t _ctx, int32_t buf);
void fz_buffer_pool_clear(int32_t _ctx);
size_t fz_buffer_pool_count(int32_t _ctx);
PoolStatsFFI fz_buffer_pool_stats(int32_t _ctx);
void fz_buffer_reserve(int32_t _ctx, int32_t buf, size_t additional);
void fz_buffer_shrink_to_fit(int32_t _ctx, int32_t buf);
size_t fz_buffer_storage(int32_t _ctx, int32_t buf, u8 * * datap);
void fz_clear_buffer(int32_t _ctx, int32_t buf);
int32_t fz_clone_buffer(int32_t _ctx, int32_t buf);
void fz_drop_buffer(int32_t _ctx, int32_t buf);
void fz_grow_buffer(int32_t _ctx, int32_t buf);
int32_t fz_keep_buffer(int32_t _ctx, int32_t buf);
void fz_md5_buffer(int32_t _ctx, int32_t buf, [u8; 16] * digest);
int32_t fz_new_buffer(int32_t _ctx, size_t capacity);
int32_t fz_new_buffer_from_copied_data(int32_t _ctx, u8 const * data, size_t size);
int32_t fz_new_buffer_from_data(int32_t _ctx, u8 * data, size_t size);
int32_t fz_new_buffer_unpooled(int32_t _ctx, size_t capacity);
int32_t fz_new_buffer_with_capacity(int32_t _ctx, size_t hint);
void fz_resize_buffer(int32_t _ctx, int32_t buf, size_t capacity);
int32_t fz_slice_buffer(int32_t _ctx, int32_t buf, size_t offset, size_t len);
const char * fz_string_from_buffer(int32_t _ctx, int32_t _buf);
void fz_terminate_buffer(int32_t _ctx, int32_t buf);
void fz_trim_buffer(int32_t _ctx, int32_t buf);

#ifdef __cplusplus
}
#endif

#endif /* MUPDF_FITZ_BUFFER_H */
