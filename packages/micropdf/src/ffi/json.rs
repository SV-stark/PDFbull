//! FFI bindings for fz_json (JSON Parsing and Output)
//!
//! Provides JSON DOM manipulation, parsing, and serialization.

use crate::ffi::{Handle, HandleStore};
use std::collections::HashMap;
use std::ffi::{CStr, CString, c_char};
use std::ptr;
use std::sync::LazyLock;

// ============================================================================
// Types
// ============================================================================

/// JSON value types
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum JsonType {
    #[default]
    Null = 0,
    True = 1,
    False = 2,
    Number = 3,
    String = 4,
    Array = 5,
    Object = 6,
}

impl JsonType {
    pub fn from_i32(value: i32) -> Self {
        match value {
            0 => JsonType::Null,
            1 => JsonType::True,
            2 => JsonType::False,
            3 => JsonType::Number,
            4 => JsonType::String,
            5 => JsonType::Array,
            6 => JsonType::Object,
            _ => JsonType::Null,
        }
    }
}

/// JSON value
#[derive(Debug, Clone)]
pub enum JsonValue {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<JsonValue>),
    Object(Vec<(String, JsonValue)>), // Use Vec to preserve order
}

impl Default for JsonValue {
    fn default() -> Self {
        JsonValue::Null
    }
}

impl JsonValue {
    pub fn json_type(&self) -> JsonType {
        match self {
            JsonValue::Null => JsonType::Null,
            JsonValue::Bool(true) => JsonType::True,
            JsonValue::Bool(false) => JsonType::False,
            JsonValue::Number(_) => JsonType::Number,
            JsonValue::String(_) => JsonType::String,
            JsonValue::Array(_) => JsonType::Array,
            JsonValue::Object(_) => JsonType::Object,
        }
    }

    pub fn is_null(&self) -> bool {
        matches!(self, JsonValue::Null)
    }

    pub fn is_boolean(&self) -> bool {
        matches!(self, JsonValue::Bool(_))
    }

    pub fn is_number(&self) -> bool {
        matches!(self, JsonValue::Number(_))
    }

    pub fn is_string(&self) -> bool {
        matches!(self, JsonValue::String(_))
    }

    pub fn is_array(&self) -> bool {
        matches!(self, JsonValue::Array(_))
    }

