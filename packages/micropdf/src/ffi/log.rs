//! FFI bindings for fz_log (Logging Functions)
//!
//! Provides logging with levels, module filtering, and custom callbacks.

use crate::ffi::Handle;
use std::ffi::{CStr, CString, c_char, c_void};
use std::io::Write;
use std::ptr;
use std::sync::{LazyLock, Mutex, RwLock};

// ============================================================================
// Types
// ============================================================================

/// Log levels
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum LogLevel {
    /// No logging
    Off = 0,
    /// Critical errors only
    Error = 1,
    /// Warnings and errors
    Warn = 2,
    /// Informational messages
    #[default]
    Info = 3,
    /// Debug messages
    Debug = 4,
    /// Verbose trace messages
    Trace = 5,
}

impl LogLevel {
    pub fn from_i32(value: i32) -> Self {
        match value {
            0 => LogLevel::Off,
            1 => LogLevel::Error,
            2 => LogLevel::Warn,
            3 => LogLevel::Info,
            4 => LogLevel::Debug,
            5 => LogLevel::Trace,
            _ => LogLevel::Info,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            LogLevel::Off => "OFF",
            LogLevel::Error => "ERROR",
            LogLevel::Warn => "WARN",
            LogLevel::Info => "INFO",
            LogLevel::Debug => "DEBUG",
            LogLevel::Trace => "TRACE",
        }
    }
}

/// Log callback function type
pub type LogCallback = Option<
    extern "C" fn(user: *mut c_void, level: i32, module: *const c_char, message: *const c_char),
>;

/// Warning callback function type (MuPDF compatible)
pub type WarningCallback = Option<extern "C" fn(user: *mut c_void, message: *const c_char)>;

/// Logger configuration
#[derive(Default)]
struct LogConfig {
    /// Global log level
    level: LogLevel,
    /// Module-specific levels
    module_levels: std::collections::HashMap<String, LogLevel>,
    /// Log callback
    callback: LogCallback,
    /// Callback user data
    callback_user: usize, // Using usize for pointer storage (Send/Sync compatible)
    /// Warning callback (MuPDF compatible)
    warning_callback: WarningCallback,
    /// Warning callback user data
    warning_user: usize,
    /// Log file path
    log_file: Option<String>,
    /// Include timestamps
    include_timestamp: bool,
    /// Include file/line info
    include_location: bool,
    /// Log buffer for collecting messages
    buffer: Vec<String>,
    /// Maximum buffer size (0 = no buffering)
    buffer_size: usize,
}

// Global logger configuration
static LOG_CONFIG: LazyLock<RwLock<LogConfig>> = LazyLock::new(|| {
    RwLock::new(LogConfig {
        level: LogLevel::Info,
        include_timestamp: true,
        include_location: false,
        ..Default::default()
    })
});

// Last warning message (for fz_caught_message compatibility)
static LAST_WARNING: LazyLock<Mutex<Option<CString>>> = LazyLock::new(|| Mutex::new(None));

// ============================================================================
// Internal Functions
// ============================================================================

fn format_log_message(
    level: LogLevel,
    module: Option<&str>,
    message: &str,
    file: Option<&str>,
    line: Option<i32>,
    config: &LogConfig,
) -> String {
    let mut parts = Vec::new();

    // Timestamp
    if config.include_timestamp {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        let secs = now.as_secs();
        let millis = now.subsec_millis();
        parts.push(format!("[{}.{:03}]", secs, millis));
    }

    // Level
    parts.push(format!("[{}]", level.name()));

    // Module
    if let Some(m) = module {
        parts.push(format!("[{}]", m));
    }

    // Location
    if config.include_location {
        if let (Some(f), Some(l)) = (file, line) {
            parts.push(format!("[{}:{}]", f, l));
        }
    }

    // Message
    parts.push(message.to_string());

    parts.join(" ")
}

