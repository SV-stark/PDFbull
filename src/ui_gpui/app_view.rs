use gpui_kit::base::StyledExt;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::input::InputEvent;
use gpui_kit::component::{ActiveTheme, Disableable as _, IconName, Sizable};
use gpui_kit::prelude::FluentBuilder;
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
    /// Annotations for every open document, keyed by `DocumentId`.
    ///
    /// The viewport holds only the *active* document's annotations, and
    /// switching tabs calls `reset_for_document`, which clears them. Nothing
    /// re-loaded them: `LoadAnnotations` was only ever sent from
    /// `open_pdf_path`. So highlight a page, switch to another tab, switch
    /// back, and the annotations were gone from the canvas — and pressing
    /// Ctrl+S then sent an *empty* list, which made the engine delete every
    /// `/PDFBULL:` annotation it had previously written and save nothing back.
    /// The user's saved highlights were erased from their file while the status
    /// bar reported success. Caching per document makes a tab switch lossless,
    /// and a document activated without a cached set is re-read from disk.
    pub annotations_by_doc:
        std::collections::HashMap<crate::models::DocumentId, Vec<crate::models::Annotation>>,
    /// Page count and page size for every open document.
    ///
    /// `reset_for_document` deliberately leaves `total_pages` alone (it is
    /// document data, not per-document *view* state), but that left the viewport
    /// reporting the geometry of whichever document was opened **last**. The
    /// "do we need to re-derive?" guard could never fire again, because
    /// `DocumentViewport::new` seeds `total_pages = 1` and `page_width = 595`.
    ///
    /// So switching from a 2-page tab to a 10-page tab left `total_pages = 2`:
    /// the Continuous strip laid out only two pages, the content fit the
    /// viewport, `max_offset` stayed 0 — and therefore no scrollbar was drawn
    /// and the wheel handler clamped the offset to 0. Continuous mode stopped
    /// scrolling entirely, and the thumbnail panel lost its scrollbar too.
    ///
    /// Geometry is expensive to derive (`inspect_pdf_geometry` parses the whole
    /// file, which froze the UI for seconds per switch), so it is cached per
    /// document rather than re-read on activation.
    pub doc_geometry: std::collections::HashMap<crate::models::DocumentId, DocGeometry>,
    /// Keeps `sidebar.search_query` in sync with the sidebar's search editor.
    _search_sub: Option<Subscription>,
}

/// Upper bound on in-document search hits kept in memory and painted.
/// The engine returns every occurrence, which for a common word in a large
/// document is tens of thousands of entries feeding both the sidebar list and
/// the highlight overlay.
const MAX_SEARCH_RESULTS: usize = 5_000;

