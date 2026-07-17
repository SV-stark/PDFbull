use crate::app::PdfBullApp;
use crate::message::Message;
use crate::models::PdfError;
use iced::Task;
use iced::widget::image as iced_image;

pub fn handle_render_message(app: &mut PdfBullApp, message: Message) -> Task<Message> {
    match message {
        Message::SetFilter(filter) => {
            if let Some(tab) = app.current_tab_mut()
                && tab.render_filter != filter
            {
                tab.render_filter = filter;
                tab.view_state.rendered_pages.clear();
            }
            app.render_visible_pages()
        }
        Message::ToggleAutoCrop => {
            if let Some(tab) = app.current_tab_mut() {
                tab.auto_crop = !tab.auto_crop;
                tab.view_state.rendered_pages.clear();
            }
            app.render_visible_pages()
        }
        Message::ViewportChanged(y, height) => {
            if let Some(tab) = app.current_tab_mut() {
                tab.view_state.viewport_y = y;
                tab.view_state.viewport_height = height;
                tab.update_visible_range();
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
                tab.view_state.sidebar_viewport_y = y;
            }
            Task::none()
        }
        Message::RequestRender(page_idx) => {
            let (doc_id, zoom, rotation, filter, auto_crop, quality) = {
                let Some(tab) = app.current_tab() else {
                    return Task::none();
                };

                let needs_render =
                    if let Some((scale, _)) = tab.view_state.rendered_pages.get(&page_idx) {
                        (scale - tab.zoom).abs() > 0.001
                    } else {
                        true
                    };

                if !needs_render
                    || app
                        .rendering_set
                        .contains(&crate::app::RenderTarget::Page(tab.id, page_idx))
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
                .insert(crate::app::RenderTarget::Page(doc_id, page_idx));

            let Some(engine) = &app.engine else {
                return Task::none();
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
                    if let Err(e) = cmd_tx.try_send(crate::commands::PdfCommand::Render(
                        doc_id, page_idx, options, resp_tx,
                    )) {
                        tracing::warn!("Failed to send Render command: {e}");
                        return Err(crate::models::PdfError::Cancelled);
                    }
                    resp_rx
                        .await
                        .unwrap_or(Err(crate::models::PdfError::ChannelClosed))
                },
                move |res| Message::PageRendered(doc_id, page_idx, zoom, res),
            )
        }
        Message::PageRendered(doc_id, page_idx, scale, result) => {
            app.rendering_set
                .remove(&crate::app::RenderTarget::Page(doc_id, page_idx));

            let mut text_tasks: Vec<Task<Message>> = Vec::new();

            if let Some(tab) = app.tabs.iter_mut().find(|t| t.id == doc_id) {
                match result {
                    Ok(res) => {
                        let width = res.width;
                        let height = res.height;
                        let pixel_data = res.data.to_vec();
                        tab.view_state.rendered_pages.insert(
                            page_idx,
                            (
                                scale,
                                iced_image::Handle::from_rgba(width, height, pixel_data),
                            ),
                        );

                        // Lazily fetch text (selection / accessibility) so the
                        // image paints without blocking on glyph extraction.
                        if !tab.view_state.text_layers.contains_key(&page_idx)
                            && !app.pending_text.contains(&(doc_id, page_idx))
                        {
                            app.pending_text.insert((doc_id, page_idx));
                            if let Some(engine) = &app.engine {
                                let cmd_tx = engine.cmd_tx.clone();
                                text_tasks.push(Task::perform(
                                    async move {
                                        let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();
                                        let _ = cmd_tx
                                            .send(crate::commands::PdfCommand::GetTextItems(
                                                doc_id, page_idx, resp_tx,
                                            ))
                                            .await;
                                        match resp_rx.await {
                                            Ok(r) => (doc_id, page_idx, r),
                                            Err(_) => {
                                                (doc_id, page_idx, Err(PdfError::ChannelClosed))
                                            }
                                        }
                                    },
                                    |(d, p, r)| Message::TextItemsLoaded(d, p, r),
                                ));
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!("Render error: {e}");
                        if e == "Engine died" || e == "Channel closed" {
                            app.engine = None;
                            app.status_message = Some(
                                "PDF engine crashed. Please try your action again to restart it."
                                    .into(),
                            );
                        } else if e.to_string().to_lowercase().contains("pdfium") {
                            app.engine = None;
                            app.status_message =
                                Some("Failed to load PDF engine (pdfium.dll missing).".into());
                        }
                    }
                }
            }

            let render_task = app.render_visible_pages();
            if text_tasks.is_empty() {
                render_task
            } else {
                text_tasks.push(render_task);
                Task::batch(text_tasks)
            }
        }
        Message::TextItemsLoaded(doc_id, page_idx, result) => {
            app.pending_text.remove(&(doc_id, page_idx));
            if let Ok(items) = result {
                if let Some(tab) = app.tabs.iter_mut().find(|t| t.id == doc_id) {
                    tab.view_state.text_layers.insert(page_idx, items);
                }
            }
            Task::none()
        }
        Message::ThumbnailRendered(doc_id, page_idx, scale, result) => {
            app.rendering_set
                .remove(&crate::app::RenderTarget::Thumbnail(doc_id, page_idx));

            if let Some(tab) = app.tabs.iter_mut().find(|t| t.id == doc_id) {
                let expected_thumb_zoom = (120.0 / tab.page_width.max(1.0)).min(5.0);
                if (expected_thumb_zoom - scale).abs() > 0.001 {
                    return Task::none();
                }

                match result {
                    Ok(res) => {
                        let width = res.width;
                        let height = res.height;
                        let pixel_data = res.data.to_vec();
                        tab.view_state.thumbnails.insert(
                            page_idx,
                            iced_image::Handle::from_rgba(width, height, pixel_data),
                        );
                    }
                    Err(e) => {
                        tracing::error!("Thumbnail render error: {e}");
                    }
                }
            }
            Task::none()
        }
        _ => Task::none(),
    }
}
