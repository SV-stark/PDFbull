use gpui_kit::component::ActiveTheme;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::*;

use super::{
    canvas::DocumentViewport,
    dialogs::{ActiveDialog, DialogsState},
    log_console::{LogAction, LogConsoleState},
    ribbon::{RibbonAction, RibbonState},
    sidebar::{SidebarAction, SidebarState},
    tabs::{DocumentTab, TabAction, TabsState},
    welcome::{WelcomeAction, WelcomeState},
};

pub struct PdfbullView {
    pub ribbon: RibbonState,
    pub tabs: TabsState,
    pub sidebar: SidebarState,
    pub viewport: DocumentViewport,
    pub dialogs: DialogsState,
    pub welcome: WelcomeState,
    pub log_console: LogConsoleState,
}

impl PdfbullView {
    pub fn new(_window: &mut Window, _cx: &mut Context<Self>) -> Self {
        Self {
            ribbon: RibbonState::new(),
            tabs: TabsState::new(),
            sidebar: SidebarState::new(),
            viewport: DocumentViewport::new(),
            dialogs: DialogsState::new(),
            welcome: WelcomeState::new(),
            log_console: LogConsoleState::new(),
        }
    }

    pub fn open_sample_doc(&mut self, title: &str) {
        let new_id = self.tabs.tabs.len();
        self.tabs.tabs.push(DocumentTab {
            id: new_id,
            title: title.to_string(),
            path: None,
            is_modified: false,
        });
        self.tabs.active_tab_index = new_id;
        self.viewport.total_pages = 5;
        self.viewport.current_page = 0;
    }
}

