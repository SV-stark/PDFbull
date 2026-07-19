use crate::app::PdfBullApp;
use crate::message::Message;
use crate::storage;
use iced::Task;

pub fn handle_app_message(app: &mut PdfBullApp, message: Message) -> Task<Message> {
    match message {
        Message::ResetZoom => {
            if let Some(tab) = app.current_tab_mut() {
                tab.zoom = 1.0;
            }
            app.render_visible_pages()
        }
        Message::OpenSettings => {
            app.show_settings = true;
            Task::none()
        }
        Message::CloseSettings => {
            app.show_settings = false;
            Task::none()
        }
        Message::SaveSettings(settings) => {
            app.settings = settings;
            storage::save_settings(&app.settings);
            Task::none()
        }
        Message::ToggleSidebar => {
            app.show_sidebar = !app.show_sidebar;
            let target = if app.show_sidebar { 280.0 } else { 0.0 };
            app.sidebar_animation
                .go_mut(target, std::time::Instant::now());
            Task::none()
        }
        Message::ToggleFormsSidebar => {
            app.show_forms_sidebar = !app.show_forms_sidebar;
            if app.show_forms_sidebar {
                return app.update(Message::LoadFormFields);
            }
            Task::none()
        }
        Message::ToggleFullscreen => {
            app.is_fullscreen = !app.is_fullscreen;
            Task::none()
        }
        Message::ToggleKeyboardHelp => {
            app.show_keyboard_help = !app.show_keyboard_help;
            Task::none()
        }
        Message::RotateClockwise => {
            if let Some(tab) = app.current_tab_mut() {
                tab.rotation = (tab.rotation + 90) % 360;
                tab.view_state.rendered_pages.clear();
            }
            app.render_visible_pages()
        }
        Message::RotateCounterClockwise => {
            if let Some(tab) = app.current_tab_mut() {
                tab.rotation = (tab.rotation - 90 + 360) % 360;
                tab.view_state.rendered_pages.clear();
            }
            app.render_visible_pages()
        }
        Message::ClearRecentFiles => {
            app.recent_files.clear();
            crate::storage::save_recent_files(&app.recent_files);
            Task::none()
        }
        Message::ToggleMetadata => {
            app.show_metadata = !app.show_metadata;
            Task::none()
        }
        Message::SetSidebarMode(mode) => {
            app.sidebar_mode = mode;
            Task::none()
        }
        Message::SetReadingMode(mode) => {
            use crate::pdf_engine::RenderFilter;
            app.reading_mode = mode;
            if let Some(tab) = app.current_tab_mut() {
                tab.render_filter = match mode {
                    crate::models::ReadingMode::Default => RenderFilter::None,
                    crate::models::ReadingMode::Inverted => RenderFilter::Inverted,
                    crate::models::ReadingMode::Sepia => RenderFilter::Sepia,
                    crate::models::ReadingMode::Grayscale => RenderFilter::Grayscale,
                };
                tab.view_state.rendered_pages.clear();
            }
            app.render_visible_pages()
        }
        Message::SetAnnotationColor(color) => {
            app.annotation_color = color;
            Task::none()
        }
        Message::SetAnnotationThickness(thickness) => {
            app.annotation_thickness = thickness;
            Task::none()
        }
        Message::SetAnnotationTextSize(size) => {
            app.annotation_text_size = size;
            Task::none()
        }
        Message::ToggleMarkupBar => {
            app.markup_active = !app.markup_active;
            if app.markup_active {
                app.annotation_mode = Some(crate::models::PendingAnnotationKind::Highlight);
            } else {
                app.annotation_mode = None;
                app.annotation_drag = None;
            }
            Task::none()
        }
        Message::ToggleTableMode => {
            app.table_mode_active = !app.table_mode_active;
            if app.table_mode_active {
                if let Some(tab) = app.current_tab() {
                    let doc_id = tab.id;
                    let (start_idx, end_idx) = tab.view_state.visible_range;
                    let mut tasks = Vec::new();
                    if let Some(engine) = &app.engine {
                        let cmd_tx = engine.cmd_tx.clone();
                        for page_idx in start_idx..end_idx {
                            if !tab.view_state.detected_tables.contains_key(&page_idx) {
                                let tx = cmd_tx.clone();
                                tasks.push(Task::perform(
                                    async move {
                                        let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();
                                        let _ = tx
                                            .send(crate::commands::PdfCommand::DetectTables(
                                                doc_id, page_idx, resp_tx,
                                            ))
                                            .await;
                                        match resp_rx.await {
                                            Ok(res) => (doc_id, page_idx, res),
                                            Err(_) => (
                                                doc_id,
                                                page_idx,
                                                Err(crate::models::PdfError::ChannelClosed),
                                            ),
                                        }
                                    },
                                    |(d, p, r)| Message::TablesDetected(d, p, r),
                                ));
                            }
                        }
                    }
                    if !tasks.is_empty() {
                        return Task::batch(tasks);
                    }
                }
            }
            Task::none()
        }
        _ => Task::none(),
    }
}
