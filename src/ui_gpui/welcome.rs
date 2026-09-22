use gpui_kit::base::StyledExt;
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
    Security,
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
        let accent = cx.theme().accent;

        let container = div()
            .flex()
            .flex_col()
            .flex_1()
            .w_full()
            .bg(bg)
            .items_center()
            .justify_center()
            .p_8()
            .gap_6()
            .child(
                // Hero Header with Brand Mark
                div().flex().flex_col().items_center().gap_3().child(
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap_3()
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .justify_center()
                                .size_12()
                                .rounded_xl()
                                .bg(primary)
                                .text_color(gpui_kit::white())
                                .text_xl()
                                .font_bold()
                                .child("PB"),
                        )
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .child(
                                    div()
                                        .flex()
                                        .flex_row()
                                        .items_center()
                                        .gap_2()
                                        .child(
                                            div()
                                                .text_3xl()
                                                .font_bold()
                                                .text_color(fg)
                                                .child("PDFbull"),
                                        )
                                        .child(
                                            div()
                                                .px_2()
                                                .py_0p5()
                                                .rounded_full()
                                                .bg(muted)
                                                .text_xs()
                                                .font_semibold()
                                                .text_color(muted_fg)
                                                .child(concat!("v", env!("CARGO_PKG_VERSION"))),
                                        ),
                                )
                                .child(
                                    div()
                                        .text_sm()
                                        .text_color(muted_fg)
                                        .child("High-Performance GPU-Accelerated PDF Studio"),
                                ),
                        ),
                ),
            )
            .child(
                // Interactive Dropzone
                div()
                    .id("welcome-dropzone")
                    .w(px(540.0))
                    .h(px(140.0))
                    .rounded_xl()
                    .border_2()
                    .border_dashed()
                    .border_color(border)
                    .bg(muted)
                    .hover(move |s| s.border_color(primary).bg(accent))
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
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap_2()
                            .text_base()
                            .font_medium()
                            .text_color(fg)
                            .child("📥")
                            .child("Drag & Drop PDF files here"),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap_2()
                            .child(
                                Button::new("btn-welcome-browse")
                                    .label("Browse Files...")
                                    .primary()
                                    .on_click(cx.listener(move |v, _, w, cx| {
                                        on_action(v, WelcomeAction::OpenFile, w, cx)
                                    })),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(muted_fg)
                                    .child("or drop anywhere on window"),
                            ),
                    ),
            )
            .child(
                // Feature Action Cards (2x2 Grid)
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .gap_3()
                            .child(render_action_card(
                                cx,
                                "card-open-browse",
                                "📂",
                                "Open Document",
                                "Browse and view PDF documents from disk",
                                WelcomeAction::OpenFile,
                                on_action,
                            ))
                            .child(render_action_card(
                                cx,
                                "card-page-organizer",
                                "📑",
                                "Page Organizer",
                                "Reorder, rotate, extract, or delete pages",
                                WelcomeAction::PageOrganizer,
                                on_action,
                            )),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .gap_3()
                            .child(render_action_card(
                                cx,
                                "card-digital-signatures",
                                "✍️",
                                "Digital Signatures",
                                "Certify and sign PDFs with PKCS#12 keys",
                                WelcomeAction::DigitalSignatures,
                                on_action,
                            ))
                            .child(render_action_card(
                                cx,
                                "card-protect-encrypt",
                                "🔒",
                                "Protect & Encrypt",
                                "Manage passwords, permissions & AES security",
                                WelcomeAction::Security,
                                on_action,
                            )),
                    ),
            );

        // Recent Documents Shelf
        let recent_section = if !self.recent_files.is_empty() {
            let mut items = Vec::new();
            for (idx, p) in self.recent_files.iter().take(4).enumerate() {
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
                        .rounded_lg()
                        .cursor_pointer()
                        .hover(move |s| s.bg(accent))
                        .on_click(cx.listener(move |v, _, w, cx| {
                            on_action(v, WelcomeAction::OpenRecent(idx), w, cx);
                        }))
                        .child(
                            div()
                                .flex()
                                .flex_row()
                                .items_center()
                                .gap_3()
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .size_7()
                                        .rounded_md()
                                        .bg(muted)
                                        .text_sm()
                                        .child("📄"),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .flex_col()
                                        .gap_0p5()
                                        .child(
                                            div()
                                                .text_sm()
                                                .font_medium()
                                                .text_color(fg)
                                                .child(file_name),
                                        )
                                        .child(
                                            div().text_xs().text_color(muted_fg).child(path_str),
                                        ),
                                ),
                        )
                        .child(
                            div()
                                .text_xs()
                                .font_medium()
                                .text_color(muted_fg)
                                .child("Open →"),
                        ),
                );
            }

            Some(
                div()
                    .w(px(540.0))
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .text_xs()
                                    .font_semibold()
                                    .text_color(muted_fg)
                                    .child("RECENT DOCUMENTS"),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .p_1p5()
                            .rounded_xl()
                            .border_1()
                            .border_color(border)
                            .bg(cx.theme().group_box)
                            .children(items),
                    ),
            )
        } else {
            None
        };

        container.children(recent_section).into_any_element()
    }
}

fn render_action_card<V: 'static>(
    cx: &mut Context<V>,
    id: &'static str,
    icon: &'static str,
    title: &'static str,
    description: &'static str,
    action: WelcomeAction,
    on_action: impl Fn(&mut V, WelcomeAction, &mut Window, &mut Context<V>) + 'static + Copy,
) -> AnyElement {
    let fg = cx.theme().foreground;
    let border = cx.theme().border;
    let muted_fg = cx.theme().muted_foreground;
    let group_box = cx.theme().group_box;
    let primary = cx.theme().primary;
    let accent = cx.theme().accent;

    div()
        .id(id)
        .flex()
        .flex_col()
        .w(px(264.0))
        .p_3p5()
        .rounded_xl()
        .border_1()
        .border_color(border)
        .bg(group_box)
        .gap_1p5()
        .cursor_pointer()
        .hover(move |s| s.bg(accent).border_color(primary))
        .on_click(cx.listener(move |v, _, w, cx| {
            on_action(v, action.clone(), w, cx);
        }))
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap_2p5()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_center()
                        .size_8()
                        .rounded_lg()
                        .bg(accent)
                        .text_base()
                        .child(icon),
                )
                .child(div().text_sm().font_semibold().text_color(fg).child(title)),
        )
        .child(div().text_xs().text_color(muted_fg).child(description))
        .into_any_element()
}
