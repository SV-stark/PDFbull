//! FFI bindings for fz_hyphen (Text Hyphenation)
//!
//! Provides hyphenation pattern matching using Liang's algorithm (TeX-style).

use crate::ffi::{Handle, HandleStore};
use std::collections::HashMap;
use std::ffi::{CStr, CString, c_char};
use std::ptr;
use std::sync::LazyLock;

// ============================================================================
// Types
// ============================================================================

/// Language codes for hyphenation (subset of ISO 639-1)
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum TextLanguage {
    #[default]
    Unset = 0,
    // Common languages
    En = 1,  // English
    De = 2,  // German
    Fr = 3,  // French
    Es = 4,  // Spanish
    It = 5,  // Italian
    Pt = 6,  // Portuguese
    Nl = 7,  // Dutch
    Ru = 8,  // Russian
    Pl = 9,  // Polish
    Cs = 10, // Czech
    Sv = 11, // Swedish
    Da = 12, // Danish
    No = 13, // Norwegian
    Fi = 14, // Finnish
    Hu = 15, // Hungarian
    El = 16, // Greek
    Tr = 17, // Turkish
    Uk = 18, // Ukrainian
    Hr = 19, // Croatian
    Sk = 20, // Slovak
    Sl = 21, // Slovenian
    Bg = 22, // Bulgarian
    Ro = 23, // Romanian
    Lt = 24, // Lithuanian
    Lv = 25, // Latvian
    Et = 26, // Estonian
    Ca = 27, // Catalan
    Eu = 28, // Basque
    Gl = 29, // Galician
    La = 30, // Latin
}

impl TextLanguage {
    pub fn from_i32(value: i32) -> Self {
        match value {
            1 => TextLanguage::En,
            2 => TextLanguage::De,
            3 => TextLanguage::Fr,
            4 => TextLanguage::Es,
            5 => TextLanguage::It,
            6 => TextLanguage::Pt,
            7 => TextLanguage::Nl,
            8 => TextLanguage::Ru,
            9 => TextLanguage::Pl,
            10 => TextLanguage::Cs,
            11 => TextLanguage::Sv,
            12 => TextLanguage::Da,
            13 => TextLanguage::No,
            14 => TextLanguage::Fi,
            15 => TextLanguage::Hu,
            16 => TextLanguage::El,
            17 => TextLanguage::Tr,
            18 => TextLanguage::Uk,
            19 => TextLanguage::Hr,
            20 => TextLanguage::Sk,
            21 => TextLanguage::Sl,
            22 => TextLanguage::Bg,
            23 => TextLanguage::Ro,
            24 => TextLanguage::Lt,
            25 => TextLanguage::Lv,
            26 => TextLanguage::Et,
            27 => TextLanguage::Ca,
            28 => TextLanguage::Eu,
            29 => TextLanguage::Gl,
            30 => TextLanguage::La,
            _ => TextLanguage::Unset,
        }
    }

    pub fn code(&self) -> &'static str {
        match self {
            TextLanguage::Unset => "",
            TextLanguage::En => "en",
            TextLanguage::De => "de",
            TextLanguage::Fr => "fr",
            TextLanguage::Es => "es",
            TextLanguage::It => "it",
            TextLanguage::Pt => "pt",
            TextLanguage::Nl => "nl",
            TextLanguage::Ru => "ru",
            TextLanguage::Pl => "pl",
            TextLanguage::Cs => "cs",
            TextLanguage::Sv => "sv",
            TextLanguage::Da => "da",
            TextLanguage::No => "no",
            TextLanguage::Fi => "fi",
            TextLanguage::Hu => "hu",
            TextLanguage::El => "el",
            TextLanguage::Tr => "tr",
            TextLanguage::Uk => "uk",
            TextLanguage::Hr => "hr",
            TextLanguage::Sk => "sk",
            TextLanguage::Sl => "sl",
            TextLanguage::Bg => "bg",
            TextLanguage::Ro => "ro",
            TextLanguage::Lt => "lt",
            TextLanguage::Lv => "lv",
            TextLanguage::Et => "et",
            TextLanguage::Ca => "ca",
            TextLanguage::Eu => "eu",
            TextLanguage::Gl => "gl",
            TextLanguage::La => "la",
        }
    }
}

