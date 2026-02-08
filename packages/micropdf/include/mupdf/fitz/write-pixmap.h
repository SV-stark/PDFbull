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
// Write Pixmap Functions (30 total)
// ============================================================================

// PNG Functions
int32_t fz_save_pixmap_as_png(uint64_t ctx, uint64_t pixmap, const char* filename);
int32_t fz_write_pixmap_as_png(uint64_t ctx, uint64_t out, uint64_t pixmap);
uint64_t fz_new_buffer_from_pixmap_as_png(uint64_t ctx, uint64_t pixmap);

// JPEG Functions
int32_t fz_save_pixmap_as_jpeg(uint64_t ctx, uint64_t pixmap, const char* filename, int32_t quality);
int32_t fz_write_pixmap_as_jpeg(uint64_t ctx, uint64_t out, uint64_t pixmap, int32_t quality, int32_t invert_cmyk);
uint64_t fz_new_buffer_from_pixmap_as_jpeg(uint64_t ctx, uint64_t pixmap, int32_t quality, int32_t invert_cmyk);

// PNM Functions (Portable Any Map - PPM/PGM)
int32_t fz_save_pixmap_as_pnm(uint64_t ctx, uint64_t pixmap, const char* filename);
int32_t fz_write_pixmap_as_pnm(uint64_t ctx, uint64_t out, uint64_t pixmap);
uint64_t fz_new_buffer_from_pixmap_as_pnm(uint64_t ctx, uint64_t pixmap);

// PAM Functions (Portable Arbitrary Map)
int32_t fz_save_pixmap_as_pam(uint64_t ctx, uint64_t pixmap, const char* filename);
int32_t fz_write_pixmap_as_pam(uint64_t ctx, uint64_t out, uint64_t pixmap);
uint64_t fz_new_buffer_from_pixmap_as_pam(uint64_t ctx, uint64_t pixmap);

// PBM Functions (Portable Bitmap - 1-bit with halftoning)
int32_t fz_save_pixmap_as_pbm(uint64_t ctx, uint64_t pixmap, const char* filename);
int32_t fz_write_pixmap_as_pbm(uint64_t ctx, uint64_t out, uint64_t pixmap);
uint64_t fz_new_buffer_from_pixmap_as_pbm(uint64_t ctx, uint64_t pixmap);

// PKM Functions (CMYK Portable)
int32_t fz_save_pixmap_as_pkm(uint64_t ctx, uint64_t pixmap, const char* filename);
int32_t fz_write_pixmap_as_pkm(uint64_t ctx, uint64_t out, uint64_t pixmap);
uint64_t fz_new_buffer_from_pixmap_as_pkm(uint64_t ctx, uint64_t pixmap);

// PSD Functions (Photoshop)
int32_t fz_save_pixmap_as_psd(uint64_t ctx, uint64_t pixmap, const char* filename);
int32_t fz_write_pixmap_as_psd(uint64_t ctx, uint64_t out, uint64_t pixmap);
uint64_t fz_new_buffer_from_pixmap_as_psd(uint64_t ctx, uint64_t pixmap);

// PostScript Functions
int32_t fz_save_pixmap_as_ps(uint64_t ctx, uint64_t pixmap, const char* filename, int32_t append);
int32_t fz_write_pixmap_as_ps(uint64_t ctx, uint64_t out, uint64_t pixmap);
int32_t fz_write_ps_file_header(uint64_t ctx, uint64_t out);
int32_t fz_write_ps_file_trailer(uint64_t ctx, uint64_t out, int32_t pages);

// Data URI Functions
int32_t fz_write_pixmap_as_data_uri(uint64_t ctx, uint64_t out, uint64_t pixmap);

#ifdef __cplusplus
}
#endif

#endif /* MUPDF_FITZ_WRITE_PIXMAP_H */

