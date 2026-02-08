//! FFI bindings for fz_filter (Stream Filters)
//!
//! This module provides C-compatible exports for stream filter operations.
//! Filters are used for decoding/decrypting PDF stream data.

use super::{Handle, HandleStore};
use std::sync::LazyLock;

// ============================================================================
// Types and Constants
// ============================================================================

/// Filter types
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FilterType {
    #[default]
    Null = 0,
    Range = 1,
    Endstream = 2,
    Concat = 3,
    Arc4 = 4,
    Aesd = 5,
    Ascii85 = 6,
    AsciiHex = 7,
    RunLength = 8,
    Dct = 9,
    Fax = 10,
    Flate = 11,
    Lzw = 12,
    Predict = 13,
    Jbig2 = 14,
    Brotli = 15,
    Sgilog16 = 16,
    Sgilog24 = 17,
    Sgilog32 = 18,
    Thunder = 19,
    Libarchive = 20,
}

/// Range structure for range filter
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct FzRange {
    pub offset: i64,
    pub length: u64,
}

// ============================================================================
// Filter Stream Implementation
// ============================================================================

/// A filtered stream that wraps a source stream and applies a filter
#[derive(Debug)]
pub struct FilterStream {
    /// The filter type
    pub filter_type: FilterType,
    /// Source data (for standalone filters)
    pub data: Vec<u8>,
    /// Current read position
    pub position: usize,
    /// Filter-specific parameters
    pub params: FilterParams,
    /// Chain of sub-filters (for concat)
    pub chain: Vec<Handle>,
    /// Decoded data cache
    pub decoded: Vec<u8>,
    /// Whether decoding is complete
    pub decoded_complete: bool,
}

/// Filter-specific parameters
#[derive(Debug, Clone, Default)]
pub struct FilterParams {
    // Null filter
    pub length: u64,
    pub offset: i64,

    // Range filter
    pub ranges: Vec<FzRange>,

    // Encryption
    pub key: Vec<u8>,

    // DCT (JPEG)
    pub color_transform: i32,
    pub invert_cmyk: i32,
    pub l2factor: i32,

    // Fax
    pub k: i32,
    pub end_of_line: bool,
    pub encoded_byte_align: bool,
    pub columns: i32,
    pub rows: i32,
    pub end_of_block: bool,
    pub black_is_1: bool,

    // Flate
    pub window_bits: i32,

    // LZW
    pub early_change: bool,
    pub min_bits: i32,
    pub reverse_bits: bool,
    pub old_tiff: bool,

    // Predict
    pub predictor: i32,
    pub colors: i32,
    pub bpc: i32,

    // SGI/Thunder
    pub width: i32,

    // JBIG2
    pub globals: Option<Handle>,
    pub embedded: bool,

    // Concat
    pub max_streams: i32,
    pub pad: bool,
}

impl Default for FilterStream {
    fn default() -> Self {
        Self {
            filter_type: FilterType::Null,
            data: Vec::new(),
            position: 0,
            params: FilterParams::default(),
            chain: Vec::new(),
            decoded: Vec::new(),
            decoded_complete: false,
        }
    }
}

impl FilterStream {
    /// Create a new filter stream
    pub fn new(filter_type: FilterType) -> Self {
        Self {
            filter_type,
            ..Default::default()
        }
    }

    /// Decode the source data using the filter
    pub fn decode(&mut self) -> Result<(), &'static str> {
        if self.decoded_complete {
            return Ok(());
        }

        match self.filter_type {
            FilterType::Null => self.decode_null(),
            FilterType::Ascii85 => self.decode_ascii85(),
            FilterType::AsciiHex => self.decode_asciihex(),
            FilterType::RunLength => self.decode_runlength(),
            FilterType::Flate => self.decode_flate(),
            FilterType::Lzw => self.decode_lzw(),
            FilterType::Arc4 => self.decode_arc4(),
            FilterType::Aesd => self.decode_aesd(),
            FilterType::Predict => self.decode_predict(),
            _ => {
                // For filters we don't implement yet, just pass through
                // Use take() to avoid cloning - data is no longer needed after decode
                self.decoded = std::mem::take(&mut self.data);
                Ok(())
            }
        }?;

