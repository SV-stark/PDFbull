// MicroPDF - MuPDF API Compatible C Header
// Auto-generated from Rust FFI - DO NOT EDIT MANUALLY
// Module: mmap

#ifndef MUPDF_FITZ_MMAP_H
#define MUPDF_FITZ_MMAP_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================================
// Mmap Functions (16 total)
// ============================================================================

void fz_close_mapped_file(int32_t _ctx, int32_t file);
void fz_drop_mapped_buffer(int32_t _ctx, int32_t buf);
size_t fz_mapped_buffer_position(int32_t _ctx, int32_t buf);
int fz_mapped_buffer_read(int32_t _ctx, int32_t buf, u8 * dst, size_t len);
int fz_mapped_buffer_read_byte(int32_t _ctx, int32_t buf);
size_t fz_mapped_buffer_remaining(int32_t _ctx, int32_t buf);
void fz_mapped_buffer_seek(int32_t _ctx, int32_t buf, size_t pos);
int fz_mapped_file_advise(int32_t _ctx, int32_t file, int advice);
int fz_mapped_file_advise(int32_t _ctx, int32_t _file, int _advice);
int64_t fz_mapped_file_find(int32_t _ctx, int32_t file, u8 const * needle, size_t needle_len);
int fz_mapped_file_read(int32_t _ctx, int32_t file, size_t offset, u8 * dst, size_t len);
int64_t fz_mapped_file_rfind(int32_t _ctx, int32_t file, u8 const * needle, size_t needle_len);
size_t fz_mapped_file_size(int32_t _ctx, int32_t file);
FfiMappedFileStats fz_mapped_file_stats(int32_t _ctx, int32_t file);
int32_t fz_new_mapped_buffer(int32_t _ctx, const char * path);
int32_t fz_open_mapped_file(int32_t _ctx, const char * path);

#ifdef __cplusplus
}
#endif

#endif /* MUPDF_FITZ_MMAP_H */
