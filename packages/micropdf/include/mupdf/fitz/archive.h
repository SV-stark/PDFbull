// MicroPDF - MuPDF API Compatible C Header
// Auto-generated from Rust FFI - DO NOT EDIT MANUALLY
// Module: archive

#ifndef MUPDF_FITZ_ARCHIVE_H
#define MUPDF_FITZ_ARCHIVE_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================================
// Archive Functions (13 total)
// ============================================================================

int32_t fz_archive_entry_names(int32_t _ctx, int32_t archive, char * buf, int32_t bufsize);
int32_t fz_archive_entry_size(int32_t _ctx, int32_t archive, const char * name);
int32_t fz_archive_format(int32_t _ctx, int32_t archive);
int32_t fz_archive_is_valid(int32_t _ctx, int32_t archive);
int32_t fz_clone_archive(int32_t _ctx, int32_t archive);
int32_t fz_count_archive_entries(int32_t _ctx, int32_t archive);
void fz_drop_archive(int32_t _ctx, int32_t archive);
int32_t fz_has_archive_entry(int32_t _ctx, int32_t archive, const char * name);
int32_t fz_keep_archive(int32_t _ctx, int32_t archive);
int32_t fz_list_archive_entry(int32_t _ctx, int32_t archive, int32_t idx, char * buf, int32_t bufsize);
int32_t fz_open_archive(int32_t _ctx, const char * path);
int32_t fz_open_archive_with_buffer(int32_t _ctx, u8 const * data, size_t size);
int32_t fz_read_archive_entry(int32_t _ctx, int32_t archive, const char * name);

#ifdef __cplusplus
}
#endif

#endif /* MUPDF_FITZ_ARCHIVE_H */
