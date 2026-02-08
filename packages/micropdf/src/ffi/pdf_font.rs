//! PDF Font FFI Module
//!
//! Provides PDF-specific font handling including font descriptors,
//! CID/GID/Unicode mapping, metrics, and font embedding.

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
type PdfObjHandle = Handle;
type FontHandle = Handle;
type CMapHandle = Handle;
type BufferHandle = Handle;
type DeviceHandle = Handle;
type OutputHandle = Handle;

// ============================================================================
// Font Descriptor Flags
// ============================================================================

/// Font is fixed-pitch (monospace)
pub const PDF_FD_FIXED_PITCH: i32 = 1 << 0;
/// Font has serifs
pub const PDF_FD_SERIF: i32 = 1 << 1;
/// Font uses symbolic character set
pub const PDF_FD_SYMBOLIC: i32 = 1 << 2;
/// Font is script/cursive
pub const PDF_FD_SCRIPT: i32 = 1 << 3;
/// Font uses standard Latin character set
pub const PDF_FD_NONSYMBOLIC: i32 = 1 << 5;
/// Font is italic
pub const PDF_FD_ITALIC: i32 = 1 << 6;
/// Font uses all capital letters
pub const PDF_FD_ALL_CAP: i32 = 1 << 16;
/// Font uses small capitals
pub const PDF_FD_SMALL_CAP: i32 = 1 << 17;
/// Font always renders bold
pub const PDF_FD_FORCE_BOLD: i32 = 1 << 18;

// ============================================================================
// Font Encoding Constants
// ============================================================================

/// Standard encoding
pub const PDF_ENCODING_STANDARD: i32 = 0;
/// MacRoman encoding
pub const PDF_ENCODING_MAC_ROMAN: i32 = 1;
/// WinAnsi encoding
pub const PDF_ENCODING_WIN_ANSI: i32 = 2;
/// MacExpert encoding
pub const PDF_ENCODING_MAC_EXPERT: i32 = 3;
/// Symbol encoding
pub const PDF_ENCODING_SYMBOL: i32 = 4;
/// ZapfDingbats encoding
pub const PDF_ENCODING_ZAPF_DINGBATS: i32 = 5;

// ============================================================================
// CJK Script Constants
// ============================================================================

/// Adobe-CNS1 (Traditional Chinese)
pub const PDF_CJK_CNS1: i32 = 0;
/// Adobe-GB1 (Simplified Chinese)
pub const PDF_CJK_GB1: i32 = 1;
/// Adobe-Japan1 (Japanese)
pub const PDF_CJK_JAPAN1: i32 = 2;
/// Adobe-Korea1 (Korean)
pub const PDF_CJK_KOREA1: i32 = 3;

// ============================================================================
// Horizontal Metrics
// ============================================================================

/// Horizontal metrics entry
#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct HorizontalMetrics {
    /// Low CID
    pub lo: u16,
    /// High CID
    pub hi: u16,
    /// Width
    pub w: i32,
}

// ============================================================================
// Vertical Metrics
// ============================================================================

/// Vertical metrics entry
#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct VerticalMetrics {
    /// Low CID
    pub lo: u16,
    /// High CID
    pub hi: u16,
    /// X displacement
    pub x: i16,
    /// Y displacement
    pub y: i16,
    /// Width
    pub w: i16,
}

// ============================================================================
// Font Descriptor
// ============================================================================

/// PDF Font Descriptor
#[derive(Debug, Clone)]
pub struct FontDesc {
    /// Reference count
    pub refs: i32,
    /// Memory size
    pub size: usize,

    /// Underlying fz_font handle (if any)
    pub font: Option<FontHandle>,
    /// Font name
    pub name: String,

    // FontDescriptor fields
    /// Font flags
    pub flags: i32,
    /// Italic angle in degrees
    pub italic_angle: f32,
    /// Ascender
    pub ascent: f32,
    /// Descender
    pub descent: f32,
    /// Cap height
    pub cap_height: f32,
    /// X-height
    pub x_height: f32,
    /// Missing glyph width
    pub missing_width: f32,

    // Encoding
    /// Encoding CMap
    pub encoding: Option<CMapHandle>,
    /// ToTTF CMap
    pub to_ttf_cmap: Option<CMapHandle>,
    /// CID to GID mapping
    pub cid_to_gid: Vec<u16>,

    // ToUnicode
    /// ToUnicode CMap
    pub to_unicode: Option<CMapHandle>,
    /// CID to UCS mapping
    pub cid_to_ucs: Vec<u16>,