/// Page count and page size of a document, cached per `DocumentId`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DocGeometry {
    pub page_count: usize,
    pub page_width: f32,
    pub page_height: f32,
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
        let sidebar = SidebarState::new(_window, _cx);
        // Mirror the search editor's contents into `sidebar.search_query`.
        // `SidebarState` is plain data rather than an `Entity`, so the
        // subscription lives here on the view that owns it.
        let _search_sub = sidebar.search_input.as_ref().map(|input| {
            _cx.subscribe(input, |this, _, ev: &InputEvent, cx| {
                if matches!(ev, InputEvent::Change) {
                    this.sidebar.on_search_input_changed(cx);
                    cx.notify();
                }
            })
        });

        Self {
            engine: crate::engine::spawn_engine_thread(64, 512),
            active_doc_id: None,
            rendering_pages: std::collections::HashSet::new(),
            ribbon: RibbonState::new(),
            tabs: TabsState::new(),
            sidebar,
            viewport: DocumentViewport::new(),
            dialogs: DialogsState::new_in(_window, _cx),
            welcome,
            log_console: LogConsoleState::new(),
            focus_handle,
            status_message: None,
            annotations_by_doc: std::collections::HashMap::new(),
            doc_geometry: std::collections::HashMap::new(),
            _search_sub,
        }
    }

    /// Record the active document's current annotations so a tab switch can put
    /// them back. Call *before* `reset_for_document` clears the viewport.
    fn stash_active_annotations(&mut self) {
        if let Some(doc_id) = self.active_doc_id {
            self.annotations_by_doc
                .insert(doc_id, self.viewport.annotations.clone());
        }
    }

    /// Restore a previously stashed annotation set for `doc_id`.
    ///
    /// Returns `false` when nothing is cached, which is the caller's signal to
    /// re-read the annotations from the file.
    fn restore_annotations_for(&mut self, doc_id: crate::models::DocumentId) -> bool {
        match self.annotations_by_doc.get(&doc_id).cloned() {
            Some(annotations) => {
                self.viewport.annotations = annotations;
                self.viewport.reindex_annotations();
                true
            }
            None => false,
        }
    }

    /// Ask the engine for a document's saved annotations and adopt the result.
    ///
    /// Used both on open and on tab re-activation, so a document that has no
    /// cached set is never left silently annotation-free.
    fn request_annotations(
        &mut self,
        doc_id: crate::models::DocumentId,
        path: std::path::PathBuf,
        cx: &mut Context<Self>,
    ) {
        let engine_tx = self.engine.cmd_tx.clone();
        let path_str = path.to_string_lossy().to_string();
        cx.spawn(async move |this, cx| {
            let (tx, rx) = tokio::sync::oneshot::channel();
            if engine_tx
                .send(crate::commands::PdfCommand::LoadAnnotations(
                    doc_id, path_str, tx,
                ))
                .await
                .is_err()
            {
                let _ = this.update(cx, |view, cx| {
                    view.report_engine_unavailable("Loading annotations", cx);
                });
                return;
            }
            let Ok(Ok(loaded)) = rx.await else {
                return;
            };
            let _ = this.update(cx, |view, cx| {
                // Guard on doc: a response for a document the user has since
                // switched away from must not land in the viewport.
                if view.active_doc_id != Some(doc_id) {
                    // Still cache it, keyed by its own document.
                    view.annotations_by_doc.insert(doc_id, loaded);
                    return;
                }
                // Only replace when the document has nothing yet. An
                // unconditional assignment threw away a highlight drawn while
                // this request was in flight.
                if !view.viewport.annotations.is_empty() {
                    return;
                }
                view.annotations_by_doc.insert(doc_id, loaded.clone());
                view.viewport.annotations = loaded;
                view.viewport.reindex_annotations();
                cx.notify();
            });
        })
        .detach();
    }

    pub fn open_pdf_path(&mut self, path_str: &str, cx: &mut Context<Self>) {
        let path = std::path::PathBuf::from(path_str);
        // Reject anything that isn't a readable regular PDF. Previously this
        // only checked `exists()`, so dropping a PNG (or passing a directory
        // on the command line) created a tab that could never render and
        // reported nothing but a `tracing::error!` the user never sees.
        if !SidebarState::is_openable_pdf(&path) {
            self.status_message = Some(format!("Skipped {}: not a PDF file.", path.display()));
            tracing::warn!("Refusing to open non-PDF path: {}", path.display());
            cx.notify();
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

        crate::storage::add_recent_file(&path);
        self.welcome.recent_files = crate::storage::load_recent_files()
            .into_iter()
            .map(|f| std::path::PathBuf::from(f.path))
            .filter(|p| p.is_file())
            .collect();

        let doc_id = crate::models::next_doc_id();
        self.active_doc_id = Some(doc_id);
        self.doc_geometry.insert(
            doc_id,
            DocGeometry {
                page_count,
                page_width,
                page_height,
            },
        );

        let new_id = self.tabs.tabs.len();
        self.tabs.push(DocumentTab {
            id: 0,
            doc_id: Some(doc_id),
            title,
            path: Some(path.clone()),
            is_modified: false,
        });
        self.tabs.active_tab_index = new_id;
        // `reset_for_document` drops every scrap of the *previous* document:
        // rendered pages, the zoom map, the text layer, search hits, cached
        // page origins, annotations, and the scroll offset. Previously only the
        // first three were cleared, so opening B after A showed A's highlights
        // and A's text — and Ctrl+S then wrote A's annotations into B's file.
        self.viewport.reset_for_document();
        self.viewport.page_width = page_width;
        self.viewport.page_height = page_height;
        self.viewport.total_pages = page_count;
        self.viewport.current_page = 0;
        self.sidebar.thumbnails.clear();
        self.sidebar.bookmarks.clear();
        self.sidebar.attachments.clear();
        self.sidebar.layers.clear();
        self.sidebar.reset_search(cx);
        self.rendering_pages.clear();
        self.dialogs.signatures.clear();

        let engine_tx = self.engine.cmd_tx.clone();
        let path_str_cloned = path.to_string_lossy().to_string();

        cx.spawn(async move |this, cx| {
            let (tx, rx) = tokio::sync::oneshot::channel();
            if let Err(e) = engine_tx
                .send(crate::commands::PdfCommand::Open(
                    path_str_cloned.clone(),
                    None,
                    doc_id,
                    tx,
                ))
                .await
            {
                tracing::error!("Failed to send Open command: {e}");
                let _ = this.update(cx, |view, cx| {
                    if view.active_doc_id == Some(doc_id) {
                        view.status_message =
                            Some("Engine is unavailable — cannot open the document.".into());
                        cx.notify();
                    }
                });
                return;
            }
            match rx.await {
                Ok(Ok(open_res)) => {
                    let _ = this.update(cx, |view, cx| {
                        // The engine's numbers are authoritative and refine the
                        // `lopdf` estimate, so record them in the per-document
                        // cache — otherwise switching away and back would restore
                        // the rougher estimate instead.
                        let page_height = open_res
                            .page_heights
                            .first()
                            .copied()
                            .filter(|h| *h > 0.0)
                            .unwrap_or(view.viewport.page_height);
                        let page_width = if open_res.max_width > 0.0 {
                            open_res.max_width
                        } else {
                            view.viewport.page_width
                        };
                        view.doc_geometry.insert(
                            doc_id,
                            DocGeometry {
                                page_count: open_res.page_count,
                                page_width,
                                page_height,
                            },
                        );
                        if view.active_doc_id == Some(doc_id) {
                            view.viewport.total_pages = open_res.page_count;
                            if open_res.max_width > 0.0 {
                                view.viewport.page_width = open_res.max_width;
                            }
                            if let Some(&first_h) = open_res.page_heights.first()
                                && first_h > 0.0
                            {
                                view.viewport.page_height = first_h;
                            }
                            view.dialogs.signatures = open_res.signatures;
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
                            // Guard: without this, a response for a document the
                            // user has since switched away from overwrites the
                            // sidebar of whatever is on screen now.
                            if view.active_doc_id != Some(doc_id) {
                                return;
                            }
                            view.sidebar.bookmarks = meta.outline;
                            view.sidebar.attachments = meta.attachments;
                            view.sidebar.layers = meta.layers;
                            view.dialogs.signatures = meta.signatures;
                            cx.notify();
                        });
                    }

                    // Load previously saved annotations. This used to be inline
                    // here and nowhere else, which is why a *tab switch* never
                    // restored them; `request_annotations` is shared with the
                    // tab-activation path.
                    let ann_path = std::path::PathBuf::from(&path_str_cloned);
                    let _ = this.update(cx, |view, cx| {
                        view.request_annotations(doc_id, ann_path.clone(), cx);
                    });
                }
                Ok(Err(e)) => {
                    tracing::error!("Failed to open PDF in engine worker: {e:?}");
                    let _ = this.update(cx, |view, cx| {
                        if view.active_doc_id == Some(doc_id) {
                            view.status_message = Some(format!("Failed to open PDF: {e}"));
                            cx.notify();
                        }
                    });
                }
                Err(e) => {
                    tracing::error!("Open response channel dropped: {e}");
                    let _ = this.update(cx, |view, cx| {
                        if view.active_doc_id == Some(doc_id) {
                            view.status_message =
                                Some("The engine stopped responding while opening.".into());
                            cx.notify();
                        }
                    });
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
        // Detach from `self.tabs` up front: the work below mutates `self`, and
        // holding a borrow into the tab list across those calls will not
        // borrow-check.
        let Some((doc_id, path)) = self
            .tabs
            .tabs
            .get(self.tabs.active_tab_index)
            .map(|t| (t.doc_id, t.path.clone()))
        else {
            return;
        };
        if self.active_doc_id != doc_id {
            // Keep the outgoing document's annotations before the reset wipes
            // them. Without this, returning to a tab showed an empty canvas and
            // the next Ctrl+S sent an empty list, which deleted every
            // previously saved annotation from the file.
            self.stash_active_annotations();
            self.active_doc_id = doc_id;
            // Full per-document reset. The old code cleared only the render
            // caches, so switching tabs leaked the previous document's text
            // layer, search hits, page origins, scroll offset and — worst of
            // all — its annotations, which then got saved into the new
            // document's file.
            self.viewport.reset_for_document();
            self.sidebar.thumbnails.clear();
            self.sidebar.bookmarks.clear();
            self.sidebar.attachments.clear();
            self.sidebar.layers.clear();
            self.sidebar.reset_search(cx);
            self.rendering_pages.clear();
            self.dialogs.signatures.clear();

            // Put this document's annotations back, or re-read them if it has
            // no cached set — a document must never be left silently
            // annotation-free.
            if let Some(doc_id) = doc_id
                && !self.restore_annotations_for(doc_id)
                && let Some(path) = path.clone()
            {
                self.request_annotations(doc_id, path, cx);
            }
        }
        // Restore *this* document's geometry. The previous guard
        // (`total_pages == 0 || page_width <= 0.0`) could never be true once a
        // document had been opened, so the viewport kept whichever document was
        // opened last — leaving the strip too short to scroll. Cached geometry
        // makes activation cheap; a document with no cached entry is derived
        // once and then remembered, so the expensive whole-file parse still
        // happens at most once per document rather than on every switch.
        if let Some(doc_id) = doc_id {
            let geometry = match self.doc_geometry.get(&doc_id).copied() {
                Some(g) => Some(g),
                None => path.as_deref().and_then(inspect_pdf_geometry).map(
                    |(page_count, page_width, page_height)| {
                        let g = DocGeometry {
                            page_count,
                            page_width,
                            page_height,
                        };
                        self.doc_geometry.insert(doc_id, g);
                        g
                    },
                ),
            };
            if let Some(g) = geometry {
                self.viewport.total_pages = g.page_count;
                self.viewport.page_width = g.page_width;
                self.viewport.page_height = g.page_height;
                // A document with fewer pages than the one we came from would
                // otherwise leave `current_page` past the end.
                if self.viewport.current_page >= self.viewport.total_pages {
                    self.viewport.current_page = self.viewport.total_pages.saturating_sub(1);
                }
            }
        }
        self.render_needed_pages(cx);
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
                            if view.active_doc_id == Some(doc_id) {
                                view.rendering_pages.remove(&page_idx);
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
                                view.viewport
                                    .rendered_pages
                                    .retain(|&p, _| p >= keep_start && p <= keep_end);
                                view.viewport
                                    .rendered_zoom
                                    .retain(|&p, _| p >= keep_start && p <= keep_end);

                                // Evict text cache outside a wider ±10 window so that
                                // text selection on recently visited pages still works
                                // without a round-trip, while bounding peak RAM.
                                let tc_start = cur.saturating_sub(10);
                                let tc_end = cur + 10;
                                view.viewport
                                    .text_cache
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
                            if view.active_doc_id == Some(doc_id) {
                                view.rendering_pages.remove(&page_idx);
                            }
                        });
                    }
                }
                Ok(Err(e)) => {
                    tracing::error!("Render page {page_idx} error: {e:?}");
                    let _ = this.update(cx, |view, _| {
                        if view.active_doc_id == Some(doc_id) {
                            view.rendering_pages.remove(&page_idx);
                        }
                    });
                }
                Err(e) => {
                    tracing::error!("Render response dropped: {e}");
                    let _ = this.update(cx, |view, _| {
                        if view.active_doc_id == Some(doc_id) {
                            view.rendering_pages.remove(&page_idx);
                        }
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
                    // Guard on doc: `text_cache` is keyed by page index only, so
                    // an unguarded write let document A's text layer answer
                    // selection requests (and paint word highlights) for
                    // document B.
                    if view.active_doc_id == Some(doc_id) {
                        view.viewport.text_cache.insert(page_idx, items);
                    }
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
                if let Some(buf) =
                    image::RgbaImage::from_raw(render_res.width, render_res.height, raw_data)
                {
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

    /// Fetch the active document's AcroForm fields and report them.
    ///
    /// The "Form Fields" ribbon button was wired to an empty match arm, so it
    /// looked interactive but did nothing at all.
    fn list_form_fields(&mut self, cx: &mut Context<Self>) {
        let Some(tab) = self.tabs.tabs.get(self.tabs.active_tab_index) else {
            return;
        };
        let Some(path) = tab.path.clone() else {
            self.status_message = Some("Open a document first to list form fields.".into());
            cx.notify();
            return;
        };
        let path_str = path.to_string_lossy().to_string();
        let cmd_tx = self.engine.cmd_tx.clone();

        cx.spawn(async move |this, cx| {
            let (tx, rx) = tokio::sync::oneshot::channel();
            if let Err(e) = cmd_tx
                .send(crate::commands::PdfCommand::GetFormFields(
                    path_str.clone(),
                    tx,
                ))
                .await
            {
                tracing::error!("Failed to send GetFormFields command: {e}");
            }
            let outcome = match rx.await {
                Ok(Ok(fields)) => Some(fields),
                Ok(Err(e)) => {
                    tracing::error!("GetFormFields failed: {e:?}");
                    None
                }
                Err(e) => {
                    tracing::error!("GetFormFields response dropped: {e}");
                    None
                }
            };
            let _ = this.update(cx, |view, cx| {
                match outcome {
                    Some(fields) if fields.is_empty() => {
                        view.status_message =
                            Some("This document has no interactive form fields.".into());
                        view.log_console
                            .log("[INFO] No AcroForm fields found.", "info");
                    }
                    Some(fields) => {
                        view.log_console.log(
                            format!("[INFO] {} form field(s) found:", fields.len()),
                            "info",
                        );
                        for f in &fields {
                            let kind = match &f.variant {
                                crate::models::FormFieldVariant::Text { .. } => "text",
                                crate::models::FormFieldVariant::Checkbox { .. } => "checkbox",
                                crate::models::FormFieldVariant::RadioButton { .. } => "radio",
                                crate::models::FormFieldVariant::ComboBox { .. } => "combo",
                            };
                            view.log_console.log(
                                format!("  - {} ({kind}, page {})", f.name, f.page + 1),
                                "info",
                            );
                        }
                        view.status_message =
                            Some(format!("Found {} form field(s).", fields.len()));
                    }
                    None => {
                        view.status_message = Some("Could not read form fields.".into());
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// Show or hide an optional-content layer.
    ///
    /// The toggle used to only flip a local flag, so nothing ever re-rendered
    /// and the change was purely cosmetic. Send the command to the engine *and*
    /// drop this side's cached bitmaps — the engine invalidates its own cache
    /// entries, but the viewport holds separate `Arc<RenderImage>`s that would
    /// otherwise keep showing the old layer state.
    fn set_layer_visibility(&mut self, idx: usize, visible: bool, cx: &mut Context<Self>) {
        let Some(doc_id) = self.active_doc_id else {
            return;
        };
        let Some(obj_id) = self.sidebar.layers.get(idx).map(|l| l.object_id) else {
            return;
        };
        if let Some(layer) = self.sidebar.layers.get_mut(idx) {
            layer.visible = visible;
        }

        let cmd_tx = self.engine.cmd_tx.clone();
        cx.spawn(async move |this, cx| {
            if let Err(e) = cmd_tx
                .send(crate::commands::PdfCommand::ToggleLayer(
                    doc_id, obj_id, visible,
                ))
                .await
            {
                tracing::error!("Failed to send ToggleLayer command: {e}");
            }
            let _ = this.update(cx, |view, cx| {
                if view.active_doc_id == Some(doc_id) {
                    view.viewport.invalidate_rendered_pages();
                    view.rendering_pages.clear();
                    view.render_needed_pages(cx);
                    cx.notify();
                }
            });
        })
        .detach();
    }

    /// Reconcile `viewport.current_page` with the scroll offset and request the
    /// pages that became visible.
    ///
    /// This is the single place the current page is derived from scroll
    /// position. It used to run inside `render`, which meant a speculative or
    /// discarded frame mutated the model and queued engine commands; it is now
    /// driven by the wheel handler and the canvas mouse-move handler instead.
    fn sync_current_page_from_scroll(&mut self, cx: &mut Context<Self>) {
        if self.viewport.layout_mode != super::ribbon::PageLayoutMode::Continuous
            || self.viewport.total_pages == 0
        {
            return;
        }
        let visible_page = self.viewport.calculate_visible_page_continuous();
        if visible_page != self.viewport.current_page {
            self.viewport.current_page = visible_page;
            self.render_needed_pages(cx);
            cx.notify();
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
            let (mut delta_x, delta_y) = match event.delta {
                ScrollDelta::Pixels(p) => (p.x, p.y),
                ScrollDelta::Lines(l) => (px(l.x * 60.0), px(l.y * 60.0)),
            };

            // Support Shift + wheel for horizontal scroll on platforms/mice where Shift does not map directly to delta_x
            if event.modifiers.shift && delta_x.abs() < px(0.01) && delta_y.abs() > px(0.01) {
                delta_x = delta_y;
            }

            // Handle horizontal scroll if content overflows horizontally
            let max_offset_x = self.viewport.scroll_handle.max_offset().x;
            if delta_x.abs() > px(0.1) && max_offset_x > px(0.0) {
                let mut offset = self.viewport.scroll_handle.offset();
                offset.x = (offset.x + delta_x).clamp(-max_offset_x, px(0.0));
                self.viewport.scroll_handle.set_offset(offset);
                cx.notify();
            }

            // If Shift was held for horizontal scrolling, do not also scroll vertically
            if event.modifiers.shift && delta_x.abs() > px(0.01) {
                return;
            }

            match self.viewport.layout_mode {
                super::ribbon::PageLayoutMode::SinglePage => {
                    let max_offset_y = self.viewport.scroll_handle.max_offset().y;
                    let mut offset = self.viewport.scroll_handle.offset();
                    let dy_f = delta_y / px(1.0);

                    // When page overflows vertically (e.g. zoomed in), scroll smoothly within the page first
                    if dy_f < 0.0 && offset.y > -max_offset_y + px(1.0) {
                        offset.y = (offset.y + delta_y).clamp(-max_offset_y, px(0.0));
                        self.viewport.scroll_handle.set_offset(offset);
                        cx.notify();
                    } else if dy_f < -5.0
                        && self.viewport.current_page + 1 < self.viewport.total_pages
                    {
                        self.viewport.scroll_to_page(self.viewport.current_page + 1);
                        self.render_needed_pages(cx);
                        cx.notify();
                    } else if dy_f > 0.0 && offset.y < px(-1.0) {
                        offset.y = (offset.y + delta_y).clamp(-max_offset_y, px(0.0));
                        self.viewport.scroll_handle.set_offset(offset);
                        cx.notify();
                    } else if dy_f > 5.0 && self.viewport.current_page > 0 {
                        self.viewport.scroll_to_page(self.viewport.current_page - 1);
                        self.render_needed_pages(cx);
                        cx.notify();
                    }
                }
                super::ribbon::PageLayoutMode::TwoPageSpread => {
                    let step = if self.viewport.standalone_cover && self.viewport.current_page == 0
                    {
                        1
                    } else {
                        2
                    };
                    let max_offset_y = self.viewport.scroll_handle.max_offset().y;
                    let mut offset = self.viewport.scroll_handle.offset();
                    let dy_f = delta_y / px(1.0);

                    if dy_f < 0.0 && offset.y > -max_offset_y + px(1.0) {
                        offset.y = (offset.y + delta_y).clamp(-max_offset_y, px(0.0));
                        self.viewport.scroll_handle.set_offset(offset);
                        cx.notify();
                    } else if dy_f < -5.0
                        && self.viewport.current_page + 1 < self.viewport.total_pages
                    {
                        let target =
                            (self.viewport.current_page + step).min(self.viewport.total_pages - 1);
                        self.viewport.scroll_to_page(target);
                        self.render_needed_pages(cx);
                        cx.notify();
                    } else if dy_f > 0.0 && offset.y < px(-1.0) {
                        offset.y = (offset.y + delta_y).clamp(-max_offset_y, px(0.0));
                        self.viewport.scroll_handle.set_offset(offset);
                        cx.notify();
                    } else if dy_f > 5.0 && self.viewport.current_page > 0 {
                        let target = self.viewport.current_page.saturating_sub(step);
                        self.viewport.scroll_to_page(target);
                        self.render_needed_pages(cx);
                        cx.notify();
                    }
                }
                super::ribbon::PageLayoutMode::Continuous => {
                    let total = self.viewport.total_pages;
                    if total > 0 {
                        // Clamp against the handle's real `max_offset`, which GPUI
                        // recomputes from the actual laid-out content size on
                        // every frame. The previous hand-rolled bound ignored the
                        // container padding and inter-page gaps, so it let the
                        // content be dragged past its end.
                        let min_scroll_y = -self.viewport.scroll_handle.max_offset().y;
                        let mut offset = self.viewport.scroll_handle.offset();
                        offset.y = (offset.y + delta_y).clamp(min_scroll_y, px(0.0));
                        self.viewport.scroll_handle.set_offset(offset);

                        self.sync_current_page_from_scroll(cx);
                        cx.notify();
                    }
                }
            }
        }
    }

    /// Record a command that never reached the engine.
    ///
    /// `cmd_tx` is a bounded 128-slot channel, and a `send` on a full channel
    /// fails. Every affected site used `let _ = cmd_tx.send(..)` followed by
    /// `if let Ok(Ok(..)) = rx.await`, so a dropped send fell straight through
    /// with no status message, no log entry and no dialog — the user clicked
    /// Apply, the modal closed, and the document was silently unchanged. A
    /// failed send has to be as visible as a failed command.
    fn report_engine_unavailable(&mut self, command: &str, cx: &mut Context<Self>) {
        tracing::error!("Could not queue {command}: the engine channel is full or closed");
        self.log_console.log(
            format!("[ERROR] {command} could not reach the engine."),
            "error",
        );
        self.status_message = Some(format!("{command} failed: the engine is unavailable."));
        cx.notify();
    }

    /// Report an engine error that came back from a completed command.
    fn report_engine_error(
        &mut self,
        command: &str,
        error: &dyn std::fmt::Display,
        cx: &mut Context<Self>,
    ) {
        tracing::error!("{command} failed: {error}");
        self.log_console
            .log(format!("[ERROR] {command} failed: {error}"), "error");
        self.status_message = Some(format!("{command} failed: {error}"));
        cx.notify();
    }

    fn render_status_bar(&self, cx: &mut Context<Self>) -> AnyElement {
        let border = cx.theme().border;
        let muted = cx.theme().muted;
        let fg = cx.theme().foreground;
        let muted_fg = cx.theme().muted_foreground;
        let primary = cx.theme().primary;
        let total = self.viewport.total_pages;
        let current = self.viewport.current_page;
        let zoom_pct = (self.viewport.zoom * 100.0).round() as u32;
        let page_w = self.viewport.page_width.round() as u32;
        let page_h = self.viewport.page_height.round() as u32;
        let has_doc = self.active_doc_id.is_some();

        let prev_click = cx.listener(|this, _, _, cx| {
            if this.viewport.current_page > 0 {
                this.viewport.scroll_to_page(this.viewport.current_page - 1);
                this.render_needed_pages(cx);
                cx.notify();
            }
        });

        let next_click = cx.listener(|this, _, _, cx| {
            if this.viewport.current_page + 1 < this.viewport.total_pages {
                this.viewport.scroll_to_page(this.viewport.current_page + 1);
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

        // Left group: document facts. Right group: page + zoom controls. The
        // transient status message sits in the middle and is allowed to take
        // the remaining space, so it never shifts the two anchored groups.
        //
        // `min_w_0()` on the status group is what lets it shrink instead of
        // pushing the page controls off the edge of a narrow window.
        div()
            .flex()
            .flex_row()
            .items_center()
            .justify_between()
            .gap_3()
            .h_7()
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
                    .flex_none()
                    .when(has_doc, |el| {
                        el.child(
                            div()
                                .text_xs()
                                .text_color(muted_fg)
                                .child(format!("{} × {} pt", page_w, page_h)),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(muted_fg)
                                .child(self.ribbon.layout_mode.label()),
                        )
                    })
                    .when(!has_doc, |el| {
                        el.child(div().text_xs().text_color(muted_fg).child("Ready"))
                    }),
            )
            .child(
                div()
                    .flex()
                    .flex_1()
                    .min_w_0()
                    .overflow_hidden()
                    .text_xs()
                    .text_color(primary)
                    .font_medium()
                    .truncate()
                    .when_some(self.status_message.clone(), |el, msg| el.child(msg)),
            )
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_2()
                    .flex_none()
                    .child(
                        Button::new("btn-status-prev")
                            .icon(IconName::ChevronLeft)
                            .ghost()
                            .small()
                            .disabled(!has_doc || current == 0)
                            .tooltip("Previous page")
                            .on_click(prev_click),
                    )
                    // Reserve a fixed lane so the readout does not shift as the
                    // page count grows from 9 to 10.
                    .child(
                        div()
                            .min_w(px(112.0))
                            .text_xs()
                            .font_medium()
                            .text_color(fg)
                            .child(if has_doc {
                                format!("Page {} of {}", current + 1, total)
                            } else {
                                "No document".to_string()
                            }),
                    )
                    .child(
                        Button::new("btn-status-next")
                            .icon(IconName::ChevronRight)
                            .ghost()
                            .small()
                            .disabled(!has_doc || current + 1 >= total)
                            .tooltip("Next page")
                            .on_click(next_click),
                    )
                    .child(div().w_px().h(px(12.0)).bg(border))
                    .child(
                        Button::new("btn-status-zoom-out")
                            .icon(IconName::Minus)
                            .ghost()
                            .small()
                            .disabled(!has_doc)
                            .tooltip("Zoom out")
                            .on_click(zoom_out_click),
                    )
                    .child(
                        Button::new("btn-status-zoom-pct")
                            .label(SharedString::from(format!("{zoom_pct}%")))
                            .ghost()
                            .small()
                            .disabled(!has_doc)
                            .tooltip("Reset zoom to 100%")
                            .on_click(zoom_reset_click),
                    )
                    .child(
                        Button::new("btn-status-zoom-in")
                            .icon(IconName::Plus)
                            .ghost()
                            .small()
                            .disabled(!has_doc)
                            .tooltip("Zoom in")
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
            if let Err(e) = cmd_tx
                .send(crate::commands::PdfCommand::SaveAnnotations(
                    doc_id,
                    annotations,
                    tx,
                ))
                .await
            {
                tracing::error!("Failed to send SaveAnnotations command: {e}");
            }
            let outcome = match rx.await {
                Ok(res) => Some(res),
                Err(e) => {
                    tracing::error!("Save response dropped: {e}");
                    None
                }
            };
            let _ = this.update(cx, |view, cx| {
                // Match on `doc_id`, not the index captured before the await:
                // if a tab before it was closed meanwhile, that index now
                // addresses a different document and would clear the wrong
                // tab's unsaved-changes marker.
                let saved_path = match outcome {
                    Some(Ok(path)) => Some(path),
                    Some(Err(e)) => {
                        view.status_message = Some(format!("Save error: {e}"));
                        cx.notify();
                        return;
                    }
                    None => {
                        view.status_message = Some("Save failed: engine unavailable.".into());
                        cx.notify();
                        return;
                    }
                };
                if let Some(t) = view.tabs.tabs.iter_mut().find(|t| t.doc_id == Some(doc_id)) {
                    t.is_modified = false;
                }
                // Report where it actually landed — the engine may have chosen a
                // sidecar path if the document's own path was unresolvable.
                let name = std::path::Path::new(saved_path.as_deref().unwrap_or_default())
                    .file_name()
                    .map(|f| f.to_string_lossy().to_string())
                    .unwrap_or_default();
                view.status_message = Some(format!("Saved as {name}."));
                cx.notify();
            });
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
            if let Err(e) = cmd_tx
                .send(crate::commands::PdfCommand::ExportPdf(
                    doc_id,
                    out_path.clone(),
                    annotations,
                    tx,
                ))
                .await
            {
                tracing::error!("Failed to send ExportPdf command: {e}");
            }
            let outcome = match rx.await {
                Ok(res) => Some(res),
                Err(e) => {
                    tracing::error!("Export response dropped: {e}");
                    None
                }
            };
            let _ = this.update(cx, |view, cx| {
                match outcome {
                    Some(Ok(_)) => {
                        // Repoint the tab by `doc_id`: the captured index may no
                        // longer refer to this document after an await.
                        if let Some(t) =
                            view.tabs.tabs.iter_mut().find(|t| t.doc_id == Some(doc_id))
                        {
                            t.path = Some(std::path::PathBuf::from(&out_path));
                            t.title = std::path::Path::new(&out_path)
                                .file_name()
                                .map(|f| f.to_string_lossy().to_string())
                                .unwrap_or_else(|| "document.pdf".into());
                            t.is_modified = false;
                        }
                        view.status_message = Some("Exported PDF successfully.".into());
                    }
                    Some(Err(e)) => {
                        view.status_message = Some(format!("Export error: {e}"));
                    }
                    None => {
                        view.status_message = Some("Export failed: engine unavailable.".into());
                    }
                }
                cx.notify();
            });
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
        self.status_message = Some("Sending to printer spooler…".into());
        cx.notify();

        cx.spawn(async move |this, cx| {
            let (tx, rx) = tokio::sync::oneshot::channel();
            // The "Sending to printer spooler..." message is written before the
            // send. A dropped send used to leave it on screen forever, because
            // the send error was discarded and the closed receiver then fell
            // into an empty `Err(_)` arm.
            if let Err(e) = cmd_tx
                .send(crate::commands::PdfCommand::PrintPdf(path_str, None, tx))
                .await
            {
                tracing::error!("Failed to send PrintPdf command: {e}");
                let _ = this.update(cx, |view, cx| {
                    view.report_engine_unavailable("Printing", cx);
                });
                return;
            }
            match rx.await {
                Ok(Ok(())) => {
                    let _ = this.update(cx, |view, cx| {
                        view.status_message = Some("Sent to the printer.".into());
                        cx.notify();
                    });
                }
                Ok(Err(e)) => {
                    let _ = this.update(cx, |view, cx| {
                        view.report_engine_error("Printing", &e, cx);
                    });
                }
                Err(e) => {
                    tracing::error!("Print response dropped: {e}");
                    let _ = this.update(cx, |view, cx| {
                        view.report_engine_unavailable("Printing", cx);
                    });
                }
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
            if let Err(e) = cmd_tx
                .send(crate::commands::PdfCommand::Search(doc_id, query_str, tx))
                .await
            {
                tracing::error!("Failed to send Search command: {e}");
            }
            let outcome = match rx.await {
                Ok(Ok(results)) => Some(results),
                Ok(Err(e)) => {
                    tracing::error!("Search failed: {e:?}");
                    None
                }
                Err(e) => {
                    tracing::error!("Search response dropped: {e}");
                    None
                }
            };
            let _ = this.update(cx, |view, cx| {
                // Guard on doc: a search started in A used to land in B's
                // results and then scrolled B to A's first hit.
                if view.active_doc_id != Some(doc_id) {
                    return;
                }
                // Always clear the spinner. Previously it was only reset on
                // success, so any engine error left the panel stuck on
                // "Searching..." forever with no way out.
                view.sidebar.is_searching = false;
                let Some(results) = outcome else {
                    view.sidebar.search_results.clear();
                    view.viewport.search_highlights.clear();
                    view.status_message = Some("Search failed.".into());
                    cx.notify();
                    return;
                };
                // Bound the result set: the sidebar and the highlight overlay
                // both grow linearly with the hit count.
                let results: Vec<crate::models::SearchResultItem> =
                    results.into_iter().take(MAX_SEARCH_RESULTS).collect();
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
                    view.viewport.scroll_to_page(first.page_index);
                    view.render_needed_pages(cx);
                }
                cx.notify();
            });
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
            if let Err(e) = cmd_tx
                .send(crate::commands::PdfCommand::AddWatermark(
                    path_str,
                    text_str,
                    out_path.clone(),
                    tx,
                ))
                .await
            {
                tracing::error!("Failed to send AddWatermark command: {e}");
                let _ = this.update(cx, |view, cx| {
                    view.report_engine_unavailable("Adding the watermark", cx);
                });
                return;
            }

            match rx.await {
                Ok(Ok(_)) => {
                    let _ = this.update(cx, |view, cx| {
                        view.open_pdf_path(&out_path, cx);
                        view.status_message = Some("Watermark applied.".into());
                        cx.notify();
                    });
                }
                Ok(Err(e)) => {
                    let _ = this.update(cx, |view, cx| {
                        view.report_engine_error("Adding the watermark", &e, cx);
                    });
                }
                Err(e) => {
                    tracing::error!("AddWatermark response dropped: {e}");
                    let _ = this.update(cx, |view, cx| {
                        view.report_engine_unavailable("Adding the watermark", cx);
                    });
                }
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
            if let Err(e) = cmd_tx
                .send(crate::commands::PdfCommand::AddHeaderFooter(
                    path_str,
                    h_str,
                    f_str,
                    out_path.clone(),
                    tx,
                ))
                .await
            {
                tracing::error!("Failed to send AddHeaderFooter command: {e}");
                let _ = this.update(cx, |view, cx| {
                    view.report_engine_unavailable("Adding the header and footer", cx);
                });
                return;
            }

            match rx.await {
                Ok(Ok(_)) => {
                    let _ = this.update(cx, |view, cx| {
                        view.open_pdf_path(&out_path, cx);
                        view.status_message = Some("Header and footer applied.".into());
                        cx.notify();
                    });
                }
                Ok(Err(e)) => {
                    let _ = this.update(cx, |view, cx| {
                        view.report_engine_error("Adding the header and footer", &e, cx);
                    });
                }
                Err(e) => {
                    tracing::error!("AddHeaderFooter response dropped: {e}");
                    let _ = this.update(cx, |view, cx| {
                        view.report_engine_unavailable("Adding the header and footer", cx);
                    });
                }
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
            if let Err(e) = cmd_tx
                .send(crate::commands::PdfCommand::EncryptPdf(
                    path_str,
                    out_path.clone(),
                    "user".to_string(),
                    "owner".to_string(),
                    algo_str,
                    tx,
                ))
                .await
            {
                tracing::error!("Failed to send EncryptPdf command: {e}");
                let _ = this.update(cx, |view, cx| {
                    view.report_engine_unavailable("Encrypting", cx);
                });
                return;
            }

            match rx.await {
                Ok(Ok(_)) => {
                    let _ = this.update(cx, |view, cx| {
                        view.open_pdf_path(&out_path, cx);
                        view.status_message = Some("Document encrypted.".into());
                        cx.notify();
                    });
                }
                Ok(Err(e)) => {
                    let _ = this.update(cx, |view, cx| {
                        view.report_engine_error("Encrypting", &e, cx);
                    });
                }
                Err(e) => {
                    tracing::error!("EncryptPdf response dropped: {e}");
                    let _ = this.update(cx, |view, cx| {
                        view.report_engine_unavailable("Encrypting", cx);
                    });
                }
            }
        })
        .detach();
    }

    pub fn rotate_active_pages(&mut self, angle: i32, cx: &mut Context<Self>) {
        self.viewport.rotation = (self.viewport.rotation as i32 + angle).rem_euclid(360) as u16;
        // Rotation is part of the engine's render key but *not* of the zoom map
        // `request_render_page` gates on. Clearing only `rendered_pages` left
        // `rendered_zoom` populated, so every page short-circuited forever and
        // the canvas sat on the "Rendering page..." placeholder until the user
        // changed zoom. Thumbnails carry the same rotation, so they must go too.
        self.viewport.invalidate_rendered_pages();
        self.sidebar.thumbnails.clear();
        self.rendering_pages.clear();
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
            if let Err(e) = cmd_tx
                .send(crate::commands::PdfCommand::ReorderPages(
                    path_str,
                    remaining_pages,
                    out_path.clone(),
                    tx,
                ))
                .await
            {
                tracing::error!("Failed to send ReorderPages command: {e}");
                let _ = this.update(cx, |view, cx| {
                    view.report_engine_unavailable("Deleting the page", cx);
                });
                return;
            }

            match rx.await {
                Ok(Ok(_)) => {
                    let _ = this.update(cx, |view, cx| {
                        view.open_pdf_path(&out_path, cx);
                        view.status_message = Some(format!("Page {} deleted.", cur + 1));
                        cx.notify();
                    });
                }
                Ok(Err(e)) => {
                    let _ = this.update(cx, |view, cx| {
                        view.report_engine_error("Deleting the page", &e, cx);
                    });
                }
                Err(e) => {
                    tracing::error!("ReorderPages response dropped: {e}");
                    let _ = this.update(cx, |view, cx| {
                        view.report_engine_unavailable("Deleting the page", cx);
                    });
                }
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
            if let Err(e) = cmd_tx
                .send(crate::commands::PdfCommand::Merge(
                    input_paths,
                    out_path.clone(),
                    tx,
                ))
                .await
            {
                tracing::error!("Failed to send Merge command: {e}");
                let _ = this.update(cx, |view, cx| {
                    view.report_engine_unavailable("Merging", cx);
                });
                return;
            }

            match rx.await {
                Ok(Ok(_)) => {
                    let _ = this.update(cx, |view, cx| {
                        view.open_pdf_path(&out_path, cx);
                        view.status_message = Some("Merged PDF opened.".into());
                        cx.notify();
                    });
                }
                Ok(Err(e)) => {
                    let _ = this.update(cx, |view, cx| {
                        view.report_engine_error("Merging", &e, cx);
                    });
                }
                Err(e) => {
                    tracing::error!("Merge response dropped: {e}");
                    let _ = this.update(cx, |view, cx| {
                        view.report_engine_unavailable("Merging", cx);
                    });
                }
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
            if let Err(e) = cmd_tx
                .send(crate::commands::PdfCommand::Split(
                    path_str, pages, out_dir, tx,
                ))
                .await
            {
                tracing::error!("Failed to send Split command: {e}");
                let _ = this.update(cx, |view, cx| {
                    view.report_engine_unavailable("Splitting", cx);
                });
                return;
            }

            match rx.await {
                Ok(Ok(split_files)) => {
                    let count = split_files.len();
                    let _ = this.update(cx, |view, cx| {
                        // A split that produced nothing is a failure, not a
                        // success worth announcing.
                        view.status_message = Some(if count == 0 {
                            "Split wrote no files — check the output folder.".to_string()
                        } else {
                            format!("Split into {count} files.")
                        });
                        cx.notify();
                    });
                }
                Ok(Err(e)) => {
                    let _ = this.update(cx, |view, cx| {
                        view.report_engine_error("Splitting", &e, cx);
                    });
                }
                Err(e) => {
                    tracing::error!("Split response dropped: {e}");
                    let _ = this.update(cx, |view, cx| {
                        view.report_engine_unavailable("Splitting", cx);
                    });
                }
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
            if let Err(e) = cmd_tx
                .send(crate::commands::PdfCommand::Optimize(
                    path_str,
                    out_path.clone(),
                    tx,
                ))
                .await
            {
                tracing::error!("Failed to send Optimize command: {e}");
                let _ = this.update(cx, |view, cx| {
                    view.report_engine_unavailable("Compressing", cx);
                });
                return;
            }

            match rx.await {
                Ok(Ok(_)) => {
                    let _ = this.update(cx, |view, cx| {
                        view.open_pdf_path(&out_path, cx);
                        view.status_message = Some("Document compressed and opened.".into());
                        cx.notify();
                    });
                }
                Ok(Err(e)) => {
                    let _ = this.update(cx, |view, cx| {
                        view.report_engine_error("Compressing", &e, cx);
                    });
                }
                Err(e) => {
                    tracing::error!("Optimize response dropped: {e}");
                    let _ = this.update(cx, |view, cx| {
                        view.report_engine_unavailable("Compressing", cx);
                    });
                }
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
        self.status_message = Some(format!("Running OCR on page {}…", cur_page + 1));
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

            // Always replace the pending "Running OCR..." message. Previously
            // only the success arm ran, so any engine error left the status bar
            // claiming OCR was running forever.
            let outcome = match rx.await {
                Ok(Ok(res)) => Some(res.lines.len()),
                Ok(Err(e)) => {
                    tracing::error!("OCR failed: {e:?}");
                    None
                }
                Err(e) => {
                    tracing::error!("OCR response dropped: {e}");
                    None
                }
            };
            let _ = this.update(cx, |view, cx| {
                view.status_message = Some(match outcome {
                    Some(lines) => format!("OCR complete: extracted {lines} lines."),
                    None => "OCR failed.".into(),
                });
                cx.notify();
            });
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
            if let Err(e) = cmd_tx
                .send(crate::commands::PdfCommand::ConvertPdf(
                    doc_id,
                    "document".to_string(),
                    format_name.to_string(),
                    tx,
                ))
                .await
            {
                tracing::error!("Failed to send ConvertPdf command: {e}");
            }

            let content = match rx.await {
                Ok(Ok(content)) => content,
                Ok(Err(e)) => {
                    tracing::error!("Convert failed: {e:?}");
                    let _ = this.update(cx, |view, cx| {
                        view.status_message = Some(format!("Convert failed: {e}"));
                        cx.notify();
                    });
                    return;
                }
                Err(e) => {
                    tracing::error!("Convert response dropped: {e}");
                    let _ = this.update(cx, |view, cx| {
                        view.status_message = Some("Convert failed: engine unavailable.".into());
                        cx.notify();
                    });
                    return;
                }
            };

            // Write atomically, on a blocking thread, and only claim success
            // once the bytes are actually on disk. The old code discarded the
            // write error and then reported "Exported ... successfully." even
            // for a permission-denied or disk-full failure.
            let out_for_status = out_path.clone();
            // `background_spawn` already surfaces a panic as a task failure, so
            // the result is a plain `io::Result`.
            let result = cx
                .background_spawn(async move {
                    crate::storage::atomic_write_bytes(
                        std::path::Path::new(&out_path),
                        content.as_bytes(),
                    )
                })
                .await;

            let _ = this.update(cx, |view, cx| {
                match result {
                    Ok(()) => {
                        view.status_message =
                            Some(format!("Exported to {out_for_status} successfully."));
                    }
                    Err(e) => {
                        tracing::error!("Failed writing {out_for_status}: {e}");
                        view.status_message = Some(format!("Export failed: {e}"));
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub fn extract_tables_active_page(&mut self, cx: &mut Context<Self>) {
        let Some(doc_id) = self.active_doc_id else {
            self.log_console
                .log("[WARN] No active document to extract tables from.", "warn");
            self.status_message = Some("Open a document first to extract tables.".into());
            cx.notify();
            return;
        };
        let page_idx = self.viewport.current_page;
        let cmd_tx = self.engine.cmd_tx.clone();

        cx.spawn(async move |this, cx| {
            let (tx, rx) = tokio::sync::oneshot::channel();
            if let Err(e) = cmd_tx
                .send(crate::commands::PdfCommand::DetectTables(
                    doc_id, page_idx, tx,
                ))
                .await
            {
                tracing::error!("Failed to send DetectTables command: {e}");
                return;
            }

            match rx.await {
                Ok(Ok(tables)) => {
                    let _ = this.update(cx, |view, cx| {
                        if tables.is_empty() {
                            view.log_console.log(
                                format!("[INFO] No tables detected on Page {}.", page_idx + 1),
                                "info",
                            );
                            view.status_message =
                                Some(format!("No tables found on Page {}.", page_idx + 1));
                        } else {
                            let mut combined_csv = String::new();
                            for (i, t) in tables.iter().enumerate() {
                                if i > 0 {
                                    combined_csv.push_str("\n\n");
                                }
                                if tables.len() > 1 {
                                    combined_csv.push_str(&format!("# Table {}\n", i + 1));
                                }
                                combined_csv.push_str(&t.csv);
                            }
                            cx.write_to_clipboard(ClipboardItem::new_string(combined_csv));
                            let msg = format!(
                                "Detected {} table(s) on Page {}. Copied CSV to clipboard!",
                                tables.len(),
                                page_idx + 1
                            );
                            view.log_console.log(format!("[SUCCESS] {msg}"), "info");
                            view.status_message = Some(msg);
                        }
                        cx.notify();
                    });
                }
                Ok(Err(e)) => {
                    let _ = this.update(cx, |view, cx| {
                        view.log_console.log(
                            format!(
                                "[ERROR] Failed to detect tables on Page {}: {e}",
                                page_idx + 1
                            ),
                            "error",
                        );
                        view.status_message = Some("Table extraction failed.".into());
                        cx.notify();
                    });
                }
                Err(e) => {
                    tracing::error!("DetectTables channel dropped: {e}");
                }
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
                                self.annotations_by_doc.remove(&doc_id);
                                // The sibling `CloseTab` path logs and reports
                                // this; Ctrl+W discarded it, so a full channel
                                // left the parsed document and its render-cache
                                // entries resident for the whole session with no
                                // visible sign.
                                if let Err(e) = self
                                    .engine
                                    .cmd_tx
                                    .try_send(crate::commands::PdfCommand::Close(doc_id))
                                {
                                    tracing::error!(
                                        "Failed to release document {doc_id:?} to the engine: {e}"
                                    );
                                    self.status_message = Some(
                                        "Document closed, but the engine kept its cache.".into(),
                                    );
                                }
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
                    } else if self.log_console.is_open {
                        self.log_console.is_open = false;
                        cx.notify();
                    } else if self.viewport.selection_page.is_some() {
                        self.viewport.selection_page = None;
                        self.viewport.selection_start = None;
                        self.viewport.selection_end = None;
                        cx.notify();
                    }
                }
                "pageup" | "left" if self.viewport.current_page > 0 => {
                    self.viewport.scroll_to_page(self.viewport.current_page - 1);
                    self.render_needed_pages(cx);
                    cx.notify();
                }
                "pagedown" | "right"
                    if self.viewport.current_page + 1 < self.viewport.total_pages =>
                {
                    self.viewport.scroll_to_page(self.viewport.current_page + 1);
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
                        this.viewport.scroll_to_page(this.viewport.current_page - 1);
                        this.render_needed_pages(cx);
                        cx.notify();
                    }
                }
                RibbonAction::NextPage => {
                    if this.viewport.current_page + 1 < this.viewport.total_pages {
                        this.viewport.scroll_to_page(this.viewport.current_page + 1);
                        this.render_needed_pages(cx);
                        cx.notify();
                    }
                }
                RibbonAction::SetLayout(mode) => {
                    this.ribbon.layout_mode = mode;
                    this.viewport.layout_mode = mode;
                    // Neither arm used to request renders. `render` deliberately
                    // no longer drives rendering (a frame can be speculative),
                    // so switching Single -> Two-Page exposed a page that had
                    // never been rasterized and left it on the "Rendering
                    // page..." placeholder with no way to recover except
                    // scrolling. Ask for the newly visible pages here, exactly
                    // as the zoom, rotate and midnight arms already do.
                    this.render_needed_pages(cx);
                }
                RibbonAction::ToggleCover => {
                    this.ribbon.standalone_cover = !this.ribbon.standalone_cover;
                    this.viewport.standalone_cover = this.ribbon.standalone_cover;
                    // Turning the cover off on page 0 in Two-Page mode reveals
                    // page 1, which was never requested.
                    this.render_needed_pages(cx);
                }
                RibbonAction::ToggleMidnight => {
                    this.ribbon.midnight_mode = !this.ribbon.midnight_mode;
                    // The render filter is not part of the zoom map that gates
                    // `request_render_page`, so the zoom map has to be cleared
                    // alongside the images or the toggle would blank every page
                    // permanently.
                    this.viewport.invalidate_rendered_pages();
                    this.rendering_pages.clear();
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
                RibbonAction::FormFields => {
                    this.list_form_fields(cx);
                }
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
                RibbonAction::ExtractTables => {
                    this.extract_tables_active_page(cx);
                }
            }
            cx.notify();
        });

        // Tabs Bar
        let tabs_bar = if has_tabs {
            Some(self.tabs.render(cx, |this, action, _, cx| {
                match action {
                    TabAction::SelectTab(idx) => {
                        // Bounds-check: an out-of-range index here left
                        // `active_tab_index == tabs.len()`, which made every
                        // downstream `tabs.get(active_idx)` silently fail —
                        // the canvas kept showing the closed document, no tab
                        // rendered as active, and Save/Print/OCR no-opped.
                        if idx < this.tabs.tabs.len() {
                            this.tabs.active_tab_index = idx;
                            this.sync_viewport_to_active_tab(cx);
                        } else {
                            tracing::warn!("Ignoring SelectTab({idx}): out of range");
                        }
                    }
                    TabAction::CloseTab(idx) => {
                        if idx < this.tabs.tabs.len() {
                            let removed = this.tabs.tabs.remove(idx);
                            if let Some(doc_id) = removed.doc_id {
                                // Release the cached annotations too, or the map
                                // grows for the life of the session.
                                this.annotations_by_doc.remove(&doc_id);
                                this.doc_geometry.remove(&doc_id);
                                // Dropping this send left the parsed document and
                                // its render-cache entries resident forever, with
                                // no user-visible sign.
                                if let Err(e) = this
                                    .engine
                                    .cmd_tx
                                    .try_send(crate::commands::PdfCommand::Close(doc_id))
                                {
                                    tracing::error!(
                                        "Failed to release document {doc_id:?} to the engine: {e}"
                                    );
                                    this.status_message =
                                        Some("Document closed, but the engine kept its cache.".into());
                                }
                            }
                            if !this.tabs.tabs.is_empty() {
                                this.tabs.active_tab_index =
                                    this.tabs.active_tab_index.min(this.tabs.tabs.len() - 1);
                            } else {
                                this.tabs.active_tab_index = 0;
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
                                    this.annotations_by_doc.remove(&doc_id);
                                    this.doc_geometry.remove(&doc_id);
                                    if let Err(e) = this
                                        .engine
                                        .cmd_tx
                                        .try_send(crate::commands::PdfCommand::Close(doc_id))
                                    {
                                        tracing::error!(
                                            "Failed to release document {doc_id:?} to the engine: {e}"
                                        );
                                    }
                                }
                            }
                            this.tabs.tabs = vec![kept];
                            this.tabs.active_tab_index = 0;
                            this.sync_viewport_to_active_tab(cx);
                        }
                    }
                    TabAction::CloseToRight(idx) => {
                        // `Vec::drain` panics when the start of the range is
                        // past the end of the vector. Neither sibling arm
                        // (`CloseTab`, `CloseOthers`) bounds-checks, so a
                        // stale index here crashed the app; clamp instead.
                        let keep = idx.saturating_add(1).min(this.tabs.tabs.len());
                        for tab in this.tabs.tabs.drain(keep..) {
                            if let Some(doc_id) = tab.doc_id {
                                this.annotations_by_doc.remove(&doc_id);
                                this.doc_geometry.remove(&doc_id);
                                if let Err(e) = this
                                    .engine
                                    .cmd_tx
                                    .try_send(crate::commands::PdfCommand::Close(doc_id))
                                {
                                    tracing::error!(
                                        "Failed to release document {doc_id:?} to the engine: {e}"
                                    );
                                }
                            }
                        }
                        if this.tabs.tabs.is_empty() {
                            this.tabs.active_tab_index = 0;
                        } else if this.tabs.active_tab_index >= this.tabs.tabs.len() {
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
        //
        // NOTE: `render` must stay side-effect free. It used to write
        // `viewport.current_page` and then call `render_needed_pages`, which
        // mutates `rendering_pages` and detaches engine tasks. A render pass
        // can be speculative and discarded, so that both mutated the model
        // outside the event loop and queued engine commands from inside the
        // frame. Page tracking is now driven by the scroll handler and
        // `sync_viewport_to_active_tab`; if the offset moved without an event
        // (e.g. a scrollbar drag), only an explicit `cx.notify()` is issued
        // and the follow-up `on_window_event` pass does the work.
        let center_area: AnyElement = if has_tabs {
            let visible_page = self.viewport.calculate_visible_page_continuous();
            if visible_page != self.viewport.current_page {
                tracing::debug!(
                    "Visible page drifted: current={} visible={visible_page}",
                    self.viewport.current_page
                );
            }
            let total = self.viewport.total_pages;
            let current = self.viewport.current_page;

            let sidebar = self.sidebar.render(
                cx,
                total,
                current,
                &self.viewport.annotations,
                |this, action, window, cx| {
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
                            this.viewport.scroll_to_page(page_idx);
                            this.render_needed_pages(cx);
                        }
                        SidebarAction::SearchQuery(q) => {
                            // The search editor is the source of truth; keep it in
                            // sync so the two can never disagree.
                            this.sidebar.search_query = q.clone();
                            if let Some(input) = this.sidebar.search_input.clone() {
                                input.update(cx, |input, cx| input.set_value(q, window, cx));
                            }
                        }
                        SidebarAction::ExecuteSearch => {
                            let q = this.sidebar.search_query.clone();
                            this.execute_search(&q, cx);
                        }
                        SidebarAction::ClearSearch => {
                            this.sidebar.clear_search(window, cx);
                            this.sidebar.search_results.clear();
                            this.viewport.search_highlights.clear();
                            this.sidebar.is_searching = false;
                        }
                        SidebarAction::SelectSearchResult(page, ..) => {
                            this.viewport.scroll_to_page(page);
                            this.sidebar
                                .scroll_thumbnails_to(page, this.viewport.total_pages);
                            this.render_needed_pages(cx);
                        }
                        SidebarAction::DeleteAnnotation(id) => {
                            // Remove exactly one match. `retain(|a| a.id != id)`
                            // dropped every annotation sharing the id, so a single
                            // click could remove two notes.
                            if let Some(pos) =
                                this.viewport.annotations.iter().position(|a| a.id == id)
                            {
                                this.viewport.annotations.remove(pos);
                                this.viewport.reindex_annotations();
                                if let Some(tab) =
                                    this.tabs.tabs.get_mut(this.tabs.active_tab_index)
                                {
                                    tab.is_modified = true;
                                }
                            }
                        }
                        SidebarAction::SelectBookmark(page) => {
                            this.viewport.scroll_to_page(page);
                            this.sidebar
                                .scroll_thumbnails_to(page, this.viewport.total_pages);
                            this.render_needed_pages(cx);
                        }
                        SidebarAction::ToggleLayer(idx, visible) => {
                            this.set_layer_visibility(idx, visible, cx);
                        }
                    }
                    cx.notify();
                },
            );

            let canvas = self.viewport.render(
                cx,
                |this, page_idx, pos, _, cx| {
                    this.viewport.is_selecting = true;
                    this.viewport.selection_page = Some(page_idx);
                    this.viewport.selection_start = Some(pos);
                    this.viewport.selection_end = Some(pos);
                    this.viewport.selected_text = None;
                    cx.notify();
                },
                |this, _page_idx, pos, _, cx| {
                    // Bug 2 fix: gate only on is_selecting, not on which card fired.
                    // In Continuous mode every card has its own listener; ignoring
                    // _page_idx lets the end-point update when the cursor crosses a
                    // page boundary. pos is in window space and is authoritative.
                    // Keep the current-page readout in sync here too: this fires on
                    // every mouse move over the canvas, which covers scrollbar
                    // drags (the scrollbar is a child, so its events bubble here)
                    // now that render no longer writes this field.
                    this.sync_current_page_from_scroll(cx);
                    if this.viewport.is_selecting {
                        this.viewport.selection_end = Some(pos);
                        cx.notify();
                    }
                },
                |this, _page_idx, _, cx| {
                    // Bug 3 fix: gate only on is_selecting; do NOT check selection_page ==
                    // Some(_page_idx).  When mousedown is on page 3 and mouseup fires on
                    // page 4, _page_idx==4 but selection_page==Some(3) — the old guard
                    // silently dropped the commit.  We always resolve against the stored
                    // selection_page so text extraction and annotation coordinates are
                    // relative to the correct page origin.
                    if this.viewport.is_selecting {
                        // Always clear is_selecting first so no edge-case can leave the
                        // viewport stuck in selecting mode.
                        this.viewport.is_selecting = false;
                        // Authoritative page: where mousedown happened, not where mouseup did.
                        let commit_page = this.viewport.selection_page.unwrap_or(_page_idx);
                        if let (Some(start), Some(end)) =
                            (this.viewport.selection_start, this.viewport.selection_end)
                        {
                            let dx = (start.x - end.x).abs();
                            let dy = (start.y - end.y).abs();
                            if dx < px(4.0) && dy < px(4.0) {
                                // Treat as a click: clear selection state.
                                this.viewport.selection_start = None;
                                this.viewport.selection_end = None;
                                this.viewport.selection_page = None;
                                this.viewport.selected_text = None;
                                // Bug 4 (b): fold page-focus into drag_end for the click case
                                // so on_click becomes redundant and can be removed.
                                this.viewport.current_page = commit_page;
                                this.render_needed_pages(cx);
                            } else {
                                // Real drag: extract text against the page where the drag started.
                                this.extract_selected_text(commit_page, start, end, cx);

                                if this.ribbon.active_tool != super::ribbon::AnnotationTool::Pointer
                                {
                                    let origin = this
                                        .viewport
                                        .page_origins
                                        .read()
                                        .ok()
                                        .and_then(|m| m.get(&commit_page).copied())
                                        .unwrap_or_default();
                                    let zoom = if this.viewport.zoom.is_finite() {
                                        this.viewport.zoom.clamp(0.01, 100.0)
                                    } else {
                                        1.0
                                    };
                                    let min_x = ((start.x.min(end.x) - origin.x) / px(1.0)) / zoom;
                                    let max_x = ((start.x.max(end.x) - origin.x) / px(1.0)) / zoom;
                                    let min_y = ((start.y.min(end.y) - origin.y) / px(1.0)) / zoom;
                                    let max_y = ((start.y.max(end.y) - origin.y) / px(1.0)) / zoom;
                                    let w = (max_x - min_x).max(10.0);
                                    let h = (max_y - min_y).max(10.0);

                                    // Use the global monotonic counter, never
                                    // `annotations.len() + 1`. `len()` shrinks on
                                    // delete, so the next annotation reused a live
                                    // id; the sidebar delete button keys on that id
                                    // and `retain(|a| a.id != id)` removed *every*
                                    // match — one click deleted two highlights.
                                    let ann_id = crate::models::next_annotation_id();
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
                                        // `Ink` and `Text` used to fall through to
                                        // this arm, so choosing the pen produced a
                                        // yellow highlight box labelled "Highlight"
                                        // in the Notes panel. `Text` now gets its
                                        // own style; `Ink` has no freehand
                                        // implementation anywhere in the engine,
                                        // so the ribbon no longer offers it
                                        // (`AnnotationTool::Ink` is kept for the
                                        // public enum, but is not rendered).
                                        super::ribbon::AnnotationTool::Text => {
                                            crate::models::AnnotationStyle::Text {
                                                text: String::new(),
                                                color: "#111827".to_string(),
                                                font_size: 12,
                                            }
                                        }
                                        super::ribbon::AnnotationTool::Ink
                                        | super::ribbon::AnnotationTool::Pointer => {
                                            this.status_message =
                                                Some("Freehand ink is not available yet.".into());
                                            cx.notify();
                                            return;
                                        }
                                    };

                                    // Annotation is stored against commit_page (the drag-start page).
                                    this.viewport.annotations.push(crate::models::Annotation {
                                        id: ann_id,
                                        page: commit_page,
                                        style,
                                        x: min_x,
                                        y: min_y,
                                        width: w,
                                        height: h,
                                    });
                                    this.viewport.reindex_annotations();
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

            div()
                .flex()
                .flex_row()
                .flex_1()
                .w_full()
                .min_h_0()
                .overflow_hidden()
                .child(sidebar)
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .flex_1()
                        .h_full()
                        .min_w_0()
                        .min_h_0()
                        .overflow_hidden()
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .flex_1()
                                .w_full()
                                .min_h_0()
                                .overflow_hidden()
                                .child(canvas),
                        ),
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
        //
        // There used to be *two* stacked bars: the document one built by
        // `render_status_bar` (page navigation + zoom, pinned to the bottom of
        // the right-hand column) and this one (page count + zoom + the status
        // message), pinned to the bottom of the window. Both reported the page
        // and the zoom, so the same facts appeared twice on screen at once, and
        // `render_status_bar` only existed on the document workspace. There is
        // now a single bar, rendered once at the window level so the welcome
        // screen and the document workspace share it.
        let status_bar = self.render_status_bar(cx);

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
                    let text = this.log_console.as_text();
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
                // Only actions that commit the whole dialog close it. Pure selections
                // (a watermark preset chip) and repeatable page operations
                // (rotate, delete) must not: the Page Organizer is a
                // multi-operation modal, and dismissing it after the first
                // button press meant rotating page after page required
                // reopening it each time.
                if !matches!(
                    action,
                    DialogAction::SelectWatermark(_)
                        | DialogAction::RotatePages(_)
                        | DialogAction::SelectSecurityAlgorithm(_)
                ) {
                    this.dialogs.active = None;
                }
                match action {
                    DialogAction::SelectWatermark(text) => {
                        this.dialogs.selected_watermark = text;
                    }
                    DialogAction::SelectSecurityAlgorithm(algo) => {
                        // Selection only — the dialog stays open until Apply.
                        this.dialogs.security_algorithm = algo;
                    }
                    DialogAction::SetWatermark(text) => {
                        this.apply_watermark(&text, cx);
                    }
                    DialogAction::SetHeaderFooter(h, f) => {
                        // Read the editors when they exist; the fields are
                        // editable now rather than being static text.
                        let h = this
                            .dialogs
                            .header_input
                            .as_ref()
                            .map(|i| i.read(cx).value().to_string())
                            .filter(|v| !v.is_empty() && v != &h)
                            .unwrap_or(h);
                        let f = this
                            .dialogs
                            .footer_input
                            .as_ref()
                            .map(|i| i.read(cx).value().to_string())
                            .filter(|v| !v.is_empty() && v != &f)
                            .unwrap_or(f);
                        this.apply_header_footer(&h, &f, cx);
                    }
                    DialogAction::SetPassword(_pwd) => {
                        // The engine has no decrypt path, so rather than close a
                        // modal that silently did nothing, say so plainly.
                        this.status_message = Some(
                            "Decrypting password-protected PDFs is not implemented yet.".into(),
                        );
                    }
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
                // The old `|| path.is_file()` disjunct made the extension test
                // redundant, so dropping a PNG opened a permanent blank tab.
                let mut opened = false;
                for path in paths.paths() {
                    if !SidebarState::is_openable_pdf(path) {
                        tracing::warn!("Ignoring dropped non-PDF file: {}", path.display());
                        continue;
                    }
                    this.open_pdf_path(&path.to_string_lossy(), cx);
                    opened = true;
                }
                if opened {
                    cx.notify();
                }
            }))
            .child(header_bar)
            .child(action_strip)
            .children(tabs_bar)
            .child(
                div()
                    .flex_1()
                    .w_full()
                    .min_h_0()
                    .flex()
                    .flex_col()
                    .overflow_hidden()
                    .child(center_area),
            )
            .children(log_drawer)
            .child(status_bar)
            .children(dialog_overlay)
    }
}
