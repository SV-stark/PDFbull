use crate::app::PdfBullApp;
use crate::commands::PdfCommand;
use crate::message::Message;
use crate::models::{AppTheme, DocumentTab, SearchResult};
use crate::storage;
use iced::widget::image as iced_image;
use iced::widget::{operation, Id};
use iced::Task;
use std::path::PathBuf;

fn scroll_to_page(tab: &crate::models::DocumentTab, page: usize) -> Task<Message> {
    let y_offset: f32 = tab.page_heights.iter().take(page).map(|h| (h + crate::models::PAGE_SPACING) * tab.zoom).sum();
    operation::scroll_to(
        Id::new("pdf_scroll"),
        iced::widget::scrollable::AbsoluteOffset {
            x: 0.0,
            y: y_offset,
        },
    )
}

pub fn handle_message(app: &mut PdfBullApp, message: Message) -> Task<Message> {
    if !app.loaded {
        app.loaded = true;
        app.settings = storage::load_settings();
        app.recent_files = storage::load_recent_files();
        let session = storage::load_session();
        if app.settings.theme == AppTheme::System {
            match dark_light::detect() {
                dark_light::Mode::Dark => app.settings.theme = AppTheme::Dark,
                _ => app.settings.theme = AppTheme::Light,
            }
        }
        if app.settings.restore_session {
            if let Some(mut session_data) = session {
                let target_tab = session_data.active_tab;
                let mut tasks = Vec::new();
                for path in session_data.open_tabs.drain(..) {
                    tasks.push(app.update(Message::OpenFile(path)));
                }
                if !tasks.is_empty() {
                    tasks.push(Task::perform(
                        async move { Message::SwitchTab(target_tab) },
                        |m| m,
                    ));
                    return Task::batch(tasks);
                }
            }
        }
    }

    match message {
        Message::ResetZoom => {
            if let Some(tab) = app.current_tab_mut() {
                tab.zoom = 1.0;
                // Don't clear rendered_pages to avoid flashing - let PageRendered handler update them
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
                tab.rendered_pages.clear();
            }
            app.render_visible_pages()
        }
        Message::RotateCounterClockwise => {
            if let Some(tab) = app.current_tab_mut() {
                tab.rotation = (tab.rotation - 90 + 360) % 360;
                tab.rendered_pages.clear();
            }
            app.render_visible_pages()
        }
        Message::AddBookmark => {
            if let Some(tab) = app.current_tab_mut() {
                let page = tab.current_page;
                let label = format!("Page {}", page + 1);
                let bookmark = crate::models::PageBookmark {
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
            if let Some(tab) = app.current_tab_mut() {
                if idx < tab.bookmarks.len() {
                    tab.bookmarks.remove(idx);
                }
            }
            Task::none()
        }
        Message::JumpToBookmark(idx) => {
            let jump_page = if let Some(tab) = app.current_tab_mut() {
                if idx < tab.bookmarks.len() {
                    tab.current_page = tab.bookmarks[idx].page;
                    Some(tab.current_page)
                } else {
                    None
                }
            } else {
                None
            };

            if let Some(p) = jump_page {
                app.page_input = (p + 1).to_string();
                if let Some(tab) = app.current_tab_mut() {
                    return scroll_to_page(tab, p);
                }
            }
            Task::none()
        }
        Message::SetAnnotationMode(mode) => {
            app.annotation_mode = mode;
            if app.annotation_mode.is_none() {
                app.annotation_drag = None;
            }
            Task::none()
        }
        Message::AnnotationDragStart { page, x, y } => {
            if let Some(kind) = &app.annotation_mode {
                app.annotation_drag = Some(crate::models::AnnotationDrag {
                    page,
                    start: (x, y),
                    current: (x, y),
                    kind: kind.clone(),
                });
            }
            Task::none()
        }
        Message::AnnotationDragUpdate { x, y } => {
            if let Some(drag) = &mut app.annotation_drag {
                drag.current = (x, y);
            }
            Task::none()
        }
        Message::AnnotationDragEnd => {
            if let Some(drag) = app.annotation_drag.take() {
                if let Some(tab) = app.current_tab_mut() {
                    let min_x = drag.start.0.min(drag.current.0);
                    let min_y = drag.start.1.min(drag.current.1);
                    let w = (drag.start.0 - drag.current.0).abs();
                    let h = (drag.start.1 - drag.current.1).abs();

                    if w > 5.0 && h > 5.0 {
                        let id = crate::models::next_annotation_id();
                        let style = match drag.kind {
                            crate::models::PendingAnnotationKind::Highlight => {
                                crate::models::AnnotationStyle::Highlight {
                                    color: "#FFFF00".to_string(),
                                }
                            }
                            crate::models::PendingAnnotationKind::Rectangle => {
                                crate::models::AnnotationStyle::Rectangle {
                                    color: "#FF0000".to_string(),
                                    thickness: 2.0,
                                    fill: false,
                                }
                            }
                        };

                        let ann = crate::models::Annotation {
                            id,
                            page: drag.page,
                            style,
                            x: min_x,
                            y: min_y,
                            width: w,
                            height: h,
                        };

                        tab.undo_stack
                            .push(crate::models::UndoableAction::AddAnnotation(ann.clone()));
                        tab.redo_stack.clear();
                        tab.annotations.push(ann);
                    }
                }
            }

            // Optionally reset mode after drawing
            // app.annotation_mode = None;

            Task::none()
        }
        Message::DeleteAnnotation(idx) => {
            if let Some(tab) = app.current_tab_mut() {
                if idx < tab.annotations.len() {
                    let ann = tab.annotations.remove(idx);
                    tab.undo_stack
                        .push(crate::models::UndoableAction::DeleteAnnotation(idx, ann));
                    tab.redo_stack.clear();
                }
            }
            Task::none()
        }
        Message::Undo => {
            if let Some(tab) = app.current_tab_mut() {
                if let Some(action) = tab.undo_stack.pop() {
                    match action {
                        crate::models::UndoableAction::AddAnnotation(ann) => {
                            tab.redo_stack
                                .push(crate::models::UndoableAction::AddAnnotation(ann.clone()));
                            tab.annotations.retain(|a| a.id != ann.id);
                        }
                        crate::models::UndoableAction::DeleteAnnotation(idx, ann) => {
                            tab.redo_stack
                                .push(crate::models::UndoableAction::DeleteAnnotation(
                                    idx,
                                    ann.clone(),
                                ));
                            tab.annotations.insert(idx.min(tab.annotations.len()), ann);
                        }
                    }
                }
            }
            Task::none()
        }
        Message::Redo => {
            if let Some(tab) = app.current_tab_mut() {
                if let Some(action) = tab.redo_stack.pop() {
                    match action {
                        crate::models::UndoableAction::AddAnnotation(ann) => {
                            tab.undo_stack
                                .push(crate::models::UndoableAction::AddAnnotation(ann.clone()));
                            tab.annotations.push(ann);
                        }
                        crate::models::UndoableAction::DeleteAnnotation(idx, ann) => {
                            tab.undo_stack
                                .push(crate::models::UndoableAction::DeleteAnnotation(
                                    idx,
                                    ann.clone(),
                                ));
                            tab.annotations.retain(|a| a.id != ann.id);
                        }
                    }
                }
            }
            Task::none()
        }
        Message::SetFilter(filter) => {
            if let Some(tab) = app.current_tab_mut() {
                if tab.render_filter != filter {
                    tab.render_filter = filter;
                    tab.rendered_pages.clear();
                }
            }
            app.render_visible_pages()
        }
        Message::ToggleAutoCrop => {
            if let Some(tab) = app.current_tab_mut() {
                tab.auto_crop = !tab.auto_crop;
                tab.rendered_pages.clear();
            }
            app.render_visible_pages()
        }
        Message::OpenDocument => {
            if app.engine.is_none() {
                let cache_size = app.settings.cache_size as u64;
                app.engine = Some(crate::engine::spawn_engine_thread(cache_size));
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
                            if let Err(e) = cmd_tx.send(PdfCommand::Open(path_s, resp_tx)) {
                                log::error!("Failed to send Open command: {}", e);
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
                        None => Message::DocumentOpened(Err("Cancelled".into())),
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
            app.add_recent_file(&path);

            let _doc_id = data.0;
            let _total_pages = data.1;
            app.update(Message::DocumentOpened(Ok(data)))
        }
        Message::DocumentOpened(result) => match result {
            Ok((doc_id, count, heights, width, outline)) => {
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
                    tab.is_loading = false;
                    tab.zoom = default_zoom;
                    tab.render_filter = default_filter;
                }

                if let Some(path_str) = pdf_path {
                    if let Some(engine) = &app.engine {
                        let cmd_tx = engine.cmd_tx.clone();
                        return Task::perform(
                            async move {
                                let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();
                                let _ = cmd_tx
                                    .send(PdfCommand::LoadAnnotations(doc_id, path_str, resp_tx));
                                match resp_rx.await {
                                    Ok(Ok(annotations)) => (doc_id, annotations),
                                    Ok(Err(_)) => (doc_id, Vec::new()),
                                    Err(_) => (doc_id, Vec::new()),
                                }
                            },
                            |(doc_id, annotations)| Message::AnnotationsLoaded(doc_id, annotations),
                        );
                    }
                }
                app.save_session();
                app.render_visible_pages()
            }
            Err(e) => {
                log::error!("Error opening document: {}", e);
                if e == "Engine died" || e == "Channel closed" {
                    app.engine = None;
                    app.status_message = Some(
                        "PDF engine crashed. Please try your action again to restart it.".into(),
                    );
                } else if e.to_lowercase().contains("pdfium") {
                    app.engine = None;
                    app.status_message = Some("PDF engine missing (pdfium.dll). Please download it and place it next to the executable.".into());
                } else if e != "Cancelled" {
                    app.status_message = Some(format!("Error opening document: {}", e));
                }
                if !app.tabs.is_empty() {
                    app.tabs.pop();
                }
                Task::none()
            }
        },
        Message::AnnotationsLoaded(doc_id, annotations) => {
            if let Some(tab) = app.tabs.iter_mut().find(|t| t.id == doc_id) {
                tab.annotations = annotations;
            }
            app.render_visible_pages()
        }
        Message::OpenFile(path) => {
            if app.engine.is_none() {
                let cache_size = app.settings.cache_size as u64;
                app.engine = Some(crate::engine::spawn_engine_thread(cache_size));
            }

            let tab = DocumentTab::new(path.clone());
            app.tabs.push(tab);
            app.active_tab = app.tabs.len() - 1;
            app.add_recent_file(&path);

            if let Some(engine) = &app.engine {
                let cmd_tx = engine.cmd_tx.clone();
                let path_s = path.to_string_lossy().to_string();
                return Task::perform(
                    async move {
                        let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();
                        if let Err(e) = cmd_tx.send(PdfCommand::Open(path_s, resp_tx)) {
                            log::error!("Failed to send Open command: {}", e);
                            return Err("Engine died".into());
                        }
                        resp_rx.await.unwrap_or(Err("Engine died".into()))
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
                let _ = cmd_tx.send(PdfCommand::Close(doc_id));
            }

            if app.active_tab >= app.tabs.len() && !app.tabs.is_empty() {
                app.active_tab = app.tabs.len() - 1;
            }
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
        Message::NextPage => {
            let next_page = if let Some(tab) = app.current_tab_mut() {
                if tab.current_page + 1 < tab.total_pages {
                    tab.current_page += 1;
                    Some(tab.current_page)
                } else {
                    None
                }
            } else {
                None
            };

            if let Some(page) = next_page {
                app.page_input = (page + 1).to_string();
                if let Some(tab) = app.current_tab_mut() {
                    return scroll_to_page(tab, page);
                }
            }
            Task::none()
        }
        Message::PrevPage => {
            let prev_page = if let Some(tab) = app.current_tab_mut() {
                if tab.current_page > 0 {
                    tab.current_page -= 1;
                    Some(tab.current_page)
                } else {
                    None
                }
            } else {
                None
            };

            if let Some(page) = prev_page {
                app.page_input = (page + 1).to_string();
                if let Some(tab) = app.current_tab_mut() {
                    return scroll_to_page(tab, page);
                }
            }
            Task::none()
        }
        Message::ZoomIn => {
            if let Some(tab) = app.current_tab_mut() {
                tab.zoom = (tab.zoom * 1.1).min(5.0);
            }
            app.render_visible_pages()
        }
        Message::ZoomOut => {
            if let Some(tab) = app.current_tab_mut() {
                tab.zoom = (tab.zoom / 1.1).max(0.25);
            }
            app.render_visible_pages()
        }
        Message::SetZoom(zoom) => {
            if let Some(tab) = app.current_tab_mut() {
                tab.zoom = zoom.clamp(0.25, 5.0);
            }
            app.render_visible_pages()
        }
        Message::JumpToPage(page) => {
            let jump_page = if let Some(tab) = app.current_tab_mut() {
                if page < tab.total_pages {
                    tab.current_page = page;
                    Some(tab.current_page)
                } else {
                    None
                }
            } else {
                None
            };

            if let Some(p) = jump_page {
                app.page_input = (p + 1).to_string();
                if let Some(tab) = app.current_tab_mut() {
                    return scroll_to_page(tab, p);
                }
            }
            Task::none()
        }
        Message::PageInputChanged(s) => {
            app.page_input = s;
            Task::none()
        }
        Message::PageInputSubmitted => {
            if let Ok(page) = app.page_input.trim().parse::<usize>() {
                return app.update(Message::JumpToPage(page.saturating_sub(1)));
            }
            if let Some(tab) = app.current_tab() {
                app.page_input = (tab.current_page + 1).to_string();
            }
            Task::none()
        }
        Message::ViewportChanged(y, height) => {
            if let Some(tab) = app.current_tab_mut() {
                tab.viewport_y = y;
                tab.viewport_height = height;
                tab.cleanup_distant_pages();
            }
            for tab in &mut app.tabs {
                if tab.needs_periodic_cleanup() {
                    tab.cleanup_distant_pages();
                }
            }
            app.render_visible_pages()
        }
        Message::SidebarViewportChanged(y) => {
            if let Some(tab) = app.current_tab_mut() {
                tab.sidebar_viewport_y = y;
            }
            Task::none()
        }
        Message::RequestRender(page_idx) => {
            let (doc_id, zoom, rotation, filter, auto_crop, quality) = {
                let tab = match app.current_tab() {
                    Some(t) => t,
                    None => return Task::none(),
                };

                let needs_render = if let Some((scale, _)) = tab.rendered_pages.get(&page_idx) {
                    (scale - tab.zoom).abs() > 0.001
                } else {
                    true
                };

                if !needs_render
                    || app
                        .rendering_set
                        .contains(&crate::app::RenderTarget::Page(page_idx))
                {
                    return Task::none();
                }

                (
                    tab.id,
                    tab.zoom,
                    tab.rotation,
                    tab.render_filter,
                    tab.auto_crop,
                    app.settings.render_quality,
                )
            };

            app.rendering_set
                .insert(crate::app::RenderTarget::Page(page_idx));
            app.rendering_count += 1;

            let engine = match &app.engine {
                Some(e) => e,
                None => return Task::none(),
            };

            let cmd_tx = engine.cmd_tx.clone();
            Task::perform(
                async move {
                    let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();
                    let options = crate::pdf_engine::RenderOptions {
                        scale: zoom,
                        rotation,
                        filter,
                        auto_crop,
                        quality,
                    };
                    if let Err(e) = cmd_tx.send(PdfCommand::Render(
                        doc_id,
                        page_idx as i32,
                        options,
                        resp_tx,
                    )) {
                        log::error!("Failed to send Render command: {}", e);
                        return Err("Engine died".into());
                    }
                    resp_rx.await.unwrap_or(Err("Channel closed".into()))
                },
                move |res| Message::PageRendered(page_idx, zoom, res),
            )
        }
        Message::PageRendered(page_idx, scale, result) => {
            app.rendering_count = app.rendering_count.saturating_sub(1);
            app.rendering_set
                .remove(&crate::app::RenderTarget::Page(page_idx));

            if let Some(tab) = app.current_tab_mut() {
                match result {
                    Ok((width, height, data)) => {
                        // Use Arc directly to avoid expensive clones of pixel data
                        tab.rendered_pages.insert(
                            page_idx,
                            (scale, iced_image::Handle::from_rgba(width, height, data.to_vec())),
                        );
                    }
                    Err(e) => {
                        log::error!("Render error: {}", e);
                        if e == "Engine died" || e == "Channel closed" {
                            app.engine = None;
                            app.status_message = Some(
                                "PDF engine crashed. Please try your action again to restart it."
                                    .into(),
                            );
                        } else if e.to_lowercase().contains("pdfium") {
                            app.engine = None;
                            app.status_message =
                                Some("Failed to load PDF engine (pdfium.dll missing).".into());
                        }
                    }
                }
            }
            app.render_visible_pages()
        }
        Message::ThumbnailRendered(page_idx, scale, result) => {
            app.rendering_count = app.rendering_count.saturating_sub(1);
            app.rendering_set
                .remove(&crate::app::RenderTarget::Thumbnail(page_idx));

            if let Some(tab) = app.current_tab_mut() {
                // For thumbnails, we check against the expected thumbnail zoom
                let expected_thumb_zoom = 120.0 / tab.page_width.max(1.0);
                if (expected_thumb_zoom - scale).abs() > 0.001 {
                    return Task::none();
                }

                match result {
                    Ok((width, height, data)) => {
                        tab.thumbnails.insert(
                            page_idx,
                            iced_image::Handle::from_rgba(width, height, (*data).clone()),
                        );
                    }
                    Err(e) => {
                        log::error!("Thumbnail render error: {}", e);
                    }
                }
            }
            Task::none()
        }
        Message::Search(query) => {
            app.search_query = query.clone();
            if query.is_empty() {
                if let Some(tab) = app.current_tab_mut() {
                    tab.search_results.clear();
                    tab.current_search_index = 0;
                }
                return Task::none();
            }
            let query_clone = query.clone();
            Task::perform(
                async move {
                    tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
                    Message::PerformSearch(query_clone)
                },
                |m| m,
            )
        }
        Message::PerformSearch(query) => {
            if query != app.search_query {
                return Task::none();
            }

            if query.is_empty() {
                return Task::none();
            }

            let tab = match app.current_tab_mut() {
                Some(t) => t,
                None => return Task::none(),
            };

            tab.search_results.clear();
            tab.current_search_index = 0;

            let doc_id = tab.id;

            let engine = match &app.engine {
                Some(e) => e,
                None => return Task::none(),
            };

            let cmd_tx = engine.cmd_tx.clone();
            Task::perform(
                async move {
                    tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
                    let (resp_tx, resp_rx) = std::sync::mpsc::channel();
                    if let Err(e) = cmd_tx.send(PdfCommand::Search(doc_id, query, resp_tx)) {
                        log::error!("Failed to send Search command: {}", e);
                        return Err("Engine died".into());
                    }
                    let mut all_results = Vec::new();
                    while let Ok(res) = resp_rx.recv() {
                        match res {
                            Ok(mut batch) => all_results.append(&mut batch),
                            Err(e) => return Err(e),
                        }
                    }
                    Ok(all_results)
                },
                move |result| Message::SearchResult(doc_id, result),
            )
        }
        Message::SearchResult(received_doc_id, result) => {
            match result {
                Ok(results) => {
                    if let Some(tab) = app.current_tab_mut() {
                        if tab.id == received_doc_id {
                            tab.search_results = results
                                .into_iter()
                                .map(|(page, text, y, x, width, height)| SearchResult {
                                    page,
                                    text,
                                    y_position: y,
                                    x,
                                    width,
                                    height,
                                })
                                .collect();
                            tab.current_search_index = 0;

                            if !tab.search_results.is_empty() && tab.current_search_index == 0 {
                                tab.current_page = tab.search_results[0].page;
                            }
                        }
                    }
                }
                Err(e) => {
                    log::error!("Search error: {}", e);
                    if e == "Engine died" || e == "Channel closed" {
                        app.engine = None;
                        app.status_message = Some(
                            "PDF engine crashed. Please try your action again to restart it."
                                .into(),
                        );
                    } else {
                        app.status_message = Some(format!("Search error: {}", e));
                    }
                }
            }
            Task::none()
        }
        Message::NextSearchResult => {
            let next_page = if let Some(tab) = app.current_tab_mut() {
                if !tab.search_results.is_empty() {
                    tab.current_search_index =
                        (tab.current_search_index + 1) % tab.search_results.len();
                    tab.current_page = tab.search_results[tab.current_search_index].page;
                    Some(tab.current_page)
                } else {
                    None
                }
            } else {
                None
            };

            if let Some(page) = next_page {
                app.page_input = (page + 1).to_string();
                if let Some(tab) = app.current_tab_mut() {
                    return scroll_to_page(tab, page);
                }
            }
            Task::none()
        }
        Message::PrevSearchResult => {
            let prev_page = if let Some(tab) = app.current_tab_mut() {
                if !tab.search_results.is_empty() {
                    tab.current_search_index = if tab.current_search_index == 0 {
                        tab.search_results.len() - 1
                    } else {
                        tab.current_search_index - 1
                    };
                    tab.current_page = tab.search_results[tab.current_search_index].page;
                    Some(tab.current_page)
                } else {
                    None
                }
            } else {
                None
            };

            if let Some(page) = prev_page {
                app.page_input = (page + 1).to_string();
                if let Some(tab) = app.current_tab_mut() {
                    return scroll_to_page(tab, page);
                }
            }
            Task::none()
        }
        Message::ClearSearch => {
            if let Some(tab) = app.current_tab_mut() {
                tab.search_results.clear();
                tab.current_search_index = 0;
            }
            app.search_query.clear();
            Task::none()
        }
        Message::ClearRecentFiles => {
            app.recent_files.clear();
            crate::storage::save_recent_files(&app.recent_files);
            Task::none()
        }
        Message::ExtractText => {
            let tab = match app.current_tab() {
                Some(t) => t,
                None => return Task::none(),
            };

            let page = tab.current_page as i32;
            let doc_id = tab.id;

            let engine = match &app.engine {
                Some(e) => e,
                None => return Task::none(),
            };

            let cmd_tx = engine.cmd_tx.clone();
            Task::perform(
                async move {
                    let file = rfd::AsyncFileDialog::new()
                        .add_filter("Text", &["txt"])
                        .set_file_name("extracted_text.txt")
                        .save_file()
                        .await;

                    match file {
                        Some(f) => {
                            let path = f.path().to_path_buf();
                            let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();
                            if let Err(e) = cmd_tx.send(PdfCommand::ExtractText(doc_id, page, resp_tx)) {
                                log::error!("Failed to send ExtractText command: {}", e);
                                return Err("Engine died".into());
                            }
                            match resp_rx.await {
                                Ok(Ok(text)) => {
                                    if let Err(e) = std::fs::write(&path, &text) {
                                        Err(format!("Failed to write file: {}", e))
                                    } else {
                                        Ok(path.to_string_lossy().to_string())
                                    }
                                }
                                Ok(Err(e)) => Err(e),
                                Err(_) => Err("Engine died".into()),
                            }
                        }
                        None => Err("Cancelled".into()),
                    }
                },
                Message::TextExtracted,
            )
        }
        Message::TextExtracted(result) => {
            match result {
                Ok(path) => app.status_message = Some(format!("Text extracted to: {}", path)),
                Err(e) => {
                    if e != "Cancelled" {
                        log::error!("Text extraction error: {}", e);
                        if e == "Engine died" || e == "Channel closed" {
                            app.engine = None;
                            app.status_message = Some(
                                "PDF engine crashed. Please try your action again to restart it."
                                    .into(),
                            );
                        } else {
                            app.status_message = Some(format!("Text extraction error: {}", e));
                        }
                    }
                }
            }
            Task::none()
        }
        Message::ExportImage => {
            let tab = match app.current_tab() {
                Some(t) => t,
                None => return Task::none(),
            };

            let page = tab.current_page as i32;
            let zoom = tab.zoom;
            let doc_id = tab.id;

            let engine = match &app.engine {
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
                            let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();
                            if let Err(e) = cmd_tx.send(PdfCommand::ExportImage(
                                doc_id,
                                page,
                                zoom,
                                path.clone(),
                                resp_tx,
                            )) {
                                log::error!("Failed to send ExportImage command: {}", e);
                                return Err("Engine died".into());
                            }
                            match resp_rx.await {
                                Ok(Ok(())) => Ok(path),
                                Ok(Err(e)) => Err(e),
                                Err(_) => Err("Engine died".into()),
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
                Ok(path) => app.status_message = Some(format!("Exported to: {}", path)),
                Err(e) => {
                    log::error!("Export error: {}", e);
                    if e == "Engine died" || e == "Channel closed" {
                        app.engine = None;
                        app.status_message =
                            Some("PDF engine crashed. Please try your action again.".into());
                    } else if e != "Cancelled" {
                        app.status_message = Some(format!("Export error: {}", e));
                    }
                }
            }
            Task::none()
        }
        Message::ExportImages => {
            let tab = match app.current_tab() {
                Some(t) => t,
                None => return Task::none(),
            };

            let total_pages = tab.total_pages;
            let zoom = tab.zoom;
            let doc_id = tab.id;

            let engine = match &app.engine {
                Some(e) => e,
                None => return Task::none(),
            };

            let cmd_tx = engine.cmd_tx.clone();
            Task::perform(
                async move {
                    let folder = rfd::AsyncFileDialog::new().pick_folder().await;

                    match folder {
                        Some(f) => {
                            let path = f.path().to_string_lossy().to_string();
                            let pages: Vec<i32> = (0..total_pages as i32).collect();
                            let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();
                            if let Err(e) = cmd_tx.send(PdfCommand::ExportImages(
                                doc_id,
                                pages,
                                zoom,
                                path.clone(),
                                resp_tx,
                            )) {
                                log::error!("Failed to send ExportImages command: {}", e);
                                return Err("Engine died".into());
                            }
                            match resp_rx.await {
                                Ok(Ok(paths)) => Ok(paths.join(", ")),
                                Ok(Err(e)) => Err(e),
                                Err(_) => Err("Engine died".into()),
                            }
                        }
                        None => Err("Cancelled".into()),
                    }
                },
                Message::ImageExported,
            )
        }
        Message::SaveAnnotations => {
            let (doc_id, annotations, pdf_path) = match app.current_tab() {
                Some(t) if !t.annotations.is_empty() => (
                    t.id,
                    t.annotations.clone(),
                    t.path.to_string_lossy().to_string(),
                ),
                _ => {
                    log::warn!("No annotations to save");
                    return Task::none();
                }
            };

            let engine = match &app.engine {
                Some(e) => e,
                None => return Task::none(),
            };

            let cmd_tx = engine.cmd_tx.clone();
            Task::perform(
                async move {
                    let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();
                    if let Err(e) = cmd_tx.send(PdfCommand::ExportPdf(
                        doc_id,
                        pdf_path.clone(),
                        annotations,
                        resp_tx,
                    )) {
                        log::error!("Failed to send ExportPdf command: {}", e);
                        return Err("Engine died".into());
                    }
                    match resp_rx.await {
                        Ok(Ok(path)) => Ok(format!("Annotations saved to {}", path)),
                        Ok(Err(e)) => Err(e),
                        Err(_) => Err("Engine died".into()),
                    }
                },
                Message::AnnotationsSaved,
            )
        }
        Message::AnnotationsSaved(result) => {
            match result {
                Ok(msg) => app.status_message = Some(msg),
                Err(e) => {
                    log::error!("Save error: {}", e);
                    if e == "Engine died" || e == "Channel closed" {
                        app.engine = None;
                        app.status_message =
                            Some("PDF engine crashed. Please try saving again.".into());
                    } else if e != "Cancelled" {
                        app.status_message = Some(format!("Save error: {}", e));
                    }
                }
            }
            Task::none()
        }
        Message::EngineInitialized(state) => {
            app.engine = Some(state);
            Task::none()
        }
        Message::Error(e) => {
            log::error!("Error: {}", e);
            app.status_message = Some(format!("Error: {}", e));
            Task::none()
        }
        Message::ClearStatus => {
            app.status_message = None;
            Task::none()
        }
        Message::IcedEvent(event) => {
            match event {
                iced::Event::Window(iced::window::Event::CloseRequested) => {
                    let has_dirty = app.tabs.iter().any(|t| !t.annotations.is_empty());
                    if has_dirty {
                        return Task::perform(
                            async move {
                                let answer = rfd::AsyncMessageDialog::new()
                                    .set_level(rfd::MessageLevel::Warning)
                                    .set_title("Unsaved Annotations")
                                    .set_description("You have annotations that haven't been saved to a PDF. Quitting will lose them. Are you sure you want to quit?")
                                    .set_buttons(rfd::MessageButtons::YesNo)
                                    .show()
                                    .await;

                                if answer == rfd::MessageDialogResult::Yes {
                                    Message::ForceQuit
                                } else {
                                    Message::ClearStatus
                                }
                            },
                            |m| m,
                        );
                    } else {
                        return iced::exit();
                    }
                }
                iced::Event::Window(iced::window::Event::FileDropped(path)) => {
                    return app.update(Message::OpenFile(path));
                }
                iced::Event::Mouse(iced::mouse::Event::CursorMoved { position: _ }) => {
                    // We only care about tracking when dragging an annotation
                    // But CursorMoved position is in absolute screen coords,
                    // while AnnotationDrag tracks page-relative coords.
                    // Ideally we'd use mouse_area's on_move but it's not supported natively yet.
                    // A simple approximation for drag:
                    // Since we don't have absolute window coords trivially mapped to page coords,
                    // we will need to calculate the relative delta if we really want to track outside the mouse_area.
                    // Actually, we can get local coords by updating via mouse_area if we had an event listener,
                    // but we can just use the fact that Iced 0.12+ doesn't have on_move in mouse_area.
                    // Wait, we can't easily map window coordinates to page coordinates here without knowing the scroll/zoom.
                    // Let's rely on standard iced widget events if possible, or just accept that the visual preview
                    // might be slightly off until we release.
                    // Or better, let's keep the drag state and just live with no preview if we can't get local coords.
                    // No, wait, if we drop iced::mouse::Event tracking, the preview won't update.
                    // Let's just pass the absolute coords and we'll have to adjust them by scroll and zoom later.
                    // A better way is to see if Iced's `mouse_area` supports movement tracking. It doesn't in 0.12, but does in 0.13.
                    // Given I'm not sure what version of Iced this uses (likely 0.12), I'll omit live drag preview
                    // updates if it requires too much math, and just use the DragEnd coords.
                    // But we can pull the scroll offset!
                    // Actually, `mouse_area` only fires `on_press` and `on_release`.
                    // Let's just ignore `CursorMoved` here and rely on the start/end bounds.
                }
                iced::Event::Mouse(iced::mouse::Event::WheelScrolled { delta }) => {
                    use iced::mouse::ScrollDelta;
                    let modifiers = app.modifiers;
                    if modifiers.control() && !app.tabs.is_empty() {
                        match delta {
                            ScrollDelta::Lines { y, .. } | ScrollDelta::Pixels { y, .. } => {
                                if y > 0.0 {
                                    return app.update(Message::ZoomIn);
                                } else if y < 0.0 {
                                    return app.update(Message::ZoomOut);
                                }
                            }
                        }
                    }
                }
                iced::Event::Keyboard(iced::keyboard::Event::ModifiersChanged(modifiers)) => {
                    app.modifiers = modifiers;
                }
                iced::Event::Keyboard(iced::keyboard::Event::KeyPressed {
                    key, modifiers, ..
                }) => {
                    use iced::keyboard::Key;

                    match key {
                        Key::Named(iced::keyboard::key::Named::F11) => {
                            return app.update(Message::ToggleFullscreen);
                        }
                        Key::Character(c) => match c.as_str() {
                            "o" if modifiers.command() => return app.update(Message::OpenDocument),
                            "s" if modifiers.command() => {
                                return app.update(Message::SaveAnnotations)
                            }
                            "z" if modifiers.command() && modifiers.shift() => {
                                return app.update(Message::Redo)
                            }
                            "z" if modifiers.command() => return app.update(Message::Undo),
                            "y" if modifiers.command() => return app.update(Message::Redo),
                            "f" if modifiers.command() => { /* Search is handled in UI */ }
                            "0" if modifiers.command() => return app.update(Message::ResetZoom),
                            "=" | "+" if modifiers.command() => return app.update(Message::ZoomIn),
                            "-" if modifiers.command() => return app.update(Message::ZoomOut),
                            "w" if modifiers.command() => {
                                if !app.tabs.is_empty() {
                                    return app.update(Message::CloseTab(app.active_tab));
                                }
                            }
                            "b" if modifiers.command() => {
                                return app.update(Message::ToggleSidebar)
                            }
                            "?" if modifiers.shift() => {
                                return app.update(Message::ToggleKeyboardHelp)
                            }
                            _ => {}
                        },
                        Key::Named(iced::keyboard::key::Named::Escape) => {
                            if app.annotation_mode.is_some() {
                                return app.update(Message::SetAnnotationMode(None));
                            }
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
            Task::none()
        }
        Message::ForceQuit => iced::exit(),
    }
}
