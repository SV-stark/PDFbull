use iced::widget::{button, column, row, text, Space};
use iced::{Alignment, Element, Length};

pub fn welcome_view(app: &crate::PdfBullApp) -> Element<crate::Message> {
    let recent_section = if !app.recent_files.is_empty() {
        let mut files = column![];
        for file in &app.recent_files {
            files = files.push(
                button(text(file.name.clone()))
                    .on_press(crate::Message::OpenRecentFile(file.clone()))
                    .width(Length::Fill),
            );
        }
        column![
            text("Recent Files").size(20),
            Space::new().height(Length::Fixed(10.0)),
            files,
            Space::new().height(Length::Fixed(10.0)),
            button("Clear Recent").on_press(crate::Message::ClearRecentFiles),
        ]
        .padding(20)
    } else {
        column![]
    };

    column![
        row![
            text("PDFbull").size(32).width(Length::Fill),
            button("Settings").on_press(crate::Message::OpenSettings),
        ]
        .padding(20),
        column![
            text("Welcome to PDFbull").size(24),
            Space::new().height(Length::Fixed(20.0)),
            button("Open PDF")
                .on_press(crate::Message::OpenDocument)
                .padding(10),
            Space::new().height(Length::Fixed(20.0)),
            recent_section,
        ]
        .align_x(Alignment::Center)
        .width(Length::Fill)
        .height(Length::Fill),
    ]
    .into()
}
