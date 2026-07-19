use crate::app::PdfBullApp;
use crate::app::{INTER_BOLD, INTER_REGULAR, LUCIDE, icons};
use crate::models::{AnnotationStyle, DocumentTab, PendingAnnotationKind};
use crate::ui::theme::{self, hex_to_rgb};
use iced::widget::{
    Space, Stack, button, canvas, column, container, mouse_area, row, scrollable, text, text_input,
};
use iced::{Alignment, Color, Element, Length, Padding, Rectangle};

use crate::ui::{sidebar, tabs, toolbar};

struct AnnotationCanvas<'a> {
    page_idx: usize,
    active: bool,
    annotations: &'a [crate::models::Annotation],
    zoom: f32,
    drag: Option<crate::models::AnnotationDrag>,
    rotation: i32,
    page_width: f32,
    page_height: f32,
}

impl<'a> canvas::Program<crate::message::Message> for AnnotationCanvas<'a> {
    type State = ();

    fn update(
        &self,
        _state: &mut Self::State,
        event: &iced::Event,
        bounds: Rectangle,
        cursor: iced::mouse::Cursor,
    ) -> Option<canvas::Action<crate::message::Message>> {
        if !self.active {
            return None;
        }

        match event {
            iced::Event::Mouse(iced::mouse::Event::ButtonPressed(iced::mouse::Button::Left)) => {
                if let Some(position) = cursor.position_in(bounds) {
                    Some(canvas::Action::publish(
                        crate::message::Message::AnnotationDragStart {
                            page: self.page_idx,
                            x: position.x,
                            y: position.y,
                        },
                    ))
                } else {
                    None
                }
            }
            iced::Event::Mouse(iced::mouse::Event::CursorMoved { .. }) => {
                if let Some(position) = cursor.position_in(bounds) {
                    Some(canvas::Action::publish(
                        crate::message::Message::AnnotationDragUpdate {
                            x: position.x,
                            y: position.y,
                        },
                    ))
                } else {
                    None
                }
            }
            iced::Event::Mouse(iced::mouse::Event::ButtonReleased(iced::mouse::Button::Left)) => {
                Some(canvas::Action::publish(
                    crate::message::Message::AnnotationDragEnd,
                ))
            }
            _ => None,
        }
    }

    #[allow(clippy::suboptimal_flops)]
    fn draw(
        &self,
        _state: &Self::State,
        renderer: &iced::Renderer,
        _theme: &iced::Theme,
        bounds: Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());

        // 1. Draw existing Line & Arrow annotations for this page
        for ann in self.annotations.iter().filter(|a| a.page == self.page_idx) {
            match &ann.style {
                AnnotationStyle::Line { color, thickness } => {
                    let (r, g, b) = hex_to_rgb(color);
                    let stroke_color = Color::from_rgb(r, g, b);
                    let (rx1, ry1, _, _) = crate::models::rotate_coords(
                        ann.x,
                        ann.y,
                        0.0,
                        0.0,
                        self.page_width,
                        self.page_height,
                        self.rotation,
                    );
                    let (rx2, ry2, _, _) = crate::models::rotate_coords(
                        ann.x + ann.width,
                        ann.y + ann.height,
                        0.0,
                        0.0,
                        self.page_width,
                        self.page_height,
                        self.rotation,
                    );
                    let p1 = iced::Point::new(rx1 * self.zoom, ry1 * self.zoom);
                    let p2 = iced::Point::new(rx2 * self.zoom, ry2 * self.zoom);

                    let path = canvas::Path::new(|builder| {
                        builder.move_to(p1);
                        builder.line_to(p2);
                    });
                    frame.stroke(
                        &path,
                        canvas::Stroke::default()
                            .with_color(stroke_color)
                            .with_width(*thickness * self.zoom),
                    );
                }
                AnnotationStyle::Arrow { color, thickness } => {
                    let (r, g, b) = hex_to_rgb(color);
                    let stroke_color = Color::from_rgb(r, g, b);
                    let (rx1, ry1, _, _) = crate::models::rotate_coords(
                        ann.x,
                        ann.y,
                        0.0,
                        0.0,
                        self.page_width,
                        self.page_height,
                        self.rotation,
                    );
                    let (rx2, ry2, _, _) = crate::models::rotate_coords(
                        ann.x + ann.width,
                        ann.y + ann.height,
                        0.0,
                        0.0,
                        self.page_width,
                        self.page_height,
                        self.rotation,
                    );
                    let x1 = rx1 * self.zoom;
                    let y1 = ry1 * self.zoom;
                    let x2 = rx2 * self.zoom;
                    let y2 = ry2 * self.zoom;

                    // Draw shaft
                    let path_shaft = canvas::Path::new(|builder| {
                        builder.move_to(iced::Point::new(x1, y1));
                        builder.line_to(iced::Point::new(x2, y2));
                    });
                    frame.stroke(
                        &path_shaft,
                        canvas::Stroke::default()
                            .with_color(stroke_color)
                            .with_width(*thickness * self.zoom),
                    );

                    // Draw wings
                    let dx = x2 - x1;
                    let dy = y2 - y1;
                    let len = dx.hypot(dy);
                    if len > 0.001 {
                        let ux = dx / len;
                        let uy = dy / len;
                        let wing_len = (10.0 + thickness * 2.0) * self.zoom;
                        let cos_30 = 0.866;
                        let sin_30 = 0.500;

                        let w1_x = x2 - wing_len * (ux * cos_30 + uy * sin_30);
                        let w1_y = y2 - wing_len * (uy * cos_30 - ux * sin_30);

                        let w2_x = x2 - wing_len * (ux * cos_30 - uy * sin_30);
                        let w2_y = y2 - wing_len * (uy * cos_30 + ux * sin_30);

                        let path_wings = canvas::Path::new(|builder| {
                            builder.move_to(iced::Point::new(x2, y2));
                            builder.line_to(iced::Point::new(w1_x, w1_y));
                            builder.move_to(iced::Point::new(x2, y2));
                            builder.line_to(iced::Point::new(w2_x, w2_y));
                        });
                        frame.stroke(
                            &path_wings,
                            canvas::Stroke::default()
                                .with_color(stroke_color)
                                .with_width(*thickness * self.zoom),
                        );
                    }
                }
                _ => {}
            }
        }

