use gpui_kit::RenderImage;
use pdfbull::commands::PdfCommand;
use pdfbull::engine::spawn_engine_thread;
use pdfbull::models::next_doc_id;
use pdfbull::pdf_engine::{RenderFilter, RenderOptions, RenderQuality};
use pdfbull::ui_gpui::app_view::inspect_pdf_geometry;
use pdfbull::ui_gpui::canvas::DocumentViewport;
use pdfbull::ui_gpui::dialogs::{ActiveDialog, DialogsState};
use pdfbull::ui_gpui::log_console::LogConsoleState;
use pdfbull::ui_gpui::ribbon::{AnnotationTool, PageLayoutMode, RibbonState, RibbonTab};
use pdfbull::ui_gpui::sidebar::{SidebarMode, SidebarState};
use pdfbull::ui_gpui::tabs::{DocumentTab, TabsState};
use pdfbull::ui_gpui::welcome::WelcomeState;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

#[test]
fn test_tabs_state_management() {
    let mut tabs_state = TabsState::new();
    assert!(tabs_state.tabs.is_empty());
    assert_eq!(tabs_state.active_tab_index, 0);

    let doc_id1 = next_doc_id();
    let doc_id2 = next_doc_id();

    // Add multiple tabs
    tabs_state.tabs.push(DocumentTab {
        id: 0,
        doc_id: Some(doc_id1),
        title: "doc1.pdf".to_string(),
        path: Some(PathBuf::from("doc1.pdf")),
        is_modified: false,
    });
    tabs_state.tabs.push(DocumentTab {
        id: 1,
        doc_id: Some(doc_id2),
        title: "doc2.pdf".to_string(),
        path: Some(PathBuf::from("doc2.pdf")),
        is_modified: true,
    });
    tabs_state.tabs.push(DocumentTab {
        id: 2,
        doc_id: None,
        title: "Sample.pdf".to_string(),
        path: None,
        is_modified: false,
    });

    assert_eq!(tabs_state.tabs.len(), 3);
    assert_eq!(tabs_state.tabs[0].doc_id, Some(doc_id1));
    assert_eq!(tabs_state.tabs[1].doc_id, Some(doc_id2));
    assert_eq!(tabs_state.tabs[2].doc_id, None);
    assert!(tabs_state.tabs[1].is_modified);

    // Switch active tab
    tabs_state.active_tab_index = 1;
    assert_eq!(tabs_state.active_tab_index, 1);

    // Close middle tab
    tabs_state.tabs.remove(1);
    assert_eq!(tabs_state.tabs.len(), 2);
    assert_eq!(tabs_state.tabs[0].title, "doc1.pdf");
    assert_eq!(tabs_state.tabs[1].title, "Sample.pdf");

    // Close others
    let kept = tabs_state.tabs[0].clone();
    tabs_state.tabs = vec![kept];
    tabs_state.active_tab_index = 0;
    assert_eq!(tabs_state.tabs.len(), 1);
    assert_eq!(tabs_state.tabs[0].title, "doc1.pdf");
}

#[test]
fn test_sidebar_state_and_thumbnails() {
    let mut sidebar = SidebarState::new();
    assert!(sidebar.is_open);
    assert_eq!(sidebar.mode, SidebarMode::Thumbnails);
    assert_eq!(sidebar.width, 250.0);
    assert!(sidebar.thumbnails.is_empty());

    // Toggle sidebar visibility
    sidebar.is_open = false;
    assert!(!sidebar.is_open);
    sidebar.is_open = true;
    assert!(sidebar.is_open);

    // Mode switching across all available modes
    let modes = [
        SidebarMode::Thumbnails,
        SidebarMode::Bookmarks,
        SidebarMode::Annotations,
        SidebarMode::Search,
        SidebarMode::Attachments,
        SidebarMode::Layers,
    ];

    for mode in modes {
        sidebar.mode = mode;
        assert_eq!(sidebar.mode, mode);
    }

    // Insert thumbnail image
    let raw_rgba = vec![255u8; 64 * 64 * 4];
    let buf = image::RgbaImage::from_raw(64, 64, raw_rgba).expect("valid buffer");
    let frame = image::Frame::new(buf);
    let render_image = Arc::new(RenderImage::new(vec![frame]));

    sidebar.thumbnails.insert(0, render_image.clone());
    assert_eq!(sidebar.thumbnails.len(), 1);
    assert!(sidebar.thumbnails.contains_key(&0));
}

