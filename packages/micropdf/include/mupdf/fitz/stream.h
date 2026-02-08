// MicroPDF - MuPDF API Compatible C Header
// Auto-generated from Rust FFI - DO NOT EDIT MANUALLY
// Module: stream

#ifndef MUPDF_FITZ_STREAM_H
#define MUPDF_FITZ_STREAM_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================================
// Stream Functions (29 total)
// ============================================================================

void fz_drop_stream(int32_t _ctx, int32_t stm);
int32_t fz_is_eof(int32_t _ctx, int32_t stm);
int32_t fz_keep_stream(int32_t _ctx, int32_t stm);
int32_t fz_open_buffer(int32_t _ctx, int32_t buf);
int32_t fz_open_file(int32_t _ctx, const char * filename);
int32_t fz_open_memory(int32_t _ctx, u8 const * data, size_t len);
int32_t fz_peek_byte(int32_t _ctx, int32_t stm);
size_t fz_read(int32_t _ctx, int32_t stm, u8 * data, size_t len);
int32_t fz_read_all(int32_t _ctx, int32_t stm);
int32_t fz_read_byte(int32_t _ctx, int32_t stm);
float fz_read_float(int32_t _ctx, int32_t stm);
float fz_read_float_le(int32_t _ctx, int32_t stm);
i16 fz_read_int16(int32_t _ctx, int32_t stm);
i16 fz_read_int16_le(int32_t _ctx, int32_t stm);
int32_t fz_read_int32(int32_t _ctx, int32_t stm);
int32_t fz_read_int32_le(int32_t _ctx, int32_t stm);
int64_t fz_read_int64(int32_t _ctx, int32_t stm);
int64_t fz_read_int64_le(int32_t _ctx, int32_t stm);
char * fz_read_line(int32_t _ctx, int32_t stm, char * buf, size_t max);
u16 fz_read_uint16(int32_t _ctx, int32_t stm);
u16 fz_read_uint16_le(int32_t _ctx, int32_t stm);
uint32_t fz_read_uint32(int32_t _ctx, int32_t stm);
uint32_t fz_read_uint32_le(int32_t _ctx, int32_t stm);
uint64_t fz_read_uint64(int32_t _ctx, int32_t stm);
uint64_t fz_read_uint64_le(int32_t _ctx, int32_t stm);
void fz_seek(int32_t _ctx, int32_t stm, int64_t offset, int32_t whence);
void fz_skip_space(int32_t _ctx, int32_t stm);
int64_t fz_tell(int32_t _ctx, int32_t stm);
void fz_unread_byte(int32_t _ctx, int32_t stm);

#ifdef __cplusplus
}
#endif

#endif /* MUPDF_FITZ_STREAM_H */
