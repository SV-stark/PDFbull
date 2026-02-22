use iced::widget::{button, column, container, text, Space};
use iced::{Element, Length};

pub fn keyboard_help_view(_app: &crate::PdfBullApp) -> Element<crate::Message> {
    use iced::Alignment;

    let shortcuts = column![
        text("Keyboard Shortcuts").size(24),
        Space::new().height(Length::Fixed(20.0)),
        text("Navigation:").size(16),
        text("Arrow Up/Down - Scroll"),
        text("Page Up/Down - Next/Prev Page"),
        text("Home/End - First/Last Page"),
        Space::new().height(Length::Fixed(10.0)),
        text("View:").size(16),
        text("Ctrl + 0 - Reset Zoom"),
        text("Ctrl + + - Zoom In"),
        text("Ctrl + - - Zoom Out"),
        text("F11 - Toggle Fullscreen"),
        Space::new().height(Length::Fixed(10.0)),
        text("Document:").size(16),
        text("Ctrl + O - Open File"),
        text("Ctrl + S - Save/Export"),
        text("Ctrl + D - Add Bookmark"),
        text("Ctrl + F - Search"),
        Space::new().height(Length::Fixed(10.0)),
        text("Press ? or F1 to close this help").size(12),
    ]
    .padding(30)
    .align_x(Alignment::Center);

    container(column![
        button("Close")
            .on_press(crate::Message::ToggleKeyboardHelp)
            .padding(10),
        shortcuts,
    ])
    .width(Length::Fill)
    .height(Length::Fill)
    .center_x(Length::Fill)
    .center_y(Length::Fill)
    .into()
}
