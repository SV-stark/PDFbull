//! PDF Parse FFI Module
//!
//! Provides PDF lexer and parsing capabilities for PDF documents.
//! This module implements the MuPDF pdf_parse API for tokenizing and
//! parsing PDF syntax structures.

use crate::ffi::{Handle, HandleStore};
use std::ffi::{CStr, CString, c_char, c_void};
use std::ptr;
use std::sync::LazyLock;

// ============================================================================
// Constants
// ============================================================================

/// Small lexer buffer size (256 bytes)
pub const PDF_LEXBUF_SMALL: usize = 256;

/// Large lexer buffer size (64KB)
pub const PDF_LEXBUF_LARGE: usize = 65536;

// ============================================================================
// Token Types
// ============================================================================

/// PDF token types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(C)]
pub enum PdfToken {
    #[default]
    Error = 0,
    Eof = 1,
    OpenArray = 2,  // [
    CloseArray = 3, // ]
    OpenDict = 4,   // <<
    CloseDict = 5,  // >>
    OpenBrace = 6,  // {
    CloseBrace = 7, // }
    Name = 8,       // /Name
    Int = 9,        // 123
    Real = 10,      // 1.23
    String = 11,    // (string) or <hex>
    Keyword = 12,   // keyword
    R = 13,         // R (reference)
    True = 14,      // true
    False = 15,     // false
    Null = 16,      // null
    Obj = 17,       // obj
    EndObj = 18,    // endobj
    Stream = 19,    // stream
    EndStream = 20, // endstream
    Xref = 21,      // xref
    Trailer = 22,   // trailer
    StartXref = 23, // startxref
    NewObj = 24,    // For incremental updates
    NumTokens = 25,
}

impl PdfToken {
    pub fn from_i32(value: i32) -> Self {
        match value {
            0 => PdfToken::Error,
            1 => PdfToken::Eof,
            2 => PdfToken::OpenArray,
            3 => PdfToken::CloseArray,
            4 => PdfToken::OpenDict,
            5 => PdfToken::CloseDict,
            6 => PdfToken::OpenBrace,
            7 => PdfToken::CloseBrace,
            8 => PdfToken::Name,
            9 => PdfToken::Int,
            10 => PdfToken::Real,
            11 => PdfToken::String,
            12 => PdfToken::Keyword,
            13 => PdfToken::R,
            14 => PdfToken::True,
            15 => PdfToken::False,
            16 => PdfToken::Null,
            17 => PdfToken::Obj,
            18 => PdfToken::EndObj,
            19 => PdfToken::Stream,
            20 => PdfToken::EndStream,
            21 => PdfToken::Xref,
            22 => PdfToken::Trailer,
            23 => PdfToken::StartXref,
            24 => PdfToken::NewObj,
            _ => PdfToken::Error,
        }
    }

    pub fn to_string(&self) -> &'static str {
        match self {
            PdfToken::Error => "error",
            PdfToken::Eof => "EOF",
            PdfToken::OpenArray => "[",
            PdfToken::CloseArray => "]",
            PdfToken::OpenDict => "<<",
            PdfToken::CloseDict => ">>",
            PdfToken::OpenBrace => "{",
            PdfToken::CloseBrace => "}",
            PdfToken::Name => "name",
            PdfToken::Int => "integer",
            PdfToken::Real => "real",
            PdfToken::String => "string",
            PdfToken::Keyword => "keyword",
            PdfToken::R => "R",
            PdfToken::True => "true",
            PdfToken::False => "false",
            PdfToken::Null => "null",
            PdfToken::Obj => "obj",
            PdfToken::EndObj => "endobj",
            PdfToken::Stream => "stream",
            PdfToken::EndStream => "endstream",
            PdfToken::Xref => "xref",
            PdfToken::Trailer => "trailer",
            PdfToken::StartXref => "startxref",
            PdfToken::NewObj => "newobj",
            PdfToken::NumTokens => "numtokens",
        }
    }
}

// ============================================================================
// Lexer Buffer Structure
// ============================================================================

/// PDF Lexer Buffer
#[derive(Debug, Clone)]
pub struct PdfLexbuf {
    /// Total allocated size
    pub size: usize,
    /// Base buffer size
    pub base_size: usize,
    /// Current content length
    pub len: usize,
    /// Integer value for Int token
    pub i: i64,
    /// Float value for Real token
    pub f: f32,
    /// Scratch buffer for string content
    pub scratch: Vec<u8>,
    /// Fixed buffer for small content
    pub buffer: Vec<u8>,
    /// Current token type
    pub token: PdfToken,
    /// String value (for Name/String/Keyword tokens)
    pub string_value: String,
}

impl Default for PdfLexbuf {
    fn default() -> Self {
        Self::new(PDF_LEXBUF_SMALL)
    }
}

impl PdfLexbuf {
    pub fn new(size: usize) -> Self {
        Self {
            size,
            base_size: size,
            len: 0,
            i: 0,
            f: 0.0,
            scratch: Vec::with_capacity(size),
            buffer: vec![0u8; size],
            token: PdfToken::Error,
            string_value: String::new(),
        }
    }

    pub fn new_large() -> Self {
        Self::new(PDF_LEXBUF_LARGE)
    }

    /// Reset the lexer buffer state
    pub fn reset(&mut self) {
        self.len = 0;
        self.i = 0;
        self.f = 0.0;
        self.scratch.clear();
        self.token = PdfToken::Error;
        self.string_value.clear();
    }

    /// Grow the scratch buffer if needed
    pub fn grow(&mut self) -> isize {
        let old_size = self.size;
        let new_size = if old_size == 0 { 256 } else { old_size * 2 };
        self.scratch.reserve(new_size);
        self.buffer.resize(new_size, 0);
        self.size = new_size;
        (new_size - old_size) as isize
    }

    /// Get the scratch buffer as a string
    pub fn get_scratch_string(&self) -> String {
        String::from_utf8_lossy(&self.scratch[..self.len]).to_string()
    }
}

// ============================================================================
// PDF Parser
// ============================================================================

