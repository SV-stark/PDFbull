use crate::app::PdfBullApp;
use crate::app::{INTER_BOLD, INTER_REGULAR};
use crate::models::{AnnotationStyle, FormFieldVariant, SidebarMode};
use crate::ui::theme;
use iced::widget::{
    Space, button, checkbox, column, container, pick_list, radio, row, scrollable, text, text_input,
};
use iced::{Border, Color, Element, Length};

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

fn sidebar_tab_style(
    active: bool,
) -> impl Fn(&iced::Theme, iced::widget::button::Status) -> iced::widget::button::Style {
    move |_theme, status| {
        let base_bg = if active {
            Some(theme::COLOR_BG_WIDGET.into())
        } else {
            None
        };
        let border_color = if active {
            theme::COLOR_ACCENT
        } else {
            Color::TRANSPARENT
        };

        let base = iced::widget::button::Style {
            background: base_bg,
            text_color: if active {
                Color::WHITE
            } else {
                theme::COLOR_TEXT_DIM
            },
            border: Border {
                radius: theme::BORDER_RADIUS_SM.into(),
                width: 1.0,
                color: border_color,
            },
            ..Default::default()
        };

        match status {
            iced::widget::button::Status::Hovered if !active => iced::widget::button::Style {
                background: Some(theme::COLOR_BG_WIDGET_HOVER.into()),
                text_color: theme::COLOR_TEXT_PRIMARY,
                ..base
            },
            _ => base,
        }
    }
}

