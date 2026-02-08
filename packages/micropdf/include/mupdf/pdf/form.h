// MicroPDF - MuPDF API Compatible C Header
// Auto-generated from Rust FFI - DO NOT EDIT MANUALLY
// Module: form

#ifndef MUPDF_PDF_FORM_H
#define MUPDF_PDF_FORM_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================================
// Form Functions (57 total)
// ============================================================================

int32_t pdf_add_field_choice(int32_t _ctx, int32_t field, const char * label, const char * value);
int32_t pdf_clone_field(int32_t _ctx, int32_t field);
int32_t pdf_create_checkbox(int32_t _ctx, int32_t _form, const char * name, float x, float y, float width, float height, int32_t checked);
int32_t pdf_create_combo_box(int32_t _ctx, int32_t _form, const char * name, float x, float y, float width, float height);
int32_t pdf_create_push_button(int32_t _ctx, int32_t _form, const char * name, float x, float y, float width, float height, const char * caption);
int32_t pdf_create_signature_field(int32_t _ctx, int32_t _form, const char * name, float x, float y, float width, float height);
int32_t pdf_create_text_field(int32_t _ctx, int32_t _form, const char * name, float x, float y, float width, float height, int32_t max_len);
int32_t pdf_delete_field(int32_t _ctx, int32_t _form, int32_t field);
void pdf_drop_form(int32_t _ctx, int32_t form);
int32_t pdf_field_alignment(int32_t _ctx, int32_t field);
void pdf_field_bg_color(int32_t _ctx, int32_t field, float * color);
void pdf_field_border_color(int32_t _ctx, int32_t field, float * color);
float pdf_field_border_width(int32_t _ctx, int32_t field);
int32_t pdf_field_choice_count(int32_t _ctx, int32_t field);
int32_t pdf_field_choice_label(int32_t _ctx, int32_t field, int32_t index, c_char * buf, int32_t size);
int32_t pdf_field_choice_value(int32_t _ctx, int32_t field, int32_t index, c_char * buf, int32_t size);
void pdf_field_clear_selection(int32_t _ctx, int32_t field);
int32_t pdf_field_default_value(int32_t _ctx, int32_t field, char * buf, int32_t buf_size);
uint32_t pdf_field_flags(int32_t _ctx, int32_t field);
float pdf_field_font_size(int32_t _ctx, int32_t field);
int32_t pdf_field_is_checked(int32_t _ctx, int32_t field);
int32_t pdf_field_is_combo(int32_t _ctx, int32_t field);
int32_t pdf_field_is_edit(int32_t _ctx, int32_t field);
int32_t pdf_field_is_multiline(int32_t _ctx, int32_t field);
int32_t pdf_field_is_multiselect(int32_t _ctx, int32_t field);
int32_t pdf_field_is_password(int32_t _ctx, int32_t field);
int32_t pdf_field_is_read_only(int32_t _ctx, int32_t field);
int32_t pdf_field_is_required(int32_t _ctx, int32_t field);
int32_t pdf_field_is_signed(int32_t _ctx, int32_t field);
int32_t pdf_field_is_valid(int32_t _ctx, int32_t field);
int32_t pdf_field_max_len(int32_t _ctx, int32_t field);
int32_t pdf_field_name(int32_t _ctx, int32_t field, c_char * buf, int32_t size);
fz_rect pdf_field_rect(int32_t _ctx, int32_t field);
int32_t pdf_field_selected_index(int32_t _ctx, int32_t field);
int32_t pdf_field_text_format(int32_t _ctx, int32_t field);
int32_t pdf_field_type(int32_t _ctx, int32_t field);
int32_t pdf_field_value(int32_t _ctx, int32_t field, c_char * buf, int32_t size);
int32_t pdf_first_widget(int32_t _ctx, int32_t page);
int32_t pdf_form(int32_t _ctx, int32_t _doc);
int32_t pdf_form_field_count(int32_t _ctx, int32_t form);
int32_t pdf_keep_form(int32_t _ctx, int32_t form);
int32_t pdf_lookup_field(int32_t _ctx, int32_t form, const char * name);
int32_t pdf_next_widget(int32_t _ctx, int32_t widget);
int32_t pdf_remove_field_choice(int32_t _ctx, int32_t field, int32_t idx);
void pdf_reset_form(int32_t _ctx, int32_t form);
void pdf_set_field_alignment(int32_t _ctx, int32_t field, int32_t align);
void pdf_set_field_bg_color(int32_t _ctx, int32_t field, float const * color);
void pdf_set_field_border_color(int32_t _ctx, int32_t field, float const * color);
void pdf_set_field_border_width(int32_t _ctx, int32_t field, float width);
int32_t pdf_set_field_checked(int32_t _ctx, int32_t field, int32_t checked);
int32_t pdf_set_field_default_value(int32_t _ctx, int32_t field, const char * value);
void pdf_set_field_flags(int32_t _ctx, int32_t field, uint32_t flags);
void pdf_set_field_font_size(int32_t _ctx, int32_t field, float size);
void pdf_set_field_max_len(int32_t _ctx, int32_t field, int32_t max_len);
int32_t pdf_set_field_selected_index(int32_t _ctx, int32_t field, int32_t idx);
int32_t pdf_set_field_value(int32_t _ctx, int32_t field, const char * value);
int32_t pdf_validate_form(int32_t _ctx, int32_t form);

#ifdef __cplusplus
}
#endif

#endif /* MUPDF_PDF_FORM_H */
