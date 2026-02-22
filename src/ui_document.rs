use crate::pdf_engine::RenderFilter;
use crate::app::PdfBullApp;
use iced::widget::{
    button, column, container, row, scrollable, text, text_input, Space,
    Id,
};
use iced::{Element, Length};

fn hex_to_rgb(hex: &str) -> (f32, f32, f32) {
    let hex = hex.trim_start_matches('#');
    if hex.len() != 6 {
        return (0.0, 0.0, 0.0);
    }
    let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0) as f32 / 255.0;
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0) as f32 / 255.0;
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0) as f32 / 255.0;
    (r, g, b)
}

fn render_toolbar(app: &PdfBullApp) -> Element<crate::message::Message> {
    let tab = &app.tabs[app.active_tab];

    let loading_indicator = if app.rendering_count > 0 {
        row![text("⟳").size(18)]
    } else {
        row![]
    };

    let row1 = row![
        button("Open").on_press(crate::message::Message::OpenDocument),
        button("Close").on_press(crate::message::Message::CloseTab(app.active_tab)),
        button("☰").on_press(crate::message::Message::ToggleSidebar),
        Space::new().width(Length::Fixed(10.0)),
        button("-").on_press(crate::message::Message::ZoomOut),
        text(format!("{}%", (tab.zoom * 100.0) as u32)),
        button("+").on_press(crate::message::Message::ZoomIn),
        Space::new().width(Length::Fixed(10.0)),
        button("↻R").on_press(crate::message::Message::RotateClockwise),
        button("↺R").on_press(crate::message::Message::RotateCounterClockwise),
        text(format!("{}°", tab.rotation)),
        Space::new().width(Length::Fixed(10.0)),
        button(match tab.render_filter {
            RenderFilter::None => "Filter",
            RenderFilter::Grayscale => "Gray",
            RenderFilter::Inverted => "Invert",
            RenderFilter::Eco => "Eco",
            RenderFilter::BlackWhite => "B&W",
            RenderFilter::Lighten => "Lighten",
            RenderFilter::NoShadow => "NoShadow",
        })
        .on_press(crate::message::Message::SetFilter(match tab.render_filter {
            RenderFilter::None => RenderFilter::Grayscale,
            RenderFilter::Grayscale => RenderFilter::Inverted,
            RenderFilter::Inverted => RenderFilter::Eco,
            RenderFilter::Eco => RenderFilter::BlackWhite,
            RenderFilter::BlackWhite => RenderFilter::Lighten,
            RenderFilter::Lighten => RenderFilter::NoShadow,
            RenderFilter::NoShadow => RenderFilter::None,
        })),
        button(if tab.auto_crop { "Crop✓" } else { "Crop" })
            .on_press(crate::message::Message::ToggleAutoCrop),
        Space::new().width(Length::Fill),
        loading_indicator,
        Space::new().width(Length::Fixed(10.0)),
        button("?").on_press(crate::message::Message::ToggleKeyboardHelp),
        button("⛶").on_press(crate::message::Message::ToggleFullscreen),
        button("⚙").on_press(crate::message::Message::OpenSettings),
    ].spacing(5).align_y(iced::Alignment::Center);

    let row2 = row![
        button("BM").on_press(crate::message::Message::AddBookmark),
        button("HiLite").on_press(crate::message::Message::AddHighlight),
        button("Rect").on_press(crate::message::Message::AddRectangle),
        button("SaveAnn").on_press(crate::message::Message::SaveAnnotations),
        Space::new().width(Length::Fixed(10.0)),
        text_input("Search...", &app.search_query)
            .on_input(crate::message::Message::Search)
            .width(Length::Fixed(200.0)),
        Space::new().width(Length::Fixed(10.0)),
        button("Text").on_press(crate::message::Message::ExtractText),
        button("Export").on_press(crate::message::Message::ExportImage),
        button("ExpAll").on_press(crate::message::Message::ExportImages),
    ].spacing(5).align_y(iced::Alignment::Center);

    column![row1, row2].spacing(10).padding(10).into()
}

