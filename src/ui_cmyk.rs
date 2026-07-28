use crate::app::{INTER_BOLD, INTER_REGULAR, PdfBullApp};
use crate::message::Message;
use iced::widget::{Space, button, column, container, row, slider, text};
use iced::{Alignment, Border, Color, Element, Length, Padding, Shadow, Vector};
// cmyk_to_rgb_naive and rgb_to_cmyk_naive are re-exported via `pub use zpdf_core::*`
use zpdf::{cmyk_to_rgb_naive, rgb_to_cmyk_naive};

// Module-level helper: must be defined before use to satisfy items_after_statements
fn channel_row(
    label: &'static str,
    idx: usize,
    val: f64,
    accent: Color,
) -> Element<'static, Message> {
    let pct = (val * 100.0).round() as u8;
    column![
        text(format!("{label}  {pct}%"))
            .size(12)
            .font(INTER_BOLD)
            .style(move |_| iced::widget::text::Style {
                color: Some(accent),
            }),
        slider(0.0..=1.0, val, move |v| Message::CmykValueChanged(idx, v))
            .step(0.01)
            .style(move |_, _| iced::widget::slider::Style {
                rail: iced::widget::slider::Rail {
                    backgrounds: (
                        accent.scale_alpha(0.35).into(),
                        accent.scale_alpha(0.12).into(),
                    ),
                    width: 4.0,
                    border: Default::default(),
                },
                handle: iced::widget::slider::Handle {
                    shape: iced::widget::slider::HandleShape::Circle { radius: 7.0 },
                    background: accent.into(),
                    border_width: 0.0,
                    border_color: Color::TRANSPARENT,
                },
            }),
    ]
    .spacing(4)
    .into()
}

/// Render the CMYK ↔ RGB live color inspector overlay (Feature 5).
///
/// Uses `zpdf_core::color::cmyk_to_rgb_naive` and `rgb_to_cmyk_naive` directly —
/// both are pure mathematical transforms with no I/O, computed on every frame.
#[allow(clippy::many_single_char_names)]
pub fn cmyk_inspector_view(app: &PdfBullApp) -> Element<'_, Message> {
    let [cyan, magenta, yellow, black] = app.cmyk_values;
    let (red, green, blue) = cmyk_to_rgb_naive(cyan, magenta, yellow, black);

    // Round-trip: RGB → CMYK to verify the naive GCR model's invertibility
    let (rt_c, rt_m, rt_y, rt_k) = rgb_to_cmyk_naive(red, green, blue);

    let swatch_color = Color::from_rgb(red as f32, green as f32, blue as f32);

    let sliders = column![
        channel_row("Cyan     ", 0, cyan, Color::from_rgb8(0, 190, 220)),
        channel_row("Magenta  ", 1, magenta, Color::from_rgb8(230, 50, 120)),
        channel_row("Yellow   ", 2, yellow, Color::from_rgb8(240, 200, 0)),
        channel_row("Key/Black", 3, black, Color::from_rgb8(180, 180, 180)),
    ]
    .spacing(10);

    // ── Live color swatch ─────────────────────────────────────────────────────
    let swatch = container(Space::new())
        .width(Length::Fill)
        .height(Length::Fixed(52.0))
        .style(move |_| iced::widget::container::Style {
            background: Some(swatch_color.into()),
            border: Border {
                radius: 8.0.into(),
                width: 1.0,
                color: Color::from_rgba(1.0, 1.0, 1.0, 0.12),
            },
            ..Default::default()
        });

    // ── RGB readout ───────────────────────────────────────────────────────────
    let r8 = (red * 255.0).round() as u8;
    let g8 = (green * 255.0).round() as u8;
    let b8 = (blue * 255.0).round() as u8;

    let rgb_row = row![
        text(format!("R {r8:3}"))
            .size(13)
            .font(INTER_BOLD)
            .style(|_| iced::widget::text::Style {
                color: Some(Color::from_rgb8(255, 100, 100)),
            }),
        Space::new().width(12),
        text(format!("G {g8:3}"))
            .size(13)
            .font(INTER_BOLD)
            .style(|_| iced::widget::text::Style {
                color: Some(Color::from_rgb8(80, 210, 100)),
            }),
        Space::new().width(12),
        text(format!("B {b8:3}"))
            .size(13)
            .font(INTER_BOLD)
            .style(|_| iced::widget::text::Style {
                color: Some(Color::from_rgb8(80, 150, 255)),
            }),
        Space::new().width(12),
        text(format!("#{r8:02X}{g8:02X}{b8:02X}"))
            .size(13)
            .font(INTER_BOLD)
            .style(|_| iced::widget::text::Style {
                color: Some(Color::from_rgb(0.65, 0.65, 0.65)),
            }),
    ]
    .align_y(Alignment::Center);

    // ── Round-trip CMYK verification ──────────────────────────────────────────
    let rt_text = format!(
        "Round-trip  C:{:.0}%  M:{:.0}%  Y:{:.0}%  K:{:.0}%",
        rt_c * 100.0,
        rt_m * 100.0,
        rt_y * 100.0,
        rt_k * 100.0,
    );

    // ── Modal panel ───────────────────────────────────────────────────────────
    let modal_content = container(
        column![
            row![
                text("\u{1f3a8}  CMYK \u{2194} RGB Inspector")
                    .size(17)
                    .font(INTER_BOLD)
                    .style(|_| iced::widget::text::Style {
                        color: Some(Color::WHITE),
                    }),
                Space::new().width(Length::Fill),
                button(text("\u{00d7}").size(16).font(INTER_BOLD).style(|_| {
                    iced::widget::text::Style {
                        color: Some(Color::from_rgb(0.55, 0.55, 0.55)),
                    }
                }))
                .on_press(Message::ToggleCmykInspector(false))
                .style(iced::widget::button::text)
                .padding([2, 6]),
            ]
            .align_y(Alignment::Center),
            Space::new().height(4),
            text(
                "Adjust CMYK channels. RGB computed via zpdf_core \
                 naive subtractive model (max GCR: k = 1 \u{2212} max(r,g,b))."
            )
            .size(11)
            .font(INTER_REGULAR)
            .style(|_| iced::widget::text::Style {
                color: Some(Color::from_rgb(0.45, 0.45, 0.45)),
            }),
            Space::new().height(16),
            sliders,
            Space::new().height(14),
            swatch,
            Space::new().height(10),
            rgb_row,
            Space::new().height(6),
            text(rt_text)
                .size(11)
                .font(INTER_REGULAR)
                .style(|_| iced::widget::text::Style {
                    color: Some(Color::from_rgb(0.45, 0.45, 0.45)),
                }),
        ]
        .spacing(0),
    )
    .padding(22)
    .width(Length::Fixed(380.0))
    .style(|_| iced::widget::container::Style {
        background: Some(Color::from_rgb8(24, 26, 32).into()),
        border: Border {
            radius: 12.0.into(),
            width: 1.0,
            color: Color::from_rgb8(50, 54, 65),
        },
        shadow: Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.55),
            offset: Vector::new(0.0, 12.0),
            blur_radius: 28.0,
        },
        ..Default::default()
    });

    container(modal_content)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_right(Length::Fill)
        .align_y(iced::alignment::Vertical::Center)
        .padding(Padding {
            top: 0.0,
            right: 20.0,
            bottom: 0.0,
            left: 0.0,
        })
        .style(|_| iced::widget::container::Style {
            background: Some(Color::from_rgba(0.0, 0.0, 0.0, 0.4).into()),
            ..Default::default()
        })
        .into()
}
