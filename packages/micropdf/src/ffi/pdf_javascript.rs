//! FFI bindings for pdf_javascript (JavaScript Support)
//!
//! Provides JavaScript scripting support for PDF forms and actions:
//! - Enable/disable JavaScript in documents
//! - Execute JavaScript code
//! - Handle form field events (validation, keystroke, etc.)
//! - Event initialization and result retrieval

use crate::ffi::{Handle, HandleStore};
use std::collections::HashMap;
use std::ffi::{CStr, CString, c_char};
use std::ptr;
use std::sync::{LazyLock, Mutex};

// ============================================================================
// Types and Structures
// ============================================================================

/// JavaScript event types
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum JsEventType {
    #[default]
    None = 0,
    /// Field validation event
    Validate = 1,
    /// Field calculation event
    Calculate = 2,
    /// Field format event
    Format = 3,
    /// Keystroke event
    Keystroke = 4,
    /// Mouse enter event
    MouseEnter = 5,
    /// Mouse exit event
    MouseExit = 6,
    /// Field focus event
    Focus = 7,
    /// Field blur event
    Blur = 8,
    /// Document open event
    DocOpen = 9,
    /// Document close event
    DocClose = 10,
    /// Page open event
    PageOpen = 11,
    /// Page close event
    PageClose = 12,
}

impl From<i32> for JsEventType {
    fn from(value: i32) -> Self {
        match value {
            1 => JsEventType::Validate,
            2 => JsEventType::Calculate,
            3 => JsEventType::Format,
            4 => JsEventType::Keystroke,
            5 => JsEventType::MouseEnter,
            6 => JsEventType::MouseExit,
            7 => JsEventType::Focus,
            8 => JsEventType::Blur,
            9 => JsEventType::DocOpen,
            10 => JsEventType::DocClose,
            11 => JsEventType::PageOpen,
            12 => JsEventType::PageClose,
            _ => JsEventType::None,
        }
    }
}

/// Keystroke event data
#[repr(C)]
#[derive(Debug, Clone, Default)]
pub struct KeystrokeEvent {
    /// The change string (characters being typed)
    pub change: String,
    /// Selection start position
    pub sel_start: i32,
    /// Selection end position
    pub sel_end: i32,
    /// Whether shift key is pressed
    pub shift: bool,
    /// Whether the change should be rejected
    pub rc: bool,
    /// The current field value
    pub value: String,
    /// Whether to commit the change
    pub will_commit: bool,
}

/// JavaScript event context
#[derive(Debug, Clone, Default)]
pub struct JsEvent {
    /// Event type
    pub event_type: JsEventType,
    /// Target field/object handle
    pub target: Handle,
    /// Current value
    pub value: String,
    /// New value (after event processing)
    pub new_value: Option<String>,
    /// Whether event result is valid
    pub rc: bool,
    /// Whether to commit changes
    pub will_commit: bool,
    /// Keystroke-specific data
    pub keystroke: Option<KeystrokeEvent>,
}

/// JavaScript context for a document
#[derive(Debug, Default)]
pub struct PdfJs {
    /// Whether JavaScript is enabled
    pub enabled: bool,
    /// Document handle this JS context belongs to
    pub document: Handle,
    /// Global variables
    pub globals: HashMap<String, String>,
    /// Current event
    pub current_event: Option<JsEvent>,
    /// Registered scripts (name -> code)
    pub scripts: HashMap<String, String>,
    /// Console log output
    pub console_log: Vec<String>,
    /// Last error message
    pub last_error: Option<String>,
}

/// Document JavaScript state
#[derive(Debug, Default)]
struct DocumentJsState {
    js_enabled: bool,
    js_handle: Option<Handle>,
}

// ============================================================================
// Global State
// ============================================================================

pub static PDF_JS_CONTEXTS: LazyLock<HandleStore<PdfJs>> = LazyLock::new(HandleStore::new);

// Track JS state per document
static DOCUMENT_JS_STATE: LazyLock<Mutex<HashMap<Handle, DocumentJsState>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

