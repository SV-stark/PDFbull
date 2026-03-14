use crate::app::{icons, INTER_BOLD, INTER_REGULAR, LUCIDE};
use iced::widget::{button, column, container, image, row, scrollable, text, Space};
use iced::{Alignment, Border, Color, Element, Length, Shadow, Vector};
// Removed usage, but kept import if needed elsewhere

fn custom_card<'a>(
    content: impl Into<Element<'a, crate::message::Message>>,
) -> Element<'a, crate::message::Message> {
    container(content)
        .padding(24)
        .style(|_theme| iced::widget::container::Style {
            background: Some(Color::from_rgb8(43, 45, 49).into()),
            border: Border {
                radius: 12.0.into(),
                width: 1.0,
                color: Color::from_rgb8(50, 52, 56),
                ..Default::default()
            },
            shadow: Shadow {
                color: Color::from_rgba(0.0, 0.0, 0.0, 0.3),
                offset: Vector::new(0.0, 4.0),
                blur_radius: 12.0,
            },
            ..Default::default()
        })
        .into()
}

pub fn welcome_view(app: &crate::app::PdfBullApp) -> Element<'_, crate::message::Message> {
    let recent_section: Element<'_, crate::message::Message> = if !app.recent_files.is_empty() {
        let mut files: iced::widget::Column<'_, crate::message::Message> =
            iced::widget::Column::new().spacing(4);
        for file in &app.recent_files {
            let file_row = row![
                text(icons::OPEN)
                    .size(16)
                    .font(LUCIDE)
                    .style(|_theme| iced::widget::text::Style {
                        color: Some(Color::from_rgb8(150, 150, 160))
                    }),
                iced::widget::tooltip(
                    text(&file.name)
                        .size(14)
                        .font(INTER_REGULAR)
                        .style(|_theme| iced::widget::text::Style {
                            color: Some(Color::WHITE)
                        }),
                    text(file.path.clone()).font(INTER_REGULAR),
                    iced::widget::tooltip::Position::Bottom
                ),
            ]
            .spacing(12)
            .align_y(iced::Alignment::Center);

            files = files.push(
                button(file_row)
                    .on_press(crate::message::Message::OpenRecentFile(file.clone()))
                    .width(Length::Fill)
                    .padding(8)
                    .style(|_theme: &iced::Theme, status| {
                        let bg = if status == iced::widget::button::Status::Hovered {
                            Some(Color::from_rgb8(60, 62, 66).into())
                        } else {
                            None
                        };
                        iced::widget::button::Style {
                            background: bg,
                            border: Border {
                                radius: 8.0.into(),
                                ..Default::default()
                            },
                            ..Default::default()
                        }
                    }),
            );
        }

        custom_card(column![
            row![
                text("Recent Files")
                    .size(16)
                    .font(INTER_BOLD)
                    .style(|_theme| iced::widget::text::Style {
                        color: Some(Color::WHITE)
                    }),
                Space::new().width(Length::Fill),
                button(
                    text("Clear All")
                        .size(12)
                        .font(INTER_REGULAR)
                        .style(|_theme| iced::widget::text::Style {
                            color: Some(Color::from_rgb8(180, 180, 180))
                        })
                )
                .on_press(crate::message::Message::ClearRecentFiles)
                .padding(5)
                .style(iced::widget::button::text)
            ]
            .align_y(Alignment::Center),
            Space::new().height(Length::Fixed(12.0)),
            files,
        ])
    } else {
        column![].into()
    };

    let drop_zone = container(
        column![
            text(icons::OPEN)
                .size(48)
                .font(LUCIDE)
                .style(|_theme| iced::widget::text::Style {
                    color: Some(Color::from_rgb8(100, 100, 110))
                }),
            text("Open a PDF")
                .size(24)
                .font(INTER_BOLD)
                .style(|_theme| iced::widget::text::Style {
                    color: Some(Color::WHITE)
                }),
            text("Drag & drop a file or click Open")
                .size(15)
                .font(INTER_REGULAR)
                .style(|_theme| iced::widget::text::Style {
                    color: Some(Color::from_rgb8(180, 180, 180))
                }),
            Space::new().height(Length::Fixed(20.0)),
            button(
                text("Open Document")
                    .size(14)
                    .font(INTER_BOLD)
                    .style(|_theme| iced::widget::text::Style {
                        color: Some(Color::WHITE)
                    })
            )
            .on_press(crate::message::Message::OpenDocument)
            .padding([12, 24])
            .style(|_theme, status| {
                let bg = if status == iced::widget::button::Status::Hovered {
                    Color::from_rgb8(90, 90, 95)
                } else {
                    Color::from_rgb8(70, 70, 75)
                };
                iced::widget::button::Style {
                    background: Some(bg.into()),
                    border: Border {
                        radius: 10.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }
            }),
        ]
        .align_x(Alignment::Center)
        .spacing(12),
    )
    .width(Length::Fill)
    .padding(48)
    .style(|_| iced::widget::container::Style {
        border: Border {
            color: Color::from_rgb8(60, 60, 65),
            width: 2.0,
            radius: 12.0.into(),
        },
        background: Some(iced::Background::Color(Color::from_rgb8(30, 31, 34))),
        ..Default::default()
    });

    let logo = image(iced::widget::image::Handle::from_bytes(
        include_bytes!("../PDFbull.png").to_vec(),
    ))
    .width(Length::Fixed(100.0));

    let open_card = custom_card(column![
        row![
            logo,
            Space::new().width(Length::Fixed(24.0)),
            column![
                text("Welcome to PDFbull")
                    .size(28)
                    .font(INTER_BOLD)
                    .style(|_theme| iced::widget::text::Style {
                        color: Some(Color::WHITE)
                    }),
                text("High-performance PDF reading")
                    .size(14)
                    .font(INTER_REGULAR)
                    .style(|_theme| iced::widget::text::Style {
                        color: Some(Color::from_rgb8(150, 150, 150))
                    }),
            ]
        ]
        .align_y(Alignment::Center),
        Space::new().height(Length::Fixed(24.0)),
        drop_zone,
    ]);

    container(
        column![
            row![
                image(iced::widget::image::Handle::from_bytes(
                    include_bytes!("../PDFbull.png").to_vec(),
                ))
                .width(Length::Fixed(32.0)),
                text("PDFbull").size(28).font(INTER_BOLD).style(|_theme| {
                    iced::widget::text::Style {
                        color: Some(Color::WHITE),
                    }
                }),
                text(format!("v{}", env!("CARGO_PKG_VERSION")))
                    .size(12)
                    .font(INTER_REGULAR)
                    .style(|_theme| iced::widget::text::Style {
                        color: Some(Color::from_rgb8(180, 180, 180))
                    }),
                Space::new().width(Length::Fill),
                button(
                    row![
                        text(icons::SETTINGS).font(LUCIDE).size(16),
                        text("Settings").font(INTER_REGULAR).size(14),
                    ]
                    .spacing(8)
                    .align_y(Alignment::Center)
                )
                .on_press(crate::message::Message::OpenSettings)
                .style(iced::widget::button::text),
            ]
            .align_y(Alignment::Center)
            .spacing(12)
            .padding(24),
            scrollable(
                column![
                    open_card,
                    Space::new().height(Length::Fixed(24.0)),
                    recent_section,
                    Space::new().height(Length::Fixed(40.0)),
                ]
                .width(Length::Fixed(640.0))
                .align_x(Alignment::Center)
            )
            .height(Length::Fill),
        ]
        .align_x(Alignment::Center)
        .width(Length::Fill)
        .height(Length::Fill),
    )
    .style(|_theme| iced::widget::container::Style {
        background: Some(Color::from_rgb8(35, 36, 40).into()),
        ..Default::default()
    })
    .into()
}
