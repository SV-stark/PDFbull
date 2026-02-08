//! PDF CMap FFI Module
//!
//! Provides Character Map (CMap) support for PDF text encoding,
//! including CID/Unicode mapping and vertical writing mode.

use crate::ffi::{Handle, HandleStore};
use std::collections::HashMap;
use std::ffi::{CStr, CString, c_char};
use std::ptr;
use std::sync::{Arc, LazyLock, Mutex};

// ============================================================================
// Constants
// ============================================================================

/// Maximum 1-to-many mapping length (256 characters for ToUnicode CMaps)
pub const PDF_MRANGE_CAP: usize = 256;

/// Maximum codespace entries
pub const PDF_CODESPACE_MAX: usize = 40;

// ============================================================================
// Type Aliases
// ============================================================================

type ContextHandle = Handle;
type DocumentHandle = Handle;
type PdfObjHandle = Handle;
type StreamHandle = Handle;

// ============================================================================
// Writing Mode
// ============================================================================

/// Writing mode for CMap
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(C)]
pub enum WritingMode {
    /// Horizontal writing mode (default)
    #[default]
    Horizontal = 0,
    /// Vertical writing mode
    Vertical = 1,
}

impl WritingMode {
    pub fn from_i32(value: i32) -> Self {
        match value {
            1 => WritingMode::Vertical,
            _ => WritingMode::Horizontal,
        }
    }
}

// ============================================================================
// Range Structures
// ============================================================================

/// Simple range mapping (16-bit)
#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct CMapRange {
    pub low: u16,
    pub high: u16,
    pub out: u16,
}

/// Extended range mapping (32-bit)
#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct CMapXRange {
    pub low: u32,
    pub high: u32,
    pub out: u32,
}

/// One-to-many range mapping
#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct CMapMRange {
    pub low: u32,
    pub out: u32,
}

/// Codespace entry
#[derive(Debug, Clone, Copy, Default)]
pub struct CodespaceEntry {
    /// Number of bytes
    pub n: i32,
    /// Low value
    pub low: u32,
    /// High value
    pub high: u32,
}

// ============================================================================
// CMap Structure
// ============================================================================

/// Character Map for PDF text encoding
#[derive(Debug, Clone)]
pub struct CMap {
    /// CMap name
    pub name: String,
    /// UseCMap name (for cascading)
    pub usecmap_name: Option<String>,
    /// Reference to parent CMap
    pub usecmap: Option<Handle>,
    /// Writing mode (0=horizontal, 1=vertical)
    pub wmode: WritingMode,
    /// Codespace ranges
    pub codespace: Vec<CodespaceEntry>,
    /// Simple ranges (16-bit)
    pub ranges: Vec<CMapRange>,
    /// Extended ranges (32-bit)
    pub xranges: Vec<CMapXRange>,
    /// One-to-many ranges
    pub mranges: Vec<CMapMRange>,
    /// Dictionary for one-to-many lookups
    pub dict: HashMap<u32, Vec<i32>>,
    /// Reference count
    pub refs: i32,
}

impl Default for CMap {
    fn default() -> Self {
        Self::new()
    }
}

impl CMap {
    pub fn new() -> Self {
        Self {
            name: String::new(),
            usecmap_name: None,
            usecmap: None,
            wmode: WritingMode::Horizontal,
            codespace: Vec::new(),
            ranges: Vec::new(),
            xranges: Vec::new(),
            mranges: Vec::new(),
            dict: HashMap::new(),
            refs: 1,
        }
    }

    /// Create a new CMap with a name
    pub fn with_name(name: &str) -> Self {
        let mut cmap = Self::new();
        cmap.name = name.to_string();
        cmap
    }

    /// Get the writing mode
    pub fn wmode(&self) -> WritingMode {
        self.wmode
    }

    /// Set the writing mode
    pub fn set_wmode(&mut self, wmode: WritingMode) {
        self.wmode = wmode;
    }

    /// Add a codespace entry
    pub fn add_codespace(&mut self, low: u32, high: u32, n: usize) {
        if self.codespace.len() < PDF_CODESPACE_MAX {
            self.codespace.push(CodespaceEntry {
                n: n as i32,
                low,
                high,
            });
        }
    }

