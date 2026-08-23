use windows::Win32::UI::WindowsAndMessaging::{
    FindWindowW, GetWindowThreadProcessId, SW_RESTORE, SetForegroundWindow, ShowWindow,
};
use windows::core::PCWSTR;

pub fn ensure_single_instance(args: &[String]) -> Result<bool, Box<dyn std::error::Error>> {
    use interprocess::local_socket::{GenericNamespaced, Stream, prelude::*};
    use std::io::Write;

    let name = "pdfbull-single-instance.sock".to_ns_name::<GenericNamespaced>()?;

    // Try to connect to a running instance
    match Stream::connect(name) {
        Ok(mut stream) => {
            // Write arguments to the pipe
            let json = serde_json::to_string(args)?;
            stream.write_all(json.as_bytes())?;
            stream.write_all(b"\n")?;
            stream.flush()?;

            // Attempt to bring the primary window to foreground
            let window_title: Vec<u16> = "PDFbull\0".encode_utf16().collect();
            unsafe {
                if let Ok(hwnd) = FindWindowW(None, PCWSTR(window_title.as_ptr()))
                    && !hwnd.0.is_null()
                {
                    let mut pid: u32 = 0;
                    let _ = GetWindowThreadProcessId(hwnd, Some(&mut pid));
                    let current_pid = std::process::id();
                    if pid != 0 && pid != current_pid {
                        let _ = ShowWindow(hwnd, SW_RESTORE);
                        let _ = SetForegroundWindow(hwnd);
                    }
                }
            }

            Ok(true)
        }
        Err(_) => {
            // No instance running, we are the primary instance
            Ok(false)
        }
    }
}

pub fn setup_jump_list(paths: &[String]) {
    for path in paths {
        let path_u16: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
        unsafe {
            windows::Win32::UI::Shell::SHAddToRecentDocs(
                windows::Win32::UI::Shell::SHARD_PATHW.0 as u32,
                Some(path_u16.as_ptr() as *const _),
            );
        }
    }
}
