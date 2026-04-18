use crate::app::PdfBullApp;
use crate::app::{INTER_BOLD, INTER_REGULAR, LUCIDE, icons};
use crate::models::PendingAnnotationKind;
use crate::pdf_engine::RenderFilter;
use crate::ui::theme;
use iced::widget::{Space, button, column, container, row, text, tooltip};
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
        tool_button(
            icons::BLOCK,
            "Redact",
            crate::message::Message::SetAnnotationMode(Some(PendingAnnotationKind::Redact)),
            app.annotation_mode == Some(PendingAnnotationKind::Redact),
            "Redact content"
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
        v_sep(),
        button(text(icons::SETTINGS).size(18).font(LUCIDE))
            .on_press(crate::message::Message::OpenSettings)
            .style(theme::button_ghost)
            .padding(8),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    container(
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
    })
    .into()
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
            filter_chip("Eco", RenderFilter::Eco, active_filter),
            filter_chip("Inv", RenderFilter::Inverted, active_filter),
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