    /// Map a range to another range
    pub fn map_range_to_range(&mut self, src_lo: u32, src_hi: u32, dst_lo: i32) {
        // Use 16-bit ranges if possible
        if src_lo <= 0xFFFF && src_hi <= 0xFFFF && dst_lo >= 0 && dst_lo <= 0xFFFF {
            self.ranges.push(CMapRange {
                low: src_lo as u16,
                high: src_hi as u16,
                out: dst_lo as u16,
            });
        } else {
            self.xranges.push(CMapXRange {
                low: src_lo,
                high: src_hi,
                out: dst_lo as u32,
            });
        }
    }

    /// Map one codepoint to many
    pub fn map_one_to_many(&mut self, one: u32, many: &[i32]) {
        if many.len() <= PDF_MRANGE_CAP {
            self.mranges.push(CMapMRange {
                low: one,
                out: self.dict.len() as u32,
            });
            self.dict.insert(one, many.to_vec());
        }
    }

    /// Sort the CMap for efficient lookup
    pub fn sort(&mut self) {
        self.ranges.sort_by_key(|r| r.low);
        self.xranges.sort_by_key(|r| r.low);
        self.mranges.sort_by_key(|r| r.low);
    }

    /// Lookup a codepoint
    pub fn lookup(&self, cpt: u32) -> i32 {
        // Check ranges first
        for range in &self.ranges {
            if cpt >= range.low as u32 && cpt <= range.high as u32 {
                return range.out as i32 + (cpt - range.low as u32) as i32;
            }
        }

        // Check extended ranges
        for xrange in &self.xranges {
            if cpt >= xrange.low && cpt <= xrange.high {
                return xrange.out as i32 + (cpt - xrange.low) as i32;
            }
        }

        // Check parent CMap
        if let Some(parent_handle) = self.usecmap {
            if let Some(parent_arc) = CMAPS.get(parent_handle) {
                let parent = parent_arc.lock().unwrap();
                return parent.lookup(cpt);
            }
        }

        // Not found - return codepoint itself (identity mapping)
        cpt as i32
    }

    /// Lookup a codepoint with full output
    pub fn lookup_full(&self, cpt: u32) -> (i32, Vec<i32>) {
        // Check one-to-many mappings first
        if let Some(many) = self.dict.get(&cpt) {
            return (many.len() as i32, many.clone());
        }

        // Fall back to single lookup
        let result = self.lookup(cpt);
        (1, vec![result])
    }

    /// Decode a multi-byte encoded string
    pub fn decode(&self, data: &[u8]) -> Option<(u32, usize)> {
        for entry in &self.codespace {
            let n = entry.n as usize;
            if n > data.len() {
                continue;
            }

            let mut cpt: u32 = 0;
            for i in 0..n {
                cpt = (cpt << 8) | (data[i] as u32);
            }

            if cpt >= entry.low && cpt <= entry.high {
                return Some((cpt, n));
            }
        }

        // Default: single byte
        if !data.is_empty() {
            Some((data[0] as u32, 1))
        } else {
            None
        }
    }

    /// Get the size of the CMap in memory
    pub fn size(&self) -> usize {
        std::mem::size_of::<CMap>()
            + self.name.len()
            + self.usecmap_name.as_ref().map_or(0, |s| s.len())
            + self.codespace.len() * std::mem::size_of::<CodespaceEntry>()
            + self.ranges.len() * std::mem::size_of::<CMapRange>()
            + self.xranges.len() * std::mem::size_of::<CMapXRange>()
            + self.mranges.len() * std::mem::size_of::<CMapMRange>()
    }
}

// ============================================================================
// Identity CMap
// ============================================================================

impl CMap {
    /// Create an Identity CMap
    pub fn identity(wmode: WritingMode, bytes: i32) -> Self {
        let name = if wmode == WritingMode::Vertical {
            if bytes == 2 {
                "Identity-V"
            } else {
                "Identity-V-1"
            }
        } else if bytes == 2 {
            "Identity-H"
        } else {
            "Identity-H-1"
        };

        let mut cmap = Self::with_name(name);
        cmap.wmode = wmode;

        // Add identity codespace
        if bytes == 2 {
            cmap.add_codespace(0x0000, 0xFFFF, 2);
        } else {
            cmap.add_codespace(0x00, 0xFF, 1);
        }

        cmap
    }
}

