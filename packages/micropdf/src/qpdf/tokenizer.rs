//! PDF tokenizer for lexical analysis of PDF content streams
//!
//! This module provides a tokenizer that can parse PDF syntax into tokens,
//! enabling content stream manipulation at the token level.
//!
//! # Example
//!
//! ```rust,ignore
//! use micropdf::qpdf::tokenizer::{Tokenizer, TokenType};
//!
//! let content = b"BT /F1 12 Tf (Hello) Tj ET";
//! let mut tokenizer = Tokenizer::new(content);
//!
//! while let Some(token) = tokenizer.next_token()? {
//!     match token.token_type {
//!         TokenType::Name => println!("Name: {}", token.value),
//!         TokenType::Integer => println!("Integer: {}", token.value),
//!         TokenType::String => println!("String: {}", token.value),
//!         TokenType::Word => println!("Operator: {}", token.value),
//!         _ => {}
//!     }
//! }
//! ```

use super::error::{QpdfError, Result};

/// Token types in PDF syntax
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenType {
    /// End of file/stream
    Eof,
    /// Array start: [
    ArrayOpen,
    /// Array end: ]
    ArrayClose,
    /// Dictionary start: <<
    DictOpen,
    /// Dictionary end: >>
    DictClose,
    /// Name: /SomeName
    Name,
    /// Literal string: (string)
    String,
    /// Hex string: <hexdata>
    HexString,
    /// Integer number
    Integer,
    /// Real (floating point) number
    Real,
    /// Boolean: true or false
    Boolean,
    /// Null keyword
    Null,
    /// Keyword/operator word
    Word,
    /// Inline image data
    InlineImage,
    /// Comment: % to end of line
    Comment,
    /// Bad/invalid token
    Bad,
    /// Space/whitespace (usually filtered out)
    Space,
}

/// A token from the PDF tokenizer
#[derive(Debug, Clone)]
pub struct Token {
    /// The type of this token
    pub token_type: TokenType,
    /// The raw value as it appeared in the input
    pub raw_value: String,
    /// The parsed/canonical value
    pub value: String,
    /// Error message if this is a bad token
    pub error_message: Option<String>,
    /// Starting offset in the input
    pub offset: usize,
}

impl Token {
    /// Create a new token
    pub fn new(token_type: TokenType, value: &str, offset: usize) -> Self {
        Self {
            token_type,
            raw_value: value.to_string(),
            value: value.to_string(),
            error_message: None,
            offset,
        }
    }

    /// Create a new token with separate raw and parsed values
    pub fn with_raw(token_type: TokenType, raw: &str, value: &str, offset: usize) -> Self {
        Self {
            token_type,
            raw_value: raw.to_string(),
            value: value.to_string(),
            error_message: None,
            offset,
        }
    }

    /// Create a bad token with an error message
    pub fn bad(raw: &str, error: &str, offset: usize) -> Self {
        Self {
            token_type: TokenType::Bad,
            raw_value: raw.to_string(),
            value: raw.to_string(),
            error_message: Some(error.to_string()),
            offset,
        }
    }

    /// Check if this token is a specific type
    pub fn is(&self, token_type: TokenType) -> bool {
        self.token_type == token_type
    }

    /// Get the integer value of this token
    pub fn as_integer(&self) -> Option<i64> {
        if self.token_type == TokenType::Integer {
            self.value.parse().ok()
        } else {
            None
        }
    }

    /// Get the real value of this token
    pub fn as_real(&self) -> Option<f64> {
        match self.token_type {
            TokenType::Integer | TokenType::Real => self.value.parse().ok(),
            _ => None,
        }
    }

    /// Get the boolean value of this token
    pub fn as_boolean(&self) -> Option<bool> {
        if self.token_type == TokenType::Boolean {
            Some(self.value == "true")
        } else {
            None
        }
    }
}

/// PDF tokenizer
pub struct Tokenizer<'a> {
    data: &'a [u8],
    position: usize,
    include_ignorable: bool,
}