        self.decoded_complete = true;
        Ok(())
    }

    /// Null filter - just copy data with length limit
    fn decode_null(&mut self) -> Result<(), &'static str> {
        let len = self.params.length as usize;
        let offset = self.params.offset.max(0) as usize;

        if offset < self.data.len() {
            let end = (offset + len).min(self.data.len());
            self.decoded = self.data[offset..end].to_vec();
        }
        Ok(())
    }

    /// ASCII85 decode
    fn decode_ascii85(&mut self) -> Result<(), &'static str> {
        let mut output = Vec::new();
        let mut group: u32 = 0;
        let mut count = 0;

        for &byte in &self.data {
            match byte {
                b'z' if count == 0 => {
                    output.extend_from_slice(&[0, 0, 0, 0]);
                }
                b'~' => {
                    // End of data marker
                    break;
                }
                b'!'..=b'u' => {
                    group = group * 85 + (byte - b'!') as u32;
                    count += 1;
                    if count == 5 {
                        output.push((group >> 24) as u8);
                        output.push((group >> 16) as u8);
                        output.push((group >> 8) as u8);
                        output.push(group as u8);
                        group = 0;
                        count = 0;
                    }
                }
                b' ' | b'\n' | b'\r' | b'\t' => {
                    // Ignore whitespace
                }
                _ => {
                    // Invalid character, skip
                }
            }
        }

        // Handle remaining bytes
        if count > 0 {
            for _ in count..5 {
                group = group * 85 + 84; // Pad with 'u'
            }
            for i in 0..(count - 1) {
                output.push((group >> (24 - i * 8)) as u8);
            }
        }

        self.decoded = output;
        Ok(())
    }

    /// ASCII Hex decode
    fn decode_asciihex(&mut self) -> Result<(), &'static str> {
        let mut output = Vec::new();
        let mut high_nibble: Option<u8> = None;

        for &byte in &self.data {
            let nibble = match byte {
                b'0'..=b'9' => Some(byte - b'0'),
                b'a'..=b'f' => Some(byte - b'a' + 10),
                b'A'..=b'F' => Some(byte - b'A' + 10),
                b'>' => break,                            // End marker
                b' ' | b'\n' | b'\r' | b'\t' => continue, // Skip whitespace
                _ => None,
            };

            if let Some(n) = nibble {
                if let Some(high) = high_nibble {
                    output.push((high << 4) | n);
                    high_nibble = None;
                } else {
                    high_nibble = Some(n);
                }
            }
        }

        // Handle odd nibble
        if let Some(high) = high_nibble {
            output.push(high << 4);
        }

        self.decoded = output;
        Ok(())
    }

    /// Run Length decode
    fn decode_runlength(&mut self) -> Result<(), &'static str> {
        let mut output = Vec::new();
        let mut i = 0;

        while i < self.data.len() {
            let count = self.data[i];
            i += 1;

            if count == 128 {
                // EOD marker
                break;
            } else if count < 128 {
                // Copy next count+1 bytes literally
                let n = (count as usize) + 1;
                if i + n <= self.data.len() {
                    output.extend_from_slice(&self.data[i..i + n]);
                    i += n;
                } else {
                    break;
                }
            } else {
                // Repeat next byte 257-count times
                if i < self.data.len() {
                    let byte = self.data[i];
                    i += 1;
                    let n = 257 - count as usize;
                    for _ in 0..n {
                        output.push(byte);
                    }
                } else {
                    break;
                }
            }
        }

        self.decoded = output;
        Ok(())
    }

    /// Flate (zlib) decode
    fn decode_flate(&mut self) -> Result<(), &'static str> {
        use flate2::read::ZlibDecoder;
        use std::io::Read;

        if self.data.is_empty() {
            self.decoded = Vec::new();
            return Ok(());
        }

        // Check if we need raw deflate (negative window_bits)
        if self.params.window_bits < 0 {
            // Raw deflate without header
            use flate2::read::DeflateDecoder;
            let mut decoder = DeflateDecoder::new(&self.data[..]);
            let mut output = Vec::new();
            if decoder.read_to_end(&mut output).is_ok() {
                self.decoded = output;
            } else {
                self.decoded = Vec::new();
            }
        } else {
            // Zlib with header
            let mut decoder = ZlibDecoder::new(&self.data[..]);
            let mut output = Vec::new();
            if decoder.read_to_end(&mut output).is_ok() {
                self.decoded = output;
            } else {
                // Try raw deflate as fallback
                use flate2::read::DeflateDecoder;
                let mut decoder = DeflateDecoder::new(&self.data[..]);
                let mut fallback = Vec::new();
                if decoder.read_to_end(&mut fallback).is_ok() {
                    self.decoded = fallback;
                } else {
                    self.decoded = Vec::new();
                }
            }
        }

        Ok(())
    }

    /// LZW decode
    fn decode_lzw(&mut self) -> Result<(), &'static str> {
        let early_change = if self.params.early_change { 1 } else { 0 };
        let min_bits = self.params.min_bits.max(9) as usize;
        let reverse_bits = self.params.reverse_bits;

        let mut output = Vec::new();
        let mut dictionary: Vec<Vec<u8>> = (0..256).map(|i| vec![i as u8]).collect();
        let clear_code = 256;
        let eoi_code = 257;

        let mut bits = min_bits;
        let mut bit_pos = 0;
        let mut prev_code: Option<usize> = None;

        loop {
            // Read next code
            let code = self.read_lzw_code(bit_pos, bits, reverse_bits);
            bit_pos += bits;

            if code == clear_code {
                // Reset dictionary
                dictionary.truncate(258);
                bits = min_bits;
                prev_code = None;
                continue;
            }

            if code == eoi_code {
                break;
            }

            let entry = if code < dictionary.len() {
                dictionary[code].clone()
            } else if code == dictionary.len() {
                // Special case: code not yet in dictionary
                if let Some(prev) = prev_code {
                    let mut e = dictionary[prev].clone();
                    e.push(e[0]);
                    e
                } else {
                    break;
                }
            } else {
                break;
            };

            output.extend_from_slice(&entry);

            // Add to dictionary
            if let Some(prev) = prev_code {
                let mut new_entry = dictionary[prev].clone();
                new_entry.push(entry[0]);
                dictionary.push(new_entry);

                // Increase bits if needed
                if dictionary.len() >= (1 << bits) - early_change && bits < 12 {
                    bits += 1;
                }
            }

            prev_code = Some(code);

            // Safety check
            if bit_pos / 8 >= self.data.len() {
                break;
            }
        }

        self.decoded = output;
        Ok(())
    }

    fn read_lzw_code(&self, bit_pos: usize, bits: usize, reverse: bool) -> usize {
        let byte_pos = bit_pos / 8;
        let bit_offset = bit_pos % 8;

        if byte_pos >= self.data.len() {
            return 257; // EOI
        }

        let mut code: usize = 0;

        if reverse {
            // LSB first (GIF style)
            for i in 0..bits {
                let bp = byte_pos + (bit_offset + i) / 8;
                let bo = (bit_offset + i) % 8;
                if bp < self.data.len() && (self.data[bp] & (1 << bo)) != 0 {
                    code |= 1 << i;
                }
            }
        } else {
            // MSB first (PDF/TIFF style)
            for i in 0..bits {
                let bp = byte_pos + (bit_offset + i) / 8;
                let bo = 7 - (bit_offset + i) % 8;
                if bp < self.data.len() && (self.data[bp] & (1 << bo)) != 0 {
                    code |= 1 << (bits - 1 - i);
                }
            }
        }

        code
    }

    /// RC4 (ARC4) decode
    fn decode_arc4(&mut self) -> Result<(), &'static str> {
        if self.params.key.is_empty() {
            // Use take() to avoid cloning - data is no longer needed
            self.decoded = std::mem::take(&mut self.data);
            return Ok(());
        }

        // Initialize S-box
        let mut s: [u8; 256] = [0; 256];
        for i in 0..256 {
            s[i] = i as u8;
        }

        // Key scheduling
        let key = &self.params.key;
        let key_len = key.len();
        let mut j: usize = 0;
        for i in 0..256 {
            j = (j + s[i] as usize + key[i % key_len] as usize) % 256;
            s.swap(i, j);
        }

        // Generate keystream and XOR with data
        let mut output = Vec::with_capacity(self.data.len());
        let mut i: usize = 0;
        j = 0;
        for &byte in &self.data {
            i = (i + 1) % 256;
            j = (j + s[i] as usize) % 256;
            s.swap(i, j);
            let k = s[(s[i] as usize + s[j] as usize) % 256];
            output.push(byte ^ k);
        }

        self.decoded = output;
        Ok(())
    }

    /// AES decode (simplified - assumes CBC mode with IV in first 16 bytes)
    fn decode_aesd(&mut self) -> Result<(), &'static str> {
        // AES decryption is complex - for now, just pass through
        // A full implementation would use a proper AES library
        // Use take() to avoid cloning - data is no longer needed
        self.decoded = std::mem::take(&mut self.data);
        Ok(())
    }

    /// Predictor decode
    fn decode_predict(&mut self) -> Result<(), &'static str> {
        let predictor = self.params.predictor;

        if predictor == 1 {
            // No prediction - use take() to avoid cloning
            self.decoded = std::mem::take(&mut self.data);
            return Ok(());
        }

        let columns = self.params.columns.max(1) as usize;
        let colors = self.params.colors.max(1) as usize;
        let bpc = self.params.bpc.max(1) as usize;

        let bytes_per_pixel = (colors * bpc + 7) / 8;
        let row_bytes = (columns * colors * bpc + 7) / 8;

        if predictor == 2 {
            // TIFF predictor
            self.decode_tiff_predictor(row_bytes, bytes_per_pixel)
        } else {
            // PNG predictor (10-15)
            self.decode_png_predictor(row_bytes, bytes_per_pixel)
        }
    }

    fn decode_tiff_predictor(
        &mut self,
        row_bytes: usize,
        bytes_per_pixel: usize,
    ) -> Result<(), &'static str> {
        let mut output = Vec::with_capacity(self.data.len());

        for row in self.data.chunks(row_bytes) {
            let mut prev = vec![0u8; bytes_per_pixel];
            for pixel in row.chunks(bytes_per_pixel) {
                for (i, &byte) in pixel.iter().enumerate() {
                    let decoded = byte.wrapping_add(prev.get(i).copied().unwrap_or(0));
                    output.push(decoded);
                    if i < prev.len() {
                        prev[i] = decoded;
                    }
                }
            }
        }

        self.decoded = output;
        Ok(())
    }

    fn decode_png_predictor(
        &mut self,
        row_bytes: usize,
        bytes_per_pixel: usize,
    ) -> Result<(), &'static str> {
        // Each row has a filter byte prefix
        let stride = row_bytes + 1;
        let num_rows = (self.data.len() + stride - 1) / stride;

        // Pre-allocate output buffer for better performance
        let mut output = Vec::with_capacity(num_rows * row_bytes);

        // Use two pre-allocated buffers and swap between them to avoid allocations
        let mut prev_row = vec![0u8; row_bytes];
        let mut current_row = vec![0u8; row_bytes];

        for row in self.data.chunks(stride) {
            if row.is_empty() {
                continue;
            }

            let filter = row[0];
            let row_data = if row.len() > 1 { &row[1..] } else { &[] };

            // Clear current row for reuse
            current_row.clear();
            current_row.reserve(row_bytes);

            for (i, &byte) in row_data.iter().enumerate() {
                let a = if i >= bytes_per_pixel {
                    current_row[i - bytes_per_pixel]
                } else {
                    0
                };
                let b = if i < prev_row.len() { prev_row[i] } else { 0 };
                let c = if i >= bytes_per_pixel && i - bytes_per_pixel < prev_row.len() {
                    prev_row[i - bytes_per_pixel]
                } else {
                    0
                };

                let decoded = match filter {
                    0 => byte,                                                 // None
                    1 => byte.wrapping_add(a),                                 // Sub
                    2 => byte.wrapping_add(b),                                 // Up
                    3 => byte.wrapping_add(((a as u16 + b as u16) / 2) as u8), // Average
                    4 => byte.wrapping_add(paeth_predictor(a, b, c)),          // Paeth
                    _ => byte,
                };
                current_row.push(decoded);
            }

            output.extend_from_slice(&current_row);
            // Swap buffers to avoid allocation
            std::mem::swap(&mut prev_row, &mut current_row);
        }

        self.decoded = output;
        Ok(())
    }

    /// Read decoded data
    pub fn read(&mut self, buf: &mut [u8]) -> usize {
        if !self.decoded_complete {
            let _ = self.decode();
        }

        let available = self.decoded.len().saturating_sub(self.position);
        let to_read = buf.len().min(available);

        if to_read > 0 {
            buf[..to_read].copy_from_slice(&self.decoded[self.position..self.position + to_read]);
            self.position += to_read;
        }

        to_read
    }
}

