// MicroPDF - MuPDF API Compatible C Header
// Auto-generated from Rust FFI - DO NOT EDIT MANUALLY
// Module: xml

#ifndef MUPDF_FITZ_XML_H
#define MUPDF_FITZ_XML_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================================
// Xml Functions (18 total)
// ============================================================================

void fz_drop_xml(int32_t _ctx, int32_t doc);
int32_t fz_new_xml_document(int32_t _ctx);
int32_t fz_parse_xml(int32_t _ctx, const char * xml_string, int32_t _preserve_whitespace);
int32_t fz_parse_xml_from_buffer(int32_t _ctx, int32_t buffer, int32_t _preserve_whitespace);
const char * fz_xml_att(int32_t _ctx, int32_t node, const char * name);
int32_t fz_xml_att_count(int32_t _ctx, int32_t node);
int32_t fz_xml_child_count(int32_t _ctx, int32_t node);
int32_t fz_xml_down(int32_t _ctx, int32_t node);
int32_t fz_xml_find(int32_t _ctx, int32_t node, const char * path);
int32_t fz_xml_find_all(int32_t _ctx, int32_t node, const char * tag, int32_t * results, int32_t max_results);
int32_t fz_xml_is_tag(int32_t _ctx, int32_t node, const char * tag);
int32_t fz_xml_next(int32_t _ctx, int32_t node);
int32_t fz_xml_node_type(int32_t _ctx, int32_t node);
int32_t fz_xml_prev(int32_t _ctx, int32_t node);
int32_t fz_xml_root(int32_t _ctx, int32_t doc);
const char * fz_xml_tag(int32_t _ctx, int32_t node);
const char * fz_xml_text(int32_t _ctx, int32_t node);
int32_t fz_xml_up(int32_t _ctx, int32_t node);

#ifdef __cplusplus
}
#endif

#endif /* MUPDF_FITZ_XML_H */
