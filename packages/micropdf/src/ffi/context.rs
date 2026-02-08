//! C FFI for context - MuPDF compatible
//! Simplified error handling without setjmp/longjmp

use super::{CONTEXTS, Handle};
use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::{CStr, CString, c_char, c_int, c_void};
use std::sync::{Arc, LazyLock, Mutex};

// Thread-local storage for cached error message to avoid memory leaks
thread_local! {
    static CACHED_ERROR_MSG: RefCell<Option<CString>> = const { RefCell::new(None) };
}

/// Error codes matching MuPDF fz_error_type
#[repr(C)]
pub enum FzErrorType {
    None = 0,
    Generic = 1,
    System = 2,      // Fatal out of memory or syscall error
    Library = 3,     // Error from third-party library
    Argument = 4,    // Invalid or out-of-range arguments
    Limit = 5,       // Resource or hard limits exceeded
    Unsupported = 6, // Unsupported feature
    Format = 7,      // Unrecoverable format errors
    Syntax = 8,      // Recoverable syntax errors
    TryLater = 9,    // Progressive loading signal
    Abort = 10,      // User requested abort
    Repaired = 11,   // PDF repair flag
}

/// Context error state
#[derive(Clone)]
struct ErrorState {
    code: c_int,
    message: String,
}

impl Default for ErrorState {
    fn default() -> Self {
        Self {
            code: FzErrorType::None as c_int,
            message: String::new(),
        }
    }
}

/// Context settings (ICC, AA level, etc.)
struct ContextSettings {
    icc_enabled: bool,
    aa_level: c_int,
}

impl Default for ContextSettings {
    fn default() -> Self {
        Self {
            icc_enabled: true, // ICC enabled by default
            aa_level: 8,       // 8-bit AA by default
        }
    }
}

/// Global context settings storage
static CONTEXT_SETTINGS: LazyLock<Mutex<HashMap<Handle, ContextSettings>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Resource store tracking (for memory management)
struct StoreState {
    allocated_bytes: usize,
    max_bytes: usize,
}

impl Default for StoreState {
    fn default() -> Self {
        Self {
            allocated_bytes: 0,
            max_bytes: 256 * 1024 * 1024, // 256 MB default
        }
    }
}

/// Global store state
static STORE_STATE: LazyLock<Mutex<StoreState>> =
    LazyLock::new(|| Mutex::new(StoreState::default()));

/// Internal context state
pub struct Context {
    /// Max store size in bytes
    max_store: usize,
    /// Error state
    error: Arc<Mutex<ErrorState>>,
    /// User data pointer
    user_data: *mut c_void,
    /// Warning callback
    warn_callback: Option<unsafe extern "C" fn(*mut c_void, *const c_char)>,
    /// Error callback
    error_callback: Option<unsafe extern "C" fn(*mut c_void, c_int, *const c_char)>,
}

// Context is Send since we protect mutable state with Arc<Mutex>
unsafe impl Send for Context {}
unsafe impl Sync for Context {}

impl Context {
    pub fn new(max_store: usize) -> Self {
        Self {
            max_store,
            error: Arc::new(Mutex::new(ErrorState::default())),
            user_data: std::ptr::null_mut(),
            warn_callback: None,
            error_callback: None,
        }
    }

    pub fn set_error(&self, code: c_int, message: String) {
        if let Ok(mut err) = self.error.lock() {
            err.code = code;
            err.message = message;
        }
    }

    pub fn get_error(&self) -> (c_int, String) {
        if let Ok(err) = self.error.lock() {
            (err.code, err.message.clone())
        } else {
            (
                FzErrorType::Generic as c_int,
                "Failed to lock error state".into(),
            )
        }
    }

    pub fn clear_error(&self) {
        if let Ok(mut err) = self.error.lock() {
            err.code = FzErrorType::None as c_int;
            err.message.clear();
        }
    }

    pub fn max_store(&self) -> usize {
        self.max_store
    }
}

/// Create a new context
///
/// # Safety
/// alloc and locks can be NULL for default behavior
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fz_new_context(
    _alloc: *const c_void, // fz_alloc_context* - ignored, we use Rust allocator
    _locks: *const c_void, // fz_locks_context* - ignored, we use Rust sync
    max_store: usize,
) -> Handle {
    CONTEXTS.insert(Context::new(max_store))
}

