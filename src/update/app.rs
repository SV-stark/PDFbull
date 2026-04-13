use crate::app::PdfBullApp;
use crate::message::Message;
use crate::storage;
use iced::Task;

pub fn handle_app_message(app: &mut PdfBullApp, message: Message) -> Task<Message> {
    match message {
        Message::ResetZoom => {
            if let Some(tab) = app.current_tab_mut() {
                tab.zoom = 1.0;
            }
            app.render_visible_pages()
        }
        Message::OpenSettings => {
            app.show_settings = true;
            Task::none()
        }
        Message::CloseSettings => {
            app.show_settings = false;
            Task::none()
        }
        Message::SaveSettings(settings) => {
            app.settings = settings;
            storage::save_settings(&app.settings);
            Task::none()
        }
        Message::ToggleSidebar => {
            app.show_sidebar = !app.show_sidebar;
            let target = if app.show_sidebar { 280.0 } else { 0.0 };
            app.sidebar_animation
                .go_mut(target, std::time::Instant::now());
            Task::none()
        }
        Message::ToggleFormsSidebar => {
            app.show_forms_sidebar = !app.show_forms_sidebar;
            if app.show_forms_sidebar {
                return app.update(Message::LoadFormFields);
            }
            Task::none()
        }
        Message::ToggleFullscreen => {
            app.is_fullscreen = !app.is_fullscreen;
            Task::none()
        }
        Message::ToggleKeyboardHelp => {
            app.show_keyboard_help = !app.show_keyboard_help;
            Task::none()
        }
        Message::RotateClockwise => {
            if let Some(tab) = app.current_tab_mut() {
                tab.rotation = (tab.rotation + 90) % 360;
                tab.view_state.rendered_pages.clear();
            }
            app.render_visible_pages()
        }
        Message::RotateCounterClockwise => {
            if let Some(tab) = app.current_tab_mut() {
                tab.rotation = (tab.rotation - 90 + 360) % 360;
                tab.view_state.rendered_pages.clear();
            }
            app.render_visible_pages()
        }
        Message::ClearRecentFiles => {
            app.recent_files.clear();
            crate::storage::save_recent_files(&app.recent_files);
            Task::none()
        }
        Message::ToggleMetadata => {
            app.show_metadata = !app.show_metadata;
            Task::none()
        }
        _ => Task::none(),
    }
}