impl<'a> Tokenizer<'a> {
    /// Create a new tokenizer for the given data
    pub fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            position: 0,
            include_ignorable: false,
        }
    }

    /// Set whether to include ignorable tokens (comments, whitespace)
    pub fn set_include_ignorable(&mut self, include: bool) {
        self.include_ignorable = include;
    }

    /// Get the current position in the input
    pub fn position(&self) -> usize {
        self.position
    }

    /// Set the position in the input
    pub fn set_position(&mut self, pos: usize) {
        self.position = pos.min(self.data.len());
    }

    /// Check if we've reached the end of input
    pub fn at_end(&self) -> bool {
        self.position >= self.data.len()
    }

    /// Peek at the next byte without consuming it
    fn peek(&self) -> Option<u8> {
        self.data.get(self.position).copied()
    }

    /// Peek at a byte at offset from current position
    fn peek_at(&self, offset: usize) -> Option<u8> {
        self.data.get(self.position + offset).copied()
    }

    /// Get the next byte and advance position
    fn next_byte(&mut self) -> Option<u8> {
        if self.position < self.data.len() {
            let b = self.data[self.position];
            self.position += 1;
            Some(b)
        } else {
            None
        }
    }

    /// Check if a byte is PDF whitespace
    fn is_whitespace(b: u8) -> bool {
        matches!(b, 0 | 9 | 10 | 12 | 13 | 32)
    }

    /// Check if a byte is a PDF delimiter
    fn is_delimiter(b: u8) -> bool {
        matches!(
            b,
            b'(' | b')' | b'<' | b'>' | b'[' | b']' | b'{' | b'}' | b'/' | b'%'
        )
    }

    /// Skip whitespace and comments
    fn skip_ignorable(&mut self) {
        while let Some(b) = self.peek() {
            if Self::is_whitespace(b) {
                self.position += 1;
            } else if b == b'%' {
                // Skip comment to end of line
                while let Some(b) = self.next_byte() {
                    if b == b'\n' || b == b'\r' {
                        break;
                    }
                }
            } else {
                break;
            }
        }
    }

    /// Read a name token
    fn read_name(&mut self) -> Token {
        let start = self.position;
        self.position += 1; // Skip the '/'

        let mut value = String::from("/");
        let mut raw = String::from("/");

        while let Some(b) = self.peek() {
            if Self::is_whitespace(b) || Self::is_delimiter(b) {
                break;
            }

            self.position += 1;
            raw.push(b as char);

            if b == b'#' {
                // Hex escape
                if let (Some(h1), Some(h2)) = (self.peek_at(0), self.peek_at(1)) {
                    if h1.is_ascii_hexdigit() && h2.is_ascii_hexdigit() {
                        self.position += 2;
                        raw.push(h1 as char);
                        raw.push(h2 as char);
                        let hex_str = format!("{}{}", h1 as char, h2 as char);
                        if let Ok(code) = u8::from_str_radix(&hex_str, 16) {
                            value.push(code as char);
                        }
                        continue;
                    }
                }
            }
            value.push(b as char);
        }

        Token::with_raw(TokenType::Name, &raw, &value, start)
    }

    /// Read a literal string token
    fn read_string(&mut self) -> Token {
        let start = self.position;
        self.position += 1; // Skip the '('

        let mut value = String::new();
        let mut raw = String::from("(");
        let mut depth = 1;

        while let Some(b) = self.next_byte() {
            raw.push(b as char);

            match b {
                b'(' => {
                    depth += 1;
                    value.push('(');
                }
                b')' => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                    value.push(')');
                }
                b'\\' => {
                    // Escape sequence
                    if let Some(escaped) = self.next_byte() {
                        raw.push(escaped as char);
                        match escaped {
                            b'n' => value.push('\n'),
                            b'r' => value.push('\r'),
                            b't' => value.push('\t'),
                            b'b' => value.push('\x08'),
                            b'f' => value.push('\x0c'),
                            b'(' => value.push('('),
                            b')' => value.push(')'),
                            b'\\' => value.push('\\'),
                            b'\r' => {
                                // Line continuation
                                if self.peek() == Some(b'\n') {
                                    self.position += 1;
                                    raw.push('\n');
                                }
                            }
                            b'\n' => {
                                // Line continuation
                            }
                            b'0'..=b'7' => {
                                // Octal escape
                                let mut octal = (escaped - b'0') as u32;
                                for _ in 0..2 {
                                    if let Some(ob) = self.peek() {
                                        if (b'0'..=b'7').contains(&ob) {
                                            self.position += 1;
                                            raw.push(ob as char);
                                            octal = octal * 8 + (ob - b'0') as u32;
                                        } else {
                                            break;
                                        }
                                    }
                                }
                                if octal <= 255 {
                                    value.push(octal as u8 as char);
                                }
                            }
                            _ => {
                                // Unknown escape, treat as literal
                                value.push(escaped as char);
                            }
                        }
                    }
                }
                _ => {
                    value.push(b as char);
                }
            }
        }

        Token::with_raw(TokenType::String, &raw, &value, start)
    }

    /// Read a hex string token
    fn read_hex_string(&mut self) -> Token {
        let start = self.position;
        self.position += 1; // Skip the '<'

        let mut raw = String::from("<");
        let mut hex_chars = String::new();

        while let Some(b) = self.next_byte() {
            raw.push(b as char);

            if b == b'>' {
                break;
            }
            if !Self::is_whitespace(b) {
                if b.is_ascii_hexdigit() {
                    hex_chars.push(b as char);
                } else {
                    return Token::bad(&raw, "Invalid character in hex string", start);
                }
            }
        }

        // Pad with trailing zero if odd number of digits
        if hex_chars.len() % 2 != 0 {
            hex_chars.push('0');
        }

        // Decode hex
        let mut value = String::new();
        let mut chars = hex_chars.chars();
        while let (Some(h1), Some(h2)) = (chars.next(), chars.next()) {
            let hex = format!("{}{}", h1, h2);
            if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                value.push(byte as char);
            }
        }

        Token::with_raw(TokenType::HexString, &raw, &value, start)
    }

    /// Read a number token (integer or real)
    fn read_number(&mut self) -> Token {
        let start = self.position;
        let mut value = String::new();
        let mut is_real = false;

        // Handle sign
        if let Some(b) = self.peek() {
            if b == b'+' || b == b'-' {
                value.push(b as char);
                self.position += 1;
            }
        }

        // Read digits and decimal point
        while let Some(b) = self.peek() {
            if b.is_ascii_digit() {
                value.push(b as char);
                self.position += 1;
            } else if b == b'.' && !is_real {
                is_real = true;
                value.push('.');
                self.position += 1;
            } else {
                break;
            }
        }

        let token_type = if is_real {
            TokenType::Real
        } else {
            TokenType::Integer
        };

        Token::new(token_type, &value, start)
    }

    /// Read a word token (keyword or operator)
    fn read_word(&mut self) -> Token {
        let start = self.position;
        let mut value = String::new();

        while let Some(b) = self.peek() {
            if Self::is_whitespace(b) || Self::is_delimiter(b) {
                break;
            }
            value.push(b as char);
            self.position += 1;
        }

        let token_type = match value.as_str() {
            "true" | "false" => TokenType::Boolean,
            "null" => TokenType::Null,
            _ => TokenType::Word,
        };

        Token::new(token_type, &value, start)
    }

    /// Read an inline image
    fn read_inline_image(&mut self) -> Token {
        let start = self.position;
        let mut raw = String::from("ID");
        self.position += 2; // Skip "ID"

        // Skip single whitespace after ID
        if let Some(b) = self.peek() {
            if Self::is_whitespace(b) {
                self.position += 1;
                if b != b'\n' && b != b'\r' {
                    raw.push(b as char);
                }
            }
        }

        // Read until we find "EI" preceded by whitespace
        let mut data = Vec::new();
        while self.position < self.data.len() {
            let b = self.data[self.position];
            self.position += 1;

            // Check for EI marker
            if Self::is_whitespace(b)
                && self.position + 1 < self.data.len()
                && self.data[self.position] == b'E'
                && self.data[self.position + 1] == b'I'
            {
                // Verify EI is followed by whitespace or delimiter
                if self.position + 2 >= self.data.len()
                    || Self::is_whitespace(self.data[self.position + 2])
                    || Self::is_delimiter(self.data[self.position + 2])
                {
                    self.position += 2; // Skip "EI"
                    break;
                }
            }

            data.push(b);
        }

        // Convert data to string (may contain binary)
        let value = String::from_utf8_lossy(&data).into_owned();
        raw.push_str(&value);
        raw.push_str("EI");

        Token::with_raw(TokenType::InlineImage, &raw, &value, start)
    }

    /// Get the next token
    pub fn next_token(&mut self) -> Result<Option<Token>> {
        if !self.include_ignorable {
            self.skip_ignorable();
        }

        if self.at_end() {
            return Ok(Some(Token::new(TokenType::Eof, "", self.position)));
        }

        let b = self.peek().unwrap();
        let start = self.position;

        let token = match b {
            // Whitespace (only if include_ignorable)
            _ if Self::is_whitespace(b) && self.include_ignorable => {
                let mut ws = String::new();
                while let Some(b) = self.peek() {
                    if Self::is_whitespace(b) {
                        ws.push(b as char);
                        self.position += 1;
                    } else {
                        break;
                    }
                }
                Token::new(TokenType::Space, &ws, start)
            }

            // Comment (only if include_ignorable)
            b'%' if self.include_ignorable => {
                let mut comment = String::from("%");
                self.position += 1;
                while let Some(b) = self.peek() {
                    if b == b'\n' || b == b'\r' {
                        break;
                    }
                    comment.push(b as char);
                    self.position += 1;
                }
                Token::new(TokenType::Comment, &comment, start)
            }

            // Array delimiters
            b'[' => {
                self.position += 1;
                Token::new(TokenType::ArrayOpen, "[", start)
            }
            b']' => {
                self.position += 1;
                Token::new(TokenType::ArrayClose, "]", start)
            }

            // Dictionary or hex string
            b'<' => {
                if self.peek_at(1) == Some(b'<') {
                    self.position += 2;
                    Token::new(TokenType::DictOpen, "<<", start)
                } else {
                    self.read_hex_string()
                }
            }
            b'>' => {
                if self.peek_at(1) == Some(b'>') {
                    self.position += 2;
                    Token::new(TokenType::DictClose, ">>", start)
                } else {
                    self.position += 1;
                    Token::bad(">", "Unexpected '>'", start)
                }
            }

            // Name
            b'/' => self.read_name(),

            // Literal string
            b'(' => self.read_string(),

            // Number
            b'+' | b'-' | b'.' | b'0'..=b'9' => self.read_number(),

            // Check for inline image
            b'I' if self.peek_at(1) == Some(b'D') => {
                // Check if this looks like ID (inline image data)
                if self.position >= 2 {
                    self.read_inline_image()
                } else {
                    self.read_word()
                }
            }

            // Word/keyword
            _ => self.read_word(),
        };

        Ok(Some(token))
    }

    /// Read all tokens into a vector
    pub fn read_all_tokens(&mut self) -> Result<Vec<Token>> {
        let mut tokens = Vec::new();
        while let Some(token) = self.next_token()? {
            if token.token_type == TokenType::Eof {
                break;
            }
            tokens.push(token);
        }
        Ok(tokens)
    }
}

