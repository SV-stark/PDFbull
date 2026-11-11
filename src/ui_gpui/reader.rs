//! Keyboard-first reading: fit-to-width / fit-to-page zoom, presentation and
//! full-screen modes, back/forward page history, a go-to-page box and a
//! command palette (Ctrl+K) that reaches every command by name.
//!
//! The state lives here as plain data; [`super::app_view::PdfbullView`] owns it
//! and applies the commands, exactly like the ribbon and the dialogs.

use gpui_kit::assets::IconName;
use gpui_kit::base::StyledExt;
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::Icon;
use gpui_kit::component::Sizable as _;
use gpui_kit::component::input::{Input, InputState};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

use super::ribbon::{PageLayoutMode, RibbonAction};

/// How the zoom level is chosen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ZoomMode {
    /// Whatever the user last set with the zoom buttons, keys or wheel.
    #[default]
    Manual,
    /// Page width fills the window width.
    FitWidth,
    /// The whole page (or spread) is visible.
    FitPage,
}

impl ZoomMode {
    pub fn label(self) -> &'static str {
        match self {
            ZoomMode::Manual => "Custom zoom",
            ZoomMode::FitWidth => "Fit width",
            ZoomMode::FitPage => "Fit page",
        }
    }
}

/// Everything the keyboard, the palette and the reader buttons can ask for.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ReaderCommand {
    NextPage,
    PrevPage,
    FirstPage,
    LastPage,
    GoToPage,
    Back,
    Forward,
    ZoomIn,
    ZoomOut,
    ActualSize,
    FitWidth,
    FitPage,
    CycleZoom,
    Layout(PageLayoutMode),
    ToggleContinuous,
    ToggleBookView,
    RotateLeft,
    RotateRight,
    ToggleSidebar,
    ShowThumbnails,
    ShowBookmarks,
    Find,
    Fullscreen,
    Presentation,
    Reload,
    NextTab,
    PrevTab,
    CloseTab,
    ToggleLogs,
    CopyPageText,
    Undo,
    Redo,
    Ribbon(RibbonAction),
}

/// One row of the command palette.
#[derive(Debug, Clone)]
pub struct PaletteEntry {
    pub title: &'static str,
    pub category: &'static str,
    pub shortcut: Option<&'static str>,
    pub command: ReaderCommand,
}

const fn entry(
    title: &'static str,
    category: &'static str,
    shortcut: Option<&'static str>,
    command: ReaderCommand,
) -> PaletteEntry {
    PaletteEntry {
        title,
        category,
        shortcut,
        command,
    }
}

