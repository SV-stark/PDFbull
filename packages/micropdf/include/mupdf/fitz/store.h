// MicroPDF - MuPDF API Compatible C Header
// Auto-generated from Rust FFI - DO NOT EDIT MANUALLY
// Module: store

#ifndef MUPDF_FITZ_STORE_H
#define MUPDF_FITZ_STORE_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================================
// Store Functions (32 total)
// ============================================================================

int32_t fz_new_store(int32_t _ctx, size_t max_size);
void fz_store_clear(int32_t _ctx);
size_t fz_store_count(int32_t _ctx);
size_t fz_store_current_size(int32_t _ctx);
void fz_store_debug(int32_t _ctx);
void fz_store_drop(int32_t _ctx, uint64_t id);
size_t fz_store_evict(int32_t _ctx, size_t target_size);
size_t fz_store_evict_old(int32_t _ctx, uint64_t max_age_ms);
size_t fz_store_evict_type(int32_t _ctx, int32_t item_type);
int32_t fz_store_find(int32_t _ctx, u8 const * key, size_t key_len);
int32_t fz_store_find_by_id(int32_t _ctx, uint64_t id);
float fz_store_hit_rate(int32_t _ctx);
uint64_t fz_store_hits(int32_t _ctx);
uint64_t fz_store_item(int32_t _ctx, int32_t item_type, int32_t handle, size_t size, u8 const * key, size_t key_len);
uint64_t fz_store_item_access_count(int32_t _ctx, uint64_t id);
uint64_t fz_store_item_age(int32_t _ctx, uint64_t id);
size_t fz_store_item_size(int32_t _ctx, uint64_t id);
int32_t fz_store_item_type(int32_t _ctx, uint64_t id);
uint64_t fz_store_keep(int32_t _ctx, uint64_t id);
size_t fz_store_max_size(int32_t _ctx);
uint64_t fz_store_misses(int32_t _ctx);
int32_t fz_store_remove(int32_t _ctx, uint64_t id);
int32_t fz_store_remove_by_key(int32_t _ctx, u8 const * key, size_t key_len);
void fz_store_reset_stats(int32_t _ctx);
void fz_store_set_evictable(int32_t _ctx, uint64_t id, int32_t evictable);
void fz_store_set_max_size(int32_t _ctx, size_t max_size);
void fz_store_set_policy(int32_t _ctx, int32_t policy);
void fz_store_set_type_limit(int32_t _ctx, int32_t item_type, size_t max_size);
uint64_t fz_store_total_evicted(int32_t _ctx);
uint64_t fz_store_total_stored(int32_t _ctx);
size_t fz_store_type_count(int32_t _ctx, int32_t item_type);
size_t fz_store_type_size(int32_t _ctx, int32_t item_type);

#ifdef __cplusplus
}
#endif

#endif /* MUPDF_FITZ_STORE_H */
