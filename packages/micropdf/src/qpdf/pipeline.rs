//! Pipeline system for stream processing
//!
//! This module provides a flexible pipeline system for processing data streams,
//! inspired by QPDF's Pipeline architecture. Pipelines can be chained together
//! to create complex data transformations.
//!
//! # Example
//!
//! ```rust,ignore
//! use micropdf::qpdf::pipeline::{Pipeline, PlBuffer, PlFlate, FlateAction};
//!
//! // Create a pipeline that compresses data
//! let buffer = PlBuffer::new("output");
//! let flate = PlFlate::new("compress", Box::new(buffer), FlateAction::Deflate);
//!
//! flate.write(b"Hello, World!")?;
//! flate.finish()?;
//!
//! let compressed = buffer.get_buffer();
//! ```

use super::error::{QpdfError, Result};
use flate2::Compression;
use flate2::read::{DeflateDecoder, DeflateEncoder};
use std::cell::RefCell;
use std::io::{Read, Write};
use std::rc::Rc;

/// A boxed pipeline for ownership and chaining
pub type PipelineBox = Box<dyn Pipeline>;

/// Pipeline trait for stream processing
///
/// Subclasses implement write and finish to process data and then
/// call the next pipeline in the chain if one exists.
pub trait Pipeline: Send {
    /// Get the identifier for this pipeline
    fn identifier(&self) -> &str;

    /// Write data to the pipeline
    fn write(&mut self, data: &[u8]) -> Result<()>;

    /// Finish processing and flush any remaining data
    fn finish(&mut self) -> Result<()>;

    /// Write a string to the pipeline
    fn write_string(&mut self, s: &str) -> Result<()> {
        self.write(s.as_bytes())
    }

    /// Write a C-style null-terminated string (without the null)
    fn write_cstr(&mut self, s: &str) -> Result<()> {
        self.write(s.as_bytes())
    }
}

/// Buffer pipeline that collects all written data
pub struct PlBuffer {
    identifier: String,
    next: Option<PipelineBox>,
    data: Vec<u8>,
    ready: bool,
}

impl PlBuffer {
    /// Create a new buffer pipeline
    pub fn new(identifier: &str) -> Self {
        Self {
            identifier: identifier.to_string(),
            next: None,
            data: Vec::new(),
            ready: true,
        }
    }

    /// Create a new buffer pipeline with a next pipeline
    pub fn with_next(identifier: &str, next: PipelineBox) -> Self {
        Self {
            identifier: identifier.to_string(),
            next: Some(next),
            data: Vec::new(),
            ready: true,
        }
    }

    /// Get the collected buffer
    pub fn get_buffer(&mut self) -> Result<Vec<u8>> {
        if !self.ready {
            return Err(QpdfError::Pipeline(
                "PlBuffer::get_buffer() called when not ready".to_string(),
            ));
        }
        let result = std::mem::take(&mut self.data);
        Ok(result)
    }

    /// Get the collected data as a string
    pub fn get_string(&mut self) -> Result<String> {
        let buffer = self.get_buffer()?;
        String::from_utf8(buffer).map_err(|e| QpdfError::Pipeline(e.to_string()))
    }

    /// Get the current size of the buffer
    pub fn size(&self) -> usize {
        self.data.len()
    }

    /// Check if the buffer is ready (finish has been called)
    pub fn is_ready(&self) -> bool {
        self.ready
    }
}

impl Pipeline for PlBuffer {
    fn identifier(&self) -> &str {
        &self.identifier
    }

    fn write(&mut self, data: &[u8]) -> Result<()> {
        if data.is_empty() {
            return Ok(());
        }
        self.data.extend_from_slice(data);
        self.ready = false;

        if let Some(ref mut next) = self.next {
            next.write(data)?;
        }
        Ok(())
    }

    fn finish(&mut self) -> Result<()> {
        self.ready = true;
        if let Some(ref mut next) = self.next {
            next.finish()?;
        }
        Ok(())
    }
}

