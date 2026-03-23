use crate::app::PdfBullApp;
use crate::message::Message;
use crate::update::scroll_to_page;
use iced::Task;

pub fn handle_bookmark_message(app: &mut PdfBullApp, message: Message) -> Task<Message> {
    match message {
        Message::AddBookmark => {
            if let Some(tab) = app.current_tab_mut() {
                let page = tab.current_page;
                let label = format!("Page {}", page + 1);
                let bookmark = crate::models::PageBookmark {
                    page,
                    label,
                    created_at: time::OffsetDateTime::now_utc().unix_timestamp() as u64,
                };
                if !tab.bookmarks.iter().any(|b| b.page == page) {
                    tab.bookmarks.push(bookmark);
                }
            }
            Task::none()
        }
        Message::RemoveBookmark(idx) => {
            if let Some(tab) = app.current_tab_mut() {
                if idx < tab.bookmarks.len() {
                    tab.bookmarks.remove(idx);
                }
            }
            Task::none()
        }
        Message::JumpToBookmark(idx) => {
            let jump_page = if let Some(tab) = app.current_tab_mut() {
                if idx < tab.bookmarks.len() {
                    tab.current_page = tab.bookmarks[idx].page;
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
        _ => Task::none(),
    }
}