// ============================================================================
// JavaScript Engine Simulation
// ============================================================================

impl PdfJs {
    /// Create a new JavaScript context for a document
    pub fn new(document: Handle) -> Self {
        Self {
            enabled: true,
            document,
            globals: HashMap::new(),
            current_event: None,
            scripts: HashMap::new(),
            console_log: Vec::new(),
            last_error: None,
        }
    }

    /// Execute a simple JavaScript expression
    /// This is a basic simulator that handles common PDF form operations
    pub fn execute(&mut self, _name: &str, code: &str) -> Option<String> {
        self.last_error = None;

        // Simple expression evaluator for common PDF JavaScript patterns
        let code = code.trim();

        // Handle console.log
        if code.starts_with("console.log(") {
            if let Some(content) = code
                .strip_prefix("console.log(")
                .and_then(|s| s.strip_suffix(')'))
            {
                let msg = self.evaluate_expression(content);
                self.console_log.push(msg.clone());
                return Some(msg);
            }
        }

        // Handle variable assignment
        if code.contains('=') && !code.contains("==") {
            let parts: Vec<&str> = code.splitn(2, '=').collect();
            if parts.len() == 2 {
                let var_name = parts[0].trim();
                let value = self.evaluate_expression(parts[1].trim());
                self.globals.insert(var_name.to_string(), value.clone());
                return Some(value);
            }
        }

        // Handle event.value access
        if code.contains("event.value") {
            if let Some(ref event) = self.current_event {
                return Some(event.value.clone());
            }
        }

        // Handle event.rc assignment
        if code.starts_with("event.rc") {
            if let Some(ref mut event) = self.current_event {
                if code.contains("true") {
                    event.rc = true;
                } else if code.contains("false") {
                    event.rc = false;
                }
                return Some(event.rc.to_string());
            }
        }

        // Handle simple arithmetic
        if let Some(result) = self.evaluate_arithmetic(code) {
            return Some(result);
        }

        // Handle string operations
        if code.starts_with('"') || code.starts_with('\'') {
            return Some(self.evaluate_expression(code));
        }

        // Variable lookup
        if let Some(value) = self.globals.get(code) {
            return Some(value.clone());
        }

        // Return undefined for unknown expressions
        Some("undefined".to_string())
    }

    /// Evaluate a simple expression
    fn evaluate_expression(&self, expr: &str) -> String {
        let expr = expr.trim();

        // String literal
        if (expr.starts_with('"') && expr.ends_with('"'))
            || (expr.starts_with('\'') && expr.ends_with('\''))
        {
            return expr[1..expr.len() - 1].to_string();
        }

        // Number literal
        if let Ok(n) = expr.parse::<f64>() {
            return n.to_string();
        }

        // Boolean literal
        if expr == "true" {
            return "true".to_string();
        }
        if expr == "false" {
            return "false".to_string();
        }

        // Variable lookup
        if let Some(value) = self.globals.get(expr) {
            return value.clone();
        }

        expr.to_string()
    }

    /// Evaluate simple arithmetic expressions
    fn evaluate_arithmetic(&self, expr: &str) -> Option<String> {
        let expr = expr.trim();

        // Simple binary operations
        for op in ['+', '-', '*', '/'] {
            if let Some(pos) = expr.rfind(op) {
                if pos > 0 {
                    let left = expr[..pos].trim();
                    let right = expr[pos + 1..].trim();

                    let left_val: f64 = self.evaluate_expression(left).parse().ok()?;
                    let right_val: f64 = self.evaluate_expression(right).parse().ok()?;

                    let result = match op {
                        '+' => left_val + right_val,
                        '-' => left_val - right_val,
                        '*' => left_val * right_val,
                        '/' => {
                            if right_val == 0.0 {
                                return Some("Infinity".to_string());
                            }
                            left_val / right_val
                        }
                        _ => return None,
                    };

                    return Some(result.to_string());
                }
            }
        }

        None
    }

