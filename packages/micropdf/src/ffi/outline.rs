//! FFI bindings for fz_outline (Document Outlines/TOC)
//!
//! This module provides C-compatible exports for document outline/TOC operations.
//! Outlines represent the hierarchical table of contents of a document.

use super::{Handle, HandleStore};
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::sync::LazyLock;

// ============================================================================
// Types and Structures
// ============================================================================

/// Outline item flags
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutlineFlag {
    None = 0,
    Bold = 1,
    Italic = 2,
}

/// Iterator position result
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutlineIteratorResult {
    DidNotMove = -1,
    AtItem = 0,
    AtEmpty = 1,
}

/// Page location for outline destination
#[derive(Debug, Clone, Default)]
pub struct Location {
    pub chapter: i32,
    pub page: i32,
}

/// Outline item - temporary structure for iterator operations
#[derive(Debug, Clone, Default)]
pub struct OutlineItem {
    pub title: Option<String>,
    pub uri: Option<String>,
    pub is_open: bool,
    pub flags: u8,
    pub r: f32,
    pub g: f32,
    pub b: f32,
}

/// C-compatible outline item for FFI
#[repr(C)]
pub struct FzOutlineItem {
    pub title: *mut c_char,
    pub uri: *mut c_char,
    pub is_open: i32,
    pub flags: i32,
    pub r: f32,
    pub g: f32,
    pub b: f32,
}

impl Default for FzOutlineItem {
    fn default() -> Self {
        Self {
            title: std::ptr::null_mut(),
            uri: std::ptr::null_mut(),
            is_open: 0,
            flags: 0,
            r: 0.0,
            g: 0.0,
            b: 0.0,
        }
    }
}

/// Outline node - tree structure for document TOC
#[derive(Debug, Clone)]
pub struct Outline {
    pub refs: i32,
    pub title: Option<String>,
    pub uri: Option<String>,
    pub page: Location,
    pub x: f32,
    pub y: f32,
    pub next: Option<Handle>,
    pub down: Option<Handle>,
    pub is_open: bool,
    pub flags: u8,
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Default for Outline {
    fn default() -> Self {
        Self {
            refs: 1,
            title: None,
            uri: None,
            page: Location::default(),
            x: 0.0,
            y: 0.0,
            next: None,
            down: None,
            is_open: false,
            flags: 0,
            r: 0,
            g: 0,
            b: 0,
        }
    }
}

/// Outline iterator for traversing/modifying outline tree
#[derive(Debug)]
pub struct OutlineIterator {
    /// Stack of (outline_handle, child_index) for navigation
    stack: Vec<(Handle, usize)>,
    /// Current outline handle
    current: Option<Handle>,
    /// Document handle (if associated)
    #[allow(dead_code)]
    document: Option<Handle>,
    /// Current item cache
    current_item: Option<OutlineItem>,
}

impl OutlineIterator {
    pub fn new() -> Self {
        Self {
            stack: Vec::new(),
            current: None,
            document: None,
            current_item: None,
        }
    }

    pub fn from_outline(outline: Handle) -> Self {
        Self {
            stack: Vec::new(),
            current: Some(outline),
            document: None,
            current_item: None,
        }
    }

    /// Get current item
    pub fn item(&mut self) -> Option<&OutlineItem> {
        if let Some(handle) = self.current {
            if let Some(outline_arc) = OUTLINES.get(handle) {
                if let Ok(outline) = outline_arc.lock() {
                    self.current_item = Some(OutlineItem {
                        title: outline.title.clone(),
                        uri: outline.uri.clone(),
                        is_open: outline.is_open,
                        flags: outline.flags,
                        r: outline.r as f32 / 255.0,
                        g: outline.g as f32 / 255.0,
                        b: outline.b as f32 / 255.0,
                    });
                    return self.current_item.as_ref();
                }
            }
        }
        None
    }

