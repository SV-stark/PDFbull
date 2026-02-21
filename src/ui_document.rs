use crate::models::DocumentTab;
use crate::pdf_engine::RenderFilter;
use crate::PdfBullApp;
use iced::widget::{
    button, column, container, image as iced_image, row, scrollable, text, text_input, Space,
};
use iced::{Alignment, Element, Length};

fn render_toolbar(app: &PdfBullApp) -> Element<crate::Message> {
    let tab = &app.tabs[app.active_tab];

    row![
        button("Open").on_press(crate::Message::OpenDocument),
        button("Close").on_press(crate::Message::CloseTab(app.active_tab)),
        button("☰").on_press(crate::Message::ToggleSidebar),
        Space::new().width(Length::Fixed(10.0)),
        button("-").on_press(crate::Message::ZoomOut(app.active_tab)),
        text(format!("{}%", (tab.zoom * 100.0) as u32)),
        button("+").on_press(crate::Message::ZoomIn(app.active_tab)),
        Space::new().width(Length::Fixed(10.0)),
        button("↻R").on_press(crate::Message::RotateClockwise(app.active_tab)),
        button("↺R").on_press(crate::Message::RotateCounterClockwise(app.active_tab)),
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
        button("BM").on_press(crate::Message::AddBookmark(app.active_tab)),
        Space::new().width(Length::Fixed(10.0)),
        text_input("Search...", &app.search_query)
            .on_input(crate::Message::Search)
            .width(Length::Fixed(200.0)),
        Space::new().width(Length::Fixed(10.0)),
        button("Text").on_press(crate::Message::ExtractText(app.active_tab)),
        button("Export").on_press(crate::Message::ExportImage(app.active_tab)),
        Space::new().width(Length::Fill),
        button("Batch").on_press(crate::Message::OpenBatchMode),
        button("?").on_press(crate::Message::ToggleKeyboardHelp),
        button("⛶").on_press(crate::Message::ToggleFullscreen),
        button("⚙").on_press(crate::Message::OpenSettings),
    ]
    .padding(10)
    .into()
}

fn render_page_nav(app: &PdfBullApp) -> Element<crate::Message> {
    let tab = &app.tabs[app.active_tab];
    let active_tab = app.active_tab;

    row![
        button("Prev").on_press(crate::Message::PrevPage(app.active_tab)),
        text(format!(
            "Page {} of {}",
            tab.current_page + 1,
            tab.total_pages.max(1)
        )),
        button("Next").on_press(crate::Message::NextPage(app.active_tab)),
        Space::new().width(Length::Fixed(20.0)),
        text_input("Go to page", &(tab.current_page + 1).to_string())
            .on_input(move |v: String| {
                if let Ok(page) = v.parse::<usize>() {
                    crate::Message::JumpToPage(active_tab, page.saturating_sub(1))
                } else {
                    crate::Message::JumpToPage(active_tab, 0)
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

    // Outline section
    if !tab.outline.is_empty() {
        sidebar_col = sidebar_col.push(text("Outline").size(14));
        for bookmark in &tab.outline {
            sidebar_col = sidebar_col.push(
                button(text(&bookmark.title))
                    .on_press(crate::Message::JumpToPage(
                        app.active_tab,
                        bookmark.page_index as usize,
                    ))
                    .width(Length::Fill),
            );
        }
    }

    // Bookmarks section
    if !tab.bookmarks.is_empty() {
        sidebar_col = sidebar_col.push(text("Bookmarks").size(14));
        for (idx, bookmark) in tab.bookmarks.iter().enumerate() {
            sidebar_col = sidebar_col.push(row![
                button(text(&bookmark.label))
                    .on_press(crate::Message::JumpToBookmark(app.active_tab, idx))
                    .width(Length::Fill),
                button("×").on_press(crate::Message::RemoveBookmark(app.active_tab, idx))
            ]);
        }
    }

    // Thumbnails section
    sidebar_col = sidebar_col.push(text("Pages").size(14));
    for page_idx in 0..tab.total_pages {
        if let Some(handle) = tab.thumbnails.get(&page_idx) {
            let img = iced::widget::Image::new(handle.clone()).width(Length::Fixed(100.0));
            sidebar_col = sidebar_col
                .push(button(img).on_press(crate::Message::JumpToPage(app.active_tab, page_idx)));
        } else {
            sidebar_col = sidebar_col.push(
                button(text(format!("P{}", page_idx + 1)))
                    .on_press(crate::Message::JumpToPage(app.active_tab, page_idx))
                    .on_press(crate::Message::RequestThumbnail(app.active_tab, page_idx))
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

    for page_idx in 0..tab.total_pages {
        if let Some(handle) = tab.rendered_pages.get(&page_idx) {
            let img = iced::widget::Image::new(handle.clone());
            pdf_column = pdf_column.push(container(img).padding(5));
        } else {
            pdf_column =
                pdf_column.push(container(text(format!("Page {}", page_idx + 1))).padding(20));
        }
    }

    scrollable(container(pdf_column).width(Length::Fill))
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

    // In fullscreen mode, show minimal UI with exit hint
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
                button("-").on_press(crate::Message::ZoomOut(app.active_tab)),
                text(format!("{}%", (tab.zoom * 100.0) as u32)),
                button("+").on_press(crate::Message::ZoomIn(app.active_tab)),
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
