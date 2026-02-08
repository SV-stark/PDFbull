//! PDF Name Table FFI Module
//!
//! Provides PDF name string optimization through interning and
//! standard PDF name constants for efficient name comparisons.

use std::collections::HashMap;
use std::ffi::{CStr, CString, c_char};
use std::ptr;
use std::sync::{LazyLock, Mutex};

// ============================================================================
// Standard PDF Name Constants
// ============================================================================

// Document Structure
pub const PDF_NAME_TYPE: &str = "Type";
pub const PDF_NAME_SUBTYPE: &str = "Subtype";
pub const PDF_NAME_CATALOG: &str = "Catalog";
pub const PDF_NAME_PAGES: &str = "Pages";
pub const PDF_NAME_PAGE: &str = "Page";
pub const PDF_NAME_PARENT: &str = "Parent";
pub const PDF_NAME_KIDS: &str = "Kids";
pub const PDF_NAME_COUNT: &str = "Count";
pub const PDF_NAME_ROOT: &str = "Root";
pub const PDF_NAME_INFO: &str = "Info";
pub const PDF_NAME_METADATA: &str = "Metadata";

// Page Properties
pub const PDF_NAME_MEDIABOX: &str = "MediaBox";
pub const PDF_NAME_CROPBOX: &str = "CropBox";
pub const PDF_NAME_BLEEDBOX: &str = "BleedBox";
pub const PDF_NAME_TRIMBOX: &str = "TrimBox";
pub const PDF_NAME_ARTBOX: &str = "ArtBox";
pub const PDF_NAME_RESOURCES: &str = "Resources";
pub const PDF_NAME_CONTENTS: &str = "Contents";
pub const PDF_NAME_ROTATE: &str = "Rotate";
pub const PDF_NAME_USERUNIT: &str = "UserUnit";

// Resources
pub const PDF_NAME_EXTGSTATE: &str = "ExtGState";
pub const PDF_NAME_COLORSPACE: &str = "ColorSpace";
pub const PDF_NAME_PATTERN: &str = "Pattern";
pub const PDF_NAME_SHADING: &str = "Shading";
pub const PDF_NAME_XOBJECT: &str = "XObject";
pub const PDF_NAME_FONT: &str = "Font";
pub const PDF_NAME_PROCSET: &str = "ProcSet";
pub const PDF_NAME_PROPERTIES: &str = "Properties";

// XObject Types
pub const PDF_NAME_IMAGE: &str = "Image";
pub const PDF_NAME_FORM: &str = "Form";

// Stream Properties
pub const PDF_NAME_LENGTH: &str = "Length";
pub const PDF_NAME_FILTER: &str = "Filter";
pub const PDF_NAME_DECODEPARMS: &str = "DecodeParms";

// Filters
pub const PDF_NAME_ASCIIHEXDECODE: &str = "ASCIIHexDecode";
pub const PDF_NAME_ASCII85DECODE: &str = "ASCII85Decode";
pub const PDF_NAME_LZWDECODE: &str = "LZWDecode";
pub const PDF_NAME_FLATEDECODE: &str = "FlateDecode";
pub const PDF_NAME_RUNLENGTHDECODE: &str = "RunLengthDecode";
pub const PDF_NAME_CCITTFAXDECODE: &str = "CCITTFaxDecode";
pub const PDF_NAME_JBIG2DECODE: &str = "JBIG2Decode";
pub const PDF_NAME_DCTDECODE: &str = "DCTDecode";
pub const PDF_NAME_JPXDECODE: &str = "JPXDecode";
pub const PDF_NAME_CRYPT: &str = "Crypt";
pub const PDF_NAME_BROTLIDECODE: &str = "BrotliDecode";

// Color Spaces
pub const PDF_NAME_DEVICEGRAY: &str = "DeviceGray";
pub const PDF_NAME_DEVICERGB: &str = "DeviceRGB";
pub const PDF_NAME_DEVICECMYK: &str = "DeviceCMYK";
pub const PDF_NAME_CALGRAY: &str = "CalGray";
pub const PDF_NAME_CALRGB: &str = "CalRGB";
pub const PDF_NAME_LAB: &str = "Lab";
pub const PDF_NAME_ICCBASED: &str = "ICCBased";
pub const PDF_NAME_INDEXED: &str = "Indexed";
pub const PDF_NAME_PATTERN_CS: &str = "Pattern";
pub const PDF_NAME_SEPARATION: &str = "Separation";
pub const PDF_NAME_DEVICEN: &str = "DeviceN";

