// MicroPDF - MuPDF API Compatible C Header
// Auto-generated from Rust FFI - DO NOT EDIT MANUALLY
// Module: pdf_font

#ifndef MUPDF_PDF_PDF_FONT_H
#define MUPDF_PDF_PDF_FONT_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================================
// Pdf_font Functions (43 total)
// ============================================================================

int32_t pdf_add_cid_font(int32_t _ctx, int32_t _doc, int32_t _font);
int32_t pdf_add_cjk_font(int32_t _ctx, int32_t _doc, int32_t _font, int32_t _script, int32_t _wmode, int32_t _serif);
void pdf_add_hmtx(int32_t _ctx, int32_t font, int32_t lo, int32_t hi, int32_t w);
int32_t pdf_add_simple_font(int32_t _ctx, int32_t _doc, int32_t _font, int32_t _encoding);
int32_t pdf_add_substitute_font(int32_t _ctx, int32_t _doc, int32_t _font);
void pdf_add_vmtx(int32_t _ctx, int32_t font, int32_t lo, int32_t hi, int32_t x, int32_t y, int32_t w);
const char * pdf_clean_font_name(const char * fontname);
void pdf_drop_font(int32_t _ctx, int32_t font);
void pdf_end_hmtx(int32_t _ctx, int32_t font);
void pdf_end_vmtx(int32_t _ctx, int32_t font);
float pdf_font_ascent(int32_t _ctx, int32_t font);
float pdf_font_cap_height(int32_t _ctx, int32_t font);
int32_t pdf_font_cid_to_gid(int32_t _ctx, int32_t font, int32_t cid);
int32_t pdf_font_cid_to_unicode(int32_t _ctx, int32_t font, int32_t cid);
float pdf_font_descent(int32_t _ctx, int32_t font);
int32_t pdf_font_flags(int32_t _ctx, int32_t font);
void pdf_font_free_string(int32_t _ctx, char * s);
int32_t pdf_font_is_embedded(int32_t _ctx, int32_t font);
float pdf_font_italic_angle(int32_t _ctx, int32_t font);
float pdf_font_missing_width(int32_t _ctx, int32_t font);
const char * pdf_font_name(int32_t _ctx, int32_t font);
int32_t pdf_font_wmode(int32_t _ctx, int32_t font);
int32_t pdf_font_writing_supported(int32_t _ctx, int32_t _font);
float pdf_font_x_height(int32_t _ctx, int32_t font);
int32_t pdf_keep_font(int32_t _ctx, int32_t font);
void pdf_load_encoding(const char * * estrings, const char * encoding);
int32_t pdf_load_font(int32_t _ctx, int32_t _doc, int32_t _rdb, int32_t _obj);
int32_t pdf_load_hail_mary_font(int32_t _ctx, int32_t _doc);
int32_t pdf_load_type3_font(int32_t _ctx, int32_t _doc, int32_t _rdb, int32_t _obj);
void pdf_load_type3_glyphs(int32_t _ctx, int32_t _doc, int32_t font);
HorizontalMetrics pdf_lookup_hmtx(int32_t _ctx, int32_t font, int32_t cid);
u8 const * pdf_lookup_substitute_font(int32_t _ctx, int32_t mono, int32_t serif, int32_t bold, int32_t italic, int32_t * len);
VerticalMetrics pdf_lookup_vmtx(int32_t _ctx, int32_t font, int32_t cid);
int32_t pdf_new_font_desc(int32_t _ctx);
void pdf_print_font(int32_t _ctx, int32_t _out, int32_t font);
void pdf_set_cid_to_gid(int32_t _ctx, int32_t font, u16 const * table, size_t len);
void pdf_set_cid_to_ucs(int32_t _ctx, int32_t font, u16 const * table, size_t len);
void pdf_set_default_hmtx(int32_t _ctx, int32_t font, int32_t w);
void pdf_set_default_vmtx(int32_t _ctx, int32_t font, int32_t y, int32_t w);
void pdf_set_font_flags(int32_t _ctx, int32_t font, int32_t flags);
void pdf_set_font_name(int32_t _ctx, int32_t font, const char * name);
void pdf_set_font_wmode(int32_t _ctx, int32_t font, int32_t wmode);
void pdf_subset_fonts(int32_t _ctx, int32_t _doc, int32_t _pages_len, int32_t const * _pages);

#ifdef __cplusplus
}
#endif

#endif /* MUPDF_PDF_PDF_FONT_H */