// ============================================================================
// Built-in CMaps
// ============================================================================

/// Built-in CMap registry
#[derive(Debug, Default)]
pub struct CMapRegistry {
    cmaps: HashMap<String, CMap>,
}

impl CMapRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            cmaps: HashMap::new(),
        };
        registry.register_builtin_cmaps();
        registry
    }

    fn register_builtin_cmaps(&mut self) {
        // Identity CMaps
        self.cmaps.insert(
            "Identity-H".to_string(),
            CMap::identity(WritingMode::Horizontal, 2),
        );
        self.cmaps.insert(
            "Identity-V".to_string(),
            CMap::identity(WritingMode::Vertical, 2),
        );

        // Adobe standard CMaps (simplified versions)
        self.register_adobe_japan1();
        self.register_adobe_gb1();
        self.register_adobe_cns1();
        self.register_adobe_korea1();
    }

    fn register_adobe_japan1(&mut self) {
        // 90ms-RKSJ-H - Shift-JIS to CID
        let mut cmap = CMap::with_name("90ms-RKSJ-H");
        cmap.add_codespace(0x00, 0x80, 1);
        cmap.add_codespace(0xA0, 0xDF, 1);
        cmap.add_codespace(0x8140, 0x9FFC, 2);
        cmap.add_codespace(0xE040, 0xFCFC, 2);
        cmap.map_range_to_range(0x20, 0x7E, 1); // ASCII
        self.cmaps.insert("90ms-RKSJ-H".to_string(), cmap);

        // 90ms-RKSJ-V
        let mut cmap = CMap::with_name("90ms-RKSJ-V");
        cmap.wmode = WritingMode::Vertical;
        cmap.usecmap_name = Some("90ms-RKSJ-H".to_string());
        self.cmaps.insert("90ms-RKSJ-V".to_string(), cmap);

        // UniJIS-UTF16-H
        let mut cmap = CMap::with_name("UniJIS-UTF16-H");
        cmap.add_codespace(0x0000, 0xFFFF, 2);
        self.cmaps.insert("UniJIS-UTF16-H".to_string(), cmap);

        // UniJIS-UTF16-V
        let mut cmap = CMap::with_name("UniJIS-UTF16-V");
        cmap.wmode = WritingMode::Vertical;
        cmap.usecmap_name = Some("UniJIS-UTF16-H".to_string());
        self.cmaps.insert("UniJIS-UTF16-V".to_string(), cmap);
    }

    fn register_adobe_gb1(&mut self) {
        // GBK-EUC-H - GBK to CID
        let mut cmap = CMap::with_name("GBK-EUC-H");
        cmap.add_codespace(0x00, 0x80, 1);
        cmap.add_codespace(0x8140, 0xFEFE, 2);
        cmap.map_range_to_range(0x20, 0x7E, 1);
        self.cmaps.insert("GBK-EUC-H".to_string(), cmap);

        // UniGB-UTF16-H
        let mut cmap = CMap::with_name("UniGB-UTF16-H");
        cmap.add_codespace(0x0000, 0xFFFF, 2);
        self.cmaps.insert("UniGB-UTF16-H".to_string(), cmap);
    }

    fn register_adobe_cns1(&mut self) {
        // B5pc-H - Big5 to CID
        let mut cmap = CMap::with_name("B5pc-H");
        cmap.add_codespace(0x00, 0x80, 1);
        cmap.add_codespace(0xA140, 0xFEFE, 2);
        cmap.map_range_to_range(0x20, 0x7E, 1);
        self.cmaps.insert("B5pc-H".to_string(), cmap);

        // UniCNS-UTF16-H
        let mut cmap = CMap::with_name("UniCNS-UTF16-H");
        cmap.add_codespace(0x0000, 0xFFFF, 2);
        self.cmaps.insert("UniCNS-UTF16-H".to_string(), cmap);
    }

    fn register_adobe_korea1(&mut self) {
        // KSCms-UHC-H - UHC to CID
        let mut cmap = CMap::with_name("KSCms-UHC-H");
        cmap.add_codespace(0x00, 0x80, 1);
        cmap.add_codespace(0x8141, 0xFEFE, 2);
        cmap.map_range_to_range(0x20, 0x7E, 1);
        self.cmaps.insert("KSCms-UHC-H".to_string(), cmap);

        // UniKS-UTF16-H
        let mut cmap = CMap::with_name("UniKS-UTF16-H");
        cmap.add_codespace(0x0000, 0xFFFF, 2);
        self.cmaps.insert("UniKS-UTF16-H".to_string(), cmap);
    }

    pub fn get(&self, name: &str) -> Option<&CMap> {
        self.cmaps.get(name)
    }
}

