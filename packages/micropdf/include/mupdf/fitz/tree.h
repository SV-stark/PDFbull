// MicroPDF - MuPDF API Compatible C Header
// Auto-generated from Rust FFI - DO NOT EDIT MANUALLY
// Module: tree

#ifndef MUPDF_FITZ_TREE_H
#define MUPDF_FITZ_TREE_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================================
// Tree Functions (27 total)
// ============================================================================

void fz_drop_structure_tree(int32_t _ctx, int32_t tree);
int32_t fz_keep_structure_tree(int32_t _ctx, int32_t tree);
int32_t fz_new_structure_tree(int32_t _ctx);
int32_t fz_tree_add_node(int32_t _ctx, int32_t tree, int32_t parent, int32_t struct_type);
int32_t fz_tree_find_by_id(int32_t _ctx, int32_t tree, const char * id);
size_t fz_tree_get_text_in_order(int32_t _ctx, int32_t node, char * buffer, size_t buffer_size);
const char * fz_tree_node_actual_text(int32_t _ctx, int32_t node);
const char * fz_tree_node_alt(int32_t _ctx, int32_t node);
void fz_tree_node_bbox(int32_t _ctx, int32_t node, float * bbox);
int32_t fz_tree_node_child(int32_t _ctx, int32_t node, int32_t index);
int32_t fz_tree_node_child_count(int32_t _ctx, int32_t node);
int32_t fz_tree_node_first_child(int32_t _ctx, int32_t node);
const char * fz_tree_node_lang(int32_t _ctx, int32_t node);
int32_t fz_tree_node_mcid(int32_t _ctx, int32_t node);
int32_t fz_tree_node_page(int32_t _ctx, int32_t node);
int32_t fz_tree_node_parent(int32_t _ctx, int32_t node);
void fz_tree_node_set_actual_text(int32_t _ctx, int32_t node, const char * text);
void fz_tree_node_set_alt(int32_t _ctx, int32_t node, const char * alt);
void fz_tree_node_set_id(int32_t _ctx, int32_t tree, int32_t node, const char * id);
void fz_tree_node_set_lang(int32_t _ctx, int32_t node, const char * lang);
void fz_tree_node_set_mcid(int32_t _ctx, int32_t node, int32_t mcid);
void fz_tree_node_set_page(int32_t _ctx, int32_t node, int32_t page, float x0, float y0, float x1, float y1);
void fz_tree_node_set_title(int32_t _ctx, int32_t node, const char * title);
const char * fz_tree_node_title(int32_t _ctx, int32_t node);
int32_t fz_tree_node_type(int32_t _ctx, int32_t node);
const char * fz_tree_node_type_name(int32_t _ctx, int32_t node);
int32_t fz_tree_root(int32_t _ctx, int32_t tree);

#ifdef __cplusplus
}
#endif

#endif /* MUPDF_FITZ_TREE_H */