    /// Initialize a field event
    pub fn init_event(&mut self, target: Handle, value: &str, will_commit: bool) {
        self.current_event = Some(JsEvent {
            event_type: JsEventType::Validate,
            target,
            value: value.to_string(),
            new_value: None,
            rc: true, // Default to accepting the event
            will_commit,
            keystroke: None,
        });
    }

    /// Initialize a keystroke event
    pub fn init_keystroke_event(&mut self, target: Handle, keystroke: KeystrokeEvent) {
        self.current_event = Some(JsEvent {
            event_type: JsEventType::Keystroke,
            target,
            value: keystroke.value.clone(),
            new_value: None,
            rc: true,
            will_commit: keystroke.will_commit,
            keystroke: Some(keystroke),
        });
    }

    /// Get the event result (true = accepted, false = rejected)
    pub fn get_event_result(&self) -> bool {
        self.current_event.as_ref().map_or(true, |e| e.rc)
    }

    /// Get the event value
    pub fn get_event_value(&self) -> Option<String> {
        self.current_event
            .as_ref()
            .map(|e| e.new_value.clone().unwrap_or_else(|| e.value.clone()))
    }
}

// ============================================================================
// FFI Functions - Enable/Disable JavaScript
// ============================================================================

/// Enable JavaScript for a document
#[unsafe(no_mangle)]
pub extern "C" fn pdf_enable_js(_ctx: Handle, doc: Handle) {
    let mut states = DOCUMENT_JS_STATE.lock().unwrap();
    let state = states.entry(doc).or_insert_with(DocumentJsState::default);

    if state.js_handle.is_none() {
        let js = PdfJs::new(doc);
        let handle = PDF_JS_CONTEXTS.insert(js);
        state.js_handle = Some(handle);
    }

    state.js_enabled = true;

    // Also enable in the JS context
    if let Some(handle) = state.js_handle {
        if let Some(js_arc) = PDF_JS_CONTEXTS.get(handle) {
            js_arc.lock().unwrap().enabled = true;
        }
    }
}

/// Disable JavaScript for a document
#[unsafe(no_mangle)]
pub extern "C" fn pdf_disable_js(_ctx: Handle, doc: Handle) {
    let mut states = DOCUMENT_JS_STATE.lock().unwrap();
    if let Some(state) = states.get_mut(&doc) {
        state.js_enabled = false;

        if let Some(handle) = state.js_handle {
            if let Some(js_arc) = PDF_JS_CONTEXTS.get(handle) {
                js_arc.lock().unwrap().enabled = false;
            }
        }
    }
}

/// Check if JavaScript is supported/enabled for a document
/// Returns 1 if supported, 0 if not
#[unsafe(no_mangle)]
pub extern "C" fn pdf_js_supported(_ctx: Handle, doc: Handle) -> i32 {
    let states = DOCUMENT_JS_STATE.lock().unwrap();
    if let Some(state) = states.get(&doc) {
        if state.js_enabled {
            return 1;
        }
    }
    0
}

/// Drop (free) a JavaScript context
#[unsafe(no_mangle)]
pub extern "C" fn pdf_drop_js(_ctx: Handle, js: Handle) {
    PDF_JS_CONTEXTS.remove(js);
}

/// Get the JavaScript context for a document
/// Creates one if it doesn't exist
#[unsafe(no_mangle)]
pub extern "C" fn pdf_get_js(_ctx: Handle, doc: Handle) -> Handle {
    let mut states = DOCUMENT_JS_STATE.lock().unwrap();
    let state = states.entry(doc).or_insert_with(DocumentJsState::default);

    if let Some(handle) = state.js_handle {
        return handle;
    }

    // Create a new JS context
    let js = PdfJs::new(doc);
    let handle = PDF_JS_CONTEXTS.insert(js);
    state.js_handle = Some(handle);
    handle
}

// ============================================================================
// FFI Functions - Event Handling
// ============================================================================

