// MicroPDF - MuPDF API Compatible C Header
// Auto-generated from Rust FFI - DO NOT EDIT MANUALLY
// Module: filter

#ifndef MUPDF_FITZ_FILTER_H
#define MUPDF_FITZ_FILTER_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================================
// Filter Functions (30 total)
// ============================================================================

void fz_concat_push_drop(int32_t _ctx, int32_t concat, int32_t chain);
void fz_drop_filter(int32_t _ctx, int32_t filter);
void fz_drop_jbig2_globals(int32_t _ctx, int32_t globals);
u8 const * fz_filter_data(int32_t _ctx, int32_t filter);
size_t fz_filter_read(int32_t _ctx, int32_t filter, u8 * buf, size_t len);
size_t fz_filter_size(int32_t _ctx, int32_t filter);
int32_t fz_jbig2_globals_data(int32_t _ctx, int32_t globals);
int32_t fz_keep_jbig2_globals(int32_t _ctx, int32_t globals);
int32_t fz_load_jbig2_globals(int32_t _ctx, int32_t buf);
int32_t fz_open_a85d(int32_t _ctx, int32_t chain);
int32_t fz_open_aesd(int32_t _ctx, int32_t chain, u8 const * key, uint32_t keylen);
int32_t fz_open_ahxd(int32_t _ctx, int32_t chain);
int32_t fz_open_arc4(int32_t _ctx, int32_t chain, u8 const * key, uint32_t keylen);
int32_t fz_open_brotlid(int32_t _ctx, int32_t chain);
int32_t fz_open_concat(int32_t _ctx, int32_t max, int32_t pad);
int32_t fz_open_dctd(int32_t _ctx, int32_t chain, int32_t color_transform, int32_t invert_cmyk, int32_t l2factor, int32_t _jpegtables);
int32_t fz_open_endstream_filter(int32_t _ctx, int32_t chain, uint64_t len, int64_t offset);
int32_t fz_open_faxd(int32_t _ctx, int32_t chain, int32_t k, int32_t end_of_line, int32_t encoded_byte_align, int32_t columns, int32_t rows, int32_t end_of_block, int32_t black_is_1);
int32_t fz_open_flated(int32_t _ctx, int32_t chain, int32_t window_bits);
int32_t fz_open_jbig2d(int32_t _ctx, int32_t chain, int32_t globals, int32_t embedded);
int32_t fz_open_libarchived(int32_t _ctx, int32_t chain);
int32_t fz_open_lzwd(int32_t _ctx, int32_t chain, int32_t early_change, int32_t min_bits, int32_t reverse_bits, int32_t old_tiff);
int32_t fz_open_null_filter(int32_t _ctx, int32_t chain, uint64_t len, int64_t offset);
int32_t fz_open_predict(int32_t _ctx, int32_t chain, int32_t predictor, int32_t columns, int32_t colors, int32_t bpc);
int32_t fz_open_range_filter(int32_t _ctx, int32_t chain, FzRange const * ranges, int32_t nranges);
int32_t fz_open_rld(int32_t _ctx, int32_t chain);
int32_t fz_open_sgilog16(int32_t _ctx, int32_t chain, int32_t w);
int32_t fz_open_sgilog24(int32_t _ctx, int32_t chain, int32_t w);
int32_t fz_open_sgilog32(int32_t _ctx, int32_t chain, int32_t w);
int32_t fz_open_thunder(int32_t _ctx, int32_t chain, int32_t w);

#ifdef __cplusplus
}
#endif

#endif /* MUPDF_FITZ_FILTER_H */
