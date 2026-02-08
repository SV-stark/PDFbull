/*
 * PDF Event FFI
 *
 * Provides PDF document event handling including alerts, print requests,
 * URL launches, email, and menu item execution.
 */

#ifndef MICROPDF_PDF_EVENT_H
#define MICROPDF_PDF_EVENT_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Handle types */
typedef uint64_t fz_context;
typedef uint64_t pdf_document;
typedef uint64_t pdf_event_handler;

/* ============================================================================
 * Event Types
 * ============================================================================ */

#define PDF_DOCUMENT_EVENT_ALERT        0
#define PDF_DOCUMENT_EVENT_PRINT        1
#define PDF_DOCUMENT_EVENT_LAUNCH_URL   2
#define PDF_DOCUMENT_EVENT_MAIL_DOC     3
#define PDF_DOCUMENT_EVENT_SUBMIT       4
#define PDF_DOCUMENT_EVENT_EXEC_MENU_ITEM 5

/* ============================================================================
 * Alert Icon Types
 * ============================================================================ */

#define PDF_ALERT_ICON_ERROR    0
#define PDF_ALERT_ICON_WARNING  1
#define PDF_ALERT_ICON_QUESTION 2
#define PDF_ALERT_ICON_STATUS   3

/* ============================================================================
 * Alert Button Groups
 * ============================================================================ */

#define PDF_ALERT_BUTTON_GROUP_OK           0
#define PDF_ALERT_BUTTON_GROUP_OK_CANCEL    1
#define PDF_ALERT_BUTTON_GROUP_YES_NO       2
#define PDF_ALERT_BUTTON_GROUP_YES_NO_CANCEL 3

/* ============================================================================
 * Alert Button Responses
 * ============================================================================ */

#define PDF_ALERT_BUTTON_NONE   0
#define PDF_ALERT_BUTTON_OK     1
#define PDF_ALERT_BUTTON_CANCEL 2
#define PDF_ALERT_BUTTON_NO     3
#define PDF_ALERT_BUTTON_YES    4

/* ============================================================================
 * Structures
 * ============================================================================ */

/** Document event */
typedef struct {
    int event_type;
} pdf_doc_event;

/** Alert event */
typedef struct pdf_alert_event pdf_alert_event;

/** Launch URL event */
typedef struct pdf_launch_url_event pdf_launch_url_event;

/** Mail document event */
typedef struct pdf_mail_doc_event pdf_mail_doc_event;

/** Event callback type */
typedef void (*pdf_doc_event_cb)(fz_context *ctx, pdf_document *doc, pdf_doc_event *evt, void *data);

/** Free event data callback type */
typedef void (*pdf_free_doc_event_data_cb)(fz_context *ctx, void *data);

/* ============================================================================
 * Event Handler Functions
 * ============================================================================ */

/**
 * Create a new event handler for a document.
 */
pdf_event_handler *pdf_new_event_handler(fz_context *ctx, pdf_document *doc);

/**
 * Drop an event handler.
 */
void pdf_drop_event_handler(fz_context *ctx, pdf_event_handler *handler);

/**
 * Set the document event callback.
 */
void pdf_set_doc_event_callback(fz_context *ctx, pdf_event_handler *handler, pdf_doc_event_cb event_cb, pdf_free_doc_event_data_cb free_cb, void *data);

/**
 * Get the event callback data.
 */
void *pdf_get_doc_event_callback_data(fz_context *ctx, pdf_event_handler *handler);

/* ============================================================================
 * Alert Event Functions
 * ============================================================================ */

/**
 * Create a new alert event.
 */
pdf_alert_event *pdf_new_alert_event(void);

/**
 * Drop an alert event.
 */
void pdf_drop_alert_event(pdf_alert_event *evt);

/**
 * Set alert event message.
 */
void pdf_alert_set_message(pdf_alert_event *evt, const char *message);

/**
 * Set alert event title.
 */
void pdf_alert_set_title(pdf_alert_event *evt, const char *title);

/**
 * Set alert event icon type.
 */
void pdf_alert_set_icon(pdf_alert_event *evt, int icon_type);

/**
 * Set alert event button group.
 */
void pdf_alert_set_button_group(pdf_alert_event *evt, int button_group);

/**
 * Get the button pressed in response.
 */
int pdf_alert_get_button_pressed(const pdf_alert_event *evt);

/**
 * Set the button pressed response.
 */
void pdf_alert_set_button_pressed(pdf_alert_event *evt, int button);

/**
 * Issue an alert event.
 */
void pdf_event_issue_alert(fz_context *ctx, pdf_event_handler *handler, const pdf_alert_event *evt);

/* ============================================================================
 * Print Event Functions
 * ============================================================================ */

/**
 * Issue a print event.
 */
void pdf_event_issue_print(fz_context *ctx, pdf_event_handler *handler);

/* ============================================================================
 * Launch URL Event Functions
 * ============================================================================ */

/**
 * Issue a launch URL event.
 */
void pdf_event_issue_launch_url(fz_context *ctx, pdf_event_handler *handler, const char *url, int new_frame);

/**
 * Access launch URL event details.
 * @return 1 on success, 0 if index out of bounds
 */
int pdf_access_launch_url_event(fz_context *ctx, pdf_event_handler *handler, int index, char **url_out, int *new_frame_out);

/* ============================================================================
 * Mail Document Event Functions
 * ============================================================================ */

/**
 * Create a new mail document event.
 */
pdf_mail_doc_event *pdf_new_mail_doc_event(void);

/**
 * Drop a mail document event.
 */
void pdf_drop_mail_doc_event(pdf_mail_doc_event *evt);

/**
 * Set mail document recipient.
 */
void pdf_mail_doc_set_to(pdf_mail_doc_event *evt, const char *to);

/**
 * Set mail document subject.
 */
void pdf_mail_doc_set_subject(pdf_mail_doc_event *evt, const char *subject);

/**
 * Issue a mail document event.
 */
void pdf_event_issue_mail_doc(fz_context *ctx, pdf_event_handler *handler, const pdf_mail_doc_event *evt);

/* ============================================================================
 * Menu Item Event Functions
 * ============================================================================ */

/**
 * Issue an execute menu item event.
 */
void pdf_event_issue_exec_menu_item(fz_context *ctx, pdf_event_handler *handler, const char *item);

/**
 * Access executed menu item.
 * @return Menu item string (caller must free) or NULL
 */
char *pdf_access_exec_menu_item_event(fz_context *ctx, pdf_event_handler *handler, int index);

/* ============================================================================
 * Event Query Functions
 * ============================================================================ */

/**
 * Get number of pending events.
 */
int pdf_count_pending_events(fz_context *ctx, pdf_event_handler *handler);

/**
 * Get pending event type at index.
 * @return Event type or -1 if index out of bounds
 */
int pdf_get_pending_event_type(fz_context *ctx, pdf_event_handler *handler, int index);

/**
 * Clear all pending events.
 */
void pdf_clear_pending_events(fz_context *ctx, pdf_event_handler *handler);

#ifdef __cplusplus
}
#endif

#endif /* MICROPDF_PDF_EVENT_H */