// ============================================================================
// Global Handle Stores
// ============================================================================

pub static CMAPS: LazyLock<HandleStore<CMap>> = LazyLock::new(HandleStore::new);
pub static CMAP_REGISTRY: LazyLock<CMapRegistry> = LazyLock::new(CMapRegistry::new);

// ============================================================================
// FFI Functions - CMap Lifecycle
// ============================================================================

/// Create a new empty CMap.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_new_cmap(_ctx: ContextHandle) -> Handle {
    let cmap = CMap::new();
    CMAPS.insert(cmap)
}

/// Keep (increment reference to) a CMap.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_keep_cmap(_ctx: ContextHandle, cmap: Handle) -> Handle {
    if let Some(cmap_arc) = CMAPS.get(cmap) {
        let mut c = cmap_arc.lock().unwrap();
        c.refs += 1;
    }
    cmap
}

/// Drop a CMap.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_drop_cmap(_ctx: ContextHandle, cmap: Handle) {
    if let Some(cmap_arc) = CMAPS.get(cmap) {
        let should_remove = {
            let mut c = cmap_arc.lock().unwrap();
            c.refs -= 1;
            c.refs <= 0
        };
        if should_remove {
            CMAPS.remove(cmap);
        }
    }
}

/// Get CMap size in memory.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_cmap_size(_ctx: ContextHandle, cmap: Handle) -> usize {
    if let Some(cmap_arc) = CMAPS.get(cmap) {
        let c = cmap_arc.lock().unwrap();
        return c.size();
    }
    0
}

// ============================================================================
// FFI Functions - CMap Properties
// ============================================================================

/// Get CMap name.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_cmap_name(_ctx: ContextHandle, cmap: Handle) -> *const c_char {
    if let Some(cmap_arc) = CMAPS.get(cmap) {
        let c = cmap_arc.lock().unwrap();
        if let Ok(cstr) = CString::new(c.name.clone()) {
            return cstr.into_raw();
        }
    }
    ptr::null()
}

/// Set CMap name.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_set_cmap_name(_ctx: ContextHandle, cmap: Handle, name: *const c_char) {
    if name.is_null() {
        return;
    }
    if let Some(cmap_arc) = CMAPS.get(cmap) {
        let mut c = cmap_arc.lock().unwrap();
        if let Ok(s) = unsafe { CStr::from_ptr(name).to_str() } {
            c.name = s.to_string();
        }
    }
}

/// Get CMap writing mode.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_cmap_wmode(_ctx: ContextHandle, cmap: Handle) -> i32 {
    if let Some(cmap_arc) = CMAPS.get(cmap) {
        let c = cmap_arc.lock().unwrap();
        return c.wmode as i32;
    }
    0
}

/// Set CMap writing mode.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_set_cmap_wmode(_ctx: ContextHandle, cmap: Handle, wmode: i32) {
    if let Some(cmap_arc) = CMAPS.get(cmap) {
        let mut c = cmap_arc.lock().unwrap();
        c.wmode = WritingMode::from_i32(wmode);
    }
}

/// Set UseCMap.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_set_usecmap(_ctx: ContextHandle, cmap: Handle, usecmap: Handle) {
    if let Some(cmap_arc) = CMAPS.get(cmap) {
        let mut c = cmap_arc.lock().unwrap();
        c.usecmap = if usecmap != 0 { Some(usecmap) } else { None };

        // Get name of usecmap
        if let Some(usecmap_arc) = CMAPS.get(usecmap) {
            let uc = usecmap_arc.lock().unwrap();
            c.usecmap_name = Some(uc.name.clone());
        }
    }
}

