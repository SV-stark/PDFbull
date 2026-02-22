#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod pdf_engine;
mod models;
mod commands;
mod ui;
mod ui_keyboard_help;
mod ui_settings;
mod ui_welcome;
mod ui_document;

use models::{AppSettings, AppTheme, DocumentId, DocumentTab, PageBookmark, RecentFile, SearchResult, SessionData};
use commands::PdfCommand;
use pdf_engine::RenderFilter;

use iced::widget::image as iced_image;
use iced::{Element, Task};
use std::sync::Arc;
use tokio::sync::mpsc;
use std::path::PathBuf;
use std::fs;

#[derive(Debug, Clone)]
struct EngineState {
    cmd_tx: mpsc::Sender<PdfCommand>,
    documents: std::collections::HashMap<DocumentId, String>,
}

#[derive(Debug, Clone)]
enum Message {
    ResetZoom,
    OpenSettings,
    CloseSettings,
    SaveSettings(AppSettings),
    ToggleSidebar,
    ToggleFullscreen,
    ToggleKeyboardHelp,
    RotateClockwise,
    RotateCounterClockwise,
    AddBookmark,
    RemoveBookmark(usize),
    JumpToBookmark(usize),
    AddHighlight,
    AddRectangle,
    DeleteAnnotation(usize),
    SetFilter(RenderFilter),
    ToggleAutoCrop,
    DocumentOpenedWithPath((PathBuf, (DocumentId, usize, Vec<f32>, f32, Vec<pdf_engine::Bookmark>))),
    OpenDocument,
    OpenFile(PathBuf),
    CloseTab(usize),
    SwitchTab(usize),
    NextPage,
    PrevPage,
    ZoomIn,
    ZoomOut,
    SetZoom(f32),
    JumpToPage(usize),
    ViewportChanged(f32, f32),
    RequestRender(usize),
    SidebarViewportChanged(f32),
    PageRendered(Result<(usize, u32, u32, Arc<Vec<u8>>), String>),
    DocumentOpened(Result<(DocumentId, usize, Vec<f32>, f32, Vec<pdf_engine::Bookmark>), String>),
    EngineInitialized(EngineState),
    Search(String),
    PerformSearch,
    SearchResult(Result<Vec<(usize, String, f32)>, String>),
    NextSearchResult,
    PrevSearchResult,
    ClearSearch,
    ExtractText,
    TextExtracted(Result<String, String>),
    ExportImage,
    ExportImages,
    SaveAnnotations,
    AnnotationsSaved(Result<String, String>),
    AnnotationsLoaded(DocumentId, Vec<models::Annotation>),
    ImageExported(Result<String, String>),
    OpenRecentFile(RecentFile),
    Error(String),
    ClearStatus,
}

pub struct PdfBullApp {
    pub tabs: Vec<DocumentTab>,
    pub active_tab: usize,
    pub settings: AppSettings,
    pub recent_files: Vec<RecentFile>,
    pub show_settings: bool,
    pub show_sidebar: bool,
    pub show_keyboard_help: bool,
    pub is_fullscreen: bool,
    pub search_query: String,
    pub search_pending: Option<String>,
    pub status_message: Option<String>,
    pub engine: Option<EngineState>,
    pub loaded: bool,
    pub rendering_count: usize,
}

impl Default for PdfBullApp {
    fn default() -> Self {
        Self {
            tabs: Vec::new(),
            active_tab: 0,
            settings: AppSettings::default(),
            recent_files: Vec::new(),
            show_settings: false,
            show_sidebar: false,
            show_keyboard_help: false,
            is_fullscreen: false,
            search_query: String::new(),
            search_pending: None,
            status_message: None,
            engine: None,
            loaded: false,
            rendering_count: 0,
        }
    }
}

fn get_config_dir() -> std::path::PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("pdfbull")
}

impl PdfBullApp {
    pub fn load_settings(&mut self) {
        let path = get_config_dir().join("settings.json");
        if let Ok(data) = fs::read_to_string(&path) {
            if let Ok(settings) = serde_json::from_str::<AppSettings>(&data) {
                self.settings = settings;
            } else {
                eprintln!("Warning: Corrupted settings.json, using defaults");
                // Try to parse old format with fewer fields
                if let Ok(value) = serde_json::from_str::<serde_json::Value>(&data) {
                    let mut settings = AppSettings::default();
                    if let Some(obj) = value.as_object() {
                        if let Some(theme) = obj.get("theme").and_then(|v| v.as_str()) {
                            settings.theme = AppTheme::from(theme);
                        }
                        if let Some(v) = obj.get("auto_save").and_then(|v| v.as_bool()) {
                            settings.auto_save = v;
                        }
                        if let Some(v) = obj.get("remember_last_file").and_then(|v| v.as_bool()) {
                            settings.remember_last_file = v;
                        }
                        if let Some(v) = obj.get("default_zoom").and_then(|v| v.as_f64()) {
                            settings.default_zoom = v as f32;
                        }
                    }
                    self.settings = settings;
                }
            }
        }
    }

