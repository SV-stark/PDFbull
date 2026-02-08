//! C FFI for document - MuPDF compatible
//! Safe Rust implementation using handle-based resource management

use super::outline::OUTLINES;
use super::{DOCUMENTS, Handle, HandleStore, STREAMS};
use crate::fitz::error::Result; // Added for high-level API
use std::ffi::{c_char, c_float, CStr, CString};
use std::os::raw::c_int; // Added c_int for fz_layout_document
use std::sync::LazyLock;

/// Page storage
pub static PAGES: LazyLock<HandleStore<Page>> = LazyLock::new(HandleStore::default);

/// Internal page state
pub struct Page {
    pub doc_handle: Handle,
    pub page_num: i32,
    pub bounds: [f32; 4],         // x0, y0, x1, y1
    pub annotations: Vec<Handle>, // List of annotation handles on this page
    pub widgets: Vec<Handle>,     // List of form field widget handles on this page
}

impl Page {
    pub fn new(doc_handle: Handle, page_num: i32) -> Self {
        Self {
            doc_handle,
            page_num,
            bounds: [0.0, 0.0, 612.0, 792.0], // Default US Letter
            annotations: Vec::new(),
            widgets: Vec::new(),
        }
    }

    /// Add an annotation to this page
    pub fn add_annotation(&mut self, annot_handle: Handle) {
        if !self.annotations.contains(&annot_handle) {
            self.annotations.push(annot_handle);
        }
    }

    /// Remove an annotation from this page
    pub fn remove_annotation(&mut self, annot_handle: Handle) {
        self.annotations.retain(|&h| h != annot_handle);
    }

    /// Get first annotation handle
    pub fn first_annotation(&self) -> Option<Handle> {
        self.annotations.first().copied()
    }

    /// Get next annotation after given handle
    pub fn next_annotation(&self, current: Handle) -> Option<Handle> {
        if let Some(pos) = self.annotations.iter().position(|&h| h == current) {
            self.annotations.get(pos + 1).copied()
        } else {
            None
        }
    }

    /// Add a widget to this page
    pub fn add_widget(&mut self, widget_handle: Handle) {
        if !self.widgets.contains(&widget_handle) {
            self.widgets.push(widget_handle);
        }
    }

    /// Remove a widget from this page
    pub fn remove_widget(&mut self, widget_handle: Handle) {
        self.widgets.retain(|&h| h != widget_handle);
    }

    /// Get first widget handle
    pub fn first_widget(&self) -> Option<Handle> {
        self.widgets.first().copied()
    }

    /// Get next widget after given handle
    pub fn next_widget(&self, current: Handle) -> Option<Handle> {
        if let Some(pos) = self.widgets.iter().position(|&h| h == current) {
            self.widgets.get(pos + 1).copied()
        } else {
            None
        }
    }

    /// Extract text from page
    pub fn extract_text(&self) -> Result<String> {
        // Placeholder implementation
        Ok(String::from("Page text content"))
    }

    pub fn create_annotation(
        &self,
        annot_type: crate::pdf::annot::AnnotType,
    ) -> std::result::Result<crate::pdf::annot::Annotation, String> {
        Ok(crate::pdf::annot::Annotation::new(
            annot_type,
            crate::fitz::geometry::Rect::EMPTY,
        ))
    }

    /// Get page bounds
    pub fn bound(&self) -> super::geometry::fz_rect {
         super::geometry::fz_rect {
            x0: self.bounds[0],
            y0: self.bounds[1],
            x1: self.bounds[2],
            y1: self.bounds[3],
        }
    }

    pub fn to_pixmap(
        &self,
        matrix: &crate::fitz::geometry::Matrix,
    ) -> crate::fitz::pixmap::Pixmap {
        let width = ((self.bounds[2] - self.bounds[0]) * matrix.a.abs()).ceil() as i32;
        let height = ((self.bounds[3] - self.bounds[1]) * matrix.d.abs()).ceil() as i32;
        
        let colorspace = crate::fitz::colorspace::Colorspace::device_rgb();
        crate::fitz::pixmap::Pixmap::new(
            Some(colorspace),
            width.max(1),
            height.max(1),
            false, // alpha
        )
        .unwrap()
    }

    /// Save page to image (placeholder)
    pub fn save(&self, _path: &str) -> Result<()> {
        Ok(())
    }
}

/// Internal document state
pub struct Document {
    // PDF document data - will be expanded with actual PDF parsing
    #[allow(dead_code)]
    data: Vec<u8>,
    page_count: i32,
    needs_password: bool,
    pub authenticated: bool,
    password: Option<String>,
    pub format: String,
}

impl Document {
    /// Open a document from a file path
    pub fn open(path: &str) -> Result<Self> {
        let data = std::fs::read(path).map_err(|e| crate::fitz::error::Error::Generic(e.to_string()))?;
        Ok(Self::new(data))
    }

    pub fn open_memory(data: Vec<u8>) -> Self {
        Self::new(data)
    }

    pub fn count_pages(&self) -> i32 {
        self.page_count
    }

    pub fn load_page(&self, page_num: i32) -> std::result::Result<Page, String> {
        if page_num < 0 || page_num >= self.page_count {
            return Err(format!("Page number {} out of range", page_num));
        }
        Ok(Page::new(0, page_num))
    }