    pub fn is_object(&self) -> bool {
        matches!(self, JsonValue::Object(_))
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            JsonValue::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_number(&self) -> Option<f64> {
        match self {
            JsonValue::Number(n) => Some(*n),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            JsonValue::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&Vec<JsonValue>> {
        match self {
            JsonValue::Array(a) => Some(a),
            _ => None,
        }
    }

    pub fn as_array_mut(&mut self) -> Option<&mut Vec<JsonValue>> {
        match self {
            JsonValue::Array(a) => Some(a),
            _ => None,
        }
    }

    pub fn as_object(&self) -> Option<&Vec<(String, JsonValue)>> {
        match self {
            JsonValue::Object(o) => Some(o),
            _ => None,
        }
    }

    pub fn as_object_mut(&mut self) -> Option<&mut Vec<(String, JsonValue)>> {
        match self {
            JsonValue::Object(o) => Some(o),
            _ => None,
        }
    }

    /// Serialize to JSON string
    pub fn to_json_string(&self) -> String {
        let mut buf = String::new();
        self.write_to(&mut buf);
        buf
    }

    /// Write JSON to string buffer
    fn write_to(&self, buf: &mut String) {
        match self {
            JsonValue::Null => buf.push_str("null"),
            JsonValue::Bool(true) => buf.push_str("true"),
            JsonValue::Bool(false) => buf.push_str("false"),
            JsonValue::Number(n) => {
                if n.is_finite() {
                    // Format number, removing unnecessary trailing zeros
                    let s = format!("{}", n);
                    buf.push_str(&s);
                } else if n.is_nan() {
                    buf.push_str("null"); // JSON doesn't support NaN
                } else if *n > 0.0 {
                    buf.push_str("1e308"); // Approximate infinity
                } else {
                    buf.push_str("-1e308");
                }
            }
            JsonValue::String(s) => {
                buf.push('"');
                escape_json_string(s, buf);
                buf.push('"');
            }
            JsonValue::Array(arr) => {
                buf.push('[');
                for (i, item) in arr.iter().enumerate() {
                    if i > 0 {
                        buf.push(',');
                    }
                    item.write_to(buf);
                }
                buf.push(']');
            }
            JsonValue::Object(obj) => {
                buf.push('{');
                for (i, (key, value)) in obj.iter().enumerate() {
                    if i > 0 {
                        buf.push(',');
                    }
                    buf.push('"');
                    escape_json_string(key, buf);
                    buf.push_str("\":");
                    value.write_to(buf);
                }
                buf.push('}');
            }
        }
    }

    /// Pretty print JSON with indentation
    pub fn to_pretty_json(&self, indent: usize) -> String {
        let mut buf = String::new();
        self.write_pretty(&mut buf, 0, indent);
        buf
    }

    fn write_pretty(&self, buf: &mut String, level: usize, indent: usize) {
        match self {
            JsonValue::Null => buf.push_str("null"),
            JsonValue::Bool(true) => buf.push_str("true"),
            JsonValue::Bool(false) => buf.push_str("false"),
            JsonValue::Number(n) => {
                if n.is_finite() {
                    buf.push_str(&format!("{}", n));
                } else {
                    buf.push_str("null");
                }
            }
            JsonValue::String(s) => {
                buf.push('"');
                escape_json_string(s, buf);
                buf.push('"');
            }
            JsonValue::Array(arr) => {
                if arr.is_empty() {
                    buf.push_str("[]");
                } else {
                    buf.push_str("[\n");
                    for (i, item) in arr.iter().enumerate() {
                        if i > 0 {
                            buf.push_str(",\n");
                        }
                        for _ in 0..(level + 1) * indent {
                            buf.push(' ');
                        }
                        item.write_pretty(buf, level + 1, indent);
                    }
                    buf.push('\n');
                    for _ in 0..level * indent {
                        buf.push(' ');
                    }
                    buf.push(']');
                }
            }
            JsonValue::Object(obj) => {
                if obj.is_empty() {
                    buf.push_str("{}");
                } else {
                    buf.push_str("{\n");
                    for (i, (key, value)) in obj.iter().enumerate() {
                        if i > 0 {
                            buf.push_str(",\n");
                        }
                        for _ in 0..(level + 1) * indent {
                            buf.push(' ');
                        }
                        buf.push('"');
                        escape_json_string(key, buf);
                        buf.push_str("\": ");
                        value.write_pretty(buf, level + 1, indent);
                    }
                    buf.push('\n');
                    for _ in 0..level * indent {
                        buf.push(' ');
                    }
                    buf.push('}');
                }
            }
        }
    }
}

/// Escape a string for JSON output
fn escape_json_string(s: &str, buf: &mut String) {
    for c in s.chars() {
        match c {
            '"' => buf.push_str("\\\""),
            '\\' => buf.push_str("\\\\"),
            '\n' => buf.push_str("\\n"),
            '\r' => buf.push_str("\\r"),
            '\t' => buf.push_str("\\t"),
            '\x08' => buf.push_str("\\b"), // Backspace
            '\x0C' => buf.push_str("\\f"), // Form feed
            c if c.is_control() => {
                // Unicode escape for other control characters
                buf.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => buf.push(c),
        }
    }
}

/// Unescape a JSON string
fn unescape_json_string(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('"') => result.push('"'),
                Some('\\') => result.push('\\'),
                Some('/') => result.push('/'),
                Some('n') => result.push('\n'),
                Some('r') => result.push('\r'),
                Some('t') => result.push('\t'),
                Some('b') => result.push('\x08'),
                Some('f') => result.push('\x0C'),
                Some('u') => {
                    // Unicode escape \uXXXX
                    let hex: String = chars.by_ref().take(4).collect();
                    if let Ok(code) = u32::from_str_radix(&hex, 16) {
                        if let Some(uc) = char::from_u32(code) {
                            result.push(uc);
                        }
                    }
                }
                Some(other) => {
                    result.push('\\');
                    result.push(other);
                }
                None => result.push('\\'),
            }
        } else {
            result.push(c);
        }
    }

    result
}

// ============================================================================
// JSON Parser
// ============================================================================

/// Simple JSON parser
pub struct JsonParser<'a> {
    input: &'a str,
    pos: usize,
}

impl<'a> JsonParser<'a> {
    pub fn new(input: &'a str) -> Self {
        JsonParser { input, pos: 0 }
    }

    pub fn parse(&mut self) -> Option<JsonValue> {
        self.skip_whitespace();
        self.parse_value()
    }

    fn skip_whitespace(&mut self) {
        while self.pos < self.input.len() {
            let c = self.input.as_bytes()[self.pos];
            if c == b' ' || c == b'\t' || c == b'\n' || c == b'\r' {
                self.pos += 1;
            } else {
                break;
            }
        }
    }

    fn peek(&self) -> Option<char> {
        self.input[self.pos..].chars().next()
    }

