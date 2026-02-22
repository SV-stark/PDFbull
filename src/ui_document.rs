use crate::pdf_engine::RenderFilter;
use crate::PdfBullApp;
use iced::widget::{
    button, column, container, image as iced_image, row, scrollable, text, text_input, Space,
};
use iced::{Element, Length};

fn hex_to_rgb(hex: &str) -> (u8, u8, u8) {
    let hex = hex.trim_start_matches('#');
    if hex.len() != 6 {
        return (0, 0, 0);
    }
    let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
    (r, g, b)
}

fn render_toolbar(app: &PdfBullApp) -> Element<crate::Message> {
    let tab = &app.tabs[app.active_tab];

    row![
        button("Open").on_press(crate::Message::OpenDocument),
        button("Close").on_press(crate::Message::CloseTab(app.active_tab)),
        button("☰").on_press(crate::Message::ToggleSidebar),
        Space::new().width(Length::Fixed(10.0)),
        button("-").on_press(crate::Message::ZoomOut),
        text(format!("{}%", (tab.zoom * 100.0) as u32)),
        button("+").on_press(crate::Message::ZoomIn),
        Space::new().width(Length::Fixed(10.0)),
        button("↻R").on_press(crate::Message::RotateClockwise),
        button("↺R").on_press(crate::Message::RotateCounterClockwise),
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
        .on_press(crate::Message::SetFilter(match tab.render_filter {
            RenderFilter::None => RenderFilter::Grayscale,
            RenderFilter::Grayscale => RenderFilter::Inverted,
            RenderFilter::Inverted => RenderFilter::Eco,
            RenderFilter::Eco => RenderFilter::BlackWhite,
            RenderFilter::BlackWhite => RenderFilter::Lighten,
            RenderFilter::Lighten => RenderFilter::NoShadow,
            RenderFilter::NoShadow => RenderFilter::None,
        })),
        button(if tab.auto_crop { "Crop✓" } else { "Crop" })
            .on_press(crate::Message::ToggleAutoCrop),
        Space::new().width(Length::Fixed(10.0)),
        button("BM").on_press(crate::Message::AddBookmark),
        button("HiLite").on_press(crate::Message::AddHighlight),
        button("Rect").on_press(crate::Message::AddRectangle),
        Space::new().width(Length::Fixed(10.0)),
        text_input("Search...", &app.search_query)
            .on_input(crate::Message::Search)
            .width(Length::Fixed(200.0)),
        Space::new().width(Length::Fixed(10.0)),
        button("Text").on_press(crate::Message::ExtractText),
        button("Export").on_press(crate::Message::ExportImage),
        Space::new().width(Length::Fill),
        button("?").on_press(crate::Message::ToggleKeyboardHelp),
        button("⛶").on_press(crate::Message::ToggleFullscreen),
        button("⚙").on_press(crate::Message::OpenSettings),
    ]
    .padding(10)
    .into()
}

fn render_page_nav(app: &PdfBullApp) -> Element<crate::Message> {
    let tab = &app.tabs[app.active_tab];

    row![
        button("Prev").on_press(crate::Message::PrevPage),
        text(format!(
            "Page {} of {}",
            tab.current_page + 1,
            tab.total_pages.max(1)
        )),
        button("Next").on_press(crate::Message::NextPage),
        Space::new().width(Length::Fixed(20.0)),
        text_input("Go to page", &(tab.current_page + 1).to_string())
            .on_input(move |v: String| {
                if let Ok(page) = v.parse::<usize>() {
                    crate::Message::JumpToPage(page.saturating_sub(1))
                } else {
                    crate::Message::JumpToPage(0)
                }
            })
            .width(Length::Fixed(80.0)),
    ]
    .padding(5)
    .into()
}

fn render_sidebar(app: &PdfBullApp) -> Element<crate::Message> {
    let tab = &app.tabs[app.active_tab];

    let mut sidebar_col = column![].spacing(10).padding(5).width(Length::Fixed(150.0));

    if !tab.outline.is_empty() {
        sidebar_col = sidebar_col.push(text("Outline").size(14));
        for bookmark in &tab.outline {
            sidebar_col = sidebar_col.push(
                button(text(&bookmark.title))
                    .on_press(crate::Message::JumpToPage(bookmark.page_index as usize))
                    .width(Length::Fill),
            );
        }
    }

    if !tab.bookmarks.is_empty() {
        sidebar_col = sidebar_col.push(text("Bookmarks").size(14));
        for (idx, bookmark) in tab.bookmarks.iter().enumerate() {
            sidebar_col = sidebar_col.push(row![
                button(text(&bookmark.label))
                    .on_press(crate::Message::JumpToBookmark(idx))
                    .width(Length::Fill),
                button("×").on_press(crate::Message::RemoveBookmark(idx))
            ]);
        }
    }

    sidebar_col = sidebar_col.push(text("Pages").size(14));
    for page_idx in 0..tab.total_pages {
        if let Some(handle) = tab.thumbnails.get(&page_idx) {
            let img = iced::widget::Image::new(handle.clone()).width(Length::Fixed(100.0));
            sidebar_col =
                sidebar_col.push(button(img).on_press(crate::Message::JumpToPage(page_idx)));
        } else {
            sidebar_col = sidebar_col.push(
                button(text(format!("P{}", page_idx + 1)))
                    .on_press(crate::Message::JumpToPage(page_idx))
                    .width(Length::Fixed(100.0)),
            );
        }
    }

    scrollable(sidebar_col).width(Length::Fixed(150.0)).into()
}