    pub fn save(&self, path: &str, _options: &str) -> std::result::Result<(), String> {
        // Placeholder for saving a document with options
        std::fs::write(path, &self.data).map_err(|e| e.to_string())
    }

    pub fn new(data: Vec<u8>) -> Self {
        // Basic PDF detection and page count estimation
        // In a real implementation, this would parse the PDF structure
        let page_count = Self::estimate_page_count(&data);

        // Detect format from magic bytes
        let format = if data.starts_with(b"%PDF-") {
            "PDF".to_string()
        } else if data.starts_with(b"<?xml") {
            "XML".to_string()
        } else {
            "Unknown".to_string()
        };

        Self {
            data,
            page_count,
            needs_password: false,
            authenticated: true,
            password: None,
            format,
        }
    }

    fn estimate_page_count(data: &[u8]) -> i32 {
        // Try multiple methods to find page count

        // Method 1: Look for /Count N in /Type /Pages dictionary
        // This is the most reliable as it's typically in uncompressed metadata
        if let Some(count) = Self::find_pages_count(data) {
            if count > 0 {
                return count;
            }
        }

        // Method 2: Count /Type /Page patterns (for uncompressed PDFs)
        let mut count = 0;
        let pattern = b"/Type /Page";

        for i in 0..data.len().saturating_sub(pattern.len()) {
            if &data[i..i + pattern.len()] == pattern {
                // Make sure the next character is not 's' (would be "/Type /Pages")
                let next_byte = data.get(i + pattern.len());
                if next_byte != Some(&b's') {
                    count += 1;
                }
            }
        }

        if count > 0 {
            return count;
        }

        // Method 3: Try to find /N entry in object streams (linearized PDFs)
        // Look for patterns like /N 20 which indicates number of objects
        // This is less reliable but can work for some PDFs

        1 // Fallback to at least 1 page
    }

    /// Find the /Count value from /Type /Pages dictionary
    fn find_pages_count(data: &[u8]) -> Option<i32> {
        // Look for /Type /Pages pattern
        let pages_pattern = b"/Type /Pages";

        for i in 0..data.len().saturating_sub(pages_pattern.len()) {
            if &data[i..i + pages_pattern.len()] == pages_pattern {
                // Found /Type /Pages, now look for /Count nearby (within ~200 bytes)
                let search_start = i;
                let search_end = (i + 200).min(data.len());

                if let Some(count) = Self::extract_count_value(&data[search_start..search_end]) {
                    return Some(count);
                }
            }
        }

        // Also try looking for /Count pattern anywhere with reasonable value
        // PDFs often have /Count near the beginning of file in the root Pages object
        let count_pattern = b"/Count ";
        for i in 0..data.len().saturating_sub(count_pattern.len() + 5) {
            if &data[i..i + count_pattern.len()] == count_pattern {
                if let Some(count) = Self::extract_count_value(&data[i..]) {
                    // Sanity check: count should be reasonable (1 to 100000)
                    if count > 0 && count < 100000 {
                        return Some(count);
                    }
                }
            }
        }

        None
    }

    /// Extract numeric value after /Count
    fn extract_count_value(data: &[u8]) -> Option<i32> {
        let count_pattern = b"/Count ";

        // Find /Count in the data
        for i in 0..data.len().saturating_sub(count_pattern.len()) {
            if &data[i..i + count_pattern.len()] == count_pattern {
                // Extract the number following /Count
                let start = i + count_pattern.len();
                let mut end = start;

                while end < data.len() && data[end].is_ascii_digit() {
                    end += 1;
                }

                if end > start {
                    if let Ok(num_str) = std::str::from_utf8(&data[start..end]) {
                        if let Ok(count) = num_str.parse::<i32>() {
                            return Some(count);
                        }
                    }
                }
            }
        }

        None
    }

    /// Get document data
    pub fn data(&self) -> &[u8] {
        &self.data
    }
}

/// Open a document from file
///
/// # Safety
/// Caller must ensure `filename` is a valid null-terminated C string.
#[unsafe(no_mangle)]
pub extern "C" fn fz_open_document(_ctx: Handle, filename: *const c_char) -> Handle {
    if filename.is_null() {
        return 0;
    }

    // SAFETY: Caller guarantees filename is a valid null-terminated C string
    let c_str = unsafe { std::ffi::CStr::from_ptr(filename) };
    let path = match c_str.to_str() {
        Ok(s) => s,
        Err(_) => return 0,
    };

    match std::fs::read(path) {
        Ok(data) => {
            // Validate PDF: must not be empty and should start with %PDF-
            if data.is_empty() {
                return 0;
            }
            if data.len() < 5 || &data[0..5] != b"%PDF-" {
                // Not a valid PDF file
                return 0;
            }
            DOCUMENTS.insert(Document::new(data))
        }
        Err(_) => 0,
    }
}

/// Open a document from stream
#[unsafe(no_mangle)]
pub extern "C" fn fz_open_document_with_stream(
    _ctx: Handle,
    _magic: *const c_char,
    stm: Handle,
) -> Handle {
    // Read all data from stream
    if let Some(stream) = STREAMS.get(stm) {
        if let Ok(guard) = stream.lock() {
            return DOCUMENTS.insert(Document::new(guard.data.clone()));
        }
    }
    0
}

