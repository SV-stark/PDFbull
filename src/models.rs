use crate::pdf_engine::RenderFilter;
use iced::widget::image as iced_image;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum PdfError {
    #[error("Failed to open document: {0}")]
    OpenFailed(String),
    #[error("Page {0} not found")]
    PageNotFound(usize),
    #[error("Render failed: {0}")]
    RenderFailed(String),
    #[error("Engine error: {0}")]
    EngineError(EngineErrorKind),
    #[error("IO error: {0}")]
    IoError(String),
    #[error("Search failed: {0}")]
    SearchError(String),
    #[error("Invalid path")]
    InvalidPath,
    #[error("Engine died")]
    EngineDied,
    #[error("Channel closed")]
    ChannelClosed,
    #[error("Cancelled")]
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EngineErrorKind {
    DocumentNotFound,
    DocumentPathNotFound,
    PdfiumError(String),
    Generic(String),
}

impl From<&str> for EngineErrorKind {
    fn from(s: &str) -> Self {
        Self::Generic(s.to_string())
    }
}

impl From<String> for EngineErrorKind {
    fn from(s: String) -> Self {
        Self::Generic(s)
    }
}

impl std::fmt::Display for EngineErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DocumentNotFound => write!(f, "Document not found"),
            Self::DocumentPathNotFound => write!(f, "Document path not found"),
            Self::PdfiumError(e) => write!(f, "PDFium error: {e}"),
            Self::Generic(e) => write!(f, "{e}"),
        }
    }
}

impl From<&str> for PdfError {
    fn from(s: &str) -> Self {
        Self::EngineError(EngineErrorKind::Generic(s.to_string()))
    }
}

impl From<String> for PdfError {
    fn from(s: String) -> Self {
        Self::EngineError(EngineErrorKind::Generic(s))
    }
}

impl PartialEq<&str> for PdfError {
    fn eq(&self, other: &&str) -> bool {
        match self {
            Self::EngineError(EngineErrorKind::Generic(s))
            | Self::OpenFailed(s)
            | Self::RenderFailed(s)
            | Self::IoError(s)
            | Self::SearchError(s) => s == *other,
            Self::EngineDied => *other == "Engine died",
            Self::ChannelClosed => *other == "Channel closed",
            Self::Cancelled => *other == "Cancelled",
            _ => false,
        }
    }
}

