//! C FFI for fz_font - MuPDF compatible font handling
//!
//! Provides FFI bindings for font loading and glyph operations.

use super::{Handle, HandleStore, safe_helpers};
use crate::fitz::font::Font;
use std::sync::LazyLock;

/// Font storage
pub static FONTS: LazyLock<HandleStore<Font>> = LazyLock::new(HandleStore::default);

/// Create a new font
///
/// # Safety
/// Caller must ensure name is a valid null-terminated C string.
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_font(
    _ctx: Handle,
    name: *const std::ffi::c_char,
    _is_bold: i32,
    _is_italic: i32,
    _font_file: Handle,
) -> Handle {
    let font_name = match safe_helpers::c_str_to_str(name) {
        Some(s) => s,
        None => return 0,
    };

    let font = Font::new(font_name);
    FONTS.insert(font)
}

/// Create a new font from data
///
/// # Safety
/// Caller must ensure data points to readable memory of at least len bytes.
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_font_from_memory(
    _ctx: Handle,
    name: *const std::ffi::c_char,
    data: *const u8,
    len: i32,
    index: i32,
    _use_glyph_bbox: i32,
) -> Handle {
    if data.is_null() || len <= 0 {
        return 0;
    }

    let font_name = safe_helpers::c_str_to_str(name).unwrap_or("Unknown");

    // Read font data
    let font_data = match safe_helpers::copy_from_ptr(data, len as usize) {
        Some(data) => data,
        None => return 0,
    };

    // Create font from data
    let font = Font::from_data(font_name, &font_data, index as usize);
    match font {
        Ok(f) => FONTS.insert(f),
        Err(_) => 0,
    }
}

/// Create a new font from file
///
/// # Safety
/// Caller must ensure path is a valid null-terminated C string.
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_font_from_file(
    _ctx: Handle,
    name: *const std::ffi::c_char,
    path: *const std::ffi::c_char,
    index: i32,
    _use_glyph_bbox: i32,
) -> Handle {
    let path_str = match safe_helpers::c_str_to_str(path) {
        Some(s) => s,
        None => return 0,
    };

    let font_name = safe_helpers::c_str_to_str(name).unwrap_or("Unknown");

    // Read font file
    match std::fs::read(path_str) {
        Ok(data) => match Font::from_data(font_name, &data, index as usize) {
            Ok(f) => FONTS.insert(f),
            Err(_) => 0,
        },
        Err(_) => 0,
    }
}

/// Keep (increment ref) font
#[unsafe(no_mangle)]
pub extern "C" fn fz_keep_font(_ctx: Handle, font: Handle) -> Handle {
    FONTS.keep(font)
}

/// Drop font reference
#[unsafe(no_mangle)]
pub extern "C" fn fz_drop_font(_ctx: Handle, font: Handle) {
    let _ = FONTS.remove(font);
}

/// Get font name
///
/// # Safety
/// Caller must ensure buf points to writable memory of at least 64 bytes.
#[unsafe(no_mangle)]
pub extern "C" fn fz_font_name(_ctx: Handle, font: Handle, buf: *mut std::ffi::c_char, size: i32) {
    if let Some(f) = FONTS.get(font) {
        if let Ok(guard) = f.lock() {
            let name = guard.name();
            safe_helpers::str_to_c_buffer(name, buf, size);
        }
    }
}

/// Check if font is bold
#[unsafe(no_mangle)]
pub extern "C" fn fz_font_is_bold(_ctx: Handle, font: Handle) -> i32 {
    if let Some(f) = FONTS.get(font) {
        if let Ok(guard) = f.lock() {
            return i32::from(guard.is_bold());
        }
    }
    0
}

/// Check if font is italic
#[unsafe(no_mangle)]
pub extern "C" fn fz_font_is_italic(_ctx: Handle, font: Handle) -> i32 {
    if let Some(f) = FONTS.get(font) {
        if let Ok(guard) = f.lock() {
            return i32::from(guard.is_italic());
        }
    }
    0
}

/// Check if font is serif
#[unsafe(no_mangle)]
pub extern "C" fn fz_font_is_serif(_ctx: Handle, font: Handle) -> i32 {
    if let Some(f) = FONTS.get(font) {
        if let Ok(guard) = f.lock() {
            return i32::from(guard.is_serif());
        }
    }
    0
}