    fn advance(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.pos += c.len_utf8();
        Some(c)
    }

    fn parse_value(&mut self) -> Option<JsonValue> {
        self.skip_whitespace();

        match self.peek()? {
            'n' => self.parse_null(),
            't' => self.parse_true(),
            'f' => self.parse_false(),
            '"' => self.parse_string(),
            '[' => self.parse_array(),
            '{' => self.parse_object(),
            c if c == '-' || c.is_ascii_digit() => self.parse_number(),
            _ => None,
        }
    }

    fn parse_null(&mut self) -> Option<JsonValue> {
        if self.input[self.pos..].starts_with("null") {
            self.pos += 4;
            Some(JsonValue::Null)
        } else {
            None
        }
    }

    fn parse_true(&mut self) -> Option<JsonValue> {
        if self.input[self.pos..].starts_with("true") {
            self.pos += 4;
            Some(JsonValue::Bool(true))
        } else {
            None
        }
    }

    fn parse_false(&mut self) -> Option<JsonValue> {
        if self.input[self.pos..].starts_with("false") {
            self.pos += 5;
            Some(JsonValue::Bool(false))
        } else {
            None
        }
    }

    fn parse_string(&mut self) -> Option<JsonValue> {
        self.advance()?; // Skip opening quote

        let start = self.pos;
        let mut escaped = false;

        while self.pos < self.input.len() {
            let c = self.input.as_bytes()[self.pos];
            if escaped {
                escaped = false;
                self.pos += 1;
            } else if c == b'\\' {
                escaped = true;
                self.pos += 1;
            } else if c == b'"' {
                let content = &self.input[start..self.pos];
                self.pos += 1; // Skip closing quote
                return Some(JsonValue::String(unescape_json_string(content)));
            } else {
                self.pos += 1;
            }
        }

        None // Unterminated string
    }

    fn parse_number(&mut self) -> Option<JsonValue> {
        let start = self.pos;

        // Optional minus
        if self.peek() == Some('-') {
            self.advance();
        }

        // Integer part
        if self.peek() == Some('0') {
            self.advance();
        } else {
            while let Some(c) = self.peek() {
                if c.is_ascii_digit() {
                    self.advance();
                } else {
                    break;
                }
            }
        }

        // Fractional part
        if self.peek() == Some('.') {
            self.advance();
            while let Some(c) = self.peek() {
                if c.is_ascii_digit() {
                    self.advance();
                } else {
                    break;
                }
            }
        }

        // Exponent
        if let Some(c) = self.peek() {
            if c == 'e' || c == 'E' {
                self.advance();
                if let Some(c) = self.peek() {
                    if c == '+' || c == '-' {
                        self.advance();
                    }
                }
                while let Some(c) = self.peek() {
                    if c.is_ascii_digit() {
                        self.advance();
                    } else {
                        break;
                    }
                }
            }
        }

        let num_str = &self.input[start..self.pos];
        num_str.parse::<f64>().ok().map(JsonValue::Number)
    }

    fn parse_array(&mut self) -> Option<JsonValue> {
        self.advance()?; // Skip '['
        self.skip_whitespace();

        let mut items = Vec::new();

        if self.peek() == Some(']') {
            self.advance();
            return Some(JsonValue::Array(items));
        }

        loop {
            let value = self.parse_value()?;
            items.push(value);

            self.skip_whitespace();

            match self.peek()? {
                ',' => {
                    self.advance();
                    self.skip_whitespace();
                }
                ']' => {
                    self.advance();
                    return Some(JsonValue::Array(items));
                }
                _ => return None,
            }
        }
    }

    fn parse_object(&mut self) -> Option<JsonValue> {
        self.advance()?; // Skip '{'
        self.skip_whitespace();

        let mut entries = Vec::new();

        if self.peek() == Some('}') {
            self.advance();
            return Some(JsonValue::Object(entries));
        }

        loop {
            self.skip_whitespace();

            // Parse key
            if self.peek() != Some('"') {
                return None;
            }
            let key = match self.parse_string()? {
                JsonValue::String(s) => s,
                _ => return None,
            };

            self.skip_whitespace();

            // Expect colon
            if self.advance() != Some(':') {
                return None;
            }

            // Parse value
            let value = self.parse_value()?;
            entries.push((key, value));

            self.skip_whitespace();

            match self.peek()? {
                ',' => {
                    self.advance();
                }
                '}' => {
                    self.advance();
                    return Some(JsonValue::Object(entries));
                }
                _ => return None,
            }
        }
    }
}