/// Every command, in the order the palette lists them before a query.
pub fn palette_entries() -> Vec<PaletteEntry> {
    use ReaderCommand as C;
    use RibbonAction as R;
    vec![
        entry(
            "New blank PDF",
            "File",
            Some("Ctrl+N"),
            C::Ribbon(R::NewFile),
        ),
        entry("Open file…", "File", Some("Ctrl+O"), C::Ribbon(R::OpenFile)),
        entry("Save", "File", Some("Ctrl+S"), C::Ribbon(R::SaveFile)),
        entry("Print…", "File", Some("Ctrl+P"), C::Ribbon(R::Print)),
        entry("Undo", "Edit", Some("Ctrl+Z"), C::Undo),
        entry("Redo", "Edit", Some("Ctrl+Y"), C::Redo),
        entry("Reload from disk", "File", Some("R"), C::Reload),
        entry("Close tab", "File", Some("Ctrl+W"), C::CloseTab),
        entry("Next tab", "File", Some("Ctrl+Tab"), C::NextTab),
        entry("Previous tab", "File", Some("Ctrl+Shift+Tab"), C::PrevTab),
        entry(
            "Document properties",
            "File",
            None,
            C::Ribbon(R::Properties),
        ),
        entry("Go to page…", "Navigate", Some("G / Ctrl+G"), C::GoToPage),
        entry("Next page", "Navigate", Some("N / Space"), C::NextPage),
        entry(
            "Previous page",
            "Navigate",
            Some("P / Shift+Space"),
            C::PrevPage,
        ),
        entry("First page", "Navigate", Some("Home"), C::FirstPage),
        entry("Last page", "Navigate", Some("End"), C::LastPage),
        entry("Back", "Navigate", Some("Alt+Left / Backspace"), C::Back),
        entry("Forward", "Navigate", Some("Alt+Right"), C::Forward),
        entry("Find in document", "Navigate", Some("Ctrl+F or /"), C::Find),
        entry("Fit width", "Zoom", Some("Ctrl+2"), C::FitWidth),
        entry("Fit page", "Zoom", Some("Ctrl+3"), C::FitPage),
        entry(
            "Actual size (100%)",
            "Zoom",
            Some("Ctrl+1 / Ctrl+0"),
            C::ActualSize,
        ),
        entry(
            "Cycle fit page / fit width / 100%",
            "Zoom",
            Some("Z"),
            C::CycleZoom,
        ),
        entry("Zoom in", "Zoom", Some("+ / Ctrl++"), C::ZoomIn),
        entry("Zoom out", "Zoom", Some("- / Ctrl+-"), C::ZoomOut),
        entry(
            "Continuous scroll",
            "View",
            Some("C"),
            C::Layout(PageLayoutMode::Continuous),
        ),
        entry(
            "Single page",
            "View",
            None,
            C::Layout(PageLayoutMode::SinglePage),
        ),
        entry(
            "Two-page (book view)",
            "View",
            Some("B"),
            C::Layout(PageLayoutMode::TwoPageSpread),
        ),
        entry(
            "Toggle book cover page",
            "View",
            None,
            C::Ribbon(R::ToggleCover),
        ),
        entry("Presentation mode", "View", Some("F5"), C::Presentation),
        entry("Full screen", "View", Some("F11"), C::Fullscreen),
        entry(
            "Invert colours (night reading)",
            "View",
            Some("I"),
            C::Ribbon(R::ToggleMidnight),
        ),
        entry(
            "Rotate view left",
            "View",
            Some("Ctrl+Shift+-"),
            C::RotateLeft,
        ),
        entry(
            "Rotate view right",
            "View",
            Some("Ctrl+Shift++"),
            C::RotateRight,
        ),
        entry(
            "Toggle sidebar",
            "View",
            Some("Ctrl+B / F12"),
            C::ToggleSidebar,
        ),
        entry("Show page thumbnails", "View", None, C::ShowThumbnails),
        entry("Show bookmarks", "View", None, C::ShowBookmarks),
        entry("Show logs", "View", None, C::ToggleLogs),
        entry("Copy page text", "Edit", Some("Ctrl+A"), C::CopyPageText),
        entry(
            "Select / pointer",
            "Annotate",
            Some("Esc"),
            C::Ribbon(R::SelectTool(super::ribbon::AnnotationTool::Pointer)),
        ),
        entry(
            "Highlight tool",
            "Annotate",
            Some("H"),
            C::Ribbon(R::SelectTool(super::ribbon::AnnotationTool::Highlight)),
        ),
        entry(
            "Rectangle tool",
            "Annotate",
            None,
            C::Ribbon(R::SelectTool(super::ribbon::AnnotationTool::Rectangle)),
        ),
        entry(
            "Text box tool",
            "Annotate",
            Some("T"),
            C::Ribbon(R::SelectTool(super::ribbon::AnnotationTool::Text)),
        ),
        entry(
            "Sticky note tool",
            "Annotate",
            None,
            C::Ribbon(R::SelectTool(super::ribbon::AnnotationTool::StickyNote)),
        ),
        entry(
            "Redact tool",
            "Annotate",
            None,
            C::Ribbon(R::SelectTool(super::ribbon::AnnotationTool::Redact)),
        ),
        entry(
            "Apply redactions",
            "Annotate",
            None,
            C::Ribbon(R::ApplyRedactions),
        ),
        entry("Merge PDFs…", "Tools", None, C::Ribbon(R::Merge)),
        entry("Split document…", "Tools", None, C::Ribbon(R::Split)),
        entry("Extract pages…", "Tools", None, C::Ribbon(R::ExtractPages)),
        entry("Page organizer", "Tools", None, C::Ribbon(R::PageOrganizer)),
        entry("Watermark…", "Tools", None, C::Ribbon(R::Watermark)),
        entry(
            "Header & page numbers…",
            "Tools",
            None,
            C::Ribbon(R::HeaderFooter),
        ),
        entry("Form fields", "Tools", None, C::Ribbon(R::FormFields)),
        entry(
            "Digital signatures",
            "Tools",
            None,
            C::Ribbon(R::DigitalSignatures),
        ),
        entry(
            "Encrypt with password…",
            "Tools",
            None,
            C::Ribbon(R::Encrypt),
        ),
        entry(
            "Compress / optimize…",
            "Tools",
            None,
            C::Ribbon(R::Compress),
        ),
        entry(
            "Recognise text (OCR) on this page",
            "Tools",
            None,
            C::Ribbon(R::Ocr),
        ),
        entry(
            "Export to Markdown…",
            "Convert",
            None,
            C::Ribbon(R::ConvertMarkdown),
        ),
        entry(
            "Export to HTML…",
            "Convert",
            None,
            C::Ribbon(R::ConvertHtml),
        ),
        entry(
            "Export to plain text…",
            "Convert",
            None,
            C::Ribbon(R::ConvertText),
        ),
        entry(
            "Detect tables on this page",
            "Convert",
            None,
            C::Ribbon(R::ExtractTables),
        ),
        entry("Settings", "App", Some("Ctrl+,"), C::Ribbon(R::Settings)),
        entry("About PDFbull", "App", Some("F1"), C::Ribbon(R::About)),
    ]
}

