use crate::app::PdfBullApp;
use crate::pdf_engine::RenderFilter;
use iced::widget::{button, column, container, row, scrollable, text, text_input, Id, Space};
use iced::{Color, Element, Length, Padding};
use iced_aw::widget::Badge;
use iced_aw::widget::Card;

fn hex_to_rgb(hex: &str) -> (f32, f32, f32) {
    let hex = hex.trim_start_matches('#');
    if hex.len() != 6 {
        return (0.0, 0.0, 0.0);
    }
    let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0) as f32 / 255.0;
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0) as f32 / 255.0;
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0) as f32 / 255.0;
    (r, g, b)
}

use std::path::PathBuf;

fn rgba_to_iced(rgba: (u8, u8, u8, u8)) -> Color {
    let (r, g, b, a) = rgba;
    Color::from_rgba8(r, g, b, a as f32 / 255.0)
}

fn rgb_to_iced(rgb: (u8, u8, u8)) -> Color {
    let (r, g, b) = rgb;
    Color::from_rgb8(r, g, b)
}

fn filter_btn(
    label: &str,
    f: RenderFilter,
    active: RenderFilter,
) -> iced::widget::Button<'static, crate::message::Message> {
    let btn = button(text(label).size(11));
    if f == active {
        btn.style(|theme: &iced::Theme, _| iced::widget::button::Appearance {
            background: Some(iced::Background::Color(
                theme.extended_palette().primary.strong.color,
            )),
            text_color: theme.extended_palette().primary.strong.text,
            ..Default::default()
        })
    } else {
        btn
    }
    .on_press(crate::message::Message::SetFilter(f))
}

use iced::widget::tooltip;

