use crate::engine::EngineState;
use crate::message::Message;
use crate::models::{AppSettings, DocumentTab, RecentFile};
use crate::ui;
use crate::update::handle_message;
use iced::futures::SinkExt;
use iced::{Element, Font, Task, animation};
use std::time::Instant;

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
    pub const TEXT: &str = "\u{e25b}";
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RenderTarget {
    Page(crate::models::DocumentId, usize),
    Thumbnail(crate::models::DocumentId, usize),
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
    pub rendering_set: std::collections::HashSet<RenderTarget>,
    pub pending_text: std::collections::HashSet<(crate::models::DocumentId, usize)>,
    pub modifiers: iced::keyboard::Modifiers,
    pub cursor_position: Option<iced::Point>,
    pub last_session_save: Instant,
    pub sidebar_animation: animation::Animation<f32>,
    pub sidebar_mode: crate::models::SidebarMode,
    pub reading_mode: crate::models::ReadingMode,
    pub annotation_color: String,
    pub annotation_thickness: f32,
    pub annotation_text_size: f32,
    pub annotation_text: String,

    // Tools state
    pub show_watermark_prompt: bool,
    pub watermark_input: String,
    pub show_signature_creator: bool,
    pub signature_lines: Vec<Vec<(f32, f32)>>,
    pub signature_drag: Option<(f32, f32)>,
    pub saved_signature: Option<Vec<Vec<(f32, f32)>>>,
    pub signature_stamp_active: bool,
    pub show_page_organizer: bool,
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
            rendering_set: std::collections::HashSet::new(),
            pending_text: std::collections::HashSet::new(),
            modifiers: iced::keyboard::Modifiers::default(),
            cursor_position: None,
            last_session_save: Instant::now(),
            sidebar_animation: animation::Animation::new(0.0),
            sidebar_mode: crate::models::SidebarMode::default(),
            reading_mode: crate::models::ReadingMode::default(),
            annotation_color: "#408cff".to_string(),
            annotation_thickness: 2.0,
            annotation_text_size: 14.0,
            annotation_text: String::new(),

            // Tools default initialization
            show_watermark_prompt: false,
            watermark_input: "CONFIDENTIAL".to_string(),
            show_signature_creator: false,
            signature_lines: Vec::new(),
            signature_drag: None,
            saved_signature: None,
            signature_stamp_active: false,
            show_page_organizer: false,
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
                .map(|t| {
                    crate::models::SessionTabEntry::Detailed(crate::models::TabSession {
                        path: t.path.to_string_lossy().to_string(),
                        current_page: t.current_page,
                        zoom: t.zoom,
                        viewport_y: t.view_state.viewport_y,
                        rotation: t.rotation,
                        auto_crop: t.auto_crop,
                    })
                })
                .collect(),
            active_tab: self.active_tab,
        };
        crate::storage::save_session(&session);
        crate::storage::save_recent_files(&self.recent_files);
        self.last_session_save = std::time::Instant::now();
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

        for page_idx in visible_pages {
            let target = RenderTarget::Page(doc_id, page_idx);

            let is_rendered = self
                .current_tab()
                .and_then(|tab| tab.view_state.rendered_pages.get(&page_idx))
                .is_some_and(|&(s, _)| (s - zoom).abs() < 0.001);

            if is_rendered || self.rendering_set.contains(&target) {
                continue;
            }

            // Translate visual page index to actual source page via page_mapping.
            let (actual_page, page_rotation) = if let Some(tab) = self.current_tab() {
                let actual_page = tab.page_mapping.get(page_idx).copied().unwrap_or(page_idx);
                let rot = tab
                    .page_rotations
                    .get(&actual_page)
                    .copied()
                    .unwrap_or(rotation);
                (actual_page, rot)
            } else {
                (page_idx, rotation)
            };

            let options = crate::pdf_engine::RenderOptions {
                scale: zoom,
                rotation: page_rotation,
                filter,
                auto_crop,
                quality,
            };

            self.rendering_set.insert(target);
            let tx = cmd_tx.clone();
            let doc_id_cloned = doc_id;
            let current_scale = options.scale;
            tasks.push(Task::perform(
                async move {
                    let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();
                    let _ = tx
                        .send(crate::commands::PdfCommand::Render(
                            doc_id_cloned,
                            actual_page,
                            options,
                            resp_tx,
                        ))
                        .await;
                    let res = resp_rx
                        .await
                        .unwrap_or_else(|_| Err(crate::models::PdfError::EngineDied));
                    (page_idx, current_scale, res)
                },
                move |(page_idx, scale, res)| Message::PageRendered(doc_id, page_idx, scale, res),
            ));
        }

        if self.show_sidebar {
            for page_idx in visible_thumbnails {
                let target = RenderTarget::Thumbnail(doc_id, page_idx);
                let is_thumb_rendered = self
                    .current_tab()
                    .map(|tab| tab.view_state.thumbnails.contains_key(&page_idx))
                    .unwrap_or(false);

                if is_thumb_rendered || self.rendering_set.contains(&target) {
                    continue;
                }
                let thumb_zoom = (120.0 / page_width.max(1.0)).min(5.0);

                let (thumb_actual, thumb_rotation) = if let Some(tab) = self.current_tab() {
                    let actual_page = tab.page_mapping.get(page_idx).copied().unwrap_or(page_idx);
                    let rot = tab.page_rotations.get(&actual_page).copied().unwrap_or(0);
                    (actual_page, rot)
                } else {
                    (page_idx, 0)
                };

                self.rendering_set.insert(target);
                let tx = cmd_tx.clone();
                let doc_id_cloned = doc_id;
                tasks.push(Task::perform(
                    async move {
                        let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();
                        let _ = tx
                            .send(crate::commands::PdfCommand::RenderThumbnail(
                                doc_id_cloned,
                                thumb_actual,
                                thumb_zoom,
                                thumb_rotation,
                                resp_tx,
                            ))
                            .await;
                        let res = match resp_rx.await {
                            Ok(result) => result,
                            Err(_) => Err(crate::models::PdfError::EngineDied),
                        };
                        (page_idx, thumb_zoom, res)
                    },
                    move |(page_idx, scale, res)| {
                        Message::ThumbnailRendered(doc_id, page_idx, scale, res)
                    },
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
        let events = iced::event::listen_with(|event, _status, _id| match event {
            iced::Event::Window(
                iced::window::Event::CloseRequested | iced::window::Event::FileDropped(_),
            )
            | iced::Event::Mouse(
                iced::mouse::Event::CursorMoved { .. } | iced::mouse::Event::WheelScrolled { .. },
            )
            | iced::Event::Keyboard(
                iced::keyboard::Event::ModifiersChanged(_)
                | iced::keyboard::Event::KeyPressed { .. },
            ) => Some(Message::IcedEvent(event)),
            _ => None,
        });

        let ipc_sub = iced::Subscription::run_with("ipc-stream", |_| {
            iced::stream::channel(
                10,
                move |mut output: iced::futures::channel::mpsc::Sender<Message>| async move {
                    use iced::futures::SinkExt;
                    use interprocess::local_socket::{
                        GenericNamespaced, ListenerOptions, tokio::prelude::*,
                    };
                    use tokio::io::{AsyncBufReadExt, BufReader};

                    let name =
                        match "pdfbull-single-instance.sock".to_ns_name::<GenericNamespaced>() {
                            Ok(n) => n,
                            Err(e) => {
                                tracing::error!("Failed to create IPC socket name: {e}");
                                return;
                            }
                        };

                    let listener = match ListenerOptions::new().name(name).create_tokio() {
                        Ok(l) => l,
                        Err(e) => {
                            tracing::error!("Failed to create IPC listener: {e}");
                            return;
                        }
                    };

                    loop {
                        if let Ok(stream) = listener.accept().await {
                            let mut reader = BufReader::new(stream);
                            let mut buffer = String::new();
                            if let Ok(_) = reader.read_line(&mut buffer).await {
                                if let Ok(args) = serde_json::from_str::<Vec<String>>(&buffer) {
                                    if args.len() > 1 {
                                        let path_str = &args[1];
                                        let path_buf = std::path::PathBuf::from(path_str);
                                        if path_buf.exists() && path_buf.is_file() {
                                            let _ = output.send(Message::OpenFile(path_buf)).await;
                                        }
                                    }
                                }
                            }
                        }
                    }
                },
            )
        });

        let paths: Vec<std::path::PathBuf> = self.tabs.iter().map(|t| t.path.clone()).collect();
        if paths.is_empty() {
            return iced::Subscription::batch(vec![events, ipc_sub]);
        }

        let watch_sub = iced::Subscription::run_with(("file-watch", paths), |(_id, paths)| {
            let paths = paths.clone();
            iced::stream::channel(
                10,
                move |mut output: iced::futures::channel::mpsc::Sender<Message>| {
                    let paths = paths.clone();
                    async move {
                        use notify_debouncer_full::{new_debouncer, notify::RecursiveMode};
                        use std::time::Duration;

                        let (tx, mut rx) = tokio::sync::mpsc::channel(10);

                        let mut debouncer = match new_debouncer(
                            Duration::from_secs(1),
                            None,
                            move |res: notify_debouncer_full::DebounceEventResult| {
                                if let Ok(events) = res {
                                    for event in events {
                                        for path in &event.paths {
                                            let _ = tx.blocking_send(path.clone());
                                        }
                                    }
                                }
                            },
                        ) {
                            Ok(d) => d,
                            Err(e) => {
                                tracing::error!("Failed to create file watcher: {e}");
                                return;
                            }
                        };

                        for path in paths {
                            let _ = debouncer.watch(path, RecursiveMode::NonRecursive);
                        }

                        while let Some(path) = rx.recv().await {
                            let _ = output.send(Message::DocumentModifiedExternally(path)).await;
                        }
                    }
                },
            )
        });

        iced::Subscription::batch(vec![events, watch_sub, ipc_sub])
    }
}
