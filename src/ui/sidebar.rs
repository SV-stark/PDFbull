use crate::app::PdfBullApp;
use crate::app::{INTER_BOLD, INTER_REGULAR};
use crate::models::{AnnotationStyle, FormFieldVariant, SidebarMode};
use crate::ui::theme;
use iced::widget::{
    Space, button, checkbox, column, container, pick_list, radio, row, scrollable, text,
    text_input, tooltip,
};
use iced::{Alignment, Border, Color, Element, Length};

fn section_header<'a>(
    label: &'a str,
    count_info: Option<String>,
) -> Element<'a, crate::message::Message> {
    let mut header = row![text(label).size(13).font(INTER_BOLD).style(|_| {
        text::Style {
            color: Some(theme::COLOR_TEXT_PRIMARY),
        }
    })]
    .spacing(8)
    .align_y(Alignment::Center);

    if let Some(count) = count_info {
        header = header.push(
            container(
                text(count)
                    .size(10)
                    .font(INTER_BOLD)
                    .style(|_| text::Style {
                        color: Some(Color::WHITE),
                    }),
            )
            .padding([2, 6])
            .style(|_| container::Style {
                background: Some(theme::COLOR_ACCENT.into()),
                border: Border {
                    radius: theme::BORDER_RADIUS_FULL.into(),
                    ..Default::default()
                },
                ..Default::default()
            }),
        );
    }

    container(header)
        .padding([8, 12])
        .width(Length::Fill)
        .style(|_| container::Style {
            background: Some(theme::COLOR_BG_HEADER.into()),
            border: Border {
                width: 1.0,
                color: Color::from_rgb(0.12, 0.14, 0.18),
                ..Default::default()
            },
            ..Default::default()
        })
        .into()
}

fn sidebar_tab_button<'a>(
    icon: &'a str,
    label: &'a str,
    mode: SidebarMode,
    current_mode: SidebarMode,
) -> Element<'a, crate::message::Message> {
    let is_active = mode == current_mode;
    tooltip(
        button(text(icon).size(15).style(move |_| text::Style {
            color: Some(if is_active {
                theme::COLOR_ACCENT
            } else {
                theme::COLOR_TEXT_DIM
            }),
        }))
        .style(move |_theme, status| {
            let base_bg = if is_active {
                Some(theme::COLOR_BG_WIDGET.into())
            } else {
                None
            };
            let border_color = if is_active {
                theme::COLOR_ACCENT
            } else {
                Color::TRANSPARENT
            };

            let base = iced::widget::button::Style {
                background: base_bg,
                border: Border {
                    radius: theme::BORDER_RADIUS_MD.into(),
                    width: if is_active { 1.0 } else { 0.0 },
                    color: border_color,
                },
                ..Default::default()
            };

            match status {
                iced::widget::button::Status::Hovered if !is_active => {
                    iced::widget::button::Style {
                        background: Some(theme::COLOR_BG_WIDGET_HOVER.into()),
                        ..base
                    }
                }
                _ => base,
            }
        })
        .on_press(crate::message::Message::SetSidebarMode(mode))
        .padding([6, 8]),
        label,
        tooltip::Position::Bottom,
    )
    .into()
}

