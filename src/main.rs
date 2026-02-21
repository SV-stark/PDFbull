// Prevent console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod pdf_engine;

use iced::widget::{
    button, column, container, image as iced_image, row, scrollable, text, 
    text_input, Space,
};
use iced::{Element, Length, Task};
use std::sync::Arc;
use tokio::sync::mpsc;
use std::path::PathBuf;
use std::fs;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub theme: String,
    pub auto_save: bool,
    pub remember_last_file: bool,
    pub default_zoom: f32,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            theme: "System".to_string(),
            auto_save: false,
            remember_last_file: true,
            default_zoom: 1.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentFile {
    pub path: String,
    pub name: String,
    pub last_opened: u64,
}

#[derive(Debug, Clone)]
pub struct DocumentTab {
    pub path: PathBuf,
    pub name: String,
    pub total_pages: usize,
    pub current_page: usize,
    pub zoom: f32,
    pub rendered_pages: std::collections::HashMap<usize, iced_image::Handle>,
    pub page_heights: Vec<f32>,
    pub page_width: f32,
    pub search_results: Vec<SearchResult>,
    pub current_search_index: usize,
    pub is_loading: bool,
    pub outline: Vec<pdf_engine::Bookmark>,
    pub viewport_y: f32,
    pub viewport_height: f32,
}

const MAX_CACHED_PAGES: usize = 10;
const VIEWPORT_BUFFER: usize = 3;

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub page: usize,
    pub text: String,
    pub y_position: f32,
}

impl DocumentTab {
    fn new(path: PathBuf) -> Self {
        Self {
            name: path.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "Untitled".to_string()),
            path,
            total_pages: 0,
            current_page: 0,
            zoom: 1.0,
            rendered_pages: std::collections::HashMap::new(),
            page_heights: Vec::new(),
            page_width: 0.0,
            search_results: Vec::new(),
            current_search_index: 0,
            is_loading: false,
            outline: Vec::new(),
            viewport_y: 0.0,
            viewport_height: 600.0,
        }
    }
    
    fn get_visible_pages(&self) -> Vec<usize> {
        let mut visible = Vec::new();
        let mut y = 0.0;
        
        for (idx, height) in self.page_heights.iter().enumerate() {
            let page_bottom = y + height + 10.0;
            let viewport_top = self.viewport_y;
            let viewport_bottom = self.viewport_y + self.viewport_height;
            
            if page_bottom >= viewport_top && y <= viewport_bottom {
                visible.push(idx);
            }
            
            if y > viewport_bottom + self.viewport_height * 2.0 {
                break;
            }
            
            y = page_bottom;
        }
        
        visible
    }
    
    fn cleanup_distant_pages(&mut self) {
        let visible = self.get_visible_pages();
        let pages_to_keep: Vec<usize> = visible.iter()
            .flat_map(|&p| {
                let start = p.saturating_sub(VIEWPORT_BUFFER);
                let end = (p + VIEWPORT_BUFFER).min(self.total_pages);
                start..end
            })
            .collect();
        
        let to_remove: Vec<usize> = self.rendered_pages.keys()
            .copied()
            .filter(|p| !pages_to_keep.contains(p))
            .collect();
        
        for p in to_remove {
            self.rendered_pages.remove(&p);
        }
    }
}

#[derive(Debug, Clone)]
enum PdfCommand {
    Open(String, mpsc::Sender<Result<(usize, Vec<f32>, f32, Vec<pdf_engine::Bookmark>), String>>),
    Render(i32, f32, mpsc::Sender<Result<(usize, u32, u32, Arc<Vec<u8>>), String>>),
    ExtractText(i32, mpsc::Sender<Result<String, String>>),
    ExportImage(i32, f32, String, mpsc::Sender<Result<(), String>>),
    Search(String, mpsc::Sender<Result<Vec<(usize, String, f32)>, String>>),
    Close,
}

#[derive(Debug, Clone)]
enum Message {
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
    PageRendered(Result<(usize, u32, u32, Arc<Vec<u8>>), String>),
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
    OpenSettings,
    CloseSettings,
    SaveSettings(AppSettings),
    OpenRecentFile(RecentFile),
    ToggleRecentFiles,
    ClearRecentFiles,
    ToggleSidebar,
}

#[derive(Debug, Clone)]
struct EngineState {
    cmd_tx: mpsc::Sender<PdfCommand>,
}

