//! PDF Object Parser - Recursive Descent Parser
//!
//! Provides comprehensive PDF object parsing independent of document loading.
//! Supports all PDF object types, references, and stream objects.

use crate::fitz::error::{Error, Result};
use crate::pdf::lexer::{LexBuf, Lexer, Token};
use crate::pdf::object::{Array, Dict, Name, ObjRef, Object, PdfString};
use crate::pdf::xref::XrefEntry;
use std::collections::HashMap;

/// PDF Parser - parses objects from a byte stream
pub struct Parser<'a> {
    lexer: Lexer<'a>,
    buf: LexBuf,
    data: &'a [u8],
    pos: usize,
}

impl<'a> Parser<'a> {
    /// Create a new parser from byte data
    pub fn new(data: &'a [u8]) -> Self {
        Self {
            lexer: Lexer::new(data),
            buf: LexBuf::new(),
            data,
            pos: 0,
        }
    }

    /// Parse the next object from the stream
    pub fn parse_object(&mut self) -> Result<Object> {
        self.skip_whitespace_and_comments();

        if self.is_eof() {
            return Err(Error::Generic("Unexpected EOF".into()));
        }

        let obj = self.parse_value()?;

        // Check for reference (num gen R pattern)
        if let Object::Int(num) = obj {
            self.skip_whitespace_and_comments();

            // Look ahead for generation number
            let saved_pos = self.lexer_pos();
            if let Ok(Token::Int) = self.peek_token() {
                if let Ok(gen) = self.parse_int() {
                    self.skip_whitespace_and_comments();

                    // Check for R keyword
                    if let Ok(Token::R) = self.peek_token() {
                        self.consume_token()?; // consume R
                        return Ok(Object::Ref(ObjRef::new(num as i32, gen as i32)));
                    }
                }
            }
            // Not a reference, restore position and return as integer
            self.set_lexer_pos(saved_pos);
            return Ok(Object::Int(num));
        }

        Ok(obj)
    }

    /// Parse a PDF value (non-reference)
    fn parse_value(&mut self) -> Result<Object> {
        let token = self.next_token()?;

        match token {
            Token::Null => Ok(Object::Null),
            Token::True => Ok(Object::Bool(true)),
            Token::False => Ok(Object::Bool(false)),
            Token::Int => {
                let val = self.buf.as_int();
                Ok(Object::Int(val))
            }
            Token::Real => {
                let val = self.buf.as_float();
                Ok(Object::Real(val))
            }
            Token::String => {
                let s = self.buf.as_str().to_string();
                // Decode hex strings
                if s.chars()
                    .all(|c| c.is_ascii_hexdigit() || c.is_whitespace())
                {
                    // Hex string
                    let bytes = hex_string_to_bytes(&s)?;
                    Ok(Object::String(PdfString::new(bytes)))
                } else {
                    // Literal string
                    Ok(Object::String(PdfString::new(s.into_bytes())))
                }
            }
            Token::Name => {
                let name = self.buf.as_str().to_string();
                Ok(Object::Name(Name::new(&name)))
            }
            Token::OpenArray => self.parse_array(),
            Token::OpenDict => self.parse_dict(),
            Token::Stream => Err(Error::Generic("Unexpected 'stream' keyword".into())),
            Token::EndStream => Err(Error::Generic("Unexpected 'endstream' keyword".into())),
            Token::Obj => Err(Error::Generic("Unexpected 'obj' keyword".into())),
            Token::EndObj => Err(Error::Generic("Unexpected 'endobj' keyword".into())),
            Token::R => Err(Error::Generic("Unexpected 'R' keyword".into())),
            Token::Eof => Err(Error::Generic("Unexpected EOF".into())),
            Token::Error => Err(Error::Generic("Lexer error".into())),
            _ => Err(Error::Generic(format!("Unexpected token: {:?}", token))),
        }
    }

    /// Parse an array
    fn parse_array(&mut self) -> Result<Object> {
        let mut arr = Array::new();

        loop {
            self.skip_whitespace_and_comments();

            if self.is_eof() {
                return Err(Error::Generic("Unterminated array".into()));
            }

            // Check for closing bracket
            if self.peek_byte() == Some(b']') {
                self.consume_byte();
                break;
            }

            let obj = self.parse_object()?;
            arr.push(obj);
        }

        Ok(Object::Array(arr))
    }