#[allow(clippy::if_not_else)]
pub fn render(app: &PdfBullApp) -> Element<'_, crate::message::Message> {
    let Some(tab) = app.current_tab() else {
        return container(column![]).into();
    };

    let tab_row = row![
        button(text("📂").size(14))
            .style(sidebar_tab_style(
                app.sidebar_mode == SidebarMode::Thumbnails
            ))
            .on_press(crate::message::Message::SetSidebarMode(
                SidebarMode::Thumbnails
            )),
        button(text("🔖").size(14))
            .style(sidebar_tab_style(app.sidebar_mode == SidebarMode::Outline))
            .on_press(crate::message::Message::SetSidebarMode(
                SidebarMode::Outline
            )),
        button(text("📝").size(14))
            .style(sidebar_tab_style(
                app.sidebar_mode == SidebarMode::Annotations
            ))
            .on_press(crate::message::Message::SetSidebarMode(
                SidebarMode::Annotations
            )),
        button(text("🔍").size(14))
            .style(sidebar_tab_style(app.sidebar_mode == SidebarMode::Search))
            .on_press(crate::message::Message::SetSidebarMode(SidebarMode::Search)),
        button(text("📎").size(14))
            .style(sidebar_tab_style(
                app.sidebar_mode == SidebarMode::Attachments
            ))
            .on_press(crate::message::Message::SetSidebarMode(
                SidebarMode::Attachments
            )),
        button(text("🥞").size(14))
            .style(sidebar_tab_style(app.sidebar_mode == SidebarMode::Layers))
            .on_press(crate::message::Message::SetSidebarMode(SidebarMode::Layers)),
    ]
    .spacing(4)
    .padding([5, 10]);

    let main_sidebar = column![tab_row].spacing(10);

    let content_scroll: Element<'_, crate::message::Message> = match app.sidebar_mode {
        SidebarMode::Thumbnails => {
            let mut thumbnail_col = column![].spacing(10).padding(5);

            thumbnail_col = thumbnail_col.push(
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

            let start_idx =
                (tab.view_state.sidebar_viewport_y / theme::THUMBNAIL_HEIGHT).max(0.0) as usize;
            let end_idx = (start_idx + 30).min(tab.total_pages);

            if start_idx > 0 {
                thumbnail_col = thumbnail_col
                    .push(Space::new().height(start_idx as f32 * theme::THUMBNAIL_HEIGHT));
            }

            for page_idx in start_idx..end_idx {
                if let Some(handle) = tab.view_state.thumbnails.get(&page_idx) {
                    let img = iced::widget::Image::new(handle.clone())
                        .width(Length::Fixed(theme::THUMBNAIL_WIDTH));
                    thumbnail_col = thumbnail_col
                        .push(button(img).on_press(crate::message::Message::JumpToPage(page_idx)));
                } else {
                    let page_label = tab
                        .page_labels
                        .get(page_idx)
                        .cloned()
                        .unwrap_or_else(|| (page_idx + 1).to_string());
                    thumbnail_col = thumbnail_col.push(
                        button(text(format!("Page {}", page_label)).font(INTER_REGULAR))
                            .on_press(crate::message::Message::JumpToPage(page_idx))
                            .width(Length::Fixed(theme::THUMBNAIL_WIDTH)),
                    );
                }
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
            let mut outline_col = column![].spacing(10).padding(5);

            if !tab.outline.is_empty() {
                let mut list_col = column![section_title("Outline")];
                for bookmark in &tab.outline {
                    list_col = list_col.push(
                        button(text(&bookmark.title))
                            .on_press(crate::message::Message::JumpToPage(
                                bookmark.page_index as usize,
                            ))
                            .width(Length::Fill),
                    );
                }
                outline_col = outline_col.push(container(list_col.spacing(5)).padding(10));
            } else {
                outline_col = outline_col.push(
                    container(text("No outline bookmarks found").size(12).style(|_| {
                        iced::widget::text::Style {
                            color: Some(theme::COLOR_TEXT_SECONDARY),
                        }
                    }))
                    .padding(15),
                );
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
                outline_col = outline_col.push(container(bookmarks_col.spacing(5)).padding(10));
            }

            scrollable(outline_col)
                .width(Length::Fixed(theme::SIDEBAR_WIDTH))
                .into()
        }
        SidebarMode::Annotations => {
            let mut ann_col = column![].spacing(10).padding(5);

            if !tab.annotations.is_empty() {
                let mut list_col = column![section_title("Annotations")];
                for (idx, ann) in tab.annotations.iter().enumerate() {
                    let label = match &ann.style {
                        AnnotationStyle::Highlight { .. } => format!("Highlight P{}", ann.page + 1),
                        AnnotationStyle::Rectangle { .. } => format!("Rect P{}", ann.page + 1),
                        AnnotationStyle::Text { text, .. } => {
                            format!("Text: {}", &text[..text.len().min(20)])
                        }
                        AnnotationStyle::Redact { .. } => format!("Redact P{}", ann.page + 1),
                        AnnotationStyle::Circle { .. } => format!("Circle P{}", ann.page + 1),
                        AnnotationStyle::Line { .. } => format!("Line P{}", ann.page + 1),
                        AnnotationStyle::Arrow { .. } => format!("Arrow P{}", ann.page + 1),
                        AnnotationStyle::StickyNote { comment, .. } => {
                            format!("Sticky: {}", &comment[..comment.len().min(20)])
                        }
                    };
                    let mut ann_row = row![].spacing(5).align_y(iced::Alignment::Center);

                    if let AnnotationStyle::Text { text, .. }
                    | AnnotationStyle::StickyNote { comment: text, .. } = &ann.style
                    {
                        ann_row = ann_row.push(
                            text_input("Edit annotation...", text)
                                .on_input(move |s| {
                                    crate::message::Message::EditAnnotationText(idx, s)
                                })
                                .size(12)
                                .padding([3, 6])
                                .width(Length::Fill),
                        );
                    } else {
                        ann_row = ann_row.push(
                            button(text(label))
                                .on_press(crate::message::Message::JumpToPage(ann.page))
                                .width(Length::Fill),
                        );
                    }

                    ann_row = ann_row
                        .push(button("×").on_press(crate::message::Message::DeleteAnnotation(idx)));

                    list_col = list_col.push(ann_row);
                }
                ann_col = ann_col.push(container(list_col.spacing(5)).padding(10));
            } else {
                ann_col = ann_col.push(
                    container(text("No annotations found").size(12).style(|_| {
                        iced::widget::text::Style {
                            color: Some(theme::COLOR_TEXT_SECONDARY),
                        }
                    }))
                    .padding(15),
                );
            }

            scrollable(ann_col)
                .width(Length::Fixed(theme::SIDEBAR_WIDTH))
                .into()
        }
        SidebarMode::Search => {
            let mut search_col = column![].spacing(10).padding(5);

            search_col = search_col.push(section_title("Search"));

            search_col = search_col.push(
                text_input("Search term...", &app.search_query)
                    .on_input(crate::message::Message::Search)
                    .padding(8),
            );

            if tab.search_results.is_empty() {
                if !app.search_query.is_empty() {
                    search_col = search_col.push(
                        container(text("No matches found").size(12).style(|_| {
                            iced::widget::text::Style {
                                color: Some(theme::COLOR_TEXT_SECONDARY),
                            }
                        }))
                        .padding(15),
                    );
                } else {
                    search_col = search_col.push(
                        container(
                            text("Type a query to search standard document text")
                                .size(12)
                                .style(|_| iced::widget::text::Style {
                                    color: Some(theme::COLOR_TEXT_SECONDARY),
                                }),
                        )
                        .padding(15),
                    );
                }
            } else {
                search_col = search_col.push(
                    text(format!("{} matches found", tab.search_results.len()))
                        .size(11)
                        .style(|_| iced::widget::text::Style {
                            color: Some(theme::COLOR_TEXT_DIM),
                        }),
                );

                let mut results_col = column![].spacing(8);
                for (idx, res) in tab.search_results.iter().enumerate() {
                    let is_current = tab.current_search_index == idx;
                    let border_color = if is_current {
                        theme::COLOR_ACCENT
                    } else {
                        Color::from_rgb(0.2, 0.2, 0.22)
                    };

                    let card = container(
                        column![
                            text(format!("Page {}", res.page + 1))
                                .size(11)
                                .font(INTER_BOLD)
                                .style(|_| iced::widget::text::Style {
                                    color: Some(theme::COLOR_ACCENT)
                                }),
                            text(res.text.clone())
                                .size(12)
                                .style(|_| iced::widget::text::Style {
                                    color: Some(theme::COLOR_TEXT_PRIMARY)
                                }),
                        ]
                        .spacing(2),
                    )
                    .padding(8)
                    .style(move |_| iced::widget::container::Style {
                        background: Some(if is_current {
                            theme::COLOR_BG_WIDGET_HOVER.into()
                        } else {
                            theme::COLOR_BG_WIDGET.into()
                        }),
                        border: Border {
                            radius: theme::BORDER_RADIUS_MD.into(),
                            width: 1.0,
                            color: border_color,
                        },
                        ..Default::default()
                    });

                    results_col = results_col.push(
                        button(card)
                            .on_press(crate::message::Message::JumpToPage(res.page))
                            .style(move |_, _| iced::widget::button::Style {
                                background: None,
                                border: Border::default(),
                                ..Default::default()
                            })
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
            let mut attach_col = column![].spacing(10).padding(5);
            attach_col = attach_col.push(section_title("Attachments"));

            if tab.attachments.is_empty() {
                attach_col = attach_col.push(
                    container(text("No attachments found").size(12).style(|_| {
                        iced::widget::text::Style {
                            color: Some(theme::COLOR_TEXT_SECONDARY),
                        }
                    }))
                    .padding(15),
                );
            } else {
                let mut list_col = column![].spacing(8);
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
                            color: Color::from_rgb(0.2, 0.2, 0.22),
                        },
                        ..Default::default()
                    });

                    list_col = list_col.push(
                        button(card)
                            .on_press(crate::message::Message::SaveAttachment(idx))
                            .style(|_, _| iced::widget::button::Style {
                                background: None,
                                border: Border::default(),
                                ..Default::default()
                            })
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
            let mut layers_col = column![].spacing(10).padding(5);
            layers_col = layers_col.push(section_title("Layers"));

            if tab.layers.is_empty() {
                layers_col = layers_col.push(
                    container(text("No optional content layers").size(12).style(|_| {
                        iced::widget::text::Style {
                            color: Some(theme::COLOR_TEXT_SECONDARY),
                        }
                    }))
                    .padding(15),
                );
            } else {
                let mut list_col = column![].spacing(10);
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
                        .align_y(iced::Alignment::Center),
                    );
                }
                layers_col = layers_col.push(container(list_col).padding(10));
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
        section_title("Interactive Form"),
        text("Fill out the fields below and save as a new PDF.")
            .size(12)
            .style(|_| iced::widget::text::Style {
                color: Some(theme::COLOR_TEXT_DIM)
            }),
        Space::new().height(10),
    ]
    .spacing(10)
    .padding(15);

    if app.form_fields.is_empty() {
        fields_col =
            fields_col.push(text("No interactive fields found in this document.").size(13));
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
                    .padding(8)
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

    scrollable(fields_col)
        .width(Length::Fixed(theme::SIDEBAR_WIDTH))
        .into()
}