#[derive(Debug)]
struct PdfBullApp {
    tabs: Vec<DocumentTab>,
    active_tab: usize,
    settings: AppSettings,
    recent_files: Vec<RecentFile>,
    show_settings: bool,
    show_sidebar: bool,
    search_query: String,
    engine: Option<EngineState>,
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
            search_query: String::new(),
            engine: None,
        }
    }
}

impl PdfBullApp {
    fn load_settings(&mut self) {
        if let Ok(data) = fs::read_to_string("settings.json") {
            if let Ok(settings) = serde_json::from_str(&data) {
                self.settings = settings;
            }
        }
    }

    fn save_settings(&self) {
        if let Ok(data) = serde_json::to_string_pretty(&self.settings) {
            let _ = fs::write("settings.json", data);
        }
    }

    fn load_recent_files(&mut self) {
        if let Ok(data) = fs::read_to_string("recent_files.json") {
            if let Ok(files) = serde_json::from_str(&data) {
                self.recent_files = files;
            }
        }
    }

    fn save_recent_files(&self) {
        if let Ok(data) = serde_json::to_string_pretty(&self.recent_files) {
            let _ = fs::write("recent_files.json", data);
        }
    }

    fn add_recent_file(&mut self, path: &PathBuf) {
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

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
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
                                    PdfCommand::Render(page, zoom, resp) => {
                                        let res = engine.render_page(page, zoom).map(|(w, h, data)| {
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
                                Ok((path, _)) => {
                                    let tab = DocumentTab::new(path.clone());
                                    let idx = 0;
                                    Message::DocumentOpened(idx, Ok((0, Vec::new(), 0.0, Vec::new())))
                                }
                                Err(_) => Message::DocumentOpened(0, Err("Cancelled".into()))
                            }
                        }
                    );
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
                            
                            // Only render first few pages initially (lazy loading)
                            let mut tasks = Task::none();
                            let initial_pages = (count as usize).min(5);
                            for page_idx in 0..initial_pages {
                                if let Some(engine) = &self.engine {
                                    let (resp_tx, mut resp_rx) = mpsc::channel(1);
                                    let cmd_tx = engine.cmd_tx.clone();
                                    let zoom = self.tabs[tab_idx].zoom;
                                    tasks = Task::batch([
                                        tasks,
                                        Task::perform(
                                            async move {
                                                let _ = cmd_tx.send(PdfCommand::Render(page_idx as i32, zoom, resp_tx)).await;
                                                resp_rx.recv().await.unwrap_or(Err("Channel closed".into()))
                                            },
                                            Message::PageRendered
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
                if idx < self.tabs.len() {
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
                    // Only render first few pages on zoom
                    let mut tasks = Task::none();
                    for page_idx in 0..5.min(tab.total_pages) {
                        if let Some(engine) = &self.engine {
                            let (resp_tx, mut resp_rx) = mpsc::channel(1);
                            let cmd_tx = engine.cmd_tx.clone();
                            let zoom = tab.zoom;
                            tasks = Task::batch([
                                tasks,
                                Task::perform(
                                    async move {
                                        let _ = cmd_tx.send(PdfCommand::Render(page_idx as i32, zoom, resp_tx)).await;
                                        resp_rx.recv().await.unwrap_or(Err("Channel closed".into()))
                                    },
                                    Message::PageRendered
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
                    // Only render first few pages on zoom
                    let mut tasks = Task::none();
                    for page_idx in 0..5.min(tab.total_pages) {
                        if let Some(engine) = &self.engine {
                            let (resp_tx, mut resp_rx) = mpsc::channel(1);
                            let cmd_tx = engine.cmd_tx.clone();
                            let zoom = tab.zoom;
                            tasks = Task::batch([
                                tasks,
                                Task::perform(
                                    async move {
                                        let _ = cmd_tx.send(PdfCommand::Render(page_idx as i32, zoom, resp_tx)).await;
                                        resp_rx.recv().await.unwrap_or(Err("Channel closed".into()))
                                    },
                                    Message::PageRendered
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
                    // Only render first few pages
                    let mut tasks = Task::none();
                    for page_idx in 0..5.min(tab.total_pages) {
                        if let Some(engine) = &self.engine {
                            let (resp_tx, mut resp_rx) = mpsc::channel(1);
                            let cmd_tx = engine.cmd_tx.clone();
                            let zoom_val = tab.zoom;
                            tasks = Task::batch([
                                tasks,
                                Task::perform(
                                    async move {
                                        let _ = cmd_tx.send(PdfCommand::Render(page_idx as i32, zoom_val, resp_tx)).await;
                                        resp_rx.recv().await.unwrap_or(Err("Channel closed".into()))
                                    },
                                    Message::PageRendered
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
                    
                    // Find pages that became visible
                    let to_render: Vec<usize> = new_visible.iter()
                        .filter(|p| !old_visible.contains(p) && !tab.rendered_pages.contains_key(*p))
                        .copied()
                        .collect();
                    
                    // Cleanup distant pages
                    tab.cleanup_distant_pages();
                    
                    // Render newly visible pages
                    let mut tasks = Task::none();
                    for page_idx in to_render {
                        if let Some(engine) = &self.engine {
                            let (resp_tx, mut resp_rx) = mpsc::channel(1);
                            let cmd_tx = engine.cmd_tx.clone();
                            let zoom = tab.zoom;
                            tasks = Task::batch([
                                tasks,
                                Task::perform(
                                    async move {
                                        let _ = cmd_tx.send(PdfCommand::Render(page_idx as i32, zoom, resp_tx)).await;
                                        resp_rx.recv().await.unwrap_or(Err("Channel closed".into()))
                                    },
                                    Message::PageRendered
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
                            return Task::perform(
                                async move {
                                    let _ = cmd_tx.send(PdfCommand::Render(page_idx as i32, zoom, resp_tx)).await;
                                    resp_rx.recv().await.unwrap_or(Err("Channel closed".into()))
                                },
                                Message::PageRendered
                            );
                        }
                    }
                }
                Task::none()
            }
            Message::PageRendered(result) => {
                if let Ok((page, width, height, data)) = result {
                    for tab in &mut self.tabs {
                        tab.rendered_pages.insert(page, iced_image::Handle::from_rgba(width, height, data.as_ref().clone()));
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
                    
                    return Task::perform(
                        async move {
                            let file = rfd::AsyncFileDialog::new()
                                .add_filter("PNG", &["png"])
                                .set_file_name("page.png")
                                .save_file()
                                .await;
                            
                            match file {
                                Some(f) => Ok(f.path().to_string_lossy().to_string()),
                                None => Err("Cancelled".to_string()),
                            }
                        },
                        |result: Result<String, String>| {
                            Message::ImageExported(result)
                        }
                    );
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

    fn view(&self) -> Element<Message> {
        if self.show_settings {
            return self.settings_view();
        }

        if self.tabs.is_empty() {
            return self.welcome_view();
        }

        self.document_view()
    }

    fn welcome_view(&self) -> Element<Message> {
        let recent_section = if !self.recent_files.is_empty() {
            let mut files = column![];
            for file in &self.recent_files {
                let path = file.path.clone();
                let name = file.name.clone();
                files = files.push(
                    button(text(name))
                        .on_press(Message::OpenRecentFile(file.clone()))
                        .width(Length::Fill)
                );
            }
            column![
                text("Recent Files").size(20),
                Space::new().height(Length::Fixed(10.0)),
                files,
                Space::new().height(Length::Fixed(10.0)),
                button("Clear Recent").on_press(Message::ClearRecentFiles),
            ]
            .padding(20)
        } else {
            column![]
        };

        column![
            row![
                text("PDFbull").size(32).width(Length::Fill),
                button("Settings").on_press(Message::OpenSettings),
            ]
            .padding(20),
            
            column![
                text("Welcome to PDFbull").size(24),
                Space::new().height(Length::Fixed(20.0)),
                button("Open PDF").on_press(Message::OpenDocument).padding(10),
                Space::new().height(Length::Fixed(20.0)),
                recent_section,
            ]
            .align_x(iced::Alignment::Center)
            .align_x(iced::Alignment::Center)
            .width(Length::Fill)
            .height(Length::Fill),
        ]
        .into()
    }

    fn settings_view(&self) -> Element<Message> {
        let theme_buttons = row![
            button("System").on_press({
                let mut s = self.settings.clone();
                s.theme = "System".to_string();
                Message::SaveSettings(s)
            }),
            button("Light").on_press({
                let mut s = self.settings.clone();
                s.theme = "Light".to_string();
                Message::SaveSettings(s)
            }),
            button("Dark").on_press({
                let mut s = self.settings.clone();
                s.theme = "Dark".to_string();
                Message::SaveSettings(s)
            }),
        ].spacing(10);

        let behavior_buttons = row![
            button(if self.settings.remember_last_file { "Remember Last File ✓" } else { "Remember Last File" }).on_press({
                let mut s = self.settings.clone();
                s.remember_last_file = !s.remember_last_file;
                Message::SaveSettings(s)
            }),
            button(if self.settings.auto_save { "Auto-save ✓" } else { "Auto-save" }).on_press({
                let mut s = self.settings.clone();
                s.auto_save = !s.auto_save;
                Message::SaveSettings(s)
            }),
        ].spacing(10);

        column![
            row![
                text("Settings").size(24),
                Space::new().width(Length::Fill),
                button("Close").on_press(Message::CloseSettings),
            ]
            .padding(20),
            
            column![
                text("Appearance").size(18),
                theme_buttons.padding(10),
                Space::new().height(Length::Fixed(20.0)),
                text("Behavior").size(18),
                behavior_buttons.padding(10),
            ]
            .padding(20)
            .width(Length::Fixed(400.0))
        ]
        .align_x(iced::Alignment::Center)
        .align_x(iced::Alignment::Center)
        .into()
    }

    fn document_view(&self) -> Element<Message> {
        let tab = &self.tabs[self.active_tab];
        
        let mut tabs_row = row![];
        for (idx, t) in self.tabs.iter().enumerate() {
            let is_active = idx == self.active_tab;
            let name = t.name.clone();
            tabs_row = tabs_row.push(
                button(text(name))
                    .on_press(Message::SwitchTab(idx))
            );
        }
        tabs_row = tabs_row.push(button("+").on_press(Message::OpenDocument));

        let toolbar = row![
            button("Open").on_press(Message::OpenDocument),
            button("Close").on_press(Message::CloseTab(self.active_tab)),
            Space::new().width(Length::Fixed(10.0)),
            button("-").on_press(Message::ZoomOut(self.active_tab)),
            text(format!("{}%", (tab.zoom * 100.0) as u32)),
            button("+").on_press(Message::ZoomIn(self.active_tab)),
            Space::new().width(Length::Fixed(10.0)),
            text_input("Search...", &self.search_query)
                .on_input(Message::Search)
                .width(Length::Fixed(200.0)),
            Space::new().width(Length::Fixed(10.0)),
            button("Text").on_press(Message::ExtractText(self.active_tab)),
            button("Export").on_press(Message::ExportImage(self.active_tab)),
            Space::new().width(Length::Fill),
            button("Settings").on_press(Message::OpenSettings),
        ]
        .padding(10);

        let page_nav = row![
            button("Prev").on_press(Message::PrevPage(self.active_tab)),
            text(format!("Page {} of {}", tab.current_page + 1, tab.total_pages.max(1))),
            button("Next").on_press(Message::NextPage(self.active_tab)),
            Space::new().width(Length::Fixed(20.0)),
            text_input("Go to page", &tab.current_page.to_string())
                .on_input(move |v| {
                    if let Ok(page) = v.parse::<usize>() {
                        Message::JumpToPage(self.active_tab, page.saturating_sub(1))
                    } else {
                        Message::JumpToPage(self.active_tab, 0)
                    }
                })
                .width(Length::Fixed(80.0)),
        ]
        .padding(5);

        let content: Element<Message> = if tab.total_pages == 0 {
            container(text(if tab.is_loading { "Loading..." } else { "No pages" }))
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .into()
        } else {
            let mut pdf_column = column![].spacing(10.0).padding(10.0);
            
            for page_idx in 0..tab.total_pages {
                if let Some(handle) = tab.rendered_pages.get(&page_idx) {
                    let img = iced::widget::Image::new(handle.clone());
                    pdf_column = pdf_column.push(container(img).padding(5));
                } else {
                    pdf_column = pdf_column.push(container(text(format!("Page {}", page_idx + 1))).padding(20));
                }
            }

            scrollable(container(pdf_column).width(Length::Fill))
                .height(Length::Fill)
                .into()
        };

        column![
            tabs_row,
            toolbar,
            page_nav,
            content,
        ]
        .into()
    }
}

pub fn main() -> iced::Result {
    let mut app = PdfBullApp::default();
    app.load_settings();
    app.load_recent_files();
    
    iced::run(PdfBullApp::update, PdfBullApp::view)
}