    /// Parse a dictionary
    fn parse_dict(&mut self) -> Result<Object> {
        let mut dict = Dict::new();

        loop {
            self.skip_whitespace_and_comments();

            if self.is_eof() {
                return Err(Error::Generic("Unterminated dictionary".into()));
            }

            // Check for closing >>
            if self.peek_two_bytes() == (Some(b'>'), Some(b'>')) {
                self.consume_byte();
                self.consume_byte();
                break;
            }

            // Parse key (must be a name)
            let key_token = self.next_token()?;
            let key = match key_token {
                Token::Name => Name::new(self.buf.as_str()),
                _ => {
                    return Err(Error::Generic(format!(
                        "Dictionary key must be a name, got {:?}",
                        key_token
                    )))
                }
            };

            // Parse value
            let value = self.parse_object()?;
            dict.insert(key, value);
        }

        Ok(Object::Dict(dict))
    }

    /// Parse an indirect object: num gen obj ... endobj
    pub fn parse_indirect_object(&mut self) -> Result<(i32, i32, Object)> {
        let obj_num = self.expect_int()?;
        let gen_num = self.expect_int()?;
        self.expect_token(Token::Obj)?;

        let obj = self.parse_object()?;

        // Check for stream after object
        self.skip_whitespace_and_comments();
        if self.peek_token()? == Token::Stream {
            return Err(Error::Generic(
                "Stream objects should use parse_stream_object".into(),
            ));
        }

        self.expect_token(Token::EndObj)?;

        Ok((obj_num as i32, gen_num as i32, obj))
    }

    /// Parse a stream object: num gen obj <<dict>> stream ... endstream endobj
    pub fn parse_stream_object(&mut self) -> Result<(i32, i32, Dict, Vec<u8>)> {
        let obj_num = self.expect_int()?;
        let gen_num = self.expect_int()?;
        self.expect_token(Token::Obj)?;

        // Parse dictionary
        let dict_obj = self.parse_object()?;
        let dict = match dict_obj {
            Object::Dict(d) => d,
            _ => {
                return Err(Error::Generic(
                    "Stream object must have a dictionary".into(),
                ))
            }
        };

        self.expect_token(Token::Stream)?;

        // Get stream length
        let length = dict
            .get(&Name::new("Length"))
            .and_then(|o| o.as_int())
            .ok_or_else(|| Error::Generic("Stream missing Length".into()))?
            as usize;

        // Read stream data (skip line break after "stream")
        self.skip_line_break();
        let stream_data = self.read_bytes(length)?;

        self.expect_token(Token::EndStream)?;
        self.expect_token(Token::EndObj)?;

        Ok((obj_num as i32, gen_num as i32, dict, stream_data))
    }

    /// Parse XREF table
    pub fn parse_xref(&mut self) -> Result<Vec<XrefEntry>> {
        self.expect_token(Token::Xref)?;

        let mut entries = Vec::new();

        loop {
            self.skip_whitespace_and_comments();

            // Check for trailer keyword
            if self.peek_token()? == Token::Trailer {
                break;
            }

            // Parse subsection header
            let start = self.expect_int()? as i32;
            let count = self.expect_int()? as i32;

            // Parse entries
            for i in 0..count {
                let entry = self.parse_xref_entry(start + i)?;
                entries.push(entry);
            }
        }

        Ok(entries)
    }

    /// Parse a single XREF entry
    fn parse_xref_entry(&mut self, obj_num: i32) -> Result<XrefEntry> {
        // XREF entry format: "nnnnnnnnnn ggggg n\r" or "nnnnnnnnnn ggggg f\r"
        self.skip_whitespace_and_comments();

        let line = self.read_line()?;
        if line.len() < 18 {
            return Err(Error::Generic(format!("Invalid XREF entry: '{}'", line)));
        }

        let offset_str = &line[0..10];
        let gen_str = &line[11..16];
        let entry_type = line.chars().nth(17).unwrap_or('f');

        let offset: i64 = offset_str
            .trim()
            .parse()
            .map_err(|_| Error::Generic("Invalid XREF offset".into()))?;
        let generation: u16 = gen_str
            .trim()
            .parse()
            .map_err(|_| Error::Generic("Invalid XREF generation".into()))?;

        let entry = match entry_type {
            'n' => XrefEntry::in_use(obj_num, generation, offset),
            'f' => XrefEntry::free(obj_num, generation),
            _ => {
                return Err(Error::Generic(format!(
                    "Unknown XREF entry type: {}",
                    entry_type
                )))
            }
        };

        Ok(entry)
    }

    /// Parse trailer dictionary
    pub fn parse_trailer(&mut self) -> Result<Dict> {
        self.expect_token(Token::Trailer)?;

        let obj = self.parse_object()?;
        match obj {
            Object::Dict(d) => Ok(d),
            _ => Err(Error::Generic("Trailer must be a dictionary".into())),
        }
    }

    /// Helper: expect a specific token
    fn expect_token(&mut self, expected: Token) -> Result<()> {
        let token = self.next_token()?;
        if token != expected {
            return Err(Error::Generic(format!(
                "Expected {:?}, got {:?}",
                expected, token
            )));
        }
        Ok(())
    }

