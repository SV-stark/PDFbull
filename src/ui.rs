use crate::ui_document::document_view;
use crate::ui_keyboard_help::keyboard_help_view;
use crate::ui_settings::settings_view;
use crate::ui_welcome::welcome_view;
use crate::PdfBullApp;
use iced::Element;

pub fn view(app: &PdfBullApp) -> Element<crate::Message> {
    if app.show_keyboard_help {
        return keyboard_help_view(app);
    }

    if app.show_settings {
        return settings_view(app);
    }

    if app.tabs.is_empty() {
        return welcome_view(app);
    }

    document_view(app)
}
