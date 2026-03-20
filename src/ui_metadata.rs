use crate::app::{PdfBullApp, INTER_BOLD, INTER_REGULAR};
use crate::message::Message;
use iced::widget::{button, column, container, row, scrollable, text, Space};
use iced::{Alignment, Color, Element, Length, Padding};

pub fn metadata_view(app: &PdfBullApp) -> Element<'_, Message> {
    let tab = match app.current_tab() {
        Some(t) => t,
        None => return container(text("No document open")).into(),
    };

    let meta = &tab.metadata;

    let mut info_col = column![
        row![
            text("Document Information")
                .size(24)
                .font(INTER_BOLD),
            Space::with_width(Length::Fill),
            button(text("Close").size(14).font(INTER_REGULAR))
                .on_press(Message::ToggleMetadata)
                .padding([6, 12])
                .style(iced::widget::button::text),
        ]
        .align_y(Alignment::Center),
        Space::with_height(20),
    ]
    .spacing(10);

    let mut add_field = |label: &str, value: &Option<String>| {
        info_col = info_col.push(
            column![
                text(label).size(14).font(INTER_BOLD).style(|_| iced::widget::text::Style {
                    color: Some(Color::from_rgb(0.5, 0.5, 0.5)),
                }),
                text(value.as_deref().unwrap_or("N/A"))
                    .size(16)
                    .font(INTER_REGULAR),
                Space::with_height(10),
            ]
            .spacing(4)
        );
    };

    add_field("Title", &meta.title);
    add_field("Author", &meta.author);
    add_field("Subject", &meta.subject);
    add_field("Keywords", &meta.keywords);
    add_field("Creator", &meta.creator);
    add_field("Producer", &meta.producer);
    add_field("Creation Date", &meta.creation_date);
    add_field("Modification Date", &meta.modification_date);

    add_field("File Path", &Some(tab.path.to_string_lossy().to_string()));
    add_field("Page Count", &Some(tab.total_pages.to_string()));

    container(
        scrollable(
            container(info_col)
                .width(Length::Fixed(500.0))
                .padding(30)
                .style(|_| iced::widget::container::Style {
                    background: Some(Color::from_rgb8(35, 36, 40).into()),
                    border: iced::Border {
                        radius: 12.0.into(),
                        width: 1.0,
                        color: Color::from_rgb8(60, 60, 65),
                    },
                    ..Default::default()
                })
        )
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .center_x(Length::Fill)
    .center_y(Length::Fill)
    .style(|_| iced::widget::container::Style {
        background: Some(Color::from_rgba(0.0, 0.0, 0.0, 0.7).into()),
        ..Default::default()
    })
    .into()
}