pub type PdfResult<T> = Result<T, PdfError>;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DocumentMetadata {
    pub title: Option<String>,
    pub author: Option<String>,
    pub subject: Option<String>,
    pub keywords: Option<String>,
    pub creator: Option<String>,
    pub producer: Option<String>,
    pub creation_date: Option<String>,
    pub modification_date: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignatureInfo {
    pub name: String,
    pub reason: Option<String>,
    pub location: Option<String>,
    pub date: Option<String>,
    pub is_valid: bool,
}

#[derive(Debug, Clone)]
pub struct OpenResult {
    pub id: DocumentId,
    pub page_count: usize,
    pub page_heights: Vec<f32>,
    pub max_width: f32,
    pub outline: Vec<crate::pdf_engine::Bookmark>,
    pub links: Vec<Hyperlink>,
    pub metadata: DocumentMetadata,
    pub signatures: Vec<SignatureInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TextItem {
    pub text: String,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RenderResult {
    pub width: u32,
    pub height: u32,
    pub data: bytes::Bytes,
    pub text_items: Vec<TextItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub page: usize,
    pub text: String,
    pub y_position: f32,
    pub x: f32,
    pub width: f32,
    pub height: f32,
}

impl SearchResult {
    pub fn from_search_result_item(item: SearchResultItem) -> Self {
        Self {
            page: item.page_index,
            text: item.text,
            y_position: item.y,
            x: item.x,
            width: item.width,
            height: item.height,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hyperlink {
    pub page: usize,
    pub bounds: (f32, f32, f32, f32), // x, y, w, h in PDF points
    pub url: Option<String>,
    pub destination_page: Option<usize>,
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DocumentId(pub u64);

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AppTheme {
    #[default]
    System,
    Light,
    Dark,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub theme: AppTheme,
    pub cache_size: usize,
    pub max_cache_memory: usize,
    pub render_quality: crate::pdf_engine::RenderQuality,
    pub default_filter: RenderFilter,
    pub accent_color: String,
    pub restore_session: bool,
    pub remember_last_file: bool,
    pub default_zoom: f32,
    pub auto_save: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            theme: AppTheme::System,
            cache_size: 100,
            max_cache_memory: 512, // 512MB
            render_quality: crate::pdf_engine::RenderQuality::Medium,
            default_filter: RenderFilter::None,
            accent_color: "#3b82f6".to_string(),
            restore_session: true,
            remember_last_file: true,
            default_zoom: 1.0,
            auto_save: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentFile {
    pub path: String,
    pub name: String,
    pub last_opened: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SessionData {
    pub open_tabs: Vec<String>,
    pub active_tab: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageBookmark {
    pub page: usize,
    pub label: String,
    pub created_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResultItem {
    pub page_index: usize,
    pub text: String,
    pub y: f32,
    pub x: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AnnotationStyle {
    Text {
        text: String,
        color: String,
        font_size: u32,
    },
    Highlight {
        color: String,
    },
    Rectangle {
        color: String,
        thickness: f32,
        fill: bool,
    },
    Redact {
        color: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Annotation {
    pub id: u64,
    pub page: usize,
    pub style: AnnotationStyle,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone)]
pub enum UndoableAction {
    AddAnnotation(Annotation),
    DeleteAnnotation(usize, Annotation),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PendingAnnotationKind {
    Highlight,
    Rectangle,
    Redact,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FormFieldVariant {
    Text {
        value: String,
    },
    Checkbox {
        is_checked: bool,
    },
    RadioButton {
        is_selected: bool,
        group_name: Option<String>,
    },
    ComboBox {
        options: Vec<String>,
        selected_index: Option<usize>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormField {
    pub name: String,
    pub variant: FormFieldVariant,
    pub page: usize,
}

#[derive(Debug, Clone)]
pub struct AnnotationDrag {
    pub page: usize,
    pub start: (f32, f32),
    pub current: (f32, f32),
    pub kind: PendingAnnotationKind,
}

pub struct TabViewState {
    pub rendered_pages: std::collections::HashMap<usize, (f32, iced_image::Handle)>,
    pub thumbnails: std::collections::HashMap<usize, iced_image::Handle>,
    pub text_layers: std::collections::HashMap<usize, Vec<TextItem>>,
    pub viewport_y: f32,
    pub viewport_height: f32,
    pub sidebar_viewport_y: f32,
    pub last_cleanup_time: std::time::Instant,
    pub visible_range: (usize, usize),
    pub is_loading: bool,
}

impl Default for TabViewState {
    fn default() -> Self {
        Self {
            rendered_pages: std::collections::HashMap::new(),
            thumbnails: std::collections::HashMap::new(),
            text_layers: std::collections::HashMap::new(),
            viewport_y: 0.0,
            viewport_height: 800.0,
            sidebar_viewport_y: 0.0,
            last_cleanup_time: std::time::Instant::now()
                .checked_sub(std::time::Duration::from_secs(10))
                .unwrap_or_else(std::time::Instant::now),
            visible_range: (0, 1),
            is_loading: false,
        }
    }
}

pub struct DocumentTab {
    pub id: DocumentId,
    pub path: PathBuf,
    pub name: String,
    pub total_pages: usize,
    pub current_page: usize,
    pub zoom: f32,
    pub rotation: i32,
    pub render_filter: RenderFilter,
    pub auto_crop: bool,
    pub page_heights: Vec<f32>,
    pub page_width: f32,
    pub search_results: Vec<SearchResult>,
    pub current_search_index: usize,
    pub undo_stack: Vec<UndoableAction>,
    pub redo_stack: Vec<UndoableAction>,
    pub outline: Vec<crate::pdf_engine::Bookmark>,
    pub bookmarks: Vec<PageBookmark>,
    pub annotations: Vec<Annotation>,
    pub links: Vec<Hyperlink>,
    pub signatures: Vec<SignatureInfo>,
    pub metadata: DocumentMetadata,
    pub view_state: TabViewState,
}

use std::sync::atomic::{AtomicU64, Ordering};

pub static NEXT_DOC_ID: AtomicU64 = AtomicU64::new(1);
pub static NEXT_ANN_ID: AtomicU64 = AtomicU64::new(1);

pub fn next_doc_id() -> DocumentId {
    DocumentId(NEXT_DOC_ID.fetch_add(1, Ordering::Relaxed))
}

pub fn next_annotation_id() -> u64 {
    NEXT_ANN_ID.fetch_add(1, Ordering::Relaxed)
}

impl DocumentTab {
    pub fn new(path: PathBuf) -> Self {
        Self {
            id: next_doc_id(),
            name: path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "Unknown".to_string()),
            path,
            total_pages: 0,
            current_page: 0,
            zoom: 1.0,
            rotation: 0,
            render_filter: RenderFilter::None,
            auto_crop: false,
            page_heights: Vec::new(),
            page_width: 0.0,
            search_results: Vec::new(),
            current_search_index: 0,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            outline: Vec::new(),
            bookmarks: Vec::new(),
            annotations: Vec::new(),
            links: Vec::new(),
            signatures: Vec::new(),
            metadata: DocumentMetadata::default(),
            view_state: TabViewState::default(),
        }
    }

    pub fn update_visible_range(&mut self) {
        if self.page_heights.is_empty() {
            self.view_state.visible_range = (0, 0);
            return;
        }

        let scaled_spacing = crate::ui::theme::PAGE_SPACING * self.zoom;
        let scaled_padding = crate::ui::theme::PAGE_PADDING * self.zoom;
        let mut y = scaled_padding;

        let v_height = if self.view_state.viewport_height > 0.0 {
            self.view_state.viewport_height
        } else {
            2000.0
        };

        let margin = v_height * 1.5;
        let viewport_top = (self.view_state.viewport_y - margin).max(0.0);
        let viewport_bottom = self.view_state.viewport_y + v_height + margin;

        let mut start = 0;
        let mut end = 0;
        let mut found_start = false;

        for (idx, height) in self.page_heights.iter().enumerate() {
            let scaled_height = height * self.zoom;
            let page_bottom = y + scaled_height;

            if !found_start && page_bottom >= viewport_top {
                start = idx;
                found_start = true;
            }

            if page_bottom >= viewport_top && y <= viewport_bottom {
                end = idx;
            } else if found_start && y > viewport_bottom {
                break;
            }

            y = page_bottom + scaled_spacing;
        }

        self.view_state.visible_range = (start, end + 1);
    }

    pub fn get_visible_pages(&self) -> std::collections::HashSet<usize> {
        (self.view_state.visible_range.0..self.view_state.visible_range.1).collect()
    }

    pub fn get_visible_thumbnails(&self) -> std::collections::HashSet<usize> {
        let mut visible = std::collections::HashSet::new();
        let start_idx = (self.view_state.sidebar_viewport_y / crate::ui::theme::THUMBNAIL_HEIGHT)
            .max(0.0) as usize;

        let v_height = if self.view_state.viewport_height > 0.0 {
            self.view_state.viewport_height
        } else {
            1000.0
        };
        let visible_count = (v_height / crate::ui::theme::THUMBNAIL_HEIGHT).ceil() as usize + 5;
        let end_idx = (start_idx + visible_count).min(self.total_pages);

        for i in start_idx..end_idx {
            visible.insert(i);
        }
        visible
    }

    pub fn cleanup_distant_pages(&mut self) {
        let (start, end) = self.view_state.visible_range;
        let buffer = crate::ui::theme::VIEWPORT_BUFFER;
        let keep_start = start.saturating_sub(buffer);
        let keep_end = (end + buffer).min(self.total_pages);

        let current_zoom = self.zoom;
        self.view_state.rendered_pages.retain(|&p, (scale, _)| {
            if p >= keep_start && p < keep_end {
                (*scale - current_zoom).abs() <= 0.01
            } else {
                false
            }
        });

        let thumb_start_idx = (self.view_state.sidebar_viewport_y
            / crate::ui::theme::THUMBNAIL_HEIGHT)
            .max(0.0) as usize;
        let thumb_keep_start = thumb_start_idx.saturating_sub(15);
        let thumb_keep_end = thumb_start_idx.saturating_add(45).min(self.total_pages);

        self.view_state
            .thumbnails
            .retain(|&p, _| p >= thumb_keep_start && p < thumb_keep_end);

        self.view_state.last_cleanup_time = std::time::Instant::now();
    }

    pub fn needs_periodic_cleanup(&self) -> bool {
        self.view_state.last_cleanup_time.elapsed().as_secs() >= 5
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_document_id_new_increments() {
        NEXT_DOC_ID.store(1, std::sync::atomic::Ordering::SeqCst);
        let id1 = next_doc_id();
        let id2 = next_doc_id();
        assert_eq!(id1.0, 1);
        assert_eq!(id2.0, 2);
    }

    #[test]
    fn test_document_id_equality() {
        let id1 = DocumentId(1);
        let id2 = DocumentId(1);
        let id3 = DocumentId(2);
        assert_eq!(id1, id2);
        assert_ne!(id1, id3);
    }

    #[test]
    fn test_document_id_default() {
        let id = DocumentId::default();
        assert_eq!(id.0, 0);
    }

    #[test]
    fn test_document_id_copy() {
        let id1 = DocumentId(42);
        let id2 = id1;
        assert_eq!(id1, id2);
    }

    #[test]
    fn test_document_id_clone() {
        let id1 = DocumentId(42);
        let id2 = id1.clone();
        assert_eq!(id1, id2);
    }

    #[test]
    fn test_document_id_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(DocumentId(1));
        set.insert(DocumentId(2));
        set.insert(DocumentId(1));
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn test_annotation_id_new_increments() {
        NEXT_ANN_ID.store(1, std::sync::atomic::Ordering::SeqCst);
        let id1 = next_annotation_id();
        let id2 = next_annotation_id();
        assert_eq!(id1, 1);
        assert_eq!(id2, 2);
    }

    #[test]
    fn test_app_settings_default() {
        let settings = AppSettings::default();
        assert_eq!(settings.theme, AppTheme::System);
        assert_eq!(settings.cache_size, 100);
        assert_eq!(settings.max_cache_memory, 512);
        assert_eq!(
            settings.render_quality,
            crate::pdf_engine::RenderQuality::Medium
        );
        assert_eq!(settings.default_filter, RenderFilter::None);
        assert_eq!(settings.accent_color, "#3b82f6");
        assert!(settings.restore_session);
        assert!(settings.remember_last_file);
        assert_eq!(settings.default_zoom, 1.0);
        assert!(settings.auto_save);
    }

    #[test]
    fn test_document_metadata_default() {
        let metadata = DocumentMetadata::default();
        assert!(metadata.title.is_none());
        assert!(metadata.author.is_none());
        assert!(metadata.subject.is_none());
        assert!(metadata.keywords.is_none());
        assert!(metadata.creator.is_none());
        assert!(metadata.producer.is_none());
        assert!(metadata.creation_date.is_none());
        assert!(metadata.modification_date.is_none());
    }

    #[test]
    fn test_session_data_default() {
        let session = SessionData::default();
        assert!(session.open_tabs.is_empty());
        assert_eq!(session.active_tab, 0);
    }

    #[test]
    fn test_document_tab_new() {
        let path = PathBuf::from("/test/document.pdf");
        let tab = DocumentTab::new(path.clone());
        assert_eq!(tab.path, path);
        assert_eq!(tab.name, "document.pdf");
        assert_eq!(tab.total_pages, 0);
        assert_eq!(tab.current_page, 0);
        assert_eq!(tab.zoom, 1.0);
        assert_eq!(tab.rotation, 0);
        assert_eq!(tab.render_filter, RenderFilter::None);
        assert!(!tab.auto_crop);
        assert!(tab.page_heights.is_empty());
        assert!(tab.search_results.is_empty());
        assert_eq!(tab.current_search_index, 0);
        assert!(tab.undo_stack.is_empty());
        assert!(tab.redo_stack.is_empty());
        assert!(tab.bookmarks.is_empty());
        assert!(tab.annotations.is_empty());
    }

    #[test]
    fn test_document_tab_new_unknown_path() {
        let tab = DocumentTab::new(PathBuf::new());
        assert_eq!(tab.name, "Unknown");
    }

    #[test]
    fn test_tab_view_state_default() {
        let state = TabViewState::default();
        assert_eq!(state.viewport_y, 0.0);
        assert_eq!(state.viewport_height, 800.0);
        assert_eq!(state.sidebar_viewport_y, 0.0);
        assert_eq!(state.visible_range, (0, 1));
        assert!(!state.is_loading);
    }

    #[test]
    fn test_update_visible_range_empty_pages() {
        let mut tab = DocumentTab::new(PathBuf::from("/test/doc.pdf"));
        tab.page_heights = vec![];
        tab.update_visible_range();
        assert_eq!(tab.view_state.visible_range, (0, 0));
    }

    #[test]
    fn test_update_visible_range_single_page() {
        let mut tab = DocumentTab::new(PathBuf::from("/test/doc.pdf"));
        tab.page_heights = vec![1000.0];
        tab.view_state.viewport_height = 800.0;
        tab.view_state.viewport_y = 0.0;
        tab.zoom = 1.0;
        tab.update_visible_range();
        assert_eq!(tab.view_state.visible_range, (0, 1));
    }

    #[test]
    fn test_update_visible_range_with_zoom() {
        let mut tab = DocumentTab::new(PathBuf::from("/test/doc.pdf"));
        tab.page_heights = vec![1000.0, 1000.0, 1000.0];
        tab.view_state.viewport_height = 800.0;
        tab.view_state.viewport_y = 0.0;
        tab.zoom = 2.0;
        tab.update_visible_range();
        assert!(tab.view_state.visible_range.1 >= 1);
    }

    #[test]
    fn test_get_visible_pages() {
        let mut tab = DocumentTab::new(PathBuf::from("/test/doc.pdf"));
        tab.page_heights = vec![100.0; 10];
        tab.view_state.visible_range = (2, 5);
        let visible = tab.get_visible_pages();
        assert!(visible.contains(&2));
        assert!(visible.contains(&3));
        assert!(visible.contains(&4));
        assert!(!visible.contains(&1));
        assert!(!visible.contains(&5));
    }

    #[test]
    fn test_get_visible_pages_empty_range() {
        let mut tab = DocumentTab::new(PathBuf::from("/test/doc.pdf"));
        tab.page_heights = vec![100.0; 10];
        tab.view_state.visible_range = (5, 5);
        let visible = tab.get_visible_pages();
        assert!(visible.is_empty());
    }

    #[test]
    fn test_cleanup_distant_pages_removes_distant() {
        let mut tab = DocumentTab::new(PathBuf::from("/test/doc.pdf"));
        tab.total_pages = 20;
        tab.page_heights = vec![100.0; 20];
        tab.view_state.visible_range = (5, 8);
        tab.view_state.rendered_pages = std::collections::HashMap::new();
        tab.view_state.thumbnails = std::collections::HashMap::new();

        for i in 0..20 {
            tab.view_state
                .rendered_pages
                .insert(i, (1.0, iced::widget::image::Handle::from_bytes(vec![])));
        }

        tab.zoom = 1.0;
        tab.cleanup_distant_pages();

        for i in 0..20 {
            if i >= 3 && i <= 9 {
                assert!(
                    tab.view_state.rendered_pages.contains_key(&i),
                    "Page {} should be kept",
                    i
                );
            }
        }
    }

    #[test]
    fn test_cleanup_distant_pages_removes_zoom_mismatch() {
        let mut tab = DocumentTab::new(PathBuf::from("/test/doc.pdf"));
        tab.total_pages = 10;
        tab.page_heights = vec![100.0; 10];
        tab.view_state.visible_range = (5, 7);
        tab.view_state.rendered_pages = std::collections::HashMap::new();
        tab.view_state.thumbnails = std::collections::HashMap::new();

        tab.view_state
            .rendered_pages
            .insert(5, (2.0, iced::widget::image::Handle::from_bytes(vec![])));
        tab.view_state
            .rendered_pages
            .insert(6, (1.0, iced::widget::image::Handle::from_bytes(vec![])));

        tab.zoom = 1.0;
        tab.cleanup_distant_pages();

        assert!(tab.view_state.rendered_pages.contains_key(&6));
        assert!(!tab.view_state.rendered_pages.contains_key(&5));
    }

    #[test]
    fn test_needs_periodic_cleanup_immediate() {
        let tab = DocumentTab::new(PathBuf::from("/test/doc.pdf"));
        assert!(tab.needs_periodic_cleanup());
    }

    #[test]
    fn test_annotation_serialization() {
        let ann = Annotation {
            id: 1,
            page: 0,
            style: AnnotationStyle::Highlight {
                color: "#FFFF00".to_string(),
            },
            x: 10.0,
            y: 20.0,
            width: 100.0,
            height: 50.0,
        };
        let json = serde_json::to_string(&ann).unwrap();
        assert!(json.contains("\"Highlight\""));
        assert!(json.contains("\"page\":0"));
    }

    #[test]
    fn test_annotation_rectangle_serialization() {
        let ann = Annotation {
            id: 2,
            page: 1,
            style: AnnotationStyle::Rectangle {
                color: "#FF0000".to_string(),
                thickness: 2.0,
                fill: true,
            },
            x: 0.0,
            y: 0.0,
            width: 200.0,
            height: 100.0,
        };
        let json = serde_json::to_string(&ann).unwrap();
        let deserialized: Annotation = serde_json::from_str(&json).unwrap();
        match deserialized.style {
            AnnotationStyle::Rectangle {
                thickness, fill, ..
            } => {
                assert_eq!(thickness, 2.0);
                assert!(fill);
            }
            _ => panic!("Expected Rectangle style"),
        }
    }

    #[test]
    fn test_annotation_text_serialization() {
        let ann = Annotation {
            id: 3,
            page: 2,
            style: AnnotationStyle::Text {
                text: "Hello".to_string(),
                color: "#0000FF".to_string(),
                font_size: 12,
            },
            x: 50.0,
            y: 100.0,
            width: 100.0,
            height: 20.0,
        };
        let json = serde_json::to_string(&ann).unwrap();
        let deserialized: Annotation = serde_json::from_str(&json).unwrap();
        match deserialized.style {
            AnnotationStyle::Text {
                text, font_size, ..
            } => {
                assert_eq!(text, "Hello");
                assert_eq!(font_size, 12);
            }
            _ => panic!("Expected Text style"),
        }
    }

    #[test]
    fn test_annotation_redact_serialization() {
        let ann = Annotation {
            id: 4,
            page: 0,
            style: AnnotationStyle::Redact {
                color: "#000000".to_string(),
            },
            x: 0.0,
            y: 0.0,
            width: 50.0,
            height: 50.0,
        };
        let json = serde_json::to_string(&ann).unwrap();
        let deserialized: Annotation = serde_json::from_str(&json).unwrap();
        match deserialized.style {
            AnnotationStyle::Redact { .. } => {}
            _ => panic!("Expected Redact style"),
        }
    }

    #[test]
    fn test_hyperlink_serialization() {
        let link = Hyperlink {
            page: 0,
            bounds: (10.0, 20.0, 100.0, 50.0),
            url: Some("https://example.com".to_string()),
            destination_page: None,
        };
        let json = serde_json::to_string(&link).unwrap();
        let deserialized: Hyperlink = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.url, Some("https://example.com".to_string()));
        assert!(deserialized.destination_page.is_none());
    }

    #[test]
    fn test_hyperlink_with_destination() {
        let link = Hyperlink {
            page: 0,
            bounds: (0.0, 0.0, 50.0, 50.0),
            url: None,
            destination_page: Some(5),
        };
        let json = serde_json::to_string(&link).unwrap();
        let deserialized: Hyperlink = serde_json::from_str(&json).unwrap();
        assert!(deserialized.url.is_none());
        assert_eq!(deserialized.destination_page, Some(5));
    }

    #[test]
    fn test_search_result_item_serialization() {
        let item = SearchResultItem {
            page_index: 3,
            text: "test result".to_string(),
            y: 100.0,
            x: 50.0,
            width: 200.0,
            height: 20.0,
        };
        let json = serde_json::to_string(&item).unwrap();
        let deserialized: SearchResultItem = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.page_index, 3);
        assert_eq!(deserialized.text, "test result");
    }

    #[test]
    fn test_text_item_serialization() {
        let item = TextItem {
            text: "Hello World".to_string(),
            x: 10.0,
            y: 20.0,
            width: 100.0,
            height: 15.0,
        };
        let json = serde_json::to_string(&item).unwrap();
        let deserialized: TextItem = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.text, "Hello World");
        assert_eq!(deserialized.x, 10.0);
    }

    #[test]
    fn test_recent_file_serialization() {
        let file = RecentFile {
            path: "/path/to/file.pdf".to_string(),
            name: "file.pdf".to_string(),
            last_opened: 1234567890,
        };
        let json = serde_json::to_string(&file).unwrap();
        let deserialized: RecentFile = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.path, "/path/to/file.pdf");
        assert_eq!(deserialized.name, "file.pdf");
    }

    #[test]
    fn test_page_bookmark_serialization() {
        let bookmark = PageBookmark {
            page: 5,
            label: "Chapter 1".to_string(),
            created_at: 1234567890,
        };
        let json = serde_json::to_string(&bookmark).unwrap();
        let deserialized: PageBookmark = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.page, 5);
        assert_eq!(deserialized.label, "Chapter 1");
    }

    #[test]
    fn test_form_field_text_serialization() {
        let field = FormField {
            name: "Name".to_string(),
            variant: FormFieldVariant::Text {
                value: "John".to_string(),
            },
            page: 0,
        };
        let json = serde_json::to_string(&field).unwrap();
        let deserialized: FormField = serde_json::from_str(&json).unwrap();
        match deserialized.variant {
            FormFieldVariant::Text { value } => assert_eq!(value, "John"),
            _ => panic!("Expected Text variant"),
        }
    }

    #[test]
    fn test_form_field_checkbox_serialization() {
        let field = FormField {
            name: "Agree".to_string(),
            variant: FormFieldVariant::Checkbox { is_checked: true },
            page: 1,
        };
        let json = serde_json::to_string(&field).unwrap();
        let deserialized: FormField = serde_json::from_str(&json).unwrap();
        match deserialized.variant {
            FormFieldVariant::Checkbox { is_checked } => assert!(is_checked),
            _ => panic!("Expected Checkbox variant"),
        }
    }

    #[test]
    fn test_app_theme_serialization() {
        let system = AppTheme::System;
        let light = AppTheme::Light;
        let dark = AppTheme::Dark;

        assert_eq!(serde_json::to_string(&system).unwrap(), "\"System\"");
        assert_eq!(serde_json::to_string(&light).unwrap(), "\"Light\"");
        assert_eq!(serde_json::to_string(&dark).unwrap(), "\"Dark\"");

        let system_back: AppTheme = serde_json::from_str("\"System\"").unwrap();
        let light_back: AppTheme = serde_json::from_str("\"Light\"").unwrap();
        let dark_back: AppTheme = serde_json::from_str("\"Dark\"").unwrap();

        assert_eq!(system_back, AppTheme::System);
        assert_eq!(light_back, AppTheme::Light);
        assert_eq!(dark_back, AppTheme::Dark);
    }

    #[test]
    fn test_render_result_clone() {
        let result = RenderResult {
            width: 100,
            height: 200,
            data: bytes::Bytes::from(vec![1, 2, 3, 4]),
            text_items: vec![TextItem {
                text: "test".to_string(),
                x: 0.0,
                y: 0.0,
                width: 10.0,
                height: 10.0,
            }],
        };
        let cloned = result.clone();
        assert_eq!(cloned.width, 100);
        assert_eq!(cloned.height, 200);
        assert_eq!(cloned.text_items.len(), 1);
    }

    #[test]
    fn test_signature_info_default() {
        let sig = SignatureInfo {
            name: "Test".to_string(),
            reason: Some("Testing".to_string()),
            location: None,
            date: None,
            is_valid: true,
        };
        assert_eq!(sig.name, "Test");
        assert!(sig.is_valid);
        assert!(sig.location.is_none());
    }

    #[test]
    fn test_open_result_clone() {
        let result = OpenResult {
            id: DocumentId(1),
            page_count: 10,
            page_heights: vec![100.0; 10],
            max_width: 800.0,
            outline: vec![],
            links: vec![],
            metadata: DocumentMetadata::default(),
            signatures: vec![],
        };
        let cloned = result.clone();
        assert_eq!(cloned.page_count, 10);
        assert_eq!(cloned.max_width, 800.0);
    }

    #[test]
    fn test_search_result_clone() {
        let result = SearchResult {
            page: 0,
            text: "found".to_string(),
            y_position: 100.0,
            x: 50.0,
            width: 200.0,
            height: 20.0,
        };
        let cloned = result.clone();
        assert_eq!(cloned.text, "found");
    }
}
