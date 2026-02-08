//! FFI bindings for fz_write_pixmap (Pixmap Output Functions)
//!
//! Provides functions to save pixmaps as various image formats.

use crate::ffi::buffer::Buffer;
use crate::ffi::output::OUTPUTS;
use crate::ffi::pixmap::Pixmap;
use crate::ffi::{BUFFERS, Handle, PIXMAPS};
use std::ffi::{CStr, c_char};
use std::fs::File;
use std::io::Write;
use std::ptr;

// ============================================================================
// PNG Functions
// ============================================================================

/// Save a pixmap as PNG to a file
#[unsafe(no_mangle)]
pub extern "C" fn fz_save_pixmap_as_png(
    _ctx: Handle,
    pixmap: Handle,
    filename: *const c_char,
) -> i32 {
    if filename.is_null() {
        return -1;
    }

    let filename_str = unsafe { CStr::from_ptr(filename) };
    let filename_str = match filename_str.to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };

    let pix_arc = match PIXMAPS.get(pixmap) {
        Some(p) => p,
        None => return -1,
    };
    let pix = pix_arc.lock().unwrap();

    let png_data = encode_png(&pix);
    if png_data.is_empty() {
        return -1;
    }

    match File::create(filename_str) {
        Ok(mut file) => {
            if file.write_all(&png_data).is_ok() {
                0
            } else {
                -1
            }
        }
        Err(_) => -1,
    }
}

/// Write a pixmap as PNG to an output stream
#[unsafe(no_mangle)]
pub extern "C" fn fz_write_pixmap_as_png(_ctx: Handle, out: Handle, pixmap: Handle) -> i32 {
    let pix_arc = match PIXMAPS.get(pixmap) {
        Some(p) => p,
        None => return -1,
    };
    let pix = pix_arc.lock().unwrap();

    let png_data = encode_png(&pix);
    if png_data.is_empty() {
        return -1;
    }

    let out_arc = match OUTPUTS.get(out) {
        Some(o) => o,
        None => return -1,
    };
    let mut output = out_arc.lock().unwrap();

    if output.write_data(&png_data).is_ok() {
        0
    } else {
        -1
    }
}

/// Create a buffer containing the pixmap as PNG
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_buffer_from_pixmap_as_png(_ctx: Handle, pixmap: Handle) -> Handle {
    let pix_arc = match PIXMAPS.get(pixmap) {
        Some(p) => p,
        None => return 0,
    };
    let pix = pix_arc.lock().unwrap();

    let png_data = encode_png(&pix);
    if png_data.is_empty() {
        return 0;
    }

    let buffer = Buffer::from_data(&png_data);
    BUFFERS.insert(buffer)
}

// ============================================================================
// JPEG Functions
// ============================================================================

/// Save a pixmap as JPEG to a file
#[unsafe(no_mangle)]
pub extern "C" fn fz_save_pixmap_as_jpeg(
    _ctx: Handle,
    pixmap: Handle,
    filename: *const c_char,
    quality: i32,
) -> i32 {
    if filename.is_null() {
        return -1;
    }

    let filename_str = unsafe { CStr::from_ptr(filename) };
    let filename_str = match filename_str.to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };

    let pix_arc = match PIXMAPS.get(pixmap) {
        Some(p) => p,
        None => return -1,
    };
    let pix = pix_arc.lock().unwrap();

    let jpeg_data = encode_jpeg(&pix, quality.clamp(1, 100) as u8);
    if jpeg_data.is_empty() {
        return -1;
    }

    match File::create(filename_str) {
        Ok(mut file) => {
            if file.write_all(&jpeg_data).is_ok() {
                0
            } else {
                -1
            }
        }
        Err(_) => -1,
    }
}

/// Write a pixmap as JPEG to an output stream
#[unsafe(no_mangle)]
pub extern "C" fn fz_write_pixmap_as_jpeg(
    _ctx: Handle,
    out: Handle,
    pixmap: Handle,
    quality: i32,
    _invert_cmyk: i32,
) -> i32 {
    let pix_arc = match PIXMAPS.get(pixmap) {
        Some(p) => p,
        None => return -1,
    };
    let pix = pix_arc.lock().unwrap();

    let jpeg_data = encode_jpeg(&pix, quality.clamp(1, 100) as u8);
    if jpeg_data.is_empty() {
        return -1;
    }

    let out_arc = match OUTPUTS.get(out) {
        Some(o) => o,
        None => return -1,
    };
    let mut output = out_arc.lock().unwrap();

    if output.write_data(&jpeg_data).is_ok() {
        0
    } else {
        -1
    }
}

/// Create a buffer containing the pixmap as JPEG
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_buffer_from_pixmap_as_jpeg(
    _ctx: Handle,
    pixmap: Handle,
    quality: i32,
    _invert_cmyk: i32,
) -> Handle {
    let pix_arc = match PIXMAPS.get(pixmap) {
        Some(p) => p,
        None => return 0,
    };
    let pix = pix_arc.lock().unwrap();

    let jpeg_data = encode_jpeg(&pix, quality.clamp(1, 100) as u8);
    if jpeg_data.is_empty() {
        return 0;
    }

    let buffer = Buffer::from_data(&jpeg_data);
    BUFFERS.insert(buffer)
}

// ============================================================================
// PNM/PPM/PGM Functions (Portable Any Map)
// ============================================================================

