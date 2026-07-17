pub mod sidebar;
pub mod tabs;
pub mod theme;
pub mod toolbar;

use crate::app::PdfBullApp;
use crate::app::{INTER_BOLD, INTER_REGULAR};
use crate::ui_document::document_view;
use crate::ui_keyboard_help::keyboard_help_view;
use crate::ui_metadata::metadata_view;
use crate::ui_settings::settings_view;
use crate::ui_welcome::welcome_view;
use iced::widget::{
    Space, Stack, button, canvas, column, container, image, row, scrollable, text, text_input,
};
use iced::{Alignment, Border, Color, Element, Length, Shadow, Vector};

// ── Signature Canvas Program ────────────────────────────────────────────────
struct SignatureCanvasProgram<'a> {
    lines: &'a [Vec<(f32, f32)>],
    active: bool,
}

impl<'a> canvas::Program<crate::message::Message> for SignatureCanvasProgram<'a> {
    type State = ();

    fn update(
        &self,
        _state: &mut Self::State,
        event: &iced::Event,
        bounds: iced::Rectangle,
        cursor: iced::mouse::Cursor,
    ) -> Option<canvas::Action<crate::message::Message>> {
        if !self.active {
            return None;
        }

        match event {
            iced::Event::Mouse(iced::mouse::Event::ButtonPressed(iced::mouse::Button::Left)) => {
                if let Some(pos) = cursor.position_in(bounds) {
                    Some(canvas::Action::publish(
                        crate::message::Message::SignatureDragStart { x: pos.x, y: pos.y },
                    ))
                } else {
                    None
                }
            }
            iced::Event::Mouse(iced::mouse::Event::CursorMoved { .. }) => {
                if let Some(pos) = cursor.position_in(bounds) {
                    Some(canvas::Action::publish(
                        crate::message::Message::SignatureDragUpdate { x: pos.x, y: pos.y },
                    ))
                } else {
                    None
                }
            }
            iced::Event::Mouse(iced::mouse::Event::ButtonReleased(iced::mouse::Button::Left)) => {
                Some(canvas::Action::publish(
                    crate::message::Message::SignatureDragEnd,
                ))
            }
            _ => None,
        }
    }

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &iced::Renderer,
        _theme: &iced::Theme,
        bounds: iced::Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());

        // Fill signature canvas background
        let rect_path = canvas::Path::rectangle(iced::Point::ORIGIN, bounds.size());
        frame.fill(&rect_path, Color::from_rgb8(242, 243, 246));

        // Draw guideline for signing alignment
        let dashed_path = canvas::Path::new(|builder| {
            let mid_y = bounds.height / 2.0 + 30.0;
            builder.move_to(iced::Point::new(15.0, mid_y));
            builder.line_to(iced::Point::new(bounds.width - 15.0, mid_y));
        });
        frame.stroke(
            &dashed_path,
            canvas::Stroke::default()
                .with_color(Color::from_rgb8(200, 201, 205))
                .with_width(1.0),
        );

        // Draw lines
        for stroke in self.lines {
            if stroke.len() < 2 {
                continue;
            }
            let path = canvas::Path::new(|builder| {
                builder.move_to(iced::Point::new(stroke[0].0, stroke[0].1));
                for &(x, y) in &stroke[1..] {
                    builder.line_to(iced::Point::new(x, y));
                }
            });
            frame.stroke(
                &path,
                canvas::Stroke::default()
                    .with_color(Color::from_rgb8(24, 28, 36)) // Ink blue-black color
                    .with_width(3.0)
                    .with_line_cap(canvas::LineCap::Round)
                    .with_line_join(canvas::LineJoin::Round),
            );
        }

        vec![frame.into_geometry()]
    }
}

// ── Overlay Modal: Watermark Prompt ──────────────────────────────────────────
fn watermark_prompt_view(app: &PdfBullApp) -> Element<'_, crate::message::Message> {
    let modal_content = container(
        column![
            text("🏷️ Add Document Watermark")
                .size(18)
                .font(INTER_BOLD)
                .style(|_| text::Style {
                    color: Some(Color::WHITE)
                }),
            Space::new().height(6),
            text("Enter the text to overlay across all pages of the document:")
                .size(13)
                .font(INTER_REGULAR)
                .style(|_| text::Style {
                    color: Some(theme::COLOR_TEXT_DIM)
                }),
            Space::new().height(12),
            text_input("e.g. CONFIDENTIAL, DRAFT", &app.watermark_input)
                .on_input(crate::message::Message::WatermarkInputChanged)
                .on_submit(crate::message::Message::SubmitWatermark)
                .padding(10)
                .size(14),
            Space::new().height(16),
            row![
                button(text("Cancel").size(13).font(INTER_REGULAR))
                    .on_press(crate::message::Message::ToggleWatermarkPrompt(false))
                    .style(theme::button_ghost)
                    .padding([8, 16]),
                Space::new().width(Length::Fill),
                button(text("Apply Watermark").size(13).font(INTER_BOLD))
                    .on_press(crate::message::Message::SubmitWatermark)
                    .padding([8, 16])
                    .style(|_theme, _status| button::Style {
                        background: Some(theme::COLOR_ACCENT.into()),
                        text_color: Color::WHITE,
                        border: Border {
                            radius: theme::BORDER_RADIUS_MD.into(),
                            ..Default::default()
                        },
                        ..Default::default()
                    }),
            ]
            .align_y(Alignment::Center)
        ]
        .spacing(10),
    )
    .padding(25)
    .width(Length::Fixed(440.0))
    .style(|_| container::Style {
        background: Some(Color::from_rgb8(30, 32, 36).into()),
        border: Border {
            radius: theme::BORDER_RADIUS_LG.into(),
            width: 1.0,
            color: Color::from_rgb8(54, 56, 62),
        },
        shadow: Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.45),
            offset: Vector::new(0.0, 8.0),
            blur_radius: 18.0,
        },
        ..Default::default()
    });

    container(modal_content)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .style(|_| container::Style {
            background: Some(Color::from_rgba(0.0, 0.0, 0.0, 0.65).into()),
            ..Default::default()
        })
        .into()
}

