//! FFI bindings for fz_bidi (Bidirectional Text Processing)
//!
//! Implementation of the Unicode Bidirectional Algorithm (UAX #9)
//! for proper display of mixed-direction text (e.g., Hebrew/Arabic with English).

use crate::ffi::Handle;
use std::ffi::c_void;

// ============================================================================
// Types
// ============================================================================

/// Bidirectional text direction
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BidiDirection {
    /// Left-to-Right
    #[default]
    Ltr = 0,
    /// Right-to-Left
    Rtl = 1,
    /// Neutral (auto-detect)
    Neutral = 2,
    /// Unset/Unknown
    Unset = 3,
}

impl BidiDirection {
    pub fn from_i32(value: i32) -> Self {
        match value {
            0 => BidiDirection::Ltr,
            1 => BidiDirection::Rtl,
            2 => BidiDirection::Neutral,
            3 => BidiDirection::Unset,
            _ => BidiDirection::Neutral,
        }
    }

    /// Check if direction is right-to-left
    pub fn is_rtl(self) -> bool {
        matches!(self, BidiDirection::Rtl)
    }

    /// Check if direction is left-to-right
    pub fn is_ltr(self) -> bool {
        matches!(self, BidiDirection::Ltr)
    }
}

/// Bidirectional processing flags
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BidiFlags {
    /// No special processing
    None = 0,
    /// Classify whitespace characters
    ClassifyWhiteSpace = 1,
    /// Replace tab characters
    ReplaceTab = 2,
    /// Both flags
    All = 3,
}

impl BidiFlags {
    pub fn from_i32(value: i32) -> Self {
        match value {
            0 => BidiFlags::None,
            1 => BidiFlags::ClassifyWhiteSpace,
            2 => BidiFlags::ReplaceTab,
            3 => BidiFlags::All,
            _ => BidiFlags::None,
        }
    }

    pub fn has_classify_whitespace(self) -> bool {
        (self as i32 & BidiFlags::ClassifyWhiteSpace as i32) != 0
    }

    pub fn has_replace_tab(self) -> bool {
        (self as i32 & BidiFlags::ReplaceTab as i32) != 0
    }
}

/// Unicode Bidi character types (simplified)
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BidiCharType {
    /// Strong Left-to-Right
    L = 0,
    /// Strong Right-to-Left
    R = 1,
    /// Arabic Letter (Right-to-Left)
    AL = 2,
    /// European Number
    EN = 3,
    /// European Number Separator
    ES = 4,
    /// European Number Terminator
    ET = 5,
    /// Arabic Number
    AN = 6,
    /// Common Number Separator
    CS = 7,
    /// Nonspacing Mark
    NSM = 8,
    /// Boundary Neutral
    BN = 9,
    /// Paragraph Separator
    B = 10,
    /// Segment Separator
    S = 11,
    /// Whitespace
    WS = 12,
    /// Other Neutral
    ON = 13,
    /// Left-to-Right Embedding
    LRE = 14,
    /// Left-to-Right Override
    LRO = 15,
    /// Right-to-Left Embedding
    RLE = 16,
    /// Right-to-Left Override
    RLO = 17,
    /// Pop Directional Format
    PDF = 18,
    /// Left-to-Right Isolate
    LRI = 19,
    /// Right-to-Left Isolate
    RLI = 20,
    /// First Strong Isolate
    FSI = 21,
    /// Pop Directional Isolate
    PDI = 22,
}

/// Bidi fragment information
#[repr(C)]
#[derive(Debug, Clone)]
pub struct BidiFragment {
    /// Start index in original text
    pub start: usize,
    /// Length of fragment
    pub len: usize,
    /// Bidi level (odd = RTL, even = LTR)
    pub level: i32,
    /// Script code
    pub script: i32,
}

/// Callback function type for fragment processing
pub type BidiFragmentFn = Option<
    extern "C" fn(
        fragment: *const u32,
        fragment_len: usize,
        bidi_level: i32,
        script: i32,
        arg: *mut c_void,
    ),
>;

// ============================================================================
// Character Classification
// ============================================================================