// Font Types
pub const PDF_NAME_TYPE0: &str = "Type0";
pub const PDF_NAME_TYPE1: &str = "Type1";
pub const PDF_NAME_MMTYPE1: &str = "MMType1";
pub const PDF_NAME_TYPE3: &str = "Type3";
pub const PDF_NAME_TRUETYPE: &str = "TrueType";
pub const PDF_NAME_CIDFONTTYPE0: &str = "CIDFontType0";
pub const PDF_NAME_CIDFONTTYPE2: &str = "CIDFontType2";

// Font Properties
pub const PDF_NAME_BASEFONT: &str = "BaseFont";
pub const PDF_NAME_ENCODING: &str = "Encoding";
pub const PDF_NAME_DESCENDANTFONTS: &str = "DescendantFonts";
pub const PDF_NAME_FONTDESCRIPTOR: &str = "FontDescriptor";
pub const PDF_NAME_WIDTHS: &str = "Widths";
pub const PDF_NAME_FIRSTCHAR: &str = "FirstChar";
pub const PDF_NAME_LASTCHAR: &str = "LastChar";
pub const PDF_NAME_TOUNICODE: &str = "ToUnicode";

// Font Descriptor
pub const PDF_NAME_FONTNAME: &str = "FontName";
pub const PDF_NAME_FONTFAMILY: &str = "FontFamily";
pub const PDF_NAME_FLAGS: &str = "Flags";
pub const PDF_NAME_FONTBBOX: &str = "FontBBox";
pub const PDF_NAME_ITALICANGLE: &str = "ItalicAngle";
pub const PDF_NAME_ASCENT: &str = "Ascent";
pub const PDF_NAME_DESCENT: &str = "Descent";
pub const PDF_NAME_CAPHEIGHT: &str = "CapHeight";
pub const PDF_NAME_STEMV: &str = "StemV";
pub const PDF_NAME_FONTFILE: &str = "FontFile";
pub const PDF_NAME_FONTFILE2: &str = "FontFile2";
pub const PDF_NAME_FONTFILE3: &str = "FontFile3";

// Image Properties
pub const PDF_NAME_WIDTH: &str = "Width";
pub const PDF_NAME_HEIGHT: &str = "Height";
pub const PDF_NAME_BITSPERCOMPONENT: &str = "BitsPerComponent";
pub const PDF_NAME_IMAGEMASK: &str = "ImageMask";
pub const PDF_NAME_MASK: &str = "Mask";
pub const PDF_NAME_SMASK: &str = "SMask";
pub const PDF_NAME_DECODE: &str = "Decode";
pub const PDF_NAME_INTERPOLATE: &str = "Interpolate";
pub const PDF_NAME_INTENT: &str = "Intent";

// Annotations
pub const PDF_NAME_ANNOT: &str = "Annot";
pub const PDF_NAME_ANNOTS: &str = "Annots";
pub const PDF_NAME_RECT: &str = "Rect";
pub const PDF_NAME_BORDER: &str = "Border";
pub const PDF_NAME_AP: &str = "AP";
pub const PDF_NAME_N: &str = "N";
pub const PDF_NAME_R: &str = "R";
pub const PDF_NAME_D: &str = "D";
pub const PDF_NAME_AS: &str = "AS";
pub const PDF_NAME_POPUP: &str = "Popup";

// Annotation Types
pub const PDF_NAME_TEXT: &str = "Text";
pub const PDF_NAME_LINK: &str = "Link";
pub const PDF_NAME_FREETEXT: &str = "FreeText";
pub const PDF_NAME_LINE: &str = "Line";
pub const PDF_NAME_SQUARE: &str = "Square";
pub const PDF_NAME_CIRCLE: &str = "Circle";
pub const PDF_NAME_POLYGON: &str = "Polygon";
pub const PDF_NAME_POLYLINE: &str = "PolyLine";
pub const PDF_NAME_HIGHLIGHT: &str = "Highlight";
pub const PDF_NAME_UNDERLINE: &str = "Underline";
pub const PDF_NAME_SQUIGGLY: &str = "Squiggly";
pub const PDF_NAME_STRIKEOUT: &str = "StrikeOut";
pub const PDF_NAME_STAMP: &str = "Stamp";
pub const PDF_NAME_CARET: &str = "Caret";
pub const PDF_NAME_INK: &str = "Ink";
pub const PDF_NAME_FILEATTACHMENT: &str = "FileAttachment";
pub const PDF_NAME_SOUND: &str = "Sound";
pub const PDF_NAME_MOVIE: &str = "Movie";
pub const PDF_NAME_WIDGET: &str = "Widget";
pub const PDF_NAME_SCREEN: &str = "Screen";
pub const PDF_NAME_PRINTERMARK: &str = "PrinterMark";
pub const PDF_NAME_TRAPNET: &str = "TrapNet";
pub const PDF_NAME_WATERMARK: &str = "Watermark";
pub const PDF_NAME_3D: &str = "3D";
pub const PDF_NAME_REDACT: &str = "Redact";

