use iced::widget::{button, column, row, text, Space};
use iced::{Alignment, Element, Length, Color};
use iced_aw::widget::Card;

pub fn welcome_view(app: &crate::app::PdfBullApp) -> Element<crate::message::Message> {
    let recent_section = if !app.recent_files.is_empty() {
        let mut files = column![];
        for file in &app.recent_files {
            let file_row = row![
                text("📄").size(20),
                text(&file.name).size(14),
            ]
            .spacing(10)
            .align_y(iced::Alignment::Center);
            
            files = files.push(
                button(file_row)
                    .on_press(crate::message::Message::OpenRecentFile(file.clone()))
                    .width(Length::Fill)
                    .padding(10),
            );
        }
        Card::new(
            text("Recent Files").size(16),
            column![
                Space::new().height(Length::Fixed(10.0)),
                files,
            ]
        )
        .padding(15)
        .style(iced_aw::widget::card::Style::Secondary)
    } else {
        column![]
    };

    let open_card = Card::new(
        text("Welcome to PDFbull").size(24),
        column![
            Space::new().height(Length::Fixed(20.0)),
            button("Open PDF")
                .on_press(crate::message::Message::OpenDocument)
                .padding(15),
            Space::new().height(Length::Fixed(20.0)),
        ]
    )
    .padding(30)
    .style(iced_aw::widget::card::Style::Default);

    column![
        row![
            text("📚 PDFbull").size(28),
            Space::new().width(Length::Fill),
            button("⚙ Settings").on_press(crate::message::Message::OpenSettings),
        ]
        .padding(20),
        column![
            open_card,
            Space::new().height(Length::Fixed(20.0)),
            recent_section,
        ]
        .align_x(Alignment::Center)
        .width(Length::Fill)
        .height(Length::Fill),
    ]
    .into()
}
