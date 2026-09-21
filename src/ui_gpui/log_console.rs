use gpui_kit::component::ActiveTheme;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogAction {
    ToggleOpen,
    Clear,
    CopyAll,
    SetFilter(&'static str),
}

pub struct LogConsoleState {
    pub is_open: bool,
    pub height: f32,
    pub filter_level: Option<&'static str>,
}

impl LogConsoleState {
    pub fn new() -> Self {
        Self {
            is_open: false,
            height: 180.0,
            filter_level: None,
        }
    }

    pub fn render<V: 'static>(
        &self,
        cx: &mut Context<V>,
        on_action: impl Fn(&mut V, LogAction, &mut Window, &mut Context<V>) + 'static + Copy,
    ) -> Option<AnyElement> {
        if !self.is_open {
            return None;
        }

        let border = cx.theme().border;
        let bg = cx.theme().background;
        let muted = cx.theme().muted;
        let fg = cx.theme().foreground;
        let muted_fg = cx.theme().muted_foreground;

        Some(
            div()
                .flex()
                .flex_col()
                .h(px(self.height))
                .border_t_1()
                .border_color(border)
                .bg(bg)
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .justify_between()
                        .h_8()
                        .px_3()
                        .bg(muted)
                        .border_b_1()
                        .border_color(border)
                        .child(
                            div()
                                .flex()
                                .flex_row()
                                .items_center()
                                .gap_2()
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(fg)
                                        .child("Developer Log Console"),
                                )
                                .child(Button::new("log-filter-all").label("All").ghost().on_click(
                                    cx.listener(move |v, _, w, cx| {
                                        on_action(v, LogAction::SetFilter("all"), w, cx)
                                    }),
                                ))
                                .child(
                                    Button::new("log-filter-info")
                                        .label("Info")
                                        .ghost()
                                        .on_click(cx.listener(move |v, _, w, cx| {
                                            on_action(v, LogAction::SetFilter("info"), w, cx)
                                        })),
                                )
                                .child(
                                    Button::new("log-filter-warn")
                                        .label("Warn")
                                        .ghost()
                                        .on_click(cx.listener(move |v, _, w, cx| {
                                            on_action(v, LogAction::SetFilter("warn"), w, cx)
                                        })),
                                )
                                .child(
                                    Button::new("log-filter-error")
                                        .label("Error")
                                        .ghost()
                                        .on_click(cx.listener(move |v, _, w, cx| {
                                            on_action(v, LogAction::SetFilter("error"), w, cx)
                                        })),
                                ),
                        )
                        .child(
                            div()
                                .flex()
                                .flex_row()
                                .gap_1()
                                .child(Button::new("log-copy").label("Copy All").ghost().on_click(
                                    cx.listener(move |v, _, w, cx| {
                                        on_action(v, LogAction::CopyAll, w, cx)
                                    }),
                                ))
                                .child(Button::new("log-clear").label("Clear").ghost().on_click(
                                    cx.listener(move |v, _, w, cx| {
                                        on_action(v, LogAction::Clear, w, cx)
                                    }),
                                ))
                                .child(Button::new("log-close").label("×").ghost().on_click(
                                    cx.listener(move |v, _, w, cx| {
                                        on_action(v, LogAction::ToggleOpen, w, cx)
                                    }),
                                )),
                        ),
                )
                .child(
                    div()
                        .flex_1()
                        .p_3()
                        .overflow_hidden()
                        .text_xs()
                        .text_color(muted_fg)
                        .child("[INFO] PDFbull GPUI Engine initialized."),
                )
                .into_any_element(),
        )
    }
}