/// Get the Bidi character type for a Unicode codepoint
fn get_bidi_type(ch: u32) -> BidiCharType {
    // Simplified classification based on Unicode ranges
    match ch {
        // ASCII letters (Latin)
        0x0041..=0x005A | 0x0061..=0x007A => BidiCharType::L,

        // ASCII digits
        0x0030..=0x0039 => BidiCharType::EN,

        // ASCII punctuation (mostly neutral)
        0x0020 => BidiCharType::WS,
        0x0009 => BidiCharType::S,          // Tab
        0x000A | 0x000D => BidiCharType::B, // Newline, CR
        0x0021..=0x002F | 0x003A..=0x0040 | 0x005B..=0x0060 | 0x007B..=0x007E => BidiCharType::ON,

        // European number separators/terminators
        0x002B | 0x002D => BidiCharType::ES,          // + -
        0x0025 | 0x00A2..=0x00A5 => BidiCharType::ET, // % and currency

        // Common separators
        0x002C | 0x002E | 0x003A => BidiCharType::CS, // , . :

        // Hebrew (RTL)
        0x0590..=0x05FF => BidiCharType::R,

        // Arabic-Indic digits (before general Arabic range)
        0x0660..=0x0669 | 0x06F0..=0x06F9 => BidiCharType::AN,

        // Arabic (RTL) - excluding digit ranges
        0x0600..=0x065F | 0x066A..=0x06EF | 0x06FA..=0x06FF | 0x0750..=0x077F | 0x08A0..=0x08FF => {
            BidiCharType::AL
        }

        // Syriac (RTL)
        0x0700..=0x074F => BidiCharType::R,

        // Thaana (RTL)
        0x0780..=0x07BF => BidiCharType::R,

        // N'Ko (RTL)
        0x07C0..=0x07FF => BidiCharType::R,

        // Latin Extended
        0x00C0..=0x00FF | 0x0100..=0x017F | 0x0180..=0x024F => BidiCharType::L,

        // Greek
        0x0370..=0x03FF => BidiCharType::L,

        // Cyrillic
        0x0400..=0x04FF | 0x0500..=0x052F => BidiCharType::L,

        // CJK (LTR)
        0x4E00..=0x9FFF | 0x3400..=0x4DBF => BidiCharType::L,

        // Hiragana/Katakana
        0x3040..=0x309F | 0x30A0..=0x30FF => BidiCharType::L,

        // Hangul
        0xAC00..=0xD7AF => BidiCharType::L,

        // Bidi control characters
        0x200E => BidiCharType::L,   // LRM
        0x200F => BidiCharType::R,   // RLM
        0x202A => BidiCharType::LRE, // LRE
        0x202B => BidiCharType::RLE, // RLE
        0x202C => BidiCharType::PDF, // PDF
        0x202D => BidiCharType::LRO, // LRO
        0x202E => BidiCharType::RLO, // RLO
        0x2066 => BidiCharType::LRI, // LRI
        0x2067 => BidiCharType::RLI, // RLI
        0x2068 => BidiCharType::FSI, // FSI
        0x2069 => BidiCharType::PDI, // PDI

        // Combining marks (NSM)
        0x0300..=0x036F => BidiCharType::NSM,

        // Format characters (BN)
        0x200B..=0x200D | 0xFEFF => BidiCharType::BN,

        // Default to neutral
        _ => BidiCharType::ON,
    }
}

/// Check if a Bidi type is strong (L, R, AL)
fn is_strong_type(t: BidiCharType) -> bool {
    matches!(t, BidiCharType::L | BidiCharType::R | BidiCharType::AL)
}

/// Check if a Bidi type is RTL strong
fn is_rtl_type(t: BidiCharType) -> bool {
    matches!(t, BidiCharType::R | BidiCharType::AL)
}

// ============================================================================
// Bidi Algorithm Implementation
// ============================================================================

/// Detect base direction from text (first strong character)
pub fn detect_base_direction(text: &[u32]) -> BidiDirection {
    for &ch in text {
        let bidi_type = get_bidi_type(ch);
        if is_strong_type(bidi_type) {
            return if is_rtl_type(bidi_type) {
                BidiDirection::Rtl
            } else {
                BidiDirection::Ltr
            };
        }
    }
    BidiDirection::Ltr // Default to LTR
}