// Global JSON store
pub static JSON_VALUES: LazyLock<HandleStore<JsonValue>> = LazyLock::new(HandleStore::new);

// ============================================================================
// FFI Functions - Creation
// ============================================================================

/// Create a null JSON value
#[unsafe(no_mangle)]
pub extern "C" fn fz_json_new_null(_ctx: Handle, _pool: Handle) -> Handle {
    JSON_VALUES.insert(JsonValue::Null)
}

/// Create a boolean JSON value
#[unsafe(no_mangle)]
pub extern "C" fn fz_json_new_boolean(_ctx: Handle, _pool: Handle, value: i32) -> Handle {
    JSON_VALUES.insert(JsonValue::Bool(value != 0))
}

/// Create a number JSON value
#[unsafe(no_mangle)]
pub extern "C" fn fz_json_new_number(_ctx: Handle, _pool: Handle, value: f64) -> Handle {
    JSON_VALUES.insert(JsonValue::Number(value))
}

/// Create a string JSON value
#[unsafe(no_mangle)]
pub extern "C" fn fz_json_new_string(_ctx: Handle, _pool: Handle, value: *const c_char) -> Handle {
    if value.is_null() {
        return JSON_VALUES.insert(JsonValue::String(String::new()));
    }

    let s = unsafe { CStr::from_ptr(value) };
    let s = s.to_str().unwrap_or("");
    JSON_VALUES.insert(JsonValue::String(s.to_string()))
}

/// Create an empty JSON array
#[unsafe(no_mangle)]
pub extern "C" fn fz_json_new_array(_ctx: Handle, _pool: Handle) -> Handle {
    JSON_VALUES.insert(JsonValue::Array(Vec::new()))
}

/// Create an empty JSON object
#[unsafe(no_mangle)]
pub extern "C" fn fz_json_new_object(_ctx: Handle, _pool: Handle) -> Handle {
    JSON_VALUES.insert(JsonValue::Object(Vec::new()))
}

/// Drop/free a JSON value
#[unsafe(no_mangle)]
pub extern "C" fn fz_drop_json(_ctx: Handle, json: Handle) {
    JSON_VALUES.remove(json);
}

// ============================================================================
// FFI Functions - Parsing
// ============================================================================

/// Parse a JSON string
#[unsafe(no_mangle)]
pub extern "C" fn fz_parse_json(_ctx: Handle, _pool: Handle, input: *const c_char) -> Handle {
    if input.is_null() {
        return 0;
    }

    let s = unsafe { CStr::from_ptr(input) };
    let s = match s.to_str() {
        Ok(s) => s,
        Err(_) => return 0,
    };

    let mut parser = JsonParser::new(s);
    match parser.parse() {
        Some(value) => JSON_VALUES.insert(value),
        None => 0,
    }
}

// ============================================================================
// FFI Functions - Serialization
// ============================================================================

/// Serialize JSON to buffer
///
/// Returns number of bytes written
#[unsafe(no_mangle)]
pub extern "C" fn fz_write_json(
    _ctx: Handle,
    json: Handle,
    output: *mut c_char,
    output_size: usize,
) -> usize {
    if output.is_null() || output_size == 0 {
        return 0;
    }

    let j = match JSON_VALUES.get(json) {
        Some(j) => j,
        None => return 0,
    };

    let guard = j.lock().unwrap();
    let result = guard.to_json_string();

    let bytes = result.as_bytes();
    let copy_len = bytes.len().min(output_size - 1);

    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), output as *mut u8, copy_len);
        *output.add(copy_len) = 0; // Null terminate
    }

    copy_len
}

/// Serialize JSON to buffer (pretty printed)
#[unsafe(no_mangle)]
pub extern "C" fn fz_write_json_pretty(
    _ctx: Handle,
    json: Handle,
    indent: usize,
    output: *mut c_char,
    output_size: usize,
) -> usize {
    if output.is_null() || output_size == 0 {
        return 0;
    }

    let j = match JSON_VALUES.get(json) {
        Some(j) => j,
        None => return 0,
    };

    let guard = j.lock().unwrap();
    let result = guard.to_pretty_json(indent);

    let bytes = result.as_bytes();
    let copy_len = bytes.len().min(output_size - 1);

    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), output as *mut u8, copy_len);
        *output.add(copy_len) = 0;
    }

    copy_len
}

/// Get JSON string length (for buffer allocation)
#[unsafe(no_mangle)]
pub extern "C" fn fz_json_string_length(_ctx: Handle, json: Handle) -> usize {
    let j = match JSON_VALUES.get(json) {
        Some(j) => j,
        None => return 0,
    };

    j.lock().unwrap().to_json_string().len()
}