/// PDF Parser state
#[derive(Debug)]
pub struct PdfParser {
    /// Lexer buffer
    pub lexbuf: PdfLexbuf,
    /// Current position in input
    pub pos: usize,
    /// Input data
    pub data: Vec<u8>,
    /// Last error message
    pub error: Option<String>,
}

impl PdfParser {
    pub fn new(data: Vec<u8>) -> Self {
        Self {
            lexbuf: PdfLexbuf::default(),
            pos: 0,
            data,
            error: None,
        }
    }

    pub fn new_with_lexbuf(data: Vec<u8>, size: usize) -> Self {
        Self {
            lexbuf: PdfLexbuf::new(size),
            pos: 0,
            data,
            error: None,
        }
    }

    /// Check if we've reached end of input
    pub fn at_eof(&self) -> bool {
        self.pos >= self.data.len()
    }

    /// Peek at the next character without consuming
    pub fn peek(&self) -> Option<u8> {
        if self.pos < self.data.len() {
            Some(self.data[self.pos])
        } else {
            None
        }
    }

    /// Peek at a character at offset
    pub fn peek_at(&self, offset: usize) -> Option<u8> {
        let idx = self.pos + offset;
        if idx < self.data.len() {
            Some(self.data[idx])
        } else {
            None
        }
    }

    /// Get the next character
    pub fn next(&mut self) -> Option<u8> {
        if self.pos < self.data.len() {
            let ch = self.data[self.pos];
            self.pos += 1;
            Some(ch)
        } else {
            None
        }
    }

    /// Skip whitespace and comments
    pub fn skip_whitespace(&mut self) {
        while let Some(ch) = self.peek() {
            match ch {
                // Whitespace characters
                b' ' | b'\t' | b'\r' | b'\n' | 0x00 | 0x0c => {
                    self.pos += 1;
                }
                // Comment
                b'%' => {
                    self.pos += 1;
                    // Skip until end of line
                    while let Some(c) = self.peek() {
                        self.pos += 1;
                        if c == b'\r' || c == b'\n' {
                            break;
                        }
                    }
                }
                _ => break,
            }
        }
    }

    /// Check if character is a PDF delimiter
    fn is_delimiter(ch: u8) -> bool {
        matches!(
            ch,
            b'(' | b')' | b'<' | b'>' | b'[' | b']' | b'{' | b'}' | b'/' | b'%'
        )
    }

    /// Check if character is whitespace
    fn is_whitespace(ch: u8) -> bool {
        matches!(ch, b' ' | b'\t' | b'\r' | b'\n' | 0x00 | 0x0c)
    }

    /// Read a name token (after /)
    fn read_name(&mut self) -> PdfToken {
        self.lexbuf.scratch.clear();

        while let Some(ch) = self.peek() {
            if Self::is_whitespace(ch) || Self::is_delimiter(ch) {
                break;
            }

            self.pos += 1;

            // Handle #XX hex escapes
            if ch == b'#' {
                if let (Some(h1), Some(h2)) = (self.peek(), self.peek_at(1)) {
                    if h1.is_ascii_hexdigit() && h2.is_ascii_hexdigit() {
                        self.pos += 2;
                        let value = (Self::hex_digit(h1) << 4) | Self::hex_digit(h2);
                        self.lexbuf.scratch.push(value);
                        continue;
                    }
                }
            }

            self.lexbuf.scratch.push(ch);
        }

        self.lexbuf.len = self.lexbuf.scratch.len();
        self.lexbuf.string_value = self.lexbuf.get_scratch_string();
        PdfToken::Name
    }

    /// Convert hex digit to value
    fn hex_digit(ch: u8) -> u8 {
        match ch {
            b'0'..=b'9' => ch - b'0',
            b'a'..=b'f' => ch - b'a' + 10,
            b'A'..=b'F' => ch - b'A' + 10,
            _ => 0,
        }
    }

    /// Read a number (integer or real)
    fn read_number(&mut self, first: u8) -> PdfToken {
        self.lexbuf.scratch.clear();
        self.lexbuf.scratch.push(first);

        // If the first character is '.', it's already a real number
        let mut is_real = first == b'.';

        while let Some(ch) = self.peek() {
            match ch {
                b'0'..=b'9' => {
                    self.pos += 1;
                    self.lexbuf.scratch.push(ch);
                }
                b'.' => {
                    if is_real {
                        break; // Second dot, stop
                    }
                    is_real = true;
                    self.pos += 1;
                    self.lexbuf.scratch.push(ch);
                }
                b'+' | b'-' => {
                    // Sign only valid at start (but we already have first char)
                    break;
                }
                _ => break,
            }
        }

        let num_str = String::from_utf8_lossy(&self.lexbuf.scratch).to_string();

        if is_real {
            self.lexbuf.f = num_str.parse().unwrap_or(0.0);
            PdfToken::Real
        } else {
            self.lexbuf.i = num_str.parse().unwrap_or(0);
            PdfToken::Int
        }
    }

    /// Read a literal string (...)
    fn read_string(&mut self) -> PdfToken {
        self.lexbuf.scratch.clear();
        let mut depth = 1;

        while depth > 0 {
            let Some(ch) = self.next() else {
                self.error = Some("Unterminated string".to_string());
                return PdfToken::Error;
            };

            match ch {
                b'(' => {
                    depth += 1;
                    self.lexbuf.scratch.push(ch);
                }
                b')' => {
                    depth -= 1;
                    if depth > 0 {
                        self.lexbuf.scratch.push(ch);
                    }
                }
                b'\\' => {
                    // Escape sequence
                    let Some(esc) = self.next() else {
                        self.error = Some("Unterminated escape".to_string());
                        return PdfToken::Error;
                    };

                    match esc {
                        b'n' => self.lexbuf.scratch.push(b'\n'),
                        b'r' => self.lexbuf.scratch.push(b'\r'),
                        b't' => self.lexbuf.scratch.push(b'\t'),
                        b'b' => self.lexbuf.scratch.push(0x08),
                        b'f' => self.lexbuf.scratch.push(0x0c),
                        b'(' => self.lexbuf.scratch.push(b'('),
                        b')' => self.lexbuf.scratch.push(b')'),
                        b'\\' => self.lexbuf.scratch.push(b'\\'),
                        b'\r' => {
                            // Line continuation
                            if self.peek() == Some(b'\n') {
                                self.pos += 1;
                            }
                        }
                        b'\n' => {
                            // Line continuation
                        }
                        b'0'..=b'7' => {
                            // Octal escape
                            let mut octal = (esc - b'0') as u8;
                            for _ in 0..2 {
                                if let Some(d) = self.peek() {
                                    if (b'0'..=b'7').contains(&d) {
                                        self.pos += 1;
                                        octal = octal * 8 + (d - b'0');
                                    } else {
                                        break;
                                    }
                                }
                            }
                            self.lexbuf.scratch.push(octal);
                        }
                        _ => self.lexbuf.scratch.push(esc),
                    }
                }
                _ => self.lexbuf.scratch.push(ch),
            }
        }

        self.lexbuf.len = self.lexbuf.scratch.len();
        self.lexbuf.string_value = self.lexbuf.get_scratch_string();
        PdfToken::String
    }