/// Initialize a JavaScript event
#[unsafe(no_mangle)]
pub extern "C" fn pdf_js_event_init(
    js: Handle,
    target: Handle,
    value: *const c_char,
    will_commit: i32,
) {
    let js_arc = match PDF_JS_CONTEXTS.get(js) {
        Some(j) => j,
        None => return,
    };

    let value_str = if value.is_null() {
        ""
    } else {
        unsafe { CStr::from_ptr(value) }.to_str().unwrap_or("")
    };

    let mut js_guard = js_arc.lock().unwrap();
    js_guard.init_event(target, value_str, will_commit != 0);
}

/// Get the result of a JavaScript event
/// Returns 1 if event was accepted, 0 if rejected
#[unsafe(no_mangle)]
pub extern "C" fn pdf_js_event_result(js: Handle) -> i32 {
    let js_arc = match PDF_JS_CONTEXTS.get(js) {
        Some(j) => j,
        None => return 1, // Default to accepted
    };

    let js_guard = js_arc.lock().unwrap();
    if js_guard.get_event_result() { 1 } else { 0 }
}

/// Get the result of a JavaScript event with validation
/// Returns 1 if valid, 0 if invalid
/// If valid and newvalue is not null, sets newvalue to the new value
#[unsafe(no_mangle)]
pub extern "C" fn pdf_js_event_result_validate(js: Handle, newvalue: *mut *mut c_char) -> i32 {
    let js_arc = match PDF_JS_CONTEXTS.get(js) {
        Some(j) => j,
        None => return 1,
    };

    let js_guard = js_arc.lock().unwrap();

    if !js_guard.get_event_result() {
        return 0;
    }

    if !newvalue.is_null() {
        if let Some(value) = js_guard.get_event_value() {
            if let Ok(cstr) = CString::new(value) {
                unsafe {
                    *newvalue = cstr.into_raw();
                }
            }
        }
    }

    1
}

/// Get the current event value
/// Returns a newly allocated string that must be freed by the caller
#[unsafe(no_mangle)]
pub extern "C" fn pdf_js_event_value(js: Handle) -> *mut c_char {
    let js_arc = match PDF_JS_CONTEXTS.get(js) {
        Some(j) => j,
        None => return ptr::null_mut(),
    };

    let js_guard = js_arc.lock().unwrap();

    if let Some(value) = js_guard.get_event_value() {
        if let Ok(cstr) = CString::new(value) {
            return cstr.into_raw();
        }
    }

    ptr::null_mut()
}

/// Initialize a keystroke event
#[unsafe(no_mangle)]
pub extern "C" fn pdf_js_event_init_keystroke(
    js: Handle,
    target: Handle,
    evt: *mut KeystrokeEvent,
) {
    let js_arc = match PDF_JS_CONTEXTS.get(js) {
        Some(j) => j,
        None => return,
    };

    if evt.is_null() {
        return;
    }

    let keystroke = unsafe { (*evt).clone() };

    let mut js_guard = js_arc.lock().unwrap();
    js_guard.init_keystroke_event(target, keystroke);
}

/// Get the result of a keystroke event
/// Returns 1 if accepted, 0 if rejected
/// Updates the event struct with the result
#[unsafe(no_mangle)]
pub extern "C" fn pdf_js_event_result_keystroke(js: Handle, evt: *mut KeystrokeEvent) -> i32 {
    let js_arc = match PDF_JS_CONTEXTS.get(js) {
        Some(j) => j,
        None => return 1,
    };

    let js_guard = js_arc.lock().unwrap();

    let result = js_guard.get_event_result();

    if !evt.is_null() {
        if let Some(ref event) = js_guard.current_event {
            if let Some(ref ks) = event.keystroke {
                unsafe {
                    (*evt).rc = result;
                    (*evt).change = ks.change.clone();
                }
            }
        }
    }

    if result { 1 } else { 0 }
}

// ============================================================================
// FFI Functions - Script Execution
// ============================================================================

