use crate::app::PdfBullApp;
use crate::pdf_engine::RenderFilter;
use iced::widget::{button, column, container, row, scrollable, text, text_input, Id, Space};
use iced::{Color, Element, Length};
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

fn filter_btn(
    label: &'static str,
    f: RenderFilter,
    active: RenderFilter,
) -> iced::widget::Button<'static, crate::message::Message> {
    let btn = button(text(label).size(11));
    if f == active {
        btn.style(|theme: &iced::Theme, _| {
            let palette = theme.extended_palette();
            iced::widget::button::Style {
                background: Some(palette.primary.strong.color.into()),
                text_color: palette.primary.strong.text,
                ..Default::default()
            }
        })
    } else {
        btn
    }
    .on_press(crate::message::Message::SetFilter(f))
}

use iced::widget::tooltip;

fn stacked_tool<'a>(
    top: impl Into<Element<'a, crate::message::Message>>,
    label: &'a str,
) -> Element<'a, crate::message::Message> {
    column![
        top.into(),
        text(label)
            .size(11)
            .style(|_theme| iced::widget::text::Style {
                color: Some(Color::from_rgb8(180, 180, 180)),
            })
    ]
    .spacing(4)
    .align_x(iced::Alignment::Center)
    .into()
}

fn filter_btn_custom(
    label: &'static str,
    f: RenderFilter,
    active: RenderFilter,
) -> iced::widget::Button<'static, crate::message::Message> {
    let btn = button(
        text(label)
            .size(11)
            .align_x(iced::alignment::Horizontal::Center),
    );
    if f == active {
        btn.style(|theme: &iced::Theme, _| {
            let _palette = theme.extended_palette();
            iced::widget::button::Style {
                background: Some(iced::Color::from_rgb8(150, 220, 220).into()),
                text_color: iced::Color::BLACK,
                border: iced::Border {
                    radius: 4.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        })
    } else {
        btn.style(|_theme, _status| iced::widget::button::Style {
            background: Some(iced::Color::from_rgb8(60, 60, 65).into()),
            text_color: iced::Color::WHITE,
            border: iced::Border {
                radius: 4.0.into(),
                ..Default::default()
            },
            ..Default::default()
        })
    }
    .on_press(crate::message::Message::SetFilter(f))
}

fn render_toolbar(app: &PdfBullApp) -> Element<'_, crate::message::Message> {
    let tab = &app.tabs[app.active_tab];

    let open_btn = stacked_tool(
        tooltip(
            button(text("📂").size(16))
                .on_press(crate::message::Message::OpenDocument)
                .style(iced::widget::button::text),
            "Open another PDF",
            tooltip::Position::Bottom,
        ),
        "Open",
    );

    let sidebar_btn = stacked_tool(
        tooltip(
            button(text("☰").size(16))
                .on_press(crate::message::Message::ToggleSidebar)
                .style(iced::widget::button::text),
            "Toggle Sidebar (Ctrl+B)",
            tooltip::Position::Bottom,
        ),
        "Sidebar",
    );

    let zoom_controls = stacked_tool(
        container(
            row![
                button(text("-").size(14))
                    .on_press(crate::message::Message::ZoomOut)
                    .style(|_theme, _status| iced::widget::button::Style {
                        text_color: iced::Color::WHITE,
                        ..Default::default()
                    }),
                text(format!("{}%", (tab.zoom * 100.0) as u32))
                    .size(13)
                    .style(|_theme| iced::widget::text::Style {
                        color: Some(Color::WHITE)
                    }),
                button(text("+").size(14))
                    .on_press(crate::message::Message::ZoomIn)
                    .style(|_theme, _status| iced::widget::button::Style {
                        text_color: iced::Color::WHITE,
                        ..Default::default()
                    }),
            ]
            .spacing(5)
            .align_y(iced::Alignment::Center),
        )
        .padding([4, 10])
        .style(|_theme| iced::widget::container::Style {
            background: Some(iced::Color::from_rgb8(30, 31, 34).into()),
            border: iced::Border {
                radius: 20.0.into(),
                ..Default::default()
            },
            ..Default::default()
        }),
        "Zoom",
    );

    let rotate_btn = stacked_tool(
        tooltip(
            button(text("↻").size(16))
                .on_press(crate::message::Message::RotateClockwise)
                .style(iced::widget::button::text),
            "Rotate 90° clockwise",
            tooltip::Position::Bottom,
        ),
        "Rotate",
    );

    let filters_dropdown = row![
        text("Filters")
            .size(12)
            .style(|_theme| iced::widget::text::Style {
                color: Some(Color::from_rgb8(180, 180, 180)),
            }),
        button(text("None v").size(12)).style(iced::widget::button::text)
    ]
    .spacing(5)
    .align_y(iced::Alignment::Center);

    let crop_filters = stacked_tool(
        row![
            filter_btn_custom("B&W", RenderFilter::BlackWhite, tab.render_filter),
            filter_btn_custom("Lighten", RenderFilter::Lighten, tab.render_filter),
            filter_btn_custom("NoShad", RenderFilter::NoShadow, tab.render_filter),
        ]
        .spacing(2),
        "Crop",
    );

    let bookmark_btn = stacked_tool(
        button(text("🔖").size(16))
            .on_press(crate::message::Message::AddBookmark)
            .style(iced::widget::button::text),
        "Bookmark",
    );

    let highlight_btn = stacked_tool(
        button(text("🖊").size(16))
            .on_press(crate::message::Message::SetAnnotationMode(Some(
                crate::models::PendingAnnotationKind::Highlight,
            )))
            .style(iced::widget::button::text),
        "Highlight",
    );

    let rectangle_btn = stacked_tool(
        button(text("□").size(16))
            .on_press(crate::message::Message::SetAnnotationMode(Some(
                crate::models::PendingAnnotationKind::Rectangle,
            )))
            .style(iced::widget::button::text),
        "Rectangle",
    );

    let save_anns_btn = stacked_tool(
        button(text("Save Anns").color(iced::Color::BLACK).size(13))
            .on_press(crate::message::Message::SaveAnnotations)
            .style(|_theme, _status| iced::widget::button::Style {
                background: Some(iced::Color::from_rgb8(150, 220, 220).into()),
                border: iced::Border {
                    radius: 8.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            })
            .padding([6, 12]),
        "Save",
    );

    let right_tools = column![
        row![
            button(text("?").size(14))
                .on_press(crate::message::Message::ToggleKeyboardHelp)
                .style(iced::widget::button::text),
            button(text("⚙").size(14))
                .on_press(crate::message::Message::OpenSettings)
                .style(iced::widget::button::text),
        ]
        .spacing(10)
        .align_y(iced::Alignment::Center),
        button(
            text("Export v")
                .size(11)
                .style(|_theme| iced::widget::text::Style {
                    color: Some(Color::from_rgb8(180, 180, 180)),
                })
        )
        .style(iced::widget::button::text),
    ]
    .spacing(2)
    .align_x(iced::Alignment::Center);

    container(
        row![
            open_btn,
            Space::new().width(Length::Fixed(15.0)),
            sidebar_btn,
            Space::new().width(Length::Fixed(25.0)),
            zoom_controls,
            Space::new().width(Length::Fixed(20.0)),
            rotate_btn,
            Space::new().width(Length::Fixed(20.0)),
            filters_dropdown,
            Space::new().width(Length::Fixed(10.0)),
            crop_filters,
            Space::new().width(Length::Fill),
            bookmark_btn,
            Space::new().width(Length::Fixed(15.0)),
            highlight_btn,
            Space::new().width(Length::Fixed(15.0)),
            rectangle_btn,
            Space::new().width(Length::Fixed(20.0)),
            save_anns_btn,
            Space::new().width(Length::Fixed(20.0)),
            right_tools,
        ]
        .spacing(5)
        .align_y(iced::Alignment::Center),
    )
    .width(Length::Fill)
    .padding(iced::Padding {
        top: 8.0,
        right: 15.0,
        bottom: 8.0,
        left: 15.0,
    })
    .style(|_theme| iced::widget::container::Style {
        background: Some(iced::Color::from_rgb8(43, 45, 49).into()),
        ..Default::default()
    })
    .into()
}

fn render_page_nav(app: &PdfBullApp) -> Element<'_, crate::message::Message> {
    let tab = &app.tabs[app.active_tab];

    let loading_indicator = if app.rendering_count > 0 {
        row![
            iced_aw::widget::Badge::new(text(format!("{}", app.rendering_count))),
            text("Rendering")
                .size(12)
                .style(|_theme| iced::widget::text::Style {
                    color: Some(Color::from_rgb8(180, 180, 180))
                })
        ]
        .spacing(5)
    } else {
        row![]
    };

    container(
        row![
            Space::new().width(Length::Fill),
            button(text("Prev").size(13))
                .on_press(crate::message::Message::PrevPage)
                .style(|_theme, _status| iced::widget::button::Style {
                    background: Some(iced::Color::from_rgb8(60, 60, 65).into()),
                    text_color: iced::Color::WHITE,
                    border: iced::Border {
                        radius: 4.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .padding([4, 12]),
            text("Page")
                .size(13)
                .style(|_theme| iced::widget::text::Style {
                    color: Some(Color::from_rgb8(180, 180, 180))
                }),
            container(
                text_input("", &app.page_input)
                    .on_input(crate::message::Message::PageInputChanged)
                    .on_submit(crate::message::Message::PageInputSubmitted)
                    .width(Length::Fixed(40.0))
            )
            .padding(1)
            .style(|_theme| iced::widget::container::Style {
                border: iced::Border {
                    width: 1.0,
                    color: iced::Color::from_rgb8(80, 80, 80),
                    radius: 4.0.into()
                },
                background: Some(iced::Color::from_rgb8(30, 31, 34).into()),
                ..Default::default()
            }),
            text(format!("of {}", tab.total_pages.max(1)))
                .size(13)
                .style(|_theme| iced::widget::text::Style {
                    color: Some(Color::from_rgb8(180, 180, 180))
                }),
            button(text("Next").size(13))
                .on_press(crate::message::Message::NextPage)
                .style(|_theme, _status| iced::widget::button::Style {
                    background: Some(iced::Color::from_rgb8(60, 60, 65).into()),
                    text_color: iced::Color::WHITE,
                    border: iced::Border {
                        radius: 4.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .padding([4, 12]),
            Space::new().width(Length::Fill),
            loading_indicator,
            Space::new().width(Length::Fixed(15.0)),
            container(
                text_input("Search...", &app.search_query)
                    .on_input(crate::message::Message::Search)
                    .on_submit(crate::message::Message::NextSearchResult)
                    .width(Length::Fixed(200.0))
            )
            .style(|_theme| iced::widget::container::Style {
                border: iced::Border {
                    width: 1.0,
                    color: iced::Color::from_rgb8(80, 80, 80),
                    radius: 20.0.into()
                },
                background: Some(iced::Color::from_rgb8(30, 31, 34).into()),
                ..Default::default()
            }),
        ]
        .spacing(10)
        .align_y(iced::Alignment::Center),
    )
    .width(Length::Fill)
    .padding(iced::Padding {
        top: 6.0,
        right: 15.0,
        bottom: 6.0,
        left: 15.0,
    })
    .style(|_theme| iced::widget::container::Style {
        background: Some(iced::Color::from_rgb8(35, 36, 40).into()),
        border: iced::Border {
            width: 1.0,
            color: iced::Color::from_rgb8(20, 20, 20),
            ..Default::default()
        },
        ..Default::default()
    })
    .into()
}

fn render_sidebar(app: &PdfBullApp) -> Element<'_, crate::message::Message> {
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
                {
                    let mut outline_col = column![];
                    for bookmark in &tab.outline {
                        outline_col = outline_col.push(
                            button(text(&bookmark.title))
                                .on_press(crate::message::Message::JumpToPage(
                                    bookmark.page_index as usize,
                                ))
                                .width(Length::Fill),
                        );
                    }
                    outline_col.spacing(5)
                },
            )
            .padding(10.0.into()),
        );
    }

    if !tab.bookmarks.is_empty() {
        sidebar_col = sidebar_col.push(
            Card::new(text("Bookmarks").size(14), {
                let mut bookmarks_col = column![];
                for (idx, bookmark) in tab.bookmarks.iter().enumerate() {
                    bookmarks_col = bookmarks_col.push(row![
                        button(text(&bookmark.label))
                            .on_press(crate::message::Message::JumpToBookmark(idx))
                            .width(Length::Fill),
                        button("×").on_press(crate::message::Message::RemoveBookmark(idx))
                    ]);
                }
                bookmarks_col.spacing(5)
            })
            .padding(10.0.into()),
        );
    }

    if !tab.annotations.is_empty() {
        sidebar_col = sidebar_col.push(
            Card::new(text("Annotations").size(14), {
                let mut annotations_col = column![];
                for (idx, ann) in tab.annotations.iter().enumerate() {
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
                    annotations_col = annotations_col.push(row![
                        button(text(label))
                            .on_press(crate::message::Message::JumpToPage(ann.page))
                            .width(Length::Fill),
                        button("×").on_press(crate::message::Message::DeleteAnnotation(idx))
                    ]);
                }
                annotations_col.spacing(5)
            })
            .padding(10.0.into()),
        );
    }

    sidebar_col = sidebar_col.push(
        Card::new(
            text(format!("Pages ({})", tab.total_pages)).size(14),
            Space::new(),
        )
        .padding(10.0.into()),
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

fn render_tabs(app: &PdfBullApp) -> Element<'_, crate::message::Message> {
    let mut tabs = row![];
    for (i, t) in app.tabs.iter().enumerate() {
        let is_active = i == app.active_tab;

        let display_name = if t.name.len() > 20 {
            format!("{}…", &t.name[..18])
        } else {
            t.name.clone()
        };

        // For the active tab we use a lighter background color to blend with the toolbar
        // The screenshot uses an arched tab background.
        let tab_bg = if is_active {
            iced::Color::from_rgb8(43, 45, 49)
        } else {
            iced::Color::from_rgb8(30, 31, 34)
        };

        let text_color = if is_active {
            iced::Color::WHITE
        } else {
            iced::Color::from_rgb8(150, 150, 150)
        };

        let tab_content = row![
            text("📄").size(14),
            Space::new().width(6.0),
            text(display_name)
                .size(13)
                .style(move |_theme| iced::widget::text::Style {
                    color: Some(text_color)
                }),
            Space::new().width(10.0),
            button(
                text("✕")
                    .size(12)
                    .style(move |_theme| iced::widget::text::Style {
                        color: Some(text_color)
                    })
            )
            .on_press(crate::message::Message::CloseTab(i))
            .style(iced::widget::button::text)
            .padding(2)
        ]
        .align_y(iced::Alignment::Center);

        tabs = tabs.push(
            container(tab_content)
                .padding(iced::Padding {
                    top: 6.0,
                    right: 12.0,
                    bottom: 6.0,
                    left: 12.0,
                })
                .style(move |_theme| iced::widget::container::Style {
                    background: Some(tab_bg.into()),
                    border: iced::Border {
                        radius: iced::border::Radius {
                            top_left: 8.0,
                            top_right: 8.0,
                            bottom_left: 0.0,
                            bottom_right: 0.0,
                        },
                        width: if is_active { 0.0 } else { 1.0 },
                        color: iced::Color::from_rgb8(20, 20, 20),
                    },
                    ..Default::default()
                }),
        );
        tabs = tabs.push(Space::new().width(2.0)); // spacing between tabs
    }

    let add_button = button(
        text("+")
            .size(16)
            .style(|_theme| iced::widget::text::Style {
                color: Some(Color::WHITE),
            }),
    )
    .padding([4, 10])
    .on_press(crate::message::Message::OpenDocument)
    .style(iced::widget::button::text);

    let tab_bar_bg = container(row![tabs, add_button].align_y(iced::Alignment::End))
        .width(Length::Fill)
        .padding(iced::Padding {
            top: 6.0,
            right: 5.0,
            bottom: 0.0,
            left: 10.0,
        })
        .style(|_theme| iced::widget::container::Style {
            background: Some(iced::Color::from_rgb8(25, 26, 28).into()),
            ..Default::default()
        });

    tab_bar_bg.into()
}

fn render_pdf_content(app: &PdfBullApp) -> Element<'_, crate::message::Message> {
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

                let page_width = tab.page_width * tab.zoom;
                let page_height = height;

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

pub fn document_view(app: &PdfBullApp) -> Element<'_, crate::message::Message> {
    let tab = &app.tabs[app.active_tab];

    let content: Element<crate::message::Message> = if app.show_sidebar && !app.is_fullscreen {
        let sidebar = render_sidebar(app);
        let main_content = render_pdf_content(app);
        row![sidebar, main_content].into()
    } else if tab.total_pages == 0 {
        let empty_content: Element<_> = if tab.is_loading {
            column![text("⏳").size(50), text("Loading Document...").size(24)]
                .align_x(iced::Alignment::Center)
                .spacing(20)
                .into()
        } else {
            text("No pages").into()
        };
        container(empty_content)
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
