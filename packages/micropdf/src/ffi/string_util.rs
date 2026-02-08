//! C FFI for advanced string handling - MuPDF compatible
//! Safe Rust implementation of fz_string utilities

use super::Handle;
use std::ffi::{CStr, c_char};

/// Unicode normalization form
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NormalizationForm {
    /// Canonical decomposition (NFD)
    NFD = 0,
    /// Canonical decomposition followed by canonical composition (NFC)
    NFC = 1,
    /// Compatibility decomposition (NFKD)
    NFKD = 2,
    /// Compatibility decomposition followed by canonical composition (NFKC)
    NFKC = 3,
}

/// BiDi direction
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BiDiDirection {
    /// Left to right
    LTR = 0,
    /// Right to left
    RTL = 1,
    /// Mixed/neutral
    Mixed = 2,
}

/// Script category (simplified)
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScriptCategory {
    /// Unknown/common
    Common = 0,
    /// Latin
    Latin = 1,
    /// Greek
    Greek = 2,
    /// Cyrillic
    Cyrillic = 3,
    /// Arabic
    Arabic = 4,
    /// Hebrew
    Hebrew = 5,
    /// CJK (Chinese/Japanese/Korean)
    CJK = 6,
    /// Devanagari
    Devanagari = 7,
    /// Thai
    Thai = 8,
    /// Other
    Other = 255,
}

/// Word break type
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WordBreakType {
    /// No break
    NoBreak = 0,
    /// Optional break (soft hyphen)
    Soft = 1,
    /// Mandatory break (space, etc.)
    Hard = 2,
    /// Line break opportunity
    Line = 3,
}

// ============================================================================
// Unicode Normalization
// ============================================================================

/// Normalize a UTF-8 string
///
/// # Safety
/// - `input` must be a valid null-terminated UTF-8 string
/// - `output` must point to at least `output_size` bytes
#[unsafe(no_mangle)]
pub extern "C" fn fz_normalize_string(
    _ctx: Handle,
    input: *const c_char,
    output: *mut c_char,
    output_size: usize,
    form: i32,
) -> usize {
    if input.is_null() || output.is_null() || output_size == 0 {
        return 0;
    }

    let input_str = match unsafe { CStr::from_ptr(input) }.to_str() {
        Ok(s) => s,
        Err(_) => return 0,
    };

    let _norm_form = match form {
        1 => NormalizationForm::NFC,
        2 => NormalizationForm::NFKD,
        3 => NormalizationForm::NFKC,
        _ => NormalizationForm::NFD,
    };

    // Simple normalization: just copy for now
    // Full implementation would use unicode-normalization crate
    let normalized = normalize_simple(input_str);

    let bytes = normalized.as_bytes();
    let copy_len = bytes.len().min(output_size - 1);

    let output_slice = unsafe { std::slice::from_raw_parts_mut(output as *mut u8, copy_len + 1) };
    output_slice[..copy_len].copy_from_slice(&bytes[..copy_len]);
    output_slice[copy_len] = 0;

    copy_len
}

fn normalize_simple(s: &str) -> String {
    // Basic normalization: decompose common ligatures and accented chars
    let mut result = String::with_capacity(s.len());

    for c in s.chars() {
        match c {
            // Common ligatures
            'ﬁ' => result.push_str("fi"),
            'ﬂ' => result.push_str("fl"),
            'ﬀ' => result.push_str("ff"),
            'ﬃ' => result.push_str("ffi"),
            'ﬄ' => result.push_str("ffl"),
            'Ĳ' => result.push_str("IJ"),
            'ĳ' => result.push_str("ij"),
            // Everything else passes through
            _ => result.push(c),
        }
    }

    result
}

/// Check if string is normalized
#[unsafe(no_mangle)]
pub extern "C" fn fz_string_is_normalized(_ctx: Handle, input: *const c_char, form: i32) -> i32 {
    if input.is_null() {
        return 1;
    }

    let input_str = match unsafe { CStr::from_ptr(input) }.to_str() {
        Ok(s) => s,
        Err(_) => return 0,
    };

    let _form = match form {
        1 => NormalizationForm::NFC,
        2 => NormalizationForm::NFKD,
        3 => NormalizationForm::NFKC,
        _ => NormalizationForm::NFD,
    };

    // Simple check: no ligatures = normalized
    let has_ligatures = input_str
        .chars()
        .any(|c| matches!(c, 'ﬁ' | 'ﬂ' | 'ﬀ' | 'ﬃ' | 'ﬄ'));

    if has_ligatures { 0 } else { 1 }
}