// Actions
pub const PDF_NAME_ACTION: &str = "Action";
pub const PDF_NAME_A: &str = "A";
pub const PDF_NAME_S: &str = "S";
pub const PDF_NAME_GOTO: &str = "GoTo";
pub const PDF_NAME_GOTOR: &str = "GoToR";
pub const PDF_NAME_GOTOE: &str = "GoToE";
pub const PDF_NAME_LAUNCH: &str = "Launch";
pub const PDF_NAME_URI: &str = "URI";
pub const PDF_NAME_NAMED: &str = "Named";
pub const PDF_NAME_SUBMITFORM: &str = "SubmitForm";
pub const PDF_NAME_RESETFORM: &str = "ResetForm";
pub const PDF_NAME_JAVASCRIPT: &str = "JavaScript";

// Destinations
pub const PDF_NAME_DEST: &str = "Dest";
pub const PDF_NAME_XYZ: &str = "XYZ";
pub const PDF_NAME_FIT: &str = "Fit";
pub const PDF_NAME_FITH: &str = "FitH";
pub const PDF_NAME_FITV: &str = "FitV";
pub const PDF_NAME_FITR: &str = "FitR";
pub const PDF_NAME_FITB: &str = "FitB";
pub const PDF_NAME_FITBH: &str = "FitBH";
pub const PDF_NAME_FITBV: &str = "FitBV";

// Forms (AcroForm)
pub const PDF_NAME_ACROFORM: &str = "AcroForm";
pub const PDF_NAME_FIELDS: &str = "Fields";
pub const PDF_NAME_FT: &str = "FT";
pub const PDF_NAME_BTN: &str = "Btn";
pub const PDF_NAME_TX: &str = "Tx";
pub const PDF_NAME_CH: &str = "Ch";
pub const PDF_NAME_SIG: &str = "Sig";
pub const PDF_NAME_T: &str = "T";
pub const PDF_NAME_V: &str = "V";
pub const PDF_NAME_DV: &str = "DV";
pub const PDF_NAME_FF: &str = "Ff";
pub const PDF_NAME_OPT: &str = "Opt";

// Metadata
pub const PDF_NAME_TITLE: &str = "Title";
pub const PDF_NAME_AUTHOR: &str = "Author";
pub const PDF_NAME_SUBJECT: &str = "Subject";
pub const PDF_NAME_KEYWORDS: &str = "Keywords";
pub const PDF_NAME_CREATOR: &str = "Creator";
pub const PDF_NAME_PRODUCER: &str = "Producer";
pub const PDF_NAME_CREATIONDATE: &str = "CreationDate";
pub const PDF_NAME_MODDATE: &str = "ModDate";

// Encryption
pub const PDF_NAME_ENCRYPT: &str = "Encrypt";
pub const PDF_NAME_STANDARD: &str = "Standard";
pub const PDF_NAME_P: &str = "P";
pub const PDF_NAME_O: &str = "O";
pub const PDF_NAME_U: &str = "U";
pub const PDF_NAME_OE: &str = "OE";
pub const PDF_NAME_UE: &str = "UE";
pub const PDF_NAME_CF: &str = "CF";
pub const PDF_NAME_STMF: &str = "StmF";
pub const PDF_NAME_STRF: &str = "StrF";

// Outlines (Bookmarks)
pub const PDF_NAME_OUTLINES: &str = "Outlines";
pub const PDF_NAME_FIRST: &str = "First";
pub const PDF_NAME_LAST: &str = "Last";
pub const PDF_NAME_NEXT: &str = "Next";
pub const PDF_NAME_PREV: &str = "Prev";
pub const PDF_NAME_C: &str = "C";
pub const PDF_NAME_F: &str = "F";

