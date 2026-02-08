// MicroPDF - MuPDF API Compatible C Header
// Auto-generated from Rust FFI - DO NOT EDIT MANUALLY
// Module: pdf_3d

#ifndef MUPDF_PDF_PDF_3D_H
#define MUPDF_PDF_PDF_3D_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================================
// Pdf_3d Functions (27 total)
// ============================================================================

int32_t pdf_3d_add_view(int32_t _ctx, int32_t annot, const char * name);
char * pdf_3d_format_to_string(int32_t _ctx, int32_t format);
void pdf_3d_free_string(char * s);
int32_t pdf_3d_get_activation(int32_t _ctx, int32_t annot);
u8 const * pdf_3d_get_data(int32_t _ctx, int32_t annot, size_t * len_out);
int32_t pdf_3d_get_deactivation(int32_t _ctx, int32_t annot);
int32_t pdf_3d_get_default_view(int32_t _ctx, int32_t annot);
int32_t pdf_3d_get_format(int32_t _ctx, int32_t annot);
int32_t pdf_3d_get_view_camera(int32_t _ctx, int32_t annot, int32_t view_index, Camera3D * camera_out);
char * pdf_3d_get_view_name(int32_t _ctx, int32_t annot, int32_t index);
char * pdf_3d_lighting_to_string(int32_t _ctx, int32_t lighting);
char * pdf_3d_render_mode_to_string(int32_t _ctx, int32_t mode);
int32_t pdf_3d_set_activation(int32_t _ctx, int32_t annot, int32_t mode);
int32_t pdf_3d_set_deactivation(int32_t _ctx, int32_t annot, int32_t mode);
int32_t pdf_3d_set_default_view(int32_t _ctx, int32_t annot, int32_t index);
int32_t pdf_3d_set_interactive(int32_t _ctx, int32_t annot, int32_t interactive);
int32_t pdf_3d_set_navigation(int32_t _ctx, int32_t annot, int32_t show);
int32_t pdf_3d_set_prc_data(int32_t _ctx, int32_t annot, u8 const * data, size_t len);
int32_t pdf_3d_set_toolbar(int32_t _ctx, int32_t annot, int32_t show);
int32_t pdf_3d_set_u3d_data(int32_t _ctx, int32_t annot, u8 const * data, size_t len);
int32_t pdf_3d_set_view_background(int32_t _ctx, int32_t annot, int32_t view_index, float r, float g, float b, float a);
int32_t pdf_3d_set_view_camera(int32_t _ctx, int32_t annot, int32_t view_index, Camera3D const * camera);
int32_t pdf_3d_set_view_lighting(int32_t _ctx, int32_t annot, int32_t view_index, int32_t lighting);
int32_t pdf_3d_set_view_render_mode(int32_t _ctx, int32_t annot, int32_t view_index, int32_t mode);
int32_t pdf_3d_view_count(int32_t _ctx, int32_t annot);
void pdf_drop_3d_annotation(int32_t _ctx, int32_t annot);
int32_t pdf_new_3d_annotation(int32_t _ctx);

#ifdef __cplusplus
}
#endif

#endif /* MUPDF_PDF_PDF_3D_H */
