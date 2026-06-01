use crate::app::PdfBullApp;
use crate::app::{INTER_BOLD, INTER_REGULAR, LUCIDE, icons};
use crate::models::PendingAnnotationKind;
use crate::pdf_engine::RenderFilter;
use crate::ui::theme;
use iced::widget::{Space, button, column, container, pick_list, row, text, tooltip};
use iced::{Alignment, Border, Color, Element, Length, Shadow, Vector};

pub fn render(app: &PdfBullApp) -> Element<'_, crate::message::Message> {
    let Some(tab) = app.current_tab() else {
        return container(row![]).into();
    };

    // --- SECTION: System actions ---
    let system_tools = row![
        tool_button(
            icons::OPEN,
            "Open",
            crate::message::Message::OpenDocument,
            false,
            "Open document (Ctrl+O)"
        ),
        tool_button(
            icons::SIDEBAR,
            "Sidebar",
            crate::message::Message::ToggleSidebar,
            app.show_sidebar,
            "Toggle sidebar (Ctrl+B)"
        ),
        tool_button(
            icons::HELP,
            "Info",
            crate::message::Message::ToggleMetadata,
            app.show_metadata,
            "Document Information"
        ),
    ]
    .spacing(8);

    // --- SECTION: Navigation & View ---
    let view_tools = row![
        zoom_control(tab.zoom),
        v_sep(),
        tool_button(
            icons::ROTATE,
            "Rotate",
            crate::message::Message::RotateClockwise,
            false,
            "Rotate 90° clockwise"
        ),
        v_sep(),
        filter_section(tab.render_filter, tab.auto_crop),
    ]
    .spacing(12)
    .align_y(Alignment::Center);

    // --- SECTION: Markup Tools ---
    let markup_tools = row![
        tool_button(
            icons::HIGHLIGHT,
            "Highlight",
            crate::message::Message::SetAnnotationMode(Some(PendingAnnotationKind::Highlight)),
            app.annotation_mode == Some(PendingAnnotationKind::Highlight),
            "Highlight text"
        ),
        tool_button(
            icons::RECTANGLE,
            "Rect",
            crate::message::Message::SetAnnotationMode(Some(PendingAnnotationKind::Rectangle)),
            app.annotation_mode == Some(PendingAnnotationKind::Rectangle),
            "Draw rectangle"
        ),
        tool_button_emoji(
            "⭕",
            "Circle",
            crate::message::Message::SetAnnotationMode(Some(PendingAnnotationKind::Circle)),
            app.annotation_mode == Some(PendingAnnotationKind::Circle),
            "Draw circle"
        ),
        tool_button_emoji(
            "📏",
            "Line",
            crate::message::Message::SetAnnotationMode(Some(PendingAnnotationKind::Line)),
            app.annotation_mode == Some(PendingAnnotationKind::Line),
            "Draw line"
        ),
        tool_button_emoji(
            "➡️",
            "Arrow",
            crate::message::Message::SetAnnotationMode(Some(PendingAnnotationKind::Arrow)),
            app.annotation_mode == Some(PendingAnnotationKind::Arrow),
            "Draw arrow"
        ),
        tool_button_emoji(
            "📌",
            "Note",
            crate::message::Message::SetAnnotationMode(Some(PendingAnnotationKind::StickyNote)),
            app.annotation_mode == Some(PendingAnnotationKind::StickyNote),
            "Add sticky note"
        ),
        tool_button(
            icons::TEXT,
            "Text",
            crate::message::Message::SetAnnotationMode(Some(PendingAnnotationKind::Text)),
            app.annotation_mode == Some(PendingAnnotationKind::Text),
            "Add text annotation"
        ),
        tool_button(
            icons::BLOCK,
            "Redact",
            crate::message::Message::SetAnnotationMode(Some(PendingAnnotationKind::Redact)),
            app.annotation_mode == Some(PendingAnnotationKind::Redact),
            "⚠ Visual redaction only"
        ),
        v_sep(),
        tool_button(
            icons::SAVE,
            "Save",
            crate::message::Message::SaveAnnotations,
            false,
            "Save all annotations"
        ),
    ]
    .spacing(8);

    // --- SECTION: Utility / Right ---
    let tools = vec![
        "🖨️ Print PDF...".to_string(),
        "✂️ Split PDF (All)...".to_string(),
        "🔀 Merge PDFs...".to_string(),
        "🏷️ Add Watermark...".to_string(),
        "✍️ Create Signature...".to_string(),
        "📂 Page Organizer...".to_string(),
    ];

    let tools_dropdown = pick_list(tools, None::<String>, |selected| match selected.as_str() {
        "🖨️ Print PDF..." => crate::message::Message::Print,
        "✂️ Split PDF (All)..." => crate::message::Message::SplitPDF(vec![]),
        "🔀 Merge PDFs..." => crate::message::Message::MergeDocuments(vec![]),
        "🏷️ Add Watermark..." => crate::message::Message::ToggleWatermarkPrompt(true),
        "✍️ Create Signature..." => crate::message::Message::ToggleSignatureCreator(true),
        "📂 Page Organizer..." => {
            crate::message::Message::TogglePageOrganizer(!app.show_page_organizer)
        }
        _ => crate::message::Message::ClearStatus,
    })
    .placeholder("🛠️ Tools")
    .width(Length::Fixed(145.0));

    let util_tools = row![
        tool_button(
            icons::FORMS,
            "Forms",
            crate::message::Message::ToggleFormsSidebar,
            app.show_forms_sidebar,
            "Form fields"
        ),
        tool_button(
            icons::BOOKMARK,
            "Mark",
            crate::message::Message::AddBookmark,
            false,
            "Add bookmark"
        ),
        Space::new().width(2),
        tools_dropdown,
        Space::new().width(2),
        v_sep(),
        button(text(icons::SETTINGS).size(18).font(LUCIDE))
            .on_press(crate::message::Message::OpenSettings)
            .style(theme::button_ghost)
            .padding(8),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    let toolbar_container = container(
        row![
            system_tools,
            Space::new().width(Length::Fill),
            view_tools,
            Space::new().width(Length::Fill),
            markup_tools,
            Space::new().width(12),
            v_sep(),
            Space::new().width(12),
            util_tools,
        ]
        .padding([0, 20])
        .align_y(Alignment::Center),
    )
    .width(Length::Fill)
    .height(Length::Fixed(theme::TOOLBAR_HEIGHT))
    .style(|_theme| iced::widget::container::Style {
        background: Some(theme::COLOR_BG_HEADER.into()),
        border: Border {
            width: 1.0,
            color: Color::from_rgb(0.05, 0.05, 0.06),
            ..Default::default()
        },
        shadow: Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.3),
            offset: Vector::new(0.0, 2.0),
            blur_radius: 10.0,
        },
        ..Default::default()
    });

    let property_bar = if let Some(mode) = app.annotation_mode {
        let colors = [
            ("#408cff", "🔵"),
            ("#ff4d4d", "🔴"),
            ("#2ecc71", "🟢"),
            ("#f1c40f", "🟡"),
            ("#2c3e50", "⚫"),
        ];

        let mut color_swatches = row![text("Color: ").size(12).font(INTER_BOLD)]
            .spacing(6)
            .align_y(Alignment::Center);

        for (hex, emoji) in colors {
            let is_active = app.annotation_color == hex;
            color_swatches = color_swatches.push(
                button(text(emoji).size(16))
                    .on_press(crate::message::Message::SetAnnotationColor(hex.to_string()))
                    .style(move |_, _status| {
                        let border_color = if is_active {
                            theme::COLOR_ACCENT
                        } else {
                            Color::TRANSPARENT
                        };
                        iced::widget::button::Style {
                            background: if is_active {
                                Some(theme::COLOR_BG_WIDGET.into())
                            } else {
                                None
                            },
                            border: Border {
                                radius: theme::BORDER_RADIUS_FULL.into(),
                                width: 2.0,
                                color: border_color,
                            },
                            ..Default::default()
                        }
                    })
                    .padding(4),
            );
        }

        let thickness_label = format!("Thickness: {:.0}px", app.annotation_thickness);
        let thickness_control = row![
            text(thickness_label).size(12).font(INTER_BOLD),
            button("-")
                .on_press(crate::message::Message::SetAnnotationThickness(
                    (app.annotation_thickness - 1.0).max(1.0)
                ))
                .padding([2, 8]),
            button("+")
                .on_press(crate::message::Message::SetAnnotationThickness(
                    (app.annotation_thickness + 1.0).min(10.0)
                ))
                .padding([2, 8]),
        ]
        .spacing(6)
        .align_y(Alignment::Center);

        let text_size_control: Element<'_, crate::message::Message> =
            if mode == PendingAnnotationKind::Text {
                let label = format!("Text Size: {:.0}pt", app.annotation_text_size);
                row![
                    text(label).size(12).font(INTER_BOLD),
                    button("-")
                        .on_press(crate::message::Message::SetAnnotationTextSize(
                            (app.annotation_text_size - 1.0).max(8.0)
                        ))
                        .padding([2, 8]),
                    button("+")
                        .on_press(crate::message::Message::SetAnnotationTextSize(
                            (app.annotation_text_size + 1.0).min(36.0)
                        ))
                        .padding([2, 8]),
                ]
                .spacing(6)
                .align_y(Alignment::Center)
                .into()
            } else {
                Space::new().into()
            };

        let bar = container(
            row![
                text("✏️ Style:")
                    .size(12)
                    .font(INTER_BOLD)
                    .style(|_| iced::widget::text::Style {
                        color: Some(theme::COLOR_ACCENT)
                    }),
                v_sep(),
                color_swatches,
                v_sep(),
                thickness_control,
                v_sep(),
                text_size_control,
            ]
            .spacing(20)
            .padding([0, 20])
            .align_y(Alignment::Center),
        )
        .width(Length::Fill)
        .height(Length::Fixed(40.0))
        .style(|_| iced::widget::container::Style {
            background: Some(theme::COLOR_BG_SIDEBAR.into()),
            border: Border {
                width: 1.0,
                color: Color::from_rgb(0.15, 0.15, 0.17),
                ..Default::default()
            },
            ..Default::default()
        });

        Element::from(bar)
    } else {
        Space::new().into()
    };

    column![toolbar_container, property_bar].into()
}