    /// Read a hex string <...>
    fn read_hex_string(&mut self) -> PdfToken {
        self.lexbuf.scratch.clear();
        let mut high_nibble = true;
        let mut byte: u8 = 0;

        loop {
            let Some(ch) = self.next() else {
                self.error = Some("Unterminated hex string".to_string());
                return PdfToken::Error;
            };

            if ch == b'>' {
                if !high_nibble {
                    // Final byte with implicit 0
                    self.lexbuf.scratch.push(byte);
                }
                break;
            }

            // Skip whitespace in hex string
            if Self::is_whitespace(ch) {
                continue;
            }

            if !ch.is_ascii_hexdigit() {
                self.error = Some(format!("Invalid hex digit: {}", ch as char));
                return PdfToken::Error;
            }

            let nibble = Self::hex_digit(ch);

            if high_nibble {
                byte = nibble << 4;
                high_nibble = false;
            } else {
                byte |= nibble;
                self.lexbuf.scratch.push(byte);
                high_nibble = true;
            }
        }

        self.lexbuf.len = self.lexbuf.scratch.len();
        self.lexbuf.string_value = self.lexbuf.get_scratch_string();
        PdfToken::String
    }

    /// Read a keyword
    fn read_keyword(&mut self, first: u8) -> PdfToken {
        self.lexbuf.scratch.clear();
        self.lexbuf.scratch.push(first);

        while let Some(ch) = self.peek() {
            if Self::is_whitespace(ch) || Self::is_delimiter(ch) {
                break;
            }
            self.pos += 1;
            self.lexbuf.scratch.push(ch);
        }

        let keyword = String::from_utf8_lossy(&self.lexbuf.scratch).to_string();
        self.lexbuf.string_value = keyword.clone();

        // Check for special keywords
        match keyword.as_str() {
            "true" => PdfToken::True,
            "false" => PdfToken::False,
            "null" => PdfToken::Null,
            "obj" => PdfToken::Obj,
            "endobj" => PdfToken::EndObj,
            "stream" => PdfToken::Stream,
            "endstream" => PdfToken::EndStream,
            "xref" => PdfToken::Xref,
            "trailer" => PdfToken::Trailer,
            "startxref" => PdfToken::StartXref,
            "R" => PdfToken::R,
            _ => PdfToken::Keyword,
        }
    }

    /// Get the next token
    pub fn lex(&mut self) -> PdfToken {
        self.skip_whitespace();
        self.lexbuf.reset();

        let Some(ch) = self.next() else {
            self.lexbuf.token = PdfToken::Eof;
            return PdfToken::Eof;
        };

        let token = match ch {
            b'[' => PdfToken::OpenArray,
            b']' => PdfToken::CloseArray,
            b'{' => PdfToken::OpenBrace,
            b'}' => PdfToken::CloseBrace,
            b'/' => self.read_name(),
            b'(' => self.read_string(),
            b'<' => {
                // Could be << or hex string
                if self.peek() == Some(b'<') {
                    self.pos += 1;
                    PdfToken::OpenDict
                } else {
                    self.read_hex_string()
                }
            }
            b'>' => {
                if self.peek() == Some(b'>') {
                    self.pos += 1;
                    PdfToken::CloseDict
                } else {
                    self.error = Some("Unexpected >".to_string());
                    PdfToken::Error
                }
            }
            b'+' | b'-' | b'.' | b'0'..=b'9' => self.read_number(ch),
            _ if ch.is_ascii_alphabetic() => self.read_keyword(ch),
            _ => {
                self.error = Some(format!("Unexpected character: {}", ch as char));
                PdfToken::Error
            }
        };

        self.lexbuf.token = token;
        token
    }

    /// Lex without processing string escapes (for faster scanning)
    pub fn lex_no_string(&mut self) -> PdfToken {
        // For simplicity, same as lex for now
        self.lex()
    }
}

// ============================================================================
// Global Handle Stores
// ============================================================================

pub static LEXBUFS: LazyLock<HandleStore<PdfLexbuf>> = LazyLock::new(HandleStore::new);
pub static PARSERS: LazyLock<HandleStore<PdfParser>> = LazyLock::new(HandleStore::new);

// ============================================================================
// FFI Functions - Lexer Buffer
// ============================================================================

/// Initialize a lexer buffer
#[unsafe(no_mangle)]
pub extern "C" fn pdf_lexbuf_init(_ctx: Handle, size: i32) -> Handle {
    let lexbuf = PdfLexbuf::new(size.max(PDF_LEXBUF_SMALL as i32) as usize);
    LEXBUFS.insert(lexbuf)
}

/// Finalize (drop) a lexer buffer
#[unsafe(no_mangle)]
pub extern "C" fn pdf_lexbuf_fin(_ctx: Handle, lexbuf: Handle) {
    LEXBUFS.remove(lexbuf);
}

/// Grow the lexer buffer
#[unsafe(no_mangle)]
pub extern "C" fn pdf_lexbuf_grow(_ctx: Handle, lexbuf: Handle) -> isize {
    if let Some(lexbuf_arc) = LEXBUFS.get(lexbuf) {
        let mut lexbuf_guard = lexbuf_arc.lock().unwrap();
        return lexbuf_guard.grow();
    }
    0
}