/// Calculate embedding levels for text
pub fn resolve_levels(text: &[u32], base_dir: BidiDirection) -> Vec<i32> {
    let base_level = if base_dir == BidiDirection::Rtl { 1 } else { 0 };
    let mut levels = vec![base_level; text.len()];

    // Simplified level resolution:
    // - Strong LTR at even level
    // - Strong RTL at odd level
    // - Weak and neutral inherit from context

    for (i, &ch) in text.iter().enumerate() {
        let bidi_type = get_bidi_type(ch);

        match bidi_type {
            BidiCharType::L => {
                // LTR character: ensure even level
                if levels[i] % 2 == 1 {
                    levels[i] += 1;
                }
            }
            BidiCharType::R | BidiCharType::AL => {
                // RTL character: ensure odd level
                if levels[i] % 2 == 0 {
                    levels[i] += 1;
                }
            }
            BidiCharType::EN | BidiCharType::AN => {
                // Numbers: level depends on context
                // In RTL context, Arabic numbers stay at odd level
                // European numbers in RTL context go to next even level
                if base_level == 1 && bidi_type == BidiCharType::EN {
                    levels[i] = 2;
                }
            }
            _ => {
                // Neutral and weak: inherit base level (simplified)
            }
        }
    }

    levels
}

/// Reorder text for visual display
pub fn reorder_text(text: &[u32], levels: &[i32]) -> Vec<u32> {
    if text.is_empty() {
        return Vec::new();
    }

    let max_level = *levels.iter().max().unwrap_or(&0);
    let mut result: Vec<u32> = text.to_vec();
    let mut indices: Vec<usize> = (0..text.len()).collect();

    // Reverse subsequences at each level from max down to 1
    for level in (1..=max_level).rev() {
        let mut start = None;

        for i in 0..=text.len() {
            let at_level = i < text.len() && levels[i] >= level;

            match (start, at_level) {
                (None, true) => start = Some(i),
                (Some(s), false) => {
                    // Reverse the run from s to i-1
                    indices[s..i].reverse();
                    start = None;
                }
                _ => {}
            }
        }
    }

    // Apply the reordering
    for (i, &idx) in indices.iter().enumerate() {
        result[i] = text[idx];
    }

    result
}

/// Fragment text into unidirectional runs
pub fn fragment_text(text: &[u32], base_dir: BidiDirection, flags: BidiFlags) -> Vec<BidiFragment> {
    if text.is_empty() {
        return Vec::new();
    }

    // Detect direction if neutral
    let effective_dir = if base_dir == BidiDirection::Neutral {
        detect_base_direction(text)
    } else {
        base_dir
    };

    // Resolve embedding levels
    let levels = resolve_levels(text, effective_dir);

    // Group consecutive characters with same level
    let mut fragments = Vec::new();
    let mut start = 0;
    let mut current_level = levels[0];

    for i in 1..=text.len() {
        let level = if i < text.len() { levels[i] } else { -1 };

        if level != current_level {
            // Handle whitespace classification if requested
            let fragment_end = if flags.has_classify_whitespace() {
                // Don't include trailing whitespace in RTL runs
                let mut end = i;
                while end > start && is_whitespace(text[end - 1]) {
                    end -= 1;
                }
                if end == start {
                    end = i; // Keep at least some content
                }
                end
            } else {
                i
            };

            fragments.push(BidiFragment {
                start,
                len: fragment_end - start,
                level: current_level,
                script: 0, // Script detection would go here
            });

            start = i;
            if i < text.len() {
                current_level = levels[i];
            }
        }
    }

    fragments
}

fn is_whitespace(ch: u32) -> bool {
    matches!(
        ch,
        0x0009 | 0x0020 | 0x00A0 | 0x1680 | 0x2000..=0x200A | 0x202F | 0x205F | 0x3000
    )
}

// ============================================================================
// FFI Functions
// ============================================================================

/// Get Bidi direction for a character
#[unsafe(no_mangle)]
pub extern "C" fn fz_bidi_direction_from_char(ch: u32) -> i32 {
    let bidi_type = get_bidi_type(ch);
    if is_rtl_type(bidi_type) {
        BidiDirection::Rtl as i32
    } else if bidi_type == BidiCharType::L {
        BidiDirection::Ltr as i32
    } else {
        BidiDirection::Neutral as i32
    }
}

