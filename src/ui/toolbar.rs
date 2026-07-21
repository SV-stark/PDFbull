use crate::app::PdfBullApp;
use crate::app::{INTER_BOLD, INTER_REGULAR, LUCIDE, icons};
use crate::models::{PendingAnnotationKind, RibbonTab};
use crate::pdf_engine::RenderFilter;
use crate::ui::theme;
use iced::widget::{Space, button, column, container, pick_list, row, text, text_input, tooltip};
use iced::{Alignment, Border, Color, Element, Length};

pub fn render(app: &PdfBullApp) -> Element<'_, crate::message::Message> {
    let Some(tab) = app.current_tab() else {
        return container(row![]).into();
    };

    // --- 1. Top Header Segmented Ribbon Selector Bar ---
    let logo_and_title = row![
        text("🐂 PDFbull")
            .size(15)
            .font(INTER_BOLD)
            .style(|_| text::Style {
                color: Some(theme::COLOR_TEXT_PRIMARY),
            }),
        text("v0.9.0")
            .size(10)
            .font(INTER_REGULAR)
            .style(|_| text::Style {
                color: Some(theme::COLOR_ACCENT),
            }),
    ]
    .spacing(6)
    .align_y(Alignment::Center);

    let ribbon_tabs = row![
        button(
            row![text("🏠").size(13), text("Home").size(12).font(INTER_BOLD)]
                .spacing(6)
                .align_y(Alignment::Center)
        )
        .on_press(crate::message::Message::SetRibbonTab(RibbonTab::Home))
        .padding([6, 14])
        .style(theme::button_ribbon_tab(
            app.active_ribbon_tab == RibbonTab::Home
        )),
        button(
            row![text("👁️").size(13), text("View").size(12).font(INTER_BOLD)]
                .spacing(6)
                .align_y(Alignment::Center)
        )
        .on_press(crate::message::Message::SetRibbonTab(RibbonTab::View))
        .padding([6, 14])
        .style(theme::button_ribbon_tab(
            app.active_ribbon_tab == RibbonTab::View
        )),
        button(
            row![
                text("✏️").size(13),
                text("Annotate").size(12).font(INTER_BOLD)
            ]
            .spacing(6)
            .align_y(Alignment::Center)
        )
        .on_press(crate::message::Message::SetRibbonTab(RibbonTab::Annotate))
        .padding([6, 14])
        .style(theme::button_ribbon_tab(
            app.active_ribbon_tab == RibbonTab::Annotate
        )),
        button(
            row![
                text("🛠️").size(13),
                text("Page & Tools").size(12).font(INTER_BOLD)
            ]
            .spacing(6)
            .align_y(Alignment::Center)
        )
        .on_press(crate::message::Message::SetRibbonTab(RibbonTab::Tools))
        .padding([6, 14])
        .style(theme::button_ribbon_tab(
            app.active_ribbon_tab == RibbonTab::Tools
        )),
    ]
    .spacing(4)
    .align_y(Alignment::Center);

    let rendering_count = app.rendering_set.len();
    let rendering_indicator: Element<'_, crate::message::Message> = if rendering_count > 0 {
        row![
            container(text(format!("{rendering_count}")).font(INTER_BOLD).size(11))
                .padding([2, 6])
                .style(|_| iced::widget::container::Style {
                    background: Some(theme::COLOR_ACCENT.into()),
                    text_color: Some(Color::WHITE),
                    border: Border {
                        radius: theme::BORDER_RADIUS_FULL.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }),
            text("Rendering...")
                .size(11)
                .font(INTER_REGULAR)
                .style(|_| text::Style {
                    color: Some(theme::COLOR_TEXT_DIM)
                })
        ]
        .spacing(6)
        .align_y(Alignment::Center)
        .into()
    } else {
        Space::new().into()
    };

    let sig_badge: Element<'_, crate::message::Message> = if tab.signatures.is_empty() {
        Space::new().into()
    } else {
        let all_valid = tab
            .signatures
            .iter()
            .all(|sig| sig.digest_verified && sig.crypto_valid);
        let badge_text = if all_valid {
            "✍️ Signed"
        } else {
            "⚠️ Signature Warning"
        };
        let badge_color = if all_valid {
            theme::COLOR_SUCCESS
        } else {
            theme::COLOR_WARNING
        };

        button(
            container(
                text(badge_text)
                    .font(INTER_BOLD)
                    .size(11)
                    .style(|_| text::Style {
                        color: Some(Color::WHITE),
                    }),
            )
            .padding([4, 10])
            .style(move |_| iced::widget::container::Style {
                background: Some(badge_color.into()),
                border: Border {
                    radius: theme::BORDER_RADIUS_MD.into(),
                    ..Default::default()
                },
                ..Default::default()
            }),
        )
        .on_press(crate::message::Message::ToggleSignaturesDetail(true))
        .into()
    };

    let right_controls = row![
        sig_badge,
        rendering_indicator,
        tool_button(
            icons::SIDEBAR,
            "Sidebar",
            crate::message::Message::ToggleSidebar,
            app.show_sidebar,
            "Toggle navigation sidebar (Ctrl+B)"
        ),
        button(text(icons::SETTINGS).size(16).font(LUCIDE))
            .on_press(crate::message::Message::OpenSettings)
            .style(theme::button_ghost)
            .padding(8),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    let top_header_strip = container(
        row![
            logo_and_title,
            v_sep(),
            ribbon_tabs,
            Space::new().width(Length::Fill),
            right_controls,
        ]
        .spacing(12)
        .padding([0, 16])
        .align_y(Alignment::Center),
    )
    .width(Length::Fill)
    .height(Length::Fixed(40.0))
    .style(|_| iced::widget::container::Style {
        background: Some(theme::COLOR_BG_HEADER.into()),
        border: Border {
            width: 1.0,
            color: Color::from_rgb(0.04, 0.05, 0.07),
            ..Default::default()
        },
        ..Default::default()
    });

    // --- 2. Action Toolbar Strip Based on Active Ribbon Tab ---
    let action_strip: Element<'_, crate::message::Message> = match app.active_ribbon_tab {
        RibbonTab::Home => {
            let file_tools = row![
                tool_button(
                    icons::OPEN,
                    "Open",
                    crate::message::Message::OpenDocument,
                    false,
                    "Open document (Ctrl+O)"
                ),
                tool_button(
                    icons::SAVE,
                    "Save",
                    crate::message::Message::SaveAnnotations,
                    tab.annotations_dirty,
                    "Save annotations to PDF (Ctrl+S)"
                ),
                tool_button(
                    icons::PRINT,
                    "Print",
                    crate::message::Message::Print,
                    false,
                    "Print document (Ctrl+P)"
                ),
            ]
            .spacing(6);

            let page_nav = row![
                button(text(icons::PREV).size(14).font(LUCIDE))
                    .on_press(crate::message::Message::PrevPage)
                    .style(theme::button_ghost)
                    .padding(6),
                container(
                    row![
                        text_input("", &app.page_input)
                            .on_input(crate::message::Message::PageInputChanged)
                            .on_submit(crate::message::Message::PageInputSubmitted)
                            .font(INTER_BOLD)
                            .size(13)
                            .width(36)
                            .align_x(iced::alignment::Horizontal::Center),
                        text(format!(" / {}", tab.total_pages.max(1)))
                            .size(13)
                            .font(INTER_REGULAR)
                            .style(|_| text::Style {
                                color: Some(theme::COLOR_TEXT_DIM)
                            })
                    ]
                    .padding([2, 6])
                    .align_y(Alignment::Center)
                )
                .style(theme::input_field),
                button(text(icons::NEXT).size(14).font(LUCIDE))
                    .on_press(crate::message::Message::NextPage)
                    .style(theme::button_ghost)
                    .padding(6),
            ]
            .spacing(4)
            .align_y(Alignment::Center);

            let search_bar = container(
                row![
                    text(icons::SEARCH)
                        .font(LUCIDE)
                        .size(14)
                        .style(|_| text::Style {
                            color: Some(theme::COLOR_TEXT_SECONDARY),
                        }),
                    text_input("Search text...", &app.search_query)
                        .on_input(crate::message::Message::Search)
                        .on_submit(crate::message::Message::NextSearchResult)
                        .font(INTER_REGULAR)
                        .size(13)
                        .width(150)
                ]
                .spacing(8)
                .align_y(Alignment::Center)
                .padding([0, 10]),
            )
            .style(theme::input_field);

            container(
                row![
                    file_tools,
                    v_sep(),
                    zoom_control(tab.zoom),
                    v_sep(),
                    page_nav,
                    v_sep(),
                    search_bar,
                    Space::new().width(Length::Fill),
                    tool_button(
                        icons::HELP,
                        "Document Info",
                        crate::message::Message::ToggleMetadata,
                        app.show_metadata,
                        "View document metadata & properties"
                    ),
                ]
                .spacing(12)
                .padding([0, 16])
                .align_y(Alignment::Center),
            )
            .width(Length::Fill)
            .height(Length::Fixed(48.0))
            .style(|_| iced::widget::container::Style {
                background: Some(theme::COLOR_BG_SIDEBAR.into()),
                border: Border {
                    width: 1.0,
                    color: Color::from_rgb(0.12, 0.14, 0.18),
                    ..Default::default()
                },
                ..Default::default()
            })
            .into()
        }
        RibbonTab::View => {
            let is_midnight = tab.render_filter == RenderFilter::Inverted;
            let midnight_btn = tool_button_emoji(
                "🌙",
                "Midnight",
                crate::message::Message::SetFilter(if is_midnight {
                    RenderFilter::None
                } else {
                    RenderFilter::Inverted
                }),
                is_midnight,
                "Toggle Midnight Mode (Color Inversion)",
            );

            container(
                row![
                    zoom_control(tab.zoom),
                    v_sep(),
                    tool_button(
                        icons::ROTATE,
                        "Rotate 90°",
                        crate::message::Message::RotateClockwise,
                        false,
                        "Rotate page 90° clockwise"
                    ),
                    v_sep(),
                    midnight_btn,
                    v_sep(),
                    filter_section(tab.render_filter, tab.auto_crop),
                    v_sep(),
                    tool_button(
                        icons::HELP,
                        "Metadata",
                        crate::message::Message::ToggleMetadata,
                        app.show_metadata,
                        "Inspect document structural metadata"
                    ),
                ]
                .spacing(12)
                .padding([0, 16])
                .align_y(Alignment::Center),
            )
            .width(Length::Fill)
            .height(Length::Fixed(48.0))
            .style(|_| iced::widget::container::Style {
                background: Some(theme::COLOR_BG_SIDEBAR.into()),
                border: Border {
                    width: 1.0,
                    color: Color::from_rgb(0.12, 0.14, 0.18),
                    ..Default::default()
                },
                ..Default::default()
            })
            .into()
        }
        RibbonTab::Annotate => {
            let markup_tools = row![
                tool_button(
                    icons::HIGHLIGHT,
                    "Highlight",
                    crate::message::Message::SetAnnotationMode(Some(
                        PendingAnnotationKind::Highlight
                    )),
                    app.annotation_mode == Some(PendingAnnotationKind::Highlight),
                    "Highlight text"
                ),
                tool_button(
                    icons::RECTANGLE,
                    "Rectangle",
                    crate::message::Message::SetAnnotationMode(Some(
                        PendingAnnotationKind::Rectangle
                    )),
                    app.annotation_mode == Some(PendingAnnotationKind::Rectangle),
                    "Draw rectangle shape"
                ),
                tool_button_emoji(
                    "⭕",
                    "Circle",
                    crate::message::Message::SetAnnotationMode(Some(PendingAnnotationKind::Circle)),
                    app.annotation_mode == Some(PendingAnnotationKind::Circle),
                    "Draw circle shape"
                ),
                tool_button_emoji(
                    "📏",
                    "Line",
                    crate::message::Message::SetAnnotationMode(Some(PendingAnnotationKind::Line)),
                    app.annotation_mode == Some(PendingAnnotationKind::Line),
                    "Draw straight line"
                ),
                tool_button_emoji(
                    "➡️",
                    "Arrow",
                    crate::message::Message::SetAnnotationMode(Some(PendingAnnotationKind::Arrow)),
                    app.annotation_mode == Some(PendingAnnotationKind::Arrow),
                    "Draw directional arrow"
                ),
                tool_button_emoji(
                    "📌",
                    "Note",
                    crate::message::Message::SetAnnotationMode(Some(
                        PendingAnnotationKind::StickyNote
                    )),
                    app.annotation_mode == Some(PendingAnnotationKind::StickyNote),
                    "Insert sticky note comment"
                ),
                tool_button(
                    icons::TEXT,
                    "Text",
                    crate::message::Message::SetAnnotationMode(Some(PendingAnnotationKind::Text)),
                    app.annotation_mode == Some(PendingAnnotationKind::Text),
                    "Place text annotation"
                ),
                tool_button(
                    icons::BLOCK,
                    "Redact",
                    crate::message::Message::SetAnnotationMode(Some(PendingAnnotationKind::Redact)),
                    app.annotation_mode == Some(PendingAnnotationKind::Redact),
                    "Visual redaction box"
                ),
            ]
            .spacing(6);

            let style_section: Element<'_, crate::message::Message> = if let Some(mode) =
                app.annotation_mode
            {
                let colors = [
                    ("#3b82f6", "🔵"),
                    ("#ef4444", "🔴"),
                    ("#10b981", "🟢"),
                    ("#f59e0b", "🟡"),
                    ("#1f2937", "⚫"),
                ];

                let mut color_swatches = row![text("Color: ").size(11).font(INTER_BOLD)]
                    .spacing(4)
                    .align_y(Alignment::Center);

                for (hex, emoji) in colors {
                    let is_active = app.annotation_color == hex;
                    color_swatches = color_swatches.push(
                        button(text(emoji).size(14))
                            .on_press(crate::message::Message::SetAnnotationColor(hex.to_string()))
                            .style(move |_, _| iced::widget::button::Style {
                                background: if is_active {
                                    Some(theme::COLOR_BG_WIDGET.into())
                                } else {
                                    None
                                },
                                border: Border {
                                    radius: theme::BORDER_RADIUS_FULL.into(),
                                    width: if is_active { 2.0 } else { 0.0 },
                                    color: theme::COLOR_ACCENT,
                                },
                                ..Default::default()
                            })
                            .padding(2),
                    );
                }

                let thickness_label = format!("Size: {:.0}px", app.annotation_thickness);
                let thickness_control = row![
                    text(thickness_label).size(11).font(INTER_BOLD),
                    button("-")
                        .on_press(crate::message::Message::SetAnnotationThickness(
                            (app.annotation_thickness - 1.0).max(1.0)
                        ))
                        .padding([2, 6]),
                    button("+")
                        .on_press(crate::message::Message::SetAnnotationThickness(
                            (app.annotation_thickness + 1.0).min(10.0)
                        ))
                        .padding([2, 6]),
                ]
                .spacing(4)
                .align_y(Alignment::Center);

                let text_content_control: Element<'_, crate::message::Message> = if mode
                    == PendingAnnotationKind::Text
                    || mode == PendingAnnotationKind::StickyNote
                {
                    let placeholder = if mode == PendingAnnotationKind::Text {
                        "Annotation text..."
                    } else {
                        "Note content..."
                    };
                    row![
                        text("Content: ").size(11).font(INTER_BOLD),
                        text_input(placeholder, &app.annotation_text)
                            .on_input(crate::message::Message::AnnotationTextChanged)
                            .width(Length::Fixed(160.0))
                            .padding([4, 8])
                            .size(12),
                    ]
                    .spacing(4)
                    .align_y(Alignment::Center)
                    .into()
                } else {
                    Space::new().into()
                };

                row![
                    v_sep(),
                    color_swatches,
                    v_sep(),
                    thickness_control,
                    text_content_control,
                ]
                .spacing(12)
                .align_y(Alignment::Center)
                .into()
            } else {
                Space::new().into()
            };

            let save_btn = button(
                row![
                    text(icons::SAVE).size(12).font(LUCIDE),
                    text("Save Annotations").size(11).font(INTER_BOLD)
                ]
                .spacing(6)
                .align_y(Alignment::Center),
            )
            .on_press(crate::message::Message::SaveAnnotations)
            .padding([6, 12])
            .style(|_, _| iced::widget::button::Style {
                background: Some(theme::COLOR_ACCENT.into()),
                text_color: Color::WHITE,
                border: Border {
                    radius: theme::BORDER_RADIUS_MD.into(),
                    ..Default::default()
                },
                ..Default::default()
            });

            container(
                row![
                    markup_tools,
                    style_section,
                    Space::new().width(Length::Fill),
                    save_btn,
                ]
                .spacing(12)
                .padding([0, 16])
                .align_y(Alignment::Center),
            )
            .width(Length::Fill)
            .height(Length::Fixed(48.0))
            .style(|_| iced::widget::container::Style {
                background: Some(theme::COLOR_BG_SIDEBAR.into()),
                border: Border {
                    width: 1.0,
                    color: Color::from_rgb(0.12, 0.14, 0.18),
                    ..Default::default()
                },
                ..Default::default()
            })
            .into()
        }
        RibbonTab::Tools => {
            let tool_action_buttons = row![
                tool_button_emoji(
                    "📂",
                    "Organizer",
                    crate::message::Message::TogglePageOrganizer(!app.show_page_organizer),
                    app.show_page_organizer,
                    "Visual page grid to rotate, reorder, or delete pages"
                ),
                tool_button_emoji(
                    "📊",
                    "Tables",
                    crate::message::Message::ToggleTableMode,
                    app.table_mode_active,
                    "Extract data grids & copy as CSV/TSV"
                ),
                tool_button(
                    icons::FORMS,
                    "Forms",
                    crate::message::Message::ToggleFormsSidebar,
                    app.show_forms_sidebar,
                    "Spec-compliant PDF form filler sidebar"
                ),
                tool_button_emoji(
                    "🏷️",
                    "Watermark",
                    crate::message::Message::ToggleWatermarkPrompt(true),
                    false,
                    "Overlay custom text watermark across document pages"
                ),
                tool_button_emoji(
                    "✍️",
                    "Signature",
                    crate::message::Message::ToggleSignatureCreator(true),
                    app.show_signature_creator,
                    "Draw and stamp custom digital signature"
                ),
                tool_button(
                    icons::MERGE,
                    "Merge",
                    crate::message::Message::MergeDocuments(vec![]),
                    false,
                    "Combine multiple PDF files into one"
                ),
                tool_button_emoji(
                    "✂️",
                    "Split",
                    crate::message::Message::SplitPDF(vec![]),
                    false,
                    "Extract or split pages into separate PDF files"
                ),
                tool_button_emoji(
                    "⚡",
                    "Optimize",
                    crate::message::Message::OptimizePDF,
                    false,
                    "Compress streams & sanitize document metadata"
                ),
            ]
            .spacing(8)
            .align_y(Alignment::Center);

            container(
                row![tool_action_buttons, Space::new().width(Length::Fill)]
                    .spacing(12)
                    .padding([0, 16])
                    .align_y(Alignment::Center),
            )
            .width(Length::Fill)
            .height(Length::Fixed(48.0))
            .style(|_| iced::widget::container::Style {
                background: Some(theme::COLOR_BG_SIDEBAR.into()),
                border: Border {
                    width: 1.0,
                    color: Color::from_rgb(0.12, 0.14, 0.18),
                    ..Default::default()
                },
                ..Default::default()
            })
            .into()
        }
    };

    column![top_header_strip, action_strip].into()
}

