use gpui_kit::component::ActiveTheme;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::scroll::ScrollableElement;
use gpui_kit::prelude::FluentBuilder;
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
    pub entries: Vec<(String, &'static str)>,
}

impl Default for LogConsoleState {
    fn default() -> Self {
        Self::new()
    }
}

impl LogConsoleState {
    pub fn new() -> Self {
        Self {
            is_open: false,
            height: 180.0,
            filter_level: None,
            entries: vec![
                (
                    concat!(
                        "[INFO] PDFbull GPUI Engine v",
                        env!("CARGO_PKG_VERSION"),
                        " initialized."
                    )
                    .to_string(),
                    "info",
                ),
                (
                    "[INFO] GPU acceleration active via DirectX 11 / tiny-skia.".to_string(),
                    "info",
                ),
            ],
        }
    }

    pub fn log(&mut self, msg: impl Into<String>, level: &'static str) {
        self.entries.push((msg.into(), level));
        if self.entries.len() > 1000 {
            self.entries.remove(0);
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
        let cur_filter = self.filter_level.unwrap_or("all");

        let filtered_entries: Vec<AnyElement> = self
            .entries
            .iter()
            .filter(|(_, lvl)| {
                if cur_filter == "all" {
                    true
                } else {
                    lvl == &cur_filter
                }
            })
            .enumerate()
            .map(|(idx, (msg, lvl))| {
                let color = match *lvl {
                    "error" => gpui_kit::red(),
                    "warn" => gpui_kit::yellow(),
                    _ => muted_fg,
                };
                div()
                    .id(SharedString::from(format!("log-entry-{}", idx)))
                    .text_xs()
                    .text_color(color)
                    .child(msg.clone())
                    .into_any_element()
            })
            .collect();

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
                                .child(
                                    Button::new("log-filter-all")
                                        .label("All")
                                        .ghost()
                                        .when(cur_filter == "all", |b| b.primary())
                                        .on_click(cx.listener(move |v, _, w, cx| {
                                            on_action(v, LogAction::SetFilter("all"), w, cx)
                                        })),
                                )
                                .child(
                                    Button::new("log-filter-info")
                                        .label("Info")
                                        .ghost()
                                        .when(cur_filter == "info", |b| b.primary())
                                        .on_click(cx.listener(move |v, _, w, cx| {
                                            on_action(v, LogAction::SetFilter("info"), w, cx)
                                        })),
                                )
                                .child(
                                    Button::new("log-filter-warn")
                                        .label("Warn")
                                        .ghost()
                                        .when(cur_filter == "warn", |b| b.primary())
                                        .on_click(cx.listener(move |v, _, w, cx| {
                                            on_action(v, LogAction::SetFilter("warn"), w, cx)
                                        })),
                                )
                                .child(
                                    Button::new("log-filter-error")
                                        .label("Error")
                                        .ghost()
                                        .when(cur_filter == "error", |b| b.primary())
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
                        .overflow_y_scrollbar()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .children(filtered_entries),
                )
                .into_any_element(),
        )
    }
}
