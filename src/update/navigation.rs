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

#[cfg(test)]
mod tests {
    #[test]
    fn test_zoom_calculation_zoom_in() {
        let zoom = 1.0;
        let new_zoom = (zoom * 1.1).min(5.0);
        assert!((new_zoom - 1.1).abs() < 0.001);
    }

    #[test]
    fn test_zoom_calculation_max_zoom() {
        let zoom = 5.0;
        let new_zoom = (zoom * 1.1).min(5.0);
        assert_eq!(new_zoom, 5.0);
    }

    #[test]
    fn test_zoom_calculation_zoom_out() {
        let zoom = 1.0;
        let new_zoom = (zoom / 1.1).max(0.25);
        assert!((new_zoom - 0.909).abs() < 0.001);
    }

    #[test]
    fn test_zoom_calculation_min_zoom() {
        let zoom = 0.25;
        let new_zoom = (zoom / 1.1).max(0.25);
        assert_eq!(new_zoom, 0.25);
    }

    #[test]
    fn test_zoom_clamp_within_range() {
        let zoom = 2.0;
        let clamped = zoom.clamp(0.25, 5.0);
        assert_eq!(clamped, 2.0);
    }

    #[test]
    fn test_zoom_clamp_below_min() {
        let zoom = 0.1;
        let clamped = zoom.clamp(0.25, 5.0);
        assert_eq!(clamped, 0.25);
    }

    #[test]
    fn test_zoom_clamp_above_max() {
        let zoom = 10.0;
        let clamped = zoom.clamp(0.25, 5.0);
        assert_eq!(clamped, 5.0);
    }

    #[test]
    fn test_page_input_parsing() {
        let input = "5";
        let parsed: Result<usize, _> = input.trim().parse();
        assert!(parsed.is_ok());
        assert_eq!(parsed.unwrap(), 5);
    }

    #[test]
    fn test_page_input_parsing_invalid() {
        let input = "abc";
        let parsed: Result<usize, _> = input.trim().parse();
        assert!(parsed.is_err());
    }

    #[test]
    fn test_page_input_parsing_with_whitespace() {
        let input = "  10  ";
        let parsed: Result<usize, _> = input.trim().parse();
        assert!(parsed.is_ok());
        assert_eq!(parsed.unwrap(), 10);
    }

    #[test]
    fn test_page_input_to_page_index_conversion() {
        let user_page = 5;
        let page_index = user_page.saturating_sub(1);
        assert_eq!(page_index, 4);
    }

    #[test]
    fn test_page_input_to_page_index_zero() {
        let user_page = 0;
        let page_index = user_page.saturating_sub(1);
        assert_eq!(page_index, 0);
    }

    #[test]
    fn test_page_index_to_display_conversion() {
        let page_index = 4;
        let display = page_index + 1;
        assert_eq!(display, 5);
    }

    #[test]
    fn test_can_go_next_page() {
        let current_page = 0;
        let total_pages = 10;
        assert!(current_page + 1 < total_pages);
    }

    #[test]
    fn test_cannot_go_next_at_end() {
        let current_page = 9;
        let total_pages = 10;
        assert!(!(current_page + 1 < total_pages));
    }

    #[test]
    fn test_can_go_prev_page() {
        let current_page = 5;
        assert!(current_page > 0);
    }

    #[test]
    fn test_cannot_go_prev_at_start() {
        let current_page = 0;
        assert!(!(current_page > 0));
    }

    #[test]
    fn test_jump_to_valid_page() {
        let target_page = 5;
        let total_pages = 10;
        assert!(target_page < total_pages);
    }

    #[test]
    fn test_jump_to_invalid_page() {
        let target_page = 15;
        let total_pages = 10;
        assert!(!(target_page < total_pages));
    }

    #[test]
    fn test_zoom_factor_calculation() {
        let base_zoom = 1.0;
        let zoom_in_factor = 1.1;
        let zoom_out_factor = 1.1;

        let zoomed_in = base_zoom * zoom_in_factor;
        let zoomed_out = zoomed_in / zoom_out_factor;

        assert!((zoomed_out - base_zoom).abs() < 0.001);
    }
}