/// Detect base direction from text
#[unsafe(no_mangle)]
pub extern "C" fn fz_bidi_detect_direction(_ctx: Handle, text: *const u32, textlen: usize) -> i32 {
    if text.is_null() || textlen == 0 {
        return BidiDirection::Ltr as i32;
    }

    let text_slice = unsafe { std::slice::from_raw_parts(text, textlen) };
    detect_base_direction(text_slice) as i32
}

/// Fragment text into unidirectional runs with callback
#[unsafe(no_mangle)]
pub extern "C" fn fz_bidi_fragment_text(
    _ctx: Handle,
    text: *const u32,
    textlen: usize,
    base_dir: *mut i32,
    callback: BidiFragmentFn,
    arg: *mut c_void,
    flags: i32,
) {
    if text.is_null() || textlen == 0 {
        return;
    }

    let text_slice = unsafe { std::slice::from_raw_parts(text, textlen) };

    // Get or detect base direction
    let dir = if !base_dir.is_null() {
        let d = unsafe { *base_dir };
        if d == BidiDirection::Neutral as i32 {
            let detected = detect_base_direction(text_slice);
            unsafe {
                *base_dir = detected as i32;
            }
            detected
        } else {
            BidiDirection::from_i32(d)
        }
    } else {
        detect_base_direction(text_slice)
    };

    // Fragment the text
    let bidi_flags = BidiFlags::from_i32(flags);
    let fragments = fragment_text(text_slice, dir, bidi_flags);

    // Invoke callback for each fragment
    if let Some(cb) = callback {
        for frag in fragments {
            let frag_ptr = unsafe { text.add(frag.start) };
            cb(frag_ptr, frag.len, frag.level, frag.script, arg);
        }
    }
}

/// Reorder text for visual display
#[unsafe(no_mangle)]
pub extern "C" fn fz_bidi_reorder_run(
    _ctx: Handle,
    text: *const u32,
    textlen: usize,
    base_dir: i32,
    output: *mut u32,
    output_len: usize,
) -> usize {
    if text.is_null() || textlen == 0 || output.is_null() || output_len < textlen {
        return 0;
    }

    let text_slice = unsafe { std::slice::from_raw_parts(text, textlen) };
    let dir = BidiDirection::from_i32(base_dir);

    // Detect direction if neutral
    let effective_dir = if dir == BidiDirection::Neutral {
        detect_base_direction(text_slice)
    } else {
        dir
    };

    // Resolve levels and reorder
    let levels = resolve_levels(text_slice, effective_dir);
    let reordered = reorder_text(text_slice, &levels);

    // Copy to output
    let output_slice = unsafe { std::slice::from_raw_parts_mut(output, output_len) };
    let copy_len = reordered.len().min(output_len);
    output_slice[..copy_len].copy_from_slice(&reordered[..copy_len]);

    copy_len
}

/// Get embedding level for a character at position
#[unsafe(no_mangle)]
pub extern "C" fn fz_bidi_get_level(
    _ctx: Handle,
    text: *const u32,
    textlen: usize,
    base_dir: i32,
    position: usize,
) -> i32 {
    if text.is_null() || textlen == 0 || position >= textlen {
        return 0;
    }

    let text_slice = unsafe { std::slice::from_raw_parts(text, textlen) };
    let dir = BidiDirection::from_i32(base_dir);

    let effective_dir = if dir == BidiDirection::Neutral {
        detect_base_direction(text_slice)
    } else {
        dir
    };

    let levels = resolve_levels(text_slice, effective_dir);
    levels[position]
}

/// Get all embedding levels for text
#[unsafe(no_mangle)]
pub extern "C" fn fz_bidi_get_levels(
    _ctx: Handle,
    text: *const u32,
    textlen: usize,
    base_dir: i32,
    levels_out: *mut i32,
    levels_len: usize,
) -> usize {
    if text.is_null() || textlen == 0 || levels_out.is_null() || levels_len < textlen {
        return 0;
    }

    let text_slice = unsafe { std::slice::from_raw_parts(text, textlen) };
    let dir = BidiDirection::from_i32(base_dir);

    let effective_dir = if dir == BidiDirection::Neutral {
        detect_base_direction(text_slice)
    } else {
        dir
    };

    let levels = resolve_levels(text_slice, effective_dir);

    let output_slice = unsafe { std::slice::from_raw_parts_mut(levels_out, levels_len) };
    let copy_len = levels.len().min(levels_len);
    output_slice[..copy_len].copy_from_slice(&levels[..copy_len]);

    copy_len
}