// ============================================================================
// FFI Functions - Type Checking
// ============================================================================

/// Get JSON value type
#[unsafe(no_mangle)]
pub extern "C" fn fz_json_type(_ctx: Handle, json: Handle) -> i32 {
    let j = match JSON_VALUES.get(json) {
        Some(j) => j,
        None => return -1,
    };
    j.lock().unwrap().json_type() as i32
}

/// Check if JSON value is null
#[unsafe(no_mangle)]
pub extern "C" fn fz_json_is_null(_ctx: Handle, json: Handle) -> i32 {
    let j = match JSON_VALUES.get(json) {
        Some(j) => j,
        None => return 0,
    };
    if j.lock().unwrap().is_null() { 1 } else { 0 }
}

/// Check if JSON value is boolean
#[unsafe(no_mangle)]
pub extern "C" fn fz_json_is_boolean(_ctx: Handle, json: Handle) -> i32 {
    let j = match JSON_VALUES.get(json) {
        Some(j) => j,
        None => return 0,
    };
    if j.lock().unwrap().is_boolean() { 1 } else { 0 }
}

/// Check if JSON value is number
#[unsafe(no_mangle)]
pub extern "C" fn fz_json_is_number(_ctx: Handle, json: Handle) -> i32 {
    let j = match JSON_VALUES.get(json) {
        Some(j) => j,
        None => return 0,
    };
    if j.lock().unwrap().is_number() { 1 } else { 0 }
}

/// Check if JSON value is string
#[unsafe(no_mangle)]
pub extern "C" fn fz_json_is_string(_ctx: Handle, json: Handle) -> i32 {
    let j = match JSON_VALUES.get(json) {
        Some(j) => j,
        None => return 0,
    };
    if j.lock().unwrap().is_string() { 1 } else { 0 }
}

/// Check if JSON value is array
#[unsafe(no_mangle)]
pub extern "C" fn fz_json_is_array(_ctx: Handle, json: Handle) -> i32 {
    let j = match JSON_VALUES.get(json) {
        Some(j) => j,
        None => return 0,
    };
    if j.lock().unwrap().is_array() { 1 } else { 0 }
}

/// Check if JSON value is object
#[unsafe(no_mangle)]
pub extern "C" fn fz_json_is_object(_ctx: Handle, json: Handle) -> i32 {
    let j = match JSON_VALUES.get(json) {
        Some(j) => j,
        None => return 0,
    };
    if j.lock().unwrap().is_object() { 1 } else { 0 }
}

// ============================================================================
// FFI Functions - Value Access
// ============================================================================

/// Get boolean value
#[unsafe(no_mangle)]
pub extern "C" fn fz_json_to_boolean(_ctx: Handle, json: Handle) -> i32 {
    let j = match JSON_VALUES.get(json) {
        Some(j) => j,
        None => return 0,
    };
    if j.lock().unwrap().as_bool().unwrap_or(false) {
        1
    } else {
        0
    }
}

/// Get number value
#[unsafe(no_mangle)]
pub extern "C" fn fz_json_to_number(_ctx: Handle, json: Handle) -> f64 {
    let j = match JSON_VALUES.get(json) {
        Some(j) => j,
        None => return 0.0,
    };
    j.lock().unwrap().as_number().unwrap_or(0.0)
}

/// Get string value
#[unsafe(no_mangle)]
pub extern "C" fn fz_json_to_string(
    _ctx: Handle,
    json: Handle,
    output: *mut c_char,
    output_size: usize,
) -> usize {
    if output.is_null() || output_size == 0 {
        return 0;
    }

    let j = match JSON_VALUES.get(json) {
        Some(j) => j,
        None => return 0,
    };

    let guard = j.lock().unwrap();
    let s = match guard.as_str() {
        Some(s) => s,
        None => return 0,
    };

    let bytes = s.as_bytes();
    let copy_len = bytes.len().min(output_size - 1);

    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), output as *mut u8, copy_len);
        *output.add(copy_len) = 0;
    }

    copy_len
}

// ============================================================================
// FFI Functions - Array Operations
// ============================================================================

/// Get array length
#[unsafe(no_mangle)]
pub extern "C" fn fz_json_array_length(_ctx: Handle, json: Handle) -> i32 {
    let j = match JSON_VALUES.get(json) {
        Some(j) => j,
        None => return 0,
    };

    let guard = j.lock().unwrap();
    guard.as_array().map_or(0, |a| a.len()) as i32
}

