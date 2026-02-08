//! FFI bindings for fz_compress (Compression)
//!
//! This module provides compression functions including zlib deflate,
//! brotli compression, and CCITT Fax Group 3/4 encoding.

use std::io::{Read, Write};
use std::sync::LazyLock;

use crate::ffi::buffer::Buffer;
use crate::ffi::{BUFFERS, Handle, HandleStore};

// ============================================================================
// Deflate Compression Level
// ============================================================================

/// Deflate compression level enumeration
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeflateLevel {
    /// No compression
    None = 0,
    /// Fastest compression
    BestSpeed = 1,
    /// Best compression ratio
    Best = 9,
    /// Default compression level
    Default = -1,
}

impl DeflateLevel {
    /// Convert to flate2 compression level
    pub fn to_flate2_level(self) -> flate2::Compression {
        match self {
            DeflateLevel::None => flate2::Compression::none(),
            DeflateLevel::BestSpeed => flate2::Compression::fast(),
            DeflateLevel::Best => flate2::Compression::best(),
            DeflateLevel::Default => flate2::Compression::default(),
        }
    }

    /// Create from integer value
    pub fn from_i32(value: i32) -> Self {
        match value {
            0 => DeflateLevel::None,
            1 => DeflateLevel::BestSpeed,
            9 => DeflateLevel::Best,
            _ => DeflateLevel::Default,
        }
    }
}

// ============================================================================
// Brotli Compression Level
// ============================================================================

/// Brotli compression level enumeration
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrotliLevel {
    /// No compression
    None = 0,
    /// Fastest compression
    BestSpeed = 1,
    /// Best compression ratio
    Best = 11,
    /// Default compression level
    Default = 6,
}

impl BrotliLevel {
    /// Convert to u32 for brotli crate
    pub fn to_u32(self) -> u32 {
        match self {
            BrotliLevel::None => 0,
            BrotliLevel::BestSpeed => 1,
            BrotliLevel::Best => 11,
            BrotliLevel::Default => 6,
        }
    }

    /// Create from integer value
    pub fn from_i32(value: i32) -> Self {
        match value {
            0 => BrotliLevel::None,
            1 => BrotliLevel::BestSpeed,
            11 => BrotliLevel::Best,
            _ => BrotliLevel::Default,
        }
    }
}

// ============================================================================
// Image Type Constants
// ============================================================================

/// Image type enumeration for compressed buffers
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ImageType {
    /// Unknown image type
    #[default]
    Unknown = 0,
    /// Uncompressed raw samples
    Raw = 1,
    /// CCITT Fax encoded
    Fax = 2,
    /// Flate/zlib compressed
    Flate = 3,
    /// LZW compressed
    Lzw = 4,
    /// Run-length decoded
    Rld = 5,
    /// Brotli compressed
    Brotli = 6,
    /// BMP image
    Bmp = 7,
    /// GIF image
    Gif = 8,
    /// JBIG2 image
    Jbig2 = 9,
    /// JPEG image
    Jpeg = 10,
    /// JPEG 2000 image
    Jpx = 11,
    /// JPEG XR image
    Jxr = 12,
    /// PNG image
    Png = 13,
    /// PNM (PPM/PGM/PBM) image
    Pnm = 14,
    /// TIFF image
    Tiff = 15,
    /// PSD (Photoshop) image
    Psd = 16,
}

impl ImageType {
    /// Get name for image type
    pub fn name(&self) -> &'static str {
        match self {
            ImageType::Unknown => "unknown",
            ImageType::Raw => "raw",
            ImageType::Fax => "fax",
            ImageType::Flate => "flate",
            ImageType::Lzw => "lzw",
            ImageType::Rld => "rld",
            ImageType::Brotli => "brotli",
            ImageType::Bmp => "bmp",
            ImageType::Gif => "gif",
            ImageType::Jbig2 => "jbig2",
            ImageType::Jpeg => "jpeg",
            ImageType::Jpx => "jpx",
            ImageType::Jxr => "jxr",
            ImageType::Png => "png",
            ImageType::Pnm => "pnm",
            ImageType::Tiff => "tiff",
            ImageType::Psd => "psd",
        }
    }

    /// Lookup image type from name
    pub fn from_name(name: &str) -> Self {
        match name.to_lowercase().as_str() {
            "raw" => ImageType::Raw,
            "fax" | "ccitt" => ImageType::Fax,
            "flate" | "deflate" | "zlib" => ImageType::Flate,
            "lzw" => ImageType::Lzw,
            "rld" | "runlength" => ImageType::Rld,
            "brotli" => ImageType::Brotli,
            "bmp" => ImageType::Bmp,
            "gif" => ImageType::Gif,
            "jbig2" => ImageType::Jbig2,
            "jpeg" | "jpg" => ImageType::Jpeg,
            "jpx" | "jp2" | "jpeg2000" => ImageType::Jpx,
            "jxr" => ImageType::Jxr,
            "png" => ImageType::Png,
            "pnm" | "pbm" | "pgm" | "ppm" => ImageType::Pnm,
            "tiff" | "tif" => ImageType::Tiff,
            "psd" => ImageType::Psd,
            _ => ImageType::Unknown,
        }
    }

    /// Convert from integer
    pub fn from_i32(value: i32) -> Self {
        match value {
            1 => ImageType::Raw,
            2 => ImageType::Fax,
            3 => ImageType::Flate,
            4 => ImageType::Lzw,
            5 => ImageType::Rld,
            6 => ImageType::Brotli,
            7 => ImageType::Bmp,
            8 => ImageType::Gif,
            9 => ImageType::Jbig2,
            10 => ImageType::Jpeg,
            11 => ImageType::Jpx,
            12 => ImageType::Jxr,
            13 => ImageType::Png,
            14 => ImageType::Pnm,
            15 => ImageType::Tiff,
            16 => ImageType::Psd,
            _ => ImageType::Unknown,
        }
    }
}