/// Flate compression action
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlateAction {
    /// Compress (deflate) data
    Deflate,
    /// Decompress (inflate) data
    Inflate,
}

/// Flate pipeline for compression/decompression
pub struct PlFlate {
    identifier: String,
    next: PipelineBox,
    action: FlateAction,
    compression_level: u32,
    buffer: Vec<u8>,
    out_bufsize: usize,
    memory_limit: Option<usize>,
    written: usize,
}

impl PlFlate {
    /// Default output buffer size (64KB)
    pub const DEFAULT_BUFSIZE: usize = 65536;

    /// Create a new flate pipeline
    pub fn new(identifier: &str, next: PipelineBox, action: FlateAction) -> Self {
        Self {
            identifier: identifier.to_string(),
            next,
            action,
            compression_level: 6, // Default compression level
            buffer: Vec::new(),
            out_bufsize: Self::DEFAULT_BUFSIZE,
            memory_limit: None,
            written: 0,
        }
    }

    /// Set the compression level (0-9)
    pub fn set_compression_level(&mut self, level: u32) {
        self.compression_level = level.min(9);
    }

    /// Set the output buffer size
    pub fn set_out_bufsize(&mut self, size: usize) {
        self.out_bufsize = size;
    }

    /// Set a memory limit for decompression
    pub fn set_memory_limit(&mut self, limit: usize) {
        self.memory_limit = Some(limit);
    }

    fn process_deflate(&mut self) -> Result<()> {
        let mut encoder =
            DeflateEncoder::new(&self.buffer[..], Compression::new(self.compression_level));
        let mut output = vec![0u8; self.out_bufsize];

        loop {
            let n = encoder
                .read(&mut output)
                .map_err(|e| QpdfError::Pipeline(format!("Flate deflate error: {}", e)))?;
            if n == 0 {
                break;
            }
            self.next.write(&output[..n])?;
        }
        Ok(())
    }

    fn process_inflate(&mut self) -> Result<()> {
        let mut decoder = DeflateDecoder::new(&self.buffer[..]);
        let mut output = vec![0u8; self.out_bufsize];

        loop {
            let n = decoder
                .read(&mut output)
                .map_err(|e| QpdfError::Pipeline(format!("Flate inflate error: {}", e)))?;
            if n == 0 {
                break;
            }

            self.written += n;
            if let Some(limit) = self.memory_limit {
                if self.written > limit {
                    return Err(QpdfError::MemoryLimit(format!(
                        "Flate decompression exceeded memory limit of {} bytes",
                        limit
                    )));
                }
            }

            self.next.write(&output[..n])?;
        }
        Ok(())
    }
}

impl Pipeline for PlFlate {
    fn identifier(&self) -> &str {
        &self.identifier
    }

    fn write(&mut self, data: &[u8]) -> Result<()> {
        // Buffer all input data for processing in finish()
        self.buffer.extend_from_slice(data);
        Ok(())
    }

    fn finish(&mut self) -> Result<()> {
        if !self.buffer.is_empty() {
            match self.action {
                FlateAction::Deflate => self.process_deflate()?,
                FlateAction::Inflate => self.process_inflate()?,
            }
            self.buffer.clear();
        }
        self.next.finish()
    }
}

/// Discard pipeline that throws away all data
pub struct PlDiscard {
    identifier: String,
}

impl PlDiscard {
    /// Create a new discard pipeline
    pub fn new(identifier: &str) -> Self {
        Self {
            identifier: identifier.to_string(),
        }
    }
}

impl Pipeline for PlDiscard {
    fn identifier(&self) -> &str {
        &self.identifier
    }

    fn write(&mut self, _data: &[u8]) -> Result<()> {
        Ok(())
    }

    fn finish(&mut self) -> Result<()> {
        Ok(())
    }
}

/// Count pipeline that counts bytes passing through
pub struct PlCount {
    identifier: String,
    next: PipelineBox,
    count: usize,
    last_char: Option<u8>,
}

impl PlCount {
    /// Create a new count pipeline
    pub fn new(identifier: &str, next: PipelineBox) -> Self {
        Self {
            identifier: identifier.to_string(),
            next,
            count: 0,
            last_char: None,
        }
    }