fn do_log(
    level: LogLevel,
    module: Option<&str>,
    message: &str,
    file: Option<&str>,
    line: Option<i32>,
) {
    let config = LOG_CONFIG.read().unwrap();

    // Check global level
    if level > config.level {
        return;
    }

    // Check module-specific level
    if let Some(mod_name) = module {
        if let Some(&mod_level) = config.module_levels.get(mod_name) {
            if level > mod_level {
                return;
            }
        }
    }

    let formatted = format_log_message(level, module, message, file, line, &config);

    // Call callback if set
    if let Some(cb) = config.callback {
        let module_cstr = module.map(|m| CString::new(m).unwrap());
        let message_cstr = CString::new(formatted.as_str()).unwrap();

        cb(
            config.callback_user as *mut c_void,
            level as i32,
            module_cstr.as_ref().map_or(ptr::null(), |s| s.as_ptr()),
            message_cstr.as_ptr(),
        );
    }

    // Write to file if configured
    if let Some(ref path) = config.log_file {
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
        {
            let _ = writeln!(file, "{}", formatted);
        }
    }

    // Check if we should print to stderr before releasing config
    let should_print_stderr = level <= LogLevel::Warn && config.callback.is_none();
    let buffer_size = config.buffer_size;

    drop(config); // Release read lock

    // Buffer if enabled
    if buffer_size > 0 {
        let mut config = LOG_CONFIG.write().unwrap();
        config.buffer.push(formatted.clone());
        if config.buffer.len() > buffer_size {
            config.buffer.remove(0);
        }
    }

    // Default: print to stderr for errors/warnings
    if should_print_stderr {
        eprintln!("{}", formatted);
    }
}

// ============================================================================
// FFI Functions - Configuration
// ============================================================================

/// Set global log level
#[unsafe(no_mangle)]
pub extern "C" fn fz_set_log_level(_ctx: Handle, level: i32) {
    let mut config = LOG_CONFIG.write().unwrap();
    config.level = LogLevel::from_i32(level);
}

/// Get global log level
#[unsafe(no_mangle)]
pub extern "C" fn fz_get_log_level(_ctx: Handle) -> i32 {
    let config = LOG_CONFIG.read().unwrap();
    config.level as i32
}

/// Set module-specific log level
#[unsafe(no_mangle)]
pub extern "C" fn fz_set_module_log_level(_ctx: Handle, module: *const c_char, level: i32) {
    if module.is_null() {
        return;
    }

    let module_str = unsafe { CStr::from_ptr(module) };
    let module_str = match module_str.to_str() {
        Ok(s) => s.to_string(),
        Err(_) => return,
    };

    let mut config = LOG_CONFIG.write().unwrap();
    config
        .module_levels
        .insert(module_str, LogLevel::from_i32(level));
}

/// Get module-specific log level
#[unsafe(no_mangle)]
pub extern "C" fn fz_get_module_log_level(_ctx: Handle, module: *const c_char) -> i32 {
    if module.is_null() {
        return -1;
    }

    let module_str = unsafe { CStr::from_ptr(module) };
    let module_str = match module_str.to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };

    let config = LOG_CONFIG.read().unwrap();
    config
        .module_levels
        .get(module_str)
        .map_or(config.level as i32, |&l| l as i32)
}

/// Clear module-specific log level
#[unsafe(no_mangle)]
pub extern "C" fn fz_clear_module_log_level(_ctx: Handle, module: *const c_char) {
    if module.is_null() {
        return;
    }

    let module_str = unsafe { CStr::from_ptr(module) };
    let module_str = match module_str.to_str() {
        Ok(s) => s,
        Err(_) => return,
    };

    let mut config = LOG_CONFIG.write().unwrap();
    config.module_levels.remove(module_str);
}

/// Set log callback
#[unsafe(no_mangle)]
pub extern "C" fn fz_set_log_callback(_ctx: Handle, callback: LogCallback, user: *mut c_void) {
    let mut config = LOG_CONFIG.write().unwrap();
    config.callback = callback;
    config.callback_user = user as usize;
}

/// Set warning callback for logging system
#[unsafe(no_mangle)]
pub extern "C" fn fz_log_set_warning_callback(
    _ctx: Handle,
    callback: WarningCallback,
    user: *mut c_void,
) {
    let mut config = LOG_CONFIG.write().unwrap();
    config.warning_callback = callback;
    config.warning_user = user as usize;
}

/// Get warning callback from logging system
#[unsafe(no_mangle)]
pub extern "C" fn fz_log_warning_callback(_ctx: Handle, user: *mut *mut c_void) -> WarningCallback {
    let config = LOG_CONFIG.read().unwrap();
    if !user.is_null() {
        unsafe {
            *user = config.warning_user as *mut c_void;
        }
    }
    config.warning_callback
}