fn render_page_nav(app: &PdfBullApp) -> Element<crate::message::Message> {
    let tab = &app.tabs[app.active_tab];

    let status = if let Some(ref msg) = app.status_message {
        row![
            Space::new().width(Length::Fill),
            text(msg).size(12),
            button("×").on_press(crate::message::Message::ClearStatus).padding(2),
        ]
    } else {
        row![]
    };

    row![
        button("Prev").on_press(crate::message::Message::PrevPage),
        text(format!(
            "Page {} of {}",
            tab.current_page + 1,
            tab.total_pages.max(1)
        )),
        button("Next").on_press(crate::message::Message::NextPage),
        Space::new().width(Length::Fixed(20.0)),
        text_input("Go to page", &(tab.current_page + 1).to_string())
            .on_input(move |v: String| {
                if let Ok(page) = v.parse::<usize>() {
                    crate::message::Message::JumpToPage(page.saturating_sub(1))
                } else {
                    crate::message::Message::JumpToPage(0)
                }
            })
            .width(Length::Fixed(80.0)),
        status,
    ]
    .padding(5)
    .into()
}

fn render_sidebar(app: &PdfBullApp) -> Element<crate::message::Message> {
    let tab = &app.tabs[app.active_tab];

    let mut sidebar_col = column![].spacing(10).padding(5).width(Length::Fixed(150.0));

    if !tab.outline.is_empty() {
        sidebar_col = sidebar_col.push(text("Outline").size(14));
        for bookmark in &tab.outline {
            sidebar_col = sidebar_col.push(
                button(text(&bookmark.title))
                    .on_press(crate::message::Message::JumpToPage(bookmark.page_index as usize))
                    .width(Length::Fill),
            );
        }
    }

    if !tab.bookmarks.is_empty() {
        sidebar_col = sidebar_col.push(text("Bookmarks").size(14));
        for (idx, bookmark) in tab.bookmarks.iter().enumerate() {
            sidebar_col = sidebar_col.push(row![
                button(text(&bookmark.label))
                    .on_press(crate::message::Message::JumpToBookmark(idx))
                    .width(Length::Fill),
                button("×").on_press(crate::message::Message::RemoveBookmark(idx))
            ]);
        }
    }

    if !tab.annotations.is_empty() {
        sidebar_col = sidebar_col.push(text("Annotations").size(14));
        for (idx, ann) in tab.annotations.iter().enumerate() {
            let label = match &ann.style {
                crate::models::AnnotationStyle::Highlight { .. } => format!("Highlight P{}", ann.page + 1),
                crate::models::AnnotationStyle::Rectangle { .. } => format!("Rect P{}", ann.page + 1),
                crate::models::AnnotationStyle::Text { text, .. } => format!("Text: {}", &text[..text.len().min(20)]),
            };
            sidebar_col = sidebar_col.push(row![
                button(text(label))
                    .on_press(crate::message::Message::JumpToPage(ann.page))
                    .width(Length::Fill),
                button("×").on_press(crate::message::Message::DeleteAnnotation(idx))
            ]);
        }
    }

    sidebar_col = sidebar_col.push(text("Pages").size(14));
    
    // Virtualization logic
    let thumbnail_height = 40.0; // Estimate height of a text button. If it's an image, it expands, but we'll use a fixed estimate for the viewport calculation.
    let start_idx = (tab.sidebar_viewport_y / thumbnail_height).max(0.0) as usize;
    let end_idx = (start_idx + 30).min(tab.total_pages);
    
    if start_idx > 0 {
        sidebar_col = sidebar_col.push(Space::new().width(Length::Fill).height(Length::Fixed(start_idx as f32 * thumbnail_height)));
    }
    
    for page_idx in start_idx..end_idx {
        if let Some(handle) = tab.thumbnails.get(&page_idx) {
            let img = iced::widget::Image::new(handle.clone()).width(Length::Fixed(100.0));
            sidebar_col =
                sidebar_col.push(button(img).on_press(crate::message::Message::JumpToPage(page_idx)));
        } else {
            sidebar_col = sidebar_col.push(
                button(text(format!("P{}", page_idx + 1)))
                    .on_press(crate::message::Message::JumpToPage(page_idx))
                    .width(Length::Fixed(100.0)),
            );
        }
    }
    
    let remaining = tab.total_pages.saturating_sub(end_idx);
    if remaining > 0 {
        sidebar_col = sidebar_col.push(Space::new().width(Length::Fill).height(Length::Fixed(remaining as f32 * thumbnail_height)));
    }

    scrollable(sidebar_col)
        .id(Id::new("sidebar_scroll"))
        .on_scroll(|viewport| crate::message::Message::SidebarViewportChanged(viewport.absolute_offset().y))
        .width(Length::Fixed(150.0))
        .into()
}

