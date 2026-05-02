use crate::app::PdfBullApp;
use crate::app::{LUCIDE, icons};
use crate::ui::theme;
use iced::widget::{button, container, row, text};
use iced::{Alignment, Color, Element, Length, Padding};
use iced_draggable_tabs::DraggableTabs;

pub fn render<'a>(
    app: &'a PdfBullApp,
) -> Element<'a, crate::message::Message> {
    let mut tab_row = row![].spacing(2).align_y(Alignment::Center);

    for (idx, tab) in app.tabs.iter().enumerate() {
        let is_active = idx == app.active_tab;
        
        let tab_button = button(
            row![
                text(&tab.name)
                    .size(13)
                    .font(crate::app::INTER_REGULAR),
                button(
                    text(icons::CLOSE)
                        .size(10)
                        .font(LUCIDE)
                )
                .on_press(crate::message::Message::CloseTab(idx))
                .style(theme::button_ghost)
                .padding(2)
            ]
            .spacing(8)
            .align_y(Alignment::Center)
        )
        .on_press(crate::message::Message::SwitchTab(idx))
        .padding([4, 12])
        .style(move |theme, status| {
            let mut style = theme::button_ghost(theme, status);
            if is_active {
                style.background = Some(theme::COLOR_BG_WIDGET.into());
                style.text_color = theme::COLOR_TEXT_PRIMARY;
            }
            style
        });

        tab_row = tab_row.push(tab_button);
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

    let tab_bar_bg = container(row![scrollable(tab_row).direction(iced::widget::scrollable::Direction::Horizontal(iced::widget::scrollable::Scrollbar::default())), add_button].align_y(Alignment::Center))
        .width(Length::Fill)
        .padding([0, 10])
        .height(36.0)
        .style(|_theme| iced::widget::container::Style {
            background: Some(theme::COLOR_BG_SIDEBAR.into()),
            border: iced::Border {
                width: 1.0,
                color: Color::from_rgb(0.05, 0.05, 0.05),
                ..Default::default()
            },
            ..Default::default()
        });

    tab_bar_bg.into()
}