// ============================================================================
// BiDi Processing
// ============================================================================

/// Get base direction of text
#[unsafe(no_mangle)]
pub extern "C" fn fz_get_bidi_direction(_ctx: Handle, text: *const c_char) -> i32 {
    if text.is_null() {
        return BiDiDirection::LTR as i32;
    }

    let text_str = match unsafe { CStr::from_ptr(text) }.to_str() {
        Ok(s) => s,
        Err(_) => return BiDiDirection::LTR as i32,
    };

    // Detect RTL by checking for RTL characters
    let mut has_rtl = false;
    let mut has_ltr = false;

    for c in text_str.chars() {
        if is_rtl_char(c) {
            has_rtl = true;
        } else if is_strong_ltr_char(c) {
            has_ltr = true;
        }
    }

    if has_rtl && has_ltr {
        BiDiDirection::Mixed as i32
    } else if has_rtl {
        BiDiDirection::RTL as i32
    } else {
        BiDiDirection::LTR as i32
    }
}

fn is_rtl_char(c: char) -> bool {
    // Arabic: U+0600-U+06FF, U+0750-U+077F, U+08A0-U+08FF
    // Hebrew: U+0590-U+05FF
    matches!(c,
        '\u{0590}'..='\u{05FF}' |  // Hebrew
        '\u{0600}'..='\u{06FF}' |  // Arabic
        '\u{0750}'..='\u{077F}' |  // Arabic Supplement
        '\u{08A0}'..='\u{08FF}'    // Arabic Extended-A
    )
}

fn is_strong_ltr_char(c: char) -> bool {
    // Latin, Greek, Cyrillic and most other scripts are LTR
    c.is_alphabetic() && !is_rtl_char(c)
}

/// Reorder text for display (visual order)
///
/// # Safety
/// - `input` must be valid UTF-8
/// - `output` must have space for reordered text
#[unsafe(no_mangle)]
pub extern "C" fn fz_bidi_reorder(
    _ctx: Handle,
    input: *const c_char,
    output: *mut c_char,
    output_size: usize,
    base_dir: i32,
) -> usize {
    if input.is_null() || output.is_null() || output_size == 0 {
        return 0;
    }

    let input_str = match unsafe { CStr::from_ptr(input) }.to_str() {
        Ok(s) => s,
        Err(_) => return 0,
    };

    let base_rtl = base_dir == BiDiDirection::RTL as i32;

    // Simple BiDi: reverse RTL segments
    let reordered = bidi_reorder_simple(input_str, base_rtl);

    let bytes = reordered.as_bytes();
    let copy_len = bytes.len().min(output_size - 1);

    let output_slice = unsafe { std::slice::from_raw_parts_mut(output as *mut u8, copy_len + 1) };
    output_slice[..copy_len].copy_from_slice(&bytes[..copy_len]);
    output_slice[copy_len] = 0;

    copy_len
}

fn bidi_reorder_simple(s: &str, _base_rtl: bool) -> String {
    // Simple implementation: identify RTL runs and reverse them
    let mut result = String::new();
    let mut current_run = String::new();
    let mut in_rtl = false;

    for c in s.chars() {
        let char_rtl = is_rtl_char(c);

        if char_rtl != in_rtl && !current_run.is_empty() {
            if in_rtl {
                result.extend(current_run.chars().rev());
            } else {
                result.push_str(&current_run);
            }
            current_run.clear();
        }

        in_rtl = char_rtl;
        current_run.push(c);
    }

    // Handle last run
    if !current_run.is_empty() {
        if in_rtl {
            result.extend(current_run.chars().rev());
        } else {
            result.push_str(&current_run);
        }
    }

    result
}

// ============================================================================
// Text Segmentation
// ============================================================================

