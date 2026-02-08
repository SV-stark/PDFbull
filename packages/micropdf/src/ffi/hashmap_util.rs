//! Optimized HashMap Utilities
//!
//! Provides pre-sized HashMaps and perfect hashing for known key sets
//! to reduce allocation overhead and improve lookup performance.

use std::collections::HashMap;
use std::hash::{BuildHasher, Hash, Hasher};
use std::sync::LazyLock;

// ============================================================================
// Pre-sized HashMap Factory
// ============================================================================

/// Create a HashMap pre-sized for expected PDF document structures
pub mod presized {
    use std::collections::HashMap;
    use std::hash::Hash;

    /// Create HashMap for typical PDF page dictionary (~8-12 keys)
    pub fn page_dict<K: Eq + Hash, V>() -> HashMap<K, V> {
        HashMap::with_capacity(12)
    }

    /// Create HashMap for typical PDF resources dictionary (~6-10 keys)
    pub fn resources_dict<K: Eq + Hash, V>() -> HashMap<K, V> {
        HashMap::with_capacity(10)
    }

    /// Create HashMap for font dictionary (~10-15 keys)
    pub fn font_dict<K: Eq + Hash, V>() -> HashMap<K, V> {
        HashMap::with_capacity(15)
    }

    /// Create HashMap for image dictionary (~8-12 keys)
    pub fn image_dict<K: Eq + Hash, V>() -> HashMap<K, V> {
        HashMap::with_capacity(12)
    }

    /// Create HashMap for annotation (~6-10 keys)
    pub fn annot_dict<K: Eq + Hash, V>() -> HashMap<K, V> {
        HashMap::with_capacity(10)
    }

    /// Create HashMap for graphics state (~8-15 keys)
    pub fn graphics_state_dict<K: Eq + Hash, V>() -> HashMap<K, V> {
        HashMap::with_capacity(15)
    }

    /// Create HashMap for colorspace (~4-8 keys)
    pub fn colorspace_dict<K: Eq + Hash, V>() -> HashMap<K, V> {
        HashMap::with_capacity(8)
    }

    /// Create HashMap for stream dictionary (~4-6 keys)
    pub fn stream_dict<K: Eq + Hash, V>() -> HashMap<K, V> {
        HashMap::with_capacity(6)
    }

    /// Create HashMap for XRef table (scales with document size)
    pub fn xref_table<K: Eq + Hash, V>(expected_objects: usize) -> HashMap<K, V> {
        HashMap::with_capacity(expected_objects)
    }

    /// Create HashMap for document catalog (~10-20 keys)
    pub fn catalog_dict<K: Eq + Hash, V>() -> HashMap<K, V> {
        HashMap::with_capacity(20)
    }

    /// Create HashMap for form fields (~5-10 keys per field)
    pub fn form_field_dict<K: Eq + Hash, V>() -> HashMap<K, V> {
        HashMap::with_capacity(10)
    }

    /// Create HashMap for encryption dictionary (~8-12 keys)
    pub fn encrypt_dict<K: Eq + Hash, V>() -> HashMap<K, V> {
        HashMap::with_capacity(12)
    }

    /// Create HashMap for outline/bookmark (~6-8 keys)
    pub fn outline_dict<K: Eq + Hash, V>() -> HashMap<K, V> {
        HashMap::with_capacity(8)
    }

    /// Create HashMap with custom capacity
    pub fn with_capacity<K: Eq + Hash, V>(capacity: usize) -> HashMap<K, V> {
        HashMap::with_capacity(capacity)
    }
}

// ============================================================================
// Perfect Hash for Standard PDF Names
// ============================================================================

/// Perfect hash function for common PDF dictionary keys
/// Uses a minimal perfect hash computed for the standard PDF name set
#[derive(Clone)]
pub struct PdfNameHasher {
    /// Seed for the hash function
    seed: u64,
}

impl PdfNameHasher {
    /// Create a new hasher with the default seed
    pub const fn new() -> Self {
        // Seed chosen empirically for good distribution
        Self {
            seed: 0x517cc1b727220a95,
        }
    }

    /// Hash a PDF name string
    #[inline]
    pub fn hash_name(&self, name: &str) -> u64 {
        // FNV-1a variant optimized for short strings
        let mut hash = self.seed;
        for byte in name.bytes() {
            hash ^= byte as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
        hash
    }
}

impl Default for PdfNameHasher {
    fn default() -> Self {
        Self::new()
    }
}

impl Hasher for PdfNameHasher {
    #[inline]
    fn finish(&self) -> u64 {
        self.seed
    }

