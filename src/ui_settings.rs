use crate::app::{INTER_BOLD, INTER_REGULAR};
use crate::models::AppTheme;
use crate::pdf_engine::{RenderFilter, RenderQuality};
use iced::widget::{button, column, container, image, row, scrollable, text, Space};
use iced::{Alignment, Border, Color, Element, Length, Shadow, Vector};

fn custom_card<'a>(
    header: impl Into<Element<'a, crate::message::Message>>,
    body: impl Into<Element<'a, crate::message::Message>>,
) -> Element<'a, crate::message::Message> {
    container(column![
        header.into(),
        Space::new(0, 15),
        body.into()
    ])
    .padding(20)
    .width(Length::Fill)
    .style(|_theme| iced::widget::container::Style {
        background: Some(Color::from_rgb8(43, 45, 49).into()),
        border: Border {
            radius: 12.0.into(),
            width: 1.0,
            color: Color::from_rgb8(50, 52, 56),
        },
        shadow: Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.2),
            offset: Vector::new(0.0, 4.0),
            blur_radius: 12.0,
        },
        text_color: None,
    })
    .into()
}

fn setting_btn<'a>(
    label: &'a str,
    is_active: bool,
    msg: crate::message::Message,
) -> iced::widget::Button<'a, crate::message::Message> {
    let btn = button(
        text(label)
            .size(13)
            .font(if is_active { INTER_BOLD } else { INTER_REGULAR })
            .align_x(iced::alignment::Horizontal::Center),
    )
    .on_press(msg)
    .width(Length::Fill)
    .padding([8, 12]);

    if is_active {
        btn.style(|_theme: &iced::Theme, _| iced::widget::button::Style {
            background: Some(iced::Color::from_rgb8(150, 220, 220).into()),
            text_color: iced::Color::BLACK,
            border: iced::Border {
                radius: 6.0.into(),
                ..Default::default()
            },
            ..Default::default()
        })
    } else {
        btn.style(|_theme, status| {
            let bg = if status == iced::widget::button::Status::Hovered {
                Color::from_rgb8(70, 70, 75)
            } else {
                Color::from_rgb8(60, 60, 65)
            };
            iced::widget::button::Style {
                background: Some(bg.into()),
                text_color: iced::Color::WHITE,
                border: iced::Border {
                    radius: 8.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        })
    }
}

fn action_btn<'a>(
    label: &'a str,
    msg: crate::message::Message,
) -> iced::widget::Button<'a, crate::message::Message> {
    button(
        text(label)
            .size(14)
            .align_x(iced::alignment::Horizontal::Center),
    )
    .on_press(msg)
    .padding([8, 16])
    .style(|_theme, status| {
        let bg = if status == iced::widget::button::Status::Hovered {
            Color::from_rgb8(70, 70, 75)
        } else {
            Color::from_rgb8(60, 60, 65)
        };
        iced::widget::button::Style {
            background: Some(bg.into()),
            text_color: iced::Color::WHITE,
            border: iced::Border {
                radius: 8.0.into(),
                ..Default::default()
            },
            ..Default::default()
        }
    })
}

