use windows::Win32::Foundation::{ERROR_ALREADY_EXISTS, GetLastError, HANDLE};
use windows::Win32::System::Threading::CreateMutexW;
use windows::Win32::UI::WindowsAndMessaging::{
    FindWindowW, GetWindowThreadProcessId, SW_RESTORE, SetForegroundWindow, ShowWindow,
};
use windows::core::PCWSTR;

static mut PRIMARY_MUTEX: Option<HANDLE> = None;

pub fn ensure_single_instance(args: &[String]) -> Result<bool, Box<dyn std::error::Error>> {
    use interprocess::local_socket::{GenericNamespaced, Stream, prelude::*};
    use std::io::Write;

    let mutex_name: Vec<u16> = "Local\\PDFbull_SingleInstance_Mutex\0"
        .encode_utf16()
        .collect();
    let is_secondary = unsafe {
        let handle = CreateMutexW(None, true, PCWSTR(mutex_name.as_ptr()))?;
        if GetLastError() == ERROR_ALREADY_EXISTS {
            true
        } else {
            PRIMARY_MUTEX = Some(handle);
            false
        }
    };

    let name = "pdfbull-single-instance.sock".to_ns_name::<GenericNamespaced>()?;

    if is_secondary {
        // Retry connection for up to 1000ms in case the primary instance is currently binding its listener
        let mut stream = None;
        for _ in 0..20 {
            if let Ok(s) = Stream::connect(name.clone()) {
                stream = Some(s);
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }

        if let Some(mut stream) = stream {
            let json = serde_json::to_string(args)?;
            stream.write_all(json.as_bytes())?;
            stream.write_all(b"\n")?;
            stream.flush()?;

            // Bring the primary window to foreground
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

            return Ok(true);
        }
    }

    Ok(false)
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

pub fn is_system_dark_mode() -> bool {
    use windows::Win32::System::Registry::{HKEY_CURRENT_USER, RRF_RT_REG_DWORD, RegGetValueW};
    use windows::core::w;

    let mut data: u32 = 0;
    let mut data_len = std::mem::size_of::<u32>() as u32;
    let status = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            w!(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize"),
            w!("AppsUseLightTheme"),
            RRF_RT_REG_DWORD,
            None,
            Some(&mut data as *mut u32 as *mut _),
            Some(&mut data_len),
        )
    };
    // 0 = Dark Mode, 1 = Light Mode. If key cannot be read, default to Light (false).
    status.is_ok() && data == 0
}
