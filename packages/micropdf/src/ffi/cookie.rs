//! FFI bindings for cookie operations (progress tracking and cancellation)
//!
//! Provides C-compatible API for progress monitoring and operation cancellation.

use std::sync::LazyLock;

use super::{Handle, HandleStore};
use crate::fitz::cookie::Cookie;

/// Global storage for cookies
pub static COOKIES: LazyLock<HandleStore<Cookie>> = LazyLock::new(HandleStore::new);

/// Create a new cookie
///
/// # Returns
/// Handle to the new cookie
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_cookie(_ctx: Handle) -> Handle {
    let cookie = Cookie::new();
    COOKIES.insert(cookie)
}

/// Keep a reference to a cookie (increment ref count)
///
/// # Arguments
/// * `cookie` - Handle to the cookie
///
/// # Returns
/// The same cookie handle
#[unsafe(no_mangle)]
pub extern "C" fn fz_keep_cookie(_ctx: Handle, cookie: Handle) -> Handle {
    cookie
}

/// Drop a cookie
///
/// # Arguments
/// * `cookie` - Handle to the cookie
#[unsafe(no_mangle)]
pub extern "C" fn fz_drop_cookie(_ctx: Handle, cookie: Handle) {
    COOKIES.remove(cookie);
}

/// Check if operation should be aborted
///
/// # Arguments
/// * `cookie` - Handle to the cookie
///
/// # Returns
/// 1 if should abort, 0 otherwise
#[unsafe(no_mangle)]
pub extern "C" fn fz_cookie_should_abort(_ctx: Handle, cookie: Handle) -> i32 {
    if let Some(c) = COOKIES.get(cookie) {
        if let Ok(guard) = c.lock() {
            return if guard.should_abort() { 1 } else { 0 };
        }
    }
    0
}

/// Request abortion of current operation
///
/// # Arguments
/// * `cookie` - Handle to the cookie
#[unsafe(no_mangle)]
pub extern "C" fn fz_cookie_abort(_ctx: Handle, cookie: Handle) {
    if let Some(c) = COOKIES.get(cookie) {
        if let Ok(guard) = c.lock() {
            guard.abort();
        }
    }
}

/// Reset abort flag
///
/// # Arguments
/// * `cookie` - Handle to the cookie
#[unsafe(no_mangle)]
pub extern "C" fn fz_cookie_reset_abort(_ctx: Handle, cookie: Handle) {
    if let Some(c) = COOKIES.get(cookie) {
        if let Ok(guard) = c.lock() {
            guard.reset_abort();
        }
    }
}

/// Get current progress
///
/// # Arguments
/// * `cookie` - Handle to the cookie
///
/// # Returns
/// Current progress value
#[unsafe(no_mangle)]
pub extern "C" fn fz_cookie_get_progress(_ctx: Handle, cookie: Handle) -> i32 {
    if let Some(c) = COOKIES.get(cookie) {
        if let Ok(guard) = c.lock() {
            return guard.progress();
        }
    }
    0
}

/// Set current progress
///
/// # Arguments
/// * `cookie` - Handle to the cookie
/// * `value` - Progress value
#[unsafe(no_mangle)]
pub extern "C" fn fz_cookie_set_progress(_ctx: Handle, cookie: Handle, value: i32) {
    if let Some(c) = COOKIES.get(cookie) {
        if let Ok(guard) = c.lock() {
            guard.set_progress(value);
        }
    }
}

/// Increment progress by 1
///
/// # Arguments
/// * `cookie` - Handle to the cookie
#[unsafe(no_mangle)]
pub extern "C" fn fz_cookie_inc_progress(_ctx: Handle, cookie: Handle) {
    if let Some(c) = COOKIES.get(cookie) {
        if let Ok(guard) = c.lock() {
            guard.inc_progress();
        }
    }
}

/// Get progress maximum
///
/// # Arguments
/// * `cookie` - Handle to the cookie
///
/// # Returns
/// Maximum progress value (0 if unknown)
#[unsafe(no_mangle)]
pub extern "C" fn fz_cookie_get_progress_max(_ctx: Handle, cookie: Handle) -> i32 {
    if let Some(c) = COOKIES.get(cookie) {
        if let Ok(guard) = c.lock() {
            return guard.progress_max();
        }
    }
    0
}

/// Set progress maximum
///
/// # Arguments
/// * `cookie` - Handle to the cookie
/// * `value` - Maximum progress value
#[unsafe(no_mangle)]
pub extern "C" fn fz_cookie_set_progress_max(_ctx: Handle, cookie: Handle, value: i32) {
    if let Some(c) = COOKIES.get(cookie) {
        if let Ok(guard) = c.lock() {
            guard.set_progress_max(value);
        }
    }
}

/// Get error count
///
/// # Arguments
/// * `cookie` - Handle to the cookie
///
/// # Returns
/// Number of errors encountered
#[unsafe(no_mangle)]
pub extern "C" fn fz_cookie_get_errors(_ctx: Handle, cookie: Handle) -> i32 {
    if let Some(c) = COOKIES.get(cookie) {
        if let Ok(guard) = c.lock() {
            return guard.errors();
        }
    }
    0
}

