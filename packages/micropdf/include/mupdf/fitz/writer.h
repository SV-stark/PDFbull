// MicroPDF - MuPDF API Compatible C Header
// Auto-generated from Rust FFI - DO NOT EDIT MANUALLY
// Module: writer

#ifndef MUPDF_FITZ_WRITER_H
#define MUPDF_FITZ_WRITER_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================================
// Writer Functions (47 total)
// ============================================================================

int32_t fz_begin_page(int32_t _ctx, int32_t wri, float mediabox_x0, float mediabox_y0, float mediabox_x1, float mediabox_y1);
void fz_close_document_writer(int32_t _ctx, int32_t wri);
size_t fz_copy_option(int32_t _ctx, const char * val, char * dest, size_t maxlen);
int32_t fz_document_writer_format(int32_t wri);
int32_t fz_document_writer_is_closed(int32_t wri);
int32_t fz_document_writer_page_count(int32_t wri);
void fz_drop_document_writer(int32_t _ctx, int32_t wri);
void fz_end_page(int32_t _ctx, int32_t wri);
int32_t fz_has_option(int32_t _ctx, const char * opts, const char * key, const char * * val);
int32_t fz_new_cbz_writer(int32_t _ctx, const char * path, const char * options);
int32_t fz_new_cbz_writer_with_output(int32_t _ctx, int32_t out, const char * options);
int32_t fz_new_csv_writer(int32_t _ctx, const char * path, const char * options);
int32_t fz_new_csv_writer_with_output(int32_t _ctx, int32_t out, const char * options);
int32_t fz_new_document_writer(int32_t _ctx, const char * path, const char * format, const char * options);
int32_t fz_new_document_writer_with_buffer(int32_t _ctx, int32_t buf, const char * format, const char * options);
int32_t fz_new_document_writer_with_output(int32_t _ctx, int32_t out, const char * format, const char * options);
int32_t fz_new_docx_writer(int32_t _ctx, const char * path, const char * options);
int32_t fz_new_docx_writer_with_output(int32_t _ctx, int32_t out, const char * options);
int32_t fz_new_jpeg_pixmap_writer(int32_t _ctx, const char * path, const char * options);
int32_t fz_new_odt_writer(int32_t _ctx, const char * path, const char * options);
int32_t fz_new_odt_writer_with_output(int32_t _ctx, int32_t out, const char * options);
int32_t fz_new_pam_pixmap_writer(int32_t _ctx, const char * path, const char * options);
int32_t fz_new_pbm_pixmap_writer(int32_t _ctx, const char * path, const char * options);
int32_t fz_new_pcl_writer(int32_t _ctx, const char * path, const char * options);
int32_t fz_new_pcl_writer_with_output(int32_t _ctx, int32_t out, const char * options);
int32_t fz_new_pclm_writer(int32_t _ctx, const char * path, const char * options);
int32_t fz_new_pclm_writer_with_output(int32_t _ctx, int32_t out, const char * options);
int32_t fz_new_pdf_writer(int32_t _ctx, const char * path, const char * options);
int32_t fz_new_pdf_writer_with_output(int32_t _ctx, int32_t out, const char * options);
int32_t fz_new_pdfocr_writer(int32_t _ctx, const char * path, const char * options);
int32_t fz_new_pdfocr_writer_with_output(int32_t _ctx, int32_t out, const char * options);
int32_t fz_new_pgm_pixmap_writer(int32_t _ctx, const char * path, const char * options);
int32_t fz_new_pkm_pixmap_writer(int32_t _ctx, const char * path, const char * options);
int32_t fz_new_png_pixmap_writer(int32_t _ctx, const char * path, const char * options);
int32_t fz_new_pnm_pixmap_writer(int32_t _ctx, const char * path, const char * options);
int32_t fz_new_ppm_pixmap_writer(int32_t _ctx, const char * path, const char * options);
int32_t fz_new_ps_writer(int32_t _ctx, const char * path, const char * options);
int32_t fz_new_ps_writer_with_output(int32_t _ctx, int32_t out, const char * options);
int32_t fz_new_pwg_writer(int32_t _ctx, const char * path, const char * options);
int32_t fz_new_pwg_writer_with_output(int32_t _ctx, int32_t out, const char * options);
int32_t fz_new_svg_writer(int32_t _ctx, const char * path, const char * options);
int32_t fz_new_svg_writer_with_output(int32_t _ctx, int32_t out, const char * options);
int32_t fz_new_text_writer(int32_t _ctx, const char * format, const char * path, const char * options);
int32_t fz_new_text_writer_with_output(int32_t _ctx, const char * format, int32_t out, const char * options);
int32_t fz_option_eq(const char * a, const char * b);
c_void, ) fz_pdfocr_writer_set_progress(int32_t _ctx, int32_t wri);
void fz_write_document(int32_t _ctx, int32_t wri, int32_t doc);

#ifdef __cplusplus
}
#endif

#endif /* MUPDF_FITZ_WRITER_H */
