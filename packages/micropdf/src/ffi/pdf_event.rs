//! PDF Event FFI Module
//!
//! Provides PDF document event handling including alerts, print requests,
//! URL launches, email, and menu item execution.

use crate::ffi::{Handle, HandleStore};
use std::ffi::{CStr, CString, c_char, c_void};
use std::ptr;
use std::sync::LazyLock;

// ============================================================================
// Type Aliases
// ============================================================================

type ContextHandle = Handle;
type DocumentHandle = Handle;

// ============================================================================
// Event Types
// ============================================================================

/// Alert dialog event
pub const PDF_DOCUMENT_EVENT_ALERT: i32 = 0;
/// Print document event
pub const PDF_DOCUMENT_EVENT_PRINT: i32 = 1;
/// Launch URL event
pub const PDF_DOCUMENT_EVENT_LAUNCH_URL: i32 = 2;
/// Mail document event
pub const PDF_DOCUMENT_EVENT_MAIL_DOC: i32 = 3;
/// Form submission event
pub const PDF_DOCUMENT_EVENT_SUBMIT: i32 = 4;
/// Execute menu item event
pub const PDF_DOCUMENT_EVENT_EXEC_MENU_ITEM: i32 = 5;

// ============================================================================
// Alert Icon Types
// ============================================================================

/// Error icon
pub const PDF_ALERT_ICON_ERROR: i32 = 0;
/// Warning icon
pub const PDF_ALERT_ICON_WARNING: i32 = 1;
/// Question icon
pub const PDF_ALERT_ICON_QUESTION: i32 = 2;
/// Status/info icon
pub const PDF_ALERT_ICON_STATUS: i32 = 3;

// ============================================================================
// Alert Button Groups
// ============================================================================

/// OK button only
pub const PDF_ALERT_BUTTON_GROUP_OK: i32 = 0;
/// OK and Cancel buttons
pub const PDF_ALERT_BUTTON_GROUP_OK_CANCEL: i32 = 1;
/// Yes and No buttons
pub const PDF_ALERT_BUTTON_GROUP_YES_NO: i32 = 2;
/// Yes, No, and Cancel buttons
pub const PDF_ALERT_BUTTON_GROUP_YES_NO_CANCEL: i32 = 3;

// ============================================================================
// Alert Button Responses
// ============================================================================

/// No button pressed
pub const PDF_ALERT_BUTTON_NONE: i32 = 0;
/// OK button pressed
pub const PDF_ALERT_BUTTON_OK: i32 = 1;
/// Cancel button pressed
pub const PDF_ALERT_BUTTON_CANCEL: i32 = 2;
/// No button pressed
pub const PDF_ALERT_BUTTON_NO: i32 = 3;
/// Yes button pressed
pub const PDF_ALERT_BUTTON_YES: i32 = 4;

// ============================================================================
// Document Event
// ============================================================================

/// Base document event structure
#[derive(Debug, Clone)]
#[repr(C)]
pub struct DocEvent {
    /// Event type (PDF_DOCUMENT_EVENT_*)
    pub event_type: i32,
}

impl DocEvent {
    pub fn new(event_type: i32) -> Self {
        Self { event_type }
    }

    pub fn alert() -> Self {
        Self::new(PDF_DOCUMENT_EVENT_ALERT)
    }

    pub fn print() -> Self {
        Self::new(PDF_DOCUMENT_EVENT_PRINT)
    }

    pub fn launch_url() -> Self {
        Self::new(PDF_DOCUMENT_EVENT_LAUNCH_URL)
    }

    pub fn mail_doc() -> Self {
        Self::new(PDF_DOCUMENT_EVENT_MAIL_DOC)
    }

    pub fn submit() -> Self {
        Self::new(PDF_DOCUMENT_EVENT_SUBMIT)
    }

    pub fn exec_menu_item() -> Self {
        Self::new(PDF_DOCUMENT_EVENT_EXEC_MENU_ITEM)
    }
}

// ============================================================================
// Alert Event
// ============================================================================