/// Indices into `entries` that match `query`, best match first. An empty query
/// keeps the original order.
pub fn filter_entries(query: &str, entries: &[PaletteEntry]) -> Vec<usize> {
    let query = query.trim();
    if query.is_empty() {
        return (0..entries.len()).collect();
    }
    let mut matcher = nucleo_matcher::Matcher::default();
    let mut pattern_buf = Vec::new();
    let pattern = nucleo_matcher::Utf32Str::new(query, &mut pattern_buf);
    let mut target_buf = Vec::new();
    let mut scored: Vec<(u16, usize)> = entries
        .iter()
        .enumerate()
        .filter_map(|(i, e)| {
            let text = format!("{} {}", e.title, e.category);
            let target = nucleo_matcher::Utf32Str::new(&text, &mut target_buf);
            matcher.fuzzy_match(target, pattern).map(|s| (s, i))
        })
        .collect();
    scored.sort_by_key(|(score, i)| (std::cmp::Reverse(*score), *i));
    scored.into_iter().map(|(_, i)| i).collect()
}

/// Zoom that fits `pages_across` pages of `page` points (side by side, `gap`
/// points apart) into an `avail` area. `None` when the area is not known yet.
pub fn fit_zoom(
    mode: ZoomMode,
    avail: (f32, f32),
    page: (f32, f32),
    pages_across: usize,
    gap: f32,
) -> Option<f32> {
    let (aw, ah) = avail;
    let (pw, ph) = page;
    if aw <= 1.0 || ah <= 1.0 || pw <= 0.0 || ph <= 0.0 {
        return None;
    }
    let across = pages_across.max(1) as f32;
    let content_w = pw * across + gap * (across - 1.0);
    let by_width = aw / content_w;
    let z = match mode {
        ZoomMode::Manual => return None,
        ZoomMode::FitWidth => by_width,
        ZoomMode::FitPage => by_width.min(ah / ph),
    };
    Some(z.clamp(0.1, 8.0))
}

/// Parse the go-to-page box: a 1-based page number, or `first` / `last`.
/// Returns the 0-based page index, clamped to the document.
pub fn parse_goto(input: &str, total: usize) -> Option<usize> {
    if total == 0 {
        return None;
    }
    let s = input.trim().to_ascii_lowercase();
    match s.as_str() {
        "" => None,
        "first" | "start" => Some(0),
        "last" | "end" => Some(total - 1),
        _ => {
            let n: usize = s.parse().ok()?;
            Some(n.clamp(1, total) - 1)
        }
    }
}

/// How the window should start, from the command line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartMode {
    Presentation,
    Fullscreen,
}

/// Command-line options: `-page N`, `-presentation`, `-fullscreen`
/// (one or two leading dashes).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct StartupOptions {
    /// 0-based page to open at.
    pub page: Option<usize>,
    pub mode: Option<StartMode>,
}

pub fn parse_startup_args(args: &[String]) -> StartupOptions {
    let mut opts = StartupOptions::default();
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        if !arg.starts_with('-') {
            continue;
        }
        let flag = arg.trim_start_matches('-').to_ascii_lowercase();
        match flag.as_str() {
            "page" => {
                if let Some(n) = it.next().and_then(|v| v.trim().parse::<usize>().ok()) {
                    opts.page = Some(n.saturating_sub(1));
                }
            }
            "presentation" | "present" => opts.mode = Some(StartMode::Presentation),
            "fullscreen" => opts.mode = Some(StartMode::Fullscreen),
            _ => {}
        }
    }
    opts
}

/// View settings put aside while presenting, restored on exit.
#[derive(Debug, Clone, Copy)]
pub struct SavedView {
    pub layout: PageLayoutMode,
    pub zoom: f32,
    pub zoom_mode: ZoomMode,
    pub sidebar_open: bool,
    pub log_open: bool,
}