    // Metrics
    /// Writing mode (0=horizontal, 1=vertical)
    pub wmode: i32,
    /// Default horizontal metrics
    pub default_hmtx: HorizontalMetrics,
    /// Horizontal metrics table
    pub hmtx: Vec<HorizontalMetrics>,
    /// Default vertical metrics
    pub default_vmtx: VerticalMetrics,
    /// Vertical metrics table
    pub vmtx: Vec<VerticalMetrics>,

    /// Is embedded font
    pub is_embedded: bool,
    /// Type 3 font loading flag
    pub t3loading: bool,

    // Type 3 specific
    /// Type 3 glyph contents
    pub t3_glyphs: HashMap<i32, Vec<u8>>,
    /// Type 3 font matrix
    pub t3_matrix: [f32; 6],
    /// Type 3 bounding box
    pub t3_bbox: [f32; 4],
}

impl Default for FontDesc {
    fn default() -> Self {
        Self::new()
    }
}

impl FontDesc {
    pub fn new() -> Self {
        Self {
            refs: 1,
            size: std::mem::size_of::<FontDesc>(),
            font: None,
            name: String::new(),
            flags: 0,
            italic_angle: 0.0,
            ascent: 0.0,
            descent: 0.0,
            cap_height: 0.0,
            x_height: 0.0,
            missing_width: 0.0,
            encoding: None,
            to_ttf_cmap: None,
            cid_to_gid: Vec::new(),
            to_unicode: None,
            cid_to_ucs: Vec::new(),
            wmode: 0,
            default_hmtx: HorizontalMetrics::default(),
            hmtx: Vec::new(),
            default_vmtx: VerticalMetrics::default(),
            vmtx: Vec::new(),
            is_embedded: false,
            t3loading: false,
            t3_glyphs: HashMap::new(),
            t3_matrix: [0.001, 0.0, 0.0, 0.001, 0.0, 0.0],
            t3_bbox: [0.0, 0.0, 1000.0, 1000.0],
        }
    }

    /// Create a font descriptor with a name
    pub fn with_name(name: &str) -> Self {
        let mut fd = Self::new();
        fd.name = name.to_string();
        fd
    }

    /// Set writing mode
    pub fn set_wmode(&mut self, wmode: i32) {
        self.wmode = wmode;
    }

    /// Set default horizontal metrics
    pub fn set_default_hmtx(&mut self, w: i32) {
        self.default_hmtx = HorizontalMetrics { lo: 0, hi: 0, w };
    }

    /// Set default vertical metrics
    pub fn set_default_vmtx(&mut self, y: i16, w: i16) {
        self.default_vmtx = VerticalMetrics {
            lo: 0,
            hi: 0,
            x: 0,
            y,
            w,
        };
    }

    /// Add horizontal metrics entry
    pub fn add_hmtx(&mut self, lo: u16, hi: u16, w: i32) {
        self.hmtx.push(HorizontalMetrics { lo, hi, w });
    }

    /// Add vertical metrics entry
    pub fn add_vmtx(&mut self, lo: u16, hi: u16, x: i16, y: i16, w: i16) {
        self.vmtx.push(VerticalMetrics { lo, hi, x, y, w });
    }

    /// Sort and finalize horizontal metrics
    pub fn end_hmtx(&mut self) {
        self.hmtx.sort_by_key(|m| m.lo);
    }

    /// Sort and finalize vertical metrics
    pub fn end_vmtx(&mut self) {
        self.vmtx.sort_by_key(|m| m.lo);
    }

    /// Lookup horizontal metrics for a CID
    pub fn lookup_hmtx(&self, cid: i32) -> HorizontalMetrics {
        let cid = cid as u16;
        for m in &self.hmtx {
            if cid >= m.lo && cid <= m.hi {
                return HorizontalMetrics {
                    lo: cid,
                    hi: cid,
                    w: m.w, // Range has same width for all CIDs
                };
            }
        }
        self.default_hmtx
    }

    /// Lookup vertical metrics for a CID
    pub fn lookup_vmtx(&self, cid: i32) -> VerticalMetrics {
        let cid = cid as u16;
        for m in &self.vmtx {
            if cid >= m.lo && cid <= m.hi {
                return *m;
            }
        }
        self.default_vmtx
    }

    /// Map CID to GID
    pub fn cid_to_gid(&self, cid: i32) -> i32 {
        if cid < 0 {
            return 0;
        }
        if (cid as usize) < self.cid_to_gid.len() {
            self.cid_to_gid[cid as usize] as i32
        } else {
            cid // Identity mapping if no table
        }
    }

