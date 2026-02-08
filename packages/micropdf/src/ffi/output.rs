//! C FFI for output - MuPDF compatible
//! Safe Rust implementation using handle-based resource management

use super::{Handle, HandleStore};
use crate::fitz::output::{MemoryOutput, Output};
use std::ffi::{c_char, c_void};
use std::sync::LazyLock;

/// Output storage
pub static OUTPUTS: LazyLock<HandleStore<Output>> = LazyLock::new(HandleStore::default);

/// Memory output storage (separate since it has different methods)
pub static MEMORY_OUTPUTS: LazyLock<HandleStore<MemoryOutput>> =
    LazyLock::new(HandleStore::default);

/// Create a new output to a file
///
/// # Safety
/// Caller must ensure `filename` is a valid null-terminated C string.
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_output_with_path(
    _ctx: Handle,
    filename: *const c_char,
    append: i32,
) -> Handle {
    if filename.is_null() {
        return 0;
    }

    // SAFETY: Caller guarantees filename is a valid null-terminated C string
    let c_str = unsafe { std::ffi::CStr::from_ptr(filename) };
    let path = match c_str.to_str() {
        Ok(s) => s,
        Err(_) => return 0,
    };

    match Output::from_path(path, append != 0) {
        Ok(output) => OUTPUTS.insert(output),
        Err(_) => 0,
    }
}

/// Create output to a buffer
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_output_with_buffer(_ctx: Handle, buf: Handle) -> Handle {
    use super::BUFFERS;
    use crate::fitz::buffer::Buffer as FitzBuffer;

    if let Some(buffer_arc) = BUFFERS.get(buf) {
        if let Ok(guard) = buffer_arc.lock() {
            // Convert from FFI buffer to Fitz buffer
            let fitz_buffer = FitzBuffer::from_data(guard.data().to_vec());
            return OUTPUTS.insert(Output::from_buffer(fitz_buffer));
        }
    }
    0
}

/// Keep (increment ref) output
#[unsafe(no_mangle)]
pub extern "C" fn fz_keep_output(_ctx: Handle, out: Handle) -> Handle {
    OUTPUTS.keep(out)
}

/// Drop output reference (closes if last reference)
#[unsafe(no_mangle)]
pub extern "C" fn fz_drop_output(_ctx: Handle, out: Handle) {
    let _ = OUTPUTS.remove(out);
}

/// Write raw data to output
///
/// # Safety
/// Caller must ensure `data` points to valid memory of at least `size` bytes.
#[unsafe(no_mangle)]
pub extern "C" fn fz_write_data(_ctx: Handle, out: Handle, data: *const c_void, size: usize) {
    if data.is_null() || size == 0 {
        return;
    }

    if let Some(output_arc) = OUTPUTS.get(out) {
        if let Ok(mut guard) = output_arc.lock() {
            // SAFETY: Caller guarantees data points to valid memory of size bytes
            let slice = unsafe { std::slice::from_raw_parts(data as *const u8, size) };
            let _ = guard.write_data(slice);
        }
    }
}

/// Write a null-terminated C string
///
/// # Safety
/// Caller must ensure `s` is a valid null-terminated C string.
#[unsafe(no_mangle)]
pub extern "C" fn fz_write_string(_ctx: Handle, out: Handle, s: *const c_char) {
    if s.is_null() {
        return;
    }

    // SAFETY: Caller guarantees s is a valid null-terminated C string
    let c_str = unsafe { std::ffi::CStr::from_ptr(s) };
    if let Ok(rust_str) = c_str.to_str() {
        if let Some(output_arc) = OUTPUTS.get(out) {
            if let Ok(mut guard) = output_arc.lock() {
                let _ = guard.write_string(rust_str);
            }
        }
    }
}

/// Write a single byte
#[unsafe(no_mangle)]
pub extern "C" fn fz_write_byte(_ctx: Handle, out: Handle, byte: u8) {
    if let Some(output_arc) = OUTPUTS.get(out) {
        if let Ok(mut guard) = output_arc.lock() {
            let _ = guard.write_byte(byte);
        }
    }
}

/// Write a single character
#[unsafe(no_mangle)]
pub extern "C" fn fz_write_char(_ctx: Handle, out: Handle, c: c_char) {
    let byte = c as u8;
    fz_write_byte(_ctx, out, byte);
}

