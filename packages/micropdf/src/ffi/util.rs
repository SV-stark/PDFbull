//! FFI bindings for fz_util (Utility Functions)
//!
//! Provides various utility functions including:
//! - String manipulation (safe string functions)
//! - Memory helpers
//! - Path manipulation
//! - URI encoding/decoding
//! - UTF-8 handling
//! - Numeric conversion

use crate::ffi::buffer::Buffer;
use crate::ffi::{BUFFERS, Handle};
use std::ffi::{CStr, CString, c_char, c_void};
use std::path::Path;
use std::ptr;

// ============================================================================
// String Length Functions
// ============================================================================

/// Return strlen(s), if less than maxlen, or maxlen if no null byte found
#[unsafe(no_mangle)]
pub extern "C" fn fz_strnlen(s: *const c_char, maxlen: usize) -> usize {
    if s.is_null() {
        return 0;
    }

    let bytes = unsafe { std::slice::from_raw_parts(s as *const u8, maxlen) };
    bytes.iter().position(|&b| b == 0).unwrap_or(maxlen)
}

// ============================================================================
// Safe String Copy/Concatenate Functions
// ============================================================================

/// Copy at most n-1 chars with null termination
/// Returns the length of src (excluding terminator)
#[unsafe(no_mangle)]
pub extern "C" fn fz_strlcpy(dst: *mut c_char, src: *const c_char, n: usize) -> usize {
    if dst.is_null() || src.is_null() || n == 0 {
        return 0;
    }

    let src_cstr = unsafe { CStr::from_ptr(src) };
    let src_bytes = src_cstr.to_bytes();
    let src_len = src_bytes.len();

    let copy_len = std::cmp::min(src_len, n - 1);
    unsafe {
        ptr::copy_nonoverlapping(src as *const u8, dst as *mut u8, copy_len);
        *dst.add(copy_len) = 0;
    }

    src_len
}

/// Concatenate strings with maximum length
/// Returns the real length that dst + src would have
#[unsafe(no_mangle)]
pub extern "C" fn fz_strlcat(dst: *mut c_char, src: *const c_char, n: usize) -> usize {
    if dst.is_null() || src.is_null() || n == 0 {
        return 0;
    }

    let dst_len = fz_strnlen(dst, n);
    if dst_len >= n {
        return dst_len + fz_strnlen(src, usize::MAX);
    }

    let remaining = n - dst_len;
    let src_cstr = unsafe { CStr::from_ptr(src) };
    let src_bytes = src_cstr.to_bytes();
    let src_len = src_bytes.len();

    let copy_len = std::cmp::min(src_len, remaining - 1);
    unsafe {
        ptr::copy_nonoverlapping(src as *const u8, dst.add(dst_len) as *mut u8, copy_len);
        *dst.add(dst_len + copy_len) = 0;
    }

    dst_len + src_len
}

/// Split string at delimiter
#[unsafe(no_mangle)]
pub extern "C" fn fz_strsep(stringp: *mut *mut c_char, delim: *const c_char) -> *mut c_char {
    if stringp.is_null() || delim.is_null() {
        return ptr::null_mut();
    }

    let s = unsafe { *stringp };
    if s.is_null() {
        return ptr::null_mut();
    }

    let delim_cstr = unsafe { CStr::from_ptr(delim) };
    let delim_bytes = delim_cstr.to_bytes();

    let mut p = s;
    loop {
        let c = unsafe { *p };
        if c == 0 {
            unsafe { *stringp = ptr::null_mut() };
            return s;
        }

        if delim_bytes.contains(&(c as u8)) {
            unsafe {
                *p = 0;
                *stringp = p.add(1);
            }
            return s;
        }

        p = unsafe { p.add(1) };
    }
}

// ============================================================================
// String Search Functions
// ============================================================================

/// Safe strstr function
#[unsafe(no_mangle)]
pub extern "C" fn fz_strstr(haystack: *const c_char, needle: *const c_char) -> *const c_char {
    if haystack.is_null() || needle.is_null() {
        return ptr::null();
    }

    let haystack_str = unsafe {
        match CStr::from_ptr(haystack).to_str() {
            Ok(s) => s,
            Err(_) => return ptr::null(),
        }
    };

    let needle_str = unsafe {
        match CStr::from_ptr(needle).to_str() {
            Ok(s) => s,
            Err(_) => return ptr::null(),
        }
    };

    match haystack_str.find(needle_str) {
        Some(pos) => unsafe { haystack.add(pos) },
        None => ptr::null(),
    }
}

/// Case-insensitive strstr (UTF-8 aware)
#[unsafe(no_mangle)]
pub extern "C" fn fz_strstrcase(haystack: *const c_char, needle: *const c_char) -> *const c_char {
    if haystack.is_null() || needle.is_null() {
        return ptr::null();
    }

    let haystack_str = unsafe {
        match CStr::from_ptr(haystack).to_str() {
            Ok(s) => s,
            Err(_) => return ptr::null(),
        }
    };

    let needle_str = unsafe {
        match CStr::from_ptr(needle).to_str() {
            Ok(s) => s,
            Err(_) => return ptr::null(),
        }
    };

    let haystack_lower = haystack_str.to_lowercase();
    let needle_lower = needle_str.to_lowercase();

    match haystack_lower.find(&needle_lower) {
        Some(pos) => {
            // Find the byte position in the original string
            let byte_pos = haystack_str
                .char_indices()
                .zip(haystack_lower.char_indices())
                .find(|((_, _), (lower_pos, _))| *lower_pos == pos)
                .map(|((orig_pos, _), _)| orig_pos)
                .unwrap_or(pos);
            unsafe { haystack.add(byte_pos) }
        }
        None => ptr::null(),
    }
}

