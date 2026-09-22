use gpui_kit::base::StyledExt;
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::Sizable;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::*;

use super::{
    canvas::DocumentViewport,
    dialogs::{ActiveDialog, DialogAction, DialogsState},
    log_console::{LogAction, LogConsoleState},
    ribbon::{RibbonAction, RibbonState},
    sidebar::{SidebarAction, SidebarState},
    tabs::{DocumentTab, TabAction, TabsState},
    welcome::{WelcomeAction, WelcomeState},
};

pub struct PdfbullView {
    pub engine: crate::engine::EngineState,
    pub active_doc_id: Option<crate::models::DocumentId>,
    pub rendering_pages: std::collections::HashSet<usize>,
    pub ribbon: RibbonState,
    pub tabs: TabsState,
    pub sidebar: SidebarState,
    pub viewport: DocumentViewport,
    pub dialogs: DialogsState,
    pub welcome: WelcomeState,
    pub log_console: LogConsoleState,
    pub focus_handle: FocusHandle,
    pub status_message: Option<String>,
}

pub fn inspect_pdf_geometry(path: &std::path::Path) -> Option<(usize, f32, f32)> {
    let doc = lopdf::Document::load(path).ok()?;
    let pages = doc.get_pages();
    let page_count = pages.len().max(1);
    let mut page_width = 595.0;
    let mut page_height = 842.0;

    if let Some((_, &first_page_id)) = pages.iter().next()
        && let Ok(first_page) = doc.get_object(first_page_id)
        && let Ok(dict) = first_page.as_dict()
        && let Ok(media_box) = dict.get(b"MediaBox")
        && let Ok(arr) = media_box.as_array()
        && arr.len() == 4
    {
        let to_f32 = |obj: &lopdf::Object| match obj {
            lopdf::Object::Real(v) => *v,
            lopdf::Object::Integer(v) => *v as f32,
            _ => 0.0,
        };
        let w = (to_f32(&arr[2]) - to_f32(&arr[0])).abs();
        let h = (to_f32(&arr[3]) - to_f32(&arr[1])).abs();
        if w > 0.0 && h > 0.0 {
            page_width = w;
            page_height = h;
        }
    }

    Some((page_count, page_width, page_height))
}

impl PdfbullView {
    pub fn new(_window: &mut Window, _cx: &mut Context<Self>) -> Self {
        let mut welcome = WelcomeState::new();
        welcome.load_recent_files();
        let focus_handle = _cx.focus_handle();

        Self {
            engine: crate::engine::spawn_engine_thread(64, 512),
            active_doc_id: None,
            rendering_pages: std::collections::HashSet::new(),
            ribbon: RibbonState::new(),
            tabs: TabsState::new(),
            sidebar: SidebarState::new(),
            viewport: DocumentViewport::new(),
            dialogs: DialogsState::new(),
            welcome,
            log_console: LogConsoleState::new(),
            focus_handle,
            status_message: None,
        }
    }

    pub fn open_sample_doc(&mut self, title: &str) {
        let new_id = self.tabs.tabs.len();
        self.tabs.tabs.push(DocumentTab {
            id: new_id,
            doc_id: None,
            title: title.to_string(),
            path: None,
            is_modified: false,
        });
        self.tabs.active_tab_index = new_id;
        self.active_doc_id = None;
        self.viewport.total_pages = 5;
        self.viewport.current_page = 0;
        self.viewport.rendered_pages.clear();
        self.viewport.rendered_zoom.clear();
        self.sidebar.thumbnails.clear();
        self.rendering_pages.clear();
    }

    pub fn open_pdf_path(&mut self, path_str: &str, cx: &mut Context<Self>) {
        let path = std::path::PathBuf::from(path_str);
        if !path.exists() {
            return;
        }

        // If document is already open in a tab, simply switch to it
        if let Some(existing_idx) = self
            .tabs
            .tabs
            .iter()
            .position(|t| t.path.as_ref() == Some(&path))
        {
            self.tabs.active_tab_index = existing_idx;
            self.sync_viewport_to_active_tab(cx);
            return;
        }

        let title = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Document.pdf")
            .to_string();

        let (page_count, page_width, page_height) =
            inspect_pdf_geometry(&path).unwrap_or((1, 595.0, 842.0));

        let mut loaded_recents = crate::storage::load_recent_files();
        crate::storage::add_recent_file(&mut loaded_recents, &path);
        self.welcome.recent_files = loaded_recents
            .into_iter()
            .map(|f| std::path::PathBuf::from(f.path))
            .filter(|p| p.exists())
            .collect();

        let doc_id = crate::models::next_doc_id();
        self.active_doc_id = Some(doc_id);

        let new_id = self.tabs.tabs.len();
        self.tabs.tabs.push(DocumentTab {
            id: new_id,
            doc_id: Some(doc_id),
            title,
            path: Some(path.clone()),
            is_modified: false,
        });
        self.tabs.active_tab_index = new_id;
        self.viewport.page_width = page_width;
        self.viewport.page_height = page_height;
        self.viewport.total_pages = page_count;
        self.viewport.current_page = 0;
        self.viewport.rendered_pages.clear();
        self.viewport.rendered_zoom.clear();
        self.sidebar.thumbnails.clear();
        self.rendering_pages.clear();

        let engine_tx = self.engine.cmd_tx.clone();
        let path_str_cloned = path.to_string_lossy().to_string();

        cx.spawn(async move |this, cx| {
            let (tx, rx) = tokio::sync::oneshot::channel();
            if let Ok(()) = engine_tx
                .send(crate::commands::PdfCommand::Open(
                    path_str_cloned.clone(),
                    None,
                    doc_id,
                    tx,
                ))
                .await
            {
                match rx.await {
                    Ok(Ok(open_res)) => {
                        let _ = this.update(cx, |view, cx| {
                            if view.active_doc_id == Some(doc_id) {
                                view.viewport.total_pages = open_res.page_count;
                                view.render_needed_pages(cx);
                                cx.notify();
                            }
                        });

                        // Load document metadata (outline bookmarks, attachments, layers)
                        let (meta_tx, meta_rx) = tokio::sync::oneshot::channel();
                        if let Ok(()) = engine_tx
                            .send(crate::commands::PdfCommand::LoadDocumentMeta(
                                doc_id, meta_tx,
                            ))
                            .await
                            && let Ok(Ok(meta)) = meta_rx.await
                        {
                            let _ = this.update(cx, |view, cx| {
                                view.sidebar.bookmarks = meta.outline;
                                view.sidebar.attachments = meta.attachments;
                                view.sidebar.layers = meta.layers;
                                cx.notify();
                            });
                        }

                        // Load previously saved annotations
                        let (ann_tx, ann_rx) = tokio::sync::oneshot::channel();
                        if let Ok(()) = engine_tx
                            .send(crate::commands::PdfCommand::LoadAnnotations(
                                doc_id,
                                path_str_cloned.clone(),
                                ann_tx,
                            ))
                            .await
                            && let Ok(Ok(loaded_anns)) = ann_rx.await
                        {
                            let _ = this.update(cx, |view, cx| {
                                view.viewport.annotations = loaded_anns;
                                cx.notify();
                            });
                        }
                    }
                    Ok(Err(e)) => {
                        tracing::error!("Failed to open PDF in engine worker: {e:?}");
                    }
                    Err(e) => {
                        tracing::error!("Open response channel dropped: {e}");
                    }
                }
            }
        })
        .detach();

        self.render_needed_pages(cx);
    }