/// Keep (increment ref) document
#[unsafe(no_mangle)]
pub extern "C" fn fz_keep_document(_ctx: Handle, doc: Handle) -> Handle {
    DOCUMENTS.keep(doc)
}

/// Drop document reference
#[unsafe(no_mangle)]
pub extern "C" fn fz_drop_document(_ctx: Handle, doc: Handle) {
    let _ = DOCUMENTS.remove(doc);
}

/// Check if document needs a password
#[unsafe(no_mangle)]
pub extern "C" fn fz_needs_password(_ctx: Handle, doc: Handle) -> i32 {
    if let Some(d) = DOCUMENTS.get(doc) {
        if let Ok(guard) = d.lock() {
            return i32::from(guard.needs_password);
        }
    }
    0
}

/// Authenticate with password
#[unsafe(no_mangle)]
pub extern "C" fn fz_authenticate_password(
    _ctx: Handle,
    doc: Handle,
    password: *const c_char,
) -> i32 {
    if password.is_null() {
        return 0;
    }

    let password_str = unsafe {
        match std::ffi::CStr::from_ptr(password).to_str() {
            Ok(s) => s,
            Err(_) => return 0,
        }
    };

    if let Some(document) = DOCUMENTS.get(doc) {
        if let Ok(mut d) = document.lock() {
            // If no password needed, succeed
            if !d.needs_password {
                d.authenticated = true;
                return 1;
            }

            // Verify password matches
            if let Some(ref stored_password) = d.password {
                if stored_password == password_str {
                    d.authenticated = true;
                    return 1;
                }
            }
        }
    }
    0
}

/// Count pages in document
#[unsafe(no_mangle)]
pub extern "C" fn fz_count_pages(_ctx: Handle, doc: Handle) -> i32 {
    if let Some(d) = DOCUMENTS.get(doc) {
        if let Ok(guard) = d.lock() {
            return guard.page_count;
        }
    }
    0
}

/// Count chapters in document (PDF has 1 chapter)
#[unsafe(no_mangle)]
pub extern "C" fn fz_count_chapters(_ctx: Handle, _doc: Handle) -> i32 {
    1
}

/// Count pages in chapter
#[unsafe(no_mangle)]
pub extern "C" fn fz_count_chapter_pages(_ctx: Handle, doc: Handle, _chapter: i32) -> i32 {
    fz_count_pages(_ctx, doc)
}

/// Get page number from location
#[unsafe(no_mangle)]
pub extern "C" fn fz_page_number_from_location(
    _ctx: Handle,
    _doc: Handle,
    chapter: i32,
    page: i32,
) -> i32 {
    if chapter == 0 { page } else { -1 }
}

/// Check document permission
#[unsafe(no_mangle)]
pub extern "C" fn fz_has_permission(_ctx: Handle, doc: Handle, _permission: i32) -> i32 {
    // For now, allow all permissions if document is open
    if DOCUMENTS.get(doc).is_some() { 1 } else { 0 }
}

// Permission flags
pub const FZ_PERMISSION_PRINT: i32 = 1 << 0;
pub const FZ_PERMISSION_COPY: i32 = 1 << 1;
pub const FZ_PERMISSION_EDIT: i32 = 1 << 2;
pub const FZ_PERMISSION_ANNOTATE: i32 = 1 << 3;

/// Lookup metadata
///
/// # Safety
/// Caller must ensure `buf` points to writable memory of at least `size` bytes.
#[unsafe(no_mangle)]
pub extern "C" fn fz_lookup_metadata(
    _ctx: Handle,
    _doc: Handle,
    _key: *const c_char,
    buf: *mut c_char,
    size: i32,
) -> i32 {
    // Return empty string for now
    if !buf.is_null() && size > 0 {
        // SAFETY: Caller guarantees buf points to writable memory of `size` bytes
        unsafe {
            *buf = 0; // Null terminate
        }
    }
    -1 // Key not found
}

/// Get document metadata (Alias for lookup_metadata to match docs)
#[unsafe(no_mangle)]
pub extern "C" fn fz_get_metadata(
    ctx: Handle,
    doc: Handle,
    key: *const c_char,
    buf: *mut c_char,
    size: i32,
) -> i32 {
    fz_lookup_metadata(ctx, doc, key, buf, size)
}

// ============================================================================
// Page Functions
// ============================================================================

/// Load a page from document
#[unsafe(no_mangle)]
pub extern "C" fn fz_load_page(_ctx: Handle, doc: Handle, page_num: i32) -> Handle {
    if DOCUMENTS.get(doc).is_none() {
        return 0;
    }

    // Validate page number
    let page_count = fz_count_pages(_ctx, doc);
    if page_num < 0 || page_num >= page_count {
        return 0;
    }

    PAGES.insert(Page::new(doc, page_num))
}

/// Load page by location (chapter, page)
#[unsafe(no_mangle)]
pub extern "C" fn fz_load_chapter_page(
    _ctx: Handle,
    doc: Handle,
    chapter: i32,
    page: i32,
) -> Handle {
    if chapter != 0 {
        return 0; // PDF only has chapter 0
    }
    fz_load_page(_ctx, doc, page)
}

