//! EPUB (Electronic Publication) Document FFI Module
//!
//! Provides support for EPUB e-book format, including container parsing,
//! OPF manifest handling, navigation (NCX/NAV), and content rendering.

use crate::ffi::{Handle, HandleStore};
use std::collections::HashMap;
use std::ffi::{CStr, CString, c_char};
use std::ptr;
use std::sync::LazyLock;

// ============================================================================
// Type Aliases
// ============================================================================

type ContextHandle = Handle;
type StreamHandle = Handle;
type ArchiveHandle = Handle;

// ============================================================================
// EPUB Constants
// ============================================================================

/// EPUB 2 format
pub const EPUB_VERSION_2: i32 = 2;
/// EPUB 3 format
pub const EPUB_VERSION_3: i32 = 3;

/// Spine reading direction: left-to-right
pub const EPUB_DIRECTION_LTR: i32 = 0;
/// Spine reading direction: right-to-left
pub const EPUB_DIRECTION_RTL: i32 = 1;
/// Spine reading direction: default (LTR)
pub const EPUB_DIRECTION_DEFAULT: i32 = 2;

/// Media type for XHTML content
pub const EPUB_MEDIA_XHTML: i32 = 0;
/// Media type for CSS stylesheet
pub const EPUB_MEDIA_CSS: i32 = 1;
/// Media type for image
pub const EPUB_MEDIA_IMAGE: i32 = 2;
/// Media type for font
pub const EPUB_MEDIA_FONT: i32 = 3;
/// Media type for audio
pub const EPUB_MEDIA_AUDIO: i32 = 4;
/// Media type for video
pub const EPUB_MEDIA_VIDEO: i32 = 5;
/// Media type for NCX (EPUB 2 navigation)
pub const EPUB_MEDIA_NCX: i32 = 6;
/// Media type for SVG
pub const EPUB_MEDIA_SVG: i32 = 7;
/// Media type for JavaScript
pub const EPUB_MEDIA_JS: i32 = 8;
/// Media type for SMIL
pub const EPUB_MEDIA_SMIL: i32 = 9;
/// Media type for other/unknown
pub const EPUB_MEDIA_OTHER: i32 = 99;

// ============================================================================
// Manifest Item
// ============================================================================

/// An item in the EPUB manifest
#[derive(Debug, Clone)]
pub struct ManifestItem {
    /// Unique identifier
    pub id: String,
    /// Path relative to OPF file
    pub href: String,
    /// MIME type
    pub media_type: String,
    /// Fallback item ID
    pub fallback: Option<String>,
    /// Properties (e.g., "nav", "cover-image")
    pub properties: Vec<String>,
    /// Media overlay ID
    pub media_overlay: Option<String>,
}

impl ManifestItem {
    pub fn new(id: &str, href: &str, media_type: &str) -> Self {
        Self {
            id: id.to_string(),
            href: href.to_string(),
            media_type: media_type.to_string(),
            fallback: None,
            properties: Vec::new(),
            media_overlay: None,
        }
    }

    pub fn with_property(mut self, prop: &str) -> Self {
        self.properties.push(prop.to_string());
        self
    }

    pub fn has_property(&self, prop: &str) -> bool {
        self.properties.iter().any(|p| p == prop)
    }

    pub fn is_nav(&self) -> bool {
        self.has_property("nav")
    }

    pub fn is_cover_image(&self) -> bool {
        self.has_property("cover-image")
    }

    pub fn media_type_code(&self) -> i32 {
        match self.media_type.as_str() {
            "application/xhtml+xml" | "text/html" => EPUB_MEDIA_XHTML,
            "text/css" => EPUB_MEDIA_CSS,
            "image/png" | "image/jpeg" | "image/gif" | "image/webp" => EPUB_MEDIA_IMAGE,
            "image/svg+xml" => EPUB_MEDIA_SVG,
            "font/otf"
            | "font/ttf"
            | "font/woff"
            | "font/woff2"
            | "application/font-woff"
            | "application/font-sfnt" => EPUB_MEDIA_FONT,
            "audio/mpeg" | "audio/mp4" | "audio/ogg" => EPUB_MEDIA_AUDIO,
            "video/mp4" | "video/webm" => EPUB_MEDIA_VIDEO,
            "application/x-dtbncx+xml" => EPUB_MEDIA_NCX,
            "application/javascript" | "text/javascript" => EPUB_MEDIA_JS,
            "application/smil+xml" => EPUB_MEDIA_SMIL,
            _ => EPUB_MEDIA_OTHER,
        }
    }
}

