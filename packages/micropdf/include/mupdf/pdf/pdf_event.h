// MicroPDF - MuPDF API Compatible C Header
// Auto-generated from Rust FFI - DO NOT EDIT MANUALLY
// Module: pdf_event

#ifndef MUPDF_PDF_PDF_EVENT_H
#define MUPDF_PDF_PDF_EVENT_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================================
// Pdf_event Functions (26 total)
// ============================================================================

char * pdf_access_exec_menu_item_event(int32_t _ctx, int32_t handler, int32_t index);
int32_t pdf_access_launch_url_event(int32_t _ctx, int32_t handler, int32_t index, char * * url_out, int32_t * new_frame_out);
int32_t pdf_alert_get_button_pressed(AlertEvent const * evt);
void pdf_alert_set_button_group(AlertEvent * evt, int32_t button_group);
void pdf_alert_set_button_pressed(AlertEvent * evt, int32_t button);
void pdf_alert_set_icon(AlertEvent * evt, int32_t icon_type);
void pdf_alert_set_message(AlertEvent * evt, const char * message);
void pdf_alert_set_title(AlertEvent * evt, const char * title);
void pdf_clear_pending_events(int32_t _ctx, int32_t handler);
int32_t pdf_count_pending_events(int32_t _ctx, int32_t handler);
void pdf_drop_alert_event(AlertEvent * evt);
void pdf_drop_event_handler(int32_t _ctx, int32_t handler);
void pdf_drop_mail_doc_event(MailDocEvent * evt);
void pdf_event_issue_alert(int32_t _ctx, int32_t handler, AlertEvent const * evt);
void pdf_event_issue_exec_menu_item(int32_t _ctx, int32_t handler, const char * item);
void pdf_event_issue_launch_url(int32_t _ctx, int32_t handler, const char * url, int32_t new_frame);
void pdf_event_issue_mail_doc(int32_t _ctx, int32_t handler, MailDocEvent const * evt);
void pdf_event_issue_print(int32_t _ctx, int32_t handler);
void * pdf_get_doc_event_callback_data(int32_t _ctx, int32_t handler);
int32_t pdf_get_pending_event_type(int32_t _ctx, int32_t handler, int32_t index);
void pdf_mail_doc_set_subject(MailDocEvent * evt, const char * subject);
void pdf_mail_doc_set_to(MailDocEvent * evt, const char * to);
AlertEvent * pdf_new_alert_event(void);
int32_t pdf_new_event_handler(int32_t _ctx, int32_t doc);
MailDocEvent * pdf_new_mail_doc_event(void);
void pdf_set_doc_event_callback(int32_t _ctx, int32_t handler, Option<DocEventCallback> event_cb, Option<FreeEventDataCallback> free_cb, void * data);

#ifdef __cplusplus
}
#endif

#endif /* MUPDF_PDF_PDF_EVENT_H */