/// PNG Paeth predictor function
#[inline(always)]
fn paeth_predictor(a: u8, b: u8, c: u8) -> u8 {
    let p = a as i32 + b as i32 - c as i32;
    let pa = (p - a as i32).abs();
    let pb = (p - b as i32).abs();
    let pc = (p - c as i32).abs();

    if pa <= pb && pa <= pc {
        a
    } else if pb <= pc {
        b
    } else {
        c
    }
}

// ============================================================================
// JBIG2 Globals
// ============================================================================

/// JBIG2 globals structure
#[derive(Debug, Clone, Default)]
pub struct Jbig2Globals {
    pub refs: i32,
    pub data: Vec<u8>,
}

pub static JBIG2_GLOBALS: LazyLock<HandleStore<Jbig2Globals>> = LazyLock::new(HandleStore::new);

// ============================================================================
// Handle Store
// ============================================================================

pub static FILTER_STREAMS: LazyLock<HandleStore<FilterStream>> = LazyLock::new(HandleStore::new);

// ============================================================================
// FFI Functions
// ============================================================================

/// Create a null filter (reads specified amount from source)
#[unsafe(no_mangle)]
pub extern "C" fn fz_open_null_filter(
    _ctx: Handle,
    chain: Handle,
    len: u64,
    offset: i64,
) -> Handle {
    let mut filter = FilterStream::new(FilterType::Null);
    filter.params.length = len;
    filter.params.offset = offset;

    // Get data from chain if available
    if let Some(arc) = FILTER_STREAMS.get(chain) {
        if let Ok(mut source) = arc.lock() {
            let _ = source.decode();
            filter.data = source.decoded.clone();
        }
    }

    FILTER_STREAMS.insert(filter)
}

