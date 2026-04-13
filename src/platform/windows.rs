use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx};
use windows::Win32::UI::Shell::{ApplicationDocumentLists, IApplicationDocumentLists};

pub fn ensure_single_instance(_args: &[String]) -> Result<bool, Box<dyn std::error::Error>> {
    Ok(false)
}

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
