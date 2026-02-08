// MicroPDF - MuPDF API Compatible C Header
// Auto-generated from Rust FFI - DO NOT EDIT MANUALLY
// Module: pdf_page

#ifndef MUPDF_PDF_PDF_PAGE_H
#define MUPDF_PDF_PDF_PAGE_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================================
// Pdf_page Functions (55 total)
// ============================================================================

int32_t fz_box_type_from_string(const char * name);
const char * fz_string_from_box_type(int32_t box_type);
Rect pdf_bound_page(int32_t _ctx, int32_t page, int32_t box_type);
void pdf_clip_page(int32_t _ctx, int32_t _page, Rect * _clip);
int32_t pdf_count_pages(int32_t _ctx, int32_t doc);
void pdf_drop_page(int32_t _ctx, int32_t page);
void pdf_drop_page_tree(int32_t _ctx, int32_t _doc);
void pdf_drop_page_tree_internal(int32_t _ctx, int32_t _doc);
void pdf_filter_annot_contents(int32_t _ctx, int32_t _doc, int32_t _annot, void * _options);
void pdf_filter_page_contents(int32_t _ctx, int32_t _doc, int32_t _page, void * _options);
void pdf_flatten_inheritable_page_items(int32_t _ctx, int32_t _pageobj);
int32_t pdf_keep_page(int32_t _ctx, int32_t page);
int32_t pdf_load_default_colorspaces(int32_t _ctx, int32_t _doc, int32_t _page);
int32_t pdf_load_links(int32_t _ctx, int32_t page);
int32_t pdf_load_page(int32_t _ctx, int32_t doc, int32_t number);
void pdf_load_page_tree(int32_t _ctx, int32_t _doc);
int32_t pdf_lookup_page_number(int32_t _ctx, int32_t _doc, int32_t pageobj);
int32_t pdf_lookup_page_obj(int32_t _ctx, int32_t doc, int32_t number);
int32_t pdf_new_pixmap_from_page_contents_with_separations_and_usage(int32_t _ctx, int32_t _page, Matrix _ctm, int32_t _cs, int32_t _seps, int32_t _alpha, const char * _usage, int32_t _box_type);
int32_t pdf_new_pixmap_from_page_contents_with_usage(int32_t _ctx, int32_t _page, Matrix _ctm, int32_t _cs, int32_t _alpha, const char * _usage, int32_t _box_type);
int32_t pdf_new_pixmap_from_page_with_separations_and_usage(int32_t _ctx, int32_t _page, Matrix _ctm, int32_t _cs, int32_t _seps, int32_t _alpha, const char * _usage, int32_t _box_type);
int32_t pdf_new_pixmap_from_page_with_usage(int32_t _ctx, int32_t _page, Matrix _ctm, int32_t _cs, int32_t _alpha, const char * _usage, int32_t _box_type);
void pdf_nuke_annots(int32_t _ctx, int32_t _page);
void pdf_nuke_links(int32_t _ctx, int32_t _page);
void pdf_nuke_page(int32_t _ctx, int32_t _page);
int32_t pdf_page_contents(int32_t _ctx, int32_t page);
int32_t pdf_page_group(int32_t _ctx, int32_t page);
int32_t pdf_page_has_transparency(int32_t _ctx, int32_t page);
int32_t pdf_page_obj(int32_t _ctx, int32_t page);
void pdf_page_obj_transform(int32_t _ctx, int32_t _pageobj, Rect * outbox, Matrix * outctm);
void pdf_page_obj_transform_box(int32_t _ctx, int32_t _pageobj, Rect * outbox, Matrix * outctm, int32_t _box_type);
void * pdf_page_presentation(int32_t _ctx, int32_t _page, void * transition, float * duration);
int32_t pdf_page_resources(int32_t _ctx, int32_t page);
int32_t pdf_page_rotation(int32_t _ctx, int32_t page);
int32_t pdf_page_separations(int32_t _ctx, int32_t _page);
void pdf_page_transform(int32_t _ctx, int32_t page, Rect * mediabox, Matrix * ctm);
void pdf_page_transform_box(int32_t _ctx, int32_t page, Rect * outbox, Matrix * outctm, int32_t box_type);
float pdf_page_user_unit(int32_t _ctx, int32_t page);
int32_t pdf_redact_page(int32_t _ctx, int32_t _doc, int32_t _page, RedactOptions * _opts);
void pdf_run_page(int32_t _ctx, int32_t page, int32_t dev, Matrix ctm, int32_t cookie);
void pdf_run_page_annots(int32_t _ctx, int32_t _page, int32_t _dev, Matrix _ctm, int32_t _cookie);
void pdf_run_page_annots_with_usage(int32_t _ctx, int32_t page, int32_t dev, Matrix ctm, const char * _usage, int32_t cookie);
void pdf_run_page_contents(int32_t _ctx, int32_t _page, int32_t _dev, Matrix _ctm, int32_t _cookie);
void pdf_run_page_contents_with_usage(int32_t _ctx, int32_t page, int32_t dev, Matrix ctm, const char * _usage, int32_t cookie);
void pdf_run_page_widgets(int32_t _ctx, int32_t _page, int32_t _dev, Matrix _ctm, int32_t _cookie);
void pdf_run_page_widgets_with_usage(int32_t _ctx, int32_t page, int32_t dev, Matrix ctm, const char * _usage, int32_t cookie);
void pdf_run_page_with_usage(int32_t _ctx, int32_t page, int32_t dev, Matrix ctm, const char * _usage, int32_t cookie);
void pdf_set_page_box(int32_t _ctx, int32_t page, int32_t box_type, Rect rect);
void pdf_set_page_tree_cache(int32_t _ctx, int32_t _doc, int32_t _enabled);
void pdf_sync_annots(int32_t _ctx, int32_t _page);
void pdf_sync_links(int32_t _ctx, int32_t _page);
void pdf_sync_open_pages(int32_t _ctx, int32_t _doc);
void pdf_sync_page(int32_t _ctx, int32_t _page);
int32_t pdf_update_default_colorspaces(int32_t _ctx, int32_t old_cs, int32_t _res);
void pdf_vectorize_page(int32_t _ctx, int32_t _page);

#ifdef __cplusplus
}
#endif

#endif /* MUPDF_PDF_PDF_PAGE_H */
