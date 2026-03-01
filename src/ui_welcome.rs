use iced::widget::{button, column, container, row, text, Space};
use iced::{Alignment, Border, Color, Element, Length};
use iced_aw::widget::Card;

pub fn welcome_view(app: &crate::app::PdfBullApp) -> Element<crate::message::Message> {
    let recent_section = if !app.recent_files.is_empty() {
        let mut files = column![];
        for file in &app.recent_files {
            let file_row = row![
                text("📄").size(20),
                iced::widget::tooltip(
                    text(&file.name).size(14),
                    file.path.clone(),
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
                    .style(iced::theme::Button::Text),
            );
        }

        Card::new(
            row![
                text("Recent Files").size(16),
                Space::new().width(Length::Fill),
                button(text("Clear All").size(12))
                    .on_press(crate::message::Message::ClearRecentFiles)
                    .padding(5)
            ]
            .align_y(Alignment::Center),
            column![Space::new().height(Length::Fixed(10.0)), files,],
        )
        .padding(15)
        .style(iced_aw::widget::card::Style::Secondary)
    } else {
        column![]
    };

    let drop_zone = container(
        column![
            text("📂").size(48),
            text("Open a PDF").size(22),
            text("Drag & drop a file or click Open").size(14),
            Space::new().height(Length::Fixed(15.0)),
            button("Open PDF")
                .on_press(crate::message::Message::OpenDocument)
                .padding(12),
        ]
        .align_x(Alignment::Center)
        .spacing(8),
    )
    .width(Length::Fill)
    .padding(40)
    .style(|_| iced::widget::container::Style {
        border: Border {
            color: Color::from_rgb(0.5, 0.5, 0.5),
            width: 2.0,
            radius: 8.0.into(),
        },
        background: Some(iced::Background::Color(Color::from_rgba(
            0.5, 0.5, 0.5, 0.05,
        ))),
        ..Default::default()
    });

    let open_card = Card::new(
        text("Welcome to PDFbull").size(24),
        column![
            Space::new().height(Length::Fixed(20.0)),
            drop_zone,
            Space::new().height(Length::Fixed(20.0)),
        ],
    )
    .padding(30)
    .style(iced_aw::widget::card::Style::Default);

    column![
        row![
            text("📚 PDFbull").size(28),
            text(format!("v{}", env!("CARGO_PKG_VERSION"))).size(12),
            Space::new().width(Length::Fill),
            button("⚙ Settings").on_press(crate::message::Message::OpenSettings),
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
    .height(Length::Fill)
    .into()
}
