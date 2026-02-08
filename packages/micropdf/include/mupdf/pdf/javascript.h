/*
 * PDF JavaScript Support FFI
 *
 * This header provides JavaScript scripting support for PDF forms and actions.
 */

#ifndef MICROPDF_PDF_JAVASCRIPT_H
#define MICROPDF_PDF_JAVASCRIPT_H

#include <stdint.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Handle types */
typedef uint64_t fz_context;
typedef uint64_t pdf_document;
typedef uint64_t pdf_js;
typedef uint64_t pdf_obj;

/* JavaScript Event Types */
typedef enum {
    PDF_JS_EVENT_NONE = 0,
    PDF_JS_EVENT_VALIDATE = 1,
    PDF_JS_EVENT_CALCULATE = 2,
    PDF_JS_EVENT_FORMAT = 3,
    PDF_JS_EVENT_KEYSTROKE = 4,
    PDF_JS_EVENT_MOUSE_ENTER = 5,
    PDF_JS_EVENT_MOUSE_EXIT = 6,
    PDF_JS_EVENT_FOCUS = 7,
    PDF_JS_EVENT_BLUR = 8,
    PDF_JS_EVENT_DOC_OPEN = 9,
    PDF_JS_EVENT_DOC_CLOSE = 10,
    PDF_JS_EVENT_PAGE_OPEN = 11,
    PDF_JS_EVENT_PAGE_CLOSE = 12
} pdf_js_event_type;

/* Keystroke Event Structure */
typedef struct pdf_keystroke_event {
    char *change;        /* The change string (characters being typed) */
    int32_t sel_start;   /* Selection start position */
    int32_t sel_end;     /* Selection end position */
    int32_t shift;       /* Whether shift key is pressed (boolean) */
    int32_t rc;          /* Whether the change should be rejected (boolean) */
    char *value;         /* The current field value */
    int32_t will_commit; /* Whether to commit the change (boolean) */
} pdf_keystroke_event;

/* ============================================================================
 * Enable/Disable JavaScript
 * ============================================================================ */

/**
 * Enable JavaScript for a document.
 *
 * @param ctx  Context handle
 * @param doc  Document handle
 */
void pdf_enable_js(fz_context *ctx, pdf_document *doc);

/**
 * Disable JavaScript for a document.
 *
 * @param ctx  Context handle
 * @param doc  Document handle
 */
void pdf_disable_js(fz_context *ctx, pdf_document *doc);

/**
 * Check if JavaScript is supported/enabled for a document.
 *
 * @param ctx  Context handle
 * @param doc  Document handle
 * @return     1 if supported, 0 if not
 */
int pdf_js_supported(fz_context *ctx, pdf_document *doc);

/**
 * Drop (free) a JavaScript context.
 *
 * @param ctx  Context handle
 * @param js   JavaScript context handle
 */
void pdf_drop_js(fz_context *ctx, pdf_js *js);

/**
 * Get the JavaScript context for a document.
 * Creates one if it doesn't exist.
 *
 * @param ctx  Context handle
 * @param doc  Document handle
 * @return     JavaScript context handle
 */
pdf_js *pdf_get_js(fz_context *ctx, pdf_document *doc);

/* ============================================================================
 * Event Handling
 * ============================================================================ */

/**
 * Initialize a JavaScript event.
 *
 * @param js          JavaScript context handle
 * @param target      Target field/object handle
 * @param value       Initial value (can be NULL)
 * @param will_commit Whether this event should commit (1 = true, 0 = false)
 */
void pdf_js_event_init(pdf_js *js, pdf_obj *target, const char *value, int will_commit);

/**
 * Get the result of a JavaScript event.
 *
 * @param js  JavaScript context handle
 * @return    1 if event was accepted, 0 if rejected
 */
int pdf_js_event_result(pdf_js *js);

/**
 * Get the result of a JavaScript event with validation.
 * If valid and newvalue is not null, sets newvalue to the new value.
 *
 * @param js        JavaScript context handle
 * @param newvalue  Receives the new value if valid (caller must free)
 * @return          1 if valid, 0 if invalid
 */
