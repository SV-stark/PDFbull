use gpui_kit::base::StyledExt;
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::Sizable;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RibbonTab {
    #[default]
    Home,
    View,
    Annotate,
    Tools,
    Convert,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AnnotationTool {
    #[default]
    Pointer,
    Highlight,
    Underline,
    Strikeout,
    Rectangle,
    Circle,
    Line,
    Arrow,
    Ink,
    StickyNote,
    Text,
    Redact,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PageLayoutMode {
    #[default]
    Continuous,
    SinglePage,
    TwoPageSpread,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RibbonAction {
    OpenFile,
    SaveFile,
    Print,
    ZoomIn,
    ZoomOut,
    ZoomReset,
    PrevPage,
    NextPage,
    SetLayout(PageLayoutMode),
    ToggleCover,
    ToggleMidnight,
    SelectTool(AnnotationTool),
    SelectColor([f32; 4]),
    // Tools
    Merge,
    Split,
    Watermark,
    HeaderFooter,
    FormFields,
    DigitalSignatures,
    Flatten,
    Encrypt,
    Decrypt,
    Compress,
    Ocr,
    PageOrganizer,
    // Convert & Tables
    ConvertMarkdown,
    ConvertHtml,
    ConvertText,
    ExtractTables,
}

pub struct RibbonState {
    pub active_tab: RibbonTab,
    pub active_tool: AnnotationTool,
    pub active_color: [f32; 4],
    pub layout_mode: PageLayoutMode,
    pub standalone_cover: bool,
    pub midnight_mode: bool,
}

impl Default for RibbonState {
    fn default() -> Self {
        Self::new()
    }
}

impl RibbonState {
    pub fn new() -> Self {
        Self {
            active_tab: RibbonTab::Home,
            active_tool: AnnotationTool::Pointer,
            active_color: [1.0, 0.9, 0.2, 0.4],
            layout_mode: PageLayoutMode::Continuous,
            standalone_cover: false,
            midnight_mode: false,
        }
    }

    pub fn render_tabs<V: 'static>(
        &self,
        cx: &mut Context<V>,
        on_tab_change: impl Fn(&mut V, RibbonTab, &mut Window, &mut Context<V>) + 'static + Copy,
    ) -> AnyElement {
        let primary = cx.theme().primary;
        let fg = cx.theme().foreground;
        let muted_fg = cx.theme().muted_foreground;
        let accent = cx.theme().accent;

        let tabs = [
            (RibbonTab::Home, "Home"),
            (RibbonTab::View, "View"),
            (RibbonTab::Annotate, "Annotate"),
            (RibbonTab::Tools, "Tools"),
            (RibbonTab::Convert, "Convert"),
        ];

        let active = self.active_tab;
        let mut tab_buttons = Vec::new();
        for (tab, label) in tabs {
            let is_active = tab == active;
            let click_listener = cx.listener(move |this, _, window, cx| {
                on_tab_change(this, tab, window, cx);
            });

            let tab_el = div()
                .id(SharedString::from(format!("ribbon-tab-{:?}", tab)))
                .flex()
                .items_center()
                .h(px(28.0))
                .px_3()
                .rounded_t_md()
                .border_b_2()
                .border_color(if is_active {
                    primary
                } else {
                    gpui_kit::transparent_black()
                })
                .bg(if is_active {
                    accent
                } else {
                    gpui_kit::transparent_black()
                })
                .hover(move |s| if !is_active { s.bg(accent) } else { s })
                .cursor_pointer()
                .on_click(click_listener)
                .child(
                    div()
                        .text_xs()
                        .font_medium()
                        .text_color(if is_active { fg } else { muted_fg })
                        .child(label),
                );

            tab_buttons.push(tab_el);
        }

        div()
            .flex()
            .flex_row()
            .items_end()
            .h(px(32.0))
            .gap_1()
            .px_3()
            .children(tab_buttons)
            .into_any_element()
    }

    pub fn render_strip<V: 'static>(
        &self,
        cx: &mut Context<V>,
        on_action: impl Fn(&mut V, RibbonAction, &mut Window, &mut Context<V>) + 'static + Copy,
    ) -> AnyElement {
        let border = cx.theme().border;
        let group_box = cx.theme().group_box;

        let content = match self.active_tab {
            RibbonTab::Home => render_home_strip(border, cx, on_action),
            RibbonTab::View => render_view_strip(
                self.layout_mode,
                self.standalone_cover,
                self.midnight_mode,
                border,
                cx,
                on_action,
            ),
            RibbonTab::Annotate => render_annotate_strip(self.active_tool, cx, on_action),
            RibbonTab::Tools => render_tools_strip(cx, on_action),
            RibbonTab::Convert => render_convert_strip(cx, on_action),
        };

        div()
            .flex()
            .flex_row()
            .items_center()
            .h(px(54.0))
            .px_3()
            .bg(group_box)
            .border_b_1()
            .border_color(border)
            .child(content)
            .into_any_element()
    }
}

fn ribbon_divider(border: Hsla) -> AnyElement {
    div()
        .w_px()
        .h(px(32.0))
        .mx_1()
        .bg(border)
        .into_any_element()
}

fn ribbon_group(label: &'static str, children: Vec<AnyElement>, muted_fg: Hsla) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .items_center()
        .gap_1()
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap_1()
                .children(children),
        )
        .child(
            div()
                .text_xs()
                .font_normal()
                .text_color(muted_fg)
                .child(label),
        )
        .into_any_element()
}