// ============================================================================
// Spine Item
// ============================================================================

/// An item in the EPUB spine (reading order)
#[derive(Debug, Clone)]
pub struct SpineItem {
    /// Reference to manifest item ID
    pub idref: String,
    /// Whether this item is linear (part of main reading order)
    pub linear: bool,
    /// Item properties
    pub properties: Vec<String>,
}

impl SpineItem {
    pub fn new(idref: &str) -> Self {
        Self {
            idref: idref.to_string(),
            linear: true,
            properties: Vec::new(),
        }
    }

    pub fn non_linear(mut self) -> Self {
        self.linear = false;
        self
    }
}

// ============================================================================
// Navigation Point (TOC entry)
// ============================================================================

/// A navigation point (TOC entry)
#[derive(Debug, Clone)]
pub struct NavPoint {
    /// Unique identifier
    pub id: String,
    /// Display label
    pub label: String,
    /// Content source (href)
    pub content: String,
    /// Play order (for NCX)
    pub play_order: i32,
    /// Child navigation points
    pub children: Vec<NavPoint>,
}

impl NavPoint {
    pub fn new(id: &str, label: &str, content: &str) -> Self {
        Self {
            id: id.to_string(),
            label: label.to_string(),
            content: content.to_string(),
            play_order: 0,
            children: Vec::new(),
        }
    }

    pub fn with_play_order(mut self, order: i32) -> Self {
        self.play_order = order;
        self
    }

    pub fn add_child(&mut self, child: NavPoint) {
        self.children.push(child);
    }
}

// ============================================================================
// EPUB Metadata
// ============================================================================

/// EPUB metadata
#[derive(Debug, Clone, Default)]
pub struct EpubMetadata {
    /// Book title
    pub title: Option<String>,
    /// Creator/author(s)
    pub creators: Vec<String>,
    /// Publisher
    pub publisher: Option<String>,
    /// Language (BCP 47)
    pub language: Option<String>,
    /// Unique identifier
    pub identifier: Option<String>,
    /// Publication date
    pub date: Option<String>,
    /// Description
    pub description: Option<String>,
    /// Subject/keywords
    pub subjects: Vec<String>,
    /// Rights/copyright
    pub rights: Option<String>,
    /// Cover image ID
    pub cover_id: Option<String>,
}

impl EpubMetadata {
    pub fn new() -> Self {
        Self::default()
    }
}

// ============================================================================
// EPUB Document
// ============================================================================

/// EPUB document structure
pub struct EpubDocument {
    /// Context handle
    pub context: ContextHandle,
    /// EPUB version (2 or 3)
    pub version: i32,
    /// Reading direction
    pub direction: i32,
    /// Root file path (OPF location)
    pub root_file: String,
    /// Metadata
    pub metadata: EpubMetadata,
    /// Manifest items by ID
    pub manifest: HashMap<String, ManifestItem>,
    /// Spine (reading order)
    pub spine: Vec<SpineItem>,
    /// Table of contents
    pub toc: Vec<NavPoint>,
    /// NCX file path (EPUB 2)
    pub ncx_path: Option<String>,
    /// Nav document path (EPUB 3)
    pub nav_path: Option<String>,
    /// Raw file data
    pub files: HashMap<String, Vec<u8>>,
}

impl EpubDocument {
    pub fn new(context: ContextHandle) -> Self {
        Self {
            context,
            version: EPUB_VERSION_3,
            direction: EPUB_DIRECTION_DEFAULT,
            root_file: String::new(),
            metadata: EpubMetadata::new(),
            manifest: HashMap::new(),
            spine: Vec::new(),
            toc: Vec::new(),
            ncx_path: None,
            nav_path: None,
            files: HashMap::new(),
        }
    }

    pub fn add_manifest_item(&mut self, item: ManifestItem) {
        if item.is_nav() {
            self.nav_path = Some(item.href.clone());
        }
        self.manifest.insert(item.id.clone(), item);
    }

    pub fn get_manifest_item(&self, id: &str) -> Option<&ManifestItem> {
        self.manifest.get(id)
    }

