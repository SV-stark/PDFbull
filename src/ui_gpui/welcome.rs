use gpui_kit::component::ActiveTheme;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::*;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WelcomeAction {
    OpenFile,
    MergeFiles,
    PageOrganizer,
    DigitalSignatures,
    OpenRecent(usize),
}

pub struct WelcomeState {
    pub recent_files: Vec<PathBuf>,
}

impl Default for WelcomeState {
    fn default() -> Self {
        Self::new()
    }
}

impl WelcomeState {
    pub fn new() -> Self {
        Self {
            recent_files: Vec::new(),
        }
    }

    pub fn render<V: 'static>(
        &self,
        cx: &mut Context<V>,
        on_action: impl Fn(&mut V, WelcomeAction, &mut Window, &mut Context<V>) + 'static + Copy,
    ) -> AnyElement {
        let bg = cx.theme().background;
        let fg = cx.theme().foreground;
        let border = cx.theme().border;
        let muted = cx.theme().muted;
        let muted_fg = cx.theme().muted_foreground;

        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(bg)
            .items_center()
            .justify_center()
            .p_8()
            .gap_8()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap_2()
                    .child(div().text_3xl().text_color(fg).child("PDFbull"))
                    .child(
                        div()
                            .text_sm()
                            .text_color(muted_fg)
                            .child("A lightweight, GPU-accelerated PDF reader & editor"),
                    ),
            )
            .child(
                div()
                    .id("welcome-dropzone")
                    .w(px(540.0))
                    .h(px(160.0))
                    .rounded_xl()
                    .border_2()
                    .border_dashed()
                    .border_color(border)
                    .bg(muted)
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .gap_3()
                    .cursor_pointer()
                    .on_click(
                        cx.listener(move |v, _, w, cx| {
                            on_action(v, WelcomeAction::OpenFile, w, cx)
                        }),
                    )
                    .child(
                        div()
                            .text_base()
                            .text_color(fg)
                            .child("Drag & Drop PDF files here"),
                    )
                    .child(
                        Button::new("btn-welcome-browse")
                            .label("Browse Files...")
                            .primary()
                            .on_click(cx.listener(move |v, _, w, cx| {
                                on_action(v, WelcomeAction::OpenFile, w, cx)
                            })),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_row()
                    .gap_4()
                    .child(
                        Button::new("card-merge")
                            .label("Merge PDFs")
                            .outline()
                            .on_click(cx.listener(move |v, _, w, cx| {
                                on_action(v, WelcomeAction::MergeFiles, w, cx)
                            })),
                    )
                    .child(
                        Button::new("card-organizer")
                            .label("Page Organizer")
                            .outline()
                            .on_click(cx.listener(move |v, _, w, cx| {
                                on_action(v, WelcomeAction::PageOrganizer, w, cx)
                            })),
                    )
                    .child(
                        Button::new("card-signature")
                            .label("Sign Document")
                            .outline()
                            .on_click(cx.listener(move |v, _, w, cx| {
                                on_action(v, WelcomeAction::DigitalSignatures, w, cx)
                            })),
                    ),
            )
            .into_any_element()
    }
}