pub struct ReaderState {
    pub zoom_mode: ZoomMode,
    /// The zoom the last fit produced. If the zoom has moved away from it, the
    /// user zoomed by hand and fitting stops.
    pub last_fitted_zoom: f32,
    /// Canvas size the last fit was computed for.
    pub last_fit_size: (f32, f32),
    /// Presentation: full screen, black, one page at a time, nothing else.
    pub presentation: bool,
    /// Full screen with the toolbars hidden.
    pub fullscreen: bool,
    /// We switched the window to full screen (rather than the user before
    /// us), so leaving the mode should switch it back.
    pub made_window_fullscreen: bool,
    pub saved_view: Option<SavedView>,
    /// Pages to return to with Back / Forward.
    pub back: Vec<usize>,
    pub forward: Vec<usize>,
    pub palette_open: bool,
    pub palette_selected: usize,
    pub palette_input: Option<Entity<InputState>>,
    pub goto_open: bool,
    pub goto_input: Option<Entity<InputState>>,
    pub entries: Vec<PaletteEntry>,
    /// From the command line: page to show once the document has opened.
    pub start_page: Option<usize>,
    /// From the command line: presentation or full screen once open.
    pub start_mode: Option<StartMode>,
    /// The document has opened and `start_mode` can be applied.
    pub start_ready: bool,
}

/// History is capped so a long session cannot grow it without bound.
const MAX_HISTORY: usize = 100;

