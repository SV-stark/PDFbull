use iced::Color;

pub const SIDEBAR_WIDTH: f32 = 180.0;
pub const THUMBNAIL_WIDTH: f32 = 120.0;
pub const THUMBNAIL_HEIGHT: f32 = 40.0;
pub const PAGE_SPACING: f32 = 10.0;
pub const PAGE_PADDING: f32 = 10.0;
pub const VIEWPORT_BUFFER: usize = 2;
pub const TOOLBAR_HEIGHT: f32 = 60.0;
pub const NAV_HEIGHT: f32 = 45.0;

pub const COLOR_BG_DARK: Color = Color::from_rgb(0.1, 0.1, 0.11);
pub const COLOR_BG_LIGHT: Color = Color::from_rgb(0.95, 0.95, 0.96);
pub const COLOR_ACCENT: Color = Color::from_rgb(0.23, 0.51, 0.96);
pub const COLOR_TEXT_DIM: Color = Color::from_rgb(0.58, 0.58, 0.62);

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
        assert!(COLOR_BG_DARK.r >= 0.0 && COLOR_BG_DARK.r <= 1.0);
        assert!(COLOR_BG_DARK.g >= 0.0 && COLOR_BG_DARK.g <= 1.0);
        assert!(COLOR_BG_DARK.b >= 0.0 && COLOR_BG_DARK.b <= 1.0);

        assert!(COLOR_BG_LIGHT.r >= 0.0 && COLOR_BG_LIGHT.r <= 1.0);
        assert!(COLOR_ACCENT.r >= 0.0 && COLOR_ACCENT.r <= 1.0);
        assert!(COLOR_TEXT_DIM.r >= 0.0 && COLOR_TEXT_DIM.r <= 1.0);
    }
}