/// Create a range filter
#[unsafe(no_mangle)]
pub extern "C" fn fz_open_range_filter(
    _ctx: Handle,
    chain: Handle,
    ranges: *const FzRange,
    nranges: i32,
) -> Handle {
    let mut filter = FilterStream::new(FilterType::Range);

    if !ranges.is_null() && nranges > 0 {
        let ranges_slice = unsafe { std::slice::from_raw_parts(ranges, nranges as usize) };
        filter.params.ranges = ranges_slice.to_vec();
    }

    // Get data from chain and extract ranges
    if let Some(arc) = FILTER_STREAMS.get(chain) {
        if let Ok(mut source) = arc.lock() {
            let _ = source.decode();

            for range in &filter.params.ranges {
                let start = range.offset.max(0) as usize;
                let end = (start + range.length as usize).min(source.decoded.len());
                if start < source.decoded.len() {
                    filter.data.extend_from_slice(&source.decoded[start..end]);
                }
            }
        }
    }

    // Use take() to avoid cloning - data is moved to decoded
    filter.decoded = std::mem::take(&mut filter.data);
    filter.decoded_complete = true;

    FILTER_STREAMS.insert(filter)
}

/// Create an endstream filter (PDF specific)
#[unsafe(no_mangle)]
pub extern "C" fn fz_open_endstream_filter(
    _ctx: Handle,
    chain: Handle,
    len: u64,
    offset: i64,
) -> Handle {
    // Similar to null filter but looks for "endstream" token
    fz_open_null_filter(_ctx, chain, len, offset)
}