/// Find substring in memory (memmem)
#[unsafe(no_mangle)]
pub extern "C" fn fz_memmem(
    haystack: *const c_void,
    haystacklen: usize,
    needle: *const c_void,
    needlelen: usize,
) -> *const c_void {
    if haystack.is_null() || needle.is_null() || needlelen == 0 || haystacklen < needlelen {
        return ptr::null();
    }

    let haystack_bytes = unsafe { std::slice::from_raw_parts(haystack as *const u8, haystacklen) };
    let needle_bytes = unsafe { std::slice::from_raw_parts(needle as *const u8, needlelen) };

    for i in 0..=(haystacklen - needlelen) {
        if &haystack_bytes[i..i + needlelen] == needle_bytes {
            return unsafe { (haystack as *const u8).add(i) as *const c_void };
        }
    }

    ptr::null()
}

// ============================================================================
// Path Manipulation Functions
// ============================================================================

/// Extract directory component from path
#[unsafe(no_mangle)]
pub extern "C" fn fz_dirname(dir: *mut c_char, path: *const c_char, dirsize: usize) {
    if dir.is_null() || path.is_null() || dirsize == 0 {
        return;
    }

    let path_str = unsafe {
        match CStr::from_ptr(path).to_str() {
            Ok(s) => s,
            Err(_) => {
                unsafe { *dir = 0 };
                return;
            }
        }
    };

    let dirname = Path::new(path_str)
        .parent()
        .map(|p| p.to_string_lossy())
        .unwrap_or_default();

    let dirname_bytes = dirname.as_bytes();
    let copy_len = std::cmp::min(dirname_bytes.len(), dirsize - 1);

    unsafe {
        ptr::copy_nonoverlapping(dirname_bytes.as_ptr(), dir as *mut u8, copy_len);
        *dir.add(copy_len) = 0;
    }
}

/// Find filename component in path
#[unsafe(no_mangle)]
pub extern "C" fn fz_basename(path: *const c_char) -> *const c_char {
    if path.is_null() {
        return ptr::null();
    }

    let path_str = unsafe {
        match CStr::from_ptr(path).to_str() {
            Ok(s) => s,
            Err(_) => return path,
        }
    };

    // Find last separator
    let last_sep = path_str
        .rfind(|c| c == '/' || c == '\\')
        .map(|i| i + 1)
        .unwrap_or(0);

    unsafe { path.add(last_sep) }
}

/// Clean path by eliminating multiple slashes and . / ..
#[unsafe(no_mangle)]
pub extern "C" fn fz_cleanname(name: *mut c_char) -> *mut c_char {
    if name.is_null() {
        return name;
    }

    let name_str = unsafe {
        match CStr::from_ptr(name).to_str() {
            Ok(s) => s.to_string(),
            Err(_) => return name,
        }
    };

    let cleaned = clean_path(&name_str);
    let cleaned_bytes = cleaned.as_bytes();

    unsafe {
        ptr::copy_nonoverlapping(cleaned_bytes.as_ptr(), name as *mut u8, cleaned_bytes.len());
        *name.add(cleaned_bytes.len()) = 0;
    }

    name
}

/// Version comparison (like strverscmp)
#[unsafe(no_mangle)]
pub extern "C" fn fz_strverscmp(s1: *const c_char, s2: *const c_char) -> i32 {
    if s1.is_null() && s2.is_null() {
        return 0;
    }
    if s1.is_null() {
        return -1;
    }
    if s2.is_null() {
        return 1;
    }

    let str1 = unsafe {
        match CStr::from_ptr(s1).to_str() {
            Ok(s) => s,
            Err(_) => return 0,
        }
    };

    let str2 = unsafe {
        match CStr::from_ptr(s2).to_str() {
            Ok(s) => s,
            Err(_) => return 0,
        }
    };

    version_compare(str1, str2)
}

// ============================================================================
// URI Encoding/Decoding Functions
// ============================================================================

/// URL decode in-place
#[unsafe(no_mangle)]
pub extern "C" fn fz_urldecode(url: *mut c_char) -> *mut c_char {
    if url.is_null() {
        return url;
    }

    let url_str = unsafe {
        match CStr::from_ptr(url).to_str() {
            Ok(s) => s.to_string(),
            Err(_) => return url,
        }
    };

    let decoded = url_decode(&url_str);
    let decoded_bytes = decoded.as_bytes();

    unsafe {
        ptr::copy_nonoverlapping(decoded_bytes.as_ptr(), url as *mut u8, decoded_bytes.len());
        *url.add(decoded_bytes.len()) = 0;
    }

    url
}