// ============================================================================
// FFI Functions - Codespace
// ============================================================================

/// Add a codespace range.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_add_codespace(
    _ctx: ContextHandle,
    cmap: Handle,
    low: u32,
    high: u32,
    n: usize,
) {
    if let Some(cmap_arc) = CMAPS.get(cmap) {
        let mut c = cmap_arc.lock().unwrap();
        c.add_codespace(low, high, n);
    }
}

/// Get number of codespace entries.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_cmap_codespace_len(_ctx: ContextHandle, cmap: Handle) -> i32 {
    if let Some(cmap_arc) = CMAPS.get(cmap) {
        let c = cmap_arc.lock().unwrap();
        return c.codespace.len() as i32;
    }
    0
}

// ============================================================================
// FFI Functions - Mappings
// ============================================================================

/// Map a range of codepoints to another range.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_map_range_to_range(
    _ctx: ContextHandle,
    cmap: Handle,
    srclo: u32,
    srchi: u32,
    dstlo: i32,
) {
    if let Some(cmap_arc) = CMAPS.get(cmap) {
        let mut c = cmap_arc.lock().unwrap();
        c.map_range_to_range(srclo, srchi, dstlo);
    }
}

/// Map one codepoint to many.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_map_one_to_many(
    _ctx: ContextHandle,
    cmap: Handle,
    one: u32,
    many: *const i32,
    len: usize,
) {
    if many.is_null() || len == 0 {
        return;
    }
    if let Some(cmap_arc) = CMAPS.get(cmap) {
        let mut c = cmap_arc.lock().unwrap();
        let many_slice = unsafe { std::slice::from_raw_parts(many, len) };
        c.map_one_to_many(one, many_slice);
    }
}

/// Sort CMap for efficient lookup.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_sort_cmap(_ctx: ContextHandle, cmap: Handle) {
    if let Some(cmap_arc) = CMAPS.get(cmap) {
        let mut c = cmap_arc.lock().unwrap();
        c.sort();
    }
}

// ============================================================================
// FFI Functions - Lookup
// ============================================================================

/// Lookup a codepoint.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_lookup_cmap(cmap: Handle, cpt: u32) -> i32 {
    if let Some(cmap_arc) = CMAPS.get(cmap) {
        let c = cmap_arc.lock().unwrap();
        return c.lookup(cpt);
    }
    cpt as i32
}

/// Lookup a codepoint with full output.
/// Returns number of output codepoints, fills out array.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_lookup_cmap_full(cmap: Handle, cpt: u32, out: *mut i32) -> i32 {
    if let Some(cmap_arc) = CMAPS.get(cmap) {
        let c = cmap_arc.lock().unwrap();
        let (len, values) = c.lookup_full(cpt);
        if !out.is_null() && !values.is_empty() {
            unsafe {
                *out = values[0];
            }
        }
        return len;
    }
    1
}

/// Decode a multi-byte encoded string.
/// Returns bytes consumed, sets cpt to decoded codepoint.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_decode_cmap(cmap: Handle, s: *const u8, e: *const u8, cpt: *mut u32) -> i32 {
    if s.is_null() || e.is_null() || cpt.is_null() {
        return 0;
    }

    let len = unsafe { e.offset_from(s) as usize };
    if len == 0 {
        return 0;
    }

    let data = unsafe { std::slice::from_raw_parts(s, len) };

    if let Some(cmap_arc) = CMAPS.get(cmap) {
        let c = cmap_arc.lock().unwrap();
        if let Some((decoded_cpt, bytes)) = c.decode(data) {
            unsafe {
                *cpt = decoded_cpt;
            }
            return bytes as i32;
        }
    }

    // Default: single byte
    if !data.is_empty() {
        unsafe {
            *cpt = data[0] as u32;
        }
        return 1;
    }

    0
}

// ============================================================================
// FFI Functions - Identity CMap
// ============================================================================

