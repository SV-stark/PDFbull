use crate::app::PdfBullApp;
use crate::app::{icons, INTER_BOLD, INTER_REGULAR, LUCIDE};
use crate::models::{
    AnnotationStyle, DocumentTab, PendingAnnotationKind,
};
use crate::pdf_engine::RenderFilter;
use crate::ui::theme::{self, hex_to_rgb};
use iced::widget::{
    button, column, container, mouse_area, row, scrollable, text, text_input, tooltip, Space, Stack,
};
use iced::{Alignment, Border, Color, Element, Length, Padding, Shadow, Vector};

fn stacked_tool<'a>(
    top: impl Into<Element<'a, crate::message::Message>>,
    label: &'a str,
) -> Element<'a, crate::message::Message> {
    column![
        top.into(),
        text(label)
            .font(INTER_REGULAR)
            .style(|_theme| iced::widget::text::Style {
                color: Some(theme::COLOR_TEXT_DIM),
            })
    ]
    .spacing(4)
    .align_x(Alignment::Center)
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
            .font(INTER_BOLD)
            .align_x(iced::alignment::Horizontal::Center),
    );
    if f == active {
        btn.style(|_theme, _| iced::widget::button::Style {
            background: Some(Color::from_rgb8(150, 220, 220).into()),
            text_color: Color::BLACK,
            border: iced::Border {
                radius: 4.0.into(),
                ..Default::default()
            },
            ..Default::default()
        })
    } else {
        btn.style(|_theme, _status| iced::widget::button::Style {
            background: Some(Color::from_rgb8(60, 60, 65).into()),
            text_color: Color::WHITE,
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
    let Some(tab) = app.current_tab() else {
        return container(row![]).into();
    };

    let open_btn = stacked_tool(
        tooltip(
            button(text(icons::OPEN).size(16).font(LUCIDE))
                .on_press(crate::message::Message::OpenDocument)
                .style(iced::widget::button::text),
            "Open another PDF",
            tooltip::Position::Bottom,
        ),
        "Open",
    );

    let sidebar_btn = stacked_tool(
        tooltip(
            button(text(icons::SIDEBAR).size(16).font(LUCIDE))
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
                button(text(icons::ZOOM_OUT).size(14).font(LUCIDE))
                    .on_press(crate::message::Message::ZoomOut)
                    .style(|_theme, _status| iced::widget::button::Style {
                        text_color: Color::WHITE,
                        ..Default::default()
                    }),
                text(format!("{}%", (tab.zoom * 100.0) as u32))
                    .size(13)
                    .font(INTER_BOLD)
                    .style(|_theme| iced::widget::text::Style {
                        color: Some(Color::WHITE)
                    }),
                button(text(icons::ZOOM_IN).size(14).font(LUCIDE))
                    .on_press(crate::message::Message::ZoomIn)
                    .style(|_theme, _status| iced::widget::button::Style {
                        text_color: Color::WHITE,
                        ..Default::default()
                    }),
            ]
            .spacing(12)
            .align_y(Alignment::Center),
        )
        .padding([4, 10])
        .style(|_theme| iced::widget::container::Style {
            background: Some(Color::from_rgb8(30, 31, 34).into()),
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
            button(text(icons::ROTATE).size(16).font(LUCIDE))
                .on_press(crate::message::Message::RotateClockwise)
                .style(iced::widget::button::text),
            "Rotate 90° clockwise",
            tooltip::Position::Bottom,
        ),
        "Rotate",
    );

    let crop_filters = stacked_tool(
        row![
            filter_btn_custom("B&W", RenderFilter::BlackWhite, tab.render_filter),
            filter_btn_custom("Lighten", RenderFilter::Lighten, tab.render_filter),
            filter_btn_custom("NoShad", RenderFilter::NoShadow, tab.render_filter),
        ]
        .spacing(2),
        "Crop",
    );

    let forms_btn = stacked_tool(
        tooltip(
            button(text(icons::FORMS).size(16).font(LUCIDE))
                .on_press(crate::message::Message::ToggleFormsSidebar)
                .style(iced::widget::button::text),
            "Interactive Forms",
            tooltip::Position::Bottom,
        ),
        "Forms",
    );

    let bookmark_btn = stacked_tool(
        button(text(icons::BOOKMARK).size(16).font(LUCIDE))
            .on_press(crate::message::Message::AddBookmark)
            .style(iced::widget::button::text),
        "Bookmark",
    );

    let highlight_btn = stacked_tool(
        button(text(icons::HIGHLIGHT).size(16).font(LUCIDE))
            .on_press(crate::message::Message::SetAnnotationMode(Some(
                PendingAnnotationKind::Highlight,
            )))
            .style(iced::widget::button::text),
        "Highlight",
    );

    let rectangle_btn = stacked_tool(
        button(text(icons::RECTANGLE).size(16).font(LUCIDE))
            .on_press(crate::message::Message::SetAnnotationMode(Some(
                PendingAnnotationKind::Rectangle,
            )))
            .style(iced::widget::button::text),
        "Rectangle",
    );

    let save_anns_btn = stacked_tool(
        button(text(icons::SAVE).font(LUCIDE).size(16).color(Color::BLACK))
            .on_press(crate::message::Message::SaveAnnotations)
            .style(|_theme, _status| iced::widget::button::Style {
                background: Some(Color::from_rgb8(150, 220, 220).into()),
                border: iced::Border {
                    radius: 12.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            })
            .padding([8, 16]),
        "Save",
    );

    let right_tools = column![
        row![
            button(text(icons::HELP).size(16).font(LUCIDE))
                .on_press(crate::message::Message::ToggleKeyboardHelp)
                .style(iced::widget::button::text),
            button(text(icons::SETTINGS).size(16).font(LUCIDE))
                .on_press(crate::message::Message::OpenSettings)
                .style(iced::widget::button::text),
        ]
        .spacing(12)
        .align_y(Alignment::Center),
        button(
            row![
                text(icons::EXPORT).size(12).font(LUCIDE),
                text("Export").size(11).font(INTER_REGULAR),
            ]
            .spacing(4)
            .align_y(Alignment::Center)
        )
        .on_press(crate::message::Message::ExportImage)
        .style(iced::widget::button::text),
        button(
            row![
                text(icons::PRINT).size(12).font(LUCIDE),
                text("Print").size(11).font(INTER_REGULAR),
            ]
            .spacing(4)
            .align_y(Alignment::Center)
        )
        .on_press(crate::message::Message::Print)
        .style(iced::widget::button::text),
        button(text("WM").size(11).font(INTER_BOLD))
            .on_press(crate::message::Message::AddWatermark("CONFIDENTIAL".into()))
            .style(iced::widget::button::text),
    ]
    .spacing(2)
    .align_x(Alignment::Center);

    container(
        row![
            open_btn,
            Space::new(15, 0),
            sidebar_btn,
            Space::new(25, 0),
            zoom_controls,
            Space::new(20, 0),
            rotate_btn,
            Space::new(20, 0),
            crop_filters,
            Space::new(Length::Fill, 0),
            forms_btn,
            Space::new(15, 0),
            bookmark_btn,
            Space::new(15, 0),
            highlight_btn,
            Space::new(15, 0),
            rectangle_btn,
            Space::new(20, 0),
            save_anns_btn,
            Space::new(20, 0),
            right_tools,
        ]
        .spacing(5)
        .align_y(Alignment::Center),
    )
    .width(Length::Fill)
    .padding(Padding {
        top: 8.0,
        right: 15.0,
        bottom: 8.0,
        left: 15.0,
    })
    .style(|_theme| iced::widget::container::Style {
        background: Some(Color::from_rgb8(43, 45, 49).into()),
        shadow: Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.2),
            offset: Vector::new(0.0, 2.0),
            blur_radius: 8.0,
        },
        ..Default::default()
    })
    .into()
}

fn render_page_nav(app: &PdfBullApp) -> Element<'_, crate::message::Message> {
    let Some(tab) = app.current_tab() else {
        return container(row![]).into();
    };

    let loading_indicator = if app.rendering_count > 0 {
        row![
            container(
                text(format!("{}", app.rendering_count))
                    .font(INTER_BOLD)
                    .size(12)
            )
            .padding([2, 6])
            .style(|_| iced::widget::container::Style {
                background: Some(theme::COLOR_ACCENT.into()),
                text_color: Some(Color::WHITE),
                border: iced::Border {
                    radius: 10.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }),
            text("Rendering")
                .size(12)
                .font(INTER_REGULAR)
                .style(|_theme| iced::widget::text::Style {
                    color: Some(theme::COLOR_TEXT_DIM)
                })
        ]
        .spacing(8)
        .align_y(Alignment::Center)
    } else {
        row![]
    };

    container(
        row![
            Space::new(Length::Fill, 0),
            button(text(icons::PREV).size(14).font(LUCIDE))
                .on_press(crate::message::Message::PrevPage)
                .style(|_theme, _status| iced::widget::button::Style {
                    background: Some(Color::from_rgb8(60, 60, 65).into()),
                    text_color: Color::WHITE,
                    border: iced::Border {
                        radius: 6.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .padding([6, 12]),
            text("Page")
                .size(13)
                .font(INTER_REGULAR)
                .style(|_theme| iced::widget::text::Style {
                    color: Some(theme::COLOR_TEXT_DIM)
                }),
            container(
                text_input("Page", &app.page_input)
                    .on_input(|input| {
                        if input.is_empty() || input.parse::<usize>().is_ok() {
                            crate::message::Message::PageInputChanged(input)
                        } else {
                            crate::message::Message::PageInputChanged(app.page_input.clone())
                        }
                    })
                    .on_submit(crate::message::Message::PageInputSubmitted)
                    .font(INTER_BOLD)
                    .width(Length::Fixed(40.0))
            )
            .padding(1)
            .style(|_theme| iced::widget::container::Style {
                border: iced::Border {
                    width: 1.0,
                    color: Color::from_rgb8(80, 80, 80),
                    radius: 6.0.into()
                },
                background: Some(Color::from_rgb8(30, 31, 34).into()),
                ..Default::default()
            }),
            text(format!("of {}", tab.total_pages.max(1)))
                .size(13)
                .font(INTER_REGULAR)
                .style(|_theme| iced::widget::text::Style {
                    color: Some(theme::COLOR_TEXT_DIM)
                }),
            button(text(icons::NEXT).size(14).font(LUCIDE))
                .on_press(crate::message::Message::NextPage)
                .style(|_theme, _status| iced::widget::button::Style {
                    background: Some(Color::from_rgb8(60, 60, 65).into()),
                    text_color: Color::WHITE,
                    border: iced::Border {
                        radius: 6.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .padding([6, 12]),
            Space::new(Length::Fill, 0),
            loading_indicator,
            Space::new(15, 0),
            container(
                row![
                    text(icons::SEARCH).font(LUCIDE).size(14).style(|_| {
                        iced::widget::text::Style {
                            color: Some(Color::from_rgb8(150, 150, 150)),
                        }
                    }),
                    text_input("Search...", &app.search_query)
                        .on_input(crate::message::Message::Search)
                        .on_submit(crate::message::Message::NextSearchResult)
                        .font(INTER_REGULAR)
                        .width(Length::Fixed(180.0))
                ]
                .spacing(8)
                .align_y(Alignment::Center)
                .padding([0, 12])
            )
            .style(|_theme| iced::widget::container::Style {
                border: iced::Border {
                    width: 1.0,
                    color: Color::from_rgb8(80, 80, 80),
                    radius: 20.0.into()
                },
                background: Some(Color::from_rgb8(30, 31, 34).into()),
                ..Default::default()
            }),
        ]
        .spacing(10)
        .align_y(Alignment::Center),
    )
    .width(Length::Fill)
    .padding(Padding {
        top: 6.0,
        right: 15.0,
        bottom: 6.0,
        left: 15.0,
    })
    .style(|_theme| iced::widget::container::Style {
        background: Some(Color::from_rgb8(35, 36, 40).into()),
        border: iced::Border {
            width: 1.0,
            color: Color::from_rgb8(20, 20, 20),
            ..Default::default()
        },
        ..Default::default()
    })
    .into()
}

fn section_title<'a>(label: &'a str) -> Element<'a, crate::message::Message> {
    container(
        text(label)
            .size(14)
            .font(INTER_BOLD)
            .style(|_| iced::widget::text::Style {
                color: Some(Color::from_rgb(0.2, 0.4, 0.6)),
            }),
    )
    .padding([5, 0])
    .into()
}

fn render_sidebar(app: &PdfBullApp) -> Element<'_, crate::message::Message> {
    let Some(tab) = app.current_tab() else {
        return container(column![]).into();
    };

    let mut sidebar_col = column![]
        .spacing(10)
        .padding(5)
        .width(Length::Fixed(theme::SIDEBAR_WIDTH));

    if !tab.outline.is_empty() {
        let mut outline_col = column![section_title("Outline")];
        for bookmark in &tab.outline {
            outline_col = outline_col.push(
                button(text(&bookmark.title))
                    .on_press(crate::message::Message::JumpToPage(
                        bookmark.page_index as usize,
                    ))
                    .width(Length::Fill),
            );
        }
        sidebar_col = sidebar_col.push(container(outline_col.spacing(5)).padding(10));
    }

    if !tab.bookmarks.is_empty() {
        let mut bookmarks_col = column![section_title("Bookmarks")];
        for (idx, bookmark) in tab.bookmarks.iter().enumerate() {
            bookmarks_col = bookmarks_col.push(row![
                button(text(&bookmark.label))
                    .on_press(crate::message::Message::JumpToBookmark(idx))
                    .width(Length::Fill),
                button("×").on_press(crate::message::Message::RemoveBookmark(idx))
            ]);
        }
        sidebar_col = sidebar_col.push(container(bookmarks_col.spacing(5)).padding(10));
    }

    if !tab.annotations.is_empty() {
        let mut annotations_col = column![section_title("Annotations")];
        for (idx, ann) in tab.annotations.iter().enumerate() {
            let label = match &ann.style {
                AnnotationStyle::Highlight { .. } => {
                    format!("Highlight P{}", ann.page + 1)
                }
                AnnotationStyle::Rectangle { .. } => {
                    format!("Rect P{}", ann.page + 1)
                }
                AnnotationStyle::Text { text, .. } => {
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
        sidebar_col = sidebar_col.push(container(annotations_col.spacing(5)).padding(10));
    }

    sidebar_col = sidebar_col.push(
        container(
            text(format!("Pages ({})", tab.total_pages))
                .size(14)
                .font(INTER_BOLD)
                .style(|_| iced::widget::text::Style {
                    color: Some(Color::from_rgb(0.2, 0.4, 0.6)),
                }),
        )
        .padding([5, 0]),
    );

    let start_idx = (tab.view_state.sidebar_viewport_y / theme::THUMBNAIL_HEIGHT).max(0.0) as usize;
    let end_idx = (start_idx + 30).min(tab.total_pages);

    if start_idx > 0 {
        sidebar_col = sidebar_col.push(Space::new(0, start_idx as f32 * theme::THUMBNAIL_HEIGHT));
    }

    for page_idx in start_idx..end_idx {
        if let Some(handle) = tab.view_state.thumbnails.get(&page_idx) {
            let img = iced::widget::Image::new(handle.clone())
                .width(Length::Fixed(theme::THUMBNAIL_WIDTH));
            sidebar_col = sidebar_col
                .push(button(img).on_press(crate::message::Message::JumpToPage(page_idx)));
        } else {
            sidebar_col = sidebar_col.push(
                button(text(format!("Page {}", page_idx + 1)).font(INTER_REGULAR))
                    .on_press(crate::message::Message::JumpToPage(page_idx))
                    .width(Length::Fixed(theme::THUMBNAIL_WIDTH)),
            );
        }
    }

    let remaining = tab.total_pages.saturating_sub(end_idx);
    if remaining > 0 {
        sidebar_col = sidebar_col.push(Space::new(0, remaining as f32 * theme::THUMBNAIL_HEIGHT));
    }

    scrollable(sidebar_col)
        .id(scrollable::Id::new("sidebar_scroll"))
        .on_scroll(|viewport| {
            crate::message::Message::SidebarViewportChanged(viewport.absolute_offset().y)
        })
        .width(Length::Fixed(theme::SIDEBAR_WIDTH))
        .into()
}

fn render_forms_sidebar(app: &PdfBullApp) -> Element<'_, crate::message::Message> {
    let mut fields_col = column![
        section_title("Interactive Form"),
        text("Fill out the fields below and save as a new PDF.")
            .size(12)
            .style(|_| iced::widget::text::Style {
                color: Some(theme::COLOR_TEXT_DIM)
            }),
        Space::new(0, 10),
    ]
    .spacing(10)
    .padding(15);

    if app.form_fields.is_empty() {
        fields_col =
            fields_col.push(text("No interactive fields found in this document.").size(13));
    } else {
        for field in &app.form_fields {
            fields_col = fields_col.push(
                column![
                    text(&field.name).size(12).font(INTER_BOLD),
                    text_input("Value...", &field.value)
                        .on_input({
                            let name = field.name.clone();
                            move |val| crate::message::Message::FormFieldChanged(name.clone(), val)
                        })
                        .padding(8)
                ]
                .spacing(4),
            );
        }

        fields_col = fields_col.push(
            button(text("Save Filled Form").font(INTER_BOLD))
                .on_press(crate::message::Message::FillForm(app.form_fields.clone()))
                .width(Length::Fill)
                .padding(10)
                .style(|_, _| iced::widget::button::Style {
                    background: Some(theme::COLOR_ACCENT.into()),
                    text_color: Color::WHITE,
                    border: Border {
                        radius: 8.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }),
        );
    }

    container(scrollable(fields_col))
        .width(Length::Fixed(250.0))
        .style(|_| iced::widget::container::Style {
            background: Some(Color::from_rgb8(35, 36, 40).into()),
            border: Border {
                width: 1.0,
                color: Color::from_rgb8(50, 52, 56),
                ..Default::default()
            },
            ..Default::default()
        })
        .into()
}

fn render_tabs(app: &PdfBullApp) -> Element<'_, crate::message::Message> {
    let mut tabs = row![];
    for (i, t) in app.tabs.iter().enumerate() {
        let is_active = i == app.active_tab;

        let display_name = if t.name.chars().count() > 20 {
            format!("{}…", t.name.chars().take(18).collect::<String>())
        } else {
            t.name.clone()
        };

        let tab_bg = if is_active {
            Color::from_rgb8(43, 45, 49)
        } else {
            Color::from_rgb8(30, 31, 34)
        };

        let text_color = if is_active {
            Color::WHITE
        } else {
            Color::from_rgb8(150, 150, 150)
        };

        let tab_content = row![
            text(icons::OPEN).size(14).font(LUCIDE),
            Space::new(6, 0),
            text(display_name)
                .size(13)
                .font(if is_active { INTER_BOLD } else { INTER_REGULAR })
                .style(move |_theme| iced::widget::text::Style {
                    color: Some(text_color)
                }),
            Space::new(10, 0),
            button(
                text(icons::CLOSE)
                    .size(12)
                    .font(LUCIDE)
                    .style(move |_theme| iced::widget::text::Style {
                        color: Some(text_color)
                    })
            )
            .on_press(crate::message::Message::CloseTab(i))
            .style(iced::widget::button::text)
            .padding(2)
        ]
        .align_y(Alignment::Center);

        tabs = tabs.push(
            container(tab_content)
                .padding(Padding {
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
                        color: Color::from_rgb8(20, 20, 20),
                    },
                    ..Default::default()
                }),
        );
        tabs = tabs.push(Space::new(2, 0));
    }

    let add_button =
        button(
            text(icons::PLUS)
                .size(16)
                .font(LUCIDE)
                .style(|_theme| iced::widget::text::Style {
                    color: Some(Color::WHITE),
                }),
        )
        .padding([4, 10])
        .on_press(crate::message::Message::OpenDocument)
        .style(iced::widget::button::text);

    let tab_bar_bg = container(row![tabs, add_button].align_y(Alignment::End))
        .width(Length::Fill)
        .padding(Padding {
            top: 6.0,
            right: 5.0,
            bottom: 0.0,
            left: 10.0,
        })
        .style(|_theme| iced::widget::container::Style {
            background: Some(Color::from_rgb8(25, 26, 28).into()),
            ..Default::default()
        });

    tab_bar_bg.into()
}

fn render_annotations<'a>(
    page_idx: usize,
    tab: &'a DocumentTab,
    zoom: f32,
) -> Vec<Element<'a, crate::message::Message>> {
    tab.annotations
        .iter()
        .filter(|ann| ann.page == page_idx)
        .map(|ann| {
            let ann_overlay = match &ann.style {
                AnnotationStyle::Highlight { color } => {
                    let (r, g, b) = hex_to_rgb(color);
                    container(Space::new(0, 0))
                        .width(Length::Fixed(ann.width * zoom))
                        .height(Length::Fixed(ann.height * zoom))
                        .style(move |_| iced::widget::container::Style {
                            background: Some(Color::from_rgba(r, g, b, 0.4).into()),
                            ..Default::default()
                        })
                }
                AnnotationStyle::Rectangle {
                    color,
                    thickness,
                    fill,
                } => {
                    let (r, g, b) = hex_to_rgb(color);
                    container(Space::new(0, 0))
                        .width(Length::Fixed(ann.width * zoom))
                        .height(Length::Fixed(ann.height * zoom))
                        .style(move |_| iced::widget::container::Style {
                            background: if *fill {
                                Some(Color::from_rgba(r, g, b, 0.2).into())
                            } else {
                                None
                            },
                            border: iced::Border {
                                color: Color::from_rgb(r, g, b),
                                width: *thickness * zoom,
                                radius: 0.0.into(),
                            },
                            ..Default::default()
                        })
                }
                AnnotationStyle::Text {
                    text,
                    color,
                    font_size,
                } => {
                    let (r, g, b) = hex_to_rgb(color);
                    container(
                        iced::widget::text(text.clone())
                            .size(*font_size as f32 * zoom)
                            .font(INTER_REGULAR)
                            .color(Color::from_rgb(r, g, b)),
                    )
                }
            };

            container(ann_overlay)
                .padding(Padding {
                    top: ann.y * zoom,
                    left: ann.x * zoom,
                    ..Default::default()
                })
                .into()
        })
        .collect()
}

fn render_hyperlinks<'a>(
    page_idx: usize,
    tab: &'a DocumentTab,
    zoom: f32,
) -> Vec<Element<'a, crate::message::Message>> {
    tab.links
        .iter()
        .filter(|link| link.page == page_idx)
        .map(|link| {
            let (lx, ly, lw, lh) = link.bounds;
            let overlay = mouse_area(
                container(Space::new(0, 0))
                    .width(Length::Fixed(lw * zoom))
                    .height(Length::Fixed(lh * zoom))
                    .style(|_| iced::widget::container::Style::default()),
            )
            .on_release(crate::message::Message::LinkClicked(link.clone()));

            container(overlay)
                .padding(Padding {
                    top: ly * zoom,
                    left: lx * zoom,
                    ..Default::default()
                })
                .into()
        })
        .collect()
}

fn render_search_highlights<'a>(
    page_idx: usize,
    tab: &'a DocumentTab,
    zoom: f32,
    app: &'a PdfBullApp,
) -> Vec<Element<'a, crate::message::Message>> {
    if app.search_query.is_empty() {
        return vec![];
    }

    tab.search_results
        .iter()
        .enumerate()
        .filter(|(_, result)| result.page == page_idx)
        .map(|(result_idx, result)| {
            let is_active = result_idx == tab.current_search_index;
            let highlight_color = if is_active {
                Color::from_rgba(1.0, 0.6, 0.0, 0.6)
            } else {
                Color::from_rgba(1.0, 1.0, 0.0, 0.4)
            };

            container(
                Space::new(0, 0)
                    .width(Length::Fixed(result.width * zoom))
                    .height(Length::Fixed(result.height * zoom)),
            )
            .style(move |_| iced::widget::container::Style {
                background: Some(iced::Background::Color(highlight_color)),
                ..Default::default()
            })
            .padding(Padding {
                top: result.y_position * zoom,
                left: result.x * zoom,
                ..Default::default()
            })
            .into()
        })
        .collect()
}

