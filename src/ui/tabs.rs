use crate::app::PdfBullApp;
use crate::app::{INTER_BOLD, INTER_REGULAR, LUCIDE, icons};
use crate::ui::theme;
use iced::widget::{button, container, row, scrollable, text, tooltip};
use iced::{Alignment, Border, Color, Element, Length};

pub fn render<'a>(app: &'a PdfBullApp) -> Element<'a, crate::message::Message> {
    let mut tab_row = row![].spacing(4).align_y(Alignment::Center);

    for (idx, tab) in app.tabs.iter().enumerate() {
        let is_active = idx == app.active_tab;

        let tab_button = button(
            row![
                text("📄").size(12),
                text(&tab.name)
                    .size(12)
                    .font(if is_active { INTER_BOLD } else { INTER_REGULAR })
                    .style(move |_| text::Style {
                        color: Some(if is_active {
                            theme::COLOR_TEXT_PRIMARY
                        } else {
                            theme::COLOR_TEXT_DIM
                        })
                    }),
                button(text(icons::CLOSE).size(10).font(LUCIDE))
                    .on_press(crate::message::Message::CloseTab(idx))
                    .style(theme::button_ghost)
                    .padding(2)
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        )
        .on_press(crate::message::Message::SwitchTab(idx))
        .padding([4, 12])
        .style(move |_theme, status| {
            let base_bg = if is_active {
                Some(theme::COLOR_BG_WIDGET.into())
            } else {
                None
            };
            let border_color = if is_active {
                theme::COLOR_ACCENT
            } else {
                Color::TRANSPARENT
            };

            let base = iced::widget::button::Style {
                background: base_bg,
                border: Border {
                    radius: theme::BORDER_RADIUS_MD.into(),
                    width: if is_active { 1.0 } else { 0.0 },
                    color: border_color,
                },
                ..Default::default()
            };

            match status {
                iced::widget::button::Status::Hovered if !is_active => {
                    iced::widget::button::Style {
                        background: Some(theme::COLOR_BG_WIDGET_HOVER.into()),
                        ..base
                    }
                }
                _ => base,
            }
        });

        tab_row = tab_row.push(tab_button);
    }

    let add_button = tooltip(
        button(
            text(icons::PLUS)
                .size(14)
                .font(LUCIDE)
                .style(|_| text::Style {
                    color: Some(theme::COLOR_TEXT_PRIMARY),
                }),
        )
        .padding([5, 9])
        .on_press(crate::message::Message::OpenDocument)
        .style(theme::button_ghost),
        "Open new document (Ctrl+O)",
        tooltip::Position::Bottom,
    );

    let tab_bar_bg = container(
        row![
            scrollable(tab_row).direction(iced::widget::scrollable::Direction::Horizontal(
                iced::widget::scrollable::Scrollbar::default()
            )),
            add_button
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    )
    .width(Length::Fill)
    .padding([2, 10])
    .height(34.0)
    .style(|_| iced::widget::container::Style {
        background: Some(theme::COLOR_BG_APP.into()),
        border: Border {
            width: 1.0,
            color: Color::from_rgb(0.12, 0.14, 0.18),
            ..Default::default()
        },
        ..Default::default()
    });

    tab_bar_bg.into()
}