        // 2. Draw active dragging preview if it is a Line or Arrow
        if let Some(drag) = &self.drag
            && drag.page == self.page_idx
        {
            match drag.kind {
                PendingAnnotationKind::Line => {
                    let stroke_color = Color::from_rgb(1.0, 0.0, 0.0);
                    let p1 = iced::Point::new(drag.start.0, drag.start.1);
                    let p2 = iced::Point::new(drag.current.0, drag.current.1);

                    let path = canvas::Path::new(|builder| {
                        builder.move_to(p1);
                        builder.line_to(p2);
                    });
                    frame.stroke(
                        &path,
                        canvas::Stroke::default()
                            .with_color(stroke_color)
                            .with_width(2.0 * self.zoom),
                    );
                }
                PendingAnnotationKind::Arrow => {
                    let stroke_color = Color::from_rgb(1.0, 0.0, 0.0);
                    let x1 = drag.start.0;
                    let y1 = drag.start.1;
                    let x2 = drag.current.0;
                    let y2 = drag.current.1;

                    let path_shaft = canvas::Path::new(|builder| {
                        builder.move_to(iced::Point::new(x1, y1));
                        builder.line_to(iced::Point::new(x2, y2));
                    });
                    frame.stroke(
                        &path_shaft,
                        canvas::Stroke::default()
                            .with_color(stroke_color)
                            .with_width(2.0 * self.zoom),
                    );

                    let dx = x2 - x1;
                    let dy = y2 - y1;
                    let len = dx.hypot(dy);
                    if len > 0.001 {
                        let ux = dx / len;
                        let uy = dy / len;
                        let wing_len = 14.0 * self.zoom;
                        let cos_30 = 0.866;
                        let sin_30 = 0.500;

                        let w1_x = x2 - wing_len * (ux * cos_30 + uy * sin_30);
                        let w1_y = y2 - wing_len * (uy * cos_30 - ux * sin_30);

                        let w2_x = x2 - wing_len * (ux * cos_30 - uy * sin_30);
                        let w2_y = y2 - wing_len * (uy * cos_30 + ux * sin_30);

                        let path_wings = canvas::Path::new(|builder| {
                            builder.move_to(iced::Point::new(x2, y2));
                            builder.line_to(iced::Point::new(w1_x, w1_y));
                            builder.move_to(iced::Point::new(x2, y2));
                            builder.line_to(iced::Point::new(w2_x, w2_y));
                        });
                        frame.stroke(
                            &path_wings,
                            canvas::Stroke::default()
                                .with_color(stroke_color)
                                .with_width(2.0 * self.zoom),
                        );
                    }
                }
                _ => {}
            }
        }

        vec![frame.into_geometry()]
    }
}

