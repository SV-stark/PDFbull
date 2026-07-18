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
            let cursor_pos = app.cursor_position;
            let mut tasks = Vec::new();
            if let Some(tab) = app.current_tab_mut() {
                let old_zoom = tab.zoom;
                let new_zoom = (old_zoom * 1.1).min(5.0);
                if (new_zoom - old_zoom).abs() > 0.001 {
                    tab.zoom = new_zoom;
                    tab.view_state.rendered_pages.clear();

                    let relative_y = if let Some(pos) = cursor_pos {
                        let toolbar_height = 50.0;
                        (pos.y - toolbar_height).max(0.0)
                    } else {
                        tab.view_state.viewport_height / 2.0
                    };
                    let old_scroll_y = tab.view_state.viewport_y;
                    let factor = new_zoom / old_zoom;
                    let new_scroll_y = old_scroll_y * factor + (factor - 1.0) * relative_y;
                    let clamped_scroll_y = new_scroll_y.max(0.0);

                    tasks.push(crate::update::scroll_to_y(clamped_scroll_y));
                }
            }
            tasks.push(app.render_visible_pages());
            Task::batch(tasks)
        }
        Message::ZoomOut => {
            let cursor_pos = app.cursor_position;
            let mut tasks = Vec::new();
            if let Some(tab) = app.current_tab_mut() {
                let old_zoom = tab.zoom;
                let new_zoom = (old_zoom / 1.1).max(0.25);
                if (new_zoom - old_zoom).abs() > 0.001 {
                    tab.zoom = new_zoom;
                    tab.view_state.rendered_pages.clear();

                    let relative_y = if let Some(pos) = cursor_pos {
                        let toolbar_height = 50.0;
                        (pos.y - toolbar_height).max(0.0)
                    } else {
                        tab.view_state.viewport_height / 2.0
                    };
                    let old_scroll_y = tab.view_state.viewport_y;
                    let factor = new_zoom / old_zoom;
                    let new_scroll_y = old_scroll_y * factor + (factor - 1.0) * relative_y;
                    let clamped_scroll_y = new_scroll_y.max(0.0);

                    tasks.push(crate::update::scroll_to_y(clamped_scroll_y));
                }
            }
            tasks.push(app.render_visible_pages());
            Task::batch(tasks)
        }
        Message::SetZoom(zoom) => {
            let cursor_pos = app.cursor_position;
            let mut tasks = Vec::new();
            if let Some(tab) = app.current_tab_mut() {
                let old_zoom = tab.zoom;
                let new_zoom = zoom.clamp(0.25, 5.0);
                if (new_zoom - old_zoom).abs() > 0.001 {
                    tab.zoom = new_zoom;
                    tab.view_state.rendered_pages.clear();

                    let relative_y = if let Some(pos) = cursor_pos {
                        let toolbar_height = 50.0;
                        (pos.y - toolbar_height).max(0.0)
                    } else {
                        tab.view_state.viewport_height / 2.0
                    };
                    let old_scroll_y = tab.view_state.viewport_y;
                    let factor = new_zoom / old_zoom;
                    let new_scroll_y = old_scroll_y * factor + (factor - 1.0) * relative_y;
                    let clamped_scroll_y = new_scroll_y.max(0.0);

                    tasks.push(crate::update::scroll_to_y(clamped_scroll_y));
                }
            }
            tasks.push(app.render_visible_pages());
            Task::batch(tasks)
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

#[cfg(test)]
#[allow(clippy::float_cmp, clippy::nonminimal_bool)]
mod tests {
    use super::*;
    use crate::models::DocumentTab;

    fn setup_test_app() -> PdfBullApp {
        let mut app = PdfBullApp::default();
        app.loaded = true;
        let mut tab = DocumentTab::new(std::path::PathBuf::from("test.pdf"));
        tab.total_pages = 10;
        tab.page_heights = vec![800.0; 10];
        app.tabs.push(tab);
        app.active_tab = 0;
        app
    }

    #[test]
    fn test_can_go_next_page() {
        let mut app = setup_test_app();
        app.tabs[0].current_page = 0;
        let _ = handle_nav_message(&mut app, Message::NextPage);
        assert_eq!(app.tabs[0].current_page, 1);
        assert_eq!(app.page_input, "2");
    }

    #[test]
    fn test_cannot_go_next_at_end() {
        let mut app = setup_test_app();
        app.tabs[0].current_page = 9;
        app.page_input = "10".to_string();
        let _ = handle_nav_message(&mut app, Message::NextPage);
        assert_eq!(app.tabs[0].current_page, 9);
        assert_eq!(app.page_input, "10");
    }

    #[test]
    fn test_can_go_prev_page() {
        let mut app = setup_test_app();
        app.tabs[0].current_page = 5;
        app.page_input = "6".to_string();
        let _ = handle_nav_message(&mut app, Message::PrevPage);
        assert_eq!(app.tabs[0].current_page, 4);
        assert_eq!(app.page_input, "5");
    }

    #[test]
    fn test_cannot_go_prev_at_start() {
        let mut app = setup_test_app();
        app.tabs[0].current_page = 0;
        let _ = handle_nav_message(&mut app, Message::PrevPage);
        assert_eq!(app.tabs[0].current_page, 0);
    }

    #[test]
    fn test_zoom_calculation_zoom_in() {
        let mut app = setup_test_app();
        app.tabs[0].zoom = 1.0;
        let _ = handle_nav_message(&mut app, Message::ZoomIn);
        assert!((app.tabs[0].zoom - 1.1).abs() < 0.001);
    }

    #[test]
    fn test_zoom_calculation_max_zoom() {
        let mut app = setup_test_app();
        app.tabs[0].zoom = 5.0;
        let _ = handle_nav_message(&mut app, Message::ZoomIn);
        assert_eq!(app.tabs[0].zoom, 5.0);
    }

    #[test]
    fn test_zoom_calculation_zoom_out() {
        let mut app = setup_test_app();
        app.tabs[0].zoom = 1.0;
        let _ = handle_nav_message(&mut app, Message::ZoomOut);
        assert!((app.tabs[0].zoom - 0.909).abs() < 0.001);
    }

    #[test]
    fn test_zoom_calculation_min_zoom() {
        let mut app = setup_test_app();
        app.tabs[0].zoom = 0.25;
        let _ = handle_nav_message(&mut app, Message::ZoomOut);
        assert_eq!(app.tabs[0].zoom, 0.25);
    }

    #[test]
    fn test_zoom_clamp_within_range() {
        let mut app = setup_test_app();
        let _ = handle_nav_message(&mut app, Message::SetZoom(2.0));
        assert_eq!(app.tabs[0].zoom, 2.0);
    }

    #[test]
    fn test_zoom_clamp_below_min() {
        let mut app = setup_test_app();
        let _ = handle_nav_message(&mut app, Message::SetZoom(0.1));
        assert_eq!(app.tabs[0].zoom, 0.25);
    }

    #[test]
    fn test_zoom_clamp_above_max() {
        let mut app = setup_test_app();
        let _ = handle_nav_message(&mut app, Message::SetZoom(10.0));
        assert_eq!(app.tabs[0].zoom, 5.0);
    }

    #[test]
    fn test_jump_to_valid_page() {
        let mut app = setup_test_app();
        let _ = handle_nav_message(&mut app, Message::JumpToPage(5));
        assert_eq!(app.tabs[0].current_page, 5);
        assert_eq!(app.page_input, "6");
    }

    #[test]
    fn test_jump_to_invalid_page() {
        let mut app = setup_test_app();
        app.tabs[0].current_page = 2;
        let _ = handle_nav_message(&mut app, Message::JumpToPage(15));
        assert_eq!(app.tabs[0].current_page, 2);
    }

    #[test]
    fn test_page_input_parsing() {
        let mut app = setup_test_app();
        app.page_input = "5".to_string();
        let _ = handle_nav_message(&mut app, Message::PageInputSubmitted);
        assert_eq!(app.tabs[0].current_page, 4);
    }

    #[test]
    fn test_page_input_parsing_invalid() {
        let mut app = setup_test_app();
        app.tabs[0].current_page = 2;
        app.page_input = "abc".to_string();
        let _ = handle_nav_message(&mut app, Message::PageInputSubmitted);
        assert_eq!(app.tabs[0].current_page, 2);
        assert_eq!(app.page_input, "3");
    }

    #[test]
    fn test_page_input_parsing_with_whitespace() {
        let mut app = setup_test_app();
        app.page_input = "  8  ".to_string();
        let _ = handle_nav_message(&mut app, Message::PageInputSubmitted);
        assert_eq!(app.tabs[0].current_page, 7);
    }

    #[test]
    fn test_page_input_to_page_index_zero() {
        let mut app = setup_test_app();
        app.page_input = "0".to_string();
        let _ = handle_nav_message(&mut app, Message::PageInputSubmitted);
        assert_eq!(app.tabs[0].current_page, 0);
    }
}