    /// Helper: expect an integer
    fn expect_int(&mut self) -> Result<i64> {
        let token = self.next_token()?;
        match token {
            Token::Int => Ok(self.buf.as_int()),
            _ => Err(Error::Generic(format!("Expected integer, got {:?}", token))),
        }
    }

    /// Helper: parse an integer without consuming as object
    fn parse_int(&mut self) -> Result<i64> {
        let token = self.next_token()?;
        match token {
            Token::Int => Ok(self.buf.as_int()),
            _ => Err(Error::Generic(format!("Expected integer, got {:?}", token))),
        }
    }

    /// Helper: get next token
    fn next_token(&mut self) -> Result<Token> {
        let token = self.lexer.lex(&mut self.buf)?;
        self.pos = self.lexer_pos();
        Ok(token)
    }

    /// Helper: peek at next token
    fn peek_token(&mut self) -> Result<Token> {
        let saved_pos = self.lexer_pos();
        let token = self.lexer.lex(&mut self.buf)?;
        self.set_lexer_pos(saved_pos);
        Ok(token)
    }

    /// Helper: get current lexer position
    fn lexer_pos(&self) -> usize {
        // This is a simplification - in reality we'd need access to lexer's internal pos
        self.pos
    }

    /// Helper: set lexer position
    fn set_lexer_pos(&mut self, pos: usize) {
        self.pos = pos;
        self.lexer = Lexer::new(&self.data[pos..]);
    }

    /// Helper: skip whitespace and comments
    fn skip_whitespace_and_comments(&mut self) {
        // Use lexer to skip
        let _ = self.lexer.lex(&mut self.buf);
        // Reset and skip again (lexer already skips whitespace)
    }

    /// Helper: check if at EOF
    fn is_eof(&self) -> bool {
        self.pos >= self.data.len()
    }

    /// Helper: peek at next byte
    fn peek_byte(&self) -> Option<u8> {
        self.data.get(self.pos).copied()
    }

    /// Helper: peek at next two bytes
    fn peek_two_bytes(&self) -> (Option<u8>, Option<u8>) {
        (
            self.data.get(self.pos).copied(),
            self.data.get(self.pos + 1).copied(),
        )
    }

    /// Helper: consume a byte
    fn consume_byte(&mut self) {
        if self.pos < self.data.len() {
            self.pos += 1;
        }
    }

    /// Helper: read a line
    fn read_line(&mut self) -> Result<String> {
        let start = self.pos;
        while self.pos < self.data.len()
            && self.data[self.pos] != b'\n'
            && self.data[self.pos] != b'\r'
        {
            self.pos += 1;
        }
        let line = String::from_utf8_lossy(&self.data[start..self.pos]).to_string();
        self.skip_line_break();
        Ok(line)
    }

    /// Helper: skip line break
    fn skip_line_break(&mut self) {
        if self.pos < self.data.len() && self.data[self.pos] == b'\r' {
            self.pos += 1;
        }
        if self.pos < self.data.len() && self.data[self.pos] == b'\n' {
            self.pos += 1;
        }
    }

    /// Helper: read N bytes
    fn read_bytes(&mut self, n: usize) -> Result<Vec<u8>> {
        if self.pos + n > self.data.len() {
            return Err(Error::Generic("Not enough bytes".into()));
        }
        let bytes = self.data[self.pos..self.pos + n].to_vec();
        self.pos += n;
        Ok(bytes)
    }
}

/// Parse a hex string to bytes
fn hex_string_to_bytes(hex: &str) -> Result<Vec<u8>> {
    let hex: String = hex.chars().filter(|c| c.is_ascii_hexdigit()).collect();

    if hex.len() % 2 != 0 {
        return Err(Error::Generic("Invalid hex string length".into()));
    }

    let mut bytes = Vec::with_capacity(hex.len() / 2);
    for i in (0..hex.len()).step_by(2) {
        let byte = u8::from_str_radix(&hex[i..i + 2], 16)
            .map_err(|_| Error::Generic("Invalid hex digit".into()))?;
        bytes.push(byte);
    }

    Ok(bytes)
}

/// High-level function to parse a PDF document header
pub fn parse_header(data: &[u8]) -> Result<String> {
    if data.len() < 8 {
        return Err(Error::Generic("File too small".into()));
    }

    let header =
        std::str::from_utf8(&data[..8]).map_err(|_| Error::Generic("Invalid PDF header".into()))?;

    if !header.starts_with("%PDF-") {
        return Err(Error::Generic("Not a PDF file".into()));
    }

    let version = header[5..].trim().to_string();
    Ok(version)
}

