#[cfg(windows)]
pub mod windows;

#[cfg(not(windows))]
pub mod dummy {
    pub fn setup_jump_list(_paths: &[String]) {}
    pub fn ensure_single_instance(_args: &[String]) -> Result<bool, Box<dyn std::error::Error>> {
        Ok(false)
    }
}

#[cfg(windows)]
pub use windows::*;

#[cfg(not(windows))]
pub use dummy::*;
