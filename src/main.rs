#![allow(clippy::all, clippy::pedantic, clippy::nursery)]
use pdfbull::app;
use pdfbull::platform;

struct DualWriter {
    file: std::sync::Arc<std::sync::Mutex<std::fs::File>>,
}

impl std::io::Write for DualWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let _ = std::io::stdout().write_all(buf);
        if let Ok(mut file) = self.file.lock() {
            let _ = file.write_all(buf);
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        let _ = std::io::stdout().flush();
        if let Ok(mut file) = self.file.lock() {
            let _ = file.flush();
        }
        Ok(())
    }
}

fn main() -> iced::Result {
    let config_dir = pdfbull::storage::get_config_dir();
    let _ = std::fs::create_dir_all(&config_dir);
    let log_path = config_dir.join("pdfbull.log");
    let panic_path = config_dir.join("panic_out.log");

    let log_path_clone = log_path.clone();
    let panic_path_clone = panic_path.clone();

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
        let _ = std::fs::write(&panic_path_clone, &panic_msg);
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path_clone)
        {
            use std::io::Write;
            let _ = f.write_all(panic_msg.as_bytes());
        }
    }));

    let log_file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&log_path)
        .expect("Failed to open pdfbull.log");
    let shared_file = std::sync::Arc::new(std::sync::Mutex::new(log_file));

    let file_clone = shared_file.clone();
    let make_writer = move || DualWriter {
        file: file_clone.clone(),
    };

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
        )
        .with_writer(make_writer)
        .init();

    human_panic::setup_panic!();

    let args: Vec<String> = std::env::args().collect();

    // Feature 10: Deep Windows Integration (Single Instance Mode)
    if let Ok(is_secondary) = platform::ensure_single_instance(&args)
        && is_secondary
    {
        tracing::info!("Sent arguments to main instance. Exiting.");
        return Ok(());
    }

    let icon = iced::window::icon::from_file_data(
        include_bytes!("../PDFbull.ico"),
        Some(image::ImageFormat::Ico),
    )
    .ok();

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
        pdfbull::models::AppTheme::Light | pdfbull::models::AppTheme::System => iced::Theme::Light,
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
