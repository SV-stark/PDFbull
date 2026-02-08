// MicroPDF - MuPDF API Compatible C Header
// Auto-generated from Rust FFI - DO NOT EDIT MANUALLY
// Module: pdf_layer

#ifndef MUPDF_PDF_PDF_LAYER_H
#define MUPDF_PDF_PDF_LAYER_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================================
// Pdf_layer Functions (27 total)
// ============================================================================

int32_t pdf_add_layer(int32_t _ctx, int32_t doc, const char * name, int32_t enabled);
int32_t pdf_add_layer_config(int32_t _ctx, int32_t doc, const char * name, const char * creator);
int32_t pdf_add_layer_config_ui(int32_t _ctx, int32_t doc, const char * text, int32_t depth, int32_t ui_type, int32_t selected, int32_t locked);
int32_t pdf_count_layer_config_ui(int32_t _ctx, int32_t doc);
int32_t pdf_count_layer_configs(int32_t _ctx, int32_t doc);
int32_t pdf_count_layers(int32_t _ctx, int32_t doc);
void pdf_deselect_layer_config_ui(int32_t _ctx, int32_t doc, int32_t ui);
void pdf_drop_ocg(int32_t _ctx, int32_t doc);
void pdf_enable_layer(int32_t _ctx, int32_t doc, int32_t layer, int32_t enabled);
void pdf_free_layer_config_ui_text(FfiLayerConfigUi * info);
int32_t pdf_get_current_layer_config(int32_t _ctx, int32_t doc);
int32_t pdf_is_ocg_hidden(int32_t _ctx, int32_t doc, int32_t _rdb, const char * _usage, int32_t _ocg);
const char * pdf_layer_config_creator(int32_t _ctx, int32_t doc, int32_t config_num);
void pdf_layer_config_info(int32_t _ctx, int32_t doc, int32_t config_num, FfiLayerConfig * info);
const char * pdf_layer_config_name(int32_t _ctx, int32_t doc, int32_t config_num);
void pdf_layer_config_ui_info(int32_t _ctx, int32_t doc, int32_t ui, FfiLayerConfigUi * info);
int32_t pdf_layer_config_ui_type_from_string(const char * s);
const char * pdf_layer_config_ui_type_to_string(int32_t ui_type);
void pdf_layer_free_string(int32_t _ctx, char * s);
int32_t pdf_layer_has_unsaved_changes(int32_t _ctx, int32_t doc);
int32_t pdf_layer_is_enabled(int32_t _ctx, int32_t doc, int32_t layer);
const char * pdf_layer_name(int32_t _ctx, int32_t doc, int32_t layer);
int32_t pdf_read_ocg(int32_t _ctx, int32_t doc);
void pdf_select_layer_config(int32_t _ctx, int32_t doc, int32_t config_num);
void pdf_select_layer_config_ui(int32_t _ctx, int32_t doc, int32_t ui);
void pdf_set_layer_config_as_default(int32_t _ctx, int32_t doc);
void pdf_toggle_layer_config_ui(int32_t _ctx, int32_t doc, int32_t ui);

#ifdef __cplusplus
}
#endif

#endif /* MUPDF_PDF_PDF_LAYER_H */
