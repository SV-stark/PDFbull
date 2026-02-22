use crate::models::{AppTheme, RenderQuality};
use crate::pdf_engine::RenderFilter;
use iced::widget::{button, column, row, text, Space};
use iced::{Alignment, Element, Length, Color};
use iced_aw::widget::Card;

pub fn settings_view(app: &crate::app::PdfBullApp) -> Element<crate::message::Message> {
    let theme_buttons = row![
        button(if app.settings.theme == AppTheme::System {
            "System ✓"
        } else {
            "System"
        })
        .on_press({
            let mut s = app.settings.clone();
            s.theme = AppTheme::System;
            crate::message::Message::SaveSettings(s)
        }),
        button(if app.settings.theme == AppTheme::Light {
            "Light ✓"
        } else {
            "Light"
        })
        .on_press({
            let mut s = app.settings.clone();
            s.theme = AppTheme::Light;
            crate::message::Message::SaveSettings(s)
        }),
        button(if app.settings.theme == AppTheme::Dark {
            "Dark ✓"
        } else {
            "Dark"
        })
        .on_press({
            let mut s = app.settings.clone();
            s.theme = AppTheme::Dark;
            crate::message::Message::SaveSettings(s)
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
            crate::message::Message::SaveSettings(s)
        }),
        button(if app.settings.auto_save {
            "Auto-save ✓"
        } else {
            "Auto-save"
        })
        .on_press({
            let mut s = app.settings.clone();
            s.auto_save = !s.auto_save;
            crate::message::Message::SaveSettings(s)
        }),
        button(if app.settings.restore_session {
            "Restore Session ✓"
        } else {
            "Restore Session"
        })
        .on_press({
            let mut s = app.settings.clone();
            s.restore_session = !s.restore_session;
            crate::message::Message::SaveSettings(s)
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
            crate::message::Message::SaveSettings(s)
        }),
        button(if app.settings.render_quality == RenderQuality::Medium {
            "Medium ✓"
        } else {
            "Medium"
        })
        .on_press({
            let mut s = app.settings.clone();
            s.render_quality = RenderQuality::Medium;
            crate::message::Message::SaveSettings(s)
        }),
        button(if app.settings.render_quality == RenderQuality::High {
            "High ✓"
        } else {
            "High"
        })
        .on_press({
            let mut s = app.settings.clone();
            s.render_quality = RenderQuality::High;
            crate::message::Message::SaveSettings(s)
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
            crate::message::Message::SaveSettings(s)
        }),
        button(if app.settings.default_filter == RenderFilter::Grayscale {
            "Gray ✓"
        } else {
            "Gray"
        })
        .on_press({
            let mut s = app.settings.clone();
            s.default_filter = RenderFilter::Grayscale;
            crate::message::Message::SaveSettings(s)
        }),
        button(if app.settings.default_filter == RenderFilter::Inverted {
            "Invert ✓"
        } else {
            "Invert"
        })
        .on_press({
            let mut s = app.settings.clone();
            s.default_filter = RenderFilter::Inverted;
            crate::message::Message::SaveSettings(s)
        }),
        button(if app.settings.default_filter == RenderFilter::Eco {
            "Eco ✓"
        } else {
            "Eco"
        })
        .on_press({
            let mut s = app.settings.clone();
            s.default_filter = RenderFilter::Eco;
            crate::message::Message::SaveSettings(s)
        }),
    ]
    .spacing(10);

    let default_zoom_row = row![
        text("Default Zoom:"),
        text(format!("{}%", (app.settings.default_zoom * 100.0) as i32)),
        button("-").on_press({
            let mut s = app.settings.clone();
            s.default_zoom = (s.default_zoom - 0.25).max(0.25);
            crate::message::Message::SaveSettings(s)
        }),
        button("+").on_press({
            let mut s = app.settings.clone();
            s.default_zoom = (s.default_zoom + 0.25).min(5.0);
            crate::message::Message::SaveSettings(s)
        }),
    ]
    .spacing(10);

    let cache_row = row![
        text(format!("Cache: {} pages", app.settings.cache_size)),
        button("-").on_press({
            let mut s = app.settings.clone();
            s.cache_size = s.cache_size.saturating_sub(10).max(10);
            crate::message::Message::SaveSettings(s)
        }),
        button("+").on_press({
            let mut s = app.settings.clone();
            s.cache_size = (s.cache_size + 10).min(200);
            crate::message::Message::SaveSettings(s)
        }),
    ]
    .spacing(10);

    let appearance_card = Card::new(
        text("Appearance").size(18),
        column![
            theme_buttons.padding(10),
            filter_buttons.padding(10),
        ]
    )
    .padding(15)
    .style(iced_aw::widget::card::Style::Secondary);

    let performance_card = Card::new(
        text("Performance").size(18),
        column![
            quality_buttons.padding(10),
            cache_row.padding(10),
        ]
    )
    .padding(15)
    .style(iced_aw::widget::card::Style::Secondary);

    let defaults_card = Card::new(
        text("Defaults").size(18),
        default_zoom_row.padding(10)
    )
    .padding(15)
    .style(iced_aw::widget::card::Style::Secondary);

    let behavior_card = Card::new(
        text("Behavior").size(18),
        behavior_buttons.padding(10)
    )
    .padding(15)
    .style(iced_aw::widget::card::Style::Secondary);

    column![
        row![
            text("Settings").size(24),
            Space::new().width(Length::Fill),
            button("Close").on_press(crate::message::Message::CloseSettings),
        ]
        .padding(20),
        appearance_card,
        Space::new().height(Length::Fixed(10.0)),
        performance_card,
        Space::new().height(Length::Fixed(10.0)),
        defaults_card,
        Space::new().height(Length::Fixed(10.0)),
        behavior_card,
    ]
    .padding(20)
    .width(Length::Fixed(500.0))
    .align_x(Alignment::Center)
    .into()
}