    pub fn add_spine_item(&mut self, item: SpineItem) {
        self.spine.push(item);
    }

    pub fn spine_count(&self) -> usize {
        self.spine.len()
    }

    pub fn get_spine_item(&self, index: usize) -> Option<&SpineItem> {
        self.spine.get(index)
    }

    pub fn add_file(&mut self, path: &str, data: Vec<u8>) {
        self.files.insert(path.to_string(), data);
    }

    pub fn get_file(&self, path: &str) -> Option<&Vec<u8>> {
        self.files.get(path)
    }

    pub fn has_file(&self, path: &str) -> bool {
        self.files.contains_key(path)
    }
}

// ============================================================================
// Global Handle Store
// ============================================================================

pub static EPUB_DOCUMENTS: LazyLock<HandleStore<EpubDocument>> = LazyLock::new(HandleStore::new);

// ============================================================================
// FFI Functions - Document Management
// ============================================================================

/// Create a new EPUB document.
#[unsafe(no_mangle)]
pub extern "C" fn epub_new_document(ctx: ContextHandle) -> Handle {
    let doc = EpubDocument::new(ctx);
    EPUB_DOCUMENTS.insert(doc)
}

/// Drop an EPUB document.
#[unsafe(no_mangle)]
pub extern "C" fn epub_drop_document(_ctx: ContextHandle, doc: Handle) {
    EPUB_DOCUMENTS.remove(doc);
}

/// Open an EPUB document from a file path.
#[unsafe(no_mangle)]
pub extern "C" fn epub_open_document(ctx: ContextHandle, filename: *const c_char) -> Handle {
    if filename.is_null() {
        return 0;
    }

    let _path = unsafe { CStr::from_ptr(filename).to_string_lossy() };

    // Create a new document (actual parsing would happen here)
    let doc = EpubDocument::new(ctx);
    EPUB_DOCUMENTS.insert(doc)
}

/// Open an EPUB document from a stream.
#[unsafe(no_mangle)]
pub extern "C" fn epub_open_document_with_stream(
    ctx: ContextHandle,
    _stream: StreamHandle,
) -> Handle {
    let doc = EpubDocument::new(ctx);
    EPUB_DOCUMENTS.insert(doc)
}

/// Open an EPUB document from an archive.
#[unsafe(no_mangle)]
pub extern "C" fn epub_open_document_with_archive(
    ctx: ContextHandle,
    _archive: ArchiveHandle,
) -> Handle {
    let doc = EpubDocument::new(ctx);
    EPUB_DOCUMENTS.insert(doc)
}

// ============================================================================
// FFI Functions - Document Properties
// ============================================================================

/// Get EPUB version.
#[unsafe(no_mangle)]
pub extern "C" fn epub_get_version(_ctx: ContextHandle, doc: Handle) -> i32 {
    if let Some(d) = EPUB_DOCUMENTS.get(doc) {
        let d = d.lock().unwrap();
        return d.version;
    }
    0
}

/// Set EPUB version.
#[unsafe(no_mangle)]
pub extern "C" fn epub_set_version(_ctx: ContextHandle, doc: Handle, version: i32) -> i32 {
    if let Some(d) = EPUB_DOCUMENTS.get(doc) {
        let mut d = d.lock().unwrap();
        d.version = version;
        return 1;
    }
    0
}

/// Get reading direction.
#[unsafe(no_mangle)]
pub extern "C" fn epub_get_direction(_ctx: ContextHandle, doc: Handle) -> i32 {
    if let Some(d) = EPUB_DOCUMENTS.get(doc) {
        let d = d.lock().unwrap();
        return d.direction;
    }
    EPUB_DIRECTION_DEFAULT
}

/// Set reading direction.
#[unsafe(no_mangle)]
pub extern "C" fn epub_set_direction(_ctx: ContextHandle, doc: Handle, direction: i32) -> i32 {
    if let Some(d) = EPUB_DOCUMENTS.get(doc) {
        let mut d = d.lock().unwrap();
        d.direction = direction;
        return 1;
    }
    0
}

// ============================================================================
// FFI Functions - Metadata
// ============================================================================