fn v_sep() -> Element<'static, crate::message::Message> {
    container(Space::new().width(1.0))
        .width(1.0)
        .height(24.0)
        .style(|_| iced::widget::container::Style {
            background: Some(Color::from_rgb(0.2, 0.2, 0.22).into()),
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
            column![
                text(icon).size(18).font(LUCIDE),
                text(label).size(10).font(INTER_REGULAR).style(move |_| {
                    iced::widget::text::Style {
                        color: Some(if active {
                            Color::WHITE
                        } else {
                            theme::COLOR_TEXT_DIM
                        }),
                    }
                })
            ]
            .spacing(4)
            .align_x(Alignment::Center),
        )
        .on_press(msg)
        .padding([6, 10])
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
            column![
                text(icon).size(18),
                text(label).size(10).font(INTER_REGULAR).style(move |_| {
                    iced::widget::text::Style {
                        color: Some(if active {
                            Color::WHITE
                        } else {
                            theme::COLOR_TEXT_DIM
                        }),
                    }
                })
            ]
            .spacing(4)
            .align_x(Alignment::Center),
        )
        .on_press(msg)
        .padding([6, 10])
        .style(theme::button_tool(active)),
        tooltip_text,
        tooltip::Position::Bottom,
    )
    .into()
}

fn zoom_control(zoom: f32) -> Element<'static, crate::message::Message> {
    container(
        row![
            button(text(icons::ZOOM_OUT).size(14).font(LUCIDE))
                .on_press(crate::message::Message::ZoomOut)
                .style(theme::button_ghost)
                .padding(6),
            text(format!("{}%", (zoom * 100.0) as u32))
                .size(13)
                .font(INTER_BOLD)
                .width(48)
                .align_x(iced::alignment::Horizontal::Center)
                .style(|_| iced::widget::text::Style {
                    color: Some(theme::COLOR_TEXT_PRIMARY)
                }),
            button(text(icons::ZOOM_IN).size(14).font(LUCIDE))
                .on_press(crate::message::Message::ZoomIn)
                .style(theme::button_ghost)
                .padding(6),
        ]
        .spacing(4)
        .align_y(Alignment::Center),
    )
    .padding(4)
    .style(theme::input_field)
    .into()
}