#[allow(clippy::if_not_else)]
pub fn render(app: &PdfBullApp) -> Element<'_, crate::message::Message> {
    let Some(tab) = app.current_tab() else {
        return container(column![]).into();
    };

    let mode_tabs = row![
        sidebar_tab_button(
            "🖼️",
            "Thumbnails",
            SidebarMode::Thumbnails,
            app.sidebar_mode
        ),
        sidebar_tab_button("🔖", "Bookmarks", SidebarMode::Outline, app.sidebar_mode),
        sidebar_tab_button(
            "📝",
            "Annotations",
            SidebarMode::Annotations,
            app.sidebar_mode
        ),
        sidebar_tab_button("🔍", "Search", SidebarMode::Search, app.sidebar_mode),
        sidebar_tab_button(
            "📎",
            "Attachments",
            SidebarMode::Attachments,
            app.sidebar_mode
        ),
        sidebar_tab_button("🥞", "Layers", SidebarMode::Layers, app.sidebar_mode),
    ]
    .spacing(4)
    .padding([6, 8])
    .align_y(Alignment::Center);

    let mode_strip = container(mode_tabs)
        .width(Length::Fill)
        .style(|_| container::Style {
            background: Some(theme::COLOR_BG_SIDEBAR.into()),
            border: Border {
                width: 1.0,
                color: Color::from_rgb(0.12, 0.14, 0.18),
                ..Default::default()
            },
            ..Default::default()
        });

    let main_sidebar = column![mode_strip].spacing(0);

    let content_scroll: Element<'_, crate::message::Message> = match app.sidebar_mode {
        SidebarMode::Thumbnails => {
            let mut thumbnail_col = column![].spacing(12).padding(8).align_x(Alignment::Center);

            thumbnail_col = thumbnail_col.push(section_header(
                "Page Thumbnails",
                Some(format!("{}", tab.total_pages)),
            ));

            let start_idx =
                (tab.view_state.sidebar_viewport_y / theme::THUMBNAIL_HEIGHT).max(0.0) as usize;
            let end_idx = (start_idx + 30).min(tab.total_pages);

            if start_idx > 0 {
                thumbnail_col = thumbnail_col
                    .push(Space::new().height(start_idx as f32 * theme::THUMBNAIL_HEIGHT));
            }

            for page_idx in start_idx..end_idx {
                let is_current = page_idx == tab.current_page;
                let page_label = tab
                    .page_labels
                    .get(page_idx)
                    .cloned()
                    .unwrap_or_else(|| (page_idx + 1).to_string());

                let thumb_widget: Element<'_, crate::message::Message> =
                    if let Some(handle) = tab.view_state.thumbnails.get(&page_idx) {
                        iced::widget::Image::new(handle.clone())
                            .width(Length::Fixed(160.0))
                            .into()
                    } else {
                        container(
                            text("📄")
                                .size(24)
                                .align_x(iced::alignment::Horizontal::Center)
                                .align_y(iced::alignment::Vertical::Center),
                        )
                        .width(Length::Fixed(160.0))
                        .height(Length::Fixed(200.0))
                        .style(|_| container::Style {
                            background: Some(theme::COLOR_BG_WIDGET.into()),
                            ..Default::default()
                        })
                        .into()
                    };

                let card = container(
                    column![
                        container(thumb_widget).padding(2),
                        Space::new().height(4),
                        text(format!("Page {}", page_label))
                            .size(11)
                            .font(INTER_BOLD)
                            .align_x(iced::alignment::Horizontal::Center)
                            .style(move |_| text::Style {
                                color: Some(if is_current {
                                    theme::COLOR_ACCENT
                                } else {
                                    theme::COLOR_TEXT_DIM
                                }),
                            }),
                    ]
                    .align_x(Alignment::Center),
                )
                .padding(6)
                .style(move |_| container::Style {
                    background: Some(if is_current {
                        theme::COLOR_BG_WIDGET_HOVER.into()
                    } else {
                        theme::COLOR_BG_WIDGET.into()
                    }),
                    border: Border {
                        radius: theme::BORDER_RADIUS_MD.into(),
                        width: if is_current { 2.0 } else { 1.0 },
                        color: if is_current {
                            theme::COLOR_ACCENT
                        } else {
                            Color::from_rgb(0.18, 0.20, 0.25)
                        },
                    },
                    ..Default::default()
                });

                thumbnail_col = thumbnail_col.push(
                    button(card)
                        .on_press(crate::message::Message::JumpToPage(page_idx))
                        .style(|_, _| iced::widget::button::Style::default())
                        .width(Length::Fixed(174.0)),
                );
            }

            let remaining = tab.total_pages.saturating_sub(end_idx);
            if remaining > 0 {
                thumbnail_col = thumbnail_col
                    .push(Space::new().height(remaining as f32 * theme::THUMBNAIL_HEIGHT));
            }

            scrollable(thumbnail_col)
                .id("sidebar_scroll")
                .on_scroll(|viewport| {
                    crate::message::Message::SidebarViewportChanged(viewport.absolute_offset().y)
                })
                .width(Length::Fixed(theme::SIDEBAR_WIDTH))
                .into()
        }
        SidebarMode::Outline => {
            let mut outline_col = column![].spacing(10).padding(8);

            if !tab.outline.is_empty() {
                outline_col = outline_col.push(section_header(
                    "Document Outline",
                    Some(format!("{}", tab.outline.len())),
                ));
                let mut list_col = column![].spacing(4);
                for bookmark in &tab.outline {
                    list_col = list_col.push(
                        button(
                            row![
                                text("🔖").size(12),
                                text(&bookmark.title)
                                    .size(12)
                                    .font(INTER_REGULAR)
                                    .style(|_| text::Style {
                                        color: Some(theme::COLOR_TEXT_PRIMARY)
                                    }),
                            ]
                            .spacing(6)
                            .align_y(Alignment::Center),
                        )
                        .on_press(crate::message::Message::JumpToPage(
                            bookmark.page_index as usize,
                        ))
                        .style(theme::button_ghost)
                        .padding([6, 8])
                        .width(Length::Fill),
                    );
                }
                outline_col = outline_col.push(list_col);
            } else {
                outline_col = outline_col.push(section_header("Document Outline", None));
                outline_col =
                    outline_col.push(
                        container(text("No outline bookmarks found").size(12).style(|_| {
                            text::Style {
                                color: Some(theme::COLOR_TEXT_SECONDARY),
                            }
                        }))
                        .padding(12),
                    );
            }

            if !tab.bookmarks.is_empty() {
                outline_col = outline_col.push(section_header(
                    "User Bookmarks",
                    Some(format!("{}", tab.bookmarks.len())),
                ));
                let mut bookmarks_col = column![].spacing(4);
                for (idx, bookmark) in tab.bookmarks.iter().enumerate() {
                    bookmarks_col = bookmarks_col.push(
                        row![
                            button(
                                row![
                                    text("📌").size(12),
                                    text(&bookmark.label).size(12).font(INTER_REGULAR),
                                ]
                                .spacing(6)
                                .align_y(Alignment::Center),
                            )
                            .on_press(crate::message::Message::JumpToBookmark(idx))
                            .style(theme::button_ghost)
                            .padding([6, 8])
                            .width(Length::Fill),
                            button("×")
                                .on_press(crate::message::Message::RemoveBookmark(idx))
                                .style(theme::button_ghost)
                                .padding([4, 8])
                        ]
                        .spacing(4)
                        .align_y(Alignment::Center),
                    );
                }
                outline_col = outline_col.push(bookmarks_col);
            }

            scrollable(outline_col)
                .width(Length::Fixed(theme::SIDEBAR_WIDTH))
                .into()
        }
        SidebarMode::Annotations => {
            let mut ann_col = column![].spacing(10).padding(8);

            ann_col = ann_col.push(section_header(
                "Annotations List",
                Some(format!("{}", tab.annotations.len())),
            ));

            if !tab.annotations.is_empty() {
                let mut list_col = column![].spacing(6);
                for (idx, ann) in tab.annotations.iter().enumerate() {
                    let label = match &ann.style {
                        AnnotationStyle::Highlight { .. } => {
                            format!("Highlight Page {}", ann.page + 1)
                        }
                        AnnotationStyle::Rectangle { .. } => format!("Rect Page {}", ann.page + 1),
                        AnnotationStyle::Text { text, .. } => {
                            format!("Text: {}", &text[..text.len().min(18)])
                        }
                        AnnotationStyle::Redact { .. } => format!("Redact Page {}", ann.page + 1),
                        AnnotationStyle::Circle { .. } => format!("Circle Page {}", ann.page + 1),
                        AnnotationStyle::Line { .. } => format!("Line Page {}", ann.page + 1),
                        AnnotationStyle::Arrow { .. } => format!("Arrow Page {}", ann.page + 1),
                        AnnotationStyle::StickyNote { comment, .. } => {
                            format!("Sticky: {}", &comment[..comment.len().min(18)])
                        }
                    };

                    let icon = match &ann.style {
                        AnnotationStyle::Highlight { .. } => "🖍️",
                        AnnotationStyle::Rectangle { .. } => "🔲",
                        AnnotationStyle::Text { .. } => "🔤",
                        AnnotationStyle::Redact { .. } => "⬛",
                        AnnotationStyle::Circle { .. } => "⭕",
                        AnnotationStyle::Line { .. } => "📏",
                        AnnotationStyle::Arrow { .. } => "➡️",
                        AnnotationStyle::StickyNote { .. } => "📌",
                    };

                    let card = container(
                        row![
                            text(icon).size(12),
                            text(label)
                                .size(12)
                                .font(INTER_REGULAR)
                                .style(|_| text::Style {
                                    color: Some(theme::COLOR_TEXT_PRIMARY),
                                }),
                            Space::new().width(Length::Fill),
                            button("×")
                                .on_press(crate::message::Message::DeleteAnnotation(idx))
                                .style(theme::button_ghost)
                                .padding([2, 6])
                        ]
                        .spacing(6)
                        .align_y(Alignment::Center),
                    )
                    .padding(8)
                    .style(|_| container::Style {
                        background: Some(theme::COLOR_BG_WIDGET.into()),
                        border: Border {
                            radius: theme::BORDER_RADIUS_MD.into(),
                            width: 1.0,
                            color: Color::from_rgb(0.18, 0.20, 0.25),
                        },
                        ..Default::default()
                    });

                    list_col = list_col.push(
                        button(card)
                            .on_press(crate::message::Message::JumpToPage(ann.page))
                            .style(|_, _| iced::widget::button::Style::default())
                            .width(Length::Fill),
                    );
                }
                ann_col = ann_col.push(list_col);
            } else {
                ann_col = ann_col.push(
                    container(
                        text("No annotations found")
                            .size(12)
                            .style(|_| text::Style {
                                color: Some(theme::COLOR_TEXT_SECONDARY),
                            }),
                    )
                    .padding(12),
                );
            }

            scrollable(ann_col)
                .width(Length::Fixed(theme::SIDEBAR_WIDTH))
                .into()
        }
        SidebarMode::Search => {
            let mut search_col = column![].spacing(10).padding(8);

            search_col = search_col.push(section_header(
                "Search Results",
                if tab.search_results.is_empty() {
                    None
                } else {
                    Some(format!("{}", tab.search_results.len()))
                },
            ));

            search_col = search_col.push(
                container(
                    row![
                        text("🔍").size(12),
                        text_input("Type term to search...", &app.search_query)
                            .on_input(crate::message::Message::Search)
                            .padding([4, 6])
                            .size(12)
                    ]
                    .spacing(6)
                    .align_y(Alignment::Center),
                )
                .padding(4)
                .style(theme::input_field),
            );

            if tab.search_results.is_empty() {
                if !app.search_query.is_empty() {
                    search_col = search_col.push(
                        container(text("No matches found").size(12).style(|_| text::Style {
                            color: Some(theme::COLOR_TEXT_SECONDARY),
                        }))
                        .padding(12),
                    );
                }
            } else {
                let mut results_col = column![].spacing(6);
                for (idx, res) in tab.search_results.iter().enumerate() {
                    let is_current = tab.current_search_index == idx;
                    let card = container(
                        column![
                            text(format!("Page {}", res.page + 1))
                                .size(11)
                                .font(INTER_BOLD)
                                .style(|_| text::Style {
                                    color: Some(theme::COLOR_ACCENT)
                                }),
                            text(res.text.clone()).size(11).style(|_| text::Style {
                                color: Some(theme::COLOR_TEXT_PRIMARY)
                            }),
                        ]
                        .spacing(2),
                    )
                    .padding(8)
                    .style(move |_| container::Style {
                        background: Some(if is_current {
                            theme::COLOR_BG_WIDGET_HOVER.into()
                        } else {
                            theme::COLOR_BG_WIDGET.into()
                        }),
                        border: Border {
                            radius: theme::BORDER_RADIUS_MD.into(),
                            width: 1.0,
                            color: if is_current {
                                theme::COLOR_ACCENT
                            } else {
                                Color::from_rgb(0.18, 0.20, 0.25)
                            },
                        },
                        ..Default::default()
                    });

                    results_col = results_col.push(
                        button(card)
                            .on_press(crate::message::Message::JumpToPage(res.page))
                            .style(|_, _| iced::widget::button::Style::default())
                            .width(Length::Fill),
                    );
                }
                search_col = search_col.push(results_col);
            }

            scrollable(search_col)
                .width(Length::Fixed(theme::SIDEBAR_WIDTH))
                .into()
        }
        SidebarMode::Attachments => {
            let mut attach_col = column![].spacing(10).padding(8);
            attach_col = attach_col.push(section_header(
                "Attachments",
                Some(format!("{}", tab.attachments.len())),
            ));

            if tab.attachments.is_empty() {
                attach_col = attach_col.push(
                    container(
                        text("No attachments found")
                            .size(12)
                            .style(|_| text::Style {
                                color: Some(theme::COLOR_TEXT_SECONDARY),
                            }),
                    )
                    .padding(12),
                );
            } else {
                let mut list_col = column![].spacing(6);
                for (idx, att) in tab.attachments.iter().enumerate() {
                    let desc = att.description.as_deref().unwrap_or("No description");
                    let size_str = att
                        .size
                        .map(|s| format!(" ({:.1} KB)", s as f64 / 1024.0))
                        .unwrap_or_default();

                    let card = container(
                        column![
                            row![
                                text(format!("📎 {}", att.name))
                                    .font(INTER_BOLD)
                                    .size(12)
                                    .style(|_| text::Style {
                                        color: Some(Color::WHITE)
                                    }),
                                text(size_str).font(INTER_REGULAR).size(10).style(|_| {
                                    text::Style {
                                        color: Some(theme::COLOR_TEXT_DIM),
                                    }
                                }),
                            ]
                            .spacing(4),
                            text(desc)
                                .font(INTER_REGULAR)
                                .size(11)
                                .style(|_| text::Style {
                                    color: Some(theme::COLOR_TEXT_SECONDARY)
                                }),
                        ]
                        .spacing(2),
                    )
                    .padding(8)
                    .style(|_| container::Style {
                        background: Some(theme::COLOR_BG_WIDGET.into()),
                        border: Border {
                            radius: theme::BORDER_RADIUS_MD.into(),
                            width: 1.0,
                            color: Color::from_rgb(0.18, 0.20, 0.25),
                        },
                        ..Default::default()
                    });

                    list_col = list_col.push(
                        button(card)
                            .on_press(crate::message::Message::SaveAttachment(idx))
                            .style(|_, _| iced::widget::button::Style::default())
                            .width(Length::Fill),
                    );
                }
                attach_col = attach_col.push(list_col);
            }

            scrollable(attach_col)
                .width(Length::Fixed(theme::SIDEBAR_WIDTH))
                .into()
        }
        SidebarMode::Layers => {
            let mut layers_col = column![].spacing(10).padding(8);
            layers_col = layers_col.push(section_header(
                "Content Layers",
                Some(format!("{}", tab.layers.len())),
            ));

            if tab.layers.is_empty() {
                layers_col =
                    layers_col.push(
                        container(text("No optional content layers").size(12).style(|_| {
                            text::Style {
                                color: Some(theme::COLOR_TEXT_SECONDARY),
                            }
                        }))
                        .padding(12),
                    );
            } else {
                let mut list_col = column![].spacing(8);
                for (idx, layer) in tab.layers.iter().enumerate() {
                    list_col = list_col.push(
                        row![
                            checkbox(layer.visible)
                                .on_toggle(move |val| crate::message::Message::ToggleLayer(
                                    idx, val
                                ))
                                .size(14),
                            text(&layer.name).size(12).font(INTER_REGULAR),
                        ]
                        .spacing(8)
                        .align_y(Alignment::Center),
                    );
                }
                layers_col = layers_col.push(container(list_col).padding(8));
            }

            scrollable(layers_col)
                .width(Length::Fixed(theme::SIDEBAR_WIDTH))
                .into()
        }
    };

    main_sidebar.push(content_scroll).into()
}