#[test]
fn test_ribbon_state_and_tools() {
    let mut ribbon = RibbonState::new();
    assert_eq!(ribbon.active_tab, RibbonTab::Home);
    assert_eq!(ribbon.active_tool, AnnotationTool::Pointer);
    assert_eq!(ribbon.layout_mode, PageLayoutMode::Continuous);
    assert!(!ribbon.standalone_cover);
    assert!(!ribbon.midnight_mode);

    // Switch all ribbon tabs
    let tabs = [
        RibbonTab::Home,
        RibbonTab::View,
        RibbonTab::Annotate,
        RibbonTab::Tools,
        RibbonTab::Convert,
    ];
    for tab in tabs {
        ribbon.active_tab = tab;
        assert_eq!(ribbon.active_tab, tab);
    }

    // Switch all annotation tools
    let tools = [
        AnnotationTool::Pointer,
        AnnotationTool::Highlight,
        AnnotationTool::Underline,
        AnnotationTool::Strikeout,
        AnnotationTool::Rectangle,
        AnnotationTool::Circle,
        AnnotationTool::Line,
        AnnotationTool::Arrow,
        AnnotationTool::Ink,
        AnnotationTool::StickyNote,
        AnnotationTool::Text,
        AnnotationTool::Redact,
    ];
    for tool in tools {
        ribbon.active_tool = tool;
        assert_eq!(ribbon.active_tool, tool);
    }

    // Switch color palette
    ribbon.active_color = [0.2, 0.8, 0.4, 0.9];
    assert_eq!(ribbon.active_color, [0.2, 0.8, 0.4, 0.9]);

    // Page layout & display modes
    ribbon.layout_mode = PageLayoutMode::TwoPageSpread;
    ribbon.standalone_cover = true;
    ribbon.midnight_mode = true;
    assert_eq!(ribbon.layout_mode, PageLayoutMode::TwoPageSpread);
    assert!(ribbon.standalone_cover);
    assert!(ribbon.midnight_mode);
}

#[test]
fn test_document_viewport_geometry_and_zoom() {
    let mut viewport = DocumentViewport::new();
    assert_eq!(viewport.zoom, 1.0);
    assert_eq!(viewport.rotation, 0);
    assert_eq!(viewport.current_page, 0);
    assert_eq!(viewport.total_pages, 1);
    assert_eq!(viewport.page_width, 595.0);
    assert_eq!(viewport.page_height, 842.0);
    assert!(viewport.rendered_pages.is_empty());

    // Zoom in with clamping
    viewport.zoom = (viewport.zoom + 0.1).clamp(0.25, 5.0);
    assert!((viewport.zoom - 1.1).abs() < 1e-5);

    // Zoom out with clamping
    viewport.zoom = (viewport.zoom - 0.2).clamp(0.25, 5.0);
    assert!((viewport.zoom - 0.9).abs() < 1e-5);

    // Minimum zoom boundary check
    viewport.zoom = (viewport.zoom - 10.0).clamp(0.25, 5.0);
    assert_eq!(viewport.zoom, 0.25);

    // Maximum zoom boundary check
    viewport.zoom = (viewport.zoom + 10.0).clamp(0.25, 5.0);
    assert_eq!(viewport.zoom, 5.0);

    // Reset zoom
    viewport.zoom = 1.0;
    assert_eq!(viewport.zoom, 1.0);

    // Rotation cycling (0 -> 90 -> 180 -> 270 -> 0)
    viewport.rotation = (viewport.rotation + 90) % 360;
    assert_eq!(viewport.rotation, 90);
    viewport.rotation = (viewport.rotation + 90) % 360;
    assert_eq!(viewport.rotation, 180);
    viewport.rotation = (viewport.rotation + 90) % 360;
    assert_eq!(viewport.rotation, 270);
    viewport.rotation = (viewport.rotation + 90) % 360;
    assert_eq!(viewport.rotation, 0);

    // Layout modes
    viewport.layout_mode = PageLayoutMode::SinglePage;
    assert_eq!(viewport.layout_mode, PageLayoutMode::SinglePage);
    viewport.layout_mode = PageLayoutMode::TwoPageSpread;
    assert_eq!(viewport.layout_mode, PageLayoutMode::TwoPageSpread);
    viewport.layout_mode = PageLayoutMode::Continuous;
    assert_eq!(viewport.layout_mode, PageLayoutMode::Continuous);
}

