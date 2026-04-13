use crate::app::{INTER_BOLD, INTER_REGULAR, PdfBullApp};
use crate::message::Message;
use iced::widget::{Space, button, column, container, row, scrollable, text};
use iced::{Alignment, Color, Element, Length};

pub fn metadata_view(app: &PdfBullApp) -> Element<'_, Message> {
    let Some(tab) = app.current_tab() else {
        return container(text("No document open")).into();
    };

    let meta = &tab.metadata;

    let header_row = row![
        text("Document Information").size(24).font(INTER_BOLD),
        Space::new().width(Length::Fill),
        button(text("Close").size(14).font(INTER_REGULAR))
            .on_press(Message::ToggleMetadata)
            .padding([6, 12])
            .style(iced::widget::button::text),
    ]
    .align_y(Alignment::Center);

    let fields: Vec<(String, String)> = vec![
        (
            "Title".to_string(),
            meta.title.as_deref().unwrap_or("N/A").to_string(),
        ),
        (
            "Author".to_string(),
            meta.author.as_deref().unwrap_or("N/A").to_string(),
        ),
        (
            "Subject".to_string(),
            meta.subject.as_deref().unwrap_or("N/A").to_string(),
        ),
        (
            "Keywords".to_string(),
            meta.keywords.as_deref().unwrap_or("N/A").to_string(),
        ),
        (
            "Creator".to_string(),
            meta.creator.as_deref().unwrap_or("N/A").to_string(),
        ),
        (
            "Producer".to_string(),
            meta.producer.as_deref().unwrap_or("N/A").to_string(),
        ),
        (
            "Creation Date".to_string(),
            meta.creation_date.as_deref().unwrap_or("N/A").to_string(),
        ),
        (
            "Modification Date".to_string(),
            meta.modification_date
                .as_deref()
                .unwrap_or("N/A")
                .to_string(),
        ),
        (
            "File Path".to_string(),
            tab.path.to_string_lossy().into_owned(),
        ),
        ("Page Count".to_string(), tab.total_pages.to_string()),
    ];

    let meta_table = iced::widget::table(
        [
            iced::widget::table::column(
                "Property",
                |row: (String, String)| -> Element<'_, Message> {
                    text(row.0)
                        .size(14)
                        .font(INTER_BOLD)
                        .style(|_| iced::widget::text::Style {
                            color: Some(Color::from_rgb(0.5, 0.5, 0.5)),
                        })
                        .into()
                },
            ),
            iced::widget::table::column("Value", |row: (String, String)| -> Element<'_, Message> {
                text(row.1).size(16).font(INTER_REGULAR).into()
            }),
        ],
        fields,
    );

    container(scrollable(
        container(column![header_row, meta_table].spacing(20))
            .width(Length::Fixed(600.0))
            .padding(30)
            .style(|_| iced::widget::container::Style {
                background: Some(Color::from_rgb8(43, 45, 49).into()),
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