// Graphics State
pub const PDF_NAME_CA: &str = "CA";
pub const PDF_NAME_CA_LOWER: &str = "ca";
pub const PDF_NAME_BM: &str = "BM";
pub const PDF_NAME_LW: &str = "LW";
pub const PDF_NAME_LC: &str = "LC";
pub const PDF_NAME_LJ: &str = "LJ";
pub const PDF_NAME_ML: &str = "ML";
pub const PDF_NAME_RI: &str = "RI";
pub const PDF_NAME_OP: &str = "OP";
pub const PDF_NAME_OP_LOWER: &str = "op";
pub const PDF_NAME_OPM: &str = "OPM";
pub const PDF_NAME_SA: &str = "SA";

// XRef
pub const PDF_NAME_XREF: &str = "XRef";
pub const PDF_NAME_SIZE: &str = "Size";
pub const PDF_NAME_INDEX: &str = "Index";
pub const PDF_NAME_W: &str = "W";

// ============================================================================
// Name Interning Registry
// ============================================================================

/// Interned name entry
#[derive(Debug, Clone)]
struct InternedName {
    /// The name string
    name: String,
    /// Reference count
    ref_count: u32,
    /// Hash code for fast comparison
    hash: u64,
}

/// Name interning registry
struct NameRegistry {
    /// Map from string to index
    name_to_index: HashMap<String, usize>,
    /// List of interned names
    names: Vec<InternedName>,
    /// Total lookups
    lookups: u64,
    /// Cache hits
    hits: u64,
}

impl NameRegistry {
    fn new() -> Self {
        let mut registry = Self {
            // Pre-size for 150+ standard names plus common custom names
            name_to_index: HashMap::with_capacity(200),
            names: Vec::with_capacity(200),
            lookups: 0,
            hits: 0,
        };
        // Pre-populate with standard names
        registry.populate_standard_names();
        registry
    }

    fn populate_standard_names(&mut self) {
        let standard_names = [
            PDF_NAME_TYPE,
            PDF_NAME_SUBTYPE,
            PDF_NAME_CATALOG,
            PDF_NAME_PAGES,
            PDF_NAME_PAGE,
            PDF_NAME_PARENT,
            PDF_NAME_KIDS,
            PDF_NAME_COUNT,
            PDF_NAME_ROOT,
            PDF_NAME_INFO,
            PDF_NAME_METADATA,
            PDF_NAME_MEDIABOX,
            PDF_NAME_CROPBOX,
            PDF_NAME_RESOURCES,
            PDF_NAME_CONTENTS,
            PDF_NAME_LENGTH,
            PDF_NAME_FILTER,
            PDF_NAME_WIDTH,
            PDF_NAME_HEIGHT,
            PDF_NAME_FONT,
            PDF_NAME_IMAGE,
            PDF_NAME_FORM,
            PDF_NAME_XOBJECT,
            PDF_NAME_EXTGSTATE,
            PDF_NAME_COLORSPACE,
            PDF_NAME_FLATEDECODE,
            PDF_NAME_DCTDECODE,
            PDF_NAME_DEVICEGRAY,
            PDF_NAME_DEVICERGB,
            PDF_NAME_DEVICECMYK,
            PDF_NAME_BASEFONT,
            PDF_NAME_ENCODING,
            PDF_NAME_WIDTHS,
            PDF_NAME_ANNOTS,
            PDF_NAME_RECT,
            PDF_NAME_AP,
            PDF_NAME_ACROFORM,
            PDF_NAME_FIELDS,
            PDF_NAME_TITLE,
            PDF_NAME_AUTHOR,
            PDF_NAME_ENCRYPT,
            PDF_NAME_OUTLINES,
        ];

        for name in standard_names {
            self.intern_internal(name);
        }
    }

    fn hash_name(name: &str) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        name.hash(&mut hasher);
        hasher.finish()
    }

    fn intern_internal(&mut self, name: &str) -> usize {
        if let Some(&idx) = self.name_to_index.get(name) {
            self.names[idx].ref_count += 1;
            return idx;
        }

        let idx = self.names.len();
        let entry = InternedName {
            name: name.to_string(),
            ref_count: 1,
            hash: Self::hash_name(name),
        };
        self.names.push(entry);
        self.name_to_index.insert(name.to_string(), idx);
        idx
    }

    fn intern(&mut self, name: &str) -> usize {
        self.lookups += 1;
        if self.name_to_index.contains_key(name) {
            self.hits += 1;
        }
        self.intern_internal(name)
    }

    fn get(&self, idx: usize) -> Option<&str> {
        self.names.get(idx).map(|n| n.name.as_str())
    }

    fn lookup(&self, name: &str) -> Option<usize> {
        self.name_to_index.get(name).copied()
    }

    fn release(&mut self, idx: usize) {
        if let Some(entry) = self.names.get_mut(idx) {
            if entry.ref_count > 0 {
                entry.ref_count -= 1;
            }
        }
    }
}

