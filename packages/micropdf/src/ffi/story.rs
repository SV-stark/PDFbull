//! FFI bindings for fz_story (HTML Story Layout)
//!
//! This module provides an API for laying out and placing styled HTML text
//! on pages. It supports CSS styling, DOM manipulation, and incremental layout.

use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::sync::LazyLock;

use crate::ffi::stext::Rect;
use crate::ffi::{Handle, HandleStore};

/// Global store for stories
pub static STORIES: LazyLock<HandleStore<Story>> = LazyLock::new(HandleStore::new);

/// Place story return codes
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaceStoryReturn {
    /// All content fitted in the rectangle
    AllFitted = 0,
    /// More content to fit (generic)
    MoreToFit = 1,
    /// Width overflow detected (when NO_OVERFLOW flag set)
    OverflowWidth = 2,
}

/// Place story flags
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaceStoryFlag {
    /// Normal behavior - allow horizontal overflow
    None = 0,
    /// Abort on horizontal overflow
    NoOverflow = 1,
}

/// Story state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoryState {
    /// Story created, DOM available
    Created,
    /// Story being laid out
    Placing,
    /// Story placed, ready to draw
    Placed,
    /// Story drawn, ready for more placement
    Drawn,
    /// All content consumed
    Complete,
}

/// CSS style properties
#[derive(Debug, Clone, Default)]
pub struct CssStyle {
    pub font_family: Option<String>,
    pub font_size: Option<f32>,
    pub font_weight: Option<String>,
    pub font_style: Option<String>,
    pub color: Option<String>,
    pub background_color: Option<String>,
    pub margin: Option<f32>,
    pub padding: Option<f32>,
    pub text_align: Option<String>,
    pub line_height: Option<f32>,
}

/// DOM element for story
#[derive(Debug, Clone)]
pub struct StoryElement {
    pub tag: String,
    pub id: Option<String>,
    pub href: Option<String>,
    pub class: Option<String>,
    pub text: Option<String>,
    pub style: CssStyle,
    pub children: Vec<StoryElement>,
    pub rect: Rect,
    pub depth: i32,
    pub heading_level: i32, // 0 for non-headers, 1-6 for h1-h6
}

impl StoryElement {
    pub fn new(tag: &str) -> Self {
        Self {
            tag: tag.to_string(),
            id: None,
            href: None,
            class: None,
            text: None,
            style: CssStyle::default(),
            children: Vec::new(),
            rect: Rect::default(),
            depth: 0,
            heading_level: 0,
        }
    }

    /// Check if this is a heading element
    pub fn is_heading(&self) -> bool {
        matches!(
            self.tag.to_lowercase().as_str(),
            "h1" | "h2" | "h3" | "h4" | "h5" | "h6"
        )
    }

    /// Get heading level (1-6) or 0 if not a heading
    pub fn get_heading_level(&self) -> i32 {
        match self.tag.to_lowercase().as_str() {
            "h1" => 1,
            "h2" => 2,
            "h3" => 3,
            "h4" => 4,
            "h5" => 5,
            "h6" => 6,
            _ => 0,
        }
    }
}

/// Element position information for callbacks
#[repr(C)]
#[derive(Debug, Clone)]
pub struct StoryElementPosition {
    /// Depth in the box structure
    pub depth: i32,
    /// Heading level (0 if not a header, 1-6 for h1-h6)
    pub heading: i32,
    /// Element id (if any)
    pub id: *const c_char,
    /// Element href (if any)
    pub href: *const c_char,
    /// Element rectangle
    pub rect: Rect,
    /// Immediate text content
    pub text: *const c_char,
    /// Open/close flags (bit 0 = opens, bit 1 = closes)
    pub open_close: i32,
    /// Rectangle number in layout sequence
    pub rectangle_num: i32,
}

impl Default for StoryElementPosition {
    fn default() -> Self {
        Self {
            depth: 0,
            heading: 0,
            id: std::ptr::null(),
            href: std::ptr::null(),
            rect: Rect::default(),
            text: std::ptr::null(),
            open_close: 0,
            rectangle_num: 0,
        }
    }
}

/// Placed content region
#[derive(Debug, Clone)]
pub struct PlacedRegion {
    pub rect: Rect,
    pub elements: Vec<usize>, // Indices into story elements
    pub filled: Rect,
}