// ============================================================================
// Compression Parameters
// ============================================================================

/// Compression parameters for compressed buffers
#[repr(C)]
#[derive(Debug, Clone, Default)]
pub struct CompressionParams {
    /// Compression type
    pub image_type: ImageType,
    /// JPEG color transform (-1 for unset)
    pub jpeg_color_transform: i32,
    /// JPEG invert CMYK
    pub jpeg_invert_cmyk: i32,
    /// JPX smask in data
    pub jpx_smask_in_data: i32,
    /// Fax columns
    pub fax_columns: i32,
    /// Fax rows
    pub fax_rows: i32,
    /// Fax K parameter
    pub fax_k: i32,
    /// Fax end of line
    pub fax_end_of_line: i32,
    /// Fax encoded byte align
    pub fax_encoded_byte_align: i32,
    /// Fax end of block
    pub fax_end_of_block: i32,
    /// Fax black is 1
    pub fax_black_is_1: i32,
    /// Flate/Brotli/LZW columns
    pub predictor_columns: i32,
    /// Flate/Brotli/LZW colors
    pub predictor_colors: i32,
    /// Flate/Brotli/LZW predictor
    pub predictor: i32,
    /// Bits per component
    pub bpc: i32,
    /// LZW early change
    pub lzw_early_change: i32,
}

// ============================================================================
// Compressed Buffer
// ============================================================================

/// Compressed buffer structure
#[derive(Debug, Clone, Default)]
pub struct CompressedBuffer {
    /// Reference count
    pub refs: i32,
    /// Compression parameters
    pub params: CompressionParams,
    /// Buffer handle
    pub buffer: Handle,
}

/// Global store for compressed buffers
pub static COMPRESSED_BUFFERS: LazyLock<HandleStore<CompressedBuffer>> =
    LazyLock::new(HandleStore::new);

// ============================================================================
// FFI Functions - Deflate
// ============================================================================

/// Returns the upper bound on the size of deflated data
#[unsafe(no_mangle)]
pub extern "C" fn fz_deflate_bound(_ctx: Handle, size: usize) -> usize {
    // zlib bound formula: size + (size >> 12) + (size >> 14) + (size >> 25) + 13
    // Use a simpler conservative estimate
    size + (size / 1000) + 12 + 6
}

/// Compress data using deflate
///
/// # Safety
/// Caller must ensure:
/// - `dest` points to valid writable memory of at least `*compressed_length` bytes
/// - `source` points to valid readable memory of at least `source_length` bytes
/// - `compressed_length` points to valid writable memory
#[unsafe(no_mangle)]
pub extern "C" fn fz_deflate(
    _ctx: Handle,
    dest: *mut u8,
    compressed_length: *mut usize,
    source: *const u8,
    source_length: usize,
    level: i32,
) {
    if dest.is_null() || compressed_length.is_null() || source.is_null() {
        return;
    }

    let deflate_level = DeflateLevel::from_i32(level);
    let source_slice = unsafe { std::slice::from_raw_parts(source, source_length) };
    let dest_capacity = unsafe { *compressed_length };

    let mut encoder = flate2::write::ZlibEncoder::new(
        Vec::with_capacity(dest_capacity),
        deflate_level.to_flate2_level(),
    );

    if encoder.write_all(source_slice).is_ok() {
        if let Ok(compressed) = encoder.finish() {
            let len = compressed.len().min(dest_capacity);
            unsafe {
                std::ptr::copy_nonoverlapping(compressed.as_ptr(), dest, len);
                *compressed_length = len;
            }
            return;
        }
    }

    unsafe {
        *compressed_length = 0;
    }
}