    /// Map CID to Unicode
    pub fn cid_to_unicode(&self, cid: i32) -> i32 {
        if cid < 0 {
            return 0;
        }
        if (cid as usize) < self.cid_to_ucs.len() {
            self.cid_to_ucs[cid as usize] as i32
        } else {
            cid // Identity mapping if no table
        }
    }

    /// Check if font is fixed-pitch
    pub fn is_fixed_pitch(&self) -> bool {
        (self.flags & PDF_FD_FIXED_PITCH) != 0
    }

    /// Check if font is serif
    pub fn is_serif(&self) -> bool {
        (self.flags & PDF_FD_SERIF) != 0
    }

    /// Check if font is symbolic
    pub fn is_symbolic(&self) -> bool {
        (self.flags & PDF_FD_SYMBOLIC) != 0
    }

    /// Check if font is italic
    pub fn is_italic(&self) -> bool {
        (self.flags & PDF_FD_ITALIC) != 0
    }

    /// Update memory size
    pub fn update_size(&mut self) {
        self.size = std::mem::size_of::<FontDesc>()
            + self.name.len()
            + self.cid_to_gid.len() * 2
            + self.cid_to_ucs.len() * 2
            + self.hmtx.len() * std::mem::size_of::<HorizontalMetrics>()
            + self.vmtx.len() * std::mem::size_of::<VerticalMetrics>();
    }
}

// ============================================================================
// Standard Encodings
// ============================================================================

/// Get encoding names for a standard encoding
pub fn get_encoding_names(encoding: i32) -> Vec<&'static str> {
    match encoding {
        PDF_ENCODING_MAC_ROMAN => vec![
            ".notdef",
            "space",
            "exclam",
            "quotedbl",
            "numbersign",
            "dollar",
            "percent",
            "ampersand",
            "quotesingle",
            "parenleft",
            "parenright",
            "asterisk",
            "plus",
            "comma",
            "hyphen",
            "period",
            "slash",
            "zero",
            "one",
            "two",
            "three",
            "four",
            "five",
            "six",
            "seven",
            "eight",
            "nine",
            "colon",
            "semicolon",
            "less",
            "equal",
            "greater",
        ],
        PDF_ENCODING_WIN_ANSI => vec![
            ".notdef",
            "space",
            "exclam",
            "quotedbl",
            "numbersign",
            "dollar",
            "percent",
            "ampersand",
            "quotesingle",
            "parenleft",
            "parenright",
            "asterisk",
            "plus",
            "comma",
            "hyphen",
            "period",
            "slash",
            "zero",
            "one",
            "two",
            "three",
            "four",
            "five",
            "six",
            "seven",
            "eight",
            "nine",
            "colon",
            "semicolon",
            "less",
            "equal",
            "greater",
        ],
        _ => vec![
            ".notdef",
            "space",
            "exclam",
            "quotedbl",
            "numbersign",
            "dollar",
            "percent",
            "ampersand",
            "quoteright",
            "parenleft",
            "parenright",
            "asterisk",
            "plus",
            "comma",
            "hyphen",
            "period",
            "slash",
            "zero",
            "one",
            "two",
            "three",
            "four",
            "five",
            "six",
            "seven",
            "eight",
            "nine",
            "colon",
            "semicolon",
            "less",
            "equal",
            "greater",
        ],
    }
}

// ============================================================================
// Substitute Font Data
// ============================================================================

/// Substitute font information
#[derive(Debug, Clone)]
pub struct SubstituteFont {
    pub name: String,
    pub mono: bool,
    pub serif: bool,
    pub bold: bool,
    pub italic: bool,
}

impl SubstituteFont {
    pub fn lookup(mono: bool, serif: bool, bold: bool, italic: bool) -> Self {
        let name = match (mono, serif, bold, italic) {
            (true, _, true, true) => "Courier-BoldOblique",
            (true, _, true, false) => "Courier-Bold",
            (true, _, false, true) => "Courier-Oblique",
            (true, _, false, false) => "Courier",
            (false, true, true, true) => "Times-BoldItalic",
            (false, true, true, false) => "Times-Bold",
            (false, true, false, true) => "Times-Italic",
            (false, true, false, false) => "Times-Roman",
            (false, false, true, true) => "Helvetica-BoldOblique",
            (false, false, true, false) => "Helvetica-Bold",
            (false, false, false, true) => "Helvetica-Oblique",
            (false, false, false, false) => "Helvetica",
        };
        Self {
            name: name.to_string(),
            mono,
            serif,
            bold,
            italic,
        }
    }
}

