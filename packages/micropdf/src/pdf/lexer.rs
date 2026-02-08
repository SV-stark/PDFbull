//! PDF tokenizer - Lexical analysis of PDF content
//!
//! Tokenizes PDF streams into meaningful tokens for parsing.

use crate::fitz::error::{Error, Result};

/// PDF token types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Token {
    /// Error token
    Error,
    /// End of file
    Eof,
    /// '[' - Start of array
    OpenArray,
    /// ']' - End of array
    CloseArray,
    /// '<<' - Start of dictionary
    OpenDict,
    /// '>>' - End of dictionary
    CloseDict,
    /// '{' - Open brace (for inline images)
    OpenBrace,
    /// '}' - Close brace
    CloseBrace,
    /// Name (e.g., /Type)
    Name,
    /// Integer number
    Int,
    /// Real (floating point) number
    Real,
    /// String literal
    String,
    /// Keyword
    Keyword,
    /// 'R' - Reference keyword
    R,
    /// 'true' boolean
    True,
    /// 'false' boolean
    False,
    /// 'null' value
    Null,
    /// 'obj' keyword
    Obj,
    /// 'endobj' keyword
    EndObj,
    /// 'stream' keyword
    Stream,
    /// 'endstream' keyword
    EndStream,
    /// 'xref' keyword
    Xref,
    /// 'trailer' keyword
    Trailer,
    /// 'startxref' keyword
    StartXref,
    /// 'newobj' keyword (for incremental updates)
    NewObj,
}

/// Lexer buffer for storing tokenized data
#[derive(Debug)]
pub struct LexBuf {
    /// String buffer for name/string tokens
    pub buffer: String,
    /// Integer value for int tokens
    pub int_value: i64,
    /// Float value for real tokens
    pub float_value: f64,
}

impl LexBuf {
    /// Create a new lexer buffer
    pub fn new() -> Self {
        Self::with_capacity(256)
    }

    /// Create a new lexer buffer with specific capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            buffer: String::with_capacity(capacity),
            int_value: 0,
            float_value: 0.0,
        }
    }

    /// Clear the buffer for reuse
    pub fn clear(&mut self) {
        self.buffer.clear();
        self.int_value = 0;
        self.float_value = 0.0;
    }

    /// Get the string value
    pub fn as_str(&self) -> &str {
        &self.buffer
    }

    /// Get the integer value
    pub fn as_int(&self) -> i64 {
        self.int_value
    }

    /// Get the float value
    pub fn as_float(&self) -> f64 {
        self.float_value
    }
}

impl Default for LexBuf {
    fn default() -> Self {
        Self::new()
    }
}