fn render_page_nav(app: &PdfBullApp) -> Element<'_, crate::message::Message> {
    let Some(tab) = app.current_tab() else {
        return container(row![]).into();
    };

    let rendering_count = app.rendering_set.len();
    let loading_indicator = if rendering_count > 0 {
        row![
            container(text(format!("{rendering_count}")).font(INTER_BOLD).size(11))
                .padding([2, 5])
                .style(|_| iced::widget::container::Style {
                    background: Some(theme::COLOR_ACCENT.into()),
                    text_color: Some(Color::WHITE),
                    border: iced::Border {
                        radius: theme::BORDER_RADIUS_FULL.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }),
            text("Rendering...")
                .size(12)
                .font(INTER_REGULAR)
                .style(|_theme| iced::widget::text::Style {
                    color: Some(theme::COLOR_TEXT_DIM)
                })
        ]
        .spacing(6)
        .align_y(Alignment::Center)
    } else {
        row![]
    };

    container(
        row![
            loading_indicator,
            Space::new().width(Length::Fill),
            row![
                button(text(icons::PREV).size(14).font(LUCIDE))
                    .on_press(crate::message::Message::PrevPage)
                    .style(theme::button_ghost)
                    .padding(8),
                container(
                    row![
                        text_input("", &app.page_input)
                            .on_input(|input| {
                                if input.is_empty() || input.parse::<usize>().is_ok() {
                                    crate::message::Message::PageInputChanged(input)
                                } else {
                                    crate::message::Message::PageInputChanged(
                                        app.page_input.clone(),
                                    )
                                }
                            })
                            .on_submit(crate::message::Message::PageInputSubmitted)
                            .font(INTER_BOLD)
                            .size(13)
                            .width(36)
                            .align_x(iced::alignment::Horizontal::Center),
                        text(format!(" / {}", tab.total_pages.max(1)))
                            .size(13)
                            .font(INTER_REGULAR)
                            .style(|_| iced::widget::text::Style {
                                color: Some(theme::COLOR_TEXT_DIM)
                            })
                    ]
                    .padding([2, 8])
                    .align_y(Alignment::Center)
                )
                .style(theme::input_field),
                button(text(icons::NEXT).size(14).font(LUCIDE))
                    .on_press(crate::message::Message::NextPage)
                    .style(theme::button_ghost)
                    .padding(8),
            ]
            .spacing(4)
            .align_y(Alignment::Center),
            Space::new().width(Length::Fill),
            container(
                row![
                    text(icons::SEARCH).font(LUCIDE).size(14).style(|_| {
                        iced::widget::text::Style {
                            color: Some(theme::COLOR_TEXT_SECONDARY),
                        }
                    }),
                    text_input("Search in document...", &app.search_query)
                        .on_input(crate::message::Message::Search)
                        .on_submit(crate::message::Message::NextSearchResult)
                        .font(INTER_REGULAR)
                        .size(13)
                        .width(180)
                ]
                .spacing(8)
                .align_y(Alignment::Center)
                .padding([0, 12])
            )
            .style(theme::input_field),
        ]
        .align_y(Alignment::Center),
    )
    .width(Length::Fill)
    .height(Length::Fixed(theme::NAV_HEIGHT))
    .padding([0, 20])
    .style(|_theme| iced::widget::container::Style {
        background: Some(theme::COLOR_BG_SIDEBAR.into()),
        border: iced::Border {
            width: 1.0,
            color: Color::from_rgb(0.05, 0.05, 0.05),
            ..Default::default()
        },
        ..Default::default()
    })
    .into()
}