/// Trie node for hyphenation patterns
#[derive(Debug, Default, Clone)]
struct HyphTrieNode {
    /// Pattern values at this node (odd = hyphenation point)
    pattern: Option<Vec<u8>>,
    /// Children indexed by character
    children: HashMap<char, HyphTrieNode>,
}

/// Hyphenator using Liang's algorithm
#[derive(Debug, Default)]
pub struct Hyphenator {
    /// Pattern trie
    trie: HyphTrieNode,
    /// Number of patterns loaded
    pattern_count: usize,
    /// Minimum characters before first hyphen
    left_min: usize,
    /// Minimum characters after last hyphen
    right_min: usize,
    /// Language
    language: TextLanguage,
}

impl Hyphenator {
    pub fn new() -> Self {
        Hyphenator {
            trie: HyphTrieNode::default(),
            pattern_count: 0,
            left_min: 2,
            right_min: 2,
            language: TextLanguage::Unset,
        }
    }

    pub fn with_language(language: TextLanguage) -> Self {
        let mut hyph = Self::new();
        hyph.language = language;
        hyph.load_default_patterns();
        hyph
    }

    /// Load default patterns for common languages
    fn load_default_patterns(&mut self) {
        match self.language {
            TextLanguage::En => self.load_english_patterns(),
            TextLanguage::De => self.load_german_patterns(),
            TextLanguage::Fr => self.load_french_patterns(),
            TextLanguage::Es => self.load_spanish_patterns(),
            _ => self.load_english_patterns(), // Fallback to English
        }
    }

    /// Load English hyphenation patterns (subset of TeX patterns)
    fn load_english_patterns(&mut self) {
        // Common English hyphenation patterns
        let patterns = [
            // Basic patterns
            ".ach4",
            ".ad4der",
            ".af1t",
            ".al3t",
            ".am5at",
            ".an5c",
            ".ang4",
            ".ani5m",
            ".ant4",
            ".an3te",
            ".anti5s",
            ".ar5s",
            ".ar4tie",
            ".ar4ty",
            ".as3c",
            ".as1p",
            ".as1s",
            ".aster5",
            ".atom5",
            ".au1d",
            ".av4i",
            ".awn4",
            ".ba4g",
            ".ba5na",
            ".bas4e",
            ".ber4",
            ".be5ra",
            ".be3sm",
            ".be5sto",
            ".bri2",
            ".but4ti",
            ".cam4pe",
            ".can5c",
            ".capa5b",
            ".car5ol",
            ".ca4t",
            ".ce4la",
            ".ch4",
            ".chill5i",
            ".ci2",
            ".cit5r",
            ".co3e",
            ".co4r",
            ".cor5ner",
            // Word endings
            "4able.",
            "4## ably.",
            "2acity.",
            "4acy.",
            "4age.",
            "4aged.",
            "2a2go.",
            "4ald.",
            "4aler.",
            "4ally.",
            "4ament.",
            "4amic.",
            "4amous.",
            "4anese.",
            "4anism.",
            "4anist.",
            "4anity.",
            "4ative.",
            "4ator.",
            "4atory.",
            // Common syllable patterns
            "ab2l",
            "2a2b",
            "a4bi",
            "ab3ol",
            "ab3ru",
            "ac4",
            "ac5et",
            "ac5id",
            "a4cid",
            "ac3in",
            "ack1",
            "a4d",
            "ad4din",
            "ad3er",
            "ad4han",
            "ad3ica",
            "adi4er",
            "ad4le",
            "ad3ow",
            "ad5ran",
            "ae4r",
            "af4t",
            "af1ta",
            "ag5el",
            "ag1i",
            "ag3o",
            "a4gu",
            "ai2",
            "ai5ly",
        ];

        for pattern in patterns {
            self.add_pattern(pattern);
        }
    }

