// MicroPDF - MuPDF API Compatible C Header
// Auto-generated from Rust FFI - DO NOT EDIT MANUALLY
// Module: pdf_conformance

#ifndef MUPDF_FITZ_PDF_CONFORMANCE_H
#define MUPDF_FITZ_PDF_CONFORMANCE_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================================
// Pdf_conformance Functions (24 total)
// ============================================================================

int fz_conformance_error_count(int32_t _ctx, int32_t validator);
int fz_conformance_is_valid(int32_t _ctx, int32_t validator);
int fz_conformance_issue_count(int32_t _ctx, int32_t validator);
int fz_conformance_pdf2_compliant(int32_t _ctx, int32_t validator);
int fz_conformance_pdf_version(int32_t _ctx, int32_t validator);
int fz_conformance_pdfa_claimed(int32_t _ctx, int32_t validator);
int fz_conformance_pdfa_valid(int32_t _ctx, int32_t validator);
int fz_conformance_pdfx_claimed(int32_t _ctx, int32_t validator);
int fz_conformance_pdfx_valid(int32_t _ctx, int32_t validator);
void fz_conformance_validator_reset(int32_t _ctx, int32_t validator);
int fz_conformance_warning_count(int32_t _ctx, int32_t validator);
void fz_drop_conformance_validator(int32_t _ctx, int32_t validator);
void fz_drop_validation_result(int32_t _ctx, int32_t result);
void fz_free_validation_string(int32_t _ctx, char * s);
int32_t fz_new_conformance_validator(int32_t _ctx, int check_pdfa, int check_pdfx, int check_pdf2);
int32_t fz_new_validation_result(int32_t _ctx);
const char * fz_pdfa_level_name(int level);
const char * fz_pdfx_level_name(int level);
void fz_validate_pdf2(int32_t _ctx, int32_t validator);
void fz_validate_pdfa(int32_t _ctx, int32_t validator);
void fz_validate_pdfx(int32_t _ctx, int32_t validator);
char * fz_validation_issue_code(int32_t _ctx, int32_t validator, int index);
char * fz_validation_issue_message(int32_t _ctx, int32_t validator, int index);
int fz_validation_issue_severity(int32_t _ctx, int32_t validator, int index);

#ifdef __cplusplus
}
#endif

#endif /* MUPDF_FITZ_PDF_CONFORMANCE_H */