impl Render for PdfbullView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let bg = cx.theme().background;
        let fg = cx.theme().foreground;
        let border = cx.theme().border;
        let muted = cx.theme().muted;
        let muted_fg = cx.theme().muted_foreground;
        let primary = cx.theme().primary;
        let has_tabs = !self.tabs.tabs.is_empty();

        let ribbon_tabs = self.ribbon.render_tabs(cx, |this, tab, _, cx| {
            this.ribbon.active_tab = tab;
            cx.notify();
        });

        // Top Header bar
        let header_bar = div()
            .flex()
            .flex_row()
            .items_center()
            .justify_between()
            .h_10()
            .px_3()
            .bg(bg)
            .border_b_1()
            .border_color(border)
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_3()
                    .child(div().text_lg().text_color(primary).child("PDFbull"))
                    .child(ribbon_tabs),
            )
            .child(
                div().flex().flex_row().items_center().gap_2().child(
                    Button::new("btn-toggle-logs")
                        .label("Logs")
                        .ghost()
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.log_console.is_open = !this.log_console.is_open;
                            cx.notify();
                        })),
                ),
            );

        // Ribbon Action Strip
        let action_strip = self.ribbon.render_strip(cx, |this, action, _, cx| {
            match action {
                RibbonAction::OpenFile => {
                    this.open_sample_doc("Opened Document.pdf");
                }
                RibbonAction::SaveFile => {}
                RibbonAction::Print => {}
                RibbonAction::ZoomIn => {
                    this.viewport.zoom = (this.viewport.zoom + 0.1).min(5.0);
                }
                RibbonAction::ZoomOut => {
                    this.viewport.zoom = (this.viewport.zoom - 0.1).max(0.25);
                }
                RibbonAction::ZoomReset => {
                    this.viewport.zoom = 1.0;
                }
                RibbonAction::PrevPage => {
                    if this.viewport.current_page > 0 {
                        this.viewport.current_page -= 1;
                    }
                }
                RibbonAction::NextPage => {
                    if this.viewport.current_page + 1 < this.viewport.total_pages {
                        this.viewport.current_page += 1;
                    }
                }
                RibbonAction::SetLayout(mode) => {
                    this.ribbon.layout_mode = mode;
                    this.viewport.layout_mode = mode;
                }
                RibbonAction::ToggleCover => {
                    this.ribbon.standalone_cover = !this.ribbon.standalone_cover;
                    this.viewport.standalone_cover = this.ribbon.standalone_cover;
                }
                RibbonAction::ToggleMidnight => {
                    this.ribbon.midnight_mode = !this.ribbon.midnight_mode;
                }
                RibbonAction::SelectTool(tool) => {
                    this.ribbon.active_tool = tool;
                }
                RibbonAction::SelectColor(color) => {
                    this.ribbon.active_color = color;
                }
                RibbonAction::Watermark => {
                    this.dialogs.active = Some(ActiveDialog::Watermark);
                }
                RibbonAction::HeaderFooter => {
                    this.dialogs.active = Some(ActiveDialog::HeaderFooter);
                }
                RibbonAction::FormFields => {}
                RibbonAction::DigitalSignatures => {
                    this.dialogs.active = Some(ActiveDialog::Signature);
                }
                RibbonAction::Merge => {}
                RibbonAction::Split => {}
                RibbonAction::Flatten => {}
                RibbonAction::Encrypt => {
                    this.dialogs.active = Some(ActiveDialog::Security);
                }
                RibbonAction::Decrypt => {
                    this.dialogs.active = Some(ActiveDialog::Password);
                }
                RibbonAction::Compress => {}
                RibbonAction::Ocr => {}
                RibbonAction::PageOrganizer => {
                    this.dialogs.active = Some(ActiveDialog::PageOrganizer);
                }
                RibbonAction::ConvertMarkdown
                | RibbonAction::ConvertHtml
                | RibbonAction::ConvertText => {}
            }
            cx.notify();
        });

        // Tabs Bar
        let tabs_bar = if has_tabs {
            Some(self.tabs.render(cx, |this, action, _, cx| {
                match action {
                    TabAction::SelectTab(idx) => {
                        this.tabs.active_tab_index = idx;
                    }
                    TabAction::CloseTab(idx) => {
                        if idx < this.tabs.tabs.len() {
                            this.tabs.tabs.remove(idx);
                            if this.tabs.active_tab_index >= this.tabs.tabs.len()
                                && !this.tabs.tabs.is_empty()
                            {
                                this.tabs.active_tab_index = this.tabs.tabs.len() - 1;
                            }
                        }
                    }
                    TabAction::NewTab => {
                        this.open_sample_doc("Untitled.pdf");
                    }
                    TabAction::CloseOthers(keep_idx) => {
                        if keep_idx < this.tabs.tabs.len() {
                            let kept = this.tabs.tabs[keep_idx].clone();
                            this.tabs.tabs = vec![kept];
                            this.tabs.active_tab_index = 0;
                        }
                    }
                    TabAction::CloseToRight(idx) => {
                        this.tabs.tabs.truncate(idx + 1);
                        if this.tabs.active_tab_index >= this.tabs.tabs.len() {
                            this.tabs.active_tab_index = this.tabs.tabs.len() - 1;
                        }
                    }
                }
                cx.notify();
            }))
        } else {
            None
        };

        // Main Center Area (Welcome vs Document Workspace)
        let center_area: AnyElement = if has_tabs {
            let total = self.viewport.total_pages;
            let current = self.viewport.current_page;

            let sidebar = self
                .sidebar
                .render(cx, total, current, |this, action, _, cx| {
                    match action {
                        SidebarAction::ToggleOpen => {
                            this.sidebar.is_open = !this.sidebar.is_open;
                        }
                        SidebarAction::SelectMode(mode) => {
                            this.sidebar.mode = mode;
                        }
                        SidebarAction::SelectPage(page_idx) => {
                            this.viewport.current_page = page_idx;
                        }
                    }
                    cx.notify();
                });

            let canvas = self.viewport.render(cx, |this, page_idx, _, cx| {
                this.viewport.current_page = page_idx;
                cx.notify();
            });

            div()
                .flex()
                .flex_row()
                .size_full()
                .overflow_hidden()
                .child(sidebar)
                .child(div().flex_1().size_full().overflow_hidden().child(canvas))
                .into_any_element()
        } else {
            self.welcome
                .render(cx, |this, action, _, cx| {
                    match action {
                        WelcomeAction::OpenFile => {
                            this.open_sample_doc("Document.pdf");
                        }
                        WelcomeAction::MergeFiles => {
                            this.open_sample_doc("Merged.pdf");
                        }
                        WelcomeAction::PageOrganizer => {
                            this.open_sample_doc("Organizer.pdf");
                            this.dialogs.active = Some(ActiveDialog::PageOrganizer);
                        }
                        WelcomeAction::DigitalSignatures => {
                            this.open_sample_doc("Signed.pdf");
                            this.dialogs.active = Some(ActiveDialog::Signature);
                        }
                        WelcomeAction::OpenRecent(_) => {
                            this.open_sample_doc("Recent.pdf");
                        }
                    }
                    cx.notify();
                })
                .into_any_element()
        };

        // Bottom Status Bar
        let status_bar = div()
            .flex()
            .flex_row()
            .items_center()
            .justify_between()
            .h_6()
            .px_3()
            .bg(muted)
            .border_t_1()
            .border_color(border)
            .text_xs()
            .text_color(muted_fg)
            .child(
                div()
                    .flex()
                    .flex_row()
                    .gap_3()
                    .child(format!(
                        "Page {} of {}",
                        self.viewport.current_page + 1,
                        self.viewport.total_pages
                    ))
                    .child(format!(
                        "Zoom: {}%",
                        (self.viewport.zoom * 100.0).round() as u32
                    ))
                    .child(format!("Mode: {:?}", self.ribbon.layout_mode)),
            )
            .child("PDFbull GPUI Core 0.16.0");

        // Log Console Drawer (if open)
        let log_drawer = self.log_console.render(cx, |this, action, _, cx| {
            match action {
                LogAction::ToggleOpen => {
                    this.log_console.is_open = !this.log_console.is_open;
                }
                LogAction::Clear => {}
                LogAction::CopyAll => {}
                LogAction::SetFilter(lvl) => {
                    this.log_console.filter_level = Some(lvl);
                }
            }
            cx.notify();
        });

        // Dialog Overlay (if active)
        let dialog_overlay = self.dialogs.render_overlay(
            cx,
            |this, _, cx| {
                this.dialogs.active = None;
                cx.notify();
            },
            |this, _action, _, cx| {
                this.dialogs.active = None;
                cx.notify();
            },
        );

        // Assemble Top-level View
        div()
            .flex()
            .flex_col()
            .size_full()
            .relative()
            .bg(bg)
            .text_color(fg)
            .child(header_bar)
            .child(action_strip)
            .children(tabs_bar)
            .child(
                div()
                    .flex_1()
                    .size_full()
                    .overflow_hidden()
                    .child(center_area),
            )
            .children(log_drawer)
            .child(status_bar)
            .children(dialog_overlay)
    }
}
