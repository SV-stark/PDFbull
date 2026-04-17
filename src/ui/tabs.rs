use crate::app::PdfBullApp;
use crate::app::{LUCIDE, icons};
use crate::ui::theme;
use iced::widget::{button, container, row, text};
use iced::{Alignment, Color, Element, Length, Padding};
use iced_draggable_tabs::DraggableTabs;

pub fn render<'a>(
    app: &'a PdfBullApp,
    // `&'static str` refs from `app.tab_display_names` (interned via Box::leak).
    // Using `'static` here means the slice itself can live as long as needed,
    // satisfying DraggableTabs<'a, _> which needs `&'a [&'a str]`.
    tab_names: &'a [&'static str],
) -> Element<'a, crate::message::Message> {
    let tabs = DraggableTabs::new(
        tab_names,
        app.active_tab,
        crate::message::Message::SwitchTab,
        crate::message::Message::TabReordered,
    )
    .on_close(crate::message::Message::CloseTab)
    .tab_height(36.0)
    .spacing(2.0)
    .tab_padding(Padding::from([4, 12]));

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

    let tab_bar_bg = container(row![tabs, add_button].align_y(Alignment::Center))
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