/// Write i16 big-endian
#[unsafe(no_mangle)]
pub extern "C" fn fz_write_int16_be(_ctx: Handle, out: Handle, x: i16) {
    if let Some(output_arc) = OUTPUTS.get(out) {
        if let Ok(mut guard) = output_arc.lock() {
            let _ = guard.write_i16_be(x);
        }
    }
}

/// Write i16 little-endian
#[unsafe(no_mangle)]
pub extern "C" fn fz_write_int16_le(_ctx: Handle, out: Handle, x: i16) {
    if let Some(output_arc) = OUTPUTS.get(out) {
        if let Ok(mut guard) = output_arc.lock() {
            let _ = guard.write_i16_le(x);
        }
    }
}

/// Write u16 big-endian
#[unsafe(no_mangle)]
pub extern "C" fn fz_write_uint16_be(_ctx: Handle, out: Handle, x: u16) {
    if let Some(output_arc) = OUTPUTS.get(out) {
        if let Ok(mut guard) = output_arc.lock() {
            let _ = guard.write_u16_be(x);
        }
    }
}

/// Write u16 little-endian
#[unsafe(no_mangle)]
pub extern "C" fn fz_write_uint16_le(_ctx: Handle, out: Handle, x: u16) {
    if let Some(output_arc) = OUTPUTS.get(out) {
        if let Ok(mut guard) = output_arc.lock() {
            let _ = guard.write_u16_le(x);
        }
    }
}

/// Write i32 big-endian
#[unsafe(no_mangle)]
pub extern "C" fn fz_write_int32_be(_ctx: Handle, out: Handle, x: i32) {
    if let Some(output_arc) = OUTPUTS.get(out) {
        if let Ok(mut guard) = output_arc.lock() {
            let _ = guard.write_i32_be(x);
        }
    }
}

/// Write i32 little-endian
#[unsafe(no_mangle)]
pub extern "C" fn fz_write_int32_le(_ctx: Handle, out: Handle, x: i32) {
    if let Some(output_arc) = OUTPUTS.get(out) {
        if let Ok(mut guard) = output_arc.lock() {
            let _ = guard.write_i32_le(x);
        }
    }
}

/// Write u32 big-endian
#[unsafe(no_mangle)]
pub extern "C" fn fz_write_uint32_be(_ctx: Handle, out: Handle, x: u32) {
    if let Some(output_arc) = OUTPUTS.get(out) {
        if let Ok(mut guard) = output_arc.lock() {
            let _ = guard.write_u32_be(x);
        }
    }
}

/// Write u32 little-endian
#[unsafe(no_mangle)]
pub extern "C" fn fz_write_uint32_le(_ctx: Handle, out: Handle, x: u32) {
    if let Some(output_arc) = OUTPUTS.get(out) {
        if let Ok(mut guard) = output_arc.lock() {
            let _ = guard.write_u32_le(x);
        }
    }
}

/// Write buffer contents
#[unsafe(no_mangle)]
pub extern "C" fn fz_write_buffer(_ctx: Handle, out: Handle, buf: Handle) {
    use super::BUFFERS;
    use crate::fitz::buffer::Buffer as FitzBuffer;

    if let Some(buffer_arc) = BUFFERS.get(buf) {
        if let Ok(buffer_guard) = buffer_arc.lock() {
            if let Some(output_arc) = OUTPUTS.get(out) {
                if let Ok(mut output_guard) = output_arc.lock() {
                    // Convert from FFI buffer to Fitz buffer for writing
                    let fitz_buffer = FitzBuffer::from_data(buffer_guard.data().to_vec());
                    let _ = output_guard.write_buffer(&fitz_buffer);
                }
            }
        }
    }
}

/// Seek within output
#[unsafe(no_mangle)]
pub extern "C" fn fz_seek_output(_ctx: Handle, out: Handle, off: i64, whence: i32) {
    use crate::fitz::output::SeekFrom;

    let seek_from = match whence {
        0 => SeekFrom::Start(off as u64),
        1 => SeekFrom::Current(off),
        2 => SeekFrom::End(off),
        _ => return,
    };

    if let Some(output_arc) = OUTPUTS.get(out) {
        if let Ok(mut guard) = output_arc.lock() {
            let _ = guard.seek(off, seek_from);
        }
    }
}

/// Get current position in output
#[unsafe(no_mangle)]
pub extern "C" fn fz_tell_output(_ctx: Handle, out: Handle) -> i64 {
    if let Some(output_arc) = OUTPUTS.get(out) {
        if let Ok(mut guard) = output_arc.lock() {
            if let Ok(pos) = guard.tell() {
                return pos as i64;
            }
        }
    }
    -1
}

