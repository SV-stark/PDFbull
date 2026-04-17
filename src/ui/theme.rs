use iced::Color;

pub const SIDEBAR_WIDTH: f32 = 240.0;
pub const THUMBNAIL_WIDTH: f32 = 180.0;
pub const THUMBNAIL_HEIGHT: f32 = 60.0;
pub const PAGE_SPACING: f32 = 20.0;
pub const PAGE_PADDING: f32 = 30.0;
pub const VIEWPORT_BUFFER: usize = 3;
pub const TOOLBAR_HEIGHT: f32 = 72.0;
pub const NAV_HEIGHT: f32 = 50.0;

// Centralized Design Tokens
pub const BORDER_RADIUS_SM: f32 = 4.0;
pub const BORDER_RADIUS_MD: f32 = 8.0;
pub const BORDER_RADIUS_LG: f32 = 12.0;
pub const BORDER_RADIUS_FULL: f32 = 999.0;

// Colors - Dark Neutrals
pub const COLOR_BG_APP: Color = Color::from_rgb(0.08, 0.08, 0.09); // Deepest
pub const COLOR_BG_HEADER: Color = Color::from_rgb(0.12, 0.12, 0.14);
pub const COLOR_BG_SIDEBAR: Color = Color::from_rgb(0.10, 0.10, 0.11);
pub const COLOR_BG_WIDGET: Color = Color::from_rgb(0.16, 0.16, 0.18);
pub const COLOR_BG_WIDGET_HOVER: Color = Color::from_rgb(0.20, 0.20, 0.22);

// Colors - Brand / Accent
pub const COLOR_ACCENT: Color = Color::from_rgb(0.25, 0.55, 1.0); // Vibrant Blue
pub const COLOR_ACCENT_DIM: Color = Color::from_rgb(0.25, 0.55, 1.0); // Simplified for now, can be alpha-fied in use

// Colors - Text
pub const COLOR_TEXT_PRIMARY: Color = Color::from_rgb(0.95, 0.95, 0.98);
pub const COLOR_TEXT_DIM: Color = Color::from_rgb(0.60, 0.60, 0.65);
pub const COLOR_TEXT_SECONDARY: Color = Color::from_rgb(0.45, 0.45, 0.48);

// Reusable Styles
pub fn button_ghost(
    _theme: &iced::Theme,
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    let base = iced::widget::button::Style {
        background: None,
        text_color: COLOR_TEXT_DIM,
        border: iced::Border {
            radius: BORDER_RADIUS_MD.into(),
            ..Default::default()
        },
        ..Default::default()
    };

    match status {
        iced::widget::button::Status::Hovered => iced::widget::button::Style {
            background: Some(COLOR_BG_WIDGET.into()),
            text_color: COLOR_TEXT_PRIMARY,
            ..base
        },
        iced::widget::button::Status::Pressed => iced::widget::button::Style {
            background: Some(COLOR_BG_HEADER.into()),
            text_color: COLOR_TEXT_PRIMARY,
            ..base
        },
        iced::widget::button::Status::Disabled => iced::widget::button::Style {
            text_color: COLOR_TEXT_SECONDARY,
            ..base
        },
        _ => base,
    }
}

pub fn button_tool(
    active: bool,
) -> impl Fn(&iced::Theme, iced::widget::button::Status) -> iced::widget::button::Style {
    move |_theme, status| {
        let base_bg = if active {
            Some(COLOR_ACCENT.into())
        } else {
            None
        };
        let base_text = if active { Color::WHITE } else { COLOR_TEXT_DIM };

        let base = iced::widget::button::Style {
            background: base_bg,
            text_color: base_text,
            border: iced::Border {
                radius: BORDER_RADIUS_MD.into(),
                ..Default::default()
            },
            ..Default::default()
        };

        match status {
            iced::widget::button::Status::Hovered if !active => iced::widget::button::Style {
                background: Some(COLOR_BG_WIDGET.into()),
                text_color: COLOR_TEXT_PRIMARY,
                ..base
            },
            iced::widget::button::Status::Pressed => iced::widget::button::Style {
                background: Some(COLOR_BG_HEADER.into()),
                ..base
            },
            _ => base,
        }
    }
}

pub fn input_field(_theme: &iced::Theme) -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: Some(COLOR_BG_HEADER.into()),
        border: iced::Border {
            radius: BORDER_RADIUS_MD.into(),
            width: 1.0,
            color: Color::from_rgb(0.2, 0.2, 0.22),
        },
        ..Default::default()
    }
}

