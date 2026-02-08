//! PDF Cross-Reference Table FFI Module
//!
//! Provides support for PDF cross-reference table operations, including
//! object management, stream handling, and document structure.

use crate::ffi::{Handle, HandleStore};
use std::collections::HashMap;
use std::ffi::{CStr, CString, c_char};
use std::ptr;
use std::sync::LazyLock;

// ============================================================================
// Type Aliases
// ============================================================================

type ContextHandle = Handle;
type DocumentHandle = Handle;
type BufferHandle = Handle;
type PdfObjHandle = Handle;

// ============================================================================
// Xref Entry Type Constants
// ============================================================================

/// Free entry (not in use)
pub const PDF_XREF_FREE: i32 = 0;
/// In-use entry
pub const PDF_XREF_INUSE: i32 = 1;
/// Object stream entry
pub const PDF_XREF_OBJSTM: i32 = 2;
/// Compressed entry
pub const PDF_XREF_COMPRESSED: i32 = 3;

// ============================================================================
// Xref Entry
// ============================================================================

/// A cross-reference table entry
#[derive(Debug, Clone)]
#[repr(C)]
pub struct XrefEntry {
    /// Entry type (free, in-use, objstm)
    pub entry_type: i32,
    /// Marked flag (for garbage collection)
    pub marked: i32,
    /// Generation number or object stream index
    pub generation: u16,
    /// Object number
    pub num: i32,
    /// File offset or object stream number
    pub offset: i64,
    /// Stream offset (on-disk)
    pub stm_offset: i64,
    /// Has in-memory stream buffer
    pub has_stm_buf: i32,
    /// Has cached object
    pub has_obj: i32,
}

impl Default for XrefEntry {
    fn default() -> Self {
        Self::new()
    }
}

impl XrefEntry {
    pub fn new() -> Self {
        Self {
            entry_type: PDF_XREF_FREE,
            marked: 0,
            generation: 0,
            num: 0,
            offset: 0,
            stm_offset: 0,
            has_stm_buf: 0,
            has_obj: 0,
        }
    }

    pub fn free(num: i32, generation: u16) -> Self {
        Self {
            entry_type: PDF_XREF_FREE,
            marked: 0,
            generation,
            num,
            offset: 0,
            stm_offset: 0,
            has_stm_buf: 0,
            has_obj: 0,
        }
    }

    pub fn inuse(num: i32, generation: u16, offset: i64) -> Self {
        Self {
            entry_type: PDF_XREF_INUSE,
            marked: 0,
            generation,
            num,
            offset,
            stm_offset: 0,
            has_stm_buf: 0,
            has_obj: 0,
        }
    }

    pub fn objstm(num: i32, objstm_num: i64, index: u16) -> Self {
        Self {
            entry_type: PDF_XREF_OBJSTM,
            marked: 0,
            generation: index,
            num,
            offset: objstm_num,
            stm_offset: 0,
            has_stm_buf: 0,
            has_obj: 0,
        }
    }
}

// ============================================================================
// Xref Subsection
// ============================================================================

/// A subsection of the xref table
#[derive(Debug, Clone)]
pub struct XrefSubsection {
    /// Starting object number
    pub start: i32,
    /// Entries in this subsection
    pub entries: Vec<XrefEntry>,
}

impl XrefSubsection {
    pub fn new(start: i32) -> Self {
        Self {
            start,
            entries: Vec::new(),
        }
    }

    pub fn with_capacity(start: i32, capacity: usize) -> Self {
        Self {
            start,
            entries: Vec::with_capacity(capacity),
        }
    }
}

// ============================================================================
// Xref Table
// ============================================================================