impl ReaderState {
    pub fn new_in<O: 'static>(window: &mut Window, owner: &mut Context<O>) -> Self {
        let palette_input = owner.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Type a command… (e.g. fit width, rotate, export)")
        });
        let goto_input =
            owner.new(|cx| InputState::new(window, cx).placeholder("Page number, first or last"));
        Self {
            palette_input: Some(palette_input),
            goto_input: Some(goto_input),
            ..Self::new()
        }
    }

    pub fn new() -> Self {
        Self {
            zoom_mode: ZoomMode::Manual,
            last_fitted_zoom: 0.0,
            last_fit_size: (0.0, 0.0),
            presentation: false,
            fullscreen: false,
            made_window_fullscreen: false,
            saved_view: None,
            back: Vec::new(),
            forward: Vec::new(),
            palette_open: false,
            palette_selected: 0,
            palette_input: None,
            goto_open: false,
            goto_input: None,
            entries: palette_entries(),
            start_page: None,
            start_mode: None,
            start_ready: false,
        }
    }

    /// Remember `from` before a jump, so Back can return to it.
    pub fn record_jump(&mut self, from: usize, to: usize) {
        if from == to {
            return;
        }
        if self.back.last() != Some(&from) {
            self.back.push(from);
            if self.back.len() > MAX_HISTORY {
                self.back.remove(0);
            }
        }
        self.forward.clear();
    }

    /// Page to show for Back, given the page on screen now.
    pub fn go_back(&mut self, current: usize) -> Option<usize> {
        let target = self.back.pop()?;
        self.forward.push(current);
        Some(target)
    }

    /// Page to show for Forward, given the page on screen now.
    pub fn go_forward(&mut self, current: usize) -> Option<usize> {
        let target = self.forward.pop()?;
        self.back.push(current);
        Some(target)
    }

    /// Hide toolbars, tabs, sidebar and status bar?
    pub fn chrome_hidden(&self) -> bool {
        self.presentation || self.fullscreen
    }

    /// Palette rows matching what is typed, best first.
    pub fn palette_matches(&self, cx: &App) -> Vec<usize> {
        let query = self
            .palette_input
            .as_ref()
            .map(|i| i.read(cx).value().to_string())
            .unwrap_or_default();
        filter_entries(&query, &self.entries)
    }

    /// The command palette overlay, or `None` when it is closed.
    pub fn render_palette<V: 'static>(
        &self,
        cx: &mut Context<V>,
        on_pick: impl Fn(&mut V, ReaderCommand, &mut Window, &mut Context<V>) + 'static + Copy,
        on_close: impl Fn(&mut V, &mut Window, &mut Context<V>) + 'static + Copy,
    ) -> Option<AnyElement> {
        if !self.palette_open {
            return None;
        }
        let bg = cx.theme().background;
        let border = cx.theme().border;
        let fg = cx.theme().foreground;
        let muted_fg = cx.theme().muted_foreground;
        let accent = cx.theme().accent;

        let matches = self.palette_matches(cx);
        let selected = self.palette_selected.min(matches.len().saturating_sub(1));
        // Keep the selected row on screen without a scroll container: show a
        // window of rows around it.
        const VISIBLE: usize = 12;
        let start = selected.saturating_sub(VISIBLE - 1);
        let rows: Vec<AnyElement> = matches
            .iter()
            .enumerate()
            .skip(start)
            .take(VISIBLE)
            .map(|(pos, &idx)| {
                let e = &self.entries[idx];
                let command = e.command;
                let is_sel = pos == selected;
                div()
                    .id(("palette-row", idx))
                    .flex()
                    .flex_row()
                    .items_center()
                    .justify_between()
                    .gap_3()
                    .px_3()
                    .py_1p5()
                    .rounded_md()
                    .cursor_pointer()
                    .when(is_sel, |el| el.bg(accent))
                    .hover(|s| s.bg(accent))
                    .on_click(cx.listener(move |v, _, w, cx| on_pick(v, command, w, cx)))
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap_2()
                            .min_w_0()
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(muted_fg)
                                    .w(px(64.0))
                                    .flex_none()
                                    .child(e.category),
                            )
                            .child(div().text_sm().text_color(fg).truncate().child(e.title)),
                    )
                    .when_some(e.shortcut, |el, sc| {
                        el.child(
                            div()
                                .flex_none()
                                .text_xs()
                                .px_1p5()
                                .rounded_sm()
                                .border_1()
                                .border_color(border)
                                .text_color(muted_fg)
                                .child(sc),
                        )
                    })
                    .into_any_element()
            })
            .collect();
        let empty = rows.is_empty();

        let panel = div()
            .id("palette-panel")
            .w(px(560.0))
            .max_w(relative(0.9))
            .mt(px(80.0))
            .flex()
            .flex_col()
            .gap_2()
            .p_3()
            .bg(bg)
            .border_1()
            .border_color(border)
            .rounded_lg()
            .shadow_lg()
            .on_click(cx.listener(|_, _, _, cx| cx.stop_propagation()))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_2()
                    .child(Icon::new(IconName::Command).small().text_color(muted_fg))
                    .when_some(self.palette_input.as_ref(), |el, input| {
                        el.child(
                            div()
                                .flex_1()
                                .child(Input::new(input).id("palette-input").w_full()),
                        )
                    }),
            )
            .child(div().flex().flex_col().gap_0p5().children(rows))
            .when(empty, |el| {
                el.child(
                    div()
                        .px_3()
                        .py_2()
                        .text_sm()
                        .text_color(muted_fg)
                        .child("No matching command"),
                )
            })
            .child(
                div()
                    .text_xs()
                    .text_color(muted_fg)
                    .child("↑ ↓ to choose · Enter to run · Esc to close"),
            );

        Some(
            div()
                .id("palette-backdrop")
                .absolute()
                .inset_0()
                .size_full()
                .bg(gpui_kit::rgba(0x0000_0060))
                .flex()
                .flex_col()
                .items_center()
                .on_click(cx.listener(move |v, _, w, cx| on_close(v, w, cx)))
                .child(panel)
                .into_any_element(),
        )
    }

    /// The go-to-page box, or `None` when it is closed.
    pub fn render_goto<V: 'static>(
        &self,
        cx: &mut Context<V>,
        current: usize,
        total: usize,
        on_close: impl Fn(&mut V, &mut Window, &mut Context<V>) + 'static + Copy,
    ) -> Option<AnyElement> {
        if !self.goto_open {
            return None;
        }
        let bg = cx.theme().background;
        let border = cx.theme().border;
        let fg = cx.theme().foreground;
        let muted_fg = cx.theme().muted_foreground;
        let panel = div()
            .id("goto-panel")
            .w(px(320.0))
            .mt(px(80.0))
            .flex()
            .flex_col()
            .gap_2()
            .p_3()
            .bg(bg)
            .border_1()
            .border_color(border)
            .rounded_lg()
            .shadow_lg()
            .on_click(cx.listener(|_, _, _, cx| cx.stop_propagation()))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_2()
                    .child(Icon::new(IconName::Hash).small().text_color(muted_fg))
                    .child(div().text_sm().font_medium().text_color(fg).child(format!(
                        "Go to page (now {} of {})",
                        current + 1,
                        total
                    ))),
            )
            .when_some(self.goto_input.as_ref(), |el, input| {
                el.child(Input::new(input).id("goto-input").w_full())
            })
            .child(
                div()
                    .text_xs()
                    .text_color(muted_fg)
                    .child("Enter to jump · Esc to cancel"),
            );
        Some(
            div()
                .id("goto-backdrop")
                .absolute()
                .inset_0()
                .size_full()
                .flex()
                .flex_col()
                .items_center()
                .on_click(cx.listener(move |v, _, w, cx| on_close(v, w, cx)))
                .child(panel)
                .into_any_element(),
        )
    }
}