    /// Move to next sibling
    pub fn next(&mut self) -> i32 {
        if let Some(handle) = self.current {
            if let Some(outline_arc) = OUTLINES.get(handle) {
                if let Ok(outline) = outline_arc.lock() {
                    if let Some(next_handle) = outline.next {
                        self.current = Some(next_handle);
                        return OutlineIteratorResult::AtItem as i32;
                    }
                }
            }
        }
        OutlineIteratorResult::DidNotMove as i32
    }

    /// Move to previous sibling
    pub fn prev(&mut self) -> i32 {
        // To find previous, we need to track parent or iterate from start
        // For simplicity, return did not move if no tracking
        if let Some((parent_handle, idx)) = self.stack.last() {
            if *idx > 0 {
                // Find the previous sibling by iterating from parent's down
                if let Some(parent_arc) = OUTLINES.get(*parent_handle) {
                    if let Ok(parent) = parent_arc.lock() {
                        if let Some(first_child) = parent.down {
                            let mut current = first_child;
                            for _ in 0..(idx - 1) {
                                if let Some(arc) = OUTLINES.get(current) {
                                    if let Ok(node) = arc.lock() {
                                        if let Some(next) = node.next {
                                            current = next;
                                        } else {
                                            return OutlineIteratorResult::DidNotMove as i32;
                                        }
                                    }
                                }
                            }
                            self.current = Some(current);
                            return OutlineIteratorResult::AtItem as i32;
                        }
                    }
                }
            }
        }
        OutlineIteratorResult::DidNotMove as i32
    }

    /// Move up to parent
    pub fn up(&mut self) -> i32 {
        if let Some((parent_handle, _)) = self.stack.pop() {
            self.current = Some(parent_handle);
            return OutlineIteratorResult::AtItem as i32;
        }
        OutlineIteratorResult::DidNotMove as i32
    }

    /// Move down to first child
    pub fn down(&mut self) -> i32 {
        if let Some(handle) = self.current {
            if let Some(outline_arc) = OUTLINES.get(handle) {
                if let Ok(outline) = outline_arc.lock() {
                    if let Some(down_handle) = outline.down {
                        self.stack.push((handle, 0));
                        self.current = Some(down_handle);
                        return OutlineIteratorResult::AtItem as i32;
                    }
                }
            }
        }
        OutlineIteratorResult::DidNotMove as i32
    }

    /// Insert item before current position
    pub fn insert(&mut self, item: &OutlineItem) -> i32 {
        let new_outline = Outline {
            refs: 1,
            title: item.title.clone(),
            uri: item.uri.clone(),
            page: Location::default(),
            x: 0.0,
            y: 0.0,
            next: self.current,
            down: None,
            is_open: false, // New items are always closed
            flags: item.flags,
            r: (item.r * 255.0) as u8,
            g: (item.g * 255.0) as u8,
            b: (item.b * 255.0) as u8,
        };

        let new_handle = OUTLINES.insert(new_outline);

        // Update parent's down pointer or previous sibling's next pointer
        if let Some((parent_handle, idx)) = self.stack.last_mut() {
            if *idx == 0 {
                // First child - update parent's down
                if let Some(parent_arc) = OUTLINES.get(*parent_handle) {
                    if let Ok(mut parent) = parent_arc.lock() {
                        parent.down = Some(new_handle);
                    }
                }
            }
            *idx += 1;
        }

        OutlineIteratorResult::AtItem as i32
    }

    /// Delete current item
    pub fn delete(&mut self) -> i32 {
        if let Some(handle) = self.current {
            // Move to next before deleting
            let next_result = self.next();
            OUTLINES.remove(handle);
            return next_result;
        }
        OutlineIteratorResult::DidNotMove as i32
    }