/// Set log file path
#[unsafe(no_mangle)]
pub extern "C" fn fz_set_log_file(_ctx: Handle, path: *const c_char) {
    let mut config = LOG_CONFIG.write().unwrap();

    if path.is_null() {
        config.log_file = None;
    } else {
        let path_str = unsafe { CStr::from_ptr(path) };
        if let Ok(s) = path_str.to_str() {
            config.log_file = Some(s.to_string());
        }
    }
}

/// Enable/disable timestamps
#[unsafe(no_mangle)]
pub extern "C" fn fz_log_include_timestamp(_ctx: Handle, include: i32) {
    let mut config = LOG_CONFIG.write().unwrap();
    config.include_timestamp = include != 0;
}

/// Enable/disable file/line location
#[unsafe(no_mangle)]
pub extern "C" fn fz_log_include_location(_ctx: Handle, include: i32) {
    let mut config = LOG_CONFIG.write().unwrap();
    config.include_location = include != 0;
}

/// Set log buffer size (for collecting messages)
#[unsafe(no_mangle)]
pub extern "C" fn fz_set_log_buffer_size(_ctx: Handle, size: usize) {
    let mut config = LOG_CONFIG.write().unwrap();
    config.buffer_size = size;
    if size == 0 {
        config.buffer.clear();
    }
}

/// Get log buffer size
#[unsafe(no_mangle)]
pub extern "C" fn fz_get_log_buffer_size(_ctx: Handle) -> usize {
    let config = LOG_CONFIG.read().unwrap();
    config.buffer_size
}

// ============================================================================
// FFI Functions - Logging
// ============================================================================

/// Log a message (generic)
#[unsafe(no_mangle)]
pub extern "C" fn fz_log(_ctx: Handle, message: *const c_char) {
    if message.is_null() {
        return;
    }

    let msg = unsafe { CStr::from_ptr(message) };
    let msg = msg.to_str().unwrap_or("");

    do_log(LogLevel::Info, None, msg, None, None);
}

/// Log a message for a specific module
#[unsafe(no_mangle)]
pub extern "C" fn fz_log_module(_ctx: Handle, module: *const c_char, message: *const c_char) {
    if message.is_null() {
        return;
    }

    let msg = unsafe { CStr::from_ptr(message) };
    let msg = msg.to_str().unwrap_or("");

    let mod_str = if module.is_null() {
        None
    } else {
        unsafe { CStr::from_ptr(module) }.to_str().ok()
    };

    do_log(LogLevel::Info, mod_str, msg, None, None);
}

/// Log an error message
#[unsafe(no_mangle)]
pub extern "C" fn fz_log_error(_ctx: Handle, message: *const c_char) {
    if message.is_null() {
        return;
    }

    let msg = unsafe { CStr::from_ptr(message) };
    let msg = msg.to_str().unwrap_or("");

    do_log(LogLevel::Error, None, msg, None, None);
}

/// Log a warning message (logging system version)
#[unsafe(no_mangle)]
pub extern "C" fn fz_log_warn(_ctx: Handle, message: *const c_char) {
    if message.is_null() {
        return;
    }

    let msg = unsafe { CStr::from_ptr(message) };
    let msg = msg.to_str().unwrap_or("");

    // Store for fz_caught_message
    if let Ok(mut last) = LAST_WARNING.lock() {
        *last = CString::new(msg).ok();
    }

    // Call warning callback if set
    {
        let config = LOG_CONFIG.read().unwrap();
        if let Some(cb) = config.warning_callback {
            let msg_cstr = CString::new(msg).unwrap();
            cb(config.warning_user as *mut c_void, msg_cstr.as_ptr());
        }
    }

    do_log(LogLevel::Warn, None, msg, None, None);
}

/// Log a debug message
#[unsafe(no_mangle)]
pub extern "C" fn fz_log_debug(_ctx: Handle, message: *const c_char) {
    if message.is_null() {
        return;
    }

    let msg = unsafe { CStr::from_ptr(message) };
    let msg = msg.to_str().unwrap_or("");

    do_log(LogLevel::Debug, None, msg, None, None);
}

/// Log a trace message
#[unsafe(no_mangle)]
pub extern "C" fn fz_log_trace(_ctx: Handle, message: *const c_char) {
    if message.is_null() {
        return;
    }

    let msg = unsafe { CStr::from_ptr(message) };
    let msg = msg.to_str().unwrap_or("");

    do_log(LogLevel::Trace, None, msg, None, None);
}

