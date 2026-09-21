use pdfbull::ui_gpui::canvas::DocumentViewport;
use pdfbull::ui_gpui::dialogs::{ActiveDialog, DialogsState};
use pdfbull::ui_gpui::log_console::LogConsoleState;
use pdfbull::ui_gpui::ribbon::{AnnotationTool, PageLayoutMode, RibbonState, RibbonTab};
use pdfbull::ui_gpui::sidebar::{SidebarMode, SidebarState};
use pdfbull::ui_gpui::tabs::{DocumentTab, TabsState};
use pdfbull::ui_gpui::welcome::WelcomeState;
use std::path::PathBuf;

#[test]
fn test_tabs_state_management() {
    let mut tabs_state = TabsState::new();
    assert!(tabs_state.tabs.is_empty());
    assert_eq!(tabs_state.active_tab_index, 0);

    // Add multiple tabs
    tabs_state.tabs.push(DocumentTab {
        id: 0,
        title: "doc1.pdf".to_string(),
        path: Some(PathBuf::from("doc1.pdf")),
        is_modified: false,
    });
    tabs_state.tabs.push(DocumentTab {
        id: 1,
        title: "doc2.pdf".to_string(),
        path: Some(PathBuf::from("doc2.pdf")),
        is_modified: true,
    });
    tabs_state.tabs.push(DocumentTab {
        id: 2,
        title: "doc3.pdf".to_string(),
        path: None,
        is_modified: false,
    });

    assert_eq!(tabs_state.tabs.len(), 3);
    assert!(tabs_state.tabs[1].is_modified);

    // Switch active tab
    tabs_state.active_tab_index = 1;
    assert_eq!(tabs_state.active_tab_index, 1);

    // Close middle tab
    tabs_state.tabs.remove(1);
    assert_eq!(tabs_state.tabs.len(), 2);
    assert_eq!(tabs_state.tabs[0].title, "doc1.pdf");
    assert_eq!(tabs_state.tabs[1].title, "doc3.pdf");
}

#[test]
fn test_sidebar_state_and_mode_switching() {
    let mut sidebar = SidebarState::new();
    assert!(sidebar.is_open);
    assert_eq!(sidebar.mode, SidebarMode::Thumbnails);
    assert_eq!(sidebar.width, 250.0);

    // Toggle sidebar visibility
    sidebar.is_open = false;
    assert!(!sidebar.is_open);
    sidebar.is_open = true;
    assert!(sidebar.is_open);

    // Mode switching
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
}

#[test]
fn test_ribbon_state_and_actions() {
    let mut ribbon = RibbonState::new();
    assert_eq!(ribbon.active_tab, RibbonTab::Home);
    assert_eq!(ribbon.active_tool, AnnotationTool::Pointer);
    assert_eq!(ribbon.layout_mode, PageLayoutMode::Continuous);
    assert!(!ribbon.standalone_cover);
    assert!(!ribbon.midnight_mode);

    // Switch tabs
    ribbon.active_tab = RibbonTab::View;
    assert_eq!(ribbon.active_tab, RibbonTab::View);
    ribbon.active_tab = RibbonTab::Annotate;
    assert_eq!(ribbon.active_tab, RibbonTab::Annotate);
    ribbon.active_tab = RibbonTab::Tools;
    assert_eq!(ribbon.active_tab, RibbonTab::Tools);
    ribbon.active_tab = RibbonTab::Convert;
    assert_eq!(ribbon.active_tab, RibbonTab::Convert);

    // Switch annotation tools
    let tools = [
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

    // Switch color
    ribbon.active_color = [0.2, 0.8, 0.4, 0.9];
    assert_eq!(ribbon.active_color, [0.2, 0.8, 0.4, 0.9]);

    // Page layout & cover & midnight
    ribbon.layout_mode = PageLayoutMode::TwoPageSpread;
    ribbon.standalone_cover = true;
    ribbon.midnight_mode = true;
    assert_eq!(ribbon.layout_mode, PageLayoutMode::TwoPageSpread);
    assert!(ribbon.standalone_cover);
    assert!(ribbon.midnight_mode);
}

#[test]
fn test_document_viewport_zoom_and_rotation() {
    let mut viewport = DocumentViewport::new();
    assert_eq!(viewport.zoom, 1.0);
    assert_eq!(viewport.rotation, 0);
    assert_eq!(viewport.current_page, 0);
    assert_eq!(viewport.total_pages, 1);
    assert_eq!(viewport.page_width, 595.0);
    assert_eq!(viewport.page_height, 842.0);

    // Zoom in
    viewport.zoom *= 1.1;
    assert!((viewport.zoom - 1.1).abs() < 1e-5);

    // Reset zoom
    viewport.zoom = 1.0;
    assert_eq!(viewport.zoom, 1.0);

    // Zoom out
    viewport.zoom /= 1.1;
    assert!((viewport.zoom - (1.0 / 1.1)).abs() < 1e-5);

    // Custom zoom level
    viewport.zoom = 2.5;
    assert_eq!(viewport.zoom, 2.5);

    // Rotation cycling (0 -> 90 -> 180 -> 270 -> 0)
    viewport.rotation = (viewport.rotation + 90) % 360;
    assert_eq!(viewport.rotation, 90);
    viewport.rotation = (viewport.rotation + 90) % 360;
    assert_eq!(viewport.rotation, 180);
    viewport.rotation = (viewport.rotation + 90) % 360;
    assert_eq!(viewport.rotation, 270);
    viewport.rotation = (viewport.rotation + 90) % 360;
    assert_eq!(viewport.rotation, 0);

    // Counter-clockwise rotation
    viewport.rotation = (viewport.rotation + 270) % 360;
    assert_eq!(viewport.rotation, 270);
}

#[test]
fn test_dialogs_overlay_state() {
    let mut dialogs = DialogsState::new();
    assert!(dialogs.active.is_none());

    // Open settings
    dialogs.active = Some(ActiveDialog::Settings);
    assert_eq!(dialogs.active, Some(ActiveDialog::Settings));

    // Open password modal
    dialogs.active = Some(ActiveDialog::Password);
    assert_eq!(dialogs.active, Some(ActiveDialog::Password));

    // Open watermark dialog
    dialogs.active = Some(ActiveDialog::Watermark);
    assert_eq!(dialogs.active, Some(ActiveDialog::Watermark));

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

    // Set filter level
    log_console.filter_level = Some("ERROR");
    assert_eq!(log_console.filter_level, Some("ERROR"));
}