impl Default for ReaderState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    // Not `use super::*`: that would pull in GPUI's own `test` macro and
    // shadow the standard `#[test]`.
    use super::{
        ReaderCommand, ReaderState, StartMode, StartupOptions, ZoomMode, filter_entries, fit_zoom,
        palette_entries, parse_goto, parse_startup_args,
    };

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn startup_flags_are_read() {
        let o = parse_startup_args(&args(&[
            "pdfbull",
            "book.pdf",
            "-page",
            "12",
            "-presentation",
        ]));
        assert_eq!(o.page, Some(11));
        assert_eq!(o.mode, Some(StartMode::Presentation));
        let o = parse_startup_args(&args(&["pdfbull", "--fullscreen", "a.cbz"]));
        assert_eq!(o.mode, Some(StartMode::Fullscreen));
        assert_eq!(o.page, None);
        assert_eq!(
            parse_startup_args(&args(&["pdfbull", "a.pdf"])),
            StartupOptions::default()
        );
        // A missing or bad number is ignored rather than misread.
        assert_eq!(
            parse_startup_args(&args(&["pdfbull", "-page", "x"])).page,
            None
        );
    }

    #[test]
    fn fit_width_fills_the_width() {
        let z = fit_zoom(ZoomMode::FitWidth, (1000.0, 500.0), (500.0, 800.0), 1, 0.0).unwrap();
        assert!((z - 2.0).abs() < 1e-4);
    }

    #[test]
    fn fit_page_is_limited_by_height() {
        let z = fit_zoom(ZoomMode::FitPage, (1000.0, 400.0), (500.0, 800.0), 1, 0.0).unwrap();
        assert!((z - 0.5).abs() < 1e-4);
    }

    #[test]
    fn fit_width_counts_both_pages_of_a_spread() {
        let z = fit_zoom(ZoomMode::FitWidth, (1020.0, 900.0), (500.0, 800.0), 2, 20.0).unwrap();
        assert!((z - 1.0).abs() < 1e-4);
    }

    #[test]
    fn unknown_area_gives_no_fit() {
        assert_eq!(
            fit_zoom(ZoomMode::FitPage, (0.0, 0.0), (500.0, 800.0), 1, 0.0),
            None
        );
        assert_eq!(
            fit_zoom(ZoomMode::Manual, (100.0, 100.0), (500.0, 800.0), 1, 0.0),
            None
        );
    }

    #[test]
    fn goto_parses_numbers_and_words() {
        assert_eq!(parse_goto("5", 10), Some(4));
        assert_eq!(parse_goto(" 1 ", 10), Some(0));
        assert_eq!(parse_goto("99", 10), Some(9));
        assert_eq!(parse_goto("0", 10), Some(0));
        assert_eq!(parse_goto("last", 10), Some(9));
        assert_eq!(parse_goto("First", 10), Some(0));
        assert_eq!(parse_goto("abc", 10), None);
        assert_eq!(parse_goto("", 10), None);
        assert_eq!(parse_goto("3", 0), None);
    }

    #[test]
    fn history_goes_back_and_forward() {
        let mut r = ReaderState::new();
        r.record_jump(0, 9);
        r.record_jump(9, 4);
        assert_eq!(r.go_back(4), Some(9));
        assert_eq!(r.go_back(9), Some(0));
        assert_eq!(r.go_back(0), None);
        assert_eq!(r.go_forward(0), Some(9));
        // A fresh jump drops the forward history.
        r.record_jump(9, 2);
        assert_eq!(r.go_forward(2), None);
    }

    #[test]
    fn palette_finds_commands_by_name() {
        let entries = palette_entries();
        let hits = filter_entries("fit width", &entries);
        assert_eq!(entries[hits[0]].command, ReaderCommand::FitWidth);
        let hits = filter_entries("present", &entries);
        assert_eq!(entries[hits[0]].command, ReaderCommand::Presentation);
        assert_eq!(filter_entries("", &entries).len(), entries.len());
        assert!(filter_entries("zzzzqqq", &entries).is_empty());
    }
}