// ── Overlay Modal: Signature Creator ─────────────────────────────────────────
fn signature_creator_view(app: &PdfBullApp) -> Element<'_, crate::message::Message> {
    let sig_canvas = canvas(SignatureCanvasProgram {
        lines: &app.signature_lines,
        active: true,
    })
    .width(Length::Fill)
    .height(Length::Fixed(220.0));

    let modal_content = container(
        column![
            text("✍️ Create Digital Signature")
                .size(18)
                .font(INTER_BOLD)
                .style(|_| text::Style {
                    color: Some(Color::WHITE)
                }),
            Space::new().height(4),
            text("Draw your signature inside the box using mouse/trackpad:")
                .size(13)
                .font(INTER_REGULAR)
                .style(|_| text::Style {
                    color: Some(theme::COLOR_TEXT_DIM)
                }),
            Space::new().height(12),
            container(sig_canvas).style(|_| container::Style {
                border: Border {
                    radius: theme::BORDER_RADIUS_MD.into(),
                    width: 1.0,
                    color: Color::from_rgb8(58, 60, 66),
                },
                ..Default::default()
            }),
            Space::new().height(16),
            row![
                button(text("Cancel").size(13).font(INTER_REGULAR))
                    .on_press(crate::message::Message::ToggleSignatureCreator(false))
                    .style(theme::button_ghost)
                    .padding([8, 16]),
                button(text("🧹 Clear").size(13).font(INTER_REGULAR))
                    .on_press(crate::message::Message::ClearSignature)
                    .style(theme::button_ghost)
                    .padding([8, 16]),
                Space::new().width(Length::Fill),
                button(text("Save Signature").size(13).font(INTER_BOLD))
                    .on_press(crate::message::Message::SaveSignature)
                    .padding([8, 16])
                    .style(|_theme, _status| button::Style {
                        background: Some(theme::COLOR_ACCENT.into()),
                        text_color: Color::WHITE,
                        border: Border {
                            radius: theme::BORDER_RADIUS_MD.into(),
                            ..Default::default()
                        },
                        ..Default::default()
                    }),
            ]
            .align_y(Alignment::Center)
        ]
        .spacing(10),
    )
    .padding(25)
    .width(Length::Fixed(500.0))
    .style(|_| container::Style {
        background: Some(Color::from_rgb8(30, 32, 36).into()),
        border: Border {
            radius: theme::BORDER_RADIUS_LG.into(),
            width: 1.0,
            color: Color::from_rgb8(54, 56, 62),
        },
        shadow: Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.45),
            offset: Vector::new(0.0, 8.0),
            blur_radius: 18.0,
        },
        ..Default::default()
    });

    container(modal_content)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .style(|_| container::Style {
            background: Some(Color::from_rgba(0.0, 0.0, 0.0, 0.65).into()),
            ..Default::default()
        })
        .into()
}

