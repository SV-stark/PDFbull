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
                    let has_dirty = app.tabs.iter().any(|t| t.annotations_dirty);
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
                iced::Event::Mouse(iced::mouse::Event::CursorMoved { position }) => {
                    app.cursor_position = Some(position);
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
                        Key::Named(iced::keyboard::key::Named::F1) => {
                            return app.update(Message::ToggleKeyboardHelp);
                        }
                        Key::Named(iced::keyboard::key::Named::Escape) if app.command_palette.is_open => {
                            app.command_palette.is_open = false;
                            return Task::none();
                        }
                        Key::Named(iced::keyboard::key::Named::ArrowDown) if app.command_palette.is_open => {
                            return app.update(Message::PaletteSelectNext);
                        }
                        Key::Named(iced::keyboard::key::Named::ArrowUp) if app.command_palette.is_open => {
                            return app.update(Message::PaletteSelectPrev);
                        }
                        Key::Named(iced::keyboard::key::Named::Enter) if app.command_palette.is_open => {
                            return app.update(Message::PaletteSubmit);
                        }
                        Key::Character(c) => match c.as_str() {
                            "k" if modifiers.command() => {
                                return app.update(Message::ToggleCommandPalette);
                            }
                            "p" if modifiers.command() && modifiers.shift() => {
                                return app.update(Message::ToggleCommandPalette);
                            }
                            "o" if modifiers.command() => return app.update(Message::OpenDocument),
                            "e" if modifiers.command() => return app.update(Message::ExportImage),
                            "p" if modifiers.command() && !app.tabs.is_empty() => {
                                return app.update(Message::Print);
                            }
                            "s" if modifiers.command() => {
                                return app.update(Message::SaveAnnotations);
                            }
                            "z" if modifiers.command() && modifiers.shift() => {
                                return app.update(Message::Redo);
                            }
                            "z" if modifiers.command() => return app.update(Message::Undo),
                            "y" if modifiers.command() => return app.update(Message::Redo),
                            "f" if modifiers.command() => { /* Search is handled in UI */ }
                            "0" if modifiers.command() => return app.update(Message::ResetZoom),
                            "=" | "+" if modifiers.command() => return app.update(Message::ZoomIn),
                            "-" if modifiers.command() => return app.update(Message::ZoomOut),
                            "w" if modifiers.command() && !app.tabs.is_empty() => {
                                return app.update(Message::CloseTab(app.active_tab));
                            }
                            "b" if modifiers.command() => {
                                return app.update(Message::ToggleSidebar);
                            }
                            "?" if modifiers.shift() => {
                                return app.update(Message::ToggleKeyboardHelp);
                            }
                            "t" if !modifiers.command() => {
                                return app.update(Message::SetAnnotationMode(Some(
                                    crate::models::PendingAnnotationKind::Text,
                                )));
                            }
                            _ => {}
                        },
                        Key::Named(iced::keyboard::key::Named::Escape)
                            if app.annotation_mode.is_some() || app.markup_active =>
                        {
                            app.markup_active = false;
                            return app.update(Message::SetAnnotationMode(None));
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
            Task::none()
        }
        Message::ToggleCommandPalette => {
            app.command_palette.is_open = !app.command_palette.is_open;
            if app.command_palette.is_open {
                app.command_palette.query.clear();
                app.command_palette.selected_index = 0;
            }
            Task::none()
        }
        Message::CommandPaletteQueryChanged(query) => {
            app.command_palette.query = query;
            app.command_palette.selected_index = 0;
            Task::none()
        }
        Message::PaletteSelectNext => {
            let all = app.build_palette_items();
            let filtered = crate::models::filter_palette_items(&app.command_palette.query, &all);
            if !filtered.is_empty() {
                app.command_palette.selected_index =
                    (app.command_palette.selected_index + 1) % filtered.len();
            }
            Task::none()
        }
        Message::PaletteSelectPrev => {
            let all = app.build_palette_items();
            let filtered = crate::models::filter_palette_items(&app.command_palette.query, &all);
            if !filtered.is_empty() {
                if app.command_palette.selected_index == 0 {
                    app.command_palette.selected_index = filtered.len().saturating_sub(1);
                } else {
                    app.command_palette.selected_index -= 1;
                }
            }
            Task::none()
        }
        Message::PaletteSubmit => {
            let all = app.build_palette_items();
            let filtered = crate::models::filter_palette_items(&app.command_palette.query, &all);
            if let Some(item) = filtered.get(app.command_palette.selected_index) {
                let action = item.action.clone();
                app.command_palette.is_open = false;
                return app.update(Message::ExecutePaletteAction(action));
            }
            Task::none()
        }
        Message::ExecutePaletteAction(action) => {
            use crate::models::CommandAction;
            match action {
                CommandAction::NextPage => app.update(Message::NextPage),
                CommandAction::PrevPage => app.update(Message::PrevPage),
                CommandAction::ZoomIn => app.update(Message::ZoomIn),
                CommandAction::ZoomOut => app.update(Message::ZoomOut),
                CommandAction::ResetZoom => app.update(Message::ResetZoom),
                CommandAction::ToggleTheme => {
                    let next_theme = match app.settings.theme {
                        crate::models::AppTheme::Dark => crate::models::AppTheme::Light,
                        _ => crate::models::AppTheme::Dark,
                    };
                    app.settings.theme = next_theme;
                    Task::none()
                }
                CommandAction::ToggleSidebar => app.update(Message::ToggleSidebar),
                CommandAction::ToggleFullscreen => app.update(Message::ToggleFullscreen),
                CommandAction::ToggleMetadata => app.update(Message::ToggleMetadata),
                CommandAction::ToggleKeyboardHelp => app.update(Message::ToggleKeyboardHelp),
                CommandAction::RotateClockwise => app.update(Message::RotateClockwise),
                CommandAction::RotateCounterClockwise => app.update(Message::RotateCounterClockwise),
                CommandAction::ExtractText => app.update(Message::ExtractTextToClipboard),
                CommandAction::TriggerOcrLatin => {
                    app.update(Message::TriggerOcrCurrentPage(crate::ocr::OcrScript::Latin))
                }
                CommandAction::TriggerOcrDevanagari => app.update(Message::TriggerOcrCurrentPage(
                    crate::ocr::OcrScript::Devanagari,
                )),
                CommandAction::ExportMarkdown => app.update(Message::ExportDocumentMarkdown),
                CommandAction::ExportHtml => app.update(Message::ExportDocumentHtml),
                CommandAction::ExportTxt => app.update(Message::ExportDocumentTxt),
                CommandAction::ExportImage => app.update(Message::ExportImage),
                CommandAction::Print => app.update(Message::Print),
                CommandAction::OptimizePDF => app.update(Message::OptimizePDF),
                CommandAction::OpenSettings => app.update(Message::OpenSettings),
                CommandAction::NewDocument => app.update(Message::CreateBlankDocument),
                CommandAction::OpenFile => app.update(Message::OpenDocument),
                CommandAction::JumpToPage(page) => app.update(Message::JumpToPage(page)),
            }
        }
        Message::LinkClicked(link) => {
            if let Some(url) = link.url {
                let url_lower = url.to_lowercase();
                if url_lower.starts_with("http://")
                    || url_lower.starts_with("https://")
                    || url_lower.starts_with("mailto:")
                {
                    let _ = open::that(&url);
                }
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
