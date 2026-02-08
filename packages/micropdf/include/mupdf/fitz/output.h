// MicroPDF - MuPDF API Compatible C Header
// Auto-generated from Rust FFI - DO NOT EDIT MANUALLY
// Module: output

#ifndef MUPDF_FITZ_OUTPUT_H
#define MUPDF_FITZ_OUTPUT_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================================
// Output Functions (34 total)
// ============================================================================

void fz_close_output(int32_t _ctx, int32_t out);
void fz_drop_output(int32_t _ctx, int32_t out);
void fz_flush_output(int32_t _ctx, int32_t out);
int32_t fz_keep_output(int32_t _ctx, int32_t out);
int32_t fz_new_output_with_buffer(int32_t _ctx, int32_t buf);
int32_t fz_new_output_with_path(int32_t _ctx, const char * filename, int32_t append);
void fz_reset_output(int32_t _ctx, int32_t out);
void fz_seek_output(int32_t _ctx, int32_t out, int64_t off, int32_t whence);
int64_t fz_tell_output(int32_t _ctx, int32_t out);
void fz_truncate_output(int32_t _ctx, int32_t out);
void fz_write_base64(int32_t _ctx, int32_t out, u8 const * data, size_t size, int32_t newline);
void fz_write_base64_uri(int32_t _ctx, int32_t out, u8 const * data, size_t size);
void fz_write_bits(int32_t _ctx, int32_t out, uint32_t value, int32_t count);
void fz_write_bits_sync(int32_t _ctx, int32_t out);
void fz_write_buffer(int32_t _ctx, int32_t out, int32_t buf);
void fz_write_byte(int32_t _ctx, int32_t out, u8 byte);
void fz_write_char(int32_t _ctx, int32_t out, char c);
void fz_write_data(int32_t _ctx, int32_t out, void const * data, size_t size);
void fz_write_float_be(int32_t _ctx, int32_t out, float x);
void fz_write_float_le(int32_t _ctx, int32_t out, float x);
void fz_write_int16_be(int32_t _ctx, int32_t out, i16 x);
void fz_write_int16_le(int32_t _ctx, int32_t out, i16 x);
void fz_write_int32_be(int32_t _ctx, int32_t out, int32_t x);
void fz_write_int32_le(int32_t _ctx, int32_t out, int32_t x);
void fz_write_int64_be(int32_t _ctx, int32_t out, int64_t x);
void fz_write_int64_le(int32_t _ctx, int32_t out, int64_t x);
void fz_write_rune(int32_t _ctx, int32_t out, int32_t rune);
void fz_write_string(int32_t _ctx, int32_t out, const char * s);
void fz_write_uint16_be(int32_t _ctx, int32_t out, u16 x);
void fz_write_uint16_le(int32_t _ctx, int32_t out, u16 x);
void fz_write_uint32_be(int32_t _ctx, int32_t out, uint32_t x);
void fz_write_uint32_le(int32_t _ctx, int32_t out, uint32_t x);
void fz_write_uint64_be(int32_t _ctx, int32_t out, uint64_t x);
void fz_write_uint64_le(int32_t _ctx, int32_t out, uint64_t x);

#ifdef __cplusplus
}
#endif

#endif /* MUPDF_FITZ_OUTPUT_H */