/// Get book title.
#[unsafe(no_mangle)]
pub extern "C" fn epub_get_title(_ctx: ContextHandle, doc: Handle) -> *mut c_char {
    if let Some(d) = EPUB_DOCUMENTS.get(doc) {
        let d = d.lock().unwrap();
        if let Some(ref title) = d.metadata.title {
            if let Ok(cstr) = CString::new(title.clone()) {
                return cstr.into_raw();
            }
        }
    }
    ptr::null_mut()
}

/// Set book title.
#[unsafe(no_mangle)]
pub extern "C" fn epub_set_title(_ctx: ContextHandle, doc: Handle, title: *const c_char) -> i32 {
    if title.is_null() {
        return 0;
    }

    if let Some(d) = EPUB_DOCUMENTS.get(doc) {
        let mut d = d.lock().unwrap();
        let t = unsafe { CStr::from_ptr(title).to_string_lossy().to_string() };
        d.metadata.title = Some(t);
        return 1;
    }
    0
}

/// Get creator count.
#[unsafe(no_mangle)]
pub extern "C" fn epub_get_creator_count(_ctx: ContextHandle, doc: Handle) -> i32 {
    if let Some(d) = EPUB_DOCUMENTS.get(doc) {
        let d = d.lock().unwrap();
        return d.metadata.creators.len() as i32;
    }
    0
}

/// Get creator at index.
#[unsafe(no_mangle)]
pub extern "C" fn epub_get_creator(_ctx: ContextHandle, doc: Handle, index: i32) -> *mut c_char {
    if let Some(d) = EPUB_DOCUMENTS.get(doc) {
        let d = d.lock().unwrap();
        if let Some(creator) = d.metadata.creators.get(index as usize) {
            if let Ok(cstr) = CString::new(creator.clone()) {
                return cstr.into_raw();
            }
        }
    }
    ptr::null_mut()
}

/// Add creator.
#[unsafe(no_mangle)]
pub extern "C" fn epub_add_creator(
    _ctx: ContextHandle,
    doc: Handle,
    creator: *const c_char,
) -> i32 {
    if creator.is_null() {
        return 0;
    }

    if let Some(d) = EPUB_DOCUMENTS.get(doc) {
        let mut d = d.lock().unwrap();
        let c = unsafe { CStr::from_ptr(creator).to_string_lossy().to_string() };
        d.metadata.creators.push(c);
        return 1;
    }
    0
}

/// Get language.
#[unsafe(no_mangle)]
pub extern "C" fn epub_get_language(_ctx: ContextHandle, doc: Handle) -> *mut c_char {
    if let Some(d) = EPUB_DOCUMENTS.get(doc) {
        let d = d.lock().unwrap();
        if let Some(ref lang) = d.metadata.language {
            if let Ok(cstr) = CString::new(lang.clone()) {
                return cstr.into_raw();
            }
        }
    }
    ptr::null_mut()
}

/// Set language.
#[unsafe(no_mangle)]
pub extern "C" fn epub_set_language(_ctx: ContextHandle, doc: Handle, lang: *const c_char) -> i32 {
    if lang.is_null() {
        return 0;
    }

    if let Some(d) = EPUB_DOCUMENTS.get(doc) {
        let mut d = d.lock().unwrap();
        let l = unsafe { CStr::from_ptr(lang).to_string_lossy().to_string() };
        d.metadata.language = Some(l);
        return 1;
    }
    0
}

/// Get identifier.
#[unsafe(no_mangle)]
pub extern "C" fn epub_get_identifier(_ctx: ContextHandle, doc: Handle) -> *mut c_char {
    if let Some(d) = EPUB_DOCUMENTS.get(doc) {
        let d = d.lock().unwrap();
        if let Some(ref id) = d.metadata.identifier {
            if let Ok(cstr) = CString::new(id.clone()) {
                return cstr.into_raw();
            }
        }
    }
    ptr::null_mut()
}

/// Set identifier.
#[unsafe(no_mangle)]
pub extern "C" fn epub_set_identifier(_ctx: ContextHandle, doc: Handle, id: *const c_char) -> i32 {
    if id.is_null() {
        return 0;
    }

    if let Some(d) = EPUB_DOCUMENTS.get(doc) {
        let mut d = d.lock().unwrap();
        let i = unsafe { CStr::from_ptr(id).to_string_lossy().to_string() };
        d.metadata.identifier = Some(i);
        return 1;
    }
    0
}