/// Get array element (returns new handle)
#[unsafe(no_mangle)]
pub extern "C" fn fz_json_array_get(_ctx: Handle, json: Handle, index: i32) -> Handle {
    let j = match JSON_VALUES.get(json) {
        Some(j) => j,
        None => return 0,
    };

    let guard = j.lock().unwrap();
    let arr = match guard.as_array() {
        Some(a) => a,
        None => return 0,
    };

    if index < 0 || index as usize >= arr.len() {
        return 0;
    }

    JSON_VALUES.insert(arr[index as usize].clone())
}

/// Push element to array
#[unsafe(no_mangle)]
pub extern "C" fn fz_json_array_push(
    _ctx: Handle,
    _pool: Handle,
    array: Handle,
    item: Handle,
) -> i32 {
    let arr_arc = match JSON_VALUES.get(array) {
        Some(j) => j,
        None => return 0,
    };

    let item_arc = match JSON_VALUES.get(item) {
        Some(j) => j,
        None => return 0,
    };

    let item_value = item_arc.lock().unwrap().clone();

    let mut arr_guard = arr_arc.lock().unwrap();
    if let Some(arr) = arr_guard.as_array_mut() {
        arr.push(item_value);
        1
    } else {
        0
    }
}

// ============================================================================
// FFI Functions - Object Operations
// ============================================================================

/// Get object property (returns new handle)
#[unsafe(no_mangle)]
pub extern "C" fn fz_json_object_get(_ctx: Handle, json: Handle, key: *const c_char) -> Handle {
    if key.is_null() {
        return 0;
    }

    let j = match JSON_VALUES.get(json) {
        Some(j) => j,
        None => return 0,
    };

    let key_str = unsafe { CStr::from_ptr(key) };
    let key_str = match key_str.to_str() {
        Ok(s) => s,
        Err(_) => return 0,
    };

    let guard = j.lock().unwrap();
    let obj = match guard.as_object() {
        Some(o) => o,
        None => return 0,
    };

    for (k, v) in obj {
        if k == key_str {
            return JSON_VALUES.insert(v.clone());
        }
    }

    0
}

/// Set object property
#[unsafe(no_mangle)]
pub extern "C" fn fz_json_object_set(
    _ctx: Handle,
    _pool: Handle,
    object: Handle,
    key: *const c_char,
    item: Handle,
) -> i32 {
    if key.is_null() {
        return 0;
    }

    let obj_arc = match JSON_VALUES.get(object) {
        Some(j) => j,
        None => return 0,
    };

    let item_arc = match JSON_VALUES.get(item) {
        Some(j) => j,
        None => return 0,
    };

    let key_str = unsafe { CStr::from_ptr(key) };
    let key_str = match key_str.to_str() {
        Ok(s) => s.to_string(),
        Err(_) => return 0,
    };

    let item_value = item_arc.lock().unwrap().clone();

    let mut obj_guard = obj_arc.lock().unwrap();
    if let Some(obj) = obj_guard.as_object_mut() {
        // Check if key exists and update
        for (k, v) in obj.iter_mut() {
            if k == &key_str {
                *v = item_value;
                return 1;
            }
        }
        // Add new key
        obj.push((key_str, item_value));
        1
    } else {
        0
    }
}

/// Get number of keys in object
#[unsafe(no_mangle)]
pub extern "C" fn fz_json_object_length(_ctx: Handle, json: Handle) -> i32 {
    let j = match JSON_VALUES.get(json) {
        Some(j) => j,
        None => return 0,
    };

    let guard = j.lock().unwrap();
    guard.as_object().map_or(0, |o| o.len()) as i32
}

// ============================================================================
// FFI Functions - String Escaping Utilities
// ============================================================================

/// Escape a string for JSON
#[unsafe(no_mangle)]
pub extern "C" fn fz_json_escape_string(
    input: *const c_char,
    output: *mut c_char,
    output_size: usize,
) -> usize {
    if input.is_null() || output.is_null() || output_size == 0 {
        return 0;
    }

    let s = unsafe { CStr::from_ptr(input) };
    let s = match s.to_str() {
        Ok(s) => s,
        Err(_) => return 0,
    };

    let mut buf = String::new();
    escape_json_string(s, &mut buf);

    let bytes = buf.as_bytes();
    let copy_len = bytes.len().min(output_size - 1);

    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), output as *mut u8, copy_len);
        *output.add(copy_len) = 0;
    }

    copy_len
}