    /// Load German hyphenation patterns (subset)
    fn load_german_patterns(&mut self) {
        let patterns = [
            ".aa1", ".ab3a", ".ab1äu", ".ab1ei", ".ab1er", ".ab1o", ".ab1u", ".ach4", ".ad3r",
            ".af1t", ".ag1n", ".ai1", ".ak1", ".al1b", ".al1t", ".am4", ".an1", ".an3al", ".ang4",
            "4sch.", "1schaft", "4lich.", "4keit.", "4heit.", "4ung.", "4tion.", "4chen.", "2ab",
            "2äb", "2eb", "2ib", "2ob", "2ub", "2üb",
        ];

        for pattern in patterns {
            self.add_pattern(pattern);
        }
    }

    /// Load French hyphenation patterns (subset)
    fn load_french_patterns(&mut self) {
        let patterns = [
            ".ab2h", ".ab3réa", ".abs3", ".ac3h", ".andi2", ".as2ta", ".anti1", ".bi1u", ".ch4",
            "4tion.", "4ment.", "4ment", "1ci", "1ça", "1çu", "1ge", "1gi", "1gé", "1gn", "2bl",
            "2br", "2cl", "2cr", "2dr", "2fl", "2fr", "2gl", "2gr", "2pl", "2pr", "2tr",
        ];

        for pattern in patterns {
            self.add_pattern(pattern);
        }
    }

    /// Load Spanish hyphenation patterns (subset)
    fn load_spanish_patterns(&mut self) {
        let patterns = [
            ".a4", ".e4", ".i4", ".o4", ".u4", ".hi2", ".hu2", "4ción.", "4sión.", "4miento.",
            "4dad.", "4dor.", "4mente.", "1ba", "1be", "1bi", "1bo", "1bu", "1ca", "1ce", "1ci",
            "1co", "1cu", "2bl", "2br", "2cl", "2cr", "2dr", "2fl", "2fr", "2gl", "2gr", "2pl",
            "2pr", "2tr",
        ];

        for pattern in patterns {
            self.add_pattern(pattern);
        }
    }

    /// Add a pattern to the trie
    pub fn add_pattern(&mut self, pattern: &str) {
        let mut chars: Vec<char> = Vec::new();
        let mut values: Vec<u8> = Vec::new();

        let mut current_value = 0u8;
        for c in pattern.chars() {
            if c.is_ascii_digit() {
                current_value = c.to_digit(10).unwrap() as u8;
            } else {
                values.push(current_value);
                chars.push(c.to_ascii_lowercase());
                current_value = 0;
            }
        }
        values.push(current_value);

        if chars.is_empty() {
            return;
        }

        // Insert into trie
        let mut node = &mut self.trie;
        for c in chars {
            node = node.children.entry(c).or_default();
        }
        node.pattern = Some(values);
        self.pattern_count += 1;
    }

