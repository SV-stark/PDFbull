//! Cookie - Progress tracking and cancellation support
//!
//! Cookies provide a way to communicate between the application and the document
//! processing routines, allowing for cancellation and progress reporting.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};

/// Cookie for tracking progress and handling cancellation
#[derive(Clone)]
pub struct Cookie {
    /// Atomic abort flag - set to true to cancel operation
    abort: Arc<AtomicBool>,
    /// Progress counter - number of operations completed
    progress: Arc<AtomicI32>,
    /// Progress maximum - total operations to complete (0 if unknown)
    progress_max: Arc<AtomicI32>,
    /// Error counter - number of errors encountered
    errors: Arc<AtomicI32>,
    /// Incomplete flag - set if operation was incomplete
    incomplete: Arc<AtomicBool>,
}

impl Cookie {
    /// Create a new cookie
    pub fn new() -> Self {
        Self {
            abort: Arc::new(AtomicBool::new(false)),
            progress: Arc::new(AtomicI32::new(0)),
            progress_max: Arc::new(AtomicI32::new(0)),
            errors: Arc::new(AtomicI32::new(0)),
            incomplete: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Check if operation should be aborted
    pub fn should_abort(&self) -> bool {
        self.abort.load(Ordering::Relaxed)
    }

    /// Request abortion of current operation
    pub fn abort(&self) {
        self.abort.store(true, Ordering::Relaxed);
    }

    /// Reset abort flag
    pub fn reset_abort(&self) {
        self.abort.store(false, Ordering::Relaxed);
    }

    /// Get current progress
    pub fn progress(&self) -> i32 {
        self.progress.load(Ordering::Relaxed)
    }

    /// Set current progress
    pub fn set_progress(&self, value: i32) {
        self.progress.store(value, Ordering::Relaxed);
    }

    /// Increment progress by 1
    pub fn inc_progress(&self) {
        self.progress.fetch_add(1, Ordering::Relaxed);
    }

    /// Get progress maximum
    pub fn progress_max(&self) -> i32 {
        self.progress_max.load(Ordering::Relaxed)
    }

    /// Set progress maximum
    pub fn set_progress_max(&self, value: i32) {
        self.progress_max.store(value, Ordering::Relaxed);
    }

    /// Get error count
    pub fn errors(&self) -> i32 {
        self.errors.load(Ordering::Relaxed)
    }

    /// Increment error count
    pub fn inc_errors(&self) {
        self.errors.fetch_add(1, Ordering::Relaxed);
    }

    /// Set error count directly
    pub fn set_error(&self, count: i32) {
        self.errors.store(count, Ordering::Relaxed);
    }

    /// Check if operation was incomplete
    pub fn is_incomplete(&self) -> bool {
        self.incomplete.load(Ordering::Relaxed)
    }

    /// Mark operation as incomplete
    pub fn set_incomplete(&self, value: bool) {
        self.incomplete.store(value, Ordering::Relaxed);
    }

    /// Get progress as percentage (0-100)
    pub fn progress_percent(&self) -> f32 {
        let max = self.progress_max();
        if max <= 0 {
            return 0.0;
        }
        let current = self.progress();
        (current as f32 / max as f32) * 100.0
    }

    /// Reset all counters
    pub fn reset(&self) {
        self.abort.store(false, Ordering::Relaxed);
        self.progress.store(0, Ordering::Relaxed);
        self.progress_max.store(0, Ordering::Relaxed);
        self.errors.store(0, Ordering::Relaxed);
        self.incomplete.store(false, Ordering::Relaxed);
    }
}

impl Default for Cookie {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cookie_new() {
        let cookie = Cookie::new();
        assert!(!cookie.should_abort());
        assert_eq!(cookie.progress(), 0);
        assert_eq!(cookie.progress_max(), 0);
        assert_eq!(cookie.errors(), 0);
        assert!(!cookie.is_incomplete());
    }

    #[test]
    fn test_abort() {
        let cookie = Cookie::new();
        assert!(!cookie.should_abort());

        cookie.abort();
        assert!(cookie.should_abort());

        cookie.reset_abort();
        assert!(!cookie.should_abort());
    }

    #[test]
    fn test_progress() {
        let cookie = Cookie::new();
        cookie.set_progress_max(100);

        assert_eq!(cookie.progress(), 0);
        assert_eq!(cookie.progress_percent(), 0.0);

        cookie.set_progress(50);
        assert_eq!(cookie.progress(), 50);
        assert_eq!(cookie.progress_percent(), 50.0);

        cookie.inc_progress();
        assert_eq!(cookie.progress(), 51);
    }

    #[test]
    fn test_errors() {
        let cookie = Cookie::new();
        assert_eq!(cookie.errors(), 0);

        cookie.inc_errors();
        assert_eq!(cookie.errors(), 1);

        cookie.inc_errors();
        assert_eq!(cookie.errors(), 2);
    }

    #[test]
    fn test_incomplete() {
        let cookie = Cookie::new();
        assert!(!cookie.is_incomplete());

        cookie.set_incomplete(true);
        assert!(cookie.is_incomplete());

        cookie.set_incomplete(false);
        assert!(!cookie.is_incomplete());
    }

    #[test]
    fn test_reset() {
        let cookie = Cookie::new();
        cookie.abort();
        cookie.set_progress(50);
        cookie.set_progress_max(100);
        cookie.inc_errors();
        cookie.set_incomplete(true);

        cookie.reset();

        assert!(!cookie.should_abort());
        assert_eq!(cookie.progress(), 0);
        assert_eq!(cookie.progress_max(), 0);
        assert_eq!(cookie.errors(), 0);
        assert!(!cookie.is_incomplete());
    }

    #[test]
    fn test_clone() {
        let cookie1 = Cookie::new();
        cookie1.set_progress(10);
        cookie1.set_progress_max(100);

        let cookie2 = cookie1.clone();
        cookie2.set_progress(20);

        // Both should see the same progress since they share the Arc
        assert_eq!(cookie1.progress(), 20);
        assert_eq!(cookie2.progress(), 20);
    }

    #[test]
    fn test_progress_percent_zero_max() {
        let cookie = Cookie::new();
        assert_eq!(cookie.progress_percent(), 0.0);

        cookie.set_progress(10);
        assert_eq!(cookie.progress_percent(), 0.0); // Still 0 because max is 0
    }
}
