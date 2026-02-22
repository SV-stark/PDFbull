#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

pub mod app;
pub mod commands;
pub mod engine;
pub mod message;
pub mod models;
pub mod pdf_engine;
pub mod storage;
pub mod ui;
pub mod ui_document;
pub mod ui_keyboard_help;
pub mod ui_settings;
pub mod ui_welcome;
pub mod update;

pub fn main() -> iced::Result {
    std::panic::set_hook(Box::new(|panic_info| {
        let msg = if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = panic_info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "Unknown panic".to_string()
        };
        
        let location = panic_info.location()
            .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
            .unwrap_or_else(|| "unknown location".to_string());
        
        eprintln!("PANIC at {}: {}", location, msg);
        
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("pdfbull");
        let _ = std::fs::create_dir_all(&config_dir);
        let crash_log = config_dir.join("crash.log");
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let log_entry = format!("[{}] PANIC at {}: {}\n", timestamp, location, msg);
        let _ = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&crash_log)
            .and_then(|mut f| std::io::Write::write_all(&mut f, log_entry.as_bytes()));
    }));
    
    let icon = match iced::window::icon::from_file_data(
        include_bytes!("../PDFbull.png"),
        None,
    ) {
        Ok(icon) => Some(icon),
        Err(_) => None,
    };

    iced::application(app::PdfBullApp::default, app::PdfBullApp::update, app::PdfBullApp::view)
        .title("PDFbull")
        .window(iced::window::Settings {
            icon,
            ..Default::default()
        })
        .run()
}