/// Alert dialog event details
#[derive(Debug, Clone)]
pub struct AlertEvent {
    /// Document handle
    pub doc: DocumentHandle,
    /// Alert message
    pub message: String,
    /// Icon type (PDF_ALERT_ICON_*)
    pub icon_type: i32,
    /// Button group type (PDF_ALERT_BUTTON_GROUP_*)
    pub button_group_type: i32,
    /// Dialog title
    pub title: String,
    /// Whether to show checkbox
    pub has_check_box: bool,
    /// Checkbox message
    pub check_box_message: String,
    /// Initial checkbox state
    pub initially_checked: bool,
    /// Final checkbox state (set by app)
    pub finally_checked: bool,
    /// Button pressed (set by app)
    pub button_pressed: i32,
}

impl Default for AlertEvent {
    fn default() -> Self {
        Self::new()
    }
}

impl AlertEvent {
    pub fn new() -> Self {
        Self {
            doc: 0,
            message: String::new(),
            icon_type: PDF_ALERT_ICON_STATUS,
            button_group_type: PDF_ALERT_BUTTON_GROUP_OK,
            title: String::new(),
            has_check_box: false,
            check_box_message: String::new(),
            initially_checked: false,
            finally_checked: false,
            button_pressed: PDF_ALERT_BUTTON_NONE,
        }
    }

    pub fn with_message(mut self, message: &str) -> Self {
        self.message = message.to_string();
        self
    }

    pub fn with_title(mut self, title: &str) -> Self {
        self.title = title.to_string();
        self
    }

    pub fn with_icon(mut self, icon: i32) -> Self {
        self.icon_type = icon;
        self
    }

    pub fn with_buttons(mut self, buttons: i32) -> Self {
        self.button_group_type = buttons;
        self
    }

    pub fn with_checkbox(mut self, message: &str, initially_checked: bool) -> Self {
        self.has_check_box = true;
        self.check_box_message = message.to_string();
        self.initially_checked = initially_checked;
        self
    }
}

// ============================================================================
// Launch URL Event
// ============================================================================

/// Launch URL event details
#[derive(Debug, Clone)]
pub struct LaunchUrlEvent {
    /// URL to open
    pub url: String,
    /// Whether to open in new frame
    pub new_frame: bool,
}

impl Default for LaunchUrlEvent {
    fn default() -> Self {
        Self::new()
    }
}

impl LaunchUrlEvent {
    pub fn new() -> Self {
        Self {
            url: String::new(),
            new_frame: false,
        }
    }

    pub fn with_url(url: &str, new_frame: bool) -> Self {
        Self {
            url: url.to_string(),
            new_frame,
        }
    }
}

// ============================================================================
// Mail Document Event
// ============================================================================

/// Mail document event details
#[derive(Debug, Clone)]
pub struct MailDocEvent {
    /// Whether to ask user for details
    pub ask_user: bool,
    /// To address
    pub to: String,
    /// CC addresses
    pub cc: String,
    /// BCC addresses
    pub bcc: String,
    /// Email subject
    pub subject: String,
    /// Email message body
    pub message: String,
}

impl Default for MailDocEvent {
    fn default() -> Self {
        Self::new()
    }
}

impl MailDocEvent {
    pub fn new() -> Self {
        Self {
            ask_user: true,
            to: String::new(),
            cc: String::new(),
            bcc: String::new(),
            subject: String::new(),
            message: String::new(),
        }
    }

    pub fn with_recipient(mut self, to: &str) -> Self {
        self.to = to.to_string();
        self
    }

    pub fn with_subject(mut self, subject: &str) -> Self {
        self.subject = subject.to_string();
        self
    }

    pub fn with_message(mut self, message: &str) -> Self {
        self.message = message.to_string();
        self
    }
}

// ============================================================================
// Event Handler Context
// ============================================================================

/// Callback type for document events
pub type DocEventCallback =
    extern "C" fn(ctx: ContextHandle, doc: DocumentHandle, evt: *const DocEvent, data: *mut c_void);

/// Callback type for freeing event data
pub type FreeEventDataCallback = extern "C" fn(ctx: ContextHandle, data: *mut c_void);

/// Event handler context
pub struct EventHandler {
    /// Document handle
    pub document: DocumentHandle,
    /// Event callback
    pub event_cb: Option<DocEventCallback>,
    /// Free data callback
    pub free_cb: Option<FreeEventDataCallback>,
    /// User data
    pub user_data: *mut c_void,
    /// Pending events
    pub pending_events: Vec<DocEvent>,
    /// Alert events
    pub alert_events: Vec<AlertEvent>,
    /// Launch URL events
    pub launch_url_events: Vec<LaunchUrlEvent>,
    /// Mail doc events
    pub mail_doc_events: Vec<MailDocEvent>,
    /// Menu items executed
    pub menu_items: Vec<String>,
}