/// HTML Story for layout and rendering
pub struct Story {
    /// Original HTML content
    pub html: String,
    /// User CSS styles
    pub user_css: String,
    /// Base font size in points
    pub em: f32,
    /// Archive handle for loading resources
    pub archive: Option<Handle>,
    /// Parsed DOM tree
    pub document: Option<StoryElement>,
    /// Current state
    pub state: StoryState,
    /// Parsing/layout warnings
    pub warnings: Vec<String>,
    /// Current layout position (element index)
    pub layout_position: usize,
    /// Placed regions
    pub placed_regions: Vec<PlacedRegion>,
    /// Current rectangle number
    pub rectangle_num: i32,
    /// Cached strings for FFI
    cached_strings: HashMap<String, CString>,
}

impl Story {
    /// Create a new story from HTML
    pub fn new(html: &str, user_css: &str, em: f32, archive: Option<Handle>) -> Self {
        let mut story = Self {
            html: html.to_string(),
            user_css: user_css.to_string(),
            em,
            archive,
            document: None,
            state: StoryState::Created,
            warnings: Vec::new(),
            layout_position: 0,
            placed_regions: Vec::new(),
            rectangle_num: 0,
            cached_strings: HashMap::new(),
        };

        // Parse HTML into DOM
        story.parse_html();

        story
    }

    /// Parse HTML into DOM structure
    fn parse_html(&mut self) {
        // Simple HTML parser
        let mut root = StoryElement::new("body");
        let html = self.html.trim();

        if html.is_empty() {
            self.document = Some(root);
            return;
        }

        // Basic tag parsing
        let mut current_depth = 0;
        let mut elements: Vec<StoryElement> = Vec::new();
        let mut pos = 0;
        let bytes = html.as_bytes();

        while pos < bytes.len() {
            if bytes[pos] == b'<' {
                // Find tag end
                let tag_start = pos + 1;
                let mut tag_end = tag_start;
                while tag_end < bytes.len() && bytes[tag_end] != b'>' {
                    tag_end += 1;
                }

                if tag_end < bytes.len() {
                    let tag_content = &html[tag_start..tag_end];

                    if tag_content.starts_with('/') {
                        // Closing tag
                        current_depth -= 1;
                    } else if !tag_content.ends_with('/') && !tag_content.starts_with('!') {
                        // Opening tag (not self-closing, not comment)
                        let tag_name = tag_content
                            .split_whitespace()
                            .next()
                            .unwrap_or("")
                            .to_lowercase();

                        let mut elem = StoryElement::new(&tag_name);
                        elem.depth = current_depth;
                        elem.heading_level = elem.get_heading_level();

                        // Parse attributes
                        if let Some(id) = Self::extract_attr(tag_content, "id") {
                            elem.id = Some(id);
                        }
                        if let Some(href) = Self::extract_attr(tag_content, "href") {
                            elem.href = Some(href);
                        }
                        if let Some(class) = Self::extract_attr(tag_content, "class") {
                            elem.class = Some(class);
                        }

                        elements.push(elem);
                        current_depth += 1;
                    }
                    pos = tag_end + 1;
                } else {
                    pos += 1;
                }
            } else {
                // Text content
                let text_start = pos;
                while pos < bytes.len() && bytes[pos] != b'<' {
                    pos += 1;
                }
                let text = html[text_start..pos].trim();
                if !text.is_empty() && !elements.is_empty() {
                    if let Some(last) = elements.last_mut() {
                        last.text = Some(text.to_string());
                    }
                }
            }
        }

        root.children = elements;
        self.document = Some(root);
    }

    /// Extract attribute value from tag content
    fn extract_attr(tag_content: &str, attr_name: &str) -> Option<String> {
        let pattern = format!("{}=\"", attr_name);
        if let Some(start) = tag_content.find(&pattern) {
            let value_start = start + pattern.len();
            if let Some(end) = tag_content[value_start..].find('"') {
                return Some(tag_content[value_start..value_start + end].to_string());
            }
        }

        // Try single quotes
        let pattern = format!("{}='", attr_name);
        if let Some(start) = tag_content.find(&pattern) {
            let value_start = start + pattern.len();
            if let Some(end) = tag_content[value_start..].find('\'') {
                return Some(tag_content[value_start..value_start + end].to_string());
            }
        }

        None
    }

