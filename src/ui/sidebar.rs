use crate::app::PdfBullApp;
use crate::app::{INTER_BOLD, INTER_REGULAR};
use crate::models::{AnnotationStyle, FormFieldVariant};
use crate::ui::theme;
use iced::widget::{
    button, checkbox, column, container, pick_list, radio, row, scrollable, text, text_input, Space,
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

pub fn render(app: &PdfBullApp) -> Element<'_, crate::message::Message> {
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
                AnnotationStyle::Redact { .. } => {
                    format!("Redact P{}", ann.page + 1)
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

    if !tab.signatures.is_empty() {
        let mut signatures_col = column![section_title("Signatures")];
        for sig in &tab.signatures {
            signatures_col = signatures_col.push(
                container(
                    column![
                        text(&sig.name).size(13).font(INTER_BOLD),
                        text(if sig.is_valid {
                            "✓ Valid Signature"
                        } else {
                            "✗ Invalid/Untrusted"
                        })
                        .size(11)
                        .style(|_| iced::widget::text::Style {
                            color: Some(if sig.is_valid {
                                Color::from_rgb(0.0, 0.6, 0.0)
                            } else {
                                Color::from_rgb(0.8, 0.0, 0.0)
                            })
                        }),
                        text(format!(
                            "Reason: {}",
                            sig.reason.as_deref().unwrap_or("N/A")
                        ))
                        .size(10),
                    ]
                    .spacing(2),
                )
                .padding(8)
                .style(|_| iced::widget::container::Style {
                    background: Some(Color::from_rgb8(45, 46, 50).into()),
                    border: Border {
                        radius: 4.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }),
            );
        }
        sidebar_col = sidebar_col.push(container(signatures_col.spacing(5)).padding(10));
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

pub fn render_forms(app: &PdfBullApp) -> Element<'_, crate::message::Message> {
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
                FormFieldVariant::Checkbox { is_checked } => checkbox("", *is_checked)
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
                                name.clone(),
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
