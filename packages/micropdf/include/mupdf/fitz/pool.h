// MicroPDF - MuPDF API Compatible C Header
// Auto-generated from Rust FFI - DO NOT EDIT MANUALLY
// Module: pool

#ifndef MUPDF_FITZ_POOL_H
#define MUPDF_FITZ_POOL_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================================
// Pool Functions (19 total)
// ============================================================================

void fz_drop_pool(int32_t _ctx, int32_t pool);
int32_t fz_keep_pool(int32_t _ctx, int32_t pool);
int32_t fz_new_pool(int32_t _ctx);
int32_t fz_new_pool_named(int32_t _ctx, const char * name);
int32_t fz_new_pool_with_size(int32_t _ctx, size_t block_size);
u8 * fz_pool_alloc(int32_t _ctx, int32_t pool, size_t size);
u8 * fz_pool_alloc_aligned(int32_t _ctx, int32_t pool, size_t size, size_t align);
size_t fz_pool_alloc_count(int32_t _ctx, int32_t pool);
size_t fz_pool_allocated(int32_t _ctx, int32_t pool);
size_t fz_pool_available(int32_t _ctx, int32_t pool);
size_t fz_pool_block_count(int32_t _ctx, int32_t pool);
u8 * fz_pool_calloc(int32_t _ctx, int32_t pool, size_t count, size_t size);
float fz_pool_fragmentation(int32_t _ctx, int32_t pool);
size_t fz_pool_high_water(int32_t _ctx, int32_t pool);
void fz_pool_reset(int32_t _ctx, int32_t pool);
void fz_pool_set_block_size(int32_t _ctx, int32_t pool, size_t size);
void fz_pool_shrink(int32_t _ctx, int32_t pool);
c_char * fz_pool_strdup(int32_t _ctx, int32_t pool, const char * s);
size_t fz_pool_used(int32_t _ctx, int32_t pool);

#ifdef __cplusplus
}
#endif

#endif /* MUPDF_FITZ_POOL_H */
