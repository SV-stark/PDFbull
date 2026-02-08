// MicroPDF - MuPDF API Compatible C Header
// Auto-generated from Rust FFI - DO NOT EDIT MANUALLY
// Module: gpu

#ifndef MUPDF_FITZ_GPU_H
#define MUPDF_FITZ_GPU_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================================
// Gpu Functions (19 total)
// ============================================================================

int32_t fz_gpu_backend_available(int32_t backend);
int32_t fz_gpu_clear_texture(int32_t device, int32_t texture, float r, float g, float b, float a);
int32_t fz_gpu_composite(int32_t device, int32_t src, int32_t dst, int32_t x, int32_t y, int32_t blend_mode);
int32_t fz_gpu_create_buffer(int32_t device, int32_t size, int32_t usage);
int32_t fz_gpu_create_device(int32_t backend);
int32_t fz_gpu_create_shader(int32_t device, const char * vertex_src, const char * fragment_src);
int32_t fz_gpu_create_texture(int32_t device, int32_t width, int32_t height, int32_t format);
int32_t fz_gpu_device_backend(int32_t device);
void fz_gpu_drop_buffer(int32_t buffer);
void fz_gpu_drop_device(int32_t device);
void fz_gpu_drop_shader(int32_t shader);
void fz_gpu_drop_texture(int32_t texture);
int32_t fz_gpu_finish(int32_t device);
int32_t fz_gpu_flush(int32_t device);
int32_t fz_gpu_render_page(int32_t device, int32_t page, int32_t texture, float const * ctm);
int32_t fz_gpu_texture_download(int32_t device, int32_t texture, u8 * data, int32_t stride);
int32_t fz_gpu_texture_height(int32_t texture);
int32_t fz_gpu_texture_upload(int32_t device, int32_t texture, u8 const * data, int32_t stride);
int32_t fz_gpu_texture_width(int32_t texture);

#ifdef __cplusplus
}
#endif

#endif /* MUPDF_FITZ_GPU_H */
