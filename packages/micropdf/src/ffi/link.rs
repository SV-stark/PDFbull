//! FFI bindings for link operations
//!
//! Provides C-compatible API for interactive hyperlinks in documents.

use std::ffi::CStr;
use std::os::raw::c_char;
use std::sync::LazyLock;

use super::{Handle, HandleStore};
use crate::fitz::geometry::Rect;
use crate::fitz::link::{Link, LinkList};

/// Global storage for links
pub static LINKS: LazyLock<HandleStore<Link>> = LazyLock::new(HandleStore::new);

/// Global storage for link lists
pub static LINK_LISTS: LazyLock<HandleStore<LinkList>> = LazyLock::new(HandleStore::new);

/// Create a new link
///
/// # Arguments
/// * `rect` - Clickable area rectangle
/// * `uri` - URI or internal destination (null-terminated C string)
///
/// # Returns
/// Handle to the new link, or 0 on error
#[unsafe(no_mangle)]
pub extern "C" fn fz_create_link(
    _ctx: Handle,
    rect: super::geometry::fz_rect,
    uri: *const c_char,
) -> Handle {
    if uri.is_null() {
        return 0;
    }

    unsafe {
        if let Ok(uri_str) = CStr::from_ptr(uri).to_str() {
            let r = Rect::new(rect.x0, rect.y0, rect.x1, rect.y1);
            let link = Link::new(r, uri_str);
            return LINKS.insert(link);
        }
    }
    0
}

/// Increment reference count for a link
///
/// # Arguments
/// * `link` - Handle to the link
///
/// # Returns
/// The same handle
#[unsafe(no_mangle)]
pub extern "C" fn fz_keep_link(_ctx: Handle, link: Handle) -> Handle {
    link
}

/// Decrement reference count and free link if zero
///
/// # Arguments
/// * `link` - Handle to the link
#[unsafe(no_mangle)]
pub extern "C" fn fz_drop_link(_ctx: Handle, link: Handle) {
    LINKS.remove(link);
}