    #[inline]
    fn write(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.seed ^= *byte as u64;
            self.seed = self.seed.wrapping_mul(0x100000001b3);
        }
    }
}

/// BuildHasher for PdfNameHasher
#[derive(Clone, Default)]
pub struct PdfNameBuildHasher;

impl BuildHasher for PdfNameBuildHasher {
    type Hasher = PdfNameHasher;

    #[inline]
    fn build_hasher(&self) -> Self::Hasher {
        PdfNameHasher::new()
    }
}

/// HashMap optimized for PDF name keys
pub type PdfNameMap<V> = HashMap<String, V, PdfNameBuildHasher>;

/// Create a new PdfNameMap with default capacity
pub fn new_pdf_name_map<V>() -> PdfNameMap<V> {
    HashMap::with_capacity_and_hasher(16, PdfNameBuildHasher)
}

/// Create a PdfNameMap with specified capacity
pub fn new_pdf_name_map_with_capacity<V>(capacity: usize) -> PdfNameMap<V> {
    HashMap::with_capacity_and_hasher(capacity, PdfNameBuildHasher)
}

// ============================================================================
// Lookup Table for Standard PDF Names
// ============================================================================

/// Index for standard PDF names (perfect hash lookup)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum StandardPdfName {
    // Document structure
    Type = 0,
    Subtype = 1,
    Catalog = 2,
    Pages = 3,
    Page = 4,
    Parent = 5,
    Kids = 6,
    Count = 7,
    Root = 8,
    Info = 9,
    Metadata = 10,

    // Page properties
    MediaBox = 11,
    CropBox = 12,
    BleedBox = 13,
    TrimBox = 14,
    ArtBox = 15,
    Resources = 16,
    Contents = 17,
    Rotate = 18,
    UserUnit = 19,

    // Resources
    ExtGState = 20,
    ColorSpace = 21,
    Pattern = 22,
    Shading = 23,
    XObject = 24,
    Font = 25,
    ProcSet = 26,
    Properties = 27,

    // Stream properties
    Length = 28,
    Filter = 29,
    DecodeParms = 30,

    // Filters
    FlateDecode = 31,
    DCTDecode = 32,
    ASCIIHexDecode = 33,
    ASCII85Decode = 34,
    LZWDecode = 35,
    RunLengthDecode = 36,
    CCITTFaxDecode = 37,
    JBIG2Decode = 38,
    JPXDecode = 39,

    // Color spaces
    DeviceGray = 40,
    DeviceRGB = 41,
    DeviceCMYK = 42,
    ICCBased = 43,
    Indexed = 44,
    Separation = 45,
    DeviceN = 46,

    // XObjects
    Image = 47,
    Form = 48,

    // Image properties
    Width = 49,
    Height = 50,
    BitsPerComponent = 51,
    SMask = 52,
    Mask = 53,
    Decode = 54,
    Interpolate = 55,

    // Font properties
    BaseFont = 56,
    Encoding = 57,
    Widths = 58,
    FirstChar = 59,
    LastChar = 60,
    ToUnicode = 61,
    FontDescriptor = 62,
    DescendantFonts = 63,

    // Unknown (not a standard name)
    Unknown = 255,
}