fn render_annotations<'a>(
    page_idx: usize,
    tab: &'a DocumentTab,
    zoom: f32,
) -> Vec<Element<'a, crate::message::Message>> {
    tab.annotations
        .iter()
        .filter(|ann| ann.page == page_idx)
        .map(|ann| {
            let actual_page = tab.page_mapping.get(page_idx).copied().unwrap_or(page_idx);
            let page_rotation = tab
                .page_rotations
                .get(&actual_page)
                .copied()
                .unwrap_or(tab.rotation);
            let original_height = tab.page_heights.get(page_idx).copied().unwrap_or(800.0);

            let (ann_x, ann_y, ann_width, ann_height) = crate::models::rotate_coords(
                ann.x,
                ann.y,
                ann.width,
                ann.height,
                tab.page_width,
                original_height,
                page_rotation,
            );

            let display_width = match &ann.style {
                AnnotationStyle::StickyNote { .. } => 24.0,
                _ => ann_width,
            };
            let display_height = match &ann.style {
                AnnotationStyle::StickyNote { .. } => 24.0,
                _ => ann_height,
            };

            let ann_overlay: Element<'a, crate::message::Message> = match &ann.style {
                AnnotationStyle::Highlight { color } => {
                    let (r, g, b) = hex_to_rgb(color);
                    container(Space::new())
                        .width(Length::Fixed(display_width * zoom))
                        .height(Length::Fixed(display_height * zoom))
                        .style(move |_| iced::widget::container::Style {
                            background: Some(Color::from_rgba(r, g, b, 0.4).into()),
                            ..Default::default()
                        })
                        .into()
                }
                AnnotationStyle::Rectangle {
                    color,
                    thickness,
                    fill,
                } => {
                    let (r, g, b) = hex_to_rgb(color);
                    container(Space::new())
                        .width(Length::Fixed(display_width * zoom))
                        .height(Length::Fixed(display_height * zoom))
                        .style(move |_| iced::widget::container::Style {
                            background: if *fill {
                                Some(Color::from_rgba(r, g, b, 0.2).into())
                            } else {
                                None
                            },
                            border: iced::Border {
                                color: Color::from_rgb(r, g, b),
                                width: *thickness * zoom,
                                radius: 0.0.into(),
                            },
                            ..Default::default()
                        })
                        .into()
                }
                AnnotationStyle::Text {
                    text,
                    color,
                    font_size,
                } => {
                    let (r, g, b) = hex_to_rgb(color);
                    container(
                        iced::widget::text(text.clone())
                            .size(*font_size as f32 * zoom)
                            .font(INTER_REGULAR)
                            .color(Color::from_rgb(r, g, b)),
                    )
                    .into()
                }
                AnnotationStyle::Redact { color } => {
                    let (r, g, b) = hex_to_rgb(color);
                    container(Space::new())
                        .width(Length::Fixed(display_width * zoom))
                        .height(Length::Fixed(display_height * zoom))
                        .style(move |_| iced::widget::container::Style {
                            background: Some(Color::from_rgb(r, g, b).into()),
                            ..Default::default()
                        })
                        .into()
                }
                AnnotationStyle::Circle {
                    color,
                    thickness,
                    fill,
                } => {
                    let (r, g, b) = hex_to_rgb(color);
                    container(Space::new())
                        .width(Length::Fixed(display_width * zoom))
                        .height(Length::Fixed(display_height * zoom))
                        .style(move |_| iced::widget::container::Style {
                            background: if *fill {
                                Some(Color::from_rgba(r, g, b, 0.2).into())
                            } else {
                                None
                            },
                            border: iced::Border {
                                color: Color::from_rgb(r, g, b),
                                width: *thickness * zoom,
                                radius: (display_width.min(display_height) * zoom / 2.0).into(),
                            },
                            ..Default::default()
                        })
                        .into()
                }
                AnnotationStyle::Line { .. } | AnnotationStyle::Arrow { .. } => {
                    container(Space::new())
                        .width(Length::Fixed(0.0))
                        .height(Length::Fixed(0.0))
                        .into()
                }
                AnnotationStyle::StickyNote { comment, color } => {
                    let (r, g, b) = hex_to_rgb(color);
                    iced::widget::tooltip(
                        container(
                            iced::widget::text("📝")
                                .size(14.0 * zoom)
                                .color(Color::BLACK),
                        )
                        .width(Length::Fixed(24.0 * zoom))
                        .height(Length::Fixed(24.0 * zoom))
                        .align_x(Alignment::Center)
                        .align_y(Alignment::Center)
                        .style(move |_| iced::widget::container::Style {
                            background: Some(Color::from_rgba(r, g, b, 0.9).into()),
                            border: iced::Border {
                                color: Color::from_rgb(r * 0.8, g * 0.8, b * 0.8),
                                width: 1.0,
                                radius: 4.0.into(),
                            },
                            ..Default::default()
                        }),
                        iced::widget::text(comment.clone()),
                        iced::widget::tooltip::Position::Top,
                    )
                    .into()
                }
            };

            let ann_idx = tab.annotations.iter().position(|a| a.id == ann.id);

            let mut delete_btn = button(
                iced::widget::text("×")
                    .size(9)
                    .font(INTER_BOLD)
                    .color(Color::WHITE),
            )
            .padding([1, 4])
            .style(|_theme, _status| button::Style {
                background: Some(Color::from_rgb(0.8, 0.2, 0.2).into()),
                text_color: Color::WHITE,
                border: iced::Border {
                    radius: theme::BORDER_RADIUS_SM.into(),
                    ..Default::default()
                },
                ..Default::default()
            });
            if let Some(idx) = ann_idx {
                delete_btn = delete_btn.on_press(crate::message::Message::DeleteAnnotation(idx));
            }

            let delete_btn_overlay = container(delete_btn)
                .width(Length::Fixed((display_width * zoom).max(16.0)))
                .height(Length::Fixed((display_height * zoom).max(16.0)))
                .align_x(Alignment::End)
                .align_y(Alignment::Start);

            let ann_stack: Element<'a, crate::message::Message> = match &ann.style {
                AnnotationStyle::Line { .. } | AnnotationStyle::Arrow { .. } => ann_overlay,
                _ => Stack::new()
                    .push(ann_overlay)
                    .push(delete_btn_overlay)
                    .into(),
            };

            container(ann_stack)
                .padding(Padding {
                    top: ann_y * zoom,
                    left: ann_x * zoom,
                    ..Default::default()
                })
                .into()
        })
        .collect()
}

