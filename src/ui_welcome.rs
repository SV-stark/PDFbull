use crate::app::{INTER_BOLD, INTER_REGULAR, LUCIDE, icons};
use crate::storage;
use crate::ui::theme;
use iced::widget::{Space, button, column, container, row, scrollable, text};
use iced::{Alignment, Border, Color, Element, Length, Shadow, Vector};

fn feature_card<'a>(
    emoji: &'static str,
    title: &'static str,
    description: &'static str,
    action_msg: crate::message::Message,
) -> Element<'a, crate::message::Message> {
    button(
        container(
            column![
                row![
                    container(text(emoji).size(24)).padding(10).style(|_| {
                        iced::widget::container::Style {
                            background: Some(theme::COLOR_BG_HEADER.into()),
                            border: Border {
                                radius: theme::BORDER_RADIUS_MD.into(),
                                ..Default::default()
                            },
                            ..Default::default()
                        }
                    }),
                    Space::new().width(Length::Fill),
                    text("➡️").size(14).style(|_| text::Style {
                        color: Some(theme::COLOR_TEXT_DIM)
                    }),
                ]
                .align_y(Alignment::Center),
                Space::new().height(16),
                text(title)
                    .size(16)
                    .font(INTER_BOLD)
                    .style(|_| text::Style {
                        color: Some(theme::COLOR_TEXT_PRIMARY)
                    }),
                Space::new().height(4),
                text(description)
                    .size(12)
                    .font(INTER_REGULAR)
                    .style(|_| text::Style {
                        color: Some(theme::COLOR_TEXT_DIM)
                    }),
            ]
            .spacing(4),
        )
        .padding(20)
        .width(Length::Fill)
        .style(|_| iced::widget::container::Style {
            background: Some(theme::COLOR_BG_WIDGET.into()),
            border: Border {
                width: 1.0,
                color: Color::from_rgb(0.18, 0.20, 0.26),
                radius: theme::BORDER_RADIUS_LG.into(),
            },
            shadow: Shadow {
                color: Color::from_rgba(0.0, 0.0, 0.0, 0.25),
                offset: Vector::new(0.0, 4.0),
                blur_radius: 12.0,
            },
            ..Default::default()
        }),
    )
    .on_press(action_msg)
    .style(|_, status| {
        let base = iced::widget::button::Style::default();
        match status {
            iced::widget::button::Status::Hovered => iced::widget::button::Style {
                border: Border {
                    radius: theme::BORDER_RADIUS_LG.into(),
                    width: 1.0,
                    color: theme::COLOR_ACCENT,
                },
                ..base
            },
            _ => base,
        }
    })
    .width(Length::FillPortion(1))
    .into()
}