/// Decode URI (preserves reserved chars)
#[unsafe(no_mangle)]
pub extern "C" fn fz_decode_uri(_ctx: Handle, s: *const c_char) -> *mut c_char {
    if s.is_null() {
        return ptr::null_mut();
    }

    let s_str = unsafe {
        match CStr::from_ptr(s).to_str() {
            Ok(s) => s,
            Err(_) => return ptr::null_mut(),
        }
    };

    let decoded = url_decode_preserve_reserved(s_str);
    match CString::new(decoded) {
        Ok(cstr) => cstr.into_raw(),
        Err(_) => ptr::null_mut(),
    }
}

/// Decode URI component (decodes all)
#[unsafe(no_mangle)]
pub extern "C" fn fz_decode_uri_component(_ctx: Handle, s: *const c_char) -> *mut c_char {
    if s.is_null() {
        return ptr::null_mut();
    }

    let s_str = unsafe {
        match CStr::from_ptr(s).to_str() {
            Ok(s) => s,
            Err(_) => return ptr::null_mut(),
        }
    };

    let decoded = url_decode(s_str);
    match CString::new(decoded) {
        Ok(cstr) => cstr.into_raw(),
        Err(_) => ptr::null_mut(),
    }
}

/// Encode URI
#[unsafe(no_mangle)]
pub extern "C" fn fz_encode_uri(_ctx: Handle, s: *const c_char) -> *mut c_char {
    if s.is_null() {
        return ptr::null_mut();
    }

    let s_str = unsafe {
        match CStr::from_ptr(s).to_str() {
            Ok(s) => s,
            Err(_) => return ptr::null_mut(),
        }
    };

    let encoded = url_encode(s_str, false);
    match CString::new(encoded) {
        Ok(cstr) => cstr.into_raw(),
        Err(_) => ptr::null_mut(),
    }
}

/// Encode URI component (encodes reserved chars too)
#[unsafe(no_mangle)]
pub extern "C" fn fz_encode_uri_component(_ctx: Handle, s: *const c_char) -> *mut c_char {
    if s.is_null() {
        return ptr::null_mut();
    }

    let s_str = unsafe {
        match CStr::from_ptr(s).to_str() {
            Ok(s) => s,
            Err(_) => return ptr::null_mut(),
        }
    };

    let encoded = url_encode(s_str, true);
    match CString::new(encoded) {
        Ok(cstr) => cstr.into_raw(),
        Err(_) => ptr::null_mut(),
    }
}

/// Encode URI pathname (encodes reserved except /)
#[unsafe(no_mangle)]
pub extern "C" fn fz_encode_uri_pathname(_ctx: Handle, s: *const c_char) -> *mut c_char {
    if s.is_null() {
        return ptr::null_mut();
    }

    let s_str = unsafe {
        match CStr::from_ptr(s).to_str() {
            Ok(s) => s,
            Err(_) => return ptr::null_mut(),
        }
    };

    let encoded = url_encode_pathname(s_str);
    match CString::new(encoded) {
        Ok(cstr) => cstr.into_raw(),
        Err(_) => ptr::null_mut(),
    }
}

// ============================================================================
// Case Conversion Functions
// ============================================================================

/// Case insensitive string comparison (UTF-8 aware)
#[unsafe(no_mangle)]
pub extern "C" fn fz_strcasecmp(a: *const c_char, b: *const c_char) -> i32 {
    if a.is_null() && b.is_null() {
        return 0;
    }
    if a.is_null() {
        return -1;
    }
    if b.is_null() {
        return 1;
    }

    let a_str = unsafe {
        match CStr::from_ptr(a).to_str() {
            Ok(s) => s.to_lowercase(),
            Err(_) => return 0,
        }
    };

    let b_str = unsafe {
        match CStr::from_ptr(b).to_str() {
            Ok(s) => s.to_lowercase(),
            Err(_) => return 0,
        }
    };

    a_str.cmp(&b_str) as i32
}

/// Case insensitive string comparison with max length
#[unsafe(no_mangle)]
pub extern "C" fn fz_strncasecmp(a: *const c_char, b: *const c_char, n: usize) -> i32 {
    if a.is_null() && b.is_null() {
        return 0;
    }
    if a.is_null() {
        return -1;
    }
    if b.is_null() {
        return 1;
    }

    let a_bytes = unsafe { std::slice::from_raw_parts(a as *const u8, fz_strnlen(a, n)) };
    let b_bytes = unsafe { std::slice::from_raw_parts(b as *const u8, fz_strnlen(b, n)) };

    let a_str = String::from_utf8_lossy(a_bytes).to_lowercase();
    let b_str = String::from_utf8_lossy(b_bytes).to_lowercase();

    a_str.cmp(&b_str) as i32
}

/// Unicode-aware tolower
#[unsafe(no_mangle)]
pub extern "C" fn fz_tolower(c: i32) -> i32 {
    if c < 0 {
        return c;
    }
    if let Some(ch) = char::from_u32(c as u32) {
        ch.to_lowercase().next().map(|c| c as i32).unwrap_or(c)
    } else {
        c
    }
}

/// Unicode-aware toupper
#[unsafe(no_mangle)]
pub extern "C" fn fz_toupper(c: i32) -> i32 {
    if c < 0 {
        return c;
    }
    if let Some(ch) = char::from_u32(c as u32) {
        ch.to_uppercase().next().map(|c| c as i32).unwrap_or(c)
    } else {
        c
    }
}