/// Save a pixmap as PNM to a file
#[unsafe(no_mangle)]
pub extern "C" fn fz_save_pixmap_as_pnm(
    _ctx: Handle,
    pixmap: Handle,
    filename: *const c_char,
) -> i32 {
    if filename.is_null() {
        return -1;
    }

    let filename_str = unsafe { CStr::from_ptr(filename) };
    let filename_str = match filename_str.to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };

    let pix_arc = match PIXMAPS.get(pixmap) {
        Some(p) => p,
        None => return -1,
    };
    let pix = pix_arc.lock().unwrap();

    let pnm_data = encode_pnm(&pix);
    if pnm_data.is_empty() {
        return -1;
    }

    match File::create(filename_str) {
        Ok(mut file) => {
            if file.write_all(&pnm_data).is_ok() {
                0
            } else {
                -1
            }
        }
        Err(_) => -1,
    }
}

/// Write a pixmap as PNM to an output stream
#[unsafe(no_mangle)]
pub extern "C" fn fz_write_pixmap_as_pnm(_ctx: Handle, out: Handle, pixmap: Handle) -> i32 {
    let pix_arc = match PIXMAPS.get(pixmap) {
        Some(p) => p,
        None => return -1,
    };
    let pix = pix_arc.lock().unwrap();

    let pnm_data = encode_pnm(&pix);
    if pnm_data.is_empty() {
        return -1;
    }

    let out_arc = match OUTPUTS.get(out) {
        Some(o) => o,
        None => return -1,
    };
    let mut output = out_arc.lock().unwrap();

    if output.write_data(&pnm_data).is_ok() {
        0
    } else {
        -1
    }
}

/// Create a buffer containing the pixmap as PNM
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_buffer_from_pixmap_as_pnm(_ctx: Handle, pixmap: Handle) -> Handle {
    let pix_arc = match PIXMAPS.get(pixmap) {
        Some(p) => p,
        None => return 0,
    };
    let pix = pix_arc.lock().unwrap();

    let pnm_data = encode_pnm(&pix);
    if pnm_data.is_empty() {
        return 0;
    }

    let buffer = Buffer::from_data(&pnm_data);
    BUFFERS.insert(buffer)
}

// ============================================================================
// PAM Functions (Portable Arbitrary Map)
// ============================================================================

/// Save a pixmap as PAM to a file
#[unsafe(no_mangle)]
pub extern "C" fn fz_save_pixmap_as_pam(
    _ctx: Handle,
    pixmap: Handle,
    filename: *const c_char,
) -> i32 {
    if filename.is_null() {
        return -1;
    }

    let filename_str = unsafe { CStr::from_ptr(filename) };
    let filename_str = match filename_str.to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };

    let pix_arc = match PIXMAPS.get(pixmap) {
        Some(p) => p,
        None => return -1,
    };
    let pix = pix_arc.lock().unwrap();

    let pam_data = encode_pam(&pix);
    if pam_data.is_empty() {
        return -1;
    }

    match File::create(filename_str) {
        Ok(mut file) => {
            if file.write_all(&pam_data).is_ok() {
                0
            } else {
                -1
            }
        }
        Err(_) => -1,
    }
}

/// Write a pixmap as PAM to an output stream
#[unsafe(no_mangle)]
pub extern "C" fn fz_write_pixmap_as_pam(_ctx: Handle, out: Handle, pixmap: Handle) -> i32 {
    let pix_arc = match PIXMAPS.get(pixmap) {
        Some(p) => p,
        None => return -1,
    };
    let pix = pix_arc.lock().unwrap();

    let pam_data = encode_pam(&pix);
    if pam_data.is_empty() {
        return -1;
    }

    let out_arc = match OUTPUTS.get(out) {
        Some(o) => o,
        None => return -1,
    };
    let mut output = out_arc.lock().unwrap();

    if output.write_data(&pam_data).is_ok() {
        0
    } else {
        -1
    }
}

/// Create a buffer containing the pixmap as PAM
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_buffer_from_pixmap_as_pam(_ctx: Handle, pixmap: Handle) -> Handle {
    let pix_arc = match PIXMAPS.get(pixmap) {
        Some(p) => p,
        None => return 0,
    };
    let pix = pix_arc.lock().unwrap();

    let pam_data = encode_pam(&pix);
    if pam_data.is_empty() {
        return 0;
    }

    let buffer = Buffer::from_data(&pam_data);
    BUFFERS.insert(buffer)
}

// ============================================================================
// PBM Functions (Portable Bitmap - 1-bit)
// ============================================================================

/// Save a pixmap as PBM to a file (with halftoning)
#[unsafe(no_mangle)]
pub extern "C" fn fz_save_pixmap_as_pbm(
    _ctx: Handle,
    pixmap: Handle,
    filename: *const c_char,
) -> i32 {
    if filename.is_null() {
        return -1;
    }

    let filename_str = unsafe { CStr::from_ptr(filename) };
    let filename_str = match filename_str.to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };

    let pix_arc = match PIXMAPS.get(pixmap) {
        Some(p) => p,
        None => return -1,
    };
    let pix = pix_arc.lock().unwrap();

    let pbm_data = encode_pbm(&pix);
    if pbm_data.is_empty() {
        return -1;
    }

    match File::create(filename_str) {
        Ok(mut file) => {
            if file.write_all(&pbm_data).is_ok() {
                0
            } else {
                -1
            }
        }
        Err(_) => -1,
    }
}

