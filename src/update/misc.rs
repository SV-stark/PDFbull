use crate::app::PdfBullApp;
use crate::message::Message;
use iced::Task;

pub fn handle_misc_message(app: &mut PdfBullApp, message: Message) -> Task<Message> {
    match message {
        Message::EngineInitialized(state) => {
            app.engine = Some(state);
            Task::none()
        }
        Message::Error(e) => {
            tracing::error!("Error: {e}");
            app.status_message = Some(format!("Error: {e}"));
            Task::none()
        }
        Message::ClearStatus => {
            app.status_message = None;
            Task::none()
        }
        Message::IcedEvent(event) => {
            match event {
                iced::Event::Window(iced::window::Event::CloseRequested) => {
                    let has_dirty = app.tabs.iter().any(|t| !t.annotations.is_empty());
                    if has_dirty {
                        return Task::perform(
                            async move {
                                rfd::AsyncMessageDialog::new()
                                    .set_level(rfd::MessageLevel::Warning)
                                    .set_title("Unsaved Annotations")
                                    .set_description("You have annotations that haven't been saved to a PDF.\n\nQuitting will lose them. Are you sure you want to quit?")
                                    .set_buttons(rfd::MessageButtons::YesNo)
                                    .show()
                                    .await == rfd::MessageDialogResult::Yes
                            },
                            |yes| {
                                if yes {
                                    Message::ForceQuit
                                } else {
                                    Message::ClearStatus
                                }
                            },
                        );
                    }
                    app.save_session_and_recent();
                    return iced::exit();
                }
                iced::Event::Window(iced::window::Event::FileDropped(path)) => {
                    return app.update(Message::OpenFile(path));
                }
                iced::Event::Mouse(iced::mouse::Event::WheelScrolled { delta }) => {
                    use iced::mouse::ScrollDelta;
                    let modifiers = app.modifiers;
                    if modifiers.control() && !app.tabs.is_empty() {
                        match delta {
                            ScrollDelta::Lines { y, .. } | ScrollDelta::Pixels { y, .. } => {
                                if y > 0.0 {
                                    return app.update(Message::ZoomIn);
                                } else if y < 0.0 {
                                    return app.update(Message::ZoomOut);
                                }
                            }
                        }
                    }
                }
                iced::Event::Keyboard(iced::keyboard::Event::ModifiersChanged(modifiers)) => {
                    app.modifiers = modifiers;
                }
                iced::Event::Keyboard(iced::keyboard::Event::KeyPressed {
                    key, modifiers, ..
                }) => {
                    use iced::keyboard::Key;

                    match key {
                        Key::Named(iced::keyboard::key::Named::F11) => {
                            return app.update(Message::ToggleFullscreen);
                        }
                        Key::Character(c) => match c.as_str() {
                            "o" if modifiers.command() => return app.update(Message::OpenDocument),
                            "p" if modifiers.command() => {
                                if !app.tabs.is_empty() {
                                    return app.update(Message::Print);
                                }
                            }
                            "s" if modifiers.command() => {
                                return app.update(Message::SaveAnnotations)
                            }
                            "z" if modifiers.command() && modifiers.shift() => {
                                return app.update(Message::Redo)
                            }
                            "z" if modifiers.command() => return app.update(Message::Undo),
                            "y" if modifiers.command() => return app.update(Message::Redo),
                            "f" if modifiers.command() => { /* Search is handled in UI */ }
                            "0" if modifiers.command() => return app.update(Message::ResetZoom),
                            "=" | "+" if modifiers.command() => return app.update(Message::ZoomIn),
                            "-" if modifiers.command() => return app.update(Message::ZoomOut),
                            "w" if modifiers.command() => {
                                if !app.tabs.is_empty() {
                                    return app.update(Message::CloseTab(app.active_tab));
                                }
                            }
                            "b" if modifiers.command() => {
                                return app.update(Message::ToggleSidebar)
                            }
                            "?" if modifiers.shift() => {
                                return app.update(Message::ToggleKeyboardHelp)
                            }
                            _ => {}
                        },
                        Key::Named(iced::keyboard::key::Named::Escape) => {
                            if app.annotation_mode.is_some() {
                                return app.update(Message::SetAnnotationMode(None));
                            }
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
            Task::none()
        }
        Message::LinkClicked(link) => {
            if let Some(url) = link.url {
                let _ = open::that(&url);
            } else if let Some(dest_page) = link.destination_page {
                return app.update(Message::JumpToPage(dest_page));
            }
            Task::none()
        }
        Message::ForceQuit => {
            app.save_session_and_recent();
            iced::exit()
        }
        _ => Task::none(),
    }
}