#[test]
fn test_render_image_creation_and_caching() {
    let mut viewport = DocumentViewport::new();
    let mut thumbnails: HashMap<usize, Arc<RenderImage>> = HashMap::new();

    // Create simulated 200x300 RGBA image buffer
    let width = 200u32;
    let height = 300u32;
    let raw_rgba = vec![128u8; (width * height * 4) as usize];

    let buf = image::RgbaImage::from_raw(width, height, raw_rgba).expect("valid buffer");
    let frame = image::Frame::new(buf);
    let render_image = Arc::new(RenderImage::new(vec![frame]));

    // Cache in viewport
    viewport.rendered_pages.insert(0, render_image.clone());
    assert_eq!(viewport.rendered_pages.len(), 1);
    assert!(viewport.rendered_pages.contains_key(&0));

    // Cache in thumbnails
    thumbnails.insert(0, render_image.clone());
    assert_eq!(thumbnails.len(), 1);
    assert!(thumbnails.contains_key(&0));

    // Cache invalidation on zoom change
    viewport.rendered_pages.clear();
    assert!(viewport.rendered_pages.is_empty());
}

#[test]
fn test_smooth_zoom_progressive_rendering() {
    let mut viewport = DocumentViewport::new();
    assert!(viewport.rendered_pages.is_empty());
    assert!(viewport.rendered_zoom.is_empty());

    // Create a 200x300 simulated image for initial zoom (1.0)
    let raw_rgba = vec![255u8; (200 * 300 * 4) as usize];
    let buf = image::RgbaImage::from_raw(200, 300, raw_rgba).expect("valid buffer");
    let render_image_1 = Arc::new(RenderImage::new(vec![image::Frame::new(buf)]));

    // Page 0 rendered at zoom 1.0
    viewport.rendered_pages.insert(0, render_image_1.clone());
    viewport.rendered_zoom.insert(0, 1.0);

    // User zooms to 1.5
    viewport.zoom = 1.5;

    // With smooth zoom: rendered_pages is NOT cleared, so the existing texture is displayed scaled on GPU (no white screen!)
    assert!(viewport.rendered_pages.contains_key(&0));
    assert_eq!(viewport.rendered_pages.len(), 1);

    // However, rendered_zoom still indicates it was rendered at 1.0, triggering background re-render
    let current_rendered_zoom = *viewport.rendered_zoom.get(&0).unwrap();
    assert!((current_rendered_zoom - viewport.zoom).abs() > 0.01);

    // When background re-render at 1.5 completes, texture and zoom are seamlessly updated
    let raw_rgba_zoomed = vec![200u8; (300 * 450 * 4) as usize];
    let buf_zoomed = image::RgbaImage::from_raw(300, 450, raw_rgba_zoomed).expect("valid buffer");
    let render_image_zoomed = Arc::new(RenderImage::new(vec![image::Frame::new(buf_zoomed)]));

    viewport.rendered_pages.insert(0, render_image_zoomed);
    viewport.rendered_zoom.insert(0, 1.5);

    // Now page 0 matches the active zoom level
    assert!((*viewport.rendered_zoom.get(&0).unwrap() - viewport.zoom).abs() < 0.01);

    // Tab switch or doc close clears both
    viewport.rendered_pages.clear();
    viewport.rendered_zoom.clear();
    assert!(viewport.rendered_pages.is_empty());
    assert!(viewport.rendered_zoom.is_empty());
}

#[test]
fn test_inspect_pdf_geometry_real_pdf() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fixture_pdf = manifest_dir.join("tests").join("test_document.pdf");

    if fixture_pdf.exists() {
        let geom = inspect_pdf_geometry(&fixture_pdf);
        assert!(geom.is_some(), "Should parse geometry from test PDF");
        let (page_count, width, height) = geom.unwrap();
        assert!(page_count >= 1, "Page count must be at least 1");
        assert!(width > 100.0, "Page width must be positive pt value");
        assert!(height > 100.0, "Page height must be positive pt value");
    }

    // Non-existent PDF returns None
    let fake_path = PathBuf::from("non_existent_dummy_file_12345.pdf");
    assert!(inspect_pdf_geometry(&fake_path).is_none());
}

#[test]
fn test_dialogs_overlay_state() {
    let mut dialogs = DialogsState::new();
    assert!(dialogs.active.is_none());

    let all_dialogs = [
        ActiveDialog::Settings,
        ActiveDialog::Password,
        ActiveDialog::Watermark,
        ActiveDialog::HeaderFooter,
        ActiveDialog::Security,
        ActiveDialog::Signature,
        ActiveDialog::PageOrganizer,
    ];

    for dialog in all_dialogs {
        dialogs.active = Some(dialog.clone());
        assert_eq!(dialogs.active, Some(dialog));
    }

    // Close dialog
    dialogs.active = None;
    assert!(dialogs.active.is_none());
}

