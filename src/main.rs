#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod pdf_engine;
mod models;
mod commands;
mod ui;
mod ui_keyboard_help;
mod ui_settings;
mod ui_welcome;
mod ui_document;

use models::{AppSettings, AppTheme, DocumentId, DocumentTab, PageBookmark, RecentFile, SearchResult};
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
    PageRendered(Result<(usize, u32, u32, Arc<Vec<u8>>), String>),
    DocumentOpened(Result<(DocumentId, usize, Vec<f32>, f32, Vec<pdf_engine::Bookmark>), String>),
    EngineInitialized(EngineState),
    Search(String),
    SearchResult(Result<Vec<(usize, String, f32)>, String>),
    NextSearchResult,
    PrevSearchResult,
    ClearSearch,
    ExtractText,
    TextExtracted(Result<String, String>),
    ExportImage,
    ImageExported(Result<String, String>),
    OpenRecentFile(RecentFile),
    Error(String),
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
    pub engine: Option<EngineState>,
    pub loaded: bool,
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
            engine: None,
            loaded: false,
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
                .unwrap_or_default()
                .as_secs(),
        };
        
        self.recent_files.insert(0, new_file);
        if self.recent_files.len() > 10 {
            self.recent_files.truncate(10);
        }
        self.save_recent_files();
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
        
        let doc_id = tab.id;
        let zoom = tab.zoom;
        let rotation = tab.rotation;
        let filter = tab.render_filter;
        let auto_crop = tab.auto_crop;
        let total = tab.total_pages;
        
        let engine = match &self.engine {
            Some(e) => e,
            None => return Task::none(),
        };
        
        let mut tasks = Vec::new();
        for page_idx in 0..5.min(total) {
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
            if self.settings.theme == AppTheme::System {
                match dark_light::detect() {
                    dark_light::Mode::Dark => self.settings.theme = AppTheme::Dark,
                    _ => self.settings.theme = AppTheme::Light,
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
                Task::none()
            }
            Message::RotateCounterClockwise => {
                if let Some(tab) = self.current_tab_mut() {
                    tab.rotation = (tab.rotation - 90 + 360) % 360;
                    tab.rendered_pages.clear();
                }
                Task::none()
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
            Message::SetFilter(filter) => {
                if let Some(tab) = self.current_tab_mut() {
                    if tab.render_filter != filter {
                        tab.render_filter = filter;
                        tab.rendered_pages.clear();
                    }
                }
                Task::none()
            }
            Message::ToggleAutoCrop => {
                if let Some(tab) = self.current_tab_mut() {
                    tab.auto_crop = !tab.auto_crop;
                    tab.rendered_pages.clear();
                }
                Task::none()
            }
            Message::OpenDocument => {
                if self.engine.is_none() {
                    let (cmd_tx, mut cmd_rx) = mpsc::channel(32);
                    
                    std::thread::spawn(move || {
                        let pdfium = match pdf_engine::PdfEngine::init_pdfium() {
                            Ok(p) => p,
                            Err(e) => {
                                eprintln!("Failed to init pdfium: {}", e);
                                return;
                            }
                        };
                        
                        let mut engines: std::collections::HashMap<DocumentId, pdf_engine::PdfEngine> = 
                            std::collections::HashMap::new();
                        let mut doc_paths: std::collections::HashMap<DocumentId, String> = 
                            std::collections::HashMap::new();
                        static mut NEXT_ID: u64 = 1;
                        
                        fn next_id() -> DocumentId {
                            unsafe {
                                let id = NEXT_ID;
                                NEXT_ID += 1;
                                DocumentId(id)
                            }
                        }
                        
                        while let Some(cmd) = cmd_rx.blocking_recv() {
                            match cmd {
                                PdfCommand::Open(path, resp) => {
                                    let id = next_id();
                                    let mut engine = pdf_engine::PdfEngine::new(&pdfium);
                                    match engine.open_document(&path) {
                                        Ok((count, heights, width)) => {
                                            let outline = engine.get_outline();
                                            engines.insert(id, engine);
                                            doc_paths.insert(id, path.clone());
                                            let _ = resp.blocking_send(Ok((id, count, heights, width, outline)));
                                        }
                                        Err(e) => {
                                            let _ = resp.blocking_send(Err(e));
                                        }
                                    }
                                }
                                PdfCommand::Render(doc_id, page, zoom, rotation, filter, auto_crop, resp) => {
                                    if let Some(engine) = engines.get(&doc_id) {
                                        match engine.render_page(page, zoom, rotation, filter, auto_crop) {
                                            Ok((w, h, data)) => {
                                                let _ = resp.blocking_send(Ok((page as usize, w, h, data)));
                                            }
                                            Err(e) => {
                                                let _ = resp.blocking_send(Err(e));
                                            }
                                        }
                                    } else {
                                        let _ = resp.blocking_send(Err("Document not found".into()));
                                    }
                                }
                                PdfCommand::ExtractText(doc_id, page, resp) => {
                                    if let Some(engine) = engines.get(&doc_id) {
                                        let _ = resp.blocking_send(engine.extract_text(page));
                                    } else {
                                        let _ = resp.blocking_send(Err("Document not found".into()));
                                    }
                                }
                                PdfCommand::ExportImage(doc_id, page, zoom, path, resp) => {
                                    if let Some(engine) = engines.get(&doc_id) {
                                        let _ = resp.blocking_send(engine.export_page_as_image(page, zoom, &path));
                                    } else {
                                        let _ = resp.blocking_send(Err("Document not found".into()));
                                    }
                                }
                                PdfCommand::Search(doc_id, query, resp) => {
                                    if let Some(engine) = engines.get(&doc_id) {
                                        let _ = resp.blocking_send(engine.search(&query));
                                    } else {
                                        let _ = resp.blocking_send(Err("Document not found".into()));
                                    }
                                }
                                PdfCommand::Close(doc_id) => {
                                    engines.remove(&doc_id);
                                    doc_paths.remove(&doc_id);
                                }
                            }
                        }
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
                        if let Some(tab) = self.current_tab_mut() {
                            tab.id = doc_id;
                            tab.total_pages = count;
                            tab.page_heights = heights;
                            tab.page_width = width;
                            tab.outline = outline;
                            tab.is_loading = false;
                        }
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
                    let _ = cmd_tx.blocking_send(PdfCommand::Close(doc_id));
                }
                
                if self.active_tab >= self.tabs.len() && !self.tabs.is_empty() {
                    self.active_tab = self.tabs.len() - 1;
                }
                Task::none()
            }
            Message::SwitchTab(idx) => {
                if idx < self.tabs.len() && idx != self.active_tab {
                    self.active_tab = idx;
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
                }
                Task::none()
            }
            Message::RequestRender(page_idx) => {
                let tab = match self.current_tab() {
                    Some(t) => t,
                    None => return Task::none(),
                };
                
                if tab.rendered_pages.contains_key(&page_idx) {
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
                match result {
                    Ok((page, width, height, data)) => {
                        if let Some(tab) = self.current_tab_mut() {
                            tab.rendered_pages.insert(page, iced_image::Handle::from_rgba(width, height, (*data).clone()));
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
                    return Task::none();
                }

                let tab = match self.current_tab() {
                    Some(t) => t,
                    None => return Task::none(),
                };
                
                let doc_id = tab.id;
                
                let engine = match &self.engine {
                    Some(e) => e,
                    None => return Task::none(),
                };
                
                let cmd_tx = engine.cmd_tx.clone();
                Task::perform(
                    async move {
                        let (resp_tx, mut resp_rx) = mpsc::channel(1);
                        let _ = cmd_tx.send(PdfCommand::Search(doc_id, query, resp_tx)).await;
                        resp_rx.recv().await.unwrap_or(Err("Search failed".into()))
                    },
                    Message::SearchResult,
                )
            }
            Message::SearchResult(result) => {
                match result {
                    Ok(results) => {
                        if let Some(tab) = self.current_tab_mut() {
                            tab.search_results = results
                                .into_iter()
                                .map(|(page, text, y)| SearchResult { page, text, y_position: y })
                                .collect();
                            tab.current_search_index = 0;
                            
                            if !tab.search_results.is_empty() {
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
                    Ok(path) => eprintln!("Image exported to: {}", path),
                    Err(e) => eprintln!("Export error: {}", e),
                }
                Task::none()
            }
            Message::EngineInitialized(state) => {
                self.engine = Some(state);
                Task::none()
            }
            Message::Error(e) => {
                eprintln!("Error: {}", e);
                Task::none()
            }
        }
    }

    pub fn view(&self) -> Element<Message> {
        ui::view(self)
    }
}

pub fn main() -> iced::Result {
    iced::run(PdfBullApp::update, PdfBullApp::view)
}