// ============================================================================
// UTF-8 Functions
// ============================================================================

/// Maximum bytes in a UTF-8 encoded character
pub const FZ_UTFMAX: i32 = 4;

/// Unicode replacement character
pub const FZ_REPLACEMENT_CHARACTER: i32 = 0xFFFD;

/// Decode single UTF-8 rune
#[unsafe(no_mangle)]
pub extern "C" fn fz_chartorune(rune: *mut i32, str: *const c_char) -> i32 {
    if rune.is_null() || str.is_null() {
        return 0;
    }

    let bytes = unsafe { std::slice::from_raw_parts(str as *const u8, 4) };

    // Determine UTF-8 sequence length
    let first = bytes[0];
    let (len, codepoint) = if first & 0x80 == 0 {
        (1, first as u32)
    } else if first & 0xE0 == 0xC0 && bytes.len() >= 2 {
        let c = ((first as u32 & 0x1F) << 6) | (bytes[1] as u32 & 0x3F);
        (2, c)
    } else if first & 0xF0 == 0xE0 && bytes.len() >= 3 {
        let c = ((first as u32 & 0x0F) << 12)
            | ((bytes[1] as u32 & 0x3F) << 6)
            | (bytes[2] as u32 & 0x3F);
        (3, c)
    } else if first & 0xF8 == 0xF0 && bytes.len() >= 4 {
        let c = ((first as u32 & 0x07) << 18)
            | ((bytes[1] as u32 & 0x3F) << 12)
            | ((bytes[2] as u32 & 0x3F) << 6)
            | (bytes[3] as u32 & 0x3F);
        (4, c)
    } else {
        unsafe { *rune = FZ_REPLACEMENT_CHARACTER };
        return 1;
    };

    unsafe { *rune = codepoint as i32 };
    len
}

/// Decode UTF-8 rune with length limit
#[unsafe(no_mangle)]
pub extern "C" fn fz_chartorunen(rune: *mut i32, str: *const c_char, n: usize) -> i32 {
    if rune.is_null() || str.is_null() || n == 0 {
        return 0;
    }

    let bytes = unsafe { std::slice::from_raw_parts(str as *const u8, n) };

    let first = bytes[0];
    let needed = if first & 0x80 == 0 {
        1
    } else if first & 0xE0 == 0xC0 {
        2
    } else if first & 0xF0 == 0xE0 {
        3
    } else if first & 0xF8 == 0xF0 {
        4
    } else {
        unsafe { *rune = FZ_REPLACEMENT_CHARACTER };
        return 1;
    };

    if n < needed {
        unsafe { *rune = FZ_REPLACEMENT_CHARACTER };
        return n as i32;
    }

    fz_chartorune(rune, str)
}

/// Encode rune to UTF-8
#[unsafe(no_mangle)]
pub extern "C" fn fz_runetochar(str: *mut c_char, rune: i32) -> i32 {
    if str.is_null() {
        return 0;
    }

    let codepoint = rune as u32;
    let dst = str as *mut u8;

    if codepoint < 0x80 {
        unsafe { *dst = codepoint as u8 };
        1
    } else if codepoint < 0x800 {
        unsafe {
            *dst = (0xC0 | (codepoint >> 6)) as u8;
            *dst.add(1) = (0x80 | (codepoint & 0x3F)) as u8;
        }
        2
    } else if codepoint < 0x10000 {
        unsafe {
            *dst = (0xE0 | (codepoint >> 12)) as u8;
            *dst.add(1) = (0x80 | ((codepoint >> 6) & 0x3F)) as u8;
            *dst.add(2) = (0x80 | (codepoint & 0x3F)) as u8;
        }
        3
    } else if codepoint < 0x110000 {
        unsafe {
            *dst = (0xF0 | (codepoint >> 18)) as u8;
            *dst.add(1) = (0x80 | ((codepoint >> 12) & 0x3F)) as u8;
            *dst.add(2) = (0x80 | ((codepoint >> 6) & 0x3F)) as u8;
            *dst.add(3) = (0x80 | (codepoint & 0x3F)) as u8;
        }
        4
    } else {
        // Invalid codepoint, encode replacement character
        unsafe {
            *dst = 0xEF;
            *dst.add(1) = 0xBF;
            *dst.add(2) = 0xBD;
        }
        3
    }
}

/// Count bytes required to encode rune
#[unsafe(no_mangle)]
pub extern "C" fn fz_runelen(rune: i32) -> i32 {
    let codepoint = rune as u32;
    if codepoint < 0x80 {
        1
    } else if codepoint < 0x800 {
        2
    } else if codepoint < 0x10000 {
        3
    } else if codepoint < 0x110000 {
        4
    } else {
        3 // Replacement character
    }
}