/// Get the rectangle (hotspot area) of a link
///
/// # Arguments
/// * `link` - Handle to the link
///
/// # Returns
/// Rectangle struct
#[unsafe(no_mangle)]
pub extern "C" fn fz_link_rect(_ctx: Handle, link: Handle) -> super::geometry::fz_rect {
    if let Some(l) = LINKS.get(link) {
        if let Ok(guard) = l.lock() {
            return super::geometry::fz_rect {
                x0: guard.rect.x0,
                y0: guard.rect.y0,
                x1: guard.rect.x1,
                y1: guard.rect.y1,
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

/// Get the URI of a link
///
/// # Arguments
/// * `link` - Handle to the link
/// * `buf` - Buffer to write URI into
/// * `bufsize` - Size of buffer
///
/// # Returns
/// Number of bytes written (excluding null terminator)
#[unsafe(no_mangle)]
pub extern "C" fn fz_link_uri(_ctx: Handle, link: Handle, buf: *mut c_char, bufsize: i32) -> i32 {
    if buf.is_null() || bufsize <= 0 {
        return 0;
    }

    if let Some(l) = LINKS.get(link) {
        if let Ok(guard) = l.lock() {
            let uri_bytes = guard.uri.as_bytes();
            let copy_len = uri_bytes.len().min((bufsize - 1) as usize);

            unsafe {
                std::ptr::copy_nonoverlapping(uri_bytes.as_ptr(), buf as *mut u8, copy_len);
                *buf.add(copy_len) = 0; // Null terminate
            }

            return copy_len as i32;
        }
    }
    0
}

/// Check if a link is external (has a scheme like http://, https://, etc.)
///
/// # Arguments
/// * `link` - Handle to the link
///
/// # Returns
/// 1 if external, 0 otherwise
#[unsafe(no_mangle)]
pub extern "C" fn fz_is_external_link(_ctx: Handle, link: Handle) -> i32 {
    if let Some(l) = LINKS.get(link) {
        if let Ok(guard) = l.lock() {
            return if guard.is_external() { 1 } else { 0 };
        }
    }
    0
}

/// Get the page number from an internal link
///
/// # Arguments
/// * `link` - Handle to the link
///
/// # Returns
/// Page number, or -1 if not a page link
#[unsafe(no_mangle)]
pub extern "C" fn fz_link_page_number(_ctx: Handle, link: Handle) -> i32 {
    if let Some(l) = LINKS.get(link) {
        if let Ok(guard) = l.lock() {
            return guard.page_number().unwrap_or(-1);
        }
    }
    -1
}

/// Check if a link is an internal page link
///
/// # Arguments
/// * `link` - Handle to the link
///
/// # Returns
/// 1 if page link, 0 otherwise
#[unsafe(no_mangle)]
pub extern "C" fn fz_is_page_link(_ctx: Handle, link: Handle) -> i32 {
    if let Some(l) = LINKS.get(link) {
        if let Ok(guard) = l.lock() {
            return if guard.is_page_link() { 1 } else { 0 };
        }
    }
    0
}

/// Create a new empty link list
///
/// # Returns
/// Handle to the new link list
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_link_list(_ctx: Handle) -> Handle {
    let list = LinkList::new();
    LINK_LISTS.insert(list)
}

/// Drop a link list
///
/// # Arguments
/// * `list` - Handle to the link list
#[unsafe(no_mangle)]
pub extern "C" fn fz_drop_link_list(_ctx: Handle, list: Handle) {
    LINK_LISTS.remove(list);
}

/// Add a link to a link list
///
/// # Arguments
/// * `list` - Handle to the link list
/// * `link` - Handle to the link to add
#[unsafe(no_mangle)]
pub extern "C" fn fz_link_list_add(_ctx: Handle, list: Handle, link: Handle) {
    if let Some(list_arc) = LINK_LISTS.get(list) {
        if let Some(link_arc) = LINKS.get(link) {
            if let (Ok(mut list_guard), Ok(link_guard)) = (list_arc.lock(), link_arc.lock()) {
                list_guard.push(link_guard.clone());
            }
        }
    }
}

/// Get the number of links in a link list
///
/// # Arguments
/// * `list` - Handle to the link list
///
/// # Returns
/// Number of links
#[unsafe(no_mangle)]
pub extern "C" fn fz_link_list_count(_ctx: Handle, list: Handle) -> i32 {
    if let Some(l) = LINK_LISTS.get(list) {
        if let Ok(guard) = l.lock() {
            return guard.len() as i32;
        }
    }
    0
}

/// Find a link at a specific point in a link list
///
/// # Arguments
/// * `list` - Handle to the link list
/// * `x` - X coordinate
/// * `y` - Y coordinate
///
/// # Returns
/// Handle to the link at that point, or 0 if none found
#[unsafe(no_mangle)]
pub extern "C" fn fz_link_list_find_at_point(_ctx: Handle, list: Handle, x: f32, y: f32) -> Handle {
    if let Some(l) = LINK_LISTS.get(list) {
        if let Ok(guard) = l.lock() {
            if let Some(link) = guard.link_at_point(x, y) {
                // Create a new handle for the found link
                return LINKS.insert(link.clone());
            }
        }
    }
    0
}

/// Check if a link list is empty
///
/// # Arguments
/// * `list` - Handle to the link list
///
/// # Returns
/// 1 if empty, 0 otherwise
#[unsafe(no_mangle)]
pub extern "C" fn fz_link_list_is_empty(_ctx: Handle, list: Handle) -> i32 {
    if let Some(l) = LINK_LISTS.get(list) {
        if let Ok(guard) = l.lock() {
            return if guard.is_empty() { 1 } else { 0 };
        }
    }
    1
}

/// Get the first link in a link list
#[unsafe(no_mangle)]
pub extern "C" fn fz_link_list_first(_ctx: Handle, list: Handle) -> Handle {
    if let Some(l) = LINK_LISTS.get(list) {
        if let Ok(guard) = l.lock() {
            if let Some(link) = guard.first() {
                return LINKS.insert(link.clone());
            }
        }
    }
    0
}

/// Get the link at a specific index
#[unsafe(no_mangle)]
pub extern "C" fn fz_link_list_get(_ctx: Handle, list: Handle, index: i32) -> Handle {
    if index < 0 {
        return 0;
    }
    if let Some(l) = LINK_LISTS.get(list) {
        if let Ok(guard) = l.lock() {
            if let Some(link) = guard.get(index as usize) {
                return LINKS.insert(link.clone());
            }
        }
    }
    0
}

/// Clone a link
#[unsafe(no_mangle)]
pub extern "C" fn fz_clone_link(_ctx: Handle, link: Handle) -> Handle {
    if let Some(l) = LINKS.get(link) {
        if let Ok(guard) = l.lock() {
            return LINKS.insert(guard.clone());
        }
    }
    0
}

/// Check if a link is valid
#[unsafe(no_mangle)]
pub extern "C" fn fz_link_is_valid(_ctx: Handle, link: Handle) -> i32 {
    if LINKS.get(link).is_some() { 1 } else { 0 }
}

/// Set the URI of a link
#[unsafe(no_mangle)]
pub extern "C" fn fz_set_link_uri(_ctx: Handle, link: Handle, uri: *const c_char) -> i32 {
    if uri.is_null() {
        return 0;
    }
    if let Some(l) = LINKS.get(link) {
        if let Ok(mut guard) = l.lock() {
            if let Ok(uri_str) = unsafe { CStr::from_ptr(uri).to_str() } {
                guard.uri = uri_str.to_string();
                return 1;
            }
        }
    }
    0
}

/// Set the rect of a link
#[unsafe(no_mangle)]
pub extern "C" fn fz_set_link_rect(_ctx: Handle, link: Handle, rect: super::geometry::fz_rect) {
    if let Some(l) = LINKS.get(link) {
        if let Ok(mut guard) = l.lock() {
            guard.rect = Rect::new(rect.x0, rect.y0, rect.x1, rect.y1);
        }
    }
}

/// Check if two links have the same URI
#[unsafe(no_mangle)]
pub extern "C" fn fz_link_eq(_ctx: Handle, link1: Handle, link2: Handle) -> i32 {
    if let (Some(l1), Some(l2)) = (LINKS.get(link1), LINKS.get(link2)) {
        if let (Ok(g1), Ok(g2)) = (l1.lock(), l2.lock()) {
            return if g1.uri == g2.uri { 1 } else { 0 };
        }
    }
    0
}

/// Clear all links from a link list
#[unsafe(no_mangle)]
pub extern "C" fn fz_link_list_clear(_ctx: Handle, list: Handle) {
    if let Some(l) = LINK_LISTS.get(list) {
        if let Ok(mut guard) = l.lock() {
            guard.clear();
        }
    }
}

/// Clone a link list
#[unsafe(no_mangle)]
pub extern "C" fn fz_clone_link_list(_ctx: Handle, list: Handle) -> Handle {
    if let Some(l) = LINK_LISTS.get(list) {
        if let Ok(guard) = l.lock() {
            let cloned = guard.clone();
            return LINK_LISTS.insert(cloned);
        }
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_link() {
        let rect = super::super::geometry::fz_rect {
            x0: 0.0,
            y0: 0.0,
            x1: 100.0,
            y1: 50.0,
        };
        let uri = c"https://example.com";
        let link = fz_create_link(0, rect, uri.as_ptr());
        assert_ne!(link, 0);
        fz_drop_link(0, link);
    }

    #[test]
    fn test_keep_link() {
        let rect = super::super::geometry::fz_rect {
            x0: 0.0,
            y0: 0.0,
            x1: 100.0,
            y1: 50.0,
        };
        let uri = c"https://example.com";
        let link = fz_create_link(0, rect, uri.as_ptr());
        let kept = fz_keep_link(0, link);
        assert_eq!(kept, link);
        fz_drop_link(0, link);
    }

    #[test]
    fn test_link_rect() {
        let rect = super::super::geometry::fz_rect {
            x0: 10.0,
            y0: 20.0,
            x1: 110.0,
            y1: 70.0,
        };
        let uri = c"https://example.com";
        let link = fz_create_link(0, rect, uri.as_ptr());

        let retrieved_rect = fz_link_rect(0, link);
        assert_eq!(retrieved_rect.x0, 10.0);
        assert_eq!(retrieved_rect.y0, 20.0);
        assert_eq!(retrieved_rect.x1, 110.0);
        assert_eq!(retrieved_rect.y1, 70.0);

        fz_drop_link(0, link);
    }

    #[test]
    fn test_link_uri() {
        let rect = super::super::geometry::fz_rect {
            x0: 0.0,
            y0: 0.0,
            x1: 100.0,
            y1: 50.0,
        };
        let uri = c"https://example.com";
        let link = fz_create_link(0, rect, uri.as_ptr());

        let mut buf = [0i8; 256];
        let len = fz_link_uri(0, link, buf.as_mut_ptr(), 256);
        assert!(len > 0);

        // Verify the URI was copied
        let uri_str = unsafe { CStr::from_ptr(buf.as_ptr()) }.to_str().unwrap();
        assert_eq!(uri_str, "https://example.com");

        fz_drop_link(0, link);
    }

    #[test]
    fn test_is_external_link() {
        let rect = super::super::geometry::fz_rect {
            x0: 0.0,
            y0: 0.0,
            x1: 100.0,
            y1: 50.0,
        };

        let external = fz_create_link(0, rect, c"https://example.com".as_ptr());
        assert_eq!(fz_is_external_link(0, external), 1);
        fz_drop_link(0, external);

        let internal = fz_create_link(0, rect, c"#page=5".as_ptr());
        assert_eq!(fz_is_external_link(0, internal), 0);
        fz_drop_link(0, internal);
    }

    #[test]
    fn test_is_page_link() {
        let rect = super::super::geometry::fz_rect {
            x0: 0.0,
            y0: 0.0,
            x1: 100.0,
            y1: 50.0,
        };

        let page_link = fz_create_link(0, rect, c"#page=5".as_ptr());
        assert_eq!(fz_is_page_link(0, page_link), 1);
        fz_drop_link(0, page_link);

        let external = fz_create_link(0, rect, c"https://example.com".as_ptr());
        assert_eq!(fz_is_page_link(0, external), 0);
        fz_drop_link(0, external);
    }

    #[test]
    fn test_link_page_number() {
        let rect = super::super::geometry::fz_rect {
            x0: 0.0,
            y0: 0.0,
            x1: 100.0,
            y1: 50.0,
        };

        let link1 = fz_create_link(0, rect, c"#page=5".as_ptr());
        assert_eq!(fz_link_page_number(0, link1), 5);
        fz_drop_link(0, link1);

        let link2 = fz_create_link(0, rect, c"#10".as_ptr());
        assert_eq!(fz_link_page_number(0, link2), 10);
        fz_drop_link(0, link2);

        let external = fz_create_link(0, rect, c"https://example.com".as_ptr());
        assert_eq!(fz_link_page_number(0, external), -1);
        fz_drop_link(0, external);
    }

    #[test]
    fn test_new_link_list() {
        let list = fz_new_link_list(0);
        assert_ne!(list, 0);
        fz_drop_link_list(0, list);
    }

    #[test]
    fn test_link_list_operations() {
        let list = fz_new_link_list(0);

        // Check initially empty
        assert_eq!(fz_link_list_is_empty(0, list), 1);
        assert_eq!(fz_link_list_count(0, list), 0);

        // Add a link
        let rect = super::super::geometry::fz_rect {
            x0: 0.0,
            y0: 0.0,
            x1: 100.0,
            y1: 50.0,
        };
        let link = fz_create_link(0, rect, c"#page=1".as_ptr());
        fz_link_list_add(0, list, link);

        // Check not empty
        assert_eq!(fz_link_list_is_empty(0, list), 0);
        assert_eq!(fz_link_list_count(0, list), 1);

        // Clean up
        fz_drop_link(0, link);
        fz_drop_link_list(0, list);
    }

    #[test]
    fn test_link_list_find_at_point() {
        let list = fz_new_link_list(0);

        // Add links at different positions
        let rect1 = super::super::geometry::fz_rect {
            x0: 0.0,
            y0: 0.0,
            x1: 100.0,
            y1: 100.0,
        };
        let link1 = fz_create_link(0, rect1, c"#page=1".as_ptr());
        fz_link_list_add(0, list, link1);

        let rect2 = super::super::geometry::fz_rect {
            x0: 200.0,
            y0: 200.0,
            x1: 300.0,
            y1: 300.0,
        };
        let link2 = fz_create_link(0, rect2, c"#page=2".as_ptr());
        fz_link_list_add(0, list, link2);

        // Find link at point inside first rect
        let found1 = fz_link_list_find_at_point(0, list, 50.0, 50.0);
        assert_ne!(found1, 0);
        fz_drop_link(0, found1);

        // Find link at point inside second rect
        let found2 = fz_link_list_find_at_point(0, list, 250.0, 250.0);
        assert_ne!(found2, 0);
        fz_drop_link(0, found2);

        // No link at this point
        let not_found = fz_link_list_find_at_point(0, list, 500.0, 500.0);
        assert_eq!(not_found, 0);

        // Clean up
        fz_drop_link(0, link1);
        fz_drop_link(0, link2);
        fz_drop_link_list(0, list);
    }
}
