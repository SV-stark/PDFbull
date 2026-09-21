#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use pdfbull::app;
use pdfbull::platform;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

fn main() -> iced::Result {
    let config_dir = pdfbull::storage::get_config_dir();
    let _ = std::fs::create_dir_all(&config_dir);
    let log_path = config_dir.join("pdfbull.log");
    let panic_path = config_dir.join("panic_out.log");

    let log_path_clone = log_path.clone();
    let panic_path_clone = panic_path.clone();

    let prev_panic_hook = std::panic::take_hook();
    human_panic::setup_panic!();
    let human_panic_hook = std::panic::take_hook();

    std::panic::set_hook(Box::new(move |info| {
        let msg = if let Some(s) = info.payload().downcast_ref::<&str>() {
            *s
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            s.as_str()
        } else {
            "unknown panic"
        };
        let location = info
            .location()
            .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
            .unwrap_or_else(|| "unknown location".to_string());
        let backtrace = std::backtrace::Backtrace::capture();
        let panic_msg = format!(
            "PANIC: {} at {}\nBacktrace:\n{:?}",
            msg, location, backtrace
        );
        tracing::error!(target: "panic", "PANIC at {}: {}\nBacktrace:\n{:?}", location, msg, backtrace);
        let _ = std::fs::write(&panic_path_clone, &panic_msg);
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path_clone)
        {
            use std::io::Write;
            let _ = f.write_all(panic_msg.as_bytes());
        }
        human_panic_hook(info);
        prev_panic_hook(info);
    }));

    let file_appender = tracing_appender::rolling::never(&config_dir, "pdfbull.log");
    let (non_blocking_file, guard) = tracing_appender::non_blocking(file_appender);
    let _guard = Box::leak(Box::new(guard));

    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info,pdfbull=debug"));
    let file_layer = tracing_subscriber::fmt::layer()
        .with_writer(non_blocking_file)
        .with_ansi(false);
    let ring_layer = pdfbull::logging::RingBufferLayer;

    #[cfg(debug_assertions)]
    {
        let stdout_layer = tracing_subscriber::fmt::layer().with_writer(std::io::stdout);
        tracing_subscriber::registry()
            .with(env_filter)
            .with(ring_layer)
            .with(file_layer)
            .with(stdout_layer)
            .init();
    }

    #[cfg(not(debug_assertions))]
    {
        tracing_subscriber::registry()
            .with(env_filter)
            .with(ring_layer)
            .with(file_layer)
            .init();
    }

    let args: Vec<String> = std::env::args().collect();

    // Feature 10: Deep Windows Integration (Single Instance Mode)
    if let Ok(is_secondary) = platform::ensure_single_instance(&args)
        && is_secondary
    {
        tracing::info!("Sent arguments to main instance. Exiting.");
        return Ok(());
    }

    let use_iced = args.iter().any(|arg| arg == "--iced");
    if !use_iced {
        tracing::info!("Starting PDFbull with GPUI-Kit presentation engine...");
        pdfbull::ui_gpui::run_gpui_app();
        return Ok(());
    }

    const ICON_RGBA: &[u8] = include_bytes!("assets/icon_32x32.rgba");
    let icon = iced::window::icon::from_rgba(ICON_RGBA.to_vec(), 32, 32).ok();

    let res = iced::application(
        app::PdfBullApp::default,
        app::PdfBullApp::update,
        app::PdfBullApp::view,
    )
    .title("PDFbull")
    .font(include_bytes!("../src/assets/fonts/Inter-Regular.ttf"))
    .font(include_bytes!("../src/assets/fonts/Inter-Bold.ttf"))
    .font(include_bytes!("../src/assets/fonts/lucide.ttf"))
    .theme(|app: &app::PdfBullApp| match app.settings.theme {
        pdfbull::models::AppTheme::Dark => iced::Theme::Dark,
        pdfbull::models::AppTheme::Light => iced::Theme::Light,
        pdfbull::models::AppTheme::System => {
            if pdfbull::platform::is_system_dark_mode() {
                iced::Theme::Dark
            } else {
                iced::Theme::Light
            }
        }
    })
    .subscription(app::PdfBullApp::subscription)
    .window(iced::window::Settings {
        icon,
        exit_on_close_request: false,
        ..Default::default()
    })
    .run();

    if let Err(ref e) = res {
        tracing::error!("Iced application error: {:?}", e);
    }
    res
}