static NAME_REGISTRY: LazyLock<Mutex<NameRegistry>> =
    LazyLock::new(|| Mutex::new(NameRegistry::new()));

// ============================================================================
// FFI Functions - Name Interning
// ============================================================================

/// Intern a PDF name string, returning an index.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_intern_name(name: *const c_char) -> i32 {
    if name.is_null() {
        return -1;
    }
    unsafe {
        let s = CStr::from_ptr(name).to_string_lossy();
        let mut registry = NAME_REGISTRY.lock().unwrap();
        registry.intern(&s) as i32
    }
}

/// Get an interned name by index.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_get_interned_name(idx: i32) -> *mut c_char {
    if idx < 0 {
        return ptr::null_mut();
    }
    let registry = NAME_REGISTRY.lock().unwrap();
    if let Some(name) = registry.get(idx as usize) {
        if let Ok(cstr) = CString::new(name) {
            return cstr.into_raw();
        }
    }
    ptr::null_mut()
}

/// Lookup a name index without interning.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_lookup_name(name: *const c_char) -> i32 {
    if name.is_null() {
        return -1;
    }
    unsafe {
        let s = CStr::from_ptr(name).to_string_lossy();
        let registry = NAME_REGISTRY.lock().unwrap();
        registry.lookup(&s).map(|i| i as i32).unwrap_or(-1)
    }
}

/// Release a reference to an interned name.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_release_name(idx: i32) {
    if idx >= 0 {
        let mut registry = NAME_REGISTRY.lock().unwrap();
        registry.release(idx as usize);
    }
}

/// Compare two name indices for equality.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_name_index_eq(a: i32, b: i32) -> i32 {
    if a == b { 1 } else { 0 }
}

/// Compare a name index with a string.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_name_eq_str(idx: i32, name: *const c_char) -> i32 {
    if idx < 0 || name.is_null() {
        return 0;
    }
    unsafe {
        let s = CStr::from_ptr(name).to_string_lossy();
        let registry = NAME_REGISTRY.lock().unwrap();
        if let Some(interned) = registry.get(idx as usize) {
            return if interned == s { 1 } else { 0 };
        }
    }
    0
}

/// Free a name string returned by pdf_get_interned_name.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_free_name_string(name: *mut c_char) {
    if !name.is_null() {
        unsafe {
            drop(CString::from_raw(name));
        }
    }
}

// ============================================================================
// FFI Functions - Standard Names
// ============================================================================

/// Get the index of a standard PDF name.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_std_name_type() -> i32 {
    let mut registry = NAME_REGISTRY.lock().unwrap();
    registry.intern(PDF_NAME_TYPE) as i32
}

/// Get the index of the Subtype name.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_std_name_subtype() -> i32 {
    let mut registry = NAME_REGISTRY.lock().unwrap();
    registry.intern(PDF_NAME_SUBTYPE) as i32
}

/// Get the index of the Length name.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_std_name_length() -> i32 {
    let mut registry = NAME_REGISTRY.lock().unwrap();
    registry.intern(PDF_NAME_LENGTH) as i32
}

/// Get the index of the Filter name.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_std_name_filter() -> i32 {
    let mut registry = NAME_REGISTRY.lock().unwrap();
    registry.intern(PDF_NAME_FILTER) as i32
}

/// Get the index of the Font name.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_std_name_font() -> i32 {
    let mut registry = NAME_REGISTRY.lock().unwrap();
    registry.intern(PDF_NAME_FONT) as i32
}

/// Get the index of the Image name.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_std_name_image() -> i32 {
    let mut registry = NAME_REGISTRY.lock().unwrap();
    registry.intern(PDF_NAME_IMAGE) as i32
}

// ============================================================================
// FFI Functions - Statistics
// ============================================================================

/// Get the number of interned names.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_name_table_count() -> i32 {
    let registry = NAME_REGISTRY.lock().unwrap();
    registry.names.len() as i32
}

/// Get the total number of lookups.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_name_table_lookups() -> u64 {
    let registry = NAME_REGISTRY.lock().unwrap();
    registry.lookups
}