/// Write a pixmap as PBM to an output stream
#[unsafe(no_mangle)]
pub extern "C" fn fz_write_pixmap_as_pbm(_ctx: Handle, out: Handle, pixmap: Handle) -> i32 {
    let pix_arc = match PIXMAPS.get(pixmap) {
        Some(p) => p,
        None => return -1,
    };
    let pix = pix_arc.lock().unwrap();

    let pbm_data = encode_pbm(&pix);
    if pbm_data.is_empty() {
        return -1;
    }

    let out_arc = match OUTPUTS.get(out) {
        Some(o) => o,
        None => return -1,
    };
    let mut output = out_arc.lock().unwrap();

    if output.write_data(&pbm_data).is_ok() {
        0
    } else {
        -1
    }
}

/// Create a buffer containing the pixmap as PBM
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_buffer_from_pixmap_as_pbm(_ctx: Handle, pixmap: Handle) -> Handle {
    let pix_arc = match PIXMAPS.get(pixmap) {
        Some(p) => p,
        None => return 0,
    };
    let pix = pix_arc.lock().unwrap();

    let pbm_data = encode_pbm(&pix);
    if pbm_data.is_empty() {
        return 0;
    }

    let buffer = Buffer::from_data(&pbm_data);
    BUFFERS.insert(buffer)
}

// ============================================================================
// PKM Functions (CMYK Portable)
// ============================================================================

/// Save a pixmap as PKM to a file
#[unsafe(no_mangle)]
pub extern "C" fn fz_save_pixmap_as_pkm(
    _ctx: Handle,
    pixmap: Handle,
    filename: *const c_char,
) -> i32 {
    if filename.is_null() {
        return -1;
    }

    let filename_str = unsafe { CStr::from_ptr(filename) };
    let filename_str = match filename_str.to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };

    let pix_arc = match PIXMAPS.get(pixmap) {
        Some(p) => p,
        None => return -1,
    };
    let pix = pix_arc.lock().unwrap();

    let pkm_data = encode_pkm(&pix);
    if pkm_data.is_empty() {
        return -1;
    }

    match File::create(filename_str) {
        Ok(mut file) => {
            if file.write_all(&pkm_data).is_ok() {
                0
            } else {
                -1
            }
        }
        Err(_) => -1,
    }
}

/// Write a pixmap as PKM to an output stream
#[unsafe(no_mangle)]
pub extern "C" fn fz_write_pixmap_as_pkm(_ctx: Handle, out: Handle, pixmap: Handle) -> i32 {
    let pix_arc = match PIXMAPS.get(pixmap) {
        Some(p) => p,
        None => return -1,
    };
    let pix = pix_arc.lock().unwrap();

    let pkm_data = encode_pkm(&pix);
    if pkm_data.is_empty() {
        return -1;
    }

    let out_arc = match OUTPUTS.get(out) {
        Some(o) => o,
        None => return -1,
    };
    let mut output = out_arc.lock().unwrap();

    if output.write_data(&pkm_data).is_ok() {
        0
    } else {
        -1
    }
}

/// Create a buffer containing the pixmap as PKM
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_buffer_from_pixmap_as_pkm(_ctx: Handle, pixmap: Handle) -> Handle {
    let pix_arc = match PIXMAPS.get(pixmap) {
        Some(p) => p,
        None => return 0,
    };
    let pix = pix_arc.lock().unwrap();

    let pkm_data = encode_pkm(&pix);
    if pkm_data.is_empty() {
        return 0;
    }

    let buffer = Buffer::from_data(&pkm_data);
    BUFFERS.insert(buffer)
}

// ============================================================================
// PSD Functions (Photoshop)
// ============================================================================

/// Save a pixmap as PSD to a file
#[unsafe(no_mangle)]
pub extern "C" fn fz_save_pixmap_as_psd(
    _ctx: Handle,
    pixmap: Handle,
    filename: *const c_char,
) -> i32 {
    if filename.is_null() {
        return -1;
    }

    let filename_str = unsafe { CStr::from_ptr(filename) };
    let filename_str = match filename_str.to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };

    let pix_arc = match PIXMAPS.get(pixmap) {
        Some(p) => p,
        None => return -1,
    };
    let pix = pix_arc.lock().unwrap();

    let psd_data = encode_psd(&pix);
    if psd_data.is_empty() {
        return -1;
    }

    match File::create(filename_str) {
        Ok(mut file) => {
            if file.write_all(&psd_data).is_ok() {
                0
            } else {
                -1
            }
        }
        Err(_) => -1,
    }
}

/// Write a pixmap as PSD to an output stream
#[unsafe(no_mangle)]
pub extern "C" fn fz_write_pixmap_as_psd(_ctx: Handle, out: Handle, pixmap: Handle) -> i32 {
    let pix_arc = match PIXMAPS.get(pixmap) {
        Some(p) => p,
        None => return -1,
    };
    let pix = pix_arc.lock().unwrap();

    let psd_data = encode_psd(&pix);
    if psd_data.is_empty() {
        return -1;
    }

    let out_arc = match OUTPUTS.get(out) {
        Some(o) => o,
        None => return -1,
    };
    let mut output = out_arc.lock().unwrap();

    if output.write_data(&psd_data).is_ok() {
        0
    } else {
        -1
    }
}

