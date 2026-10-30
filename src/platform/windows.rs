use std::sync::OnceLock;
use windows::Win32::Foundation::{CloseHandle, ERROR_ALREADY_EXISTS, GetLastError, HANDLE};
use windows::Win32::System::Threading::CreateMutexW;
use windows::Win32::UI::WindowsAndMessaging::{
    FindWindowW, GetWindowThreadProcessId, SW_RESTORE, SetForegroundWindow, ShowWindow,
};
use windows::core::w;

struct MutexHolder(HANDLE);

// SAFETY: Windows kernel HANDLE for a process-lifetime named mutex is safe
// to send and share across thread boundaries.
unsafe impl Send for MutexHolder {}
unsafe impl Sync for MutexHolder {}

impl Drop for MutexHolder {
    fn drop(&mut self) {
        // SAFETY: CloseHandle is called on the valid HANDLE owned by MutexHolder to release the kernel object.
        unsafe {
            let _ = CloseHandle(self.0);
        }
    }
}

static PRIMARY_MUTEX: OnceLock<MutexHolder> = OnceLock::new();

/// Command lines forwarded by a second instance, for the UI to drain.
///
/// The listener thread hands them over through this channel; it is taken exactly
/// once, by `run_gpui_app`, which then `await`s it. `None` on a secondary
/// instance and on any platform without forwarding.
static FORWARD_RX: OnceLock<std::sync::Mutex<Option<ForwardedArgs>>> = OnceLock::new();

/// Receiver end of the forwarded-command-line channel.
type ForwardedArgs = tokio::sync::mpsc::UnboundedReceiver<Vec<String>>;

/// Takes the channel of command lines forwarded by a second instance.
///
/// Returns `None` when this process is not the primary, or when the listener
/// could not be started — in both cases there is nothing to drain.
pub fn take_forwarded_args() -> Option<ForwardedArgs> {
    FORWARD_RX.get()?.lock().ok()?.take()
}

pub fn ensure_single_instance(args: &[String]) -> Result<bool, Box<dyn std::error::Error>> {
    use interprocess::local_socket::{GenericNamespaced, ListenerOptions, Stream, prelude::*};
    use std::io::Write;

    // SAFETY: We call CreateMutexW with a static null-terminated UTF-16 mutex name.
    // The call does not dereference invalid memory. The returned HANDLE is stored
    // in a thread-safe OnceLock to keep the mutex alive for the process lifetime.
    let is_secondary = unsafe {
        let handle = CreateMutexW(None, true, w!("Local\\PDFbull_SingleInstance_Mutex"))?;
        if GetLastError() == ERROR_ALREADY_EXISTS {
            true
        } else {
            let _ = PRIMARY_MUTEX.set(MutexHolder(handle));
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
            // SAFETY: FindWindowW is called with a compile-time static null-terminated UTF-16 string.
            // If a window is found, GetWindowThreadProcessId, ShowWindow, and SetForegroundWindow
            // operate on a valid HWND and valid stack pointers.
            unsafe {
                if let Ok(hwnd) = FindWindowW(None, w!("PDFbull"))
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

        // The mutex says another PDFbull exists but it did not answer. Its mutex
        // handle dies with its process, so a stale lock cannot persist; this is
        // either a foreign process holding the name or a primary that is wedged.
        // Starting normally keeps the app launchable in that case.
        tracing::warn!(
            "Another PDFbull instance owns the single-instance mutex but did not \
             accept the arguments; starting a separate instance."
        );
        return Ok(false);
    }

    // Primary. Bind the endpoint a second instance connects to and forward what
    // it sends to the UI. Nothing bound this before, so a second copy's connect
    // always failed, it fell through to the branch above, and it started a whole
    // second instance — "single instance" mode never actually applied.
    //
    // Bound synchronously so a second instance launched immediately afterwards
    // finds a live endpoint instead of racing the bind (its 1s retry loop would
    // cover that, but there is no reason to rely on it).
    match ListenerOptions::new().name(name).create_sync() {
        Ok(listener) => {
            let (forward_tx, forward_rx) = tokio::sync::mpsc::unbounded_channel();
            // Publishing the channel only once the endpoint is genuinely
            // accepting keeps `take_forwarded_args() == Some` meaningful: it
            // means a second instance can actually be heard.
            let _ = FORWARD_RX.set(std::sync::Mutex::new(Some(forward_rx)));

            let spawned = std::thread::Builder::new()
                .name("pdfbull-single-instance".to_string())
                .spawn(move || {
                    use std::io::{BufRead, BufReader, Read};
                    for conn in listener.incoming() {
                        let Ok(stream) = conn else { continue };
                        let mut line = String::new();
                        if BufReader::new(stream.take(64 * 1024))
                            .read_line(&mut line)
                            .is_err()
                        {
                            continue;
                        }
                        match serde_json::from_str::<Vec<String>>(line.trim()) {
                            Ok(args) => {
                                if forward_tx.send(args).is_err() {
                                    // The UI dropped the receiver; nothing left to forward to.
                                    break;
                                }
                            }
                            Err(e) => {
                                tracing::warn!("Ignoring malformed single-instance payload: {e}");
                            }
                        }
                    }
                });
            if let Err(e) = spawned {
                tracing::warn!("Could not start the single-instance listener thread: {e}");
            }
        }
        Err(e) => {
            tracing::warn!(
                "Could not bind the single-instance socket; file arguments from a \
                 second launch will not be forwarded: {e}"
            );
        }
    }

    Ok(false)
}

pub fn setup_jump_list(paths: &[String]) {
    for path in paths {
        let path_u16: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
        // SAFETY: path_u16 is a null-terminated UTF-16 wide string passed as a valid
        // pointer to SHAddToRecentDocs with SHARD_PATHW flag.
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

    let mut data: u32 = 0;
    let mut data_len = std::mem::size_of::<u32>() as u32;
    // SAFETY: RegGetValueW is invoked with compile-time static null-terminated wide strings
    // and valid stack pointers to `data` (u32) and its length `data_len`.
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