fn render_toolbar(app: &PdfBullApp) -> Element<crate::message::Message> {
    let tab = &app.tabs[app.active_tab];

    let loading_indicator = if app.rendering_count > 0 {
        row![
            iced_aw::widget::Badge::new(text(format!("{}", app.rendering_count))),
            text("Rendering").size(12)
        ]
        .spacing(5)
    } else {
        row![]
    };

    let row1 = row![
        tooltip(
            button("📂 Open").on_press(crate::message::Message::OpenDocument),
            "Open another PDF",
            tooltip::Position::Bottom
        ),
        tooltip(
            button("✕ Close").on_press(crate::message::Message::CloseTab(app.active_tab)),
            "Close current tab (Ctrl+W)",
            tooltip::Position::Bottom
        ),
        tooltip(
            button("☰ Sidebar").on_press(crate::message::Message::ToggleSidebar),
            "Toggle Sidebar (Ctrl+B)",
            tooltip::Position::Bottom
        ),
        Space::new().width(Length::Fixed(10.0)),
        button("-").on_press(crate::message::Message::ZoomOut),
        text(format!("{}%", (tab.zoom * 100.0) as u32)),
        button("+").on_press(crate::message::Message::ZoomIn),
        Space::new().width(Length::Fixed(10.0)),
        tooltip(
            button("↻").on_press(crate::message::Message::RotateClockwise),
            "Rotate 90° clockwise",
            tooltip::Position::Bottom
        ),
        tooltip(
            button("↺").on_press(crate::message::Message::RotateCounterClockwise),
            "Rotate 90° counter-clockwise",
            tooltip::Position::Bottom
        ),
        text(format!("{}°", tab.rotation)),
        Space::new().width(Length::Fixed(10.0)),
        row![
            text("Filter:").size(12),
            filter_btn("None", RenderFilter::None, tab.render_filter),
            filter_btn("Gray", RenderFilter::Grayscale, tab.render_filter),
            filter_btn("Invert", RenderFilter::Inverted, tab.render_filter),
            filter_btn("Eco", RenderFilter::Eco, tab.render_filter),
            filter_btn("B&W", RenderFilter::BlackWhite, tab.render_filter),
            filter_btn("Lighten", RenderFilter::Lighten, tab.render_filter),
            filter_btn("NoShad", RenderFilter::NoShadow, tab.render_filter),
        ]
        .spacing(3)
        .align_y(iced::Alignment::Center),
        tooltip(
            button(if tab.auto_crop { "Crop✓" } else { "Crop" })
                .on_press(crate::message::Message::ToggleAutoCrop),
            "Auto-crop whitespace margins",
            tooltip::Position::Bottom
        ),
        Space::new().width(Length::Fill),
        loading_indicator,
        Space::new().width(Length::Fixed(10.0)),
        tooltip(
            button("?").on_press(crate::message::Message::ToggleKeyboardHelp),
            "Keyboard Shortcuts",
            tooltip::Position::Bottom
        ),
        tooltip(
            button("⛶").on_press(crate::message::Message::ToggleFullscreen),
            "Toggle Fullscreen (F11)",
            tooltip::Position::Bottom
        ),
        tooltip(
            button("⚙").on_press(crate::message::Message::OpenSettings),
            "Settings",
            tooltip::Position::Bottom
        ),
    ]
    .spacing(5)
    .align_y(iced::Alignment::Center);

    let row2 = row![
        tooltip(
            button("🔖 Bookmark").on_press(crate::message::Message::AddBookmark),
            "Add bookmark for current page (Ctrl+D)",
            tooltip::Position::Bottom
        ),
        tooltip(
            button(
                if app.annotation_mode == Some(crate::models::PendingAnnotationKind::Highlight) {
                    "🖊 Highlight*"
                } else {
                    "🖊 Highlight"
                }
            )
            .on_press(crate::message::Message::SetAnnotationMode(
                if app.annotation_mode == Some(crate::models::PendingAnnotationKind::Highlight) {
                    None
                } else {
                    Some(crate::models::PendingAnnotationKind::Highlight)
                }
            )),
            "Draw highlight annotation",
            tooltip::Position::Bottom
        ),
        tooltip(
            button(
                if app.annotation_mode == Some(crate::models::PendingAnnotationKind::Rectangle) {
                    "□ Rectangle*"
                } else {
                    "□ Rectangle"
                }
            )
            .on_press(crate::message::Message::SetAnnotationMode(
                if app.annotation_mode == Some(crate::models::PendingAnnotationKind::Rectangle) {
                    None
                } else {
                    Some(crate::models::PendingAnnotationKind::Rectangle)
                }
            )),
            "Draw rectangle annotation",
            tooltip::Position::Bottom
        ),
        tooltip(
            button("💾 Save Anns").on_press(crate::message::Message::SaveAnnotations),
            "Save annotations to JSON sidecar (Ctrl+S)",
            tooltip::Position::Bottom
        ),
        Space::new().width(Length::Fixed(10.0)),
        text_input("Search...", &app.search_query)
            .on_input(crate::message::Message::Search)
            .on_submit(crate::message::Message::NextSearchResult)
            .width(Length::Fixed(180.0)),
        if let Some(t) = app.current_tab() {
            if !t.search_results.is_empty() {
                text(format!(
                    "{}/{}",
                    t.current_search_index + 1,
                    t.search_results.len()
                ))
                .size(12)
            } else if !app.search_query.is_empty() {
                text("No results").size(12)
            } else {
                text("").size(12)
            }
        } else {
            text("").size(12)
        },
        button("▲")
            .on_press(crate::message::Message::PrevSearchResult)
            .padding(3),
        button("▼")
            .on_press(crate::message::Message::NextSearchResult)
            .padding(3),
        button("✕")
            .on_press(crate::message::Message::ClearSearch)
            .padding(3),
        Space::new().width(Length::Fixed(10.0)),
        tooltip(
            button("📋 Extract Text").on_press(crate::message::Message::ExtractText),
            "Extract text from current page to .txt file",
            tooltip::Position::Bottom
        ),
        tooltip(
            button("🖼 Export Page").on_press(crate::message::Message::ExportImage),
            "Export current page as PNG image (Ctrl+E)",
            tooltip::Position::Bottom
        ),
        tooltip(
            button("🗂 Export All").on_press(crate::message::Message::ExportImages),
            "Export all pages as PNG files",
            tooltip::Position::Bottom
        ),
    ]
    .spacing(5)
    .align_y(iced::Alignment::Center);

    column![row1, row2].spacing(10).padding(10).into()
}

