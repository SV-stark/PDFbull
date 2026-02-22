use crate::models::{AppTheme, RenderQuality};
use crate::pdf_engine::RenderFilter;
use iced::widget::{button, column, row, slider, text, Space};
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
            "Remember Last ✓"
        } else {
            "Remember Last"
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

    let quality_buttons = row![
        button(if app.settings.render_quality == RenderQuality::Low {
            "Low ✓"
        } else {
            "Low"
        })
        .on_press({
            let mut s = app.settings.clone();
            s.render_quality = RenderQuality::Low;
            crate::Message::SaveSettings(s)
        }),
        button(if app.settings.render_quality == RenderQuality::Medium {
            "Medium ✓"
        } else {
            "Medium"
        })
        .on_press({
            let mut s = app.settings.clone();
            s.render_quality = RenderQuality::Medium;
            crate::Message::SaveSettings(s)
        }),
        button(if app.settings.render_quality == RenderQuality::High {
            "High ✓"
        } else {
            "High"
        })
        .on_press({
            let mut s = app.settings.clone();
            s.render_quality = RenderQuality::High;
            crate::Message::SaveSettings(s)
        }),
    ]
    .spacing(10);

    let filter_buttons = row![
        button(if app.settings.default_filter == RenderFilter::None {
            "None ✓"
        } else {
            "None"
        })
        .on_press({
            let mut s = app.settings.clone();
            s.default_filter = RenderFilter::None;
            crate::Message::SaveSettings(s)
        }),
        button(if app.settings.default_filter == RenderFilter::Grayscale {
            "Gray ✓"
        } else {
            "Gray"
        })
        .on_press({
            let mut s = app.settings.clone();
            s.default_filter = RenderFilter::Grayscale;
            crate::Message::SaveSettings(s)
        }),
        button(if app.settings.default_filter == RenderFilter::Inverted {
            "Invert ✓"
        } else {
            "Invert"
        })
        .on_press({
            let mut s = app.settings.clone();
            s.default_filter = RenderFilter::Inverted;
            crate::Message::SaveSettings(s)
        }),
        button(if app.settings.default_filter == RenderFilter::Eco {
            "Eco ✓"
        } else {
            "Eco"
        })
        .on_press({
            let mut s = app.settings.clone();
            s.default_filter = RenderFilter::Eco;
            crate::Message::SaveSettings(s)
        }),
    ]
    .spacing(10);

    let default_zoom_row = row![
        text("Default Zoom:"),
        text(format!("{}%", (app.settings.default_zoom * 100.0) as i32)),
        button("-").on_press({
            let mut s = app.settings.clone();
            s.default_zoom = (s.default_zoom - 0.25).max(0.25);
            crate::Message::SaveSettings(s)
        }),
        button("+").on_press({
            let mut s = app.settings.clone();
            s.default_zoom = (s.default_zoom + 0.25).min(5.0);
            crate::Message::SaveSettings(s)
        }),
    ]
    .spacing(10);

    let cache_row = row![
        text(format!("Cache: {} pages", app.settings.cache_size)),
        button("-").on_press({
            let mut s = app.settings.clone();
            s.cache_size = s.cache_size.saturating_sub(10).max(10);
            crate::Message::SaveSettings(s)
        }),
        button("+").on_press({
            let mut s = app.settings.clone();
            s.cache_size = (s.cache_size + 10).min(200);
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
            Space::new().height(Length::Fixed(10.0)),
            filter_buttons.padding(10),
            Space::new().height(Length::Fixed(20.0)),
            text("Performance").size(18),
            quality_buttons.padding(10),
            cache_row.padding(10),
            Space::new().height(Length::Fixed(20.0)),
            text("Defaults").size(18),
            default_zoom_row.padding(10),
            Space::new().height(Length::Fixed(10.0)),
            text("Behavior").size(18),
            behavior_buttons.padding(10),
        ]
        .padding(20)
        .width(Length::Fixed(450.0))
    ]
    .align_x(Alignment::Center)
    .into()
}
