// MicroPDF - MuPDF API Compatible C Header
// Auto-generated from Rust FFI - DO NOT EDIT MANUALLY
// Module: pdf_resource

#ifndef MUPDF_PDF_PDF_RESOURCE_H
#define MUPDF_PDF_PDF_RESOURCE_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================================
// Pdf_resource Functions (48 total)
// ============================================================================

int32_t pdf_add_colorspace(int32_t _ctx, int32_t _doc, int32_t _cs);
int32_t pdf_add_image(int32_t _ctx, int32_t _doc, int32_t _image);
int32_t pdf_document_output_intent(int32_t _ctx, int32_t _doc);
void pdf_drop_function(int32_t _ctx, int32_t func);
void pdf_drop_pattern(int32_t _ctx, int32_t pat);
void pdf_drop_resource_stack(int32_t _ctx, int32_t stack);
void pdf_drop_resource_tables(int32_t _ctx, int32_t doc);
void pdf_empty_store(int32_t _ctx, int32_t doc);
void pdf_eval_function(int32_t _ctx, int32_t func, float const * input, int32_t inlen, float * output, int32_t outlen);
int32_t pdf_find_colorspace_resource(int32_t _ctx, int32_t doc, int32_t _item, ColorspaceResourceKey * key);
int32_t pdf_find_font_resource(int32_t _ctx, int32_t doc, int32_t _font_type, int32_t _encoding, int32_t _item, FontResourceKey * key);
void * pdf_find_item(int32_t _ctx, void const * _drop, int32_t _key);
size_t pdf_function_size(int32_t _ctx, int32_t func);
int32_t pdf_guess_colorspace_components(int32_t _ctx, int32_t _obj);
int32_t pdf_insert_colorspace_resource(int32_t _ctx, int32_t doc, ColorspaceResourceKey const * key, int32_t obj);
int32_t pdf_insert_font_resource(int32_t _ctx, int32_t doc, FontResourceKey const * key, int32_t obj);
int32_t pdf_is_jpx_image(int32_t _ctx, int32_t _dict);
int32_t pdf_is_tint_colorspace(int32_t _ctx, int32_t _cs);
int32_t pdf_keep_function(int32_t _ctx, int32_t func);
int32_t pdf_keep_pattern(int32_t _ctx, int32_t pat);
int32_t pdf_load_colorspace(int32_t _ctx, int32_t _obj);
int32_t pdf_load_function(int32_t _ctx, int32_t _ref, int32_t n_in, int32_t n_out);
int32_t pdf_load_image(int32_t _ctx, int32_t _doc, int32_t _obj);
int32_t pdf_load_inline_image(int32_t _ctx, int32_t _doc, int32_t _rdb, int32_t _dict, int32_t _file);
int32_t pdf_load_pattern(int32_t _ctx, int32_t doc, int32_t _obj);
int32_t pdf_load_shading(int32_t _ctx, int32_t _doc, int32_t _obj);
int32_t pdf_lookup_resource(int32_t _ctx, int32_t _stack, int32_t _res_type, const char * _name);
int32_t pdf_new_resource_stack(int32_t _ctx, int32_t resources);
int32_t pdf_new_xobject(int32_t _ctx, int32_t _doc, float const * _bbox, float const * _matrix, int32_t _res, int32_t _buffer);
int32_t pdf_pattern_is_mask(int32_t _ctx, int32_t pat);
float pdf_pattern_xstep(int32_t _ctx, int32_t pat);
float pdf_pattern_ystep(int32_t _ctx, int32_t pat);
int32_t pdf_pop_resource_stack(int32_t _ctx, int32_t stack);
void pdf_purge_local_resources(int32_t _ctx, int32_t doc);
void pdf_purge_locals_from_store(int32_t _ctx, int32_t doc);
void pdf_purge_object_from_store(int32_t _ctx, int32_t _doc, int32_t _num);
int32_t pdf_push_resource_stack(int32_t _ctx, int32_t stack, int32_t resources);
void pdf_remove_item(int32_t _ctx, void const * _drop, int32_t _key);
void pdf_sample_shade_function(int32_t _ctx, float * samples, int32_t n, int32_t funcs, int32_t const * func_handles, float t0, float t1);
void pdf_store_item(int32_t _ctx, int32_t _key, void * _val, size_t _itemsize);
void pdf_update_xobject(int32_t _ctx, int32_t _doc, int32_t _xobj, float const * _bbox, float const * _matrix, int32_t _res, int32_t _buffer);
void pdf_xobject_bbox(int32_t _ctx, int32_t _xobj, float * bbox);
int32_t pdf_xobject_colorspace(int32_t _ctx, int32_t _xobj);
int32_t pdf_xobject_isolated(int32_t _ctx, int32_t _xobj);
int32_t pdf_xobject_knockout(int32_t _ctx, int32_t _xobj);
void pdf_xobject_matrix(int32_t _ctx, int32_t _xobj, float * matrix);
int32_t pdf_xobject_resources(int32_t _ctx, int32_t _xobj);
int32_t pdf_xobject_transparency(int32_t _ctx, int32_t _xobj);

#ifdef __cplusplus
}
#endif

#endif /* MUPDF_PDF_PDF_RESOURCE_H */
