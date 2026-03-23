#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![allow(
    clippy::struct_excessive_bools,
    clippy::too_many_lines,
    clippy::missing_panics_doc,
    clippy::missing_errors_doc,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::cast_lossless,
    clippy::must_use_candidate,
    clippy::needless_pass_by_value,
    clippy::elidable_lifetime_names,
    clippy::option_if_let_else,
    clippy::map_unwrap_or,
    clippy::match_wildcard_for_single_variants,
    clippy::unused_self,
    clippy::manual_string_new,
    clippy::ignored_unit_patterns,
    clippy::branches_sharing_code,
    clippy::implicit_clone,
    clippy::default_trait_access
)]

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
pub mod platform;

pub fn main() -> iced::Result {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    human_panic::setup_panic!();

    let args: Vec<String> = std::env::args().collect();
    
    // Feature 10: Deep Windows Integration (Single Instance Mode)
    if let Ok(is_secondary) = platform::ensure_single_instance(&args) {
        if is_secondary {
            tracing::info!("Sent arguments to main instance. Exiting.");
            return Ok(());
        }
    }


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