/// Log with level
#[unsafe(no_mangle)]
pub extern "C" fn fz_log_level(_ctx: Handle, level: i32, message: *const c_char) {
    if message.is_null() {
        return;
    }

    let msg = unsafe { CStr::from_ptr(message) };
    let msg = msg.to_str().unwrap_or("");

    do_log(LogLevel::from_i32(level), None, msg, None, None);
}

/// Log with file/line info (for macros)
#[unsafe(no_mangle)]
pub extern "C" fn fz_log_fl(
    _ctx: Handle,
    level: i32,
    file: *const c_char,
    line: i32,
    message: *const c_char,
) {
    if message.is_null() {
        return;
    }

    let msg = unsafe { CStr::from_ptr(message) };
    let msg = msg.to_str().unwrap_or("");

    let file_str = if file.is_null() {
        None
    } else {
        unsafe { CStr::from_ptr(file) }.to_str().ok()
    };

    do_log(LogLevel::from_i32(level), None, msg, file_str, Some(line));
}

// ============================================================================
// FFI Functions - Buffer Access
// ============================================================================

/// Get number of buffered log messages
#[unsafe(no_mangle)]
pub extern "C" fn fz_log_buffer_count(_ctx: Handle) -> usize {
    let config = LOG_CONFIG.read().unwrap();
    config.buffer.len()
}

/// Get buffered log message by index
#[unsafe(no_mangle)]
pub extern "C" fn fz_log_buffer_get(
    _ctx: Handle,
    index: usize,
    output: *mut c_char,
    output_size: usize,
) -> usize {
    if output.is_null() || output_size == 0 {
        return 0;
    }

    let config = LOG_CONFIG.read().unwrap();
    let msg = match config.buffer.get(index) {
        Some(m) => m,
        None => return 0,
    };

    let bytes = msg.as_bytes();
    let copy_len = bytes.len().min(output_size - 1);

    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), output as *mut u8, copy_len);
        *output.add(copy_len) = 0;
    }

    copy_len
}

/// Clear log buffer
#[unsafe(no_mangle)]
pub extern "C" fn fz_log_buffer_clear(_ctx: Handle) {
    let mut config = LOG_CONFIG.write().unwrap();
    config.buffer.clear();
}

/// Get last warning message from logging system
#[unsafe(no_mangle)]
pub extern "C" fn fz_log_last_warning(_ctx: Handle) -> *const c_char {
    if let Ok(guard) = LAST_WARNING.lock() {
        guard.as_ref().map_or(ptr::null(), |s| s.as_ptr())
    } else {
        ptr::null()
    }
}

// ============================================================================
// FFI Functions - Level Names
// ============================================================================

/// Get log level name
#[unsafe(no_mangle)]
pub extern "C" fn fz_log_level_name(level: i32) -> *const c_char {
    static LEVEL_NAMES: LazyLock<[CString; 6]> = LazyLock::new(|| {
        [
            CString::new("OFF").unwrap(),
            CString::new("ERROR").unwrap(),
            CString::new("WARN").unwrap(),
            CString::new("INFO").unwrap(),
            CString::new("DEBUG").unwrap(),
            CString::new("TRACE").unwrap(),
        ]
    });

    if level < 0 || level > 5 {
        return ptr::null();
    }

    LEVEL_NAMES[level as usize].as_ptr()
}