fn render_hyperlinks<'a>(
    page_idx: usize,
    tab: &'a DocumentTab,
    zoom: f32,
) -> Vec<Element<'a, crate::message::Message>> {
    tab.links
        .iter()
        .filter(|link| link.page == page_idx)
        .map(|link| {
            let actual_page = tab.page_mapping.get(page_idx).copied().unwrap_or(page_idx);
            let page_rotation = tab
                .page_rotations
                .get(&actual_page)
                .copied()
                .unwrap_or(tab.rotation);
            let original_height = tab.page_heights.get(page_idx).copied().unwrap_or(800.0);

            let (lx, ly, lw, lh) = crate::models::rotate_coords(
                link.bounds.0,
                link.bounds.1,
                link.bounds.2,
                link.bounds.3,
                tab.page_width,
                original_height,
                page_rotation,
            );

            let overlay = mouse_area(
                container(Space::new())
                    .width(Length::Fixed(lw * zoom))
                    .height(Length::Fixed(lh * zoom))
                    .style(|_| iced::widget::container::Style::default()),
            )
            .on_release(crate::message::Message::LinkClicked(link.clone()));

            container(overlay)
                .padding(Padding {
                    top: ly * zoom,
                    left: lx * zoom,
                    ..Default::default()
                })
                .into()
        })
        .collect()
}

fn render_search_highlights<'a>(
    page_idx: usize,
    tab: &'a DocumentTab,
    zoom: f32,
    app: &'a PdfBullApp,
) -> Vec<Element<'a, crate::message::Message>> {
    if app.search_query.is_empty() {
        return vec![];
    }

    tab.search_results
        .iter()
        .enumerate()
        .filter(|(_, result)| result.page == page_idx)
        .map(|(result_idx, result)| {
            let is_active = result_idx == tab.current_search_index;
            let highlight_color = if is_active {
                Color::from_rgba(1.0, 0.6, 0.0, 0.6)
            } else {
                Color::from_rgba(1.0, 1.0, 0.0, 0.4)
            };

            let actual_page = tab.page_mapping.get(page_idx).copied().unwrap_or(page_idx);
            let page_rotation = tab
                .page_rotations
                .get(&actual_page)
                .copied()
                .unwrap_or(tab.rotation);
            let original_height = tab.page_heights.get(page_idx).copied().unwrap_or(800.0);

            let (rx, ry, rw, rh) = crate::models::rotate_coords(
                result.x,
                result.y_position,
                result.width,
                result.height,
                tab.page_width,
                original_height,
                page_rotation,
            );

            container(
                Space::new()
                    .width(Length::Fixed(rw * zoom))
                    .height(Length::Fixed(rh * zoom)),
            )
            .style(move |_| iced::widget::container::Style {
                background: Some(iced::Background::Color(highlight_color)),
                ..Default::default()
            })
            .padding(Padding {
                top: ry * zoom,
                left: rx * zoom,
                ..Default::default()
            })
            .into()
        })
        .collect()
}

