use crate::app::PdfBullApp;
use crate::commands::PdfCommand;
use crate::message::Message;
use crate::models::{AppTheme, DocumentTab, SearchResult};
use crate::storage;
use iced::{widget::image as iced_image, Task};
use std::sync::Arc;
use tokio::sync::mpsc;
use std::path::PathBuf;
use std::fs;

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
                    tasks.push(Task::perform(async move {
                        Message::SwitchTab(target_tab)
                    }, |m| m));
                    return Task::batch(tasks);
                }
            }
        }
    }

    match message {
        Message::ResetZoom => {
            if let Some(tab) = app.current_tab_mut() {
                tab.zoom = 1.0;
                tab.rendered_pages.clear();
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
            app.show_settings = false;
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
            if let Some(tab) = app.current_tab_mut() {
                if idx < tab.bookmarks.len() {
                    tab.current_page = tab.bookmarks[idx].page;
                }
            }
            Task::none()
        }
        Message::AddHighlight => {
            let accent_color = app.settings.accent_color.clone();
            if let Some(tab) = app.current_tab_mut() {
                let page = tab.current_page;
                let annotation = crate::models::Annotation {
                    id: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_millis() as u64)
                        .unwrap_or(0),
                    page,
                    style: crate::models::AnnotationStyle::Highlight { color: accent_color },
                    x: 100.0,
                    y: 100.0,
                    width: 200.0,
                    height: 50.0,
                };
                tab.annotations.push(annotation);
            }
            app.save_session();
            Task::none()
        }
        Message::AddRectangle => {
            if let Some(tab) = app.current_tab_mut() {
                let page = tab.current_page;
                let annotation = crate::models::Annotation {
                    id: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_millis() as u64)
                        .unwrap_or(0),
                    page,
                    style: crate::models::AnnotationStyle::Rectangle { color: "#ff0000".to_string(), thickness: 2.0, fill: false },
                    x: 150.0,
                    y: 150.0,
                    width: 150.0,
                    height: 100.0,
                };
                tab.annotations.push(annotation);
            }
            app.save_session();
            Task::none()
        }
        Message::DeleteAnnotation(idx) => {
            if let Some(tab) = app.current_tab_mut() {
                if idx < tab.annotations.len() {
                    tab.annotations.remove(idx);
                }
            }
            app.save_session();
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
                app.engine = Some(crate::engine::spawn_engine_thread());
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
                            let (resp_tx, mut resp_rx) = mpsc::channel(1);
                            let _ = cmd_tx.send(PdfCommand::Open(path_s, resp_tx)).await;
                            match resp_rx.recv().await {
                                Some(Ok(data)) => Some((path, data)),
                                _ => None,
                            }
                        } else {
                            None
                        }
                    },
                    |result| {
                        match result {
                            Some((path, data)) => Message::DocumentOpenedWithPath((path, data)),
                            None => Message::DocumentOpened(Err("Cancelled".into())),
                        }
                    }
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
            
            let doc_id = data.0;
            if let Some(engine) = &mut app.engine {
                engine.documents.insert(doc_id, path.to_string_lossy().to_string());
            }
            
            app.update(Message::DocumentOpened(Ok(data)))
        }
        Message::DocumentOpened(result) => {
            match result {
                Ok((doc_id, count, heights, width, outline)) => {
                    let default_zoom = app.settings.default_zoom;
                    let default_filter = app.settings.default_filter;
                    let pdf_path = if let Some(tab) = app.tabs.get(app.active_tab) {
                        Some(tab.path.to_string_lossy().to_string())
                    } else {
                        None
                    };
                    
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
                            let doc_id = doc_id;
                            return Task::perform(
                                async move {
                                    let (resp_tx, mut resp_rx) = mpsc::channel(1);
                                    let _ = cmd_tx.send(PdfCommand::LoadAnnotations(doc_id, path_str, resp_tx)).await;
                                    match resp_rx.recv().await {
                                        Some(Ok(annotations)) => (doc_id, annotations),
                                        Some(Err(_)) => (doc_id, Vec::new()),
                                        None => (doc_id, Vec::new()),
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
                    eprintln!("Error opening document: {}", e);
                    if !app.tabs.is_empty() {
                        app.tabs.pop();
                    }
                    Task::none()
                }
            }
        }
        Message::AnnotationsLoaded(doc_id, annotations) => {
            if let Some(tab) = app.tabs.iter_mut().find(|t| t.id == doc_id) {
                tab.annotations = annotations;
            }
            app.render_visible_pages()
        }
        Message::OpenFile(path) => {
            if app.engine.is_none() {
                return app.update(Message::OpenDocument);
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
                        let (resp_tx, mut resp_rx) = mpsc::channel(1);
                        let _ = cmd_tx.send(PdfCommand::Open(path_s, resp_tx)).await;
                        resp_rx.recv().await.unwrap_or(Err("Engine died".into()))
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
                let _ = cmd_tx.try_send(PdfCommand::Close(doc_id));
            }
            
            if app.active_tab >= app.tabs.len() && !app.tabs.is_empty() {
                app.active_tab = app.tabs.len() - 1;
            }
            app.save_session();
            Task::none()
        }
        Message::SwitchTab(idx) => {
            if idx < app.tabs.len() && idx != app.active_tab {
                app.active_tab = idx;
                app.save_session();
            }
            Task::none()
        }
        Message::NextPage => {
            if let Some(tab) = app.current_tab_mut() {
                if tab.current_page + 1 < tab.total_pages {
                    tab.current_page += 1;
                }
            }
            Task::none()
        }
        Message::PrevPage => {
            if let Some(tab) = app.current_tab_mut() {
                if tab.current_page > 0 {
                    tab.current_page -= 1;
                }
            }
            Task::none()
        }
        Message::ZoomIn => {
            if let Some(tab) = app.current_tab_mut() {
                tab.zoom = (tab.zoom * 1.25).min(5.0);
                tab.rendered_pages.clear();
            }
            app.render_visible_pages()
        }
        Message::ZoomOut => {
            if let Some(tab) = app.current_tab_mut() {
                tab.zoom = (tab.zoom / 1.25).max(0.25);
                tab.rendered_pages.clear();
            }
            app.render_visible_pages()
        }
        Message::SetZoom(zoom) => {
            if let Some(tab) = app.current_tab_mut() {
                tab.zoom = zoom.clamp(0.25, 5.0);
                tab.rendered_pages.clear();
            }
            app.render_visible_pages()
        }
        Message::JumpToPage(page) => {
            if let Some(tab) = app.current_tab_mut() {
                if page < tab.total_pages {
                    tab.current_page = page;
                }
            }
            Task::none()
        }
        Message::ViewportChanged(y, height) => {
            if let Some(tab) = app.current_tab_mut() {
                tab.viewport_y = y;
                tab.viewport_height = height;
                tab.cleanup_distant_pages();
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
            let (doc_id, zoom, rotation, filter, auto_crop) = {
                let tab = match app.current_tab() {
                    Some(t) => t,
                    None => return Task::none(),
                };
                
                if tab.rendered_pages.contains_key(&page_idx) {
                    return Task::none();
                }
                
                (tab.id, tab.zoom, tab.rotation, tab.render_filter, tab.auto_crop)
            };
            
            app.rendering_count += 1;
            
            let engine = match &app.engine {
                Some(e) => e,
                None => return Task::none(),
            };
            
            let cmd_tx = engine.cmd_tx.clone();
            Task::perform(
                async move {
                    let (resp_tx, mut resp_rx) = mpsc::channel(1);
                let _ = cmd_tx.send(PdfCommand::Render(doc_id, page_idx as i32, zoom, rotation, filter, auto_crop, resp_tx)).await;
                    resp_rx.recv().await.unwrap_or(Err("Channel closed".into()))
                },
                Message::PageRendered,
            )
        }
        Message::PageRendered(result) => {
            app.rendering_count = app.rendering_count.saturating_sub(1);
            match result {
                Ok((page, width, height, data)) => {
                    if let Some(tab) = app.current_tab_mut() {
                        let rgba_data = Arc::try_unwrap(data).unwrap_or_else(|arc| (*arc).clone());
                        tab.rendered_pages.insert(page, iced_image::Handle::from_rgba(width, height, rgba_data));
                    }
                }
                Err(e) => {
                    eprintln!("Render error: {}", e);
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
            app.search_pending = Some(query.clone());
            Task::perform(
                async move {
                    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
                    Message::PerformSearch
                },
                |m| m,
            )
        }
        Message::PerformSearch => {
            let query = match app.search_pending.take() {
                Some(q) => q,
                None => return Task::none(),
            };

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
                    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
                    let (resp_tx, mut resp_rx) = mpsc::channel(1);
                    let _ = cmd_tx.send(PdfCommand::Search(doc_id, query, resp_tx)).await;
                    let mut all_results = Vec::new();
                    while let Some(res) = resp_rx.recv().await {
                        match res {
                            Ok(mut batch) => all_results.append(&mut batch),
                            Err(e) => return Err(e),
                        }
                    }
                    Ok(all_results)
                },
                Message::SearchResult,
            )
        }
        Message::SearchResult(result) => {
            match result {
                Ok(results) => {
                    if let Some(tab) = app.current_tab_mut() {
                        for (page, text, y) in results {
                            tab.search_results.push(SearchResult { page, text, y_position: y });
                        }
                        
                        if !tab.search_results.is_empty() && tab.current_search_index == 0 {
                            tab.current_page = tab.search_results[0].page;
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Search error: {}", e);
                }
            }
            Task::none()
        }
        Message::NextSearchResult => {
            if let Some(tab) = app.current_tab_mut() {
                if !tab.search_results.is_empty() {
                    tab.current_search_index = (tab.current_search_index + 1) % tab.search_results.len();
                    tab.current_page = tab.search_results[tab.current_search_index].page;
                }
            }
            Task::none()
        }
        Message::PrevSearchResult => {
            if let Some(tab) = app.current_tab_mut() {
                if !tab.search_results.is_empty() {
                    tab.current_search_index = if tab.current_search_index == 0 {
                        tab.search_results.len() - 1
                    } else {
                        tab.current_search_index - 1
                    };
                    tab.current_page = tab.search_results[tab.current_search_index].page;
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
        Message::ExtractText => {
            let tab = match app.current_tab() {
                Some(t) => t,
                None => return Task::none(),
            };
            
            let page = tab.current_page as i32;
            let doc_id = tab.id;
            let path = tab.path.clone();
            
            let engine = match &app.engine {
                Some(e) => e,
                None => return Task::none(),
            };
            
            let cmd_tx = engine.cmd_tx.clone();
            Task::perform(
                async move {
                    let (resp_tx, mut resp_rx) = mpsc::channel(1);
                    let _ = cmd_tx.send(PdfCommand::ExtractText(doc_id, page, resp_tx)).await;
                    let result = resp_rx.recv().await.unwrap_or(Err("Extract failed".into()));
                    (path, result)
                },
                |(path, result)| {
                    let result_clone = result.clone();
                    if let Ok(text) = result {
                        let txt_path = path.with_extension("txt");
                        let _ = fs::write(&txt_path, &text);
                        eprintln!("Text extracted to: {}", txt_path.display());
                    }
                    Message::TextExtracted(result_clone)
                },
            )
        }
        Message::TextExtracted(result) => {
            if let Err(e) = result {
                eprintln!("Text extraction error: {}", e);
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
                            let (resp_tx, mut resp_rx) = mpsc::channel(1);
                            let _ = cmd_tx.send(PdfCommand::ExportImage(doc_id, page, zoom, path.clone(), resp_tx)).await;
                            match resp_rx.recv().await {
                                Some(Ok(())) => Ok(path),
                                Some(Err(e)) => Err(e),
                                None => Err("Engine died".into()),
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
                Err(e) => app.status_message = Some(format!("Export error: {}", e)),
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
                    let folder = rfd::AsyncFileDialog::new()
                        .pick_folder()
                        .await;
                    
                    match folder {
                        Some(f) => {
                            let path = f.path().to_string_lossy().to_string();
                            let pages: Vec<i32> = (0..total_pages as i32).collect();
                            let (resp_tx, mut resp_rx) = mpsc::channel(1);
                            let _ = cmd_tx.send(PdfCommand::ExportImages(doc_id, pages, zoom, path.clone(), resp_tx)).await;
                            match resp_rx.recv().await {
                                Some(Ok(paths)) => Ok(paths.join(", ")),
                                Some(Err(e)) => Err(e),
                                None => Err("Engine died".into()),
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
                Some(t) if !t.annotations.is_empty() => (t.id, t.annotations.clone(), t.path.to_string_lossy().to_string()),
                _ => {
                    eprintln!("No annotations to save");
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
                    let (resp_tx, mut resp_rx) = mpsc::channel(1);
                    let _ = cmd_tx.send(PdfCommand::ExportPdf(doc_id, pdf_path.clone(), annotations, resp_tx)).await;
                    match resp_rx.recv().await {
                        Some(Ok(path)) => Ok(format!("Annotations saved to {}", path)),
                        Some(Err(e)) => Err(e),
                        None => Err("Engine died".into()),
                    }
                },
                Message::AnnotationsSaved,
            )
        }
        Message::AnnotationsSaved(result) => {
            match result {
                Ok(msg) => app.status_message = Some(msg),
                Err(e) => app.status_message = Some(format!("Save error: {}", e)),
            }
            Task::none()
        }
        Message::EngineInitialized(state) => {
            app.engine = Some(state);
            Task::none()
        }
        Message::Error(e) => {
            app.status_message = Some(format!("Error: {}", e));
            eprintln!("Error: {}", e);
            Task::none()
        }
        Message::ClearStatus => {
            app.status_message = None;
            Task::none()
        }
    }
}