/// Execute JavaScript code
/// name: optional name for the script (for debugging)
/// code: the JavaScript code to execute
/// result: if not null, receives the result of the execution
#[unsafe(no_mangle)]
pub extern "C" fn pdf_js_execute(
    js: Handle,
    name: *const c_char,
    code: *const c_char,
    result: *mut *mut c_char,
) {
    let js_arc = match PDF_JS_CONTEXTS.get(js) {
        Some(j) => j,
        None => return,
    };

    if code.is_null() {
        return;
    }

    let code_str = unsafe { CStr::from_ptr(code) }.to_str().unwrap_or("");

    let name_str = if name.is_null() {
        "anonymous"
    } else {
        unsafe { CStr::from_ptr(name) }
            .to_str()
            .unwrap_or("anonymous")
    };

    let mut js_guard = js_arc.lock().unwrap();

    if !js_guard.enabled {
        // JavaScript is disabled
        return;
    }

    let exec_result = js_guard.execute(name_str, code_str);

    if !result.is_null() {
        if let Some(res) = exec_result {
            if let Ok(cstr) = CString::new(res) {
                unsafe {
                    *result = cstr.into_raw();
                }
            }
        } else {
            unsafe {
                *result = ptr::null_mut();
            }
        }
    }
}

/// Free a string returned by pdf_js functions
#[unsafe(no_mangle)]
pub extern "C" fn pdf_js_free_string(_ctx: Handle, s: *mut c_char) {
    if !s.is_null() {
        unsafe {
            let _ = CString::from_raw(s);
        }
    }
}

// ============================================================================
// FFI Functions - Additional Utilities
// ============================================================================

/// Set a global variable in the JavaScript context
#[unsafe(no_mangle)]
pub extern "C" fn pdf_js_set_global(js: Handle, name: *const c_char, value: *const c_char) {
    let js_arc = match PDF_JS_CONTEXTS.get(js) {
        Some(j) => j,
        None => return,
    };

    if name.is_null() {
        return;
    }

    let name_str = unsafe { CStr::from_ptr(name) }
        .to_str()
        .unwrap_or("")
        .to_string();

    let value_str = if value.is_null() {
        "undefined".to_string()
    } else {
        unsafe { CStr::from_ptr(value) }
            .to_str()
            .unwrap_or("")
            .to_string()
    };

    let mut js_guard = js_arc.lock().unwrap();
    js_guard.globals.insert(name_str, value_str);
}

/// Get a global variable from the JavaScript context
/// Returns a newly allocated string that must be freed by the caller
#[unsafe(no_mangle)]
pub extern "C" fn pdf_js_get_global(js: Handle, name: *const c_char) -> *mut c_char {
    let js_arc = match PDF_JS_CONTEXTS.get(js) {
        Some(j) => j,
        None => return ptr::null_mut(),
    };

    if name.is_null() {
        return ptr::null_mut();
    }

    let name_str = unsafe { CStr::from_ptr(name) }.to_str().unwrap_or("");

    let js_guard = js_arc.lock().unwrap();

    if let Some(value) = js_guard.globals.get(name_str) {
        if let Ok(cstr) = CString::new(value.clone()) {
            return cstr.into_raw();
        }
    }

    ptr::null_mut()
}

/// Register a named script
#[unsafe(no_mangle)]
pub extern "C" fn pdf_js_register_script(js: Handle, name: *const c_char, code: *const c_char) {
    let js_arc = match PDF_JS_CONTEXTS.get(js) {
        Some(j) => j,
        None => return,
    };

    if name.is_null() || code.is_null() {
        return;
    }

    let name_str = unsafe { CStr::from_ptr(name) }
        .to_str()
        .unwrap_or("")
        .to_string();

    let code_str = unsafe { CStr::from_ptr(code) }
        .to_str()
        .unwrap_or("")
        .to_string();

    let mut js_guard = js_arc.lock().unwrap();
    js_guard.scripts.insert(name_str, code_str);
}

/// Execute a registered script by name
#[unsafe(no_mangle)]
pub extern "C" fn pdf_js_run_script(js: Handle, name: *const c_char, result: *mut *mut c_char) {
    let js_arc = match PDF_JS_CONTEXTS.get(js) {
        Some(j) => j,
        None => return,
    };

    if name.is_null() {
        return;
    }

    let name_str = unsafe { CStr::from_ptr(name) }.to_str().unwrap_or("");

    let mut js_guard = js_arc.lock().unwrap();

    let code = match js_guard.scripts.get(name_str) {
        Some(c) => c.clone(),
        None => return,
    };

    let exec_result = js_guard.execute(name_str, &code);

    if !result.is_null() {
        if let Some(res) = exec_result {
            if let Ok(cstr) = CString::new(res) {
                unsafe {
                    *result = cstr.into_raw();
                }
            }
        } else {
            unsafe {
                *result = ptr::null_mut();
            }
        }
    }
}