/// Parse objects from a PDF content stream
pub fn parse_content_stream(data: &[u8]) -> Result<Vec<(Vec<Object>, String)>> {
    let mut parser = Parser::new(data);
    let mut operations = Vec::new();

    while !parser.is_eof() {
        let mut operands = Vec::new();

        // Collect operands until we hit an operator
        loop {
            parser.skip_whitespace_and_comments();

            if parser.is_eof() {
                break;
            }

            // Try to parse as object
            match parser.parse_object() {
                Ok(obj) => operands.push(obj),
                Err(_) => {
                    // Might be an operator
                    break;
                }
            }
        }

        // Parse operator
        if let Ok(op) = parser.parse_operator() {
            operations.push((operands, op));
        } else {
            break;
        }
    }

    Ok(operations)
}

impl<'a> Parser<'a> {
    /// Parse an operator name (for content streams)
    fn parse_operator(&mut self) -> Result<String> {
        self.skip_whitespace_and_comments();

        let start = self.pos;
        while self.pos < self.data.len() {
            let ch = self.data[self.pos] as char;
            if ch.is_ascii_alphabetic() || ch == '*' || ch == '\'' || ch == '"' {
                self.pos += 1;
            } else {
                break;
            }
        }

        if self.pos == start {
            return Err(Error::Generic("Expected operator".into()));
        }

        let op = String::from_utf8_lossy(&self.data[start..self.pos]).to_string();
        Ok(op)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_null() {
        let data = b"null";
        let mut parser = Parser::new(data);
        let obj = parser.parse_object().unwrap();
        assert_eq!(obj, Object::Null);
    }

    #[test]
    fn test_parse_bool() {
        let data = b"true";
        let mut parser = Parser::new(data);
        let obj = parser.parse_object().unwrap();
        assert_eq!(obj, Object::Bool(true));

        let data = b"false";
        let mut parser = Parser::new(data);
        let obj = parser.parse_object().unwrap();
        assert_eq!(obj, Object::Bool(false));
    }

    #[test]
    fn test_parse_int() {
        let data = b"42";
        let mut parser = Parser::new(data);
        let obj = parser.parse_object().unwrap();
        assert_eq!(obj, Object::Int(42));

        let data = b"-123";
        let mut parser = Parser::new(data);
        let obj = parser.parse_object().unwrap();
        assert_eq!(obj, Object::Int(-123));
    }

    #[test]
    fn test_parse_real() {
        let data = b"3.14159";
        let mut parser = Parser::new(data);
        let obj = parser.parse_object().unwrap();
        assert!(matches!(obj, Object::Real(v) if (v - 3.14159).abs() < 0.00001));
    }

    #[test]
    fn test_parse_string() {
        let data = b"(Hello World)";
        let mut parser = Parser::new(data);
        let obj = parser.parse_object().unwrap();

        if let Object::String(s) = obj {
            assert_eq!(s.as_str(), Some("Hello World"));
        } else {
            panic!("Expected string");
        }
    }

    #[test]
    fn test_parse_name() {
        let data = b"/Type";
        let mut parser = Parser::new(data);
        let obj = parser.parse_object().unwrap();

        if let Object::Name(name) = obj {
            assert_eq!(name.as_str(), "Type");
        } else {
            panic!("Expected name");
        }
    }

    #[test]
    fn test_parse_array() {
        let data = b"[1 2 3]";
        let mut parser = Parser::new(data);
        let obj = parser.parse_object().unwrap();

        if let Object::Array(arr) = obj {
            assert_eq!(arr.len(), 3);
            assert_eq!(arr[0], Object::Int(1));
        } else {
            panic!("Expected array");
        }
    }

    #[test]
    fn test_parse_dict() {
        let data = b"<< /Type /Catalog /Pages 5 0 R >>";
        let mut parser = Parser::new(data);
        let obj = parser.parse_object().unwrap();

        if let Object::Dict(dict) = obj {
            assert!(dict.contains_key(&Name::new("Type")));
        } else {
            panic!("Expected dictionary");
        }
    }

    #[test]
    fn test_parse_reference() {
        let data = b"5 0 R";
        let mut parser = Parser::new(data);
        let obj = parser.parse_object().unwrap();

        if let Object::Ref(r) = obj {
            assert_eq!(r.num, 5);
            assert_eq!(r.generation, 0);
        } else {
            panic!("Expected reference");
        }
    }

    #[test]
    fn test_parse_header() {
        let data = b"%PDF-1.4\n";
        let version = parse_header(data).unwrap();
        assert_eq!(version, "1.4");
    }

    #[test]
    fn test_hex_string_to_bytes() {
        let hex = "48656C6C6F";
        let bytes = hex_string_to_bytes(hex).unwrap();
        assert_eq!(bytes, b"Hello");
    }
}
