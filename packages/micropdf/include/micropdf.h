/**
 * MicroPDF - Fast, lightweight PDF library
 *
 * This is a comprehensive C FFI header for the MicroPDF Rust library.
 * All functions are prefixed with fz_ or pdf_ for compatibility with MuPDF.
 *
 * This header includes all auto-generated module headers with complete
 * function declarations for all 660+ FFI functions.
 *
 * Usage:
 *   #include <micropdf.h>
 *
 * For MuPDF drop-in compatibility:
 *   #include <mupdf.h>
 */

#ifndef MICROPDF_H
#define MICROPDF_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================================
// Type Definitions - Opaque handles for resource management
// ============================================================================

typedef int32_t fz_context;
typedef int32_t fz_document;
typedef int32_t fz_page;
typedef int32_t fz_device;
typedef int32_t fz_pixmap;
typedef int32_t fz_buffer;
typedef int32_t fz_stream;
typedef int32_t fz_output;
typedef int32_t fz_colorspace;
typedef int32_t fz_font;
typedef int32_t fz_image;
typedef int32_t fz_path;
typedef int32_t fz_text;
typedef int32_t fz_cookie;
typedef int32_t fz_display_list;
typedef int32_t fz_link;
typedef int32_t fz_archive;
typedef int32_t pdf_obj;
typedef int32_t pdf_annot;
typedef int32_t pdf_form_field;

// ============================================================================
// Geometry types (used by many modules)
// ============================================================================

typedef struct {
    float x, y;
} fz_point;

typedef struct {
    float x0, y0;
    float x1, y1;
} fz_rect;

typedef struct {
    int x0, y0;
    int x1, y1;
} fz_irect;

typedef struct {
    float a, b, c, d, e, f;
} fz_matrix;

typedef struct {
    fz_point ul, ur, ll, lr;
} fz_quad;

// ============================================================================
// Common type aliases
// ============================================================================

typedef int32_t PdfObjHandle;
typedef int32_t Handle;

// ============================================================================
// Function Declarations
// ============================================================================

/*
 * All function declarations are auto-generated from Rust FFI source.
 * See individual module headers in mupdf/fitz/ and mupdf/pdf/ for details.
 *
 * Total: 660+ functions covering:
 * - Core fitz functions (geometry, buffers, streams, devices, etc.)
 * - PDF-specific functions (annotations, forms, objects, etc.)
 */

// For complete function declarations, include the comprehensive header:
#include "mupdf.h"

#ifdef __cplusplus
}
#endif

#endif /* MICROPDF_H */