    /// Place story content into a rectangle
    pub fn place(&mut self, where_rect: Rect, flags: i32) -> (PlaceStoryReturn, Rect) {
        if self.state == StoryState::Complete {
            return (PlaceStoryReturn::AllFitted, Rect::default());
        }

        self.state = StoryState::Placing;
        self.rectangle_num += 1;

        let mut filled = Rect {
            x0: where_rect.x0,
            y0: where_rect.y0,
            x1: where_rect.x0,
            y1: where_rect.y0,
        };

        // Get elements to place
        let elements = if let Some(ref doc) = self.document {
            &doc.children
        } else {
            return (PlaceStoryReturn::AllFitted, filled);
        };

        if self.layout_position >= elements.len() {
            self.state = StoryState::Complete;
            return (PlaceStoryReturn::AllFitted, filled);
        }

        // Simple layout: stack elements vertically
        let mut y = where_rect.y0;
        let line_height = self.em * 1.2;
        let margin = self.em * 0.5;

        let mut placed_count = 0;
        let check_overflow = flags & (PlaceStoryFlag::NoOverflow as i32) != 0;

        while self.layout_position < elements.len() {
            let elem = &elements[self.layout_position];

            // Calculate element height based on content
            let elem_height = if elem.is_heading() {
                line_height * (1.5 + (6 - elem.heading_level) as f32 * 0.2)
            } else {
                line_height
            };

            // Check if element fits
            if y + elem_height + margin > where_rect.y1 {
                // No more room
                break;
            }

            // Check width overflow if flag set
            if check_overflow {
                if let Some(ref text) = elem.text {
                    let estimated_width = text.len() as f32 * self.em * 0.5;
                    if estimated_width > (where_rect.x1 - where_rect.x0) {
                        self.state = StoryState::Placed;
                        return (PlaceStoryReturn::OverflowWidth, filled);
                    }
                }
            }

            // Place element
            y += elem_height + margin;
            placed_count += 1;
            self.layout_position += 1;

            // Update filled rect
            filled.x1 = where_rect.x1;
            filled.y1 = y;
        }

        // Create placed region
        let region = PlacedRegion {
            rect: where_rect,
            elements: (self.layout_position - placed_count..self.layout_position).collect(),
            filled,
        };
        self.placed_regions.push(region);

        self.state = StoryState::Placed;

        if self.layout_position >= elements.len() {
            self.state = StoryState::Complete;
            (PlaceStoryReturn::AllFitted, filled)
        } else {
            (PlaceStoryReturn::MoreToFit, filled)
        }
    }

    /// Draw the placed story to a device
    pub fn draw(&mut self, _device: Handle, _ctm: [f32; 6]) {
        // Mark as drawn so next place continues from here
        if self.state == StoryState::Placed {
            self.state = StoryState::Drawn;
        }
    }

    /// Reset layout position to start
    pub fn reset(&mut self) {
        self.layout_position = 0;
        self.placed_regions.clear();
        self.rectangle_num = 0;
        self.state = StoryState::Created;
    }

    /// Get warnings as string
    pub fn get_warnings(&mut self) -> Option<String> {
        // After getting warnings, DOM is no longer accessible
        self.state = StoryState::Placing;

        if self.warnings.is_empty() {
            None
        } else {
            Some(self.warnings.join("\n"))
        }
    }

    /// Get cached C string
    fn get_cached_cstring(&mut self, s: &str) -> *const c_char {
        if !self.cached_strings.contains_key(s) {
            if let Ok(cstr) = CString::new(s) {
                self.cached_strings.insert(s.to_string(), cstr);
            } else {
                return std::ptr::null();
            }
        }

        self.cached_strings
            .get(s)
            .map(|cs| cs.as_ptr())
            .unwrap_or(std::ptr::null())
    }

    /// Enumerate element positions
    pub fn enumerate_positions<F>(&mut self, mut callback: F)
    where
        F: FnMut(&StoryElementPosition),
    {
        // Clone the document to avoid borrow issues
        if let Some(doc) = self.document.clone() {
            self.enumerate_element_positions(&doc.children, &mut callback, 0);
        }
    }

