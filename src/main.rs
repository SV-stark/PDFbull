#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

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
pub mod ui_metadata;
pub mod ui_settings;
pub mod ui_welcome;
pub mod update;

pub fn main() -> iced::Result {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    human_panic::setup_panic!();

    let icon = iced::window::icon::from_file_data(include_bytes!("../PDFbull.png"), None).ok();

    iced::application("PDFbull", app::PdfBullApp::update, app::PdfBullApp::view)
        .font(include_bytes!("../src/assets/fonts/Inter-Regular.ttf"))
        .font(include_bytes!("../src/assets/fonts/Inter-Bold.ttf"))
        .font(include_bytes!("../src/assets/fonts/lucide.ttf"))
        .theme(|app: &app::PdfBullApp| match app.settings.theme {
            crate::models::AppTheme::Light => iced::Theme::Light,
            _ => iced::Theme::Dark,
        })
        .subscription(app::PdfBullApp::subscription)
        .window(iced::window::Settings {
            icon,
            exit_on_close_request: false,
            ..Default::default()
        })
        .run()
}