// ============================================================================
// Global Handle Store
// ============================================================================

pub static FONT_DESCS: LazyLock<HandleStore<FontDesc>> = LazyLock::new(HandleStore::new);

// ============================================================================
// FFI Functions - Font Descriptor Lifecycle
// ============================================================================

/// Create a new font descriptor.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_new_font_desc(_ctx: ContextHandle) -> Handle {
    let fd = FontDesc::new();
    FONT_DESCS.insert(fd)
}

/// Keep (increment reference to) a font descriptor.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_keep_font(_ctx: ContextHandle, font: Handle) -> Handle {
    if let Some(font_arc) = FONT_DESCS.get(font) {
        let mut f = font_arc.lock().unwrap();
        f.refs += 1;
    }
    font
}

/// Drop a font descriptor.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_drop_font(_ctx: ContextHandle, font: Handle) {
    if let Some(font_arc) = FONT_DESCS.get(font) {
        let should_remove = {
            let mut f = font_arc.lock().unwrap();
            f.refs -= 1;
            f.refs <= 0
        };
        if should_remove {
            FONT_DESCS.remove(font);
        }
    }
}

// ============================================================================
// FFI Functions - Font Properties
// ============================================================================

/// Get font name.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_font_name(_ctx: ContextHandle, font: Handle) -> *const c_char {
    if let Some(font_arc) = FONT_DESCS.get(font) {
        let f = font_arc.lock().unwrap();
        if let Ok(cstr) = CString::new(f.name.clone()) {
            return cstr.into_raw();
        }
    }
    ptr::null()
}

/// Set font name.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_set_font_name(_ctx: ContextHandle, font: Handle, name: *const c_char) {
    if name.is_null() {
        return;
    }
    if let Some(font_arc) = FONT_DESCS.get(font) {
        let mut f = font_arc.lock().unwrap();
        if let Ok(s) = unsafe { CStr::from_ptr(name).to_str() } {
            f.name = s.to_string();
        }
    }
}

/// Get font flags.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_font_flags(_ctx: ContextHandle, font: Handle) -> i32 {
    if let Some(font_arc) = FONT_DESCS.get(font) {
        let f = font_arc.lock().unwrap();
        return f.flags;
    }
    0
}

/// Set font flags.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_set_font_flags(_ctx: ContextHandle, font: Handle, flags: i32) {
    if let Some(font_arc) = FONT_DESCS.get(font) {
        let mut f = font_arc.lock().unwrap();
        f.flags = flags;
    }
}

/// Get italic angle.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_font_italic_angle(_ctx: ContextHandle, font: Handle) -> f32 {
    if let Some(font_arc) = FONT_DESCS.get(font) {
        let f = font_arc.lock().unwrap();
        return f.italic_angle;
    }
    0.0
}

/// Get ascent.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_font_ascent(_ctx: ContextHandle, font: Handle) -> f32 {
    if let Some(font_arc) = FONT_DESCS.get(font) {
        let f = font_arc.lock().unwrap();
        return f.ascent;
    }
    0.0
}

/// Get descent.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_font_descent(_ctx: ContextHandle, font: Handle) -> f32 {
    if let Some(font_arc) = FONT_DESCS.get(font) {
        let f = font_arc.lock().unwrap();
        return f.descent;
    }
    0.0
}

/// Get cap height.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_font_cap_height(_ctx: ContextHandle, font: Handle) -> f32 {
    if let Some(font_arc) = FONT_DESCS.get(font) {
        let f = font_arc.lock().unwrap();
        return f.cap_height;
    }
    0.0
}

/// Get x-height.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_font_x_height(_ctx: ContextHandle, font: Handle) -> f32 {
    if let Some(font_arc) = FONT_DESCS.get(font) {
        let f = font_arc.lock().unwrap();
        return f.x_height;
    }
    0.0
}

/// Get missing width.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_font_missing_width(_ctx: ContextHandle, font: Handle) -> f32 {
    if let Some(font_arc) = FONT_DESCS.get(font) {
        let f = font_arc.lock().unwrap();
        return f.missing_width;
    }
    0.0
}

/// Check if font is embedded.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_font_is_embedded(_ctx: ContextHandle, font: Handle) -> i32 {
    if let Some(font_arc) = FONT_DESCS.get(font) {
        let f = font_arc.lock().unwrap();
        return if f.is_embedded { 1 } else { 0 };
    }
    0
}