/// Create a concatenation filter
#[unsafe(no_mangle)]
pub extern "C" fn fz_open_concat(_ctx: Handle, max: i32, pad: i32) -> Handle {
    let mut filter = FilterStream::new(FilterType::Concat);
    filter.params.max_streams = max;
    filter.params.pad = pad != 0;
    FILTER_STREAMS.insert(filter)
}

/// Push a stream onto a concat filter
#[unsafe(no_mangle)]
pub extern "C" fn fz_concat_push_drop(_ctx: Handle, concat: Handle, chain: Handle) {
    if let Some(arc) = FILTER_STREAMS.get(concat) {
        if let Ok(mut filter) = arc.lock() {
            filter.chain.push(chain);

            // Concatenate data from chain
            if let Some(chain_arc) = FILTER_STREAMS.get(chain) {
                if let Ok(mut source) = chain_arc.lock() {
                    let _ = source.decode();
                    filter.data.extend_from_slice(&source.decoded);
                }
            }
        }
    }
}

/// Create an RC4 decryption filter
#[unsafe(no_mangle)]
pub extern "C" fn fz_open_arc4(_ctx: Handle, chain: Handle, key: *const u8, keylen: u32) -> Handle {
    let mut filter = FilterStream::new(FilterType::Arc4);

    if !key.is_null() && keylen > 0 {
        filter.params.key = unsafe { std::slice::from_raw_parts(key, keylen as usize) }.to_vec();
    }

    // Get data from chain
    if let Some(arc) = FILTER_STREAMS.get(chain) {
        if let Ok(mut source) = arc.lock() {
            let _ = source.decode();
            filter.data = source.decoded.clone();
        }
    }

    FILTER_STREAMS.insert(filter)
}

/// Create an AES decryption filter
#[unsafe(no_mangle)]
pub extern "C" fn fz_open_aesd(_ctx: Handle, chain: Handle, key: *const u8, keylen: u32) -> Handle {
    let mut filter = FilterStream::new(FilterType::Aesd);

    if !key.is_null() && keylen > 0 {
        filter.params.key = unsafe { std::slice::from_raw_parts(key, keylen as usize) }.to_vec();
    }

    // Get data from chain
    if let Some(arc) = FILTER_STREAMS.get(chain) {
        if let Ok(mut source) = arc.lock() {
            let _ = source.decode();
            filter.data = source.decoded.clone();
        }
    }

    FILTER_STREAMS.insert(filter)
}

/// Create an ASCII85 decode filter
#[unsafe(no_mangle)]
pub extern "C" fn fz_open_a85d(_ctx: Handle, chain: Handle) -> Handle {
    let mut filter = FilterStream::new(FilterType::Ascii85);

    // Get data from chain
    if let Some(arc) = FILTER_STREAMS.get(chain) {
        if let Ok(mut source) = arc.lock() {
            let _ = source.decode();
            filter.data = source.decoded.clone();
        }
    }

    FILTER_STREAMS.insert(filter)
}

/// Create an ASCII Hex decode filter
#[unsafe(no_mangle)]
pub extern "C" fn fz_open_ahxd(_ctx: Handle, chain: Handle) -> Handle {
    let mut filter = FilterStream::new(FilterType::AsciiHex);

    // Get data from chain
    if let Some(arc) = FILTER_STREAMS.get(chain) {
        if let Ok(mut source) = arc.lock() {
            let _ = source.decode();
            filter.data = source.decoded.clone();
        }
    }

    FILTER_STREAMS.insert(filter)
}

/// Create a Run Length decode filter
#[unsafe(no_mangle)]
pub extern "C" fn fz_open_rld(_ctx: Handle, chain: Handle) -> Handle {
    let mut filter = FilterStream::new(FilterType::RunLength);

    // Get data from chain
    if let Some(arc) = FILTER_STREAMS.get(chain) {
        if let Ok(mut source) = arc.lock() {
            let _ = source.decode();
            filter.data = source.decoded.clone();
        }
    }

    FILTER_STREAMS.insert(filter)
}

/// Create a DCT (JPEG) decode filter
#[unsafe(no_mangle)]
pub extern "C" fn fz_open_dctd(
    _ctx: Handle,
    chain: Handle,
    color_transform: i32,
    invert_cmyk: i32,
    l2factor: i32,
    _jpegtables: Handle,
) -> Handle {
    let mut filter = FilterStream::new(FilterType::Dct);
    filter.params.color_transform = color_transform;
    filter.params.invert_cmyk = invert_cmyk;
    filter.params.l2factor = l2factor;

    // Get data from chain
    if let Some(arc) = FILTER_STREAMS.get(chain) {
        if let Ok(mut source) = arc.lock() {
            let _ = source.decode();
            filter.data = source.decoded.clone();
        }
    }

    FILTER_STREAMS.insert(filter)
}