    pub fn prompt_open_file(&self, cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| {
            let files = rfd::AsyncFileDialog::new()
                .add_filter("PDF Documents (*.pdf)", &["pdf"])
                .pick_files()
                .await;

            if let Some(files) = files {
                let paths: Vec<std::path::PathBuf> =
                    files.into_iter().map(|f| f.path().to_path_buf()).collect();

                let _ = this.update(cx, |view, cx| {
                    let mut opened = false;
                    for path in paths {
                        view.open_pdf_path(&path.to_string_lossy(), cx);
                        opened = true;
                    }
                    if opened {
                        cx.notify();
                    }
                });
            }
        })
        .detach();
    }

    pub fn sync_viewport_to_active_tab(&mut self, cx: &mut Context<Self>) {
        if let Some(tab) = self.tabs.tabs.get(self.tabs.active_tab_index) {
            let doc_id = tab.doc_id;
            if self.active_doc_id != doc_id {
                self.active_doc_id = doc_id;
                self.viewport.rendered_pages.clear();
                self.viewport.rendered_zoom.clear();
                self.sidebar.thumbnails.clear();
                self.rendering_pages.clear();
                self.viewport.selection_page = None;
                self.viewport.selection_start = None;
                self.viewport.selection_end = None;
                self.viewport.selected_text = None;
                self.viewport.annotations.clear();
            }
            if let Some(ref path) = tab.path
                && let Some((page_count, page_width, page_height)) = inspect_pdf_geometry(path)
            {
                self.viewport.total_pages = page_count;
                self.viewport.page_width = page_width;
                self.viewport.page_height = page_height;
                if self.viewport.current_page >= self.viewport.total_pages {
                    self.viewport.current_page = 0;
                }
            }
            self.render_needed_pages(cx);
        }
    }

    pub fn render_needed_pages(&mut self, cx: &mut Context<Self>) {
        let Some(doc_id) = self.active_doc_id else {
            return;
        };
        let total = self.viewport.total_pages;
        if total == 0 {
            return;
        }

        let cur = self.viewport.current_page;
        let pages_to_render: Vec<usize> = match self.viewport.layout_mode {
            super::ribbon::PageLayoutMode::SinglePage => {
                vec![cur.min(total - 1)]
            }
            super::ribbon::PageLayoutMode::TwoPageSpread => {
                let left = cur.min(total - 1);
                if left + 1 < total {
                    vec![left, left + 1]
                } else {
                    vec![left]
                }
            }
            super::ribbon::PageLayoutMode::Continuous => {
                let start = cur.saturating_sub(3);
                let end = (cur + 3).min(total - 1);
                (start..=end).collect()
            }
        };

        for page_idx in pages_to_render {
            self.request_render_page(doc_id, page_idx, cx);
            if !self.viewport.text_cache.contains_key(&page_idx) {
                self.request_text_items(doc_id, page_idx, cx);
            }
        }
    }

    pub fn request_render_page(
        &mut self,
        doc_id: crate::models::DocumentId,
        page_idx: usize,
        cx: &mut Context<Self>,
    ) {
        if let Some(&rendered_z) = self.viewport.rendered_zoom.get(&page_idx)
            && (rendered_z - self.viewport.zoom).abs() < 0.01
        {
            return;
        }
        if self.rendering_pages.contains(&page_idx) {
            return;
        }

        self.rendering_pages.insert(page_idx);
        let engine_tx = self.engine.cmd_tx.clone();
        let zoom = self.viewport.zoom;
        let rotation = self.viewport.rotation as i32;
        let is_midnight = self.ribbon.midnight_mode;

        cx.spawn(async move |this, cx| {
            let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();
            // 1.5x scale multiplier ensures crisp text and graphics on HiDPI displays
            let scale = (zoom * 1.5).clamp(0.75, 3.0);
            let options = crate::pdf_engine::RenderOptions {
                scale,
                rotation,
                filter: if is_midnight {
                    crate::pdf_engine::RenderFilter::Inverted
                } else {
                    crate::pdf_engine::RenderFilter::None
                },
                auto_crop: false,
                quality: crate::pdf_engine::RenderQuality::High,
            };

            if let Err(e) = engine_tx
                .send(crate::commands::PdfCommand::Render(
                    doc_id, page_idx, options, resp_tx,
                ))
                .await
            {
                tracing::error!("Failed to send Render command: {e}");
                let _ = this.update(cx, |view, _| {
                    view.rendering_pages.remove(&page_idx);
                });
                return;
            }

            match resp_rx.await {
                Ok(Ok(render_res)) => {
                    let mut raw_data = render_res.data.to_vec();
                    crate::ui_gpui::canvas::convert_rgba_to_bgra(&mut raw_data);

                    if let Some(buf) =
                        image::RgbaImage::from_raw(render_res.width, render_res.height, raw_data)
                    {
                        let frame = image::Frame::new(buf);
                        let render_image = std::sync::Arc::new(RenderImage::new(vec![frame]));

                        let _ = this.update(cx, |view, cx| {
                            view.rendering_pages.remove(&page_idx);
                            if view.active_doc_id == Some(doc_id) {
                                view.viewport
                                    .rendered_pages
                                    .insert(page_idx, render_image.clone());
                                view.viewport.rendered_zoom.insert(page_idx, zoom);

                                // Evict full-res page buffers outside the ±3 page window to
                                // bound memory usage. search_highlights are coordinate-only
                                // and must NOT be evicted here.
                                let cur = view.viewport.current_page;
                                let keep_start = cur.saturating_sub(3);
                                let keep_end = cur + 3;
                                view.viewport.rendered_pages
                                    .retain(|&p, _| p >= keep_start && p <= keep_end);
                                view.viewport.rendered_zoom
                                    .retain(|&p, _| p >= keep_start && p <= keep_end);

                                // Evict text cache outside a wider ±10 window so that
                                // text selection on recently visited pages still works
                                // without a round-trip, while bounding peak RAM.
                                let tc_start = cur.saturating_sub(10);
                                let tc_end = cur + 10;
                                view.viewport.text_cache
                                    .retain(|&p, _| p >= tc_start && p <= tc_end);

                                // Request a dedicated low-res thumbnail instead of
                                // re-using the full-resolution canvas image.
                                if view.sidebar.is_open
                                    && !view.sidebar.thumbnails.contains_key(&page_idx)
                                {
                                    view.request_thumbnail(doc_id, page_idx, cx);
                                }

                                if !view.viewport.text_cache.contains_key(&page_idx) {
                                    view.request_text_items(doc_id, page_idx, cx);
                                }
                                if (view.viewport.zoom - zoom).abs() >= 0.01 {
                                    view.request_render_page(doc_id, page_idx, cx);
                                }
                                cx.notify();
                            }
                        });
                    } else {
                        tracing::error!("Failed to create RgbaImage for page {page_idx}");
                        let _ = this.update(cx, |view, _| {
                            view.rendering_pages.remove(&page_idx);
                        });
                    }
                }
                Ok(Err(e)) => {
                    tracing::error!("Render page {page_idx} error: {e:?}");
                    let _ = this.update(cx, |view, _| {
                        view.rendering_pages.remove(&page_idx);
                    });
                }
                Err(e) => {
                    tracing::error!("Render response dropped: {e}");
                    let _ = this.update(cx, |view, _| {
                        view.rendering_pages.remove(&page_idx);
                    });
                }
            }
        })
        .detach();
    }

    pub fn request_text_items(
        &mut self,
        doc_id: crate::models::DocumentId,
        page_idx: usize,
        cx: &mut Context<Self>,
    ) {
        if self.viewport.text_cache.contains_key(&page_idx) {
            return;
        }
        let engine_tx = self.engine.cmd_tx.clone();
        cx.spawn(async move |this, cx| {
            let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();
            if engine_tx
                .send(crate::commands::PdfCommand::GetTextItems(
                    doc_id, page_idx, resp_tx,
                ))
                .await
                .is_ok()
                && let Ok(Ok(items)) = resp_rx.await
            {
                let _ = this.update(cx, |view, _| {
                    view.viewport.text_cache.insert(page_idx, items);
                });
            }
        })
        .detach();
    }

    /// Requests a low-resolution thumbnail for the sidebar panel.
    /// Uses the dedicated `RenderThumbnail` engine command (0.25× scale,
    /// `RenderQuality::Low`) so the sidebar never holds full-resolution pixel
    /// data. This is the primary driver of the memory reduction.
    pub fn request_thumbnail(
        &mut self,
        doc_id: crate::models::DocumentId,
        page_idx: usize,
        cx: &mut Context<Self>,
    ) {
        // Already have a thumbnail for this page — nothing to do.
        if self.sidebar.thumbnails.contains_key(&page_idx) {
            return;
        }
        let engine_tx = self.engine.cmd_tx.clone();
        let rotation = self.viewport.rotation as i32;

        cx.spawn(async move |this, cx| {
            let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();
            // 0.25× scale → A4 thumbnail ≈ 149×211 px → ~125 KB per page
            // vs. full-res at 1.5× ≈ 8.9 MB per page (72× reduction).
            if engine_tx
                .send(crate::commands::PdfCommand::RenderThumbnail(
                    doc_id, page_idx, 0.25, rotation, resp_tx,
                ))
                .await
                .is_err()
            {
                return;
            }

            if let Ok(Ok(render_res)) = resp_rx.await {
                let mut raw_data = render_res.data.to_vec();
                crate::ui_gpui::canvas::convert_rgba_to_bgra(&mut raw_data);
                if let Some(buf) = image::RgbaImage::from_raw(
                    render_res.width,
                    render_res.height,
                    raw_data,
                ) {
                    let frame = image::Frame::new(buf);
                    let thumb = std::sync::Arc::new(RenderImage::new(vec![frame]));
                    let _ = this.update(cx, |view, cx| {
                        if view.active_doc_id == Some(doc_id) {
                            view.sidebar.thumbnails.insert(page_idx, thumb);
                            cx.notify();
                        }
                    });
                }
            }
        })
        .detach();
    }

    pub fn extract_selected_text(
        &mut self,
        page_idx: usize,
        start: Point<Pixels>,
        end: Point<Pixels>,
        cx: &mut Context<Self>,
    ) {
        let zoom = self.viewport.zoom.max(0.1);
        let Some(items) = self.viewport.text_cache.get(&page_idx).cloned() else {
            if let Some(doc_id) = self.active_doc_id {
                self.request_text_items(doc_id, page_idx, cx);
            }
            return;
        };

        let origin = self
            .viewport
            .page_origins
            .read()
            .ok()
            .and_then(|m| m.get(&page_idx).copied())
            .unwrap_or_default();

        let min_x = ((start.x.min(end.x) - origin.x) / px(1.0)) / zoom;
        let max_x = ((start.x.max(end.x) - origin.x) / px(1.0)) / zoom;
        let min_y = ((start.y.min(end.y) - origin.y) / px(1.0)) / zoom;
        let max_y = ((start.y.max(end.y) - origin.y) / px(1.0)) / zoom;

        let mut matched_words: Vec<String> = Vec::new();
        for item in &items {
            let ix2 = item.x + item.width;
            let iy2 = item.y + item.height;
            if ix2 >= min_x && item.x <= max_x && iy2 >= min_y && item.y <= max_y {
                matched_words.push(item.text.clone());
            }
        }

        if !matched_words.is_empty() {
            let combined = matched_words.join(" ");
            self.viewport.selected_text = Some(combined.clone());
            cx.write_to_clipboard(ClipboardItem::new_string(combined));
        }
    }

    pub fn handle_canvas_scroll_wheel(&mut self, event: &ScrollWheelEvent, cx: &mut Context<Self>) {
        let is_ctrl = event.modifiers.control || event.modifiers.platform;
        if is_ctrl {
            let delta = match event.delta {
                ScrollDelta::Pixels(p) => p.y / px(1.0),
                ScrollDelta::Lines(l) => l.y * 20.0,
            };
            let factor = if delta > 0.0 {
                1.1
            } else if delta < 0.0 {
                1.0 / 1.1
            } else {
                1.0
            };
            let new_zoom = (self.viewport.zoom * factor).clamp(0.25, 5.0);
            if (new_zoom - self.viewport.zoom).abs() > 0.001 {
                self.viewport.zoom = new_zoom;
                self.render_needed_pages(cx);
                cx.notify();
            }
        } else {
            let top_item = self.viewport.scroll_handle.top_item();
            if top_item < self.viewport.total_pages && top_item != self.viewport.current_page {
                self.viewport.current_page = top_item;
                self.render_needed_pages(cx);
                cx.notify();
            }
        }
    }

    fn render_status_bar(&self, cx: &mut Context<Self>) -> AnyElement {
        let border = cx.theme().border;
        let muted = cx.theme().muted;
        let fg = cx.theme().foreground;
        let muted_fg = cx.theme().muted_foreground;
        let total = self.viewport.total_pages;
        let current = self.viewport.current_page;
        let zoom_pct = (self.viewport.zoom * 100.0).round() as u32;
        let page_w = self.viewport.page_width.round() as u32;
        let page_h = self.viewport.page_height.round() as u32;

        let prev_click = cx.listener(|this, _, _, cx| {
            if this.viewport.current_page > 0 {
                this.viewport.current_page -= 1;
                this.viewport
                    .scroll_handle
                    .scroll_to_item(this.viewport.current_page);
                this.render_needed_pages(cx);
                cx.notify();
            }
        });

        let next_click = cx.listener(|this, _, _, cx| {
            if this.viewport.current_page + 1 < this.viewport.total_pages {
                this.viewport.current_page += 1;
                this.viewport
                    .scroll_handle
                    .scroll_to_item(this.viewport.current_page);
                this.render_needed_pages(cx);
                cx.notify();
            }
        });

        let zoom_in_click = cx.listener(|this, _, _, cx| {
            this.viewport.zoom = (this.viewport.zoom + 0.1).min(5.0);
            this.render_needed_pages(cx);
            cx.notify();
        });

        let zoom_out_click = cx.listener(|this, _, _, cx| {
            this.viewport.zoom = (this.viewport.zoom - 0.1).max(0.25);
            this.render_needed_pages(cx);
            cx.notify();
        });

        let zoom_reset_click = cx.listener(|this, _, _, cx| {
            this.viewport.zoom = 1.0;
            this.render_needed_pages(cx);
            cx.notify();
        });

        div()
            .flex()
            .flex_row()
            .items_center()
            .justify_between()
            .h(px(28.0))
            .px_3()
            .bg(muted)
            .border_t_1()
            .border_color(border)
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_3()
                    .child(
                        div()
                            .text_xs()
                            .text_color(muted_fg)
                            .child(format!("{} × {} pt", page_w, page_h)),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(muted_fg)
                            .child(format!("{:?}", self.viewport.layout_mode)),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_2()
                    .child(
                        Button::new("btn-status-prev")
                            .label("◀")
                            .ghost()
                            .small()
                            .on_click(prev_click),
                    )
                    .child(div().text_xs().font_medium().text_color(fg).child(format!(
                        "Page {} of {}",
                        current + 1,
                        total
                    )))
                    .child(
                        Button::new("btn-status-next")
                            .label("▶")
                            .ghost()
                            .small()
                            .on_click(next_click),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_1()
                    .child(
                        Button::new("btn-status-zoom-out")
                            .label("－")
                            .ghost()
                            .small()
                            .on_click(zoom_out_click),
                    )
                    .child(
                        Button::new("btn-status-zoom-pct")
                            .label(SharedString::from(format!("{}%", zoom_pct)))
                            .ghost()
                            .small()
                            .on_click(zoom_reset_click),
                    )
                    .child(
                        Button::new("btn-status-zoom-in")
                            .label("＋")
                            .ghost()
                            .small()
                            .on_click(zoom_in_click),
                    ),
            )
            .into_any_element()
    }

    pub fn save_active_document(&mut self, cx: &mut Context<Self>) {
        let active_idx = self.tabs.active_tab_index;
        let Some(tab) = self.tabs.tabs.get(active_idx) else {
            return;
        };
        let Some(doc_id) = tab.doc_id else {
            return;
        };
        let Some(_path) = tab.path.clone() else {
            self.save_as_active_document(cx);
            return;
        };

        let annotations = self.viewport.annotations.clone();
        let cmd_tx = self.engine.cmd_tx.clone();

        cx.spawn(async move |this, cx| {
            let (tx, rx) = tokio::sync::oneshot::channel();
            let _ = cmd_tx
                .send(crate::commands::PdfCommand::SaveAnnotations(
                    doc_id,
                    annotations,
                    tx,
                ))
                .await;
            match rx.await {
                Ok(Ok(_)) => {
                    let _ = this.update(cx, |view, cx| {
                        if let Some(t) = view.tabs.tabs.get_mut(active_idx) {
                            t.is_modified = false;
                        }
                        view.status_message = Some("Document saved successfully.".into());
                        cx.notify();
                    });
                }
                Ok(Err(e)) => {
                    let _ = this.update(cx, |view, cx| {
                        view.status_message = Some(format!("Save error: {e}"));
                        cx.notify();
                    });
                }
                Err(_) => {}
            }
        })
        .detach();
    }

    pub fn save_as_active_document(&mut self, cx: &mut Context<Self>) {
        let active_idx = self.tabs.active_tab_index;
        let Some(tab) = self.tabs.tabs.get(active_idx) else {
            return;
        };
        let Some(doc_id) = tab.doc_id else {
            return;
        };
        let default_name = tab.title.clone();
        let annotations = self.viewport.annotations.clone();
        let cmd_tx = self.engine.cmd_tx.clone();

        cx.spawn(async move |this, cx| {
            let file = rfd::AsyncFileDialog::new()
                .set_file_name(&default_name)
                .add_filter("PDF files (*.pdf)", &["pdf"])
                .save_file()
                .await;
            let Some(file) = file else {
                return;
            };
            let out_path = file.path().to_string_lossy().to_string();

            let (tx, rx) = tokio::sync::oneshot::channel();
            let _ = cmd_tx
                .send(crate::commands::PdfCommand::ExportPdf(
                    doc_id,
                    out_path.clone(),
                    annotations,
                    tx,
                ))
                .await;
            if let Ok(Ok(_)) = rx.await {
                let _ = this.update(cx, |view, cx| {
                    if let Some(t) = view.tabs.tabs.get_mut(active_idx) {
                        t.path = Some(std::path::PathBuf::from(&out_path));
                        t.title = std::path::Path::new(&out_path)
                            .file_name()
                            .map(|f| f.to_string_lossy().to_string())
                            .unwrap_or_else(|| "document.pdf".into());
                        t.is_modified = false;
                    }
                    view.status_message = Some("Exported PDF successfully.".into());
                    cx.notify();
                });
            }
        })
        .detach();
    }

    pub fn print_active_document(&mut self, cx: &mut Context<Self>) {
        let Some(tab) = self.tabs.tabs.get(self.tabs.active_tab_index) else {
            return;
        };
        let Some(path) = tab.path.clone() else {
            return;
        };
        let path_str = path.to_string_lossy().to_string();
        let cmd_tx = self.engine.cmd_tx.clone();
        self.status_message = Some("Sending to printer spooler...".into());
        cx.notify();

        cx.spawn(async move |this, cx| {
            let (tx, rx) = tokio::sync::oneshot::channel();
            let _ = cmd_tx
                .send(crate::commands::PdfCommand::PrintPdf(path_str, None, tx))
                .await;
            match rx.await {
                Ok(Ok(())) => {
                    let _ = this.update(cx, |view, cx| {
                        view.status_message = Some("Printed successfully.".into());
                        cx.notify();
                    });
                }
                Ok(Err(e)) => {
                    let _ = this.update(cx, |view, cx| {
                        view.status_message = Some(format!("Print failed: {e}"));
                        cx.notify();
                    });
                }
                Err(_) => {}
            }
        })
        .detach();
    }

    pub fn execute_search(&mut self, query: &str, cx: &mut Context<Self>) {
        let Some(doc_id) = self.active_doc_id else {
            return;
        };
        if query.trim().is_empty() {
            self.sidebar.search_results.clear();
            self.viewport.search_highlights.clear();
            self.sidebar.is_searching = false;
            cx.notify();
            return;
        }

        self.sidebar.is_searching = true;
        self.sidebar.search_query = query.to_string();
        let cmd_tx = self.engine.cmd_tx.clone();
        let query_str = query.to_string();

        cx.spawn(async move |this, cx| {
            let (tx, rx) = tokio::sync::oneshot::channel();
            let _ = cmd_tx
                .send(crate::commands::PdfCommand::Search(doc_id, query_str, tx))
                .await;
            if let Ok(Ok(results)) = rx.await {
                let _ = this.update(cx, |view, cx| {
                    view.sidebar.is_searching = false;
                    view.sidebar.search_results = results.clone();
                    view.viewport.search_highlights.clear();
                    for item in &results {
                        view.viewport
                            .search_highlights
                            .entry(item.page_index)
                            .or_default()
                            .push((item.x, item.y, item.width, item.height));
                    }
                    if let Some(first) = results.first() {
                        view.viewport.current_page = first.page_index;
                        view.viewport.scroll_handle.scroll_to_item(first.page_index);
                        view.render_needed_pages(cx);
                    }
                    cx.notify();
                });
            }
        })
        .detach();
    }

    pub fn apply_watermark(&mut self, text: &str, cx: &mut Context<Self>) {
        let Some(tab) = self.tabs.tabs.get(self.tabs.active_tab_index) else {
            return;
        };
        let Some(path) = tab.path.clone() else {
            return;
        };
        let text_str = text.to_string();
        let path_str = path.to_string_lossy().to_string();
        let cmd_tx = self.engine.cmd_tx.clone();

        cx.spawn(async move |this, cx| {
            let save_file = rfd::AsyncFileDialog::new()
                .add_filter("PDF files (*.pdf)", &["pdf"])
                .set_file_name("watermarked.pdf")
                .save_file()
                .await;
            let Some(save_file) = save_file else {
                return;
            };
            let out_path = save_file.path().to_string_lossy().to_string();

            let (tx, rx) = tokio::sync::oneshot::channel();
            let _ = cmd_tx
                .send(crate::commands::PdfCommand::AddWatermark(
                    path_str,
                    text_str,
                    out_path.clone(),
                    tx,
                ))
                .await;

            if let Ok(Ok(_)) = rx.await {
                let _ = this.update(cx, |view, cx| {
                    view.open_pdf_path(&out_path, cx);
                    view.status_message = Some("Watermark applied successfully.".into());
                    cx.notify();
                });
            }
        })
        .detach();
    }

    pub fn apply_header_footer(&mut self, header: &str, footer: &str, cx: &mut Context<Self>) {
        let Some(tab) = self.tabs.tabs.get(self.tabs.active_tab_index) else {
            return;
        };
        let Some(path) = tab.path.clone() else {
            return;
        };
        let (h_str, f_str) = (header.to_string(), footer.to_string());
        let path_str = path.to_string_lossy().to_string();
        let cmd_tx = self.engine.cmd_tx.clone();

        cx.spawn(async move |this, cx| {
            let save_file = rfd::AsyncFileDialog::new()
                .add_filter("PDF files (*.pdf)", &["pdf"])
                .set_file_name("header_footer.pdf")
                .save_file()
                .await;
            let Some(save_file) = save_file else {
                return;
            };
            let out_path = save_file.path().to_string_lossy().to_string();

            let (tx, rx) = tokio::sync::oneshot::channel();
            let _ = cmd_tx
                .send(crate::commands::PdfCommand::AddHeaderFooter(
                    path_str,
                    h_str,
                    f_str,
                    out_path.clone(),
                    tx,
                ))
                .await;

            if let Ok(Ok(_)) = rx.await {
                let _ = this.update(cx, |view, cx| {
                    view.open_pdf_path(&out_path, cx);
                    view.status_message = Some("Headers and footers added successfully.".into());
                    cx.notify();
                });
            }
        })
        .detach();
    }

    pub fn apply_security(&mut self, algo: &str, cx: &mut Context<Self>) {
        let Some(tab) = self.tabs.tabs.get(self.tabs.active_tab_index) else {
            return;
        };
        let Some(path) = tab.path.clone() else {
            return;
        };
        let algo_str = algo.to_string();
        let path_str = path.to_string_lossy().to_string();
        let cmd_tx = self.engine.cmd_tx.clone();

        cx.spawn(async move |this, cx| {
            let save_file = rfd::AsyncFileDialog::new()
                .add_filter("PDF files (*.pdf)", &["pdf"])
                .set_file_name("encrypted.pdf")
                .save_file()
                .await;
            let Some(save_file) = save_file else {
                return;
            };
            let out_path = save_file.path().to_string_lossy().to_string();

            let (tx, rx) = tokio::sync::oneshot::channel();
            let _ = cmd_tx
                .send(crate::commands::PdfCommand::EncryptPdf(
                    path_str,
                    out_path.clone(),
                    "user".to_string(),
                    "owner".to_string(),
                    algo_str,
                    tx,
                ))
                .await;

            if let Ok(Ok(_)) = rx.await {
                let _ = this.update(cx, |view, cx| {
                    view.open_pdf_path(&out_path, cx);
                    view.status_message = Some("Document encrypted successfully.".into());
                    cx.notify();
                });
            }
        })
        .detach();
    }

    pub fn rotate_active_pages(&mut self, angle: i32, cx: &mut Context<Self>) {
        self.viewport.rotation = (self.viewport.rotation as i32 + angle).rem_euclid(360) as u16;
        self.viewport.rendered_pages.clear();
        self.render_needed_pages(cx);
        self.status_message = Some(format!("Rotated by {} degrees.", angle));
        cx.notify();
    }

    pub fn delete_current_page(&mut self, cx: &mut Context<Self>) {
        let Some(tab) = self.tabs.tabs.get(self.tabs.active_tab_index) else {
            return;
        };
        let Some(path) = tab.path.clone() else {
            return;
        };
        let total = self.viewport.total_pages;
        if total <= 1 {
            self.status_message = Some("Cannot delete the only page in document.".into());
            cx.notify();
            return;
        }

        let cur = self.viewport.current_page;
        let remaining_pages: Vec<usize> = (0..total).filter(|&p| p != cur).collect();
        let path_str = path.to_string_lossy().to_string();
        let cmd_tx = self.engine.cmd_tx.clone();

        cx.spawn(async move |this, cx| {
            let save_file = rfd::AsyncFileDialog::new()
                .add_filter("PDF files (*.pdf)", &["pdf"])
                .set_file_name("reordered.pdf")
                .save_file()
                .await;
            let Some(save_file) = save_file else {
                return;
            };
            let out_path = save_file.path().to_string_lossy().to_string();

            let (tx, rx) = tokio::sync::oneshot::channel();
            let _ = cmd_tx
                .send(crate::commands::PdfCommand::ReorderPages(
                    path_str,
                    remaining_pages,
                    out_path.clone(),
                    tx,
                ))
                .await;

            if let Ok(Ok(_)) = rx.await {
                let _ = this.update(cx, |view, cx| {
                    view.open_pdf_path(&out_path, cx);
                    view.status_message = Some(format!("Page {} deleted.", cur + 1));
                    cx.notify();
                });
            }
        })
        .detach();
    }

    pub fn prompt_merge_files(&mut self, cx: &mut Context<Self>) {
        let cmd_tx = self.engine.cmd_tx.clone();
        cx.spawn(async move |this, cx| {
            let files = rfd::AsyncFileDialog::new()
                .add_filter("PDF files (*.pdf)", &["pdf"])
                .set_title("Select PDFs to Merge")
                .pick_files()
                .await;
            let Some(files) = files else {
                return;
            };
            if files.len() < 2 {
                let _ = this.update(cx, |view, cx| {
                    view.status_message =
                        Some("Please select at least 2 PDF files to merge.".into());
                    cx.notify();
                });
                return;
            }
            let save_file = rfd::AsyncFileDialog::new()
                .add_filter("PDF files (*.pdf)", &["pdf"])
                .set_title("Save Merged PDF As")
                .set_file_name("merged.pdf")
                .save_file()
                .await;
            let Some(save_file) = save_file else {
                return;
            };
            let out_path = save_file.path().to_string_lossy().to_string();
            let input_paths: Vec<String> = files
                .into_iter()
                .map(|f| f.path().to_string_lossy().to_string())
                .collect();

            let (tx, rx) = tokio::sync::oneshot::channel();
            let _ = cmd_tx
                .send(crate::commands::PdfCommand::Merge(
                    input_paths,
                    out_path.clone(),
                    tx,
                ))
                .await;

            if let Ok(Ok(_)) = rx.await {
                let _ = this.update(cx, |view, cx| {
                    view.open_pdf_path(&out_path, cx);
                    view.status_message =
                        Some("Merged PDF created and opened successfully.".into());
                    cx.notify();
                });
            }
        })
        .detach();
    }

    pub fn prompt_split_document(&mut self, cx: &mut Context<Self>) {
        let Some(tab) = self.tabs.tabs.get(self.tabs.active_tab_index) else {
            return;
        };
        let Some(path) = tab.path.clone() else {
            return;
        };
        let total_pages = self.viewport.total_pages;
        let path_str = path.to_string_lossy().to_string();
        let cmd_tx = self.engine.cmd_tx.clone();

        cx.spawn(async move |this, cx| {
            let folder = rfd::AsyncFileDialog::new()
                .set_title("Select Output Folder for Split Pages")
                .pick_folder()
                .await;
            let Some(folder) = folder else {
                return;
            };
            let out_dir = folder.path().to_string_lossy().to_string();
            let pages: Vec<usize> = (0..total_pages).collect();

            let (tx, rx) = tokio::sync::oneshot::channel();
            let _ = cmd_tx
                .send(crate::commands::PdfCommand::Split(
                    path_str, pages, out_dir, tx,
                ))
                .await;

            if let Ok(Ok(split_files)) = rx.await {
                let _ = this.update(cx, |view, cx| {
                    view.status_message = Some(format!(
                        "Successfully split into {} pages.",
                        split_files.len()
                    ));
                    cx.notify();
                });
            }
        })
        .detach();
    }

    pub fn prompt_compress_document(&mut self, cx: &mut Context<Self>) {
        let Some(tab) = self.tabs.tabs.get(self.tabs.active_tab_index) else {
            return;
        };
        let Some(path) = tab.path.clone() else {
            return;
        };
        let path_str = path.to_string_lossy().to_string();
        let cmd_tx = self.engine.cmd_tx.clone();

        cx.spawn(async move |this, cx| {
            let save_file = rfd::AsyncFileDialog::new()
                .add_filter("PDF files (*.pdf)", &["pdf"])
                .set_file_name("compressed.pdf")
                .save_file()
                .await;
            let Some(save_file) = save_file else {
                return;
            };
            let out_path = save_file.path().to_string_lossy().to_string();

            let (tx, rx) = tokio::sync::oneshot::channel();
            let _ = cmd_tx
                .send(crate::commands::PdfCommand::Optimize(
                    path_str,
                    out_path.clone(),
                    tx,
                ))
                .await;

            if let Ok(Ok(_)) = rx.await {
                let _ = this.update(cx, |view, cx| {
                    view.open_pdf_path(&out_path, cx);
                    view.status_message = Some("Document compressed and opened.".into());
                    cx.notify();
                });
            }
        })
        .detach();
    }

    pub fn prompt_ocr_page(&mut self, cx: &mut Context<Self>) {
        let Some(doc_id) = self.active_doc_id else {
            return;
        };
        let cur_page = self.viewport.current_page;
        let cmd_tx = self.engine.cmd_tx.clone();
        self.status_message = Some(format!("Running OCR on page {}...", cur_page + 1));
        cx.notify();

        cx.spawn(async move |this, cx| {
            let (tx, rx) = tokio::sync::oneshot::channel();
            let _ = cmd_tx
                .send(crate::commands::PdfCommand::OcrPage(
                    doc_id,
                    cur_page,
                    crate::ocr::OcrScript::Latin,
                    tx,
                ))
                .await;

            if let Ok(Ok(ocr_res)) = rx.await {
                let _ = this.update(cx, |view, cx| {
                    view.status_message = Some(format!(
                        "OCR complete: extracted {} lines.",
                        ocr_res.lines.len()
                    ));
                    cx.notify();
                });
            }
        })
        .detach();
    }

    pub fn convert_active_document(
        &mut self,
        ext: &'static str,
        format_name: &'static str,
        filter_name: &'static str,
        cx: &mut Context<Self>,
    ) {
        let Some(doc_id) = self.active_doc_id else {
            return;
        };
        let cmd_tx = self.engine.cmd_tx.clone();

        cx.spawn(async move |this, cx| {
            let save_file = rfd::AsyncFileDialog::new()
                .add_filter(filter_name, &[ext])
                .set_file_name(format!("document.{}", ext))
                .save_file()
                .await;
            let Some(save_file) = save_file else {
                return;
            };
            let out_path = save_file.path().to_string_lossy().to_string();

            let (tx, rx) = tokio::sync::oneshot::channel();
            let _ = cmd_tx
                .send(crate::commands::PdfCommand::ConvertPdf(
                    doc_id,
                    "document".to_string(),
                    format_name.to_string(),
                    tx,
                ))
                .await;

            if let Ok(Ok(content)) = rx.await {
                let _ = std::fs::write(&out_path, content);
                let _ = this.update(cx, |view, cx| {
                    view.status_message = Some(format!("Exported to {} successfully.", out_path));
                    cx.notify();
                });
            }
        })
        .detach();
    }

    pub fn handle_key_down(&mut self, event: &gpui::KeyDownEvent, cx: &mut Context<Self>) {
        let key = event.keystroke.key.to_lowercase();
        let ctrl = event.keystroke.modifiers.control || event.keystroke.modifiers.platform;

        if ctrl {
            match key.as_str() {
                "s" => {
                    self.save_active_document(cx);
                    cx.notify();
                }
                "o" => {
                    self.prompt_open_file(cx);
                }
                "p" => {
                    self.print_active_document(cx);
                    cx.notify();
                }
                "w" => {
                    if !self.tabs.tabs.is_empty() {
                        let idx = self.tabs.active_tab_index;
                        if idx < self.tabs.tabs.len() {
                            let removed = self.tabs.tabs.remove(idx);
                            if let Some(doc_id) = removed.doc_id {
                                let _ = self
                                    .engine
                                    .cmd_tx
                                    .try_send(crate::commands::PdfCommand::Close(doc_id));
                            }
                            if self.tabs.active_tab_index >= self.tabs.tabs.len()
                                && !self.tabs.tabs.is_empty()
                            {
                                self.tabs.active_tab_index = self.tabs.tabs.len() - 1;
                            }
                            self.sync_viewport_to_active_tab(cx);
                        }
                    }
                    cx.notify();
                }
                "f" => {
                    self.sidebar.is_open = true;
                    self.sidebar.mode = super::sidebar::SidebarMode::Search;
                    cx.notify();
                }
                "=" | "+" => {
                    self.viewport.zoom = (self.viewport.zoom + 0.1).min(5.0);
                    self.render_needed_pages(cx);
                    cx.notify();
                }
                "-" => {
                    self.viewport.zoom = (self.viewport.zoom - 0.1).max(0.25);
                    self.render_needed_pages(cx);
                    cx.notify();
                }
                "0" => {
                    self.viewport.zoom = 1.0;
                    self.render_needed_pages(cx);
                    cx.notify();
                }
                _ => {}
            }
        } else {
            match key.as_str() {
                "escape" => {
                    if self.dialogs.active.is_some() {
                        self.dialogs.active = None;
                        cx.notify();
                    } else if self.viewport.selection_page.is_some() {
                        self.viewport.selection_page = None;
                        self.viewport.selection_start = None;
                        self.viewport.selection_end = None;
                        cx.notify();
                    }
                }
                "pageup" | "left" if self.viewport.current_page > 0 => {
                    self.viewport.current_page -= 1;
                    self.viewport
                        .scroll_handle
                        .scroll_to_item(self.viewport.current_page);
                    self.render_needed_pages(cx);
                    cx.notify();
                }
                "pagedown" | "right"
                    if self.viewport.current_page + 1 < self.viewport.total_pages =>
                {
                    self.viewport.current_page += 1;
                    self.viewport
                        .scroll_handle
                        .scroll_to_item(self.viewport.current_page);
                    self.render_needed_pages(cx);
                    cx.notify();
                }
                _ => {}
            }
        }
    }
}