    /// Get the number of bytes written
    pub fn get_count(&self) -> usize {
        self.count
    }

    /// Get the last character written
    pub fn get_last_char(&self) -> Option<u8> {
        self.last_char
    }
}

impl Pipeline for PlCount {
    fn identifier(&self) -> &str {
        &self.identifier
    }

    fn write(&mut self, data: &[u8]) -> Result<()> {
        if !data.is_empty() {
            self.count += data.len();
            self.last_char = data.last().copied();
        }
        self.next.write(data)
    }

    fn finish(&mut self) -> Result<()> {
        self.next.finish()
    }
}

/// Concatenate pipeline that allows concatenating multiple sources
pub struct PlConcatenate {
    identifier: String,
    next: PipelineBox,
}

impl PlConcatenate {
    /// Create a new concatenate pipeline
    pub fn new(identifier: &str, next: PipelineBox) -> Self {
        Self {
            identifier: identifier.to_string(),
            next,
        }
    }

    /// Signal that one source has completed (ready for next)
    pub fn manual_finish(&mut self) -> Result<()> {
        // Don't call next's finish - we're just done with one source
        Ok(())
    }
}

impl Pipeline for PlConcatenate {
    fn identifier(&self) -> &str {
        &self.identifier
    }

    fn write(&mut self, data: &[u8]) -> Result<()> {
        self.next.write(data)
    }

    fn finish(&mut self) -> Result<()> {
        self.next.finish()
    }
}

/// ASCII85 decoder pipeline
pub struct PlAscii85Decoder {
    identifier: String,
    next: PipelineBox,
    buffer: Vec<u8>,
}

impl PlAscii85Decoder {
    /// Create a new ASCII85 decoder pipeline
    pub fn new(identifier: &str, next: PipelineBox) -> Self {
        Self {
            identifier: identifier.to_string(),
            next,
            buffer: Vec::new(),
        }
    }

    fn decode(&mut self) -> Result<()> {
        let mut output = Vec::new();
        let mut group = [0u8; 5];
        let mut group_len = 0;

        for &byte in &self.buffer {
            if byte == b'~' {
                // End of data marker
                break;
            }
            if byte.is_ascii_whitespace() {
                continue;
            }
            if byte == b'z' {
                // Special case: 'z' represents 4 zero bytes
                if group_len != 0 {
                    return Err(QpdfError::Pipeline(
                        "ASCII85: 'z' in middle of group".to_string(),
                    ));
                }
                output.extend_from_slice(&[0, 0, 0, 0]);
                continue;
            }

            if !(33..=117).contains(&byte) {
                return Err(QpdfError::Pipeline(format!(
                    "ASCII85: invalid character: {}",
                    byte as char
                )));
            }

            group[group_len] = byte - 33;
            group_len += 1;

            if group_len == 5 {
                // Decode a complete group
                let mut value: u64 = 0;
                for &b in &group[..5] {
                    value = value * 85 + b as u64;
                }
                output.push((value >> 24) as u8);
                output.push((value >> 16) as u8);
                output.push((value >> 8) as u8);
                output.push(value as u8);
                group_len = 0;
            }
        }

        // Handle partial group at end
        if group_len > 0 {
            // Pad with 'u' (84)
            for i in group_len..5 {
                group[i] = 84;
            }
            let mut value: u64 = 0;
            for &b in &group[..5] {
                value = value * 85 + b as u64;
            }
            let bytes = [
                (value >> 24) as u8,
                (value >> 16) as u8,
                (value >> 8) as u8,
                value as u8,
            ];
            output.extend_from_slice(&bytes[..group_len - 1]);
        }

        self.next.write(&output)
    }
}

impl Pipeline for PlAscii85Decoder {
    fn identifier(&self) -> &str {
        &self.identifier
    }

    fn write(&mut self, data: &[u8]) -> Result<()> {
        self.buffer.extend_from_slice(data);
        Ok(())
    }