// ============================================================================
// FFI Functions - Writing Mode
// ============================================================================

/// Get font writing mode.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_font_wmode(_ctx: ContextHandle, font: Handle) -> i32 {
    if let Some(font_arc) = FONT_DESCS.get(font) {
        let f = font_arc.lock().unwrap();
        return f.wmode;
    }
    0
}

/// Set font writing mode.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_set_font_wmode(_ctx: ContextHandle, font: Handle, wmode: i32) {
    if let Some(font_arc) = FONT_DESCS.get(font) {
        let mut f = font_arc.lock().unwrap();
        f.set_wmode(wmode);
    }
}

// ============================================================================
// FFI Functions - Metrics
// ============================================================================

/// Set default horizontal metrics.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_set_default_hmtx(_ctx: ContextHandle, font: Handle, w: i32) {
    if let Some(font_arc) = FONT_DESCS.get(font) {
        let mut f = font_arc.lock().unwrap();
        f.set_default_hmtx(w);
    }
}

/// Set default vertical metrics.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_set_default_vmtx(_ctx: ContextHandle, font: Handle, y: i32, w: i32) {
    if let Some(font_arc) = FONT_DESCS.get(font) {
        let mut f = font_arc.lock().unwrap();
        f.set_default_vmtx(y as i16, w as i16);
    }
}

/// Add horizontal metrics.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_add_hmtx(_ctx: ContextHandle, font: Handle, lo: i32, hi: i32, w: i32) {
    if let Some(font_arc) = FONT_DESCS.get(font) {
        let mut f = font_arc.lock().unwrap();
        f.add_hmtx(lo as u16, hi as u16, w);
    }
}

/// Add vertical metrics.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_add_vmtx(
    _ctx: ContextHandle,
    font: Handle,
    lo: i32,
    hi: i32,
    x: i32,
    y: i32,
    w: i32,
) {
    if let Some(font_arc) = FONT_DESCS.get(font) {
        let mut f = font_arc.lock().unwrap();
        f.add_vmtx(lo as u16, hi as u16, x as i16, y as i16, w as i16);
    }
}

/// Finalize horizontal metrics.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_end_hmtx(_ctx: ContextHandle, font: Handle) {
    if let Some(font_arc) = FONT_DESCS.get(font) {
        let mut f = font_arc.lock().unwrap();
        f.end_hmtx();
    }
}

/// Finalize vertical metrics.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_end_vmtx(_ctx: ContextHandle, font: Handle) {
    if let Some(font_arc) = FONT_DESCS.get(font) {
        let mut f = font_arc.lock().unwrap();
        f.end_vmtx();
    }
}

/// Lookup horizontal metrics.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_lookup_hmtx(
    _ctx: ContextHandle,
    font: Handle,
    cid: i32,
) -> HorizontalMetrics {
    if let Some(font_arc) = FONT_DESCS.get(font) {
        let f = font_arc.lock().unwrap();
        return f.lookup_hmtx(cid);
    }
    HorizontalMetrics::default()
}

/// Lookup vertical metrics.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_lookup_vmtx(_ctx: ContextHandle, font: Handle, cid: i32) -> VerticalMetrics {
    if let Some(font_arc) = FONT_DESCS.get(font) {
        let f = font_arc.lock().unwrap();
        return f.lookup_vmtx(cid);
    }
    VerticalMetrics::default()
}

// ============================================================================
// FFI Functions - CID Mapping
// ============================================================================

/// Map CID to GID.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_font_cid_to_gid(_ctx: ContextHandle, font: Handle, cid: i32) -> i32 {
    if let Some(font_arc) = FONT_DESCS.get(font) {
        let f = font_arc.lock().unwrap();
        return f.cid_to_gid(cid);
    }
    cid
}

/// Map CID to Unicode.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_font_cid_to_unicode(_ctx: ContextHandle, font: Handle, cid: i32) -> i32 {
    if let Some(font_arc) = FONT_DESCS.get(font) {
        let f = font_arc.lock().unwrap();
        return f.cid_to_unicode(cid);
    }
    cid
}

/// Set CID to GID mapping table.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_set_cid_to_gid(
    _ctx: ContextHandle,
    font: Handle,
    table: *const u16,
    len: usize,
) {
    if table.is_null() || len == 0 {
        return;
    }
    if let Some(font_arc) = FONT_DESCS.get(font) {
        let mut f = font_arc.lock().unwrap();
        f.cid_to_gid = unsafe { std::slice::from_raw_parts(table, len).to_vec() };
        f.update_size();
    }
}