/// Clone a context (returns same handle with incremented refcount)
#[unsafe(no_mangle)]
pub extern "C" fn fz_clone_context(ctx: Handle) -> Handle {
    CONTEXTS.keep(ctx)
}

/// Drop a context
#[unsafe(no_mangle)]
pub extern "C" fn fz_drop_context(ctx: Handle) {
    let _ = CONTEXTS.remove(ctx);
}

/// Set user data on context
///
/// # Safety
/// Caller must ensure user pointer remains valid
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fz_set_user_context(ctx: Handle, user: *mut c_void) {
    if let Some(context) = CONTEXTS.get(ctx) {
        if let Ok(mut guard) = context.lock() {
            guard.user_data = user;
        }
    }
}

/// Get user data from context
#[unsafe(no_mangle)]
pub extern "C" fn fz_user_context(ctx: Handle) -> *mut c_void {
    if let Some(context) = CONTEXTS.get(ctx) {
        if let Ok(guard) = context.lock() {
            return guard.user_data;
        }
    }
    std::ptr::null_mut()
}

/// Throw an error (sets error state, does not longjmp)
///
/// # Safety
/// Caller must ensure fmt is a valid null-terminated C string
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fz_throw(ctx: Handle, errcode: c_int, fmt: *const c_char) {
    if fmt.is_null() {
        return;
    }

    // SAFETY: Caller guarantees fmt is a valid null-terminated C string
    let message = unsafe {
        if let Ok(c_str) = CStr::from_ptr(fmt).to_str() {
            c_str.to_string()
        } else {
            "Invalid error message".to_string()
        }
    };

    if let Some(context) = CONTEXTS.get(ctx) {
        if let Ok(guard) = context.lock() {
            guard.set_error(errcode, message.clone());

            // Call error callback if set
            if let Some(callback) = guard.error_callback {
                let msg_cstr = std::ffi::CString::new(message).unwrap_or_default();
                unsafe {
                    callback(guard.user_data, errcode, msg_cstr.as_ptr());
                }
            }
        }
    }
}

/// Rethrow the current error
#[unsafe(no_mangle)]
pub extern "C" fn fz_rethrow(ctx: Handle) {
    if let Some(context) = CONTEXTS.get(ctx) {
        if let Ok(guard) = context.lock() {
            let (code, message) = guard.get_error();
            if code != FzErrorType::None as c_int {
                // Error already set, just call callback if present
                if let Some(callback) = guard.error_callback {
                    let msg_cstr = std::ffi::CString::new(message).unwrap_or_default();
                    unsafe {
                        callback(guard.user_data, code, msg_cstr.as_ptr());
                    }
                }
            }
        }
    }
}

/// Get the current error code
#[unsafe(no_mangle)]
pub extern "C" fn fz_caught(ctx: Handle) -> c_int {
    if let Some(context) = CONTEXTS.get(ctx) {
        if let Ok(guard) = context.lock() {
            let (code, _) = guard.get_error();
            return code;
        }
    }
    FzErrorType::Generic as c_int
}

/// Get the current error message
///
/// Returns a pointer to a string that remains valid until next error or next call
/// to this function. The string is cached in thread-local storage.
#[unsafe(no_mangle)]
pub extern "C" fn fz_caught_message(ctx: Handle) -> *const c_char {
    if let Some(context) = CONTEXTS.get(ctx) {
        if let Ok(guard) = context.lock() {
            let (_, message) = guard.get_error();
            if message.is_empty() {
                return c"No error".as_ptr();
            }
            // Cache the message in thread-local storage to avoid leaks
            if let Ok(c_str) = CString::new(message) {
                CACHED_ERROR_MSG.with(|cell| {
                    *cell.borrow_mut() = Some(c_str);
                });
                return CACHED_ERROR_MSG.with(|cell| {
                    cell.borrow()
                        .as_ref()
                        .map(|s| s.as_ptr())
                        .unwrap_or(c"Unknown error".as_ptr())
                });
            }
        }
    }
    c"Unknown error".as_ptr()
}

/// Clear the current error
#[unsafe(no_mangle)]
pub extern "C" fn fz_ignore_error(ctx: Handle) {
    if let Some(context) = CONTEXTS.get(ctx) {
        if let Ok(guard) = context.lock() {
            guard.clear_error();
        }
    }
}