fn render_page_nav(app: &PdfBullApp) -> Element<crate::message::Message> {
    let tab = &app.tabs[app.active_tab];

    let status = if let Some(ref msg) = app.status_message {
        row![
            Space::new().width(Length::Fill),
            Card::new(text(msg).size(12), Space::new()).padding(Padding::from(5)),
            button("×")
                .on_press(crate::message::Message::ClearStatus)
                .padding(2),
        ]
    } else {
        row![]
    };

    row![
        button("Prev").on_press(crate::message::Message::PrevPage),
        text(format!(
            "Page {} of {}",
            tab.current_page + 1,
            tab.total_pages.max(1)
        )),
        button("Next").on_press(crate::message::Message::NextPage),
        Space::new().width(Length::Fixed(20.0)),
        text_input("Go to page", &app.page_input)
            .on_input(crate::message::Message::PageInputChanged)
            .on_submit(crate::message::Message::PageInputSubmitted)
            .width(Length::Fixed(80.0)),
        status,
    ]
    .padding(5)
    .into()
}

fn render_sidebar(app: &PdfBullApp) -> Element<crate::message::Message> {
    let tab = &app.tabs[app.active_tab];

    let mut sidebar_col = column![].spacing(10).padding(5).width(Length::Fixed(180.0));

    if !tab.outline.is_empty() {
        sidebar_col = sidebar_col.push(
            Card::new(
                text("Outline")
                    .size(14)
                    .style(|_theme| iced::widget::text::Style {
                        color: Some(Color::from_rgb(0.2, 0.4, 0.6)),
                    }),
                column![for bookmark in &tab.outline {
                    button(text(&bookmark.title))
                        .on_press(crate::message::Message::JumpToPage(
                            bookmark.page_index as usize,
                        ))
                        .width(Length::Fill)
                }]
                .spacing(5),
            )
            .padding(Padding::from(10)),
        );
    }

    if !tab.bookmarks.is_empty() {
        sidebar_col = sidebar_col.push(
            Card::new(
                text("Bookmarks").size(14),
                column![for (idx, bookmark) in tab.bookmarks.iter().enumerate() {
                    row![
                        button(text(&bookmark.label))
                            .on_press(crate::message::Message::JumpToBookmark(idx))
                            .width(Length::Fill),
                        button("×").on_press(crate::message::Message::RemoveBookmark(idx))
                    ]
                }]
                .spacing(5),
            )
            .padding(Padding::from(10)),
        );
    }

    if !tab.annotations.is_empty() {
        sidebar_col = sidebar_col.push(
            Card::new(
                text("Annotations").size(14),
                column![for (idx, ann) in tab.annotations.iter().enumerate() {
                    let label = match &ann.style {
                        crate::models::AnnotationStyle::Highlight { .. } => {
                            format!("Highlight P{}", ann.page + 1)
                        }
                        crate::models::AnnotationStyle::Rectangle { .. } => {
                            format!("Rect P{}", ann.page + 1)
                        }
                        crate::models::AnnotationStyle::Text { text, .. } => {
                            format!("Text: {}", &text[..text.len().min(20)])
                        }
                    };
                    row![
                        button(text(label))
                            .on_press(crate::message::Message::JumpToPage(ann.page))
                            .width(Length::Fill),
                        button("×").on_press(crate::message::Message::DeleteAnnotation(idx))
                    ]
                }]
                .spacing(5),
            )
            .padding(Padding::from(10)),
        );
    }

    sidebar_col = sidebar_col.push(
        Card::new(
            text(format!("Pages ({})", tab.total_pages)).size(14),
            Space::new(),
        )
        .padding(Padding::from(10)),
    );

    let thumbnail_height = 40.0;
    let start_idx = (tab.sidebar_viewport_y / thumbnail_height).max(0.0) as usize;
    let end_idx = (start_idx + 30).min(tab.total_pages);

    if start_idx > 0 {
        sidebar_col = sidebar_col
            .push(Space::new().height(Length::Fixed(start_idx as f32 * thumbnail_height)));
    }

    for page_idx in start_idx..end_idx {
        if let Some(handle) = tab.thumbnails.get(&page_idx) {
            let img = iced::widget::Image::new(handle.clone()).width(Length::Fixed(120.0));
            sidebar_col = sidebar_col
                .push(button(img).on_press(crate::message::Message::JumpToPage(page_idx)));
        } else {
            sidebar_col = sidebar_col.push(
                button(text(format!("Page {}", page_idx + 1)))
                    .on_press(crate::message::Message::JumpToPage(page_idx))
                    .width(Length::Fixed(120.0)),
            );
        }
    }

    let remaining = tab.total_pages.saturating_sub(end_idx);
    if remaining > 0 {
        sidebar_col = sidebar_col
            .push(Space::new().height(Length::Fixed(remaining as f32 * thumbnail_height)));
    }

    scrollable(sidebar_col)
        .id(Id::new("sidebar_scroll"))
        .on_scroll(|viewport| {
            crate::message::Message::SidebarViewportChanged(viewport.absolute_offset().y)
        })
        .width(Length::Fixed(180.0))
        .into()
}