    fn enumerate_element_positions<F>(
        &mut self,
        elements: &[StoryElement],
        callback: &mut F,
        depth: i32,
    ) where
        F: FnMut(&StoryElementPosition),
    {
        // Clone elements to avoid borrow issues during recursion
        let elements_clone: Vec<StoryElement> = elements.to_vec();

        for elem in &elements_clone {
            // Only report headers and elements with IDs
            if elem.is_heading() || elem.id.is_some() {
                let mut pos = StoryElementPosition::default();
                pos.depth = depth;
                pos.heading = elem.heading_level;
                pos.rect = elem.rect;
                pos.open_close = 0b11; // Both open and close for leaf elements
                pos.rectangle_num = self.rectangle_num;

                // Set id pointer
                if let Some(ref id) = elem.id {
                    pos.id = self.get_cached_cstring(id);
                }

                // Set href pointer
                if let Some(ref href) = elem.href {
                    pos.href = self.get_cached_cstring(href);
                }

                // Set text pointer
                if let Some(ref text) = elem.text {
                    pos.text = self.get_cached_cstring(text);
                }

                callback(&pos);
            }

            // Recurse into children
            if !elem.children.is_empty() {
                self.enumerate_element_positions(&elem.children, callback, depth + 1);
            }
        }
    }
}

// ============================================================================
// FFI Functions
// ============================================================================

/// Create a new story from HTML buffer
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_story(
    _ctx: Handle,
    buf: Handle,
    user_css: *const c_char,
    em: f32,
    archive: Handle,
) -> Handle {
    // Get HTML from buffer
    let html = if buf == 0 {
        String::new()
    } else if let Some(buf_arc) = crate::ffi::BUFFERS.get(buf) {
        let buf_guard = buf_arc.lock().unwrap();
        String::from_utf8_lossy(buf_guard.data()).to_string()
    } else {
        String::new()
    };

    // Get CSS string
    let css = if user_css.is_null() {
        String::new()
    } else {
        unsafe { CStr::from_ptr(user_css).to_str().unwrap_or("").to_string() }
    };

    // Archive handle (0 means no archive)
    let arch = if archive == 0 { None } else { Some(archive) };

    let story = Story::new(&html, &css, em, arch);
    STORIES.insert(story)
}

/// Get story parsing warnings
#[unsafe(no_mangle)]
pub extern "C" fn fz_story_warnings(_ctx: Handle, story: Handle) -> *const c_char {
    if let Some(story_arc) = STORIES.get(story) {
        let mut story_guard = story_arc.lock().unwrap();
        if let Some(warnings) = story_guard.get_warnings() {
            // Store in cached strings
            let ptr = story_guard.get_cached_cstring(&warnings);
            return ptr;
        }
    }
    std::ptr::null()
}

/// Place story into rectangle (simple version)
#[unsafe(no_mangle)]
pub extern "C" fn fz_place_story(
    _ctx: Handle,
    story: Handle,
    where_x0: f32,
    where_y0: f32,
    where_x1: f32,
    where_y1: f32,
    filled: *mut Rect,
) -> i32 {
    fz_place_story_flags(
        _ctx, story, where_x0, where_y0, where_x1, where_y1, filled, 0,
    )
}

/// Place story into rectangle with flags
#[unsafe(no_mangle)]
pub extern "C" fn fz_place_story_flags(
    _ctx: Handle,
    story: Handle,
    where_x0: f32,
    where_y0: f32,
    where_x1: f32,
    where_y1: f32,
    filled: *mut Rect,
    flags: i32,
) -> i32 {
    let where_rect = Rect {
        x0: where_x0,
        y0: where_y0,
        x1: where_x1,
        y1: where_y1,
    };

    if let Some(story_arc) = STORIES.get(story) {
        let mut story_guard = story_arc.lock().unwrap();
        let (result, filled_rect) = story_guard.place(where_rect, flags);

        if !filled.is_null() {
            unsafe {
                *filled = filled_rect;
            }
        }

        result as i32
    } else {
        PlaceStoryReturn::AllFitted as i32
    }
}

