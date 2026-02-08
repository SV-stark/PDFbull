//! C FFI for fz_device - MuPDF compatible rendering device
//!
//! Provides FFI bindings for the device abstraction layer.

use super::{Handle, HandleStore, PIXMAPS};
use crate::fitz::colorspace::Colorspace as FitzColorspace;
use crate::fitz::device::{BBoxDevice, Device, NullDevice, TraceDevice};
use crate::fitz::display_list::ListDevice;
use crate::fitz::geometry::{Matrix, Rect};
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

/// Device storage
pub static DEVICES: LazyLock<HandleStore<Box<dyn Device + Send + Sync>>> =
    LazyLock::new(HandleStore::default);

/// Device hints storage (device handle -> hints bitfield)
static DEVICE_HINTS: LazyLock<Mutex<HashMap<Handle, i32>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Convert FFI colorspace to Fitz colorspace
fn get_fitz_colorspace(handle: Handle) -> Option<FitzColorspace> {
    super::colorspace::COLORSPACES
        .get(handle)
        .and_then(|cs_arc| {
            cs_arc.lock().ok().map(|cs_guard| {
                match cs_guard.cs_type {
                    super::colorspace::ColorspaceType::Gray => FitzColorspace::device_gray(),
                    super::colorspace::ColorspaceType::Rgb => FitzColorspace::device_rgb(),
                    super::colorspace::ColorspaceType::Cmyk => FitzColorspace::device_cmyk(),
                    _ => FitzColorspace::device_rgb(), // Default fallback
                }
            })
        })
}

/// Create a new draw device for rendering to a pixmap
///
/// # Safety
/// Caller must ensure pixmap is a valid handle.
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_draw_device(
    _ctx: Handle,
    _transform: super::geometry::fz_matrix,
    pixmap: Handle,
) -> Handle {
    // Get pixmap from handle
    if let Some(pm) = PIXMAPS.get(pixmap) {
        if let Ok(_guard) = pm.lock() {
            // Create a null device for now (draw device requires more infrastructure)
            let device: Box<dyn Device + Send + Sync> = Box::new(NullDevice);
            return DEVICES.insert(device);
        }
    }
    0
}

/// Create a bounding box device
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_bbox_device(_ctx: Handle, rect: *mut super::geometry::fz_rect) -> Handle {
    if rect.is_null() {
        return 0;
    }

    let device: Box<dyn Device + Send + Sync> = Box::new(BBoxDevice::new());
    DEVICES.insert(device)
}

/// Create a trace device (debug output)
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_trace_device(_ctx: Handle) -> Handle {
    let device: Box<dyn Device + Send + Sync> = Box::new(TraceDevice::new());
    DEVICES.insert(device)
}

/// Create a list device for recording display list
///
/// # Safety
/// Caller must ensure list is a valid handle.
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_list_device(_ctx: Handle, _list: Handle) -> Handle {
    // Create list device with default media box (will be updated when used)
    let mediabox = Rect::new(0.0, 0.0, 612.0, 792.0); // Letter size default
    let device: Box<dyn Device + Send + Sync> = Box::new(ListDevice::new(mediabox));
    DEVICES.insert(device)
}

/// Keep (increment ref) device
#[unsafe(no_mangle)]
pub extern "C" fn fz_keep_device(_ctx: Handle, dev: Handle) -> Handle {
    DEVICES.keep(dev)
}

/// Drop device reference
#[unsafe(no_mangle)]
pub extern "C" fn fz_drop_device(_ctx: Handle, dev: Handle) {
    let _ = DEVICES.remove(dev);
}

/// Close device (finalize rendering)
#[unsafe(no_mangle)]
pub extern "C" fn fz_close_device(_ctx: Handle, dev: Handle) {
    if let Some(device) = DEVICES.get(dev) {
        if let Ok(mut guard) = device.lock() {
            guard.close();
        }
    }
}