#[inline(never)]
fn render_home_strip<V: 'static>(
    border: Hsla,
    cx: &mut Context<V>,
    on_action: impl Fn(&mut V, RibbonAction, &mut Window, &mut Context<V>) + 'static + Copy,
) -> AnyElement {
    let muted_fg = cx.theme().muted_foreground;

    // Group 1: Document
    let doc_group = ribbon_group(
        "File",
        vec![
            Button::new("btn-open")
                .label("Open")
                .outline()
                .small()
                .on_click(
                    cx.listener(move |v, _, w, cx| on_action(v, RibbonAction::OpenFile, w, cx)),
                )
                .into_any_element(),
            Button::new("btn-save")
                .label("Save")
                .outline()
                .small()
                .on_click(
                    cx.listener(move |v, _, w, cx| on_action(v, RibbonAction::SaveFile, w, cx)),
                )
                .into_any_element(),
            Button::new("btn-print")
                .label("Print")
                .ghost()
                .small()
                .on_click(cx.listener(move |v, _, w, cx| on_action(v, RibbonAction::Print, w, cx)))
                .into_any_element(),
        ],
        muted_fg,
    );

    // Group 2: Navigation
    let nav_group = ribbon_group(
        "Page",
        vec![
            Button::new("btn-page-prev")
                .label("◀ Prev")
                .ghost()
                .small()
                .on_click(
                    cx.listener(move |v, _, w, cx| on_action(v, RibbonAction::PrevPage, w, cx)),
                )
                .into_any_element(),
            Button::new("btn-page-next")
                .label("Next ▶")
                .ghost()
                .small()
                .on_click(
                    cx.listener(move |v, _, w, cx| on_action(v, RibbonAction::NextPage, w, cx)),
                )
                .into_any_element(),
        ],
        muted_fg,
    );

    // Group 3: Zoom
    let zoom_group = ribbon_group(
        "Zoom",
        vec![
            Button::new("btn-zoom-out")
                .label("－")
                .ghost()
                .small()
                .on_click(
                    cx.listener(move |v, _, w, cx| on_action(v, RibbonAction::ZoomOut, w, cx)),
                )
                .into_any_element(),
            Button::new("btn-zoom-reset")
                .label("100%")
                .outline()
                .small()
                .on_click(
                    cx.listener(move |v, _, w, cx| on_action(v, RibbonAction::ZoomReset, w, cx)),
                )
                .into_any_element(),
            Button::new("btn-zoom-in")
                .label("＋")
                .ghost()
                .small()
                .on_click(cx.listener(move |v, _, w, cx| on_action(v, RibbonAction::ZoomIn, w, cx)))
                .into_any_element(),
        ],
        muted_fg,
    );

    div()
        .flex()
        .flex_row()
        .items_center()
        .gap_2()
        .child(doc_group)
        .child(ribbon_divider(border))
        .child(nav_group)
        .child(ribbon_divider(border))
        .child(zoom_group)
        .into_any_element()
}