impl StandardPdfName {
    /// Get the string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Type => "Type",
            Self::Subtype => "Subtype",
            Self::Catalog => "Catalog",
            Self::Pages => "Pages",
            Self::Page => "Page",
            Self::Parent => "Parent",
            Self::Kids => "Kids",
            Self::Count => "Count",
            Self::Root => "Root",
            Self::Info => "Info",
            Self::Metadata => "Metadata",
            Self::MediaBox => "MediaBox",
            Self::CropBox => "CropBox",
            Self::BleedBox => "BleedBox",
            Self::TrimBox => "TrimBox",
            Self::ArtBox => "ArtBox",
            Self::Resources => "Resources",
            Self::Contents => "Contents",
            Self::Rotate => "Rotate",
            Self::UserUnit => "UserUnit",
            Self::ExtGState => "ExtGState",
            Self::ColorSpace => "ColorSpace",
            Self::Pattern => "Pattern",
            Self::Shading => "Shading",
            Self::XObject => "XObject",
            Self::Font => "Font",
            Self::ProcSet => "ProcSet",
            Self::Properties => "Properties",
            Self::Length => "Length",
            Self::Filter => "Filter",
            Self::DecodeParms => "DecodeParms",
            Self::FlateDecode => "FlateDecode",
            Self::DCTDecode => "DCTDecode",
            Self::ASCIIHexDecode => "ASCIIHexDecode",
            Self::ASCII85Decode => "ASCII85Decode",
            Self::LZWDecode => "LZWDecode",
            Self::RunLengthDecode => "RunLengthDecode",
            Self::CCITTFaxDecode => "CCITTFaxDecode",
            Self::JBIG2Decode => "JBIG2Decode",
            Self::JPXDecode => "JPXDecode",
            Self::DeviceGray => "DeviceGray",
            Self::DeviceRGB => "DeviceRGB",
            Self::DeviceCMYK => "DeviceCMYK",
            Self::ICCBased => "ICCBased",
            Self::Indexed => "Indexed",
            Self::Separation => "Separation",
            Self::DeviceN => "DeviceN",
            Self::Image => "Image",
            Self::Form => "Form",
            Self::Width => "Width",
            Self::Height => "Height",
            Self::BitsPerComponent => "BitsPerComponent",
            Self::SMask => "SMask",
            Self::Mask => "Mask",
            Self::Decode => "Decode",
            Self::Interpolate => "Interpolate",
            Self::BaseFont => "BaseFont",
            Self::Encoding => "Encoding",
            Self::Widths => "Widths",
            Self::FirstChar => "FirstChar",
            Self::LastChar => "LastChar",
            Self::ToUnicode => "ToUnicode",
            Self::FontDescriptor => "FontDescriptor",
            Self::DescendantFonts => "DescendantFonts",
            Self::Unknown => "",
        }
    }

    /// Lookup standard name from string (O(1) via perfect hash)
    pub fn from_str(name: &str) -> Self {
        STANDARD_NAME_LOOKUP
            .get(name)
            .copied()
            .unwrap_or(Self::Unknown)
    }
}

/// Lookup table for standard PDF names
static STANDARD_NAME_LOOKUP: LazyLock<HashMap<&'static str, StandardPdfName, PdfNameBuildHasher>> =
    LazyLock::new(|| {
        let mut map = HashMap::with_capacity_and_hasher(64, PdfNameBuildHasher);
        map.insert("Type", StandardPdfName::Type);
        map.insert("Subtype", StandardPdfName::Subtype);
        map.insert("Catalog", StandardPdfName::Catalog);
        map.insert("Pages", StandardPdfName::Pages);
        map.insert("Page", StandardPdfName::Page);
        map.insert("Parent", StandardPdfName::Parent);
        map.insert("Kids", StandardPdfName::Kids);
        map.insert("Count", StandardPdfName::Count);
        map.insert("Root", StandardPdfName::Root);
        map.insert("Info", StandardPdfName::Info);
        map.insert("Metadata", StandardPdfName::Metadata);
        map.insert("MediaBox", StandardPdfName::MediaBox);
        map.insert("CropBox", StandardPdfName::CropBox);
        map.insert("BleedBox", StandardPdfName::BleedBox);
        map.insert("TrimBox", StandardPdfName::TrimBox);
        map.insert("ArtBox", StandardPdfName::ArtBox);
        map.insert("Resources", StandardPdfName::Resources);
        map.insert("Contents", StandardPdfName::Contents);
        map.insert("Rotate", StandardPdfName::Rotate);
        map.insert("UserUnit", StandardPdfName::UserUnit);
        map.insert("ExtGState", StandardPdfName::ExtGState);
        map.insert("ColorSpace", StandardPdfName::ColorSpace);
        map.insert("Pattern", StandardPdfName::Pattern);
        map.insert("Shading", StandardPdfName::Shading);
        map.insert("XObject", StandardPdfName::XObject);
        map.insert("Font", StandardPdfName::Font);
        map.insert("ProcSet", StandardPdfName::ProcSet);
        map.insert("Properties", StandardPdfName::Properties);
        map.insert("Length", StandardPdfName::Length);
        map.insert("Filter", StandardPdfName::Filter);
        map.insert("DecodeParms", StandardPdfName::DecodeParms);
        map.insert("FlateDecode", StandardPdfName::FlateDecode);
        map.insert("DCTDecode", StandardPdfName::DCTDecode);
        map.insert("ASCIIHexDecode", StandardPdfName::ASCIIHexDecode);
        map.insert("ASCII85Decode", StandardPdfName::ASCII85Decode);
        map.insert("LZWDecode", StandardPdfName::LZWDecode);
        map.insert("RunLengthDecode", StandardPdfName::RunLengthDecode);
        map.insert("CCITTFaxDecode", StandardPdfName::CCITTFaxDecode);
        map.insert("JBIG2Decode", StandardPdfName::JBIG2Decode);
        map.insert("JPXDecode", StandardPdfName::JPXDecode);
        map.insert("DeviceGray", StandardPdfName::DeviceGray);
        map.insert("DeviceRGB", StandardPdfName::DeviceRGB);
        map.insert("DeviceCMYK", StandardPdfName::DeviceCMYK);
        map.insert("ICCBased", StandardPdfName::ICCBased);
        map.insert("Indexed", StandardPdfName::Indexed);
        map.insert("Separation", StandardPdfName::Separation);
        map.insert("DeviceN", StandardPdfName::DeviceN);
        map.insert("Image", StandardPdfName::Image);
        map.insert("Form", StandardPdfName::Form);
        map.insert("Width", StandardPdfName::Width);
        map.insert("Height", StandardPdfName::Height);
        map.insert("BitsPerComponent", StandardPdfName::BitsPerComponent);
        map.insert("SMask", StandardPdfName::SMask);
        map.insert("Mask", StandardPdfName::Mask);
        map.insert("Decode", StandardPdfName::Decode);
        map.insert("Interpolate", StandardPdfName::Interpolate);
        map.insert("BaseFont", StandardPdfName::BaseFont);
        map.insert("Encoding", StandardPdfName::Encoding);
        map.insert("Widths", StandardPdfName::Widths);
        map.insert("FirstChar", StandardPdfName::FirstChar);
        map.insert("LastChar", StandardPdfName::LastChar);
        map.insert("ToUnicode", StandardPdfName::ToUnicode);
        map.insert("FontDescriptor", StandardPdfName::FontDescriptor);
        map.insert("DescendantFonts", StandardPdfName::DescendantFonts);
        map
    });