/// Log a warning
///
/// # Safety
/// Caller must ensure fmt is a valid null-terminated C string
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fz_warn(ctx: Handle, fmt: *const c_char) {
    if fmt.is_null() {
        return;
    }

    if let Some(context) = CONTEXTS.get(ctx) {
        if let Ok(guard) = context.lock() {
            if let Some(callback) = guard.warn_callback {
                unsafe {
                    callback(guard.user_data, fmt);
                }
            } else {
                // Default: print to stderr
                // SAFETY: Caller guarantees fmt is a valid null-terminated C string
                unsafe {
                    if let Ok(c_str) = CStr::from_ptr(fmt).to_str() {
                        eprintln!("warning: {}", c_str);
                    }
                }
            }
        }
    }
}

/// Flush any repeated warnings (no-op in our implementation)
#[unsafe(no_mangle)]
pub extern "C" fn fz_flush_warnings(_ctx: Handle) {
    // No-op: we don't buffer warnings
}

/// Register error callback
///
/// # Safety
/// Callback must be a valid function pointer
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fz_set_error_callback(
    ctx: Handle,
    callback: Option<unsafe extern "C" fn(*mut c_void, c_int, *const c_char)>,
    user: *mut c_void,
) {
    if let Some(context) = CONTEXTS.get(ctx) {
        if let Ok(mut guard) = context.lock() {
            guard.error_callback = callback;
            guard.user_data = user;
        }
    }
}

/// Register warning callback
///
/// # Safety
/// Callback must be a valid function pointer
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fz_set_warning_callback(
    ctx: Handle,
    callback: Option<unsafe extern "C" fn(*mut c_void, *const c_char)>,
    user: *mut c_void,
) {
    if let Some(context) = CONTEXTS.get(ctx) {
        if let Ok(mut guard) = context.lock() {
            guard.warn_callback = callback;
            guard.user_data = user;
        }
    }
}

/// Get max store size
#[unsafe(no_mangle)]
pub extern "C" fn fz_store_size(ctx: Handle) -> usize {
    if let Some(context) = CONTEXTS.get(ctx) {
        if let Ok(guard) = context.lock() {
            return guard.max_store();
        }
    }
    0
}

/// Convert error to string (for language bindings)
///
/// Returns error message and sets code. The returned pointer is valid
/// until the next call to this function or fz_caught_message.
#[unsafe(no_mangle)]
pub extern "C" fn fz_convert_error(ctx: Handle, code: *mut c_int) -> *const c_char {
    if let Some(context) = CONTEXTS.get(ctx) {
        if let Ok(guard) = context.lock() {
            let (err_code, message) = guard.get_error();
            if !code.is_null() {
                unsafe {
                    *code = err_code;
                }
            }

            if message.is_empty() {
                return c"No error".as_ptr();
            }

            // Cache the message in thread-local storage to avoid leaks
            if let Ok(c_str) = CString::new(message) {
                CACHED_ERROR_MSG.with(|cell| {
                    *cell.borrow_mut() = Some(c_str);
                });
                return CACHED_ERROR_MSG.with(|cell| {
                    cell.borrow()
                        .as_ref()
                        .map(|s| s.as_ptr())
                        .unwrap_or(c"Unknown error".as_ptr())
                });
            }
        }
    }

    if !code.is_null() {
        unsafe {
            *code = FzErrorType::Generic as c_int;
        }
    }
    c"Unknown error".as_ptr()
}

/// Report error (calls error callback)
#[unsafe(no_mangle)]
pub extern "C" fn fz_report_error(ctx: Handle) {
    if let Some(context) = CONTEXTS.get(ctx) {
        if let Ok(guard) = context.lock() {
            let (code, message) = guard.get_error();
            if code != FzErrorType::None as c_int {
                if let Some(callback) = guard.error_callback {
                    let msg_cstr = std::ffi::CString::new(message).unwrap_or_default();
                    unsafe {
                        callback(guard.user_data, code, msg_cstr.as_ptr());
                    }
                }
            }
        }
    }
}

// Helper function to create default context
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_default_context() -> Handle {
    unsafe { fz_new_context(std::ptr::null(), std::ptr::null(), 256 * 1024 * 1024) } // 256 MB default
}

/// Keep a context (alias for clone, increments refcount)
#[unsafe(no_mangle)]
pub extern "C" fn fz_keep_context(ctx: Handle) -> Handle {
    fz_clone_context(ctx)
}

