use crate::app::{INTER_BOLD, INTER_REGULAR, LUCIDE, icons};
use crate::storage;
use crate::ui::theme;
use iced::widget::{Space, button, column, container, row, scrollable, text};
use iced::{Alignment, Border, Color, Element, Length, Shadow, Vector};

fn custom_card<'a>(
    content: impl Into<Element<'a, crate::message::Message>>,
) -> Element<'a, crate::message::Message> {
    container(content)
        .padding(32)
        .style(|_theme| iced::widget::container::Style {
            background: Some(theme::COLOR_BG_WIDGET.into()),
            border: Border {
                radius: theme::BORDER_RADIUS_LG.into(),
                width: 1.0,
                color: Color::from_rgb(0.2, 0.2, 0.22),
            },
            shadow: Shadow {
                color: Color::from_rgba(0.0, 0.0, 0.0, 0.4),
                offset: Vector::new(0.0, 8.0),
                blur_radius: 20.0,
            },
            ..Default::default()
        })
        .into()
}

fn quick_action_card<'a>(
    icon: &'static str,
    title: &'static str,
    description: &'static str,
    action_label: &'static str,
    on_press: crate::message::Message,
) -> Element<'a, crate::message::Message> {
    container(
        column![
            container(text(icon).size(32).font(LUCIDE))
                .padding(16)
                .style(|_| iced::widget::container::Style {
                    background: Some(theme::COLOR_BG_HEADER.into()),
                    border: Border {
                        radius: theme::BORDER_RADIUS_MD.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }),
            column![
                text(title)
                    .size(18)
                    .font(INTER_BOLD)
                    .style(|_| iced::widget::text::Style {
                        color: Some(theme::COLOR_TEXT_PRIMARY)
                    }),
                text(description)
                    .size(13)
                    .font(INTER_REGULAR)
                    .style(|_| iced::widget::text::Style {
                        color: Some(theme::COLOR_TEXT_DIM)
                    })
                    .align_x(Alignment::Center),
            ]
            .spacing(4)
            .align_x(Alignment::Center),
            button(text(action_label).size(13).font(INTER_BOLD))
                .on_press(on_press)
                .padding([10, 24])
                .style(theme::button_tool(true)),
        ]
        .align_x(Alignment::Center)
        .spacing(20),
    )
    .width(Length::FillPortion(1))
    .padding(32)
    .style(|_| iced::widget::container::Style {
        background: Some(theme::COLOR_BG_APP.into()),
        border: Border {
            width: 1.0,
            color: Color::from_rgb(0.15, 0.15, 0.17),
            radius: theme::BORDER_RADIUS_LG.into(),
        },
        ..Default::default()
    })
    .into()
}

pub fn welcome_view(app: &crate::app::PdfBullApp) -> Element<'_, crate::message::Message> {
    let recent_section: Element<'_, crate::message::Message> = if app.recent_files.is_empty() {
        column![].into()
    } else {
        let files = iced::widget::column(app.recent_files.iter().map(|file| {
            let file_row = row![
                container(text(icons::OPEN).size(14).font(LUCIDE))
                    .padding(8)
                    .style(|_| iced::widget::container::Style {
                        background: Some(theme::COLOR_BG_HEADER.into()),
                        border: Border {
                            radius: theme::BORDER_RADIUS_SM.into(),
                            ..Default::default()
                        },
                        ..Default::default()
                    }),
                column![
                    text(&file.name).size(14).font(INTER_BOLD).style(|_| {
                        iced::widget::text::Style {
                            color: Some(theme::COLOR_TEXT_PRIMARY),
                        }
                    }),
                    text(file.path.clone())
                        .size(12)
                        .font(INTER_REGULAR)
                        .style(|_| iced::widget::text::Style {
                            color: Some(theme::COLOR_TEXT_SECONDARY)
                        }),
                ]
                .spacing(2),
                Space::new().width(Length::Fill),
                text(storage::time_ago(file.last_opened))
                    .size(11)
                    .font(INTER_REGULAR)
                    .style(|_| iced::widget::text::Style { color: Some(theme::COLOR_TEXT_SECONDARY) }),
            ]
            .spacing(16)
            .align_y(iced::Alignment::Center);

            button(file_row)
                .on_press(crate::message::Message::OpenRecentFile(file.clone()))
                .width(Length::Fill)
                .padding(12)
                .style(theme::button_ghost)
                .into()
        }))
        .spacing(8);

        column![
            row![
                text("Recent Documents")
                    .size(14)
                    .font(INTER_BOLD)
                    .style(|_| iced::widget::text::Style { color: Some(theme::COLOR_TEXT_DIM) }),
                Space::new().width(Length::Fill),
                button(text("Clear History").size(12).font(INTER_REGULAR))
                    .on_press(crate::message::Message::ClearRecentFiles)
                    .style(iced::widget::button::text)
            ]
            .align_y(Alignment::Center)
            .padding([0, 8]),
            Space::new().height(12),
            files,
        ].into()
    };

    let actions = row![
        quick_action_card(icons::OPEN, "Open Document", "Pick a PDF to start reading", "Browse Files", crate::message::Message::OpenDocument),
        quick_action_card(icons::MERGE, "Merge PDFs", "Combine multiple files easily", "Select Files", crate::message::Message::MergeDocuments(vec![])),
    ]
    .spacing(24)
    .width(Length::Fill);

    let content = column![
        column![
            text("PDFbull").size(42).font(INTER_BOLD).style(|_| iced::widget::text::Style { color: Some(theme::COLOR_TEXT_PRIMARY) }),
            text("FAST • LIGHT • SECURE").size(12).font(INTER_BOLD).style(|_| iced::widget::text::Style { color: Some(theme::COLOR_ACCENT) }),
        ].align_x(Alignment::Center).spacing(8),
        Space::new().height(48),
        custom_card(column![
            text("Getting Started").size(20).font(INTER_BOLD).style(|_| iced::widget::text::Style { color: Some(theme::COLOR_TEXT_PRIMARY) }),
            Space::new().height(24),
            actions,
        ].align_x(Alignment::Center)),
        Space::new().height(32),
        recent_section,
    ]
    .width(Length::Fixed(800.0))
    .align_x(Alignment::Center);

    container(scrollable(
        container(content)
            .width(Length::Fill)
            .padding(64)
            .center_x(Length::Fill),
    ))
    .width(Length::Fill)
    .height(Length::Fill)
    .style(|_| iced::widget::container::Style {
        background: Some(theme::COLOR_BG_APP.into()),
        ..Default::default()
    })
    .into()
}
