use crate::app::PdfBullApp;
use crate::message::Message;
use iced::Task;
use iced::widget::image as iced_image;

pub fn handle_render_message(app: &mut PdfBullApp, message: Message) -> Task<Message> {
    match message {
        Message::SetFilter(filter) => {
            if let Some(tab) = app.current_tab_mut() {
                if tab.render_filter != filter {
                    tab.render_filter = filter;
                    tab.view_state.rendered_pages.clear();
                }
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
                    if let Err(e) = cmd_tx.send(crate::commands::PdfCommand::Render(
                        doc_id, page_idx, options, resp_tx,
                    )) {
                        tracing::error!("Failed to send Render command: {e}");

                        return Err(crate::models::PdfError::EngineDied);
                    }
                    resp_rx
                        .await
                        .unwrap_or(Err(crate::models::PdfError::ChannelClosed))
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
                        tab.view_state
                            .text_layers
                            .insert(page_idx, res.text_items.clone());
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
            app.render_visible_pages()
        }
        Message::ThumbnailRendered(page_idx, scale, result) => {
            app.rendering_count = app.rendering_count.saturating_sub(1);
            app.rendering_set
                .remove(&crate::app::RenderTarget::Thumbnail(page_idx));

            if let Some(tab) = app.current_tab_mut() {
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
