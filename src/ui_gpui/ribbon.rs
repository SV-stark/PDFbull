use gpui_kit::component::ActiveTheme;
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
    // Convert
    ConvertMarkdown,
    ConvertHtml,
    ConvertText,
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
            let mut btn =
                Button::new(SharedString::from(format!("ribbon-tab-{:?}", tab))).label(label);

            if is_active {
                btn = btn.primary();
            } else {
                btn = btn.ghost();
            }

            tab_buttons.push(btn.on_click(click_listener));
        }

        div()
            .flex()
            .flex_row()
            .gap_1()
            .px_2()
            .pt_1()
            .children(tab_buttons)
            .into_any_element()
    }

    pub fn render_strip<V: 'static>(
        &self,
        cx: &mut Context<V>,
        on_action: impl Fn(&mut V, RibbonAction, &mut Window, &mut Context<V>) + 'static + Copy,
    ) -> AnyElement {
        let border = cx.theme().border;
        let muted = cx.theme().muted;

        let content: AnyElement = match self.active_tab {
            RibbonTab::Home => div()
                .flex()
                .flex_row()
                .items_center()
                .gap_2()
                .child(Button::new("btn-open").label("Open").outline().on_click(
                    cx.listener(move |v, _, w, cx| on_action(v, RibbonAction::OpenFile, w, cx)),
                ))
                .child(Button::new("btn-save").label("Save").outline().on_click(
                    cx.listener(move |v, _, w, cx| on_action(v, RibbonAction::SaveFile, w, cx)),
                ))
                .child(Button::new("btn-print").label("Print").ghost().on_click(
                    cx.listener(move |v, _, w, cx| on_action(v, RibbonAction::Print, w, cx)),
                ))
                .child(div().w_px().h_6().bg(border))
                .child(Button::new("btn-zoom-in").label("Zoom +").ghost().on_click(
                    cx.listener(move |v, _, w, cx| on_action(v, RibbonAction::ZoomIn, w, cx)),
                ))
                .child(
                    Button::new("btn-zoom-out")
                        .label("Zoom -")
                        .ghost()
                        .on_click(cx.listener(move |v, _, w, cx| {
                            on_action(v, RibbonAction::ZoomOut, w, cx)
                        })),
                )
                .child(
                    Button::new("btn-zoom-reset")
                        .label("100%")
                        .ghost()
                        .on_click(cx.listener(move |v, _, w, cx| {
                            on_action(v, RibbonAction::ZoomReset, w, cx)
                        })),
                )
                .child(div().w_px().h_6().bg(border))
                .child(Button::new("btn-page-prev").label("Prev").ghost().on_click(
                    cx.listener(move |v, _, w, cx| on_action(v, RibbonAction::PrevPage, w, cx)),
                ))
                .child(Button::new("btn-page-next").label("Next").ghost().on_click(
                    cx.listener(move |v, _, w, cx| on_action(v, RibbonAction::NextPage, w, cx)),
                ))
                .into_any_element(),

            RibbonTab::View => {
                let layout = self.layout_mode;
                let cover = self.standalone_cover;
                let midnight = self.midnight_mode;

                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_2()
                    .child(
                        Button::new("btn-continuous")
                            .label("Continuous")
                            .when(layout == PageLayoutMode::Continuous, |b| b.primary())
                            .when(layout != PageLayoutMode::Continuous, |b| b.outline())
                            .on_click(cx.listener(move |v, _, w, cx| {
                                on_action(
                                    v,
                                    RibbonAction::SetLayout(PageLayoutMode::Continuous),
                                    w,
                                    cx,
                                )
                            })),
                    )
                    .child(
                        Button::new("btn-single")
                            .label("Single")
                            .when(layout == PageLayoutMode::SinglePage, |b| b.primary())
                            .when(layout != PageLayoutMode::SinglePage, |b| b.outline())
                            .on_click(cx.listener(move |v, _, w, cx| {
                                on_action(
                                    v,
                                    RibbonAction::SetLayout(PageLayoutMode::SinglePage),
                                    w,
                                    cx,
                                )
                            })),
                    )
                    .child(
                        Button::new("btn-spread")
                            .label("Spread")
                            .when(layout == PageLayoutMode::TwoPageSpread, |b| b.primary())
                            .when(layout != PageLayoutMode::TwoPageSpread, |b| b.outline())
                            .on_click(cx.listener(move |v, _, w, cx| {
                                on_action(
                                    v,
                                    RibbonAction::SetLayout(PageLayoutMode::TwoPageSpread),
                                    w,
                                    cx,
                                )
                            })),
                    )
                    .child(div().w_px().h_6().bg(border))
                    .child(
                        Button::new("btn-cover")
                            .label(if cover { "Cover: On" } else { "Cover: Off" })
                            .ghost()
                            .on_click(cx.listener(move |v, _, w, cx| {
                                on_action(v, RibbonAction::ToggleCover, w, cx)
                            })),
                    )
                    .child(
                        Button::new("btn-midnight")
                            .label(if midnight {
                                "Midnight: On"
                            } else {
                                "Midnight: Off"
                            })
                            .ghost()
                            .on_click(cx.listener(move |v, _, w, cx| {
                                on_action(v, RibbonAction::ToggleMidnight, w, cx)
                            })),
                    )
                    .into_any_element()
            }

            RibbonTab::Annotate => {
                let cur_tool = self.active_tool;
                let tools = [
                    (AnnotationTool::Pointer, "Pointer"),
                    (AnnotationTool::Highlight, "Highlight"),
                    (AnnotationTool::Underline, "Underline"),
                    (AnnotationTool::Strikeout, "Strikeout"),
                    (AnnotationTool::Rectangle, "Rectangle"),
                    (AnnotationTool::Circle, "Circle"),
                    (AnnotationTool::Line, "Line"),
                    (AnnotationTool::Arrow, "Arrow"),
                    (AnnotationTool::Ink, "Ink"),
                    (AnnotationTool::StickyNote, "Note"),
                    (AnnotationTool::Text, "Text"),
                    (AnnotationTool::Redact, "Redact"),
                ];

                let mut tool_buttons = Vec::new();
                for (tool, label) in tools {
                    let is_selected = tool == cur_tool;
                    let click_listener = cx.listener(move |v, _, w, cx| {
                        on_action(v, RibbonAction::SelectTool(tool), w, cx)
                    });
                    let mut btn =
                        Button::new(SharedString::from(format!("tool-{:?}", tool))).label(label);
                    if is_selected {
                        btn = btn.primary();
                    } else {
                        btn = btn.ghost();
                    }
                    tool_buttons.push(btn.on_click(click_listener));
                }

                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_1()
                    .children(tool_buttons)
                    .into_any_element()
            }

            RibbonTab::Tools => div()
                .flex()
                .flex_row()
                .items_center()
                .gap_1()
                .child(Button::new("tool-merge").label("Merge").ghost().on_click(
                    cx.listener(move |v, _, w, cx| on_action(v, RibbonAction::Merge, w, cx)),
                ))
                .child(Button::new("tool-split").label("Split").ghost().on_click(
                    cx.listener(move |v, _, w, cx| on_action(v, RibbonAction::Split, w, cx)),
                ))
                .child(
                    Button::new("tool-watermark")
                        .label("Watermark")
                        .ghost()
                        .on_click(cx.listener(move |v, _, w, cx| {
                            on_action(v, RibbonAction::Watermark, w, cx)
                        })),
                )
                .child(
                    Button::new("tool-header")
                        .label("Header/Footer")
                        .ghost()
                        .on_click(cx.listener(move |v, _, w, cx| {
                            on_action(v, RibbonAction::HeaderFooter, w, cx)
                        })),
                )
                .child(
                    Button::new("tool-forms")
                        .label("Form Fields")
                        .ghost()
                        .on_click(cx.listener(move |v, _, w, cx| {
                            on_action(v, RibbonAction::FormFields, w, cx)
                        })),
                )
                .child(
                    Button::new("tool-sigs")
                        .label("Signatures")
                        .ghost()
                        .on_click(cx.listener(move |v, _, w, cx| {
                            on_action(v, RibbonAction::DigitalSignatures, w, cx)
                        })),
                )
                .child(
                    Button::new("tool-encrypt")
                        .label("Encrypt")
                        .ghost()
                        .on_click(cx.listener(move |v, _, w, cx| {
                            on_action(v, RibbonAction::Encrypt, w, cx)
                        })),
                )
                .child(
                    Button::new("tool-decrypt")
                        .label("Decrypt")
                        .ghost()
                        .on_click(cx.listener(move |v, _, w, cx| {
                            on_action(v, RibbonAction::Decrypt, w, cx)
                        })),
                )
                .child(
                    Button::new("tool-compress")
                        .label("Compress")
                        .ghost()
                        .on_click(cx.listener(move |v, _, w, cx| {
                            on_action(v, RibbonAction::Compress, w, cx)
                        })),
                )
                .child(Button::new("tool-ocr").label("OCR").ghost().on_click(
                    cx.listener(move |v, _, w, cx| on_action(v, RibbonAction::Ocr, w, cx)),
                ))
                .child(
                    Button::new("tool-organizer")
                        .label("Page Organizer")
                        .primary()
                        .on_click(cx.listener(move |v, _, w, cx| {
                            on_action(v, RibbonAction::PageOrganizer, w, cx)
                        })),
                )
                .into_any_element(),

            RibbonTab::Convert => div()
                .flex()
                .flex_row()
                .items_center()
                .gap_2()
                .child(
                    Button::new("conv-md")
                        .label("Convert to Markdown")
                        .outline()
                        .on_click(cx.listener(move |v, _, w, cx| {
                            on_action(v, RibbonAction::ConvertMarkdown, w, cx)
                        })),
                )
                .child(
                    Button::new("conv-html")
                        .label("Convert to HTML5")
                        .outline()
                        .on_click(cx.listener(move |v, _, w, cx| {
                            on_action(v, RibbonAction::ConvertHtml, w, cx)
                        })),
                )
                .child(
                    Button::new("conv-txt")
                        .label("Extract Plain Text")
                        .outline()
                        .on_click(cx.listener(move |v, _, w, cx| {
                            on_action(v, RibbonAction::ConvertText, w, cx)
                        })),
                )
                .into_any_element(),
        };

        div()
            .flex()
            .flex_row()
            .items_center()
            .h_11()
            .px_3()
            .bg(muted)
            .border_b_1()
            .border_color(border)
            .child(content)
            .into_any_element()
    }
}
