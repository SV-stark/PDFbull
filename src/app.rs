use crate::engine::EngineState;
use crate::message::Message;
use crate::models::{AppSettings, DocumentTab, RecentFile};
use crate::ui;
use crate::update::handle_message;
use iced::{Element, Task};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RenderTarget {
    Page(usize),
    Thumbnail(usize),
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
    pub rendering_set: std::collections::HashSet<RenderTarget>,
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
            rendering_set: std::collections::HashSet::new(),
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
        
        let quality = self.settings.render_quality;
        let mut tasks = Vec::new();
        for page_idx in visible_pages {
            let target = RenderTarget::Page(page_idx);
            if tab.rendered_pages.contains_key(&page_idx) || self.rendering_set.contains(&target) {
                continue;
            }
            self.rendering_set.insert(target);
            let cmd_tx = engine.cmd_tx.clone();
            tasks.push(Task::perform(
                async move {
                    let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();
                    let _ = cmd_tx.send(crate::commands::PdfCommand::Render(doc_id, page_idx as i32, zoom, rotation, filter, auto_crop, quality, resp_tx));
                    let res = resp_rx.await.unwrap_or(Err("Channel closed".into()));
                    (page_idx, res)
                },
                |(page_idx, res)| {
                    let formatted_res = match res {
                        Ok((_, w, h, data)) => Ok((w, h, data)),
                        Err(e) => Err(e),
                    };
                    Message::PageRendered(page_idx, formatted_res)
                }
            ));
        }

        if self.show_sidebar {
            let visible_thumbnails = tab.get_visible_thumbnails();
            for page_idx in visible_thumbnails {
                let target = RenderTarget::Thumbnail(page_idx);
                if tab.thumbnails.contains_key(&page_idx) || self.rendering_set.contains(&target) {
                    continue;
                }
                self.rendering_count += 1;
                let thumb_zoom = 120.0 / tab.page_width.max(1.0);
                self.rendering_set.insert(target);
                let cmd_tx = engine.cmd_tx.clone();
                tasks.push(Task::perform(
                    async move {
                        let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();
                        let _ = cmd_tx.send(crate::commands::PdfCommand::RenderThumbnail(doc_id, page_idx as i32, thumb_zoom, resp_tx));
                        let res = resp_rx.await.unwrap_or(Err("Channel closed".into()));
                        (page_idx, res)
                    },
                    |(page_idx, res)| {
                        let formatted_res = match res {
                            Ok((_, w, h, data)) => Ok((w, h, data)),
                            Err(e) => Err(e),
                        };
                        Message::ThumbnailRendered(page_idx, formatted_res)
                    }
                ));
            }
        }

        Task::batch(tasks)
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        let old_status = self.status_message.clone();
        let task = handle_message(self, message);
        if self.status_message.is_some() && self.status_message != old_status {
            let msg = self.status_message.clone().unwrap();
            let is_critical = msg.contains("crashed") || msg.contains("missing");
            
            let mut tasks = vec![task];
            
            if is_critical {
                tasks.push(Task::perform(
                    async move {
                        rfd::AsyncMessageDialog::new()
                            .set_level(rfd::MessageLevel::Error)
                            .set_title("PDFbull Error")
                            .set_description(&msg)
                            .show()
                            .await;
                    },
                    |_| Message::ClearStatus,
                ));
            }
            
            tasks.push(Task::perform(
                async move {
                    tokio::time::sleep(tokio::time::Duration::from_secs(4)).await;
                },
                |_| Message::ClearStatus,
            ));
            
            Task::batch(tasks)
        } else {
            task
        }
    }

    pub fn view(&self) -> Element<Message> {
        ui::view(self)
    }

    pub fn subscription(&self) -> iced::Subscription<Message> {
        iced::event::listen_with(|event, _status, _id| {
            Some(Message::IcedEvent(event))
        })
    }
}
