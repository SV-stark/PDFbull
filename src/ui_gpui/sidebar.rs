use gpui_kit::component::ActiveTheme;
use gpui_kit::component::button::{Button, ButtonVariants};
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidebarAction {
    ToggleOpen,
    SelectMode(SidebarMode),
    SelectPage(usize),
}

pub struct SidebarState {
    pub is_open: bool,
    pub mode: SidebarMode,
    pub width: f32,
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
        }
    }

    pub fn render<V: 'static>(
        &self,
        cx: &mut Context<V>,
        total_pages: usize,
        current_page: usize,
        on_action: impl Fn(&mut V, SidebarAction, &mut Window, &mut Context<V>) + 'static + Copy,
    ) -> AnyElement {
        let primary = cx.theme().primary;
        let border = cx.theme().border;
        let accent = cx.theme().accent;
        let background = cx.theme().background;
        let muted = cx.theme().muted;
        let muted_fg = cx.theme().muted_foreground;
        let _fg = cx.theme().foreground;

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
                            .child(
                                div()
                                    .w_32()
                                    .h(px(160.0))
                                    .bg(muted)
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .text_xs()
                                    .text_color(muted_fg)
                                    .child(format!("Page {}", page_idx + 1)),
                            )
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
                    .overflow_hidden()
                    .children(thumb_cards)
                    .into_any_element()
            }

            SidebarMode::Bookmarks => div()
                .flex()
                .flex_col()
                .p_3()
                .text_sm()
                .text_color(muted_fg)
                .child("Document Outline / Bookmarks")
                .into_any_element(),

            SidebarMode::Annotations => div()
                .flex()
                .flex_col()
                .p_3()
                .text_sm()
                .text_color(muted_fg)
                .child("Annotations & Comments")
                .into_any_element(),

            SidebarMode::Search => div()
                .flex()
                .flex_col()
                .p_3()
                .gap_2()
                .child(
                    div()
                        .text_sm()
                        .text_color(muted_fg)
                        .child("Fuzzy Content Search"),
                )
                .into_any_element(),

            SidebarMode::Attachments => div()
                .flex()
                .flex_col()
                .p_3()
                .text_sm()
                .text_color(muted_fg)
                .child("Embedded Attachments")
                .into_any_element(),

            SidebarMode::Layers => div()
                .flex()
                .flex_col()
                .p_3()
                .text_sm()
                .text_color(muted_fg)
                .child("PDF Optional Content Layers")
                .into_any_element(),
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
                    let mut btn =
                        Button::new(SharedString::from(format!("side-mode-{:?}", m))).label(label);
                    if is_active {
                        btn = btn.primary();
                    } else {
                        btn = btn.ghost();
                    }
                    mode_buttons.push(btn.on_click(click_listener));
                }

                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .justify_between()
                    .h_9()
                    .px_2()
                    .border_b_1()
                    .border_color(border)
                    .bg(muted)
                    .child(div().flex().flex_row().gap_1().children(mode_buttons))
                    .child(
                        Button::new("btn-collapse-sidebar")
                            .label("◀")
                            .ghost()
                            .on_click(cx.listener(move |v, _, w, cx| {
                                on_action(v, SidebarAction::ToggleOpen, w, cx)
                            })),
                    )
            })
            .child(div().flex_1().overflow_hidden().child(content))
            .into_any_element()
    }
}
