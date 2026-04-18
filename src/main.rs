use pdfbull::app;
use pdfbull::platform;

fn main() -> iced::Result {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
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

    let icon = iced::window::icon::from_file_data(include_bytes!("../PDFbull.png"), None).ok();

    iced::application(
        app::PdfBullApp::default,
        app::PdfBullApp::update,
        app::PdfBullApp::view,
    )
    .title("PDFbull")
    .font(include_bytes!("../src/assets/fonts/Inter-Regular.ttf"))
    .font(include_bytes!("../src/assets/fonts/Inter-Bold.ttf"))
    .font(include_bytes!("../src/assets/fonts/lucide.ttf"))
    .font(include_bytes!("../src/assets/fonts/Phosphor.ttf"))
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
    .run()
}