    pub fn save_settings(&self) {
        let dir = get_config_dir();
        let _ = fs::create_dir_all(&dir);
        let path = dir.join("settings.json");
        if let Ok(data) = serde_json::to_string_pretty(&self.settings) {
            let _ = fs::write(path, data);
        }
    }

    pub fn load_recent_files(&mut self) {
        let path = get_config_dir().join("recent_files.json");
        if let Ok(data) = fs::read_to_string(&path) {
            if let Ok(files) = serde_json::from_str(&data) {
                self.recent_files = files;
            } else {
                eprintln!("Warning: Corrupted recent_files.json, using empty list");
            }
        }
    }

    pub fn save_recent_files(&self) {
        let dir = get_config_dir();
        let _ = fs::create_dir_all(&dir);
        let path = dir.join("recent_files.json");
        if let Ok(data) = serde_json::to_string_pretty(&self.recent_files) {
            let _ = fs::write(path, data);
        }
    }

    pub fn add_recent_file(&mut self, path: &PathBuf) {
        let name = path.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        
        self.recent_files.retain(|f| f.path != path.to_string_lossy());
        
        let new_file = RecentFile {
            path: path.to_string_lossy().to_string(),
            name,
            last_opened: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
        };
        
        self.recent_files.insert(0, new_file);
        if self.recent_files.len() > 10 {
            self.recent_files.truncate(10);
        }
        self.save_recent_files();
    }
    
    pub fn load_session(&mut self) -> Option<SessionData> {
        let path = get_config_dir().join("session.json");
        if let Ok(data) = fs::read_to_string(&path) {
            match serde_json::from_str::<SessionData>(&data) {
                Ok(session) => return Some(session),
                Err(e) => eprintln!("Warning: Corrupted session.json: {}", e),
            }
        }
        None
    }

    pub fn save_session(&self) {
        if !self.settings.restore_session {
            return;
        }
        let dir = get_config_dir();
        let _ = fs::create_dir_all(&dir);
        let path = dir.join("session.json");
        
        let session = SessionData {
            open_tabs: self.tabs.iter().map(|t| t.path.clone()).collect(),
            active_tab: self.active_tab,
        };

        if let Ok(data) = serde_json::to_string_pretty(&session) {
            let _ = fs::write(path, data);
        }
    }

    fn current_tab(&self) -> Option<&DocumentTab> {
        self.tabs.get(self.active_tab)
    }

    fn current_tab_mut(&mut self) -> Option<&mut DocumentTab> {
        self.tabs.get_mut(self.active_tab)
    }