fn v_sep() -> Element<'static, crate::message::Message> {
    container(Space::new().width(1.0))
        .width(1.0)
        .height(20.0)
        .style(|_| iced::widget::container::Style {
            background: Some(Color::from_rgb(0.18, 0.20, 0.25).into()),
            ..Default::default()
        })
        .into()
}

fn tool_button<'a>(
    icon: &'a str,
    label: &'a str,
    msg: crate::message::Message,
    active: bool,
    tooltip_text: &'a str,
) -> Element<'a, crate::message::Message> {
    tooltip(
        button(
            row![
                text(icon).size(14).font(LUCIDE),
                text(label).size(11).font(INTER_REGULAR).style(move |_| {
                    text::Style {
                        color: Some(if active {
                            Color::WHITE
                        } else {
                            theme::COLOR_TEXT_DIM
                        }),
                    }
                })
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        )
        .on_press(msg)
        .padding([5, 9])
        .style(theme::button_tool(active)),
        tooltip_text,
        tooltip::Position::Bottom,
    )
    .into()
}

fn tool_button_emoji<'a>(
    icon: &'a str,
    label: &'a str,
    msg: crate::message::Message,
    active: bool,
    tooltip_text: &'a str,
) -> Element<'a, crate::message::Message> {
    tooltip(
        button(
            row![
                text(icon).size(14),
                text(label).size(11).font(INTER_REGULAR).style(move |_| {
                    text::Style {
                        color: Some(if active {
                            Color::WHITE
                        } else {
                            theme::COLOR_TEXT_DIM
                        }),
                    }
                })
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        )
        .on_press(msg)
        .padding([5, 9])
        .style(theme::button_tool(active)),
        tooltip_text,
        tooltip::Position::Bottom,
    )
    .into()
}