/// Get the integer value from lexer buffer
#[unsafe(no_mangle)]
pub extern "C" fn pdf_lexbuf_get_int(_ctx: Handle, lexbuf: Handle) -> i64 {
    if let Some(lexbuf_arc) = LEXBUFS.get(lexbuf) {
        let lexbuf_guard = lexbuf_arc.lock().unwrap();
        return lexbuf_guard.i;
    }
    0
}

/// Get the float value from lexer buffer
#[unsafe(no_mangle)]
pub extern "C" fn pdf_lexbuf_get_float(_ctx: Handle, lexbuf: Handle) -> f32 {
    if let Some(lexbuf_arc) = LEXBUFS.get(lexbuf) {
        let lexbuf_guard = lexbuf_arc.lock().unwrap();
        return lexbuf_guard.f;
    }
    0.0
}

/// Get the string length from lexer buffer
#[unsafe(no_mangle)]
pub extern "C" fn pdf_lexbuf_get_len(_ctx: Handle, lexbuf: Handle) -> usize {
    if let Some(lexbuf_arc) = LEXBUFS.get(lexbuf) {
        let lexbuf_guard = lexbuf_arc.lock().unwrap();
        return lexbuf_guard.len;
    }
    0
}

/// Get the string value from lexer buffer (returns pointer to internal buffer)
#[unsafe(no_mangle)]
pub extern "C" fn pdf_lexbuf_get_string(_ctx: Handle, lexbuf: Handle) -> *const c_char {
    if let Some(lexbuf_arc) = LEXBUFS.get(lexbuf) {
        let lexbuf_guard = lexbuf_arc.lock().unwrap();
        if let Ok(cstr) = CString::new(lexbuf_guard.string_value.clone()) {
            return cstr.into_raw();
        }
    }
    ptr::null()
}

/// Free a string returned by pdf_lexbuf_get_string
#[unsafe(no_mangle)]
pub extern "C" fn pdf_lexbuf_free_string(_ctx: Handle, s: *mut c_char) {
    if !s.is_null() {
        unsafe {
            drop(CString::from_raw(s));
        }
    }
}

// ============================================================================
// FFI Functions - Lexer Operations
// ============================================================================

/// Create a new parser from data
#[unsafe(no_mangle)]
pub extern "C" fn pdf_parser_new(_ctx: Handle, data: *const u8, len: usize) -> Handle {
    if data.is_null() || len == 0 {
        return 0;
    }

    let data_vec = unsafe { std::slice::from_raw_parts(data, len).to_vec() };
    let parser = PdfParser::new(data_vec);
    PARSERS.insert(parser)
}

/// Drop a parser
#[unsafe(no_mangle)]
pub extern "C" fn pdf_parser_drop(_ctx: Handle, parser: Handle) {
    PARSERS.remove(parser);
}

/// Lex the next token
#[unsafe(no_mangle)]
pub extern "C" fn pdf_lex(_ctx: Handle, parser: Handle) -> i32 {
    if let Some(parser_arc) = PARSERS.get(parser) {
        let mut parser_guard = parser_arc.lock().unwrap();
        return parser_guard.lex() as i32;
    }
    PdfToken::Error as i32
}

/// Lex without processing string escapes
#[unsafe(no_mangle)]
pub extern "C" fn pdf_lex_no_string(_ctx: Handle, parser: Handle) -> i32 {
    if let Some(parser_arc) = PARSERS.get(parser) {
        let mut parser_guard = parser_arc.lock().unwrap();
        return parser_guard.lex_no_string() as i32;
    }
    PdfToken::Error as i32
}

/// Get the current token type
#[unsafe(no_mangle)]
pub extern "C" fn pdf_parser_get_token(_ctx: Handle, parser: Handle) -> i32 {
    if let Some(parser_arc) = PARSERS.get(parser) {
        let parser_guard = parser_arc.lock().unwrap();
        return parser_guard.lexbuf.token as i32;
    }
    PdfToken::Error as i32
}

/// Get the integer value from parser
#[unsafe(no_mangle)]
pub extern "C" fn pdf_parser_get_int(_ctx: Handle, parser: Handle) -> i64 {
    if let Some(parser_arc) = PARSERS.get(parser) {
        let parser_guard = parser_arc.lock().unwrap();
        return parser_guard.lexbuf.i;
    }
    0
}

/// Get the float value from parser
#[unsafe(no_mangle)]
pub extern "C" fn pdf_parser_get_float(_ctx: Handle, parser: Handle) -> f32 {
    if let Some(parser_arc) = PARSERS.get(parser) {
        let parser_guard = parser_arc.lock().unwrap();
        return parser_guard.lexbuf.f;
    }
    0.0
}

/// Get the string value from parser
#[unsafe(no_mangle)]
pub extern "C" fn pdf_parser_get_string(_ctx: Handle, parser: Handle) -> *const c_char {
    if let Some(parser_arc) = PARSERS.get(parser) {
        let parser_guard = parser_arc.lock().unwrap();
        if let Ok(cstr) = CString::new(parser_guard.lexbuf.string_value.clone()) {
            return cstr.into_raw();
        }
    }
    ptr::null()
}

/// Get the current position in the input
#[unsafe(no_mangle)]
pub extern "C" fn pdf_parser_get_pos(_ctx: Handle, parser: Handle) -> usize {
    if let Some(parser_arc) = PARSERS.get(parser) {
        let parser_guard = parser_arc.lock().unwrap();
        return parser_guard.pos;
    }
    0
}

/// Set the current position in the input
#[unsafe(no_mangle)]
pub extern "C" fn pdf_parser_set_pos(_ctx: Handle, parser: Handle, pos: usize) {
    if let Some(parser_arc) = PARSERS.get(parser) {
        let mut parser_guard = parser_arc.lock().unwrap();
        parser_guard.pos = pos.min(parser_guard.data.len());
    }
}

/// Check if parser has error
#[unsafe(no_mangle)]
pub extern "C" fn pdf_parser_has_error(_ctx: Handle, parser: Handle) -> i32 {
    if let Some(parser_arc) = PARSERS.get(parser) {
        let parser_guard = parser_arc.lock().unwrap();
        return if parser_guard.error.is_some() { 1 } else { 0 };
    }
    0
}