unsafe impl Send for EventHandler {}
unsafe impl Sync for EventHandler {}

impl Default for EventHandler {
    fn default() -> Self {
        Self::new(0)
    }
}

impl EventHandler {
    pub fn new(document: DocumentHandle) -> Self {
        Self {
            document,
            event_cb: None,
            free_cb: None,
            user_data: ptr::null_mut(),
            pending_events: Vec::new(),
            alert_events: Vec::new(),
            launch_url_events: Vec::new(),
            mail_doc_events: Vec::new(),
            menu_items: Vec::new(),
        }
    }

    pub fn set_callback(
        &mut self,
        event_cb: Option<DocEventCallback>,
        free_cb: Option<FreeEventDataCallback>,
        data: *mut c_void,
    ) {
        self.event_cb = event_cb;
        self.free_cb = free_cb;
        self.user_data = data;
    }

    pub fn issue_event(&mut self, event: DocEvent) {
        self.pending_events.push(event);
    }
}

// ============================================================================
// Global Handle Store
// ============================================================================

pub static EVENT_HANDLERS: LazyLock<HandleStore<EventHandler>> = LazyLock::new(HandleStore::new);

// ============================================================================
// FFI Functions - Event Handler Setup
// ============================================================================

/// Create a new event handler for a document.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_new_event_handler(_ctx: ContextHandle, doc: DocumentHandle) -> Handle {
    let handler = EventHandler::new(doc);
    EVENT_HANDLERS.insert(handler)
}

/// Drop an event handler.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_drop_event_handler(_ctx: ContextHandle, handler: Handle) {
    EVENT_HANDLERS.remove(handler);
}

/// Set the document event callback.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_set_doc_event_callback(
    _ctx: ContextHandle,
    handler: Handle,
    event_cb: Option<DocEventCallback>,
    free_cb: Option<FreeEventDataCallback>,
    data: *mut c_void,
) {
    if let Some(h) = EVENT_HANDLERS.get(handler) {
        let mut h = h.lock().unwrap();
        h.set_callback(event_cb, free_cb, data);
    }
}

/// Get the event callback data.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_get_doc_event_callback_data(
    _ctx: ContextHandle,
    handler: Handle,
) -> *mut c_void {
    if let Some(h) = EVENT_HANDLERS.get(handler) {
        let h = h.lock().unwrap();
        return h.user_data;
    }
    ptr::null_mut()
}

// ============================================================================
// FFI Functions - Alert Events
// ============================================================================

/// Create a new alert event.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_new_alert_event() -> *mut AlertEvent {
    Box::into_raw(Box::new(AlertEvent::new()))
}

/// Drop an alert event.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_drop_alert_event(evt: *mut AlertEvent) {
    if !evt.is_null() {
        unsafe {
            drop(Box::from_raw(evt));
        }
    }
}

/// Set alert event message.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_alert_set_message(evt: *mut AlertEvent, message: *const c_char) {
    if evt.is_null() || message.is_null() {
        return;
    }
    unsafe {
        let msg = CStr::from_ptr(message).to_string_lossy().to_string();
        (*evt).message = msg;
    }
}

/// Set alert event title.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_alert_set_title(evt: *mut AlertEvent, title: *const c_char) {
    if evt.is_null() || title.is_null() {
        return;
    }
    unsafe {
        let t = CStr::from_ptr(title).to_string_lossy().to_string();
        (*evt).title = t;
    }
}

/// Set alert event icon type.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_alert_set_icon(evt: *mut AlertEvent, icon_type: i32) {
    if evt.is_null() {
        return;
    }
    unsafe {
        (*evt).icon_type = icon_type;
    }
}

/// Set alert event button group.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_alert_set_button_group(evt: *mut AlertEvent, button_group: i32) {
    if evt.is_null() {
        return;
    }
    unsafe {
        (*evt).button_group_type = button_group;
    }
}

/// Get the button pressed in response to an alert.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_alert_get_button_pressed(evt: *const AlertEvent) -> i32 {
    if evt.is_null() {
        return PDF_ALERT_BUTTON_NONE;
    }
    unsafe { (*evt).button_pressed }
}