/// Create an Identity CMap.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_new_identity_cmap(_ctx: ContextHandle, wmode: i32, bytes: i32) -> Handle {
    let cmap = CMap::identity(WritingMode::from_i32(wmode), bytes);
    CMAPS.insert(cmap)
}

// ============================================================================
// FFI Functions - Load CMap
// ============================================================================

/// Load a built-in CMap by name.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_load_builtin_cmap(_ctx: ContextHandle, name: *const c_char) -> Handle {
    if name.is_null() {
        return 0;
    }

    let name_str = unsafe { CStr::from_ptr(name).to_str().unwrap_or("") };
    if let Some(cmap) = CMAP_REGISTRY.get(name_str) {
        return CMAPS.insert(cmap.clone());
    }

    0
}

/// Load a system CMap by name (same as builtin for now).
#[unsafe(no_mangle)]
pub extern "C" fn pdf_load_system_cmap(_ctx: ContextHandle, name: *const c_char) -> Handle {
    pdf_load_builtin_cmap(_ctx, name)
}

/// Load CMap from a stream (simplified - creates empty CMap).
#[unsafe(no_mangle)]
pub extern "C" fn pdf_load_cmap(_ctx: ContextHandle, _file: StreamHandle) -> Handle {
    // In a full implementation, this would parse CMap data from the stream
    let cmap = CMap::new();
    CMAPS.insert(cmap)
}

/// Load embedded CMap from PDF document.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_load_embedded_cmap(
    _ctx: ContextHandle,
    _doc: DocumentHandle,
    _ref: PdfObjHandle,
) -> Handle {
    // In a full implementation, this would load and parse the CMap from PDF
    let cmap = CMap::new();
    CMAPS.insert(cmap)
}

// ============================================================================
// FFI Functions - CMap Information
// ============================================================================

/// Get number of ranges.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_cmap_range_count(_ctx: ContextHandle, cmap: Handle) -> i32 {
    if let Some(cmap_arc) = CMAPS.get(cmap) {
        let c = cmap_arc.lock().unwrap();
        return c.ranges.len() as i32;
    }
    0
}

/// Get number of extended ranges.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_cmap_xrange_count(_ctx: ContextHandle, cmap: Handle) -> i32 {
    if let Some(cmap_arc) = CMAPS.get(cmap) {
        let c = cmap_arc.lock().unwrap();
        return c.xranges.len() as i32;
    }
    0
}

/// Get number of one-to-many ranges.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_cmap_mrange_count(_ctx: ContextHandle, cmap: Handle) -> i32 {
    if let Some(cmap_arc) = CMAPS.get(cmap) {
        let c = cmap_arc.lock().unwrap();
        return c.mranges.len() as i32;
    }
    0
}

/// Check if CMap has usecmap.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_cmap_has_usecmap(_ctx: ContextHandle, cmap: Handle) -> i32 {
    if let Some(cmap_arc) = CMAPS.get(cmap) {
        let c = cmap_arc.lock().unwrap();
        return if c.usecmap.is_some() { 1 } else { 0 };
    }
    0
}