/// Check if context has an error
#[unsafe(no_mangle)]
pub extern "C" fn fz_has_error(ctx: Handle) -> c_int {
    let code = fz_caught(ctx);
    if code == FzErrorType::None as c_int {
        0
    } else {
        1
    }
}

/// Check if context is valid
#[unsafe(no_mangle)]
pub extern "C" fn fz_context_is_valid(ctx: Handle) -> c_int {
    if CONTEXTS.get(ctx).is_some() { 1 } else { 0 }
}

/// Shrink store to given percentage of maximum
#[unsafe(no_mangle)]
pub extern "C" fn fz_shrink_store(_ctx: Handle, percent: c_int) {
    if let Ok(mut store) = STORE_STATE.lock() {
        // Calculate target size as percentage of max
        let target = (store.max_bytes as f32 * (percent as f32 / 100.0)) as usize;

        // Reduce store allocation to target
        // Note: Advanced cache management with LRU eviction could be added here
        // for systems that need fine-grained memory control
        if target < store.allocated_bytes {
            store.allocated_bytes = target;
        }
    }
}

/// Empty the store completely
#[unsafe(no_mangle)]
pub extern "C" fn fz_empty_store(_ctx: Handle) {
    if let Ok(mut store) = STORE_STATE.lock() {
        // Reset store allocation tracking
        // Note: Advanced implementations could walk cached resources and free them
        store.allocated_bytes = 0;
    }
}

/// Scavenge store to free up specified amount of memory
#[unsafe(no_mangle)]
pub extern "C" fn fz_store_scavenge(_ctx: Handle, size: usize, phase: *mut c_int) -> c_int {
    if let Ok(mut store) = STORE_STATE.lock() {
        // Calculate how much we can free
        let available = store.allocated_bytes;
        let to_free = size.min(available);

        // Phase indicates scavenging aggressiveness (0=gentle, 1=moderate, 2=aggressive)
        // Advanced cache implementations could use this to prioritize what to free
        if !phase.is_null() {
            unsafe {
                *phase = 0; // Gentle phase
            }
        }

        store.allocated_bytes = store.allocated_bytes.saturating_sub(to_free);
        return to_free as c_int;
    }
    0
}

/// Enable ICC color management
#[unsafe(no_mangle)]
pub extern "C" fn fz_enable_icc(ctx: Handle) {
    if CONTEXTS.get(ctx).is_some() {
        if let Ok(mut settings) = CONTEXT_SETTINGS.lock() {
            settings
                .entry(ctx)
                .or_insert_with(ContextSettings::default)
                .icc_enabled = true;
        }
    }
}

/// Disable ICC color management
#[unsafe(no_mangle)]
pub extern "C" fn fz_disable_icc(ctx: Handle) {
    if CONTEXTS.get(ctx).is_some() {
        if let Ok(mut settings) = CONTEXT_SETTINGS.lock() {
            settings
                .entry(ctx)
                .or_insert_with(ContextSettings::default)
                .icc_enabled = false;
        }
    }
}

/// Set AA level (anti-aliasing bits per pixel)
/// Common values: 0 (disabled), 2 (4-level), 4 (16-level), 8 (256-level)
#[unsafe(no_mangle)]
pub extern "C" fn fz_set_aa_level(ctx: Handle, bits: c_int) {
    if CONTEXTS.get(ctx).is_some() {
        if let Ok(mut settings) = CONTEXT_SETTINGS.lock() {
            settings
                .entry(ctx)
                .or_insert_with(ContextSettings::default)
                .aa_level = bits.clamp(0, 8); // Clamp to 0-8 bits
        }
    }
}