/// Increment error count
///
/// # Arguments
/// * `cookie` - Handle to the cookie
#[unsafe(no_mangle)]
pub extern "C" fn fz_cookie_inc_errors(_ctx: Handle, cookie: Handle) {
    if let Some(c) = COOKIES.get(cookie) {
        if let Ok(guard) = c.lock() {
            guard.inc_errors();
        }
    }
}

/// Check if operation was incomplete
///
/// # Arguments
/// * `cookie` - Handle to the cookie
///
/// # Returns
/// 1 if incomplete, 0 otherwise
#[unsafe(no_mangle)]
pub extern "C" fn fz_cookie_is_incomplete(_ctx: Handle, cookie: Handle) -> i32 {
    if let Some(c) = COOKIES.get(cookie) {
        if let Ok(guard) = c.lock() {
            return if guard.is_incomplete() { 1 } else { 0 };
        }
    }
    0
}

/// Mark operation as incomplete
///
/// # Arguments
/// * `cookie` - Handle to the cookie
/// * `value` - 1 to mark incomplete, 0 otherwise
#[unsafe(no_mangle)]
pub extern "C" fn fz_cookie_set_incomplete(_ctx: Handle, cookie: Handle, value: i32) {
    if let Some(c) = COOKIES.get(cookie) {
        if let Ok(guard) = c.lock() {
            guard.set_incomplete(value != 0);
        }
    }
}

/// Get progress as percentage (0-100)
///
/// # Arguments
/// * `cookie` - Handle to the cookie
///
/// # Returns
/// Progress percentage as integer (0-100)
#[unsafe(no_mangle)]
pub extern "C" fn fz_cookie_progress_percent(_ctx: Handle, cookie: Handle) -> i32 {
    if let Some(c) = COOKIES.get(cookie) {
        if let Ok(guard) = c.lock() {
            return guard.progress_percent() as i32;
        }
    }
    0
}

/// Reset all counters and flags
///
/// # Arguments
/// * `cookie` - Handle to the cookie
#[unsafe(no_mangle)]
pub extern "C" fn fz_cookie_reset(_ctx: Handle, cookie: Handle) {
    if let Some(c) = COOKIES.get(cookie) {
        if let Ok(guard) = c.lock() {
            guard.reset();
        }
    }
}

/// Set error count directly
///
/// # Arguments
/// * `cookie` - Handle to the cookie
/// * `count` - Error count to set
#[unsafe(no_mangle)]
pub extern "C" fn fz_cookie_set_errors(_ctx: Handle, cookie: Handle, count: i32) {
    if let Some(c) = COOKIES.get(cookie) {
        if let Ok(guard) = c.lock() {
            guard.set_error(count);
        }
    }
}

/// Check if cookie has any errors
///
/// # Arguments
/// * `cookie` - Handle to the cookie
///
/// # Returns
/// 1 if errors > 0, 0 otherwise
#[unsafe(no_mangle)]
pub extern "C" fn fz_cookie_has_errors(_ctx: Handle, cookie: Handle) -> i32 {
    if let Some(c) = COOKIES.get(cookie) {
        if let Ok(guard) = c.lock() {
            return if guard.errors() > 0 { 1 } else { 0 };
        }
    }
    0
}

/// Get progress as float (0.0-1.0)
///
/// # Arguments
/// * `cookie` - Handle to the cookie
///
/// # Returns
/// Progress as float between 0.0 and 1.0
#[unsafe(no_mangle)]
pub extern "C" fn fz_cookie_progress_float(_ctx: Handle, cookie: Handle) -> f32 {
    if let Some(c) = COOKIES.get(cookie) {
        if let Ok(guard) = c.lock() {
            let prog = guard.progress();
            let max = guard.progress_max();
            if max > 0 {
                return prog as f32 / max as f32;
            }
        }
    }
    0.0
}

/// Check if operation is complete
///
/// # Arguments
/// * `cookie` - Handle to the cookie
///
/// # Returns
/// 1 if complete (progress >= progress_max), 0 otherwise
#[unsafe(no_mangle)]
pub extern "C" fn fz_cookie_is_complete(_ctx: Handle, cookie: Handle) -> i32 {
    if let Some(c) = COOKIES.get(cookie) {
        if let Ok(guard) = c.lock() {
            let prog = guard.progress();
            let max = guard.progress_max();
            if max > 0 && prog >= max {
                return 1;
            }
        }
    }
    0
}

/// Get remaining progress
///
/// # Arguments
/// * `cookie` - Handle to the cookie
///
/// # Returns
/// Remaining progress (progress_max - progress), or 0 if unknown
#[unsafe(no_mangle)]
pub extern "C" fn fz_cookie_progress_remaining(_ctx: Handle, cookie: Handle) -> i32 {
    if let Some(c) = COOKIES.get(cookie) {
        if let Ok(guard) = c.lock() {
            let prog = guard.progress();
            let max = guard.progress_max();
            if max > prog {
                return max - prog;
            }
        }
    }
    0
}

