// MicroPDF - MuPDF API Compatible C Header
// Auto-generated from Rust FFI - DO NOT EDIT MANUALLY
// Module: table_detect

#ifndef MUPDF_FITZ_TABLE_DETECT_H
#define MUPDF_FITZ_TABLE_DETECT_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================================
// Table_detect Functions (14 total)
// ============================================================================

void fz_drop_table(int32_t _ctx, int32_t table);
void fz_drop_table_detector(int32_t _ctx, int32_t detector);
void fz_free_table_string(int32_t _ctx, char * s);
int32_t fz_new_table(int32_t _ctx, float x0, float y0, float x1, float y1);
int32_t fz_new_table_detector(int32_t _ctx, int min_rows, int min_cols, float min_confidence);
int fz_table_col_count(int32_t _ctx, int32_t table);
float fz_table_confidence(int32_t _ctx, int32_t table);
void fz_table_detector_clear(int32_t _ctx, int32_t detector);
int fz_table_detector_count(int32_t _ctx, int32_t detector);
char * fz_table_get_cell_text(int32_t _ctx, int32_t table, int row, int col);
int fz_table_row_count(int32_t _ctx, int32_t table);
char * fz_table_to_csv(int32_t _ctx, int32_t table);
char * fz_table_to_html(int32_t _ctx, int32_t table);
char * fz_table_to_markdown(int32_t _ctx, int32_t table);

#ifdef __cplusplus
}
#endif

#endif /* MUPDF_FITZ_TABLE_DETECT_H */