/// Compress data and return new allocated buffer
///
/// # Safety
/// Caller must ensure:
/// - `source` points to valid readable memory of at least `source_length` bytes
/// - `compressed_length` points to valid writable memory
/// - The returned pointer must be freed by the caller
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_deflated_data(
    _ctx: Handle,
    compressed_length: *mut usize,
    source: *const u8,
    source_length: usize,
    level: i32,
) -> *mut u8 {
    if compressed_length.is_null() || source.is_null() {
        return std::ptr::null_mut();
    }

    let deflate_level = DeflateLevel::from_i32(level);
    let source_slice = unsafe { std::slice::from_raw_parts(source, source_length) };

    let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), deflate_level.to_flate2_level());

    if encoder.write_all(source_slice).is_ok() {
        if let Ok(compressed) = encoder.finish() {
            let len = compressed.len();
            let ptr = compressed.as_ptr() as *mut u8;

            // SAFETY: We deliberately leak this Vec to transfer ownership to the C caller.
            // The caller is responsible for freeing this memory. Memory layout:
            // - Contiguous array of `len` bytes at `ptr`
            // - Allocated via Rust's global allocator
            // To properly deallocate from Rust: Vec::from_raw_parts(ptr, len, len)
            // Or call fz_free_compressed_data() which handles this cleanup.
            std::mem::forget(compressed);

            // SAFETY: compressed_length was checked for null at function entry
            unsafe {
                *compressed_length = len;
            }
            return ptr;
        }
    }

    unsafe {
        *compressed_length = 0;
    }
    std::ptr::null_mut()
}

/// Compress buffer contents using deflate
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_deflated_data_from_buffer(
    _ctx: Handle,
    compressed_length: *mut usize,
    buffer: Handle,
    level: i32,
) -> *mut u8 {
    if compressed_length.is_null() {
        return std::ptr::null_mut();
    }

    let buf_arc = match BUFFERS.get(buffer) {
        Some(b) => b,
        None => {
            unsafe {
                *compressed_length = 0;
            }
            return std::ptr::null_mut();
        }
    };

    let buf_guard = buf_arc.lock().unwrap();
    let data = buf_guard.data();

    fz_new_deflated_data(_ctx, compressed_length, data.as_ptr(), data.len(), level)
}

/// Compress data into a buffer handle
#[unsafe(no_mangle)]
pub extern "C" fn fz_deflate_to_buffer(
    _ctx: Handle,
    source: *const u8,
    source_length: usize,
    level: i32,
) -> Handle {
    if source.is_null() {
        return 0;
    }

    let deflate_level = DeflateLevel::from_i32(level);
    let source_slice = unsafe { std::slice::from_raw_parts(source, source_length) };

    let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), deflate_level.to_flate2_level());

    if encoder.write_all(source_slice).is_ok() {
        if let Ok(compressed) = encoder.finish() {
            let buffer = Buffer::from_data(&compressed);
            return BUFFERS.insert(buffer);
        }
    }

    0
}

// ============================================================================
// FFI Functions - Brotli
// ============================================================================

/// Returns the upper bound on brotli compressed data size
#[unsafe(no_mangle)]
pub extern "C" fn fz_brotli_bound(_ctx: Handle, size: usize) -> usize {
    // Brotli worst case is slightly larger than input
    size + (size / 100) + 503
}

/// Compress data using brotli
///
/// # Safety
/// Caller must ensure:
/// - `dest` points to valid writable memory of at least `*compressed_length` bytes
/// - `source` points to valid readable memory of at least `source_length` bytes
/// - `compressed_length` points to valid writable memory
#[unsafe(no_mangle)]
pub extern "C" fn fz_compress_brotli(
    _ctx: Handle,
    dest: *mut u8,
    compressed_length: *mut usize,
    source: *const u8,
    source_length: usize,
    level: i32,
) {
    if dest.is_null() || compressed_length.is_null() || source.is_null() {
        return;
    }

    let brotli_level = BrotliLevel::from_i32(level);
    let source_slice = unsafe { std::slice::from_raw_parts(source, source_length) };
    let dest_capacity = unsafe { *compressed_length };

    let mut compressed = Vec::with_capacity(dest_capacity);

    let params = brotli::enc::BrotliEncoderParams {
        quality: brotli_level.to_u32() as i32,
        ..Default::default()
    };

    let mut encoder = brotli::CompressorWriter::with_params(&mut compressed, 4096, &params);

    if encoder.write_all(source_slice).is_ok() {
        drop(encoder);
        let len = compressed.len().min(dest_capacity);
        unsafe {
            std::ptr::copy_nonoverlapping(compressed.as_ptr(), dest, len);
            *compressed_length = len;
        }
        return;
    }

    unsafe {
        *compressed_length = 0;
    }
}

/// Compress data using brotli and return new allocated buffer
///
/// # Safety
/// Caller must ensure:
/// - `source` points to valid readable memory of at least `source_length` bytes
/// - `compressed_length` points to valid writable memory
/// - The returned pointer must be freed by the caller
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_brotli_data(
    _ctx: Handle,
    compressed_length: *mut usize,
    source: *const u8,
    source_length: usize,
    level: i32,
) -> *mut u8 {
    if compressed_length.is_null() || source.is_null() {
        return std::ptr::null_mut();
    }

    let brotli_level = BrotliLevel::from_i32(level);
    let source_slice = unsafe { std::slice::from_raw_parts(source, source_length) };

    let mut compressed = Vec::new();

    let params = brotli::enc::BrotliEncoderParams {
        quality: brotli_level.to_u32() as i32,
        ..Default::default()
    };

    let mut encoder = brotli::CompressorWriter::with_params(&mut compressed, 4096, &params);

    if encoder.write_all(source_slice).is_ok() {
        drop(encoder);
        let len = compressed.len();
        let ptr = compressed.as_ptr() as *mut u8;

        // SAFETY: We deliberately leak this Vec to transfer ownership to the C caller.
        // The caller is responsible for freeing this memory. Memory layout:
        // - Contiguous array of `len` bytes at `ptr`
        // - Allocated via Rust's global allocator
        // To properly deallocate from Rust: Vec::from_raw_parts(ptr, len, len)
        // Or call fz_free_compressed_data() which handles this cleanup.
        std::mem::forget(compressed);

        // SAFETY: compressed_length was checked for null at function entry
        unsafe {
            *compressed_length = len;
        }
        return ptr;
    }

    unsafe {
        *compressed_length = 0;
    }
    std::ptr::null_mut()
}