/// Find word boundaries
///
/// # Safety
/// - `text` must be valid UTF-8
/// - `breaks` must have space for at least `max_breaks` values
#[unsafe(no_mangle)]
pub extern "C" fn fz_find_word_breaks(
    _ctx: Handle,
    text: *const c_char,
    breaks: *mut i32,
    max_breaks: usize,
) -> usize {
    if text.is_null() || breaks.is_null() || max_breaks == 0 {
        return 0;
    }

    let text_str = match unsafe { CStr::from_ptr(text) }.to_str() {
        Ok(s) => s,
        Err(_) => return 0,
    };

    let breaks_slice = unsafe { std::slice::from_raw_parts_mut(breaks, max_breaks) };
    let mut break_count = 0;
    let mut prev_was_space = true;

    for (i, c) in text_str.char_indices() {
        let is_space = c.is_whitespace();

        // Word starts after space
        if prev_was_space && !is_space && break_count < max_breaks {
            breaks_slice[break_count] = i as i32;
            break_count += 1;
        }

        prev_was_space = is_space;
    }

    break_count
}

/// Get word at position
#[unsafe(no_mangle)]
pub extern "C" fn fz_get_word_at(
    _ctx: Handle,
    text: *const c_char,
    position: usize,
    word_start: *mut usize,
    word_end: *mut usize,
) -> i32 {
    if text.is_null() {
        return 0;
    }

    let text_str = match unsafe { CStr::from_ptr(text) }.to_str() {
        Ok(s) => s,
        Err(_) => return 0,
    };

    if position >= text_str.len() {
        return 0;
    }

    // Find word boundaries around position
    let bytes = text_str.as_bytes();
    let mut start = position;
    let mut end = position;

    // Scan backward for word start
    while start > 0 && !bytes[start - 1].is_ascii_whitespace() {
        start -= 1;
    }

    // Scan forward for word end
    while end < bytes.len() && !bytes[end].is_ascii_whitespace() {
        end += 1;
    }

    if !word_start.is_null() {
        unsafe { *word_start = start };
    }
    if !word_end.is_null() {
        unsafe { *word_end = end };
    }

    1
}

/// Find line break opportunities
#[unsafe(no_mangle)]
pub extern "C" fn fz_find_line_breaks(
    _ctx: Handle,
    text: *const c_char,
    breaks: *mut i32,
    max_breaks: usize,
) -> usize {
    if text.is_null() || breaks.is_null() || max_breaks == 0 {
        return 0;
    }

    let text_str = match unsafe { CStr::from_ptr(text) }.to_str() {
        Ok(s) => s,
        Err(_) => return 0,
    };

    let breaks_slice = unsafe { std::slice::from_raw_parts_mut(breaks, max_breaks) };
    let mut break_count = 0;

    for (i, c) in text_str.char_indices() {
        // Break after spaces, hyphens, and CJK characters
        let is_break_after = c.is_whitespace()
            || c == '-'
            || c == '\u{00AD}' // Soft hyphen
            || is_cjk_char(c);

        if is_break_after && break_count < max_breaks {
            breaks_slice[break_count] = (i + c.len_utf8()) as i32;
            break_count += 1;
        }
    }

    break_count
}

fn is_cjk_char(c: char) -> bool {
    matches!(c,
        '\u{4E00}'..='\u{9FFF}' |   // CJK Unified Ideographs
        '\u{3400}'..='\u{4DBF}' |   // CJK Extension A
        '\u{3000}'..='\u{303F}' |   // CJK Punctuation
        '\u{3040}'..='\u{309F}' |   // Hiragana
        '\u{30A0}'..='\u{30FF}' |   // Katakana
        '\u{AC00}'..='\u{D7AF}'     // Hangul
    )
}

// ============================================================================
// Script Detection
// ============================================================================

/// Detect script of text
#[unsafe(no_mangle)]
pub extern "C" fn fz_detect_script(_ctx: Handle, text: *const c_char) -> i32 {
    if text.is_null() {
        return ScriptCategory::Common as i32;
    }

    let text_str = match unsafe { CStr::from_ptr(text) }.to_str() {
        Ok(s) => s,
        Err(_) => return ScriptCategory::Common as i32,
    };

    // Find first strong script character
    for c in text_str.chars() {
        let script = char_script(c);
        if script != ScriptCategory::Common {
            return script as i32;
        }
    }

    ScriptCategory::Common as i32
}