/// Set the button pressed response.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_alert_set_button_pressed(evt: *mut AlertEvent, button: i32) {
    if evt.is_null() {
        return;
    }
    unsafe {
        (*evt).button_pressed = button;
    }
}

/// Issue an alert event.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_event_issue_alert(
    _ctx: ContextHandle,
    handler: Handle,
    evt: *const AlertEvent,
) {
    if evt.is_null() {
        return;
    }
    if let Some(h) = EVENT_HANDLERS.get(handler) {
        let mut h = h.lock().unwrap();
        unsafe {
            h.alert_events.push((*evt).clone());
        }
        h.issue_event(DocEvent::alert());
    }
}

// ============================================================================
// FFI Functions - Print Events
// ============================================================================

/// Issue a print event.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_event_issue_print(_ctx: ContextHandle, handler: Handle) {
    if let Some(h) = EVENT_HANDLERS.get(handler) {
        let mut h = h.lock().unwrap();
        h.issue_event(DocEvent::print());
    }
}

// ============================================================================
// FFI Functions - Launch URL Events
// ============================================================================

/// Issue a launch URL event.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_event_issue_launch_url(
    _ctx: ContextHandle,
    handler: Handle,
    url: *const c_char,
    new_frame: i32,
) {
    if url.is_null() {
        return;
    }
    if let Some(h) = EVENT_HANDLERS.get(handler) {
        let mut h = h.lock().unwrap();
        unsafe {
            let u = CStr::from_ptr(url).to_string_lossy().to_string();
            h.launch_url_events
                .push(LaunchUrlEvent::with_url(&u, new_frame != 0));
        }
        h.issue_event(DocEvent::launch_url());
    }
}

/// Access launch URL event details.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_access_launch_url_event(
    _ctx: ContextHandle,
    handler: Handle,
    index: i32,
    url_out: *mut *mut c_char,
    new_frame_out: *mut i32,
) -> i32 {
    if let Some(h) = EVENT_HANDLERS.get(handler) {
        let h = h.lock().unwrap();
        if let Some(evt) = h.launch_url_events.get(index as usize) {
            if !url_out.is_null() {
                if let Ok(cstr) = CString::new(evt.url.clone()) {
                    unsafe {
                        *url_out = cstr.into_raw();
                    }
                }
            }
            if !new_frame_out.is_null() {
                unsafe {
                    *new_frame_out = if evt.new_frame { 1 } else { 0 };
                }
            }
            return 1;
        }
    }
    0
}

// ============================================================================
// FFI Functions - Mail Document Events
// ============================================================================

/// Create a new mail document event.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_new_mail_doc_event() -> *mut MailDocEvent {
    Box::into_raw(Box::new(MailDocEvent::new()))
}

/// Drop a mail document event.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_drop_mail_doc_event(evt: *mut MailDocEvent) {
    if !evt.is_null() {
        unsafe {
            drop(Box::from_raw(evt));
        }
    }
}

/// Set mail document recipient.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_mail_doc_set_to(evt: *mut MailDocEvent, to: *const c_char) {
    if evt.is_null() || to.is_null() {
        return;
    }
    unsafe {
        let t = CStr::from_ptr(to).to_string_lossy().to_string();
        (*evt).to = t;
    }
}

/// Set mail document subject.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_mail_doc_set_subject(evt: *mut MailDocEvent, subject: *const c_char) {
    if evt.is_null() || subject.is_null() {
        return;
    }
    unsafe {
        let s = CStr::from_ptr(subject).to_string_lossy().to_string();
        (*evt).subject = s;
    }
}

/// Issue a mail document event.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_event_issue_mail_doc(
    _ctx: ContextHandle,
    handler: Handle,
    evt: *const MailDocEvent,
) {
    if evt.is_null() {
        return;
    }
    if let Some(h) = EVENT_HANDLERS.get(handler) {
        let mut h = h.lock().unwrap();
        unsafe {
            h.mail_doc_events.push((*evt).clone());
        }
        h.issue_event(DocEvent::mail_doc());
    }
}

// ============================================================================
// FFI Functions - Menu Item Events
// ============================================================================

/// Issue an execute menu item event.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_event_issue_exec_menu_item(
    _ctx: ContextHandle,
    handler: Handle,
    item: *const c_char,
) {
    if item.is_null() {
        return;
    }
    if let Some(h) = EVENT_HANDLERS.get(handler) {
        let mut h = h.lock().unwrap();
        unsafe {
            let i = CStr::from_ptr(item).to_string_lossy().to_string();
            h.menu_items.push(i);
        }
        h.issue_event(DocEvent::exec_menu_item());
    }
}