pub fn render_forms(app: &PdfBullApp) -> Element<'_, crate::message::Message> {
    let mut fields_col = column![
        section_header("Interactive Form Filler", None),
        text("Fill out form fields and save updated PDF.")
            .size(11)
            .style(|_| text::Style {
                color: Some(theme::COLOR_TEXT_DIM)
            }),
        Space::new().height(8),
    ]
    .spacing(8)
    .padding(10);

    if app.form_fields.is_empty() {
        fields_col =
            fields_col.push(text("No interactive form fields found in this document.").size(12));
    } else {
        for field in &app.form_fields {
            let field_widget: Element<_> = match &field.variant {
                FormFieldVariant::Text { value } => text_input("Value...", value)
                    .on_input({
                        let name = field.name.clone();
                        move |val| {
                            crate::message::Message::FormFieldChanged(
                                name.clone(),
                                FormFieldVariant::Text { value: val },
                            )
                        }
                    })
                    .padding(6)
                    .into(),
                FormFieldVariant::Checkbox { is_checked } => checkbox(*is_checked)
                    .on_toggle({
                        let name = field.name.clone();
                        move |val| {
                            crate::message::Message::FormFieldChanged(
                                name.clone(),
                                FormFieldVariant::Checkbox { is_checked: val },
                            )
                        }
                    })
                    .into(),
                FormFieldVariant::RadioButton { is_selected, .. } => {
                    radio("", true, if *is_selected { Some(true) } else { None }, {
                        let name = field.name.clone();
                        move |_| {
                            crate::message::Message::FormFieldChanged(
                                name,
                                FormFieldVariant::RadioButton {
                                    is_selected: true,
                                    group_name: None,
                                },
                            )
                        }
                    })
                    .into()
                }
                FormFieldVariant::ComboBox {
                    options,
                    selected_index,
                } => pick_list(
                    options.clone(),
                    selected_index.and_then(|i| options.get(i).cloned()),
                    {
                        let name = field.name.clone();
                        let options = options.clone();
                        move |val| {
                            let idx = options.iter().position(|o| *o == val);
                            crate::message::Message::FormFieldChanged(
                                name.clone(),
                                FormFieldVariant::ComboBox {
                                    options: options.clone(),
                                    selected_index: idx,
                                },
                            )
                        }
                    },
                )
                .into(),
            };

            fields_col = fields_col.push(
                column![text(&field.name).size(12).font(INTER_BOLD), field_widget].spacing(4),
            );
        }

        fields_col = fields_col.push(
            button(text("💾 Save Filled Form").font(INTER_BOLD).size(12))
                .on_press(crate::message::Message::FillForm(app.form_fields.clone()))
                .width(Length::Fill)
                .padding(10)
                .style(|_, _| iced::widget::button::Style {
                    background: Some(theme::COLOR_ACCENT.into()),
                    text_color: Color::WHITE,
                    border: Border {
                        radius: theme::BORDER_RADIUS_MD.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }),
        );
    }

    scrollable(fields_col)
        .width(Length::Fixed(theme::SIDEBAR_WIDTH))
        .into()
}
