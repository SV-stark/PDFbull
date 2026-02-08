// MicroPDF - MuPDF API Compatible C Header
// Auto-generated from Rust FFI - DO NOT EDIT MANUALLY
// Module: convenience

#ifndef MUPDF_FITZ_CONVENIENCE_H
#define MUPDF_FITZ_CONVENIENCE_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================================
// Convenience Functions (16 total)
// ============================================================================

int32_t mp_copy_pages(const char * pdf_path, const char * output_path, int32_t const * page_numbers, int32_t page_count);
char * mp_extract_page_text(const char * pdf_path, int32_t page_num);
int32_t mp_extract_text(const char * pdf_path, MpExtractedText * result_out);
void mp_free_bytes(u8 * data, size_t len);
void mp_free_extracted_text(MpExtractedText * result);
void mp_free_pdf_info(MpPdfInfo * info);
void mp_free_rendered_page(MpRenderedPage * result);
int32_t mp_get_page_count(const char * pdf_path);
int32_t mp_get_page_dimensions(const char * pdf_path, int32_t page_num, MpPageDimensions * dims_out);
int32_t mp_get_pdf_info(const char * pdf_path, MpPdfInfo * info_out);
int32_t mp_is_valid_pdf(const char * pdf_path);
int32_t mp_merge_pdf_files(const char * const * input_paths, int32_t input_count, const char * output_path);
int32_t mp_render_page_to_png(const char * pdf_path, int32_t page_num, float scale, MpRenderedPage * result_out);
int32_t mp_render_page_to_rgb(const char * pdf_path, int32_t page_num, float scale, MpRenderedPage * result_out);
int32_t mp_repair_damaged_pdf(const char * pdf_path, const char * output_path);
int32_t mp_split_pdf_to_pages(const char * pdf_path, const char * output_dir);

#ifdef __cplusplus
}
#endif

#endif /* MUPDF_FITZ_CONVENIENCE_H */