#[test]
fn test_welcome_state() {
    let mut welcome = WelcomeState::new();
    assert!(welcome.recent_files.is_empty());

    welcome.recent_files.push(PathBuf::from("recent1.pdf"));
    welcome.recent_files.push(PathBuf::from("recent2.pdf"));
    assert_eq!(welcome.recent_files.len(), 2);
    assert_eq!(welcome.recent_files[0], PathBuf::from("recent1.pdf"));
    assert_eq!(welcome.recent_files[1], PathBuf::from("recent2.pdf"));
}

#[test]
fn test_log_console_state() {
    let mut log_console = LogConsoleState::new();
    assert!(!log_console.is_open);
    assert_eq!(log_console.height, 180.0);
    assert!(log_console.filter_level.is_none());

    // Toggle open
    log_console.is_open = true;
    assert!(log_console.is_open);

    // Set filter levels
    let levels = ["INFO", "WARN", "ERROR", "DEBUG"];
    for lvl in levels {
        log_console.filter_level = Some(lvl);
        assert_eq!(log_console.filter_level, Some(lvl));
    }
}

#[tokio::test]
async fn test_engine_rasterization_roundtrip() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fixture_pdf = manifest_dir.join("tests").join("test_document.pdf");

    if !fixture_pdf.exists() {
        return;
    }

    let engine = spawn_engine_thread(16, 128);
    let doc_id = next_doc_id();

    // 1. Open document in engine
    let (open_tx, open_rx) = tokio::sync::oneshot::channel();
    let path_str = fixture_pdf.to_string_lossy().to_string();
    engine
        .cmd_tx
        .send(PdfCommand::Open(path_str, None, doc_id, open_tx))
        .await
        .expect("send Open command");

    let open_res = open_rx.await.expect("open response channel");
    assert!(open_res.is_ok(), "Document should open successfully");
    let open_data = open_res.unwrap();
    assert!(open_data.page_count >= 1);

    // 2. Render Page 0
    let (render_tx, render_rx) = tokio::sync::oneshot::channel();
    let options = RenderOptions {
        scale: 1.0,
        rotation: 0,
        filter: RenderFilter::None,
        auto_crop: false,
        quality: RenderQuality::Low,
    };

    engine
        .cmd_tx
        .send(PdfCommand::Render(doc_id, 0, options, render_tx))
        .await
        .expect("send Render command");

    let render_res = render_rx.await.expect("render response channel");
    assert!(render_res.is_ok(), "Page 0 should render successfully");
    let result = render_res.unwrap();
    assert!(result.width > 0, "Rendered width must be > 0");
    assert!(result.height > 0, "Rendered height must be > 0");
    assert_eq!(
        result.data.len(),
        (result.width * result.height * 4) as usize,
        "Buffer length must equal width * height * 4"
    );

    // 3. Convert into RenderImage with BGRA swap
    let mut raw = result.data.to_vec();
    pdfbull::ui_gpui::canvas::convert_rgba_to_bgra(&mut raw);
    let buf = image::RgbaImage::from_raw(result.width, result.height, raw)
        .expect("valid RGBA buffer");
    let frame = image::Frame::new(buf);
    let render_image = Arc::new(RenderImage::new(vec![frame]));
    assert_eq!(Arc::strong_count(&render_image), 1);

    // 4. Close document
    engine
        .cmd_tx
        .send(PdfCommand::Close(doc_id))
        .await
        .expect("send Close command");
}

#[test]
fn test_bgra_channel_swap() {
    // Pure Red (RGBA: [255, 0, 0, 255])
    let mut red_pixel = vec![255, 0, 0, 255];
    pdfbull::ui_gpui::canvas::convert_rgba_to_bgra(&mut red_pixel);
    // In BGRA, Blue is at index 0, Red is at index 2
    assert_eq!(red_pixel, vec![0, 0, 255, 255], "Red should swap with Blue");

    // Pure Blue (RGBA: [0, 0, 255, 255])
    let mut blue_pixel = vec![0, 0, 255, 255];
    pdfbull::ui_gpui::canvas::convert_rgba_to_bgra(&mut blue_pixel);
    assert_eq!(blue_pixel, vec![255, 0, 0, 255], "Blue should swap with Red");

    // Pure Green (RGBA: [0, 255, 0, 255])
    let mut green_pixel = vec![0, 255, 0, 255];
    pdfbull::ui_gpui::canvas::convert_rgba_to_bgra(&mut green_pixel);
    assert_eq!(green_pixel, vec![0, 255, 0, 255], "Green channel should remain at index 1");

    // White (RGBA: [255, 255, 255, 255])
    let mut white_pixel = vec![255, 255, 255, 255];
    pdfbull::ui_gpui::canvas::convert_rgba_to_bgra(&mut white_pixel);
    assert_eq!(white_pixel, vec![255, 255, 255, 255], "White should stay white");

    // Black (RGBA: [0, 0, 0, 255])
    let mut black_pixel = vec![0, 0, 0, 255];
    pdfbull::ui_gpui::canvas::convert_rgba_to_bgra(&mut black_pixel);
    assert_eq!(black_pixel, vec![0, 0, 0, 255], "Black should stay black");
}

