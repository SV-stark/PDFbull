use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::input::{Input, InputState};
use gpui_kit::component::scroll::ScrollableElement;
use gpui_kit::component::{
    ActiveTheme, Disableable as _, Icon, IconName, Selectable as _, Sizable,
};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

/// Format a byte count the way a file manager does.
///
/// The sidebar used to print the raw count ("4194304 bytes"), which nobody can
/// compare at a glance. Unit-scaled so the values line up in the column.
fn human_bytes(bytes: u64) -> String {
    const UNITS: [(u64, &str); 4] = [
        (1024 * 1024 * 1024, "GB"),
        (1024 * 1024, "MB"),
        (1024, "KB"),
        (1, "B"),
    ];
    for (scale, suffix) in UNITS {
        if bytes < scale {
            continue;
        }
        let value = bytes as f64 / scale as f64;
        return if scale == 1 {
            format!("{bytes} {suffix}")
        } else if value >= 100.0 {
            format!("{value:.0} {suffix}")
        } else if value >= 10.0 {
            format!("{value:.1} {suffix}")
        } else {
            format!("{value:.2} {suffix}")
        };
    }
    format!("{bytes} B")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SidebarMode {
    #[default]
    Thumbnails,
    Bookmarks,
    Annotations,
    Search,
    Attachments,
    Layers,
}

/// Approximate vertical stride of one thumbnail card (160px image + label +
/// padding + gap). Used to map scroll offsets to page indices.
const THUMB_STRIDE: f32 = 200.0;
/// Pixels of thumbnail content to keep built above and below the viewport.
const THUMB_OVERSCAN: f32 = 400.0;
/// Hard cap on thumbnail cards built per frame, independent of layout state.
const MAX_THUMB_WINDOW: usize = 64;

#[derive(Debug, Clone, PartialEq)]
pub enum SidebarAction {
    ToggleOpen,
    SelectMode(SidebarMode),
    SelectPage(usize),
    SearchQuery(String),
    ExecuteSearch,
    ClearSearch,
    SelectSearchResult(usize, f32, f32, f32, f32),
    DeleteAnnotation(u64),
    SelectBookmark(usize),
    ToggleLayer(usize, bool),
}

pub struct SidebarState {
    pub is_open: bool,
    pub mode: SidebarMode,
    pub width: f32,
    pub thumbnails: std::collections::HashMap<usize, std::sync::Arc<RenderImage>>,
    /// Scroll position of the thumbnail strip, so the visible window can follow
    /// it. `overflow_y_scrollbar()` creates its own private handle, which the
    /// windowing maths could not read.
    pub thumb_scroll: ScrollHandle,
    pub search_query: String,
    /// Backing editor for the search box. `None` outside a live GPUI context
    /// (see [`SidebarState::new_for_test`]). The panel previously had no input
    /// at all, so `search_query` was never set from the UI and in-document
    /// search was unreachable.
    pub search_input: Option<Entity<InputState>>,
    pub search_results: Vec<crate::models::SearchResultItem>,
    pub is_searching: bool,
    pub bookmarks: Vec<crate::pdf_engine::Bookmark>,
    pub attachments: Vec<crate::models::AttachmentInfo>,
    pub layers: Vec<crate::models::LayerInfo>,
}

impl SidebarState {
    /// Build the sidebar. The search editor must be created from the *owner's*
    /// entity scope (an `InputState` needs its own `Context`), so `owner` is
    /// the `Context` of whichever entity embeds this state — `SidebarState` is
    /// plain data, not an `Entity` of its own.
    pub fn new<O: 'static>(window: &mut Window, owner: &mut Context<O>) -> Self {
        Self {
            search_input: Some(owner.new(|cx| InputState::new(window, cx))),
            ..Self::new_for_test()
        }
    }

    /// Build a sidebar with no editor attached, for tests and for any context
    /// that has no window to create an `InputState` in. The search panel renders
    /// without its input field in that case.
    pub fn new_for_test() -> Self {
        Self {
            is_open: true,
            mode: SidebarMode::Thumbnails,
            width: 250.0,
            thumbnails: std::collections::HashMap::new(),
            thumb_scroll: ScrollHandle::new(),
            search_query: String::new(),
            search_input: None,
            search_results: Vec::new(),
            is_searching: false,
            bookmarks: Vec::new(),
            attachments: Vec::new(),
            layers: Vec::new(),
        }
    }

    /// Mirror the editor's contents into `search_query`.
    ///
    /// Called from the owner's `InputEvent::Change` subscription. The old panel
    /// had no input at all, so `search_query` stayed permanently empty and
    /// `execute_search` always short-circuited.
    pub fn on_search_input_changed(&mut self, cx: &mut gpui_kit::App) {
        if let Some(input) = &self.search_input {
            self.search_query = input.read(cx).value().to_string();
        }
    }

    /// Clear the query in both the editor and the mirror string.
    pub fn clear_search(&mut self, window: &mut Window, cx: &mut gpui_kit::App) {
        self.search_query.clear();
        if let Some(input) = &self.search_input {
            input.update(cx, |input, cx| input.set_value(String::new(), window, cx));
        }
    }

    /// Discard the previous document's search state when switching documents.
    ///
    /// The clear paths used to set `search_query` directly, which is only a
    /// *mirror* of `search_input`. The visible text box therefore kept the
    /// previous document's query while the panel's own status line read "Enter
    /// search query above" (because the mirror was empty) — and pressing
    /// Search then ran an empty query and appeared to do nothing. Re-reading
    /// the editor here keeps the two in lockstep without needing the `Window`
    /// that `clear_search` requires.
    pub fn reset_search(&mut self, cx: &mut gpui_kit::App) {
        self.search_results.clear();
        self.is_searching = false;
        self.on_search_input_changed(cx);
    }

    /// Half-open range of page indices whose thumbnail cards should be built,
    /// derived from the strip's live scroll position.
    ///
    /// The viewport height comes from the scroll handle: GPUI sets `max_offset`
    /// to `content_size - bounds` on each prepaint, so
    /// `content_size - max_offset` is the current viewport height. The width is
    /// *always* capped at [`MAX_THUMB_WINDOW`]: before the first layout pass
    /// `max_offset` is zero, which would make the derived viewport equal the
    /// whole document and reintroduce the O(total_pages) build.
    pub fn thumbnail_window(&self, total_pages: usize) -> (usize, usize) {
        if total_pages == 0 {
            return (0, 0);
        }
        let content_h = px(THUMB_STRIDE * total_pages as f32);
        let viewport_h = (content_h - self.thumb_scroll.max_offset().y).max(px(0.0));
        let scrolled = (-self.thumb_scroll.offset().y).max(px(0.0));

        let first_px = (scrolled - px(THUMB_OVERSCAN)).max(px(0.0));
        let last_px = scrolled + viewport_h + px(THUMB_OVERSCAN);

        let first = ((first_px / px(THUMB_STRIDE)).floor().max(0.0) as usize)
            .min(total_pages.saturating_sub(1));
        let last =
            (((last_px + px(THUMB_STRIDE)) / px(THUMB_STRIDE)).ceil() as usize).min(total_pages);
        let last = last.max(first + 1);

        if last - first <= MAX_THUMB_WINDOW {
            (first, last)
        } else {
            // Too wide: keep the window around the scroll position instead.
            let start = first.min(total_pages.saturating_sub(MAX_THUMB_WINDOW));
            (start, start + MAX_THUMB_WINDOW)
        }
    }

    /// True when `path` is something this app should try to open.
    ///
    /// Kept in one place so every entry point (drop handler, command line,
    /// recent files) applies the same rule. A directory or a non-PDF produced
    /// a permanent unrenderable tab before this was centralised.
    pub fn is_openable_pdf(path: &std::path::Path) -> bool {
        path.extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("pdf"))
            && path.is_file()
    }

    /// Bring `page_idx`'s thumbnail into view in the strip.
    ///
    /// Clamped against the strip's own content height rather than the scroll
    /// handle's `max_offset`: before the first layout pass `max_offset` is zero,
    /// which would pin the strip at the top and make "jump to page N" a no-op.
    /// GPUI re-clamps against the real bounds on the next prepaint regardless.
    pub fn scroll_thumbnails_to(&mut self, page_idx: usize, total_pages: usize) {
        let target = px(-THUMB_STRIDE * page_idx as f32);
        let min = px(-THUMB_STRIDE * total_pages as f32);
        self.thumb_scroll
            .set_offset(Point::new(px(0.0), target.clamp(min, px(0.0))));
    }

    pub fn render<V: 'static>(
        &self,
        cx: &mut Context<V>,
        total_pages: usize,
        current_page: usize,
        annotations: &[crate::models::Annotation],
        on_action: impl Fn(&mut V, SidebarAction, &mut Window, &mut Context<V>) + 'static + Copy,
    ) -> AnyElement {
        let primary = cx.theme().primary;
        let border = cx.theme().border;
        let accent = cx.theme().accent;
        let background = cx.theme().background;
        let muted = cx.theme().muted;
        let muted_fg = cx.theme().muted_foreground;
        let fg = cx.theme().foreground;

        if !self.is_open {
            return div()
                .flex()
                .flex_col()
                .items_center()
                .w_10()
                .border_r_1()
                .border_color(border)
                .bg(muted)
                .py_2()
                .child(
                    // Icon-only, so it needs a tooltip. The bare "▶" glyph had
                    // neither a name nor an explanation.
                    Button::new("btn-expand-sidebar")
                        .icon(IconName::PanelLeft)
                        .ghost()
                        .tooltip("Show sidebar")
                        .on_click(cx.listener(move |v, _, w, cx| {
                            on_action(v, SidebarAction::ToggleOpen, w, cx)
                        })),
                )
                .into_any_element();
        }

        let modes = [
            (SidebarMode::Thumbnails, "Pages"),
            (SidebarMode::Bookmarks, "Bookmarks"),
            (SidebarMode::Annotations, "Notes"),
            (SidebarMode::Search, "Search"),
            (SidebarMode::Attachments, "Files"),
            (SidebarMode::Layers, "Layers"),
        ];

        let cur_mode = self.mode;

        let content: AnyElement = match cur_mode {
            SidebarMode::Thumbnails => {
                // Virtualized on the *scroll position*, not the selected page,
                // so the panel still navigates the whole document. Building one
                // card per page meant a 1000-page document created 1000 element
                // subtrees every frame, on top of the canvas's own cards.
                let (first, last) = self.thumbnail_window(total_pages);
                let top_pad = THUMB_STRIDE * first as f32;
                let bottom_pad = THUMB_STRIDE * total_pages.saturating_sub(last) as f32;

                let mut thumb_cards: Vec<AnyElement> = Vec::new();
                if top_pad > 0.0 {
                    thumb_cards.push(div().flex_none().h(px(top_pad)).into_any_element());
                }
                for page_idx in first..last {
                    let is_selected = page_idx == current_page;
                    let click_listener = cx.listener(move |v, _, w, cx| {
                        on_action(v, SidebarAction::SelectPage(page_idx), w, cx)
                    });
                    thumb_cards.push(
                        div()
                            .id(SharedString::from(format!("side-page-{}", page_idx)))
                            .flex()
                            .flex_col()
                            .items_center()
                            .p_2()
                            .rounded_md()
                            .border_1()
                            .border_color(if is_selected { primary } else { border })
                            .bg(if is_selected { accent } else { background })
                            .cursor_pointer()
                            .on_click(click_listener)
                            .child(if let Some(thumb_img) = self.thumbnails.get(&page_idx) {
                                div()
                                    .w_32()
                                    .h(px(160.0))
                                    .bg(gpui_kit::white())
                                    .overflow_hidden()
                                    .rounded_sm()
                                    .border_1()
                                    .border_color(border)
                                    .child(img(thumb_img.clone()).w_full().h_full())
                            } else {
                                div()
                                    .w_32()
                                    .h(px(160.0))
                                    .bg(muted)
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .text_xs()
                                    .text_color(muted_fg)
                                    .child(format!("Page {}", page_idx + 1))
                            })
                            .child(
                                div()
                                    .pt_1()
                                    .text_xs()
                                    .text_color(muted_fg)
                                    .child(format!("{}", page_idx + 1)),
                            )
                            .into_any_element(),
                    );
                }
                if bottom_pad > 0.0 {
                    thumb_cards.push(div().flex_none().h(px(bottom_pad)).into_any_element());
                }

                // `overflow_y_scroll` + `track_scroll`: GPUI owns the wheel
                // handling and clamping, and notifies on every change so this
                // window is recomputed. We deliberately do *not* also apply the
                // wheel delta ourselves (that combination is what made the
                // canvas scroll at 2x speed).
                div()
                    .id("sidebar-thumbnails-scroll")
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_h_0()
                    .gap_2()
                    .p_2()
                    .overflow_y_scroll()
                    .track_scroll(&self.thumb_scroll)
                    .vertical_scrollbar(&self.thumb_scroll)
                    .children(thumb_cards)
                    .into_any_element()
            }

            SidebarMode::Bookmarks => {
                if self.bookmarks.is_empty() {
                    div()
                        .flex()
                        .flex_col()
                        .items_center()
                        .justify_center()
                        .p_6()
                        .text_xs()
                        .text_color(muted_fg)
                        .child("No bookmarks in this document.")
                        .into_any_element()
                } else {
                    let mut items = Vec::new();
                    for bm in self.bookmarks.iter() {
                        let target_page = bm.page_index;
                        let title = bm.title.clone();
                        // Keyed on the bookmark's target and title rather than
                        // its list position. These lists are rebuilt from
                        // scratch on every document switch, so position-based
                        // ids handed document A's row *n* retained hover state
                        // to document B's row *n*.
                        items.push(
                            div()
                                .id(SharedString::from(format!(
                                    "bm-item-{}-{}",
                                    target_page,
                                    title.len()
                                )))
                                .flex()
                                .flex_row()
                                .items_center()
                                .justify_between()
                                .px_3()
                                .py_2()
                                .rounded_md()
                                .hover(|s| s.bg(accent))
                                .cursor_pointer()
                                .on_click(cx.listener(move |v, _, w, cx| {
                                    on_action(v, SidebarAction::SelectBookmark(target_page), w, cx)
                                }))
                                .child(
                                    div()
                                        .flex_1()
                                        .text_xs()
                                        .text_color(fg)
                                        .overflow_hidden()
                                        .child(title),
                                )
                                .child(
                                    div()
                                        .px_1p5()
                                        .py_0p5()
                                        .bg(muted)
                                        .rounded_sm()
                                        .text_xs()
                                        .text_color(muted_fg)
                                        .child(format!("p. {}", target_page + 1)),
                                ),
                        );
                    }
                    div()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .p_2()
                        .overflow_y_scrollbar()
                        .children(items)
                        .into_any_element()
                }
            }

            SidebarMode::Annotations => {
                if annotations.is_empty() {
                    div()
                        .flex()
                        .flex_col()
                        .items_center()
                        .justify_center()
                        .p_6()
                        .text_xs()
                        .text_color(muted_fg)
                        .child("No annotations in this document.")
                        .into_any_element()
                } else {
                    let mut items = Vec::new();
                    for ann in annotations {
                        let ann_id = ann.id;
                        let page_idx = ann.page;
                        let type_str = match &ann.style {
                            crate::models::AnnotationStyle::Highlight { .. } => "Highlight",
                            crate::models::AnnotationStyle::Rectangle { .. } => "Rectangle",
                            crate::models::AnnotationStyle::Circle { .. } => "Circle",
                            crate::models::AnnotationStyle::Line { .. } => "Line",
                            crate::models::AnnotationStyle::Arrow { .. } => "Arrow",
                            crate::models::AnnotationStyle::StickyNote { comment, .. } => {
                                if comment.is_empty() {
                                    "Note"
                                } else {
                                    comment.as_str()
                                }
                            }
                            crate::models::AnnotationStyle::Text { text, .. } => text.as_str(),
                            crate::models::AnnotationStyle::Redact { .. } => "Redact",
                        };

                        items.push(
                            div()
                                .id(SharedString::from(format!("ann-card-{}", ann_id)))
                                .flex()
                                .flex_row()
                                .items_center()
                                .justify_between()
                                .p_2()
                                .rounded_md()
                                .border_1()
                                .border_color(border)
                                .bg(background)
                                .hover(|s| s.bg(accent))
                                .child(
                                    div()
                                        .id(SharedString::from(format!("ann-info-{}", ann_id)))
                                        .flex()
                                        .flex_col()
                                        .gap_0p5()
                                        .cursor_pointer()
                                        .on_click(cx.listener(move |v, _, w, cx| {
                                            on_action(v, SidebarAction::SelectPage(page_idx), w, cx)
                                        }))
                                        .child(
                                            div()
                                                .text_xs()
                                                .text_color(fg)
                                                .child(type_str.to_string()),
                                        )
                                        .child(
                                            div()
                                                .text_xs()
                                                .text_color(muted_fg)
                                                .child(format!("Page {}", page_idx + 1)),
                                        ),
                                )
                                .child(
                                    Button::new(format!("btn-del-ann-{}", ann_id))
                                        .icon(IconName::Delete)
                                        .ghost()
                                        .small()
                                        .tooltip("Delete this annotation")
                                        .on_click(cx.listener(move |v, _, w, cx| {
                                            on_action(
                                                v,
                                                SidebarAction::DeleteAnnotation(ann_id),
                                                w,
                                                cx,
                                            )
                                        })),
                                ),
                        );
                    }
                    div()
                        .flex()
                        .flex_col()
                        .gap_1p5()
                        .p_2()
                        .overflow_y_scrollbar()
                        .children(items)
                        .into_any_element()
                }
            }

            SidebarMode::Search => {
                let match_count = self.search_results.len();
                let is_searching = self.is_searching;

                let search_box = div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .p_3()
                    .border_b_1()
                    .border_color(border)
                    .bg(muted)
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .justify_between()
                            .child(div().text_xs().text_color(fg).child("Find in Document"))
                            .child(
                                Button::new("btn-clear-search")
                                    .label("Clear")
                                    .ghost()
                                    .on_click(cx.listener(move |v, _, w, cx| {
                                        on_action(v, SidebarAction::ClearSearch, w, cx)
                                    })),
                            ),
                    )
                    // The actual query field. Its absence was why
                    // `SidebarAction::SearchQuery` was never constructed and
                    // in-document search could not be started at all.
                    .when(self.search_input.is_some(), |el| {
                        el.child(
                            Input::new(
                                self.search_input
                                    .as_ref()
                                    .expect("checked by the when() guard"),
                            )
                            .id("sidebar-search-input")
                            .small()
                            .w_full(),
                        )
                    })
                    .child(
                        // A disabled button is the honest state here: the
                        // handler's `if !is_searching` guard made a pending
                        // search look clickable while doing nothing.
                        Button::new("btn-exec-search")
                            .icon(if is_searching {
                                Icon::new(IconName::Loader).small()
                            } else {
                                Icon::new(IconName::Search)
                            })
                            .label(if is_searching {
                                "Searching…"
                            } else {
                                "Search"
                            })
                            .primary()
                            .small()
                            .disabled(is_searching)
                            .on_click(cx.listener(move |v, _, w, cx| {
                                on_action(v, SidebarAction::ExecuteSearch, w, cx)
                            })),
                    )
                    .child(div().text_xs().text_color(muted_fg).child(if is_searching {
                        "Searching the whole document…".to_string()
                    } else if match_count > 0 {
                        format!("Found {match_count} occurrences")
                    } else if !self.search_query.is_empty() {
                        "No matches found".to_string()
                    } else {
                        "Type a query, then press Search.".to_string()
                    }));

                let mut result_items = Vec::new();
                for item in self.search_results.iter() {
                    let page = item.page_index;
                    let snippet = item.text.clone();
                    let (x, y, w, h) = (item.x, item.y, item.width, item.height);
                    // Keyed on the hit's position rather than its list index: a
                    // new search replaces the whole list, and index-keyed ids
                    // made every row inherit the previous search's element state.
                    let res_id = format!("search-res-{page}-{:.0}-{:.0}", x, y);

                    result_items.push(
                        div()
                            .id(SharedString::from(res_id))
                            .flex()
                            .flex_col()
                            .p_2()
                            .rounded_md()
                            .border_1()
                            .border_color(border)
                            .bg(background)
                            .hover(|s| s.bg(accent))
                            .cursor_pointer()
                            .on_click(cx.listener(move |v, _, win, cx| {
                                on_action(
                                    v,
                                    SidebarAction::SelectSearchResult(page, x, y, w, h),
                                    win,
                                    cx,
                                )
                            }))
                            .child(
                                div().flex().flex_row().justify_between().child(
                                    div()
                                        .text_xs()
                                        .text_color(primary)
                                        .child(format!("Page {}", page + 1)),
                                ),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(fg)
                                    .overflow_hidden()
                                    .child(snippet),
                            ),
                    );
                }

                div()
                    .flex()
                    .flex_col()
                    .size_full()
                    .child(search_box)
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .gap_1p5()
                            .p_2()
                            .overflow_y_scrollbar()
                            .children(result_items),
                    )
                    .into_any_element()
            }

            SidebarMode::Attachments => {
                if self.attachments.is_empty() {
                    div()
                        .flex()
                        .flex_col()
                        .items_center()
                        .justify_center()
                        .p_6()
                        .text_xs()
                        .text_color(muted_fg)
                        .child("No embedded attachments found.")
                        .into_any_element()
                } else {
                    let mut items = Vec::new();
                    for att in self.attachments.iter() {
                        let name = att.name.clone();
                        let size = att.size;
                        // Stable key: `object_id` when the PDF provided one,
                        // otherwise the embedded file's name.
                        let key = match att.object_id {
                            Some((num, generation)) => format!("{num}-{generation}"),
                            None => name.clone(),
                        };
                        items.push(
                            div()
                                .id(SharedString::from(format!("att-{key}")))
                                .flex()
                                .flex_row()
                                .items_center()
                                .justify_between()
                                .p_2()
                                .rounded_md()
                                .border_1()
                                .border_color(border)
                                .bg(background)
                                .child(
                                    div()
                                        .flex()
                                        .flex_col()
                                        .child(div().text_xs().text_color(fg).child(name))
                                        .child(
                                            div().text_xs().text_color(muted_fg).child(
                                                human_bytes(size.unwrap_or(0).max(0) as u64),
                                            ),
                                        ),
                                ),
                        );
                    }
                    div()
                        .flex()
                        .flex_col()
                        .gap_1p5()
                        .p_2()
                        .overflow_y_scrollbar()
                        .children(items)
                        .into_any_element()
                }
            }

            SidebarMode::Layers => {
                if self.layers.is_empty() {
                    div()
                        .flex()
                        .flex_col()
                        .items_center()
                        .justify_center()
                        .p_6()
                        .text_xs()
                        .text_color(muted_fg)
                        .child("No optional content layers.")
                        .into_any_element()
                } else {
                    let mut items = Vec::new();
                    for (idx, layer) in self.layers.iter().enumerate() {
                        let name = layer.name.clone();
                        let is_vis = layer.visible;
                        let (num, generation) = layer.object_id;
                        items.push(
                            div()
                                .id(SharedString::from(format!("layer-{num}-{generation}")))
                                .flex()
                                .flex_row()
                                .items_center()
                                .justify_between()
                                .p_2()
                                .rounded_md()
                                .border_1()
                                .border_color(border)
                                .bg(background)
                                .child(div().text_xs().text_color(fg).child(name))
                                .child(
                                    Button::new(format!("btn-layer-{num}-{generation}"))
                                        .label(if is_vis { "Visible" } else { "Hidden" })
                                        .ghost()
                                        .small()
                                        .tooltip(if is_vis {
                                            "Hide this layer"
                                        } else {
                                            "Show this layer"
                                        })
                                        .on_click(cx.listener(move |v, _, w, cx| {
                                            on_action(
                                                v,
                                                SidebarAction::ToggleLayer(idx, !is_vis),
                                                w,
                                                cx,
                                            )
                                        })),
                                ),
                        );
                    }
                    div()
                        .flex()
                        .flex_col()
                        .gap_1p5()
                        .p_2()
                        .overflow_y_scrollbar()
                        .children(items)
                        .into_any_element()
                }
            }
        };

        div()
            .flex()
            .flex_col()
            .w(px(self.width))
            .border_r_1()
            .border_color(border)
            .bg(background)
            .child({
                let mut mode_buttons = Vec::new();
                for (m, label) in modes {
                    let is_active = m == cur_mode;
                    let click_listener = cx.listener(move |v, _, w, cx| {
                        on_action(v, SidebarAction::SelectMode(m), w, cx)
                    });
                    mode_buttons.push(
                        Button::new(format!("side-mode-{:?}", m))
                            .label(label)
                            .small()
                            // The active panel is a *selection*, not the
                            // primary commit of a decision area.
                            .selected(is_active)
                            .on_click(click_listener),
                    );
                }
                div()
                    .flex()
                    .flex_row()
                    .flex_wrap()
                    .gap_1()
                    .p_2()
                    .border_b_1()
                    .border_color(border)
                    .bg(muted)
                    .children(mode_buttons)
            })
            .child(div().flex_1().overflow_hidden().child(content))
            .into_any_element()
    }
}