fn render_selection_overlay<'a>(
    page_idx: usize,
    tab: &'a DocumentTab,
    zoom: f32,
) -> Vec<Element<'a, crate::message::Message>> {
    let mut overlays = Vec::new();

    // 1. Draw the active selection drag blue box
    if let Some((drag_page, start, current)) = tab.selection_drag {
        if drag_page == page_idx {
            let x = start.0.min(current.0);
            let y = start.1.min(current.1);
            let w = (current.0 - start.0).abs();
            let h = (current.1 - start.1).abs();

            overlays.push(
                container(Space::new())
                    .width(Length::Fixed(w * zoom))
                    .height(Length::Fixed(h * zoom))
                    .style(move |_| iced::widget::container::Style {
                        background: Some(Color::from_rgba(0.0, 0.4, 1.0, 0.15).into()),
                        border: iced::Border {
                            color: Color::from_rgba(0.0, 0.4, 1.0, 0.5),
                            width: 1.0,
                            radius: 0.0.into(),
                        },
                        ..Default::default()
                    })
                    .padding(Padding {
                        top: y * zoom,
                        left: x * zoom,
                        ..Default::default()
                    })
                    .into(),
            );
        }
    }

    // 2. Draw permanent selection highlight boxes for selected words
    for &(bx, by, bw, bh) in &tab.selected_boxes {
        let actual_page = tab.page_mapping.get(page_idx).copied().unwrap_or(page_idx);
        let page_rotation = tab
            .page_rotations
            .get(&actual_page)
            .copied()
            .unwrap_or(tab.rotation);
        let original_height = tab.page_heights.get(page_idx).copied().unwrap_or(800.0);

        let (rx, ry, rw, rh) = crate::models::rotate_coords(
            bx,
            by,
            bw,
            bh,
            tab.page_width,
            original_height,
            page_rotation,
        );

        overlays.push(
            container(Space::new())
                .width(Length::Fixed(rw * zoom))
                .height(Length::Fixed(rh * zoom))
                .style(move |_| iced::widget::container::Style {
                    background: Some(Color::from_rgba(0.0, 0.4, 1.0, 0.25).into()),
                    ..Default::default()
                })
                .padding(Padding {
                    top: ry * zoom,
                    left: rx * zoom,
                    ..Default::default()
                })
                .into(),
        );
    }

    overlays
}

fn render_active_drag<'a>(
    page_idx: usize,
    zoom: f32,
    app: &'a PdfBullApp,
) -> Vec<Element<'a, crate::message::Message>> {
    if let Some(drag) = &app.annotation_drag
        && drag.page == page_idx
    {
        if drag.kind == PendingAnnotationKind::Line || drag.kind == PendingAnnotationKind::Arrow {
            return vec![];
        }

        let min_x = drag.start.0.min(drag.current.0);
        let min_y = drag.start.1.min(drag.current.1);
        let w = (drag.start.0 - drag.current.0).abs();
        let h = (drag.start.1 - drag.current.1).abs();

        let preview_bg = match drag.kind {
            PendingAnnotationKind::Highlight => Color::from_rgba(1.0, 1.0, 0.0, 0.4),
            PendingAnnotationKind::Rectangle => Color::from_rgba(1.0, 0.0, 0.0, 0.2),
            PendingAnnotationKind::Redact => Color::from_rgba(0.0, 0.0, 0.0, 0.8),
            PendingAnnotationKind::Text => Color::from_rgba(0.0, 0.0, 1.0, 0.1),
            PendingAnnotationKind::Circle => Color::from_rgba(1.0, 0.0, 0.0, 0.2),
            PendingAnnotationKind::StickyNote => Color::from_rgba(1.0, 0.9, 0.3, 0.6),
            PendingAnnotationKind::Line | PendingAnnotationKind::Arrow => Color::TRANSPARENT,
        };

        let preview_border = match drag.kind {
            PendingAnnotationKind::Highlight => iced::Border::default(),
            PendingAnnotationKind::Rectangle
            | PendingAnnotationKind::Redact
            | PendingAnnotationKind::Text
            | PendingAnnotationKind::StickyNote => iced::Border {
                color: Color::from_rgb(1.0, 0.0, 0.0),
                width: 2.0 * zoom,
                radius: 0.0.into(),
            },
            PendingAnnotationKind::Circle => iced::Border {
                color: Color::from_rgb(1.0, 0.0, 0.0),
                width: 2.0 * zoom,
                radius: (w.min(h) * zoom / 2.0).into(),
            },
            PendingAnnotationKind::Line | PendingAnnotationKind::Arrow => iced::Border::default(),
        };

        return vec![
            container(
                Space::new()
                    .width(Length::Fixed(w * zoom))
                    .height(Length::Fixed(h * zoom)),
            )
            .style(move |_| iced::widget::container::Style {
                background: Some(iced::Background::Color(preview_bg)),
                border: preview_border,
                ..Default::default()
            })
            .padding(Padding {
                top: min_y * zoom,
                left: min_x * zoom,
                ..Default::default()
            })
            .into(),
        ];
    }
    vec![]
}