fn render_tabs(app: &PdfBullApp) -> Element<crate::message::Message> {
    let mut tabs = row![];
    for (i, t) in app.tabs.iter().enumerate() {
        let is_active = i == app.active_tab;

        let display_name = if t.name.len() > 20 {
            format!("{}…", &t.name[..18])
        } else {
            t.name.clone()
        };

        let style = if is_active {
            iced::theme::Button::Custom(Box::new(ActiveTabStyle))
        } else {
            iced::theme::Button::Custom(Box::new(InactiveTabStyle))
        };

        let tab_content = row![
            button(text(display_name).size(14))
                .on_press(crate::message::Message::SwitchTab(i))
                .style(style.clone())
                .padding([5, 10]),
            button(text("✕").size(12))
                .on_press(crate::message::Message::CloseTab(i))
                .style(style.clone())
                .padding([5, 8])
        ]
        .spacing(0);

        let wrapped = if t.name.len() > 20 {
            tooltip(
                tab_content,
                t.name.clone(),
                iced::widget::tooltip::Position::Bottom,
            )
            .into()
        } else {
            Element::from(tab_content)
        };

        tabs = tabs.push(wrapped);
    }

    // Fill the rest of the tab bar
    let tab_bar_bg = container(tabs)
        .width(Length::Fill)
        .padding([5, 5, 0, 5])
        .style(|theme: &iced::Theme| {
            let palette = theme.extended_palette();
            container::Style {
                background: Some(iced::Background::Color(palette.background.weak.color)),
                ..Default::default()
            }
        });

    let add_button = button("+")
        .padding(5)
        .on_press(crate::message::Message::OpenDocument);

    row![tab_bar_bg, add_button].padding(5).into()
}

struct ActiveTabStyle;
impl iced::widget::button::StyleSheet for ActiveTabStyle {
    type Style = iced::Theme;
    fn active(&self, theme: &Self::Style) -> iced::widget::button::Appearance {
        let palette = theme.extended_palette();
        iced::widget::button::Appearance {
            background: Some(iced::Background::Color(palette.background.base.color)),
            text_color: palette.background.base.text,
            border: iced::Border {
                radius: [4.0, 4.0, 0.0, 0.0].into(),
                ..Default::default()
            },
            ..Default::default()
        }
    }
}

struct InactiveTabStyle;
impl iced::widget::button::StyleSheet for InactiveTabStyle {
    type Style = iced::Theme;
    fn active(&self, theme: &Self::Style) -> iced::widget::button::Appearance {
        let palette = theme.extended_palette();
        iced::widget::button::Appearance {
            background: Some(iced::Background::Color(palette.background.weak.color)),
            text_color: palette.background.weak.text,
            border: iced::Border {
                radius: [4.0, 4.0, 0.0, 0.0].into(),
                ..Default::default()
            },
            ..Default::default()
        }
    }
}