/// Set CID to UCS mapping table.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_set_cid_to_ucs(
    _ctx: ContextHandle,
    font: Handle,
    table: *const u16,
    len: usize,
) {
    if table.is_null() || len == 0 {
        return;
    }
    if let Some(font_arc) = FONT_DESCS.get(font) {
        let mut f = font_arc.lock().unwrap();
        f.cid_to_ucs = unsafe { std::slice::from_raw_parts(table, len).to_vec() };
        f.update_size();
    }
}

// ============================================================================
// FFI Functions - Font Loading
// ============================================================================

/// Load font from PDF (simplified).
#[unsafe(no_mangle)]
pub extern "C" fn pdf_load_font(
    _ctx: ContextHandle,
    _doc: DocumentHandle,
    _rdb: Handle,
    _obj: PdfObjHandle,
) -> Handle {
    // In a full implementation, this would parse the PDF font object
    let fd = FontDesc::new();
    FONT_DESCS.insert(fd)
}

/// Load Type 3 font from PDF (simplified).
#[unsafe(no_mangle)]
pub extern "C" fn pdf_load_type3_font(
    _ctx: ContextHandle,
    _doc: DocumentHandle,
    _rdb: Handle,
    _obj: PdfObjHandle,
) -> Handle {
    // Create a Type 3 font descriptor
    let mut fd = FontDesc::new();
    fd.t3loading = true;
    FONT_DESCS.insert(fd)
}

/// Load Type 3 glyphs.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_load_type3_glyphs(_ctx: ContextHandle, _doc: DocumentHandle, font: Handle) {
    if let Some(font_arc) = FONT_DESCS.get(font) {
        let mut f = font_arc.lock().unwrap();
        f.t3loading = false;
    }
}

/// Load "hail mary" fallback font.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_load_hail_mary_font(_ctx: ContextHandle, _doc: DocumentHandle) -> Handle {
    let mut fd = FontDesc::with_name("Helvetica");
    fd.flags = PDF_FD_NONSYMBOLIC;
    FONT_DESCS.insert(fd)
}

// ============================================================================
// FFI Functions - Encoding
// ============================================================================

/// Load encoding strings.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_load_encoding(estrings: *mut *const c_char, encoding: *const c_char) {
    if estrings.is_null() || encoding.is_null() {
        return;
    }

    let enc_str = unsafe { CStr::from_ptr(encoding).to_str().unwrap_or("") };
    let enc_type = match enc_str {
        "MacRomanEncoding" => PDF_ENCODING_MAC_ROMAN,
        "WinAnsiEncoding" => PDF_ENCODING_WIN_ANSI,
        "MacExpertEncoding" => PDF_ENCODING_MAC_EXPERT,
        _ => PDF_ENCODING_STANDARD,
    };

    let names = get_encoding_names(enc_type);
    for (i, name) in names.iter().enumerate() {
        if let Ok(cstr) = CString::new(*name) {
            unsafe {
                *estrings.add(i) = cstr.into_raw();
            }
        }
    }
}

// ============================================================================
// FFI Functions - Substitute Fonts
// ============================================================================

/// Lookup substitute font.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_lookup_substitute_font(
    _ctx: ContextHandle,
    mono: i32,
    serif: i32,
    bold: i32,
    italic: i32,
    len: *mut i32,
) -> *const u8 {
    let sf = SubstituteFont::lookup(mono != 0, serif != 0, bold != 0, italic != 0);

    if !len.is_null() {
        unsafe {
            *len = sf.name.len() as i32;
        }
    }

    // Return pointer to static font name
    // In a real implementation, this would return font data
    ptr::null()
}

/// Clean font name.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_clean_font_name(fontname: *const c_char) -> *const c_char {
    if fontname.is_null() {
        return ptr::null();
    }

    let name = unsafe { CStr::from_ptr(fontname).to_str().unwrap_or("") };

    // Remove common prefixes
    let cleaned = name
        .trim_start_matches("AAAAAA+")
        .trim_start_matches("BAAAAA+")
        .trim_start_matches("CAAAAA+");

    if let Ok(cstr) = CString::new(cleaned) {
        return cstr.into_raw();
    }
    fontname
}

// ============================================================================
// FFI Functions - Font Addition
// ============================================================================

/// Add simple font to document.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_add_simple_font(
    _ctx: ContextHandle,
    _doc: DocumentHandle,
    _font: FontHandle,
    _encoding: i32,
) -> PdfObjHandle {
    // In a full implementation, this would create a PDF font object
    0
}