int pdf_js_event_result_validate(pdf_js *js, char **newvalue);

/**
 * Get the current event value.
 *
 * @param js  JavaScript context handle
 * @return    Newly allocated string (caller must free), or NULL
 */
char *pdf_js_event_value(pdf_js *js);

/**
 * Initialize a keystroke event.
 *
 * @param js      JavaScript context handle
 * @param target  Target field handle
 * @param evt     Keystroke event data
 */
void pdf_js_event_init_keystroke(pdf_js *js, pdf_obj *target, pdf_keystroke_event *evt);

/**
 * Get the result of a keystroke event.
 * Updates the event struct with the result.
 *
 * @param js   JavaScript context handle
 * @param evt  Keystroke event data (updated with result)
 * @return     1 if accepted, 0 if rejected
 */
int pdf_js_event_result_keystroke(pdf_js *js, pdf_keystroke_event *evt);

/* ============================================================================
 * Script Execution
 * ============================================================================ */

/**
 * Execute JavaScript code.
 *
 * @param js      JavaScript context handle
 * @param name    Optional name for the script (for debugging, can be NULL)
 * @param code    The JavaScript code to execute
 * @param result  If not null, receives the result (caller must free)
 */
void pdf_js_execute(pdf_js *js, const char *name, const char *code, char **result);

/**
 * Free a string returned by pdf_js functions.
 *
 * @param ctx  Context handle
 * @param s    String to free
 */
void pdf_js_free_string(fz_context *ctx, char *s);

/* ============================================================================
 * Additional Utilities
 * ============================================================================ */

/**
 * Set a global variable in the JavaScript context.
 *
 * @param js     JavaScript context handle
 * @param name   Variable name
 * @param value  Value to set (can be NULL for "undefined")
 */
void pdf_js_set_global(pdf_js *js, const char *name, const char *value);

/**
 * Get a global variable from the JavaScript context.
 *
 * @param js    JavaScript context handle
 * @param name  Variable name
 * @return      Newly allocated string (caller must free), or NULL
 */
char *pdf_js_get_global(pdf_js *js, const char *name);

/**
 * Register a named script.
 *
 * @param js    JavaScript context handle
 * @param name  Script name
 * @param code  JavaScript code
 */
void pdf_js_register_script(pdf_js *js, const char *name, const char *code);

/**
 * Execute a registered script by name.
 *
 * @param js      JavaScript context handle
 * @param name    Script name
 * @param result  If not null, receives the result (caller must free)
 */
void pdf_js_run_script(pdf_js *js, const char *name, char **result);

/**
 * Get the console log output.
 *
 * @param js  JavaScript context handle
 * @return    Newly allocated string with all console.log messages (caller must free)
 */
char *pdf_js_get_console_log(pdf_js *js);

/**
 * Clear the console log.
 *
 * @param js  JavaScript context handle
 */
void pdf_js_clear_console_log(pdf_js *js);

/**
 * Get the last error message.
 *
 * @param js  JavaScript context handle
 * @return    Newly allocated string or NULL if no error (caller must free)
 */
char *pdf_js_get_last_error(pdf_js *js);

/**
 * Clear the last error.
 *
 * @param js  JavaScript context handle
 */
void pdf_js_clear_last_error(pdf_js *js);

/**
 * Set the event.rc value (result code).
 *
 * @param js  JavaScript context handle
 * @param rc  Result code (1 = accept, 0 = reject)
 */
void pdf_js_event_set_rc(pdf_js *js, int rc);

/**
 * Set the event.value.
 *
 * @param js     JavaScript context handle
 * @param value  New value (can be NULL)
 */
void pdf_js_event_set_value(pdf_js *js, const char *value);

/**
 * Check if JavaScript is enabled in the context.
 *
 * @param js  JavaScript context handle
 * @return    1 if enabled, 0 if disabled
 */
int pdf_js_is_enabled(pdf_js *js);

#ifdef __cplusplus
}
#endif

#endif /* MICROPDF_PDF_JAVASCRIPT_H */