fn render_active_drag<'a>(
    page_idx: usize,
    zoom: f32,
    app: &'a PdfBullApp,
) -> Vec<Element<'a, crate::message::Message>> {
    if let Some(drag) = &app.annotation_drag {
        if drag.page == page_idx {
            let min_x = drag.start.0.min(drag.current.0);
            let min_y = drag.start.1.min(drag.current.1);
            let w = (drag.start.0 - drag.current.0).abs();
            let h = (drag.start.1 - drag.current.1).abs();

            let preview_bg = match drag.kind {
                PendingAnnotationKind::Highlight => Color::from_rgba(1.0, 1.0, 0.0, 0.4),
                PendingAnnotationKind::Rectangle => Color::from_rgba(1.0, 0.0, 0.0, 0.2),
            };

            let preview_border = match drag.kind {
                PendingAnnotationKind::Highlight => iced::Border::default(),
                PendingAnnotationKind::Rectangle => iced::Border {
                    color: Color::from_rgb(1.0, 0.0, 0.0),
                    width: 2.0 * zoom,
                    radius: 0.0.into(),
                },
            };

            return vec![container(
                Space::new(0, 0)
                    .width(Length::Fixed(w * zoom))
                    .height(Length::Fixed(h * zoom)),
            )
            .style(move |_| iced::widget::container::Style {
                background: Some(iced::Background::Color(preview_bg)),
                border: preview_border,
                ..Default::default()
            })
            .padding(Padding {
                top: min_y * zoom,
                left: min_x * zoom,
                ..Default::default()
            })
            .into()];
        }
    }
    vec![]
}