/// Unescape a JSON string
#[unsafe(no_mangle)]
pub extern "C" fn fz_json_unescape_string(
    input: *const c_char,
    output: *mut c_char,
    output_size: usize,
) -> usize {
    if input.is_null() || output.is_null() || output_size == 0 {
        return 0;
    }

    let s = unsafe { CStr::from_ptr(input) };
    let s = match s.to_str() {
        Ok(s) => s,
        Err(_) => return 0,
    };

    let result = unescape_json_string(s);

    let bytes = result.as_bytes();
    let copy_len = bytes.len().min(output_size - 1);

    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), output as *mut u8, copy_len);
        *output.add(copy_len) = 0;
    }

    copy_len
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_type_enum() {
        assert_eq!(JsonType::from_i32(0), JsonType::Null);
        assert_eq!(JsonType::from_i32(1), JsonType::True);
        assert_eq!(JsonType::from_i32(5), JsonType::Array);
        assert_eq!(JsonType::from_i32(99), JsonType::Null);
    }

    #[test]
    fn test_create_null() {
        let ctx = 1;
        let json = fz_json_new_null(ctx, 0);
        assert!(json > 0);
        assert_eq!(fz_json_is_null(ctx, json), 1);
        assert_eq!(fz_json_type(ctx, json), JsonType::Null as i32);
        fz_drop_json(ctx, json);
    }

    #[test]
    fn test_create_boolean() {
        let ctx = 1;
        let t = fz_json_new_boolean(ctx, 0, 1);
        let f = fz_json_new_boolean(ctx, 0, 0);

        assert_eq!(fz_json_is_boolean(ctx, t), 1);
        assert_eq!(fz_json_to_boolean(ctx, t), 1);
        assert_eq!(fz_json_to_boolean(ctx, f), 0);

        fz_drop_json(ctx, t);
        fz_drop_json(ctx, f);
    }

    #[test]
    fn test_create_number() {
        let ctx = 1;
        let n = fz_json_new_number(ctx, 0, 42.5);

        assert_eq!(fz_json_is_number(ctx, n), 1);
        assert!((fz_json_to_number(ctx, n) - 42.5).abs() < 0.001);

        fz_drop_json(ctx, n);
    }

    #[test]
    fn test_create_string() {
        let ctx = 1;
        let s = CString::new("hello world").unwrap();
        let json = fz_json_new_string(ctx, 0, s.as_ptr());

        assert_eq!(fz_json_is_string(ctx, json), 1);

        let mut output = vec![0u8; 100];
        let len = fz_json_to_string(ctx, json, output.as_mut_ptr() as *mut c_char, output.len());
        assert_eq!(len, 11);

        fz_drop_json(ctx, json);
    }

    #[test]
    fn test_create_array() {
        let ctx = 1;
        let arr = fz_json_new_array(ctx, 0);

        assert_eq!(fz_json_is_array(ctx, arr), 1);
        assert_eq!(fz_json_array_length(ctx, arr), 0);

        // Add items
        let n1 = fz_json_new_number(ctx, 0, 1.0);
        let n2 = fz_json_new_number(ctx, 0, 2.0);

        fz_json_array_push(ctx, 0, arr, n1);
        fz_json_array_push(ctx, 0, arr, n2);

        assert_eq!(fz_json_array_length(ctx, arr), 2);

        fz_drop_json(ctx, arr);
        fz_drop_json(ctx, n1);
        fz_drop_json(ctx, n2);
    }

    #[test]
    fn test_create_object() {
        let ctx = 1;
        let obj = fz_json_new_object(ctx, 0);

        assert_eq!(fz_json_is_object(ctx, obj), 1);
        assert_eq!(fz_json_object_length(ctx, obj), 0);

        // Add property
        let key = CString::new("name").unwrap();
        let value = fz_json_new_string(ctx, 0, CString::new("test").unwrap().as_ptr());

        fz_json_object_set(ctx, 0, obj, key.as_ptr(), value);
        assert_eq!(fz_json_object_length(ctx, obj), 1);

        // Get property
        let retrieved = fz_json_object_get(ctx, obj, key.as_ptr());
        assert!(retrieved > 0);
        assert_eq!(fz_json_is_string(ctx, retrieved), 1);

        fz_drop_json(ctx, obj);
        fz_drop_json(ctx, value);
        fz_drop_json(ctx, retrieved);
    }

    #[test]
    fn test_parse_json() {
        let ctx = 1;
        let input = CString::new(r#"{"name":"test","value":42}"#).unwrap();

        let json = fz_parse_json(ctx, 0, input.as_ptr());
        assert!(json > 0);
        assert_eq!(fz_json_is_object(ctx, json), 1);

        fz_drop_json(ctx, json);
    }

    #[test]
    fn test_parse_array() {
        let ctx = 1;
        let input = CString::new("[1, 2, 3, 4, 5]").unwrap();

        let json = fz_parse_json(ctx, 0, input.as_ptr());
        assert!(json > 0);
        assert_eq!(fz_json_is_array(ctx, json), 1);
        assert_eq!(fz_json_array_length(ctx, json), 5);

        fz_drop_json(ctx, json);
    }

    #[test]
    fn test_serialize_json() {
        let ctx = 1;
        let obj = fz_json_new_object(ctx, 0);

        let key = CString::new("value").unwrap();
        let num = fz_json_new_number(ctx, 0, 42.0);
        fz_json_object_set(ctx, 0, obj, key.as_ptr(), num);

        let mut output = vec![0u8; 100];
        let len = fz_write_json(ctx, obj, output.as_mut_ptr() as *mut c_char, output.len());
        assert!(len > 0);

        let result = std::str::from_utf8(&output[..len]).unwrap();
        assert!(result.contains("42"));

        fz_drop_json(ctx, obj);
        fz_drop_json(ctx, num);
    }

    #[test]
    fn test_escape_string() {
        let input = CString::new("hello\nworld\t\"test\"").unwrap();
        let mut output = vec![0u8; 100];

        let len = fz_json_escape_string(
            input.as_ptr(),
            output.as_mut_ptr() as *mut c_char,
            output.len(),
        );

        let result = std::str::from_utf8(&output[..len]).unwrap();
        assert!(result.contains("\\n"));
        assert!(result.contains("\\t"));
        assert!(result.contains("\\\""));
    }

    #[test]
    fn test_unescape_string() {
        let input = CString::new(r#"hello\nworld\t\"test\""#).unwrap();
        let mut output = vec![0u8; 100];

        let len = fz_json_unescape_string(
            input.as_ptr(),
            output.as_mut_ptr() as *mut c_char,
            output.len(),
        );

        let result = std::str::from_utf8(&output[..len]).unwrap();
        assert!(result.contains('\n'));
        assert!(result.contains('\t'));
        assert!(result.contains('"'));
    }

    #[test]
    fn test_pretty_print() {
        let ctx = 1;
        let obj = fz_json_new_object(ctx, 0);

        let key = CString::new("items").unwrap();
        let arr = fz_json_new_array(ctx, 0);
        fz_json_array_push(ctx, 0, arr, fz_json_new_number(ctx, 0, 1.0));
        fz_json_array_push(ctx, 0, arr, fz_json_new_number(ctx, 0, 2.0));
        fz_json_object_set(ctx, 0, obj, key.as_ptr(), arr);

        let mut output = vec![0u8; 500];
        let len = fz_write_json_pretty(
            ctx,
            obj,
            2,
            output.as_mut_ptr() as *mut c_char,
            output.len(),
        );

        let result = std::str::from_utf8(&output[..len]).unwrap();
        assert!(result.contains('\n'));
        assert!(result.contains("  ")); // Indentation

        fz_drop_json(ctx, obj);
        fz_drop_json(ctx, arr);
    }

    #[test]
    fn test_null_handling() {
        let ctx = 1;

        assert_eq!(fz_json_type(ctx, 0), -1);
        assert_eq!(fz_json_is_null(ctx, 0), 0);
        assert_eq!(fz_parse_json(ctx, 0, ptr::null()), 0);
        assert_eq!(fz_json_escape_string(ptr::null(), ptr::null_mut(), 0), 0);
    }

    #[test]
    fn test_nested_parse() {
        let ctx = 1;
        let input = CString::new(r#"{"outer":{"inner":[1,2,{"deep":true}]}}"#).unwrap();

        let json = fz_parse_json(ctx, 0, input.as_ptr());
        assert!(json > 0);
        assert_eq!(fz_json_is_object(ctx, json), 1);

        let outer_key = CString::new("outer").unwrap();
        let outer = fz_json_object_get(ctx, json, outer_key.as_ptr());
        assert!(outer > 0);
        assert_eq!(fz_json_is_object(ctx, outer), 1);

        fz_drop_json(ctx, json);
        fz_drop_json(ctx, outer);
    }

    #[test]
    fn test_unicode_string() {
        let ctx = 1;
        let s = CString::new("こんにちは").unwrap();
        let json = fz_json_new_string(ctx, 0, s.as_ptr());

        assert_eq!(fz_json_is_string(ctx, json), 1);

        let mut output = vec![0u8; 100];
        let len = fz_json_to_string(ctx, json, output.as_mut_ptr() as *mut c_char, output.len());
        assert!(len > 0);

        fz_drop_json(ctx, json);
    }
}