/// Get rune index at position
#[unsafe(no_mangle)]
pub extern "C" fn fz_runeidx(str: *const c_char, p: *const c_char) -> i32 {
    if str.is_null() || p.is_null() {
        return 0;
    }

    let str_ptr = str as usize;
    let p_ptr = p as usize;
    if p_ptr < str_ptr {
        return 0;
    }

    let offset = p_ptr - str_ptr;
    let str_str = unsafe {
        match CStr::from_ptr(str).to_str() {
            Ok(s) => s,
            Err(_) => return 0,
        }
    };

    str_str
        .char_indices()
        .take_while(|(i, _)| *i < offset)
        .count() as i32
}

/// Get pointer to rune at index
#[unsafe(no_mangle)]
pub extern "C" fn fz_runeptr(str: *const c_char, idx: i32) -> *const c_char {
    if str.is_null() || idx < 0 {
        return ptr::null();
    }

    let str_str = unsafe {
        match CStr::from_ptr(str).to_str() {
            Ok(s) => s,
            Err(_) => return ptr::null(),
        }
    };

    if let Some((byte_pos, _)) = str_str.char_indices().nth(idx as usize) {
        unsafe { str.add(byte_pos) }
    } else {
        ptr::null()
    }
}

/// Count runes in UTF-8 string
#[unsafe(no_mangle)]
pub extern "C" fn fz_utflen(s: *const c_char) -> i32 {
    if s.is_null() {
        return 0;
    }

    let s_str = unsafe {
        match CStr::from_ptr(s).to_str() {
            Ok(s) => s,
            Err(_) => return 0,
        }
    };

    s_str.chars().count() as i32
}

// ============================================================================
// Numeric Conversion Functions
// ============================================================================

/// Locale-independent atof
#[unsafe(no_mangle)]
pub extern "C" fn fz_atof(s: *const c_char) -> f32 {
    if s.is_null() {
        return 0.0;
    }

    let s_str = unsafe {
        match CStr::from_ptr(s).to_str() {
            Ok(s) => s.trim(),
            Err(_) => return 0.0,
        }
    };

    // Handle special cases
    let lower = s_str.to_lowercase();
    if lower == "nan" {
        return f32::NAN;
    }
    if lower == "inf" || lower == "infinity" {
        return f32::INFINITY;
    }
    if lower == "-inf" || lower == "-infinity" {
        return f32::NEG_INFINITY;
    }

    s_str.parse::<f32>().unwrap_or(0.0)
}

/// Locale-independent atoi
#[unsafe(no_mangle)]
pub extern "C" fn fz_atoi(s: *const c_char) -> i32 {
    if s.is_null() {
        return 0;
    }

    let s_str = unsafe {
        match CStr::from_ptr(s).to_str() {
            Ok(s) => s.trim(),
            Err(_) => return 0,
        }
    };

    s_str.parse::<i32>().unwrap_or(0)
}

/// Locale-independent atoi64
#[unsafe(no_mangle)]
pub extern "C" fn fz_atoi64(s: *const c_char) -> i64 {
    if s.is_null() {
        return 0;
    }

    let s_str = unsafe {
        match CStr::from_ptr(s).to_str() {
            Ok(s) => s.trim(),
            Err(_) => return 0,
        }
    };

    s_str.parse::<i64>().unwrap_or(0)
}

/// Locale-independent strtof
#[unsafe(no_mangle)]
pub extern "C" fn fz_strtof(s: *const c_char, es: *mut *mut c_char) -> f32 {
    if s.is_null() {
        if !es.is_null() {
            unsafe { *es = s as *mut c_char };
        }
        return 0.0;
    }

    let s_str = unsafe {
        match CStr::from_ptr(s).to_str() {
            Ok(s) => s,
            Err(_) => {
                if !es.is_null() {
                    unsafe { *es = s as *mut c_char };
                }
                return 0.0;
            }
        }
    };

    // Find end of number
    let trimmed = s_str.trim_start();
    let start_offset = s_str.len() - trimmed.len();

    // Parse prefix (sign, digits, decimal, exponent)
    let mut end_idx = 0;
    let chars: Vec<char> = trimmed.chars().collect();

    // Sign
    if !chars.is_empty() && (chars[0] == '+' || chars[0] == '-') {
        end_idx += 1;
    }

    // Integer part
    while end_idx < chars.len() && chars[end_idx].is_ascii_digit() {
        end_idx += 1;
    }

    // Decimal
    if end_idx < chars.len() && chars[end_idx] == '.' {
        end_idx += 1;
        while end_idx < chars.len() && chars[end_idx].is_ascii_digit() {
            end_idx += 1;
        }
    }

    // Exponent
    if end_idx < chars.len() && (chars[end_idx] == 'e' || chars[end_idx] == 'E') {
        end_idx += 1;
        if end_idx < chars.len() && (chars[end_idx] == '+' || chars[end_idx] == '-') {
            end_idx += 1;
        }
        while end_idx < chars.len() && chars[end_idx].is_ascii_digit() {
            end_idx += 1;
        }
    }

    let num_str: String = chars[..end_idx].iter().collect();
    let value = num_str.parse::<f32>().unwrap_or(0.0);

    if !es.is_null() {
        let byte_offset =
            start_offset + chars[..end_idx].iter().map(|c| c.len_utf8()).sum::<usize>();
        unsafe { *es = s.add(byte_offset) as *mut c_char };
    }

    value
}

// ============================================================================
// Memory Helpers
// ============================================================================

