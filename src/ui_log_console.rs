use crate::app::{INTER_BOLD, INTER_REGULAR, PdfBullApp};
use crate::logging::{LogEntry, LogLevelFilter};
use crate::message::Message;
use iced::widget::{Space, button, column, container, pick_list, row, scrollable, text};
use iced::{Alignment, Border, Color, Element, Length, Shadow, Vector};

pub fn log_console_view<'a>(app: &'a PdfBullApp) -> Element<'a, Message> {
    let filtered_entries: Vec<&LogEntry> = app
        .log_entries
        .iter()
        .filter(|entry| app.log_level_filter.matches(&entry.level))
        .collect();

    let total_count = app.log_entries.len();
    let filtered_count = filtered_entries.len();

    // Format plain text for Copy All
    let mut clipboard_text = String::new();
    for entry in &filtered_entries {
        clipboard_text.push_str(&format!(
            "[{}] [{:<5}] {:<25} {}\n",
            entry.time, entry.level, entry.target, entry.message
        ));
    }

    let header = row![
        text("🖥️ Developer Log Console")
            .size(15)
            .font(INTER_BOLD)
            .style(|_| text::Style {
                color: Some(Color::WHITE),
            }),
        container(
            text(format!("{filtered_count} / {total_count}"))
                .size(11)
                .font(INTER_BOLD)
                .style(|_| text::Style {
                    color: Some(Color::from_rgb8(160, 165, 175)),
                })
        )
        .padding([2, 6])
        .style(|_| container::Style {
            background: Some(Color::from_rgb8(45, 47, 52).into()),
            border: Border {
                radius: 4.0.into(),
                ..Default::default()
            },
            ..Default::default()
        }),
        Space::new().width(Length::Fill),
        // Filter PickList
        row![
            text("Filter:")
                .size(12)
                .font(INTER_REGULAR)
                .style(|_| text::Style {
                    color: Some(Color::from_rgb8(160, 165, 175)),
                }),
            pick_list(
                &LogLevelFilter::ALL[..],
                Some(app.log_level_filter),
                Message::SetLogLevelFilter
            )
            .text_size(12)
            .padding([4, 8]),
        ]
        .spacing(6)
        .align_y(Alignment::Center),
        // Autoscroll toggle
        button(
            text(if app.log_autoscroll {
                "⏬ Auto-scroll: ON"
            } else {
                "⏸️ Auto-scroll: OFF"
            })
            .size(12)
            .font(INTER_REGULAR)
            .style(|_| text::Style {
                color: Some(Color::WHITE),
            })
        )
        .on_press(Message::ToggleLogAutoscroll)
        .padding([4, 10])
        .style(|_, status| {
            let bg = if status == button::Status::Hovered {
                Color::from_rgb8(55, 58, 65)
            } else {
                Color::from_rgb8(40, 42, 48)
            };
            button::Style {
                background: Some(bg.into()),
                border: Border {
                    radius: 6.0.into(),
                    width: 1.0,
                    color: Color::from_rgb8(60, 63, 70),
                },
                ..Default::default()
            }
        }),
        // Copy All Button
        button(
            text("📋 Copy All")
                .size(12)
                .font(INTER_REGULAR)
                .style(|_| text::Style {
                    color: Some(Color::WHITE),
                })
        )
        .on_press(Message::CopyToClipboard(clipboard_text))
        .padding([4, 10])
        .style(|_, status| {
            let bg = if status == button::Status::Hovered {
                Color::from_rgb8(55, 58, 65)
            } else {
                Color::from_rgb8(40, 42, 48)
            };
            button::Style {
                background: Some(bg.into()),
                border: Border {
                    radius: 6.0.into(),
                    width: 1.0,
                    color: Color::from_rgb8(60, 63, 70),
                },
                ..Default::default()
            }
        }),
        // Clear Button
        button(
            text("🗑️ Clear")
                .size(12)
                .font(INTER_REGULAR)
                .style(|_| text::Style {
                    color: Some(Color::WHITE),
                })
        )
        .on_press(Message::ClearLogs)
        .padding([4, 10])
        .style(|_, status| {
            let bg = if status == button::Status::Hovered {
                Color::from_rgb8(75, 45, 45)
            } else {
                Color::from_rgb8(55, 35, 35)
            };
            button::Style {
                background: Some(bg.into()),
                border: Border {
                    radius: 6.0.into(),
                    width: 1.0,
                    color: Color::from_rgb8(90, 45, 45),
                },
                ..Default::default()
            }
        }),
        // Close Button
        button(text("✕").size(14).font(INTER_BOLD).style(|_| text::Style {
            color: Some(Color::from_rgb8(200, 200, 210)),
        }))
        .on_press(Message::ToggleLogConsole(Some(false)))
        .padding([4, 8])
        .style(button::text),
    ]
    .spacing(10)
    .align_y(Alignment::Center)
    .padding([10, 14]);

    // Build log rows
    let log_rows = if filtered_entries.is_empty() {
        column![
            Space::new().height(40),
            text("No log entries match the current filter.")
                .size(13)
                .font(INTER_REGULAR)
                .style(|_| text::Style {
                    color: Some(Color::from_rgb8(130, 135, 145)),
                })
                .align_x(iced::alignment::Horizontal::Center),
        ]
        .width(Length::Fill)
        .align_x(Alignment::Center)
    } else {
        let mut col = column![].spacing(3).width(Length::Fill);
        for entry in filtered_entries {
            let level_color = match entry.level.as_str() {
                "ERROR" => Color::from_rgb8(248, 113, 113),
                "WARN" => Color::from_rgb8(251, 191, 36),
                "INFO" => Color::from_rgb8(96, 165, 250),
                "DEBUG" | "TRACE" => Color::from_rgb8(156, 163, 175),
                _ => Color::from_rgb8(209, 213, 219),
            };

            let log_line = row![
                text(&entry.time)
                    .size(11)
                    .font(INTER_REGULAR)
                    .style(|_| text::Style {
                        color: Some(Color::from_rgb8(107, 114, 128)),
                    }),
                text(format!("[{:<5}]", entry.level))
                    .size(11)
                    .font(INTER_BOLD)
                    .style(move |_| text::Style {
                        color: Some(level_color),
                    }),
                text(&entry.target)
                    .size(11)
                    .font(INTER_REGULAR)
                    .style(|_| text::Style {
                        color: Some(Color::from_rgb8(167, 139, 250)),
                    }),
                text(&entry.message)
                    .size(11)
                    .font(INTER_REGULAR)
                    .style(|_| text::Style {
                        color: Some(Color::from_rgb8(229, 231, 235)),
                    }),
            ]
            .spacing(8)
            .align_y(Alignment::Center);

            col = col.push(log_line);
        }
        col
    };

    let log_scroll = scrollable(container(log_rows).padding([8, 14]).width(Length::Fill))
        .id("log_console_scroll")
        .height(Length::Fill);

    let drawer_content = container(column![
        header,
        container(Space::new().height(1).width(Length::Fill)).style(|_| container::Style {
            background: Some(Color::from_rgb8(45, 48, 55).into()),
            ..Default::default()
        }),
        log_scroll,
    ])
    .height(Length::Fixed(290.0))
    .width(Length::Fill)
    .style(|_| container::Style {
        background: Some(Color::from_rgb8(24, 25, 29).into()),
        border: Border {
            radius: 0.0.into(),
            width: 1.0,
            color: Color::from_rgb8(45, 48, 55),
        },
        shadow: Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.4),
            offset: Vector::new(0.0, -4.0),
            blur_radius: 12.0,
        },
        ..Default::default()
    });

    // Layout docked at bottom
    column![Space::new().height(Length::Fill), drawer_content,]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}
