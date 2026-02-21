use crate::models::AppTheme;
use iced::widget::{button, column, row, text, Space};
use iced::{Alignment, Element, Length};

pub fn settings_view(app: &crate::PdfBullApp) -> Element<crate::Message> {
    let theme_buttons = row![
        button(if app.settings.theme == AppTheme::System {
            "System ✓"
        } else {
            "System"
        })
        .on_press({
            let mut s = app.settings.clone();
            s.theme = AppTheme::System;
            crate::Message::SaveSettings(s)
        }),
        button(if app.settings.theme == AppTheme::Light {
            "Light ✓"
        } else {
            "Light"
        })
        .on_press({
            let mut s = app.settings.clone();
            s.theme = AppTheme::Light;
            crate::Message::SaveSettings(s)
        }),
        button(if app.settings.theme == AppTheme::Dark {
            "Dark ✓"
        } else {
            "Dark"
        })
        .on_press({
            let mut s = app.settings.clone();
            s.theme = AppTheme::Dark;
            crate::Message::SaveSettings(s)
        }),
    ]
    .spacing(10);

    let behavior_buttons = row![
        button(if app.settings.remember_last_file {
            "Remember Last File ✓"
        } else {
            "Remember Last File"
        })
        .on_press({
            let mut s = app.settings.clone();
            s.remember_last_file = !s.remember_last_file;
            crate::Message::SaveSettings(s)
        }),
        button(if app.settings.auto_save {
            "Auto-save ✓"
        } else {
            "Auto-save"
        })
        .on_press({
            let mut s = app.settings.clone();
            s.auto_save = !s.auto_save;
            crate::Message::SaveSettings(s)
        }),
    ]
    .spacing(10);

    column![
        row![
            text("Settings").size(24),
            Space::new().width(Length::Fill),
            button("Close").on_press(crate::Message::CloseSettings),
        ]
        .padding(20),
        column![
            text("Appearance").size(18),
            theme_buttons.padding(10),
            Space::new().height(Length::Fixed(20.0)),
            text("Behavior").size(18),
            behavior_buttons.padding(10),
        ]
        .padding(20)
        .width(Length::Fixed(400.0))
    ]
    .align_x(Alignment::Center)
    .into()
}