/// PDF cross-reference table
#[derive(Debug, Clone)]
pub struct Xref {
    /// Document handle
    pub document: DocumentHandle,
    /// Total number of objects
    pub num_objects: i32,
    /// Subsections
    pub subsections: Vec<XrefSubsection>,
    /// Trailer dictionary handle
    pub trailer: PdfObjHandle,
    /// File offset to end of xref
    pub end_offset: i64,
    /// Object cache
    pub cache: HashMap<i32, PdfObjHandle>,
    /// Stream buffer cache
    pub stream_cache: HashMap<i32, BufferHandle>,
    /// PDF version (major * 10 + minor, e.g., 17 for 1.7)
    pub version: i32,
}

impl Xref {
    pub fn new(document: DocumentHandle) -> Self {
        Self {
            document,
            num_objects: 0,
            subsections: Vec::new(),
            trailer: 0,
            end_offset: 0,
            cache: HashMap::new(),
            stream_cache: HashMap::new(),
            version: 17, // Default to PDF 1.7
        }
    }

    pub fn add_subsection(&mut self, subsection: XrefSubsection) {
        self.num_objects = self
            .num_objects
            .max(subsection.start + subsection.entries.len() as i32);
        self.subsections.push(subsection);
    }

    pub fn get_entry(&self, num: i32) -> Option<&XrefEntry> {
        for subsec in &self.subsections {
            let idx = num - subsec.start;
            if idx >= 0 && (idx as usize) < subsec.entries.len() {
                return Some(&subsec.entries[idx as usize]);
            }
        }
        None
    }

    pub fn get_entry_mut(&mut self, num: i32) -> Option<&mut XrefEntry> {
        for subsec in &mut self.subsections {
            let idx = num - subsec.start;
            if idx >= 0 && (idx as usize) < subsec.entries.len() {
                return Some(&mut subsec.entries[idx as usize]);
            }
        }
        None
    }

    pub fn object_exists(&self, num: i32) -> bool {
        if let Some(entry) = self.get_entry(num) {
            entry.entry_type != PDF_XREF_FREE
        } else {
            false
        }
    }

    pub fn create_object(&mut self) -> i32 {
        // Add to first subsection or create new one
        if self.subsections.is_empty() {
            let mut subsec = XrefSubsection::new(0);
            subsec.entries.push(XrefEntry::free(0, 65535)); // Object 0 is always free
            self.subsections.push(subsec);
            self.num_objects = 1;
        }

        // Find a free slot or create a new one
        let num = self.num_objects;
        self.num_objects += 1;

        // Add the new entry
        if let Some(subsec) = self.subsections.first_mut() {
            let entry = XrefEntry::inuse(num, 0, 0);
            subsec.entries.push(entry);
        }

        num
    }

    pub fn delete_object(&mut self, num: i32) {
        if let Some(entry) = self.get_entry_mut(num) {
            entry.entry_type = PDF_XREF_FREE;
            entry.generation = entry.generation.saturating_add(1);
        }
        self.cache.remove(&num);
        self.stream_cache.remove(&num);
    }
}

// ============================================================================
// Global Handle Store
// ============================================================================

pub static XREFS: LazyLock<HandleStore<Xref>> = LazyLock::new(HandleStore::new);

// ============================================================================
// FFI Functions - Xref Management
// ============================================================================

/// Create a new xref table for a document.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_new_xref(_ctx: ContextHandle, doc: DocumentHandle) -> Handle {
    let xref = Xref::new(doc);
    XREFS.insert(xref)
}

/// Drop an xref table.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_drop_xref(_ctx: ContextHandle, xref: Handle) {
    XREFS.remove(xref);
}

/// Get the number of objects in the xref.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_xref_len(_ctx: ContextHandle, xref: Handle) -> i32 {
    if let Some(x) = XREFS.get(xref) {
        let x = x.lock().unwrap();
        return x.num_objects;
    }
    0
}

/// Count objects in a document.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_count_objects(_ctx: ContextHandle, xref: Handle) -> i32 {
    pdf_xref_len(_ctx, xref)
}

/// Get the PDF version.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_version(_ctx: ContextHandle, xref: Handle) -> i32 {
    if let Some(x) = XREFS.get(xref) {
        let x = x.lock().unwrap();
        return x.version;
    }
    0
}