/// Get the console log output
/// Returns a newly allocated string with all console.log messages joined by newlines
#[unsafe(no_mangle)]
pub extern "C" fn pdf_js_get_console_log(js: Handle) -> *mut c_char {
    let js_arc = match PDF_JS_CONTEXTS.get(js) {
        Some(j) => j,
        None => return ptr::null_mut(),
    };

    let js_guard = js_arc.lock().unwrap();

    let log = js_guard.console_log.join("\n");
    if let Ok(cstr) = CString::new(log) {
        return cstr.into_raw();
    }

    ptr::null_mut()
}

/// Clear the console log
#[unsafe(no_mangle)]
pub extern "C" fn pdf_js_clear_console_log(js: Handle) {
    let js_arc = match PDF_JS_CONTEXTS.get(js) {
        Some(j) => j,
        None => return,
    };

    let mut js_guard = js_arc.lock().unwrap();
    js_guard.console_log.clear();
}

/// Get the last error message
/// Returns a newly allocated string or null if no error
#[unsafe(no_mangle)]
pub extern "C" fn pdf_js_get_last_error(js: Handle) -> *mut c_char {
    let js_arc = match PDF_JS_CONTEXTS.get(js) {
        Some(j) => j,
        None => return ptr::null_mut(),
    };

    let js_guard = js_arc.lock().unwrap();

    if let Some(ref error) = js_guard.last_error {
        if let Ok(cstr) = CString::new(error.clone()) {
            return cstr.into_raw();
        }
    }

    ptr::null_mut()
}

/// Clear the last error
#[unsafe(no_mangle)]
pub extern "C" fn pdf_js_clear_last_error(js: Handle) {
    let js_arc = match PDF_JS_CONTEXTS.get(js) {
        Some(j) => j,
        None => return,
    };

    let mut js_guard = js_arc.lock().unwrap();
    js_guard.last_error = None;
}

/// Set the event.rc value (result code)
#[unsafe(no_mangle)]
pub extern "C" fn pdf_js_event_set_rc(js: Handle, rc: i32) {
    let js_arc = match PDF_JS_CONTEXTS.get(js) {
        Some(j) => j,
        None => return,
    };

    let mut js_guard = js_arc.lock().unwrap();
    if let Some(ref mut event) = js_guard.current_event {
        event.rc = rc != 0;
    }
}

/// Set the event.value
#[unsafe(no_mangle)]
pub extern "C" fn pdf_js_event_set_value(js: Handle, value: *const c_char) {
    let js_arc = match PDF_JS_CONTEXTS.get(js) {
        Some(j) => j,
        None => return,
    };

    let value_str = if value.is_null() {
        String::new()
    } else {
        unsafe { CStr::from_ptr(value) }
            .to_str()
            .unwrap_or("")
            .to_string()
    };

    let mut js_guard = js_arc.lock().unwrap();
    if let Some(ref mut event) = js_guard.current_event {
        event.new_value = Some(value_str);
    }
}

