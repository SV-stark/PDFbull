use crate::app::{INTER_BOLD, INTER_REGULAR};
use iced::widget::{button, column, container, scrollable, text, Space};
use iced::{Alignment, Color, Element, Length, Shadow, Vector};

pub fn keyboard_help_view(_app: &crate::app::PdfBullApp) -> Element<'_, crate::message::Message> {
    let shortcuts = column![
        text("Keyboard Shortcuts")
            .size(28)
            .font(INTER_BOLD)
            .style(|_| iced::widget::text::Style {
                color: Some(Color::WHITE)
            }),
        Space::new().height(Length::Fixed(24.0)),
        shortcut_section(
            "Navigation",
            vec![
                ("Arrow Up/Down", "Scroll"),
                ("Page Up/Down", "Next/Prev Page"),
                ("Home/End", "First/Last Page"),
                ("Space", "Scroll Down"),
            ]
        ),
        shortcut_section(
            "View",
            vec![
                ("Ctrl + 0", "Reset Zoom"),
                ("Ctrl + +", "Zoom In"),
                ("Ctrl + -", "Zoom Out"),
                ("F11", "Toggle Fullscreen"),
            ]
        ),
        shortcut_section(
            "Document",
            vec![
                ("Ctrl + O", "Open File"),
                ("Ctrl + S", "Save/Export"),
                ("Ctrl + D", "Add Bookmark"),
                ("Ctrl + F", "Search"),
                ("Ctrl + B", "Toggle Sidebar"),
            ]
        ),
        Space::new().height(Length::Fixed(20.0)),
        text("Press ? or F1 to close this help")
            .size(13)
            .font(INTER_REGULAR)
            .style(|_| iced::widget::text::Style {
                color: Some(Color::from_rgb8(150, 150, 160))
            }),
    ]
    .padding(40)
    .width(Length::Fixed(500.0))
    .align_x(Alignment::Start);

    container(
        container(column![
            row![
                Space::new().width(Length::Fill),
                button(text("Close").font(INTER_BOLD).size(14))
                    .on_press(crate::message::Message::ToggleKeyboardHelp)
                    .style(iced::widget::button::text)
                    .padding(10),
            ],
            scrollable(shortcuts),
        ])
        .style(|_| iced::widget::container::Style {
            background: Some(Color::from_rgb8(43, 45, 49).into()),
            border: iced::Border {
                radius: 12.0.into(),
                width: 1.0,
                color: Color::from_rgb8(60, 60, 65),
                ..Default::default()
            },
            shadow: Shadow {
                color: Color::from_rgba(0.0, 0.0, 0.0, 0.4),
                offset: Vector::new(0.0, 10.0),
                blur_radius: 30.0,
            },
            ..Default::default()
        }),
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

fn shortcut_section<'a>(
    title: &'a str,
    items: Vec<(&'a str, &'a str)>,
) -> Element<'a, crate::message::Message> {
    let mut col = column![
        text(title)
            .size(16)
            .font(INTER_BOLD)
            .style(|_| iced::widget::text::Style {
                color: Some(Color::from_rgb8(150, 220, 220))
            }),
        Space::new().height(Length::Fixed(10.0)),
    ]
    .spacing(8);

    for (keys, action) in items {
        col = col.push(
            row![
                container(text(keys).font(INTER_BOLD).size(12))
                    .padding([4, 8])
                    .style(|_| iced::widget::container::Style {
                        background: Some(Color::from_rgb8(60, 60, 65).into()),
                        border: iced::Border {
                            radius: 4.0.into(),
                            ..Default::default()
                        },
                        ..Default::default()
                    }),
                text(action)
                    .size(13)
                    .font(INTER_REGULAR)
                    .style(|_| iced::widget::text::Style {
                        color: Some(Color::from_rgb8(200, 200, 210))
                    }),
            ]
            .spacing(15)
            .align_y(Alignment::Center),
        );
    }

    col.push(Space::new().height(Length::Fixed(20.0))).into()
}

use iced::widget::row;