/// Create a buffer containing the pixmap as PSD
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_buffer_from_pixmap_as_psd(_ctx: Handle, pixmap: Handle) -> Handle {
    let pix_arc = match PIXMAPS.get(pixmap) {
        Some(p) => p,
        None => return 0,
    };
    let pix = pix_arc.lock().unwrap();

    let psd_data = encode_psd(&pix);
    if psd_data.is_empty() {
        return 0;
    }

    let buffer = Buffer::from_data(&psd_data);
    BUFFERS.insert(buffer)
}

// ============================================================================
// PostScript Functions
// ============================================================================

/// Save a pixmap as PostScript to a file
#[unsafe(no_mangle)]
pub extern "C" fn fz_save_pixmap_as_ps(
    _ctx: Handle,
    pixmap: Handle,
    filename: *const c_char,
    _append: i32,
) -> i32 {
    if filename.is_null() {
        return -1;
    }

    let filename_str = unsafe { CStr::from_ptr(filename) };
    let filename_str = match filename_str.to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };

    let pix_arc = match PIXMAPS.get(pixmap) {
        Some(p) => p,
        None => return -1,
    };
    let pix = pix_arc.lock().unwrap();

    let ps_data = encode_ps(&pix);
    if ps_data.is_empty() {
        return -1;
    }

    match File::create(filename_str) {
        Ok(mut file) => {
            if file.write_all(&ps_data).is_ok() {
                0
            } else {
                -1
            }
        }
        Err(_) => -1,
    }
}

/// Write a pixmap as PostScript to an output stream
#[unsafe(no_mangle)]
pub extern "C" fn fz_write_pixmap_as_ps(_ctx: Handle, out: Handle, pixmap: Handle) -> i32 {
    let pix_arc = match PIXMAPS.get(pixmap) {
        Some(p) => p,
        None => return -1,
    };
    let pix = pix_arc.lock().unwrap();

    let ps_data = encode_ps(&pix);
    if ps_data.is_empty() {
        return -1;
    }

    let out_arc = match OUTPUTS.get(out) {
        Some(o) => o,
        None => return -1,
    };
    let mut output = out_arc.lock().unwrap();

    if output.write_data(&ps_data).is_ok() {
        0
    } else {
        -1
    }
}

/// Write PostScript file header
#[unsafe(no_mangle)]
pub extern "C" fn fz_write_ps_file_header(_ctx: Handle, out: Handle) -> i32 {
    let header = b"%!PS-Adobe-3.0\n%%Creator: MicroPDF\n%%Pages: (atend)\n%%EndComments\n";

    let out_arc = match OUTPUTS.get(out) {
        Some(o) => o,
        None => return -1,
    };
    let mut output = out_arc.lock().unwrap();

    if output.write_data(header).is_ok() {
        0
    } else {
        -1
    }
}

/// Write PostScript file trailer
#[unsafe(no_mangle)]
pub extern "C" fn fz_write_ps_file_trailer(_ctx: Handle, out: Handle, pages: i32) -> i32 {
    let trailer = format!("%%Trailer\n%%Pages: {}\n%%EOF\n", pages);

    let out_arc = match OUTPUTS.get(out) {
        Some(o) => o,
        None => return -1,
    };
    let mut output = out_arc.lock().unwrap();

    if output.write_data(trailer.as_bytes()).is_ok() {
        0
    } else {
        -1
    }
}

// ============================================================================
// Data URI Functions
// ============================================================================

/// Write a pixmap as a data URI (base64 encoded PNG)
#[unsafe(no_mangle)]
pub extern "C" fn fz_write_pixmap_as_data_uri(_ctx: Handle, out: Handle, pixmap: Handle) -> i32 {
    let pix_arc = match PIXMAPS.get(pixmap) {
        Some(p) => p,
        None => return -1,
    };
    let pix = pix_arc.lock().unwrap();

    let png_data = encode_png(&pix);
    if png_data.is_empty() {
        return -1;
    }

    // Encode as base64
    let base64_encoded =
        base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &png_data);
    let data_uri = format!("data:image/png;base64,{}", base64_encoded);

    let out_arc = match OUTPUTS.get(out) {
        Some(o) => o,
        None => return -1,
    };
    let mut output = out_arc.lock().unwrap();

    if output.write_data(data_uri.as_bytes()).is_ok() {
        0
    } else {
        -1
    }
}

// ============================================================================
// Internal Encoding Functions
// ============================================================================

/// Encode pixmap as PNG using the image crate
fn encode_png(pix: &Pixmap) -> Vec<u8> {
    use image::codecs::png::PngEncoder;
    use image::{ExtendedColorType, ImageEncoder};

    let width = pix.w() as u32;
    let height = pix.h() as u32;
    let n = pix.n();
    let samples = pix.samples();

    let color_type = match (n, pix.has_alpha()) {
        (1, false) => ExtendedColorType::L8,
        (2, true) => ExtendedColorType::La8,
        (3, false) => ExtendedColorType::Rgb8,
        (4, true) => ExtendedColorType::Rgba8,
        _ => return Vec::new(), // Unsupported format
    };

    let mut buf = Vec::new();
    let encoder = PngEncoder::new(&mut buf);
    if encoder
        .write_image(samples, width, height, color_type)
        .is_ok()
    {
        buf
    } else {
        Vec::new()
    }
}

