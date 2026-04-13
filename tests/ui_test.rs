use iced_test::simulator;
use pdfbull::app::PdfBullApp;
use pdfbull::message::Message;

#[test]
fn test_sidebar_toggle() {
    let mut app = PdfBullApp::default();

    // Simulate initial view
    let mut ui = simulator(app.view());

    // Check if sidebar is hidden initially (assuming 0 width at start)
    let sidebar_width = app
        .sidebar_animation
        .interpolate(0.0, 280.0, std::time::Instant::now());
    assert!(sidebar_width < 0.1);

    // Toggle sidebar
    app.update(Message::ToggleSidebar);

    // In a headless test, we can check if the internal state updated
    assert!(app.show_sidebar);

    // Note: Animation interpolation depends on time,
    // in a real test we'd mock Instant or wait.
}