/// Compress buffer contents using brotli
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_brotli_data_from_buffer(
    _ctx: Handle,
    compressed_length: *mut usize,
    buffer: Handle,
    level: i32,
) -> *mut u8 {
    if compressed_length.is_null() {
        return std::ptr::null_mut();
    }

    let buf_arc = match BUFFERS.get(buffer) {
        Some(b) => b,
        None => {
            unsafe {
                *compressed_length = 0;
            }
            return std::ptr::null_mut();
        }
    };

    let buf_guard = buf_arc.lock().unwrap();
    let data = buf_guard.data();

    fz_new_brotli_data(_ctx, compressed_length, data.as_ptr(), data.len(), level)
}

/// Compress data into a buffer handle using brotli
#[unsafe(no_mangle)]
pub extern "C" fn fz_brotli_to_buffer(
    _ctx: Handle,
    source: *const u8,
    source_length: usize,
    level: i32,
) -> Handle {
    if source.is_null() {
        return 0;
    }

    let brotli_level = BrotliLevel::from_i32(level);
    let source_slice = unsafe { std::slice::from_raw_parts(source, source_length) };

    let mut compressed = Vec::new();

    let params = brotli::enc::BrotliEncoderParams {
        quality: brotli_level.to_u32() as i32,
        ..Default::default()
    };

    let mut encoder = brotli::CompressorWriter::with_params(&mut compressed, 4096, &params);

    if encoder.write_all(source_slice).is_ok() {
        drop(encoder);
        let buffer = Buffer::from_data(&compressed);
        return BUFFERS.insert(buffer);
    }

    0
}

// ============================================================================
// FFI Functions - CCITT Fax
// ============================================================================

/// Compress bitmap data as CCITT Group 3 1D fax image
#[unsafe(no_mangle)]
pub extern "C" fn fz_compress_ccitt_fax_g3(
    _ctx: Handle,
    data: *const u8,
    columns: i32,
    rows: i32,
    stride: isize,
) -> Handle {
    if data.is_null() || columns <= 0 || rows <= 0 {
        return 0;
    }

    let stride = if stride == 0 {
        (columns + 7) / 8
    } else {
        stride.unsigned_abs() as i32
    };

    let total_bytes = (stride * rows) as usize;
    let input_data = unsafe { std::slice::from_raw_parts(data, total_bytes) };

    // Simple CCITT Group 3 encoding (1D mode)
    let mut output = Vec::new();

    // Write simple header marker
    for row in 0..rows as usize {
        let row_start = row * stride as usize;
        let row_end = row_start + stride as usize;
        let row_data = &input_data[row_start..row_end];

        // Encode row using simple run-length encoding for G3
        encode_g3_row(&mut output, row_data, columns as usize);
    }

    // Add EOL markers
    output.extend_from_slice(&[0x00, 0x01]); // Final EOL

    let buffer = Buffer::from_data(&output);
    BUFFERS.insert(buffer)
}

/// Compress bitmap data as CCITT Group 4 2D fax image
#[unsafe(no_mangle)]
pub extern "C" fn fz_compress_ccitt_fax_g4(
    _ctx: Handle,
    data: *const u8,
    columns: i32,
    rows: i32,
    stride: isize,
) -> Handle {
    if data.is_null() || columns <= 0 || rows <= 0 {
        return 0;
    }

    let stride = if stride == 0 {
        (columns + 7) / 8
    } else {
        stride.unsigned_abs() as i32
    };

    let total_bytes = (stride * rows) as usize;
    let input_data = unsafe { std::slice::from_raw_parts(data, total_bytes) };

    // Simple CCITT Group 4 encoding (2D mode)
    let mut output = Vec::new();

    // Reference line (all white initially)
    let mut reference_line = vec![0u8; stride as usize];

    for row in 0..rows as usize {
        let row_start = row * stride as usize;
        let row_end = row_start + stride as usize;
        let row_data = &input_data[row_start..row_end];

        // Encode row using 2D encoding against reference line
        encode_g4_row(&mut output, row_data, &reference_line, columns as usize);

        // Current row becomes reference for next
        reference_line.copy_from_slice(row_data);
    }

    // Add EOFB (End of Facsimile Block)
    output.extend_from_slice(&[0x00, 0x10, 0x01]); // EOFB marker

    let buffer = Buffer::from_data(&output);
    BUFFERS.insert(buffer)
}

