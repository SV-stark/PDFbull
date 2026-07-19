#![allow(clippy::field_reassign_with_default)]
use pdfbull::app::PdfBullApp;
use pdfbull::message::Message;

#[tokio::test(flavor = "current_thread")]
async fn test_sidebar_toggle() {
    let mut app = PdfBullApp::default();

    // Check if sidebar is hidden initially
    let at = std::time::Instant::now();
    let sidebar_width: f32 = app.sidebar_animation.interpolate_with(|v| v, at);
    let sidebar_width = sidebar_width * 280.0;
    assert!(sidebar_width < 0.1);

    // Toggle sidebar
    let _ = app.update(Message::ToggleSidebar);

    // In a headless test, we can check if the internal state updated
    assert!(app.show_sidebar);

    // After toggling, the animation value will start changing on next frame
    // but here we just check the boolean state.
}

#[tokio::test(flavor = "current_thread")]
async fn test_ui_initial_state() {
    let app = PdfBullApp::default();
    assert!(!app.show_settings);
    assert!(!app.show_sidebar);
    assert!(!app.is_fullscreen);
    assert!(app.tabs.is_empty());
}

#[tokio::test(flavor = "current_thread")]
async fn test_settings_toggle() {
    let mut app = PdfBullApp::default();
    assert!(!app.show_settings);

    // Open settings
    let _ = app.update(Message::OpenSettings);
    assert!(app.show_settings);

    // Close settings
    let _ = app.update(Message::CloseSettings);
    assert!(!app.show_settings);
}

#[tokio::test(flavor = "current_thread")]
async fn test_fullscreen_toggle() {
    let mut app = PdfBullApp::default();
    assert!(!app.is_fullscreen);

    // Toggle fullscreen
    let _ = app.update(Message::ToggleFullscreen);
    assert!(app.is_fullscreen);

    // Toggle back
    let _ = app.update(Message::ToggleFullscreen);
    assert!(!app.is_fullscreen);
}

#[tokio::test(flavor = "current_thread")]
async fn test_keyboard_help_toggle() {
    let mut app = PdfBullApp::default();
    assert!(!app.show_keyboard_help);

    let _ = app.update(Message::ToggleKeyboardHelp);
    assert!(app.show_keyboard_help);

    let _ = app.update(Message::ToggleKeyboardHelp);
    assert!(!app.show_keyboard_help);
}

#[tokio::test(flavor = "current_thread")]
async fn test_metadata_toggle() {
    let mut app = PdfBullApp::default();
    assert!(!app.show_metadata);

    let _ = app.update(Message::ToggleMetadata);
    assert!(app.show_metadata);

    let _ = app.update(Message::ToggleMetadata);
    assert!(!app.show_metadata);
}

#[tokio::test(flavor = "current_thread")]
async fn test_document_rotation() {
    let mut app = PdfBullApp::default();
    let tab = pdfbull::models::DocumentTab::new(std::path::PathBuf::from("test.pdf"));
    app.tabs.push(tab);
    app.active_tab = 0;

    // Initial rotation should be 0
    assert_eq!(app.current_tab().unwrap().rotation, 0);

    // Rotate clockwise
    let _ = app.update(Message::RotateClockwise);
    assert_eq!(app.current_tab().unwrap().rotation, 90);

    // Rotate clockwise again
    let _ = app.update(Message::RotateClockwise);
    assert_eq!(app.current_tab().unwrap().rotation, 180);

    // Rotate counter-clockwise
    let _ = app.update(Message::RotateCounterClockwise);
    assert_eq!(app.current_tab().unwrap().rotation, 90);

    // Rotate counter-clockwise to negative/wrap
    let _ = app.update(Message::RotateCounterClockwise);
    assert_eq!(app.current_tab().unwrap().rotation, 0);
    let _ = app.update(Message::RotateCounterClockwise);
    assert_eq!(app.current_tab().unwrap().rotation, 270);
}

#[tokio::test(flavor = "current_thread")]
#[allow(clippy::float_cmp)]
async fn test_document_zoom() {
    let mut app = PdfBullApp::default();
    let tab = pdfbull::models::DocumentTab::new(std::path::PathBuf::from("test.pdf"));
    app.tabs.push(tab);
    app.active_tab = 0;

    // Initial zoom should be 1.0
    assert_eq!(app.current_tab().unwrap().zoom, 1.0);

    // Zoom in
    let _ = app.update(Message::ZoomIn);
    assert!((app.current_tab().unwrap().zoom - 1.1).abs() < 1e-5);

    // Reset zoom
    let _ = app.update(Message::ResetZoom);
    assert_eq!(app.current_tab().unwrap().zoom, 1.0);

    // Set zoom to specific value
    let _ = app.update(Message::SetZoom(2.5));
    assert_eq!(app.current_tab().unwrap().zoom, 2.5);

    // Set zoom out
    let _ = app.update(Message::ZoomOut);
    assert!((app.current_tab().unwrap().zoom - (2.5 / 1.1)).abs() < 1e-5);
}

#[tokio::test(flavor = "current_thread")]
async fn test_open_document_flow() {
    let mut app = PdfBullApp::default();
    app.loaded = true; // Prevent loading real session/settings in test
    let test_path = std::path::PathBuf::from("tests/test_document.pdf");

    // Simulate what happens after the file is picked and loaded by the engine
    let doc_id = pdfbull::models::DocumentId(42);
    let open_res = pdfbull::models::OpenResult {
        id: doc_id,
        page_count: 5,
        page_heights: vec![800.0; 5],
        max_width: 600.0,
        outline: Vec::new(),
        links: Vec::new(),
        metadata: pdfbull::models::DocumentMetadata::default(),
    };

    // Send DocumentOpenedWithPath message
    let _ = app.update(Message::DocumentOpenedWithPath((test_path, open_res)));

    // The tab should have been created with the correct ID matching the engine's doc_id
    assert_eq!(app.tabs.len(), 1);
    assert_eq!(app.tabs[0].id, doc_id);
    assert_eq!(app.active_tab, 0);
}