/// Get the number of cache hits.
#[unsafe(no_mangle)]
pub extern "C" fn pdf_name_table_hits() -> u64 {
    let registry = NAME_REGISTRY.lock().unwrap();
    registry.hits
}

/// Get the hit rate (0.0 - 1.0).
#[unsafe(no_mangle)]
pub extern "C" fn pdf_name_table_hit_rate() -> f64 {
    let registry = NAME_REGISTRY.lock().unwrap();
    if registry.lookups == 0 {
        return 0.0;
    }
    registry.hits as f64 / registry.lookups as f64
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_standard_names() {
        assert_eq!(PDF_NAME_TYPE, "Type");
        assert_eq!(PDF_NAME_SUBTYPE, "Subtype");
        assert_eq!(PDF_NAME_CATALOG, "Catalog");
        assert_eq!(PDF_NAME_PAGES, "Pages");
        assert_eq!(PDF_NAME_FONT, "Font");
        assert_eq!(PDF_NAME_IMAGE, "Image");
    }

    #[test]
    fn test_filter_names() {
        assert_eq!(PDF_NAME_FLATEDECODE, "FlateDecode");
        assert_eq!(PDF_NAME_DCTDECODE, "DCTDecode");
        assert_eq!(PDF_NAME_ASCII85DECODE, "ASCII85Decode");
    }

    #[test]
    fn test_colorspace_names() {
        assert_eq!(PDF_NAME_DEVICEGRAY, "DeviceGray");
        assert_eq!(PDF_NAME_DEVICERGB, "DeviceRGB");
        assert_eq!(PDF_NAME_DEVICECMYK, "DeviceCMYK");
    }

    #[test]
    fn test_ffi_intern() {
        let name = CString::new("TestName").unwrap();
        let idx = pdf_intern_name(name.as_ptr());
        assert!(idx >= 0);

        // Same name should return same index
        let idx2 = pdf_intern_name(name.as_ptr());
        assert_eq!(idx, idx2);
    }

    #[test]
    fn test_ffi_get_interned() {
        let name = CString::new("AnotherTest").unwrap();
        let idx = pdf_intern_name(name.as_ptr());
        assert!(idx >= 0);

        let result = pdf_get_interned_name(idx);
        assert!(!result.is_null());
        unsafe {
            let s = CStr::from_ptr(result).to_string_lossy();
            assert_eq!(s, "AnotherTest");
            pdf_free_name_string(result);
        }
    }

    #[test]
    fn test_ffi_lookup() {
        let name = CString::new("Type").unwrap();
        let idx = pdf_lookup_name(name.as_ptr());
        assert!(idx >= 0); // Should be pre-populated

        let unknown = CString::new("UnknownName12345").unwrap();
        let idx2 = pdf_lookup_name(unknown.as_ptr());
        assert_eq!(idx2, -1);
    }

    #[test]
    fn test_ffi_name_eq() {
        let name1 = CString::new("Same").unwrap();
        let idx1 = pdf_intern_name(name1.as_ptr());
        let idx2 = pdf_intern_name(name1.as_ptr());

        assert_eq!(pdf_name_index_eq(idx1, idx2), 1);
        assert_eq!(pdf_name_index_eq(idx1, -1), 0);
    }

    #[test]
    fn test_ffi_name_eq_str() {
        let name = CString::new("TestEq").unwrap();
        let idx = pdf_intern_name(name.as_ptr());

        let same = CString::new("TestEq").unwrap();
        let diff = CString::new("Different").unwrap();

        assert_eq!(pdf_name_eq_str(idx, same.as_ptr()), 1);
        assert_eq!(pdf_name_eq_str(idx, diff.as_ptr()), 0);
    }

    #[test]
    fn test_ffi_std_names() {
        let type_idx = pdf_std_name_type();
        let subtype_idx = pdf_std_name_subtype();

        assert!(type_idx >= 0);
        assert!(subtype_idx >= 0);
        assert_ne!(type_idx, subtype_idx);
    }

    #[test]
    fn test_ffi_table_count() {
        let count = pdf_name_table_count();
        assert!(count > 0);
    }

    #[test]
    fn test_null_handling() {
        assert_eq!(pdf_intern_name(ptr::null()), -1);
        assert!(pdf_get_interned_name(-1).is_null());
        assert_eq!(pdf_lookup_name(ptr::null()), -1);
        assert_eq!(pdf_name_eq_str(-1, ptr::null()), 0);
    }
}