#[test]
fn test_continuous_scroll_handle() {
    let mut viewport = DocumentViewport::new();
    viewport.total_pages = 20;
    viewport.layout_mode = PageLayoutMode::Continuous;

    assert_eq!(viewport.layout_mode, PageLayoutMode::Continuous);
    assert_eq!(viewport.current_page, 0);

    // Verify scroll handle exists and can be scrolled to a target item
    viewport.scroll_handle.scroll_to_item(5);
    viewport.current_page = 5;
    assert_eq!(viewport.current_page, 5);
}

#[test]
fn test_ctrl_wheel_zoom_math() {
    let mut zoom = 1.0f32;

    // Simulate Zoom In (delta > 0)
    let zoom_in_factor = 1.1f32;
    zoom = (zoom * zoom_in_factor).clamp(0.25, 5.0);
    assert!((zoom - 1.1).abs() < 1e-4);

    // Simulate Zoom Out (delta < 0)
    let zoom_out_factor = 1.0 / 1.1f32;
    zoom = (zoom * zoom_out_factor).clamp(0.25, 5.0);
    assert!((zoom - 1.0).abs() < 1e-4);

    // Test upper clamp
    let excessive_zoom = 100.0f32;
    assert_eq!(excessive_zoom.clamp(0.25, 5.0), 5.0);

    // Test lower clamp
    let tiny_zoom = 0.01f32;
    assert_eq!(tiny_zoom.clamp(0.25, 5.0), 0.25);
}

#[test]
fn test_drag_selection_and_text_extraction() {
    let mut viewport = DocumentViewport::new();
    viewport.zoom = 1.0;
    viewport.total_pages = 3;

    // Populate simulated TextItems for Page 0
    let items = vec![
        pdfbull::models::TextItem {
            text: "Hello".to_string(),
            x: 50.0,
            y: 100.0,
            width: 40.0,
            height: 12.0,
        },
        pdfbull::models::TextItem {
            text: "World".to_string(),
            x: 100.0,
            y: 100.0,
            width: 45.0,
            height: 12.0,
        },
        pdfbull::models::TextItem {
            text: "Unselected".to_string(),
            x: 50.0,
            y: 300.0,
            width: 80.0,
            height: 12.0,
        },
    ];
    viewport.text_cache.insert(0, items.clone());

    // Simulate Drag selection on Page 0 covering "Hello" and "World"
    viewport.is_selecting = true;
    viewport.selection_page = Some(0);
    viewport.selection_start = Some(gpui_kit::Point::new(gpui_kit::px(40.0), gpui_kit::px(90.0)));
    viewport.selection_end = Some(gpui_kit::Point::new(gpui_kit::px(160.0), gpui_kit::px(120.0)));

    let start = viewport.selection_start.unwrap();
    let end = viewport.selection_end.unwrap();
    let min_x = (start.x.min(end.x) / gpui_kit::px(1.0)) / viewport.zoom;
    let max_x = (start.x.max(end.x) / gpui_kit::px(1.0)) / viewport.zoom;
    let min_y = (start.y.min(end.y) / gpui_kit::px(1.0)) / viewport.zoom;
    let max_y = (start.y.max(end.y) / gpui_kit::px(1.0)) / viewport.zoom;

    let mut matched = Vec::new();
    for item in &items {
        let ix2 = item.x + item.width;
        let iy2 = item.y + item.height;
        if ix2 >= min_x && item.x <= max_x && iy2 >= min_y && item.y <= max_y {
            matched.push(item.text.clone());
        }
    }

    assert_eq!(matched, vec!["Hello", "World"]);
    let combined = matched.join(" ");
    viewport.selected_text = Some(combined);
    assert_eq!(viewport.selected_text.as_deref(), Some("Hello World"));
}

