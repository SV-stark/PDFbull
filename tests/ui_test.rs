use pdfbull::app::PdfBullApp;
use pdfbull::message::Message;
use pdfbull::pdf_engine::{DocumentStore, create_render_cache};
use pdfium_render::prelude::*;

#[test]
fn test_sidebar_toggle() {
    let mut app = PdfBullApp::default();

    // Check if sidebar is hidden initially
    let at = std::time::Instant::now();
    let sidebar_width: f32 = app.sidebar_animation.interpolate_with(|v| v, at);
    let sidebar_width = sidebar_width * 280.0; 
    assert!(sidebar_width < 0.1);

    // Toggle sidebar
    app.update(Message::ToggleSidebar);

    // In a headless test, we can check if the internal state updated
    assert!(app.show_sidebar);

    // After toggling, the animation value will start changing on next frame
    // but here we just check the boolean state.
}

#[test]
fn test_ui_initial_state() {
    let app = PdfBullApp::default();
    assert!(!app.show_settings);
    assert!(!app.show_sidebar);
    assert!(!app.is_fullscreen);
    assert!(app.tabs.is_empty());
}