/// Set the PDF version.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_set_version(_ctx: ContextHandle, xref: Handle, version: i32) -> i32 {
    if let Some(x) = XREFS.get(xref) {
        let mut x = x.lock().unwrap();
        x.version = version;
        return 1;
    }
    0
}

// ============================================================================
// FFI Functions - Object Management
// ============================================================================

/// Create a new object and return its number.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_create_object(_ctx: ContextHandle, xref: Handle) -> i32 {
    if let Some(x) = XREFS.get(xref) {
        let mut x = x.lock().unwrap();
        return x.create_object();
    }
    -1
}

/// Delete an object.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_delete_object(_ctx: ContextHandle, xref: Handle, num: i32) {
    if let Some(x) = XREFS.get(xref) {
        let mut x = x.lock().unwrap();
        x.delete_object(num);
    }
}

/// Check if an object exists.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_object_exists(_ctx: ContextHandle, xref: Handle, num: i32) -> i32 {
    if let Some(x) = XREFS.get(xref) {
        let x = x.lock().unwrap();
        return if x.object_exists(num) { 1 } else { 0 };
    }
    0
}

/// Update an object in the xref.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_update_object(
    _ctx: ContextHandle,
    xref: Handle,
    num: i32,
    obj: PdfObjHandle,
) -> i32 {
    if let Some(x) = XREFS.get(xref) {
        let mut x = x.lock().unwrap();
        if let Some(entry) = x.get_entry_mut(num) {
            entry.has_obj = 1;
            x.cache.insert(num, obj);
            return 1;
        }
    }
    0
}

/// Cache an object.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_cache_object(_ctx: ContextHandle, xref: Handle, num: i32) -> i32 {
    if let Some(x) = XREFS.get(xref) {
        let x = x.lock().unwrap();
        if x.cache.contains_key(&num) {
            return 1;
        }
    }
    0
}

/// Get a cached object.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_get_cached_object(
    _ctx: ContextHandle,
    xref: Handle,
    num: i32,
) -> PdfObjHandle {
    if let Some(x) = XREFS.get(xref) {
        let x = x.lock().unwrap();
        if let Some(&obj) = x.cache.get(&num) {
            return obj;
        }
    }
    0
}

// ============================================================================
// FFI Functions - Xref Entry Access
// ============================================================================

/// Get xref entry info.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_get_xref_entry(
    _ctx: ContextHandle,
    xref: Handle,
    num: i32,
    entry_out: *mut XrefEntry,
) -> i32 {
    if entry_out.is_null() {
        return 0;
    }

    if let Some(x) = XREFS.get(xref) {
        let x = x.lock().unwrap();
        if let Some(entry) = x.get_entry(num) {
            unsafe {
                *entry_out = entry.clone();
            }
            return 1;
        }
    }
    0
}

/// Add a subsection to the xref.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_xref_add_subsection(
    _ctx: ContextHandle,
    xref: Handle,
    start: i32,
    count: i32,
) -> i32 {
    if let Some(x) = XREFS.get(xref) {
        let mut x = x.lock().unwrap();
        let subsec = XrefSubsection::with_capacity(start, count as usize);
        x.add_subsection(subsec);
        return 1;
    }
    0
}