/// Get parser error message
#[unsafe(no_mangle)]
pub extern "C" fn pdf_parser_get_error(_ctx: Handle, parser: Handle) -> *const c_char {
    if let Some(parser_arc) = PARSERS.get(parser) {
        let parser_guard = parser_arc.lock().unwrap();
        if let Some(ref error) = parser_guard.error {
            if let Ok(cstr) = CString::new(error.clone()) {
                return cstr.into_raw();
            }
        }
    }
    ptr::null()
}

// ============================================================================
// FFI Functions - Object Parsing
// ============================================================================

/// Parse a PDF array [...] and return a handle to a parsed object
#[unsafe(no_mangle)]
pub extern "C" fn pdf_parse_array(_ctx: Handle, _doc: Handle, parser: Handle) -> Handle {
    if let Some(parser_arc) = PARSERS.get(parser) {
        let mut parser_guard = parser_arc.lock().unwrap();

        // Expect we're already positioned after [
        let mut elements: Vec<ParsedValue> = Vec::new();

        loop {
            let token = parser_guard.lex();

            match token {
                PdfToken::CloseArray => break,
                PdfToken::Eof => {
                    parser_guard.error = Some("Unexpected EOF in array".to_string());
                    return 0;
                }
                PdfToken::Error => return 0,
                _ => {
                    if let Some(value) = token_to_value(&parser_guard.lexbuf, token) {
                        elements.push(value);
                    }
                }
            }
        }

        let parsed = ParsedObject::Array(elements);
        return PARSED_OBJECTS.insert(parsed);
    }
    0
}

/// Parse a PDF dictionary <<...>> and return a handle
#[unsafe(no_mangle)]
pub extern "C" fn pdf_parse_dict(_ctx: Handle, _doc: Handle, parser: Handle) -> Handle {
    let Some(parser_arc) = PARSERS.get(parser) else {
        return 0;
    };

    let mut entries: Vec<(String, ParsedValue)> = Vec::new();

    loop {
        let token = {
            let mut parser_guard = parser_arc.lock().unwrap();
            parser_guard.lex()
        };

        match token {
            PdfToken::CloseDict => break,
            PdfToken::Eof => {
                let mut parser_guard = parser_arc.lock().unwrap();
                parser_guard.error = Some("Unexpected EOF in dict".to_string());
                return 0;
            }
            PdfToken::Error => return 0,
            PdfToken::Name => {
                let key = {
                    let parser_guard = parser_arc.lock().unwrap();
                    parser_guard.lexbuf.string_value.clone()
                };

                // Get the value
                let value_token = {
                    let mut parser_guard = parser_arc.lock().unwrap();
                    parser_guard.lex()
                };

                // Check for indirect reference (num gen R)
                if value_token == PdfToken::Int {
                    let (num, save_pos) = {
                        let parser_guard = parser_arc.lock().unwrap();
                        (parser_guard.lexbuf.i as i32, parser_guard.pos)
                    };

                    // Peek ahead to see if this is a reference
                    let next_token = {
                        let mut parser_guard = parser_arc.lock().unwrap();
                        parser_guard.lex()
                    };

                    if next_token == PdfToken::Int {
                        let gen_num = {
                            let parser_guard = parser_arc.lock().unwrap();
                            parser_guard.lexbuf.i as i32
                        };

                        let r_token = {
                            let mut parser_guard = parser_arc.lock().unwrap();
                            parser_guard.lex()
                        };

                        if r_token == PdfToken::R {
                            // It's a reference
                            entries.push((
                                key,
                                ParsedValue::Reference {
                                    num,
                                    generation: gen_num,
                                },
                            ));
                            continue;
                        }
                    }
                    // Not a reference, restore position and use as int
                    {
                        let mut parser_guard = parser_arc.lock().unwrap();
                        parser_guard.pos = save_pos;
                    }
                    entries.push((key, ParsedValue::Int(num as i64)));
                    continue;
                }

                let lexbuf_snapshot = {
                    let parser_guard = parser_arc.lock().unwrap();
                    parser_guard.lexbuf.clone()
                };

                if let Some(value) = token_to_value(&lexbuf_snapshot, value_token) {
                    entries.push((key, value));
                } else if value_token == PdfToken::OpenArray {
                    // Parse nested array recursively
                    let arr_handle = pdf_parse_array(_ctx, _doc, parser);
                    if arr_handle == 0 {
                        return 0;
                    }
                    // Store as array placeholder
                    entries.push((key, ParsedValue::Array(Vec::new())));
                    PARSED_OBJECTS.remove(arr_handle);
                } else if value_token == PdfToken::OpenDict {
                    // Parse nested dict recursively
                    let dict_handle = pdf_parse_dict(_ctx, _doc, parser);
                    if dict_handle == 0 {
                        return 0;
                    }
                    entries.push((key, ParsedValue::Dict(Vec::new())));
                    PARSED_OBJECTS.remove(dict_handle);
                } else {
                    let mut parser_guard = parser_arc.lock().unwrap();
                    parser_guard.error = Some("Invalid value in dict".to_string());
                    return 0;
                }
            }
            _ => {
                let mut parser_guard = parser_arc.lock().unwrap();
                parser_guard.error = Some("Expected name in dict".to_string());
                return 0;
            }
        }
    }

    let parsed = ParsedObject::Dict(entries);
    PARSED_OBJECTS.insert(parsed)
}

/// Parse a stream object
#[unsafe(no_mangle)]
pub extern "C" fn pdf_parse_stm_obj(_ctx: Handle, _doc: Handle, parser: Handle) -> Handle {
    if let Some(parser_arc) = PARSERS.get(parser) {
        let mut parser_guard = parser_arc.lock().unwrap();

        let token = parser_guard.lex();
        if let Some(value) = token_to_value(&parser_guard.lexbuf, token) {
            let parsed = ParsedObject::Value(value);
            return PARSED_OBJECTS.insert(parsed);
        }
    }
    0
}

