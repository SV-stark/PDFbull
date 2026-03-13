use crate::pdf_engine::RenderFilter;
use iced::widget::image as iced_image;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;

pub type OpenResult = (
    DocumentId,
    usize,
    Vec<f32>,
    f32,
    Vec<crate::pdf_engine::Bookmark>,
    Vec<Hyperlink>,
);
pub type RenderResult = (u32, u32, Arc<Vec<u8>>);
pub type SearchResultItem = (usize, String, f32, f32, f32, f32);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hyperlink {
    pub page: usize,
    pub bounds: (f32, f32, f32, f32), // x, y, w, h in PDF points
    pub url: Option<String>,
    pub destination_page: Option<usize>,
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DocumentId(pub u64);

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub page: usize,
    pub text: String,
    pub y_position: f32,
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

#[derive(Debug, Clone)]
pub enum PendingAnnotationKind {
    Highlight,
    Rectangle,
}

#[derive(Debug, Clone)]
pub struct AnnotationDrag {
    pub page: usize,
    pub start: (f32, f32),
    pub current: (f32, f32),
    pub kind: PendingAnnotationKind,
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
    pub rendered_pages: std::collections::HashMap<usize, (f32, iced_image::Handle)>,
    pub thumbnails: std::collections::HashMap<usize, iced_image::Handle>,
    pub page_heights: Vec<f32>,
    pub page_width: f32,
    pub search_results: Vec<SearchResult>,
    pub current_search_index: usize,
    pub undo_stack: Vec<UndoableAction>,
    pub redo_stack: Vec<UndoableAction>,
    pub is_loading: bool,
    pub outline: Vec<crate::pdf_engine::Bookmark>,
    pub bookmarks: Vec<PageBookmark>,
    pub annotations: Vec<Annotation>,
    pub links: Vec<Hyperlink>,
    pub viewport_y: f32,
    pub viewport_height: f32,
    pub sidebar_viewport_y: f32,
    pub last_cleanup_time: std::time::Instant,
}

const VIEWPORT_BUFFER: usize = 3;
pub const PAGE_SPACING: f32 = 10.0;

use std::sync::atomic::{AtomicU64, Ordering};

pub static NEXT_DOC_ID: AtomicU64 = AtomicU64::new(1);
pub static NEXT_ANN_ID: AtomicU64 = AtomicU64::new(1);

pub fn next_doc_id() -> DocumentId {
    DocumentId(NEXT_DOC_ID.fetch_add(1, Ordering::SeqCst))
}

pub fn next_annotation_id() -> u64 {
    NEXT_ANN_ID.fetch_add(1, Ordering::SeqCst)
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
            rendered_pages: std::collections::HashMap::new(),
            thumbnails: std::collections::HashMap::new(),
            page_heights: Vec::new(),
            page_width: 0.0,
            search_results: Vec::new(),
            current_search_index: 0,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            is_loading: false,
            outline: Vec::new(),
            bookmarks: Vec::new(),
            annotations: Vec::new(),
            links: Vec::new(),
            viewport_y: 0.0,
            viewport_height: 800.0,
            sidebar_viewport_y: 0.0,
            last_cleanup_time: std::time::Instant::now(),
        }
    }

    pub fn get_visible_pages(&self) -> std::collections::HashSet<usize> {
        let mut visible = std::collections::HashSet::new();
        
        let scaled_spacing = PAGE_SPACING * self.zoom;
        let scaled_padding = 10.0 * self.zoom; 
        
        let mut y = scaled_padding;

        let v_height = if self.viewport_height > 0.0 { self.viewport_height } else { 2000.0 };
        
        // Use a VERY large margin to ensure pages don't go blank when viewport_y is slightly stale
        let margin = v_height * 4.0; 
        let viewport_top = (self.viewport_y - margin).max(0.0);
        let viewport_bottom = self.viewport_y + v_height + margin;

        for (idx, height) in self.page_heights.iter().enumerate() {
            let scaled_height = height * self.zoom;
            let page_bottom = y + scaled_height;

            if page_bottom >= viewport_top && y <= viewport_bottom {
                visible.insert(idx);
            }

            y = page_bottom + scaled_spacing;
        }

        visible
    }

    pub fn get_visible_thumbnails(&self) -> std::collections::HashSet<usize> {
        let mut visible = std::collections::HashSet::new();
        let thumbnail_height = 40.0;
        let start_idx = (self.sidebar_viewport_y / thumbnail_height).max(0.0) as usize;
        let end_idx = (start_idx + 30).min(self.total_pages);
        for i in start_idx..end_idx {
            visible.insert(i);
        }
        visible
    }

    pub fn cleanup_distant_pages(&mut self) {
        let visible = self.get_visible_pages();
        let pages_to_keep: std::collections::HashSet<usize> = visible
            .iter()
            .flat_map(|&p| {
                let start = p.saturating_sub(VIEWPORT_BUFFER);
                let end = (p + VIEWPORT_BUFFER).min(self.total_pages);
                start..end
            })
            .collect();

        let current_zoom = self.zoom;
        let to_remove_pages: Vec<usize> = self
            .rendered_pages
            .iter()
            .filter(|(&p, (scale, _))| {
                if pages_to_keep.contains(&p) {
                    (scale - current_zoom).abs() > 1.0
                } else {
                    true
                }
            })
            .map(|(&p, _)| p)
            .collect();

        for p in to_remove_pages {
            self.rendered_pages.remove(&p);
        }

        let thumb_start = visible.iter().min().copied().unwrap_or(0).saturating_sub(5);
        let thumb_end = visible.iter().max().copied().unwrap_or(0).saturating_add(5);
        let thumbs_to_keep: std::collections::HashSet<usize> =
            (thumb_start..=thumb_end.min(self.total_pages.saturating_sub(1))).collect();

        let to_remove_thumbs: Vec<usize> = self
            .thumbnails
            .keys()
            .copied()
            .filter(|p| !thumbs_to_keep.contains(p))
            .collect();

        for p in to_remove_thumbs {
            self.thumbnails.remove(&p);
        }
        self.last_cleanup_time = std::time::Instant::now();
    }

    pub fn needs_periodic_cleanup(&self) -> bool {
        self.last_cleanup_time.elapsed().as_secs() >= 5
    }
}