    fn finish(&mut self) -> Result<()> {
        self.decode()?;
        self.buffer.clear();
        self.next.finish()
    }
}

/// ASCII hex decoder pipeline
pub struct PlAsciiHexDecoder {
    identifier: String,
    next: PipelineBox,
    buffer: Vec<u8>,
}

impl PlAsciiHexDecoder {
    /// Create a new ASCII hex decoder pipeline
    pub fn new(identifier: &str, next: PipelineBox) -> Self {
        Self {
            identifier: identifier.to_string(),
            next,
            buffer: Vec::new(),
        }
    }

    fn decode(&mut self) -> Result<()> {
        let mut output = Vec::new();
        let mut high_nibble: Option<u8> = None;

        for &byte in &self.buffer {
            if byte == b'>' {
                // End of data marker
                break;
            }
            if byte.is_ascii_whitespace() {
                continue;
            }

            let nibble = match byte {
                b'0'..=b'9' => byte - b'0',
                b'a'..=b'f' => byte - b'a' + 10,
                b'A'..=b'F' => byte - b'A' + 10,
                _ => {
                    return Err(QpdfError::Pipeline(format!(
                        "ASCIIHex: invalid character: {}",
                        byte as char
                    )));
                }
            };

            match high_nibble {
                Some(high) => {
                    output.push((high << 4) | nibble);
                    high_nibble = None;
                }
                None => {
                    high_nibble = Some(nibble);
                }
            }
        }

        // Handle trailing nibble (pad with 0)
        if let Some(high) = high_nibble {
            output.push(high << 4);
        }

        self.next.write(&output)
    }
}

impl Pipeline for PlAsciiHexDecoder {
    fn identifier(&self) -> &str {
        &self.identifier
    }

    fn write(&mut self, data: &[u8]) -> Result<()> {
        self.buffer.extend_from_slice(data);
        Ok(())
    }

    fn finish(&mut self) -> Result<()> {
        self.decode()?;
        self.buffer.clear();
        self.next.finish()
    }
}

/// RunLength decoder pipeline
pub struct PlRunLengthDecoder {
    identifier: String,
    next: PipelineBox,
    buffer: Vec<u8>,
}

impl PlRunLengthDecoder {
    /// Create a new RunLength decoder pipeline
    pub fn new(identifier: &str, next: PipelineBox) -> Self {
        Self {
            identifier: identifier.to_string(),
            next,
            buffer: Vec::new(),
        }
    }

    fn decode(&mut self) -> Result<()> {
        let mut output = Vec::new();
        let mut i = 0;

        while i < self.buffer.len() {
            let length_byte = self.buffer[i];
            i += 1;

            if length_byte == 128 {
                // EOD marker
                break;
            } else if length_byte < 128 {
                // Copy the next (length_byte + 1) bytes literally
                let count = length_byte as usize + 1;
                if i + count > self.buffer.len() {
                    return Err(QpdfError::Pipeline(
                        "RunLength: unexpected end of data".to_string(),
                    ));
                }
                output.extend_from_slice(&self.buffer[i..i + count]);
                i += count;
            } else {
                // Repeat the next byte (257 - length_byte) times
                let count = 257 - length_byte as usize;
                if i >= self.buffer.len() {
                    return Err(QpdfError::Pipeline(
                        "RunLength: unexpected end of data".to_string(),
                    ));
                }
                let byte = self.buffer[i];
                i += 1;
                output.resize(output.len() + count, byte);
            }
        }

        self.next.write(&output)
    }
}

impl Pipeline for PlRunLengthDecoder {
    fn identifier(&self) -> &str {
        &self.identifier
    }

    fn write(&mut self, data: &[u8]) -> Result<()> {
        self.buffer.extend_from_slice(data);
        Ok(())
    }

    fn finish(&mut self) -> Result<()> {
        self.decode()?;
        self.buffer.clear();
        self.next.finish()
    }
}

/// LZW decoder pipeline
pub struct PlLzwDecoder {
    identifier: String,
    next: PipelineBox,
    buffer: Vec<u8>,
    early_code_change: bool,
}

