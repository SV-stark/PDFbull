/**
 * MuPDF FFI Compatibility Header
 *
 * This header provides 100% MuPDF-compatible FFI bindings.
 * Include this for drop-in compatibility with MuPDF-based applications.
 *
 * All 660+ fz_* and pdf_* functions are available through this header.
 *
 * Usage:
 *   #include <mupdf-ffi.h>
 *
 * Or for complete MuPDF compatibility:
 *   #include <mupdf.h>
 */

#ifndef MUPDF_FFI_H
#define MUPDF_FFI_H

#include "micropdf.h"

/*
 * All MuPDF-compatible functions are available through micropdf.h
 *
 * Function categories:
 * - Context management (fz_new_context, fz_drop_context, etc.)
 * - Document operations (fz_open_document, fz_load_page, etc.)
 * - Geometry operations (fz_concat, fz_transform_rect, etc.)
 * - Buffer operations (fz_new_buffer, fz_append_data, etc.)
 * - Device operations (fz_new_bbox_device, fz_fill_path, etc.)
 * - Image operations (fz_new_image_from_pixmap, fz_decode_image, etc.)
 * - Text operations (fz_new_text, fz_show_string, etc.)
 * - PDF object operations (pdf_new_dict, pdf_dict_get, etc.)
 * - PDF annotation operations (pdf_create_annot, pdf_set_annot_contents, etc.)
 * - PDF form operations (pdf_next_widget, pdf_set_field_value, etc.)
 *
 * Total coverage: 660+ functions
 */

#endif /* MUPDF_FFI_H */