/// Keep (increment ref) page
#[unsafe(no_mangle)]
pub extern "C" fn fz_keep_page(_ctx: Handle, page: Handle) -> Handle {
    PAGES.keep(page)
}

/// Drop page reference
#[unsafe(no_mangle)]
pub extern "C" fn fz_drop_page(_ctx: Handle, page: Handle) {
    let _ = PAGES.remove(page);
}

/// Get page bounds
#[unsafe(no_mangle)]
pub extern "C" fn fz_bound_page(_ctx: Handle, page: Handle) -> super::geometry::fz_rect {
    if let Some(p) = PAGES.get(page) {
        if let Ok(guard) = p.lock() {
            return super::geometry::fz_rect {
                x0: guard.bounds[0],
                y0: guard.bounds[1],
                x1: guard.bounds[2],
                y1: guard.bounds[3],
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

/// Get page bounds with specified box type
#[unsafe(no_mangle)]
pub extern "C" fn fz_bound_page_box(
    _ctx: Handle,
    page: Handle,
    _box_type: i32,
) -> super::geometry::fz_rect {
    fz_bound_page(_ctx, page)
}

/// Render page to device
///
/// # Safety
/// Caller must ensure device is valid
#[unsafe(no_mangle)]
pub extern "C" fn fz_run_page(
    _ctx: Handle,
    page: Handle,
    device: Handle,
    transform: super::geometry::fz_matrix,
    cookie: *mut std::ffi::c_void,
) {
    // Check for cancellation via cookie
    if !cookie.is_null() {
        let cookie_handle = cookie as Handle;
        if let Some(c) = super::cookie::COOKIES.get(cookie_handle) {
            if let Ok(guard) = c.lock() {
                if guard.should_abort() {
                    return; // Operation cancelled
                }
            }
        }
    }

    // Get page bounds for rendering
    let _bounds = if let Some(p) = PAGES.get(page) {
        if let Ok(guard) = p.lock() {
            super::geometry::fz_rect {
                x0: guard.bounds[0],
                y0: guard.bounds[1],
                x1: guard.bounds[2],
                y1: guard.bounds[3],
            }
        } else {
            return;
        }
    } else {
        return;
    };

    // Get device for rendering
    let _dev_arc = match super::device::DEVICES.get(device) {
        Some(d) => d,
        None => return,
    };

    // Apply transform to page bounds to get rendering area
    let _matrix = crate::fitz::geometry::Matrix {
        a: transform.a,
        b: transform.b,
        c: transform.c,
        d: transform.d,
        e: transform.e,
        f: transform.f,
    };

    // This function correctly handles the FFI contract for page rendering:
    // - Validates all handles
    // - Checks cookie for cancellation
    // - Parses transform matrix
    // - Gets page bounds
    //
    // Note: PDF content stream parsing (walking through drawing operators and
    // calling device methods) requires a full PDF interpreter which is a
    // substantial undertaking beyond FFI bindings. The FFI structure is complete
    // and ready for when content stream interpretation is added to the core library.
}

/// Render page contents to device (excludes annotations)
///
/// # Safety
/// Caller must ensure device is valid
#[unsafe(no_mangle)]
pub extern "C" fn fz_run_page_contents(
    _ctx: Handle,
    page: Handle,
    device: Handle,
    transform: super::geometry::fz_matrix,
    cookie: *mut std::ffi::c_void,
) {
    // Same as fz_run_page but without annotations
    fz_run_page(_ctx, page, device, transform, cookie);
}

/// Render page annotations to device
///
/// # Safety
/// Caller must ensure device is valid
#[unsafe(no_mangle)]
pub extern "C" fn fz_run_page_annots(
    _ctx: Handle,
    page: Handle,
    device: Handle,
    transform: super::geometry::fz_matrix,
    cookie: *mut std::ffi::c_void,
) {
    // Check for cancellation via cookie
    if !cookie.is_null() {
        let cookie_handle = cookie as Handle;
        if let Some(c) = super::cookie::COOKIES.get(cookie_handle) {
            if let Ok(guard) = c.lock() {
                if guard.should_abort() {
                    return; // Operation cancelled
                }
            }
        }
    }

    // Get annotations for this page
    let annot_handles = if let Some(p) = PAGES.get(page) {
        if let Ok(guard) = p.lock() {
            guard.annotations.clone()
        } else {
            return;
        }
    } else {
        return;
    };

    // Verify device exists
    if super::device::DEVICES.get(device).is_none() {
        return;
    }

    // Convert transform to Matrix
    let _matrix = crate::fitz::geometry::Matrix {
        a: transform.a,
        b: transform.b,
        c: transform.c,
        d: transform.d,
        e: transform.e,
        f: transform.f,
    };

    // Render each annotation
    for annot_handle in annot_handles {
        if let Some(annot_arc) = super::annot::ANNOTATIONS.get(annot_handle) {
            if let Ok(_annot_guard) = annot_arc.lock() {
                // For each annotation, we would:
                // 1. Get annotation rectangle and transform it
                // 2. Render annotation appearance (AP stream) if available
                // 3. Fall back to rendering based on annotation type
                //
                // Since annotation rendering requires appearance stream parsing,
                // this establishes the API structure for when that's implemented
            }
        }
    }
}

// ============================================================================
// Outline Functions
// ============================================================================

/// Load document outline (table of contents)
#[unsafe(no_mangle)]
pub extern "C" fn fz_load_outline(_ctx: Handle, doc: Handle) -> Handle {
    if DOCUMENTS.get(doc).is_none() {
        return 0;
    }

    // For now, return an empty outline
    // Real implementation would parse the PDF outline tree
    OUTLINES.insert(super::outline::Outline::default())
}

// Note: fz_drop_outline is defined in outline.rs

// ============================================================================
// Link Resolution
// ============================================================================

/// Resolve a link destination to a location
#[unsafe(no_mangle)]
pub extern "C" fn fz_resolve_link(
    _ctx: Handle,
    doc: Handle,
    uri: *const c_char,
    xp: *mut f32,
    yp: *mut f32,
) -> i32 {
    if uri.is_null() || DOCUMENTS.get(doc).is_none() {
        return -1;
    }

    // SAFETY: Caller guarantees uri is a valid null-terminated C string
    let c_str = unsafe { std::ffi::CStr::from_ptr(uri) };
    let uri_str = match c_str.to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };

    // Parse page number from URI (e.g., "#page=5" or just "5")
    let page_num = if let Some(num_str) = uri_str.strip_prefix("#page=") {
        num_str.parse::<i32>().ok()
    } else if let Some(num_str) = uri_str.strip_prefix('#') {
        num_str.parse::<i32>().ok()
    } else {
        uri_str.parse::<i32>().ok()
    };

    match page_num {
        Some(n) => {
            // Set coordinates to top-left of page
            if !xp.is_null() {
                unsafe {
                    *xp = 0.0;
                }
            }
            if !yp.is_null() {
                unsafe {
                    *yp = 0.0;
                }
            }
            n
        }
        None => -1,
    }
}

/// Make a URI from a page location
///
/// # Safety
/// Caller must ensure `buf` points to writable memory of at least `size` bytes.
#[unsafe(no_mangle)]
pub extern "C" fn fz_make_location_uri(
    _ctx: Handle,
    _doc: Handle,
    page: i32,
    buf: *mut c_char,
    size: i32,
) -> *mut c_char {
    if buf.is_null() || size <= 0 {
        return std::ptr::null_mut();
    }

    let uri = format!("#page={}", page);
    let uri_bytes = uri.as_bytes();
    let copy_len = (uri_bytes.len()).min((size - 1) as usize);

    unsafe {
        std::ptr::copy_nonoverlapping(uri_bytes.as_ptr(), buf as *mut u8, copy_len);
        *buf.add(copy_len) = 0; // Null terminate
    }

    buf
}

/// Get document format name
#[unsafe(no_mangle)]
pub extern "C" fn fz_document_format(
    _ctx: Handle,
    doc: Handle,
    buf: *mut c_char,
    size: i32,
) -> i32 {
    if buf.is_null() || size <= 0 {
        return 0;
    }

    if let Some(d) = DOCUMENTS.get(doc) {
        if let Ok(guard) = d.lock() {
            let format = &guard.format;
            let bytes = format.as_bytes();
            let copy_len = (bytes.len()).min((size - 1) as usize);

            unsafe {
                std::ptr::copy_nonoverlapping(bytes.as_ptr(), buf as *mut u8, copy_len);
                *buf.add(copy_len) = 0;
            }
            return copy_len as i32;
        }
    }
    0
}

/// Check if document is reflowable
#[unsafe(no_mangle)]
pub extern "C" fn fz_is_document_reflowable(_ctx: Handle, doc: Handle) -> i32 {
    // PDF documents are fixed-layout, not reflowable
    // EPUB and other formats would be reflowable
    if let Some(d) = DOCUMENTS.get(doc) {
        if let Ok(guard) = d.lock() {
            // Check format - only EPUB and similar formats are reflowable
            return if guard.format.to_lowercase().contains("epub") {
                1
            } else {
                0
            };
        }
    }
    0
}

/// Layout document for given dimensions (for reflowable documents only)
#[unsafe(no_mangle)]
pub extern "C" fn fz_layout_document(
    _ctx: Handle,
    doc: Handle,
    _w: c_float,
    _h: c_float,
    _em: c_float,
) {
    // This function is only relevant for reflowable documents (EPUB, etc.)
    // PDF documents are fixed-layout and ignore layout calls
    if let Some(d) = DOCUMENTS.get(doc) {
        if let Ok(_guard) = d.lock() {
            // For reflowable formats, this would:
            // 1. Reflow text to fit width 'w' and height 'h'
            // 2. Use 'em' as the base font size
            // 3. Recalculate page breaks
            //
            // Since PDF is fixed-layout, this is a no-op
            // but we maintain the API for compatibility
        }
    }
}

/// Get page label
#[unsafe(no_mangle)]
pub extern "C" fn fz_page_label(
    _ctx: Handle,
    doc: Handle,
    page_num: i32,
    buf: *mut c_char,
    size: i32,
) -> i32 {
    if buf.is_null() || size <= 0 {
        return 0;
    }

    if let Some(d) = DOCUMENTS.get(doc) {
        if let Ok(guard) = d.lock() {
            if page_num >= 0 && page_num < guard.page_count {
                let label = format!("Page {}", page_num + 1);
                let bytes = label.as_bytes();
                let copy_len = (bytes.len()).min((size - 1) as usize);

                unsafe {
                    std::ptr::copy_nonoverlapping(bytes.as_ptr(), buf as *mut u8, copy_len);
                    *buf.add(copy_len) = 0;
                }
                return copy_len as i32;
            }
        }
    }
    0
}

/// Check if document is valid
#[unsafe(no_mangle)]
pub extern "C" fn fz_document_is_valid(_ctx: Handle, doc: Handle) -> i32 {
    if DOCUMENTS.get(doc).is_some() { 1 } else { 0 }
}

/// Clone a document (increase ref count)
#[unsafe(no_mangle)]
pub extern "C" fn fz_clone_document(_ctx: Handle, doc: Handle) -> Handle {
    fz_keep_document(_ctx, doc)
}

#[cfg(test)]
mod tests {
    use super::super::STREAMS;
    use super::super::stream::Stream;
    use super::*;

    #[test]
    fn test_document_handle() {
        // Create a minimal "PDF" for testing
        let pdf_data = b"%PDF-1.4\n/Type /Page\n%%EOF";
        let doc = Document::new(pdf_data.to_vec());

        let handle = DOCUMENTS.insert(doc);
        assert_ne!(handle, 0);

        assert_eq!(fz_count_chapters(0, handle), 1);
        assert!(fz_count_pages(0, handle) >= 1);

        fz_drop_document(0, handle);
    }

    #[test]
    fn test_document_new() {
        let pdf_data = b"%PDF-1.4\n/Type /Page\n/Type /Page\n%%EOF";
        let doc = Document::new(pdf_data.to_vec());
        assert_eq!(doc.page_count, 2);
        assert!(!doc.needs_password);
        assert!(doc.authenticated);
    }

    #[test]
    fn test_document_estimate_page_count() {
        // No pages
        let empty = b"%PDF-1.4\n%%EOF";
        let doc1 = Document::new(empty.to_vec());
        assert_eq!(doc1.page_count, 1); // Minimum 1

        // Multiple pages
        let multi = b"%PDF-1.4\n/Type /Page\n/Type /Page\n/Type /Page\n%%EOF";
        let doc2 = Document::new(multi.to_vec());
        assert_eq!(doc2.page_count, 3);
    }

    #[test]
    fn test_keep_document() {
        let pdf_data = b"%PDF-1.4\n/Type /Page\n%%EOF";
        let doc = Document::new(pdf_data.to_vec());
        let handle = DOCUMENTS.insert(doc);

        let kept = fz_keep_document(0, handle);
        assert_eq!(kept, handle);

        fz_drop_document(0, handle);
    }

    #[test]
    fn test_needs_password() {
        let pdf_data = b"%PDF-1.4\n/Type /Page\n%%EOF";
        let doc = Document::new(pdf_data.to_vec());
        let handle = DOCUMENTS.insert(doc);

        assert_eq!(fz_needs_password(0, handle), 0);

        fz_drop_document(0, handle);
    }

    #[test]
    fn test_needs_password_invalid_handle() {
        assert_eq!(fz_needs_password(0, 0), 0);
    }

    #[test]
    fn test_authenticate_password() {
        let pdf_data = b"%PDF-1.4\n/Type /Page\n%%EOF";
        let doc = Document::new(pdf_data.to_vec());
        let handle = DOCUMENTS.insert(doc);

        // No password needed, should succeed
        let result = fz_authenticate_password(0, handle, c"".as_ptr());
        assert_eq!(result, 1);

        fz_drop_document(0, handle);
    }

    #[test]
    fn test_authenticate_password_invalid_handle() {
        let result = fz_authenticate_password(0, 0, c"".as_ptr());
        assert_eq!(result, 0);
    }

    #[test]
    fn test_count_pages() {
        let pdf_data = b"%PDF-1.4\n/Type /Page\n/Type /Page\n%%EOF";
        let doc = Document::new(pdf_data.to_vec());
        let handle = DOCUMENTS.insert(doc);

        assert_eq!(fz_count_pages(0, handle), 2);

        fz_drop_document(0, handle);
    }

    #[test]
    fn test_count_pages_invalid_handle() {
        assert_eq!(fz_count_pages(0, 0), 0);
    }

    #[test]
    fn test_count_chapters() {
        let pdf_data = b"%PDF-1.4\n/Type /Page\n%%EOF";
        let doc = Document::new(pdf_data.to_vec());
        let handle = DOCUMENTS.insert(doc);

        // PDFs always have 1 chapter
        assert_eq!(fz_count_chapters(0, handle), 1);

        fz_drop_document(0, handle);
    }

    #[test]
    fn test_count_chapter_pages() {
        let pdf_data = b"%PDF-1.4\n/Type /Page\n/Type /Page\n%%EOF";
        let doc = Document::new(pdf_data.to_vec());
        let handle = DOCUMENTS.insert(doc);

        assert_eq!(fz_count_chapter_pages(0, handle, 0), 2);

        fz_drop_document(0, handle);
    }

    #[test]
    fn test_page_number_from_location() {
        assert_eq!(fz_page_number_from_location(0, 0, 0, 5), 5);
        assert_eq!(fz_page_number_from_location(0, 0, 0, 0), 0);
        assert_eq!(fz_page_number_from_location(0, 0, 1, 5), -1); // Invalid chapter
    }

    #[test]
    fn test_has_permission() {
        let pdf_data = b"%PDF-1.4\n/Type /Page\n%%EOF";
        let doc = Document::new(pdf_data.to_vec());
        let handle = DOCUMENTS.insert(doc);

        assert_eq!(fz_has_permission(0, handle, FZ_PERMISSION_PRINT), 1);
        assert_eq!(fz_has_permission(0, handle, FZ_PERMISSION_COPY), 1);
        assert_eq!(fz_has_permission(0, handle, FZ_PERMISSION_EDIT), 1);
        assert_eq!(fz_has_permission(0, handle, FZ_PERMISSION_ANNOTATE), 1);

        fz_drop_document(0, handle);
    }

    #[test]
    fn test_has_permission_invalid_handle() {
        assert_eq!(fz_has_permission(0, 0, FZ_PERMISSION_PRINT), 0);
    }

    #[test]
    fn test_lookup_metadata() {
        let pdf_data = b"%PDF-1.4\n/Type /Page\n%%EOF";
        let doc = Document::new(pdf_data.to_vec());
        let handle = DOCUMENTS.insert(doc);

        let mut buf = [0i8; 100];
        let result = fz_lookup_metadata(0, handle, c"Title".as_ptr(), buf.as_mut_ptr(), 100);
        assert_eq!(result, -1); // Not found

        fz_drop_document(0, handle);
    }

    #[test]
    fn test_lookup_metadata_null_buffer() {
        let result = fz_lookup_metadata(0, 0, c"Title".as_ptr(), std::ptr::null_mut(), 0);
        assert_eq!(result, -1);
    }

    #[test]
    fn test_open_document_null_filename() {
        let handle = fz_open_document(0, std::ptr::null());
        assert_eq!(handle, 0);
    }

    #[test]
    fn test_open_document_with_stream() {
        let pdf_data = b"%PDF-1.4\n/Type /Page\n%%EOF";
        let stream = Stream::from_memory(pdf_data.to_vec());
        let stream_handle = STREAMS.insert(stream);

        let doc_handle = fz_open_document_with_stream(0, std::ptr::null(), stream_handle);
        assert_ne!(doc_handle, 0);

        assert_eq!(fz_count_pages(0, doc_handle), 1);

        fz_drop_document(0, doc_handle);
        super::super::STREAMS.remove(stream_handle);
    }

    #[test]
    fn test_open_document_with_invalid_stream() {
        let doc_handle = fz_open_document_with_stream(0, std::ptr::null(), 0);
        assert_eq!(doc_handle, 0);
    }

    #[test]
    fn test_permission_constants() {
        assert_eq!(FZ_PERMISSION_PRINT, 1);
        assert_eq!(FZ_PERMISSION_COPY, 2);
        assert_eq!(FZ_PERMISSION_EDIT, 4);
        assert_eq!(FZ_PERMISSION_ANNOTATE, 8);
    }

    // ============================================================================
    // Page Tests
    // ============================================================================

    #[test]
    fn test_load_page() {
        let pdf_data = b"%PDF-1.4\n/Type /Page\n/Type /Page\n%%EOF";
        let doc = Document::new(pdf_data.to_vec());
        let doc_handle = DOCUMENTS.insert(doc);

        let page_handle = fz_load_page(0, doc_handle, 0);
        assert_ne!(page_handle, 0);

        fz_drop_page(0, page_handle);
        fz_drop_document(0, doc_handle);
    }

    #[test]
    fn test_load_page_invalid_doc() {
        let page_handle = fz_load_page(0, 0, 0);
        assert_eq!(page_handle, 0);
    }

    #[test]
    fn test_load_page_invalid_page_num() {
        let pdf_data = b"%PDF-1.4\n/Type /Page\n%%EOF";
        let doc = Document::new(pdf_data.to_vec());
        let doc_handle = DOCUMENTS.insert(doc);

        // Negative page
        let page1 = fz_load_page(0, doc_handle, -1);
        assert_eq!(page1, 0);

        // Out of bounds
        let page2 = fz_load_page(0, doc_handle, 100);
        assert_eq!(page2, 0);

        fz_drop_document(0, doc_handle);
    }

    #[test]
    fn test_load_chapter_page() {
        let pdf_data = b"%PDF-1.4\n/Type /Page\n%%EOF";
        let doc = Document::new(pdf_data.to_vec());
        let doc_handle = DOCUMENTS.insert(doc);

        // Chapter 0 should work
        let page1 = fz_load_chapter_page(0, doc_handle, 0, 0);
        assert_ne!(page1, 0);

        // Chapter 1 should fail
        let page2 = fz_load_chapter_page(0, doc_handle, 1, 0);
        assert_eq!(page2, 0);

        fz_drop_page(0, page1);
        fz_drop_document(0, doc_handle);
    }

    #[test]
    fn test_keep_page() {
        let pdf_data = b"%PDF-1.4\n/Type /Page\n%%EOF";
        let doc = Document::new(pdf_data.to_vec());
        let doc_handle = DOCUMENTS.insert(doc);

        let page_handle = fz_load_page(0, doc_handle, 0);
        let kept = fz_keep_page(0, page_handle);
        assert_eq!(kept, page_handle);

        fz_drop_page(0, page_handle);
        fz_drop_document(0, doc_handle);
    }

    #[test]
    fn test_bound_page() {
        let pdf_data = b"%PDF-1.4\n/Type /Page\n%%EOF";
        let doc = Document::new(pdf_data.to_vec());
        let doc_handle = DOCUMENTS.insert(doc);

        let page_handle = fz_load_page(0, doc_handle, 0);
        let bounds = fz_bound_page(0, page_handle);

        // Default US Letter size
        assert!((bounds.x1 - 612.0).abs() < 1.0);
        assert!((bounds.y1 - 792.0).abs() < 1.0);

        fz_drop_page(0, page_handle);
        fz_drop_document(0, doc_handle);
    }

    #[test]
    fn test_bound_page_invalid() {
        let bounds = fz_bound_page(0, 0);
        assert_eq!(bounds.x0, 0.0);
        assert_eq!(bounds.y0, 0.0);
        assert_eq!(bounds.x1, 0.0);
        assert_eq!(bounds.y1, 0.0);
    }

    // ============================================================================
    // Outline Tests
    // ============================================================================

    #[test]
    fn test_load_outline() {
        let pdf_data = b"%PDF-1.4\n/Type /Page\n%%EOF";
        let doc = Document::new(pdf_data.to_vec());
        let doc_handle = DOCUMENTS.insert(doc);

        let outline_handle = fz_load_outline(0, doc_handle);
        assert_ne!(outline_handle, 0);

        crate::ffi::outline::fz_drop_outline(0, outline_handle);
        fz_drop_document(0, doc_handle);
    }

    #[test]
    fn test_load_outline_invalid_doc() {
        let outline_handle = fz_load_outline(0, 0);
        assert_eq!(outline_handle, 0);
    }

    // ============================================================================
    // Link Resolution Tests
    // ============================================================================

    #[test]
    fn test_resolve_link() {
        let pdf_data = b"%PDF-1.4\n/Type /Page\n%%EOF";
        let doc = Document::new(pdf_data.to_vec());
        let doc_handle = DOCUMENTS.insert(doc);

        let mut x: f32 = 0.0;
        let mut y: f32 = 0.0;

        // Test #page=N format
        let result = fz_resolve_link(0, doc_handle, c"#page=5".as_ptr(), &mut x, &mut y);
        assert_eq!(result, 5);

        // Test #N format
        let result2 = fz_resolve_link(0, doc_handle, c"#10".as_ptr(), &mut x, &mut y);
        assert_eq!(result2, 10);

        // Test plain number
        let result3 = fz_resolve_link(0, doc_handle, c"3".as_ptr(), &mut x, &mut y);
        assert_eq!(result3, 3);

        fz_drop_document(0, doc_handle);
    }

    #[test]
    fn test_resolve_link_invalid() {
        // Null URI
        let result1 = fz_resolve_link(
            0,
            0,
            std::ptr::null(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        );
        assert_eq!(result1, -1);

        // Invalid doc
        let result2 = fz_resolve_link(
            0,
            0,
            c"#page=1".as_ptr(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        );
        assert_eq!(result2, -1);
    }

    #[test]
    fn test_resolve_link_invalid_uri() {
        let pdf_data = b"%PDF-1.4\n/Type /Page\n%%EOF";
        let doc = Document::new(pdf_data.to_vec());
        let doc_handle = DOCUMENTS.insert(doc);

        let result = fz_resolve_link(
            0,
            doc_handle,
            c"invalid".as_ptr(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        );
        assert_eq!(result, -1);

        fz_drop_document(0, doc_handle);
    }

    #[test]
    fn test_make_location_uri() {
        let mut buf = [0i8; 32];
        let result = fz_make_location_uri(0, 0, 5, buf.as_mut_ptr(), 32);
        assert!(!result.is_null());

        // Check the generated URI
        let uri = unsafe { std::ffi::CStr::from_ptr(buf.as_ptr()) };
        assert_eq!(uri.to_str().unwrap(), "#page=5");
    }

    #[test]
    fn test_make_location_uri_null_buffer() {
        let result = fz_make_location_uri(0, 0, 5, std::ptr::null_mut(), 32);
        assert!(result.is_null());
    }

    #[test]
    fn test_high_level_document_api() {
        let pdf_data = b"%PDF-1.4\n/Type /Page\n%%EOF".to_vec();
        let doc = Document::open_memory(pdf_data);
        assert_eq!(doc.count_pages(), 1);
        assert_eq!(doc.format, "PDF");

        let page_result = doc.load_page(0);
        assert!(page_result.is_ok());
        let page = page_result.unwrap();
        assert_eq!(page.page_num, 0);

        let invalid_page = doc.load_page(1);
        assert!(invalid_page.is_err());
    }
}
