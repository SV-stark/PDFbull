use crate::app::PdfBullApp;
use crate::app::{INTER_BOLD, INTER_REGULAR, LUCIDE, icons};
use crate::ui::theme;
use iced::widget::{Space, button, container, row, scrollable, text, tooltip};
use iced::{Alignment, Border, Color, Element, Length};

pub fn render<'a>(app: &'a PdfBullApp) -> Element<'a, crate::message::Message> {
    let mut tab_row = row![].spacing(4).align_y(Alignment::Center);

    for (idx, tab) in app.tabs.iter().enumerate() {
        let is_active = idx == app.active_tab;

        let select_tab_button = button(
            row![
                text(icons::FILE_TEXT).size(12).font(LUCIDE),
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

        let interactive_tab = iced::widget::mouse_area(tab_item)
            .on_middle_release(crate::message::Message::CloseTab(idx))
            .on_right_release(crate::message::Message::ShowTabContextMenu(idx));

        tab_row = tab_row.push(interactive_tab);
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

pub fn render_tab_context_menu<'a>(
    app: &'a PdfBullApp,
    tab_idx: usize,
) -> Element<'a, crate::message::Message> {
    let tab_name = app
        .tabs
        .get(tab_idx)
        .map(|t| t.name.as_str())
        .unwrap_or("Tab");
    let has_path = app
        .tabs
        .get(tab_idx)
        .is_some_and(|t| !t.path.as_os_str().is_empty());
    let has_tabs_to_right = tab_idx + 1 < app.tabs.len();
    let has_other_tabs = app.tabs.len() > 1;

    let h_sep = || -> Element<'static, crate::message::Message> {
        container(Space::new().height(1.0))
            .width(Length::Fill)
            .height(1.0)
            .style(|_| iced::widget::container::Style {
                background: Some(Color::from_rgb(0.18, 0.20, 0.25).into()),
                ..Default::default()
            })
            .into()
    };

    let menu_item = |label: &'static str,
                     icon: &'static str,
                     msg: Option<crate::message::Message>|
     -> Element<'a, crate::message::Message> {
        let content = row![
            text(icon).size(13).font(LUCIDE),
            text(label).size(12).font(INTER_REGULAR),
        ]
        .spacing(10)
        .align_y(Alignment::Center);

        if let Some(msg) = msg {
            button(content)
                .width(Length::Fill)
                .padding([6, 12])
                .on_press(msg)
                .style(theme::button_ghost)
                .into()
        } else {
            container(
                row![
                    text(icon).size(13).font(LUCIDE).style(|_| text::Style {
                        color: Some(theme::COLOR_TEXT_DIM)
                    }),
                    text(label)
                        .size(12)
                        .font(INTER_REGULAR)
                        .style(|_| text::Style {
                            color: Some(theme::COLOR_TEXT_DIM)
                        }),
                ]
                .spacing(10)
                .align_y(Alignment::Center),
            )
            .width(Length::Fill)
            .padding([6, 12])
            .into()
        }
    };

    let menu_col = iced::widget::column![
        container(
            text(format!("Tab: {}", tab_name))
                .size(11)
                .font(INTER_BOLD)
                .style(|_| text::Style {
                    color: Some(theme::COLOR_TEXT_DIM)
                })
        )
        .padding([4, 12]),
        h_sep(),
        menu_item(
            "Close Tab",
            icons::CLOSE,
            Some(crate::message::Message::CloseTab(tab_idx))
        ),
        menu_item(
            "Close Others",
            icons::BAN,
            if has_other_tabs {
                Some(crate::message::Message::CloseOtherTabs(tab_idx))
            } else {
                None
            },
        ),
        menu_item(
            "Close Tabs to the Right",
            icons::ARROW_RIGHT,
            if has_tabs_to_right {
                Some(crate::message::Message::CloseTabsToRight(tab_idx))
            } else {
                None
            },
        ),
        h_sep(),
        menu_item(
            "Copy File Path",
            icons::COPY,
            if has_path {
                Some(crate::message::Message::CopyTabPath(tab_idx))
            } else {
                None
            },
        ),
        menu_item(
            "Open Containing Folder",
            icons::FOLDER_OPEN,
            if has_path {
                Some(crate::message::Message::OpenTabFolder(tab_idx))
            } else {
                None
            },
        ),
    ]
    .spacing(2)
    .width(Length::Fixed(200.0));

    let menu_card = container(menu_col)
        .padding(6)
        .style(|_| iced::widget::container::Style {
            background: Some(theme::COLOR_BG_WIDGET.into()),
            border: Border {
                radius: theme::BORDER_RADIUS_MD.into(),
                width: 1.0,
                color: Color::from_rgb(0.18, 0.20, 0.25),
            },
            shadow: iced::Shadow {
                color: Color::from_rgba(0.0, 0.0, 0.0, 0.5),
                offset: iced::Vector::new(0.0, 6.0),
                blur_radius: 16.0,
            },
            ..Default::default()
        });

    let backdrop = iced::widget::mouse_area(
        container(menu_card)
            .padding(iced::Padding {
                top: 40.0,
                left: (20.0 + (tab_idx as f32 * 110.0)).min(600.0),
                ..Default::default()
            })
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(iced::alignment::Horizontal::Left)
            .align_y(iced::alignment::Vertical::Top),
    )
    .on_press(crate::message::Message::DismissTabContextMenu);

    backdrop.into()
}