/// PDF lexer for tokenizing PDF streams
pub struct Lexer<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Lexer<'a> {
    /// Create a new lexer from a byte slice
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    /// Get the next token
    pub fn lex(&mut self, buf: &mut LexBuf) -> Result<Token> {
        buf.clear();
        self.skip_whitespace_and_comments();

        if self.is_eof() {
            return Ok(Token::Eof);
        }

        let ch = self.peek()?;

        match ch {
            b'[' => {
                self.advance();
                Ok(Token::OpenArray)
            }
            b']' => {
                self.advance();
                Ok(Token::CloseArray)
            }
            b'{' => {
                self.advance();
                Ok(Token::OpenBrace)
            }
            b'}' => {
                self.advance();
                Ok(Token::CloseBrace)
            }
            b'<' => {
                self.advance();
                if self.peek_eq(b'<') {
                    self.advance();
                    Ok(Token::OpenDict)
                } else {
                    // Hexadecimal string
                    self.lex_hex_string(buf)
                }
            }
            b'>' => {
                self.advance();
                if self.peek_eq(b'>') {
                    self.advance();
                    Ok(Token::CloseDict)
                } else {
                    Err(Error::Generic("Unexpected '>' character".into()))
                }
            }
            b'/' => {
                self.advance();
                self.lex_name(buf)
            }
            b'(' => {
                self.advance();
                self.lex_string(buf)
            }
            b'+' | b'-' | b'.' | b'0'..=b'9' => self.lex_number(buf),
            _ => self.lex_keyword(buf),
        }
    }

    fn skip_whitespace_and_comments(&mut self) {
        while !self.is_eof() {
            match self.data[self.pos] {
                b' ' | b'\t' | b'\r' | b'\n' | b'\x0C' => {
                    self.pos += 1;
                }
                b'%' => {
                    // Skip comment until end of line
                    self.pos += 1;
                    while !self.is_eof()
                        && self.data[self.pos] != b'\n'
                        && self.data[self.pos] != b'\r'
                    {
                        self.pos += 1;
                    }
                }
                _ => break,
            }
        }
    }

    fn lex_name(&mut self, buf: &mut LexBuf) -> Result<Token> {
        while !self.is_eof() {
            let ch = self.data[self.pos];
            if Self::is_delimiter(ch) || Self::is_whitespace(ch) {
                break;
            }
            if ch == b'#' && !self.is_eof_at(self.pos + 2) {
                // Hex escape sequence
                self.pos += 1;
                let hex_str = std::str::from_utf8(&self.data[self.pos..self.pos + 2])
                    .map_err(|_| Error::Generic("Invalid hex in name".into()))?;
                let byte = u8::from_str_radix(hex_str, 16)
                    .map_err(|_| Error::Generic("Invalid hex digits in name".into()))?;
                buf.buffer.push(byte as char);
                self.pos += 2;
            } else {
                buf.buffer.push(ch as char);
                self.pos += 1;
            }
        }
        Ok(Token::Name)
    }

    fn lex_string(&mut self, buf: &mut LexBuf) -> Result<Token> {
        let mut depth = 1;
        while !self.is_eof() && depth > 0 {
            let ch = self.data[self.pos];
            self.pos += 1;

            match ch {
                b'(' => {
                    depth += 1;
                    buf.buffer.push('(');
                }
                b')' => {
                    depth -= 1;
                    if depth > 0 {
                        buf.buffer.push(')');
                    }
                }
                b'\\' => {
                    if !self.is_eof() {
                        let next = self.data[self.pos];
                        self.pos += 1;
                        match next {
                            b'n' => buf.buffer.push('\n'),
                            b'r' => buf.buffer.push('\r'),
                            b't' => buf.buffer.push('\t'),
                            b'b' => buf.buffer.push('\x08'),
                            b'f' => buf.buffer.push('\x0C'),
                            b'(' => buf.buffer.push('('),
                            b')' => buf.buffer.push(')'),
                            b'\\' => buf.buffer.push('\\'),
                            b'0'..=b'7' => {
                                // Octal escape
                                let mut octal = next - b'0';
                                if !self.is_eof() && (b'0'..=b'7').contains(&self.data[self.pos]) {
                                    octal = octal * 8 + (self.data[self.pos] - b'0');
                                    self.pos += 1;
                                }
                                if !self.is_eof() && (b'0'..=b'7').contains(&self.data[self.pos]) {
                                    octal = octal * 8 + (self.data[self.pos] - b'0');
                                    self.pos += 1;
                                }
                                buf.buffer.push(octal as char);
                            }
                            b'\r' | b'\n' => {
                                // Line continuation - skip newline
                                if next == b'\r' && !self.is_eof() && self.data[self.pos] == b'\n' {
                                    self.pos += 1;
                                }
                            }
                            _ => buf.buffer.push(next as char),
                        }
                    }
                }
                _ => buf.buffer.push(ch as char),
            }
        }
        Ok(Token::String)
    }

    fn lex_hex_string(&mut self, buf: &mut LexBuf) -> Result<Token> {
        while !self.is_eof() {
            let ch = self.data[self.pos];
            if ch == b'>' {
                self.pos += 1;
                break;
            }
            if Self::is_whitespace(ch) {
                self.pos += 1;
                continue;
            }
            buf.buffer.push(ch as char);
            self.pos += 1;
        }
        Ok(Token::String)
    }

    fn lex_number(&mut self, buf: &mut LexBuf) -> Result<Token> {
        let _start = self.pos;
        let mut is_real = false;

        // Optional sign
        if self.peek_eq(b'+') || self.peek_eq(b'-') {
            buf.buffer.push(self.data[self.pos] as char);
            self.pos += 1;
        }

        // Digits before decimal point
        while !self.is_eof() && self.data[self.pos].is_ascii_digit() {
            buf.buffer.push(self.data[self.pos] as char);
            self.pos += 1;
        }

        // Decimal point
        if !self.is_eof() && self.data[self.pos] == b'.' {
            is_real = true;
            buf.buffer.push('.');
            self.pos += 1;

            // Digits after decimal point
            while !self.is_eof() && self.data[self.pos].is_ascii_digit() {
                buf.buffer.push(self.data[self.pos] as char);
                self.pos += 1;
            }
        }

        if buf.buffer.is_empty() || buf.buffer == "+" || buf.buffer == "-" {
            return Err(Error::Generic("Invalid number".into()));
        }

        if is_real {
            buf.float_value = buf
                .buffer
                .parse()
                .map_err(|_| Error::Generic("Invalid real number".into()))?;
            Ok(Token::Real)
        } else {
            buf.int_value = buf
                .buffer
                .parse()
                .map_err(|_| Error::Generic("Invalid integer".into()))?;
            Ok(Token::Int)
        }
    }

    fn lex_keyword(&mut self, buf: &mut LexBuf) -> Result<Token> {
        while !self.is_eof() {
            let ch = self.data[self.pos];
            if Self::is_delimiter(ch) || Self::is_whitespace(ch) {
                break;
            }
            buf.buffer.push(ch as char);
            self.pos += 1;
        }

        // Match known keywords
        match buf.buffer.as_str() {
            "R" => Ok(Token::R),
            "true" => Ok(Token::True),
            "false" => Ok(Token::False),
            "null" => Ok(Token::Null),
            "obj" => Ok(Token::Obj),
            "endobj" => Ok(Token::EndObj),
            "stream" => Ok(Token::Stream),
            "endstream" => Ok(Token::EndStream),
            "xref" => Ok(Token::Xref),
            "trailer" => Ok(Token::Trailer),
            "startxref" => Ok(Token::StartXref),
            "newobj" => Ok(Token::NewObj),
            _ => Ok(Token::Keyword),
        }
    }

    fn is_eof(&self) -> bool {
        self.pos >= self.data.len()
    }

    fn is_eof_at(&self, pos: usize) -> bool {
        pos >= self.data.len()
    }

    fn peek(&self) -> Result<u8> {
        if self.is_eof() {
            Err(Error::Generic("Unexpected EOF".into()))
        } else {
            Ok(self.data[self.pos])
        }
    }

    fn peek_eq(&self, ch: u8) -> bool {
        !self.is_eof() && self.data[self.pos] == ch
    }

    fn advance(&mut self) {
        if !self.is_eof() {
            self.pos += 1;
        }
    }

    fn is_delimiter(ch: u8) -> bool {
        matches!(
            ch,
            b'(' | b')' | b'<' | b'>' | b'[' | b']' | b'{' | b'}' | b'/' | b'%'
        )
    }

    fn is_whitespace(ch: u8) -> bool {
        matches!(ch, b' ' | b'\t' | b'\r' | b'\n' | b'\x0C')
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lex_integers() {
        let data = b"123 -456 +789";
        let mut lexer = Lexer::new(data);
        let mut buf = LexBuf::new();

        assert_eq!(lexer.lex(&mut buf).unwrap(), Token::Int);
        assert_eq!(buf.as_int(), 123);

        assert_eq!(lexer.lex(&mut buf).unwrap(), Token::Int);
        assert_eq!(buf.as_int(), -456);

        assert_eq!(lexer.lex(&mut buf).unwrap(), Token::Int);
        assert_eq!(buf.as_int(), 789);
    }

    #[test]
    fn test_lex_reals() {
        let data = b"3.25 -0.5 +2.75";
        let mut lexer = Lexer::new(data);
        let mut buf = LexBuf::new();

        assert_eq!(lexer.lex(&mut buf).unwrap(), Token::Real);
        assert!((buf.as_float() - 3.25).abs() < 0.001);

        assert_eq!(lexer.lex(&mut buf).unwrap(), Token::Real);
        assert!((buf.as_float() + 0.5).abs() < 0.001);
    }

    #[test]
    fn test_lex_names() {
        let data = b"/Type /Font /BaseFont";
        let mut lexer = Lexer::new(data);
        let mut buf = LexBuf::new();

        assert_eq!(lexer.lex(&mut buf).unwrap(), Token::Name);
        assert_eq!(buf.as_str(), "Type");

        assert_eq!(lexer.lex(&mut buf).unwrap(), Token::Name);
        assert_eq!(buf.as_str(), "Font");
    }

    #[test]
    fn test_lex_strings() {
        let data = b"(Hello World)";
        let mut lexer = Lexer::new(data);
        let mut buf = LexBuf::new();

        assert_eq!(lexer.lex(&mut buf).unwrap(), Token::String);
        assert_eq!(buf.as_str(), "Hello World");
    }

    #[test]
    fn test_lex_string_escapes() {
        let data = b"(Line\\nBreak\\tTab)";
        let mut lexer = Lexer::new(data);
        let mut buf = LexBuf::new();

        assert_eq!(lexer.lex(&mut buf).unwrap(), Token::String);
        assert_eq!(buf.as_str(), "Line\nBreak\tTab");
    }

    #[test]
    fn test_lex_keywords() {
        let data = b"true false null R obj endobj";
        let mut lexer = Lexer::new(data);
        let mut buf = LexBuf::new();

        assert_eq!(lexer.lex(&mut buf).unwrap(), Token::True);
        assert_eq!(lexer.lex(&mut buf).unwrap(), Token::False);
        assert_eq!(lexer.lex(&mut buf).unwrap(), Token::Null);
        assert_eq!(lexer.lex(&mut buf).unwrap(), Token::R);
        assert_eq!(lexer.lex(&mut buf).unwrap(), Token::Obj);
        assert_eq!(lexer.lex(&mut buf).unwrap(), Token::EndObj);
    }

    #[test]
    fn test_lex_arrays() {
        let data = b"[1 2 3]";
        let mut lexer = Lexer::new(data);
        let mut buf = LexBuf::new();

        assert_eq!(lexer.lex(&mut buf).unwrap(), Token::OpenArray);
        assert_eq!(lexer.lex(&mut buf).unwrap(), Token::Int);
        assert_eq!(buf.as_int(), 1);
        assert_eq!(lexer.lex(&mut buf).unwrap(), Token::Int);
        assert_eq!(lexer.lex(&mut buf).unwrap(), Token::Int);
        assert_eq!(lexer.lex(&mut buf).unwrap(), Token::CloseArray);
    }

    #[test]
    fn test_lex_dicts() {
        let data = b"<< /Key /Value >>";
        let mut lexer = Lexer::new(data);
        let mut buf = LexBuf::new();

        assert_eq!(lexer.lex(&mut buf).unwrap(), Token::OpenDict);
        assert_eq!(lexer.lex(&mut buf).unwrap(), Token::Name);
        assert_eq!(buf.as_str(), "Key");
        assert_eq!(lexer.lex(&mut buf).unwrap(), Token::Name);
        assert_eq!(buf.as_str(), "Value");
        assert_eq!(lexer.lex(&mut buf).unwrap(), Token::CloseDict);
    }

    #[test]
    fn test_lex_comments() {
        let data = b"123 % This is a comment\n456";
        let mut lexer = Lexer::new(data);
        let mut buf = LexBuf::new();

        assert_eq!(lexer.lex(&mut buf).unwrap(), Token::Int);
        assert_eq!(buf.as_int(), 123);
        assert_eq!(lexer.lex(&mut buf).unwrap(), Token::Int);
        assert_eq!(buf.as_int(), 456);
    }

    #[test]
    fn test_lex_eof() {
        let data = b"123";
        let mut lexer = Lexer::new(data);
        let mut buf = LexBuf::new();

        assert_eq!(lexer.lex(&mut buf).unwrap(), Token::Int);
        assert_eq!(lexer.lex(&mut buf).unwrap(), Token::Eof);
    }

    #[test]
    fn test_lexbuf_clear() {
        let mut buf = LexBuf::new();
        buf.buffer = "test".to_string();
        buf.int_value = 42;
        buf.float_value = std::f64::consts::PI;

        buf.clear();
        assert!(buf.buffer.is_empty());
        assert_eq!(buf.int_value, 0);
        assert_eq!(buf.float_value, 0.0);
    }
}