/// Check if JavaScript is enabled in the context
#[unsafe(no_mangle)]
pub extern "C" fn pdf_js_is_enabled(js: Handle) -> i32 {
    let js_arc = match PDF_JS_CONTEXTS.get(js) {
        Some(j) => j,
        None => return 0,
    };

    let js_guard = js_arc.lock().unwrap();
    if js_guard.enabled { 1 } else { 0 }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enable_disable_js() {
        let ctx = 1;
        let doc = 100;

        // Initially not supported
        assert_eq!(pdf_js_supported(ctx, doc), 0);

        // Enable
        pdf_enable_js(ctx, doc);
        assert_eq!(pdf_js_supported(ctx, doc), 1);

        // Disable
        pdf_disable_js(ctx, doc);
        assert_eq!(pdf_js_supported(ctx, doc), 0);

        // Re-enable
        pdf_enable_js(ctx, doc);
        assert_eq!(pdf_js_supported(ctx, doc), 1);
    }

    #[test]
    fn test_get_js_context() {
        let ctx = 1;
        let doc = 200;

        let js = pdf_get_js(ctx, doc);
        assert!(js > 0);

        // Same document should return same handle
        let js2 = pdf_get_js(ctx, doc);
        assert_eq!(js, js2);

        pdf_drop_js(ctx, js);
    }

    #[test]
    fn test_event_init_result() {
        let ctx = 1;
        let doc = 300;

        pdf_enable_js(ctx, doc);
        let js = pdf_get_js(ctx, doc);

        let value = CString::new("test value").unwrap();
        pdf_js_event_init(js, 1, value.as_ptr(), 1);

        // Default result should be accepted
        assert_eq!(pdf_js_event_result(js), 1);

        // Get event value
        let result_value = pdf_js_event_value(js);
        assert!(!result_value.is_null());
        let result_str = unsafe { CStr::from_ptr(result_value) }.to_str().unwrap();
        assert_eq!(result_str, "test value");
        pdf_js_free_string(ctx, result_value);

        pdf_drop_js(ctx, js);
    }

    #[test]
    fn test_event_set_rc() {
        let ctx = 1;
        let doc = 400;

        pdf_enable_js(ctx, doc);
        let js = pdf_get_js(ctx, doc);

        let value = CString::new("").unwrap();
        pdf_js_event_init(js, 1, value.as_ptr(), 0);

        // Set rc to false (reject)
        pdf_js_event_set_rc(js, 0);
        assert_eq!(pdf_js_event_result(js), 0);

        // Set rc back to true (accept)
        pdf_js_event_set_rc(js, 1);
        assert_eq!(pdf_js_event_result(js), 1);

        pdf_drop_js(ctx, js);
    }

    #[test]
    fn test_execute_simple() {
        let ctx = 1;
        let doc = 500;

        pdf_enable_js(ctx, doc);
        let js = pdf_get_js(ctx, doc);

        // Test arithmetic
        let code = CString::new("2 + 3").unwrap();
        let mut result: *mut c_char = ptr::null_mut();
        pdf_js_execute(js, ptr::null(), code.as_ptr(), &mut result);
        assert!(!result.is_null());
        let result_str = unsafe { CStr::from_ptr(result) }.to_str().unwrap();
        assert_eq!(result_str, "5");
        pdf_js_free_string(ctx, result);

        pdf_drop_js(ctx, js);
    }

    #[test]
    fn test_execute_variable() {
        let ctx = 1;
        let doc = 600;

        pdf_enable_js(ctx, doc);
        let js = pdf_get_js(ctx, doc);

        // Set variable
        let code1 = CString::new("x = 42").unwrap();
        pdf_js_execute(js, ptr::null(), code1.as_ptr(), ptr::null_mut());

        // Read variable
        let code2 = CString::new("x").unwrap();
        let mut result: *mut c_char = ptr::null_mut();
        pdf_js_execute(js, ptr::null(), code2.as_ptr(), &mut result);
        assert!(!result.is_null());
        let result_str = unsafe { CStr::from_ptr(result) }.to_str().unwrap();
        assert_eq!(result_str, "42");
        pdf_js_free_string(ctx, result);

        pdf_drop_js(ctx, js);
    }

    #[test]
    fn test_globals() {
        let ctx = 1;
        let doc = 700;

        pdf_enable_js(ctx, doc);
        let js = pdf_get_js(ctx, doc);

        let name = CString::new("myVar").unwrap();
        let value = CString::new("hello world").unwrap();

        pdf_js_set_global(js, name.as_ptr(), value.as_ptr());

        let result = pdf_js_get_global(js, name.as_ptr());
        assert!(!result.is_null());
        let result_str = unsafe { CStr::from_ptr(result) }.to_str().unwrap();
        assert_eq!(result_str, "hello world");
        pdf_js_free_string(ctx, result);

        pdf_drop_js(ctx, js);
    }

    #[test]
    fn test_console_log() {
        let ctx = 1;
        let doc = 800;

        pdf_enable_js(ctx, doc);
        let js = pdf_get_js(ctx, doc);

        let code = CString::new("console.log(\"Hello, World!\")").unwrap();
        pdf_js_execute(js, ptr::null(), code.as_ptr(), ptr::null_mut());

        let log = pdf_js_get_console_log(js);
        assert!(!log.is_null());
        let log_str = unsafe { CStr::from_ptr(log) }.to_str().unwrap();
        assert!(log_str.contains("Hello, World!"));
        pdf_js_free_string(ctx, log);

        pdf_js_clear_console_log(js);
        let log2 = pdf_js_get_console_log(js);
        let log2_str = unsafe { CStr::from_ptr(log2) }.to_str().unwrap();
        assert!(log2_str.is_empty());
        pdf_js_free_string(ctx, log2);

        pdf_drop_js(ctx, js);
    }

    #[test]
    fn test_register_script() {
        let ctx = 1;
        let doc = 900;

        pdf_enable_js(ctx, doc);
        let js = pdf_get_js(ctx, doc);

        let name = CString::new("myScript").unwrap();
        let code = CString::new("10 * 5").unwrap();

        pdf_js_register_script(js, name.as_ptr(), code.as_ptr());

        let mut result: *mut c_char = ptr::null_mut();
        pdf_js_run_script(js, name.as_ptr(), &mut result);
        assert!(!result.is_null());
        let result_str = unsafe { CStr::from_ptr(result) }.to_str().unwrap();
        assert_eq!(result_str, "50");
        pdf_js_free_string(ctx, result);

        pdf_drop_js(ctx, js);
    }

    #[test]
    fn test_js_disabled() {
        let ctx = 1;
        let doc = 1000;

        pdf_enable_js(ctx, doc);
        let js = pdf_get_js(ctx, doc);
        pdf_disable_js(ctx, doc);

        // Execute should do nothing when disabled
        let code = CString::new("x = 123").unwrap();
        let mut result: *mut c_char = ptr::null_mut();
        pdf_js_execute(js, ptr::null(), code.as_ptr(), &mut result);
        assert!(result.is_null());

        assert_eq!(pdf_js_is_enabled(js), 0);

        pdf_drop_js(ctx, js);
    }

    #[test]
    fn test_event_result_validate() {
        let ctx = 1;
        let doc = 1100;

        pdf_enable_js(ctx, doc);
        let js = pdf_get_js(ctx, doc);

        let value = CString::new("original").unwrap();
        pdf_js_event_init(js, 1, value.as_ptr(), 1);

        // Set a new value
        let new_value = CString::new("modified").unwrap();
        pdf_js_event_set_value(js, new_value.as_ptr());

        let mut result_value: *mut c_char = ptr::null_mut();
        let rc = pdf_js_event_result_validate(js, &mut result_value);
        assert_eq!(rc, 1);
        assert!(!result_value.is_null());
        let result_str = unsafe { CStr::from_ptr(result_value) }.to_str().unwrap();
        assert_eq!(result_str, "modified");
        pdf_js_free_string(ctx, result_value);

        pdf_drop_js(ctx, js);
    }

    #[test]
    fn test_null_handling() {
        let ctx = 1;
        let doc = 1200;

        pdf_enable_js(ctx, doc);
        let js = pdf_get_js(ctx, doc);

        // Should not crash with null inputs
        pdf_js_event_init(js, 0, ptr::null(), 0);
        pdf_js_execute(js, ptr::null(), ptr::null(), ptr::null_mut());
        pdf_js_set_global(js, ptr::null(), ptr::null());
        pdf_js_register_script(js, ptr::null(), ptr::null());

        let result = pdf_js_get_global(js, ptr::null());
        assert!(result.is_null());

        pdf_drop_js(ctx, js);
    }
}
