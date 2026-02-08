//! FFI bindings for display list operations
//!
//! Display lists record drawing operations for caching and playback.

use std::sync::LazyLock;

use super::{Handle, HandleStore};
use crate::fitz::display_list::DisplayList;
use crate::fitz::geometry::{Matrix, Rect};

/// Global storage for display lists
pub static DISPLAY_LISTS: LazyLock<HandleStore<DisplayList>> = LazyLock::new(HandleStore::new);

/// Create a new display list with specified media box
///
/// # Arguments
/// * `x0`, `y0`, `x1`, `y1` - Media box coordinates
///
/// # Returns
/// Handle to the new display list, or 0 on error
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_display_list(_ctx: Handle, x0: f32, y0: f32, x1: f32, y1: f32) -> Handle {
    let mediabox = Rect::new(x0, y0, x1, y1);
    let list = DisplayList::new(mediabox);
    DISPLAY_LISTS.insert(list)
}

/// Increment reference count for a display list
///
/// # Arguments
/// * `list` - Handle to the display list
///
/// # Returns
/// The same handle
#[unsafe(no_mangle)]
pub extern "C" fn fz_keep_display_list(_ctx: Handle, list: Handle) -> Handle {
    list
}

/// Decrement reference count and free display list if zero
///
/// # Arguments
/// * `list` - Handle to the display list
#[unsafe(no_mangle)]
pub extern "C" fn fz_drop_display_list(_ctx: Handle, list: Handle) {
    DISPLAY_LISTS.remove(list);
}