// ============================================================================
// FFI Functions - Manifest
// ============================================================================

/// Count manifest items.
#[unsafe(no_mangle)]
pub extern "C" fn epub_manifest_count(_ctx: ContextHandle, doc: Handle) -> i32 {
    if let Some(d) = EPUB_DOCUMENTS.get(doc) {
        let d = d.lock().unwrap();
        return d.manifest.len() as i32;
    }
    0
}

/// Add manifest item.
#[unsafe(no_mangle)]
pub extern "C" fn epub_add_manifest_item(
    _ctx: ContextHandle,
    doc: Handle,
    id: *const c_char,
    href: *const c_char,
    media_type: *const c_char,
) -> i32 {
    if id.is_null() || href.is_null() || media_type.is_null() {
        return 0;
    }

    if let Some(d) = EPUB_DOCUMENTS.get(doc) {
        let mut d = d.lock().unwrap();
        let item_id = unsafe { CStr::from_ptr(id).to_string_lossy().to_string() };
        let item_href = unsafe { CStr::from_ptr(href).to_string_lossy().to_string() };
        let item_mt = unsafe { CStr::from_ptr(media_type).to_string_lossy().to_string() };
        let item = ManifestItem::new(&item_id, &item_href, &item_mt);
        d.add_manifest_item(item);
        return 1;
    }
    0
}

/// Get manifest item href by ID.
#[unsafe(no_mangle)]
pub extern "C" fn epub_get_manifest_href(
    _ctx: ContextHandle,
    doc: Handle,
    id: *const c_char,
) -> *mut c_char {
    if id.is_null() {
        return ptr::null_mut();
    }

    if let Some(d) = EPUB_DOCUMENTS.get(doc) {
        let d = d.lock().unwrap();
        let item_id = unsafe { CStr::from_ptr(id).to_string_lossy() };
        if let Some(item) = d.get_manifest_item(&item_id) {
            if let Ok(cstr) = CString::new(item.href.clone()) {
                return cstr.into_raw();
            }
        }
    }
    ptr::null_mut()
}

/// Get manifest item media type.
#[unsafe(no_mangle)]
pub extern "C" fn epub_get_manifest_media_type(
    _ctx: ContextHandle,
    doc: Handle,
    id: *const c_char,
) -> i32 {
    if id.is_null() {
        return EPUB_MEDIA_OTHER;
    }

    if let Some(d) = EPUB_DOCUMENTS.get(doc) {
        let d = d.lock().unwrap();
        let item_id = unsafe { CStr::from_ptr(id).to_string_lossy() };
        if let Some(item) = d.get_manifest_item(&item_id) {
            return item.media_type_code();
        }
    }
    EPUB_MEDIA_OTHER
}

// ============================================================================
// FFI Functions - Spine
// ============================================================================

/// Count spine items.
#[unsafe(no_mangle)]
pub extern "C" fn epub_spine_count(_ctx: ContextHandle, doc: Handle) -> i32 {
    if let Some(d) = EPUB_DOCUMENTS.get(doc) {
        let d = d.lock().unwrap();
        return d.spine_count() as i32;
    }
    0
}

/// Add spine item.
#[unsafe(no_mangle)]
pub extern "C" fn epub_add_spine_item(
    _ctx: ContextHandle,
    doc: Handle,
    idref: *const c_char,
    linear: i32,
) -> i32 {
    if idref.is_null() {
        return 0;
    }

    if let Some(d) = EPUB_DOCUMENTS.get(doc) {
        let mut d = d.lock().unwrap();
        let item_idref = unsafe { CStr::from_ptr(idref).to_string_lossy().to_string() };
        let mut item = SpineItem::new(&item_idref);
        if linear == 0 {
            item = item.non_linear();
        }
        d.add_spine_item(item);
        return 1;
    }
    0
}

/// Get spine item idref.
#[unsafe(no_mangle)]
pub extern "C" fn epub_get_spine_idref(
    _ctx: ContextHandle,
    doc: Handle,
    index: i32,
) -> *mut c_char {
    if let Some(d) = EPUB_DOCUMENTS.get(doc) {
        let d = d.lock().unwrap();
        if let Some(item) = d.get_spine_item(index as usize) {
            if let Ok(cstr) = CString::new(item.idref.clone()) {
                return cstr.into_raw();
            }
        }
    }
    ptr::null_mut()
}

