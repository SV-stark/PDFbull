// MicroPDF - MuPDF API Compatible C Header
// Auto-generated from Rust FFI - DO NOT EDIT MANUALLY
// Module: svg

#ifndef MUPDF_FITZ_SVG_H
#define MUPDF_FITZ_SVG_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================================
// Svg Functions (32 total)
// ============================================================================

int32_t svg_add_path_command(int32_t _ctx, int32_t elem, int32_t cmd, int32_t relative, float const * args, int32_t num_args);
void svg_drop_document(int32_t _ctx, int32_t doc);
void svg_drop_element(int32_t _ctx, int32_t elem);
char * svg_element_type_name(int32_t _ctx, int32_t element_type);
void svg_free_string(char * s);
char * svg_get_attribute(int32_t _ctx, int32_t elem, const char * name);
char * svg_get_element_id(int32_t _ctx, int32_t elem);
int32_t svg_get_element_type(int32_t _ctx, int32_t elem);
float svg_get_height(int32_t _ctx, int32_t doc);
int32_t svg_get_viewbox(int32_t _ctx, int32_t doc, float * min_x, float * min_y, float * width, float * height);
float svg_get_width(int32_t _ctx, int32_t doc);
int32_t svg_new_device(int32_t _ctx, int32_t _output, float page_width, float page_height, int32_t text_format, int32_t reuse_images);
int32_t svg_new_document(int32_t ctx);
int32_t svg_new_element(int32_t _ctx, int32_t element_type);
int32_t svg_open_document(int32_t ctx, const char * filename);
int32_t svg_open_document_with_stream(int32_t ctx, int32_t _stream);
int32_t svg_parse_color(int32_t _ctx, const char * str, u8 * r, u8 * g, u8 * b);
int32_t svg_parse_device_options(int32_t _ctx, const char * args, int32_t * text_format, int32_t * reuse_images, int32_t * resolution);
int32_t svg_path_command_count(int32_t _ctx, int32_t elem);
char * svg_path_command_name(int32_t _ctx, int32_t cmd, int32_t relative);
int32_t svg_set_attribute(int32_t _ctx, int32_t elem, const char * name, const char * value);
int32_t svg_set_element_id(int32_t _ctx, int32_t elem, const char * id);
int32_t svg_set_fill(int32_t _ctx, int32_t elem, u8 r, u8 g, u8 b, u8 a);
int32_t svg_set_opacity(int32_t _ctx, int32_t elem, float opacity);
int32_t svg_set_size(int32_t _ctx, int32_t doc, float width, float height);
int32_t svg_set_stroke(int32_t _ctx, int32_t elem, u8 r, u8 g, u8 b, u8 a);
int32_t svg_set_stroke_width(int32_t _ctx, int32_t elem, float width);
int32_t svg_set_transform_matrix(int32_t _ctx, int32_t elem, float a, float b, float c, float d, float e, float f);
int32_t svg_set_transform_rotate(int32_t _ctx, int32_t elem, float angle, float cx, float cy);
int32_t svg_set_transform_scale(int32_t _ctx, int32_t elem, float sx, float sy);
int32_t svg_set_transform_translate(int32_t _ctx, int32_t elem, float tx, float ty);
int32_t svg_set_viewbox(int32_t _ctx, int32_t doc, float min_x, float min_y, float width, float height);

#ifdef __cplusplus
}
#endif

#endif /* MUPDF_FITZ_SVG_H */