/// Simple Group 3 row encoding (1D)
fn encode_g3_row(output: &mut Vec<u8>, row: &[u8], columns: usize) {
    // Add EOL marker at start of each row
    output.push(0x00);
    output.push(0x01);

    let mut bit_pos = 0;
    let mut current_color = 0u8; // Start with white

    while bit_pos < columns {
        // Count run of current color
        let mut run_length = 0;
        while bit_pos + run_length < columns {
            let byte_idx = (bit_pos + run_length) / 8;
            let bit_idx = 7 - ((bit_pos + run_length) % 8);
            let bit = (row[byte_idx] >> bit_idx) & 1;

            if bit != current_color {
                break;
            }
            run_length += 1;
        }

        // Encode run length (simplified - just store length as bytes)
        // Real G3 would use Huffman codes
        encode_run_length(output, run_length, current_color);

        bit_pos += run_length;
        current_color = 1 - current_color; // Switch color
    }
}

/// Simple Group 4 row encoding (2D)
fn encode_g4_row(output: &mut Vec<u8>, current: &[u8], _reference: &[u8], columns: usize) {
    // Simplified G4: encode each row similarly to G3 but with pass/vertical/horizontal modes
    // For simplicity, we'll use horizontal mode for all
    let mut bit_pos = 0;
    let mut current_color = 0u8;

    while bit_pos < columns {
        let mut run_length = 0;
        while bit_pos + run_length < columns {
            let byte_idx = (bit_pos + run_length) / 8;
            let bit_idx = 7 - ((bit_pos + run_length) % 8);
            let bit = (current[byte_idx] >> bit_idx) & 1;

            if bit != current_color {
                break;
            }
            run_length += 1;
        }

        // Horizontal mode marker + run lengths
        output.push(0x02); // H mode marker (simplified)
        encode_run_length(output, run_length, current_color);

        bit_pos += run_length;
        current_color = 1 - current_color;
    }
}

/// Encode run length (simplified)
fn encode_run_length(output: &mut Vec<u8>, length: usize, _color: u8) {
    // Simplified encoding: store length as variable-length integer
    if length < 64 {
        output.push(length as u8);
    } else {
        // Makeup codes for longer runs
        output.push(0xFF);
        output.push((length >> 8) as u8);
        output.push((length & 0xFF) as u8);
    }
}

// ============================================================================
// FFI Functions - Compressed Buffer
// ============================================================================

/// Create new compressed buffer
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_compressed_buffer(_ctx: Handle) -> Handle {
    COMPRESSED_BUFFERS.insert(CompressedBuffer {
        refs: 1, // Start with ref count of 1
        ..Default::default()
    })
}

/// Keep compressed buffer reference
#[unsafe(no_mangle)]
pub extern "C" fn fz_keep_compressed_buffer(_ctx: Handle, cbuf: Handle) -> Handle {
    if let Some(cb) = COMPRESSED_BUFFERS.get(cbuf) {
        cb.lock().unwrap().refs += 1;
    }
    cbuf
}

/// Drop compressed buffer reference
#[unsafe(no_mangle)]
pub extern "C" fn fz_drop_compressed_buffer(_ctx: Handle, cbuf: Handle) {
    if let Some(cb) = COMPRESSED_BUFFERS.get(cbuf) {
        let mut guard = cb.lock().unwrap();
        guard.refs -= 1;
        if guard.refs <= 0 {
            // Drop the internal buffer
            if guard.buffer != 0 {
                crate::ffi::buffer::fz_drop_buffer(_ctx, guard.buffer);
            }
            drop(guard);
            COMPRESSED_BUFFERS.remove(cbuf);
        }
    }
}

/// Get compressed buffer size
#[unsafe(no_mangle)]
pub extern "C" fn fz_compressed_buffer_size(cbuf: Handle) -> usize {
    if let Some(cb) = COMPRESSED_BUFFERS.get(cbuf) {
        let guard = cb.lock().unwrap();
        if let Some(buf) = BUFFERS.get(guard.buffer) {
            return buf.lock().unwrap().len();
        }
    }
    0
}

/// Set compressed buffer data
#[unsafe(no_mangle)]
pub extern "C" fn fz_compressed_buffer_set_data(_ctx: Handle, cbuf: Handle, buffer: Handle) {
    if let Some(cb) = COMPRESSED_BUFFERS.get(cbuf) {
        cb.lock().unwrap().buffer = buffer;
    }
}

/// Get compressed buffer data
#[unsafe(no_mangle)]
pub extern "C" fn fz_compressed_buffer_get_data(_ctx: Handle, cbuf: Handle) -> Handle {
    if let Some(cb) = COMPRESSED_BUFFERS.get(cbuf) {
        return cb.lock().unwrap().buffer;
    }
    0
}

/// Set compression type
#[unsafe(no_mangle)]
pub extern "C" fn fz_compressed_buffer_set_type(_ctx: Handle, cbuf: Handle, image_type: i32) {
    if let Some(cb) = COMPRESSED_BUFFERS.get(cbuf) {
        cb.lock().unwrap().params.image_type = ImageType::from_i32(image_type);
    }
}