#[inline(never)]
fn render_view_strip<V: 'static>(
    layout: PageLayoutMode,
    cover: bool,
    midnight: bool,
    border: Hsla,
    cx: &mut Context<V>,
    on_action: impl Fn(&mut V, RibbonAction, &mut Window, &mut Context<V>) + 'static + Copy,
) -> AnyElement {
    let muted_fg = cx.theme().muted_foreground;

    let layout_group = ribbon_group(
        "Page Layout",
        vec![
            Button::new("btn-continuous")
                .label("Continuous")
                .small()
                .when(layout == PageLayoutMode::Continuous, |b| b.primary())
                .when(layout != PageLayoutMode::Continuous, |b| b.outline())
                .on_click(cx.listener(move |v, _, w, cx| {
                    on_action(
                        v,
                        RibbonAction::SetLayout(PageLayoutMode::Continuous),
                        w,
                        cx,
                    )
                }))
                .into_any_element(),
            Button::new("btn-single")
                .label("Single")
                .small()
                .when(layout == PageLayoutMode::SinglePage, |b| b.primary())
                .when(layout != PageLayoutMode::SinglePage, |b| b.outline())
                .on_click(cx.listener(move |v, _, w, cx| {
                    on_action(
                        v,
                        RibbonAction::SetLayout(PageLayoutMode::SinglePage),
                        w,
                        cx,
                    )
                }))
                .into_any_element(),
            Button::new("btn-spread")
                .label("Two-Page")
                .small()
                .when(layout == PageLayoutMode::TwoPageSpread, |b| b.primary())
                .when(layout != PageLayoutMode::TwoPageSpread, |b| b.outline())
                .on_click(cx.listener(move |v, _, w, cx| {
                    on_action(
                        v,
                        RibbonAction::SetLayout(PageLayoutMode::TwoPageSpread),
                        w,
                        cx,
                    )
                }))
                .into_any_element(),
        ],
        muted_fg,
    );

    let display_group =
        ribbon_group(
            "Display Mode",
            vec![
                Button::new("btn-cover")
                    .label(if cover { "Cover: On" } else { "Cover: Off" })
                    .small()
                    .when(cover, |b| b.primary())
                    .when(!cover, |b| b.ghost())
                    .on_click(cx.listener(move |v, _, w, cx| {
                        on_action(v, RibbonAction::ToggleCover, w, cx)
                    }))
                    .into_any_element(),
                Button::new("btn-midnight")
                    .label(if midnight {
                        "Midnight: On"
                    } else {
                        "Midnight: Off"
                    })
                    .small()
                    .when(midnight, |b| b.primary())
                    .when(!midnight, |b| b.ghost())
                    .on_click(cx.listener(move |v, _, w, cx| {
                        on_action(v, RibbonAction::ToggleMidnight, w, cx)
                    }))
                    .into_any_element(),
            ],
            muted_fg,
        );

    div()
        .flex()
        .flex_row()
        .items_center()
        .gap_2()
        .child(layout_group)
        .child(ribbon_divider(border))
        .child(display_group)
        .into_any_element()
}