/// Flush buffered data
#[unsafe(no_mangle)]
pub extern "C" fn fz_flush_output(_ctx: Handle, out: Handle) {
    if let Some(output_arc) = OUTPUTS.get(out) {
        if let Ok(mut guard) = output_arc.lock() {
            let _ = guard.flush();
        }
    }
}

/// Close output (flushes and finalizes)
#[unsafe(no_mangle)]
pub extern "C" fn fz_close_output(_ctx: Handle, out: Handle) {
    if let Some(output_arc) = OUTPUTS.get(out) {
        if let Ok(mut guard) = output_arc.lock() {
            let _ = guard.close();
        }
    }
}

/// Truncate output at current position
#[unsafe(no_mangle)]
pub extern "C" fn fz_truncate_output(_ctx: Handle, out: Handle) {
    if let Some(output_arc) = OUTPUTS.get(out) {
        if let Ok(mut guard) = output_arc.lock() {
            let _ = guard.truncate();
        }
    }
}

/// Reset output to initial state
#[unsafe(no_mangle)]
pub extern "C" fn fz_reset_output(_ctx: Handle, out: Handle) {
    if let Some(output_arc) = OUTPUTS.get(out) {
        if let Ok(mut guard) = output_arc.lock() {
            let _ = guard.reset();
        }
    }
}

/// Write i64 big-endian
#[unsafe(no_mangle)]
pub extern "C" fn fz_write_int64_be(_ctx: Handle, out: Handle, x: i64) {
    if let Some(output_arc) = OUTPUTS.get(out) {
        if let Ok(mut guard) = output_arc.lock() {
            let _ = guard.write_i64_be(x);
        }
    }
}

/// Write i64 little-endian
#[unsafe(no_mangle)]
pub extern "C" fn fz_write_int64_le(_ctx: Handle, out: Handle, x: i64) {
    if let Some(output_arc) = OUTPUTS.get(out) {
        if let Ok(mut guard) = output_arc.lock() {
            let _ = guard.write_i64_le(x);
        }
    }
}

/// Write u64 big-endian
#[unsafe(no_mangle)]
pub extern "C" fn fz_write_uint64_be(_ctx: Handle, out: Handle, x: u64) {
    if let Some(output_arc) = OUTPUTS.get(out) {
        if let Ok(mut guard) = output_arc.lock() {
            let _ = guard.write_u64_be(x);
        }
    }
}

/// Write u64 little-endian
#[unsafe(no_mangle)]
pub extern "C" fn fz_write_uint64_le(_ctx: Handle, out: Handle, x: u64) {
    if let Some(output_arc) = OUTPUTS.get(out) {
        if let Ok(mut guard) = output_arc.lock() {
            let _ = guard.write_u64_le(x);
        }
    }
}

/// Write float big-endian
#[unsafe(no_mangle)]
pub extern "C" fn fz_write_float_be(_ctx: Handle, out: Handle, x: f32) {
    if let Some(output_arc) = OUTPUTS.get(out) {
        if let Ok(mut guard) = output_arc.lock() {
            let _ = guard.write_f32_be(x);
        }
    }
}

/// Write float little-endian
#[unsafe(no_mangle)]
pub extern "C" fn fz_write_float_le(_ctx: Handle, out: Handle, x: f32) {
    if let Some(output_arc) = OUTPUTS.get(out) {
        if let Ok(mut guard) = output_arc.lock() {
            let _ = guard.write_f32_le(x);
        }
    }
}

/// Write a Unicode rune (codepoint) as UTF-8
#[unsafe(no_mangle)]
pub extern "C" fn fz_write_rune(_ctx: Handle, out: Handle, rune: i32) {
    if let Some(output_arc) = OUTPUTS.get(out) {
        if let Ok(mut guard) = output_arc.lock() {
            if let Some(ch) = char::from_u32(rune as u32) {
                let mut utf8_buf = [0u8; 4];
                let utf8_str = ch.encode_utf8(&mut utf8_buf);
                let _ = guard.write_string(utf8_str);
            }
        }
    }
}