/// Duplicate string
#[unsafe(no_mangle)]
pub extern "C" fn fz_strdup(_ctx: Handle, s: *const c_char) -> *mut c_char {
    if s.is_null() {
        return ptr::null_mut();
    }

    let cstr = unsafe { CStr::from_ptr(s) };
    match CString::new(cstr.to_bytes()) {
        Ok(dup) => dup.into_raw(),
        Err(_) => ptr::null_mut(),
    }
}

/// Free string (for strings allocated by this module)
#[unsafe(no_mangle)]
pub extern "C" fn fz_free_string(_ctx: Handle, s: *mut c_char) {
    if !s.is_null() {
        unsafe {
            let _ = CString::from_raw(s);
        }
    }
}

// ============================================================================
// Page Range Functions
// ============================================================================

/// Check if string is a valid page range
#[unsafe(no_mangle)]
pub extern "C" fn fz_is_page_range(_ctx: Handle, s: *const c_char) -> i32 {
    if s.is_null() {
        return 0;
    }

    let s_str = unsafe {
        match CStr::from_ptr(s).to_str() {
            Ok(s) => s,
            Err(_) => return 0,
        }
    };

    // Check for pattern: (/,?(-?\d+|N)(-(-?\d+|N))?/)+
    let parts: Vec<&str> = s_str.split(',').collect();
    for part in parts {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }

        // Check for range (a-b) or single (a)
        let range_parts: Vec<&str> = part.splitn(2, '-').collect();
        for rp in &range_parts {
            let rp = rp.trim();
            if rp == "N" || rp.is_empty() {
                continue;
            }
            // Check if it's a valid number (possibly negative)
            if rp.starts_with('-') {
                if rp[1..].parse::<i32>().is_err() {
                    return 0;
                }
            } else if rp.parse::<i32>().is_err() {
                return 0;
            }
        }
    }

    1
}

// ============================================================================
// Output Path Formatting
// ============================================================================

/// Format output path with page number
#[unsafe(no_mangle)]
pub extern "C" fn fz_format_output_path(
    _ctx: Handle,
    path: *mut c_char,
    size: usize,
    fmt: *const c_char,
    page: i32,
) {
    if path.is_null() || fmt.is_null() || size == 0 {
        return;
    }

    let fmt_str = unsafe {
        match CStr::from_ptr(fmt).to_str() {
            Ok(s) => s,
            Err(_) => return,
        }
    };

    // Look for %d or %0Nd pattern
    let result = if let Some(pos) = fmt_str.find('%') {
        let remaining = &fmt_str[pos..];
        if remaining.starts_with("%d") {
            format!("{}{}{}", &fmt_str[..pos], page, &fmt_str[pos + 2..])
        } else {
            // Check for %0Nd pattern
            let mut width = 0;
            let mut idx = 1;
            if remaining.chars().nth(1) == Some('0') {
                idx = 2;
                while let Some(c) = remaining.chars().nth(idx) {
                    if c.is_ascii_digit() {
                        width = width * 10 + (c as i32 - '0' as i32);
                        idx += 1;
                    } else {
                        break;
                    }
                }
            }
            if remaining.chars().nth(idx) == Some('d') {
                format!(
                    "{}{:0width$}{}",
                    &fmt_str[..pos],
                    page,
                    &fmt_str[pos + idx + 1..],
                    width = width as usize
                )
            } else {
                // No valid pattern, insert before extension
                insert_page_number(fmt_str, page)
            }
        }
    } else {
        // No % pattern, insert before extension
        insert_page_number(fmt_str, page)
    };

    let result_bytes = result.as_bytes();
    let copy_len = std::cmp::min(result_bytes.len(), size - 1);
    unsafe {
        ptr::copy_nonoverlapping(result_bytes.as_ptr(), path as *mut u8, copy_len);
        *path.add(copy_len) = 0;
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Clean path by removing . and .. and multiple slashes
fn clean_path(path: &str) -> String {
    let mut parts: Vec<&str> = Vec::new();
    let is_absolute = path.starts_with('/');

    for part in path.split('/') {
        match part {
            "" | "." => continue,
            ".." => {
                if !parts.is_empty() && parts.last() != Some(&"..") {
                    parts.pop();
                } else if !is_absolute {
                    parts.push("..");
                }
            }
            _ => parts.push(part),
        }
    }

    let result = parts.join("/");
    if is_absolute {
        format!("/{}", result)
    } else if result.is_empty() {
        ".".to_string()
    } else {
        result
    }
}

/// Version comparison
fn version_compare(s1: &str, s2: &str) -> i32 {
    let mut it1 = s1.chars().peekable();
    let mut it2 = s2.chars().peekable();

    loop {
        let c1 = it1.peek().copied();
        let c2 = it2.peek().copied();

        match (c1, c2) {
            (None, None) => return 0,
            (None, Some(_)) => return -1,
            (Some(_), None) => return 1,
            (Some(a), Some(b)) => {
                // Compare numeric chunks
                if a.is_ascii_digit() && b.is_ascii_digit() {
                    let mut n1: u64 = 0;
                    let mut n2: u64 = 0;

                    while let Some(c) = it1.peek() {
                        if c.is_ascii_digit() {
                            n1 = n1 * 10 + (*c as u64 - '0' as u64);
                            it1.next();
                        } else {
                            break;
                        }
                    }

                    while let Some(c) = it2.peek() {
                        if c.is_ascii_digit() {
                            n2 = n2 * 10 + (*c as u64 - '0' as u64);
                            it2.next();
                        } else {
                            break;
                        }
                    }

                    if n1 != n2 {
                        return if n1 < n2 { -1 } else { 1 };
                    }
                } else {
                    if a != b {
                        return if a < b { -1 } else { 1 };
                    }
                    it1.next();
                    it2.next();
                }
            }
        }
    }
}

/// URL decode
fn url_decode(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars();

    while let Some(c) = chars.next() {
        if c == '%' {
            let hex: String = chars.by_ref().take(2).collect();
            if hex.len() == 2 {
                if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                    result.push(byte as char);
                    continue;
                }
            }
            result.push('%');
            result.push_str(&hex);
        } else if c == '+' {
            result.push(' ');
        } else {
            result.push(c);
        }
    }

    result
}

/// URL decode preserving reserved characters
fn url_decode_preserve_reserved(s: &str) -> String {
    const RESERVED: &[char] = &[';', '/', '?', ':', '@', '&', '=', '+', '$', ',', '#'];

    let mut result = String::new();
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '%' {
            let hex: String = chars.by_ref().take(2).collect();
            if hex.len() == 2 {
                if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                    let decoded = byte as char;
                    if RESERVED.contains(&decoded) {
                        // Keep encoded
                        result.push('%');
                        result.push_str(&hex);
                    } else {
                        result.push(decoded);
                    }
                    continue;
                }
            }
            result.push('%');
            result.push_str(&hex);
        } else {
            result.push(c);
        }
    }

    result
}