/// Check if font is monospaced
#[unsafe(no_mangle)]
pub extern "C" fn fz_font_is_monospaced(_ctx: Handle, font: Handle) -> i32 {
    if let Some(f) = FONTS.get(font) {
        if let Ok(guard) = f.lock() {
            return i32::from(guard.is_monospace());
        }
    }
    0
}

/// Encode character to glyph ID
#[unsafe(no_mangle)]
pub extern "C" fn fz_encode_character(_ctx: Handle, font: Handle, unicode: i32) -> i32 {
    if let Some(f) = FONTS.get(font) {
        if let Ok(guard) = f.lock() {
            return guard.encode_character(unicode as u32) as i32;
        }
    }
    0
}

/// Encode character with fallback
#[unsafe(no_mangle)]
pub extern "C" fn fz_encode_character_with_fallback(
    _ctx: Handle,
    font: Handle,
    unicode: i32,
    _script: i32,
    _language: i32,
    out_font: *mut Handle,
) -> i32 {
    if let Some(f) = FONTS.get(font) {
        if let Ok(guard) = f.lock() {
            let glyph = guard.encode_character(unicode as u32);

            // Set output font to same font
            safe_helpers::write_ptr(font, out_font);

            return glyph as i32;
        }
    }
    0
}

/// Get glyph advance width
#[unsafe(no_mangle)]
pub extern "C" fn fz_advance_glyph(_ctx: Handle, font: Handle, glyph: i32, _wmode: i32) -> f32 {
    if let Some(f) = FONTS.get(font) {
        if let Ok(guard) = f.lock() {
            return guard.glyph_advance(glyph as u16);
        }
    }
    0.0
}

/// Get glyph bounding box
#[unsafe(no_mangle)]
pub extern "C" fn fz_bound_glyph(
    _ctx: Handle,
    font: Handle,
    glyph: i32,
    _transform: super::geometry::fz_matrix,
) -> super::geometry::fz_rect {
    if let Some(f) = FONTS.get(font) {
        if let Ok(guard) = f.lock() {
            let bbox = guard.glyph_bbox(glyph as u16);
            return super::geometry::fz_rect {
                x0: bbox.x0,
                y0: bbox.y0,
                x1: bbox.x1,
                y1: bbox.y1,
            };
        }
    }
    super::geometry::fz_rect {
        x0: 0.0,
        y0: 0.0,
        x1: 0.0,
        y1: 0.0,
    }
}

/// Get font bbox
#[unsafe(no_mangle)]
pub extern "C" fn fz_font_bbox(_ctx: Handle, font: Handle) -> super::geometry::fz_rect {
    if let Some(f) = FONTS.get(font) {
        if let Ok(guard) = f.lock() {
            let bbox = guard.bbox();
            return super::geometry::fz_rect {
                x0: bbox.x0,
                y0: bbox.y0,
                x1: bbox.x1,
                y1: bbox.y1,
            };
        }
    }
    super::geometry::fz_rect {
        x0: 0.0,
        y0: 0.0,
        x1: 0.0,
        y1: 0.0,
    }
}

/// Outline glyph (extract vector path)
#[unsafe(no_mangle)]
pub extern "C" fn fz_outline_glyph(
    _ctx: Handle,
    font: Handle,
    glyph: i32,
    _transform: super::geometry::fz_matrix,
) -> Handle {
    if let Some(f) = FONTS.get(font) {
        if let Ok(guard) = f.lock() {
            let path = guard.outline_glyph(glyph as u16);
            return super::path::PATHS.insert(path);
        }
    }
    0
}

/// Check if a font is valid
#[unsafe(no_mangle)]
pub extern "C" fn fz_font_is_valid(_ctx: Handle, font: Handle) -> i32 {
    if FONTS.get(font).is_some() { 1 } else { 0 }
}

/// Clone a font (increase ref count)
#[unsafe(no_mangle)]
pub extern "C" fn fz_clone_font(_ctx: Handle, font: Handle) -> Handle {
    fz_keep_font(_ctx, font)
}

/// Get font ascender height
#[unsafe(no_mangle)]
pub extern "C" fn fz_font_ascender(_ctx: Handle, font: Handle) -> f32 {
    if FONTS.get(font).is_some() {
        return 0.8; // Default ascender (80% of em-square)
    }
    0.0
}

/// Get font descender height
#[unsafe(no_mangle)]
pub extern "C" fn fz_font_descender(_ctx: Handle, font: Handle) -> f32 {
    if FONTS.get(font).is_some() {
        return -0.2; // Default descender (-20% of em-square)
    }
    0.0
}

