use gpui_kit::component::ActiveTheme;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::*;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WelcomeAction {
    OpenFile,
    DropFiles(Vec<PathBuf>),
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

    pub fn load_recent_files(&mut self) {
        let loaded = crate::storage::load_recent_files();
        self.recent_files = loaded
            .into_iter()
            .map(|f| PathBuf::from(f.path))
            .filter(|p| p.exists())
            .collect();
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
        let primary = cx.theme().primary;

        let container = div()
            .flex()
            .flex_col()
            .flex_1()
            .w_full()
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
                    .hover(move |s| s.border_color(primary))
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
                    .on_drop(cx.listener(move |v, paths: &ExternalPaths, w, cx| {
                        let pdf_paths: Vec<PathBuf> = paths
                            .paths()
                            .iter()
                            .filter(|p| {
                                p.to_string_lossy().to_lowercase().ends_with(".pdf") || p.is_file()
                            })
                            .cloned()
                            .collect();
                        if !pdf_paths.is_empty() {
                            on_action(v, WelcomeAction::DropFiles(pdf_paths), w, cx);
                        }
                    }))
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
            );

        let recent_section = if !self.recent_files.is_empty() {
            let mut items = Vec::new();
            for (idx, p) in self.recent_files.iter().take(5).enumerate() {
                let file_name = p
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("Document.pdf")
                    .to_string();
                let path_str = p.to_string_lossy().to_string();

                items.push(
                    div()
                        .id(SharedString::from(format!("recent-item-{}", idx)))
                        .flex()
                        .flex_row()
                        .items_center()
                        .justify_between()
                        .w_full()
                        .px_3()
                        .py_2()
                        .rounded_md()
                        .cursor_pointer()
                        .hover(move |s| s.bg(muted))
                        .on_click(cx.listener(move |v, _, w, cx| {
                            on_action(v, WelcomeAction::OpenRecent(idx), w, cx);
                        }))
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap_1()
                                .child(div().text_sm().text_color(fg).child(file_name))
                                .child(div().text_xs().text_color(muted_fg).child(path_str)),
                        ),
                );
            }

            Some(
                div()
                    .w(px(540.0))
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(div().text_xs().text_color(muted_fg).child("RECENT FILES"))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .p_2()
                            .rounded_lg()
                            .border_1()
                            .border_color(border)
                            .children(items),
                    ),
            )
        } else {
            None
        };

        container.children(recent_section).into_any_element()
    }
}
