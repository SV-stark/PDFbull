use interprocess::local_socket::{LocalSocketListener, LocalSocketStream, NameTypeSupport};
use std::io::{BufRead, BufReader, Write};
use windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};
use windows::Win32::UI::Shell::{
    IApplicationDocumentLists, ApplicationDocumentLists,
};

const PIPE_NAME: &str = "pdfbull-single-instance-pipe.sock";

/// Checks if another instance is running. If it is, sends our arguments to it and returns true.
/// If not, it sets up a listener and returns false.
pub fn ensure_single_instance(args: &[String]) -> Result<bool, Box<dyn std::error::Error>> {
    let name = if NameTypeSupport::query() == NameTypeSupport::OnlyPaths {
        format!("/tmp/{PIPE_NAME}")
    } else {
        format!("@{PIPE_NAME}")
    };

    // Try to connect to an existing instance
    if let Ok(mut stream) = LocalSocketStream::connect(name.clone()) {
        tracing::info!("Found existing instance, sending args...");
        let payload = args.join("\x00") + "\n";
        stream.write_all(payload.as_bytes())?;
        return Ok(true);
    }

    // If we get here, no other instance is listening (or it's broken). Start our own listener.
    tracing::info!("Starting as main instance, listening for other launches...");
    let listener = LocalSocketListener::bind(name)?;

    // Spawn a background thread to listen for future connections
    std::thread::spawn(move || {
        for conn in listener.incoming() {
            match conn {
                Ok(stream) => {
                    let mut reader = BufReader::new(stream);
                    let mut buffer = String::new();
                    if reader.read_line(&mut buffer).is_ok() {
                        let incoming_args: Vec<String> =
                            buffer.trim().split('\x00').map(|s| s.to_string()).collect();
                        
                        tracing::info!("Received args from another instance: {:?}", incoming_args);
                    }
                }
                Err(e) => tracing::error!("Error accepting connection: {}", e),
            }
        }
    });

    Ok(false)
}

/// Registers files with the Windows Jump List
pub fn setup_jump_list(paths: &[String]) {
    if paths.is_empty() {
        return;
    }

    let _ = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };

    unsafe {
        let app_doc_lists: Result<IApplicationDocumentLists, _> =
            windows::Win32::System::Com::CoCreateInstance(
                &ApplicationDocumentLists,
                None,
                windows::Win32::System::Com::CLSCTX_INPROC_SERVER,
            );

        if app_doc_lists.is_ok() {
            tracing::info!("Windows Jump List COM object created.");
        }
    }
}