#[inline(never)]
fn render_annotate_strip<V: 'static>(
    cur_tool: AnnotationTool,
    cx: &mut Context<V>,
    on_action: impl Fn(&mut V, RibbonAction, &mut Window, &mut Context<V>) + 'static + Copy,
) -> AnyElement {
    let border = cx.theme().border;
    let muted_fg = cx.theme().muted_foreground;

    let text_tools = [
        (AnnotationTool::Pointer, "Pointer"),
        (AnnotationTool::Highlight, "Highlight"),
        (AnnotationTool::Underline, "Underline"),
        (AnnotationTool::Strikeout, "Strikeout"),
    ];

    let mut text_btns = Vec::new();
    for (tool, label) in text_tools {
        let is_selected = tool == cur_tool;
        let click_listener =
            cx.listener(move |v, _, w, cx| on_action(v, RibbonAction::SelectTool(tool), w, cx));
        let mut btn = Button::new(SharedString::from(format!("tool-{:?}", tool)))
            .label(label)
            .small();
        if is_selected {
            btn = btn.primary();
        } else {
            btn = btn.ghost();
        }
        text_btns.push(btn.on_click(click_listener).into_any_element());
    }

    let shape_tools = [
        (AnnotationTool::Rectangle, "Rectangle"),
        (AnnotationTool::Circle, "Circle"),
        (AnnotationTool::Line, "Line"),
        (AnnotationTool::Arrow, "Arrow"),
    ];

    let mut shape_btns = Vec::new();
    for (tool, label) in shape_tools {
        let is_selected = tool == cur_tool;
        let click_listener =
            cx.listener(move |v, _, w, cx| on_action(v, RibbonAction::SelectTool(tool), w, cx));
        let mut btn = Button::new(SharedString::from(format!("tool-{:?}", tool)))
            .label(label)
            .small();
        if is_selected {
            btn = btn.primary();
        } else {
            btn = btn.ghost();
        }
        shape_btns.push(btn.on_click(click_listener).into_any_element());
    }

    let draw_tools = [
        (AnnotationTool::Ink, "Ink"),
        (AnnotationTool::StickyNote, "Note"),
        (AnnotationTool::Text, "Text"),
        (AnnotationTool::Redact, "Redact"),
    ];

    let mut draw_btns = Vec::new();
    for (tool, label) in draw_tools {
        let is_selected = tool == cur_tool;
        let click_listener =
            cx.listener(move |v, _, w, cx| on_action(v, RibbonAction::SelectTool(tool), w, cx));
        let mut btn = Button::new(SharedString::from(format!("tool-{:?}", tool)))
            .label(label)
            .small();
        if is_selected {
            btn = btn.primary();
        } else {
            btn = btn.ghost();
        }
        draw_btns.push(btn.on_click(click_listener).into_any_element());
    }

    div()
        .flex()
        .flex_row()
        .items_center()
        .gap_2()
        .child(ribbon_group("Text & Markup", text_btns, muted_fg))
        .child(ribbon_divider(border))
        .child(ribbon_group("Shapes", shape_btns, muted_fg))
        .child(ribbon_divider(border))
        .child(ribbon_group("Draw & Notes", draw_btns, muted_fg))
        .into_any_element()
}