impl Render for PdfbullView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let bg = cx.theme().background;
        let fg = cx.theme().foreground;
        let border = cx.theme().border;
        let muted = cx.theme().muted;
        let muted_fg = cx.theme().muted_foreground;
        let primary = cx.theme().primary;
        let has_tabs = !self.tabs.tabs.is_empty();

        let ribbon_tabs = self.ribbon.render_tabs(cx, |this, tab, _, cx| {
            this.ribbon.active_tab = tab;
            cx.notify();
        });

        // Top Header bar
        let header_bar = div()
            .flex()
            .flex_row()
            .items_center()
            .justify_between()
            .h_10()
            .px_3()
            .bg(bg)
            .border_b_1()
            .border_color(border)
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_3()
                    .child(div().text_lg().text_color(primary).child("PDFbull"))
                    .child(ribbon_tabs),
            )
            .child(
                div().flex().flex_row().items_center().gap_2().child(
                    Button::new("btn-toggle-logs")
                        .label("Logs")
                        .ghost()
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.log_console.is_open = !this.log_console.is_open;
                            cx.notify();
                        })),
                ),
            );

        // Ribbon Action Strip
        let action_strip = self.ribbon.render_strip(cx, |this, action, _, cx| {
            match action {
                RibbonAction::OpenFile => {
                    this.prompt_open_file(cx);
                }
                RibbonAction::SaveFile => {
                    this.save_active_document(cx);
                }
                RibbonAction::Print => {
                    this.print_active_document(cx);
                }
                RibbonAction::ZoomIn => {
                    this.viewport.zoom = (this.viewport.zoom + 0.1).min(5.0);
                    this.render_needed_pages(cx);
                }
                RibbonAction::ZoomOut => {
                    this.viewport.zoom = (this.viewport.zoom - 0.1).max(0.25);
                    this.render_needed_pages(cx);
                }
                RibbonAction::ZoomReset => {
                    this.viewport.zoom = 1.0;
                    this.render_needed_pages(cx);
                }
                RibbonAction::PrevPage => {
                    if this.viewport.current_page > 0 {
                        this.viewport.current_page -= 1;
                        this.viewport
                            .scroll_handle
                            .scroll_to_item(this.viewport.current_page);
                        this.render_needed_pages(cx);
                    }
                }
                RibbonAction::NextPage => {
                    if this.viewport.current_page + 1 < this.viewport.total_pages {
                        this.viewport.current_page += 1;
                        this.viewport
                            .scroll_handle
                            .scroll_to_item(this.viewport.current_page);
                        this.render_needed_pages(cx);
                    }
                }
                RibbonAction::SetLayout(mode) => {
                    this.ribbon.layout_mode = mode;
                    this.viewport.layout_mode = mode;
                }
                RibbonAction::ToggleCover => {
                    this.ribbon.standalone_cover = !this.ribbon.standalone_cover;
                    this.viewport.standalone_cover = this.ribbon.standalone_cover;
                }
                RibbonAction::ToggleMidnight => {
                    this.ribbon.midnight_mode = !this.ribbon.midnight_mode;
                    this.viewport.rendered_pages.clear();
                    this.render_needed_pages(cx);
                }
                RibbonAction::SelectTool(tool) => {
                    this.ribbon.active_tool = tool;
                    this.viewport.highlight_color = match tool {
                        super::ribbon::AnnotationTool::Highlight => {
                            Some(hsla(50.0 / 360.0, 1.0, 0.5, 0.45))
                        }
                        _ => None,
                    };
                }
                RibbonAction::SelectColor(color) => {
                    this.ribbon.active_color = color;
                    this.viewport.highlight_color = Some(hsla(
                        color[0].clamp(0.0, 1.0),
                        color[1].clamp(0.0, 1.0),
                        color[2].clamp(0.0, 1.0),
                        color[3].clamp(0.0, 1.0),
                    ));
                }
                RibbonAction::Watermark => {
                    this.dialogs.active = Some(ActiveDialog::Watermark);
                }
                RibbonAction::HeaderFooter => {
                    this.dialogs.active = Some(ActiveDialog::HeaderFooter);
                }
                RibbonAction::FormFields => {}
                RibbonAction::DigitalSignatures => {
                    this.dialogs.active = Some(ActiveDialog::Signature);
                }
                RibbonAction::Merge => {
                    this.prompt_merge_files(cx);
                }
                RibbonAction::Split => {
                    this.prompt_split_document(cx);
                }
                RibbonAction::Flatten => {
                    this.save_as_active_document(cx);
                }
                RibbonAction::Encrypt => {
                    this.dialogs.active = Some(ActiveDialog::Security);
                }
                RibbonAction::Decrypt => {
                    this.dialogs.active = Some(ActiveDialog::Password);
                }
                RibbonAction::Compress => {
                    this.prompt_compress_document(cx);
                }
                RibbonAction::Ocr => {
                    this.prompt_ocr_page(cx);
                }
                RibbonAction::PageOrganizer => {
                    this.dialogs.active = Some(ActiveDialog::PageOrganizer);
                }
                RibbonAction::ConvertMarkdown => {
                    this.convert_active_document("md", "Markdown", "Markdown files (*.md)", cx);
                }
                RibbonAction::ConvertHtml => {
                    this.convert_active_document("html", "HTML", "HTML files (*.html)", cx);
                }
                RibbonAction::ConvertText => {
                    this.convert_active_document("txt", "Text", "Text files (*.txt)", cx);
                }
            }
            cx.notify();
        });

        // Tabs Bar
        let tabs_bar = if has_tabs {
            Some(self.tabs.render(cx, |this, action, _, cx| {
                match action {
                    TabAction::SelectTab(idx) => {
                        this.tabs.active_tab_index = idx;
                        this.sync_viewport_to_active_tab(cx);
                    }
                    TabAction::CloseTab(idx) => {
                        if idx < this.tabs.tabs.len() {
                            let removed = this.tabs.tabs.remove(idx);
                            if let Some(doc_id) = removed.doc_id {
                                let _ = this
                                    .engine
                                    .cmd_tx
                                    .try_send(crate::commands::PdfCommand::Close(doc_id));
                            }
                            if this.tabs.active_tab_index >= this.tabs.tabs.len()
                                && !this.tabs.tabs.is_empty()
                            {
                                this.tabs.active_tab_index = this.tabs.tabs.len() - 1;
                            }
                            this.sync_viewport_to_active_tab(cx);
                        }
                    }
                    TabAction::NewTab => {
                        this.prompt_open_file(cx);
                    }
                    TabAction::CloseOthers(keep_idx) => {
                        if keep_idx < this.tabs.tabs.len() {
                            let kept = this.tabs.tabs[keep_idx].clone();
                            for (i, tab) in this.tabs.tabs.iter().enumerate() {
                                if i != keep_idx
                                    && let Some(doc_id) = tab.doc_id
                                {
                                    let _ = this
                                        .engine
                                        .cmd_tx
                                        .try_send(crate::commands::PdfCommand::Close(doc_id));
                                }
                            }
                            this.tabs.tabs = vec![kept];
                            this.tabs.active_tab_index = 0;
                            this.sync_viewport_to_active_tab(cx);
                        }
                    }
                    TabAction::CloseToRight(idx) => {
                        for tab in this.tabs.tabs.drain((idx + 1)..) {
                            if let Some(doc_id) = tab.doc_id {
                                let _ = this
                                    .engine
                                    .cmd_tx
                                    .try_send(crate::commands::PdfCommand::Close(doc_id));
                            }
                        }
                        if this.tabs.active_tab_index >= this.tabs.tabs.len() {
                            this.tabs.active_tab_index = this.tabs.tabs.len() - 1;
                        }
                        this.sync_viewport_to_active_tab(cx);
                    }
                }
                cx.notify();
            }))
        } else {
            None
        };

        // Main Center Area (Welcome vs Document Workspace)
        let center_area: AnyElement = if has_tabs {
            self.render_needed_pages(cx);
            let total = self.viewport.total_pages;
            let current = self.viewport.current_page;

            let sidebar = self.sidebar.render(
                cx,
                total,
                current,
                &self.viewport.annotations,
                |this, action, _, cx| {
                    match action {
                        SidebarAction::ToggleOpen => {
                            this.sidebar.is_open = !this.sidebar.is_open;
                            // When the sidebar becomes visible, lazily pre-load
                            // thumbnails for pages near the current view.
                            if this.sidebar.is_open
                                && this.sidebar.mode == super::sidebar::SidebarMode::Thumbnails
                                && let Some(doc_id) = this.active_doc_id
                            {
                                let cur = this.viewport.current_page;
                                let total = this.viewport.total_pages;
                                let start = cur.saturating_sub(5);
                                let end = (cur + 5).min(total.saturating_sub(1));
                                for p in start..=end {
                                    this.request_thumbnail(doc_id, p, cx);
                                }
                            }
                        }
                        SidebarAction::SelectMode(mode) => {
                            this.sidebar.mode = mode;
                        }
                        SidebarAction::SelectPage(page_idx) => {
                            this.viewport.current_page = page_idx;
                            this.viewport.scroll_handle.scroll_to_item(page_idx);
                            this.render_needed_pages(cx);
                        }
                        SidebarAction::SearchQuery(q) => {
                            this.sidebar.search_query = q;
                        }
                        SidebarAction::ExecuteSearch => {
                            let q = this.sidebar.search_query.clone();
                            this.execute_search(&q, cx);
                        }
                        SidebarAction::ClearSearch => {
                            this.sidebar.search_query.clear();
                            this.sidebar.search_results.clear();
                            this.viewport.search_highlights.clear();
                            this.sidebar.is_searching = false;
                        }
                        SidebarAction::SelectSearchResult(page, ..) => {
                            this.viewport.current_page = page;
                            this.viewport.scroll_handle.scroll_to_item(page);
                            this.render_needed_pages(cx);
                        }
                        SidebarAction::DeleteAnnotation(id) => {
                            this.viewport.annotations.retain(|a| a.id != id);
                            if let Some(tab) = this.tabs.tabs.get_mut(this.tabs.active_tab_index) {
                                tab.is_modified = true;
                            }
                        }
                        SidebarAction::SelectBookmark(page) => {
                            this.viewport.current_page = page;
                            this.viewport.scroll_handle.scroll_to_item(page);
                            this.render_needed_pages(cx);
                        }
                        SidebarAction::ToggleLayer(idx, visible) => {
                            if let Some(layer) = this.sidebar.layers.get_mut(idx) {
                                layer.visible = visible;
                            }
                        }
                    }
                    cx.notify();
                },
            );

            let canvas = self.viewport.render(
                cx,
                |this, page_idx, _, cx| {
                    this.viewport.current_page = page_idx;
                    this.render_needed_pages(cx);
                    cx.notify();
                },
                |this, page_idx, pos, _, cx| {
                    this.viewport.is_selecting = true;
                    this.viewport.selection_page = Some(page_idx);
                    this.viewport.selection_start = Some(pos);
                    this.viewport.selection_end = Some(pos);
                    this.viewport.selected_text = None;
                    cx.notify();
                },
                |this, page_idx, pos, _, cx| {
                    if this.viewport.is_selecting && this.viewport.selection_page == Some(page_idx)
                    {
                        this.viewport.selection_end = Some(pos);
                        cx.notify();
                    }
                },
                |this, page_idx, _, cx| {
                    if this.viewport.is_selecting && this.viewport.selection_page == Some(page_idx)
                    {
                        this.viewport.is_selecting = false;
                        if let (Some(start), Some(end)) =
                            (this.viewport.selection_start, this.viewport.selection_end)
                        {
                            let dx = (start.x - end.x).abs();
                            let dy = (start.y - end.y).abs();
                            if dx < px(4.0) && dy < px(4.0) {
                                this.viewport.selection_start = None;
                                this.viewport.selection_end = None;
                                this.viewport.selection_page = None;
                                this.viewport.selected_text = None;
                            } else {
                                this.extract_selected_text(page_idx, start, end, cx);

                                if this.ribbon.active_tool != super::ribbon::AnnotationTool::Pointer
                                {
                                    let origin = this
                                        .viewport
                                        .page_origins
                                        .read()
                                        .ok()
                                        .and_then(|m| m.get(&page_idx).copied())
                                        .unwrap_or_default();
                                    let zoom = this.viewport.zoom.max(0.1);
                                    let min_x = ((start.x.min(end.x) - origin.x) / px(1.0)) / zoom;
                                    let max_x = ((start.x.max(end.x) - origin.x) / px(1.0)) / zoom;
                                    let min_y = ((start.y.min(end.y) - origin.y) / px(1.0)) / zoom;
                                    let max_y = ((start.y.max(end.y) - origin.y) / px(1.0)) / zoom;
                                    let w = (max_x - min_x).max(10.0);
                                    let h = (max_y - min_y).max(10.0);

                                    let ann_id = this.viewport.annotations.len() as u64 + 1;
                                    let style = match this.ribbon.active_tool {
                                        super::ribbon::AnnotationTool::Highlight => {
                                            crate::models::AnnotationStyle::Highlight {
                                                color: "#FFF000".to_string(),
                                            }
                                        }
                                        super::ribbon::AnnotationTool::Underline => {
                                            crate::models::AnnotationStyle::Line {
                                                color: "#2563EB".to_string(),
                                                thickness: 2.0,
                                            }
                                        }
                                        super::ribbon::AnnotationTool::Strikeout => {
                                            crate::models::AnnotationStyle::Line {
                                                color: "#DC2626".to_string(),
                                                thickness: 2.0,
                                            }
                                        }
                                        super::ribbon::AnnotationTool::Rectangle => {
                                            crate::models::AnnotationStyle::Rectangle {
                                                color: "#2563EB".to_string(),
                                                thickness: 2.0,
                                                fill: false,
                                            }
                                        }
                                        super::ribbon::AnnotationTool::Circle => {
                                            crate::models::AnnotationStyle::Circle {
                                                color: "#2563EB".to_string(),
                                                thickness: 2.0,
                                                fill: false,
                                            }
                                        }
                                        super::ribbon::AnnotationTool::Line => {
                                            crate::models::AnnotationStyle::Line {
                                                color: "#000000".to_string(),
                                                thickness: 2.0,
                                            }
                                        }
                                        super::ribbon::AnnotationTool::Arrow => {
                                            crate::models::AnnotationStyle::Arrow {
                                                color: "#000000".to_string(),
                                                thickness: 2.0,
                                            }
                                        }
                                        super::ribbon::AnnotationTool::StickyNote => {
                                            crate::models::AnnotationStyle::StickyNote {
                                                comment: "Note".to_string(),
                                                color: "#FEF08A".to_string(),
                                            }
                                        }
                                        super::ribbon::AnnotationTool::Redact => {
                                            crate::models::AnnotationStyle::Redact {
                                                color: "#000000".to_string(),
                                            }
                                        }
                                        _ => crate::models::AnnotationStyle::Highlight {
                                            color: "#FFF000".to_string(),
                                        },
                                    };

                                    this.viewport.annotations.push(crate::models::Annotation {
                                        id: ann_id,
                                        page: page_idx,
                                        style,
                                        x: min_x,
                                        y: min_y,
                                        width: w,
                                        height: h,
                                    });
                                    if let Some(tab) =
                                        this.tabs.tabs.get_mut(this.tabs.active_tab_index)
                                    {
                                        tab.is_modified = true;
                                    }
                                }
                            }
                        }
                        cx.notify();
                    }
                },
                |this, event, _, cx| {
                    this.handle_canvas_scroll_wheel(event, cx);
                },
            );

            let status_bar = self.render_status_bar(cx);

            div()
                .flex()
                .flex_row()
                .flex_1()
                .w_full()
                .overflow_hidden()
                .child(sidebar)
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .flex_1()
                        .h_full()
                        .overflow_hidden()
                        .child(div().flex_1().w_full().overflow_hidden().child(canvas))
                        .child(status_bar),
                )
                .into_any_element()
        } else {
            self.welcome
                .render(cx, |this, action, _, cx| {
                    match action {
                        WelcomeAction::OpenFile => {
                            this.prompt_open_file(cx);
                        }
                        WelcomeAction::DropFiles(paths) => {
                            let mut opened = false;
                            for path in paths {
                                this.open_pdf_path(&path.to_string_lossy(), cx);
                                opened = true;
                            }
                            if opened {
                                cx.notify();
                            }
                        }
                        WelcomeAction::MergeFiles => {
                            this.prompt_merge_files(cx);
                        }
                        WelcomeAction::PageOrganizer => {
                            this.dialogs.active = Some(ActiveDialog::PageOrganizer);
                        }
                        WelcomeAction::DigitalSignatures => {
                            this.dialogs.active = Some(ActiveDialog::Signature);
                        }
                        WelcomeAction::Security => {
                            this.dialogs.active = Some(ActiveDialog::Security);
                        }
                        WelcomeAction::OpenRecent(idx) => {
                            if let Some(path) = this.welcome.recent_files.get(idx).cloned() {
                                this.open_pdf_path(&path.to_string_lossy(), cx);
                            }
                        }
                    }
                    cx.notify();
                })
                .into_any_element()
        };

        // Bottom Status Bar
        let status_message_str = self.status_message.clone();
        let status_bar = div()
            .flex()
            .flex_row()
            .items_center()
            .justify_between()
            .h_6()
            .px_3()
            .bg(muted)
            .border_t_1()
            .border_color(border)
            .text_xs()
            .text_color(muted_fg)
            .child(
                div()
                    .flex()
                    .flex_row()
                    .gap_3()
                    .child(format!(
                        "Page {} of {}",
                        self.viewport.current_page + 1,
                        self.viewport.total_pages
                    ))
                    .child(format!(
                        "Zoom: {}%",
                        (self.viewport.zoom * 100.0).round() as u32
                    ))
                    .child(format!("Mode: {:?}", self.ribbon.layout_mode))
                    .children(
                        status_message_str
                            .map(|msg| div().text_color(primary).font_medium().child(msg)),
                    ),
            )
            .child(concat!("PDFbull GPUI Core ", env!("CARGO_PKG_VERSION")));

        // Log Console Drawer (if open)
        let log_drawer = self.log_console.render(cx, |this, action, _, cx| {
            match action {
                LogAction::ToggleOpen => {
                    this.log_console.is_open = !this.log_console.is_open;
                }
                LogAction::Clear => {
                    this.log_console.entries.clear();
                }
                LogAction::CopyAll => {
                    let text = this
                        .log_console
                        .entries
                        .iter()
                        .map(|(msg, _)| msg.as_str())
                        .collect::<Vec<_>>()
                        .join("\n");
                    cx.write_to_clipboard(ClipboardItem::new_string(text));
                    this.status_message = Some("Logs copied to clipboard.".into());
                }
                LogAction::SetFilter(lvl) => {
                    if lvl == "all" {
                        this.log_console.filter_level = None;
                    } else {
                        this.log_console.filter_level = Some(lvl);
                    }
                }
            }
            cx.notify();
        });

        // Dialog Overlay (if active)
        let dialog_overlay = self.dialogs.render_overlay(
            cx,
            |this, _, cx| {
                this.dialogs.active = None;
                cx.notify();
            },
            |this, action, _, cx| {
                this.dialogs.active = None;
                match action {
                    DialogAction::SetWatermark(text) => {
                        this.apply_watermark(&text, cx);
                    }
                    DialogAction::SetHeaderFooter(h, f) => {
                        this.apply_header_footer(&h, &f, cx);
                    }
                    DialogAction::SetPassword(_pwd) => {}
                    DialogAction::RotatePages(angle) => {
                        this.rotate_active_pages(angle, cx);
                    }
                    DialogAction::DeleteCurrentPage => {
                        this.delete_current_page(cx);
                    }
                    DialogAction::ApplySecurity(algo) => {
                        this.apply_security(&algo, cx);
                    }
                    DialogAction::Close => {}
                }
                cx.notify();
            },
        );

        // Assemble Top-level View
        div()
            .track_focus(&self.focus_handle)
            .key_context("pdfbull_main")
            .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, _, cx| {
                this.handle_key_down(event, cx);
            }))
            .flex()
            .flex_col()
            .size_full()
            .relative()
            .bg(bg)
            .text_color(fg)
            .on_drop(cx.listener(|this, paths: &ExternalPaths, _, cx| {
                let mut opened = false;
                for path in paths.paths() {
                    if path.to_string_lossy().to_lowercase().ends_with(".pdf") || path.is_file() {
                        this.open_pdf_path(&path.to_string_lossy(), cx);
                        opened = true;
                    }
                }
                if opened {
                    cx.notify();
                }
            }))
            .child(header_bar)
            .child(action_strip)
            .children(tabs_bar)
            .child(div().flex_1().w_full().overflow_hidden().child(center_area))
            .children(log_drawer)
            .child(status_bar)
            .children(dialog_overlay)
    }
}