/// Get glyph name
#[unsafe(no_mangle)]
pub extern "C" fn fz_glyph_name(
    _ctx: Handle,
    _font: Handle,
    glyph: i32,
    buf: *mut std::ffi::c_char,
    size: i32,
) {
    if buf.is_null() || size <= 0 {
        return;
    }

    // Generate default glyph name
    let name = format!("glyph{}", glyph);
    let bytes = name.as_bytes();
    let copy_len = bytes.len().min((size - 1) as usize);

    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), buf as *mut u8, copy_len);
        *buf.add(copy_len) = 0;
    }
}

/// Check if font is embedded
#[unsafe(no_mangle)]
pub extern "C" fn fz_font_is_embedded(_ctx: Handle, _font: Handle) -> i32 {
    1 // Assume all fonts are embedded
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_font() {
        let font_handle = fz_new_font(0, c"Helvetica".as_ptr(), 0, 0, 0);
        assert_ne!(font_handle, 0);
        fz_drop_font(0, font_handle);
    }

    #[test]
    fn test_new_font_null_name() {
        let font_handle = fz_new_font(0, std::ptr::null(), 0, 0, 0);
        assert_eq!(font_handle, 0);
    }

    #[test]
    fn test_keep_font() {
        let font_handle = fz_new_font(0, c"Arial".as_ptr(), 0, 0, 0);
        let kept = fz_keep_font(0, font_handle);
        assert_eq!(kept, font_handle);
        fz_drop_font(0, font_handle);
    }

    #[test]
    fn test_font_name() {
        let font_handle = fz_new_font(0, c"Times".as_ptr(), 0, 0, 0);
        let mut buf = [0i8; 64];
        fz_font_name(0, font_handle, buf.as_mut_ptr(), 64);

        let name = unsafe { std::ffi::CStr::from_ptr(buf.as_ptr()).to_str().unwrap() };
        assert_eq!(name, "Times");

        fz_drop_font(0, font_handle);
    }

    #[test]
    fn test_font_properties() {
        let font_handle = fz_new_font(0, c"Courier".as_ptr(), 1, 1, 0);

        // These will return default values since we're not loading actual font files
        let _is_bold = fz_font_is_bold(0, font_handle);
        let _is_italic = fz_font_is_italic(0, font_handle);
        let _is_serif = fz_font_is_serif(0, font_handle);
        let _is_monospaced = fz_font_is_monospaced(0, font_handle);

        fz_drop_font(0, font_handle);
    }

    #[test]
    fn test_encode_character() {
        let font_handle = fz_new_font(0, c"Arial".as_ptr(), 0, 0, 0);

        // Encode 'A' (65)
        let glyph = fz_encode_character(0, font_handle, 65);
        assert!(glyph >= 0);

        fz_drop_font(0, font_handle);
    }

    #[test]
    fn test_advance_glyph() {
        let font_handle = fz_new_font(0, c"Arial".as_ptr(), 0, 0, 0);
        let glyph = fz_encode_character(0, font_handle, 65);

        let advance = fz_advance_glyph(0, font_handle, glyph, 0);
        assert!(advance >= 0.0);

        fz_drop_font(0, font_handle);
    }

    #[test]
    fn test_bound_glyph() {
        let font_handle = fz_new_font(0, c"Arial".as_ptr(), 0, 0, 0);
        let glyph = fz_encode_character(0, font_handle, 65);

        let bbox = fz_bound_glyph(
            0,
            font_handle,
            glyph,
            super::super::geometry::fz_matrix::identity(),
        );
        // Valid bounding box should have x1 > x0
        assert!(bbox.x1 >= bbox.x0);

        fz_drop_font(0, font_handle);
    }

    #[test]
    fn test_font_bbox() {
        let font_handle = fz_new_font(0, c"Arial".as_ptr(), 0, 0, 0);

        let bbox = fz_font_bbox(0, font_handle);
        assert!(bbox.x1 >= bbox.x0);
        assert!(bbox.y1 >= bbox.y0);

        fz_drop_font(0, font_handle);
    }

    #[test]
    fn test_new_font_from_memory() {
        let font_data = b"Fake font data";
        let font_handle = fz_new_font_from_memory(
            0,
            c"Test".as_ptr(),
            font_data.as_ptr(),
            font_data.len() as i32,
            0,
            0,
        );
        // May return 0 if font parsing fails (which is expected with fake data)
        if font_handle != 0 {
            fz_drop_font(0, font_handle);
        }
    }
}
