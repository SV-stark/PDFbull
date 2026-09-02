use crate::app::PdfBullApp;
use crate::app::{INTER_BOLD, INTER_REGULAR, LUCIDE, icons};
use crate::ui::theme;
use iced::widget::{button, container, row, scrollable, text, tooltip};
use iced::{Alignment, Border, Color, Element, Length};

pub fn render<'a>(app: &'a PdfBullApp) -> Element<'a, crate::message::Message> {
    let mut tab_row = row![].spacing(4).align_y(Alignment::Center);

    for (idx, tab) in app.tabs.iter().enumerate() {
        let is_active = idx == app.active_tab;

        let select_tab_button = button(
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
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        )
        .on_press(crate::message::Message::SwitchTab(idx))
        .padding([4, 6])
        .style(theme::button_ghost);

        let close_tab_button = button(text(icons::CLOSE).size(10).font(LUCIDE))
            .on_press(crate::message::Message::CloseTab(idx))
            .style(theme::button_ghost)
            .padding(2);

        let tab_item = container(
            row![select_tab_button, close_tab_button]
                .spacing(4)
                .align_y(Alignment::Center),
        )
        .padding([2, 6])
        .style(move |_theme| {
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

            iced::widget::container::Style {
                background: base_bg,
                border: Border {
                    radius: theme::BORDER_RADIUS_MD.into(),
                    width: if is_active { 1.0 } else { 0.0 },
                    color: border_color,
                },
                ..Default::default()
            }
        });

        tab_row = tab_row.push(tab_item);
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