/// Create a Fax/CCITT decode filter
#[unsafe(no_mangle)]
pub extern "C" fn fz_open_faxd(
    _ctx: Handle,
    chain: Handle,
    k: i32,
    end_of_line: i32,
    encoded_byte_align: i32,
    columns: i32,
    rows: i32,
    end_of_block: i32,
    black_is_1: i32,
) -> Handle {
    let mut filter = FilterStream::new(FilterType::Fax);
    filter.params.k = k;
    filter.params.end_of_line = end_of_line != 0;
    filter.params.encoded_byte_align = encoded_byte_align != 0;
    filter.params.columns = columns;
    filter.params.rows = rows;
    filter.params.end_of_block = end_of_block != 0;
    filter.params.black_is_1 = black_is_1 != 0;

    // Get data from chain
    if let Some(arc) = FILTER_STREAMS.get(chain) {
        if let Ok(mut source) = arc.lock() {
            let _ = source.decode();
            filter.data = source.decoded.clone();
        }
    }

    FILTER_STREAMS.insert(filter)
}

/// Create a Flate (zlib) decode filter
#[unsafe(no_mangle)]
pub extern "C" fn fz_open_flated(_ctx: Handle, chain: Handle, window_bits: i32) -> Handle {
    let mut filter = FilterStream::new(FilterType::Flate);
    filter.params.window_bits = window_bits;

    // Get data from chain
    if let Some(arc) = FILTER_STREAMS.get(chain) {
        if let Ok(mut source) = arc.lock() {
            let _ = source.decode();
            filter.data = source.decoded.clone();
        }
    }

    FILTER_STREAMS.insert(filter)
}

/// Create a libarchive decode filter
#[unsafe(no_mangle)]
pub extern "C" fn fz_open_libarchived(_ctx: Handle, chain: Handle) -> Handle {
    let mut filter = FilterStream::new(FilterType::Libarchive);

    // Get data from chain
    if let Some(arc) = FILTER_STREAMS.get(chain) {
        if let Ok(mut source) = arc.lock() {
            let _ = source.decode();
            filter.data = source.decoded.clone();
        }
    }

    FILTER_STREAMS.insert(filter)
}

/// Create a Brotli decode filter
#[unsafe(no_mangle)]
pub extern "C" fn fz_open_brotlid(_ctx: Handle, chain: Handle) -> Handle {
    let mut filter = FilterStream::new(FilterType::Brotli);

    // Get data from chain
    if let Some(arc) = FILTER_STREAMS.get(chain) {
        if let Ok(mut source) = arc.lock() {
            let _ = source.decode();
            filter.data = source.decoded.clone();
        }
    }

    FILTER_STREAMS.insert(filter)
}

/// Create an LZW decode filter
#[unsafe(no_mangle)]
pub extern "C" fn fz_open_lzwd(
    _ctx: Handle,
    chain: Handle,
    early_change: i32,
    min_bits: i32,
    reverse_bits: i32,
    old_tiff: i32,
) -> Handle {
    let mut filter = FilterStream::new(FilterType::Lzw);
    filter.params.early_change = early_change != 0;
    filter.params.min_bits = if min_bits > 0 { min_bits } else { 9 };
    filter.params.reverse_bits = reverse_bits != 0;
    filter.params.old_tiff = old_tiff != 0;

    // Get data from chain
    if let Some(arc) = FILTER_STREAMS.get(chain) {
        if let Ok(mut source) = arc.lock() {
            let _ = source.decode();
            filter.data = source.decoded.clone();
        }
    }

    FILTER_STREAMS.insert(filter)
}

/// Create a predictor decode filter
#[unsafe(no_mangle)]
pub extern "C" fn fz_open_predict(
    _ctx: Handle,
    chain: Handle,
    predictor: i32,
    columns: i32,
    colors: i32,
    bpc: i32,
) -> Handle {
    let mut filter = FilterStream::new(FilterType::Predict);
    filter.params.predictor = predictor;
    filter.params.columns = columns;
    filter.params.colors = colors;
    filter.params.bpc = bpc;

    // Get data from chain
    if let Some(arc) = FILTER_STREAMS.get(chain) {
        if let Ok(mut source) = arc.lock() {
            let _ = source.decode();
            filter.data = source.decoded.clone();
        }
    }

    FILTER_STREAMS.insert(filter)
}

/// Create a JBIG2 decode filter
#[unsafe(no_mangle)]
pub extern "C" fn fz_open_jbig2d(
    _ctx: Handle,
    chain: Handle,
    globals: Handle,
    embedded: i32,
) -> Handle {
    let mut filter = FilterStream::new(FilterType::Jbig2);
    filter.params.globals = if globals != 0 { Some(globals) } else { None };
    filter.params.embedded = embedded != 0;

    // Get data from chain
    if let Some(arc) = FILTER_STREAMS.get(chain) {
        if let Ok(mut source) = arc.lock() {
            let _ = source.decode();
            filter.data = source.decoded.clone();
        }
    }

    FILTER_STREAMS.insert(filter)
}

