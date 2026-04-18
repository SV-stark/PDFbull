use crate::app::PdfBullApp;
use crate::app::{INTER_BOLD, INTER_REGULAR, LUCIDE, icons};
use crate::models::{AnnotationStyle, DocumentTab, PendingAnnotationKind};
use crate::ui::theme::{self, hex_to_rgb};
use iced::widget::{
    Space, Stack, button, column, container, mouse_area, row, scrollable, text, text_input,
};
use iced::{Alignment, Color, Element, Length, Padding};

use crate::ui::{sidebar, tabs, toolbar};

fn render_page_nav(app: &PdfBullApp) -> Element<'_, crate::message::Message> {
    let Some(tab) = app.current_tab() else {
        return container(row![]).into();
    };

    let loading_indicator = if app.rendering_count > 0 {
        row![
            container(
                text(format!("{}", app.rendering_count))
                    .font(INTER_BOLD)
                    .size(11)
            )
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
            let ann_overlay = match &ann.style {
                AnnotationStyle::Highlight { color } => {
                    let (r, g, b) = hex_to_rgb(color);
                    container(Space::new())
                        .width(Length::Fixed(ann.width * zoom))
                        .height(Length::Fixed(ann.height * zoom))
                        .style(move |_| iced::widget::container::Style {
                            background: Some(Color::from_rgba(r, g, b, 0.4).into()),
                            ..Default::default()
                        })
                }
                AnnotationStyle::Rectangle {
                    color,
                    thickness,
                    fill,
                } => {
                    let (r, g, b) = hex_to_rgb(color);
                    container(Space::new())
                        .width(Length::Fixed(ann.width * zoom))
                        .height(Length::Fixed(ann.height * zoom))
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
                }
                AnnotationStyle::Redact { color } => {
                    let (r, g, b) = hex_to_rgb(color);
                    container(Space::new())
                        .width(Length::Fixed(ann.width * zoom))
                        .height(Length::Fixed(ann.height * zoom))
                        .style(move |_| iced::widget::container::Style {
                            background: Some(Color::from_rgb(r, g, b).into()),
                            ..Default::default()
                        })
                }
            };

            container(ann_overlay)
                .padding(Padding {
                    top: ann.y * zoom,
                    left: ann.x * zoom,
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
            let (lx, ly, lw, lh) = link.bounds;
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

            container(
                Space::new()
                    .width(Length::Fixed(result.width * zoom))
                    .height(Length::Fixed(result.height * zoom)),
            )
            .style(move |_| iced::widget::container::Style {
                background: Some(iced::Background::Color(highlight_color)),
                ..Default::default()
            })
            .padding(Padding {
                top: result.y_position * zoom,
                left: result.x * zoom,
                ..Default::default()
            })
            .into()
        })
        .collect()
}

fn render_active_drag<'a>(
    page_idx: usize,
    zoom: f32,
    app: &'a PdfBullApp,
) -> Vec<Element<'a, crate::message::Message>> {
    if let Some(drag) = &app.annotation_drag
        && drag.page == page_idx
    {
        let min_x = drag.start.0.min(drag.current.0);
        let min_y = drag.start.1.min(drag.current.1);
        let w = (drag.start.0 - drag.current.0).abs();
        let h = (drag.start.1 - drag.current.1).abs();

        let preview_bg = match drag.kind {
            PendingAnnotationKind::Highlight => Color::from_rgba(1.0, 1.0, 0.0, 0.4),
            PendingAnnotationKind::Rectangle => Color::from_rgba(1.0, 0.0, 0.0, 0.2),
            PendingAnnotationKind::Redact => Color::from_rgba(0.0, 0.0, 0.0, 0.8),
        };

        let preview_border = match drag.kind {
            PendingAnnotationKind::Highlight => iced::Border::default(),
            PendingAnnotationKind::Rectangle | PendingAnnotationKind::Redact => iced::Border {
                color: Color::from_rgb(1.0, 0.0, 0.0),
                width: 2.0 * zoom,
                radius: 0.0.into(),
            },
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
            items
                .iter()
                .map(|item| {
                    container(text(item.text.clone()).size(item.height * zoom).style(|_| {
                        iced::widget::text::Style {
                            color: Some(Color::TRANSPARENT),
                        }
                    }))
                    .padding(Padding {
                        top: item.y * zoom,
                        left: item.x * zoom,
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
    let original_height = tab.page_heights.get(page_idx).copied().unwrap_or(800.0);
    let scaled_height = original_height * zoom;
    let scaled_width = tab.page_width * zoom;

    if let Some((_, handle)) = tab.view_state.rendered_pages.get(&page_idx) {
        let img = iced::widget::Image::new(handle.clone())
            .width(Length::Fixed(scaled_width))
            .height(Length::Fixed(scaled_height));

        let mut page_stack = Stack::new().push(img);

        for el in render_accessibility_layer(page_idx, tab, zoom) {
            page_stack = page_stack.push(el);
        }
        for el in render_annotations(page_idx, tab, zoom) {
            page_stack = page_stack.push(el);
        }
        for el in render_hyperlinks(page_idx, tab, zoom) {
            page_stack = page_stack.push(el);
        }
        for el in render_search_highlights(page_idx, tab, zoom, app) {
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
            .width(Length::Fill)
            .center_x(Length::Fill),
    )
    .id("pdf_scroll")
    .auto_scroll(true)
    .on_scroll(|viewport| {
        crate::message::Message::ViewportChanged(
            viewport.absolute_offset().y,
            viewport.bounds().height,
        )
    })
    .height(Length::Fill)
    .into()
}

pub fn document_view<'a>(
    app: &'a PdfBullApp,
    tab_names: &'a [&'static str],
) -> Element<'a, crate::message::Message> {
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
            tabs::render(app, tab_names),
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