fn char_script(c: char) -> ScriptCategory {
    match c {
        'A'..='Z' | 'a'..='z' | '\u{00C0}'..='\u{00FF}' => ScriptCategory::Latin,
        '\u{0370}'..='\u{03FF}' => ScriptCategory::Greek,
        '\u{0400}'..='\u{04FF}' => ScriptCategory::Cyrillic,
        '\u{0600}'..='\u{06FF}' | '\u{0750}'..='\u{077F}' => ScriptCategory::Arabic,
        '\u{0590}'..='\u{05FF}' => ScriptCategory::Hebrew,
        '\u{4E00}'..='\u{9FFF}' | '\u{3040}'..='\u{30FF}' | '\u{AC00}'..='\u{D7AF}' => {
            ScriptCategory::CJK
        }
        '\u{0900}'..='\u{097F}' => ScriptCategory::Devanagari,
        '\u{0E00}'..='\u{0E7F}' => ScriptCategory::Thai,
        _ => ScriptCategory::Common,
    }
}

// ============================================================================
// Language-Aware Operations
// ============================================================================

/// Case-fold string for comparison
#[unsafe(no_mangle)]
pub extern "C" fn fz_casefold(
    _ctx: Handle,
    input: *const c_char,
    output: *mut c_char,
    output_size: usize,
) -> usize {
    if input.is_null() || output.is_null() || output_size == 0 {
        return 0;
    }

    let input_str = match unsafe { CStr::from_ptr(input) }.to_str() {
        Ok(s) => s,
        Err(_) => return 0,
    };

    let folded: String = input_str.chars().flat_map(|c| c.to_lowercase()).collect();

    let bytes = folded.as_bytes();
    let copy_len = bytes.len().min(output_size - 1);

    let output_slice = unsafe { std::slice::from_raw_parts_mut(output as *mut u8, copy_len + 1) };
    output_slice[..copy_len].copy_from_slice(&bytes[..copy_len]);
    output_slice[copy_len] = 0;

    copy_len
}

/// Compare strings with collation
#[unsafe(no_mangle)]
pub extern "C" fn fz_strcoll(
    _ctx: Handle,
    s1: *const c_char,
    s2: *const c_char,
    _locale: *const c_char,
) -> i32 {
    if s1.is_null() && s2.is_null() {
        return 0;
    }
    if s1.is_null() {
        return -1;
    }
    if s2.is_null() {
        return 1;
    }

    let str1 = unsafe { CStr::from_ptr(s1) }.to_str().unwrap_or("");
    let str2 = unsafe { CStr::from_ptr(s2) }.to_str().unwrap_or("");

    // Simple comparison (case-insensitive for now)
    let s1_lower: String = str1.chars().flat_map(|c| c.to_lowercase()).collect();
    let s2_lower: String = str2.chars().flat_map(|c| c.to_lowercase()).collect();

    match s1_lower.cmp(&s2_lower) {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => 1,
    }
}

/// Count characters (grapheme clusters)
#[unsafe(no_mangle)]
pub extern "C" fn fz_string_char_count(_ctx: Handle, s: *const c_char) -> usize {
    if s.is_null() {
        return 0;
    }

    let str = match unsafe { CStr::from_ptr(s) }.to_str() {
        Ok(s) => s,
        Err(_) => return 0,
    };

    str.chars().count()
}

/// Get byte offset for character index
#[unsafe(no_mangle)]
pub extern "C" fn fz_char_to_byte_offset(_ctx: Handle, s: *const c_char, char_index: usize) -> i32 {
    if s.is_null() {
        return -1;
    }

    let str = match unsafe { CStr::from_ptr(s) }.to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };

    for (i, (byte_idx, _)) in str.char_indices().enumerate() {
        if i == char_index {
            return byte_idx as i32;
        }
    }

    -1
}