/// Set an xref entry.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_xref_set_entry(
    _ctx: ContextHandle,
    xref: Handle,
    num: i32,
    entry_type: i32,
    generation: u16,
    offset: i64,
) -> i32 {
    if let Some(x) = XREFS.get(xref) {
        let mut x = x.lock().unwrap();

        // Find the right subsection
        for subsec in &mut x.subsections {
            let idx = num - subsec.start;
            if idx >= 0 {
                let idx = idx as usize;
                // Grow if needed
                while subsec.entries.len() <= idx {
                    let entry_num = subsec.start + subsec.entries.len() as i32;
                    subsec.entries.push(XrefEntry::free(entry_num, 0));
                }
                subsec.entries[idx] = XrefEntry {
                    entry_type,
                    marked: 0,
                    generation,
                    num,
                    offset,
                    stm_offset: 0,
                    has_stm_buf: 0,
                    has_obj: 0,
                };
                x.num_objects = x.num_objects.max(num + 1);
                return 1;
            }
        }

        // Create new subsection if none exists
        if x.subsections.is_empty() {
            let mut subsec = XrefSubsection::new(0);
            while subsec.entries.len() <= num as usize {
                let entry_num = subsec.entries.len() as i32;
                subsec.entries.push(XrefEntry::free(entry_num, 0));
            }
            subsec.entries[num as usize] = XrefEntry {
                entry_type,
                marked: 0,
                generation,
                num,
                offset,
                stm_offset: 0,
                has_stm_buf: 0,
                has_obj: 0,
            };
            x.num_objects = num + 1;
            x.subsections.push(subsec);
            return 1;
        }
    }
    0
}

/// Mark an entry for garbage collection.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_mark_xref(_ctx: ContextHandle, xref: Handle, num: i32) -> i32 {
    if let Some(x) = XREFS.get(xref) {
        let mut x = x.lock().unwrap();
        if let Some(entry) = x.get_entry_mut(num) {
            entry.marked = 1;
            return 1;
        }
    }
    0
}

/// Clear all marks.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_clear_xref_marks(_ctx: ContextHandle, xref: Handle) {
    if let Some(x) = XREFS.get(xref) {
        let mut x = x.lock().unwrap();
        for subsec in &mut x.subsections {
            for entry in &mut subsec.entries {
                entry.marked = 0;
            }
        }
    }
}

// ============================================================================
// FFI Functions - Trailer
// ============================================================================

/// Get the trailer dictionary.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_trailer(_ctx: ContextHandle, xref: Handle) -> PdfObjHandle {
    if let Some(x) = XREFS.get(xref) {
        let x = x.lock().unwrap();
        return x.trailer;
    }
    0
}

/// Set the trailer dictionary.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_set_trailer(_ctx: ContextHandle, xref: Handle, trailer: PdfObjHandle) -> i32 {
    if let Some(x) = XREFS.get(xref) {
        let mut x = x.lock().unwrap();
        x.trailer = trailer;
        return 1;
    }
    0
}

// ============================================================================
// FFI Functions - Stream Operations
// ============================================================================

/// Update stream contents.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_update_stream(
    _ctx: ContextHandle,
    xref: Handle,
    num: i32,
    buffer: BufferHandle,
    compressed: i32,
) -> i32 {
    if let Some(x) = XREFS.get(xref) {
        let mut x = x.lock().unwrap();
        if let Some(entry) = x.get_entry_mut(num) {
            entry.has_stm_buf = if compressed != 0 { 2 } else { 1 };
            x.stream_cache.insert(num, buffer);
            return 1;
        }
    }
    0
}

/// Get cached stream buffer.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_get_stream_buffer(
    _ctx: ContextHandle,
    xref: Handle,
    num: i32,
) -> BufferHandle {
    if let Some(x) = XREFS.get(xref) {
        let x = x.lock().unwrap();
        if let Some(&buf) = x.stream_cache.get(&num) {
            return buf;
        }
    }
    0
}

/// Check if object is a local object.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_is_local_object(_ctx: ContextHandle, xref: Handle, num: i32) -> i32 {
    if let Some(x) = XREFS.get(xref) {
        let x = x.lock().unwrap();
        // Local objects are in the last xref section (incremental save)
        if let Some(subsec) = x.subsections.last() {
            let idx = num - subsec.start;
            if idx >= 0 && (idx as usize) < subsec.entries.len() {
                return 1;
            }
        }
    }
    0
}

// ============================================================================
// FFI Functions - Utility
// ============================================================================