/// URL encode
fn url_encode(s: &str, encode_reserved: bool) -> String {
    const RESERVED: &[u8] = b";/?:@&=+$,#";

    let mut result = String::new();

    for byte in s.bytes() {
        if byte.is_ascii_alphanumeric()
            || byte == b'-'
            || byte == b'_'
            || byte == b'.'
            || byte == b'~'
        {
            result.push(byte as char);
        } else if !encode_reserved && RESERVED.contains(&byte) {
            result.push(byte as char);
        } else {
            result.push_str(&format!("%{:02X}", byte));
        }
    }

    result
}

/// URL encode pathname (preserves /)
fn url_encode_pathname(s: &str) -> String {
    let mut result = String::new();

    for byte in s.bytes() {
        if byte.is_ascii_alphanumeric()
            || byte == b'-'
            || byte == b'_'
            || byte == b'.'
            || byte == b'~'
            || byte == b'/'
        {
            result.push(byte as char);
        } else {
            result.push_str(&format!("%{:02X}", byte));
        }
    }

    result
}

/// Insert page number before file extension
fn insert_page_number(path: &str, page: i32) -> String {
    if let Some(dot_pos) = path.rfind('.') {
        format!("{}{}{}", &path[..dot_pos], page, &path[dot_pos..])
    } else {
        format!("{}{}", path, page)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    #[test]
    fn test_strnlen() {
        let s = CString::new("hello").unwrap();
        assert_eq!(fz_strnlen(s.as_ptr(), 10), 5);
        assert_eq!(fz_strnlen(s.as_ptr(), 3), 3);
        assert_eq!(fz_strnlen(ptr::null(), 10), 0);
    }

    #[test]
    fn test_strlcpy() {
        let src = CString::new("hello world").unwrap();
        let mut dst = [0i8; 10];

        let len = fz_strlcpy(dst.as_mut_ptr(), src.as_ptr(), 10);
        assert_eq!(len, 11); // Length of source
        let result = unsafe { CStr::from_ptr(dst.as_ptr()) };
        assert_eq!(result.to_str().unwrap(), "hello wor");
    }

    #[test]
    fn test_strlcat() {
        let mut buf = [0i8; 20];
        let hello = CString::new("hello").unwrap();
        let world = CString::new(" world").unwrap();

        fz_strlcpy(buf.as_mut_ptr(), hello.as_ptr(), 20);
        let len = fz_strlcat(buf.as_mut_ptr(), world.as_ptr(), 20);
        assert_eq!(len, 11);
        let result = unsafe { CStr::from_ptr(buf.as_ptr()) };
        assert_eq!(result.to_str().unwrap(), "hello world");
    }

    #[test]
    fn test_strstr() {
        let haystack = CString::new("hello world").unwrap();
        let needle = CString::new("world").unwrap();

        let result = fz_strstr(haystack.as_ptr(), needle.as_ptr());
        assert!(!result.is_null());
        let found = unsafe { CStr::from_ptr(result) };
        assert_eq!(found.to_str().unwrap(), "world");
    }

    #[test]
    fn test_strstrcase() {
        let haystack = CString::new("Hello World").unwrap();
        let needle = CString::new("WORLD").unwrap();

        let result = fz_strstrcase(haystack.as_ptr(), needle.as_ptr());
        assert!(!result.is_null());
    }

    #[test]
    fn test_dirname() {
        let path = CString::new("/home/user/file.txt").unwrap();
        let mut dir = [0i8; 100];

        fz_dirname(dir.as_mut_ptr(), path.as_ptr(), 100);
        let result = unsafe { CStr::from_ptr(dir.as_ptr()) };
        assert_eq!(result.to_str().unwrap(), "/home/user");
    }

    #[test]
    fn test_basename() {
        let path = CString::new("/home/user/file.txt").unwrap();
        let result = fz_basename(path.as_ptr());
        assert!(!result.is_null());
        let name = unsafe { CStr::from_ptr(result) };
        assert_eq!(name.to_str().unwrap(), "file.txt");
    }

    #[test]
    fn test_cleanname() {
        let mut path = CString::new("/home/../home/./user//file.txt")
            .unwrap()
            .into_raw();
        fz_cleanname(path);
        let result = unsafe { CStr::from_ptr(path) };
        assert_eq!(result.to_str().unwrap(), "/home/user/file.txt");
        unsafe {
            let _ = CString::from_raw(path);
        }
    }

    #[test]
    fn test_strverscmp() {
        let v1 = CString::new("file1.txt").unwrap();
        let v2 = CString::new("file2.txt").unwrap();
        let v10 = CString::new("file10.txt").unwrap();

        assert!(fz_strverscmp(v1.as_ptr(), v2.as_ptr()) < 0);
        assert!(fz_strverscmp(v2.as_ptr(), v10.as_ptr()) < 0);
        assert!(fz_strverscmp(v10.as_ptr(), v1.as_ptr()) > 0);
    }

    #[test]
    fn test_url_encode_decode() {
        let original = "hello world";
        let encoded = url_encode(original, true);
        assert_eq!(encoded, "hello%20world");

        let decoded = url_decode(&encoded);
        assert_eq!(decoded, original);
    }

    #[test]
    fn test_utf8_functions() {
        let s = CString::new("héllo").unwrap();
        assert_eq!(fz_utflen(s.as_ptr()), 5);

        let mut rune: i32 = 0;
        let bytes = fz_chartorune(&mut rune, unsafe { s.as_ptr().offset(1) });
        assert_eq!(bytes, 2); // é is 2 bytes
        assert_eq!(rune, 'é' as i32);
    }

    #[test]
    fn test_runetochar() {
        let mut buf = [0i8; 5];
        let len = fz_runetochar(buf.as_mut_ptr(), '€' as i32);
        assert_eq!(len, 3); // € is 3 bytes in UTF-8
    }

    #[test]
    fn test_tolower_toupper() {
        assert_eq!(fz_tolower('A' as i32), 'a' as i32);
        assert_eq!(fz_toupper('a' as i32), 'A' as i32);
        assert_eq!(fz_tolower('É' as i32), 'é' as i32);
    }

    #[test]
    fn test_atof() {
        let s = CString::new("3.5").unwrap();
        let v = fz_atof(s.as_ptr());
        assert!((v - 3.5).abs() < 0.001);

        let nan = CString::new("NaN").unwrap();
        assert!(fz_atof(nan.as_ptr()).is_nan());

        let inf = CString::new("Inf").unwrap();
        assert!(fz_atof(inf.as_ptr()).is_infinite());
    }

    #[test]
    fn test_atoi() {
        let s = CString::new("42").unwrap();
        assert_eq!(fz_atoi(s.as_ptr()), 42);

        let neg = CString::new("-123").unwrap();
        assert_eq!(fz_atoi(neg.as_ptr()), -123);
    }

    #[test]
    fn test_strcasecmp() {
        let a = CString::new("Hello").unwrap();
        let b = CString::new("HELLO").unwrap();
        assert_eq!(fz_strcasecmp(a.as_ptr(), b.as_ptr()), 0);
    }

    #[test]
    fn test_memmem() {
        let haystack = b"hello world";
        let needle = b"world";

        let result = fz_memmem(
            haystack.as_ptr() as *const c_void,
            haystack.len(),
            needle.as_ptr() as *const c_void,
            needle.len(),
        );

        assert!(!result.is_null());
    }

    #[test]
    fn test_is_page_range() {
        let ctx = 1;
        let valid = CString::new("1-5,7,10-N").unwrap();
        assert_eq!(fz_is_page_range(ctx, valid.as_ptr()), 1);

        let invalid = CString::new("abc").unwrap();
        assert_eq!(fz_is_page_range(ctx, invalid.as_ptr()), 0);
    }

    #[test]
    fn test_format_output_path() {
        let ctx = 1;
        let mut path = [0i8; 100];
        let fmt = CString::new("output%03d.png").unwrap();

        fz_format_output_path(ctx, path.as_mut_ptr(), 100, fmt.as_ptr(), 5);
        let result = unsafe { CStr::from_ptr(path.as_ptr()) };
        assert_eq!(result.to_str().unwrap(), "output005.png");
    }

    #[test]
    fn test_strdup() {
        let ctx = 1;
        let original = CString::new("test string").unwrap();
        let dup = fz_strdup(ctx, original.as_ptr());

        assert!(!dup.is_null());
        let dup_str = unsafe { CStr::from_ptr(dup) };
        assert_eq!(dup_str.to_str().unwrap(), "test string");

        fz_free_string(ctx, dup);
    }
}