/// Access executed menu item.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_access_exec_menu_item_event(
    _ctx: ContextHandle,
    handler: Handle,
    index: i32,
) -> *mut c_char {
    if let Some(h) = EVENT_HANDLERS.get(handler) {
        let h = h.lock().unwrap();
        if let Some(item) = h.menu_items.get(index as usize) {
            if let Ok(cstr) = CString::new(item.clone()) {
                return cstr.into_raw();
            }
        }
    }
    ptr::null_mut()
}

// ============================================================================
// FFI Functions - Event Query
// ============================================================================

/// Get number of pending events.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_count_pending_events(_ctx: ContextHandle, handler: Handle) -> i32 {
    if let Some(h) = EVENT_HANDLERS.get(handler) {
        let h = h.lock().unwrap();
        return h.pending_events.len() as i32;
    }
    0
}

/// Get pending event type at index.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_get_pending_event_type(
    _ctx: ContextHandle,
    handler: Handle,
    index: i32,
) -> i32 {
    if let Some(h) = EVENT_HANDLERS.get(handler) {
        let h = h.lock().unwrap();
        if let Some(evt) = h.pending_events.get(index as usize) {
            return evt.event_type;
        }
    }
    -1
}

/// Clear all pending events.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_clear_pending_events(_ctx: ContextHandle, handler: Handle) {
    if let Some(h) = EVENT_HANDLERS.get(handler) {
        let mut h = h.lock().unwrap();
        h.pending_events.clear();
        h.alert_events.clear();
        h.launch_url_events.clear();
        h.mail_doc_events.clear();
        h.menu_items.clear();
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_type_constants() {
        assert_eq!(PDF_DOCUMENT_EVENT_ALERT, 0);
        assert_eq!(PDF_DOCUMENT_EVENT_PRINT, 1);
        assert_eq!(PDF_DOCUMENT_EVENT_LAUNCH_URL, 2);
        assert_eq!(PDF_DOCUMENT_EVENT_MAIL_DOC, 3);
        assert_eq!(PDF_DOCUMENT_EVENT_SUBMIT, 4);
        assert_eq!(PDF_DOCUMENT_EVENT_EXEC_MENU_ITEM, 5);
    }

    #[test]
    fn test_alert_icon_constants() {
        assert_eq!(PDF_ALERT_ICON_ERROR, 0);
        assert_eq!(PDF_ALERT_ICON_WARNING, 1);
        assert_eq!(PDF_ALERT_ICON_QUESTION, 2);
        assert_eq!(PDF_ALERT_ICON_STATUS, 3);
    }

    #[test]
    fn test_alert_button_group_constants() {
        assert_eq!(PDF_ALERT_BUTTON_GROUP_OK, 0);
        assert_eq!(PDF_ALERT_BUTTON_GROUP_OK_CANCEL, 1);
        assert_eq!(PDF_ALERT_BUTTON_GROUP_YES_NO, 2);
        assert_eq!(PDF_ALERT_BUTTON_GROUP_YES_NO_CANCEL, 3);
    }

    #[test]
    fn test_alert_button_constants() {
        assert_eq!(PDF_ALERT_BUTTON_NONE, 0);
        assert_eq!(PDF_ALERT_BUTTON_OK, 1);
        assert_eq!(PDF_ALERT_BUTTON_CANCEL, 2);
        assert_eq!(PDF_ALERT_BUTTON_NO, 3);
        assert_eq!(PDF_ALERT_BUTTON_YES, 4);
    }

    #[test]
    fn test_doc_event() {
        let evt = DocEvent::alert();
        assert_eq!(evt.event_type, PDF_DOCUMENT_EVENT_ALERT);

        let evt = DocEvent::print();
        assert_eq!(evt.event_type, PDF_DOCUMENT_EVENT_PRINT);

        let evt = DocEvent::launch_url();
        assert_eq!(evt.event_type, PDF_DOCUMENT_EVENT_LAUNCH_URL);
    }

    #[test]
    fn test_alert_event() {
        let evt = AlertEvent::new()
            .with_message("Test message")
            .with_title("Test Title")
            .with_icon(PDF_ALERT_ICON_WARNING)
            .with_buttons(PDF_ALERT_BUTTON_GROUP_YES_NO);

        assert_eq!(evt.message, "Test message");
        assert_eq!(evt.title, "Test Title");
        assert_eq!(evt.icon_type, PDF_ALERT_ICON_WARNING);
        assert_eq!(evt.button_group_type, PDF_ALERT_BUTTON_GROUP_YES_NO);
    }

    #[test]
    fn test_alert_event_checkbox() {
        let evt = AlertEvent::new().with_checkbox("Don't show again", true);

        assert!(evt.has_check_box);
        assert_eq!(evt.check_box_message, "Don't show again");
        assert!(evt.initially_checked);
    }

    #[test]
    fn test_launch_url_event() {
        let evt = LaunchUrlEvent::with_url("https://example.com", true);
        assert_eq!(evt.url, "https://example.com");
        assert!(evt.new_frame);
    }

    #[test]
    fn test_mail_doc_event() {
        let evt = MailDocEvent::new()
            .with_recipient("user@example.com")
            .with_subject("Test Subject")
            .with_message("Test body");

        assert_eq!(evt.to, "user@example.com");
        assert_eq!(evt.subject, "Test Subject");
        assert_eq!(evt.message, "Test body");
    }

    #[test]
    fn test_event_handler() {
        let mut handler = EventHandler::new(1);
        assert_eq!(handler.document, 1);
        assert!(handler.pending_events.is_empty());

        handler.issue_event(DocEvent::print());
        assert_eq!(handler.pending_events.len(), 1);
    }

    #[test]
    fn test_ffi_event_handler() {
        let ctx = 0;
        let doc = 1;

        let handler = pdf_new_event_handler(ctx, doc);
        assert!(handler > 0);

        assert_eq!(pdf_count_pending_events(ctx, handler), 0);

        pdf_event_issue_print(ctx, handler);
        assert_eq!(pdf_count_pending_events(ctx, handler), 1);
        assert_eq!(
            pdf_get_pending_event_type(ctx, handler, 0),
            PDF_DOCUMENT_EVENT_PRINT
        );

        pdf_clear_pending_events(ctx, handler);
        assert_eq!(pdf_count_pending_events(ctx, handler), 0);

        pdf_drop_event_handler(ctx, handler);
    }

    #[test]
    fn test_ffi_alert_event() {
        let evt = pdf_new_alert_event();
        assert!(!evt.is_null());

        let msg = CString::new("Test").unwrap();
        pdf_alert_set_message(evt, msg.as_ptr());

        let title = CString::new("Title").unwrap();
        pdf_alert_set_title(evt, title.as_ptr());

        pdf_alert_set_icon(evt, PDF_ALERT_ICON_ERROR);
        pdf_alert_set_button_group(evt, PDF_ALERT_BUTTON_GROUP_OK_CANCEL);
        pdf_alert_set_button_pressed(evt, PDF_ALERT_BUTTON_OK);

        assert_eq!(pdf_alert_get_button_pressed(evt), PDF_ALERT_BUTTON_OK);

        pdf_drop_alert_event(evt);
    }

    #[test]
    fn test_ffi_launch_url() {
        let ctx = 0;
        let handler = pdf_new_event_handler(ctx, 1);

        let url = CString::new("https://example.com").unwrap();
        pdf_event_issue_launch_url(ctx, handler, url.as_ptr(), 1);

        assert_eq!(pdf_count_pending_events(ctx, handler), 1);
        assert_eq!(
            pdf_get_pending_event_type(ctx, handler, 0),
            PDF_DOCUMENT_EVENT_LAUNCH_URL
        );

        pdf_drop_event_handler(ctx, handler);
    }

    #[test]
    fn test_ffi_menu_item() {
        let ctx = 0;
        let handler = pdf_new_event_handler(ctx, 1);

        let item = CString::new("File:Print").unwrap();
        pdf_event_issue_exec_menu_item(ctx, handler, item.as_ptr());

        assert_eq!(pdf_count_pending_events(ctx, handler), 1);

        let result = pdf_access_exec_menu_item_event(ctx, handler, 0);
        assert!(!result.is_null());
        unsafe {
            let s = CStr::from_ptr(result).to_string_lossy();
            assert_eq!(s, "File:Print");
            drop(CString::from_raw(result));
        }

        pdf_drop_event_handler(ctx, handler);
    }
}