fn render_page_canvas<'a>(
    page_idx: usize,
    tab: &'a DocumentTab,
    app: &'a PdfBullApp,
) -> Element<'a, crate::message::Message> {
    let zoom = tab.zoom;
    let original_height = tab.page_heights.get(page_idx).copied().unwrap_or(800.0);
    let scaled_height = original_height * zoom;
    let scaled_width = tab.page_width * zoom;

    if let Some((_, handle)) = tab.view_state.rendered_pages.get(&page_idx) {
        let img = iced::widget::Image::new(handle.clone())
            .width(Length::Fixed(scaled_width))
            .height(Length::Fixed(scaled_height));

        let mut page_stack = Stack::new().push(img);

        for el in render_annotations(page_idx, tab, zoom) {
            page_stack = page_stack.push(el);
        }
        for el in render_hyperlinks(page_idx, tab, zoom) {
            page_stack = page_stack.push(el);
        }
        for el in render_search_highlights(page_idx, tab, zoom, app) {
            page_stack = page_stack.push(el);
        }
        for el in render_active_drag(page_idx, zoom, app) {
            page_stack = page_stack.push(el);
        }

        container(page_stack)
            .width(Length::Fixed(scaled_width))
            .height(Length::Fixed(scaled_height))
            .style(|_| iced::widget::container::Style {
                background: Some(iced::Background::Color(Color::WHITE)),
                ..Default::default()
            })
            .into()
    } else {
        container(
            column![
                text(format!("Page {}", page_idx + 1))
                    .font(INTER_REGULAR)
                    .size(16),
                text("Loading...").size(12).font(INTER_REGULAR).style(|_| {
                    iced::widget::text::Style {
                        color: Some(theme::COLOR_TEXT_DIM),
                    }
                })
            ]
            .spacing(10)
            .align_x(Alignment::Center),
        )
        .width(Length::Fixed(scaled_width))
        .height(Length::Fixed(scaled_height))
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .style(|_| iced::widget::container::Style {
            background: Some(Color::from_rgb8(45, 46, 50).into()),
            ..Default::default()
        })
        .into()
    }
}