/// Write base64 encoded data
///
/// # Safety
/// Caller must ensure `data` points to valid memory of at least `size` bytes.
#[unsafe(no_mangle)]
pub extern "C" fn fz_write_base64(
    _ctx: Handle,
    out: Handle,
    data: *const u8,
    size: usize,
    newline: i32,
) {
    if data.is_null() || size == 0 {
        return;
    }

    if let Some(output_arc) = OUTPUTS.get(out) {
        if let Ok(mut guard) = output_arc.lock() {
            // SAFETY: Caller guarantees data points to valid memory of size bytes
            let data_slice = unsafe { std::slice::from_raw_parts(data, size) };

            // Simple base64 encoding
            const BASE64_CHARS: &[u8] =
                b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
            let mut col = 0;

            for chunk in data_slice.chunks(3) {
                let b1 = chunk[0];
                let b2 = chunk.get(1).copied().unwrap_or(0);
                let b3 = chunk.get(2).copied().unwrap_or(0);

                let _ = guard.write_byte(BASE64_CHARS[((b1 >> 2) & 0x3F) as usize]);
                let _ = guard.write_byte(BASE64_CHARS[(((b1 << 4) | (b2 >> 4)) & 0x3F) as usize]);

                if chunk.len() > 1 {
                    let _ =
                        guard.write_byte(BASE64_CHARS[(((b2 << 2) | (b3 >> 6)) & 0x3F) as usize]);
                } else {
                    let _ = guard.write_byte(b'=');
                }

                if chunk.len() > 2 {
                    let _ = guard.write_byte(BASE64_CHARS[(b3 & 0x3F) as usize]);
                } else {
                    let _ = guard.write_byte(b'=');
                }

                col += 4;
                if newline > 0 && col >= newline {
                    let _ = guard.write_byte(b'\n');
                    col = 0;
                }
            }
        }
    }
}

/// Write base64 URL-safe encoded data
///
/// # Safety
/// Caller must ensure `data` points to valid memory of at least `size` bytes.
#[unsafe(no_mangle)]
pub extern "C" fn fz_write_base64_uri(_ctx: Handle, out: Handle, data: *const u8, size: usize) {
    if data.is_null() || size == 0 {
        return;
    }

    if let Some(output_arc) = OUTPUTS.get(out) {
        if let Ok(mut guard) = output_arc.lock() {
            // SAFETY: Caller guarantees data points to valid memory of size bytes
            let data_slice = unsafe { std::slice::from_raw_parts(data, size) };

            // URL-safe base64 encoding (- and _ instead of + and /)
            const BASE64_CHARS: &[u8] =
                b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

            for chunk in data_slice.chunks(3) {
                let b1 = chunk[0];
                let b2 = chunk.get(1).copied().unwrap_or(0);
                let b3 = chunk.get(2).copied().unwrap_or(0);

                let _ = guard.write_byte(BASE64_CHARS[((b1 >> 2) & 0x3F) as usize]);
                let _ = guard.write_byte(BASE64_CHARS[(((b1 << 4) | (b2 >> 4)) & 0x3F) as usize]);

                if chunk.len() > 1 {
                    let _ =
                        guard.write_byte(BASE64_CHARS[(((b2 << 2) | (b3 >> 6)) & 0x3F) as usize]);
                }

                if chunk.len() > 2 {
                    let _ = guard.write_byte(BASE64_CHARS[(b3 & 0x3F) as usize]);
                }
            }
        }
    }
}

/// Write bits to output (for compressed data)
#[unsafe(no_mangle)]
pub extern "C" fn fz_write_bits(_ctx: Handle, out: Handle, value: u32, count: i32) {
    if count <= 0 || count > 32 {
        return;
    }

    if let Some(output_arc) = OUTPUTS.get(out) {
        if let Ok(mut guard) = output_arc.lock() {
            // Write bits most significant first
            let val = value;
            let mut bits_left = count;

            while bits_left >= 8 {
                bits_left -= 8;
                let byte = ((val >> bits_left) & 0xFF) as u8;
                let _ = guard.write_byte(byte);
            }

            if bits_left > 0 {
                let byte = ((val & ((1 << bits_left) - 1)) << (8 - bits_left)) as u8;
                let _ = guard.write_byte(byte);
            }
        }
    }
}

/// Sync bits (flush partial byte)
#[unsafe(no_mangle)]
pub extern "C" fn fz_write_bits_sync(_ctx: Handle, out: Handle) {
    // In our simplified implementation, bits are already synced
    // This is a no-op for compatibility
    if let Some(output_arc) = OUTPUTS.get(out) {
        if let Ok(mut guard) = output_arc.lock() {
            let _ = guard.flush();
        }
    }
}