/// Parse an indirect object (num gen obj ... endobj)
#[unsafe(no_mangle)]
pub extern "C" fn pdf_parse_ind_obj(
    _ctx: Handle,
    _doc: Handle,
    parser: Handle,
    num: *mut i32,
    generation: *mut i32,
    stm_ofs: *mut i64,
    _try_repair: *mut i32,
) -> Handle {
    if let Some(parser_arc) = PARSERS.get(parser) {
        let mut parser_guard = parser_arc.lock().unwrap();

        // Parse object number
        let token1 = parser_guard.lex();
        if token1 != PdfToken::Int {
            parser_guard.error = Some("Expected object number".to_string());
            return 0;
        }
        let obj_num = parser_guard.lexbuf.i as i32;

        // Parse generation number
        let token2 = parser_guard.lex();
        if token2 != PdfToken::Int {
            parser_guard.error = Some("Expected generation number".to_string());
            return 0;
        }
        let obj_gen = parser_guard.lexbuf.i as i32;

        // Parse 'obj' keyword
        let token3 = parser_guard.lex();
        if token3 != PdfToken::Obj {
            parser_guard.error = Some("Expected 'obj' keyword".to_string());
            return 0;
        }

        unsafe {
            if !num.is_null() {
                *num = obj_num;
            }
            if !generation.is_null() {
                *generation = obj_gen;
            }
        }

        // Parse the object value
        let value_token = parser_guard.lex();
        let value = match value_token {
            PdfToken::OpenArray => {
                drop(parser_guard);
                return pdf_parse_array(_ctx, _doc, parser);
            }
            PdfToken::OpenDict => {
                // Check if this is a stream
                let dict_handle = {
                    drop(parser_guard);
                    pdf_parse_dict(_ctx, _doc, parser)
                };

                if dict_handle == 0 {
                    return 0;
                }

                // Re-acquire lock to check for stream
                let parser_arc2 = PARSERS.get(parser).unwrap();
                let mut parser_guard2 = parser_arc2.lock().unwrap();

                let next_token = parser_guard2.lex();
                if next_token == PdfToken::Stream {
                    unsafe {
                        if !stm_ofs.is_null() {
                            *stm_ofs = parser_guard2.pos as i64;
                        }
                    }
                }

                return dict_handle;
            }
            _ => token_to_value(&parser_guard.lexbuf, value_token),
        };

        if let Some(v) = value {
            let parsed = ParsedObject::IndirectObject {
                num: obj_num,
                generation: obj_gen,
                value: Box::new(v),
            };
            return PARSED_OBJECTS.insert(parsed);
        }
    }
    0
}

// ============================================================================
// Parsed Value Types
// ============================================================================

/// Parsed value types
#[derive(Debug, Clone)]
pub enum ParsedValue {
    Null,
    Bool(bool),
    Int(i64),
    Real(f32),
    String(Vec<u8>),
    Name(String),
    Array(Vec<ParsedValue>),
    Dict(Vec<(String, ParsedValue)>),
    Reference { num: i32, generation: i32 },
}

/// Parsed object wrapper
#[derive(Debug)]
pub enum ParsedObject {
    Value(ParsedValue),
    Array(Vec<ParsedValue>),
    Dict(Vec<(String, ParsedValue)>),
    IndirectObject {
        num: i32,
        generation: i32,
        value: Box<ParsedValue>,
    },
}

pub static PARSED_OBJECTS: LazyLock<HandleStore<ParsedObject>> = LazyLock::new(HandleStore::new);

/// Convert a token to a parsed value
fn token_to_value(lexbuf: &PdfLexbuf, token: PdfToken) -> Option<ParsedValue> {
    match token {
        PdfToken::Null => Some(ParsedValue::Null),
        PdfToken::True => Some(ParsedValue::Bool(true)),
        PdfToken::False => Some(ParsedValue::Bool(false)),
        PdfToken::Int => Some(ParsedValue::Int(lexbuf.i)),
        PdfToken::Real => Some(ParsedValue::Real(lexbuf.f)),
        PdfToken::String => Some(ParsedValue::String(lexbuf.scratch.clone())),
        PdfToken::Name => Some(ParsedValue::Name(lexbuf.string_value.clone())),
        _ => None,
    }
}

// ============================================================================
// FFI Functions - Parsed Object Access
// ============================================================================

/// Drop a parsed object
#[unsafe(no_mangle)]
pub extern "C" fn pdf_parsed_obj_drop(_ctx: Handle, obj: Handle) {
    PARSED_OBJECTS.remove(obj);
}

/// Get the type of a parsed object
#[unsafe(no_mangle)]
pub extern "C" fn pdf_parsed_obj_type(_ctx: Handle, obj: Handle) -> i32 {
    if let Some(obj_arc) = PARSED_OBJECTS.get(obj) {
        let obj_guard = obj_arc.lock().unwrap();
        return match &*obj_guard {
            ParsedObject::Value(ParsedValue::Null) => 0,
            ParsedObject::Value(ParsedValue::Bool(_)) => 1,
            ParsedObject::Value(ParsedValue::Int(_)) => 2,
            ParsedObject::Value(ParsedValue::Real(_)) => 3,
            ParsedObject::Value(ParsedValue::String(_)) => 4,
            ParsedObject::Value(ParsedValue::Name(_)) => 5,
            ParsedObject::Value(ParsedValue::Array(_)) | ParsedObject::Array(_) => 6,
            ParsedObject::Value(ParsedValue::Dict(_)) | ParsedObject::Dict(_) => 7,
            ParsedObject::Value(ParsedValue::Reference { .. }) => 8,
            ParsedObject::IndirectObject { .. } => 9,
        };
    }
    -1
}

/// Get array length
#[unsafe(no_mangle)]
pub extern "C" fn pdf_parsed_array_len(_ctx: Handle, obj: Handle) -> i32 {
    if let Some(obj_arc) = PARSED_OBJECTS.get(obj) {
        let obj_guard = obj_arc.lock().unwrap();
        return match &*obj_guard {
            ParsedObject::Array(arr) => arr.len() as i32,
            ParsedObject::Value(ParsedValue::Array(arr)) => arr.len() as i32,
            _ => 0,
        };
    }
    0
}