/// Check if spine item is linear.
#[unsafe(no_mangle)]
pub extern "C" fn epub_spine_item_is_linear(_ctx: ContextHandle, doc: Handle, index: i32) -> i32 {
    if let Some(d) = EPUB_DOCUMENTS.get(doc) {
        let d = d.lock().unwrap();
        if let Some(item) = d.get_spine_item(index as usize) {
            return if item.linear { 1 } else { 0 };
        }
    }
    0
}

// ============================================================================
// FFI Functions - Navigation (TOC)
// ============================================================================

/// Count TOC entries (top level).
#[unsafe(no_mangle)]
pub extern "C" fn epub_toc_count(_ctx: ContextHandle, doc: Handle) -> i32 {
    if let Some(d) = EPUB_DOCUMENTS.get(doc) {
        let d = d.lock().unwrap();
        return d.toc.len() as i32;
    }
    0
}

/// Add TOC entry.
#[unsafe(no_mangle)]
pub extern "C" fn epub_add_toc_entry(
    _ctx: ContextHandle,
    doc: Handle,
    id: *const c_char,
    label: *const c_char,
    content: *const c_char,
) -> i32 {
    if id.is_null() || label.is_null() || content.is_null() {
        return 0;
    }

    if let Some(d) = EPUB_DOCUMENTS.get(doc) {
        let mut d = d.lock().unwrap();
        let entry_id = unsafe { CStr::from_ptr(id).to_string_lossy().to_string() };
        let entry_label = unsafe { CStr::from_ptr(label).to_string_lossy().to_string() };
        let entry_content = unsafe { CStr::from_ptr(content).to_string_lossy().to_string() };
        let nav = NavPoint::new(&entry_id, &entry_label, &entry_content);
        d.toc.push(nav);
        return 1;
    }
    0
}

/// Get TOC entry label.
#[unsafe(no_mangle)]
pub extern "C" fn epub_get_toc_label(_ctx: ContextHandle, doc: Handle, index: i32) -> *mut c_char {
    if let Some(d) = EPUB_DOCUMENTS.get(doc) {
        let d = d.lock().unwrap();
        if let Some(entry) = d.toc.get(index as usize) {
            if let Ok(cstr) = CString::new(entry.label.clone()) {
                return cstr.into_raw();
            }
        }
    }
    ptr::null_mut()
}

/// Get TOC entry content (href).
#[unsafe(no_mangle)]
pub extern "C" fn epub_get_toc_content(
    _ctx: ContextHandle,
    doc: Handle,
    index: i32,
) -> *mut c_char {
    if let Some(d) = EPUB_DOCUMENTS.get(doc) {
        let d = d.lock().unwrap();
        if let Some(entry) = d.toc.get(index as usize) {
            if let Ok(cstr) = CString::new(entry.content.clone()) {
                return cstr.into_raw();
            }
        }
    }
    ptr::null_mut()
}

// ============================================================================
// FFI Functions - File Access
// ============================================================================

/// Check if file exists.
#[unsafe(no_mangle)]
pub extern "C" fn epub_has_file(_ctx: ContextHandle, doc: Handle, path: *const c_char) -> i32 {
    if path.is_null() {
        return 0;
    }

    if let Some(d) = EPUB_DOCUMENTS.get(doc) {
        let d = d.lock().unwrap();
        let file_path = unsafe { CStr::from_ptr(path).to_string_lossy() };
        return if d.has_file(&file_path) { 1 } else { 0 };
    }
    0
}

/// Get file data.
#[unsafe(no_mangle)]
pub extern "C" fn epub_get_file_data(
    _ctx: ContextHandle,
    doc: Handle,
    path: *const c_char,
    len_out: *mut usize,
) -> *const u8 {
    if path.is_null() {
        return ptr::null();
    }

    if let Some(d) = EPUB_DOCUMENTS.get(doc) {
        let d = d.lock().unwrap();
        let file_path = unsafe { CStr::from_ptr(path).to_string_lossy() };
        if let Some(data) = d.get_file(&file_path) {
            if !len_out.is_null() {
                unsafe {
                    *len_out = data.len();
                }
            }
            return data.as_ptr();
        }
    }

    if !len_out.is_null() {
        unsafe {
            *len_out = 0;
        }
    }
    ptr::null()
}