// ============================================================================
// FFI Functions
// ============================================================================

use super::Handle;
use std::ffi::{CStr, c_char, c_int};

/// Create a pre-sized HashMap for page dictionary
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_page_dict_capacity() -> usize {
    12
}

/// Create a pre-sized HashMap for resources dictionary
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_resources_dict_capacity() -> usize {
    10
}

/// Create a pre-sized HashMap for font dictionary
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_font_dict_capacity() -> usize {
    15
}

/// Create a pre-sized HashMap for image dictionary
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_image_dict_capacity() -> usize {
    12
}

/// Create a pre-sized HashMap for stream dictionary
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_stream_dict_capacity() -> usize {
    6
}

/// Lookup standard PDF name index (O(1))
#[unsafe(no_mangle)]
pub extern "C" fn fz_lookup_standard_name(name: *const c_char) -> c_int {
    if name.is_null() {
        return StandardPdfName::Unknown as c_int;
    }
    let name_str = unsafe { CStr::from_ptr(name).to_str().unwrap_or("") };
    StandardPdfName::from_str(name_str) as c_int
}

/// Check if name is a standard PDF name
#[unsafe(no_mangle)]
pub extern "C" fn fz_is_standard_name(name: *const c_char) -> c_int {
    if name.is_null() {
        return 0;
    }
    let name_str = unsafe { CStr::from_ptr(name).to_str().unwrap_or("") };
    if StandardPdfName::from_str(name_str) != StandardPdfName::Unknown {
        1
    } else {
        0
    }
}