fn render_pdf_content(app: &PdfBullApp) -> Element<crate::message::Message> {
    let tab = &app.tabs[app.active_tab];

    let mut pdf_column = column![].spacing(10.0).padding(10.0);

    let visible_pages = tab.get_visible_pages();

    for page_idx in 0..tab.total_pages {
        let height = tab.page_heights.get(page_idx).copied().unwrap_or(800.0);
        let placeholder = container(text(format!("Page {}", page_idx + 1)))
            .width(Length::Fill)
            .height(Length::Fixed(height))
            .center_x(Length::Fill)
            .center_y(Length::Fill);

        if visible_pages.contains(&page_idx) {
            if let Some(handle) = tab.rendered_pages.get(&page_idx) {
                let img = iced::widget::Image::new(handle.clone()).width(Length::Fill);

                let mut page_stack = iced::widget::Stack::new().push(img);

                for ann in &tab.annotations {
                    if ann.page == page_idx {
                        let ann_overlay = match &ann.style {
                            crate::models::AnnotationStyle::Highlight { color } => {
                                let (r, g, b) = hex_to_rgb(color);
                                container(
                                    Space::new()
                                        .width(Length::Fixed(ann.width))
                                        .height(Length::Fixed(ann.height)),
                                )
                                .style(move |_| {
                                    iced::widget::container::Style {
                                        background: Some(iced::Background::Color(
                                            iced::Color::from_rgba(r, g, b, 0.4),
                                        )),
                                        ..Default::default()
                                    }
                                })
                            }
                            crate::models::AnnotationStyle::Rectangle {
                                color,
                                thickness,
                                fill,
                            } => {
                                let (r, g, b) = hex_to_rgb(color);
                                container(
                                    Space::new()
                                        .width(Length::Fixed(ann.width))
                                        .height(Length::Fixed(ann.height)),
                                )
                                .style(move |_| {
                                    iced::widget::container::Style {
                                        background: if *fill {
                                            Some(iced::Background::Color(iced::Color::from_rgba(
                                                r, g, b, 0.2,
                                            )))
                                        } else {
                                            None
                                        },
                                        border: iced::Border {
                                            color: iced::Color::from_rgb(r, g, b),
                                            width: *thickness,
                                            radius: iced::border::Radius::from(0.0),
                                        },
                                        ..Default::default()
                                    }
                                })
                            }
                            crate::models::AnnotationStyle::Text {
                                text,
                                color,
                                font_size,
                            } => {
                                let (r, g, b) = hex_to_rgb(color);
                                container(
                                    iced::widget::text(text.clone())
                                        .size(*font_size)
                                        .color(iced::Color::from_rgb(r, g, b)),
                                )
                            }
                        };

                        page_stack =
                            page_stack.push(container(ann_overlay).padding(iced::Padding {
                                top: ann.y,
                                right: 0.0,
                                bottom: 0.0,
                                left: ann.x,
                            }));
                    }
                }

                // Draw search result overlays
                if !app.search_query.is_empty() {
                    for (result_idx, result) in tab.search_results.iter().enumerate() {
                        if result.page == page_idx {
                            let is_active = result_idx == tab.current_search_index;
                            let highlight_color = if is_active {
                                iced::Color::from_rgba(1.0, 0.6, 0.0, 0.6) // orange for active
                            } else {
                                iced::Color::from_rgba(1.0, 1.0, 0.0, 0.4) // yellow for others
                            };

                            // Scale the bounding box coordinates from PDF space to screen space
                            let scale = tab.zoom;
                            let px = result.x * scale;
                            let py = result.y_position * scale;
                            let pw = result.width * scale;
                            let ph = result.height * scale;

                            page_stack = page_stack.push(
                                container(
                                    Space::new()
                                        .width(Length::Fixed(pw))
                                        .height(Length::Fixed(ph)),
                                )
                                .style(move |_| iced::widget::container::Style {
                                    background: Some(iced::Background::Color(highlight_color)),
                                    ..Default::default()
                                })
                                .padding(iced::Padding {
                                    top: py,
                                    left: px,
                                    ..Default::default()
                                }),
                            );
                        }
                    }
                }

                // Draw active drag overlay if any
                if let Some(drag) = &app.annotation_drag {
                    if drag.page == page_idx {
                        let min_x = drag.start.0.min(drag.current.0);
                        let min_y = drag.start.1.min(drag.current.1);
                        let w = (drag.start.0 - drag.current.0).abs();
                        let h = (drag.start.1 - drag.current.1).abs();

                        let preview_bg = match drag.kind {
                            crate::models::PendingAnnotationKind::Highlight => {
                                iced::Color::from_rgba(1.0, 1.0, 0.0, 0.4)
                            }
                            crate::models::PendingAnnotationKind::Rectangle => {
                                iced::Color::from_rgba(1.0, 0.0, 0.0, 0.2)
                            }
                        };

                        let preview_border = match drag.kind {
                            crate::models::PendingAnnotationKind::Highlight => {
                                iced::Border::default()
                            }
                            crate::models::PendingAnnotationKind::Rectangle => iced::Border {
                                color: iced::Color::from_rgb(1.0, 0.0, 0.0),
                                width: 2.0,
                                radius: 0.0.into(),
                            },
                        };

                        page_stack = page_stack.push(
                            container(
                                Space::new()
                                    .width(Length::Fixed(w))
                                    .height(Length::Fixed(h)),
                            )
                            .style(move |_| iced::widget::container::Style {
                                background: Some(iced::Background::Color(preview_bg)),
                                border: preview_border,
                                ..Default::default()
                            })
                            .padding(iced::Padding {
                                top: min_y,
                                left: min_x,
                                ..Default::default()
                            }),
                        );
                    }
                }

                let page_container = container(page_stack)
                    .width(Length::Fixed(page_width))
                    .height(Length::Fixed(page_height))
                    .style(|_| iced::widget::container::Style {
                        background: Some(iced::Background::Color(iced::Color::WHITE)),
                        ..Default::default()
                    });
                pdf_column = pdf_column.push(container(page_container).padding(5));
            } else {
                pdf_column = pdf_column.push(placeholder);
            }
        } else {
            pdf_column = pdf_column.push(placeholder);
        }
    }

    scrollable(container(pdf_column).width(Length::Fill))
        .id(Id::new("pdf_scroll"))
        .on_scroll(|viewport| {
            crate::message::Message::ViewportChanged(
                viewport.absolute_offset().y,
                viewport.bounds().height,
            )
        })
        .height(Length::Fill)
        .into()
}