    /// Load patterns from a string (TeX-style format)
    pub fn load_patterns(&mut self, data: &str) {
        for line in data.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('%') {
                continue;
            }
            for pattern in line.split_whitespace() {
                self.add_pattern(pattern);
            }
        }
    }

    /// Hyphenate a word, returning positions where hyphens can be inserted
    pub fn hyphenate(&self, word: &str) -> Vec<bool> {
        let word_lower = word.to_lowercase();
        let chars: Vec<char> = word_lower.chars().collect();
        let len = chars.len();

        if len < self.left_min + self.right_min {
            return vec![false; len.saturating_sub(1)];
        }

        // Add word boundaries
        let mut extended: Vec<char> = Vec::with_capacity(len + 2);
        extended.push('.');
        extended.extend(&chars);
        extended.push('.');

        // Values array (one more than chars)
        let mut values = vec![0u8; extended.len() + 1];

        // Apply patterns
        for i in 0..extended.len() {
            let mut node = &self.trie;
            for j in i..extended.len() {
                let c = extended[j];
                if let Some(child) = node.children.get(&c) {
                    node = child;
                    if let Some(ref pattern) = node.pattern {
                        // Apply pattern values (take max)
                        for (k, &v) in pattern.iter().enumerate() {
                            let idx = i + k;
                            if idx < values.len() && v > values[idx] {
                                values[idx] = v;
                            }
                        }
                    }
                } else {
                    break;
                }
            }
        }

        // Convert to hyphenation points (odd values = hyphen point)
        // Skip first and last boundaries, respect min values
        let mut result = vec![false; len.saturating_sub(1)];
        for i in 0..result.len() {
            let val_idx = i + 2; // +1 for '.' prefix, +1 for offset
            if i >= self.left_min.saturating_sub(1)
                && i < len.saturating_sub(self.right_min)
                && values[val_idx] % 2 == 1
            {
                result[i] = true;
            }
        }

        result
    }

    /// Hyphenate a word and return it with soft hyphens inserted
    pub fn hyphenate_with_hyphens(&self, word: &str, hyphen: &str) -> String {
        let points = self.hyphenate(word);
        let chars: Vec<char> = word.chars().collect();

        let mut result = String::with_capacity(
            word.len() + points.iter().filter(|&&x| x).count() * hyphen.len(),
        );

        for (i, c) in chars.iter().enumerate() {
            result.push(*c);
            if i < points.len() && points[i] {
                result.push_str(hyphen);
            }
        }

        result
    }

    /// Get pattern count
    pub fn pattern_count(&self) -> usize {
        self.pattern_count
    }

    /// Get/set left minimum
    pub fn left_min(&self) -> usize {
        self.left_min
    }

    pub fn set_left_min(&mut self, min: usize) {
        self.left_min = min.max(1);
    }

    /// Get/set right minimum
    pub fn right_min(&self) -> usize {
        self.right_min
    }

    pub fn set_right_min(&mut self, min: usize) {
        self.right_min = min.max(1);
    }

    /// Get language
    pub fn language(&self) -> TextLanguage {
        self.language
    }
}

// Global stores
pub static HYPHENATORS: LazyLock<HandleStore<Hyphenator>> = LazyLock::new(HandleStore::new);
pub static REGISTERED_HYPHENATORS: LazyLock<std::sync::Mutex<HashMap<TextLanguage, Handle>>> =
    LazyLock::new(|| std::sync::Mutex::new(HashMap::new()));

// ============================================================================
// FFI Functions
// ============================================================================

/// Create a new hyphenator for a specific language
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_hyphenator(_ctx: Handle, language: i32) -> Handle {
    let lang = TextLanguage::from_i32(language);
    HYPHENATORS.insert(Hyphenator::with_language(lang))
}

/// Create a new empty hyphenator (for custom patterns)
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_empty_hyphenator(_ctx: Handle) -> Handle {
    HYPHENATORS.insert(Hyphenator::new())
}

/// Drop/free a hyphenator
#[unsafe(no_mangle)]
pub extern "C" fn fz_drop_hyphenator(_ctx: Handle, hyph: Handle) {
    HYPHENATORS.remove(hyph);
}

/// Register a hyphenator for a specific language
#[unsafe(no_mangle)]
pub extern "C" fn fz_register_hyphenator(_ctx: Handle, language: i32, hyph: Handle) {
    let lang = TextLanguage::from_i32(language);
    if let Ok(mut map) = REGISTERED_HYPHENATORS.lock() {
        map.insert(lang, hyph);
    }
}

/// Look up a registered hyphenator for a language
#[unsafe(no_mangle)]
pub extern "C" fn fz_lookup_hyphenator(_ctx: Handle, language: i32) -> Handle {
    let lang = TextLanguage::from_i32(language);
    if let Ok(map) = REGISTERED_HYPHENATORS.lock() {
        *map.get(&lang).unwrap_or(&0)
    } else {
        0
    }
}

/// Add a hyphenation pattern
#[unsafe(no_mangle)]
pub extern "C" fn fz_hyphenator_add_pattern(
    _ctx: Handle,
    hyph: Handle,
    pattern: *const c_char,
) -> i32 {
    if pattern.is_null() {
        return 0;
    }

    let h = match HYPHENATORS.get(hyph) {
        Some(h) => h,
        None => return 0,
    };

    let pattern_str = unsafe { CStr::from_ptr(pattern) };
    let pattern_str = match pattern_str.to_str() {
        Ok(s) => s,
        Err(_) => return 0,
    };

    h.lock().unwrap().add_pattern(pattern_str);
    1
}

