use crate::app::PdfBullApp;
use crate::app::{icons, LUCIDE};
use iced::widget::{button, container, row, text, Space};
use iced::{Alignment, Color, Element, Length, Padding};
use iced_draggable_tabs::DraggableTabs;

pub fn render(app: &PdfBullApp) -> Element<'_, crate::message::Message> {
    let names: Vec<String> = app
        .tabs
        .iter()
        .map(|t| {
            if t.name.chars().count() > 20 {
                format!("{}…", t.name.chars().take(18).collect::<String>())
            } else {
                t.name.clone()
            }
        })
        .collect();

    let name_refs: Vec<&str> = names.iter().map(|s| s.as_str()).collect();

    let tabs = DraggableTabs::new(
        &name_refs,
        app.active_tab,
        crate::message::Message::SwitchTab,
        crate::message::Message::TabReordered,
    )
    .on_close(crate::message::Message::CloseTab)
    .tab_height(36.0)
    .spacing(2.0)
    .padding(Padding::from([4, 12]));

    let add_button = button(
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

    let tab_bar_bg = container(row![tabs, add_button].align_y(Alignment::Center))
        .width(Length::Fill)
        .padding(Padding {
            top: 2.0,
            right: 5.0,
            bottom: 0.0,
            left: 5.0,
        })
        .style(|_theme| iced::widget::container::Style {
            background: Some(Color::from_rgb8(25, 26, 28).into()),
            ..Default::default()
        });

    tab_bar_bg.into()
}
