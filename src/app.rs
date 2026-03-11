use crate::engine::EngineState;
use crate::message::Message;
use crate::models::{AppSettings, DocumentTab, RecentFile};
use crate::ui;
use crate::update::handle_message;
use iced::{Element, Task, Font};

pub const INTER_REGULAR: Font = Font::with_name("Inter Regular");
pub const INTER_BOLD: Font = Font::with_name("Inter Bold");
pub const LUCIDE: Font = Font::with_name("lucide");

pub mod icons {
    pub const OPEN: &str = "\u{e247}";
    pub const SIDEBAR: &str = "\u{e115}";
    pub const ZOOM_OUT: &str = "\u{e1b7}";
    pub const ZOOM_IN: &str = "\u{e1b6}";
    pub const ROTATE: &str = "\u{e149}";
    pub const BOOKMARK: &str = "\u{e060}";
    pub const HIGHLIGHT: &str = "\u{e0f4}";
    pub const RECTANGLE: &str = "\u{e167}";
    pub const SAVE: &str = "\u{e14d}";
    pub const HELP: &str = "\u{e082}";
    pub const SETTINGS: &str = "\u{e154}";
    pub const SEARCH: &str = "\u{e151}";
    pub const PREV: &str = "\u{e06e}";
    pub const NEXT: &str = "\u{e06f}";
    pub const CLOSE: &str = "\u{e1b2}";
    pub const PLUS: &str = "\u{e13d}";
    pub const EXPORT: &str = "\u{e0b9}";
}

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
    pub page_input: String,
    pub status_message: Option<String>,
    pub annotation_mode: Option<crate::models::PendingAnnotationKind>,
    pub annotation_drag: Option<crate::models::AnnotationDrag>,
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
            page_input: "1".to_string(),
            status_message: None,
            annotation_mode: None,
            annotation_drag: None,
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

    pub fn add_recent_file(&mut self, path: &std::path::Path) {
        crate::storage::add_recent_file(&mut self.recent_files, path);
    }

    pub fn render_visible_pages(&mut self) -> Task<Message> {
        let (
            visible_pages,
            visible_thumbnails,
            doc_id,
            zoom,
            rotation,
            filter,
            auto_crop,
            page_width,
        ) = {
            let tab = match self.current_tab() {
                Some(t) => t,
                None => return Task::none(),
            };
            (
                tab.get_visible_pages().iter().cloned().collect::<Vec<_>>(),
                tab.get_visible_thumbnails(),
                tab.id,
                tab.zoom,
                tab.rotation,
                tab.render_filter,
                tab.auto_crop,
                tab.page_width,
            )
        };

        if visible_pages.is_empty() {
            return Task::none();
        }

        let cmd_tx = match &self.engine {
            Some(e) => e.cmd_tx.clone(),
            None => return Task::none(),
        };

        let quality = self.settings.render_quality;
        let mut tasks = Vec::new();
        for page_idx in visible_pages {
            let target = RenderTarget::Page(page_idx);

            let is_rendered = self
                .current_tab()
                .is_some_and(|t| t.rendered_pages.contains_key(&page_idx));
            if is_rendered || self.rendering_set.contains(&target) {
                continue;
            }

            let options = crate::pdf_engine::RenderOptions {
                scale: zoom,
                rotation,
                filter,
                auto_crop,
                quality,
            };

            self.rendering_set.insert(target);
            let tx = cmd_tx.clone();
            let doc_id_cloned = doc_id;
            tasks.push(Task::perform(
                async move {
                    let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();
                    let _ = tx.send(crate::commands::PdfCommand::Render(
                        doc_id_cloned,
                        page_idx as i32,
                        options,
                        resp_tx,
                    ));
                    let res = match resp_rx.await {
                        Ok(result) => result,
                        Err(_) => Err("Engine died".into()),
                    };
                    (page_idx, options.scale, res)
                },
                |(page_idx, scale, res)| Message::PageRendered(page_idx, scale, res),
            ));
        }

        if self.show_sidebar {
            for page_idx in visible_thumbnails {
                let target = RenderTarget::Thumbnail(page_idx);
                let is_thumb_rendered = self
                    .current_tab()
                    .is_some_and(|t| t.thumbnails.contains_key(&page_idx));

                if is_thumb_rendered || self.rendering_set.contains(&target) {
                    continue;
                }
                self.rendering_count += 1;
                let thumb_zoom = 120.0 / page_width.max(1.0);
                self.rendering_set.insert(target);
                let tx = cmd_tx.clone();
                let doc_id_cloned = doc_id;
                tasks.push(Task::perform(
                    async move {
                        let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();
                        let _ = tx.send(crate::commands::PdfCommand::RenderThumbnail(
                            doc_id_cloned,
                            page_idx as i32,
                            thumb_zoom,
                            resp_tx,
                        ));
                        let res = match resp_rx.await {
                            Ok(result) => result,
                            Err(_) => Err("Engine died".into()),
                        };
                        (page_idx, thumb_zoom, res)
                    },
                    |(page_idx, scale, res)| Message::ThumbnailRendered(page_idx, scale, res),
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
            let msg_clone = msg.clone();
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
                    let duration = if msg_clone.len() > 60 { 8 } else { 5 };
                    tokio::time::sleep(tokio::time::Duration::from_secs(duration)).await;
                },
                |_| Message::ClearStatus,
            ));

            Task::batch(tasks)
        } else {
            task
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        ui::view(self)
    }

    pub fn subscription(&self) -> iced::Subscription<Message> {
        iced::event::listen_with(|event, _status, _id| Some(Message::IcedEvent(event)))
    }
}