pub fn hex_to_rgb(hex: &str) -> (f32, f32, f32) {
    let hex = hex.trim().trim_start_matches('#');
    if hex.len() != 6 {
        return (0.0, 0.0, 0.0);
    }
    let r = f32::from(u8::from_str_radix(&hex[0..2], 16).unwrap_or(0)) / 255.0;
    let g = f32::from(u8::from_str_radix(&hex[2..4], 16).unwrap_or(0)) / 255.0;
    let b = f32::from(u8::from_str_radix(&hex[4..6], 16).unwrap_or(0)) / 255.0;
    (r, g, b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hex_to_rgb_black() {
        let (r, g, b) = hex_to_rgb("#000000");
        assert_eq!(r, 0.0);
        assert_eq!(g, 0.0);
        assert_eq!(b, 0.0);
    }

    #[test]
    fn test_hex_to_rgb_white() {
        let (r, g, b) = hex_to_rgb("#FFFFFF");
        assert_eq!(r, 1.0);
        assert_eq!(g, 1.0);
        assert_eq!(b, 1.0);
    }

    #[test]
    fn test_hex_to_rgb_red() {
        let (r, g, b) = hex_to_rgb("#FF0000");
        assert!((r - 1.0).abs() < 0.01);
        assert_eq!(g, 0.0);
        assert_eq!(b, 0.0);
    }

    #[test]
    fn test_hex_to_rgb_green() {
        let (r, g, b) = hex_to_rgb("#00FF00");
        assert_eq!(r, 0.0);
        assert!((g - 1.0).abs() < 0.01);
        assert_eq!(b, 0.0);
    }

    #[test]
    fn test_hex_to_rgb_blue() {
        let (r, g, b) = hex_to_rgb("#0000FF");
        assert_eq!(r, 0.0);
        assert_eq!(g, 0.0);
        assert!((b - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_hex_to_rgb_without_hash() {
        let (r, g, b) = hex_to_rgb("FF0000");
        assert!((r - 1.0).abs() < 0.01);
        assert_eq!(g, 0.0);
        assert_eq!(b, 0.0);
    }

    #[test]
    fn test_hex_to_rgb_with_hash() {
        let (r, g, b) = hex_to_rgb("#AABBCC");
        assert!((r - 171.0 / 255.0).abs() < 0.01);
        assert!((g - 187.0 / 255.0).abs() < 0.01);
        assert!((b - 204.0 / 255.0).abs() < 0.01);
    }

    #[test]
    fn test_hex_to_rgb_invalid_short() {
        let (r, g, b) = hex_to_rgb("#FFF");
        assert_eq!(r, 0.0);
        assert_eq!(g, 0.0);
        assert_eq!(b, 0.0);
    }

    #[test]
    fn test_hex_to_rgb_invalid_empty() {
        let (r, g, b) = hex_to_rgb("");
        assert_eq!(r, 0.0);
        assert_eq!(g, 0.0);
        assert_eq!(b, 0.0);
    }

    #[test]
    fn test_hex_to_rgb_invalid_chars() {
        let (r, g, b) = hex_to_rgb("#GGHHII");
        assert_eq!(r, 0.0);
        assert_eq!(g, 0.0);
        assert_eq!(b, 0.0);
    }

    #[test]
    fn test_hex_to_rgb_lowercase() {
        let (r, g, b) = hex_to_rgb("#aabbcc");
        assert!((r - 170.0 / 255.0).abs() < 0.01);
        assert!((g - 187.0 / 255.0).abs() < 0.01);
        assert!((b - 204.0 / 255.0).abs() < 0.01);
    }

    #[test]
    fn test_hex_to_rgb_mixed_case() {
        let (r, g, b) = hex_to_rgb("#AaBbCc");
        assert!((r - 170.0 / 255.0).abs() < 0.01);
        assert!((g - 187.0 / 255.0).abs() < 0.01);
        assert!((b - 204.0 / 255.0).abs() < 0.01);
    }

    #[test]
    fn test_hex_to_rgb_spaces() {
        let (r, g, b) = hex_to_rgb("  #FF0000  ");
        assert!((r - 1.0).abs() < 0.01);
        assert_eq!(g, 0.0);
        assert_eq!(b, 0.0);
    }

    #[test]
    fn test_constants() {
        assert!(SIDEBAR_WIDTH > 0.0);
        assert!(THUMBNAIL_WIDTH > 0.0);
        assert!(THUMBNAIL_HEIGHT > 0.0);
        assert!(PAGE_SPACING >= 0.0);
        assert!(PAGE_PADDING >= 0.0);
        assert!(VIEWPORT_BUFFER > 0);
        assert!(TOOLBAR_HEIGHT > 0.0);
        assert!(NAV_HEIGHT > 0.0);
    }

    #[test]
    fn test_color_constants() {
        assert!(COLOR_BG_APP.r >= 0.0 && COLOR_BG_APP.r <= 1.0);
        assert!(COLOR_ACCENT.r >= 0.0 && COLOR_ACCENT.r <= 1.0);
        assert!(COLOR_TEXT_DIM.r >= 0.0 && COLOR_TEXT_DIM.r <= 1.0);
        assert!(COLOR_TEXT_PRIMARY.r >= 0.0 && COLOR_TEXT_PRIMARY.r <= 1.0);
    }
}
