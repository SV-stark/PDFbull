use crate::app::PdfBullApp;
use crate::message::Message;
use crate::models::DocumentTab;
use crate::storage;
use iced::Task;
use std::path::PathBuf;

pub fn handle_tab_message(app: &mut PdfBullApp, message: Message) -> Task<Message> {
    match message {
        Message::OpenDocument => {
            if app.engine.is_none() {
                let cache_size = app.settings.cache_size as u64;
                let max_mem = app.settings.max_cache_memory as u64;
                app.engine = Some(crate::engine::spawn_engine_thread(cache_size, max_mem));
            }

            if let Some(engine) = &app.engine {
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
                            let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();
                            let doc_id = crate::models::next_doc_id();
                            if let Err(e) = cmd_tx
                                .send(crate::commands::PdfCommand::Open(path_s, doc_id, resp_tx))
                            {
                                tracing::error!("Failed to send Open command: {e}");
                                return None;
                            }
                            match resp_rx.await {
                                Ok(Ok(data)) => Some((path, data)),
                                _ => None,
                            }
                        } else {
                            None
                        }
                    },
                    |result| match result {
                        Some((path, data)) => Message::DocumentOpenedWithPath((path, data)),
                        None => Message::DocumentOpened(Err(crate::models::PdfError::OpenFailed(
                            "Cancelled".into(),
                        ))),
                    },
                );
            }
            Task::none()
        }
        Message::DocumentOpenedWithPath((path, data)) => {
            if !path.as_path().exists() {
                return Task::none();
            }

            let tab = DocumentTab::new(path.clone());
            let tab_idx = app.tabs.len();
            app.tabs.push(tab);
            app.active_tab = tab_idx;
            app.sync_tab_display_names();
            app.add_recent_file(&path);

            app.update(Message::DocumentOpened(Ok(data)))
        }
        Message::DocumentOpened(result) => match result {
            Ok(res) => {
                let doc_id = res.id;
                let count = res.page_count;
                let heights = res.page_heights;
                let width = res.max_width;
                let outline = res.outline;
                let links = res.links;
                let signatures = res.signatures;

                let default_zoom = app.settings.default_zoom;
                let default_filter = app.settings.default_filter;
                let pdf_path = app
                    .tabs
                    .get(app.active_tab)
                    .map(|tab| tab.path.to_string_lossy().to_string());

                if let Some(tab) = app.tabs.get_mut(app.active_tab) {
                    tab.id = doc_id;
                    tab.total_pages = count;
                    tab.page_heights = heights;
                    tab.page_width = width;
                    tab.outline = outline;
                    tab.links = links;
                    tab.signatures = signatures;
                    tab.view_state.is_loading = false;
                    tab.zoom = default_zoom;
                    tab.render_filter = default_filter;
                }

                if let Some(path_str) = pdf_path
                    && let Some(engine) = &app.engine
                {
                    let cmd_tx = engine.cmd_tx.clone();
                    return Task::perform(
                        async move {
                            let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();
                            let _ = cmd_tx.send(crate::commands::PdfCommand::LoadAnnotations(
                                doc_id, path_str, resp_tx,
                            ));
                            match resp_rx.await {
                                Ok(Ok(annotations)) => (doc_id, annotations),
                                Ok(Err(_)) | Err(_) => (doc_id, Vec::new()),
                            }
                        },
                        |(doc_id, annotations)| Message::AnnotationsLoaded(doc_id, annotations),
                    );
                }
                app.save_session();
                app.render_visible_pages()
            }
            Err(e) => {
                if e == "Engine died" || e == "Channel closed" {
                    tracing::error!("Error opening document: {e}");
                    app.engine = None;
                    app.status_message = Some(
                        "PDF engine crashed. Please try your action again to restart it.".into(),
                    );
                } else if e.to_string().to_lowercase().contains("pdfium") {
                    tracing::error!("Error opening document: {e}");
                    app.engine = None;
                    app.status_message = Some("PDF engine missing (pdfium.dll). Please download it and place it next to the executable.".into());
                } else if e != "Cancelled" {
                    tracing::error!("Error opening document: {e}");
                    app.status_message = Some(format!("Error opening document: {e}"));
                }
                // "Cancelled" is a normal user action — no log, no status message.
                if !app.tabs.is_empty() {
                    app.tabs.pop();
                    app.sync_tab_display_names();
                }
                Task::none()
            }
        },
        Message::OpenFile(path) => {
            if app.engine.is_none() {
                let cache_size = app.settings.cache_size as u64;
                let max_mem = app.settings.max_cache_memory as u64;
                app.engine = Some(crate::engine::spawn_engine_thread(cache_size, max_mem));
            }

            let tab = DocumentTab::new(path.clone());
            let doc_id = tab.id;
            app.tabs.push(tab);
            app.active_tab = app.tabs.len() - 1;
            app.sync_tab_display_names();
            app.add_recent_file(&path);

            if let Some(engine) = &app.engine {
                let cmd_tx = engine.cmd_tx.clone();
                let path_s = path.to_string_lossy().to_string();
                return Task::perform(
                    async move {
                        let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();
                        if let Err(e) =
                            cmd_tx.send(crate::commands::PdfCommand::Open(path_s, doc_id, resp_tx))
                        {
                            tracing::error!("Failed to send Open command: {e}");
                            return Err(crate::models::PdfError::EngineDied);
                        }
                        resp_rx
                            .await
                            .unwrap_or(Err(crate::models::PdfError::EngineDied))
                    },
                    Message::DocumentOpened,
                );
            }
            Task::none()
        }
        Message::OpenRecentFile(file) => {
            let path = PathBuf::from(&file.path);
            if path.exists() {
                return app.update(Message::OpenFile(path));
            }
            app.recent_files.retain(|f| f.path != file.path);
            storage::save_recent_files(&app.recent_files);
            Task::none()
        }
        Message::CloseTab(idx) => {
            if idx >= app.tabs.len() {
                return Task::none();
            }

            let tab = app.tabs.remove(idx);
            if let Some(engine) = &app.engine {
                let cmd_tx = engine.cmd_tx.clone();
                let doc_id = tab.id;
                let _ = cmd_tx.send(crate::commands::PdfCommand::Close(doc_id));
            }

            if app.active_tab >= app.tabs.len() && !app.tabs.is_empty() {
                app.active_tab = app.tabs.len() - 1;
            }
            app.sync_tab_display_names();
            app.save_session();
            Task::none()
        }
        Message::SwitchTab(idx) => {
            if !app.tabs.is_empty() {
                let safe_idx = idx.min(app.tabs.len() - 1);
                if safe_idx != app.active_tab {
                    app.active_tab = safe_idx;
                    app.save_session();
                }
            }
            Task::none()
        }
        Message::TabReordered(new_order) => {
            let active_tab_id = app.tabs.get(app.active_tab).map(|t| t.id);
            let old_tabs = std::mem::take(&mut app.tabs);
            let mut temp_tabs: Vec<Option<crate::models::DocumentTab>> =
                old_tabs.into_iter().map(Some).collect();

            let mut reordered_tabs = Vec::with_capacity(temp_tabs.len());
            for idx in new_order {
                if idx < temp_tabs.len()
                    && let Some(tab) = temp_tabs[idx].take()
                {
                    reordered_tabs.push(tab);
                }
            }

            // Clean up any remaining
            for tab in temp_tabs.into_iter().flatten() {
                reordered_tabs.push(tab);
            }

            app.tabs = reordered_tabs;

            if let Some(id) = active_tab_id
                && let Some(new_idx) = app.tabs.iter().position(|t| t.id == id)
            {
                app.active_tab = new_idx;
            }

            app.sync_tab_display_names();
            app.save_session();
            Task::none()
        }

        Message::DocumentModifiedExternally(path) => {
            if app.tabs.iter().any(|t| t.path == path) {
                let path_clone = path.clone();
                let file_name = path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();

                return Task::perform(
                    async move {
                        let yes = rfd::AsyncMessageDialog::new()
                            .set_level(rfd::MessageLevel::Info)
                            .set_title("File Modified Externally")
                            .set_description(format!("The file '{file_name}' has been modified by another program.\n\nWould you like to reload it?"))
                            .set_buttons(rfd::MessageButtons::YesNo)
                            .show()
                            .await == rfd::MessageDialogResult::Yes;

                        if yes {
                            Some(Message::ReloadDocument(path_clone))
                        } else {
                            None
                        }
                    },
                    |m| m.unwrap_or(Message::ClearStatus),
                );
            }
            Task::none()
        }
        Message::ReloadDocument(path) => {
            if let Some(idx) = app.tabs.iter().position(|t| t.path == path) {
                let doc_id = app.tabs[idx].id;
                if let Some(engine) = &app.engine {
                    let cmd_tx = engine.cmd_tx.clone();
                    let _ = cmd_tx.send(crate::commands::PdfCommand::Close(doc_id));

                    let new_tab = DocumentTab::new(path.clone());
                    let new_doc_id = new_tab.id;
                    app.tabs[idx] = new_tab;
                    app.active_tab = idx;
                    app.sync_tab_display_names();

                    let path_s = path.to_string_lossy().to_string();
                    return Task::perform(
                        async move {
                            let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();
                            if let Err(e) = cmd_tx.send(crate::commands::PdfCommand::Open(
                                path_s, new_doc_id, resp_tx,
                            )) {
                                tracing::error!("Failed to send Open command: {e}");
                                return Err(crate::models::PdfError::EngineDied);
                            }
                            resp_rx
                                .await
                                .unwrap_or(Err(crate::models::PdfError::EngineDied))
                        },
                        Message::DocumentOpened,
                    );
                }
            }
            Task::none()
        }
        _ => Task::none(),
    }
}