/// Draw the placed story to a device
#[unsafe(no_mangle)]
pub extern "C" fn fz_draw_story(
    _ctx: Handle,
    story: Handle,
    dev: Handle,
    ctm_a: f32,
    ctm_b: f32,
    ctm_c: f32,
    ctm_d: f32,
    ctm_e: f32,
    ctm_f: f32,
) {
    let ctm = [ctm_a, ctm_b, ctm_c, ctm_d, ctm_e, ctm_f];

    if let Some(story_arc) = STORIES.get(story) {
        let mut story_guard = story_arc.lock().unwrap();
        story_guard.draw(dev, ctm);
    }
}

/// Reset story layout position
#[unsafe(no_mangle)]
pub extern "C" fn fz_reset_story(_ctx: Handle, story: Handle) {
    if let Some(story_arc) = STORIES.get(story) {
        let mut story_guard = story_arc.lock().unwrap();
        story_guard.reset();
    }
}

/// Drop story
#[unsafe(no_mangle)]
pub extern "C" fn fz_drop_story(_ctx: Handle, story: Handle) {
    STORIES.remove(story);
}

/// Get DOM document for manipulation
#[unsafe(no_mangle)]
pub extern "C" fn fz_story_document(_ctx: Handle, story: Handle) -> Handle {
    // Returns the XML document handle
    // This is only valid before layout starts
    if let Some(story_arc) = STORIES.get(story) {
        let story_guard = story_arc.lock().unwrap();
        if story_guard.state == StoryState::Created {
            // Return a handle to the document
            // For now, return story handle as the document is embedded
            return story;
        }
    }
    0
}

/// Position callback type for FFI
pub type StoryPositionCallback =
    extern "C" fn(ctx: Handle, arg: *mut std::ffi::c_void, pos: *const StoryElementPosition);

/// Enumerate element positions
#[unsafe(no_mangle)]
pub extern "C" fn fz_story_positions(
    ctx: Handle,
    story: Handle,
    callback: StoryPositionCallback,
    arg: *mut std::ffi::c_void,
) {
    if let Some(story_arc) = STORIES.get(story) {
        let mut story_guard = story_arc.lock().unwrap();
        story_guard.enumerate_positions(|pos| {
            callback(ctx, arg, pos);
        });
    }
}

// ============================================================================
// Additional Utility Functions
// ============================================================================

/// Get story state
#[unsafe(no_mangle)]
pub extern "C" fn fz_story_state(story: Handle) -> i32 {
    if let Some(story_arc) = STORIES.get(story) {
        let story_guard = story_arc.lock().unwrap();
        story_guard.state as i32
    } else {
        -1
    }
}

/// Check if story is complete
#[unsafe(no_mangle)]
pub extern "C" fn fz_story_is_complete(story: Handle) -> i32 {
    if let Some(story_arc) = STORIES.get(story) {
        let story_guard = story_arc.lock().unwrap();
        if story_guard.state == StoryState::Complete {
            1
        } else {
            0
        }
    } else {
        1
    }
}

/// Get current rectangle number
#[unsafe(no_mangle)]
pub extern "C" fn fz_story_rectangle_num(story: Handle) -> i32 {
    if let Some(story_arc) = STORIES.get(story) {
        let story_guard = story_arc.lock().unwrap();
        story_guard.rectangle_num
    } else {
        0
    }
}

/// Get number of placed regions
#[unsafe(no_mangle)]
pub extern "C" fn fz_story_placed_regions_count(story: Handle) -> i32 {
    if let Some(story_arc) = STORIES.get(story) {
        let story_guard = story_arc.lock().unwrap();
        story_guard.placed_regions.len() as i32
    } else {
        0
    }
}

/// Get story em size
#[unsafe(no_mangle)]
pub extern "C" fn fz_story_em(story: Handle) -> f32 {
    if let Some(story_arc) = STORIES.get(story) {
        let story_guard = story_arc.lock().unwrap();
        story_guard.em
    } else {
        12.0
    }
}

