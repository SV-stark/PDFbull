// MicroPDF - MuPDF API Compatible C Header
// Auto-generated from Rust FFI - DO NOT EDIT MANUALLY
// Module: heap

#ifndef MUPDF_FITZ_HEAP_H
#define MUPDF_FITZ_HEAP_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================================
// Heap Functions (23 total)
// ============================================================================

void fz_drop_heap(int32_t _ctx, int32_t heap);
size_t fz_heap_capacity(int32_t _ctx, int32_t heap);
void fz_heap_clear(int32_t _ctx, int32_t heap);
int32_t fz_heap_is_empty(int32_t _ctx, int32_t heap);
size_t fz_heap_len(int32_t _ctx, int32_t heap);
void fz_heap_reserve(int32_t _ctx, int32_t heap, size_t additional);
int32_t fz_heap_type(int32_t _ctx, int32_t heap);
int32_t fz_int2_heap_insert(int32_t _ctx, int32_t heap, int32_t a, int32_t b);
int32_t fz_int2_heap_pop(int32_t _ctx, int32_t heap, int32_t * a_out, int32_t * b_out);
void fz_int2_heap_sort(int32_t _ctx, int32_t heap);
size_t fz_int2_heap_uniq(int32_t _ctx, int32_t heap);
int32_t fz_int_heap_insert(int32_t _ctx, int32_t heap, int32_t value);
int32_t fz_int_heap_peek(int32_t _ctx, int32_t heap, int32_t * value_out);
int32_t fz_int_heap_pop(int32_t _ctx, int32_t heap, int32_t * value_out);
size_t fz_int_heap_sort(int32_t _ctx, int32_t heap, int32_t * out, size_t out_len);
int32_t fz_intptr_heap_insert(int32_t _ctx, int32_t heap, int32_t a, size_t b);
int32_t fz_intptr_heap_pop(int32_t _ctx, int32_t heap, int32_t * a_out, size_t * b_out);
void fz_intptr_heap_sort(int32_t _ctx, int32_t heap);
size_t fz_intptr_heap_uniq(int32_t _ctx, int32_t heap);
int32_t fz_new_heap(int32_t _ctx, int32_t heap_type);
int32_t fz_new_heap_with_capacity(int32_t _ctx, int32_t heap_type, size_t capacity);
int32_t fz_ptr_heap_insert(int32_t _ctx, int32_t heap, size_t value);
int32_t fz_ptr_heap_pop(int32_t _ctx, int32_t heap, size_t * value_out);

#ifdef __cplusplus
}
#endif

#endif /* MUPDF_FITZ_HEAP_H */