#[inline(never)]
fn render_tools_strip<V: 'static>(
    cx: &mut Context<V>,
    on_action: impl Fn(&mut V, RibbonAction, &mut Window, &mut Context<V>) + 'static + Copy,
) -> AnyElement {
    let border = cx.theme().border;
    let muted_fg = cx.theme().muted_foreground;

    let organize_group = ribbon_group(
        "Organize",
        vec![
            Button::new("tool-organizer")
                .label("Organizer")
                .primary()
                .small()
                .on_click(
                    cx.listener(move |v, _, w, cx| {
                        on_action(v, RibbonAction::PageOrganizer, w, cx)
                    }),
                )
                .into_any_element(),
            Button::new("tool-merge")
                .label("Merge")
                .outline()
                .small()
                .on_click(cx.listener(move |v, _, w, cx| on_action(v, RibbonAction::Merge, w, cx)))
                .into_any_element(),
            Button::new("tool-split")
                .label("Split")
                .outline()
                .small()
                .on_click(cx.listener(move |v, _, w, cx| on_action(v, RibbonAction::Split, w, cx)))
                .into_any_element(),
        ],
        muted_fg,
    );

    let security_group = ribbon_group(
        "Security & Signing",
        vec![
            Button::new("tool-sigs")
                .label("Signatures")
                .outline()
                .small()
                .on_click(cx.listener(move |v, _, w, cx| {
                    on_action(v, RibbonAction::DigitalSignatures, w, cx)
                }))
                .into_any_element(),
            Button::new("tool-encrypt")
                .label("Encrypt")
                .ghost()
                .small()
                .on_click(
                    cx.listener(move |v, _, w, cx| on_action(v, RibbonAction::Encrypt, w, cx)),
                )
                .into_any_element(),
            Button::new("tool-decrypt")
                .label("Decrypt")
                .ghost()
                .small()
                .on_click(
                    cx.listener(move |v, _, w, cx| on_action(v, RibbonAction::Decrypt, w, cx)),
                )
                .into_any_element(),
        ],
        muted_fg,
    );

    let content_group = ribbon_group(
        "Content",
        vec![
            Button::new("tool-watermark")
                .label("Watermark")
                .ghost()
                .small()
                .on_click(
                    cx.listener(move |v, _, w, cx| on_action(v, RibbonAction::Watermark, w, cx)),
                )
                .into_any_element(),
            Button::new("tool-header")
                .label("Header/Footer")
                .ghost()
                .small()
                .on_click(
                    cx.listener(move |v, _, w, cx| on_action(v, RibbonAction::HeaderFooter, w, cx)),
                )
                .into_any_element(),
            Button::new("tool-forms")
                .label("Form Fields")
                .ghost()
                .small()
                .on_click(
                    cx.listener(move |v, _, w, cx| on_action(v, RibbonAction::FormFields, w, cx)),
                )
                .into_any_element(),
            Button::new("tool-ocr")
                .label("OCR")
                .ghost()
                .small()
                .on_click(cx.listener(move |v, _, w, cx| on_action(v, RibbonAction::Ocr, w, cx)))
                .into_any_element(),
            Button::new("tool-compress")
                .label("Compress")
                .ghost()
                .small()
                .on_click(
                    cx.listener(move |v, _, w, cx| on_action(v, RibbonAction::Compress, w, cx)),
                )
                .into_any_element(),
            Button::new("tool-tables")
                .label("Tables")
                .outline()
                .small()
                .on_click(
                    cx.listener(move |v, _, w, cx| {
                        on_action(v, RibbonAction::ExtractTables, w, cx)
                    }),
                )
                .into_any_element(),
        ],
        muted_fg,
    );

    div()
        .flex()
        .flex_row()
        .items_center()
        .gap_2()
        .child(organize_group)
        .child(ribbon_divider(border))
        .child(security_group)
        .child(ribbon_divider(border))
        .child(content_group)
        .into_any_element()
}

#[inline(never)]
fn render_convert_strip<V: 'static>(
    cx: &mut Context<V>,
    on_action: impl Fn(&mut V, RibbonAction, &mut Window, &mut Context<V>) + 'static + Copy,
) -> AnyElement {
    let muted_fg = cx.theme().muted_foreground;

    let export_group =
        ribbon_group(
            "Export Documents",
            vec![
                Button::new("conv-md")
                    .label("Markdown (.md)")
                    .outline()
                    .small()
                    .on_click(cx.listener(move |v, _, w, cx| {
                        on_action(v, RibbonAction::ConvertMarkdown, w, cx)
                    }))
                    .into_any_element(),
                Button::new("conv-html")
                    .label("HTML5 (.html)")
                    .outline()
                    .small()
                    .on_click(cx.listener(move |v, _, w, cx| {
                        on_action(v, RibbonAction::ConvertHtml, w, cx)
                    }))
                    .into_any_element(),
                Button::new("conv-txt")
                    .label("Plain Text (.txt)")
                    .outline()
                    .small()
                    .on_click(cx.listener(move |v, _, w, cx| {
                        on_action(v, RibbonAction::ConvertText, w, cx)
                    }))
                    .into_any_element(),
                Button::new("conv-tables")
                    .label("Tables (.csv)")
                    .outline()
                    .small()
                    .on_click(cx.listener(move |v, _, w, cx| {
                        on_action(v, RibbonAction::ExtractTables, w, cx)
                    }))
                    .into_any_element(),
            ],
            muted_fg,
        );

    div()
        .flex()
        .flex_row()
        .items_center()
        .gap_2()
        .child(export_group)
        .into_any_element()
}
