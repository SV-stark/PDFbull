// MicroPDF - MuPDF API Compatible C Header
// Auto-generated from Rust FFI - DO NOT EDIT MANUALLY
// Module: write_pixmap

#ifndef MUPDF_FITZ_WRITE_PIXMAP_H
#define MUPDF_FITZ_WRITE_PIXMAP_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================================
// Write_pixmap Functions (26 total)
// ============================================================================

int32_t fz_new_buffer_from_pixmap_as_jpeg(int32_t _ctx, int32_t pixmap, int32_t quality, int32_t _invert_cmyk);
int32_t fz_new_buffer_from_pixmap_as_pam(int32_t _ctx, int32_t pixmap);
int32_t fz_new_buffer_from_pixmap_as_pbm(int32_t _ctx, int32_t pixmap);
int32_t fz_new_buffer_from_pixmap_as_pkm(int32_t _ctx, int32_t pixmap);
int32_t fz_new_buffer_from_pixmap_as_png(int32_t _ctx, int32_t pixmap);
int32_t fz_new_buffer_from_pixmap_as_pnm(int32_t _ctx, int32_t pixmap);
int32_t fz_new_buffer_from_pixmap_as_psd(int32_t _ctx, int32_t pixmap);
int32_t fz_save_pixmap_as_jpeg(int32_t _ctx, int32_t pixmap, const char * filename, int32_t quality);
int32_t fz_save_pixmap_as_pam(int32_t _ctx, int32_t pixmap, const char * filename);
int32_t fz_save_pixmap_as_pbm(int32_t _ctx, int32_t pixmap, const char * filename);
int32_t fz_save_pixmap_as_pkm(int32_t _ctx, int32_t pixmap, const char * filename);
int32_t fz_save_pixmap_as_png(int32_t _ctx, int32_t pixmap, const char * filename);
int32_t fz_save_pixmap_as_pnm(int32_t _ctx, int32_t pixmap, const char * filename);
int32_t fz_save_pixmap_as_ps(int32_t _ctx, int32_t pixmap, const char * filename, int32_t _append);
int32_t fz_save_pixmap_as_psd(int32_t _ctx, int32_t pixmap, const char * filename);
int32_t fz_write_pixmap_as_data_uri(int32_t _ctx, int32_t out, int32_t pixmap);
int32_t fz_write_pixmap_as_jpeg(int32_t _ctx, int32_t out, int32_t pixmap, int32_t quality, int32_t _invert_cmyk);
int32_t fz_write_pixmap_as_pam(int32_t _ctx, int32_t out, int32_t pixmap);
int32_t fz_write_pixmap_as_pbm(int32_t _ctx, int32_t out, int32_t pixmap);
int32_t fz_write_pixmap_as_pkm(int32_t _ctx, int32_t out, int32_t pixmap);
int32_t fz_write_pixmap_as_png(int32_t _ctx, int32_t out, int32_t pixmap);
int32_t fz_write_pixmap_as_pnm(int32_t _ctx, int32_t out, int32_t pixmap);
int32_t fz_write_pixmap_as_ps(int32_t _ctx, int32_t out, int32_t pixmap);
int32_t fz_write_pixmap_as_psd(int32_t _ctx, int32_t out, int32_t pixmap);
int32_t fz_write_ps_file_header(int32_t _ctx, int32_t out);
int32_t fz_write_ps_file_trailer(int32_t _ctx, int32_t out, int32_t pages);

#ifdef __cplusplus
}
#endif

#endif /* MUPDF_FITZ_WRITE_PIXMAP_H */