/// Get dict length (number of key-value pairs)
#[unsafe(no_mangle)]
pub extern "C" fn pdf_parsed_dict_len(_ctx: Handle, obj: Handle) -> i32 {
    if let Some(obj_arc) = PARSED_OBJECTS.get(obj) {
        let obj_guard = obj_arc.lock().unwrap();
        return match &*obj_guard {
            ParsedObject::Dict(entries) => entries.len() as i32,
            ParsedObject::Value(ParsedValue::Dict(entries)) => entries.len() as i32,
            _ => 0,
        };
    }
    0
}

// ============================================================================
// FFI Functions - Token Utilities
// ============================================================================

/// Append a token representation to a buffer
#[unsafe(no_mangle)]
pub extern "C" fn pdf_append_token(_ctx: Handle, buf: Handle, tok: i32, lexbuf: Handle) {
    let token = PdfToken::from_i32(tok);
    let token_str = token.to_string();

    // Get the lexbuf for string values
    let value_str = if let Some(lexbuf_arc) = LEXBUFS.get(lexbuf) {
        let lexbuf_guard = lexbuf_arc.lock().unwrap();
        match token {
            PdfToken::Name => format!("/{}", lexbuf_guard.string_value),
            PdfToken::Int => format!("{}", lexbuf_guard.i),
            PdfToken::Real => format!("{}", lexbuf_guard.f),
            PdfToken::String => format!("({})", lexbuf_guard.string_value),
            PdfToken::Keyword => lexbuf_guard.string_value.clone(),
            _ => token_str.to_string(),
        }
    } else {
        token_str.to_string()
    };

    // Append to buffer if available
    if let Some(buf_arc) = crate::ffi::BUFFERS.get(buf) {
        let mut buf_guard = buf_arc.lock().unwrap();
        buf_guard.append(value_str.as_bytes());
    }
}

