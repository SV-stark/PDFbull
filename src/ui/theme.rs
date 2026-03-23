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
    let hex = hex.trim_start_matches('#');
    if hex.len() != 6 {
        return (0.0, 0.0, 0.0);
    }
    let r = f32::from(u8::from_str_radix(&hex[0..2], 16).unwrap_or(0)) / 255.0;
    let g = f32::from(u8::from_str_radix(&hex[2..4], 16).unwrap_or(0)) / 255.0;
    let b = f32::from(u8::from_str_radix(&hex[4..6], 16).unwrap_or(0)) / 255.0;
    (r, g, b)
}