/// Get AA level
#[unsafe(no_mangle)]
pub extern "C" fn fz_aa_level(ctx: Handle) -> c_int {
    if CONTEXTS.get(ctx).is_some() {
        if let Ok(settings) = CONTEXT_SETTINGS.lock() {
            return settings.get(&ctx).map(|s| s.aa_level).unwrap_or(8); // Default 8-bit AA
        }
    }
    8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_create_drop() {
        let ctx = unsafe { fz_new_context(std::ptr::null(), std::ptr::null(), 1024 * 1024) };
        assert_ne!(ctx, 0);
        fz_drop_context(ctx);
    }

    #[test]
    fn test_context_clone() {
        let ctx1 = unsafe { fz_new_context(std::ptr::null(), std::ptr::null(), 1024 * 1024) };
        let ctx2 = fz_clone_context(ctx1);
        assert_eq!(ctx1, ctx2);
        fz_drop_context(ctx1);
        fz_drop_context(ctx2);
    }

    #[test]
    fn test_error_handling() {
        let ctx = unsafe { fz_new_context(std::ptr::null(), std::ptr::null(), 1024 * 1024) };

        let msg = std::ffi::CString::new("Test error").unwrap();
        unsafe {
            fz_throw(ctx, FzErrorType::Argument as c_int, msg.as_ptr());
        }

        let code = fz_caught(ctx);
        assert_eq!(code, FzErrorType::Argument as c_int);

        fz_ignore_error(ctx);
        let code = fz_caught(ctx);
        assert_eq!(code, FzErrorType::None as c_int);

        fz_drop_context(ctx);
    }

    #[test]
    fn test_user_context() {
        let ctx = unsafe { fz_new_context(std::ptr::null(), std::ptr::null(), 1024 * 1024) };

        let user_data = 42i32;
        let user_ptr = &user_data as *const i32 as *mut c_void;

        unsafe {
            fz_set_user_context(ctx, user_ptr);
        }

        let retrieved = fz_user_context(ctx);
        assert_eq!(retrieved, user_ptr);

        fz_drop_context(ctx);
    }

    #[test]
    fn test_store_size() {
        let ctx = unsafe { fz_new_context(std::ptr::null(), std::ptr::null(), 1024 * 1024) };
        let size = fz_store_size(ctx);
        assert_eq!(size, 1024 * 1024);
        fz_drop_context(ctx);
    }

    #[test]
    fn test_warning() {
        let ctx = unsafe { fz_new_context(std::ptr::null(), std::ptr::null(), 1024 * 1024) };
        let msg = std::ffi::CString::new("Test warning").unwrap();
        unsafe {
            fz_warn(ctx, msg.as_ptr());
        }
        fz_drop_context(ctx);
    }

    #[test]
    fn test_default_context() {
        let ctx = fz_new_default_context();
        assert_ne!(ctx, 0);
        let size = fz_store_size(ctx);
        assert_eq!(size, 256 * 1024 * 1024);
        fz_drop_context(ctx);
    }

    #[test]
    fn test_rethrow() {
        let ctx = unsafe { fz_new_context(std::ptr::null(), std::ptr::null(), 1024 * 1024) };

        let msg = std::ffi::CString::new("Original error").unwrap();
        unsafe {
            fz_throw(ctx, FzErrorType::Format as c_int, msg.as_ptr());
        }

        fz_rethrow(ctx);

        let code = fz_caught(ctx);
        assert_eq!(code, FzErrorType::Format as c_int);

        fz_drop_context(ctx);
    }

    #[test]
    fn test_convert_error() {
        let ctx = unsafe { fz_new_context(std::ptr::null(), std::ptr::null(), 1024 * 1024) };

        let msg = std::ffi::CString::new("Conversion test").unwrap();
        unsafe {
            fz_throw(ctx, FzErrorType::Limit as c_int, msg.as_ptr());
        }

        let mut code: c_int = 0;
        let _msg_ptr = fz_convert_error(ctx, &mut code);
        assert_eq!(code, FzErrorType::Limit as c_int);

        fz_drop_context(ctx);
    }

    #[test]
    fn test_caught_message() {
        let ctx = unsafe { fz_new_context(std::ptr::null(), std::ptr::null(), 1024 * 1024) };

        let msg = std::ffi::CString::new("Detailed error").unwrap();
        unsafe {
            fz_throw(ctx, FzErrorType::Generic as c_int, msg.as_ptr());
        }

        let msg_ptr = fz_caught_message(ctx);
        assert!(!msg_ptr.is_null());

        fz_drop_context(ctx);
    }

    #[test]
    fn test_flush_warnings() {
        let ctx = unsafe { fz_new_context(std::ptr::null(), std::ptr::null(), 1024 * 1024) };
        fz_flush_warnings(ctx); // Should not panic
        fz_drop_context(ctx);
    }

    #[test]
    fn test_report_error() {
        let ctx = unsafe { fz_new_context(std::ptr::null(), std::ptr::null(), 1024 * 1024) };

        let msg = std::ffi::CString::new("Report test").unwrap();
        unsafe {
            fz_throw(ctx, FzErrorType::System as c_int, msg.as_ptr());
        }

        fz_report_error(ctx); // Should not panic

        fz_drop_context(ctx);
    }
}