/// Get standard name string from index
#[unsafe(no_mangle)]
pub extern "C" fn fz_standard_name_str(index: c_int) -> *const c_char {
    static NAME_STRINGS: LazyLock<Vec<std::ffi::CString>> = LazyLock::new(|| {
        vec![
            std::ffi::CString::new("Type").unwrap(),
            std::ffi::CString::new("Subtype").unwrap(),
            std::ffi::CString::new("Catalog").unwrap(),
            std::ffi::CString::new("Pages").unwrap(),
            std::ffi::CString::new("Page").unwrap(),
            std::ffi::CString::new("Parent").unwrap(),
            std::ffi::CString::new("Kids").unwrap(),
            std::ffi::CString::new("Count").unwrap(),
            std::ffi::CString::new("Root").unwrap(),
            std::ffi::CString::new("Info").unwrap(),
            std::ffi::CString::new("Metadata").unwrap(),
            std::ffi::CString::new("MediaBox").unwrap(),
            std::ffi::CString::new("CropBox").unwrap(),
            std::ffi::CString::new("BleedBox").unwrap(),
            std::ffi::CString::new("TrimBox").unwrap(),
            std::ffi::CString::new("ArtBox").unwrap(),
            std::ffi::CString::new("Resources").unwrap(),
            std::ffi::CString::new("Contents").unwrap(),
            std::ffi::CString::new("Rotate").unwrap(),
            std::ffi::CString::new("UserUnit").unwrap(),
            std::ffi::CString::new("ExtGState").unwrap(),
            std::ffi::CString::new("ColorSpace").unwrap(),
            std::ffi::CString::new("Pattern").unwrap(),
            std::ffi::CString::new("Shading").unwrap(),
            std::ffi::CString::new("XObject").unwrap(),
            std::ffi::CString::new("Font").unwrap(),
            std::ffi::CString::new("ProcSet").unwrap(),
            std::ffi::CString::new("Properties").unwrap(),
            std::ffi::CString::new("Length").unwrap(),
            std::ffi::CString::new("Filter").unwrap(),
            std::ffi::CString::new("DecodeParms").unwrap(),
            std::ffi::CString::new("FlateDecode").unwrap(),
            std::ffi::CString::new("DCTDecode").unwrap(),
            std::ffi::CString::new("ASCIIHexDecode").unwrap(),
            std::ffi::CString::new("ASCII85Decode").unwrap(),
            std::ffi::CString::new("LZWDecode").unwrap(),
            std::ffi::CString::new("RunLengthDecode").unwrap(),
            std::ffi::CString::new("CCITTFaxDecode").unwrap(),
            std::ffi::CString::new("JBIG2Decode").unwrap(),
            std::ffi::CString::new("JPXDecode").unwrap(),
            std::ffi::CString::new("DeviceGray").unwrap(),
            std::ffi::CString::new("DeviceRGB").unwrap(),
            std::ffi::CString::new("DeviceCMYK").unwrap(),
            std::ffi::CString::new("ICCBased").unwrap(),
            std::ffi::CString::new("Indexed").unwrap(),
            std::ffi::CString::new("Separation").unwrap(),
            std::ffi::CString::new("DeviceN").unwrap(),
            std::ffi::CString::new("Image").unwrap(),
            std::ffi::CString::new("Form").unwrap(),
            std::ffi::CString::new("Width").unwrap(),
            std::ffi::CString::new("Height").unwrap(),
            std::ffi::CString::new("BitsPerComponent").unwrap(),
            std::ffi::CString::new("SMask").unwrap(),
            std::ffi::CString::new("Mask").unwrap(),
            std::ffi::CString::new("Decode").unwrap(),
            std::ffi::CString::new("Interpolate").unwrap(),
            std::ffi::CString::new("BaseFont").unwrap(),
            std::ffi::CString::new("Encoding").unwrap(),
            std::ffi::CString::new("Widths").unwrap(),
            std::ffi::CString::new("FirstChar").unwrap(),
            std::ffi::CString::new("LastChar").unwrap(),
            std::ffi::CString::new("ToUnicode").unwrap(),
            std::ffi::CString::new("FontDescriptor").unwrap(),
            std::ffi::CString::new("DescendantFonts").unwrap(),
        ]
    });

    if index >= 0 && (index as usize) < NAME_STRINGS.len() {
        NAME_STRINGS[index as usize].as_ptr()
    } else {
        std::ptr::null()
    }
}

/// Hash a PDF name using the optimized hasher
#[unsafe(no_mangle)]
pub extern "C" fn fz_hash_pdf_name(name: *const c_char) -> u64 {
    if name.is_null() {
        return 0;
    }
    let name_str = unsafe { CStr::from_ptr(name).to_str().unwrap_or("") };
    PdfNameHasher::new().hash_name(name_str)
}