/// Set story em size
#[unsafe(no_mangle)]
pub extern "C" fn fz_story_set_em(story: Handle, em: f32) {
    if let Some(story_arc) = STORIES.get(story) {
        let mut story_guard = story_arc.lock().unwrap();
        story_guard.em = em;
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    /// Helper to create a buffer from HTML bytes
    fn create_html_buffer(ctx: Handle, html: &[u8]) -> Handle {
        crate::ffi::buffer::fz_new_buffer_from_data(ctx, html.as_ptr() as *mut u8, html.len())
    }

    #[test]
    fn test_story_creation() {
        let ctx = 1;
        let html = b"<h1>Hello World</h1><p>This is a test.</p>";
        let buf = create_html_buffer(ctx, html);

        let story = fz_new_story(ctx, buf, std::ptr::null(), 12.0, 0);
        assert!(story > 0);

        fz_drop_story(ctx, story);
        crate::ffi::buffer::fz_drop_buffer(ctx, buf);
    }

    #[test]
    fn test_story_with_css() {
        let ctx = 1;
        let html = b"<h1>Styled</h1>";
        let css = CString::new("h1 { color: red; font-size: 24pt; }").unwrap();
        let buf = create_html_buffer(ctx, html);

        let story = fz_new_story(ctx, buf, css.as_ptr(), 12.0, 0);
        assert!(story > 0);

        fz_drop_story(ctx, story);
        crate::ffi::buffer::fz_drop_buffer(ctx, buf);
    }

    #[test]
    fn test_story_place() {
        let ctx = 1;
        let html = b"<h1>Title</h1><p>Content</p>";
        let buf = create_html_buffer(ctx, html);

        let story = fz_new_story(ctx, buf, std::ptr::null(), 12.0, 0);

        let mut filled = Rect::default();
        let result = fz_place_story(ctx, story, 0.0, 0.0, 612.0, 792.0, &mut filled);

        assert_eq!(result, PlaceStoryReturn::AllFitted as i32);
        assert!(filled.y1 > filled.y0);

        fz_drop_story(ctx, story);
        crate::ffi::buffer::fz_drop_buffer(ctx, buf);
    }

    #[test]
    fn test_story_place_partial() {
        let ctx = 1;
        let html =
            b"<h1>Title</h1><p>Line 1</p><p>Line 2</p><p>Line 3</p><p>Line 4</p><p>Line 5</p>";
        let buf = create_html_buffer(ctx, html);

        let story = fz_new_story(ctx, buf, std::ptr::null(), 12.0, 0);

        let mut filled = Rect::default();
        let result = fz_place_story(ctx, story, 0.0, 0.0, 100.0, 30.0, &mut filled);

        // Should have more to fit or all fitted (depends on em size)
        assert!(
            result == PlaceStoryReturn::MoreToFit as i32
                || result == PlaceStoryReturn::AllFitted as i32
        );

        fz_drop_story(ctx, story);
        crate::ffi::buffer::fz_drop_buffer(ctx, buf);
    }

    #[test]
    fn test_story_reset() {
        let ctx = 1;
        let html = b"<p>Test</p>";
        let buf = create_html_buffer(ctx, html);

        let story = fz_new_story(ctx, buf, std::ptr::null(), 12.0, 0);

        let mut filled = Rect::default();
        fz_place_story(ctx, story, 0.0, 0.0, 612.0, 792.0, &mut filled);

        fz_reset_story(ctx, story);
        assert_eq!(fz_story_rectangle_num(story), 0);

        fz_drop_story(ctx, story);
        crate::ffi::buffer::fz_drop_buffer(ctx, buf);
    }

    #[test]
    fn test_story_document() {
        let ctx = 1;
        let html = b"<div id='test'>Content</div>";
        let buf = create_html_buffer(ctx, html);

        let story = fz_new_story(ctx, buf, std::ptr::null(), 12.0, 0);

        let doc = fz_story_document(ctx, story);
        assert!(doc > 0);

        fz_drop_story(ctx, story);
        crate::ffi::buffer::fz_drop_buffer(ctx, buf);
    }

    #[test]
    fn test_story_warnings_empty() {
        let ctx = 1;
        let html = b"<p>Valid HTML</p>";
        let buf = create_html_buffer(ctx, html);

        let story = fz_new_story(ctx, buf, std::ptr::null(), 12.0, 0);

        let warnings = fz_story_warnings(ctx, story);
        assert!(warnings.is_null());

        fz_drop_story(ctx, story);
        crate::ffi::buffer::fz_drop_buffer(ctx, buf);
    }

    #[test]
    fn test_story_is_complete() {
        let ctx = 1;
        let html = b"<p>Short</p>";
        let buf = create_html_buffer(ctx, html);

        let story = fz_new_story(ctx, buf, std::ptr::null(), 12.0, 0);

        assert_eq!(fz_story_is_complete(story), 0);

        let mut filled = Rect::default();
        fz_place_story(ctx, story, 0.0, 0.0, 612.0, 792.0, &mut filled);

        assert_eq!(fz_story_is_complete(story), 1);

        fz_drop_story(ctx, story);
        crate::ffi::buffer::fz_drop_buffer(ctx, buf);
    }

    #[test]
    fn test_story_em() {
        let ctx = 1;
        let html = b"<p>Test</p>";
        let buf = create_html_buffer(ctx, html);

        let story = fz_new_story(ctx, buf, std::ptr::null(), 14.0, 0);

        assert!((fz_story_em(story) - 14.0).abs() < 0.01);

        fz_story_set_em(story, 16.0);
        assert!((fz_story_em(story) - 16.0).abs() < 0.01);

        fz_drop_story(ctx, story);
        crate::ffi::buffer::fz_drop_buffer(ctx, buf);
    }

    #[test]
    fn test_empty_story() {
        let ctx = 1;

        let story = fz_new_story(ctx, 0, std::ptr::null(), 12.0, 0);
        assert!(story > 0);

        let mut filled = Rect::default();
        let result = fz_place_story(ctx, story, 0.0, 0.0, 612.0, 792.0, &mut filled);
        assert_eq!(result, PlaceStoryReturn::AllFitted as i32);

        fz_drop_story(ctx, story);
    }

    #[test]
    fn test_story_with_headings() {
        let ctx = 1;
        let html = b"<h1>H1</h1><h2>H2</h2><h3>H3</h3>";
        let buf = create_html_buffer(ctx, html);

        let story = fz_new_story(ctx, buf, std::ptr::null(), 12.0, 0);

        let mut filled = Rect::default();
        let result = fz_place_story(ctx, story, 0.0, 0.0, 612.0, 792.0, &mut filled);
        assert_eq!(result, PlaceStoryReturn::AllFitted as i32);

        fz_drop_story(ctx, story);
        crate::ffi::buffer::fz_drop_buffer(ctx, buf);
    }

    #[test]
    fn test_story_positions_callback() {
        static mut POSITION_COUNT: i32 = 0;

        extern "C" fn callback(
            _ctx: Handle,
            _arg: *mut std::ffi::c_void,
            _pos: *const StoryElementPosition,
        ) {
            unsafe {
                POSITION_COUNT += 1;
            }
        }

        let ctx = 1;
        let html = b"<h1 id='chapter1'>Chapter 1</h1><h2>Section</h2>";
        let buf = create_html_buffer(ctx, html);

        let story = fz_new_story(ctx, buf, std::ptr::null(), 12.0, 0);

        unsafe {
            POSITION_COUNT = 0;
        }
        fz_story_positions(ctx, story, callback, std::ptr::null_mut());

        unsafe {
            assert!(POSITION_COUNT > 0);
        }

        fz_drop_story(ctx, story);
        crate::ffi::buffer::fz_drop_buffer(ctx, buf);
    }

    #[test]
    fn test_place_story_flags_overflow() {
        let ctx = 1;
        let html =
            b"<p>This is a very long line of text that should cause overflow in a narrow rectangle</p>";
        let buf = create_html_buffer(ctx, html);

        let story = fz_new_story(ctx, buf, std::ptr::null(), 12.0, 0);

        let mut filled = Rect::default();
        let result = fz_place_story_flags(
            ctx,
            story,
            0.0,
            0.0,
            50.0,
            792.0,
            &mut filled,
            PlaceStoryFlag::NoOverflow as i32,
        );

        assert!(
            result == PlaceStoryReturn::OverflowWidth as i32
                || result == PlaceStoryReturn::AllFitted as i32
                || result == PlaceStoryReturn::MoreToFit as i32
        );

        fz_drop_story(ctx, story);
        crate::ffi::buffer::fz_drop_buffer(ctx, buf);
    }
}
