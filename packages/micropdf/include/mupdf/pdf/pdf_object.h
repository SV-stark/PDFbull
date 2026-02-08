// MicroPDF - MuPDF API Compatible C Header
// Auto-generated from Rust FFI - DO NOT EDIT MANUALLY
// Module: pdf_object

#ifndef MUPDF_PDF_PDF_OBJECT_H
#define MUPDF_PDF_PDF_OBJECT_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================================
// Pdf_object Functions (99 total)
// ============================================================================

void pdf_arena_free_obj(int32_t _ctx, int32_t handle);
int32_t pdf_arena_new_array(int32_t _ctx, uint32_t arena_id, size_t capacity);
int32_t pdf_arena_new_bool(int32_t _ctx, uint32_t arena_id, int32_t value);
int32_t pdf_arena_new_dict(int32_t _ctx, uint32_t arena_id, size_t capacity);
int32_t pdf_arena_new_indirect(int32_t _ctx, uint32_t arena_id, int32_t num, int32_t generation);
int32_t pdf_arena_new_int(int32_t _ctx, uint32_t arena_id, int64_t value);
int32_t pdf_arena_new_name(int32_t _ctx, uint32_t arena_id, const char * name);
int32_t pdf_arena_new_null(int32_t _ctx, uint32_t arena_id);
int32_t pdf_arena_new_real(int32_t _ctx, uint32_t arena_id, float value);
int32_t pdf_arena_new_string(int32_t _ctx, uint32_t arena_id, u8 const * data, size_t len);
void pdf_array_delete(int32_t _ctx, int32_t array, int32_t index);
int32_t pdf_array_get(int32_t _ctx, int32_t array, int32_t index);
void pdf_array_insert(int32_t _ctx, int32_t array, int32_t index, int32_t obj);
int32_t pdf_array_len(int32_t _ctx, int32_t array);
void pdf_array_push(int32_t _ctx, int32_t array, int32_t obj);
void pdf_array_push_bool(int32_t _ctx, int32_t array, int32_t x);
void pdf_array_push_int(int32_t _ctx, int32_t array, int64_t x);
void pdf_array_push_name(int32_t _ctx, int32_t array, const char * name);
void pdf_array_push_real(int32_t _ctx, int32_t array, double x);
void pdf_array_push_string(int32_t _ctx, int32_t array, const char * str, size_t len);
void pdf_array_put(int32_t _ctx, int32_t array, int32_t index, int32_t obj);
void pdf_clean_obj(int32_t _ctx, int32_t obj);
void pdf_clear_object_arena(int32_t _ctx, uint32_t arena_id);
void pdf_compact_object_arena(int32_t _ctx, uint32_t arena_id);
int32_t pdf_copy_array(int32_t _ctx, int32_t _doc, int32_t array);
int32_t pdf_copy_dict(int32_t _ctx, int32_t _doc, int32_t dict);
int32_t pdf_deep_copy_obj(int32_t _ctx, int32_t _doc, int32_t obj);
void pdf_dict_dels(int32_t _ctx, int32_t dict, const char * key);
int32_t pdf_dict_get(int32_t _ctx, int32_t dict, int32_t key);
int32_t pdf_dict_get_key(int32_t _ctx, int32_t dict, int32_t index);
int32_t pdf_dict_get_val(int32_t _ctx, int32_t dict, int32_t index);
int32_t pdf_dict_gets(int32_t _ctx, int32_t dict, const char * key);
int32_t pdf_dict_len(int32_t _ctx, int32_t dict);
void pdf_dict_put(int32_t _ctx, int32_t dict, int32_t key, int32_t val);
void pdf_dict_put_bool(int32_t _ctx, int32_t dict, int32_t key, int32_t x);
void pdf_dict_put_int(int32_t _ctx, int32_t dict, int32_t key, int64_t x);
void pdf_dict_put_name(int32_t _ctx, int32_t dict, int32_t key, const char * name);
void pdf_dict_put_real(int32_t _ctx, int32_t dict, int32_t key, double x);
void pdf_dict_put_string(int32_t _ctx, int32_t dict, int32_t key, const char * str, size_t len);
void pdf_dict_puts(int32_t _ctx, int32_t dict, const char * key, int32_t val);
void pdf_dirty_obj(int32_t _ctx, int32_t obj);
void pdf_drop_obj(int32_t _ctx, int32_t obj);
void pdf_drop_object_arena(int32_t _ctx, uint32_t arena_id);
int32_t pdf_is_arena_handle(int32_t _ctx, int32_t handle);
int32_t pdf_is_array(int32_t _ctx, int32_t obj);
int32_t pdf_is_bool(int32_t _ctx, int32_t obj);
int32_t pdf_is_dict(int32_t _ctx, int32_t obj);
int32_t pdf_is_indirect(int32_t _ctx, int32_t obj);
int32_t pdf_is_int(int32_t _ctx, int32_t obj);
int32_t pdf_is_name(int32_t _ctx, int32_t obj);
int32_t pdf_is_null(int32_t _ctx, int32_t obj);
int32_t pdf_is_number(int32_t _ctx, int32_t obj);
int32_t pdf_is_real(int32_t _ctx, int32_t obj);
int32_t pdf_is_stream(int32_t _ctx, int32_t obj);
int32_t pdf_is_string(int32_t _ctx, int32_t obj);
int32_t pdf_keep_obj(int32_t _ctx, int32_t obj);
int32_t pdf_load_object(int32_t _ctx, int32_t _doc, int32_t num, int32_t generation);
int32_t pdf_mark_obj(int32_t _ctx, int32_t obj);
int32_t pdf_name_eq(int32_t _ctx, int32_t a, int32_t b);
int32_t pdf_new_array(int32_t _ctx, int32_t _doc, int32_t initialcap);
int32_t pdf_new_bool(int32_t _ctx, int32_t b);
int32_t pdf_new_date(int32_t _ctx, int32_t _doc, int32_t year, int32_t month, int32_t day, int32_t hour, int32_t minute, int32_t second);
int32_t pdf_new_dict(int32_t _ctx, int32_t _doc, int32_t initialcap);
int32_t pdf_new_indirect(int32_t _ctx, int32_t _doc, int32_t num, int32_t generation);
int32_t pdf_new_int(int32_t _ctx, int64_t i);
int32_t pdf_new_matrix(int32_t _ctx, int32_t _doc, float a, float b, float c, float d, float e, float f);
int32_t pdf_new_name(int32_t _ctx, const char * str);
int32_t pdf_new_null(int32_t _ctx);
uint32_t pdf_new_object_arena(int32_t _ctx);
uint32_t pdf_new_object_arena_with_size(int32_t _ctx, size_t chunk_size);
int32_t pdf_new_point(int32_t _ctx, int32_t _doc, float x, float y);
int32_t pdf_new_real(int32_t _ctx, float f);
int32_t pdf_new_rect(int32_t _ctx, int32_t _doc, float x0, float y0, float x1, float y1);
int32_t pdf_new_string(int32_t _ctx, const char * str, size_t len);
int32_t pdf_new_text_string(int32_t _ctx, const char * s);
int32_t pdf_obj_is_dirty(int32_t _ctx, int32_t obj);
int32_t pdf_obj_is_resolved(int32_t _ctx, int32_t _doc, int32_t obj);
int32_t pdf_obj_marked(int32_t _ctx, int32_t obj);
int32_t pdf_obj_parent_num(int32_t _ctx, int32_t obj);
int32_t pdf_obj_refs(int32_t _ctx, int32_t obj);
int32_t pdf_objcmp(int32_t _ctx, int32_t a, int32_t b);
size_t pdf_object_arena_count(int32_t _ctx);
ArenaStats pdf_object_arena_stats(int32_t _ctx, uint32_t arena_id);
int32_t pdf_resolve_indirect(int32_t _ctx, int32_t _doc, int32_t obj);
void pdf_set_obj_parent(int32_t _ctx, int32_t obj, int32_t num);
int32_t pdf_to_bool(int32_t _ctx, int32_t obj);
int32_t pdf_to_bool_default(int32_t _ctx, int32_t obj, int32_t def);
int32_t pdf_to_gen(int32_t _ctx, int32_t obj);
int32_t pdf_to_int(int32_t _ctx, int32_t obj);
int64_t pdf_to_int64(int32_t _ctx, int32_t obj);
int32_t pdf_to_int_default(int32_t _ctx, int32_t obj, int32_t def);
const char * pdf_to_name(int32_t _ctx, int32_t obj);
int32_t pdf_to_num(int32_t _ctx, int32_t obj);
float pdf_to_real(int32_t _ctx, int32_t obj);
float pdf_to_real_default(int32_t _ctx, int32_t obj, float def);
const char * pdf_to_str_buf(int32_t _ctx, int32_t obj);
size_t pdf_to_str_len(int32_t _ctx, int32_t obj);
const char * pdf_to_string(int32_t _ctx, int32_t obj, size_t * sizep);
void pdf_unmark_obj(int32_t _ctx, int32_t obj);

#ifdef __cplusplus
}
#endif

#endif /* MUPDF_PDF_PDF_OBJECT_H */