/// Token filter trait for transforming content streams
///
/// Implement this trait to create custom content stream transformations.
pub trait TokenFilter {
    /// Handle a single token
    ///
    /// Return the tokens to emit (can be empty, the same token, or multiple tokens)
    fn handle_token(&mut self, token: &Token) -> Vec<Token>;

    /// Called at end of stream
    fn handle_eof(&mut self) -> Vec<Token> {
        Vec::new()
    }
}

/// A simple token filter that passes through all tokens unchanged
pub struct PassThroughFilter;

impl TokenFilter for PassThroughFilter {
    fn handle_token(&mut self, token: &Token) -> Vec<Token> {
        vec![token.clone()]
    }
}

/// Apply a token filter to content stream data
pub fn filter_content_stream<F: TokenFilter>(data: &[u8], filter: &mut F) -> Result<Vec<u8>> {
    let mut tokenizer = Tokenizer::new(data);
    let mut output = Vec::new();

    while let Some(token) = tokenizer.next_token()? {
        let is_eof = token.token_type == TokenType::Eof;
        let output_tokens = if is_eof {
            filter.handle_eof()
        } else {
            filter.handle_token(&token)
        };

        for out_token in output_tokens {
            output.extend_from_slice(out_token.raw_value.as_bytes());
            output.push(b' '); // Add space between tokens
        }

        if is_eof {
            break;
        }
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize_simple() {
        let data = b"BT /F1 12 Tf ET";
        let mut tokenizer = Tokenizer::new(data);

        let tokens = tokenizer.read_all_tokens().unwrap();
        assert_eq!(tokens.len(), 5);
        assert_eq!(tokens[0].token_type, TokenType::Word);
        assert_eq!(tokens[0].value, "BT");
        assert_eq!(tokens[1].token_type, TokenType::Name);
        assert_eq!(tokens[1].value, "/F1");
        assert_eq!(tokens[2].token_type, TokenType::Integer);
        assert_eq!(tokens[2].value, "12");
        assert_eq!(tokens[3].token_type, TokenType::Word);
        assert_eq!(tokens[3].value, "Tf");
        assert_eq!(tokens[4].token_type, TokenType::Word);
        assert_eq!(tokens[4].value, "ET");
    }

    #[test]
    fn test_tokenize_string() {
        let data = b"(Hello, World!)";
        let mut tokenizer = Tokenizer::new(data);

        let token = tokenizer.next_token().unwrap().unwrap();
        assert_eq!(token.token_type, TokenType::String);
        assert_eq!(token.value, "Hello, World!");
    }

    #[test]
    fn test_tokenize_hex_string() {
        let data = b"<48656C6C6F>";
        let mut tokenizer = Tokenizer::new(data);

        let token = tokenizer.next_token().unwrap().unwrap();
        assert_eq!(token.token_type, TokenType::HexString);
        assert_eq!(token.value, "Hello");
    }

    #[test]
    fn test_tokenize_name_with_escape() {
        let data = b"/Name#20With#20Spaces";
        let mut tokenizer = Tokenizer::new(data);

        let token = tokenizer.next_token().unwrap().unwrap();
        assert_eq!(token.token_type, TokenType::Name);
        assert_eq!(token.value, "/Name With Spaces");
    }

    #[test]
    fn test_tokenize_array() {
        let data = b"[1 2 3]";
        let mut tokenizer = Tokenizer::new(data);

        let tokens = tokenizer.read_all_tokens().unwrap();
        assert_eq!(tokens.len(), 5);
        assert_eq!(tokens[0].token_type, TokenType::ArrayOpen);
        assert_eq!(tokens[1].token_type, TokenType::Integer);
        assert_eq!(tokens[4].token_type, TokenType::ArrayClose);
    }

    #[test]
    fn test_tokenize_dict() {
        let data = b"<</Type/Page>>";
        let mut tokenizer = Tokenizer::new(data);

        let tokens = tokenizer.read_all_tokens().unwrap();
        assert_eq!(tokens.len(), 4);
        assert_eq!(tokens[0].token_type, TokenType::DictOpen);
        assert_eq!(tokens[1].token_type, TokenType::Name);
        assert_eq!(tokens[2].token_type, TokenType::Name);
        assert_eq!(tokens[3].token_type, TokenType::DictClose);
    }

    #[test]
    fn test_tokenize_real() {
        let data = b"3.14159 -2.5 +0.5";
        let mut tokenizer = Tokenizer::new(data);

        let tokens = tokenizer.read_all_tokens().unwrap();
        assert_eq!(tokens.len(), 3);
        assert_eq!(tokens[0].token_type, TokenType::Real);
        assert_eq!(tokens[0].value, "3.14159");
        assert_eq!(tokens[1].token_type, TokenType::Real);
        assert_eq!(tokens[1].value, "-2.5");
    }

    #[test]
    fn test_tokenize_boolean_null() {
        let data = b"true false null";
        let mut tokenizer = Tokenizer::new(data);

        let tokens = tokenizer.read_all_tokens().unwrap();
        assert_eq!(tokens[0].token_type, TokenType::Boolean);
        assert_eq!(tokens[0].value, "true");
        assert_eq!(tokens[1].token_type, TokenType::Boolean);
        assert_eq!(tokens[1].value, "false");
        assert_eq!(tokens[2].token_type, TokenType::Null);
    }
}