/// Encode pixmap as JPEG using the image crate
fn encode_jpeg(pix: &Pixmap, quality: u8) -> Vec<u8> {
    use image::codecs::jpeg::JpegEncoder;
    use image::{ExtendedColorType, ImageEncoder};

    let width = pix.w() as u32;
    let height = pix.h() as u32;
    let n = pix.n();
    let samples = pix.samples();

    // JPEG only supports L8 and Rgb8
    let color_type = match n {
        1 => ExtendedColorType::L8,
        3 => ExtendedColorType::Rgb8,
        4 => {
            // Remove alpha channel for JPEG
            let rgb_samples = remove_alpha(samples, 4);
            let mut buf = Vec::new();
            let encoder = JpegEncoder::new_with_quality(&mut buf, quality);
            if encoder
                .write_image(&rgb_samples, width, height, ExtendedColorType::Rgb8)
                .is_ok()
            {
                return buf;
            }
            return Vec::new();
        }
        _ => return Vec::new(),
    };

    let mut buf = Vec::new();
    let encoder = JpegEncoder::new_with_quality(&mut buf, quality);
    if encoder
        .write_image(samples, width, height, color_type)
        .is_ok()
    {
        buf
    } else {
        Vec::new()
    }
}

/// Encode pixmap as PNM (PPM for RGB, PGM for grayscale)
fn encode_pnm(pix: &Pixmap) -> Vec<u8> {
    let width = pix.w();
    let height = pix.h();
    let n = pix.n();
    let samples = pix.samples();

    let mut buf = Vec::new();

    match n {
        1 => {
            // PGM (grayscale)
            buf.extend_from_slice(format!("P5\n{} {}\n255\n", width, height).as_bytes());
            buf.extend_from_slice(samples);
        }
        3 => {
            // PPM (RGB)
            buf.extend_from_slice(format!("P6\n{} {}\n255\n", width, height).as_bytes());
            buf.extend_from_slice(samples);
        }
        4 => {
            // RGBA -> PPM (remove alpha)
            buf.extend_from_slice(format!("P6\n{} {}\n255\n", width, height).as_bytes());
            let rgb_samples = remove_alpha(samples, 4);
            buf.extend_from_slice(&rgb_samples);
        }
        _ => return Vec::new(),
    }

    buf
}

/// Encode pixmap as PAM (supports alpha)
fn encode_pam(pix: &Pixmap) -> Vec<u8> {
    let width = pix.w();
    let height = pix.h();
    let n = pix.n();
    let samples = pix.samples();

    let tupltype = match n {
        1 => "GRAYSCALE",
        2 => "GRAYSCALE_ALPHA",
        3 => "RGB",
        4 => "RGB_ALPHA",
        _ => return Vec::new(),
    };

    let mut buf = Vec::new();
    buf.extend_from_slice(
        format!(
            "P7\nWIDTH {}\nHEIGHT {}\nDEPTH {}\nMAXVAL 255\nTUPLTYPE {}\nENDHDR\n",
            width, height, n, tupltype
        )
        .as_bytes(),
    );
    buf.extend_from_slice(samples);

    buf
}

/// Encode pixmap as PBM (1-bit, with halftoning)
fn encode_pbm(pix: &Pixmap) -> Vec<u8> {
    let width = pix.w();
    let height = pix.h();
    let samples = pix.samples();

    // Convert to grayscale if needed
    let gray = to_grayscale(pix);

    // Apply Floyd-Steinberg dithering
    let mut dithered = gray.clone();
    floyd_steinberg_dither(&mut dithered, width as usize, height as usize);

    // Pack bits
    let row_bytes = (width + 7) / 8;
    let mut buf = Vec::new();
    buf.extend_from_slice(format!("P4\n{} {}\n", width, height).as_bytes());

    for y in 0..height as usize {
        for byte_x in 0..row_bytes as usize {
            let mut byte = 0u8;
            for bit in 0..8 {
                let x = byte_x * 8 + bit;
                if x < width as usize {
                    let idx = y * width as usize + x;
                    if dithered[idx] < 128 {
                        byte |= 0x80 >> bit;
                    }
                }
            }
            buf.push(byte);
        }
    }

    buf
}

/// Encode pixmap as PKM (CMYK portable)
fn encode_pkm(pix: &Pixmap) -> Vec<u8> {
    let width = pix.w();
    let height = pix.h();
    let n = pix.n();
    let samples = pix.samples();

    // PKM is a simple CMYK format
    let mut buf = Vec::new();
    buf.extend_from_slice(
        format!(
            "P7\nWIDTH {}\nHEIGHT {}\nDEPTH {}\nMAXVAL 255\nTUPLTYPE CMYK\nENDHDR\n",
            width,
            height,
            n.max(4)
        )
        .as_bytes(),
    );

    if n == 4 {
        buf.extend_from_slice(samples);
    } else {
        // Convert RGB to CMYK
        let cmyk = rgb_to_cmyk(samples, n);
        buf.extend_from_slice(&cmyk);
    }

    buf
}