    /// Update current item
    pub fn update(&mut self, item: &OutlineItem) {
        if let Some(handle) = self.current {
            if let Some(outline_arc) = OUTLINES.get(handle) {
                if let Ok(mut outline) = outline_arc.lock() {
                    outline.title = item.title.clone();
                    outline.uri = item.uri.clone();
                    outline.is_open = item.is_open;
                    outline.flags = item.flags;
                    outline.r = (item.r * 255.0) as u8;
                    outline.g = (item.g * 255.0) as u8;
                    outline.b = (item.b * 255.0) as u8;
                }
            }
        }
    }
}

impl Default for OutlineIterator {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Handle Stores
// ============================================================================

pub static OUTLINES: LazyLock<HandleStore<Outline>> = LazyLock::new(HandleStore::new);
pub static OUTLINE_ITERATORS: LazyLock<HandleStore<OutlineIterator>> =
    LazyLock::new(HandleStore::new);

// Thread-local storage for current item strings (to return stable pointers)
thread_local! {
    static CURRENT_TITLE: std::cell::RefCell<Option<CString>> = const { std::cell::RefCell::new(None) };
    static CURRENT_URI: std::cell::RefCell<Option<CString>> = const { std::cell::RefCell::new(None) };
}

// ============================================================================
// Outline Structure API
// ============================================================================

/// Create a new outline entry with zeroed fields
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_outline(_ctx: Handle) -> Handle {
    OUTLINES.insert(Outline::default())
}

/// Increment the reference count
#[unsafe(no_mangle)]
pub extern "C" fn fz_keep_outline(_ctx: Handle, outline: Handle) -> Handle {
    if let Some(arc) = OUTLINES.get(outline) {
        if let Ok(mut o) = arc.lock() {
            o.refs += 1;
        }
    }
    outline
}

/// Decrement the reference count
#[unsafe(no_mangle)]
pub extern "C" fn fz_drop_outline(_ctx: Handle, outline: Handle) {
    if outline == 0 {
        return;
    }

    let should_drop = if let Some(arc) = OUTLINES.get(outline) {
        if let Ok(mut o) = arc.lock() {
            o.refs -= 1;
            o.refs <= 0
        } else {
            false
        }
    } else {
        false
    };

    if should_drop {
        // Get next and down before removing
        let (next, down) = if let Some(arc) = OUTLINES.get(outline) {
            if let Ok(o) = arc.lock() {
                (o.next, o.down)
            } else {
                (None, None)
            }
        } else {
            (None, None)
        };

        OUTLINES.remove(outline);

        // Recursively drop linked outlines
        if let Some(next_handle) = next {
            fz_drop_outline(_ctx, next_handle);
        }
        if let Some(down_handle) = down {
            fz_drop_outline(_ctx, down_handle);
        }
    }
}

/// Get outline title
#[unsafe(no_mangle)]
pub extern "C" fn fz_outline_title(_ctx: Handle, outline: Handle) -> *const c_char {
    if let Some(arc) = OUTLINES.get(outline) {
        if let Ok(o) = arc.lock() {
            if let Some(ref title) = o.title {
                if let Ok(cstr) = CString::new(title.as_str()) {
                    CURRENT_TITLE.with(|cell| {
                        *cell.borrow_mut() = Some(cstr);
                    });
                    return CURRENT_TITLE.with(|cell| {
                        cell.borrow()
                            .as_ref()
                            .map(|s| s.as_ptr())
                            .unwrap_or(std::ptr::null())
                    });
                }
            }
        }
    }
    std::ptr::null()
}

/// Set outline title
#[unsafe(no_mangle)]
pub extern "C" fn fz_set_outline_title(_ctx: Handle, outline: Handle, title: *const c_char) {
    if let Some(arc) = OUTLINES.get(outline) {
        if let Ok(mut o) = arc.lock() {
            if title.is_null() {
                o.title = None;
            } else {
                let cstr = unsafe { CStr::from_ptr(title) };
                o.title = cstr.to_str().ok().map(String::from);
            }
        }
    }
}

/// Get outline URI
#[unsafe(no_mangle)]
pub extern "C" fn fz_outline_uri(_ctx: Handle, outline: Handle) -> *const c_char {
    if let Some(arc) = OUTLINES.get(outline) {
        if let Ok(o) = arc.lock() {
            if let Some(ref uri) = o.uri {
                if let Ok(cstr) = CString::new(uri.as_str()) {
                    CURRENT_URI.with(|cell| {
                        *cell.borrow_mut() = Some(cstr);
                    });
                    return CURRENT_URI.with(|cell| {
                        cell.borrow()
                            .as_ref()
                            .map(|s| s.as_ptr())
                            .unwrap_or(std::ptr::null())
                    });
                }
            }
        }
    }
    std::ptr::null()
}

/// Set outline URI
#[unsafe(no_mangle)]
pub extern "C" fn fz_set_outline_uri(_ctx: Handle, outline: Handle, uri: *const c_char) {
    if let Some(arc) = OUTLINES.get(outline) {
        if let Ok(mut o) = arc.lock() {
            if uri.is_null() {
                o.uri = None;
            } else {
                let cstr = unsafe { CStr::from_ptr(uri) };
                o.uri = cstr.to_str().ok().map(String::from);
            }
        }
    }
}

/// Get outline page location
#[unsafe(no_mangle)]
pub extern "C" fn fz_outline_page(_ctx: Handle, outline: Handle) -> i32 {
    if let Some(arc) = OUTLINES.get(outline) {
        if let Ok(o) = arc.lock() {
            return o.page.page;
        }
    }
    -1
}

/// Set outline page location  
#[unsafe(no_mangle)]
pub extern "C" fn fz_set_outline_page(_ctx: Handle, outline: Handle, chapter: i32, page: i32) {
    if let Some(arc) = OUTLINES.get(outline) {
        if let Ok(mut o) = arc.lock() {
            o.page.chapter = chapter;
            o.page.page = page;
        }
    }
}

/// Get outline is_open flag
#[unsafe(no_mangle)]
pub extern "C" fn fz_outline_is_open(_ctx: Handle, outline: Handle) -> i32 {
    if let Some(arc) = OUTLINES.get(outline) {
        if let Ok(o) = arc.lock() {
            return o.is_open as i32;
        }
    }
    0
}

/// Set outline is_open flag
#[unsafe(no_mangle)]
pub extern "C" fn fz_set_outline_is_open(_ctx: Handle, outline: Handle, is_open: i32) {
    if let Some(arc) = OUTLINES.get(outline) {
        if let Ok(mut o) = arc.lock() {
            o.is_open = is_open != 0;
        }
    }
}

/// Get outline flags
#[unsafe(no_mangle)]
pub extern "C" fn fz_outline_flags(_ctx: Handle, outline: Handle) -> i32 {
    if let Some(arc) = OUTLINES.get(outline) {
        if let Ok(o) = arc.lock() {
            return o.flags as i32;
        }
    }
    0
}

/// Set outline flags
#[unsafe(no_mangle)]
pub extern "C" fn fz_set_outline_flags(_ctx: Handle, outline: Handle, flags: i32) {
    if let Some(arc) = OUTLINES.get(outline) {
        if let Ok(mut o) = arc.lock() {
            o.flags = flags as u8;
        }
    }
}

/// Get outline color (r component)
#[unsafe(no_mangle)]
pub extern "C" fn fz_outline_color_r(_ctx: Handle, outline: Handle) -> f32 {
    if let Some(arc) = OUTLINES.get(outline) {
        if let Ok(o) = arc.lock() {
            return o.r as f32 / 255.0;
        }
    }
    0.0
}

/// Get outline color (g component)
#[unsafe(no_mangle)]
pub extern "C" fn fz_outline_color_g(_ctx: Handle, outline: Handle) -> f32 {
    if let Some(arc) = OUTLINES.get(outline) {
        if let Ok(o) = arc.lock() {
            return o.g as f32 / 255.0;
        }
    }
    0.0
}

/// Get outline color (b component)
#[unsafe(no_mangle)]
pub extern "C" fn fz_outline_color_b(_ctx: Handle, outline: Handle) -> f32 {
    if let Some(arc) = OUTLINES.get(outline) {
        if let Ok(o) = arc.lock() {
            return o.b as f32 / 255.0;
        }
    }
    0.0
}

/// Set outline color
#[unsafe(no_mangle)]
pub extern "C" fn fz_set_outline_color(_ctx: Handle, outline: Handle, r: f32, g: f32, b: f32) {
    if let Some(arc) = OUTLINES.get(outline) {
        if let Ok(mut o) = arc.lock() {
            o.r = (r.clamp(0.0, 1.0) * 255.0) as u8;
            o.g = (g.clamp(0.0, 1.0) * 255.0) as u8;
            o.b = (b.clamp(0.0, 1.0) * 255.0) as u8;
        }
    }
}

/// Get next sibling outline
#[unsafe(no_mangle)]
pub extern "C" fn fz_outline_next(_ctx: Handle, outline: Handle) -> Handle {
    if let Some(arc) = OUTLINES.get(outline) {
        if let Ok(o) = arc.lock() {
            return o.next.unwrap_or(0);
        }
    }
    0
}

/// Set next sibling outline
#[unsafe(no_mangle)]
pub extern "C" fn fz_set_outline_next(_ctx: Handle, outline: Handle, next: Handle) {
    if let Some(arc) = OUTLINES.get(outline) {
        if let Ok(mut o) = arc.lock() {
            o.next = if next == 0 { None } else { Some(next) };
        }
    }
}

/// Get first child outline
#[unsafe(no_mangle)]
pub extern "C" fn fz_outline_down(_ctx: Handle, outline: Handle) -> Handle {
    if let Some(arc) = OUTLINES.get(outline) {
        if let Ok(o) = arc.lock() {
            return o.down.unwrap_or(0);
        }
    }
    0
}

/// Set first child outline
#[unsafe(no_mangle)]
pub extern "C" fn fz_set_outline_down(_ctx: Handle, outline: Handle, down: Handle) {
    if let Some(arc) = OUTLINES.get(outline) {
        if let Ok(mut o) = arc.lock() {
            o.down = if down == 0 { None } else { Some(down) };
        }
    }
}

/// Get outline destination X coordinate
#[unsafe(no_mangle)]
pub extern "C" fn fz_outline_x(_ctx: Handle, outline: Handle) -> f32 {
    if let Some(arc) = OUTLINES.get(outline) {
        if let Ok(o) = arc.lock() {
            return o.x;
        }
    }
    0.0
}

/// Get outline destination Y coordinate
#[unsafe(no_mangle)]
pub extern "C" fn fz_outline_y(_ctx: Handle, outline: Handle) -> f32 {
    if let Some(arc) = OUTLINES.get(outline) {
        if let Ok(o) = arc.lock() {
            return o.y;
        }
    }
    0.0
}

/// Set outline destination coordinates
#[unsafe(no_mangle)]
pub extern "C" fn fz_set_outline_xy(_ctx: Handle, outline: Handle, x: f32, y: f32) {
    if let Some(arc) = OUTLINES.get(outline) {
        if let Ok(mut o) = arc.lock() {
            o.x = x;
            o.y = y;
        }
    }
}

// ============================================================================
// Outline Iterator API
// ============================================================================

/// Create an iterator from an outline
#[unsafe(no_mangle)]
pub extern "C" fn fz_outline_iterator_from_outline(_ctx: Handle, outline: Handle) -> Handle {
    if outline == 0 {
        return 0;
    }
    OUTLINE_ITERATORS.insert(OutlineIterator::from_outline(outline))
}

/// Create a new empty iterator
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_outline_iterator(_ctx: Handle) -> Handle {
    OUTLINE_ITERATORS.insert(OutlineIterator::new())
}

/// Drop the iterator
#[unsafe(no_mangle)]
pub extern "C" fn fz_drop_outline_iterator(_ctx: Handle, iter: Handle) {
    if iter != 0 {
        OUTLINE_ITERATORS.remove(iter);
    }
}

/// Get current item from iterator
/// Returns pointer to FzOutlineItem or NULL
#[unsafe(no_mangle)]
pub extern "C" fn fz_outline_iterator_item(_ctx: Handle, iter: Handle) -> *const FzOutlineItem {
    thread_local! {
        static ITEM: std::cell::RefCell<FzOutlineItem> = std::cell::RefCell::new(FzOutlineItem::default());
    }

    if let Some(arc) = OUTLINE_ITERATORS.get(iter) {
        if let Ok(mut iterator) = arc.lock() {
            if let Some(item) = iterator.item() {
                // Store title and uri in thread-local storage
                if let Some(ref title) = item.title {
                    if let Ok(cstr) = CString::new(title.as_str()) {
                        CURRENT_TITLE.with(|cell| {
                            *cell.borrow_mut() = Some(cstr);
                        });
                    }
                }
                if let Some(ref uri) = item.uri {
                    if let Ok(cstr) = CString::new(uri.as_str()) {
                        CURRENT_URI.with(|cell| {
                            *cell.borrow_mut() = Some(cstr);
                        });
                    }
                }

                ITEM.with(|cell| {
                    let mut fz_item = cell.borrow_mut();
                    fz_item.title = CURRENT_TITLE.with(|c| {
                        c.borrow()
                            .as_ref()
                            .map(|s| s.as_ptr() as *mut c_char)
                            .unwrap_or(std::ptr::null_mut())
                    });
                    fz_item.uri = CURRENT_URI.with(|c| {
                        c.borrow()
                            .as_ref()
                            .map(|s| s.as_ptr() as *mut c_char)
                            .unwrap_or(std::ptr::null_mut())
                    });
                    fz_item.is_open = item.is_open as i32;
                    fz_item.flags = item.flags as i32;
                    fz_item.r = item.r;
                    fz_item.g = item.g;
                    fz_item.b = item.b;
                });

                return ITEM.with(|cell| cell.as_ptr());
            }
        }
    }
    std::ptr::null()
}

/// Move iterator to next sibling
#[unsafe(no_mangle)]
pub extern "C" fn fz_outline_iterator_next(_ctx: Handle, iter: Handle) -> i32 {
    if let Some(arc) = OUTLINE_ITERATORS.get(iter) {
        if let Ok(mut iterator) = arc.lock() {
            return iterator.next();
        }
    }
    OutlineIteratorResult::DidNotMove as i32
}

/// Move iterator to previous sibling
#[unsafe(no_mangle)]
pub extern "C" fn fz_outline_iterator_prev(_ctx: Handle, iter: Handle) -> i32 {
    if let Some(arc) = OUTLINE_ITERATORS.get(iter) {
        if let Ok(mut iterator) = arc.lock() {
            return iterator.prev();
        }
    }
    OutlineIteratorResult::DidNotMove as i32
}

/// Move iterator up to parent
#[unsafe(no_mangle)]
pub extern "C" fn fz_outline_iterator_up(_ctx: Handle, iter: Handle) -> i32 {
    if let Some(arc) = OUTLINE_ITERATORS.get(iter) {
        if let Ok(mut iterator) = arc.lock() {
            return iterator.up();
        }
    }
    OutlineIteratorResult::DidNotMove as i32
}

/// Move iterator down to first child
#[unsafe(no_mangle)]
pub extern "C" fn fz_outline_iterator_down(_ctx: Handle, iter: Handle) -> i32 {
    if let Some(arc) = OUTLINE_ITERATORS.get(iter) {
        if let Ok(mut iterator) = arc.lock() {
            return iterator.down();
        }
    }
    OutlineIteratorResult::DidNotMove as i32
}

/// Insert item before current position
#[unsafe(no_mangle)]
pub extern "C" fn fz_outline_iterator_insert(
    _ctx: Handle,
    iter: Handle,
    item: *const FzOutlineItem,
) -> i32 {
    if item.is_null() {
        return OutlineIteratorResult::DidNotMove as i32;
    }

    let fz_item = unsafe { &*item };
    let rust_item = OutlineItem {
        title: if fz_item.title.is_null() {
            None
        } else {
            unsafe { CStr::from_ptr(fz_item.title) }
                .to_str()
                .ok()
                .map(String::from)
        },
        uri: if fz_item.uri.is_null() {
            None
        } else {
            unsafe { CStr::from_ptr(fz_item.uri) }
                .to_str()
                .ok()
                .map(String::from)
        },
        is_open: fz_item.is_open != 0,
        flags: fz_item.flags as u8,
        r: fz_item.r,
        g: fz_item.g,
        b: fz_item.b,
    };

    if let Some(arc) = OUTLINE_ITERATORS.get(iter) {
        if let Ok(mut iterator) = arc.lock() {
            return iterator.insert(&rust_item);
        }
    }
    OutlineIteratorResult::DidNotMove as i32
}

/// Delete current item
#[unsafe(no_mangle)]
pub extern "C" fn fz_outline_iterator_delete(_ctx: Handle, iter: Handle) -> i32 {
    if let Some(arc) = OUTLINE_ITERATORS.get(iter) {
        if let Ok(mut iterator) = arc.lock() {
            return iterator.delete();
        }
    }
    OutlineIteratorResult::DidNotMove as i32
}

/// Update current item
#[unsafe(no_mangle)]
pub extern "C" fn fz_outline_iterator_update(
    _ctx: Handle,
    iter: Handle,
    item: *const FzOutlineItem,
) {
    if item.is_null() {
        return;
    }

    let fz_item = unsafe { &*item };
    let rust_item = OutlineItem {
        title: if fz_item.title.is_null() {
            None
        } else {
            unsafe { CStr::from_ptr(fz_item.title) }
                .to_str()
                .ok()
                .map(String::from)
        },
        uri: if fz_item.uri.is_null() {
            None
        } else {
            unsafe { CStr::from_ptr(fz_item.uri) }
                .to_str()
                .ok()
                .map(String::from)
        },
        is_open: fz_item.is_open != 0,
        flags: fz_item.flags as u8,
        r: fz_item.r,
        g: fz_item.g,
        b: fz_item.b,
    };

    if let Some(arc) = OUTLINE_ITERATORS.get(iter) {
        if let Ok(mut iterator) = arc.lock() {
            iterator.update(&rust_item);
        }
    }
}

/// Load outline tree from iterator (structure-based API)
#[unsafe(no_mangle)]
pub extern "C" fn fz_load_outline_from_iterator(_ctx: Handle, iter: Handle) -> Handle {
    if let Some(arc) = OUTLINE_ITERATORS.get(iter) {
        if let Ok(iterator) = arc.lock() {
            if let Some(current) = iterator.current {
                return current;
            }
        }
    }
    0
}

// ============================================================================
// Utility Functions
// ============================================================================

/// Count total outline entries in tree
#[unsafe(no_mangle)]
pub extern "C" fn fz_outline_count(_ctx: Handle, outline: Handle) -> i32 {
    fn count_recursive(handle: Handle) -> i32 {
        if handle == 0 {
            return 0;
        }

        let (next, down) = if let Some(arc) = OUTLINES.get(handle) {
            if let Ok(o) = arc.lock() {
                (o.next, o.down)
            } else {
                (None, None)
            }
        } else {
            return 0;
        };

        let mut count = 1;
        if let Some(next_handle) = next {
            count += count_recursive(next_handle);
        }
        if let Some(down_handle) = down {
            count += count_recursive(down_handle);
        }
        count
    }

    count_recursive(outline)
}

/// Get outline depth (level in hierarchy)
#[unsafe(no_mangle)]
pub extern "C" fn fz_outline_depth(_ctx: Handle, iter: Handle) -> i32 {
    if let Some(arc) = OUTLINE_ITERATORS.get(iter) {
        if let Ok(iterator) = arc.lock() {
            return iterator.stack.len() as i32;
        }
    }
    0
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_outline_create() {
        let ctx = 0;
        let outline = fz_new_outline(ctx);
        assert!(outline > 0);
        fz_drop_outline(ctx, outline);
    }

    #[test]
    fn test_outline_title() {
        let ctx = 0;
        let outline = fz_new_outline(ctx);

        let title = CString::new("Chapter 1").unwrap();
        fz_set_outline_title(ctx, outline, title.as_ptr());

        let got_title = fz_outline_title(ctx, outline);
        assert!(!got_title.is_null());
        let got_str = unsafe { CStr::from_ptr(got_title) };
        assert_eq!(got_str.to_str().unwrap(), "Chapter 1");

        fz_drop_outline(ctx, outline);
    }

    #[test]
    fn test_outline_tree() {
        let ctx = 0;

        // Create root
        let root = fz_new_outline(ctx);
        let title = CString::new("Root").unwrap();
        fz_set_outline_title(ctx, root, title.as_ptr());

        // Create child
        let child = fz_new_outline(ctx);
        let title = CString::new("Child").unwrap();
        fz_set_outline_title(ctx, child, title.as_ptr());

        // Link them
        fz_set_outline_down(ctx, root, child);

        // Verify structure
        let got_child = fz_outline_down(ctx, root);
        assert_eq!(got_child, child);

        // Count
        let count = fz_outline_count(ctx, root);
        assert_eq!(count, 2);

        fz_drop_outline(ctx, root);
    }

    #[test]
    fn test_outline_iterator() {
        let ctx = 0;

        // Create outline tree
        let root = fz_new_outline(ctx);
        let title = CString::new("Root").unwrap();
        fz_set_outline_title(ctx, root, title.as_ptr());

        let child1 = fz_new_outline(ctx);
        let title = CString::new("Child 1").unwrap();
        fz_set_outline_title(ctx, child1, title.as_ptr());

        let child2 = fz_new_outline(ctx);
        let title = CString::new("Child 2").unwrap();
        fz_set_outline_title(ctx, child2, title.as_ptr());

        fz_set_outline_down(ctx, root, child1);
        fz_set_outline_next(ctx, child1, child2);

        // Create iterator
        let iter = fz_outline_iterator_from_outline(ctx, root);
        assert!(iter > 0);

        // Get root item
        let item = fz_outline_iterator_item(ctx, iter);
        assert!(!item.is_null());

        // Move down to child1
        let result = fz_outline_iterator_down(ctx, iter);
        assert_eq!(result, OutlineIteratorResult::AtItem as i32);

        // Move to child2
        let result = fz_outline_iterator_next(ctx, iter);
        assert_eq!(result, OutlineIteratorResult::AtItem as i32);

        // Move back up
        let result = fz_outline_iterator_up(ctx, iter);
        assert_eq!(result, OutlineIteratorResult::AtItem as i32);

        fz_drop_outline_iterator(ctx, iter);
        fz_drop_outline(ctx, root);
    }

    #[test]
    fn test_outline_color() {
        let ctx = 0;
        let outline = fz_new_outline(ctx);

        fz_set_outline_color(ctx, outline, 1.0, 0.5, 0.0);

        let r = fz_outline_color_r(ctx, outline);
        let g = fz_outline_color_g(ctx, outline);
        let b = fz_outline_color_b(ctx, outline);

        assert!((r - 1.0).abs() < 0.01);
        assert!((g - 0.5).abs() < 0.01);
        assert!((b - 0.0).abs() < 0.01);

        fz_drop_outline(ctx, outline);
    }

    #[test]
    fn test_outline_flags() {
        let ctx = 0;
        let outline = fz_new_outline(ctx);

        fz_set_outline_flags(
            ctx,
            outline,
            OutlineFlag::Bold as i32 | OutlineFlag::Italic as i32,
        );

        let flags = fz_outline_flags(ctx, outline);
        assert_eq!(flags, 3);

        fz_drop_outline(ctx, outline);
    }

    #[test]
    fn test_outline_page() {
        let ctx = 0;
        let outline = fz_new_outline(ctx);

        fz_set_outline_page(ctx, outline, 0, 42);

        let page = fz_outline_page(ctx, outline);
        assert_eq!(page, 42);

        fz_drop_outline(ctx, outline);
    }
}