    fn render_visible_pages(&mut self) -> Task<Message> {
        let tab = match self.current_tab() {
            Some(t) => t,
            None => return Task::none(),
        };
        
        let visible_pages: Vec<usize> = tab.get_visible_pages().iter().cloned().collect();
        if visible_pages.is_empty() {
            return Task::none();
        }
        
        let doc_id = tab.id;
        let zoom = tab.zoom;
        let rotation = tab.rotation;
        let filter = tab.render_filter;
        let auto_crop = tab.auto_crop;
        
        let engine = match &self.engine {
            Some(e) => e,
            None => return Task::none(),
        };
        
        let mut tasks = Vec::new();
        for page_idx in visible_pages {
            if tab.rendered_pages.contains_key(&page_idx) {
                continue;
            }
            let cmd_tx = engine.cmd_tx.clone();
            tasks.push(Task::perform(
                async move {
                    let (resp_tx, mut resp_rx) = mpsc::channel(1);
                    let _ = cmd_tx.send(PdfCommand::Render(doc_id, page_idx as i32, zoom, rotation, filter, auto_crop, resp_tx)).await;
                    resp_rx.recv().await.unwrap_or(Err("Channel closed".into()))
                },
                Message::PageRendered,
            ));
        }
        Task::batch(tasks)
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        if !self.loaded {
            self.loaded = true;
            self.load_settings();
            self.load_recent_files();
            let session = self.load_session();
            if self.settings.theme == AppTheme::System {
                match dark_light::detect() {
                    dark_light::Mode::Dark => self.settings.theme = AppTheme::Dark,
                    _ => self.settings.theme = AppTheme::Light,
                }
            }
            if self.settings.restore_session {
                if let Some(mut session_data) = session {
                    let target_tab = session_data.active_tab;
                    let mut tasks = Vec::new();
                    for path in session_data.open_tabs.drain(..) {
                        tasks.push(self.update(Message::OpenFile(path)));
                    }
                    if !tasks.is_empty() {
                        tasks.push(Task::perform(async move {
                            Message::SwitchTab(target_tab)
                        }, |m| m));
                        return Task::batch(tasks);
                    }
                }
            }
        }
        
        match message {
            Message::ResetZoom => {
                if let Some(tab) = self.current_tab_mut() {
                    tab.zoom = 1.0;
                    tab.rendered_pages.clear();
                }
                self.render_visible_pages()
            }
            Message::OpenSettings => {
                self.show_settings = true;
                Task::none()
            }
            Message::CloseSettings => {
                self.show_settings = false;
                Task::none()
            }
            Message::SaveSettings(settings) => {
                self.settings = settings;
                self.save_settings();
                self.show_settings = false;
                Task::none()
            }
            Message::ToggleSidebar => {
                self.show_sidebar = !self.show_sidebar;
                Task::none()
            }
            Message::ToggleFullscreen => {
                self.is_fullscreen = !self.is_fullscreen;
                Task::none()
            }
            Message::ToggleKeyboardHelp => {
                self.show_keyboard_help = !self.show_keyboard_help;
                Task::none()
            }
            Message::RotateClockwise => {
                if let Some(tab) = self.current_tab_mut() {
                    tab.rotation = (tab.rotation + 90) % 360;
                    tab.rendered_pages.clear();
                }
                self.render_visible_pages()
            }
            Message::RotateCounterClockwise => {
                if let Some(tab) = self.current_tab_mut() {
                    tab.rotation = (tab.rotation - 90 + 360) % 360;
                    tab.rendered_pages.clear();
                }
                self.render_visible_pages()
            }
            Message::AddBookmark => {
                if let Some(tab) = self.current_tab_mut() {
                    let page = tab.current_page;
                    let label = format!("Page {}", page + 1);
                    let bookmark = PageBookmark {
                        page,
                        label,
                        created_at: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs(),
                    };
                    if !tab.bookmarks.iter().any(|b| b.page == page) {
                        tab.bookmarks.push(bookmark);
                    }
                }
                Task::none()
            }
            Message::RemoveBookmark(idx) => {
                if let Some(tab) = self.current_tab_mut() {
                    if idx < tab.bookmarks.len() {
                        tab.bookmarks.remove(idx);
                    }
                }
                Task::none()
            }
            Message::JumpToBookmark(idx) => {
                if let Some(tab) = self.current_tab_mut() {
                    if idx < tab.bookmarks.len() {
                        tab.current_page = tab.bookmarks[idx].page;
                    }
                }
                Task::none()
            }
            Message::AddHighlight => {
                let accent_color = self.settings.accent_color.clone();
                if let Some(tab) = self.current_tab_mut() {
                    let page = tab.current_page;
                    let annotation = models::Annotation {
                        id: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
                            .map(|d| d.as_millis() as u64)
                            .unwrap_or(0),
                        page,
                        style: models::AnnotationStyle::Highlight { color: accent_color },
                        x: 100.0,
                        y: 100.0,
                        width: 200.0,
                        height: 50.0,
                    };
                    tab.annotations.push(annotation);
                }
                self.save_session();
                Task::none()
            }
            Message::AddRectangle => {
                if let Some(tab) = self.current_tab_mut() {
                    let page = tab.current_page;
                    let annotation = models::Annotation {
                        id: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
                            .map(|d| d.as_millis() as u64)
                            .unwrap_or(0),
                        page,
                        style: models::AnnotationStyle::Rectangle { color: "#ff0000".to_string(), thickness: 2.0, fill: false },
                        x: 150.0,
                        y: 150.0,
                        width: 150.0,
                        height: 100.0,
                    };
                    tab.annotations.push(annotation);
                }
                self.save_session();
                Task::none()
            }
            Message::DeleteAnnotation(idx) => {
                if let Some(tab) = self.current_tab_mut() {
                    if idx < tab.annotations.len() {
                        tab.annotations.remove(idx);
                    }
                }
                self.save_session();
                Task::none()
            }
            Message::SetFilter(filter) => {
                if let Some(tab) = self.current_tab_mut() {
                    if tab.render_filter != filter {
                        tab.render_filter = filter;
                        tab.rendered_pages.clear();
                    }
                }
                self.render_visible_pages()
            }
            Message::ToggleAutoCrop => {
                if let Some(tab) = self.current_tab_mut() {
                    tab.auto_crop = !tab.auto_crop;
                    tab.rendered_pages.clear();
                }
                self.render_visible_pages()
            }
            Message::OpenDocument => {
                if self.engine.is_none() {
                    let (cmd_tx, mut cmd_rx) = mpsc::channel(32);
                    
                    std::thread::spawn(move || {
                        use std::collections::HashMap;
                        
                        let doc_paths: Arc<tokio::sync::Mutex<HashMap<DocumentId, String>>> = 
                            Arc::new(tokio::sync::Mutex::new(HashMap::new()));
                        use std::sync::atomic::{AtomicU64, Ordering};
                        static NEXT_ID: AtomicU64 = AtomicU64::new(1);
                        
                        let next_id = || DocumentId(NEXT_ID.fetch_add(1, Ordering::Relaxed));

                        let rt = match tokio::runtime::Builder::new_multi_thread()
                            .worker_threads(4)
                            .enable_all()
                            .build()
                        {
                            Ok(rt) => rt,
                            Err(e) => {
                                eprintln!("Failed to create Tokio runtime: {}", e);
                                return;
                            }
                        };
                        
                        rt.block_on(async move {
                            while let Some(cmd) = cmd_rx.recv().await {
                                let doc_paths = doc_paths.clone();
                                match cmd {
                                    PdfCommand::Open(path, resp) => {
                                        let id = next_id();
                                        let pdfium = match pdf_engine::PdfEngine::init_pdfium() {
                                            Ok(p) => p,
                                            Err(e) => {
                                                let _ = resp.blocking_send(Err(e));
                                                continue;
                                            }
                                        };
                                        let mut engine = pdf_engine::PdfEngine::new(&pdfium);
                                        match engine.open_document(&path) {
                                            Ok((count, heights, width)) => {
                                                let outline = engine.get_outline();
                                                doc_paths.lock().await.insert(id, path);
                                                
                                                let _ = resp.blocking_send(Ok((id, count, heights, width, outline)));
                                            }
                                            Err(e) => {
                                                let _ = resp.blocking_send(Err(e));
                                            }
                                        }
                                    }
                                    PdfCommand::Render(doc_id, page, zoom, rotation, filter, auto_crop, resp) => {
                                        let path = doc_paths.lock().await.get(&doc_id).cloned();
                                        if let Some(path) = path {
                                            let pdfium = match pdf_engine::PdfEngine::init_pdfium() {
                                                Ok(p) => p,
                                                Err(e) => {
                                                    let _ = resp.blocking_send(Err(e));
                                                    continue;
                                                }
                                            };
                                            let mut engine = pdf_engine::PdfEngine::new(&pdfium);
                                            if let Err(e) = engine.open_document(&path) {
                                                let _ = resp.blocking_send(Err(e));
                                            } else {
                                                let res = engine.render_page(page, zoom, rotation, filter, auto_crop);
                                                match res {
                                                    Ok((w, h, data)) => {
                                                        let _ = resp.blocking_send(Ok((page as usize, w, h, data)));
                                                    }
                                                    Err(e) => {
                                                        let _ = resp.blocking_send(Err(e));
                                                    }
                                                }
                                            }
                                        } else {
                                            let _ = resp.blocking_send(Err("Document not found".into()));
                                        }
                                    }
                                    PdfCommand::ExtractText(doc_id, page, resp) => {
                                        let path = doc_paths.lock().await.get(&doc_id).cloned();
                                        if let Some(path) = path {
                                            let pdfium = match pdf_engine::PdfEngine::init_pdfium() {
                                                Ok(p) => p,
                                                Err(e) => {
                                                    let _ = resp.blocking_send(Err(e));
                                                    continue;
                                                }
                                            };
                                            let mut engine = pdf_engine::PdfEngine::new(&pdfium);
                                            let res = if engine.open_document(&path).is_ok() {
                                                engine.extract_text(page)
                                            } else {
                                                Err("Failed to open document".into())
                                            };
                                            let _ = resp.blocking_send(res);
                                        } else {
                                            let _ = resp.blocking_send(Err("Document not found".into()));
                                        }
                                    }
                                    PdfCommand::ExportImage(doc_id, page, zoom, path, resp) => {
                                        let doc_path = doc_paths.lock().await.get(&doc_id).cloned();
                                        if let Some(doc_path) = doc_path {
                                            let pdfium = match pdf_engine::PdfEngine::init_pdfium() {
                                                Ok(p) => p,
                                                Err(e) => {
                                                    let _ = resp.blocking_send(Err(e));
                                                    continue;
                                                }
                                            };
                                            let mut engine = pdf_engine::PdfEngine::new(&pdfium);
                                            let res = if engine.open_document(&doc_path).is_ok() {
                                                engine.export_page_as_image(page, zoom, &path)
                                            } else {
                                                Err("Failed to open document".into())
                                            };
                                            let _ = resp.blocking_send(res);
                                        } else {
                                            let _ = resp.blocking_send(Err("Document not found".into()));
                                        }
                                    }
                                    PdfCommand::ExportImages(doc_id, pages, zoom, output_dir, resp) => {
                                        let path = doc_paths.lock().await.get(&doc_id).cloned();
                                        if let Some(path) = path {
                                            let pdfium = match pdf_engine::PdfEngine::init_pdfium() {
                                                Ok(p) => p,
                                                Err(e) => {
                                                    let _ = resp.blocking_send(Err(e));
                                                    continue;
                                                }
                                            };
                                            let mut engine = pdf_engine::PdfEngine::new(&pdfium);
                                            let res = if engine.open_document(&path).is_ok() {
                                                engine.export_pages_as_images(&pages, zoom, &output_dir)
                                            } else {
                                                Err("Failed to open document".into())
                                            };
                                            let _ = resp.blocking_send(res);
                                        } else {
                                            let _ = resp.blocking_send(Err("Document not found".into()));
                                        }
                                    }
                                    PdfCommand::ExportPdf(doc_id, pdf_path, annotations, resp) => {
                                        let path = doc_paths.lock().await.get(&doc_id).cloned();
                                        if let Some(path) = path {
                                            let pdfium = match pdf_engine::PdfEngine::init_pdfium() {
                                                Ok(p) => p,
                                                Err(e) => {
                                                    let _ = resp.blocking_send(Err(e));
                                                    continue;
                                                }
                                            };
                                            let mut engine = pdf_engine::PdfEngine::new(&pdfium);
                                            let res = if engine.open_document(&path).is_ok() {
                                                engine.save_annotations(&annotations, &pdf_path)
                                            } else {
                                                Err("Failed to open document".into())
                                            };
                                            let _ = resp.blocking_send(res);
                                        } else {
                                            let _ = resp.blocking_send(Err("Document not found".into()));
                                        }
                                    }
                                    PdfCommand::Search(doc_id, query, resp) => {
                                        let path = doc_paths.lock().await.get(&doc_id).cloned();
                                        if let Some(path) = path {
                                            let pdfium = match pdf_engine::PdfEngine::init_pdfium() {
                                                Ok(p) => p,
                                                Err(e) => {
                                                    let _ = resp.blocking_send(Err(e));
                                                    continue;
                                                }
                                            };
                                            let mut engine = pdf_engine::PdfEngine::new(&pdfium);
                                            let res = if engine.open_document(&path).is_ok() {
                                                engine.search(&query, None)
                                            } else {
                                                Err("Failed to open document".into())
                                            };
                                            let _ = resp.blocking_send(res);
                                        } else {
                                            let _ = resp.blocking_send(Err("Document not found".into()));
                                        }
                                    }
                                    PdfCommand::LoadAnnotations(_doc_id, pdf_path, resp) => {
                                        let path = doc_paths.lock().await.values().next().cloned();
                                        if let Some(path) = path {
                                            let pdfium = match pdf_engine::PdfEngine::init_pdfium() {
                                                Ok(p) => p,
                                                Err(e) => {
                                                    let _ = resp.blocking_send(Err(e));
                                                    continue;
                                                }
                                            };
                                            let mut engine = pdf_engine::PdfEngine::new(&pdfium);
                                            let res = if engine.open_document(&path).is_ok() {
                                                engine.load_annotations(&pdf_path)
                                            } else {
                                                Err("Failed to open document".into())
                                            };
                                            let _ = resp.blocking_send(res);
                                        } else {
                                            let _ = resp.blocking_send(Err("No document found".into()));
                                        }
                                    }
                                    PdfCommand::Close(doc_id) => {
                                        doc_paths.lock().await.remove(&doc_id);
                                    }
                                }
                            }
                        });
                    });
                    
                    self.engine = Some(EngineState { 
                        cmd_tx, 
                        documents: std::collections::HashMap::new(),
                    });
                }

                if let Some(engine) = &self.engine {
                    let cmd_tx = engine.cmd_tx.clone();
                    return Task::perform(
                        async move {
                            let file = rfd::AsyncFileDialog::new()
                                .add_filter("PDF", &["pdf"])
                                .pick_file()
                                .await;
                            
                            if let Some(file) = file {
                                let path = file.path().to_path_buf();
                                let path_s = path.to_string_lossy().to_string();
                                let (resp_tx, mut resp_rx) = mpsc::channel(1);
                                let _ = cmd_tx.send(PdfCommand::Open(path_s, resp_tx)).await;
                                match resp_rx.recv().await {
                                    Some(Ok(data)) => Some((path, data)),
                                    _ => None,
                                }
                            } else {
                                None
                            }
                        },
                        |result| {
                            match result {
                                Some((path, data)) => Message::DocumentOpenedWithPath((path, data)),
                                None => Message::DocumentOpened(Err("Cancelled".into())),
                            }
                        }
                    );
                }
                Task::none()
            }
            Message::DocumentOpenedWithPath((path, data)) => {
                if !path.as_path().exists() {
                    return Task::none();
                }
                
                let tab = DocumentTab::new(path.clone());
                let tab_idx = self.tabs.len();
                self.tabs.push(tab);
                self.active_tab = tab_idx;
                self.add_recent_file(&path);
                
                let doc_id = data.0;
                if let Some(engine) = &mut self.engine {
                    engine.documents.insert(doc_id, path.to_string_lossy().to_string());
                }
                
                self.update(Message::DocumentOpened(Ok(data)))
            }
            Message::DocumentOpened(result) => {
                match result {
                    Ok((doc_id, count, heights, width, outline)) => {
                        let default_zoom = self.settings.default_zoom;
                        let default_filter = self.settings.default_filter;
                        let pdf_path = if let Some(tab) = self.tabs.get(self.active_tab) {
                            Some(tab.path.to_string_lossy().to_string())
                        } else {
                            None
                        };
                        
                        if let Some(tab) = self.tabs.get_mut(self.active_tab) {
                            tab.id = doc_id;
                            tab.total_pages = count;
                            tab.page_heights = heights;
                            tab.page_width = width;
                            tab.outline = outline;
                            tab.is_loading = false;
                            tab.zoom = default_zoom;
                            tab.render_filter = default_filter;
                        }
                        
                        if let Some(path_str) = pdf_path {
                            if let Some(engine) = &self.engine {
                                let cmd_tx = engine.cmd_tx.clone();
                                let doc_id = doc_id;
                                return Task::perform(
                                    async move {
                                        let (resp_tx, mut resp_rx) = mpsc::channel(1);
                                        let _ = cmd_tx.send(PdfCommand::LoadAnnotations(doc_id, path_str, resp_tx)).await;
                                        match resp_rx.recv().await {
                                            Some(Ok(annotations)) => (doc_id, annotations),
                                            Some(Err(_)) => (doc_id, Vec::new()),
                                            None => (doc_id, Vec::new()),
                                        }
                                    },
                                    |(doc_id, annotations)| Message::AnnotationsLoaded(doc_id, annotations),
                                );
                            }
                        }
                        self.save_session();
                        self.render_visible_pages()
                    }
                    Err(e) => {
                        eprintln!("Error opening document: {}", e);
                        if !self.tabs.is_empty() {
                            self.tabs.pop();
                        }
                        Task::none()
                    }
                }
            }
            Message::AnnotationsLoaded(doc_id, annotations) => {
                if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == doc_id) {
                    tab.annotations = annotations;
                }
                self.render_visible_pages()
            }
            Message::OpenFile(path) => {
                if self.engine.is_none() {
                    return self.update(Message::OpenDocument);
                }

                let tab = DocumentTab::new(path.clone());
                self.tabs.push(tab);
                self.active_tab = self.tabs.len() - 1;
                self.add_recent_file(&path);

                if let Some(engine) = &self.engine {
                    let cmd_tx = engine.cmd_tx.clone();
                    let path_s = path.to_string_lossy().to_string();
                    return Task::perform(
                        async move {
                            let (resp_tx, mut resp_rx) = mpsc::channel(1);
                            let _ = cmd_tx.send(PdfCommand::Open(path_s, resp_tx)).await;
                            resp_rx.recv().await.unwrap_or(Err("Engine died".into()))
                        },
                        Message::DocumentOpened,
                    );
                }
                Task::none()
            }
            Message::OpenRecentFile(file) => {
                let path = PathBuf::from(&file.path);
                if path.exists() {
                    return self.update(Message::OpenFile(path));
                }
                self.recent_files.retain(|f| f.path != file.path);
                self.save_recent_files();
                Task::none()
            }
            Message::CloseTab(idx) => {
                if idx >= self.tabs.len() {
                    return Task::none();
                }
                
                let tab = self.tabs.remove(idx);
                if let Some(engine) = &self.engine {
                    let cmd_tx = engine.cmd_tx.clone();
                    let doc_id = tab.id;
                    let _ = cmd_tx.try_send(PdfCommand::Close(doc_id));
                }
                
                if self.active_tab >= self.tabs.len() && !self.tabs.is_empty() {
                    self.active_tab = self.tabs.len() - 1;
                }
                self.save_session();
                Task::none()
            }
            Message::SwitchTab(idx) => {
                if idx < self.tabs.len() && idx != self.active_tab {
                    self.active_tab = idx;
                    self.save_session();
                }
                Task::none()
            }
            Message::NextPage => {
                if let Some(tab) = self.current_tab_mut() {
                    if tab.current_page + 1 < tab.total_pages {
                        tab.current_page += 1;
                    }
                }
                Task::none()
            }
            Message::PrevPage => {
                if let Some(tab) = self.current_tab_mut() {
                    if tab.current_page > 0 {
                        tab.current_page -= 1;
                    }
                }
                Task::none()
            }
            Message::ZoomIn => {
                if let Some(tab) = self.current_tab_mut() {
                    tab.zoom = (tab.zoom * 1.25).min(5.0);
                    tab.rendered_pages.clear();
                }
                self.render_visible_pages()
            }
            Message::ZoomOut => {
                if let Some(tab) = self.current_tab_mut() {
                    tab.zoom = (tab.zoom / 1.25).max(0.25);
                    tab.rendered_pages.clear();
                }
                self.render_visible_pages()
            }
            Message::SetZoom(zoom) => {
                if let Some(tab) = self.current_tab_mut() {
                    tab.zoom = zoom.clamp(0.25, 5.0);
                    tab.rendered_pages.clear();
                }
                self.render_visible_pages()
            }
            Message::JumpToPage(page) => {
                if let Some(tab) = self.current_tab_mut() {
                    if page < tab.total_pages {
                        tab.current_page = page;
                    }
                }
                Task::none()
            }
            Message::ViewportChanged(y, height) => {
                if let Some(tab) = self.current_tab_mut() {
                    tab.viewport_y = y;
                    tab.viewport_height = height;
                    tab.cleanup_distant_pages();
                }
                self.render_visible_pages()
            }
            Message::SidebarViewportChanged(y) => {
                if let Some(tab) = self.current_tab_mut() {
                    tab.sidebar_viewport_y = y;
                }
                Task::none()
            }
            Message::RequestRender(page_idx) => {
                let (doc_id, zoom, rotation, filter, auto_crop) = {
                    let tab = match self.current_tab() {
                        Some(t) => t,
                        None => return Task::none(),
                    };
                    
                    if tab.rendered_pages.contains_key(&page_idx) {
                        return Task::none();
                    }
                    
                    (tab.id, tab.zoom, tab.rotation, tab.render_filter, tab.auto_crop)
                };
                
                self.rendering_count += 1;
                
                let engine = match &self.engine {
                    Some(e) => e,
                    None => return Task::none(),
                };
                
                let cmd_tx = engine.cmd_tx.clone();
                Task::perform(
                    async move {
                        let (resp_tx, mut resp_rx) = mpsc::channel(1);
                    let _ = cmd_tx.send(PdfCommand::Render(doc_id, page_idx as i32, zoom, rotation, filter, auto_crop, resp_tx)).await;
                        resp_rx.recv().await.unwrap_or(Err("Channel closed".into()))
                    },
                    Message::PageRendered,
                )
            }
            Message::PageRendered(result) => {
                self.rendering_count = self.rendering_count.saturating_sub(1);
                match result {
                    Ok((page, width, height, data)) => {
                        if let Some(tab) = self.current_tab_mut() {
                            let rgba_data = Arc::try_unwrap(data).unwrap_or_else(|arc| (*arc).clone());
                            tab.rendered_pages.insert(page, iced_image::Handle::from_rgba(width, height, rgba_data));
                        }
                    }
                    Err(e) => {
                        eprintln!("Render error: {}", e);
                    }
                }
                Task::none()
            }
            Message::Search(query) => {
                self.search_query = query.clone();
                if query.is_empty() {
                    if let Some(tab) = self.current_tab_mut() {
                        tab.search_results.clear();
                        tab.current_search_index = 0;
                    }
                    return Task::none();
                }
                self.search_pending = Some(query.clone());
                Task::perform(
                    async move {
                        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
                        Message::PerformSearch
                    },
                    |m| m,
                )
            }
            Message::PerformSearch => {
                let query = match self.search_pending.take() {
                    Some(q) => q,
                    None => return Task::none(),
                };

                if query.is_empty() {
                    return Task::none();
                }

                let tab = match self.current_tab_mut() {
                    Some(t) => t,
                    None => return Task::none(),
                };
                
                tab.search_results.clear();
                tab.current_search_index = 0;
                
                let doc_id = tab.id;
                
                let engine = match &self.engine {
                    Some(e) => e,
                    None => return Task::none(),
                };
                
                let cmd_tx = engine.cmd_tx.clone();
                Task::perform(
                    async move {
                        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
                        let (resp_tx, mut resp_rx) = mpsc::channel(1);
                        let _ = cmd_tx.send(PdfCommand::Search(doc_id, query, resp_tx)).await;
                        let mut all_results = Vec::new();
                        while let Some(res) = resp_rx.recv().await {
                            match res {
                                Ok(mut batch) => all_results.append(&mut batch),
                                Err(e) => return Err(e),
                            }
                        }
                        Ok(all_results)
                    },
                    Message::SearchResult,
                )
            }
            Message::SearchResult(result) => {
                match result {
                    Ok(results) => {
                        if let Some(tab) = self.current_tab_mut() {
                            for (page, text, y) in results {
                                tab.search_results.push(SearchResult { page, text, y_position: y });
                            }
                            
                            if !tab.search_results.is_empty() && tab.current_search_index == 0 {
                                tab.current_page = tab.search_results[0].page;
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Search error: {}", e);
                    }
                }
                Task::none()
            }
            Message::NextSearchResult => {
                if let Some(tab) = self.current_tab_mut() {
                    if !tab.search_results.is_empty() {
                        tab.current_search_index = (tab.current_search_index + 1) % tab.search_results.len();
                        tab.current_page = tab.search_results[tab.current_search_index].page;
                    }
                }
                Task::none()
            }
            Message::PrevSearchResult => {
                if let Some(tab) = self.current_tab_mut() {
                    if !tab.search_results.is_empty() {
                        tab.current_search_index = if tab.current_search_index == 0 {
                            tab.search_results.len() - 1
                        } else {
                            tab.current_search_index - 1
                        };
                        tab.current_page = tab.search_results[tab.current_search_index].page;
                    }
                }
                Task::none()
            }
            Message::ClearSearch => {
                if let Some(tab) = self.current_tab_mut() {
                    tab.search_results.clear();
                    tab.current_search_index = 0;
                }
                self.search_query.clear();
                Task::none()
            }
            Message::ExtractText => {
                let tab = match self.current_tab() {
                    Some(t) => t,
                    None => return Task::none(),
                };
                
                let page = tab.current_page as i32;
                let doc_id = tab.id;
                let path = tab.path.clone();
                
                let engine = match &self.engine {
                    Some(e) => e,
                    None => return Task::none(),
                };
                
                let cmd_tx = engine.cmd_tx.clone();
                Task::perform(
                    async move {
                        let (resp_tx, mut resp_rx) = mpsc::channel(1);
                        let _ = cmd_tx.send(PdfCommand::ExtractText(doc_id, page, resp_tx)).await;
                        let result = resp_rx.recv().await.unwrap_or(Err("Extract failed".into()));
                        (path, result)
                    },
                    |(path, result)| {
                        let result_clone = result.clone();
                        if let Ok(text) = result {
                            let txt_path = path.with_extension("txt");
                            let _ = fs::write(&txt_path, &text);
                            eprintln!("Text extracted to: {}", txt_path.display());
                        }
                        Message::TextExtracted(result_clone)
                    },
                )
            }
            Message::TextExtracted(result) => {
                if let Err(e) = result {
                    eprintln!("Text extraction error: {}", e);
                }
                Task::none()
            }
            Message::ExportImage => {
                let tab = match self.current_tab() {
                    Some(t) => t,
                    None => return Task::none(),
                };
                
                let page = tab.current_page as i32;
                let zoom = tab.zoom;
                let doc_id = tab.id;
                
                let engine = match &self.engine {
                    Some(e) => e,
                    None => return Task::none(),
                };
                
                let cmd_tx = engine.cmd_tx.clone();
                Task::perform(
                    async move {
                        let file = rfd::AsyncFileDialog::new()
                            .add_filter("PNG", &["png"])
                            .set_file_name("page.png")
                            .save_file()
                            .await;
                        
                        match file {
                            Some(f) => {
                                let path = f.path().to_string_lossy().to_string();
                                let (resp_tx, mut resp_rx) = mpsc::channel(1);
                                let _ = cmd_tx.send(PdfCommand::ExportImage(doc_id, page, zoom, path.clone(), resp_tx)).await;
                                match resp_rx.recv().await {
                                    Some(Ok(())) => Ok(path),
                                    Some(Err(e)) => Err(e),
                                    None => Err("Engine died".into()),
                                }
                            }
                            None => Err("Cancelled".into()),
                        }
                    },
                    Message::ImageExported,
                )
            }
            Message::ImageExported(result) => {
                match result {
                    Ok(path) => self.status_message = Some(format!("Exported to: {}", path)),
                    Err(e) => self.status_message = Some(format!("Export error: {}", e)),
                }
                Task::none()
            }
            Message::ExportImages => {
                let tab = match self.current_tab() {
                    Some(t) => t,
                    None => return Task::none(),
                };
                
                let total_pages = tab.total_pages;
                let zoom = tab.zoom;
                let doc_id = tab.id;
                
                let engine = match &self.engine {
                    Some(e) => e,
                    None => return Task::none(),
                };
                
                let cmd_tx = engine.cmd_tx.clone();
                Task::perform(
                    async move {
                        let folder = rfd::AsyncFileDialog::new()
                            .pick_folder()
                            .await;
                        
                        match folder {
                            Some(f) => {
                                let path = f.path().to_string_lossy().to_string();
                                let pages: Vec<i32> = (0..total_pages as i32).collect();
                                let (resp_tx, mut resp_rx) = mpsc::channel(1);
                                let _ = cmd_tx.send(PdfCommand::ExportImages(doc_id, pages, zoom, path.clone(), resp_tx)).await;
                                match resp_rx.recv().await {
                                    Some(Ok(paths)) => Ok(paths.join(", ")),
                                    Some(Err(e)) => Err(e),
                                    None => Err("Engine died".into()),
                                }
                            }
                            None => Err("Cancelled".into()),
                        }
                    },
                    Message::ImageExported,
                )
            }
            Message::SaveAnnotations => {
                let (doc_id, annotations, pdf_path) = match self.current_tab() {
                    Some(t) if !t.annotations.is_empty() => (t.id, t.annotations.clone(), t.path.to_string_lossy().to_string()),
                    _ => {
                        eprintln!("No annotations to save");
                        return Task::none();
                    }
                };
                
                let engine = match &self.engine {
                    Some(e) => e,
                    None => return Task::none(),
                };
                
                let cmd_tx = engine.cmd_tx.clone();
                Task::perform(
                    async move {
                        let (resp_tx, mut resp_rx) = mpsc::channel(1);
                        let _ = cmd_tx.send(PdfCommand::ExportPdf(doc_id, pdf_path.clone(), annotations, resp_tx)).await;
                        match resp_rx.recv().await {
                            Some(Ok(path)) => Ok(format!("Annotations saved to {}", path)),
                            Some(Err(e)) => Err(e),
                            None => Err("Engine died".into()),
                        }
                    },
                    Message::AnnotationsSaved,
                )
            }
            Message::AnnotationsSaved(result) => {
                match result {
                    Ok(msg) => self.status_message = Some(msg),
                    Err(e) => self.status_message = Some(format!("Save error: {}", e)),
                }
                Task::none()
            }
            Message::EngineInitialized(state) => {
                self.engine = Some(state);
                Task::none()
            }
            Message::Error(e) => {
                self.status_message = Some(format!("Error: {}", e));
                eprintln!("Error: {}", e);
                Task::none()
            }
            Message::ClearStatus => {
                self.status_message = None;
                Task::none()
            }
        }
    }

    pub fn view(&self) -> Element<Message> {
        ui::view(self)
    }
}

pub fn main() -> iced::Result {
    std::panic::set_hook(Box::new(|panic_info| {
        let msg = if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = panic_info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "Unknown panic".to_string()
        };
        
        let location = panic_info.location()
            .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
            .unwrap_or_else(|| "unknown location".to_string());
        
        eprintln!("PANIC at {}: {}", location, msg);
        
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("pdfbull");
        let _ = std::fs::create_dir_all(&config_dir);
        let crash_log = config_dir.join("crash.log");
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let log_entry = format!("[{}] PANIC at {}: {}\n", timestamp, location, msg);
        let _ = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&crash_log)
            .and_then(|mut f| std::io::Write::write_all(&mut f, log_entry.as_bytes()));
    }));
    
    let icon = match iced::window::icon::from_file_data(
        include_bytes!("../PDFbull.png"),
        None,
    ) {
        Ok(icon) => Some(icon),
        Err(_) => None,
    };

    iced::application("PDFbull", PdfBullApp::update, PdfBullApp::view)
        .window(iced::window::Settings {
            icon,
            ..Default::default()
        })
        .run()
}