// POSIX-style whence constants for fz_seek_output
pub const SEEK_SET: i32 = 0;
pub const SEEK_CUR: i32 = 1;
pub const SEEK_END: i32 = 2;

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;
    use tempfile::NamedTempFile;

    #[test]
    fn test_output_to_file() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();
        let c_path = CString::new(path.to_str().unwrap()).unwrap();

        let ctx = 0;
        let out = fz_new_output_with_path(ctx, c_path.as_ptr(), 0);
        assert_ne!(out, 0);

        fz_write_string(ctx, out, c"Hello, World!".as_ptr());
        fz_close_output(ctx, out);
        fz_drop_output(ctx, out);

        let content = std::fs::read_to_string(path).unwrap();
        assert_eq!(content, "Hello, World!");
    }

    #[test]
    fn test_output_write_data() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();
        let c_path = CString::new(path.to_str().unwrap()).unwrap();

        let ctx = 0;
        let out = fz_new_output_with_path(ctx, c_path.as_ptr(), 0);

        let data = b"Test data";
        fz_write_data(ctx, out, data.as_ptr() as *const c_void, data.len());

        fz_close_output(ctx, out);
        fz_drop_output(ctx, out);

        let content = std::fs::read(path).unwrap();
        assert_eq!(&content, data);
    }

    #[test]
    fn test_output_write_integers() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();
        let c_path = CString::new(path.to_str().unwrap()).unwrap();

        let ctx = 0;
        let out = fz_new_output_with_path(ctx, c_path.as_ptr(), 0);

        fz_write_int16_be(ctx, out, 0x1234);
        fz_write_uint32_le(ctx, out, 0xDEADBEEF);

        fz_close_output(ctx, out);
        fz_drop_output(ctx, out);

        let content = std::fs::read(path).unwrap();
        assert_eq!(content.len(), 6); // 2 + 4 bytes
        assert_eq!(&content[0..2], &[0x12, 0x34]); // Big-endian i16
        assert_eq!(&content[2..6], &[0xEF, 0xBE, 0xAD, 0xDE]); // Little-endian u32
    }

    #[test]
    fn test_output_seek_tell() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();
        let c_path = CString::new(path.to_str().unwrap()).unwrap();

        let ctx = 0;
        let out = fz_new_output_with_path(ctx, c_path.as_ptr(), 0);

        fz_write_string(ctx, out, c"Hello".as_ptr());
        let pos1 = fz_tell_output(ctx, out);
        assert_eq!(pos1, 5);

        fz_seek_output(ctx, out, 0, SEEK_SET);
        let pos2 = fz_tell_output(ctx, out);
        assert_eq!(pos2, 0);

        fz_write_string(ctx, out, c"Jello".as_ptr());

        fz_close_output(ctx, out);
        fz_drop_output(ctx, out);

        let content = std::fs::read_to_string(path).unwrap();
        assert_eq!(content, "Jello");
    }

    #[test]
    fn test_output_null_filename() {
        let out = fz_new_output_with_path(0, std::ptr::null(), 0);
        assert_eq!(out, 0);
    }

    #[test]
    fn test_output_write_byte() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();
        let c_path = CString::new(path.to_str().unwrap()).unwrap();

        let ctx = 0;
        let out = fz_new_output_with_path(ctx, c_path.as_ptr(), 0);

        fz_write_byte(ctx, out, b'A');
        fz_write_byte(ctx, out, b'B');
        fz_write_byte(ctx, out, b'C');

        fz_close_output(ctx, out);
        fz_drop_output(ctx, out);

        let content = std::fs::read_to_string(path).unwrap();
        assert_eq!(content, "ABC");
    }

    #[test]
    fn test_output_flush() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();
        let c_path = CString::new(path.to_str().unwrap()).unwrap();

        let ctx = 0;
        let out = fz_new_output_with_path(ctx, c_path.as_ptr(), 0);

        fz_write_string(ctx, out, c"Data".as_ptr());
        fz_flush_output(ctx, out);

        // Data should be flushed to disk
        let content = std::fs::read_to_string(path).unwrap();
        assert_eq!(content, "Data");

        fz_drop_output(ctx, out);
    }

    #[test]
    fn test_output_truncate() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();
        let c_path = CString::new(path.to_str().unwrap()).unwrap();

        let ctx = 0;
        let out = fz_new_output_with_path(ctx, c_path.as_ptr(), 0);

        fz_write_string(ctx, out, c"Hello, World!".as_ptr());
        fz_seek_output(ctx, out, 5, SEEK_SET);
        fz_truncate_output(ctx, out);

        fz_close_output(ctx, out);
        fz_drop_output(ctx, out);

        let content = std::fs::read_to_string(path).unwrap();
        assert_eq!(content, "Hello");
    }

    #[test]
    fn test_output_keep() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();
        let c_path = CString::new(path.to_str().unwrap()).unwrap();

        let ctx = 0;
        let out = fz_new_output_with_path(ctx, c_path.as_ptr(), 0);
        let kept = fz_keep_output(ctx, out);
        assert_eq!(kept, out);

        fz_drop_output(ctx, out);
    }

    #[test]
    fn test_seek_constants() {
        assert_eq!(SEEK_SET, 0);
        assert_eq!(SEEK_CUR, 1);
        assert_eq!(SEEK_END, 2);
    }

    #[test]
    fn test_output_write_int64() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();
        let c_path = CString::new(path.to_str().unwrap()).unwrap();

        let ctx = 0;
        let out = fz_new_output_with_path(ctx, c_path.as_ptr(), 0);

        fz_write_int64_be(ctx, out, 0x0123456789ABCDEF_i64);
        fz_write_uint64_le(ctx, out, 0xFEDCBA9876543210_u64);

        fz_close_output(ctx, out);
        fz_drop_output(ctx, out);

        let content = std::fs::read(path).unwrap();
        assert_eq!(content.len(), 16); // 8 + 8 bytes
    }

    #[test]
    fn test_output_write_float() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();
        let c_path = CString::new(path.to_str().unwrap()).unwrap();

        let ctx = 0;
        let out = fz_new_output_with_path(ctx, c_path.as_ptr(), 0);

        fz_write_float_be(ctx, out, std::f32::consts::PI);
        fz_write_float_le(ctx, out, std::f32::consts::E);

        fz_close_output(ctx, out);
        fz_drop_output(ctx, out);

        let content = std::fs::read(path).unwrap();
        assert_eq!(content.len(), 8); // 4 + 4 bytes
    }

    #[test]
    fn test_output_write_rune() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();
        let c_path = CString::new(path.to_str().unwrap()).unwrap();

        let ctx = 0;
        let out = fz_new_output_with_path(ctx, c_path.as_ptr(), 0);

        fz_write_rune(ctx, out, 0x1F600); // 😀 emoji
        fz_write_rune(ctx, out, 0x41); // 'A'

        fz_close_output(ctx, out);
        fz_drop_output(ctx, out);

        let content = std::fs::read_to_string(path).unwrap();
        assert_eq!(content, "😀A");
    }

    #[test]
    fn test_output_write_base64() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();
        let c_path = CString::new(path.to_str().unwrap()).unwrap();

        let ctx = 0;
        let out = fz_new_output_with_path(ctx, c_path.as_ptr(), 0);

        let data = b"Hello";
        fz_write_base64(ctx, out, data.as_ptr(), data.len(), 0);

        fz_close_output(ctx, out);
        fz_drop_output(ctx, out);

        let content = std::fs::read_to_string(path).unwrap();
        assert_eq!(content, "SGVsbG8=");
    }

    #[test]
    fn test_output_write_base64_uri() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();
        let c_path = CString::new(path.to_str().unwrap()).unwrap();

        let ctx = 0;
        let out = fz_new_output_with_path(ctx, c_path.as_ptr(), 0);

        let data = b"Test";
        fz_write_base64_uri(ctx, out, data.as_ptr(), data.len());

        fz_close_output(ctx, out);
        fz_drop_output(ctx, out);

        let content = std::fs::read_to_string(path).unwrap();
        assert!(content.contains("VGVzdA")); // URL-safe base64 for "Test"
    }

    #[test]
    fn test_output_write_bits() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();
        let c_path = CString::new(path.to_str().unwrap()).unwrap();

        let ctx = 0;
        let out = fz_new_output_with_path(ctx, c_path.as_ptr(), 0);

        fz_write_bits(ctx, out, 0xFF, 8); // 8 bits = 1 byte
        fz_write_bits_sync(ctx, out);

        fz_close_output(ctx, out);
        fz_drop_output(ctx, out);

        let content = std::fs::read(path).unwrap();
        assert_eq!(content.len(), 1);
        assert_eq!(content[0], 0xFF);
    }
}
