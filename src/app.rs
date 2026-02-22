use crate::engine::EngineState;
use crate::message::Message;
use crate::models::{AppSettings, DocumentTab, RecentFile};
use crate::ui;
use crate::update::handle_message;
use iced::{Element, Task};

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

impl PdfBullApp {
    pub fn current_tab(&self) -> Option<&DocumentTab> {
        self.tabs.get(self.active_tab)
    }

    pub fn current_tab_mut(&mut self) -> Option<&mut DocumentTab> {
        self.tabs.get_mut(self.active_tab)
    }

    pub fn save_session(&self) {
        if !self.settings.restore_session {
            return;
        }
        let session = crate::models::SessionData {
            open_tabs: self.tabs.iter().map(|t| t.path.clone()).collect(),
            active_tab: self.active_tab,
        };
        crate::storage::save_session(&session);
    }

    pub fn add_recent_file(&mut self, path: &std::path::PathBuf) {
        crate::storage::add_recent_file(&mut self.recent_files, path);
    }

    pub fn render_visible_pages(&mut self) -> Task<Message> {
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
                    let (resp_tx, mut resp_rx) = tokio::sync::mpsc::channel(1);
                    let _ = cmd_tx.send(crate::commands::PdfCommand::Render(doc_id, page_idx as i32, zoom, rotation, filter, auto_crop, resp_tx)).await;
                    resp_rx.recv().await.unwrap_or(Err("Channel closed".into()))
                },
                Message::PageRendered,
            ));
        }
        Task::batch(tasks)
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        handle_message(self, message)
    }

    pub fn view(&self) -> Element<Message> {
        ui::view(self)
    }
}