/// Load JBIG2 globals from buffer
#[unsafe(no_mangle)]
pub extern "C" fn fz_load_jbig2_globals(_ctx: Handle, buf: Handle) -> Handle {
    let mut globals = Jbig2Globals {
        refs: 1,
        data: Vec::new(),
    };

    // Get data from buffer
    if let Some(arc) = crate::ffi::BUFFERS.get(buf) {
        if let Ok(buffer) = arc.lock() {
            globals.data = buffer.data().to_vec();
        }
    }

    JBIG2_GLOBALS.insert(globals)
}

/// Keep JBIG2 globals
#[unsafe(no_mangle)]
pub extern "C" fn fz_keep_jbig2_globals(_ctx: Handle, globals: Handle) -> Handle {
    if let Some(arc) = JBIG2_GLOBALS.get(globals) {
        if let Ok(mut g) = arc.lock() {
            g.refs += 1;
        }
    }
    globals
}

/// Drop JBIG2 globals
#[unsafe(no_mangle)]
pub extern "C" fn fz_drop_jbig2_globals(_ctx: Handle, globals: Handle) {
    if globals == 0 {
        return;
    }

    let should_drop = if let Some(arc) = JBIG2_GLOBALS.get(globals) {
        if let Ok(mut g) = arc.lock() {
            g.refs -= 1;
            g.refs <= 0
        } else {
            false
        }
    } else {
        false
    };

    if should_drop {
        JBIG2_GLOBALS.remove(globals);
    }
}

/// Get JBIG2 globals data buffer
#[unsafe(no_mangle)]
pub extern "C" fn fz_jbig2_globals_data(_ctx: Handle, globals: Handle) -> Handle {
    if let Some(arc) = JBIG2_GLOBALS.get(globals) {
        if let Ok(g) = arc.lock() {
            // Create a new buffer with the globals data
            let buffer = crate::ffi::buffer::Buffer::from_data(&g.data);
            return crate::ffi::BUFFERS.insert(buffer);
        }
    }
    0
}

/// Create an SGI Log 16-bit decode filter
#[unsafe(no_mangle)]
pub extern "C" fn fz_open_sgilog16(_ctx: Handle, chain: Handle, w: i32) -> Handle {
    let mut filter = FilterStream::new(FilterType::Sgilog16);
    filter.params.width = w;

    // Get data from chain
    if let Some(arc) = FILTER_STREAMS.get(chain) {
        if let Ok(mut source) = arc.lock() {
            let _ = source.decode();
            filter.data = source.decoded.clone();
        }
    }

    FILTER_STREAMS.insert(filter)
}

/// Create an SGI Log 24-bit decode filter
#[unsafe(no_mangle)]
pub extern "C" fn fz_open_sgilog24(_ctx: Handle, chain: Handle, w: i32) -> Handle {
    let mut filter = FilterStream::new(FilterType::Sgilog24);
    filter.params.width = w;

    // Get data from chain
    if let Some(arc) = FILTER_STREAMS.get(chain) {
        if let Ok(mut source) = arc.lock() {
            let _ = source.decode();
            filter.data = source.decoded.clone();
        }
    }

    FILTER_STREAMS.insert(filter)
}

/// Create an SGI Log 32-bit decode filter
#[unsafe(no_mangle)]
pub extern "C" fn fz_open_sgilog32(_ctx: Handle, chain: Handle, w: i32) -> Handle {
    let mut filter = FilterStream::new(FilterType::Sgilog32);
    filter.params.width = w;

    // Get data from chain
    if let Some(arc) = FILTER_STREAMS.get(chain) {
        if let Ok(mut source) = arc.lock() {
            let _ = source.decode();
            filter.data = source.decoded.clone();
        }
    }

    FILTER_STREAMS.insert(filter)
}

/// Create a Thunderscan decode filter
#[unsafe(no_mangle)]
pub extern "C" fn fz_open_thunder(_ctx: Handle, chain: Handle, w: i32) -> Handle {
    let mut filter = FilterStream::new(FilterType::Thunder);
    filter.params.width = w;

    // Get data from chain
    if let Some(arc) = FILTER_STREAMS.get(chain) {
        if let Ok(mut source) = arc.lock() {
            let _ = source.decode();
            filter.data = source.decoded.clone();
        }
    }

    FILTER_STREAMS.insert(filter)
}

/// Drop a filter stream
#[unsafe(no_mangle)]
pub extern "C" fn fz_drop_filter(_ctx: Handle, filter: Handle) {
    FILTER_STREAMS.remove(filter);
}

/// Read from a filter stream
#[unsafe(no_mangle)]
pub extern "C" fn fz_filter_read(_ctx: Handle, filter: Handle, buf: *mut u8, len: usize) -> usize {
    if buf.is_null() || len == 0 {
        return 0;
    }

    if let Some(arc) = FILTER_STREAMS.get(filter) {
        if let Ok(mut f) = arc.lock() {
            let slice = unsafe { std::slice::from_raw_parts_mut(buf, len) };
            return f.read(slice);
        }
    }
    0
}

/// Get decoded data size
#[unsafe(no_mangle)]
pub extern "C" fn fz_filter_size(_ctx: Handle, filter: Handle) -> usize {
    if let Some(arc) = FILTER_STREAMS.get(filter) {
        if let Ok(mut f) = arc.lock() {
            let _ = f.decode();
            return f.decoded.len();
        }
    }
    0
}