/// Get token name string
#[unsafe(no_mangle)]
pub extern "C" fn pdf_token_name(tok: i32) -> *const c_char {
    let token = PdfToken::from_i32(tok);
    match token {
        PdfToken::Error => c"error".as_ptr(),
        PdfToken::Eof => c"EOF".as_ptr(),
        PdfToken::OpenArray => c"[".as_ptr(),
        PdfToken::CloseArray => c"]".as_ptr(),
        PdfToken::OpenDict => c"<<".as_ptr(),
        PdfToken::CloseDict => c">>".as_ptr(),
        PdfToken::OpenBrace => c"{".as_ptr(),
        PdfToken::CloseBrace => c"}".as_ptr(),
        PdfToken::Name => c"name".as_ptr(),
        PdfToken::Int => c"integer".as_ptr(),
        PdfToken::Real => c"real".as_ptr(),
        PdfToken::String => c"string".as_ptr(),
        PdfToken::Keyword => c"keyword".as_ptr(),
        PdfToken::R => c"R".as_ptr(),
        PdfToken::True => c"true".as_ptr(),
        PdfToken::False => c"false".as_ptr(),
        PdfToken::Null => c"null".as_ptr(),
        PdfToken::Obj => c"obj".as_ptr(),
        PdfToken::EndObj => c"endobj".as_ptr(),
        PdfToken::Stream => c"stream".as_ptr(),
        PdfToken::EndStream => c"endstream".as_ptr(),
        PdfToken::Xref => c"xref".as_ptr(),
        PdfToken::Trailer => c"trailer".as_ptr(),
        PdfToken::StartXref => c"startxref".as_ptr(),
        PdfToken::NewObj => c"newobj".as_ptr(),
        PdfToken::NumTokens => c"numtokens".as_ptr(),
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lexbuf_creation() {
        let lexbuf = PdfLexbuf::new(256);
        assert_eq!(lexbuf.size, 256);
        assert_eq!(lexbuf.base_size, 256);
    }

    #[test]
    fn test_lexbuf_grow() {
        let mut lexbuf = PdfLexbuf::new(256);
        let growth = lexbuf.grow();
        assert!(growth > 0);
        assert!(lexbuf.size > 256);
    }

    #[test]
    fn test_lex_integers() {
        let data = b"123 -456 +789 0".to_vec();
        let mut parser = PdfParser::new(data);

        assert_eq!(parser.lex(), PdfToken::Int);
        assert_eq!(parser.lexbuf.i, 123);

        assert_eq!(parser.lex(), PdfToken::Int);
        assert_eq!(parser.lexbuf.i, -456);

        assert_eq!(parser.lex(), PdfToken::Int);
        assert_eq!(parser.lexbuf.i, 789);

        assert_eq!(parser.lex(), PdfToken::Int);
        assert_eq!(parser.lexbuf.i, 0);
    }

    #[test]
    fn test_lex_reals() {
        let data = b"1.23 -4.56 .789 0.0".to_vec();
        let mut parser = PdfParser::new(data);

        assert_eq!(parser.lex(), PdfToken::Real);
        assert!((parser.lexbuf.f - 1.23).abs() < 0.001);

        assert_eq!(parser.lex(), PdfToken::Real);
        assert!((parser.lexbuf.f - (-4.56)).abs() < 0.001);

        assert_eq!(parser.lex(), PdfToken::Real);
        assert!((parser.lexbuf.f - 0.789).abs() < 0.001);

        assert_eq!(parser.lex(), PdfToken::Real);
        assert!((parser.lexbuf.f - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_lex_names() {
        let data = b"/Name /Name#20With#20Spaces /Type".to_vec();
        let mut parser = PdfParser::new(data);

        assert_eq!(parser.lex(), PdfToken::Name);
        assert_eq!(parser.lexbuf.string_value, "Name");

        assert_eq!(parser.lex(), PdfToken::Name);
        assert_eq!(parser.lexbuf.string_value, "Name With Spaces");

        assert_eq!(parser.lex(), PdfToken::Name);
        assert_eq!(parser.lexbuf.string_value, "Type");
    }

    #[test]
    fn test_lex_strings() {
        let data = b"(Hello World) (Nested (parens)) (Escape \\n\\t)".to_vec();
        let mut parser = PdfParser::new(data);

        assert_eq!(parser.lex(), PdfToken::String);
        assert_eq!(parser.lexbuf.string_value, "Hello World");

        assert_eq!(parser.lex(), PdfToken::String);
        assert_eq!(parser.lexbuf.string_value, "Nested (parens)");

        assert_eq!(parser.lex(), PdfToken::String);
        assert_eq!(parser.lexbuf.string_value, "Escape \n\t");
    }

    #[test]
    fn test_lex_hex_strings() {
        let data = b"<48656C6C6F> <4865 6C6C 6F>".to_vec();
        let mut parser = PdfParser::new(data);

        assert_eq!(parser.lex(), PdfToken::String);
        assert_eq!(parser.lexbuf.string_value, "Hello");

        assert_eq!(parser.lex(), PdfToken::String);
        assert_eq!(parser.lexbuf.string_value, "Hello");
    }

    #[test]
    fn test_lex_keywords() {
        let data = b"true false null obj endobj stream endstream".to_vec();
        let mut parser = PdfParser::new(data);

        assert_eq!(parser.lex(), PdfToken::True);
        assert_eq!(parser.lex(), PdfToken::False);
        assert_eq!(parser.lex(), PdfToken::Null);
        assert_eq!(parser.lex(), PdfToken::Obj);
        assert_eq!(parser.lex(), PdfToken::EndObj);
        assert_eq!(parser.lex(), PdfToken::Stream);
        assert_eq!(parser.lex(), PdfToken::EndStream);
    }

    #[test]
    fn test_lex_delimiters() {
        let data = b"[ ] << >> { }".to_vec();
        let mut parser = PdfParser::new(data);

        assert_eq!(parser.lex(), PdfToken::OpenArray);
        assert_eq!(parser.lex(), PdfToken::CloseArray);
        assert_eq!(parser.lex(), PdfToken::OpenDict);
        assert_eq!(parser.lex(), PdfToken::CloseDict);
        assert_eq!(parser.lex(), PdfToken::OpenBrace);
        assert_eq!(parser.lex(), PdfToken::CloseBrace);
    }

    #[test]
    fn test_skip_comments() {
        let data = b"123 % this is a comment\n456".to_vec();
        let mut parser = PdfParser::new(data);

        assert_eq!(parser.lex(), PdfToken::Int);
        assert_eq!(parser.lexbuf.i, 123);

        assert_eq!(parser.lex(), PdfToken::Int);
        assert_eq!(parser.lexbuf.i, 456);
    }

    #[test]
    fn test_parse_array() {
        let ctx = 1;
        let data = b"[1 2 3 /Name (string)]".to_vec();

        let parser = pdf_parser_new(ctx, data.as_ptr(), data.len());
        assert!(parser > 0);

        // Lex the opening bracket
        let tok = pdf_lex(ctx, parser);
        assert_eq!(tok, PdfToken::OpenArray as i32);

        // Parse the array
        let arr = pdf_parse_array(ctx, 0, parser);
        assert!(arr > 0);

        // Check array length
        assert_eq!(pdf_parsed_array_len(ctx, arr), 5);

        pdf_parsed_obj_drop(ctx, arr);
        pdf_parser_drop(ctx, parser);
    }

    #[test]
    fn test_parse_dict() {
        let ctx = 1;
        let data = b"<</Type /Catalog /Pages 1 0 R>>".to_vec();

        let parser = pdf_parser_new(ctx, data.as_ptr(), data.len());
        assert!(parser > 0);

        // Lex the opening dict
        let tok = pdf_lex(ctx, parser);
        assert_eq!(tok, PdfToken::OpenDict as i32);

        // Parse the dict
        let dict = pdf_parse_dict(ctx, 0, parser);
        assert!(dict > 0);

        // Check dict length (2 key-value pairs)
        assert_eq!(pdf_parsed_dict_len(ctx, dict), 2);

        pdf_parsed_obj_drop(ctx, dict);
        pdf_parser_drop(ctx, parser);
    }

    #[test]
    fn test_parse_indirect_obj() {
        let ctx = 1;
        let data = b"1 0 obj\n<</Type /Catalog>>\nendobj".to_vec();

        let parser = pdf_parser_new(ctx, data.as_ptr(), data.len());
        assert!(parser > 0);

        let mut num: i32 = 0;
        let mut generation: i32 = 0;
        let mut stm_ofs: i64 = 0;

        let obj = pdf_parse_ind_obj(
            ctx,
            0,
            parser,
            &mut num,
            &mut generation,
            &mut stm_ofs,
            ptr::null_mut(),
        );

        assert!(obj > 0);
        assert_eq!(num, 1);
        assert_eq!(generation, 0);

        pdf_parsed_obj_drop(ctx, obj);
        pdf_parser_drop(ctx, parser);
    }

    #[test]
    fn test_token_conversion() {
        assert_eq!(PdfToken::from_i32(0), PdfToken::Error);
        assert_eq!(PdfToken::from_i32(1), PdfToken::Eof);
        assert_eq!(PdfToken::from_i32(14), PdfToken::True);
        assert_eq!(PdfToken::from_i32(99), PdfToken::Error);
    }

    #[test]
    fn test_ffi_lexbuf() {
        let ctx = 1;

        let lexbuf = pdf_lexbuf_init(ctx, 512);
        assert!(lexbuf > 0);

        let growth = pdf_lexbuf_grow(ctx, lexbuf);
        assert!(growth > 0);

        pdf_lexbuf_fin(ctx, lexbuf);
    }

    #[test]
    fn test_octal_escape() {
        let data = b"(\\101\\102\\103)".to_vec(); // ABC
        let mut parser = PdfParser::new(data);

        assert_eq!(parser.lex(), PdfToken::String);
        assert_eq!(parser.lexbuf.string_value, "ABC");
    }

    #[test]
    fn test_reference() {
        let data = b"1 0 R".to_vec();
        let mut parser = PdfParser::new(data);

        assert_eq!(parser.lex(), PdfToken::Int);
        assert_eq!(parser.lexbuf.i, 1);

        assert_eq!(parser.lex(), PdfToken::Int);
        assert_eq!(parser.lexbuf.i, 0);

        assert_eq!(parser.lex(), PdfToken::R);
    }
}
