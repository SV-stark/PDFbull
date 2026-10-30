use super::sidebar::{self, SidebarState};
use gpui_kit::assets::IconName;
use gpui_kit::base::StyledExt;
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::Icon;
use gpui_kit::component::Sizable as _;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::scroll::ScrollableElement;
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;
use std::path::PathBuf;

/// Icons are Lucide vectors from the component bundle. This screen previously
/// used emoji (📂 📑 ✍️ 🔒 📄 📥), which render differently on every platform,
/// ignore the theme's foreground colour, and cannot dim with a row. One icon
/// family, themeable, everywhere.
const ICON_DROP: IconName = IconName::Inbox;
const ICON_MERGE: IconName = IconName::Merge;
const ICON_ORGANIZE: IconName = IconName::PencilRuler;
const ICON_SIGN: IconName = IconName::Signature;
const ICON_PROTECT: IconName = IconName::ShieldLock;
const ICON_FILE: IconName = IconName::FileText;

/// Only the first few recents are probed for metadata. These are the caps that
/// keep the dashboard's first paint cheap: `inspect_pdf_geometry` re-parses the
/// file with `lopdf`, which is linear in file size, so a 500 MB scan job would
/// freeze the welcome screen for nothing a user can perceptibly use.
const PAGE_COUNT_SCAN_COUNT: usize = 8;
const PAGE_COUNT_SCAN_MAX_BYTES: u64 = 20 * 1024 * 1024;

/// A recent document plus the metadata the recents list shows inline.
///
/// The old list rendered a bare `PathBuf` per row: no timestamp, no size, no
/// page count, so the "recents" area only ever told you a file name you already
/// knew when you last looked at this screen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecentEntry {
    pub path: PathBuf,
    pub name: String,
    /// Unix timestamp, seconds.
    pub last_opened: u64,
    pub size_bytes: u64,
    /// `None` when unknown (probing skipped, failed, or unavailable).
    pub page_count: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WelcomeAction {
    OpenFile,
    DropFiles(Vec<PathBuf>),
    MergeFiles,
    PageOrganizer,
    DigitalSignatures,
    Security,
    OpenRecent(usize),
    Settings,
    About,
}

pub struct WelcomeState {
    pub recent_files: Vec<RecentEntry>,
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

    /// Initial load without the expensive synchronous `lopdf` rescan on the UI thread.
    pub fn load_recent_files(&mut self) {
        self.load_recent_files_impl(false)
    }

    /// List refresh without the expensive `lopdf` rescan. The welcome screen
    /// reloads on every open/toggle, and probing eight PDFs per paint burst is
    /// how a "fast dashboard" becomes a beached one; prior page counts are
    /// kept by path instead.
    pub fn refresh_recent_files(&mut self) {
        self.load_recent_files_impl(false)
    }

    fn load_recent_files_impl(&mut self, scan_page_counts: bool) {
        let loaded = crate::storage::load_recent_files();
        self.recent_files = loaded
            .into_iter()
            .map(|f| {
                let path = PathBuf::from(&f.path);
                // `is_file()` rather than `exists()`: a directory or a socket in
                // the recent list used to produce a clickable entry that could
                // never open anything.
                let size_bytes = if path.is_file() {
                    std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0)
                } else {
                    0
                };
                RecentEntry {
                    path: path.clone(),
                    name: if f.name.is_empty() {
                        path.file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("Document.pdf")
                            .to_string()
                    } else {
                        f.name
                    },
                    last_opened: f.last_opened,
                    size_bytes,
                    page_count: None,
                }
            })
            .filter(|e| e.path.is_file())
            .collect();

        if scan_page_counts {
            for entry in self.recent_files.iter_mut().take(PAGE_COUNT_SCAN_COUNT) {
                if entry.size_bytes > PAGE_COUNT_SCAN_MAX_BYTES {
                    continue;
                }
                if let Some((pages, _, _)) =
                    crate::ui_gpui::app_view::inspect_pdf_geometry(&entry.path)
                {
                    entry.page_count = Some(pages);
                }
            }
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
                                    // The hero tile used to hold the literal
                                    // letters "PB", which repeated the name
                                    // beside it one syllable later. A document
                                    // glyph says what the app is; the wordmark
                                    // keeps the brand.
                                    .child(Icon::new(ICON_FILE).large()),
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
                        .w_full()
                        .max_w(px(540.0))
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
                    // Feature Action Cards (2x2 grid) — the quick actions a
                    // person actually opens a PDF studio for. "Open document"
                    // used to take one of the four slots while the dropzone one
                    // row above already had the same job covered by *two*
                    // controls; the redundant card is gone and its single
                    // purpose lives in the dropzone it duplicated.
                    div()
                        .flex()
                        .flex_wrap()
                        .justify_center()
                        .gap_3()
                        .max_w(px(680.0))
                        .child(render_action_card(
                            cx,
                            "card-merge-pdfs",
                            ICON_MERGE,
                            "Merge PDFs",
                            "Combine multiple documents into one file",
                            WelcomeAction::MergeFiles,
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
                        ))
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
                );