fn render_pdf_content(app: &PdfBullApp) -> Element<'_, crate::message::Message> {
    let Some(tab) = app.current_tab() else {
        return container(column![]).into();
    };
    let zoom = tab.zoom;
    let scaled_spacing = theme::PAGE_SPACING * zoom;

    let mut pdf_column = column![]
        .spacing(scaled_spacing)
        .padding(theme::PAGE_PADDING * zoom)
        .align_x(Alignment::Center);

    let (start_idx, end_idx) = tab.view_state.visible_range;

    if start_idx > 0 {
        let y_above: f32 = tab
            .page_heights
            .iter()
            .take(start_idx)
            .map(|h| (h + theme::PAGE_SPACING) * zoom)
            .sum();
        let y_above = (y_above - scaled_spacing).max(0.0);
        if y_above > 0.0 {
            pdf_column = pdf_column.push(Space::new(0, y_above));
        }
    }

    for page_idx in start_idx..end_idx {
        pdf_column = pdf_column.push(render_page_canvas(page_idx, tab, app));
    }

    if end_idx < tab.total_pages {
        let y_below: f32 = tab
            .page_heights
            .iter()
            .skip(end_idx)
            .map(|h| (h + theme::PAGE_SPACING) * zoom)
            .sum();
        let y_below = (y_below - scaled_spacing).max(0.0);
        if y_below > 0.0 {
            pdf_column = pdf_column.push(Space::new(0, y_below));
        }
    }

    scrollable(
        container(pdf_column)
            .width(Length::Fill)
            .center_x(Length::Fill),
    )
    .id(scrollable::Id::new("pdf_scroll"))
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
    let Some(tab) = app.current_tab() else {
        return container(text("Loading tab..."))
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into();
    };

    let mut content_row = row![];

    if app.show_sidebar && !app.is_fullscreen {
        content_row = content_row.push(render_sidebar(app));
    }

    content_row = content_row.push(render_pdf_content(app));

    if app.show_forms_sidebar && !app.is_fullscreen {
        content_row = content_row.push(render_forms_sidebar(app));
    }

    let content: Element<crate::message::Message> = if tab.total_pages == 0 {
        let empty_content: Element<_> = if tab.view_state.is_loading {
            column![text("⏳").size(50), text("Loading Document...").size(24)]
                .align_x(Alignment::Center)
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
        content_row.into()
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
        column![
            render_tabs(app),
            render_toolbar(app),
            render_page_nav(app),
            content
        ]
        .into()
    }
}