fn render_tabs(app: &PdfBullApp) -> Element<crate::Message> {
    let mut tabs_row = row![];
    for (idx, t) in app.tabs.iter().enumerate() {
        let name = t.name.clone();
        tabs_row = tabs_row.push(button(text(name)).on_press(crate::Message::SwitchTab(idx)));
    }
    tabs_row
        .push(button("+").on_press(crate::Message::OpenDocument))
        .into()
}

fn render_pdf_content(app: &PdfBullApp) -> Element<crate::Message> {
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
                
                // Draw annotations
                let mut page_content = iced::widget::Stack::new().push(img);
                
                let scale = if let Some(h) = tab.page_heights.get(page_idx) {
                    *h / 800.0 // Approximate scale ratio based on default rendering 
                } else {
                    1.0
                };

                for ann in tab.annotations.iter().filter(|a| a.page == page_idx) {
                    let ann_widget: Element<_> = match &ann.style {
                        crate::models::AnnotationStyle::Highlight { color } => {
                            let (r, g, b) = hex_to_rgb(color);
                            container(Space::new(Length::Fixed(ann.width * scale), Length::Fixed(ann.height * scale)))
                                .style(move |_| container::Appearance {
                                    background: Some(iced::Color::from_rgba8(r, g, b, 0.4).into()),
                                    ..Default::default()
                                })
                                .into()
                        },
                        crate::models::AnnotationStyle::Rectangle { color, thickness, fill } => {
                            let (r, g, b) = hex_to_rgb(color);
                            container(Space::new(Length::Fixed(ann.width * scale), Length::Fixed(ann.height * scale)))
                                .style(move |_| container::Appearance {
                                    background: if *fill { Some(iced::Color::from_rgba8(r, g, b, 0.2).into()) } else { None },
                                    border: iced::border::Border {
                                        color: iced::Color::from_rgb8(r, g, b),
                                        width: *thickness,
                                        radius: 0.0.into(),
                                    },
                                    ..Default::default()
                                })
                                .into()
                        },
                        crate::models::AnnotationStyle::Text { text, color, font_size } => {
                            let (r, g, b) = hex_to_rgb(color);
                            iced::widget::text(text.clone())
                                .color(iced::Color::from_rgb8(r, g, b))
                                .size(*font_size as u16)
                                .into()
                        }
                    };
                    // Instead of full absolute positioning with stack, iced doesn't have a default AbsPos widget.
                    // But Stack supports absolute children if padded.
                    let positioned = container(ann_widget)
                        .padding(iced::Padding {
                            top: ann.y * scale,
                            right: 0.0,
                            bottom: 0.0,
                            left: ann.x * scale,
                        });
                    page_content = page_content.push(positioned);
                }

                pdf_column = pdf_column.push(container(page_content).padding(5));
            } else {
                pdf_column = pdf_column.push(placeholder);
            }
        } else {
            pdf_column = pdf_column.push(placeholder);
        }
    }

    scrollable(container(pdf_column).width(Length::Fill))
        .id(iced::widget::scrollable::Id::new("pdf_scroll"))
        .on_scroll(|viewport| {
            crate::Message::ViewportChanged(
                viewport.absolute_offset().y,
                viewport.bounds().height,
            )
        })
        .height(Length::Fill)
        .into()
}

pub fn document_view(app: &PdfBullApp) -> Element<crate::Message> {
    let tab = &app.tabs[app.active_tab];

    let content: Element<crate::Message> = if app.show_sidebar && !app.is_fullscreen {
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
                button("Exit Fullscreen (F)").on_press(crate::Message::ToggleFullscreen),
                container(text(format!(
                    "Page {} of {}",
                    tab.current_page + 1,
                    tab.total_pages
                )))
                .padding(10),
                button("-").on_press(crate::Message::ZoomOut),
                text(format!("{}%", (tab.zoom * 100.0) as u32)),
                button("+").on_press(crate::Message::ZoomIn),
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