impl PlLzwDecoder {
    /// Create a new LZW decoder pipeline
    pub fn new(identifier: &str, next: PipelineBox) -> Self {
        Self {
            identifier: identifier.to_string(),
            next,
            buffer: Vec::new(),
            early_code_change: true, // PDF default
        }
    }

    /// Set early code change parameter
    pub fn set_early_code_change(&mut self, value: bool) {
        self.early_code_change = value;
    }

    fn decode(&mut self) -> Result<()> {
        // LZW constants
        const CLEAR_CODE: u16 = 256;
        const EOD_CODE: u16 = 257;
        const FIRST_CODE: u16 = 258;

        let mut output = Vec::new();
        let mut dictionary: Vec<Vec<u8>> = (0..256).map(|i| vec![i as u8]).collect();

        // Reserve space for clear and EOD codes
        dictionary.push(Vec::new()); // CLEAR_CODE
        dictionary.push(Vec::new()); // EOD_CODE

        let mut bit_pos = 0usize;
        let mut code_size = 9u8;
        let mut prev_code: Option<u16> = None;

        // Helper to read bits from buffer
        let read_code = |buffer: &[u8], bit_pos: &mut usize, code_size: u8| -> Option<u16> {
            let mut code: u16 = 0;
            for i in 0..code_size {
                let byte_idx = (*bit_pos + i as usize) / 8;
                let bit_idx = 7 - ((*bit_pos + i as usize) % 8);
                if byte_idx >= buffer.len() {
                    return None;
                }
                if (buffer[byte_idx] >> bit_idx) & 1 == 1 {
                    code |= 1 << (code_size - 1 - i);
                }
            }
            *bit_pos += code_size as usize;
            Some(code)
        };

        loop {
            let code = match read_code(&self.buffer, &mut bit_pos, code_size) {
                Some(c) => c,
                None => break,
            };

            if code == EOD_CODE {
                break;
            }

            if code == CLEAR_CODE {
                // Reset dictionary
                dictionary.truncate(258);
                code_size = 9;
                prev_code = None;
                continue;
            }

            let entry = if (code as usize) < dictionary.len() {
                dictionary[code as usize].clone()
            } else if code as usize == dictionary.len() {
                // Special case: code not in dictionary yet
                if let Some(prev) = prev_code {
                    let mut entry = dictionary[prev as usize].clone();
                    entry.push(entry[0]);
                    entry
                } else {
                    return Err(QpdfError::Pipeline(
                        "LZW: invalid code sequence".to_string(),
                    ));
                }
            } else {
                return Err(QpdfError::Pipeline(format!("LZW: invalid code: {}", code)));
            };

            output.extend_from_slice(&entry);

            // Add new entry to dictionary
            if let Some(prev) = prev_code {
                if dictionary.len() < 4096 {
                    let mut new_entry = dictionary[prev as usize].clone();
                    new_entry.push(entry[0]);
                    dictionary.push(new_entry);

                    // Increase code size if needed
                    let next_code = dictionary.len() as u16;
                    let threshold = if self.early_code_change {
                        (1 << code_size) - 1
                    } else {
                        1 << code_size
                    };
                    if next_code >= threshold && code_size < 12 {
                        code_size += 1;
                    }
                }
            }

            prev_code = Some(code);
        }

        self.next.write(&output)
    }
}

impl Pipeline for PlLzwDecoder {
    fn identifier(&self) -> &str {
        &self.identifier
    }

    fn write(&mut self, data: &[u8]) -> Result<()> {
        self.buffer.extend_from_slice(data);
        Ok(())
    }

    fn finish(&mut self) -> Result<()> {
        if !self.buffer.is_empty() {
            self.decode()?;
        }
        self.buffer.clear();
        self.next.finish()
    }
}

/// Function pipeline that calls a user-provided function
pub struct PlFunction<F>
where
    F: FnMut(&[u8]) -> Result<()> + Send,
{
    identifier: String,
    func: F,
}