fn render_tabs(app: &PdfBullApp) -> Element<crate::message::Message> {
    let mut tabs_row = row![];
    for (idx, t) in app.tabs.iter().enumerate() {
        let name = t.name.clone();
        let is_active = idx == app.active_tab;
        let tab_button = if is_active {
            button(text(format!("● {}", name)))
        } else {
            button(text(name))
        };
        tabs_row = tabs_row.push(tab_button.on_press(crate::message::Message::SwitchTab(idx)));
    }
    tabs_row
        .push(button("+").on_press(crate::message::Message::OpenDocument))
        .into()
}

fn render_pdf_content(app: &PdfBullApp) -> Element<crate::message::Message> {
    let tab = &app.tabs[app.active_tab];

    let mut pdf_column = column![].spacing(10.0).padding(10.0);

    let visible_pages = tab.get_visible_pages();

    for page_idx in 0..tab.total_pages {
        let height = tab.page_heights.get(page_idx).copied().unwrap_or(800.0);
        let placeholder = container(text(format!("Page {}", page_idx + 1)))
            .width(Length::Fill)
            .height(Length::Fixed(height))
            .center_x(Length::Fill)
            .center_y(Length::Fill);

        if visible_pages.contains(&page_idx) {
            if let Some(handle) = tab.rendered_pages.get(&page_idx) {
                let img = iced::widget::Image::new(handle.clone()).width(Length::Fill);
                
                let mut page_stack = iced::widget::Stack::new().push(img);
                
                for ann in &tab.annotations {
                    if ann.page == page_idx {
                        let ann_overlay = match &ann.style {
                            crate::models::AnnotationStyle::Highlight { color } => {
                                let (r, g, b) = hex_to_rgb(color);
                                container(Space::new().width(Length::Fixed(ann.width)).height(Length::Fixed(ann.height)))
                                    .style(move |_| iced::widget::container::Style {
                                        background: Some(iced::Background::Color(iced::Color::from_rgba(r, g, b, 0.4))),
                                        ..Default::default()
                                    })
                            }
                            crate::models::AnnotationStyle::Rectangle { color, thickness, fill } => {
                                let (r, g, b) = hex_to_rgb(color);
                                container(Space::new().width(Length::Fixed(ann.width)).height(Length::Fixed(ann.height)))
                                    .style(move |_| iced::widget::container::Style {
                                        background: if *fill {
                                            Some(iced::Background::Color(iced::Color::from_rgba(r, g, b, 0.2)))
                                        } else {
                                            None
                                        },
                                        border: iced::Border {
                                            color: iced::Color::from_rgb(r, g, b),
                                            width: *thickness,
                                            radius: iced::border::Radius::from(0.0),
                                        },
                                        ..Default::default()
                                    })
                            }
                            crate::models::AnnotationStyle::Text { text, color, font_size } => {
                                let (r, g, b) = hex_to_rgb(color);
                                container(iced::widget::text(text.clone()).size(*font_size).color(iced::Color::from_rgb(r, g, b)))
                            }
                        };
                        
                        page_stack = page_stack.push(
                            container(ann_overlay)
                                .padding(iced::Padding {
                                    top: ann.y,
                                    right: 0.0,
                                    bottom: 0.0,
                                    left: ann.x,
                                })
                        );
                    }
                }
                
                pdf_column = pdf_column.push(container(page_stack).padding(5));
            } else {
                pdf_column = pdf_column.push(placeholder);
            }
        } else {
            pdf_column = pdf_column.push(placeholder);
        }
    }

    scrollable(container(pdf_column).width(Length::Fill))
        .id(Id::new("pdf_scroll"))
        .on_scroll(|viewport| {
            crate::message::Message::ViewportChanged(
                viewport.absolute_offset().y,
                viewport.bounds().height,
            )
        })
        .height(Length::Fill)
        .into()
}

pub fn document_view(app: &PdfBullApp) -> Element<crate::message::Message> {
    let tab = &app.tabs[app.active_tab];

    let content: Element<crate::message::Message> = if app.show_sidebar && !app.is_fullscreen {
        let sidebar = render_sidebar(app);
        let main_content = render_pdf_content(app);
        row![sidebar, main_content].into()
    } else if tab.total_pages == 0 {
        container(text(if tab.is_loading {
            "Loading..."
        } else {
            "No pages"
        }))
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
    } else {
        render_pdf_content(app)
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
        let tabs_row = render_tabs(app);
        let toolbar = render_toolbar(app);
        let page_nav = render_page_nav(app);

        column![tabs_row, toolbar, page_nav, content].into()
    }
}