/// Add CID font to document.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_add_cid_font(
    _ctx: ContextHandle,
    _doc: DocumentHandle,
    _font: FontHandle,
) -> PdfObjHandle {
    // In a full implementation, this would create a PDF CID font object
    0
}

/// Add CJK font to document.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_add_cjk_font(
    _ctx: ContextHandle,
    _doc: DocumentHandle,
    _font: FontHandle,
    _script: i32,
    _wmode: i32,
    _serif: i32,
) -> PdfObjHandle {
    // In a full implementation, this would create a PDF CJK font object
    0
}

/// Add substitute font to document.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_add_substitute_font(
    _ctx: ContextHandle,
    _doc: DocumentHandle,
    _font: FontHandle,
) -> PdfObjHandle {
    // In a full implementation, this would create a PDF substitute font object
    0
}

/// Check if font writing is supported.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_font_writing_supported(_ctx: ContextHandle, _font: FontHandle) -> i32 {
    // Most fonts support writing
    1
}

/// Subset fonts in document.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_subset_fonts(
    _ctx: ContextHandle,
    _doc: DocumentHandle,
    _pages_len: i32,
    _pages: *const i32,
) {
    // In a full implementation, this would subset embedded fonts
}

// ============================================================================
// FFI Functions - Font Printing
// ============================================================================

/// Print font information.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_print_font(_ctx: ContextHandle, _out: OutputHandle, font: Handle) {
    if let Some(font_arc) = FONT_DESCS.get(font) {
        let f = font_arc.lock().unwrap();
        // In a full implementation, this would write to the output stream
        eprintln!("Font: {}", f.name);
        eprintln!("  Flags: 0x{:x}", f.flags);
        eprintln!("  Embedded: {}", f.is_embedded);
        eprintln!("  WMode: {}", f.wmode);
    }
}