/// Encode pixmap as PSD (Photoshop)
fn encode_psd(pix: &Pixmap) -> Vec<u8> {
    let width = pix.w() as u32;
    let height = pix.h() as u32;
    let n = pix.n() as u16;
    let samples = pix.samples();

    let mut buf = Vec::new();

    // PSD file header
    buf.extend_from_slice(b"8BPS"); // Signature
    buf.extend_from_slice(&2u16.to_be_bytes()); // Version
    buf.extend_from_slice(&[0u8; 6]); // Reserved
    buf.extend_from_slice(&n.to_be_bytes()); // Channels
    buf.extend_from_slice(&height.to_be_bytes()); // Height
    buf.extend_from_slice(&width.to_be_bytes()); // Width
    buf.extend_from_slice(&8u16.to_be_bytes()); // Bits per channel

    // Color mode
    let color_mode: u16 = match n {
        1 => 1, // Grayscale
        3 => 3, // RGB
        4 => 3, // RGBA (still RGB mode)
        _ => 3,
    };
    buf.extend_from_slice(&color_mode.to_be_bytes());

    // Color mode data (empty)
    buf.extend_from_slice(&0u32.to_be_bytes());

    // Image resources (empty)
    buf.extend_from_slice(&0u32.to_be_bytes());

    // Layer and mask info (empty)
    buf.extend_from_slice(&0u32.to_be_bytes());

    // Image data
    buf.extend_from_slice(&0u16.to_be_bytes()); // Compression: raw

    // Write channel data (planar format)
    let pixel_count = (width * height) as usize;
    for channel in 0..n as usize {
        for i in 0..pixel_count {
            let idx = i * n as usize + channel;
            if idx < samples.len() {
                buf.push(samples[idx]);
            } else {
                buf.push(255); // Alpha default
            }
        }
    }

    buf
}