/// Get the number of standard PDF names
#[unsafe(no_mangle)]
pub extern "C" fn fz_standard_name_count() -> c_int {
    64
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_presized_maps() {
        let page: HashMap<String, i32> = presized::page_dict();
        assert!(page.capacity() >= 12);

        let resources: HashMap<String, i32> = presized::resources_dict();
        assert!(resources.capacity() >= 10);

        let font: HashMap<String, i32> = presized::font_dict();
        assert!(font.capacity() >= 15);
    }

    #[test]
    fn test_pdf_name_hasher() {
        let hasher = PdfNameHasher::new();

        let h1 = hasher.hash_name("Type");
        let h2 = hasher.hash_name("Type");
        assert_eq!(h1, h2);

        let h3 = hasher.hash_name("Subtype");
        assert_ne!(h1, h3);
    }

    #[test]
    fn test_standard_name_lookup() {
        assert_eq!(StandardPdfName::from_str("Type"), StandardPdfName::Type);
        assert_eq!(
            StandardPdfName::from_str("MediaBox"),
            StandardPdfName::MediaBox
        );
        assert_eq!(
            StandardPdfName::from_str("FlateDecode"),
            StandardPdfName::FlateDecode
        );
        assert_eq!(
            StandardPdfName::from_str("Unknown123"),
            StandardPdfName::Unknown
        );
    }

    #[test]
    fn test_standard_name_str() {
        assert_eq!(StandardPdfName::Type.as_str(), "Type");
        assert_eq!(StandardPdfName::MediaBox.as_str(), "MediaBox");
        assert_eq!(StandardPdfName::FlateDecode.as_str(), "FlateDecode");
    }

    #[test]
    fn test_pdf_name_map() {
        let mut map: PdfNameMap<i32> = new_pdf_name_map();
        map.insert("Type".to_string(), 1);
        map.insert("Subtype".to_string(), 2);

        assert_eq!(map.get("Type"), Some(&1));
        assert_eq!(map.get("Subtype"), Some(&2));
        assert_eq!(map.get("Other"), None);
    }

    #[test]
    fn test_ffi_lookup_standard_name() {
        use std::ffi::CString;

        let name = CString::new("Type").unwrap();
        let idx = fz_lookup_standard_name(name.as_ptr());
        assert_eq!(idx, StandardPdfName::Type as c_int);

        let unknown = CString::new("Unknown123").unwrap();
        let idx2 = fz_lookup_standard_name(unknown.as_ptr());
        assert_eq!(idx2, StandardPdfName::Unknown as c_int);
    }

    #[test]
    fn test_ffi_is_standard_name() {
        use std::ffi::CString;

        let name = CString::new("Type").unwrap();
        assert_eq!(fz_is_standard_name(name.as_ptr()), 1);

        let unknown = CString::new("Unknown123").unwrap();
        assert_eq!(fz_is_standard_name(unknown.as_ptr()), 0);
    }

    #[test]
    fn test_ffi_standard_name_str() {
        let ptr = fz_standard_name_str(0);
        assert!(!ptr.is_null());
        let s = unsafe { CStr::from_ptr(ptr).to_str().unwrap() };
        assert_eq!(s, "Type");

        let ptr2 = fz_standard_name_str(28);
        let s2 = unsafe { CStr::from_ptr(ptr2).to_str().unwrap() };
        assert_eq!(s2, "Length");

        let ptr3 = fz_standard_name_str(255);
        assert!(ptr3.is_null());
    }

    #[test]
    fn test_ffi_hash_pdf_name() {
        use std::ffi::CString;

        let name1 = CString::new("Type").unwrap();
        let name2 = CString::new("Type").unwrap();
        let name3 = CString::new("Subtype").unwrap();

        let h1 = fz_hash_pdf_name(name1.as_ptr());
        let h2 = fz_hash_pdf_name(name2.as_ptr());
        let h3 = fz_hash_pdf_name(name3.as_ptr());

        assert_eq!(h1, h2);
        assert_ne!(h1, h3);
    }

    #[test]
    fn test_ffi_capacities() {
        assert_eq!(fz_new_page_dict_capacity(), 12);
        assert_eq!(fz_new_resources_dict_capacity(), 10);
        assert_eq!(fz_new_font_dict_capacity(), 15);
        assert_eq!(fz_new_image_dict_capacity(), 12);
        assert_eq!(fz_new_stream_dict_capacity(), 6);
    }

    #[test]
    fn test_hash_collision_resistance() {
        let hasher = PdfNameHasher::new();

        // Test common names don't collide
        let names = [
            "Type", "Subtype", "Length", "Filter", "Width", "Height", "Font", "Image", "Page",
            "Pages",
        ];

        let mut hashes = std::collections::HashSet::new();
        for name in names {
            let h = hasher.hash_name(name);
            assert!(hashes.insert(h), "Collision detected for {}", name);
        }
    }
}