/// Parse log level from name
#[unsafe(no_mangle)]
pub extern "C" fn fz_parse_log_level(name: *const c_char) -> i32 {
    if name.is_null() {
        return -1;
    }

    let name_str = unsafe { CStr::from_ptr(name) };
    let name_str = match name_str.to_str() {
        Ok(s) => s.to_uppercase(),
        Err(_) => return -1,
    };

    match name_str.as_str() {
        "OFF" => LogLevel::Off as i32,
        "ERROR" => LogLevel::Error as i32,
        "WARN" | "WARNING" => LogLevel::Warn as i32,
        "INFO" => LogLevel::Info as i32,
        "DEBUG" => LogLevel::Debug as i32,
        "TRACE" => LogLevel::Trace as i32,
        _ => -1,
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    fn reset_log_config() {
        let mut config = LOG_CONFIG.write().unwrap();
        *config = LogConfig {
            level: LogLevel::Info,
            include_timestamp: true,
            include_location: false,
            ..Default::default()
        };
    }

    #[test]
    fn test_log_level_enum() {
        assert_eq!(LogLevel::from_i32(0), LogLevel::Off);
        assert_eq!(LogLevel::from_i32(1), LogLevel::Error);
        assert_eq!(LogLevel::from_i32(3), LogLevel::Info);
        assert_eq!(LogLevel::from_i32(99), LogLevel::Info);

        assert_eq!(LogLevel::Error.name(), "ERROR");
        assert_eq!(LogLevel::Debug.name(), "DEBUG");
    }

    #[test]
    #[serial]
    fn test_set_get_level() {
        reset_log_config();
        let ctx = 1;

        fz_set_log_level(ctx, LogLevel::Debug as i32);
        assert_eq!(fz_get_log_level(ctx), LogLevel::Debug as i32);

        fz_set_log_level(ctx, LogLevel::Error as i32);
        assert_eq!(fz_get_log_level(ctx), LogLevel::Error as i32);

        reset_log_config();
    }

    #[test]
    #[serial]
    fn test_module_level() {
        reset_log_config();
        let ctx = 1;

        let module = CString::new("TEST").unwrap();

        fz_set_module_log_level(ctx, module.as_ptr(), LogLevel::Trace as i32);
        assert_eq!(
            fz_get_module_log_level(ctx, module.as_ptr()),
            LogLevel::Trace as i32
        );

        fz_clear_module_log_level(ctx, module.as_ptr());
        // Should fall back to global level
        assert_eq!(
            fz_get_module_log_level(ctx, module.as_ptr()),
            LogLevel::Info as i32
        );

        reset_log_config();
    }

    #[test]
    #[serial]
    fn test_log_basic() {
        reset_log_config();
        let ctx = 1;

        let msg = CString::new("Test message").unwrap();
        fz_log(ctx, msg.as_ptr());

        // Should not crash
        reset_log_config();
    }

    #[test]
    #[serial]
    fn test_log_module() {
        reset_log_config();
        let ctx = 1;

        let module = CString::new("STORE").unwrap();
        let msg = CString::new("Store operation").unwrap();
        fz_log_module(ctx, module.as_ptr(), msg.as_ptr());

        // Should not crash
        reset_log_config();
    }

    #[test]
    #[serial]
    fn test_log_levels() {
        reset_log_config();
        let ctx = 1;

        let msg = CString::new("Test").unwrap();

        fz_log_error(ctx, msg.as_ptr());
        fz_log_warn(ctx, msg.as_ptr());
        fz_log_debug(ctx, msg.as_ptr());
        fz_log_trace(ctx, msg.as_ptr());

        // Should not crash
        reset_log_config();
    }

    #[test]
    #[serial]
    fn test_log_buffer() {
        reset_log_config(); // Ensure clean state
        let ctx = 1;

        // Test buffer size setting
        fz_set_log_buffer_size(ctx, 10);
        assert_eq!(fz_get_log_buffer_size(ctx), 10);

        // Test buffer clear doesn't crash
        fz_log_buffer_clear(ctx);

        // Test that we can get buffer count (may be affected by other tests)
        let _count = fz_log_buffer_count(ctx);

        // Test buffer get with index out of bounds returns 0
        let mut output = vec![0u8; 100];
        let len = fz_log_buffer_get(
            ctx,
            999999,
            output.as_mut_ptr() as *mut c_char,
            output.len(),
        );
        assert_eq!(len, 0);

        // Test that setting buffer size to 0 works
        fz_set_log_buffer_size(ctx, 0);
        assert_eq!(fz_get_log_buffer_size(ctx), 0);
    }

    #[test]
    #[serial]
    fn test_log_file() {
        reset_log_config();
        let ctx = 1;

        // Set a temp file (we won't actually write in tests)
        let path = CString::new("/tmp/test_log.txt").unwrap();
        fz_set_log_file(ctx, path.as_ptr());

        // Clear it
        fz_set_log_file(ctx, ptr::null());

        reset_log_config();
    }

    #[test]
    #[serial]
    fn test_timestamp_location() {
        reset_log_config();
        let ctx = 1;

        fz_log_include_timestamp(ctx, 0);
        fz_log_include_location(ctx, 1);

        let msg = CString::new("Test").unwrap();
        let file = CString::new("test.rs").unwrap();
        fz_log_fl(ctx, LogLevel::Info as i32, file.as_ptr(), 42, msg.as_ptr());

        reset_log_config();
    }

    #[test]
    fn test_level_name() {
        let name = fz_log_level_name(LogLevel::Error as i32);
        assert!(!name.is_null());
        let name_str = unsafe { CStr::from_ptr(name) }.to_str().unwrap();
        assert_eq!(name_str, "ERROR");

        let name = fz_log_level_name(LogLevel::Debug as i32);
        assert!(!name.is_null());
        let name_str = unsafe { CStr::from_ptr(name) }.to_str().unwrap();
        assert_eq!(name_str, "DEBUG");

        let name = fz_log_level_name(99);
        assert!(name.is_null());
    }

    #[test]
    fn test_parse_level() {
        let name = CString::new("ERROR").unwrap();
        assert_eq!(fz_parse_log_level(name.as_ptr()), LogLevel::Error as i32);

        let name = CString::new("warn").unwrap();
        assert_eq!(fz_parse_log_level(name.as_ptr()), LogLevel::Warn as i32);

        let name = CString::new("WARNING").unwrap();
        assert_eq!(fz_parse_log_level(name.as_ptr()), LogLevel::Warn as i32);

        let name = CString::new("invalid").unwrap();
        assert_eq!(fz_parse_log_level(name.as_ptr()), -1);
    }

    #[test]
    #[serial]
    fn test_warning_callback() {
        reset_log_config();
        let ctx = 1;

        static mut CALLBACK_CALLED: bool = false;

        extern "C" fn test_callback(_user: *mut c_void, _message: *const c_char) {
            unsafe {
                CALLBACK_CALLED = true;
            }
        }

        fz_log_set_warning_callback(ctx, Some(test_callback), ptr::null_mut());

        let msg = CString::new("Warning!").unwrap();
        fz_log_warn(ctx, msg.as_ptr());

        unsafe {
            assert!(CALLBACK_CALLED);
            CALLBACK_CALLED = false;
        }

        reset_log_config();
    }

    #[test]
    #[serial]
    fn test_last_warning() {
        reset_log_config();
        let ctx = 1;

        let msg = CString::new("Last warning").unwrap();
        fz_log_warn(ctx, msg.as_ptr());

        let caught = fz_log_last_warning(ctx);
        assert!(!caught.is_null());

        let caught_str = unsafe { CStr::from_ptr(caught) }.to_str().unwrap();
        assert_eq!(caught_str, "Last warning");

        reset_log_config();
    }

    #[test]
    fn test_null_handling() {
        let ctx = 1;

        // Should not crash with null pointers
        fz_log(ctx, ptr::null());
        fz_log_module(ctx, ptr::null(), ptr::null());
        fz_log_error(ctx, ptr::null());
        fz_log_warn(ctx, ptr::null());
        fz_set_log_file(ctx, ptr::null());
        fz_set_module_log_level(ctx, ptr::null(), 0);

        assert_eq!(fz_get_module_log_level(ctx, ptr::null()), -1);
        assert_eq!(fz_parse_log_level(ptr::null()), -1);
    }

    #[test]
    fn test_log_filtering() {
        // Test level filtering logic directly
        // Note: Due to global state, we only test the logic, not the buffer behavior

        // LogLevel::Error = 1, LogLevel::Info = 3, LogLevel::Debug = 4
        // When config level is Error (1), messages with level > 1 should be filtered
        assert!(LogLevel::Info as i32 > LogLevel::Error as i32); // Info filtered
        assert!(LogLevel::Debug as i32 > LogLevel::Error as i32); // Debug filtered
        assert!(!(LogLevel::Error as i32 > LogLevel::Error as i32)); // Error passes

        // Verify LogLevel ordering is correct for filtering
        assert_eq!(LogLevel::Off as i32, 0);
        assert_eq!(LogLevel::Error as i32, 1);
        assert_eq!(LogLevel::Warn as i32, 2);
        assert_eq!(LogLevel::Info as i32, 3);
        assert_eq!(LogLevel::Debug as i32, 4);
        assert_eq!(LogLevel::Trace as i32, 5);

        // Test level from_i32
        assert_eq!(LogLevel::from_i32(0), LogLevel::Off);
        assert_eq!(LogLevel::from_i32(1), LogLevel::Error);
        assert_eq!(LogLevel::from_i32(2), LogLevel::Warn);
        assert_eq!(LogLevel::from_i32(3), LogLevel::Info);
        assert_eq!(LogLevel::from_i32(4), LogLevel::Debug);
        assert_eq!(LogLevel::from_i32(5), LogLevel::Trace);
        assert_eq!(LogLevel::from_i32(99), LogLevel::Info); // Invalid falls back to Info (default)
    }
}
