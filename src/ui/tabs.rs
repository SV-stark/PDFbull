use crate::app::PdfBullApp;
use crate::app::{icons, INTER_BOLD, INTER_REGULAR, LUCIDE};
use iced::widget::{button, container, row, text, Space};
use iced::{Alignment, Color, Element, Length, Padding};

pub fn render(app: &PdfBullApp) -> Element<'_, crate::message::Message> {
    let mut tabs = row![];
    for (i, t) in app.tabs.iter().enumerate() {
        let is_active = i == app.active_tab;

        let display_name = if t.name.chars().count() > 20 {
            format!("{}…", t.name.chars().take(18).collect::<String>())
        } else {
            t.name.clone()
        };

        let tab_bg = if is_active {
            Color::from_rgb8(43, 45, 49)
        } else {
            Color::from_rgb8(30, 31, 34)
        };

        let text_color = if is_active {
            Color::WHITE
        } else {
            Color::from_rgb8(150, 150, 150)
        };

        let tab_content = row![
            text(icons::OPEN).size(14).font(LUCIDE),
            Space::new(6, 0),
            text(display_name)
                .size(13)
                .font(if is_active { INTER_BOLD } else { INTER_REGULAR })
                .style(move |_theme| iced::widget::text::Style {
                    color: Some(text_color)
                }),
            Space::new(10, 0),
            button(
                text(icons::CLOSE)
                    .size(12)
                    .font(LUCIDE)
                    .style(move |_theme| iced::widget::text::Style {
                        color: Some(text_color)
                    })
            )
            .on_press(crate::message::Message::CloseTab(i))
            .style(iced::widget::button::text)
            .padding(2)
        ]
        .align_y(Alignment::Center);

        tabs = tabs.push(
            container(tab_content)
                .padding(Padding {
                    top: 6.0,
                    right: 12.0,
                    bottom: 6.0,
                    left: 12.0,
                })
                .style(move |_theme| iced::widget::container::Style {
                    background: Some(tab_bg.into()),
                    border: iced::Border {
                        radius: iced::border::Radius {
                            top_left: 8.0,
                            top_right: 8.0,
                            bottom_left: 0.0,
                            bottom_right: 0.0,
                        },
                        width: if is_active { 0.0 } else { 1.0 },
                        color: Color::from_rgb8(20, 20, 20),
                    },
                    ..Default::default()
                }),
        );
        tabs = tabs.push(Space::new(2, 0));
    }

    let add_button =
        button(
            text(icons::PLUS)
                .size(16)
                .font(LUCIDE)
                .style(|_theme| iced::widget::text::Style {
                    color: Some(Color::WHITE),
                }),
        )
        .padding([4, 10])
        .on_press(crate::message::Message::OpenDocument)
        .style(iced::widget::button::text);

    let tab_bar_bg = container(row![tabs, add_button].align_y(Alignment::End))
        .width(Length::Fill)
        .padding(Padding {
            top: 6.0,
            right: 5.0,
            bottom: 0.0,
            left: 10.0,
        })
        .style(|_theme| iced::widget::container::Style {
            background: Some(Color::from_rgb8(25, 26, 28).into()),
            ..Default::default()
        });

    tab_bar_bg.into()
}