/// Get compression type
#[unsafe(no_mangle)]
pub extern "C" fn fz_compressed_buffer_get_type(_ctx: Handle, cbuf: Handle) -> i32 {
    if let Some(cb) = COMPRESSED_BUFFERS.get(cbuf) {
        return cb.lock().unwrap().params.image_type as i32;
    }
    0
}

// ============================================================================
// FFI Functions - Image Type
// ============================================================================

/// Recognize image format from first 8 bytes
#[unsafe(no_mangle)]
pub extern "C" fn fz_recognize_image_format(_ctx: Handle, data: *const u8) -> i32 {
    if data.is_null() {
        return ImageType::Unknown as i32;
    }

    let bytes = unsafe { std::slice::from_raw_parts(data, 8) };

    // PNG signature
    if bytes.starts_with(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]) {
        return ImageType::Png as i32;
    }

    // JPEG signature
    if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return ImageType::Jpeg as i32;
    }

    // GIF signature
    if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        return ImageType::Gif as i32;
    }

    // BMP signature
    if bytes.starts_with(b"BM") {
        return ImageType::Bmp as i32;
    }

    // TIFF signatures (little and big endian)
    if bytes.starts_with(&[0x49, 0x49, 0x2A, 0x00]) || bytes.starts_with(&[0x4D, 0x4D, 0x00, 0x2A])
    {
        return ImageType::Tiff as i32;
    }

    // JPEG 2000 signature
    if bytes.starts_with(&[0x00, 0x00, 0x00, 0x0C, 0x6A, 0x50, 0x20, 0x20]) {
        return ImageType::Jpx as i32;
    }

    // PSD signature
    if bytes.starts_with(b"8BPS") {
        return ImageType::Psd as i32;
    }

    // PNM signatures
    if bytes.starts_with(b"P1")
        || bytes.starts_with(b"P2")
        || bytes.starts_with(b"P3")
        || bytes.starts_with(b"P4")
        || bytes.starts_with(b"P5")
        || bytes.starts_with(b"P6")
    {
        return ImageType::Pnm as i32;
    }

    ImageType::Unknown as i32
}

/// Get image type name
#[unsafe(no_mangle)]
pub extern "C" fn fz_image_type_name(image_type: i32) -> *const std::ffi::c_char {
    let t = ImageType::from_i32(image_type);
    match t {
        ImageType::Unknown => c"unknown".as_ptr(),
        ImageType::Raw => c"raw".as_ptr(),
        ImageType::Fax => c"fax".as_ptr(),
        ImageType::Flate => c"flate".as_ptr(),
        ImageType::Lzw => c"lzw".as_ptr(),
        ImageType::Rld => c"rld".as_ptr(),
        ImageType::Brotli => c"brotli".as_ptr(),
        ImageType::Bmp => c"bmp".as_ptr(),
        ImageType::Gif => c"gif".as_ptr(),
        ImageType::Jbig2 => c"jbig2".as_ptr(),
        ImageType::Jpeg => c"jpeg".as_ptr(),
        ImageType::Jpx => c"jpx".as_ptr(),
        ImageType::Jxr => c"jxr".as_ptr(),
        ImageType::Png => c"png".as_ptr(),
        ImageType::Pnm => c"pnm".as_ptr(),
        ImageType::Tiff => c"tiff".as_ptr(),
        ImageType::Psd => c"psd".as_ptr(),
    }
}

/// Lookup image type from name
#[unsafe(no_mangle)]
pub extern "C" fn fz_lookup_image_type(name: *const std::ffi::c_char) -> i32 {
    if name.is_null() {
        return ImageType::Unknown as i32;
    }

    let name_str = unsafe { std::ffi::CStr::from_ptr(name).to_str().unwrap_or("") };
    ImageType::from_name(name_str) as i32
}

// ============================================================================
// FFI Functions - Decompression
// ============================================================================

/// Decompress deflated data
#[unsafe(no_mangle)]
pub extern "C" fn fz_inflate(
    _ctx: Handle,
    dest: *mut u8,
    dest_length: *mut usize,
    source: *const u8,
    source_length: usize,
) -> i32 {
    if dest.is_null() || dest_length.is_null() || source.is_null() {
        return -1;
    }

    let source_slice = unsafe { std::slice::from_raw_parts(source, source_length) };
    let dest_capacity = unsafe { *dest_length };

    let mut decoder = flate2::read::ZlibDecoder::new(source_slice);
    let mut decompressed = Vec::with_capacity(dest_capacity);

    if decoder.read_to_end(&mut decompressed).is_ok() {
        let len = decompressed.len().min(dest_capacity);
        unsafe {
            std::ptr::copy_nonoverlapping(decompressed.as_ptr(), dest, len);
            *dest_length = len;
        }
        return 0;
    }

    -1
}

