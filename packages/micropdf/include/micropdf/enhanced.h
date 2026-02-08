// MicroPDF - Enhanced/Extended Functions
// Auto-generated from Rust FFI - DO NOT EDIT MANUALLY
// Module: enhanced
//
// These are MicroPDF-specific extensions beyond MuPDF compatibility.
// All functions are prefixed with mp_* to distinguish from MuPDF functions.

#ifndef MICROPDF_ENHANCED_H
#define MICROPDF_ENHANCED_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================================
// Enhanced Functions (126 total)
// ============================================================================

int32_t mp_add_blank_page(int32_t _ctx, int32_t _doc, float width, float height);
int32_t mp_add_watermark(int32_t _ctx, const char * input_path, const char * output_path, const char * text, float _x, float _y, float font_size, float opacity);
void mp_certificate_drop(int32_t _cert);
const char * mp_certificate_get_issuer(int32_t cert);
const char * mp_certificate_get_subject(int32_t cert);
int mp_certificate_is_valid(int32_t cert);
int32_t mp_certificate_load_pem(const char * cert_path, const char * key_path, const char * key_password);
int32_t mp_certificate_load_pkcs12(const char * path, const char * password);
int mp_create_2up(const char * input_path, const char * output_path, int page_size);
int mp_create_4up(const char * input_path, const char * output_path, int page_size);
int mp_create_9up(const char * input_path, const char * output_path, int page_size);
int mp_create_booklet(const char * input_path, const char * output_path, int binding_type, int page_size, int add_blanks);
int32_t mp_create_highlight_overlay(const char * output_path, PageDim const * page_dims, int32_t page_count, HighlightRect const * highlights, int32_t highlight_count);
int mp_create_nup(const char * input_path, const char * output_path, int cols, int rows, int page_size);
int mp_create_poster(const char * input_path, const char * output_path, int tile_size, float overlap_mm, int cut_marks);
int mp_create_saddle_stitch_booklet(const char * input_path, const char * output_path);
int32_t mp_create_text_overlay(const char * output_path, float width, float height, uint64_t const * font_handles, int32_t font_count, TextOverlayElement const * texts, int32_t text_count, ImageOverlayElement const * image);
int mp_decrypt_pdf(const char * input_path, const char * output_path, const char * password);
int32_t mp_doc_template_create(const char * filename);
void mp_doc_template_free(int32_t handle);
int32_t mp_doc_template_set_margins(int32_t handle, float left, float right, float top, float bottom);
int32_t mp_doc_template_set_page_size(int32_t handle, float width, float height);
int32_t mp_draw_circle(int32_t _ctx, int32_t _page, float _x, float _y, float radius, float r, float g, float b, float alpha, int32_t _fill);
int32_t mp_draw_line(int32_t _ctx, int32_t _page, float _x0, float _y0, float _x1, float _y1, float r, float g, float b, float alpha, float line_width);
int32_t mp_draw_rectangle(int32_t _ctx, int32_t _page, float _x, float _y, float width, float height, float r, float g, float b, float alpha, int32_t _fill);
int mp_encrypt_pdf(const char * input_path, const char * output_path, int32_t options);
void mp_encryption_options_drop(int32_t _options);
int32_t mp_encryption_options_new(void);
int mp_encryption_set_algorithm(int32_t options, int algorithm);
int mp_encryption_set_owner_password(int32_t options, const char * password);
int mp_encryption_set_permissions(int32_t options, int permissions);
int mp_encryption_set_user_password(int32_t options, const char * password);
void mp_font_free(uint64_t handle);
float mp_frame_available_height(int32_t handle);
float mp_frame_available_width(int32_t handle);
int32_t mp_frame_create(const char * id, float x, float y, float width, float height);
void mp_frame_free(int32_t handle);
void mp_free_string(char * s);
void mp_free_timestamp(u8 * data, size_t len);
int32_t mp_hr_create(void);
void mp_hr_free(int32_t handle);
int32_t mp_hr_set_thickness(int32_t handle, float thickness);
int32_t mp_html_file_to_pdf(const char * html_path, const char * output_path, int32_t options);
int32_t mp_html_options_create(void);
void mp_html_options_free(int32_t handle);
float mp_html_options_get_content_height(int32_t handle);
float mp_html_options_get_content_width(int32_t handle);
float mp_html_options_get_page_height(int32_t handle);
float mp_html_options_get_page_width(int32_t handle);
int32_t mp_html_options_set_base_url(int32_t handle, const char * url);
int32_t mp_html_options_set_footer(int32_t handle, const char * html);
int32_t mp_html_options_set_header(int32_t handle, const char * html);
int32_t mp_html_options_set_javascript(int32_t handle, int32_t enabled);
int32_t mp_html_options_set_landscape(int32_t handle, int32_t landscape);
int32_t mp_html_options_set_margins(int32_t handle, float top, float right, float bottom, float left);
int32_t mp_html_options_set_page_size(int32_t handle, int32_t page_size);
int32_t mp_html_options_set_page_size_custom(int32_t handle, float width, float height);
int32_t mp_html_options_set_print_background(int32_t handle, int32_t enabled);
int32_t mp_html_options_set_scale(int32_t handle, float scale);
int32_t mp_html_options_set_stylesheet(int32_t handle, const char * css);
int32_t mp_html_to_pdf(const char * html, const char * output_path, int32_t options);
int32_t mp_image_create(const char * path);
void mp_image_free(int32_t handle);
int32_t mp_image_set_height(int32_t handle, float height);
int32_t mp_image_set_width(int32_t handle, float width);
int mp_is_encrypted(const char * pdf_path);
int32_t mp_linearize_pdf(int32_t _ctx, const char * input_path, const char * output_path);
int32_t mp_list_item_bullet(const char * text);
void mp_list_item_free(int32_t handle);
int32_t mp_list_item_numbered(size_t number, const char * text);
int32_t mp_merge_pdfs(int32_t _ctx, const char * const * paths, int32_t count, const char * output_path);
int32_t mp_optimize_pdf(int32_t _ctx, const char * path);
int32_t mp_overlay_pdf(int32_t _ctx, const char * base_path, const char * output_path, const char * overlay_path, float _opacity);
int mp_page_box_add_bleed(int32_t handle, float bleed, int unit);
int mp_page_box_get(int32_t handle, int page, int box_type, NpRectangle * rect_out);
int32_t mp_page_box_manager_create(const char * pdf_path);
void mp_page_box_manager_free(int32_t handle);
int mp_page_box_manager_page_count(int32_t handle);
int mp_page_box_save(int32_t handle, const char * output_path);
int mp_page_box_set(int32_t handle, int page, int box_type, float llx, float lly, float urx, float ury);
int32_t mp_paragraph_create(const char * text);
void mp_paragraph_free(int32_t handle);
int32_t mp_paragraph_set_font_size(int32_t handle, float size);
int32_t mp_paragraph_set_leading(int32_t handle, float leading);
int32_t mp_paragraph_style_create(const char * name);
void mp_paragraph_style_free(int32_t handle);
int32_t mp_paragraph_style_set_alignment(int32_t handle, int32_t align);
int32_t mp_paragraph_style_set_font_size(int32_t handle, float size);
int32_t mp_paragraph_style_set_leading(int32_t handle, float leading);
int mp_poster_tile_count(const char * pdf_path, int tile_size, float overlap_mm);
int mp_quick_validate(const char * pdf_path);
uint64_t mp_register_font(const char * font_name, u8 const * font_data, size_t data_len);
int mp_repair_pdf(const char * input_path, const char * output_path);
int32_t mp_restore_bookmarks(const char * input_path, const char * output_path, const char * bookmarks_json);
int mp_signature_count(const char * pdf_path);
int mp_signature_create(const char * input_path, const char * output_path, int32_t cert, const char * field_name, int page, float x, float y, float width, float height, const char * reason, const char * location);
int mp_signature_create_invisible(const char * input_path, const char * output_path, int32_t cert, const char * field_name, const char * reason, const char * location);
int mp_signature_verify(const char * pdf_path, const char * field_name, SignatureVerifyResult * result);
void mp_signature_verify_result_free(SignatureVerifyResult * result);
int32_t mp_spacer_create(float height);
void mp_spacer_free(int32_t handle);
int32_t mp_split_pdf(int32_t _ctx, const char * input_path, const char * output_dir);
int32_t mp_story_create(void);
void mp_story_free(int32_t handle);
size_t mp_story_len(int32_t handle);
int32_t mp_stylesheet_add_style(int32_t sheet_handle, int32_t style_handle);
int32_t mp_stylesheet_create(void);
void mp_stylesheet_free(int32_t handle);
int32_t mp_table_create(size_t rows, size_t cols);
void mp_table_free(int32_t handle);
size_t mp_table_num_cols(int32_t handle);
size_t mp_table_num_rows(int32_t handle);
int32_t mp_table_style_add_background(int32_t handle, int32_t start_col, int32_t start_row, int32_t end_col, int32_t end_row, float r, float g, float b);
int32_t mp_table_style_add_grid(int32_t handle, float weight, float r, float g, float b);
int32_t mp_table_style_create(void);
void mp_table_style_free(int32_t handle);
int32_t mp_toc_add_entry(int32_t handle, const char * title, u8 level, size_t page);
int32_t mp_toc_builder_add_heading(int32_t handle, const char * title, u8 level, size_t page);
int32_t mp_toc_builder_create(void);
void mp_toc_builder_free(int32_t handle);
int32_t mp_toc_create(void);
void mp_toc_free(int32_t handle);
int32_t mp_toc_set_title(int32_t handle, const char * title);
int mp_tsa_timestamp(const char * tsa_url, u8 const * data, size_t data_len, u8 const * * timestamp_out, size_t * timestamp_len_out);
int mp_validate_pdf(const char * pdf_path, int mode, NpValidationResult * result_out);
int32_t mp_write_pdf(int32_t _ctx, int32_t _doc, const char * _path);

#ifdef __cplusplus
}
#endif

#endif /* MICROPDF_ENHANCED_H */