/// Begin rendering a tile
#[unsafe(no_mangle)]
pub extern "C" fn fz_begin_tile(
    _ctx: Handle,
    dev: Handle,
    area: super::geometry::fz_rect,
    view: super::geometry::fz_rect,
    xstep: f32,
    ystep: f32,
    transform: super::geometry::fz_matrix,
) -> i32 {
    if let Some(device) = DEVICES.get(dev) {
        if let Ok(mut guard) = device.lock() {
            let area_rect = Rect::new(area.x0, area.y0, area.x1, area.y1);
            let view_rect = Rect::new(view.x0, view.y0, view.x1, view.y1);
            let matrix = Matrix::new(
                transform.a,
                transform.b,
                transform.c,
                transform.d,
                transform.e,
                transform.f,
            );

            return guard.begin_tile(area_rect, view_rect, xstep, ystep, &matrix);
        }
    }
    0
}

/// End tile rendering
#[unsafe(no_mangle)]
pub extern "C" fn fz_end_tile(_ctx: Handle, dev: Handle) {
    if let Some(device) = DEVICES.get(dev) {
        if let Ok(mut guard) = device.lock() {
            guard.end_tile();
        }
    }
}

/// Fill a path
///
/// # Safety
/// Caller must ensure path and colorspace handles are valid.
#[unsafe(no_mangle)]
pub extern "C" fn fz_fill_path(
    _ctx: Handle,
    dev: Handle,
    path: Handle,
    even_odd: i32,
    transform: super::geometry::fz_matrix,
    colorspace: Handle,
    color: *const f32,
    alpha: f32,
) {
    if color.is_null() {
        return;
    }

    if let Some(device) = DEVICES.get(dev) {
        if let Ok(mut guard) = device.lock() {
            // Get path from handle
            if let Some(p) = super::path::PATHS.get(path) {
                if let Ok(path_guard) = p.lock() {
                    let matrix = Matrix::new(
                        transform.a,
                        transform.b,
                        transform.c,
                        transform.d,
                        transform.e,
                        transform.f,
                    );

                    // Get colorspace
                    if let Some(cs) = get_fitz_colorspace(colorspace) {
                        // Read color components
                        let n = cs.n() as usize;
                        let mut color_vec = vec![0.0; n];
                        unsafe {
                            for (i, item) in color_vec.iter_mut().enumerate() {
                                *item = *color.add(i);
                            }
                        }

                        guard.fill_path(
                            &path_guard,
                            even_odd != 0,
                            &matrix,
                            &cs,
                            &color_vec,
                            alpha,
                        );
                    }
                }
            }
        }
    }
}