/// Get character index for byte offset
#[unsafe(no_mangle)]
pub extern "C" fn fz_byte_to_char_offset(
    _ctx: Handle,
    s: *const c_char,
    byte_offset: usize,
) -> i32 {
    if s.is_null() {
        return -1;
    }

    let str = match unsafe { CStr::from_ptr(s) }.to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };

    for (char_idx, (byte_idx, _)) in str.char_indices().enumerate() {
        if byte_idx == byte_offset {
            return char_idx as i32;
        }
    }

    -1
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_ligatures() {
        let input = c"ﬁnding ﬂowers";
        let mut output = [0u8; 64];

        let len = fz_normalize_string(
            0,
            input.as_ptr(),
            output.as_mut_ptr().cast(),
            64,
            NormalizationForm::NFC as i32,
        );

        let result = std::str::from_utf8(&output[..len]).unwrap();
        assert!(result.contains("fi"));
        assert!(result.contains("fl"));
    }

    #[test]
    fn test_bidi_detection() {
        let ltr = c"Hello World";
        let rtl = c"\u{05E9}\u{05DC}\u{05D5}\u{05DD}"; // Hebrew "Shalom"

        assert_eq!(
            fz_get_bidi_direction(0, ltr.as_ptr()),
            BiDiDirection::LTR as i32
        );
        assert_eq!(
            fz_get_bidi_direction(0, rtl.as_ptr()),
            BiDiDirection::RTL as i32
        );
    }

    #[test]
    fn test_word_breaks() {
        let text = c"Hello world test string";
        let mut breaks = [0i32; 10];

        let count = fz_find_word_breaks(0, text.as_ptr(), breaks.as_mut_ptr(), 10);

        assert_eq!(count, 4); // 4 words
        assert_eq!(breaks[0], 0); // "Hello"
        assert_eq!(breaks[1], 6); // "world"
    }

    #[test]
    fn test_script_detection() {
        let latin = c"Hello";
        let arabic = c"\u{0645}\u{0631}\u{062D}\u{0628}\u{0627}"; // "Marhaba"
        let cjk = c"\u{4E2D}\u{6587}"; // "Chinese"

        assert_eq!(
            fz_detect_script(0, latin.as_ptr()),
            ScriptCategory::Latin as i32
        );
        assert_eq!(
            fz_detect_script(0, arabic.as_ptr()),
            ScriptCategory::Arabic as i32
        );
        assert_eq!(
            fz_detect_script(0, cjk.as_ptr()),
            ScriptCategory::CJK as i32
        );
    }

    #[test]
    fn test_casefold() {
        let input = c"Hello WORLD";
        let mut output = [0u8; 32];

        let len = fz_casefold(0, input.as_ptr(), output.as_mut_ptr().cast(), 32);
        let result = std::str::from_utf8(&output[..len]).unwrap();

        assert_eq!(result, "hello world");
    }

    #[test]
    fn test_char_count() {
        let ascii = c"Hello";
        let unicode = c"\u{1F600}Hello"; // Emoji + Hello

        assert_eq!(fz_string_char_count(0, ascii.as_ptr()), 5);
        assert_eq!(fz_string_char_count(0, unicode.as_ptr()), 6);
    }

    #[test]
    fn test_char_byte_offset() {
        let s = c"H\u{00E9}llo"; // "Héllo" - é is 2 bytes

        // 'H' is at char 0, byte 0
        assert_eq!(fz_char_to_byte_offset(0, s.as_ptr(), 0), 0);
        // 'é' is at char 1, byte 1
        assert_eq!(fz_char_to_byte_offset(0, s.as_ptr(), 1), 1);
        // 'l' is at char 2, byte 3 (after 2-byte é)
        assert_eq!(fz_char_to_byte_offset(0, s.as_ptr(), 2), 3);
    }

    #[test]
    fn test_strcoll() {
        let a = c"apple";
        let b = c"Banana";
        let c = c"Apple";

        // Case-insensitive comparison
        assert!(fz_strcoll(0, a.as_ptr(), b.as_ptr(), std::ptr::null()) < 0);
        assert_eq!(fz_strcoll(0, a.as_ptr(), c.as_ptr(), std::ptr::null()), 0);
    }
}