pub fn settings_view(app: &crate::app::PdfBullApp) -> Element<'_, crate::message::Message> {
    let theme_buttons = row![
        setting_btn("System", app.settings.theme == AppTheme::System, {
            let mut s = app.settings.clone();
            s.theme = AppTheme::System;
            crate::message::Message::SaveSettings(s)
        }),
        setting_btn("Light", app.settings.theme == AppTheme::Light, {
            let mut s = app.settings.clone();
            s.theme = AppTheme::Light;
            crate::message::Message::SaveSettings(s)
        }),
        setting_btn("Dark", app.settings.theme == AppTheme::Dark, {
            let mut s = app.settings.clone();
            s.theme = AppTheme::Dark;
            crate::message::Message::SaveSettings(s)
        }),
    ]
    .spacing(10);

    let behavior_buttons = row![
        setting_btn("Remember Last File", app.settings.remember_last_file, {
            let mut s = app.settings.clone();
            s.remember_last_file = !s.remember_last_file;
            crate::message::Message::SaveSettings(s)
        }),
        setting_btn("Auto-save", app.settings.auto_save, {
            let mut s = app.settings.clone();
            s.auto_save = !s.auto_save;
            crate::message::Message::SaveSettings(s)
        }),
        setting_btn("Restore Session", app.settings.restore_session, {
            let mut s = app.settings.clone();
            s.restore_session = !s.restore_session;
            crate::message::Message::SaveSettings(s)
        }),
    ]
    .spacing(10);

    let quality_buttons = row![
        setting_btn("Low", app.settings.render_quality == RenderQuality::Low, {
            let mut s = app.settings.clone();
            s.render_quality = RenderQuality::Low;
            crate::message::Message::SaveSettings(s)
        }),
        setting_btn(
            "Medium",
            app.settings.render_quality == RenderQuality::Medium,
            {
                let mut s = app.settings.clone();
                s.render_quality = RenderQuality::Medium;
                crate::message::Message::SaveSettings(s)
            }
        ),
        setting_btn(
            "High",
            app.settings.render_quality == RenderQuality::High,
            {
                let mut s = app.settings.clone();
                s.render_quality = RenderQuality::High;
                crate::message::Message::SaveSettings(s)
            }
        ),
    ]
    .spacing(10);

    let filter_buttons = row![
        setting_btn("None", app.settings.default_filter == RenderFilter::None, {
            let mut s = app.settings.clone();
            s.default_filter = RenderFilter::None;
            crate::message::Message::SaveSettings(s)
        }),
        setting_btn(
            "Grayscale",
            app.settings.default_filter == RenderFilter::Grayscale,
            {
                let mut s = app.settings.clone();
                s.default_filter = RenderFilter::Grayscale;
                crate::message::Message::SaveSettings(s)
            }
        ),
        setting_btn(
            "Invert",
            app.settings.default_filter == RenderFilter::Inverted,
            {
                let mut s = app.settings.clone();
                s.default_filter = RenderFilter::Inverted;
                crate::message::Message::SaveSettings(s)
            }
        ),
        setting_btn("Eco", app.settings.default_filter == RenderFilter::Eco, {
            let mut s = app.settings.clone();
            s.default_filter = RenderFilter::Eco;
            crate::message::Message::SaveSettings(s)
        }),
    ]
    .spacing(10);

    let default_zoom_row = row![
        text("Default Zoom:")
            .font(INTER_REGULAR)
            .style(|_theme| iced::widget::text::Style {
                color: Some(Color::WHITE)
            }),
        Space::new(10, 0),
        text(format!("{}%", (app.settings.default_zoom * 100.0) as i32))
            .font(INTER_BOLD)
            .style(|_theme| {
                iced::widget::text::Style {
                    color: Some(Color::from_rgb8(180, 180, 180)),
                }
            }),
        Space::new(Length::Fill, 0),
        action_btn("-", {
            let mut s = app.settings.clone();
            s.default_zoom = (s.default_zoom - 0.1).max(0.25);
            crate::message::Message::SaveSettings(s)
        }),
        Space::new(10, 0),
        action_btn("+", {
            let mut s = app.settings.clone();
            s.default_zoom = (s.default_zoom + 0.1).min(5.0);
            crate::message::Message::SaveSettings(s)
        }),
    ]
    .align_y(Alignment::Center);

    let cache_row = row![
        text(format!("Cache: {} pages", app.settings.cache_size))
            .font(INTER_REGULAR)
            .style(|_theme| {
                iced::widget::text::Style {
                    color: Some(Color::WHITE),
                }
            }),
        Space::new(Length::Fill, 0),
        action_btn("-", {
            let mut s = app.settings.clone();
            s.cache_size = s.cache_size.saturating_sub(10).max(10);
            crate::message::Message::SaveSettings(s)
        }),
        Space::new(10, 0),
        action_btn("+", {
            let mut s = app.settings.clone();
            s.cache_size = (s.cache_size + 10).min(200);
            crate::message::Message::SaveSettings(s)
        }),
    ]
    .align_y(Alignment::Center);

    let appearance_card = custom_card(
        text("Appearance")
            .size(18)
            .font(INTER_BOLD)
            .style(|_theme| iced::widget::text::Style {
                color: Some(Color::WHITE),
            }),
        column![theme_buttons, filter_buttons].spacing(16),
    );

    let performance_card = custom_card(
        text("Performance")
            .size(18)
            .font(INTER_BOLD)
            .style(|_theme| iced::widget::text::Style {
                color: Some(Color::WHITE),
            }),
        column![quality_buttons, cache_row].spacing(16),
    );

    let defaults_card = custom_card(
        text("Defaults")
            .size(18)
            .font(INTER_BOLD)
            .style(|_theme| iced::widget::text::Style {
                color: Some(Color::WHITE),
            }),
        default_zoom_row,
    );

    let behavior_card = custom_card(
        text("Behavior")
            .size(18)
            .font(INTER_BOLD)
            .style(|_theme| iced::widget::text::Style {
                color: Some(Color::WHITE),
            }),
        behavior_buttons,
    );

    container(scrollable(
        column![
            row![
                image(iced::widget::image::Handle::from_bytes(
                    include_bytes!("../PDFbull.png").to_vec(),
                ))
                .width(Length::Fixed(48.0)),
                column![
                    text("Settings").size(28).font(INTER_BOLD).style(|_theme| {
                        iced::widget::text::Style {
                            color: Some(Color::WHITE),
                        }
                    }),
                    text(format!("v{}", env!("CARGO_PKG_VERSION")))
                        .size(12)
                        .font(INTER_REGULAR)
                        .style(|_theme| iced::widget::text::Style {
                            color: Some(Color::from_rgb8(180, 180, 180))
                        }),
                ],
                Space::new(Length::Fill, 0),
                button(text("Close").size(16).font(INTER_BOLD).style(|_theme| {
                    iced::widget::text::Style {
                        color: Some(Color::WHITE),
                    }
                }))
                .on_press(crate::message::Message::CloseSettings)
                .style(iced::widget::button::text),
            ]
            .spacing(15)
            .align_y(Alignment::Center)
            .padding(20),
            appearance_card,
            Space::new(0, 20),
            performance_card,
            Space::new(0, 20),
            defaults_card,
            Space::new(0, 20),
            behavior_card,
            Space::new(0, 40),
        ]
        .padding(30)
        .width(Length::Fixed(640.0))
        .align_x(Alignment::Center),
    ))
    .width(Length::Fill)
    .height(Length::Fill)
    .style(|_| iced::widget::container::Style {
        background: Some(iced::Background::Color(Color::from_rgb8(35, 36, 40))),
        ..Default::default()
    })
    .into()
}