/// Clone a cookie (creates a new cookie with same state)
///
/// # Arguments
/// * `cookie` - Handle to the cookie to clone
///
/// # Returns
/// Handle to the new cookie
#[unsafe(no_mangle)]
pub extern "C" fn fz_clone_cookie(_ctx: Handle, cookie: Handle) -> Handle {
    if let Some(c) = COOKIES.get(cookie) {
        if let Ok(guard) = c.lock() {
            let new_cookie = Cookie::new();
            new_cookie.set_progress(guard.progress());
            new_cookie.set_progress_max(guard.progress_max());
            new_cookie.set_error(guard.errors());
            new_cookie.set_incomplete(guard.is_incomplete());
            if guard.should_abort() {
                new_cookie.abort();
            }
            return COOKIES.insert(new_cookie);
        }
    }
    0
}

/// Check if cookie is valid
///
/// # Arguments
/// * `cookie` - Handle to check
///
/// # Returns
/// 1 if valid, 0 otherwise
#[unsafe(no_mangle)]
pub extern "C" fn fz_cookie_is_valid(_ctx: Handle, cookie: Handle) -> i32 {
    if COOKIES.get(cookie).is_some() { 1 } else { 0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_cookie() {
        let cookie = fz_new_cookie(0);
        assert_ne!(cookie, 0);
        fz_drop_cookie(0, cookie);
    }

    #[test]
    fn test_abort() {
        let cookie = fz_new_cookie(0);
        assert_eq!(fz_cookie_should_abort(0, cookie), 0);

        fz_cookie_abort(0, cookie);
        assert_eq!(fz_cookie_should_abort(0, cookie), 1);

        fz_cookie_reset_abort(0, cookie);
        assert_eq!(fz_cookie_should_abort(0, cookie), 0);

        fz_drop_cookie(0, cookie);
    }

    #[test]
    fn test_progress() {
        let cookie = fz_new_cookie(0);
        fz_cookie_set_progress_max(0, cookie, 100);

        assert_eq!(fz_cookie_get_progress(0, cookie), 0);
        assert_eq!(fz_cookie_progress_percent(0, cookie), 0);

        fz_cookie_set_progress(0, cookie, 50);
        assert_eq!(fz_cookie_get_progress(0, cookie), 50);
        assert_eq!(fz_cookie_progress_percent(0, cookie), 50);

        fz_cookie_inc_progress(0, cookie);
        assert_eq!(fz_cookie_get_progress(0, cookie), 51);

        fz_drop_cookie(0, cookie);
    }

    #[test]
    fn test_errors() {
        let cookie = fz_new_cookie(0);
        assert_eq!(fz_cookie_get_errors(0, cookie), 0);

        fz_cookie_inc_errors(0, cookie);
        assert_eq!(fz_cookie_get_errors(0, cookie), 1);

        fz_cookie_inc_errors(0, cookie);
        assert_eq!(fz_cookie_get_errors(0, cookie), 2);

        fz_drop_cookie(0, cookie);
    }

    #[test]
    fn test_incomplete() {
        let cookie = fz_new_cookie(0);
        assert_eq!(fz_cookie_is_incomplete(0, cookie), 0);

        fz_cookie_set_incomplete(0, cookie, 1);
        assert_eq!(fz_cookie_is_incomplete(0, cookie), 1);

        fz_cookie_set_incomplete(0, cookie, 0);
        assert_eq!(fz_cookie_is_incomplete(0, cookie), 0);

        fz_drop_cookie(0, cookie);
    }

    #[test]
    fn test_reset() {
        let cookie = fz_new_cookie(0);
        fz_cookie_abort(0, cookie);
        fz_cookie_set_progress(0, cookie, 50);
        fz_cookie_set_progress_max(0, cookie, 100);
        fz_cookie_inc_errors(0, cookie);
        fz_cookie_set_incomplete(0, cookie, 1);

        fz_cookie_reset(0, cookie);

        assert_eq!(fz_cookie_should_abort(0, cookie), 0);
        assert_eq!(fz_cookie_get_progress(0, cookie), 0);
        assert_eq!(fz_cookie_get_progress_max(0, cookie), 0);
        assert_eq!(fz_cookie_get_errors(0, cookie), 0);
        assert_eq!(fz_cookie_is_incomplete(0, cookie), 0);

        fz_drop_cookie(0, cookie);
    }

    #[test]
    fn test_invalid_cookie() {
        assert_eq!(fz_cookie_should_abort(0, 9999), 0);
        assert_eq!(fz_cookie_get_progress(0, 9999), 0);
        assert_eq!(fz_cookie_get_errors(0, 9999), 0);

        // These should not crash
        fz_cookie_abort(0, 9999);
        fz_cookie_set_progress(0, 9999, 50);
    }
}