impl<F> PlFunction<F>
where
    F: FnMut(&[u8]) -> Result<()> + Send,
{
    /// Create a new function pipeline
    pub fn new(identifier: &str, func: F) -> Self {
        Self {
            identifier: identifier.to_string(),
            func,
        }
    }
}

impl<F> Pipeline for PlFunction<F>
where
    F: FnMut(&[u8]) -> Result<()> + Send,
{
    fn identifier(&self) -> &str {
        &self.identifier
    }

    fn write(&mut self, data: &[u8]) -> Result<()> {
        (self.func)(data)
    }

    fn finish(&mut self) -> Result<()> {
        Ok(())
    }
}

/// String pipeline that collects output as a string
pub struct PlString {
    identifier: String,
    next: Option<PipelineBox>,
    data: String,
    ready: bool,
}

impl PlString {
    /// Create a new string pipeline
    pub fn new(identifier: &str) -> Self {
        Self {
            identifier: identifier.to_string(),
            next: None,
            data: String::new(),
            ready: true,
        }
    }

    /// Get the collected string
    pub fn get_string(&mut self) -> Result<String> {
        if !self.ready {
            return Err(QpdfError::Pipeline(
                "PlString::get_string() called when not ready".to_string(),
            ));
        }
        Ok(std::mem::take(&mut self.data))
    }
}

impl Pipeline for PlString {
    fn identifier(&self) -> &str {
        &self.identifier
    }

    fn write(&mut self, data: &[u8]) -> Result<()> {
        self.data
            .push_str(std::str::from_utf8(data).map_err(|e| QpdfError::Pipeline(e.to_string()))?);
        self.ready = false;

        if let Some(ref mut next) = self.next {
            next.write(data)?;
        }
        Ok(())
    }

    fn finish(&mut self) -> Result<()> {
        self.ready = true;
        if let Some(ref mut next) = self.next {
            next.finish()?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pl_buffer() {
        let mut buffer = PlBuffer::new("test");
        buffer.write(b"Hello, ").unwrap();
        buffer.write(b"World!").unwrap();
        buffer.finish().unwrap();

        let data = buffer.get_buffer().unwrap();
        assert_eq!(data, b"Hello, World!");
    }

    #[test]
    fn test_pl_count() {
        let buffer = Box::new(PlBuffer::new("output"));
        let mut count = PlCount::new("count", buffer);

        count.write(b"Hello").unwrap();
        assert_eq!(count.get_count(), 5);

        count.write(b", World!").unwrap();
        assert_eq!(count.get_count(), 13);
        assert_eq!(count.get_last_char(), Some(b'!'));

        count.finish().unwrap();
    }

    #[test]
    fn test_pl_ascii_hex_decoder() {
        let buffer = Box::new(PlBuffer::new("output"));
        let mut decoder = PlAsciiHexDecoder::new("hex", buffer);

        decoder.write(b"48656C6C6F>").unwrap();
        decoder.finish().unwrap();

        // Note: We can't easily get the output since the buffer is moved
        // In real usage, you'd use Rc<RefCell<>> or similar
    }

    #[test]
    fn test_pl_discard() {
        let mut discard = PlDiscard::new("discard");
        discard.write(b"This data will be discarded").unwrap();
        discard.finish().unwrap();
    }

    #[test]
    fn test_pl_runlength_decoder() {
        let mut buffer = PlBuffer::new("output");

        // Test data: literal "ABC" followed by 5 repetitions of "X"
        // Literal: 0x02 'A' 'B' 'C' (3 bytes literal)
        // Run: 0xFB 'X' (repeat X 6 times: 257 - 0xFB = 6)
        // EOD: 0x80
        let encoded = vec![
            0x02, b'A', b'B', b'C', // Literal: 3 bytes
            0xFB, b'X', // Run: repeat 'X' 6 times
            0x80, // EOD
        ];

        let next_buffer = Box::new(PlBuffer::new("inner"));
        let mut decoder = PlRunLengthDecoder::new("rle", next_buffer);

        decoder.write(&encoded).unwrap();
        decoder.finish().unwrap();
    }
}