/// Stroke a path
///
/// # Safety
/// Caller must ensure path, stroke, and colorspace handles are valid.
#[unsafe(no_mangle)]
pub extern "C" fn fz_stroke_path(
    _ctx: Handle,
    dev: Handle,
    path: Handle,
    stroke: Handle,
    transform: super::geometry::fz_matrix,
    colorspace: Handle,
    color: *const f32,
    alpha: f32,
) {
    if color.is_null() {
        return;
    }

    if let Some(device) = DEVICES.get(dev) {
        if let Ok(mut guard) = device.lock() {
            // Get path from handle
            if let Some(p) = super::path::PATHS.get(path) {
                if let Ok(path_guard) = p.lock() {
                    // Get stroke state
                    if let Some(s) = super::path::STROKE_STATES.get(stroke) {
                        if let Ok(stroke_guard) = s.lock() {
                            let matrix = Matrix::new(
                                transform.a,
                                transform.b,
                                transform.c,
                                transform.d,
                                transform.e,
                                transform.f,
                            );

                            // Get colorspace
                            if let Some(cs) = get_fitz_colorspace(colorspace) {
                                // Read color components
                                let n = cs.n() as usize;
                                let mut color_vec = vec![0.0; n];
                                unsafe {
                                    for (i, item) in color_vec.iter_mut().enumerate() {
                                        *item = *color.add(i);
                                    }
                                }

                                guard.stroke_path(
                                    &path_guard,
                                    &stroke_guard,
                                    &matrix,
                                    &cs,
                                    &color_vec,
                                    alpha,
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Clip to a path
///
/// # Safety
/// Caller must ensure path handle is valid.
#[unsafe(no_mangle)]
pub extern "C" fn fz_clip_path(
    _ctx: Handle,
    dev: Handle,
    path: Handle,
    even_odd: i32,
    transform: super::geometry::fz_matrix,
) {
    if let Some(device) = DEVICES.get(dev) {
        if let Ok(mut guard) = device.lock() {
            if let Some(p) = super::path::PATHS.get(path) {
                if let Ok(path_guard) = p.lock() {
                    let matrix = Matrix::new(
                        transform.a,
                        transform.b,
                        transform.c,
                        transform.d,
                        transform.e,
                        transform.f,
                    );

                    // Use infinite scissor rect for now
                    let scissor = Rect::new(-1e6, -1e6, 1e6, 1e6);
                    guard.clip_path(&path_guard, even_odd != 0, &matrix, scissor);
                }
            }
        }
    }
}

/// Clip to a stroke path
///
/// # Safety
/// Caller must ensure path and stroke handles are valid.
#[unsafe(no_mangle)]
pub extern "C" fn fz_clip_stroke_path(
    _ctx: Handle,
    dev: Handle,
    path: Handle,
    stroke: Handle,
    transform: super::geometry::fz_matrix,
) {
    if let Some(device) = DEVICES.get(dev) {
        if let Ok(mut guard) = device.lock() {
            if let Some(p) = super::path::PATHS.get(path) {
                if let Ok(path_guard) = p.lock() {
                    if let Some(s) = super::path::STROKE_STATES.get(stroke) {
                        if let Ok(stroke_guard) = s.lock() {
                            let matrix = Matrix::new(
                                transform.a,
                                transform.b,
                                transform.c,
                                transform.d,
                                transform.e,
                                transform.f,
                            );

                            // Use infinite scissor rect for now
                            let scissor = Rect::new(-1e6, -1e6, 1e6, 1e6);
                            guard.clip_stroke_path(&path_guard, &stroke_guard, &matrix, scissor);
                        }
                    }
                }
            }
        }
    }
}

/// Fill text
///
/// # Safety
/// Caller must ensure text and colorspace handles are valid.
#[unsafe(no_mangle)]
pub extern "C" fn fz_fill_text(
    _ctx: Handle,
    dev: Handle,
    text: Handle,
    transform: super::geometry::fz_matrix,
    colorspace: Handle,
    color: *const f32,
    alpha: f32,
) {
    if color.is_null() {
        return;
    }

    if let Some(device) = DEVICES.get(dev) {
        if let Ok(mut guard) = device.lock() {
            if let Some(t) = super::text::TEXTS.get(text) {
                if let Ok(text_guard) = t.lock() {
                    let matrix = Matrix::new(
                        transform.a,
                        transform.b,
                        transform.c,
                        transform.d,
                        transform.e,
                        transform.f,
                    );

                    // Get colorspace
                    if let Some(cs) = get_fitz_colorspace(colorspace) {
                        // Read color components
                        let n = cs.n() as usize;
                        let mut color_vec = vec![0.0; n];
                        unsafe {
                            for (i, item) in color_vec.iter_mut().enumerate() {
                                *item = *color.add(i);
                            }
                        }

                        guard.fill_text(&text_guard, &matrix, &cs, &color_vec, alpha);
                    }
                }
            }
        }
    }
}

/// Stroke text
///
/// # Safety
/// Caller must ensure text, stroke, and colorspace handles are valid.
#[unsafe(no_mangle)]
pub extern "C" fn fz_stroke_text(
    _ctx: Handle,
    dev: Handle,
    text: Handle,
    stroke: Handle,
    transform: super::geometry::fz_matrix,
    colorspace: Handle,
    color: *const f32,
    alpha: f32,
) {
    if color.is_null() {
        return;
    }

    if let Some(device) = DEVICES.get(dev) {
        if let Ok(mut guard) = device.lock() {
            if let Some(t) = super::text::TEXTS.get(text) {
                if let Ok(text_guard) = t.lock() {
                    if let Some(s) = super::path::STROKE_STATES.get(stroke) {
                        if let Ok(stroke_guard) = s.lock() {
                            let matrix = Matrix::new(
                                transform.a,
                                transform.b,
                                transform.c,
                                transform.d,
                                transform.e,
                                transform.f,
                            );

                            // Get colorspace
                            if let Some(cs) = get_fitz_colorspace(colorspace) {
                                // Read color components
                                let n = cs.n() as usize;
                                let mut color_vec = vec![0.0; n];
                                unsafe {
                                    for (i, item) in color_vec.iter_mut().enumerate() {
                                        *item = *color.add(i);
                                    }
                                }

                                guard.stroke_text(
                                    &text_guard,
                                    &stroke_guard,
                                    &matrix,
                                    &cs,
                                    &color_vec,
                                    alpha,
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Clip to text
///
/// # Safety
/// Caller must ensure text handle is valid.
#[unsafe(no_mangle)]
pub extern "C" fn fz_clip_text(
    _ctx: Handle,
    dev: Handle,
    text: Handle,
    transform: super::geometry::fz_matrix,
) {
    if let Some(device) = DEVICES.get(dev) {
        if let Ok(mut guard) = device.lock() {
            if let Some(t) = super::text::TEXTS.get(text) {
                if let Ok(text_guard) = t.lock() {
                    let matrix = Matrix::new(
                        transform.a,
                        transform.b,
                        transform.c,
                        transform.d,
                        transform.e,
                        transform.f,
                    );

                    // Use infinite scissor rect for now
                    let scissor = Rect::new(-1e6, -1e6, 1e6, 1e6);
                    guard.clip_text(&text_guard, &matrix, scissor);
                }
            }
        }
    }
}

/// Clip to stroke text
///
/// # Safety
/// Caller must ensure text and stroke handles are valid.
#[unsafe(no_mangle)]
pub extern "C" fn fz_clip_stroke_text(
    _ctx: Handle,
    dev: Handle,
    text: Handle,
    stroke: Handle,
    transform: super::geometry::fz_matrix,
) {
    if let Some(device) = DEVICES.get(dev) {
        if let Ok(mut guard) = device.lock() {
            if let Some(t) = super::text::TEXTS.get(text) {
                if let Ok(text_guard) = t.lock() {
                    if let Some(s) = super::path::STROKE_STATES.get(stroke) {
                        if let Ok(stroke_guard) = s.lock() {
                            let matrix = Matrix::new(
                                transform.a,
                                transform.b,
                                transform.c,
                                transform.d,
                                transform.e,
                                transform.f,
                            );

                            // Use infinite scissor rect for now
                            let scissor = Rect::new(-1e6, -1e6, 1e6, 1e6);
                            guard.clip_stroke_text(&text_guard, &stroke_guard, &matrix, scissor);
                        }
                    }
                }
            }
        }
    }
}

/// Ignore text (for search/extraction without rendering)
///
/// # Safety
/// Caller must ensure text handle is valid.
#[unsafe(no_mangle)]
pub extern "C" fn fz_ignore_text(
    _ctx: Handle,
    dev: Handle,
    text: Handle,
    transform: super::geometry::fz_matrix,
) {
    if let Some(device) = DEVICES.get(dev) {
        if let Ok(mut guard) = device.lock() {
            if let Some(t) = super::text::TEXTS.get(text) {
                if let Ok(text_guard) = t.lock() {
                    let matrix = Matrix::new(
                        transform.a,
                        transform.b,
                        transform.c,
                        transform.d,
                        transform.e,
                        transform.f,
                    );

                    guard.ignore_text(&text_guard, &matrix);
                }
            }
        }
    }
}

/// Fill an image
///
/// # Safety
/// Caller must ensure image and colorspace handles are valid.
#[unsafe(no_mangle)]
pub extern "C" fn fz_fill_image(
    _ctx: Handle,
    dev: Handle,
    image: Handle,
    transform: super::geometry::fz_matrix,
    alpha: f32,
) {
    if let Some(device) = DEVICES.get(dev) {
        if let Ok(mut guard) = device.lock() {
            if let Some(img) = super::image::IMAGES.get(image) {
                if let Ok(img_guard) = img.lock() {
                    let matrix = Matrix::new(
                        transform.a,
                        transform.b,
                        transform.c,
                        transform.d,
                        transform.e,
                        transform.f,
                    );

                    guard.fill_image(&img_guard, &matrix, alpha);
                }
            }
        }
    }
}

/// Fill an image mask
///
/// # Safety
/// Caller must ensure image and colorspace handles are valid.
#[unsafe(no_mangle)]
pub extern "C" fn fz_fill_image_mask(
    _ctx: Handle,
    dev: Handle,
    image: Handle,
    transform: super::geometry::fz_matrix,
    colorspace: Handle,
    color: *const f32,
    alpha: f32,
) {
    if color.is_null() {
        return;
    }

    if let Some(device) = DEVICES.get(dev) {
        if let Ok(mut guard) = device.lock() {
            if let Some(img) = super::image::IMAGES.get(image) {
                if let Ok(img_guard) = img.lock() {
                    let matrix = Matrix::new(
                        transform.a,
                        transform.b,
                        transform.c,
                        transform.d,
                        transform.e,
                        transform.f,
                    );

                    // Get colorspace
                    if let Some(cs) = get_fitz_colorspace(colorspace) {
                        // Read color components
                        let n = cs.n() as usize;
                        let mut color_vec = vec![0.0; n];
                        unsafe {
                            for (i, item) in color_vec.iter_mut().enumerate() {
                                *item = *color.add(i);
                            }
                        }

                        guard.fill_image_mask(&img_guard, &matrix, &cs, &color_vec, alpha);
                    }
                }
            }
        }
    }
}

/// Clip to image mask
///
/// # Safety
/// Caller must ensure image handle is valid.
#[unsafe(no_mangle)]
pub extern "C" fn fz_clip_image_mask(
    _ctx: Handle,
    dev: Handle,
    image: Handle,
    transform: super::geometry::fz_matrix,
) {
    if let Some(device) = DEVICES.get(dev) {
        if let Ok(mut guard) = device.lock() {
            if let Some(img) = super::image::IMAGES.get(image) {
                if let Ok(img_guard) = img.lock() {
                    let matrix = Matrix::new(
                        transform.a,
                        transform.b,
                        transform.c,
                        transform.d,
                        transform.e,
                        transform.f,
                    );

                    // Use infinite scissor rect for now
                    let scissor = Rect::new(-1e6, -1e6, 1e6, 1e6);
                    guard.clip_image_mask(&img_guard, &matrix, scissor);
                }
            }
        }
    }
}

/// Pop clip
#[unsafe(no_mangle)]
pub extern "C" fn fz_pop_clip(_ctx: Handle, dev: Handle) {
    if let Some(device) = DEVICES.get(dev) {
        if let Ok(mut guard) = device.lock() {
            guard.pop_clip();
        }
    }
}

/// Begin mask (transparency group)
#[unsafe(no_mangle)]
pub extern "C" fn fz_begin_mask(
    _ctx: Handle,
    dev: Handle,
    area: super::geometry::fz_rect,
    luminosity: i32,
    colorspace: Handle,
    color: *const f32,
) {
    if let Some(device) = DEVICES.get(dev) {
        if let Ok(mut guard) = device.lock() {
            let rect = Rect::new(area.x0, area.y0, area.x1, area.y1);

            // Get colorspace
            let cs = if !color.is_null() && colorspace != 0 {
                get_fitz_colorspace(colorspace)
            } else {
                None
            };

            // Read color if provided
            let color_vec = if !color.is_null() {
                if let Some(ref cs_ref) = cs {
                    let n = cs_ref.n() as usize;
                    let mut vec = vec![0.0; n];
                    unsafe {
                        for (i, item) in vec.iter_mut().enumerate() {
                            *item = *color.add(i);
                        }
                    }
                    Some(vec)
                } else {
                    None
                }
            } else {
                None
            };

            if let Some(ref cs_val) = cs {
                let color_slice = color_vec.as_deref().unwrap_or(&[]);
                guard.begin_mask(rect, luminosity != 0, cs_val, color_slice);
            }
        }
    }
}

/// End mask
#[unsafe(no_mangle)]
pub extern "C" fn fz_end_mask(_ctx: Handle, dev: Handle) {
    if let Some(device) = DEVICES.get(dev) {
        if let Ok(mut guard) = device.lock() {
            guard.end_mask();
        }
    }
}

/// Begin group (transparency group)
#[unsafe(no_mangle)]
pub extern "C" fn fz_begin_group(
    _ctx: Handle,
    dev: Handle,
    area: super::geometry::fz_rect,
    colorspace: Handle,
    isolated: i32,
    knockout: i32,
    blendmode: i32,
    alpha: f32,
) {
    if let Some(device) = DEVICES.get(dev) {
        if let Ok(mut guard) = device.lock() {
            let rect = Rect::new(area.x0, area.y0, area.x1, area.y1);

            // Get colorspace
            let cs = if colorspace != 0 {
                get_fitz_colorspace(colorspace)
            } else {
                None
            };

            // Convert blend mode int to BlendMode enum
            use crate::fitz::device::BlendMode;
            let blend = match blendmode {
                0 => BlendMode::Normal,
                1 => BlendMode::Multiply,
                2 => BlendMode::Screen,
                3 => BlendMode::Overlay,
                4 => BlendMode::Darken,
                5 => BlendMode::Lighten,
                6 => BlendMode::ColorDodge,
                7 => BlendMode::ColorBurn,
                8 => BlendMode::HardLight,
                9 => BlendMode::SoftLight,
                10 => BlendMode::Difference,
                11 => BlendMode::Exclusion,
                _ => BlendMode::Normal,
            };

            guard.begin_group(
                rect,
                cs.as_ref(),
                isolated != 0,
                knockout != 0,
                blend,
                alpha,
            );
        }
    }
}

/// End group
#[unsafe(no_mangle)]
pub extern "C" fn fz_end_group(_ctx: Handle, dev: Handle) {
    if let Some(device) = DEVICES.get(dev) {
        if let Ok(mut guard) = device.lock() {
            guard.end_group();
        }
    }
}

/// Check if device is valid
#[unsafe(no_mangle)]
pub extern "C" fn fz_device_is_valid(_ctx: Handle, dev: Handle) -> i32 {
    if DEVICES.get(dev).is_some() { 1 } else { 0 }
}

/// Get device name/type
#[unsafe(no_mangle)]
pub extern "C" fn fz_device_type(_ctx: Handle, dev: Handle) -> *const std::ffi::c_char {
    if DEVICES.get(dev).is_some() {
        c"generic_device".as_ptr()
    } else {
        c"invalid".as_ptr()
    }
}

/// Enable device hints
#[unsafe(no_mangle)]
pub extern "C" fn fz_enable_device_hints(_ctx: Handle, dev: Handle, hints: i32) {
    if DEVICES.get(dev).is_some() {
        if let Ok(mut hints_map) = DEVICE_HINTS.lock() {
            let current_hints = hints_map.get(&dev).copied().unwrap_or(0);
            hints_map.insert(dev, current_hints | hints);
        }
    }
}

/// Disable device hints
#[unsafe(no_mangle)]
pub extern "C" fn fz_disable_device_hints(_ctx: Handle, dev: Handle, hints: i32) {
    if DEVICES.get(dev).is_some() {
        if let Ok(mut hints_map) = DEVICE_HINTS.lock() {
            let current_hints = hints_map.get(&dev).copied().unwrap_or(0);
            hints_map.insert(dev, current_hints & !hints);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_bbox_device() {
        let mut rect = super::super::geometry::fz_rect {
            x0: 0.0,
            y0: 0.0,
            x1: 0.0,
            y1: 0.0,
        };
        let dev_handle = fz_new_bbox_device(0, &mut rect as *mut _);
        assert_ne!(dev_handle, 0);

        fz_drop_device(0, dev_handle);
    }

    #[test]
    fn test_new_trace_device() {
        let dev_handle = fz_new_trace_device(0);
        assert_ne!(dev_handle, 0);

        fz_drop_device(0, dev_handle);
    }

    #[test]
    fn test_keep_device() {
        let dev_handle = fz_new_trace_device(0);
        let kept = fz_keep_device(0, dev_handle);
        assert_eq!(kept, dev_handle);

        fz_drop_device(0, dev_handle);
    }

    #[test]
    fn test_close_device() {
        let dev_handle = fz_new_trace_device(0);
        fz_close_device(0, dev_handle);
        fz_drop_device(0, dev_handle);
    }

    #[test]
    fn test_pop_clip() {
        let dev_handle = fz_new_trace_device(0);
        fz_pop_clip(0, dev_handle);
        fz_drop_device(0, dev_handle);
    }

    #[test]
    fn test_end_mask() {
        let dev_handle = fz_new_trace_device(0);
        fz_end_mask(0, dev_handle);
        fz_drop_device(0, dev_handle);
    }

    #[test]
    fn test_end_group() {
        let dev_handle = fz_new_trace_device(0);
        fz_end_group(0, dev_handle);
        fz_drop_device(0, dev_handle);
    }
}
