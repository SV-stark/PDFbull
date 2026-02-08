// MicroPDF - MuPDF API Compatible C Header
// Auto-generated from Rust FFI - DO NOT EDIT MANUALLY
// Module: lockfree

#ifndef MUPDF_FITZ_LOCKFREE_H
#define MUPDF_FITZ_LOCKFREE_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================================
// Lockfree Functions (5 total)
// ============================================================================

size_t fz_lockfree_queue_capacity(void);
int fz_lockfree_queue_is_empty(void);
size_t fz_lockfree_queue_len(void);
uint64_t fz_lockfree_queue_pop(void);
int fz_lockfree_queue_push(uint64_t task_id);

#ifdef __cplusplus
}
#endif

#endif /* MUPDF_FITZ_LOCKFREE_H */