pub fn welcome_view(app: &crate::app::PdfBullApp) -> Element<'_, crate::message::Message> {
    let recent_section: Element<'_, crate::message::Message> = if app.recent_files.is_empty() {
        container(
            column![
                text("📂 No Recent Documents")
                    .size(14)
                    .font(INTER_BOLD)
                    .style(|_| text::Style {
                        color: Some(theme::COLOR_TEXT_DIM)
                    }),
                Space::new().height(4),
                text("Open a PDF file or drag and drop one into PDFbull to start reading.")
                    .size(12)
                    .font(INTER_REGULAR)
                    .style(|_| text::Style {
                        color: Some(theme::COLOR_TEXT_SECONDARY)
                    }),
            ]
            .align_x(Alignment::Center),
        )
        .padding(32)
        .width(Length::Fill)
        .style(|_| container::Style {
            background: Some(theme::COLOR_BG_WIDGET.into()),
            border: Border {
                radius: theme::BORDER_RADIUS_LG.into(),
                width: 1.0,
                color: Color::from_rgb(0.16, 0.18, 0.24),
            },
            ..Default::default()
        })
        .into()
    } else {
        let files = iced::widget::column(app.recent_files.iter().map(|file| {
            let file_row = row![
                container(text(icons::OPEN).size(14).font(LUCIDE))
                    .padding(10)
                    .style(|_| iced::widget::container::Style {
                        background: Some(theme::COLOR_BG_HEADER.into()),
                        border: Border {
                            radius: theme::BORDER_RADIUS_MD.into(),
                            ..Default::default()
                        },
                        ..Default::default()
                    }),
                column![
                    text(&file.name)
                        .size(14)
                        .font(INTER_BOLD)
                        .style(|_| text::Style {
                            color: Some(theme::COLOR_TEXT_PRIMARY),
                        }),
                    text(file.path.clone())
                        .size(11)
                        .font(INTER_REGULAR)
                        .style(|_| text::Style {
                            color: Some(theme::COLOR_TEXT_SECONDARY)
                        }),
                ]
                .spacing(2),
                Space::new().width(Length::Fill),
                container(
                    text(storage::time_ago(file.last_opened))
                        .size(11)
                        .font(INTER_REGULAR)
                        .style(|_| text::Style {
                            color: Some(theme::COLOR_TEXT_DIM)
                        }),
                )
                .padding([4, 10])
                .style(|_| container::Style {
                    background: Some(theme::COLOR_BG_HEADER.into()),
                    border: Border {
                        radius: theme::BORDER_RADIUS_FULL.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }),
            ]
            .spacing(16)
            .align_y(Alignment::Center);

            button(file_row)
                .on_press(crate::message::Message::OpenRecentFile(file.clone()))
                .width(Length::Fill)
                .padding(12)
                .style(theme::button_ghost)
                .into()
        }))
        .spacing(6);

        container(column![
            row![
                text("Recent Documents")
                    .size(15)
                    .font(INTER_BOLD)
                    .style(|_| text::Style {
                        color: Some(theme::COLOR_TEXT_PRIMARY)
                    }),
                Space::new().width(Length::Fill),
                button(text("Clear History").size(12).font(INTER_REGULAR))
                    .on_press(crate::message::Message::ClearRecentFiles)
                    .style(theme::button_ghost)
                    .padding([4, 10]),
            ]
            .align_y(Alignment::Center),
            Space::new().height(16),
            files,
        ])
        .padding(24)
        .width(Length::Fill)
        .style(|_| container::Style {
            background: Some(theme::COLOR_BG_WIDGET.into()),
            border: Border {
                radius: theme::BORDER_RADIUS_LG.into(),
                width: 1.0,
                color: Color::from_rgb(0.18, 0.20, 0.26),
            },
            ..Default::default()
        })
        .into()
    };

    let hero_header = column![
        row![
            container(text("🐂").size(36))
                .padding(16)
                .style(|_| container::Style {
                    background: Some(theme::COLOR_BG_WIDGET.into()),
                    border: Border {
                        radius: theme::BORDER_RADIUS_LG.into(),
                        width: 1.0,
                        color: theme::COLOR_ACCENT,
                    },
                    shadow: Shadow {
                        color: Color::from_rgba8(59, 130, 246, 0.3),
                        offset: Vector::new(0.0, 4.0),
                        blur_radius: 16.0,
                    },
                    ..Default::default()
                }),
            column![
                row![
                    text("PDFbull")
                        .size(36)
                        .font(INTER_BOLD)
                        .style(|_| text::Style {
                            color: Some(theme::COLOR_TEXT_PRIMARY)
                        }),
                    container(text("v0.9.0").size(11).font(INTER_BOLD).style(|_| {
                        text::Style {
                            color: Some(Color::WHITE),
                        }
                    }))
                    .padding([2, 8])
                    .style(|_| container::Style {
                        background: Some(theme::COLOR_ACCENT.into()),
                        border: Border {
                            radius: theme::BORDER_RADIUS_FULL.into(),
                            ..Default::default()
                        },
                        ..Default::default()
                    }),
                ]
                .spacing(10)
                .align_y(Alignment::Center),
                text("HIGH PERFORMANCE PDF READER & EDITOR • PURE RUST ENGINE")
                    .size(11)
                    .font(INTER_BOLD)
                    .style(|_| text::Style {
                        color: Some(theme::COLOR_TEXT_DIM)
                    }),
            ]
            .spacing(4),
        ]
        .spacing(20)
        .align_y(Alignment::Center),
    ]
    .align_x(Alignment::Center);

    let quick_cards_row1 = row![
        feature_card(
            "📂",
            "Open Document",
            "Browse and open local PDF files instantly",
            crate::message::Message::OpenDocument
        ),
        feature_card(
            "🔀",
            "Merge PDFs",
            "Combine multiple PDF documents into a single file",
            crate::message::Message::MergeDocuments(vec![])
        ),
    ]
    .spacing(16)
    .width(Length::Fill);

    let quick_cards_row2 = row![
        feature_card(
            "📑",
            "Page Organizer",
            "Rotate, reorder, or delete pages visually",
            crate::message::Message::TogglePageOrganizer(true)
        ),
        feature_card(
            "✍️",
            "Digital Signatures",
            "Draw, save, and stamp custom signatures",
            crate::message::Message::ToggleSignatureCreator(true)
        ),
    ]
    .spacing(16)
    .width(Length::Fill);

    let dropzone_banner = container(
        row![
            text("📥").size(20),
            text("Tip: You can drag and drop any PDF file directly into this window to open it.")
                .size(12)
                .font(INTER_REGULAR)
                .style(|_| text::Style {
                    color: Some(theme::COLOR_TEXT_DIM)
                }),
        ]
        .spacing(12)
        .align_y(Alignment::Center),
    )
    .padding(16)
    .width(Length::Fill)
    .style(|_| container::Style {
        background: Some(theme::COLOR_BG_HEADER.into()),
        border: Border {
            radius: theme::BORDER_RADIUS_MD.into(),
            width: 1.0,
            color: Color::from_rgb(0.18, 0.20, 0.25),
        },
        ..Default::default()
    });

    let content = column![
        hero_header,
        Space::new().height(32),
        quick_cards_row1,
        Space::new().height(16),
        quick_cards_row2,
        Space::new().height(24),
        dropzone_banner,
        Space::new().height(24),
        recent_section,
    ]
    .width(Length::Fixed(840.0))
    .align_x(Alignment::Center);

    container(scrollable(
        container(content)
            .width(Length::Fill)
            .padding([48, 24])
            .center_x(Length::Fill),
    ))
    .width(Length::Fill)
    .height(Length::Fill)
    .style(|_| container::Style {
        background: Some(theme::COLOR_BG_APP.into()),
        ..Default::default()
    })
    .into()
}