fn render_accessibility_layer<'a>(
    page_idx: usize,
    tab: &'a DocumentTab,
    zoom: f32,
) -> Vec<Element<'a, crate::message::Message>> {
    tab.view_state
        .text_layers
        .get(&page_idx)
        .map(|items| {
            let actual_page = tab.page_mapping.get(page_idx).copied().unwrap_or(page_idx);
            let page_rotation = tab
                .page_rotations
                .get(&actual_page)
                .copied()
                .unwrap_or(tab.rotation);
            let original_height = tab.page_heights.get(page_idx).copied().unwrap_or(800.0);

            items
                .iter()
                .map(|item| {
                    let (rx, ry, rw, rh) = crate::models::rotate_coords(
                        item.x,
                        item.y,
                        item.width,
                        item.height,
                        tab.page_width,
                        original_height,
                        page_rotation,
                    );
                    container(text(item.text.clone()).size(item.height * zoom).style(|_| {
                        iced::widget::text::Style {
                            color: Some(Color::TRANSPARENT),
                        }
                    }))
                    .width(Length::Fixed((rx + rw) * zoom))
                    .height(Length::Fixed((ry + rh) * zoom))
                    .padding(Padding {
                        top: ry * zoom,
                        left: rx * zoom,
                        ..Default::default()
                    })
                    .into()
                })
                .collect()
        })
        .unwrap_or_default()
}

fn render_page_canvas<'a>(
    page_idx: usize,
    tab: &'a DocumentTab,
    app: &'a PdfBullApp,
) -> Element<'a, crate::message::Message> {
    let zoom = tab.zoom;
    let actual_page = tab.page_mapping.get(page_idx).copied().unwrap_or(page_idx);
    let page_rotation = tab
        .page_rotations
        .get(&actual_page)
        .copied()
        .unwrap_or(tab.rotation);
    let original_height = tab.page_heights.get(page_idx).copied().unwrap_or(800.0);
    let is_landscape = page_rotation % 180 != 0;

    let (original_width, original_height_layout) = if is_landscape {
        (original_height, tab.page_width)
    } else {
        (tab.page_width, original_height)
    };
    let scaled_height = original_height_layout * zoom;
    let scaled_width = original_width * zoom;

    if let Some((_, handle)) = tab.view_state.rendered_pages.get(&page_idx) {
        let img = iced::widget::Image::new(handle.clone())
            .width(Length::Fixed(scaled_width))
            .height(Length::Fixed(scaled_height));

        let mut page_stack = Stack::new().push(img);

        for el in render_accessibility_layer(page_idx, tab, zoom) {
            page_stack = page_stack.push(el);
        }
        for el in render_selection_overlay(page_idx, tab, zoom) {
            page_stack = page_stack.push(el);
        }
        for el in render_search_highlights(page_idx, tab, zoom, app) {
            page_stack = page_stack.push(el);
        }

        // Add the annotation interaction layer (positioned underneath annotations & hyperlinks to let them capture clicks)
        page_stack = page_stack.push(
            canvas(AnnotationCanvas {
                page_idx,
                active: true,
                annotations: &tab.annotations,
                zoom,
                drag: app.annotation_drag.clone(),
                rotation: page_rotation,
                page_width: tab.page_width,
                page_height: original_height,
            })
            .width(Length::Fixed(scaled_width))
            .height(Length::Fixed(scaled_height)),
        );

        for el in render_annotations(page_idx, tab, zoom) {
            page_stack = page_stack.push(el);
        }
        for el in render_hyperlinks(page_idx, tab, zoom) {
            page_stack = page_stack.push(el);
        }
        for el in render_active_drag(page_idx, zoom, app) {
            page_stack = page_stack.push(el);
        }

        container(page_stack)
            .width(Length::Fixed(scaled_width))
            .height(Length::Fixed(scaled_height))
            .style(|_| iced::widget::container::Style {
                background: Some(iced::Background::Color(Color::WHITE)),
                ..Default::default()
            })
            .into()
    } else {
        container(
            column![
                text(format!("Page {}", page_idx + 1))
                    .font(INTER_REGULAR)
                    .size(16),
                text("Loading...").size(12).font(INTER_REGULAR).style(|_| {
                    iced::widget::text::Style {
                        color: Some(theme::COLOR_TEXT_DIM),
                    }
                })
            ]
            .spacing(10)
            .align_x(Alignment::Center),
        )
        .width(Length::Fixed(scaled_width))
        .height(Length::Fixed(scaled_height))
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .style(|_| iced::widget::container::Style {
            background: Some(Color::from_rgb8(45, 46, 50).into()),
            ..Default::default()
        })
        .into()
    }
}

