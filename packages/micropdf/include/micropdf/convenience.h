// MicroPDF - Convenience Functions
// Auto-generated from Rust FFI - DO NOT EDIT MANUALLY
// Module: convenience
//
// These are high-level convenience wrappers for common PDF operations.
// All functions handle resource management internally, making them easier
// to use from C, Go, Python, and other FFI consumers.

#ifndef MICROPDF_CONVENIENCE_H
#define MICROPDF_CONVENIENCE_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================================
// Result Structures
// ============================================================================

// PDF document information
typedef struct MpPdfInfo {
    int32_t page_count;       // Number of pages in the document
    int32_t is_encrypted;     // Whether the PDF is encrypted
    int32_t needs_password;   // Whether a password is required
    char* version;            // PDF version string (e.g., "1.7") - must be freed
    char* title;              // Document title - must be freed (null if not present)
    char* author;             // Document author - must be freed (null if not present)
    char* subject;            // Document subject - must be freed (null if not present)
    char* creator;            // Document creator - must be freed (null if not present)
} MpPdfInfo;

// Page dimensions (in points, 1/72 inch)
typedef struct MpPageDimensions {
    float width;              // Page width in points
    float height;             // Page height in points
} MpPageDimensions;

// Rendered page data
typedef struct MpRenderedPage {
    uint8_t* data;            // Image data (PNG or RGB) - must be freed
    size_t data_len;          // Length of data in bytes
    int32_t width;            // Image width in pixels
    int32_t height;           // Image height in pixels
} MpRenderedPage;

// Text extraction result
typedef struct MpExtractedText {
    char* text;               // Extracted text - must be freed
    size_t text_len;          // Length of text in bytes (not including null terminator)
    int32_t pages_processed;  // Number of pages processed
} MpExtractedText;

// ============================================================================
// Document Information Functions
// ============================================================================

// Get basic information about a PDF file.
// Returns: 0 on success, negative error code on failure
// Memory: Caller must free string fields using mp_free_pdf_info()
int32_t mp_get_pdf_info(const char* pdf_path, MpPdfInfo* info_out);

// Free an MpPdfInfo structure's string fields.
// Does NOT free the MpPdfInfo struct itself, only its string fields.
void mp_free_pdf_info(MpPdfInfo* info);

// Get the number of pages in a PDF file.
// Returns: Page count on success (>= 0), negative error code on failure
int32_t mp_get_page_count(const char* pdf_path);

// Get the dimensions of a specific page.
// page_num is zero-based.
// Returns: 0 on success, negative error code on failure
int32_t mp_get_page_dimensions(const char* pdf_path, int32_t page_num, MpPageDimensions* dims_out);

// ============================================================================
// Text Extraction Functions
// ============================================================================

// Extract all text from a PDF file.
// Returns: 0 on success, negative error code on failure
// Memory: Caller must free result->text using mp_free_extracted_text()
int32_t mp_extract_text(const char* pdf_path, MpExtractedText* result_out);

// Extract text from a specific page of a PDF file.
// page_num is zero-based.
// Returns: Pointer to null-terminated text on success, NULL on failure
// Memory: Caller must free result using mp_free_string()
char* mp_extract_page_text(const char* pdf_path, int32_t page_num);

// Free extracted text result.
void mp_free_extracted_text(MpExtractedText* result);

// ============================================================================
// Page Rendering Functions
// ============================================================================

// Render a page to PNG image data.
// scale: Scale factor (1.0 = 72 DPI, 2.0 = 144 DPI, etc.)
// Returns: 0 on success, negative error code on failure
// Memory: Caller must free result->data using mp_free_rendered_page()
int32_t mp_render_page_to_png(const char* pdf_path, int32_t page_num, float scale, MpRenderedPage* result_out);

// Render a page to raw RGB pixel data.
// scale: Scale factor (1.0 = 72 DPI)
// Returns: 0 on success, negative error code on failure
// Memory: Caller must free result->data using mp_free_rendered_page()
int32_t mp_render_page_to_rgb(const char* pdf_path, int32_t page_num, float scale, MpRenderedPage* result_out);

// Free a rendered page's data buffer.
void mp_free_rendered_page(MpRenderedPage* result);

// ============================================================================
// File Operations
// ============================================================================

// Merge multiple PDF files into one.
// input_paths: Array of null-terminated path strings
// input_count: Number of paths in the array
// output_path: Path for the output merged PDF
// Returns: 0 on success, negative error code on failure
int32_t mp_merge_pdf_files(const char* const* input_paths, int32_t input_count, const char* output_path);

// Split a PDF into individual page files.
// Creates files named page_001.pdf, page_002.pdf, etc. in the output directory.
// Returns: Number of pages created on success, negative error code on failure
int32_t mp_split_pdf_to_pages(const char* pdf_path, const char* output_dir);

// Copy specific pages from a PDF to a new file.
// page_numbers: Array of zero-based page numbers to copy
// page_count: Number of pages to copy
// Returns: 0 on success, negative error code on failure
int32_t mp_copy_pages(const char* pdf_path, const char* output_path, const int32_t* page_numbers, int32_t page_count);

// ============================================================================
// Validation and Repair
// ============================================================================

// Quick validation check on a PDF file.
// Returns: 1 if PDF appears valid, 0 if invalid, negative on error
int32_t mp_is_valid_pdf(const char* pdf_path);

// Attempt to repair a damaged PDF.
// Returns: 0 on success, negative error code on failure
int32_t mp_repair_damaged_pdf(const char* pdf_path, const char* output_path);

// ============================================================================
// Memory Management
// ============================================================================

// Free a byte buffer allocated by convenience functions.
// Use this to free data returned by mp_render_page_to_png, etc.
void mp_free_bytes(uint8_t* data, size_t len);

// ============================================================================
// Error Codes
// ============================================================================

// Common error codes returned by convenience functions:
// -1: Null parameter (pdf_path, output_path, etc.)
// -2: Null output parameter (info_out, result_out, etc.)
// -3: Failed to open file
// -4: Page number out of range
// -5: Operation failed (rendering, etc.)

#ifdef __cplusplus
}
#endif

#endif /* MICROPDF_CONVENIENCE_H */