/// Free a string allocated by font functions.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_font_free_string(_ctx: ContextHandle, s: *mut c_char) {
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
    fn test_font_desc_new() {
        let fd = FontDesc::new();
        assert!(fd.name.is_empty());
        assert_eq!(fd.flags, 0);
        assert_eq!(fd.refs, 1);
        assert!(!fd.is_embedded);
    }

    #[test]
    fn test_font_desc_with_name() {
        let fd = FontDesc::with_name("Helvetica");
        assert_eq!(fd.name, "Helvetica");
    }

    #[test]
    fn test_horizontal_metrics() {
        let mut fd = FontDesc::new();
        fd.set_default_hmtx(1000);
        fd.add_hmtx(0, 127, 500);
        fd.add_hmtx(128, 255, 600);
        fd.end_hmtx();

        let m = fd.lookup_hmtx(65); // 'A'
        assert_eq!(m.w, 500);

        let m2 = fd.lookup_hmtx(200);
        assert_eq!(m2.w, 600);

        let m3 = fd.lookup_hmtx(300); // Out of range
        assert_eq!(m3.w, 1000); // Default
    }

    #[test]
    fn test_vertical_metrics() {
        let mut fd = FontDesc::new();
        fd.set_default_vmtx(-500, 1000);
        fd.add_vmtx(0, 127, 0, -500, 1000);
        fd.end_vmtx();

        let m = fd.lookup_vmtx(65);
        assert_eq!(m.y, -500);
        assert_eq!(m.w, 1000);
    }

    #[test]
    fn test_cid_to_gid() {
        let mut fd = FontDesc::new();
        fd.cid_to_gid = vec![0, 1, 2, 100, 101, 102];

        assert_eq!(fd.cid_to_gid(0), 0);
        assert_eq!(fd.cid_to_gid(3), 100);
        assert_eq!(fd.cid_to_gid(100), 100); // Identity for out of range
    }

    #[test]
    fn test_cid_to_unicode() {
        let mut fd = FontDesc::new();
        fd.cid_to_ucs = vec![0x0000, 0x0041, 0x0042, 0x0043]; // .notdef, A, B, C

        assert_eq!(fd.cid_to_unicode(1), 0x0041); // A
        assert_eq!(fd.cid_to_unicode(2), 0x0042); // B
    }

    #[test]
    fn test_font_flags() {
        let mut fd = FontDesc::new();
        fd.flags = PDF_FD_SERIF | PDF_FD_ITALIC;

        assert!(fd.is_serif());
        assert!(fd.is_italic());
        assert!(!fd.is_fixed_pitch());
        assert!(!fd.is_symbolic());
    }

    #[test]
    fn test_substitute_font() {
        let sf = SubstituteFont::lookup(true, false, true, false);
        assert_eq!(sf.name, "Courier-Bold");

        let sf2 = SubstituteFont::lookup(false, true, false, true);
        assert_eq!(sf2.name, "Times-Italic");

        let sf3 = SubstituteFont::lookup(false, false, false, false);
        assert_eq!(sf3.name, "Helvetica");
    }

    #[test]
    fn test_ffi_lifecycle() {
        let ctx = 0;

        let font = pdf_new_font_desc(ctx);
        assert!(font > 0);

        let kept = pdf_keep_font(ctx, font);
        assert_eq!(kept, font);

        pdf_drop_font(ctx, font);
        pdf_drop_font(ctx, font);
    }

    #[test]
    fn test_ffi_properties() {
        let ctx = 0;

        let font = pdf_new_font_desc(ctx);

        // Set name
        let name = CString::new("Times-Roman").unwrap();
        pdf_set_font_name(ctx, font, name.as_ptr());

        // Get name
        let got_name = pdf_font_name(ctx, font);
        assert!(!got_name.is_null());
        unsafe {
            assert_eq!(CStr::from_ptr(got_name).to_str().unwrap(), "Times-Roman");
            pdf_font_free_string(ctx, got_name as *mut c_char);
        }

        // Set flags
        pdf_set_font_flags(ctx, font, PDF_FD_SERIF | PDF_FD_ITALIC);
        assert_eq!(pdf_font_flags(ctx, font), PDF_FD_SERIF | PDF_FD_ITALIC);

        pdf_drop_font(ctx, font);
    }

    #[test]
    fn test_ffi_wmode() {
        let ctx = 0;

        let font = pdf_new_font_desc(ctx);
        assert_eq!(pdf_font_wmode(ctx, font), 0);

        pdf_set_font_wmode(ctx, font, 1);
        assert_eq!(pdf_font_wmode(ctx, font), 1);

        pdf_drop_font(ctx, font);
    }

    #[test]
    fn test_ffi_metrics() {
        let ctx = 0;

        let font = pdf_new_font_desc(ctx);

        pdf_set_default_hmtx(ctx, font, 1000);
        pdf_add_hmtx(ctx, font, 0, 127, 500);
        pdf_end_hmtx(ctx, font);

        let m = pdf_lookup_hmtx(ctx, font, 65);
        assert_eq!(m.w, 500);

        pdf_drop_font(ctx, font);
    }

    #[test]
    fn test_ffi_cid_mapping() {
        let ctx = 0;

        let font = pdf_new_font_desc(ctx);

        let gid_table: Vec<u16> = vec![0, 1, 2, 100, 101, 102];
        pdf_set_cid_to_gid(ctx, font, gid_table.as_ptr(), gid_table.len());

        assert_eq!(pdf_font_cid_to_gid(ctx, font, 3), 100);

        pdf_drop_font(ctx, font);
    }

    #[test]
    fn test_ffi_load_font() {
        let ctx = 0;
        let doc = 0;
        let rdb = 0;
        let obj = 0;

        let font = pdf_load_font(ctx, doc, rdb, obj);
        assert!(font > 0);

        pdf_drop_font(ctx, font);
    }

    #[test]
    fn test_ffi_type3_font() {
        let ctx = 0;
        let doc = 0;

        let font = pdf_load_type3_font(ctx, doc, 0, 0);
        assert!(font > 0);

        pdf_load_type3_glyphs(ctx, doc, font);

        pdf_drop_font(ctx, font);
    }

    #[test]
    fn test_clean_font_name() {
        let name = CString::new("AAAAAA+Helvetica").unwrap();
        let cleaned = pdf_clean_font_name(name.as_ptr());
        assert!(!cleaned.is_null());
        unsafe {
            assert_eq!(CStr::from_ptr(cleaned).to_str().unwrap(), "Helvetica");
            pdf_font_free_string(0, cleaned as *mut c_char);
        }
    }

    #[test]
    fn test_hail_mary_font() {
        let ctx = 0;
        let doc = 0;

        let font = pdf_load_hail_mary_font(ctx, doc);
        assert!(font > 0);

        let name = pdf_font_name(ctx, font);
        unsafe {
            assert_eq!(CStr::from_ptr(name).to_str().unwrap(), "Helvetica");
            pdf_font_free_string(ctx, name as *mut c_char);
        }

        pdf_drop_font(ctx, font);
    }
}
