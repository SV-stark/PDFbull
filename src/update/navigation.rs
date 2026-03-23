use crate::app::PdfBullApp;
use crate::message::Message;
use crate::update::scroll_to_page;
use iced::Task;

pub fn handle_nav_message(app: &mut PdfBullApp, message: Message) -> Task<Message> {
    match message {
        Message::NextPage => {
            let next_page = if let Some(tab) = app.current_tab_mut() {
                if tab.current_page + 1 < tab.total_pages {
                    tab.current_page += 1;
                    Some(tab.current_page)
                } else {
                    None
                }
            } else {
                None
            };

            if let Some(page) = next_page {
                app.page_input = (page + 1).to_string();
                if let Some(tab) = app.current_tab_mut() {
                    return scroll_to_page(tab, page);
                }
            }
            Task::none()
        }
        Message::PrevPage => {
            let prev_page = if let Some(tab) = app.current_tab_mut() {
                if tab.current_page > 0 {
                    tab.current_page -= 1;
                    Some(tab.current_page)
                } else {
                    None
                }
            } else {
                None
            };

            if let Some(page) = prev_page {
                app.page_input = (page + 1).to_string();
                if let Some(tab) = app.current_tab_mut() {
                    return scroll_to_page(tab, page);
                }
            }
            Task::none()
        }
        Message::ZoomIn => {
            if let Some(tab) = app.current_tab_mut() {
                tab.zoom = (tab.zoom * 1.1).min(5.0);
            }
            app.render_visible_pages()
        }
        Message::ZoomOut => {
            if let Some(tab) = app.current_tab_mut() {
                tab.zoom = (tab.zoom / 1.1).max(0.25);
            }
            app.render_visible_pages()
        }
        Message::SetZoom(zoom) => {
            if let Some(tab) = app.current_tab_mut() {
                tab.zoom = zoom.clamp(0.25, 5.0);
            }
            app.render_visible_pages()
        }
        Message::JumpToPage(page) => {
            let jump_page = if let Some(tab) = app.current_tab_mut() {
                if page < tab.total_pages {
                    tab.current_page = page;
                    Some(tab.current_page)
                } else {
                    None
                }
            } else {
                None
            };

            if let Some(p) = jump_page {
                app.page_input = (p + 1).to_string();
                if let Some(tab) = app.current_tab_mut() {
                    return scroll_to_page(tab, p);
                }
            }
            Task::none()
        }
        Message::PageInputChanged(s) => {
            app.page_input = s;
            Task::none()
        }
        Message::PageInputSubmitted => {
            if let Ok(page) = app.page_input.trim().parse::<usize>() {
                return app.update(Message::JumpToPage(page.saturating_sub(1)));
            }
            if let Some(tab) = app.current_tab() {
                app.page_input = (tab.current_page + 1).to_string();
            }
            Task::none()
        }
        _ => Task::none(),
    }
}