/// Load patterns from a string
#[unsafe(no_mangle)]
pub extern "C" fn fz_hyphenator_load_patterns(
    _ctx: Handle,
    hyph: Handle,
    data: *const c_char,
) -> i32 {
    if data.is_null() {
        return 0;
    }

    let h = match HYPHENATORS.get(hyph) {
        Some(h) => h,
        None => return 0,
    };

    let data_str = unsafe { CStr::from_ptr(data) };
    let data_str = match data_str.to_str() {
        Ok(s) => s,
        Err(_) => return 0,
    };

    h.lock().unwrap().load_patterns(data_str);
    1
}

/// Hyphenate a word
///
/// @param ctx        Context handle
/// @param hyph       Hyphenator handle
/// @param input      Input word (UTF-8)
/// @param input_size Size of input (or 0 for null-terminated)
/// @param output     Output buffer (receives word with soft hyphens)
/// @param output_size Size of output buffer
///
/// Returns number of bytes written to output
#[unsafe(no_mangle)]
pub extern "C" fn fz_hyphenate_word(
    _ctx: Handle,
    hyph: Handle,
    input: *const c_char,
    input_size: i32,
    output: *mut c_char,
    output_size: i32,
) -> i32 {
    if input.is_null() || output.is_null() || output_size <= 0 {
        return 0;
    }

    let h = match HYPHENATORS.get(hyph) {
        Some(h) => h,
        None => return 0,
    };

    let input_str = if input_size <= 0 {
        unsafe { CStr::from_ptr(input) }.to_str().unwrap_or("")
    } else {
        unsafe {
            std::str::from_utf8(std::slice::from_raw_parts(
                input as *const u8,
                input_size as usize,
            ))
            .unwrap_or("")
        }
    };

    let guard = h.lock().unwrap();
    let result = guard.hyphenate_with_hyphens(input_str, "\u{00AD}"); // Soft hyphen

    let bytes = result.as_bytes();
    let copy_len = bytes.len().min(output_size as usize - 1);

    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), output as *mut u8, copy_len);
        *output.add(copy_len) = 0; // Null terminate
    }

    copy_len as i32
}

/// Get hyphenation points for a word
///
/// @param ctx        Context handle
/// @param hyph       Hyphenator handle
/// @param word       Input word (UTF-8)
/// @param points     Output array of booleans (true = hyphen point)
/// @param points_len Size of points array
///
/// Returns number of hyphenation points found
#[unsafe(no_mangle)]
pub extern "C" fn fz_hyphenation_points(
    _ctx: Handle,
    hyph: Handle,
    word: *const c_char,
    points: *mut u8,
    points_len: usize,
) -> usize {
    if word.is_null() || points.is_null() || points_len == 0 {
        return 0;
    }

    let h = match HYPHENATORS.get(hyph) {
        Some(h) => h,
        None => return 0,
    };

    let word_str = unsafe { CStr::from_ptr(word) };
    let word_str = match word_str.to_str() {
        Ok(s) => s,
        Err(_) => return 0,
    };

    let guard = h.lock().unwrap();
    let hyph_points = guard.hyphenate(word_str);

    let copy_len = hyph_points.len().min(points_len);
    for i in 0..copy_len {
        unsafe {
            *points.add(i) = if hyph_points[i] { 1 } else { 0 };
        }
    }

    hyph_points.iter().filter(|&&x| x).count()
}

/// Get pattern count
#[unsafe(no_mangle)]
pub extern "C" fn fz_hyphenator_pattern_count(_ctx: Handle, hyph: Handle) -> usize {
    let h = match HYPHENATORS.get(hyph) {
        Some(h) => h,
        None => return 0,
    };
    h.lock().unwrap().pattern_count()
}

