use crate::app::PdfBullApp;
use crate::app::{LUCIDE, icons};
use iced::widget::{button, container, row, text};
use iced::{Alignment, Color, Element, Length, Padding};
use iced_draggable_tabs::DraggableTabs;

thread_local! {
    static TAB_NAMES: std::cell::RefCell<Vec<String>> = const { std::cell::RefCell::new(Vec::new()) };
    static TAB_REFS: std::cell::RefCell<Vec<&'static str>> = const { std::cell::RefCell::new(Vec::new()) };
}

pub fn render(app: &PdfBullApp) -> Element<'_, crate::message::Message> {
    let name_refs_slice: &'static [&'static str] = TAB_NAMES.with(|names_cell| {
        TAB_REFS.with(|refs_cell| {
            let mut names = names_cell.borrow_mut();
            let mut refs = refs_cell.borrow_mut();
            *names = app.tabs.iter().map(|t| t.name.clone()).collect();

            *refs = names
                .iter()
                .map(|s| unsafe { std::mem::transmute::<&str, &'static str>(s.as_str()) })
                .collect();

            unsafe {
                std::mem::transmute::<&[&'static str], &'static [&'static str]>(refs.as_slice())
            }
        })
    });

    let tabs = DraggableTabs::new(
        name_refs_slice,
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