/// Encode pixmap as PostScript
fn encode_ps(pix: &Pixmap) -> Vec<u8> {
    let width = pix.w();
    let height = pix.h();
    let n = pix.n();
    let samples = pix.samples();

    let mut buf = Vec::new();

    // EPS header
    buf.extend_from_slice(b"%!PS-Adobe-3.0 EPSF-3.0\n");
    buf.extend_from_slice(format!("%%BoundingBox: 0 0 {} {}\n", width, height).as_bytes());
    buf.extend_from_slice(b"%%EndComments\n");

    // Image setup
    buf.extend_from_slice(format!("/width {} def\n", width).as_bytes());
    buf.extend_from_slice(format!("/height {} def\n", height).as_bytes());
    buf.extend_from_slice(format!("/ncomp {} def\n", n.min(3)).as_bytes());
    buf.extend_from_slice(b"/picstr width ncomp mul string def\n");

    buf.extend_from_slice(
        format!(
            "width height 8 [width 0 0 height neg 0 height]\n\
             {{ currentfile picstr readhexstring pop }}\n\
             {} {} image\n",
            if n == 1 { "false" } else { "false 3" },
            if n == 1 { "" } else { "colorimage" }
        )
        .as_bytes(),
    );

    // Image data as hex
    let effective_n = n.min(3) as usize;
    for i in 0..(width * height) as usize {
        for c in 0..effective_n {
            let idx = i * n as usize + c;
            if idx < samples.len() {
                buf.extend_from_slice(format!("{:02x}", samples[idx]).as_bytes());
            }
        }
        if (i + 1) % 32 == 0 {
            buf.push(b'\n');
        }
    }

    buf.extend_from_slice(b"\nshowpage\n%%EOF\n");

    buf
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Remove alpha channel from samples
fn remove_alpha(samples: &[u8], n: i32) -> Vec<u8> {
    if n < 4 {
        return samples.to_vec();
    }

    let pixel_count = samples.len() / n as usize;
    let mut rgb = Vec::with_capacity(pixel_count * 3);

    for i in 0..pixel_count {
        let base = i * n as usize;
        rgb.push(samples[base]);
        rgb.push(samples[base + 1]);
        rgb.push(samples[base + 2]);
    }

    rgb
}

/// Convert to grayscale
fn to_grayscale(pix: &Pixmap) -> Vec<u8> {
    let n = pix.n() as usize;
    let samples = pix.samples();
    let pixel_count = samples.len() / n;

    let mut gray = Vec::with_capacity(pixel_count);

    for i in 0..pixel_count {
        let base = i * n;
        let value = if n == 1 {
            samples[base]
        } else if n >= 3 {
            // Luminosity formula
            let r = samples[base] as u32;
            let g = samples[base + 1] as u32;
            let b = samples[base + 2] as u32;
            ((r * 299 + g * 587 + b * 114) / 1000) as u8
        } else {
            samples[base]
        };
        gray.push(value);
    }

    gray
}

/// Floyd-Steinberg dithering
fn floyd_steinberg_dither(data: &mut [u8], width: usize, height: usize) {
    for y in 0..height {
        for x in 0..width {
            let idx = y * width + x;
            let old_val = data[idx] as i32;
            let new_val = if old_val < 128 { 0 } else { 255 };
            data[idx] = new_val as u8;
            let error = old_val - new_val;

            // Distribute error
            if x + 1 < width {
                let val = (data[idx + 1] as i32 + error * 7 / 16).clamp(0, 255);
                data[idx + 1] = val as u8;
            }
            if y + 1 < height {
                if x > 0 {
                    let next_idx = (y + 1) * width + x - 1;
                    let val = (data[next_idx] as i32 + error * 3 / 16).clamp(0, 255);
                    data[next_idx] = val as u8;
                }
                let next_idx = (y + 1) * width + x;
                let val = (data[next_idx] as i32 + error * 5 / 16).clamp(0, 255);
                data[next_idx] = val as u8;
                if x + 1 < width {
                    let next_idx = (y + 1) * width + x + 1;
                    let val = (data[next_idx] as i32 + error * 1 / 16).clamp(0, 255);
                    data[next_idx] = val as u8;
                }
            }
        }
    }
}

/// Convert RGB to CMYK
fn rgb_to_cmyk(samples: &[u8], n: i32) -> Vec<u8> {
    let n = n as usize;
    let pixel_count = samples.len() / n;
    let mut cmyk = Vec::with_capacity(pixel_count * 4);

    for i in 0..pixel_count {
        let base = i * n;
        let (r, g, b) = if n >= 3 {
            (
                samples[base] as f32 / 255.0,
                samples[base + 1] as f32 / 255.0,
                samples[base + 2] as f32 / 255.0,
            )
        } else {
            let gray = samples[base] as f32 / 255.0;
            (gray, gray, gray)
        };

        let k = 1.0 - r.max(g).max(b);
        let (c, m, y) = if k < 1.0 {
            (
                (1.0 - r - k) / (1.0 - k),
                (1.0 - g - k) / (1.0 - k),
                (1.0 - b - k) / (1.0 - k),
            )
        } else {
            (0.0, 0.0, 0.0)
        };

        cmyk.push((c * 255.0) as u8);
        cmyk.push((m * 255.0) as u8);
        cmyk.push((y * 255.0) as u8);
        cmyk.push((k * 255.0) as u8);
    }

    cmyk
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ffi::colorspace::fz_device_rgb;
    use crate::ffi::pixmap::fz_new_pixmap;
    use std::ffi::CString;

    fn create_test_pixmap() -> Handle {
        let ctx = 1;
        let cs = fz_device_rgb(ctx);
        let pix = fz_new_pixmap(ctx, cs, 10, 10, 0, 1); // seps=0, alpha=1

        // Fill with test pattern
        if let Some(pix_arc) = PIXMAPS.get(pix) {
            let mut pix_guard = pix_arc.lock().unwrap();
            let samples = pix_guard.samples_mut();
            for i in 0..(10 * 10) {
                let base = i * 4;
                samples[base] = (i % 256) as u8; // R
                samples[base + 1] = ((i * 2) % 256) as u8; // G
                samples[base + 2] = ((i * 3) % 256) as u8; // B
                samples[base + 3] = 255; // A
            }
        }

        pix
    }

    #[test]
    fn test_png_buffer() {
        let ctx = 1;
        let pix = create_test_pixmap();

        let buf = fz_new_buffer_from_pixmap_as_png(ctx, pix);
        assert!(buf > 0);

        // Check PNG header
        if let Some(buf_arc) = BUFFERS.get(buf) {
            let buf_guard = buf_arc.lock().unwrap();
            let data = buf_guard.data();
            assert!(data.len() > 8);
            // PNG magic bytes
            assert_eq!(
                &data[0..8],
                &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]
            );
        }

        crate::ffi::buffer::fz_drop_buffer(ctx, buf);
        crate::ffi::pixmap::fz_drop_pixmap(ctx, pix);
    }

    #[test]
    fn test_jpeg_buffer() {
        let ctx = 1;
        let pix = create_test_pixmap();

        let buf = fz_new_buffer_from_pixmap_as_jpeg(ctx, pix, 85, 0);
        assert!(buf > 0);

        // Check JPEG header
        if let Some(buf_arc) = BUFFERS.get(buf) {
            let buf_guard = buf_arc.lock().unwrap();
            let data = buf_guard.data();
            assert!(data.len() > 2);
            // JPEG magic bytes (SOI marker)
            assert_eq!(&data[0..2], &[0xFF, 0xD8]);
        }

        crate::ffi::buffer::fz_drop_buffer(ctx, buf);
        crate::ffi::pixmap::fz_drop_pixmap(ctx, pix);
    }

    #[test]
    fn test_pnm_buffer() {
        let ctx = 1;
        let pix = create_test_pixmap();

        let buf = fz_new_buffer_from_pixmap_as_pnm(ctx, pix);
        assert!(buf > 0);

        // Check PNM header (P6 for RGB)
        if let Some(buf_arc) = BUFFERS.get(buf) {
            let buf_guard = buf_arc.lock().unwrap();
            let data = buf_guard.data();
            assert!(data.len() > 2);
            assert_eq!(&data[0..2], b"P6");
        }

        crate::ffi::buffer::fz_drop_buffer(ctx, buf);
        crate::ffi::pixmap::fz_drop_pixmap(ctx, pix);
    }

    #[test]
    fn test_pam_buffer() {
        let ctx = 1;
        let pix = create_test_pixmap();

        let buf = fz_new_buffer_from_pixmap_as_pam(ctx, pix);
        assert!(buf > 0);

        // Check PAM header
        if let Some(buf_arc) = BUFFERS.get(buf) {
            let buf_guard = buf_arc.lock().unwrap();
            let data = buf_guard.data();
            assert!(data.len() > 2);
            assert_eq!(&data[0..2], b"P7");
        }

        crate::ffi::buffer::fz_drop_buffer(ctx, buf);
        crate::ffi::pixmap::fz_drop_pixmap(ctx, pix);
    }

    #[test]
    fn test_pbm_buffer() {
        let ctx = 1;
        let pix = create_test_pixmap();

        let buf = fz_new_buffer_from_pixmap_as_pbm(ctx, pix);
        assert!(buf > 0);

        // Check PBM header
        if let Some(buf_arc) = BUFFERS.get(buf) {
            let buf_guard = buf_arc.lock().unwrap();
            let data = buf_guard.data();
            assert!(data.len() > 2);
            assert_eq!(&data[0..2], b"P4");
        }

        crate::ffi::buffer::fz_drop_buffer(ctx, buf);
        crate::ffi::pixmap::fz_drop_pixmap(ctx, pix);
    }

    #[test]
    fn test_pkm_buffer() {
        let ctx = 1;
        let pix = create_test_pixmap();

        let buf = fz_new_buffer_from_pixmap_as_pkm(ctx, pix);
        assert!(buf > 0);

        // Check PKM header (P7 for PAM/CMYK)
        if let Some(buf_arc) = BUFFERS.get(buf) {
            let buf_guard = buf_arc.lock().unwrap();
            let data = buf_guard.data();
            assert!(data.len() > 2);
            assert_eq!(&data[0..2], b"P7");
        }

        crate::ffi::buffer::fz_drop_buffer(ctx, buf);
        crate::ffi::pixmap::fz_drop_pixmap(ctx, pix);
    }

    #[test]
    fn test_psd_buffer() {
        let ctx = 1;
        let pix = create_test_pixmap();

        let buf = fz_new_buffer_from_pixmap_as_psd(ctx, pix);
        assert!(buf > 0);

        // Check PSD header
        if let Some(buf_arc) = BUFFERS.get(buf) {
            let buf_guard = buf_arc.lock().unwrap();
            let data = buf_guard.data();
            assert!(data.len() > 4);
            assert_eq!(&data[0..4], b"8BPS");
        }

        crate::ffi::buffer::fz_drop_buffer(ctx, buf);
        crate::ffi::pixmap::fz_drop_pixmap(ctx, pix);
    }

    #[test]
    fn test_invalid_pixmap() {
        let ctx = 1;

        assert_eq!(fz_new_buffer_from_pixmap_as_png(ctx, 999999), 0);
        assert_eq!(fz_new_buffer_from_pixmap_as_jpeg(ctx, 999999, 85, 0), 0);
        assert_eq!(fz_new_buffer_from_pixmap_as_pnm(ctx, 999999), 0);
        assert_eq!(fz_new_buffer_from_pixmap_as_pam(ctx, 999999), 0);
        assert_eq!(fz_new_buffer_from_pixmap_as_pbm(ctx, 999999), 0);
        assert_eq!(fz_new_buffer_from_pixmap_as_pkm(ctx, 999999), 0);
        assert_eq!(fz_new_buffer_from_pixmap_as_psd(ctx, 999999), 0);
    }

    #[test]
    fn test_null_filename() {
        let ctx = 1;
        let pix = create_test_pixmap();

        assert_eq!(fz_save_pixmap_as_png(ctx, pix, ptr::null()), -1);
        assert_eq!(fz_save_pixmap_as_jpeg(ctx, pix, ptr::null(), 85), -1);
        assert_eq!(fz_save_pixmap_as_pnm(ctx, pix, ptr::null()), -1);
        assert_eq!(fz_save_pixmap_as_pam(ctx, pix, ptr::null()), -1);
        assert_eq!(fz_save_pixmap_as_pbm(ctx, pix, ptr::null()), -1);
        assert_eq!(fz_save_pixmap_as_pkm(ctx, pix, ptr::null()), -1);
        assert_eq!(fz_save_pixmap_as_psd(ctx, pix, ptr::null()), -1);
        assert_eq!(fz_save_pixmap_as_ps(ctx, pix, ptr::null(), 0), -1);

        crate::ffi::pixmap::fz_drop_pixmap(ctx, pix);
    }

    #[test]
    fn test_grayscale_conversion() {
        let ctx = 1;
        let cs = crate::ffi::colorspace::fz_device_gray(ctx);
        let pix = fz_new_pixmap(ctx, cs, 5, 5, 0, 0); // seps=0, alpha=0

        // Fill with gray values
        if let Some(pix_arc) = PIXMAPS.get(pix) {
            let mut pix_guard = pix_arc.lock().unwrap();
            let samples = pix_guard.samples_mut();
            for i in 0..25 {
                samples[i] = (i * 10) as u8;
            }
        }

        let buf = fz_new_buffer_from_pixmap_as_png(ctx, pix);
        assert!(buf > 0);

        crate::ffi::buffer::fz_drop_buffer(ctx, buf);
        crate::ffi::pixmap::fz_drop_pixmap(ctx, pix);
    }

    #[test]
    fn test_helper_functions() {
        // Test remove_alpha
        let rgba = vec![255, 0, 0, 255, 0, 255, 0, 128];
        let rgb = remove_alpha(&rgba, 4);
        assert_eq!(rgb, vec![255, 0, 0, 0, 255, 0]);

        // Test rgb_to_cmyk
        let rgb = vec![255, 0, 0]; // Pure red
        let cmyk = rgb_to_cmyk(&rgb, 3);
        assert_eq!(cmyk.len(), 4);
        assert_eq!(cmyk[0], 0); // C
        assert_eq!(cmyk[1], 255); // M
        assert_eq!(cmyk[2], 255); // Y
        assert_eq!(cmyk[3], 0); // K
    }
}