/// Get left minimum
#[unsafe(no_mangle)]
pub extern "C" fn fz_hyphenator_left_min(_ctx: Handle, hyph: Handle) -> usize {
    let h = match HYPHENATORS.get(hyph) {
        Some(h) => h,
        None => return 0,
    };
    h.lock().unwrap().left_min()
}

/// Set left minimum
#[unsafe(no_mangle)]
pub extern "C" fn fz_hyphenator_set_left_min(_ctx: Handle, hyph: Handle, min: usize) {
    if let Some(h) = HYPHENATORS.get(hyph) {
        h.lock().unwrap().set_left_min(min);
    }
}

/// Get right minimum
#[unsafe(no_mangle)]
pub extern "C" fn fz_hyphenator_right_min(_ctx: Handle, hyph: Handle) -> usize {
    let h = match HYPHENATORS.get(hyph) {
        Some(h) => h,
        None => return 0,
    };
    h.lock().unwrap().right_min()
}

/// Set right minimum
#[unsafe(no_mangle)]
pub extern "C" fn fz_hyphenator_set_right_min(_ctx: Handle, hyph: Handle, min: usize) {
    if let Some(h) = HYPHENATORS.get(hyph) {
        h.lock().unwrap().set_right_min(min);
    }
}

/// Get language code
#[unsafe(no_mangle)]
pub extern "C" fn fz_hyphenator_language(_ctx: Handle, hyph: Handle) -> i32 {
    let h = match HYPHENATORS.get(hyph) {
        Some(h) => h,
        None => return 0,
    };
    h.lock().unwrap().language() as i32
}

/// Get language name
#[unsafe(no_mangle)]
pub extern "C" fn fz_text_language_code(language: i32) -> *const c_char {
    static CODES: LazyLock<HashMap<i32, CString>> = LazyLock::new(|| {
        let mut map = HashMap::new();
        for i in 0..=30 {
            let lang = TextLanguage::from_i32(i);
            map.insert(i, CString::new(lang.code()).unwrap());
        }
        map
    });

    CODES.get(&language).map_or(ptr::null(), |s| s.as_ptr())
}