        // Recent Documents — a row list, not a shelf of cards. Name in the
        // primary column, then when it was touched, how big it is and how
        // many pages it has; the path is demoted to a muted, truncated
        // second line. The old rows showed only a name and a full-length
        // path, and dresses the group in a bordered "card" that just boxed
        // the border it already had.
        let recent_section = if !self.recent_files.is_empty() {
            let mut items = Vec::new();
            for (idx, entry) in self.recent_files.iter().enumerate().take(6) {
                let path_str = entry.path.to_string_lossy().to_string();
                // Keyed on the path, not the list position. The recent list is
                // re-sorted most-recent-first on every open, so a positional id
                // handed one file's hover state to a different file.
                let key = format!("recent-item-{}", path_str);
                let time_label = crate::storage::time_ago(entry.last_opened);
                let size_label = sidebar::human_bytes(entry.size_bytes);
                let pages_label = match entry.page_count {
                    Some(n) => format!("{} pg", n),
                    None => String::new(),
                };

                // A real Button rather than a clickable div: the previous row was
                // a bare `on_click` div, so it could not be reached with Tab and
                // could not be activated with Enter or Space. The trailing "Open"
                // text was decorative rather than an actual control.
                items.push(
                    Button::new(SharedString::from(key))
                        .w_full()
                        .ghost()
                        .h_auto()
                        .px_2()
                        .py_1p5()
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
                                                .truncate()
                                                .max_w_full()
                                                .child(entry.name.clone()),
                                        )
                                        .child(
                                            div()
                                                .text_xs()
                                                .text_color(muted_fg)
                                                .truncate()
                                                .max_w_full()
                                                .child(path_str),
                                        ),
                                )
                                .child(
                                    // Meta column: kept out of the primary
                                    // column so it stays scannable as a table.
                                    div()
                                        .flex()
                                        .flex_row()
                                        .items_center()
                                        .gap_3()
                                        .flex_none()
                                        .child(
                                            div().text_xs().text_color(muted_fg).child(time_label),
                                        )
                                        .when(!size_label.is_empty(), |el| {
                                            el.child(
                                                div()
                                                    .text_xs()
                                                    .text_color(muted_fg)
                                                    .child(size_label),
                                            )
                                        })
                                        .when(!pages_label.is_empty(), |el| {
                                            el.child(
                                                div()
                                                    .text_xs()
                                                    .text_color(muted_fg)
                                                    .child(pages_label),
                                            )
                                        })
                                        .child(
                                            Icon::new(IconName::ArrowRight)
                                                .small()
                                                .text_color(muted_fg),
                                        ),
                                ),
                        ),
                );
            }

            Some(
                div()
                    .w_full()
                    .max_w(px(540.0))
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        div()
                            .text_xs()
                            .font_semibold()
                            .text_color(muted_fg)
                            .child("Recent documents"),
                    )
                    .child(div().flex().flex_col().gap_0p5().children(items)),
            )
        } else {
            None
        };

        let quick_footer = div()
            .flex()
            .flex_row()
            .items_center()
            .justify_center()
            .gap_3()
            .pt_2()
            .pb_6()
            .child(
                Button::new("btn-welcome-settings")
                    .label("Settings")
                    .ghost()
                    .small()
                    .icon(IconName::Settings)
                    .on_click(cx.listener(move |v, _, w, cx| {
                        on_action(v, WelcomeAction::Settings, w, cx);
                    })),
            )
            .child(div().text_xs().text_color(muted_fg).child("•"))
            .child(
                Button::new("btn-welcome-about")
                    .label("About PDFbull")
                    .ghost()
                    .small()
                    .icon(IconName::Info)
                    .on_click(cx.listener(move |v, _, w, cx| {
                        on_action(v, WelcomeAction::About, w, cx);
                    })),
            );

        div()
            .id("welcome-scroll-container")
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .size_full()
            .bg(bg)
            .overflow_y_scrollbar()
            .child(container.children(recent_section).child(quick_footer))
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
        .w(px(320.0))
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
