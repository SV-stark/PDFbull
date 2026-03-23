use crate::engine::EngineState;
use crate::message::Message;
use crate::models::{AppSettings, DocumentTab, RecentFile};
use crate::ui;
use crate::update::handle_message;
use iced::futures::SinkExt;
use iced::{Element, Font, Task};

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
    pub const COPY: &str = "\u{e03f}";
    pub const MERGE: &str = "\u{e0dc}";
    pub const FORMS: &str = "\u{e2a8}";
    pub const PRINT: &str = "\u{e13f}";
    pub const BLOCK: &str = "\u{e021}";
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
    pub show_forms_sidebar: bool,
    pub show_metadata: bool,
    pub form_fields: Vec<crate::models::FormField>,
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
    pub modifiers: iced::keyboard::Modifiers,
    pub last_session_save: std::time::Instant,
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
            show_forms_sidebar: false,
            show_metadata: false,
            form_fields: Vec::new(),
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
            modifiers: iced::keyboard::Modifiers::default(),
            last_session_save: std::time::Instant::now(),
        }
    }
}

impl PdfBullApp {
    #[must_use]
    pub fn current_tab(&self) -> Option<&DocumentTab> {
        self.tabs.get(self.active_tab)
    }

    pub fn current_tab_mut(&mut self) -> Option<&mut DocumentTab> {
        self.tabs.get_mut(self.active_tab)
    }

    pub fn save_session(&mut self) {
        if !self.settings.restore_session {
            return;
        }

        if self.last_session_save.elapsed() < std::time::Duration::from_secs(5) {
            return;
        }

        self.save_session_and_recent();
    }

    pub fn save_session_and_recent(&mut self) {
        let session = crate::models::SessionData {
            open_tabs: self
                .tabs
                .iter()
                .map(|t| t.path.to_string_lossy().to_string())
                .collect(),
            active_tab: self.active_tab,
        };
        crate::storage::save_session(&session);
        crate::storage::save_recent_files(&self.recent_files);
        self.last_session_save = std::time::Instant::now();
    }

    pub fn add_recent_file(&mut self, path: &std::path::Path) {
        let path_str = path.to_string_lossy().to_string();
        self.recent_files.retain(|f| f.path != path_str);

        let new_file = RecentFile {
            path: path_str,
            name: path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default(),
            last_opened: time::OffsetDateTime::now_utc().unix_timestamp() as u64,
        };

        self.recent_files.insert(0, new_file);
        if self.recent_files.len() > 20 {
            self.recent_files.truncate(20);
        }
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
            let Some(tab) = self.current_tab_mut() else {
                return Task::none();
            };
            tab.update_visible_range();
            (
                tab.get_visible_pages().into_iter().collect::<Vec<_>>(),
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

        let mut tasks = Vec::new();
        let quality = self.settings.render_quality;

        let rendered_pages = {
            let tab = self.current_tab().unwrap();
            tab.view_state
                .rendered_pages
                .iter()
                .map(|(&p, &(s, _))| (p, s))
                .collect::<std::collections::HashMap<usize, f32>>()
        };

        for page_idx in visible_pages {
            let target = RenderTarget::Page(page_idx);

            let is_rendered = rendered_pages
                .get(&page_idx)
                .is_some_and(|&s| (s - zoom).abs() < 0.001);

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
            self.rendering_count += 1;
            let tx = cmd_tx.clone();
            let doc_id_cloned = doc_id;
            let current_scale = options.scale;
            tasks.push(Task::perform(
                async move {
                    let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();
                    let _ = tx.send(crate::commands::PdfCommand::Render(
                        doc_id_cloned,
                        page_idx,
                        options,
                        resp_tx,
                    ));
                    let res = resp_rx.await.unwrap_or_else(|_| Err("Engine died".into()));
                    (page_idx, current_scale, res)
                },
                |(page_idx, scale, res)| Message::PageRendered(page_idx, scale, res),
            ));
        }

        if self.show_sidebar {
            let rendered_thumbnails = {
                let tab = self.current_tab().unwrap();
                tab.view_state
                    .thumbnails
                    .keys()
                    .copied()
                    .collect::<std::collections::HashSet<usize>>()
            };

            for page_idx in visible_thumbnails {
                let target = RenderTarget::Thumbnail(page_idx);
                let is_thumb_rendered = rendered_thumbnails.contains(&page_idx);

                if is_thumb_rendered || self.rendering_set.contains(&target) {
                    continue;
                }
                self.rendering_count += 1;
                let thumb_zoom = (120.0 / page_width.max(1.0)).min(5.0);
                self.rendering_set.insert(target);
                let tx = cmd_tx.clone();
                let doc_id_cloned = doc_id;
                tasks.push(Task::perform(
                    async move {
                        let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();
                        let _ = tx.send(crate::commands::PdfCommand::RenderThumbnail(
                            doc_id_cloned,
                            page_idx,
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

        if tasks.is_empty() {
            return Task::none();
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
                    |()| Message::ClearStatus,
                ));
            }

            tasks.push(Task::perform(
                async move {
                    let duration = if msg_clone.len() > 60 { 8 } else { 5 };
                    tokio::time::sleep(tokio::time::Duration::from_secs(duration)).await;
                },
                |()| Message::ClearStatus,
            ));

            Task::batch(tasks)
        } else {
            task
        }
    }

    #[must_use]
    pub fn view(&self) -> Element<'_, Message> {
        ui::view(self)
    }

    pub fn subscription(&self) -> iced::Subscription<Message> {
        let events =
            iced::event::listen_with(|event, _status, _id| Some(Message::IcedEvent(event)));

        let paths: Vec<std::path::PathBuf> = self.tabs.iter().map(|t| t.path.clone()).collect();
        if paths.is_empty() {
            return events;
        }

        let watch_sub = iced::Subscription::run_with_id(
            paths.clone(),
            iced::stream::channel(10, move |mut output| async move {
                use notify_debouncer_mini::{new_debouncer, notify::RecursiveMode};
                let (tx, mut rx) = tokio::sync::mpsc::channel(10);

                let Ok(mut debouncer) = new_debouncer(
                    std::time::Duration::from_secs(1),
                    move |res: Result<Vec<notify_debouncer_mini::DebouncedEvent>, _>| {
                        if let Ok(events) = res {
                            for event in events {
                                let _ = tx.blocking_send(event.path);
                            }
                        }
                    },
                ) else {
                    return;
                };

                let watcher = debouncer.watcher();
                for path in &paths {
                    let _ = watcher.watch(path, RecursiveMode::NonRecursive);
                }

                while let Some(path) = rx.recv().await {
                    let _ = output.send(Message::DocumentModifiedExternally(path)).await;
                }
            }),
        );

        iced::Subscription::batch(vec![events, watch_sub])
    }
}