/// Check if text contains RTL characters
#[unsafe(no_mangle)]
pub extern "C" fn fz_bidi_has_rtl(_ctx: Handle, text: *const u32, textlen: usize) -> i32 {
    if text.is_null() || textlen == 0 {
        return 0;
    }

    let text_slice = unsafe { std::slice::from_raw_parts(text, textlen) };

    for &ch in text_slice {
        if is_rtl_type(get_bidi_type(ch)) {
            return 1;
        }
    }
    0
}

/// Check if text is entirely LTR
#[unsafe(no_mangle)]
pub extern "C" fn fz_bidi_is_ltr_only(_ctx: Handle, text: *const u32, textlen: usize) -> i32 {
    if text.is_null() || textlen == 0 {
        return 1;
    }

    let text_slice = unsafe { std::slice::from_raw_parts(text, textlen) };

    for &ch in text_slice {
        if is_rtl_type(get_bidi_type(ch)) {
            return 0;
        }
    }
    1
}

/// Check if text is entirely RTL
#[unsafe(no_mangle)]
pub extern "C" fn fz_bidi_is_rtl_only(_ctx: Handle, text: *const u32, textlen: usize) -> i32 {
    if text.is_null() || textlen == 0 {
        return 0;
    }

    let text_slice = unsafe { std::slice::from_raw_parts(text, textlen) };
    let mut has_strong = false;

    for &ch in text_slice {
        let bidi_type = get_bidi_type(ch);
        if bidi_type == BidiCharType::L {
            return 0;
        }
        if is_rtl_type(bidi_type) {
            has_strong = true;
        }
    }

    if has_strong { 1 } else { 0 }
}

/// Get Bidi character type
#[unsafe(no_mangle)]
pub extern "C" fn fz_bidi_char_type(ch: u32) -> i32 {
    get_bidi_type(ch) as i32
}

/// Check if character is a Bidi control character
#[unsafe(no_mangle)]
pub extern "C" fn fz_bidi_is_control(ch: u32) -> i32 {
    let bidi_type = get_bidi_type(ch);
    match bidi_type {
        BidiCharType::LRE
        | BidiCharType::RLE
        | BidiCharType::LRO
        | BidiCharType::RLO
        | BidiCharType::PDF
        | BidiCharType::LRI
        | BidiCharType::RLI
        | BidiCharType::FSI
        | BidiCharType::PDI => 1,
        _ => {
            // Also check explicit marks
            if ch == 0x200E || ch == 0x200F { 1 } else { 0 }
        }
    }
}

/// Get mirrored character for RTL display
#[unsafe(no_mangle)]
pub extern "C" fn fz_bidi_get_mirror(ch: u32) -> u32 {
    // Common mirrored pairs
    match ch {
        0x0028 => 0x0029, // ( -> )
        0x0029 => 0x0028, // ) -> (
        0x003C => 0x003E, // < -> >
        0x003E => 0x003C, // > -> <
        0x005B => 0x005D, // [ -> ]
        0x005D => 0x005B, // ] -> [
        0x007B => 0x007D, // { -> }
        0x007D => 0x007B, // } -> {
        0x00AB => 0x00BB, // « -> »
        0x00BB => 0x00AB, // » -> «
        0x2039 => 0x203A, // ‹ -> ›
        0x203A => 0x2039, // › -> ‹
        0x2045 => 0x2046, // ⁅ -> ⁆
        0x2046 => 0x2045, // ⁆ -> ⁅
        0x207D => 0x207E, // ⁽ -> ⁾
        0x207E => 0x207D, // ⁾ -> ⁽
        0x208D => 0x208E, // ₍ -> ₎
        0x208E => 0x208D, // ₎ -> ₍
        0x2208 => 0x220B, // ∈ -> ∋
        0x220B => 0x2208, // ∋ -> ∈
        0x2264 => 0x2265, // ≤ -> ≥
        0x2265 => 0x2264, // ≥ -> ≤
        0x2329 => 0x232A, // 〈 -> 〉
        0x232A => 0x2329, // 〉 -> 〈
        0x3008 => 0x3009, // 〈 -> 〉
        0x3009 => 0x3008, // 〉 -> 〈
        0x300A => 0x300B, // 《 -> 》
        0x300B => 0x300A, // 》 -> 《
        0x300C => 0x300D, // 「 -> 」
        0x300D => 0x300C, // 」 -> 「
        0x300E => 0x300F, // 『 -> 』
        0x300F => 0x300E, // 』 -> 『
        0x3010 => 0x3011, // 【 -> 】
        0x3011 => 0x3010, // 】 -> 【
        0x3014 => 0x3015, // 〔 -> 〕
        0x3015 => 0x3014, // 〕 -> 〔
        0x3016 => 0x3017, // 〖 -> 〗
        0x3017 => 0x3016, // 〗 -> 〖
        0x3018 => 0x3019, // 〘 -> 〙
        0x3019 => 0x3018, // 〙 -> 〘
        0x301A => 0x301B, // 〚 -> 〛
        0x301B => 0x301A, // 〛 -> 〚
        _ => ch,          // No mirror
    }
}

