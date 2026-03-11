use iced::widget::{button, column, container, image, row, text, Space};
use iced::{Alignment, Border, Color, Element, Length};
// Removed usage, but kept import if needed elsewhere

fn custom_card<'a>(
    content: impl Into<Element<'a, crate::message::Message>>,
) -> Element<'a, crate::message::Message> {
    container(content)
        .padding(20)
        .style(|_theme| iced::widget::container::Style {
            background: Some(Color::from_rgb8(43, 45, 49).into()),
            border: Border {
                radius: 12.0.into(),
                width: 1.0,
                color: Color::from_rgb8(30, 31, 34),
                ..Default::default()
            },
            ..Default::default()
        })
        .into()
}

pub fn welcome_view(app: &crate::app::PdfBullApp) -> Element<'_, crate::message::Message> {
    let recent_section: Element<'_, crate::message::Message> = if !app.recent_files.is_empty() {
        let mut files: iced::widget::Column<'_, crate::message::Message> =
            iced::widget::Column::new();
        for file in &app.recent_files {
            let file_row = row![
                text("📄")
                    .size(20)
                    .style(|_theme| iced::widget::text::Style {
                        color: Some(Color::from_rgb8(180, 180, 180))
                    }),
                iced::widget::tooltip(
                    text(&file.name)
                        .size(14)
                        .style(|_theme| iced::widget::text::Style {
                            color: Some(Color::WHITE)
                        }),
                    text(file.path.clone()),
                    iced::widget::tooltip::Position::Bottom
                ),
            ]
            .spacing(10)
            .align_y(iced::Alignment::Center);

            files = files.push(
                button(file_row)
                    .on_press(crate::message::Message::OpenRecentFile(file.clone()))
                    .width(Length::Fill)
                    .padding(10)
                    .style(|_theme: &iced::Theme, _status| iced::widget::button::Style::default()),
            );
        }

        custom_card(column![
            row![
                text("Recent Files")
                    .size(16)
                    .style(|_theme| iced::widget::text::Style {
                        color: Some(Color::WHITE)
                    }),
                Space::new().width(Length::Fill),
                button(
                    text("Clear All")
                        .size(12)
                        .style(|_theme| iced::widget::text::Style {
                            color: Some(Color::from_rgb8(180, 180, 180))
                        })
                )
                .on_press(crate::message::Message::ClearRecentFiles)
                .padding(5)
                .style(iced::widget::button::text)
            ]
            .align_y(Alignment::Center),
            Space::new().height(Length::Fixed(10.0)),
            files,
        ])
    } else {
        column![].into()
    };

    let drop_zone = container(
        column![
            text("📂").size(48),
            text("Open a PDF")
                .size(22)
                .style(|_theme| iced::widget::text::Style {
                    color: Some(Color::WHITE)
                }),
            text("Drag & drop a file or click Open")
                .size(14)
                .style(|_theme| iced::widget::text::Style {
                    color: Some(Color::from_rgb8(180, 180, 180))
                }),
            Space::new().height(Length::Fixed(15.0)),
            button(
                text("Open PDF")
                    .size(14)
                    .style(|_theme| iced::widget::text::Style {
                        color: Some(Color::WHITE)
                    })
            )
            .on_press(crate::message::Message::OpenDocument)
            .padding([10, 20])
            .style(|_theme, _status| iced::widget::button::Style {
                background: Some(Color::from_rgb8(80, 80, 85).into()),
                border: Border {
                    radius: 8.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }),
        ]
        .align_x(Alignment::Center)
        .spacing(8),
    )
    .width(Length::Fill)
    .padding(40)
    .style(|_| iced::widget::container::Style {
        border: Border {
            color: Color::from_rgb8(60, 60, 65),
            width: 2.0,
            radius: 8.0.into(),
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
            Space::new().width(Length::Fixed(20.0)),
            text("Welcome to PDFbull")
                .size(24)
                .style(|_theme| iced::widget::text::Style {
                    color: Some(Color::WHITE)
                })
        ]
        .align_y(Alignment::Center),
        Space::new().height(Length::Fixed(20.0)),
        drop_zone,
    ]);

    container(
        column![
            row![
                image(iced::widget::image::Handle::from_bytes(
                    include_bytes!("../PDFbull.png").to_vec(),
                ))
                .width(Length::Fixed(32.0)),
                text("PDFbull")
                    .size(28)
                    .style(|_theme| iced::widget::text::Style {
                        color: Some(Color::WHITE)
                    }),
                text(format!("v{}", env!("CARGO_PKG_VERSION")))
                    .size(12)
                    .style(|_theme| iced::widget::text::Style {
                        color: Some(Color::from_rgb8(180, 180, 180))
                    }),
                Space::new().width(Length::Fill),
                button(
                    text("⚙ Settings")
                        .size(14)
                        .style(|_theme| iced::widget::text::Style {
                            color: Some(Color::WHITE)
                        })
                )
                .on_press(crate::message::Message::OpenSettings)
                .style(iced::widget::button::text),
            ]
            .align_y(Alignment::Center)
            .spacing(10)
            .padding(20),
            column![
                open_card,
                Space::new().height(Length::Fixed(20.0)),
                recent_section,
            ]
            .width(Length::Fixed(600.0))
            .align_x(Alignment::Center),
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