fn zoom_control(zoom: f32) -> Element<'static, crate::message::Message> {
    container(
        row![
            button(text(icons::ZOOM_OUT).size(13).font(LUCIDE))
                .on_press(crate::message::Message::ZoomOut)
                .style(theme::button_ghost)
                .padding(4),
            text(format!("{}%", (zoom * 100.0) as u32))
                .size(12)
                .font(INTER_BOLD)
                .width(44)
                .align_x(iced::alignment::Horizontal::Center)
                .style(|_| text::Style {
                    color: Some(theme::COLOR_TEXT_PRIMARY)
                }),
            button(text(icons::ZOOM_IN).size(13).font(LUCIDE))
                .on_press(crate::message::Message::ZoomIn)
                .style(theme::button_ghost)
                .padding(4),
        ]
        .spacing(2)
        .align_y(Alignment::Center),
    )
    .padding([2, 4])
    .style(theme::input_field)
    .into()
}

fn filter_section(
    active_filter: RenderFilter,
    auto_crop: bool,
) -> Element<'static, crate::message::Message> {
    let filters = vec![
        "Normal".to_string(),
        "Grayscale".to_string(),
        "Inverted (Midnight)".to_string(),
        "Eco Mode".to_string(),
        "Black & White".to_string(),
        "Lighten".to_string(),
        "No Shadow".to_string(),
        "Sepia".to_string(),
    ];

    let current_filter_str = match active_filter {
        RenderFilter::None => "Normal",
        RenderFilter::Grayscale => "Grayscale",
        RenderFilter::Inverted => "Inverted (Midnight)",
        RenderFilter::Eco => "Eco Mode",
        RenderFilter::BlackWhite => "Black & White",
        RenderFilter::Lighten => "Lighten",
        RenderFilter::NoShadow => "No Shadow",
        RenderFilter::Sepia => "Sepia",
    }
    .to_string();

    let filter_dropdown = pick_list(filters, Some(current_filter_str), |selected| match selected
        .as_str()
    {
        "Normal" => crate::message::Message::SetFilter(RenderFilter::None),
        "Grayscale" => crate::message::Message::SetFilter(RenderFilter::Grayscale),
        "Inverted (Midnight)" => crate::message::Message::SetFilter(RenderFilter::Inverted),
        "Eco Mode" => crate::message::Message::SetFilter(RenderFilter::Eco),
        "Black & White" => crate::message::Message::SetFilter(RenderFilter::BlackWhite),
        "Lighten" => crate::message::Message::SetFilter(RenderFilter::Lighten),
        "No Shadow" => crate::message::Message::SetFilter(RenderFilter::NoShadow),
        "Sepia" => crate::message::Message::SetFilter(RenderFilter::Sepia),
        _ => crate::message::Message::ClearStatus,
    })
    .placeholder("Filters")
    .width(Length::Fixed(135.0));

    row![
        button(
            text(if auto_crop { "CROP ON" } else { "AUTO CROP" })
                .size(10)
                .font(INTER_BOLD)
        )
        .on_press(crate::message::Message::ToggleAutoCrop)
        .style(theme::button_tool(auto_crop))
        .padding([5, 9]),
        v_sep(),
        filter_dropdown,
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .into()
}
