// MicroPDF - MuPDF API Compatible C Header
// Auto-generated from Rust FFI - DO NOT EDIT MANUALLY
// Module: buffered_io

#ifndef MUPDF_FITZ_BUFFERED_IO_H
#define MUPDF_FITZ_BUFFERED_IO_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================================
// Buffered_io Functions (17 total)
// ============================================================================

int fz_buffered_flush(int32_t _ctx, int32_t writer);
int fz_buffered_sync(int32_t _ctx, int32_t writer);
int fz_buffered_write(int32_t _ctx, int32_t writer, u8 const * data, size_t len);
int fz_buffered_write_string(int32_t _ctx, int32_t writer, const char * s);
size_t fz_buffered_writer_buffered(int32_t _ctx, int32_t writer);
FfiWriterStats fz_buffered_writer_stats(int32_t _ctx, int32_t writer);
void fz_drop_buffered_writer(int32_t _ctx, int32_t writer);
void fz_drop_vectored_writer(int32_t _ctx, int32_t writer);
int32_t fz_new_buffered_writer(int32_t _ctx, const char * path);
int32_t fz_new_buffered_writer_with_capacity(int32_t _ctx, const char * path, size_t capacity);
int32_t fz_new_vectored_writer(int32_t _ctx, const char * path);
int fz_vectored_flush(int32_t _ctx, int32_t writer);
int fz_vectored_queue(int32_t _ctx, int32_t writer, u8 const * data, size_t len);
int fz_vectored_sync(int32_t _ctx, int32_t writer);
size_t fz_vectored_writer_pending_bytes(int32_t _ctx, int32_t writer);
size_t fz_vectored_writer_pending_count(int32_t _ctx, int32_t writer);
FfiWriterStats fz_vectored_writer_stats(int32_t _ctx, int32_t writer);

#ifdef __cplusplus
}
#endif

#endif /* MUPDF_FITZ_BUFFERED_IO_H */