/// Check if character has a mirror
#[unsafe(no_mangle)]
pub extern "C" fn fz_bidi_has_mirror(ch: u32) -> i32 {
    if fz_bidi_get_mirror(ch) != ch { 1 } else { 0 }
}

/// Strip Bidi control characters from text
#[unsafe(no_mangle)]
pub extern "C" fn fz_bidi_strip_controls(
    _ctx: Handle,
    text: *const u32,
    textlen: usize,
    output: *mut u32,
    output_len: usize,
) -> usize {
    if text.is_null() || textlen == 0 || output.is_null() {
        return 0;
    }

    let text_slice = unsafe { std::slice::from_raw_parts(text, textlen) };
    let output_slice = unsafe { std::slice::from_raw_parts_mut(output, output_len) };

    let mut out_idx = 0;
    for &ch in text_slice {
        if fz_bidi_is_control(ch) == 0 && out_idx < output_len {
            output_slice[out_idx] = ch;
            out_idx += 1;
        }
    }

    out_idx
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bidi_direction_enum() {
        assert_eq!(BidiDirection::from_i32(0), BidiDirection::Ltr);
        assert_eq!(BidiDirection::from_i32(1), BidiDirection::Rtl);
        assert_eq!(BidiDirection::from_i32(2), BidiDirection::Neutral);
        assert_eq!(BidiDirection::from_i32(3), BidiDirection::Unset);
        assert_eq!(BidiDirection::from_i32(99), BidiDirection::Neutral);
    }

    #[test]
    fn test_bidi_flags() {
        assert!(BidiFlags::ClassifyWhiteSpace.has_classify_whitespace());
        assert!(!BidiFlags::ClassifyWhiteSpace.has_replace_tab());
        assert!(BidiFlags::ReplaceTab.has_replace_tab());
        assert!(BidiFlags::All.has_classify_whitespace());
        assert!(BidiFlags::All.has_replace_tab());
    }

    #[test]
    fn test_detect_ltr() {
        let text: Vec<u32> = "Hello World".chars().map(|c| c as u32).collect();
        let dir = detect_base_direction(&text);
        assert_eq!(dir, BidiDirection::Ltr);
    }

    #[test]
    fn test_detect_rtl_hebrew() {
        // Hebrew text "שלום"
        let text: Vec<u32> = vec![0x05E9, 0x05DC, 0x05D5, 0x05DD];
        let dir = detect_base_direction(&text);
        assert_eq!(dir, BidiDirection::Rtl);
    }

    #[test]
    fn test_detect_rtl_arabic() {
        // Arabic text "مرحبا"
        let text: Vec<u32> = vec![0x0645, 0x0631, 0x062D, 0x0628, 0x0627];
        let dir = detect_base_direction(&text);
        assert_eq!(dir, BidiDirection::Rtl);
    }

    #[test]
    fn test_bidi_char_types() {
        assert_eq!(get_bidi_type('A' as u32), BidiCharType::L);
        assert_eq!(get_bidi_type('z' as u32), BidiCharType::L);
        assert_eq!(get_bidi_type('5' as u32), BidiCharType::EN);
        assert_eq!(get_bidi_type(' ' as u32), BidiCharType::WS);
        assert_eq!(get_bidi_type(0x05D0), BidiCharType::R); // Hebrew Alef
        assert_eq!(get_bidi_type(0x0627), BidiCharType::AL); // Arabic Alif
    }

    #[test]
    fn test_resolve_levels_ltr() {
        let text: Vec<u32> = "Hello".chars().map(|c| c as u32).collect();
        let levels = resolve_levels(&text, BidiDirection::Ltr);
        assert!(levels.iter().all(|&l| l == 0));
    }

    #[test]
    fn test_resolve_levels_rtl() {
        // Hebrew "שלום"
        let text: Vec<u32> = vec![0x05E9, 0x05DC, 0x05D5, 0x05DD];
        let levels = resolve_levels(&text, BidiDirection::Rtl);
        assert!(levels.iter().all(|&l| l == 1));
    }

    #[test]
    fn test_resolve_levels_mixed() {
        // Hebrew + English: "שלום Hello"
        let text: Vec<u32> = vec![
            0x05E9, 0x05DC, 0x05D5, 0x05DD, // שלום
            ' ' as u32, 'H' as u32, 'e' as u32, 'l' as u32, 'l' as u32, 'o' as u32,
        ];
        let levels = resolve_levels(&text, BidiDirection::Rtl);

        // Hebrew should be level 1, English should be level 2
        assert_eq!(levels[0], 1); // ש
        assert_eq!(levels[5], 2); // H
    }

    #[test]
    fn test_reorder_ltr() {
        let text: Vec<u32> = "Hello".chars().map(|c| c as u32).collect();
        let levels = resolve_levels(&text, BidiDirection::Ltr);
        let reordered = reorder_text(&text, &levels);
        assert_eq!(reordered, text);
    }

    #[test]
    fn test_reorder_rtl() {
        // Simple RTL text should be reversed at level 1
        let text: Vec<u32> = vec![0x05D0, 0x05D1, 0x05D2]; // אבג
        let levels = resolve_levels(&text, BidiDirection::Rtl);
        let reordered = reorder_text(&text, &levels);
        assert_eq!(reordered, vec![0x05D2, 0x05D1, 0x05D0]); // גבא
    }

    #[test]
    fn test_fragment_text_ltr() {
        let text: Vec<u32> = "Hello World".chars().map(|c| c as u32).collect();
        let fragments = fragment_text(&text, BidiDirection::Ltr, BidiFlags::None);
        assert_eq!(fragments.len(), 1);
        assert_eq!(fragments[0].level, 0);
    }

    #[test]
    fn test_fragment_text_rtl() {
        let text: Vec<u32> = vec![0x05E9, 0x05DC, 0x05D5, 0x05DD]; // שלום
        let fragments = fragment_text(&text, BidiDirection::Rtl, BidiFlags::None);
        assert_eq!(fragments.len(), 1);
        assert_eq!(fragments[0].level, 1);
    }

    #[test]
    fn test_ffi_detect_direction() {
        let text: Vec<u32> = "Hello".chars().map(|c| c as u32).collect();
        let dir = fz_bidi_detect_direction(1, text.as_ptr(), text.len());
        assert_eq!(dir, BidiDirection::Ltr as i32);
    }

    #[test]
    fn test_ffi_has_rtl() {
        let ltr_text: Vec<u32> = "Hello".chars().map(|c| c as u32).collect();
        assert_eq!(fz_bidi_has_rtl(1, ltr_text.as_ptr(), ltr_text.len()), 0);

        let rtl_text: Vec<u32> = vec![0x05D0, 0x05D1]; // אב
        assert_eq!(fz_bidi_has_rtl(1, rtl_text.as_ptr(), rtl_text.len()), 1);
    }

    #[test]
    fn test_ffi_is_ltr_only() {
        let ltr_text: Vec<u32> = "Hello123".chars().map(|c| c as u32).collect();
        assert_eq!(fz_bidi_is_ltr_only(1, ltr_text.as_ptr(), ltr_text.len()), 1);

        let mixed: Vec<u32> = vec!['H' as u32, 0x05D0]; // H + alef
        assert_eq!(fz_bidi_is_ltr_only(1, mixed.as_ptr(), mixed.len()), 0);
    }

    #[test]
    fn test_ffi_char_type() {
        assert_eq!(fz_bidi_char_type('A' as u32), BidiCharType::L as i32);
        assert_eq!(fz_bidi_char_type(0x05D0), BidiCharType::R as i32);
        assert_eq!(fz_bidi_char_type(0x0627), BidiCharType::AL as i32);
    }

    #[test]
    fn test_ffi_is_control() {
        assert_eq!(fz_bidi_is_control(0x200E), 1); // LRM
        assert_eq!(fz_bidi_is_control(0x200F), 1); // RLM
        assert_eq!(fz_bidi_is_control(0x202A), 1); // LRE
        assert_eq!(fz_bidi_is_control('A' as u32), 0);
    }

    #[test]
    fn test_ffi_get_mirror() {
        assert_eq!(fz_bidi_get_mirror('(' as u32), ')' as u32);
        assert_eq!(fz_bidi_get_mirror(')' as u32), '(' as u32);
        assert_eq!(fz_bidi_get_mirror('<' as u32), '>' as u32);
        assert_eq!(fz_bidi_get_mirror('A' as u32), 'A' as u32);
    }

    #[test]
    fn test_ffi_has_mirror() {
        assert_eq!(fz_bidi_has_mirror('(' as u32), 1);
        assert_eq!(fz_bidi_has_mirror('A' as u32), 0);
    }

    #[test]
    fn test_ffi_reorder_run() {
        let text: Vec<u32> = vec![0x05D0, 0x05D1, 0x05D2]; // אבג
        let mut output = vec![0u32; 3];

        let len = fz_bidi_reorder_run(
            1,
            text.as_ptr(),
            text.len(),
            BidiDirection::Rtl as i32,
            output.as_mut_ptr(),
            output.len(),
        );

        assert_eq!(len, 3);
        assert_eq!(output, vec![0x05D2, 0x05D1, 0x05D0]); // גבא
    }

    #[test]
    fn test_ffi_get_levels() {
        let text: Vec<u32> = "Hello".chars().map(|c| c as u32).collect();
        let mut levels = vec![0i32; text.len()];

        let len = fz_bidi_get_levels(
            1,
            text.as_ptr(),
            text.len(),
            BidiDirection::Ltr as i32,
            levels.as_mut_ptr(),
            levels.len(),
        );

        assert_eq!(len, text.len());
        assert!(levels.iter().all(|&l| l == 0));
    }

    #[test]
    fn test_ffi_strip_controls() {
        let text: Vec<u32> = vec![
            'H' as u32, 0x200E, // LRM
            'i' as u32, 0x200F, // RLM
        ];
        let mut output = vec![0u32; 4];

        let len = fz_bidi_strip_controls(
            1,
            text.as_ptr(),
            text.len(),
            output.as_mut_ptr(),
            output.len(),
        );

        assert_eq!(len, 2);
        assert_eq!(output[0], 'H' as u32);
        assert_eq!(output[1], 'i' as u32);
    }

    #[test]
    fn test_ffi_fragment_callback() {
        use std::sync::atomic::{AtomicI32, AtomicUsize, Ordering};

        static CALLBACK_COUNT: AtomicI32 = AtomicI32::new(0);
        static TOTAL_LEN: AtomicUsize = AtomicUsize::new(0);

        extern "C" fn callback(
            _fragment: *const u32,
            fragment_len: usize,
            _bidi_level: i32,
            _script: i32,
            _arg: *mut c_void,
        ) {
            CALLBACK_COUNT.fetch_add(1, Ordering::SeqCst);
            TOTAL_LEN.fetch_add(fragment_len, Ordering::SeqCst);
        }

        let text: Vec<u32> = "Hello".chars().map(|c| c as u32).collect();
        let mut base_dir = BidiDirection::Neutral as i32;

        CALLBACK_COUNT.store(0, Ordering::SeqCst);
        TOTAL_LEN.store(0, Ordering::SeqCst);

        fz_bidi_fragment_text(
            1,
            text.as_ptr(),
            text.len(),
            &mut base_dir,
            Some(callback),
            std::ptr::null_mut(),
            0,
        );

        assert!(CALLBACK_COUNT.load(Ordering::SeqCst) >= 1);
        assert_eq!(TOTAL_LEN.load(Ordering::SeqCst), text.len());
    }
}
