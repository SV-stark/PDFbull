use super::sidebar::SidebarState;
use gpui_kit::base::StyledExt;
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::Sizable as _;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::scroll::ScrollableElement;
use gpui_kit::component::{Icon, IconName};
use gpui_kit::*;
use std::path::PathBuf;

/// Icons are Lucide vectors from the component bundle. This screen previously
/// used emoji (📂 📑 ✍️ 🔒 📄 📥), which render differently on every platform,
/// ignore the theme's foreground colour, and cannot dim with a row. One icon
/// family, themeable, everywhere.
const ICON_DROP: IconName = IconName::Inbox;
const ICON_OPEN: IconName = IconName::FolderOpen;
const ICON_ORGANIZE: IconName = IconName::FileText;
const ICON_SIGN: IconName = IconName::Check;
const ICON_PROTECT: IconName = IconName::Ban;
const ICON_FILE: IconName = IconName::FileText;

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
            // `is_file()` rather than `exists()`: a directory or a socket in the
            // recent list used to produce a clickable entry that could never
            // open anything.
            .filter(|p| p.is_file())
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

        let container =
            div()
                .flex()
                .flex_col()
                .items_center()
                .min_h_full()
                .w_full()
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
                                    .text_color(cx.theme().primary_foreground)
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
                                                    .rounded_md()
                                                    .border_1()
                                                    .border_color(border)
                                                    .bg(muted)
                                                    .text_xs()
                                                    .font_medium()
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
                        .on_click(cx.listener(move |v, _, w, cx| {
                            on_action(v, WelcomeAction::OpenFile, w, cx)
                        }))
                        .on_drop(cx.listener(move |v, paths: &ExternalPaths, w, cx| {
                            // `|| p.is_file()` made the extension test redundant, so
                            // dropping a PNG created a permanent unrenderable tab.
                            let pdf_paths: Vec<PathBuf> = paths
                                .paths()
                                .iter()
                                .filter(|p| SidebarState::is_openable_pdf(p))
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
                                .child(Icon::new(ICON_DROP).text_color(fg))
                                .child("Drag & drop PDF files here"),
                        )
                        .child(
                            div()
                                .flex()
                                .flex_row()
                                .items_center()
                                .gap_2()
                                .child(
                                    Button::new("btn-welcome-browse")
                                        .label("Browse files…")
                                        .primary()
                                        // The Browse button is a *descendant* of the
                                        // clickable dropzone, and GPUI dispatches
                                        // bubble-phase listeners child-first. Without
                                        // this, one click fired both handlers and
                                        // opened two native file dialogs at once.
                                        .on_click(cx.listener(move |v, _, w, cx| {
                                            cx.stop_propagation();
                                            on_action(v, WelcomeAction::OpenFile, w, cx)
                                        })),
                                )
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(muted_fg)
                                        .child("or drop anywhere on this window"),
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
                                    ICON_OPEN,
                                    "Open Document",
                                    "Browse and view PDF documents from disk",
                                    WelcomeAction::OpenFile,
                                    on_action,
                                ))
                                .child(render_action_card(
                                    cx,
                                    "card-page-organizer",
                                    ICON_ORGANIZE,
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
                                    ICON_SIGN,
                                    "Digital Signatures",
                                    "Review signature and certificate trust status",
                                    WelcomeAction::DigitalSignatures,
                                    on_action,
                                ))
                                .child(render_action_card(
                                    cx,
                                    "card-protect-encrypt",
                                    ICON_PROTECT,
                                    "Protect & Encrypt",
                                    "Set passwords and restrict printing or copying",
                                    WelcomeAction::Security,
                                    on_action,
                                )),
                        ),
                );

        // Recent Documents Shelf
        let recent_section = if !self.recent_files.is_empty() {
            let mut items = Vec::new();
            for p in self.recent_files.iter().take(4) {
                let file_name = p
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("Document.pdf")
                    .to_string();
                let path_str = p.to_string_lossy().to_string();
                // Keyed on the path, not the list position. The recent list is
                // re-sorted most-recent-first on every open, so a positional id
                // handed one file's hover state to a different file.
                let key = format!("recent-item-{}", path_str);
                let idx = self
                    .recent_files
                    .iter()
                    .take(4)
                    .position(|candidate| candidate == p)
                    .unwrap_or(0);

                // A real Button rather than a clickable div: the previous row was
                // a bare `on_click` div, so it could not be reached with Tab and
                // could not be activated with Enter or Space. The trailing "Open"
                // text was decorative rather than an actual control.
                items.push(
                    Button::new(SharedString::from(key))
                        .w_full()
                        .ghost()
                        .h_auto()
                        .px_3()
                        .py_2()
                        .rounded_lg()
                        .justify_start()
                        .on_click(cx.listener(move |v, _, w, cx| {
                            on_action(v, WelcomeAction::OpenRecent(idx), w, cx);
                        }))
                        .child(
                            div()
                                .flex()
                                .flex_row()
                                .items_center()
                                .gap_3()
                                .w_full()
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .size_7()
                                        .rounded_md()
                                        .bg(muted)
                                        .child(Icon::new(ICON_FILE).text_color(muted_fg)),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .flex_col()
                                        .items_start()
                                        .gap_0p5()
                                        .min_w_0()
                                        .flex_1()
                                        .child(
                                            div()
                                                .text_sm()
                                                .font_medium()
                                                .text_color(fg)
                                                .child(file_name),
                                        )
                                        .child(
                                            div()
                                                .text_xs()
                                                .text_color(muted_fg)
                                                .truncate()
                                                .child(path_str),
                                        ),
                                )
                                .child(
                                    Icon::new(IconName::ArrowRight)
                                        .small()
                                        .flex_none()
                                        .text_color(muted_fg),
                                ),
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
                                    .child("Recent documents"),
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

        div()
            .id("welcome-scroll-container")
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .size_full()
            .bg(bg)
            .overflow_y_scrollbar()
            .child(container.children(recent_section))
            .into_any_element()
    }
}

fn render_action_card<V: 'static>(
    cx: &mut Context<V>,
    id: &'static str,
    icon: IconName,
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
                        .child(Icon::new(icon).text_color(primary)),
                )
                .child(div().text_sm().font_semibold().text_color(fg).child(title)),
        )
        .child(div().text_xs().text_color(muted_fg).child(description))
        .into_any_element()
}
