// Prevent console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod pdf_engine;
mod models;
mod commands;
mod ui;
mod ui_keyboard_help;
mod ui_settings;
mod ui_welcome;
mod ui_document;

use models::{AppSettings, RecentFile, DocumentTab, PageBookmark, SearchResult};
use commands::PdfCommand;
use pdf_engine::RenderFilter;

use iced::widget::{
    button, column, container, image as iced_image, row, scrollable, text, 
    text_input, Space,
};
use iced::{Element, Length, Task};
use std::sync::Arc;
use tokio::sync::mpsc;
use std::path::PathBuf;
use std::fs;

#[derive(Debug, Clone)]
struct EngineState {
    cmd_tx: mpsc::Sender<PdfCommand>,
}

#[derive(Debug, Clone)]
enum Message {
    ResetZoom(usize),
    OpenSettings,
    CloseSettings,
    SaveSettings(AppSettings),
    ToggleSidebar,
    ToggleFullscreen,
    ToggleKeyboardHelp,
    RotateClockwise(usize),
    RotateCounterClockwise(usize),
    AddBookmark(usize),
    RemoveBookmark(usize, usize),
    JumpToBookmark(usize, usize),
    SetFilter(RenderFilter),
    ToggleAutoCrop,
    RequestThumbnail(usize, usize),
    ThumbnailRendered(usize, Result<(usize, u32, u32, Arc<Vec<u8>>), String>),
    DocumentOpenedWithPath((PathBuf, (usize, Vec<f32>, f32, Vec<pdf_engine::Bookmark>))),
    OpenBatchMode,
    CloseBatchMode,
    AddToBatch(String),
    ProcessBatch,
    OpenDocument,
    OpenFile(PathBuf),
    CloseTab(usize),
    SwitchTab(usize),
    NextPage(usize),
    PrevPage(usize),
    ZoomIn(usize),
    ZoomOut(usize),
    SetZoom(usize, f32),
    JumpToPage(usize, usize),
    ViewportChanged(usize, f32, f32),
    RequestRender(usize, usize),
    PageRendered(usize, Result<(usize, u32, u32, Arc<Vec<u8>>), String>),
    DocumentOpened(usize, Result<(usize, Vec<f32>, f32, Vec<pdf_engine::Bookmark>), String>),
    EngineInitialized(EngineState),
    Search(String),
    SearchResult(usize, Result<Vec<(usize, String, f32)>, String>),
    NextSearchResult(usize),
    PrevSearchResult(usize),
    ClearSearch(usize),
    ExtractText(usize),
    TextExtracted(usize, Result<String, String>),
    ExportImage(usize),
    ImageExported(Result<String, String>),
    OpenRecentFile(RecentFile),
    ToggleRecentFiles,
    ClearRecentFiles,
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
    pub show_batch_mode: bool,
    pub batch_files: Vec<String>,
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
            show_batch_mode: false,
            batch_files: Vec::new(),
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
            if let Ok(settings) = serde_json::from_str(&data) {
                self.settings = settings;
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

    fn render_pages(&self, tab_idx: usize, pages: impl Iterator<Item = usize>) -> Task<Message> {
        if tab_idx >= self.tabs.len() {
            return Task::none();
        }
        let tab = &self.tabs[tab_idx];
        let Some(engine) = &self.engine else {
            return Task::none();
        };
        
        let zoom = tab.zoom;
        let rotation = tab.rotation;
        let filter = tab.render_filter;
        let cmd_tx = engine.cmd_tx.clone();
        
        pages
            .map(|page_idx| {
                let tx = cmd_tx.clone();
                Task::perform(
                    async move {
                        let (resp_tx, mut resp_rx) = mpsc::channel(1);
                        let _ = tx.send(PdfCommand::Render(page_idx as i32, zoom, rotation, filter, resp_tx)).await;
                        resp_rx.recv().await.unwrap_or(Err("Channel closed".into()))
                    },
                    move |result| Message::PageRendered(tab_idx, result)
                )
            })
            .fold(Task::none(), |acc, t| Task::batch([acc, t]))
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        // Lazy load settings on first update
        if !self.loaded {
            self.loaded = true;
            self.load_settings();
            self.load_recent_files();
        }
        
        match message {
            Message::ResetZoom(tab_idx) => {
                if tab_idx < self.tabs.len() {
                    let tab = &mut self.tabs[tab_idx];
                    tab.zoom = 1.0;
                    tab.rendered_pages.clear();
                    let mut tasks = Task::none();
                    for page_idx in 0..5.min(tab.total_pages) {
                        if let Some(engine) = &self.engine {
                            let (resp_tx, mut resp_rx) = mpsc::channel(1);
                            let cmd_tx = engine.cmd_tx.clone();
                            let zoom = tab.zoom;
                            let rotation = tab.rotation;
                            let filter = tab.render_filter;
                            tasks = Task::batch([
                                tasks,
                                Task::perform(
                                    async move {
                                        let _ = cmd_tx.send(PdfCommand::Render(page_idx as i32, zoom, rotation, filter, resp_tx)).await;
                                        resp_rx.recv().await.unwrap_or(Err("Channel closed".into()))
                                    },
                                    move |result| Message::PageRendered(tab_idx, result)
                                )
                            ]);
                        }
                    }
                    return tasks;
                }
                Task::none()
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
            Message::RotateClockwise(tab_idx) => {
                if tab_idx < self.tabs.len() {
                    let tab = &mut self.tabs[tab_idx];
                    tab.rotation = (tab.rotation + 90) % 360;
                    tab.rendered_pages.clear();
                }
                Task::none()
            }
            Message::RotateCounterClockwise(tab_idx) => {
                if tab_idx < self.tabs.len() {
                    let tab = &mut self.tabs[tab_idx];
                    tab.rotation = (tab.rotation - 90 + 360) % 360;
                    tab.rendered_pages.clear();
                }
                Task::none()
            }
            Message::AddBookmark(tab_idx) => {
                if tab_idx < self.tabs.len() {
                    let tab = &mut self.tabs[tab_idx];
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
            Message::RemoveBookmark(tab_idx, bookmark_idx) => {
                if tab_idx < self.tabs.len() {
                    let tab = &mut self.tabs[tab_idx];
                    if bookmark_idx < tab.bookmarks.len() {
                        tab.bookmarks.remove(bookmark_idx);
                    }
                }
                Task::none()
            }
            Message::JumpToBookmark(tab_idx, bookmark_idx) => {
                if tab_idx < self.tabs.len() {
                    if bookmark_idx < self.tabs[tab_idx].bookmarks.len() {
                        self.tabs[tab_idx].current_page = self.tabs[tab_idx].bookmarks[bookmark_idx].page;
                    }
                }
                Task::none()
            }
            Message::SetFilter(filter) => {
                if self.active_tab < self.tabs.len() {
                    let tab = &mut self.tabs[self.active_tab];
                    if tab.render_filter != filter {
                        tab.render_filter = filter;
                        tab.rendered_pages.clear();
                    }
                }
                Task::none()
            }
            Message::ToggleAutoCrop => {
                if self.active_tab < self.tabs.len() {
                    let tab = &mut self.tabs[self.active_tab];
                    tab.auto_crop = !tab.auto_crop;
                    tab.rendered_pages.clear();
                }
                Task::none()
            }
            Message::RequestThumbnail(tab_idx, page_idx) => {
                if tab_idx < self.tabs.len() {
                    let tab = &mut self.tabs[tab_idx];
                    if !tab.thumbnails.contains_key(&page_idx) {
                        if let Some(engine) = &self.engine {
                            let (resp_tx, mut resp_rx) = mpsc::channel(1);
                            let cmd_tx = engine.cmd_tx.clone();
                            return Task::perform(
                                async move {
                                    let _ = cmd_tx.send(PdfCommand::RenderThumbnail(page_idx as i32, resp_tx)).await;
                                    resp_rx.recv().await.unwrap_or(Err("Channel closed".into()))
                                },
                                move |result| Message::ThumbnailRendered(tab_idx, result)
                            );
                        }
                    }
                }
                Task::none()
            }
            Message::ThumbnailRendered(tab_idx, result) => {
                if let Ok((page, width, height, data)) = result {
                    if tab_idx < self.tabs.len() {
                        self.tabs[tab_idx].thumbnails.insert(page, iced_image::Handle::from_rgba(width, height, data.as_ref().clone()));
                    }
                }
                Task::none()
            }
            Message::OpenBatchMode => {
                self.show_batch_mode = true;
                self.batch_files.clear();
                Task::none()
            }
            Message::CloseBatchMode => {
                self.show_batch_mode = false;
                Task::none()
            }
            Message::AddToBatch(path) => {
                self.batch_files.push(path);
                Task::none()
            }
            Message::ProcessBatch => {
                let files = self.batch_files.clone();
                self.show_batch_mode = false;
                if let Some(first_file) = files.first() {
                    let remaining: Vec<String> = files[1..].to_vec();
                    self.batch_files = remaining;
                    let path = PathBuf::from(first_file);
                    return self.update(Message::OpenFile(path));
                }
                Task::none()
            }
            Message::OpenDocument => {
                if self.engine.is_none() {
                    return Task::perform(async {
                        let (cmd_tx, mut cmd_rx) = mpsc::channel(32);
                        std::thread::spawn(move || {
                            let pdfium = match pdf_engine::PdfEngine::init_pdfium() {
                                Ok(p) => p,
                                Err(e) => { println!("Engine init failed: {}", e); return; }
                            };
                            let mut engine = pdf_engine::PdfEngine::new(&pdfium);
                            while let Some(cmd) = cmd_rx.blocking_recv() {
                                match cmd {
                                    PdfCommand::Open(path, resp) => {
                                        let res = engine.open_document(&path).map(|(c, h, w)| {
                                            (c, h, w, engine.get_outline())
                                        });
                                        let _ = resp.blocking_send(res);
                                    }
                                    PdfCommand::Render(page, zoom, rotation, filter, resp) => {
                                        let res = engine.render_page(page, zoom, rotation, filter).map(|(w, h, data)| {
                                            (page as usize, w, h, data)
                                        });
                                        let _ = resp.blocking_send(res);
                                    }
                                    PdfCommand::RenderThumbnail(page, resp) => {
                                        let res = engine.render_page(page, 0.2, 0, RenderFilter::None).map(|(w, h, data)| {
                                            (page as usize, w, h, data)
                                        });
                                        let _ = resp.blocking_send(res);
                                    }
                                    PdfCommand::ExtractText(page, resp) => {
                                        let res = engine.extract_text(page);
                                        let _ = resp.blocking_send(res);
                                    }
                                    PdfCommand::ExportImage(page, zoom, path, resp) => {
                                        let res = engine.export_page_as_image(page, zoom, &path);
                                        let _ = resp.blocking_send(res);
                                    }
                                    PdfCommand::Search(query, resp) => {
                                        let res = engine.search(&query);
                                        let _ = resp.blocking_send(res);
                                    }
                                    PdfCommand::Close => { engine.close_document(); }
                                }
                            }
                        });
                        EngineState { cmd_tx }
                    }, Message::EngineInitialized);
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
                                    Some(Ok(data)) => Ok((path, data)),
                                    Some(Err(e)) => Err(e),
                                    None => Err("Engine died".into()),
                                }
                            } else {
                                Err("Cancelled".into())
                            }
                        },
                        |result| {
                            match result {
                                Ok((path, data)) => Message::DocumentOpenedWithPath((path, data)),
                                Err(e) => Message::DocumentOpenedWithPath((PathBuf::new(), (0, Vec::new(), 0.0, Vec::new())))
                            }
                        }
                    );
                }
                Task::none()
            }
            Message::DocumentOpenedWithPath((path, data)) => {
                if path.as_path().exists() {
                    let tab_idx = self.tabs.len();
                    let path_clone = path.clone();
                    self.tabs.push(DocumentTab::new(path));
                    self.active_tab = tab_idx;
                    self.add_recent_file(&path_clone);
                    return self.update(Message::DocumentOpened(tab_idx, Ok(data)));
                }
                Task::none()
            }
            Message::OpenFile(path) => {
                if self.engine.is_none() {
                    return self.update(Message::OpenDocument);
                }

                let path_s = path.to_string_lossy().to_string();
                let tab_idx = self.tabs.len();
                self.tabs.push(DocumentTab::new(path.clone()));
                self.active_tab = tab_idx;
                self.add_recent_file(&path);

                if let Some(engine) = &self.engine {
                    let cmd_tx = engine.cmd_tx.clone();
                    return Task::perform(
                        async move {
                            let (resp_tx, mut resp_rx) = mpsc::channel(1);
                            let _ = cmd_tx.send(PdfCommand::Open(path_s, resp_tx)).await;
                            match resp_rx.recv().await {
                                Some(Ok(data)) => Ok((tab_idx, data)),
                                Some(Err(e)) => Err(e),
                                None => Err("Engine died".to_string()),
                            }
                        },
                        move |result| {
                            match result {
                                Ok((idx, data)) => Message::DocumentOpened(idx, Ok(data)),
                                Err(e) => Message::DocumentOpened(tab_idx, Err(e))
                            }
                        }
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
            Message::ToggleRecentFiles | Message::ClearRecentFiles => Task::none(),
            Message::EngineInitialized(state) => {
                self.engine = Some(state);
                Task::none()
            }
            Message::DocumentOpened(tab_idx, result) => {
                if tab_idx < self.tabs.len() {
                    match result {
                        Ok((count, heights, width, outline)) => {
                            self.tabs[tab_idx].total_pages = count;
                            self.tabs[tab_idx].page_heights = heights;
                            self.tabs[tab_idx].page_width = width;
                            self.tabs[tab_idx].outline = outline;
                            self.tabs[tab_idx].is_loading = false;
                            
                            let mut tasks = Task::none();
                            let initial_pages = (count as usize).min(5);
                            for page_idx in 0..initial_pages {
                                if let Some(engine) = &self.engine {
                                    let (resp_tx, mut resp_rx) = mpsc::channel(1);
                                    let cmd_tx = engine.cmd_tx.clone();
                                    let tab = &self.tabs[tab_idx];
                                    let zoom = tab.zoom;
                                    let rotation = tab.rotation;
                                    let filter = tab.render_filter;
                                    tasks = Task::batch([
                                        tasks,
                                        Task::perform(
                                            async move {
                                                let _ = cmd_tx.send(PdfCommand::Render(page_idx as i32, zoom, rotation, filter, resp_tx)).await;
                                                resp_rx.recv().await.unwrap_or(Err("Channel closed".into()))
                                            },
                                            move |result| Message::PageRendered(tab_idx, result)
                                        )
                                    ]);
                                }
                            }
                            return tasks;
                        }
                        Err(e) => {
                            println!("Error opening document: {}", e);
                            self.tabs.remove(tab_idx);
                            if self.active_tab >= self.tabs.len() && !self.tabs.is_empty() {
                                self.active_tab = self.tabs.len() - 1;
                            }
                        }
                    }
                }
                Task::none()
            }
            Message::CloseTab(idx) => {
                if idx < self.tabs.len() {
                    self.tabs.remove(idx);
                    if self.active_tab >= self.tabs.len() && !self.tabs.is_empty() {
                        self.active_tab = self.tabs.len() - 1;
                    }
                }
                Task::none()
            }
            Message::SwitchTab(idx) => {
                if idx < self.tabs.len() && idx != self.active_tab {
                    let target_path = self.tabs[idx].path.to_string_lossy().to_string();
                    
                    // Check if we need to load a different document
                    let need_load = match &self.engine {
                        Some(engine) => {
                            // For now, always reload when switching tabs since engine holds single doc
                            // TODO: Implement document pool for true multi-tab support
                            true
                        }
                        None => true,
                    };
                    
                    if need_load {
                        let tab_idx = idx;
                        let path = target_path.clone();
                        
                        if let Some(engine) = &self.engine {
                            let cmd_tx = engine.cmd_tx.clone();
                            return Task::perform(
                                async move {
                                    let (resp_tx, mut resp_rx) = mpsc::channel(1);
                                    let _ = cmd_tx.send(PdfCommand::Open(path, resp_tx)).await;
                                    resp_rx.recv().await.unwrap_or(Err("Engine died".to_string()))
                                },
                                move |result| Message::DocumentOpened(tab_idx, result)
                            );
                        }
                    }
                    
                    self.active_tab = idx;
                }
                Task::none()
            }
            Message::NextPage(tab_idx) => {
                if tab_idx < self.tabs.len() {
                    let tab = &mut self.tabs[tab_idx];
                    if tab.current_page + 1 < tab.total_pages {
                        tab.current_page += 1;
                    }
                }
                Task::none()
            }
            Message::PrevPage(tab_idx) => {
                if tab_idx < self.tabs.len() {
                    let tab = &mut self.tabs[tab_idx];
                    if tab.current_page > 0 {
                        tab.current_page -= 1;
                    }
                }
                Task::none()
            }
            Message::ZoomIn(tab_idx) => {
                if tab_idx < self.tabs.len() {
                    let tab = &mut self.tabs[tab_idx];
                    tab.zoom = (tab.zoom * 1.25).min(5.0);
                    tab.rendered_pages.clear();
                    let mut tasks = Task::none();
                    for page_idx in 0..5.min(tab.total_pages) {
                        if let Some(engine) = &self.engine {
                            let (resp_tx, mut resp_rx) = mpsc::channel(1);
                            let cmd_tx = engine.cmd_tx.clone();
                            let zoom = tab.zoom;
                            let rotation = tab.rotation;
                            let filter = tab.render_filter;
                            tasks = Task::batch([
                                tasks,
                                Task::perform(
                                    async move {
                                        let _ = cmd_tx.send(PdfCommand::Render(page_idx as i32, zoom, rotation, filter, resp_tx)).await;
                                        resp_rx.recv().await.unwrap_or(Err("Channel closed".into()))
                                    },
                                    move |result| Message::PageRendered(tab_idx, result)
                                )
                            ]);
                        }
                    }
                    return tasks;
                }
                Task::none()
            }
            Message::ZoomOut(tab_idx) => {
                if tab_idx < self.tabs.len() {
                    let tab = &mut self.tabs[tab_idx];
                    tab.zoom = (tab.zoom / 1.25).max(0.25);
                    tab.rendered_pages.clear();
                    let mut tasks = Task::none();
                    for page_idx in 0..5.min(tab.total_pages) {
                        if let Some(engine) = &self.engine {
                            let (resp_tx, mut resp_rx) = mpsc::channel(1);
                            let cmd_tx = engine.cmd_tx.clone();
                            let zoom = tab.zoom;
                            let rotation = tab.rotation;
                            let filter = tab.render_filter;
                            tasks = Task::batch([
                                tasks,
                                Task::perform(
                                    async move {
                                        let _ = cmd_tx.send(PdfCommand::Render(page_idx as i32, zoom, rotation, filter, resp_tx)).await;
                                        resp_rx.recv().await.unwrap_or(Err("Channel closed".into()))
                                    },
                                    move |result| Message::PageRendered(tab_idx, result)
                                )
                            ]);
                        }
                    }
                    return tasks;
                }
                Task::none()
            }
            Message::SetZoom(tab_idx, zoom) => {
                if tab_idx < self.tabs.len() {
                    let tab = &mut self.tabs[tab_idx];
                    tab.zoom = zoom.clamp(0.25, 5.0);
                    tab.rendered_pages.clear();
                    let mut tasks = Task::none();
                    for page_idx in 0..5.min(tab.total_pages) {
                        if let Some(engine) = &self.engine {
                            let (resp_tx, mut resp_rx) = mpsc::channel(1);
                            let cmd_tx = engine.cmd_tx.clone();
                            let zoom_val = tab.zoom;
                            let rotation = tab.rotation;
                            let filter = tab.render_filter;
                            tasks = Task::batch([
                                tasks,
                                Task::perform(
                                    async move {
                                        let _ = cmd_tx.send(PdfCommand::Render(page_idx as i32, zoom_val, rotation, filter, resp_tx)).await;
                                        resp_rx.recv().await.unwrap_or(Err("Channel closed".into()))
                                    },
                                    move |result| Message::PageRendered(tab_idx, result)
                                )
                            ]);
                        }
                    }
                    return tasks;
                }
                Task::none()
            }
            Message::JumpToPage(tab_idx, page) => {
                if tab_idx < self.tabs.len() {
                    let tab = &mut self.tabs[tab_idx];
                    if page < tab.total_pages {
                        tab.current_page = page;
                    }
                }
                Task::none()
            }
            Message::ViewportChanged(tab_idx, viewport_y, viewport_height) => {
                if tab_idx < self.tabs.len() {
                    let tab = &mut self.tabs[tab_idx];
                    let old_visible: Vec<usize> = tab.get_visible_pages();
                    tab.viewport_y = viewport_y;
                    tab.viewport_height = viewport_height;
                    let new_visible: Vec<usize> = tab.get_visible_pages();
                    
                    let to_render: Vec<usize> = new_visible.iter()
                        .filter(|p| !old_visible.contains(p) && !tab.rendered_pages.contains_key(*p))
                        .copied()
                        .collect();
                    
                    tab.cleanup_distant_pages();
                    
                    let mut tasks = Task::none();
                    for page_idx in to_render {
                        if let Some(engine) = &self.engine {
                            let (resp_tx, mut resp_rx) = mpsc::channel(1);
                            let cmd_tx = engine.cmd_tx.clone();
                            let zoom = tab.zoom;
                            let rotation = tab.rotation;
                            let filter = tab.render_filter;
                            tasks = Task::batch([
                                tasks,
                                Task::perform(
                                    async move {
                                        let _ = cmd_tx.send(PdfCommand::Render(page_idx as i32, zoom, rotation, filter, resp_tx)).await;
                                        resp_rx.recv().await.unwrap_or(Err("Channel closed".into()))
                                    },
                                    move |result| Message::PageRendered(tab_idx, result)
                                )
                            ]);
                        }
                    }
                    return tasks;
                }
                Task::none()
            }
            Message::RequestRender(tab_idx, page_idx) => {
                if tab_idx < self.tabs.len() {
                    let tab = &mut self.tabs[tab_idx];
                    if !tab.rendered_pages.contains_key(&page_idx) {
                        if let Some(engine) = &self.engine {
                            let (resp_tx, mut resp_rx) = mpsc::channel(1);
                            let cmd_tx = engine.cmd_tx.clone();
                            let zoom = tab.zoom;
                            let rotation = tab.rotation;
                            let filter = tab.render_filter;
                            return Task::perform(
                                async move {
                                    let _ = cmd_tx.send(PdfCommand::Render(page_idx as i32, zoom, rotation, filter, resp_tx)).await;
                                    resp_rx.recv().await.unwrap_or(Err("Channel closed".into()))
                                },
                                move |result| Message::PageRendered(tab_idx, result)
                            );
                        }
                    }
                }
                Task::none()
            }
            Message::PageRendered(tab_idx, result) => {
                if let Ok((page, width, height, data)) = result {
                    if tab_idx < self.tabs.len() {
                        self.tabs[tab_idx].rendered_pages.insert(page, iced_image::Handle::from_rgba(width, height, data.as_ref().clone()));
                    }
                }
                Task::none()
            }
            Message::Search(query) => {
                self.search_query = query.clone();
                if query.is_empty() || self.active_tab >= self.tabs.len() {
                    return Task::none();
                }

                let tab_idx = self.active_tab;
                if let Some(engine) = &self.engine {
                    let cmd_tx = engine.cmd_tx.clone();
                    return Task::perform(
                        async move {
                            let (resp_tx, mut resp_rx) = mpsc::channel(1);
                            let _ = cmd_tx.send(PdfCommand::Search(query, resp_tx)).await;
                            resp_rx.recv().await.unwrap_or(Err("Search failed".into()))
                        },
                        move |result| Message::SearchResult(tab_idx, result)
                    );
                }
                Task::none()
            }
            Message::SearchResult(tab_idx, result) => {
                if tab_idx < self.tabs.len() {
                    if let Ok(results) = result {
                        self.tabs[tab_idx].search_results = results
                            .into_iter()
                            .map(|(page, text, y)| SearchResult { page, text, y_position: y })
                            .collect();
                        self.tabs[tab_idx].current_search_index = 0;
                        
                        if !self.tabs[tab_idx].search_results.is_empty() {
                            self.tabs[tab_idx].current_page = self.tabs[tab_idx].search_results[0].page;
                        }
                    }
                }
                Task::none()
            }
            Message::NextSearchResult(tab_idx) => {
                if tab_idx < self.tabs.len() {
                    let tab = &mut self.tabs[tab_idx];
                    if !tab.search_results.is_empty() {
                        tab.current_search_index = (tab.current_search_index + 1) % tab.search_results.len();
                        let result = &tab.search_results[tab.current_search_index];
                        tab.current_page = result.page;
                    }
                }
                Task::none()
            }
            Message::PrevSearchResult(tab_idx) => {
                if tab_idx < self.tabs.len() {
                    let tab = &mut self.tabs[tab_idx];
                    if !tab.search_results.is_empty() {
                        if tab.current_search_index == 0 {
                            tab.current_search_index = tab.search_results.len() - 1;
                        } else {
                            tab.current_search_index -= 1;
                        }
                        let result = &tab.search_results[tab.current_search_index];
                        tab.current_page = result.page;
                    }
                }
                Task::none()
            }
            Message::ClearSearch(tab_idx) => {
                if tab_idx < self.tabs.len() {
                    self.tabs[tab_idx].search_results.clear();
                    self.tabs[tab_idx].current_search_index = 0;
                }
                self.search_query.clear();
                Task::none()
            }
            Message::ExtractText(tab_idx) => {
                if tab_idx < self.tabs.len() {
                    let tab = &self.tabs[tab_idx];
                    let page = tab.current_page as i32;
                    
                    if let Some(engine) = &self.engine {
                        let cmd_tx = engine.cmd_tx.clone();
                        return Task::perform(
                            async move {
                                let (resp_tx, mut resp_rx) = mpsc::channel(1);
                                let _ = cmd_tx.send(PdfCommand::ExtractText(page, resp_tx)).await;
                                resp_rx.recv().await.unwrap_or(Err("Extract failed".into()))
                            },
                            move |result| Message::TextExtracted(tab_idx, result)
                        );
                    }
                }
                Task::none()
            }
            Message::TextExtracted(tab_idx, result) => {
                if let Ok(text) = result {
                    if let Some(tab) = self.tabs.get(tab_idx) {
                        let txt_path = tab.path.with_extension("txt");
                        let _ = fs::write(&txt_path, &text);
                        println!("Text extracted to: {}", txt_path.display());
                    }
                }
                Task::none()
            }
            Message::ExportImage(tab_idx) => {
                if tab_idx < self.tabs.len() {
                    let tab = &self.tabs[tab_idx];
                    let page = tab.current_page as i32;
                    let zoom = tab.zoom;
                    
                    if let Some(engine) = &self.engine {
                        let cmd_tx = engine.cmd_tx.clone();
                        return Task::perform(
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
                                        let _ = cmd_tx.send(PdfCommand::ExportImage(page, zoom, path.clone(), resp_tx)).await;
                                        match resp_rx.recv().await {
                                            Some(Ok(())) => Ok(path),
                                            Some(Err(e)) => Err(e),
                                            None => Err("Engine died".into()),
                                        }
                                    }
                                    None => Err("Cancelled".to_string()),
                                }
                            },
                            Message::ImageExported
                        );
                    }
                }
                Task::none()
            }
            Message::ImageExported(result) => {
                if let Ok(path) = result {
                    println!("Image exported to: {}", path);
                }
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