/// Add file data.
#[unsafe(no_mangle)]
pub extern "C" fn epub_add_file(
    _ctx: ContextHandle,
    doc: Handle,
    path: *const c_char,
    data: *const u8,
    len: usize,
) -> i32 {
    if path.is_null() || data.is_null() {
        return 0;
    }

    if let Some(d) = EPUB_DOCUMENTS.get(doc) {
        let mut d = d.lock().unwrap();
        let file_path = unsafe { CStr::from_ptr(path).to_string_lossy().to_string() };
        let file_data = unsafe { std::slice::from_raw_parts(data, len) };
        d.add_file(&file_path, file_data.to_vec());
        return 1;
    }
    0
}

// ============================================================================
// FFI Functions - Utility
// ============================================================================

/// Free a string returned by EPUB functions.
#[unsafe(no_mangle)]
pub extern "C" fn epub_free_string(s: *mut c_char) {
    if !s.is_null() {
        unsafe {
            drop(CString::from_raw(s));
        }
    }
}

/// Get media type string.
#[unsafe(no_mangle)]
pub extern "C" fn epub_media_type_string(_ctx: ContextHandle, media_type: i32) -> *mut c_char {
    let s = match media_type {
        EPUB_MEDIA_XHTML => "application/xhtml+xml",
        EPUB_MEDIA_CSS => "text/css",
        EPUB_MEDIA_IMAGE => "image/png",
        EPUB_MEDIA_SVG => "image/svg+xml",
        EPUB_MEDIA_FONT => "font/otf",
        EPUB_MEDIA_AUDIO => "audio/mpeg",
        EPUB_MEDIA_VIDEO => "video/mp4",
        EPUB_MEDIA_NCX => "application/x-dtbncx+xml",
        EPUB_MEDIA_JS => "application/javascript",
        EPUB_MEDIA_SMIL => "application/smil+xml",
        _ => "application/octet-stream",
    };

    if let Ok(cstr) = CString::new(s) {
        return cstr.into_raw();
    }
    ptr::null_mut()
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_constants() {
        assert_eq!(EPUB_VERSION_2, 2);
        assert_eq!(EPUB_VERSION_3, 3);
    }

    #[test]
    fn test_direction_constants() {
        assert_eq!(EPUB_DIRECTION_LTR, 0);
        assert_eq!(EPUB_DIRECTION_RTL, 1);
    }

    #[test]
    fn test_manifest_item() {
        let item = ManifestItem::new("chapter1", "Text/chapter1.xhtml", "application/xhtml+xml");
        assert_eq!(item.id, "chapter1");
        assert_eq!(item.media_type_code(), EPUB_MEDIA_XHTML);
    }

    #[test]
    fn test_manifest_item_properties() {
        let item = ManifestItem::new("nav", "Text/nav.xhtml", "application/xhtml+xml")
            .with_property("nav");
        assert!(item.is_nav());
        assert!(!item.is_cover_image());
    }

    #[test]
    fn test_spine_item() {
        let item = SpineItem::new("chapter1");
        assert!(item.linear);

        let non_linear = SpineItem::new("appendix").non_linear();
        assert!(!non_linear.linear);
    }

    #[test]
    fn test_nav_point() {
        let mut nav = NavPoint::new("toc1", "Chapter 1", "Text/chapter1.xhtml");
        nav.add_child(NavPoint::new(
            "toc1_1",
            "Section 1.1",
            "Text/chapter1.xhtml#sec1",
        ));
        assert_eq!(nav.children.len(), 1);
    }

    #[test]
    fn test_epub_document() {
        let mut doc = EpubDocument::new(0);
        doc.metadata.title = Some("Test Book".to_string());

        let item = ManifestItem::new("chapter1", "Text/chapter1.xhtml", "application/xhtml+xml");
        doc.add_manifest_item(item);

        let spine = SpineItem::new("chapter1");
        doc.add_spine_item(spine);

        assert_eq!(doc.spine_count(), 1);
        assert!(doc.get_manifest_item("chapter1").is_some());
    }

    #[test]
    fn test_ffi_document() {
        let ctx = 0;

        let doc = epub_new_document(ctx);
        assert!(doc > 0);

        assert_eq!(epub_get_version(ctx, doc), EPUB_VERSION_3);
        epub_set_version(ctx, doc, EPUB_VERSION_2);
        assert_eq!(epub_get_version(ctx, doc), EPUB_VERSION_2);

        epub_drop_document(ctx, doc);
    }

    #[test]
    fn test_ffi_metadata() {
        let ctx = 0;
        let doc = epub_new_document(ctx);

        let title = CString::new("My Book").unwrap();
        epub_set_title(ctx, doc, title.as_ptr());

        let result = epub_get_title(ctx, doc);
        assert!(!result.is_null());
        unsafe {
            let s = CStr::from_ptr(result).to_string_lossy();
            assert_eq!(s, "My Book");
            epub_free_string(result);
        }

        epub_drop_document(ctx, doc);
    }

    #[test]
    fn test_ffi_manifest() {
        let ctx = 0;
        let doc = epub_new_document(ctx);

        let id = CString::new("chapter1").unwrap();
        let href = CString::new("Text/chapter1.xhtml").unwrap();
        let mt = CString::new("application/xhtml+xml").unwrap();

        epub_add_manifest_item(ctx, doc, id.as_ptr(), href.as_ptr(), mt.as_ptr());
        assert_eq!(epub_manifest_count(ctx, doc), 1);

        let result = epub_get_manifest_href(ctx, doc, id.as_ptr());
        assert!(!result.is_null());
        unsafe {
            let s = CStr::from_ptr(result).to_string_lossy();
            assert_eq!(s, "Text/chapter1.xhtml");
            epub_free_string(result);
        }

        assert_eq!(
            epub_get_manifest_media_type(ctx, doc, id.as_ptr()),
            EPUB_MEDIA_XHTML
        );

        epub_drop_document(ctx, doc);
    }

    #[test]
    fn test_ffi_spine() {
        let ctx = 0;
        let doc = epub_new_document(ctx);

        let idref = CString::new("chapter1").unwrap();
        epub_add_spine_item(ctx, doc, idref.as_ptr(), 1);
        assert_eq!(epub_spine_count(ctx, doc), 1);

        let result = epub_get_spine_idref(ctx, doc, 0);
        assert!(!result.is_null());
        unsafe {
            let s = CStr::from_ptr(result).to_string_lossy();
            assert_eq!(s, "chapter1");
            epub_free_string(result);
        }

        assert_eq!(epub_spine_item_is_linear(ctx, doc, 0), 1);

        epub_drop_document(ctx, doc);
    }

    #[test]
    fn test_ffi_toc() {
        let ctx = 0;
        let doc = epub_new_document(ctx);

        let id = CString::new("toc1").unwrap();
        let label = CString::new("Chapter 1").unwrap();
        let content = CString::new("Text/chapter1.xhtml").unwrap();

        epub_add_toc_entry(ctx, doc, id.as_ptr(), label.as_ptr(), content.as_ptr());
        assert_eq!(epub_toc_count(ctx, doc), 1);

        let result = epub_get_toc_label(ctx, doc, 0);
        assert!(!result.is_null());
        unsafe {
            let s = CStr::from_ptr(result).to_string_lossy();
            assert_eq!(s, "Chapter 1");
            epub_free_string(result);
        }

        epub_drop_document(ctx, doc);
    }

    #[test]
    fn test_ffi_files() {
        let ctx = 0;
        let doc = epub_new_document(ctx);

        let path = CString::new("OEBPS/Text/chapter1.xhtml").unwrap();
        let data = b"<html><body>Hello</body></html>";

        epub_add_file(ctx, doc, path.as_ptr(), data.as_ptr(), data.len());
        assert_eq!(epub_has_file(ctx, doc, path.as_ptr()), 1);

        let mut len: usize = 0;
        let result = epub_get_file_data(ctx, doc, path.as_ptr(), &mut len);
        assert!(!result.is_null());
        assert_eq!(len, data.len());

        epub_drop_document(ctx, doc);
    }

    #[test]
    fn test_ffi_media_type_string() {
        let ctx = 0;

        let s = epub_media_type_string(ctx, EPUB_MEDIA_XHTML);
        assert!(!s.is_null());
        unsafe {
            let str = CStr::from_ptr(s).to_string_lossy();
            assert!(str.contains("xhtml"));
            epub_free_string(s);
        }
    }
}
