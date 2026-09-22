use gpui_kit::component::ActiveTheme;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::*;

use super::{
    canvas::DocumentViewport,
    dialogs::{ActiveDialog, DialogsState},
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
                    path_str_cloned,
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
                let start = cur.saturating_sub(4);
                let end = (cur + 8).min(total - 1);
                (start..=end).collect()
            }
        };

        for page_idx in pages_to_render {
            self.request_render_page(doc_id, page_idx, cx);
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

        cx.spawn(async move |this, cx| {
            let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();
            // 1.5x scale multiplier ensures crisp text and graphics on HiDPI displays
            let scale = (zoom * 1.5).clamp(0.75, 3.0);
            let options = crate::pdf_engine::RenderOptions {
                scale,
                rotation,
                filter: crate::pdf_engine::RenderFilter::None,
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

                    if let Some(buf) = image::RgbaImage::from_raw(
                        render_res.width,
                        render_res.height,
                        raw_data,
                    ) {
                        let frame = image::Frame::new(buf);
                        let render_image = std::sync::Arc::new(RenderImage::new(vec![frame]));

                        let _ = this.update(cx, |view, cx| {
                            view.rendering_pages.remove(&page_idx);
                            if view.active_doc_id == Some(doc_id) {
                                view.viewport
                                    .rendered_pages
                                    .insert(page_idx, render_image.clone());
                                view.viewport.rendered_zoom.insert(page_idx, zoom);
                                view.sidebar.thumbnails.insert(page_idx, render_image);
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
                .send(crate::commands::PdfCommand::GetTextItems(doc_id, page_idx, resp_tx))
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

        let min_x = ((start.x.min(end.x)) / px(1.0)) / zoom;
        let max_x = ((start.x.max(end.x)) / px(1.0)) / zoom;
        let min_y = ((start.y.min(end.y)) / px(1.0)) / zoom;
        let max_y = ((start.y.max(end.y)) / px(1.0)) / zoom;

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

    pub fn handle_canvas_scroll_wheel(
        &mut self,
        event: &ScrollWheelEvent,
        cx: &mut Context<Self>,
    ) {
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
                RibbonAction::SaveFile => {}
                RibbonAction::Print => {}
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
                        this.render_needed_pages(cx);
                    }
                }
                RibbonAction::NextPage => {
                    if this.viewport.current_page + 1 < this.viewport.total_pages {
                        this.viewport.current_page += 1;
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
                }
                RibbonAction::SelectTool(tool) => {
                    this.ribbon.active_tool = tool;
                }
                RibbonAction::SelectColor(color) => {
                    this.ribbon.active_color = color;
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
                RibbonAction::Merge => {}
                RibbonAction::Split => {}
                RibbonAction::Flatten => {}
                RibbonAction::Encrypt => {
                    this.dialogs.active = Some(ActiveDialog::Security);
                }
                RibbonAction::Decrypt => {
                    this.dialogs.active = Some(ActiveDialog::Password);
                }
                RibbonAction::Compress => {}
                RibbonAction::Ocr => {}
                RibbonAction::PageOrganizer => {
                    this.dialogs.active = Some(ActiveDialog::PageOrganizer);
                }
                RibbonAction::ConvertMarkdown
                | RibbonAction::ConvertHtml
                | RibbonAction::ConvertText => {}
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

            let sidebar = self
                .sidebar
                .render(cx, total, current, |this, action, _, cx| {
                    match action {
                        SidebarAction::ToggleOpen => {
                            this.sidebar.is_open = !this.sidebar.is_open;
                        }
                        SidebarAction::SelectMode(mode) => {
                            this.sidebar.mode = mode;
                        }
                        SidebarAction::SelectPage(page_idx) => {
                            this.viewport.current_page = page_idx;
                            this.viewport.scroll_handle.scroll_to_item(page_idx);
                            this.render_needed_pages(cx);
                        }
                    }
                    cx.notify();
                });

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
                    if this.viewport.is_selecting && this.viewport.selection_page == Some(page_idx) {
                        this.viewport.selection_end = Some(pos);
                        cx.notify();
                    }
                },
                |this, page_idx, _, cx| {
                    if this.viewport.is_selecting && this.viewport.selection_page == Some(page_idx) {
                        this.viewport.is_selecting = false;
                        if let (Some(start), Some(end)) = (this.viewport.selection_start, this.viewport.selection_end) {
                            let dx = (start.x - end.x).abs();
                            let dy = (start.y - end.y).abs();
                            if dx < px(4.0) && dy < px(4.0) {
                                this.viewport.selection_start = None;
                                this.viewport.selection_end = None;
                                this.viewport.selection_page = None;
                                this.viewport.selected_text = None;
                            } else {
                                this.extract_selected_text(page_idx, start, end, cx);
                            }
                        }
                        cx.notify();
                    }
                },
                |this, event, _, cx| {
                    this.handle_canvas_scroll_wheel(event, cx);
                },
            );

            div()
                .flex()
                .flex_row()
                .flex_1()
                .w_full()
                .overflow_hidden()
                .child(sidebar)
                .child(div().flex_1().h_full().overflow_hidden().child(canvas))
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
                            this.prompt_open_file(cx);
                        }
                        WelcomeAction::PageOrganizer => {
                            this.prompt_open_file(cx);
                            this.dialogs.active = Some(ActiveDialog::PageOrganizer);
                        }
                        WelcomeAction::DigitalSignatures => {
                            this.prompt_open_file(cx);
                            this.dialogs.active = Some(ActiveDialog::Signature);
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
                    .child(format!("Mode: {:?}", self.ribbon.layout_mode)),
            )
            .child(concat!("PDFbull GPUI Core ", env!("CARGO_PKG_VERSION")));

        // Log Console Drawer (if open)
        let log_drawer = self.log_console.render(cx, |this, action, _, cx| {
            match action {
                LogAction::ToggleOpen => {
                    this.log_console.is_open = !this.log_console.is_open;
                }
                LogAction::Clear => {}
                LogAction::CopyAll => {}
                LogAction::SetFilter(lvl) => {
                    this.log_console.filter_level = Some(lvl);
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
            |this, _action, _, cx| {
                this.dialogs.active = None;
                cx.notify();
            },
        );

        // Assemble Top-level View
        div()
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
