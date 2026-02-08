// MicroPDF - MuPDF API Compatible C Header
// Auto-generated from Rust FFI - DO NOT EDIT MANUALLY
// Module: link

#ifndef MUPDF_FITZ_LINK_H
#define MUPDF_FITZ_LINK_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================================
// Link Functions (23 total)
// ============================================================================

int32_t fz_clone_link(int32_t _ctx, int32_t link);
int32_t fz_clone_link_list(int32_t _ctx, int32_t list);
int32_t fz_create_link(int32_t _ctx, fz_rect rect, const char * uri);
void fz_drop_link(int32_t _ctx, int32_t link);
void fz_drop_link_list(int32_t _ctx, int32_t list);
int32_t fz_is_external_link(int32_t _ctx, int32_t link);
int32_t fz_is_page_link(int32_t _ctx, int32_t link);
int32_t fz_keep_link(int32_t _ctx, int32_t link);
int32_t fz_link_eq(int32_t _ctx, int32_t link1, int32_t link2);
int32_t fz_link_is_valid(int32_t _ctx, int32_t link);
void fz_link_list_add(int32_t _ctx, int32_t list, int32_t link);
void fz_link_list_clear(int32_t _ctx, int32_t list);
int32_t fz_link_list_count(int32_t _ctx, int32_t list);
int32_t fz_link_list_find_at_point(int32_t _ctx, int32_t list, float x, float y);
int32_t fz_link_list_first(int32_t _ctx, int32_t list);
int32_t fz_link_list_get(int32_t _ctx, int32_t list, int32_t index);
int32_t fz_link_list_is_empty(int32_t _ctx, int32_t list);
int32_t fz_link_page_number(int32_t _ctx, int32_t link);
fz_rect fz_link_rect(int32_t _ctx, int32_t link);
int32_t fz_link_uri(int32_t _ctx, int32_t link, char * buf, int32_t bufsize);
int32_t fz_new_link_list(int32_t _ctx);
void fz_set_link_rect(int32_t _ctx, int32_t link, fz_rect rect);
int32_t fz_set_link_uri(int32_t _ctx, int32_t link, const char * uri);

#ifdef __cplusplus
}
#endif

#endif /* MUPDF_FITZ_LINK_H */