/// Free a string allocated by CMap functions.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_cmap_free_string(_ctx: ContextHandle, s: *mut c_char) {
    if !s.is_null() {
        unsafe {
            drop(CString::from_raw(s));
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cmap_new() {
        let cmap = CMap::new();
        assert!(cmap.name.is_empty());
        assert_eq!(cmap.wmode, WritingMode::Horizontal);
        assert!(cmap.codespace.is_empty());
        assert_eq!(cmap.refs, 1);
    }

    #[test]
    fn test_cmap_with_name() {
        let cmap = CMap::with_name("Test-CMap");
        assert_eq!(cmap.name, "Test-CMap");
    }

    #[test]
    fn test_writing_mode() {
        assert_eq!(WritingMode::from_i32(0), WritingMode::Horizontal);
        assert_eq!(WritingMode::from_i32(1), WritingMode::Vertical);
        assert_eq!(WritingMode::from_i32(99), WritingMode::Horizontal);
    }

    #[test]
    fn test_codespace() {
        let mut cmap = CMap::new();
        cmap.add_codespace(0x00, 0x7F, 1);
        cmap.add_codespace(0x8000, 0xFFFF, 2);

        assert_eq!(cmap.codespace.len(), 2);
        assert_eq!(cmap.codespace[0].n, 1);
        assert_eq!(cmap.codespace[0].low, 0x00);
        assert_eq!(cmap.codespace[0].high, 0x7F);
        assert_eq!(cmap.codespace[1].n, 2);
    }

    #[test]
    fn test_range_mapping() {
        let mut cmap = CMap::new();
        cmap.map_range_to_range(0x20, 0x7E, 1);

        assert_eq!(cmap.ranges.len(), 1);
        assert_eq!(cmap.lookup(0x20), 1);
        assert_eq!(cmap.lookup(0x21), 2);
        assert_eq!(cmap.lookup(0x7E), 95);
    }

    #[test]
    fn test_extended_range() {
        let mut cmap = CMap::new();
        cmap.map_range_to_range(0x10000, 0x1FFFF, 1000);

        assert_eq!(cmap.xranges.len(), 1);
        assert_eq!(cmap.lookup(0x10000), 1000);
        assert_eq!(cmap.lookup(0x10001), 1001);
    }

    #[test]
    fn test_one_to_many() {
        let mut cmap = CMap::new();
        let many = vec![0x0066, 0x0069]; // 'fi' ligature
        cmap.map_one_to_many(0xFB01, &many);

        let (len, values) = cmap.lookup_full(0xFB01);
        assert_eq!(len, 2);
        assert_eq!(values, vec![0x0066, 0x0069]);
    }

    #[test]
    fn test_decode_single_byte() {
        let mut cmap = CMap::new();
        cmap.add_codespace(0x00, 0xFF, 1);

        let data = [0x41u8, 0x42, 0x43]; // "ABC"
        if let Some((cpt, bytes)) = cmap.decode(&data) {
            assert_eq!(cpt, 0x41);
            assert_eq!(bytes, 1);
        }
    }

    #[test]
    fn test_decode_two_byte() {
        let mut cmap = CMap::new();
        cmap.add_codespace(0x8140, 0x9FFC, 2);

        let data = [0x82u8, 0xA0]; // Two-byte code
        if let Some((cpt, bytes)) = cmap.decode(&data) {
            assert_eq!(cpt, 0x82A0);
            assert_eq!(bytes, 2);
        }
    }

    #[test]
    fn test_identity_cmap() {
        let cmap = CMap::identity(WritingMode::Horizontal, 2);
        assert_eq!(cmap.name, "Identity-H");
        assert_eq!(cmap.wmode, WritingMode::Horizontal);
        assert!(!cmap.codespace.is_empty());

        let cmap_v = CMap::identity(WritingMode::Vertical, 2);
        assert_eq!(cmap_v.name, "Identity-V");
        assert_eq!(cmap_v.wmode, WritingMode::Vertical);
    }

    #[test]
    fn test_cmap_sort() {
        let mut cmap = CMap::new();
        cmap.map_range_to_range(0x80, 0xFF, 200);
        cmap.map_range_to_range(0x00, 0x7F, 1);
        cmap.map_range_to_range(0x40, 0x5F, 100);

        cmap.sort();

        // Should be sorted by low value
        assert_eq!(cmap.ranges[0].low, 0x00);
        assert_eq!(cmap.ranges[1].low, 0x40);
        assert_eq!(cmap.ranges[2].low, 0x80);
    }

    #[test]
    fn test_ffi_lifecycle() {
        let ctx = 0;

        let cmap = pdf_new_cmap(ctx);
        assert!(cmap > 0);

        let kept = pdf_keep_cmap(ctx, cmap);
        assert_eq!(kept, cmap);

        pdf_drop_cmap(ctx, cmap);
        pdf_drop_cmap(ctx, cmap);
    }

    #[test]
    fn test_ffi_properties() {
        let ctx = 0;

        let cmap = pdf_new_cmap(ctx);

        // Set name
        let name = CString::new("Test-CMap").unwrap();
        pdf_set_cmap_name(ctx, cmap, name.as_ptr());

        // Get name
        let got_name = pdf_cmap_name(ctx, cmap);
        assert!(!got_name.is_null());
        unsafe {
            assert_eq!(CStr::from_ptr(got_name).to_str().unwrap(), "Test-CMap");
            pdf_cmap_free_string(ctx, got_name as *mut c_char);
        }

        // Set wmode
        pdf_set_cmap_wmode(ctx, cmap, 1);
        assert_eq!(pdf_cmap_wmode(ctx, cmap), 1);

        pdf_drop_cmap(ctx, cmap);
    }

    #[test]
    fn test_ffi_codespace() {
        let ctx = 0;

        let cmap = pdf_new_cmap(ctx);
        pdf_add_codespace(ctx, cmap, 0x00, 0x7F, 1);
        pdf_add_codespace(ctx, cmap, 0x8000, 0xFFFF, 2);

        assert_eq!(pdf_cmap_codespace_len(ctx, cmap), 2);

        pdf_drop_cmap(ctx, cmap);
    }

    #[test]
    fn test_ffi_range_mapping() {
        let ctx = 0;

        let cmap = pdf_new_cmap(ctx);
        pdf_map_range_to_range(ctx, cmap, 0x20, 0x7E, 1);

        assert_eq!(pdf_cmap_range_count(ctx, cmap), 1);
        assert_eq!(pdf_lookup_cmap(cmap, 0x41), 34); // 'A' -> 34

        pdf_drop_cmap(ctx, cmap);
    }

    #[test]
    fn test_ffi_decode() {
        let ctx = 0;

        let cmap = pdf_new_cmap(ctx);
        pdf_add_codespace(ctx, cmap, 0x00, 0xFF, 1);

        let data = [0x41u8, 0x42, 0x43];
        let mut cpt: u32 = 0;
        let bytes = pdf_decode_cmap(
            cmap,
            data.as_ptr(),
            unsafe { data.as_ptr().add(3) },
            &mut cpt,
        );

        assert_eq!(bytes, 1);
        assert_eq!(cpt, 0x41);

        pdf_drop_cmap(ctx, cmap);
    }

    #[test]
    fn test_ffi_identity_cmap() {
        let ctx = 0;

        let cmap = pdf_new_identity_cmap(ctx, 0, 2);
        assert!(cmap > 0);

        let name = pdf_cmap_name(ctx, cmap);
        assert!(!name.is_null());
        unsafe {
            assert_eq!(CStr::from_ptr(name).to_str().unwrap(), "Identity-H");
            pdf_cmap_free_string(ctx, name as *mut c_char);
        }

        assert_eq!(pdf_cmap_wmode(ctx, cmap), 0);

        pdf_drop_cmap(ctx, cmap);
    }

    #[test]
    fn test_ffi_builtin_cmap() {
        let ctx = 0;
        let name = CString::new("Identity-H").unwrap();

        let cmap = pdf_load_builtin_cmap(ctx, name.as_ptr());
        assert!(cmap > 0);

        let got_name = pdf_cmap_name(ctx, cmap);
        unsafe {
            assert_eq!(CStr::from_ptr(got_name).to_str().unwrap(), "Identity-H");
            pdf_cmap_free_string(ctx, got_name as *mut c_char);
        }

        pdf_drop_cmap(ctx, cmap);
    }

    #[test]
    fn test_cmap_registry() {
        let registry = CMapRegistry::new();

        // Check Identity CMaps
        assert!(registry.get("Identity-H").is_some());
        assert!(registry.get("Identity-V").is_some());

        // Check Japanese CMaps
        assert!(registry.get("90ms-RKSJ-H").is_some());
        assert!(registry.get("UniJIS-UTF16-H").is_some());

        // Check Chinese CMaps
        assert!(registry.get("GBK-EUC-H").is_some());
        assert!(registry.get("UniGB-UTF16-H").is_some());

        // Check unknown CMap
        assert!(registry.get("Unknown-CMap").is_none());
    }

    #[test]
    fn test_cmap_size() {
        let ctx = 0;

        let cmap = pdf_new_cmap(ctx);
        let size = pdf_cmap_size(ctx, cmap);
        assert!(size > 0);

        pdf_drop_cmap(ctx, cmap);
    }
}