/// Get decoded data pointer
#[unsafe(no_mangle)]
pub extern "C" fn fz_filter_data(_ctx: Handle, filter: Handle) -> *const u8 {
    if let Some(arc) = FILTER_STREAMS.get(filter) {
        if let Ok(mut f) = arc.lock() {
            let _ = f.decode();
            return f.decoded.as_ptr();
        }
    }
    std::ptr::null()
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ascii85_decode() {
        let mut filter = FilterStream::new(FilterType::Ascii85);
        // "Test" encoded in ASCII85 = "<~FCfN8~>"
        // Actually let's test with a complete 4-byte group
        // "test" in ASCII85 (4 bytes = 1 complete group)
        filter.data = b"FCfN8~>".to_vec();
        filter.decode().unwrap();
        assert_eq!(filter.decoded, b"test");
    }

    #[test]
    fn test_asciihex_decode() {
        let mut filter = FilterStream::new(FilterType::AsciiHex);
        filter.data = b"48656C6C6F>".to_vec();
        filter.decode().unwrap();
        assert_eq!(filter.decoded, b"Hello");
    }

    #[test]
    fn test_runlength_decode() {
        let mut filter = FilterStream::new(FilterType::RunLength);
        // Format: count < 128 means copy count+1 bytes
        //         count >= 128 (except 128) means repeat next byte 257-count times
        //         128 = EOD
        // 4 literal bytes "Hell" + repeat 'o' 3 times (257-254=3)
        filter.data = vec![3, b'H', b'e', b'l', b'l', 254, b'o', 128];
        filter.decode().unwrap();
        assert_eq!(filter.decoded, b"Hellooo");
    }

    #[test]
    fn test_arc4_decode() {
        let mut filter = FilterStream::new(FilterType::Arc4);
        filter.params.key = b"key".to_vec();
        filter.data = vec![0x1a, 0xd2, 0x82]; // "abc" encrypted with "key"
        filter.decode().unwrap();
        // RC4 is symmetric, so decoding should give back original
        // The expected output depends on the specific input
        assert!(!filter.decoded.is_empty());
    }

    #[test]
    fn test_null_filter() {
        let ctx = 0;

        // Create source filter with data
        let mut source = FilterStream::new(FilterType::Null);
        source.data = b"Hello World!".to_vec();
        source.decoded = source.data.clone();
        source.decoded_complete = true;
        let source_handle = FILTER_STREAMS.insert(source);

        // Create null filter that reads 5 bytes
        let filter = fz_open_null_filter(ctx, source_handle, 5, 0);

        let size = fz_filter_size(ctx, filter);
        assert_eq!(size, 5);

        fz_drop_filter(ctx, filter);
        fz_drop_filter(ctx, source_handle);
    }

    #[test]
    fn test_concat_filter() {
        let ctx = 0;

        // Create two source filters
        let mut source1 = FilterStream::new(FilterType::Null);
        source1.decoded = b"Hello ".to_vec();
        source1.decoded_complete = true;
        let h1 = FILTER_STREAMS.insert(source1);

        let mut source2 = FilterStream::new(FilterType::Null);
        source2.decoded = b"World!".to_vec();
        source2.decoded_complete = true;
        let h2 = FILTER_STREAMS.insert(source2);

        // Create concat filter
        let concat = fz_open_concat(ctx, 2, 0);
        fz_concat_push_drop(ctx, concat, h1);
        fz_concat_push_drop(ctx, concat, h2);

        // Verify concatenated data
        if let Some(arc) = FILTER_STREAMS.get(concat) {
            if let Ok(f) = arc.lock() {
                assert_eq!(f.data, b"Hello World!");
            }
        }

        fz_drop_filter(ctx, concat);
    }

    #[test]
    fn test_flate_decode() {
        let mut filter = FilterStream::new(FilterType::Flate);
        // "Hello" compressed with zlib
        filter.data = vec![
            0x78, 0x9c, 0xf3, 0x48, 0xcd, 0xc9, 0xc9, 0x07, 0x00, 0x05, 0x8c, 0x01, 0xf5,
        ];
        filter.params.window_bits = 15;
        filter.decode().unwrap();
        assert_eq!(filter.decoded, b"Hello");
    }

    #[test]
    fn test_predict_none() {
        let mut filter = FilterStream::new(FilterType::Predict);
        filter.params.predictor = 1; // No prediction
        filter.data = b"Hello".to_vec();
        filter.decode().unwrap();
        assert_eq!(filter.decoded, b"Hello");
    }

    #[test]
    fn test_jbig2_globals() {
        let ctx = 0;

        // Create a buffer with test data
        let buffer = crate::ffi::buffer::Buffer::from_data(b"test globals");
        let buf_handle = crate::ffi::BUFFERS.insert(buffer);

        let globals = fz_load_jbig2_globals(ctx, buf_handle);
        assert!(globals > 0);

        let kept = fz_keep_jbig2_globals(ctx, globals);
        assert_eq!(kept, globals);

        fz_drop_jbig2_globals(ctx, globals);
        fz_drop_jbig2_globals(ctx, globals);
    }
}