/// Get entry type as string.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_xref_entry_type_string(_ctx: ContextHandle, entry_type: i32) -> *mut c_char {
    let s = match entry_type {
        PDF_XREF_FREE => "f",
        PDF_XREF_INUSE => "n",
        PDF_XREF_OBJSTM => "o",
        PDF_XREF_COMPRESSED => "c",
        _ => "?",
    };

    if let Ok(cstr) = CString::new(s) {
        return cstr.into_raw();
    }
    ptr::null_mut()
}

/// Free a string.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_xref_free_string(s: *mut c_char) {
    if !s.is_null() {
        unsafe {
            drop(CString::from_raw(s));
        }
    }
}

/// Get end offset.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_xref_end_offset(_ctx: ContextHandle, xref: Handle) -> i64 {
    if let Some(x) = XREFS.get(xref) {
        let x = x.lock().unwrap();
        return x.end_offset;
    }
    0
}

/// Set end offset.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_xref_set_end_offset(_ctx: ContextHandle, xref: Handle, offset: i64) -> i32 {
    if let Some(x) = XREFS.get(xref) {
        let mut x = x.lock().unwrap();
        x.end_offset = offset;
        return 1;
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
    fn test_xref_entry_constants() {
        assert_eq!(PDF_XREF_FREE, 0);
        assert_eq!(PDF_XREF_INUSE, 1);
        assert_eq!(PDF_XREF_OBJSTM, 2);
        assert_eq!(PDF_XREF_COMPRESSED, 3);
    }

    #[test]
    fn test_xref_entry() {
        let entry = XrefEntry::free(0, 65535);
        assert_eq!(entry.entry_type, PDF_XREF_FREE);
        assert_eq!(entry.generation, 65535);

        let entry = XrefEntry::inuse(1, 0, 12345);
        assert_eq!(entry.entry_type, PDF_XREF_INUSE);
        assert_eq!(entry.num, 1);
        assert_eq!(entry.offset, 12345);

        let entry = XrefEntry::objstm(5, 10, 2);
        assert_eq!(entry.entry_type, PDF_XREF_OBJSTM);
        assert_eq!(entry.offset, 10); // objstm number
        assert_eq!(entry.generation, 2); // index
    }

    #[test]
    fn test_xref_subsection() {
        let mut subsec = XrefSubsection::new(0);
        subsec.entries.push(XrefEntry::free(0, 65535));
        subsec.entries.push(XrefEntry::inuse(1, 0, 100));

        assert_eq!(subsec.start, 0);
        assert_eq!(subsec.entries.len(), 2);
    }

    #[test]
    fn test_xref() {
        let mut xref = Xref::new(1);

        let mut subsec = XrefSubsection::new(0);
        subsec.entries.push(XrefEntry::free(0, 65535));
        subsec.entries.push(XrefEntry::inuse(1, 0, 100));
        subsec.entries.push(XrefEntry::inuse(2, 0, 200));
        xref.add_subsection(subsec);

        assert_eq!(xref.num_objects, 3);
        assert!(xref.object_exists(1));
        assert!(xref.object_exists(2));
        assert!(!xref.object_exists(0)); // Free

        let entry = xref.get_entry(1).unwrap();
        assert_eq!(entry.offset, 100);
    }

    #[test]
    fn test_xref_create_delete() {
        let mut xref = Xref::new(1);

        let num = xref.create_object();
        assert!(num >= 0);
        assert!(xref.object_exists(num));

        xref.delete_object(num);
        assert!(!xref.object_exists(num));
    }

    #[test]
    fn test_ffi_xref() {
        let ctx = 0;
        let doc = 1;

        let xref = pdf_new_xref(ctx, doc);
        assert!(xref > 0);

        assert_eq!(pdf_xref_len(ctx, xref), 0);
        assert_eq!(pdf_version(ctx, xref), 17);

        pdf_set_version(ctx, xref, 20);
        assert_eq!(pdf_version(ctx, xref), 20);

        pdf_drop_xref(ctx, xref);
    }

    #[test]
    fn test_ffi_create_object() {
        let ctx = 0;
        let doc = 1;

        let xref = pdf_new_xref(ctx, doc);

        let num = pdf_create_object(ctx, xref);
        assert!(num >= 0);
        assert_eq!(pdf_object_exists(ctx, xref, num), 1);

        pdf_delete_object(ctx, xref, num);
        assert_eq!(pdf_object_exists(ctx, xref, num), 0);

        pdf_drop_xref(ctx, xref);
    }

    #[test]
    fn test_ffi_xref_entry() {
        let ctx = 0;
        let doc = 1;

        let xref = pdf_new_xref(ctx, doc);

        // Set an entry
        let result = pdf_xref_set_entry(ctx, xref, 5, PDF_XREF_INUSE, 0, 5000);
        assert_eq!(result, 1);

        // Get the entry
        let mut entry = XrefEntry::new();
        let result = pdf_get_xref_entry(ctx, xref, 5, &mut entry);
        assert_eq!(result, 1);
        assert_eq!(entry.entry_type, PDF_XREF_INUSE);
        assert_eq!(entry.offset, 5000);

        pdf_drop_xref(ctx, xref);
    }

    #[test]
    fn test_ffi_trailer() {
        let ctx = 0;
        let doc = 1;

        let xref = pdf_new_xref(ctx, doc);

        assert_eq!(pdf_trailer(ctx, xref), 0);

        pdf_set_trailer(ctx, xref, 100);
        assert_eq!(pdf_trailer(ctx, xref), 100);

        pdf_drop_xref(ctx, xref);
    }

    #[test]
    fn test_ffi_stream() {
        let ctx = 0;
        let doc = 1;

        let xref = pdf_new_xref(ctx, doc);

        // Create object first
        pdf_xref_set_entry(ctx, xref, 1, PDF_XREF_INUSE, 0, 100);

        // Update stream
        let buf_handle = 42;
        let result = pdf_update_stream(ctx, xref, 1, buf_handle, 0);
        assert_eq!(result, 1);

        assert_eq!(pdf_get_stream_buffer(ctx, xref, 1), buf_handle);

        pdf_drop_xref(ctx, xref);
    }

    #[test]
    fn test_ffi_marks() {
        let ctx = 0;
        let doc = 1;

        let xref = pdf_new_xref(ctx, doc);

        pdf_xref_set_entry(ctx, xref, 1, PDF_XREF_INUSE, 0, 100);
        pdf_xref_set_entry(ctx, xref, 2, PDF_XREF_INUSE, 0, 200);

        pdf_mark_xref(ctx, xref, 1);

        let mut entry = XrefEntry::new();
        pdf_get_xref_entry(ctx, xref, 1, &mut entry);
        assert_eq!(entry.marked, 1);

        pdf_get_xref_entry(ctx, xref, 2, &mut entry);
        assert_eq!(entry.marked, 0);

        pdf_clear_xref_marks(ctx, xref);

        pdf_get_xref_entry(ctx, xref, 1, &mut entry);
        assert_eq!(entry.marked, 0);

        pdf_drop_xref(ctx, xref);
    }

    #[test]
    fn test_ffi_entry_type_string() {
        let ctx = 0;

        let s = pdf_xref_entry_type_string(ctx, PDF_XREF_FREE);
        assert!(!s.is_null());
        unsafe {
            let str = CStr::from_ptr(s).to_string_lossy();
            assert_eq!(str, "f");
            pdf_xref_free_string(s);
        }

        let s = pdf_xref_entry_type_string(ctx, PDF_XREF_INUSE);
        assert!(!s.is_null());
        unsafe {
            let str = CStr::from_ptr(s).to_string_lossy();
            assert_eq!(str, "n");
            pdf_xref_free_string(s);
        }
    }
}
