#[cfg(windows)]
pub mod windows;

#[cfg(not(windows))]
pub mod dummy {
    pub fn setup_jump_list(_paths: &[String]) {}
    pub fn ensure_single_instance(_args: &[String]) -> Result<bool, Box<dyn std::error::Error>> {
        Ok(false)
    }
    /// Single-instance forwarding is a Windows-only feature; there is never
    /// anything to drain elsewhere.
    pub fn take_forwarded_args() -> Option<tokio::sync::mpsc::UnboundedReceiver<Vec<String>>> {
        None
    }
    pub fn is_system_dark_mode() -> bool {
        false
    }
}

#[cfg(windows)]
pub use windows::*;

#[cfg(not(windows))]
pub use dummy::*;