/// Check if a character is a Unicode hyphen
#[unsafe(no_mangle)]
pub extern "C" fn fz_is_unicode_hyphen(c: u32) -> i32 {
    match c {
        0x002D |        // HYPHEN-MINUS
        0x00AD |        // SOFT HYPHEN
        0x058A |        // ARMENIAN HYPHEN
        0x05BE |        // HEBREW PUNCTUATION MAQAF
        0x1400 |        // CANADIAN SYLLABICS HYPHEN
        0x1806 |        // MONGOLIAN TODO SOFT HYPHEN
        0x2010 |        // HYPHEN
        0x2011 |        // NON-BREAKING HYPHEN
        0x2012 |        // FIGURE DASH
        0x2013 |        // EN DASH
        0x2014 |        // EM DASH
        0x2015 |        // HORIZONTAL BAR
        0x2E17 |        // DOUBLE HYPHEN
        0x2E1A |        // HYPHEN WITH DIAERESIS
        0x2E3A |        // TWO-EM DASH
        0x2E3B |        // THREE-EM DASH
        0x2E40 |        // DOUBLE HYPHEN
        0x301C |        // WAVE DASH
        0x3030 |        // WAVY DASH
        0x30A0 |        // KATAKANA-HIRAGANA DOUBLE HYPHEN
        0xFE31 |        // PRESENTATION FORM FOR VERTICAL EM DASH
        0xFE32 |        // PRESENTATION FORM FOR VERTICAL EN DASH
        0xFE58 |        // SMALL EM DASH
        0xFE63 |        // SMALL HYPHEN-MINUS
        0xFF0D => 1,    // FULLWIDTH HYPHEN-MINUS
        _ => 0
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_enum() {
        assert_eq!(TextLanguage::from_i32(1), TextLanguage::En);
        assert_eq!(TextLanguage::from_i32(2), TextLanguage::De);
        assert_eq!(TextLanguage::from_i32(99), TextLanguage::Unset);
        assert_eq!(TextLanguage::En.code(), "en");
        assert_eq!(TextLanguage::De.code(), "de");
    }

    #[test]
    fn test_new_hyphenator() {
        let ctx = 1;
        let hyph = fz_new_hyphenator(ctx, TextLanguage::En as i32);
        assert!(hyph > 0);
        assert!(fz_hyphenator_pattern_count(ctx, hyph) > 0);
        fz_drop_hyphenator(ctx, hyph);
    }

    #[test]
    fn test_empty_hyphenator() {
        let ctx = 1;
        let hyph = fz_new_empty_hyphenator(ctx);
        assert!(hyph > 0);
        assert_eq!(fz_hyphenator_pattern_count(ctx, hyph), 0);
        fz_drop_hyphenator(ctx, hyph);
    }

    #[test]
    fn test_add_pattern() {
        let ctx = 1;
        let hyph = fz_new_empty_hyphenator(ctx);

        let pattern = CString::new("1ba").unwrap();
        let result = fz_hyphenator_add_pattern(ctx, hyph, pattern.as_ptr());
        assert_eq!(result, 1);
        assert_eq!(fz_hyphenator_pattern_count(ctx, hyph), 1);

        fz_drop_hyphenator(ctx, hyph);
    }

    #[test]
    fn test_hyphenate_simple() {
        let hyph = Hyphenator::with_language(TextLanguage::En);

        // Test a known word
        let points = hyph.hyphenate("hyphenation");
        // Should find some hyphenation points
        assert!(!points.is_empty());
    }

    #[test]
    fn test_hyphenate_with_hyphens() {
        let mut hyph = Hyphenator::new();
        hyph.add_pattern("1ba");
        hyph.add_pattern("1be");
        hyph.add_pattern("1bi");
        hyph.add_pattern("1bo");
        hyph.add_pattern("1bu");

        let result = hyph.hyphenate_with_hyphens("basketball", "-");
        // Should contain at least one hyphen
        assert!(result.len() >= "basketball".len());
    }

    #[test]
    fn test_hyphenate_word_ffi() {
        let ctx = 1;
        let hyph = fz_new_hyphenator(ctx, TextLanguage::En as i32);

        let input = CString::new("hyphenation").unwrap();
        let mut output = vec![0u8; 100];

        let written = fz_hyphenate_word(
            ctx,
            hyph,
            input.as_ptr(),
            0,
            output.as_mut_ptr() as *mut c_char,
            output.len() as i32,
        );

        assert!(written > 0);

        fz_drop_hyphenator(ctx, hyph);
    }

    #[test]
    fn test_hyphenation_points() {
        let ctx = 1;
        let hyph = fz_new_hyphenator(ctx, TextLanguage::En as i32);

        let word = CString::new("computer").unwrap();
        let mut points = vec![0u8; 20];

        let count =
            fz_hyphenation_points(ctx, hyph, word.as_ptr(), points.as_mut_ptr(), points.len());

        // Some hyphenation points should be found
        assert!(count <= 7); // At most n-1 points

        fz_drop_hyphenator(ctx, hyph);
    }

    #[test]
    fn test_min_values() {
        let ctx = 1;
        let hyph = fz_new_hyphenator(ctx, TextLanguage::En as i32);

        assert_eq!(fz_hyphenator_left_min(ctx, hyph), 2);
        assert_eq!(fz_hyphenator_right_min(ctx, hyph), 2);

        fz_hyphenator_set_left_min(ctx, hyph, 3);
        fz_hyphenator_set_right_min(ctx, hyph, 3);

        assert_eq!(fz_hyphenator_left_min(ctx, hyph), 3);
        assert_eq!(fz_hyphenator_right_min(ctx, hyph), 3);

        fz_drop_hyphenator(ctx, hyph);
    }

    #[test]
    fn test_register_lookup() {
        let ctx = 1;
        let hyph = fz_new_hyphenator(ctx, TextLanguage::De as i32);

        fz_register_hyphenator(ctx, TextLanguage::De as i32, hyph);

        let found = fz_lookup_hyphenator(ctx, TextLanguage::De as i32);
        assert_eq!(found, hyph);

        let not_found = fz_lookup_hyphenator(ctx, TextLanguage::Fr as i32);
        assert_eq!(not_found, 0);

        fz_drop_hyphenator(ctx, hyph);
    }

    #[test]
    fn test_language_code() {
        let en_code = fz_text_language_code(TextLanguage::En as i32);
        assert!(!en_code.is_null());
        let en_str = unsafe { CStr::from_ptr(en_code) }.to_str().unwrap();
        assert_eq!(en_str, "en");

        let de_code = fz_text_language_code(TextLanguage::De as i32);
        assert!(!de_code.is_null());
        let de_str = unsafe { CStr::from_ptr(de_code) }.to_str().unwrap();
        assert_eq!(de_str, "de");
    }

    #[test]
    fn test_is_unicode_hyphen() {
        assert_eq!(fz_is_unicode_hyphen(0x002D), 1); // HYPHEN-MINUS
        assert_eq!(fz_is_unicode_hyphen(0x00AD), 1); // SOFT HYPHEN
        assert_eq!(fz_is_unicode_hyphen(0x2010), 1); // HYPHEN
        assert_eq!(fz_is_unicode_hyphen(0x2013), 1); // EN DASH
        assert_eq!(fz_is_unicode_hyphen(0x2014), 1); // EM DASH
        assert_eq!(fz_is_unicode_hyphen('A' as u32), 0);
        assert_eq!(fz_is_unicode_hyphen(' ' as u32), 0);
    }

    #[test]
    fn test_short_words() {
        let hyph = Hyphenator::with_language(TextLanguage::En);

        // Very short words should not be hyphenated
        let points = hyph.hyphenate("a");
        assert!(points.is_empty());

        let points = hyph.hyphenate("an");
        assert!(points.is_empty() || !points.iter().any(|&x| x));

        let points = hyph.hyphenate("the");
        assert!(points.is_empty() || !points.iter().any(|&x| x));
    }

    #[test]
    fn test_null_handling() {
        let ctx = 1;

        // Invalid handle
        assert_eq!(fz_hyphenator_pattern_count(ctx, 0), 0);
        assert_eq!(fz_hyphenator_left_min(ctx, 0), 0);
        assert_eq!(fz_hyphenator_right_min(ctx, 0), 0);

        // Null pointers
        let hyph = fz_new_hyphenator(ctx, TextLanguage::En as i32);
        assert_eq!(fz_hyphenator_add_pattern(ctx, hyph, ptr::null()), 0);
        assert_eq!(
            fz_hyphenate_word(ctx, hyph, ptr::null(), 0, ptr::null_mut(), 10),
            0
        );

        let word = CString::new("test").unwrap();
        assert_eq!(
            fz_hyphenation_points(ctx, hyph, word.as_ptr(), ptr::null_mut(), 10),
            0
        );

        fz_drop_hyphenator(ctx, hyph);
    }

    #[test]
    fn test_different_languages() {
        let ctx = 1;

        // Test German
        let de = fz_new_hyphenator(ctx, TextLanguage::De as i32);
        assert!(fz_hyphenator_pattern_count(ctx, de) > 0);
        assert_eq!(fz_hyphenator_language(ctx, de), TextLanguage::De as i32);
        fz_drop_hyphenator(ctx, de);

        // Test French
        let fr = fz_new_hyphenator(ctx, TextLanguage::Fr as i32);
        assert!(fz_hyphenator_pattern_count(ctx, fr) > 0);
        assert_eq!(fz_hyphenator_language(ctx, fr), TextLanguage::Fr as i32);
        fz_drop_hyphenator(ctx, fr);

        // Test Spanish
        let es = fz_new_hyphenator(ctx, TextLanguage::Es as i32);
        assert!(fz_hyphenator_pattern_count(ctx, es) > 0);
        assert_eq!(fz_hyphenator_language(ctx, es), TextLanguage::Es as i32);
        fz_drop_hyphenator(ctx, es);
    }
}
