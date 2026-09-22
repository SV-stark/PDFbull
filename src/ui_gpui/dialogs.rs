use gpui_kit::component::ActiveTheme;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActiveDialog {
    Watermark,
    HeaderFooter,
    Security,
    Password,
    Signature,
    PageOrganizer,
    Settings,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DialogAction {
    SetWatermark(String),
    SetHeaderFooter(String, String),
    SetPassword(String),
    RotatePages(i32),
    DeleteCurrentPage,
    ApplySecurity(String),
    Close,
}

pub struct DialogsState {
    pub active: Option<ActiveDialog>,
    pub selected_watermark: String,
    pub header_text: String,
    pub footer_text: String,
    pub password_input: String,
}

impl Default for DialogsState {
    fn default() -> Self {
        Self::new()
    }
}

impl DialogsState {
    pub fn new() -> Self {
        Self {
            active: None,
            selected_watermark: "CONFIDENTIAL".to_string(),
            header_text: "Confidential Document".to_string(),
            footer_text: "Page %PAGE% of %TOTAL%".to_string(),
            password_input: String::new(),
        }
    }

    pub fn render_overlay<V: 'static>(
        &self,
        cx: &mut Context<V>,
        on_close: impl Fn(&mut V, &mut Window, &mut Context<V>) + 'static + Copy,
        on_action: impl Fn(&mut V, DialogAction, &mut Window, &mut Context<V>) + 'static + Copy,
    ) -> Option<AnyElement> {
        let active = self.active.clone()?;
        let bg = cx.theme().background;
        let border = cx.theme().border;
        let fg = cx.theme().foreground;
        let muted = cx.theme().muted;
        let muted_fg = cx.theme().muted_foreground;
        let _primary = cx.theme().primary;

        let (title, description): (&str, &str) = match active {
            ActiveDialog::Watermark => (
                "Add Watermark",
                "Apply a prominent diagonal watermark across document pages.",
            ),
            ActiveDialog::HeaderFooter => (
                "Header & Page Numbers",
                "Configure running headers and footer page numbers.",
            ),
            ActiveDialog::Security => (
                "Security & Permissions",
                "Encrypt document with password and restrict printing/copying.",
            ),
            ActiveDialog::Password => (
                "Document Password",
                "This document is encrypted. Enter password to unlock.",
            ),
            ActiveDialog::Signature => (
                "Digital Signature",
                "Place a cryptographic digital signature on the document.",
            ),
            ActiveDialog::PageOrganizer => {
                ("Visual Page Organizer", "Rotate or remove document pages.")
            }
            ActiveDialog::Settings => (
                "Settings & Preferences",
                "Configure theme, default zoom, and startup behavior.",
            ),
        };

        let cur_watermark = self.selected_watermark.clone();
        let cur_header = self.header_text.clone();
        let cur_footer = self.footer_text.clone();

        let panel_body: AnyElement = match active {
            ActiveDialog::Watermark => {
                let presets = [
                    "CONFIDENTIAL",
                    "DRAFT",
                    "SAMPLE",
                    "DO NOT COPY",
                    "URGENT",
                    "ORIGINAL",
                ];
                let mut chips = Vec::new();
                for text in presets {
                    let is_sel = cur_watermark == text;
                    chips.push(
                        Button::new(format!("wm-chip-{}", text))
                            .label(text)
                            .ghost()
                            .when(is_sel, |b| b.primary())
                            .on_click(cx.listener(move |v, _, w, cx| {
                                on_action(v, DialogAction::SetWatermark(text.to_string()), w, cx)
                            })),
                    );
                }

                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(
                        div()
                            .text_xs()
                            .text_color(muted_fg)
                            .child("Select watermark text stamp:"),
                    )
                    .child(div().flex().flex_row().flex_wrap().gap_2().children(chips))
                    .into_any_element()
            }

            ActiveDialog::HeaderFooter => div()
                .flex()
                .flex_col()
                .gap_3()
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .child(div().text_xs().text_color(muted_fg).child("Header Text:"))
                        .child(
                            div()
                                .p_2()
                                .bg(muted)
                                .rounded_md()
                                .border_1()
                                .border_color(border)
                                .text_xs()
                                .text_color(fg)
                                .child(cur_header.clone()),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .child(div().text_xs().text_color(muted_fg).child("Footer Text:"))
                        .child(
                            div()
                                .p_2()
                                .bg(muted)
                                .rounded_md()
                                .border_1()
                                .border_color(border)
                                .text_xs()
                                .text_color(fg)
                                .child(cur_footer.clone()),
                        ),
                )
                .into_any_element(),

            ActiveDialog::Security => div()
                .flex()
                .flex_col()
                .gap_3()
                .child(
                    div()
                        .text_xs()
                        .text_color(muted_fg)
                        .child("Choose encryption standard:"),
                )
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .gap_2()
                        .child(
                            Button::new("sec-aes-256")
                                .label("AES-256 (High Security)")
                                .primary()
                                .on_click(cx.listener(move |v, _, w, cx| {
                                    on_action(
                                        v,
                                        DialogAction::ApplySecurity("AES-256".to_string()),
                                        w,
                                        cx,
                                    )
                                })),
                        )
                        .child(
                            Button::new("sec-aes-128")
                                .label("AES-128 (Standard)")
                                .outline()
                                .on_click(cx.listener(move |v, _, w, cx| {
                                    on_action(
                                        v,
                                        DialogAction::ApplySecurity("AES-128".to_string()),
                                        w,
                                        cx,
                                    )
                                })),
                        ),
                )
                .into_any_element(),

            ActiveDialog::PageOrganizer => div()
                .flex()
                .flex_col()
                .gap_3()
                .child(
                    div()
                        .text_xs()
                        .text_color(muted_fg)
                        .child("Page Actions for current document:"),
                )
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .flex_wrap()
                        .gap_2()
                        .child(
                            Button::new("btn-rot-cw")
                                .label("Rotate 90° CW")
                                .outline()
                                .on_click(cx.listener(move |v, _, w, cx| {
                                    on_action(v, DialogAction::RotatePages(90), w, cx)
                                })),
                        )
                        .child(
                            Button::new("btn-rot-ccw")
                                .label("Rotate 90° CCW")
                                .outline()
                                .on_click(cx.listener(move |v, _, w, cx| {
                                    on_action(v, DialogAction::RotatePages(-90), w, cx)
                                })),
                        )
                        .child(
                            Button::new("btn-rot-180")
                                .label("Rotate 180°")
                                .outline()
                                .on_click(cx.listener(move |v, _, w, cx| {
                                    on_action(v, DialogAction::RotatePages(180), w, cx)
                                })),
                        )
                        .child(
                            Button::new("btn-del-page")
                                .label("Delete Current Page")
                                .danger()
                                .on_click(cx.listener(move |v, _, w, cx| {
                                    on_action(v, DialogAction::DeleteCurrentPage, w, cx)
                                })),
                        ),
                )
                .into_any_element(),

            _ => div()
                .h_24()
                .bg(muted)
                .rounded_md()
                .flex()
                .items_center()
                .justify_center()
                .text_sm()
                .text_color(muted_fg)
                .child(format!("{} Configuration", title))
                .into_any_element(),
        };

        let dialog_el = div()
            .flex()
            .flex_col()
            .w(px(480.0))
            .bg(bg)
            .border_1()
            .border_color(border)
            .rounded_lg()
            .shadow_xl()
            .p_6()
            .gap_4()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(div().text_lg().text_color(fg).child(title))
                    .child(div().text_sm().text_color(muted_fg).child(description)),
            )
            .child(panel_body)
            .child(
                div()
                    .flex()
                    .flex_row()
                    .justify_end()
                    .gap_2()
                    .child(
                        Button::new("btn-dialog-cancel")
                            .label("Close")
                            .outline()
                            .on_click(cx.listener(move |v, _, w, cx| on_close(v, w, cx))),
                    )
                    .child(
                        Button::new("btn-dialog-apply")
                            .label("Apply")
                            .primary()
                            .on_click(cx.listener(move |v, _, w, cx| match active {
                                ActiveDialog::Watermark => on_action(
                                    v,
                                    DialogAction::SetWatermark(cur_watermark.clone()),
                                    w,
                                    cx,
                                ),
                                ActiveDialog::HeaderFooter => on_action(
                                    v,
                                    DialogAction::SetHeaderFooter(
                                        cur_header.clone(),
                                        cur_footer.clone(),
                                    ),
                                    w,
                                    cx,
                                ),
                                ActiveDialog::Security => on_action(
                                    v,
                                    DialogAction::ApplySecurity("AES-256".to_string()),
                                    w,
                                    cx,
                                ),
                                _ => on_close(v, w, cx),
                            })),
                    ),
            );

        // Render full screen backdrop with centered modal
        Some(
            div()
                .absolute()
                .size_full()
                .bg(gpui_kit::rgba(0x00000080))
                .flex()
                .items_center()
                .justify_center()
                .child(dialog_el)
                .into_any_element(),
        )
    }
}
