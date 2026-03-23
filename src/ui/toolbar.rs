use crate::app::PdfBullApp;
use crate::app::{icons, INTER_BOLD, INTER_REGULAR, LUCIDE};
use crate::models::PendingAnnotationKind;
use crate::pdf_engine::RenderFilter;
use crate::ui::theme;
use iced::widget::{button, column, container, row, text, tooltip, Space};
use iced::{Alignment, Border, Color, Element, Length, Padding, Shadow, Vector};

pub fn render(app: &PdfBullApp) -> Element<'_, crate::message::Message> {
    let Some(tab) = app.current_tab() else {
        return container(row![]).into();
    };

    let open_btn = stacked_tool(
        tooltip(
            button(text(icons::OPEN).size(16).font(LUCIDE))
                .on_press(crate::message::Message::OpenDocument)
                .style(iced::widget::button::text),
            "Open another PDF",
            tooltip::Position::Bottom,
        ),
        "Open",
    );

    let sidebar_btn = stacked_tool(
        tooltip(
            button(text(icons::SIDEBAR).size(16).font(LUCIDE))
                .on_press(crate::message::Message::ToggleSidebar)
                .style(iced::widget::button::text),
            "Toggle Sidebar (Ctrl+B)",
            tooltip::Position::Bottom,
        ),
        "Sidebar",
    );

    let zoom_controls = stacked_tool(
        container(
            row![
                button(text(icons::ZOOM_OUT).size(14).font(LUCIDE))
                    .on_press(crate::message::Message::ZoomOut)
                    .style(|_theme, _status| iced::widget::button::Style {
                        text_color: Color::WHITE,
                        ..Default::default()
                    }),
                text(format!("{}%", (tab.zoom * 100.0) as u32))
                    .size(13)
                    .font(INTER_BOLD)
                    .style(|_theme| iced::widget::text::Style {
                        color: Some(Color::WHITE)
                    }),
                button(text(icons::ZOOM_IN).size(14).font(LUCIDE))
                    .on_press(crate::message::Message::ZoomIn)
                    .style(|_theme, _status| iced::widget::button::Style {
                        text_color: Color::WHITE,
                        ..Default::default()
                    }),
            ]
            .spacing(12)
            .align_y(Alignment::Center),
        )
        .padding([4, 10])
        .style(|_theme| iced::widget::container::Style {
            background: Some(Color::from_rgb8(30, 31, 34).into()),
            border: iced::Border {
                radius: 20.0.into(),
                ..Default::default()
            },
            ..Default::default()
        }),
        "Zoom",
    );

    let rotate_btn = stacked_tool(
        tooltip(
            button(text(icons::ROTATE).size(16).font(LUCIDE))
                .on_press(crate::message::Message::RotateClockwise)
                .style(iced::widget::button::text),
            "Rotate 90° clockwise",
            tooltip::Position::Bottom,
        ),
        "Rotate",
    );

    let crop_filters = stacked_tool(
        column![
            row![
                filter_btn_custom("None", RenderFilter::None, tab.render_filter),
                filter_btn_custom("Gray", RenderFilter::Grayscale, tab.render_filter),
                filter_btn_custom("Inv", RenderFilter::Inverted, tab.render_filter),
            ]
            .spacing(2),
            row![
                filter_btn_custom("Eco", RenderFilter::Eco, tab.render_filter),
                filter_btn_custom("B&W", RenderFilter::BlackWhite, tab.render_filter),
                filter_btn_custom("Light", RenderFilter::Lighten, tab.render_filter),
            ]
            .spacing(2),
        ]
        .spacing(2),
        "Filters",
    );

    let autocrop_btn = stacked_tool(
        button(
            text(if tab.auto_crop { "ON" } else { "OFF" })
                .size(11)
                .font(INTER_BOLD)
                .align_x(iced::alignment::Horizontal::Center),
        )
        .on_press(crate::message::Message::ToggleAutoCrop)
        .style(move |_, _| {
            if tab.auto_crop {
                iced::widget::button::Style {
                    background: Some(theme::COLOR_ACCENT.into()),
                    text_color: Color::WHITE,
                    border: Border {
                        radius: 4.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }
            } else {
                iced::widget::button::Style {
                    background: Some(Color::from_rgb8(60, 60, 65).into()),
                    text_color: Color::WHITE,
                    border: Border {
                        radius: 4.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }
            }
        })
        .padding([4, 8]),
        "Auto-Crop",
    );

    let split_btn = stacked_tool(
        button(text("Split").size(11).font(INTER_BOLD))
            .on_press(crate::message::Message::SplitPDF(vec![tab.current_page]))
            .style(iced::widget::button::text),
        "Split",
    );

    let forms_btn = stacked_tool(
        tooltip(
            button(text(icons::FORMS).size(16).font(LUCIDE))
                .on_press(crate::message::Message::ToggleFormsSidebar)
                .style(iced::widget::button::text),
            "Toggle Form Fields",
            tooltip::Position::Bottom,
        ),
        "Forms",
    );

    let bookmark_btn = stacked_tool(
        tooltip(
            button(text(icons::BOOKMARK).size(16).font(LUCIDE))
                .on_press(crate::message::Message::AddBookmark)
                .style(iced::widget::button::text),
            "Add Bookmark",
            tooltip::Position::Bottom,
        ),
        "Bookmark",
    );

    let highlight_btn = stacked_tool(
        tooltip(
            button(text(icons::HIGHLIGHT).size(16).font(LUCIDE))
                .on_press(crate::message::Message::SetAnnotationMode(Some(
                    PendingAnnotationKind::Highlight,
                )))
                .style(iced::widget::button::text),
            "Highlight Text",
            tooltip::Position::Bottom,
        ),
        "Highlight",
    );

    let rectangle_btn = stacked_tool(
        tooltip(
            button(text(icons::RECTANGLE).size(16).font(LUCIDE))
                .on_press(crate::message::Message::SetAnnotationMode(Some(
                    PendingAnnotationKind::Rectangle,
                )))
                .style(iced::widget::button::text),
            "Draw Rectangle",
            tooltip::Position::Bottom,
        ),
        "Rectangle",
    );

    let redact_btn = stacked_tool(
        tooltip(
            button(text(icons::BLOCK).size(16).font(LUCIDE))
                .on_press(crate::message::Message::SetAnnotationMode(Some(
                    PendingAnnotationKind::Redact,
                )))
                .style(iced::widget::button::text),
            "Redact Sensitive Content",
            tooltip::Position::Bottom,
        ),
        "Redact",
    );

    let save_anns_btn = stacked_tool(
        tooltip(
            button(text(icons::SAVE).size(16).font(LUCIDE))
                .on_press(crate::message::Message::SaveAnnotations)
                .style(iced::widget::button::text),
            "Save Annotations",
            tooltip::Position::Bottom,
        ),
        "Save",
    );

    let right_tools = column![
        row![
            button(text("Info").size(12).font(INTER_BOLD))
                .on_press(crate::message::Message::ToggleMetadata)
                .style(iced::widget::button::text),
            button(text(icons::HELP).size(16).font(LUCIDE))
                .on_press(crate::message::Message::ToggleKeyboardHelp)
                .style(iced::widget::button::text),
            button(text(icons::SETTINGS).size(16).font(LUCIDE))
                .on_press(crate::message::Message::OpenSettings)
                .style(iced::widget::button::text),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
        row![
            button(
                row![
                    text(icons::COPY).size(12).font(LUCIDE),
                    text("Text").size(11).font(INTER_REGULAR),
                ]
                .spacing(4)
                .align_y(Alignment::Center)
            )
            .on_press(crate::message::Message::ExtractTextToClipboard)
            .style(iced::widget::button::text),
            button(
                row![
                    text(icons::COPY).size(12).font(LUCIDE),
                    text("Image").size(11).font(INTER_REGULAR),
                ]
                .spacing(4)
                .align_y(Alignment::Center)
            )
            .on_press(crate::message::Message::CopyImageToClipboard)
            .style(iced::widget::button::text),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
        button(
            row![
                text(icons::EXPORT).size(12).font(LUCIDE),
                text("Export").size(11).font(INTER_REGULAR),
            ]
            .spacing(4)
            .align_y(Alignment::Center)
        )
        .on_press(crate::message::Message::ExportImage)
        .style(iced::widget::button::text),
        button(
            row![
                text(icons::PRINT).size(12).font(LUCIDE),
                text("Print").size(11).font(INTER_REGULAR),
            ]
            .spacing(4)
            .align_y(Alignment::Center)
        )
        .on_press(crate::message::Message::Print)
        .style(iced::widget::button::text),
    ]
    .spacing(2)
    .align_x(Alignment::Center);

    container(
        row![
            open_btn,
            Space::new(15, 0),
            sidebar_btn,
            Space::new(25, 0),
            zoom_controls,
            Space::new(20, 0),
            rotate_btn,
            Space::new(20, 0),
            crop_filters,
            Space::new(10, 0),
            autocrop_btn,
            Space::new(Length::Fill, 0),
            split_btn,
            Space::new(15, 0),
            forms_btn,
            Space::new(15, 0),
            bookmark_btn,
            Space::new(15, 0),
            highlight_btn,
            Space::new(15, 0),
            rectangle_btn,
            Space::new(15, 0),
            redact_btn,
            Space::new(20, 0),
            save_anns_btn,
            Space::new(20, 0),
            right_tools,
        ]
        .spacing(5)
        .align_y(Alignment::Center),
    )
    .width(Length::Fill)
    .padding(Padding {
        top: 8.0,
        right: 15.0,
        bottom: 8.0,
        left: 15.0,
    })
    .style(|_theme| iced::widget::container::Style {
        background: Some(Color::from_rgb8(43, 45, 49).into()),
        shadow: Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.2),
            offset: Vector::new(0.0, 2.0),
            blur_radius: 8.0,
        },
        ..Default::default()
    })
    .into()
}

fn stacked_tool<'a>(
    top: impl Into<Element<'a, crate::message::Message>>,
    label: &'a str,
) -> Element<'a, crate::message::Message> {
    column![
        top.into(),
        text(label)
            .font(INTER_REGULAR)
            .style(|_theme| iced::widget::text::Style {
                color: Some(theme::COLOR_TEXT_DIM),
            })
    ]
    .spacing(4)
    .align_x(Alignment::Center)
    .into()
}

fn filter_btn_custom(
    label: &'static str,
    f: RenderFilter,
    active: RenderFilter,
) -> iced::widget::Button<'static, crate::message::Message> {
    let btn = button(
        text(label)
            .size(11)
            .font(INTER_BOLD)
            .align_x(iced::alignment::Horizontal::Center),
    );
    if f == active {
        btn.style(|_theme, _| iced::widget::button::Style {
            background: Some(Color::from_rgb8(150, 220, 220).into()),
            text_color: Color::BLACK,
            border: iced::Border {
                radius: 4.0.into(),
                ..Default::default()
            },
            ..Default::default()
        })
    } else {
        btn.style(|_theme, _status| iced::widget::button::Style {
            background: Some(Color::from_rgb8(60, 60, 65).into()),
            text_color: Color::WHITE,
            border: iced::Border {
                radius: 4.0.into(),
                ..Default::default()
            },
            ..Default::default()
        })
    }
    .on_press(crate::message::Message::SetFilter(f))
}