/// Get the media box of a display list
///
/// # Arguments
/// * `list` - Handle to the display list
///
/// # Returns
/// Media box rectangle
#[unsafe(no_mangle)]
pub extern "C" fn fz_bound_display_list(_ctx: Handle, list: Handle) -> super::geometry::fz_rect {
    if let Some(l) = DISPLAY_LISTS.get(list) {
        if let Ok(guard) = l.lock() {
            let mediabox = guard.mediabox();
            return super::geometry::fz_rect {
                x0: mediabox.x0,
                y0: mediabox.y0,
                x1: mediabox.x1,
                y1: mediabox.y1,
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

/// Run a display list through a device
///
/// # Arguments
/// * `list` - Handle to the display list
/// * `dev` - Handle to the device to run the list on
/// * `ctm` - Current transformation matrix (use identity matrix for no transform)
/// * `scissor` - Scissor rectangle for clipping
#[unsafe(no_mangle)]
pub extern "C" fn fz_run_display_list(
    _ctx: Handle,
    list: Handle,
    dev: Handle,
    ctm: super::geometry::fz_matrix,
    scissor: super::geometry::fz_rect,
) {
    if let Some(l) = DISPLAY_LISTS.get(list) {
        if let Ok(list_guard) = l.lock() {
            // Get the device from one of the device stores
            let matrix = Matrix {
                a: ctm.a,
                b: ctm.b,
                c: ctm.c,
                d: ctm.d,
                e: ctm.e,
                f: ctm.f,
            };
            let rect = Rect::new(scissor.x0, scissor.y0, scissor.x1, scissor.y1);

            // Try to get device from DEVICES store
            if let Some(device) = super::device::DEVICES.get(dev) {
                if let Ok(mut dev_guard) = device.lock() {
                    list_guard.run(&mut **dev_guard, &matrix, rect);
                }
            }
        }
    }
}

/// Get the number of commands in a display list
///
/// # Arguments
/// * `list` - Handle to the display list
///
/// # Returns
/// Number of commands
#[unsafe(no_mangle)]
pub extern "C" fn fz_display_list_count_commands(_ctx: Handle, list: Handle) -> i32 {
    if let Some(l) = DISPLAY_LISTS.get(list) {
        if let Ok(guard) = l.lock() {
            return guard.len() as i32;
        }
    }
    0
}

/// Check if a display list is empty
///
/// # Arguments
/// * `list` - Handle to the display list
///
/// # Returns
/// 1 if empty, 0 otherwise
#[unsafe(no_mangle)]
pub extern "C" fn fz_display_list_is_empty(_ctx: Handle, list: Handle) -> i32 {
    if let Some(l) = DISPLAY_LISTS.get(list) {
        if let Ok(guard) = l.lock() {
            return if guard.is_empty() { 1 } else { 0 };
        }
    }
    0
}

/// Clear all commands from a display list
///
/// # Arguments
/// * `list` - Handle to the display list
#[unsafe(no_mangle)]
pub extern "C" fn fz_display_list_clear(_ctx: Handle, list: Handle) {
    if let Some(l) = DISPLAY_LISTS.get(list) {
        if let Ok(mut guard) = l.lock() {
            guard.clear();
        }
    }
}

/// Check if a display list is valid
///
/// # Arguments
/// * `list` - Handle to check
///
/// # Returns
/// 1 if valid, 0 otherwise
#[unsafe(no_mangle)]
pub extern "C" fn fz_display_list_is_valid(_ctx: Handle, list: Handle) -> i32 {
    if DISPLAY_LISTS.get(list).is_some() {
        1
    } else {
        0
    }
}

/// Clone a display list (create a new copy)
///
/// # Arguments
/// * `list` - Handle to the display list
///
/// # Returns
/// Handle to the cloned display list, or 0 on error
#[unsafe(no_mangle)]
pub extern "C" fn fz_clone_display_list(_ctx: Handle, list: Handle) -> Handle {
    if let Some(l) = DISPLAY_LISTS.get(list) {
        if let Ok(guard) = l.lock() {
            let cloned = DisplayList::new(guard.mediabox());
            return DISPLAY_LISTS.insert(cloned);
        }
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ffi::device::fz_new_bbox_device;

    #[test]
    fn test_new_display_list() {
        let list = fz_new_display_list(0, 0.0, 0.0, 100.0, 100.0);
        assert_ne!(list, 0);
        fz_drop_display_list(0, list);
    }

    #[test]
    fn test_keep_display_list() {
        let list = fz_new_display_list(0, 0.0, 0.0, 100.0, 100.0);
        let kept = fz_keep_display_list(0, list);
        assert_eq!(kept, list);
        fz_drop_display_list(0, list);
    }

    #[test]
    fn test_bound_display_list() {
        let list = fz_new_display_list(0, 10.0, 20.0, 100.0, 200.0);
        let bounds = fz_bound_display_list(0, list);
        assert_eq!(bounds.x0, 10.0);
        assert_eq!(bounds.y0, 20.0);
        assert_eq!(bounds.x1, 100.0);
        assert_eq!(bounds.y1, 200.0);
        fz_drop_display_list(0, list);
    }

    #[test]
    fn test_display_list_count_commands() {
        let list = fz_new_display_list(0, 0.0, 0.0, 100.0, 100.0);
        let count = fz_display_list_count_commands(0, list);
        assert_eq!(count, 0); // New list is empty
        fz_drop_display_list(0, list);
    }

    #[test]
    fn test_display_list_is_empty() {
        let list = fz_new_display_list(0, 0.0, 0.0, 100.0, 100.0);
        let empty = fz_display_list_is_empty(0, list);
        assert_eq!(empty, 1); // New list is empty
        fz_drop_display_list(0, list);
    }

    #[test]
    fn test_display_list_clear() {
        let list = fz_new_display_list(0, 0.0, 0.0, 100.0, 100.0);
        fz_display_list_clear(0, list);
        let empty = fz_display_list_is_empty(0, list);
        assert_eq!(empty, 1);
        fz_drop_display_list(0, list);
    }

    #[test]
    fn test_run_display_list() {
        // Create an empty display list
        let list = fz_new_display_list(0, 0.0, 0.0, 100.0, 100.0);

        // Create a bbox device to run the list on
        let mut bbox = super::super::geometry::fz_rect {
            x0: 0.0,
            y0: 0.0,
            x1: 0.0,
            y1: 0.0,
        };
        let bbox_dev = fz_new_bbox_device(0, &mut bbox as *mut _);

        // Run the list
        let identity = super::super::geometry::fz_matrix::identity();
        let infinite = super::super::geometry::fz_rect {
            x0: f32::NEG_INFINITY,
            y0: f32::NEG_INFINITY,
            x1: f32::INFINITY,
            y1: f32::INFINITY,
        };
        fz_run_display_list(0, list, bbox_dev, identity, infinite);

        // Clean up
        crate::ffi::device::fz_drop_device(0, bbox_dev);
        fz_drop_display_list(0, list);
    }
}
