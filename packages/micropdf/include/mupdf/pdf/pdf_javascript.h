// MicroPDF - MuPDF API Compatible C Header
// Auto-generated from Rust FFI - DO NOT EDIT MANUALLY
// Module: pdf_javascript

#ifndef MUPDF_PDF_PDF_JAVASCRIPT_H
#define MUPDF_PDF_PDF_JAVASCRIPT_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================================
// Pdf_javascript Functions (24 total)
// ============================================================================

void pdf_disable_js(int32_t _ctx, int32_t doc);
void pdf_drop_js(int32_t _ctx, int32_t js);
void pdf_enable_js(int32_t _ctx, int32_t doc);
int32_t pdf_get_js(int32_t _ctx, int32_t doc);
void pdf_js_clear_console_log(int32_t js);
void pdf_js_clear_last_error(int32_t js);
void pdf_js_event_init(int32_t js, int32_t target, const char * value, int32_t will_commit);
void pdf_js_event_init_keystroke(int32_t js, int32_t target, KeystrokeEvent * evt);
int32_t pdf_js_event_result(int32_t js);
int32_t pdf_js_event_result_keystroke(int32_t js, KeystrokeEvent * evt);
int32_t pdf_js_event_result_validate(int32_t js, char * * newvalue);
void pdf_js_event_set_rc(int32_t js, int32_t rc);
void pdf_js_event_set_value(int32_t js, const char * value);
char * pdf_js_event_value(int32_t js);
void pdf_js_execute(int32_t js, const char * name, const char * code, char * * result);
void pdf_js_free_string(int32_t _ctx, char * s);
char * pdf_js_get_console_log(int32_t js);
char * pdf_js_get_global(int32_t js, const char * name);
char * pdf_js_get_last_error(int32_t js);
int32_t pdf_js_is_enabled(int32_t js);
void pdf_js_register_script(int32_t js, const char * name, const char * code);
void pdf_js_run_script(int32_t js, const char * name, char * * result);
void pdf_js_set_global(int32_t js, const char * name, const char * value);
int32_t pdf_js_supported(int32_t _ctx, int32_t doc);

#ifdef __cplusplus
}
#endif

#endif /* MUPDF_PDF_PDF_JAVASCRIPT_H */
