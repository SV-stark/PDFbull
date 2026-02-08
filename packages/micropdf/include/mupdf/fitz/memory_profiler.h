// MicroPDF - MuPDF API Compatible C Header
// Auto-generated from Rust FFI - DO NOT EDIT MANUALLY
// Module: memory_profiler

#ifndef MUPDF_FITZ_MEMORY_PROFILER_H
#define MUPDF_FITZ_MEMORY_PROFILER_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================================
// Memory_profiler Functions (10 total)
// ============================================================================

void fz_enable_memory_profiling(int32_t enabled);
void fz_enable_stack_traces(int32_t enabled);
uint64_t fz_profiler_current_bytes(void);
uint64_t fz_profiler_handle_count_by_type(ResourceType resource_type);
uint64_t fz_profiler_live_handle_count(void);
uint64_t fz_profiler_peak_bytes(void);
uint64_t fz_profiler_peak_handles(void);
uint64_t fz_profiler_potential_leak_count(uint64_t min_age_seconds);
void fz_profiler_print_leak_report(uint64_t min_age_seconds);
void fz_profiler_reset(void);

#ifdef __cplusplus
}
#endif

#endif /* MUPDF_FITZ_MEMORY_PROFILER_H */