// ── View: Page Organizer Grid ────────────────────────────────────────────────
fn organizer_view(app: &PdfBullApp) -> Element<'_, crate::message::Message> {
    let Some(tab) = app.current_tab() else {
        return container(text("No open document")).into();
    };

    let header = container(
        row![
            column![
                text("📂 Visual Page Organizer")
                    .size(24)
                    .font(INTER_BOLD)
                    .style(|_| text::Style {
                        color: Some(Color::WHITE)
                    }),
                text(format!(
                    "Rearrange, rotate, or delete pages • {} pages in document",
                    tab.page_mapping.len()
                ))
                .size(13)
                .font(INTER_REGULAR)
                .style(|_| text::Style {
                    color: Some(theme::COLOR_TEXT_DIM)
                }),
            ]
            .spacing(4),
            Space::new().width(Length::Fill),
            button(text("❌ Close Organizer").size(13).font(INTER_BOLD))
                .on_press(crate::message::Message::TogglePageOrganizer(false))
                .padding([10, 20])
                .style(|_theme, _status| button::Style {
                    background: Some(theme::COLOR_BG_WIDGET.into()),
                    text_color: Color::WHITE,
                    border: Border {
                        radius: theme::BORDER_RADIUS_MD.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }),
        ]
        .align_y(Alignment::Center),
    )
    .padding(20)
    .width(Length::Fill)
    .style(|_| container::Style {
        background: Some(theme::COLOR_BG_HEADER.into()),
        ..Default::default()
    });

    let mut grid_col = column![].spacing(30).align_x(Alignment::Center);
    let mut current_row = row![].spacing(30).align_y(Alignment::Start);

    for (ui_idx, &actual_page) in tab.page_mapping.iter().enumerate() {
        let thumb_widget: Element<'_, crate::message::Message> =
            if let Some(handle) = tab.view_state.thumbnails.get(&actual_page) {
                image(handle.clone())
                    .width(Length::Fixed(120.0))
                    .height(Length::Fixed(160.0))
                    .into()
            } else {
                container(
                    text("📄")
                        .size(36)
                        .align_y(iced::alignment::Vertical::Center)
                        .align_x(iced::alignment::Horizontal::Center),
                )
                .width(Length::Fixed(120.0))
                .height(Length::Fixed(160.0))
                .style(|_| container::Style {
                    background: Some(Color::from_rgb8(40, 42, 46).into()),
                    border: Border {
                        radius: theme::BORDER_RADIUS_SM.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .into()
            };

        let mut move_left_btn = button(text("◀").size(11))
            .style(theme::button_ghost)
            .padding(6);
        if ui_idx > 0 {
            move_left_btn =
                move_left_btn.on_press(crate::message::Message::OrganizerMovePage(ui_idx, -1));
        }

        let mut move_right_btn = button(text("▶").size(11))
            .style(theme::button_ghost)
            .padding(6);
        if ui_idx + 1 < tab.page_mapping.len() {
            move_right_btn =
                move_right_btn.on_press(crate::message::Message::OrganizerMovePage(ui_idx, 1));
        }

        let page_card = container(
            column![
                container(thumb_widget)
                    .padding(5)
                    .style(|_| container::Style {
                        background: Some(Color::from_rgb8(20, 21, 23).into()),
                        border: Border {
                            radius: theme::BORDER_RADIUS_MD.into(),
                            ..Default::default()
                        },
                        ..Default::default()
                    }),
                Space::new().height(5),
                text(format!("Page {}", ui_idx + 1))
                    .size(13)
                    .font(INTER_BOLD)
                    .align_x(iced::alignment::Horizontal::Center)
                    .style(|_| text::Style {
                        color: Some(Color::WHITE)
                    }),
                text(format!("Source index: {}", actual_page + 1))
                    .size(10)
                    .align_x(iced::alignment::Horizontal::Center)
                    .style(|_| text::Style {
                        color: Some(theme::COLOR_TEXT_SECONDARY)
                    }),
                Space::new().height(5),
                row![
                    move_left_btn,
                    button(text("🔄").size(11))
                        .on_press(crate::message::Message::OrganizerRotatePage(ui_idx, 90))
                        .style(theme::button_ghost)
                        .padding(6),
                    button(text("🗑️").size(11))
                        .on_press(crate::message::Message::OrganizerDeletePage(ui_idx))
                        .style(theme::button_ghost)
                        .padding(6),
                    move_right_btn,
                ]
                .spacing(10)
                .align_y(Alignment::Center)
            ]
            .align_x(Alignment::Center)
            .spacing(5),
        )
        .padding(12)
        .style(|_| container::Style {
            background: Some(theme::COLOR_BG_WIDGET.into()),
            border: Border {
                radius: theme::BORDER_RADIUS_LG.into(),
                width: 1.0,
                color: Color::from_rgb8(50, 52, 56),
            },
            ..Default::default()
        });

        current_row = current_row.push(page_card);

        if (ui_idx + 1) % 5 == 0 {
            grid_col = grid_col.push(current_row);
            current_row = row![].spacing(30).align_y(Alignment::Start);
        }
    }

    if tab.page_mapping.len() % 5 != 0 {
        grid_col = grid_col.push(current_row);
    }

    let content_scroll = scrollable(
        container(grid_col)
            .width(Length::Fill)
            .padding(30)
            .center_x(Length::Fill),
    )
    .width(Length::Fill)
    .height(Length::Fill);

    column![header, content_scroll]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

// ── Central UI View Coordinator ──────────────────────────────────────────────
pub fn view(app: &PdfBullApp) -> Element<'_, crate::message::Message> {
    if app.show_keyboard_help {
        return keyboard_help_view(app);
    }

    if app.show_settings {
        return settings_view(app);
    }

    let base = if app.tabs.is_empty() {
        welcome_view(app)
    } else if app.show_page_organizer {
        organizer_view(app)
    } else {
        document_view(app)
    };

    let mut base_stack = Stack::new().push(base);

    if app.show_watermark_prompt {
        base_stack = base_stack.push(watermark_prompt_view(app));
    }

    if app.show_signature_creator {
        base_stack = base_stack.push(signature_creator_view(app));
    }

    if app.show_metadata {
        base_stack = base_stack.push(metadata_view(app));
    }

    base_stack.into()
}
