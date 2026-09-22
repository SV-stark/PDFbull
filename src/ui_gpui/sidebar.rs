use gpui_kit::component::ActiveTheme;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::scroll::ScrollableElement;
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

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
    pub search_query: String,
    pub search_results: Vec<crate::models::SearchResultItem>,
    pub is_searching: bool,
    pub bookmarks: Vec<crate::pdf_engine::Bookmark>,
    pub attachments: Vec<crate::models::AttachmentInfo>,
    pub layers: Vec<crate::models::LayerInfo>,
}

impl Default for SidebarState {
    fn default() -> Self {
        Self::new()
    }
}

impl SidebarState {
    pub fn new() -> Self {
        Self {
            is_open: true,
            mode: SidebarMode::Thumbnails,
            width: 250.0,
            thumbnails: std::collections::HashMap::new(),
            search_query: String::new(),
            search_results: Vec::new(),
            is_searching: false,
            bookmarks: Vec::new(),
            attachments: Vec::new(),
            layers: Vec::new(),
        }
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
                    Button::new("btn-expand-sidebar")
                        .label("▶")
                        .ghost()
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
                let mut thumb_cards = Vec::new();
                for page_idx in 0..total_pages {
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
                            ),
                    );
                }

                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .p_2()
                    .overflow_y_scrollbar()
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
                    for (idx, bm) in self.bookmarks.iter().enumerate() {
                        let target_page = bm.page_index;
                        let title = bm.title.clone();
                        items.push(
                            div()
                                .id(SharedString::from(format!("bm-item-{}", idx)))
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
                                        .label("×")
                                        .ghost()
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
                    .child(
                        div().flex().flex_row().gap_1().child(
                            Button::new("btn-exec-search")
                                .label(if is_searching {
                                    "Searching..."
                                } else {
                                    "Search"
                                })
                                .primary()
                                .on_click(cx.listener(move |v, _, w, cx| {
                                    on_action(v, SidebarAction::ExecuteSearch, w, cx)
                                })),
                        ),
                    )
                    .child(div().text_xs().text_color(muted_fg).child(if is_searching {
                        "Searching across document...".to_string()
                    } else if match_count > 0 {
                        format!("Found {} occurrences", match_count)
                    } else if !self.search_query.is_empty() {
                        "No matches found".to_string()
                    } else {
                        "Enter search query above".to_string()
                    }));

                let mut result_items = Vec::new();
                for (idx, item) in self.search_results.iter().enumerate() {
                    let page = item.page_index;
                    let snippet = item.text.clone();
                    let (x, y, w, h) = (item.x, item.y, item.width, item.height);

                    result_items.push(
                        div()
                            .id(SharedString::from(format!("search-res-{}", idx)))
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
                    for (idx, att) in self.attachments.iter().enumerate() {
                        let name = att.name.clone();
                        let size = att.size;
                        items.push(
                            div()
                                .id(SharedString::from(format!("att-{}", idx)))
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
                                            div()
                                                .text_xs()
                                                .text_color(muted_fg)
                                                .child(format!("{} bytes", size.unwrap_or(0))),
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
                        items.push(
                            div()
                                .id(SharedString::from(format!("layer-{}", idx)))
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
                                    Button::new(format!("btn-layer-{}", idx))
                                        .label(if is_vis { "Visible" } else { "Hidden" })
                                        .ghost()
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
                            .ghost()
                            .when(is_active, |b| b.primary())
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