fn render_pdf_content(app: &PdfBullApp) -> Element<'_, crate::message::Message> {
    let Some(tab) = app.current_tab() else {
        return container(column![]).into();
    };
    let zoom = tab.zoom;
    let scaled_spacing = theme::PAGE_SPACING * zoom;

    let mut pdf_column = column![]
        .spacing(scaled_spacing)
        .padding(theme::PAGE_PADDING * zoom)
        .align_x(Alignment::Center);

    let (start_idx, end_idx) = tab.view_state.visible_range;

    if start_idx > 0 {
        let y_above: f32 = tab
            .page_heights
            .iter()
            .take(start_idx)
            .map(|h| (h + theme::PAGE_SPACING) * zoom)
            .sum();
        let y_above = (y_above - scaled_spacing).max(0.0);
        if y_above > 0.0 {
            pdf_column = pdf_column.push(Space::new().height(y_above));
        }
    }

    for page_idx in start_idx..end_idx {
        pdf_column = pdf_column.push(render_page_canvas(page_idx, tab, app));
    }

    if end_idx < tab.total_pages {
        let y_below: f32 = tab
            .page_heights
            .iter()
            .skip(end_idx)
            .map(|h| (h + theme::PAGE_SPACING) * zoom)
            .sum();
        let y_below = (y_below - scaled_spacing).max(0.0);
        if y_below > 0.0 {
            pdf_column = pdf_column.push(Space::new().height(y_below));
        }
    }

    scrollable(
        container(pdf_column)
            .width(Length::Shrink)
            .center_x(Length::Fill),
    )
    .id("pdf_scroll")
    .auto_scroll(true)
    .direction(iced::widget::scrollable::Direction::Both {
        vertical: iced::widget::scrollable::Scrollbar::new(),
        horizontal: iced::widget::scrollable::Scrollbar::new(),
    })
    .on_scroll(|viewport| {
        crate::message::Message::ViewportChanged(
            viewport.absolute_offset().y,
            viewport.bounds().height,
        )
    })
    .height(Length::Fill)
    .into()
}

pub fn document_view<'a>(app: &'a PdfBullApp) -> Element<'a, crate::message::Message> {
    let Some(tab) = app.current_tab() else {
        return container(text("Loading tab..."))
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into();
    };

    let mut content_row = row![];

    let sidebar_width = app
        .sidebar_animation
        .interpolate_with(|v| v, std::time::Instant::now());

    if sidebar_width > 0.1 && !app.is_fullscreen {
        content_row = content_row.push(
            container(sidebar::render(app))
                .width(Length::Fixed(sidebar_width))
                .clip(true),
        );
    }

    content_row = content_row.push(render_pdf_content(app));

    if app.show_forms_sidebar && !app.is_fullscreen {
        content_row = content_row.push(sidebar::render_forms(app));
    }

    let content: Element<crate::message::Message> = if tab.total_pages == 0 {
        let empty_content: Element<_> = if tab.view_state.is_loading {
            column![
                text("⏳").size(40),
                text("Loading Document...")
                    .font(INTER_BOLD)
                    .size(20)
                    .style(|_| iced::widget::text::Style {
                        color: Some(theme::COLOR_TEXT_DIM)
                    })
            ]
            .align_x(Alignment::Center)
            .spacing(16)
            .into()
        } else {
            column![
                text(icons::OPEN)
                    .size(48)
                    .font(LUCIDE)
                    .style(|_| iced::widget::text::Style {
                        color: Some(theme::COLOR_TEXT_SECONDARY)
                    }),
                text("No Pages Found").font(INTER_BOLD).size(20).style(|_| {
                    iced::widget::text::Style {
                        color: Some(theme::COLOR_TEXT_DIM),
                    }
                }),
                text("This document might be empty or corrupted.")
                    .size(14)
                    .style(|_| iced::widget::text::Style {
                        color: Some(theme::COLOR_TEXT_SECONDARY)
                    }),
            ]
            .align_x(Alignment::Center)
            .spacing(12)
            .into()
        };
        container(empty_content)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .style(|_| iced::widget::container::Style {
                background: Some(theme::COLOR_BG_APP.into()),
                ..Default::default()
            })
            .into()
    } else {
        content_row.into()
    };

    if app.is_fullscreen {
        column![
            content,
            row![
                button("Exit Fullscreen (F)").on_press(crate::message::Message::ToggleFullscreen),
                container(text(format!(
                    "Page {} of {}",
                    tab.current_page + 1,
                    tab.total_pages
                )))
                .padding(10),
                button("-").on_press(crate::message::Message::ZoomOut),
                text(format!("{}%", (tab.zoom * 100.0) as u32)),
                button("+").on_press(crate::message::Message::ZoomIn),
            ]
            .padding(5)
        ]
        .into()
    } else {
        column![
            tabs::render(app),
            toolbar::render(app),
            render_page_nav(app),
            container(content).style(|_| iced::widget::container::Style {
                background: Some(theme::COLOR_BG_APP.into()),
                ..Default::default()
            })
        ]
        .into()
    }
}