fn filter_section(
    active_filter: RenderFilter,
    auto_crop: bool,
) -> Element<'static, crate::message::Message> {
    row![
        button(
            text(if auto_crop { "CROP ON" } else { "AUTO CROP" })
                .size(10)
                .font(INTER_BOLD)
        )
        .on_press(crate::message::Message::ToggleAutoCrop)
        .style(theme::button_tool(auto_crop))
        .padding([6, 10]),
        v_sep(),
        // Small dropdown-like selector for filters or just compact buttons
        row![
            filter_chip("None", RenderFilter::None, active_filter),
            filter_chip("Gray", RenderFilter::Grayscale, active_filter),
            filter_chip("Inv", RenderFilter::Inverted, active_filter),
            filter_chip("Sepia", RenderFilter::Sepia, active_filter),
            filter_chip("Eco", RenderFilter::Eco, active_filter),
            filter_chip("B&W", RenderFilter::BlackWhite, active_filter),
            filter_chip("Light", RenderFilter::Lighten, active_filter),
            filter_chip("NoShd", RenderFilter::NoShadow, active_filter),
        ]
        .spacing(4)
    ]
    .spacing(12)
    .align_y(Alignment::Center)
    .into()
}

fn filter_chip(
    label: &'static str,
    f: RenderFilter,
    active: RenderFilter,
) -> Element<'static, crate::message::Message> {
    let is_active = f == active;
    button(text(label).size(10).font(INTER_BOLD))
        .on_press(crate::message::Message::SetFilter(f))
        .padding([4, 8])
        .style(theme::button_tool(is_active))
        .into()
}
