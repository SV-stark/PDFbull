// MicroPDF - MuPDF API Compatible C Header
// Auto-generated from Rust FFI - DO NOT EDIT MANUALLY
// Module: annot

#ifndef MUPDF_PDF_ANNOT_H
#define MUPDF_PDF_ANNOT_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================================
// Annot Functions (31 total)
// ============================================================================

int32_t pdf_annot_author(int32_t _ctx, int32_t annot, c_char * buf, int32_t size);
float pdf_annot_border_width(int32_t _ctx, int32_t annot);
void pdf_annot_clear_dirty(int32_t _ctx, int32_t annot);
void pdf_annot_color(int32_t _ctx, int32_t annot, int32_t * n, float * color);
int32_t pdf_annot_contents(int32_t _ctx, int32_t annot, c_char * buf, int32_t size);
uint32_t pdf_annot_flags(int32_t _ctx, int32_t annot);
int32_t pdf_annot_has_dirty(int32_t _ctx, int32_t annot);
int32_t pdf_annot_has_popup(int32_t _ctx, int32_t annot);
void pdf_annot_interior_color(int32_t _ctx, int32_t annot, int32_t * n, float * color);
int32_t pdf_annot_is_valid(int32_t _ctx, int32_t annot);
int32_t pdf_annot_line(int32_t _ctx, int32_t annot, fz_point * a, fz_point * b);
float pdf_annot_opacity(int32_t _ctx, int32_t annot);
fz_rect pdf_annot_rect(int32_t _ctx, int32_t annot);
int32_t pdf_annot_type(int32_t _ctx, int32_t annot);
int32_t pdf_clone_annot(int32_t _ctx, int32_t annot);
int32_t pdf_create_annot(int32_t _ctx, int32_t _page, int32_t annot_type);
void pdf_delete_annot(int32_t _ctx, int32_t _page, int32_t annot);
void pdf_drop_annot(int32_t _ctx, int32_t annot);
int32_t pdf_first_annot(int32_t _ctx, int32_t page);
int32_t pdf_keep_annot(int32_t _ctx, int32_t annot);
int32_t pdf_next_annot(int32_t _ctx, int32_t annot);
void pdf_set_annot_author(int32_t _ctx, int32_t annot, const char * text);
void pdf_set_annot_border_width(int32_t _ctx, int32_t annot, float width);
void pdf_set_annot_color(int32_t _ctx, int32_t annot, int32_t n, float const * color);
void pdf_set_annot_contents(int32_t _ctx, int32_t annot, const char * text);
void pdf_set_annot_flags(int32_t _ctx, int32_t annot, uint32_t flags);
void pdf_set_annot_interior_color(int32_t _ctx, int32_t annot, int32_t n, float const * color);
void pdf_set_annot_line(int32_t _ctx, int32_t annot, fz_point a, fz_point b);
void pdf_set_annot_opacity(int32_t _ctx, int32_t annot, float opacity);
void pdf_set_annot_rect(int32_t _ctx, int32_t annot, fz_rect rect);
int32_t pdf_update_annot(int32_t _ctx, int32_t annot);

#ifdef __cplusplus
}
#endif

#endif /* MUPDF_PDF_ANNOT_H */
