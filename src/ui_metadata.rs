use crate::app::{PdfBullApp, INTER_BOLD, INTER_REGULAR};
use crate::message::Message;
use iced::widget::{button, column, container, row, scrollable, text, Space};
use iced::{Alignment, Color, Element, Length};

pub fn metadata_view(app: &PdfBullApp) -> Element<'_, Message> {
    let tab = match app.current_tab() {
        Some(t) => t,
        None => return container(text("No document open")).into(),
    };

    let meta = &tab.metadata;

    let mut info_col = column![
        row![
            text("Document Information").size(24).font(INTER_BOLD),
            Space::new().width(Length::Fill),
            button(text("Close").size(14).font(INTER_REGULAR))
                .on_press(Message::ToggleMetadata)
                .padding([6, 12])
                .style(iced::widget::button::text),
        ]
        .align_y(Alignment::Center),
        Space::new().height(20),
    ]
    .spacing(10);

    let fields = vec![
        ("Title", meta.title.as_deref().unwrap_or("N/A")),
        ("Author", meta.author.as_deref().unwrap_or("N/A")),
        ("Subject", meta.subject.as_deref().unwrap_or("N/A")),
        ("Keywords", meta.keywords.as_deref().unwrap_or("N/A")),
        ("Creator", meta.creator.as_deref().unwrap_or("N/A")),
        ("Producer", meta.producer.as_deref().unwrap_or("N/A")),
        (
            "Creation Date",
            meta.creation_date.as_deref().unwrap_or("N/A"),
        ),
        (
            "Modification Date",
            meta.modification_date.as_deref().unwrap_or("N/A"),
        ),
    ];

    for (label, value) in fields {
        info_col = info_col.push(
            column![
                text(label)
                    .size(14)
                    .font(INTER_BOLD)
                    .style(|_| iced::widget::text::Style {
                        color: Some(Color::from_rgb(0.5, 0.5, 0.5)),
                    }),
                text(value).size(16).font(INTER_REGULAR),
                Space::new().height(10),
            ]
            .spacing(4),
        );
    }

    // Additional fields
    let path_str = tab.path.to_string_lossy().to_string();
    let page_count_str = tab.total_pages.to_string();

    info_col = info_col.push(
        column![
            text("File Path")
                .size(14)
                .font(INTER_BOLD)
                .style(|_| iced::widget::text::Style {
                    color: Some(Color::from_rgb(0.5, 0.5, 0.5)),
                }),
            text(path_str).size(16).font(INTER_REGULAR),
            Space::new().height(10),
        ]
        .spacing(4),
    );

    info_col = info_col.push(
        column![
            text("Page Count")
                .size(14)
                .font(INTER_BOLD)
                .style(|_| iced::widget::text::Style {
                    color: Some(Color::from_rgb(0.5, 0.5, 0.5)),
                }),
            text(page_count_str).size(16).font(INTER_REGULAR),
            Space::new().height(10),
        ]
        .spacing(4),
    );

    container(scrollable(
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
            }),
    ))
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