/// Decompress brotli data
#[unsafe(no_mangle)]
pub extern "C" fn fz_decompress_brotli(
    _ctx: Handle,
    dest: *mut u8,
    dest_length: *mut usize,
    source: *const u8,
    source_length: usize,
) -> i32 {
    if dest.is_null() || dest_length.is_null() || source.is_null() {
        return -1;
    }

    let source_slice = unsafe { std::slice::from_raw_parts(source, source_length) };
    let dest_capacity = unsafe { *dest_length };

    let mut decompressed = Vec::with_capacity(dest_capacity);
    let mut decoder = brotli::Decompressor::new(source_slice, 4096);

    if decoder.read_to_end(&mut decompressed).is_ok() {
        let len = decompressed.len().min(dest_capacity);
        unsafe {
            std::ptr::copy_nonoverlapping(decompressed.as_ptr(), dest, len);
            *dest_length = len;
        }
        return 0;
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
    fn test_deflate_level_from_i32() {
        assert_eq!(DeflateLevel::from_i32(0), DeflateLevel::None);
        assert_eq!(DeflateLevel::from_i32(1), DeflateLevel::BestSpeed);
        assert_eq!(DeflateLevel::from_i32(9), DeflateLevel::Best);
        assert_eq!(DeflateLevel::from_i32(-1), DeflateLevel::Default);
        assert_eq!(DeflateLevel::from_i32(5), DeflateLevel::Default);
    }

    #[test]
    fn test_brotli_level_from_i32() {
        assert_eq!(BrotliLevel::from_i32(0), BrotliLevel::None);
        assert_eq!(BrotliLevel::from_i32(1), BrotliLevel::BestSpeed);
        assert_eq!(BrotliLevel::from_i32(11), BrotliLevel::Best);
        assert_eq!(BrotliLevel::from_i32(6), BrotliLevel::Default);
    }

    #[test]
    fn test_deflate_bound() {
        let bound = fz_deflate_bound(1, 1000);
        assert!(bound > 1000);
    }

    #[test]
    fn test_brotli_bound() {
        let bound = fz_brotli_bound(1, 1000);
        assert!(bound > 1000);
    }

    #[test]
    fn test_deflate_roundtrip() {
        let ctx = 1;
        let original = b"Hello, World! This is a test of deflate compression.";
        let mut compressed_len = fz_deflate_bound(ctx, original.len());
        let mut compressed = vec![0u8; compressed_len];

        fz_deflate(
            ctx,
            compressed.as_mut_ptr(),
            &mut compressed_len,
            original.as_ptr(),
            original.len(),
            DeflateLevel::Default as i32,
        );

        assert!(compressed_len > 0);
        assert!(compressed_len < original.len() + 20); // Should not grow much

        // Decompress
        let mut decompressed_len = original.len() * 2;
        let mut decompressed = vec![0u8; decompressed_len];

        let result = fz_inflate(
            ctx,
            decompressed.as_mut_ptr(),
            &mut decompressed_len,
            compressed.as_ptr(),
            compressed_len,
        );

        assert_eq!(result, 0);
        assert_eq!(decompressed_len, original.len());
        assert_eq!(&decompressed[..decompressed_len], original);
    }

    #[test]
    fn test_brotli_roundtrip() {
        let ctx = 1;
        let original = b"Hello, World! This is a test of brotli compression.";
        let mut compressed_len = fz_brotli_bound(ctx, original.len());
        let mut compressed = vec![0u8; compressed_len];

        fz_compress_brotli(
            ctx,
            compressed.as_mut_ptr(),
            &mut compressed_len,
            original.as_ptr(),
            original.len(),
            BrotliLevel::Default as i32,
        );

        assert!(compressed_len > 0);

        // Decompress
        let mut decompressed_len = original.len() * 2;
        let mut decompressed = vec![0u8; decompressed_len];

        let result = fz_decompress_brotli(
            ctx,
            decompressed.as_mut_ptr(),
            &mut decompressed_len,
            compressed.as_ptr(),
            compressed_len,
        );

        assert_eq!(result, 0);
        assert_eq!(decompressed_len, original.len());
        assert_eq!(&decompressed[..decompressed_len], original);
    }

    #[test]
    fn test_deflate_to_buffer() {
        let ctx = 1;
        let original = b"Test data for buffer compression";

        let buffer = fz_deflate_to_buffer(ctx, original.as_ptr(), original.len(), -1);
        assert!(buffer > 0);

        // Cleanup
        crate::ffi::buffer::fz_drop_buffer(ctx, buffer);
    }

    #[test]
    fn test_brotli_to_buffer() {
        let ctx = 1;
        let original = b"Test data for brotli buffer compression";

        let buffer = fz_brotli_to_buffer(ctx, original.as_ptr(), original.len(), 6);
        assert!(buffer > 0);

        // Cleanup
        crate::ffi::buffer::fz_drop_buffer(ctx, buffer);
    }

    #[test]
    fn test_image_type_from_name() {
        assert_eq!(ImageType::from_name("png"), ImageType::Png);
        assert_eq!(ImageType::from_name("PNG"), ImageType::Png);
        assert_eq!(ImageType::from_name("jpeg"), ImageType::Jpeg);
        assert_eq!(ImageType::from_name("jpg"), ImageType::Jpeg);
        assert_eq!(ImageType::from_name("unknown_format"), ImageType::Unknown);
    }

    #[test]
    fn test_image_type_name() {
        assert_eq!(ImageType::Png.name(), "png");
        assert_eq!(ImageType::Jpeg.name(), "jpeg");
        assert_eq!(ImageType::Unknown.name(), "unknown");
    }

    #[test]
    fn test_recognize_image_format_png() {
        let png_header = [0x89u8, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        assert_eq!(
            fz_recognize_image_format(1, png_header.as_ptr()),
            ImageType::Png as i32
        );
    }

    #[test]
    fn test_recognize_image_format_jpeg() {
        let jpeg_header = [0xFFu8, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46];
        assert_eq!(
            fz_recognize_image_format(1, jpeg_header.as_ptr()),
            ImageType::Jpeg as i32
        );
    }

    #[test]
    fn test_recognize_image_format_gif() {
        let gif_header = b"GIF89a\x00\x00";
        assert_eq!(
            fz_recognize_image_format(1, gif_header.as_ptr()),
            ImageType::Gif as i32
        );
    }

    #[test]
    fn test_recognize_image_format_unknown() {
        let unknown = [0x00u8; 8];
        assert_eq!(
            fz_recognize_image_format(1, unknown.as_ptr()),
            ImageType::Unknown as i32
        );
    }

    #[test]
    fn test_compressed_buffer_lifecycle() {
        let ctx = 1;

        let cbuf = fz_new_compressed_buffer(ctx);
        assert!(cbuf > 0);

        fz_compressed_buffer_set_type(ctx, cbuf, ImageType::Flate as i32);
        assert_eq!(
            fz_compressed_buffer_get_type(ctx, cbuf),
            ImageType::Flate as i32
        );

        // Test keep increments ref count
        let cbuf2 = fz_keep_compressed_buffer(ctx, cbuf);
        assert_eq!(cbuf, cbuf2);

        // First drop - should still exist due to extra ref from keep
        fz_drop_compressed_buffer(ctx, cbuf);

        // Verify it still exists by checking we can get its type
        let type_after_first_drop = fz_compressed_buffer_get_type(ctx, cbuf);
        assert_eq!(type_after_first_drop, ImageType::Flate as i32);

        // Second drop - now it should be removed
        fz_drop_compressed_buffer(ctx, cbuf);

        // After final drop, the buffer is gone, so type returns 0 (Unknown)
        let type_after_final_drop = fz_compressed_buffer_get_type(ctx, cbuf);
        assert_eq!(type_after_final_drop, ImageType::Unknown as i32);
    }

    #[test]
    fn test_ccitt_g3_basic() {
        let ctx = 1;
        // Simple 8x1 bitmap (all white)
        let data = [0x00u8];
        let buffer = fz_compress_ccitt_fax_g3(ctx, data.as_ptr(), 8, 1, 1);
        assert!(buffer > 0);

        crate::ffi::buffer::fz_drop_buffer(ctx, buffer);
    }

    #[test]
    fn test_ccitt_g4_basic() {
        let ctx = 1;
        // Simple 8x1 bitmap (all white)
        let data = [0x00u8];
        let buffer = fz_compress_ccitt_fax_g4(ctx, data.as_ptr(), 8, 1, 1);
        assert!(buffer > 0);

        crate::ffi::buffer::fz_drop_buffer(ctx, buffer);
    }

    #[test]
    fn test_deflate_levels() {
        let ctx = 1;
        let data = b"Test compression with different levels";

        for level in [0, 1, 9, -1] {
            let mut len = fz_deflate_bound(ctx, data.len());
            let mut compressed = vec![0u8; len];

            fz_deflate(
                ctx,
                compressed.as_mut_ptr(),
                &mut len,
                data.as_ptr(),
                data.len(),
                level,
            );

            assert!(len > 0);
        }
    }

    #[test]
    fn test_new_deflated_data() {
        let ctx = 1;
        let data = b"Data to compress into new allocation";
        let mut len = 0usize;

        let ptr = fz_new_deflated_data(ctx, &mut len, data.as_ptr(), data.len(), -1);
        assert!(!ptr.is_null());
        assert!(len > 0);

        // Free the allocated memory
        unsafe {
            let _ = Vec::from_raw_parts(ptr, len, len);
        }
    }

    #[test]
    fn test_new_brotli_data() {
        let ctx = 1;
        let data = b"Data to compress with brotli";
        let mut len = 0usize;

        let ptr = fz_new_brotli_data(ctx, &mut len, data.as_ptr(), data.len(), 6);
        assert!(!ptr.is_null());
        assert!(len > 0);

        // Free the allocated memory
        unsafe {
            let _ = Vec::from_raw_parts(ptr, len, len);
        }
    }

    #[test]
    fn test_lookup_image_type() {
        let name = c"png";
        assert_eq!(fz_lookup_image_type(name.as_ptr()), ImageType::Png as i32);

        let name = c"jpeg";
        assert_eq!(fz_lookup_image_type(name.as_ptr()), ImageType::Jpeg as i32);
    }

    #[test]
    fn test_image_type_name_ffi() {
        let name = fz_image_type_name(ImageType::Png as i32);
        assert!(!name.is_null());

        let name = fz_image_type_name(ImageType::Unknown as i32);
        assert!(!name.is_null());
    }
}