pub fn document_view(app: &PdfBullApp) -> Element<crate::message::Message> {
    let tab = &app.tabs[app.active_tab];

    let content: Element<crate::message::Message> = if app.show_sidebar && !app.is_fullscreen {
        let sidebar = render_sidebar(app);
        let main_content = render_pdf_content(app);
        row![sidebar, main_content].into()
    } else if tab.total_pages == 0 {
        container(if tab.is_loading {
            column![text("⏳").size(50), text("Loading Document...").size(24)]
                .align_x(iced::Alignment::Center)
                .spacing(20)
                .into()
        } else {
            text("No pages").into()
        })
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
    } else {
        render_pdf_content(app)
    };

    if app.is_fullscreen {
        column![
            content,
            row![
                button("Exit Fullscreen (F)").on_press(crate::message::Message::ToggleFullscreen),
                container(text(format!(
                    "Page {} of {}",
                    tab.current_page + 1,
                    tab.total_pages
                )))
                .padding(10),
                button("-").on_press(crate::message::Message::ZoomOut),
                text(format!("{}%", (tab.zoom * 100.0) as u32)),
                button("+").on_press(crate::message::Message::ZoomIn),
            ]
            .padding(5)
        ]
        .into()
    } else {
        let tabs_row = render_tabs(app);
        let toolbar = render_toolbar(app);
        let page_nav = render_page_nav(app);

        column![tabs_row, toolbar, page_nav, content].into()
    }
}
