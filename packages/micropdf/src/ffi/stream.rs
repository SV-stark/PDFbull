//! C FFI for stream - MuPDF compatible
//! Safe Rust implementation using handle-based resource management

use super::{BUFFERS, Handle, STREAMS};
use std::ffi::c_char;

/// Internal stream state
pub struct Stream {
    pub(crate) data: Vec<u8>,
    position: usize,
    eof: bool,
}

impl Stream {
    pub fn new() -> Self {
        Self {
            data: Vec::new(),
            position: 0,
            eof: true,
        }
    }

    pub fn from_memory(data: Vec<u8>) -> Self {
        let eof = data.is_empty();
        Self {
            data,
            position: 0,
            eof,
        }
    }

    pub fn read(&mut self, buf: &mut [u8]) -> usize {
        if self.position >= self.data.len() {
            self.eof = true;
            return 0;
        }

        let available = self.data.len() - self.position;
        let to_read = buf.len().min(available);
        buf[..to_read].copy_from_slice(&self.data[self.position..self.position + to_read]);
        self.position += to_read;

        if self.position >= self.data.len() {
            self.eof = true;
        }

        to_read
    }

    pub fn read_byte(&mut self) -> Option<u8> {
        if self.position >= self.data.len() {
            self.eof = true;
            return None;
        }
        let byte = self.data[self.position];
        self.position += 1;
        if self.position >= self.data.len() {
            self.eof = true;
        }
        Some(byte)
    }

    pub fn peek_byte(&self) -> Option<u8> {
        if self.position >= self.data.len() {
            return None;
        }
        Some(self.data[self.position])
    }

    pub fn seek(&mut self, offset: i64, whence: i32) {
        let new_pos = match whence {
            0 => offset as usize,                            // SEEK_SET
            1 => (self.position as i64 + offset) as usize,   // SEEK_CUR
            2 => (self.data.len() as i64 + offset) as usize, // SEEK_END
            _ => self.position,
        };
        self.position = new_pos.min(self.data.len());
        self.eof = self.position >= self.data.len();
    }

    pub fn tell(&self) -> i64 {
        self.position as i64
    }

    pub fn is_eof(&self) -> bool {
        self.eof
    }
}

impl Default for Stream {
    fn default() -> Self {
        Self::new()
    }
}

/// Keep (increment ref) a stream
#[unsafe(no_mangle)]
pub extern "C" fn fz_keep_stream(_ctx: Handle, stm: Handle) -> Handle {
    STREAMS.keep(stm)
}

/// Drop a stream reference
#[unsafe(no_mangle)]
pub extern "C" fn fz_drop_stream(_ctx: Handle, stm: Handle) {
    let _ = STREAMS.remove(stm);
}

/// Open a file for reading
///
/// # Safety
/// Caller must ensure `filename` is a valid null-terminated C string.
#[unsafe(no_mangle)]
pub extern "C" fn fz_open_file(_ctx: Handle, filename: *const c_char) -> Handle {
    if filename.is_null() {
        return 0;
    }

    // SAFETY: Caller guarantees filename is a valid null-terminated C string
    let c_str = unsafe { std::ffi::CStr::from_ptr(filename) };
    let path = match c_str.to_str() {
        Ok(s) => s,
        Err(_) => return 0,
    };

    match std::fs::read(path) {
        Ok(data) => STREAMS.insert(Stream::from_memory(data)),
        Err(_) => 0,
    }
}

/// Open a stream from memory
///
/// # Safety
/// Caller must ensure `data` points to valid memory of at least `len` bytes.
#[unsafe(no_mangle)]
pub extern "C" fn fz_open_memory(_ctx: Handle, data: *const u8, len: usize) -> Handle {
    if data.is_null() || len == 0 {
        return STREAMS.insert(Stream::new());
    }

    // SAFETY: Caller guarantees data points to valid memory of `len` bytes
    let slice = unsafe { std::slice::from_raw_parts(data, len) };
    STREAMS.insert(Stream::from_memory(slice.to_vec()))
}

/// Open a stream from a buffer handle
#[unsafe(no_mangle)]
pub extern "C" fn fz_open_buffer(_ctx: Handle, buf: Handle) -> Handle {
    if let Some(buffer) = BUFFERS.get(buf) {
        if let Ok(guard) = buffer.lock() {
            return STREAMS.insert(Stream::from_memory(guard.data().to_vec()));
        }
    }
    0
}

/// Read from stream into buffer
///
/// # Safety
/// Caller must ensure `data` points to writable memory of at least `len` bytes.
#[unsafe(no_mangle)]
pub extern "C" fn fz_read(_ctx: Handle, stm: Handle, data: *mut u8, len: usize) -> usize {
    if data.is_null() || len == 0 {
        return 0;
    }

    if let Some(stream) = STREAMS.get(stm) {
        if let Ok(mut guard) = stream.lock() {
            // SAFETY: Caller guarantees data points to writable memory of `len` bytes
            let buf = unsafe { std::slice::from_raw_parts_mut(data, len) };
            return guard.read(buf);
        }
    }
    0
}

/// Read a single byte from stream
#[unsafe(no_mangle)]
pub extern "C" fn fz_read_byte(_ctx: Handle, stm: Handle) -> i32 {
    if let Some(stream) = STREAMS.get(stm) {
        if let Ok(mut guard) = stream.lock() {
            if let Some(byte) = guard.read_byte() {
                return byte as i32;
            }
        }
    }
    -1 // EOF
}

/// Peek at next byte without consuming
#[unsafe(no_mangle)]
pub extern "C" fn fz_peek_byte(_ctx: Handle, stm: Handle) -> i32 {
    if let Some(stream) = STREAMS.get(stm) {
        if let Ok(guard) = stream.lock() {
            if let Some(byte) = guard.peek_byte() {
                return byte as i32;
            }
        }
    }
    -1
}

/// Check if stream is at EOF
#[unsafe(no_mangle)]
pub extern "C" fn fz_is_eof(_ctx: Handle, stm: Handle) -> i32 {
    if let Some(stream) = STREAMS.get(stm) {
        if let Ok(guard) = stream.lock() {
            return i32::from(guard.is_eof());
        }
    }
    1
}

/// Seek in stream
#[unsafe(no_mangle)]
pub extern "C" fn fz_seek(_ctx: Handle, stm: Handle, offset: i64, whence: i32) {
    if let Some(stream) = STREAMS.get(stm) {
        if let Ok(mut guard) = stream.lock() {
            guard.seek(offset, whence);
        }
    }
}

/// Get current position in stream
#[unsafe(no_mangle)]
pub extern "C" fn fz_tell(_ctx: Handle, stm: Handle) -> i64 {
    if let Some(stream) = STREAMS.get(stm) {
        if let Ok(guard) = stream.lock() {
            return guard.tell();
        }
    }
    0
}

// Integer reading functions
#[unsafe(no_mangle)]
pub extern "C" fn fz_read_uint16(_ctx: Handle, stm: Handle) -> u16 {
    let mut buf = [0u8; 2];
    if fz_read(_ctx, stm, buf.as_mut_ptr(), 2) == 2 {
        u16::from_be_bytes(buf)
    } else {
        0
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn fz_read_uint32(_ctx: Handle, stm: Handle) -> u32 {
    let mut buf = [0u8; 4];
    if fz_read(_ctx, stm, buf.as_mut_ptr(), 4) == 4 {
        u32::from_be_bytes(buf)
    } else {
        0
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn fz_read_uint16_le(_ctx: Handle, stm: Handle) -> u16 {
    let mut buf = [0u8; 2];
    if fz_read(_ctx, stm, buf.as_mut_ptr(), 2) == 2 {
        u16::from_le_bytes(buf)
    } else {
        0
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn fz_read_uint32_le(_ctx: Handle, stm: Handle) -> u32 {
    let mut buf = [0u8; 4];
    if fz_read(_ctx, stm, buf.as_mut_ptr(), 4) == 4 {
        u32::from_le_bytes(buf)
    } else {
        0
    }
}

/// Read i16 big-endian
#[unsafe(no_mangle)]
pub extern "C" fn fz_read_int16(_ctx: Handle, stm: Handle) -> i16 {
    let mut buf = [0u8; 2];
    if fz_read(_ctx, stm, buf.as_mut_ptr(), 2) == 2 {
        i16::from_be_bytes(buf)
    } else {
        0
    }
}

/// Read i16 little-endian
#[unsafe(no_mangle)]
pub extern "C" fn fz_read_int16_le(_ctx: Handle, stm: Handle) -> i16 {
    let mut buf = [0u8; 2];
    if fz_read(_ctx, stm, buf.as_mut_ptr(), 2) == 2 {
        i16::from_le_bytes(buf)
    } else {
        0
    }
}

/// Read i32 big-endian
#[unsafe(no_mangle)]
pub extern "C" fn fz_read_int32(_ctx: Handle, stm: Handle) -> i32 {
    let mut buf = [0u8; 4];
    if fz_read(_ctx, stm, buf.as_mut_ptr(), 4) == 4 {
        i32::from_be_bytes(buf)
    } else {
        0
    }
}

/// Read i32 little-endian
#[unsafe(no_mangle)]
pub extern "C" fn fz_read_int32_le(_ctx: Handle, stm: Handle) -> i32 {
    let mut buf = [0u8; 4];
    if fz_read(_ctx, stm, buf.as_mut_ptr(), 4) == 4 {
        i32::from_le_bytes(buf)
    } else {
        0
    }
}

/// Read i64 big-endian
#[unsafe(no_mangle)]
pub extern "C" fn fz_read_int64(_ctx: Handle, stm: Handle) -> i64 {
    let mut buf = [0u8; 8];
    if fz_read(_ctx, stm, buf.as_mut_ptr(), 8) == 8 {
        i64::from_be_bytes(buf)
    } else {
        0
    }
}

/// Read i64 little-endian
#[unsafe(no_mangle)]
pub extern "C" fn fz_read_int64_le(_ctx: Handle, stm: Handle) -> i64 {
    let mut buf = [0u8; 8];
    if fz_read(_ctx, stm, buf.as_mut_ptr(), 8) == 8 {
        i64::from_le_bytes(buf)
    } else {
        0
    }
}

/// Read u64 big-endian
#[unsafe(no_mangle)]
pub extern "C" fn fz_read_uint64(_ctx: Handle, stm: Handle) -> u64 {
    let mut buf = [0u8; 8];
    if fz_read(_ctx, stm, buf.as_mut_ptr(), 8) == 8 {
        u64::from_be_bytes(buf)
    } else {
        0
    }
}

/// Read u64 little-endian
#[unsafe(no_mangle)]
pub extern "C" fn fz_read_uint64_le(_ctx: Handle, stm: Handle) -> u64 {
    let mut buf = [0u8; 8];
    if fz_read(_ctx, stm, buf.as_mut_ptr(), 8) == 8 {
        u64::from_le_bytes(buf)
    } else {
        0
    }
}

/// Read f32 big-endian
#[unsafe(no_mangle)]
pub extern "C" fn fz_read_float(_ctx: Handle, stm: Handle) -> f32 {
    let mut buf = [0u8; 4];
    if fz_read(_ctx, stm, buf.as_mut_ptr(), 4) == 4 {
        f32::from_be_bytes(buf)
    } else {
        0.0
    }
}

/// Read f32 little-endian
#[unsafe(no_mangle)]
pub extern "C" fn fz_read_float_le(_ctx: Handle, stm: Handle) -> f32 {
    let mut buf = [0u8; 4];
    if fz_read(_ctx, stm, buf.as_mut_ptr(), 4) == 4 {
        f32::from_le_bytes(buf)
    } else {
        0.0
    }
}

/// Read a line of text (up to newline or EOF)
///
/// # Safety
/// Caller must ensure `buf` points to valid writable memory of at least `max` bytes.
#[unsafe(no_mangle)]
pub extern "C" fn fz_read_line(
    _ctx: Handle,
    stm: Handle,
    buf: *mut c_char,
    max: usize,
) -> *mut c_char {
    if buf.is_null() || max == 0 {
        return std::ptr::null_mut();
    }

    if let Some(stream) = STREAMS.get(stm) {
        if let Ok(mut guard) = stream.lock() {
            let mut count = 0;
            let mut result: Vec<u8> = Vec::new();

            while count < max - 1 {
                if let Some(byte) = guard.read_byte() {
                    if byte == b'\n' {
                        break;
                    }
                    if byte != b'\r' {
                        // Skip carriage returns
                        result.push(byte);
                        count += 1;
                    }
                } else {
                    break;
                }
            }

            if !result.is_empty() || count > 0 {
                // SAFETY: Caller guarantees buf points to valid memory of max bytes
                let dest = unsafe { std::slice::from_raw_parts_mut(buf as *mut u8, max) };
                let to_copy = result.len().min(max - 1);
                dest[..to_copy].copy_from_slice(&result[..to_copy]);
                dest[to_copy] = 0; // Null terminate
                return buf;
            }
        }
    }
    std::ptr::null_mut()
}

/// Unread a byte (push back)
#[unsafe(no_mangle)]
pub extern "C" fn fz_unread_byte(_ctx: Handle, stm: Handle) {
    if let Some(stream) = STREAMS.get(stm) {
        if let Ok(mut guard) = stream.lock() {
            if guard.position > 0 {
                guard.position -= 1;
                guard.eof = false;
            }
        }
    }
}

/// Skip whitespace
#[unsafe(no_mangle)]
pub extern "C" fn fz_skip_space(_ctx: Handle, stm: Handle) {
    if let Some(stream) = STREAMS.get(stm) {
        if let Ok(mut guard) = stream.lock() {
            while let Some(byte) = guard.peek_byte() {
                if byte.is_ascii_whitespace() {
                    guard.read_byte();
                } else {
                    break;
                }
            }
        }
    }
}

/// Read all remaining data into buffer
#[unsafe(no_mangle)]
pub extern "C" fn fz_read_all(_ctx: Handle, stm: Handle) -> Handle {
    use super::BUFFERS;
    use super::buffer::Buffer;

    if let Some(stream) = STREAMS.get(stm) {
        if let Ok(mut guard) = stream.lock() {
            let remaining = &guard.data[guard.position..];
            let buffer = Buffer::from_data(remaining);
            guard.position = guard.data.len();
            guard.eof = true;
            return BUFFERS.insert(buffer);
        }
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stream_from_memory() {
        let data = vec![1, 2, 3, 4, 5];
        let handle = STREAMS.insert(Stream::from_memory(data));

        assert_eq!(fz_read_byte(0, handle), 1);
        assert_eq!(fz_read_byte(0, handle), 2);
        assert_eq!(fz_tell(0, handle), 2);

        fz_seek(0, handle, 0, 0); // SEEK_SET
        assert_eq!(fz_read_byte(0, handle), 1);

        fz_drop_stream(0, handle);
    }

    #[test]
    fn test_stream_eof() {
        let data = vec![1];
        let handle = STREAMS.insert(Stream::from_memory(data));

        assert_eq!(fz_is_eof(0, handle), 0);
        fz_read_byte(0, handle);
        assert_eq!(fz_is_eof(0, handle), 1);

        fz_drop_stream(0, handle);
    }

    #[test]
    fn test_stream_keep() {
        let data = vec![1, 2, 3];
        let handle = STREAMS.insert(Stream::from_memory(data));
        let kept = fz_keep_stream(0, handle);
        assert_eq!(kept, handle);
        fz_drop_stream(0, handle);
    }

    #[test]
    fn test_stream_peek_byte() {
        let data = vec![42, 43, 44];
        let handle = STREAMS.insert(Stream::from_memory(data));

        // Peek should return byte without advancing
        let peeked = fz_peek_byte(0, handle);
        assert_eq!(peeked, 42);
        assert_eq!(fz_tell(0, handle), 0); // Position unchanged

        // Read should advance
        let read = fz_read_byte(0, handle);
        assert_eq!(read, 42);
        assert_eq!(fz_tell(0, handle), 1);

        fz_drop_stream(0, handle);
    }

    #[test]
    fn test_stream_read_multiple() {
        let data = vec![1, 2, 3, 4, 5, 6, 7, 8];
        let handle = STREAMS.insert(Stream::from_memory(data));

        let mut buf = [0u8; 4];
        let n = fz_read(0, handle, buf.as_mut_ptr(), 4);
        assert_eq!(n, 4);
        assert_eq!(buf, [1, 2, 3, 4]);

        let n = fz_read(0, handle, buf.as_mut_ptr(), 4);
        assert_eq!(n, 4);
        assert_eq!(buf, [5, 6, 7, 8]);

        fz_drop_stream(0, handle);
    }

    #[test]
    fn test_stream_seek_modes() {
        let data = vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
        let handle = STREAMS.insert(Stream::from_memory(data));

        // SEEK_SET (0)
        fz_seek(0, handle, 5, 0);
        assert_eq!(fz_tell(0, handle), 5);

        // SEEK_CUR (1)
        fz_seek(0, handle, 2, 1);
        assert_eq!(fz_tell(0, handle), 7);

        // SEEK_END (2)
        fz_seek(0, handle, -3, 2);
        assert_eq!(fz_tell(0, handle), 7);

        fz_drop_stream(0, handle);
    }

    #[test]
    fn test_stream_read_uint16() {
        let data = vec![0x12, 0x34, 0x56, 0x78];
        let handle = STREAMS.insert(Stream::from_memory(data));

        let val = fz_read_uint16(0, handle);
        assert_eq!(val, 0x1234);

        fz_drop_stream(0, handle);
    }

    #[test]
    fn test_stream_read_uint32() {
        let data = vec![0x12, 0x34, 0x56, 0x78];
        let handle = STREAMS.insert(Stream::from_memory(data));

        let val = fz_read_uint32(0, handle);
        assert_eq!(val, 0x12345678);

        fz_drop_stream(0, handle);
    }

    #[test]
    fn test_stream_read_uint16_le() {
        let data = vec![0x34, 0x12, 0x78, 0x56];
        let handle = STREAMS.insert(Stream::from_memory(data));

        let val = fz_read_uint16_le(0, handle);
        assert_eq!(val, 0x1234);

        fz_drop_stream(0, handle);
    }

    #[test]
    fn test_stream_read_uint32_le() {
        let data = vec![0x78, 0x56, 0x34, 0x12];
        let handle = STREAMS.insert(Stream::from_memory(data));

        let val = fz_read_uint32_le(0, handle);
        assert_eq!(val, 0x12345678);

        fz_drop_stream(0, handle);
    }

    #[test]
    fn test_stream_invalid_handle() {
        assert_eq!(fz_read_byte(0, 0), -1);
        assert_eq!(fz_peek_byte(0, 0), -1);
        assert_eq!(fz_is_eof(0, 0), 1);
        assert_eq!(fz_tell(0, 0), 0);
    }

    #[test]
    fn test_stream_internal_new() {
        let stream = Stream::new();
        assert!(stream.data.is_empty());
        assert_eq!(stream.position, 0);
    }

    #[test]
    fn test_stream_internal_from_memory() {
        let stream = Stream::from_memory(vec![1, 2, 3]);
        assert_eq!(stream.data.len(), 3);
        assert_eq!(stream.position, 0);
    }

    #[test]
    fn test_stream_internal_read_byte() {
        let mut stream = Stream::from_memory(vec![10, 20, 30]);
        assert_eq!(stream.read_byte(), Some(10));
        assert_eq!(stream.read_byte(), Some(20));
        assert_eq!(stream.read_byte(), Some(30));
        assert_eq!(stream.read_byte(), None);
    }

    #[test]
    fn test_stream_internal_peek_byte() {
        let mut stream = Stream::from_memory(vec![99]);
        assert_eq!(stream.peek_byte(), Some(99));
        assert_eq!(stream.peek_byte(), Some(99)); // Should not advance
        assert_eq!(stream.read_byte(), Some(99));
        assert_eq!(stream.peek_byte(), None);
    }

    #[test]
    fn test_stream_internal_is_eof() {
        let mut stream = Stream::from_memory(vec![1]);
        assert!(!stream.is_eof());
        stream.read_byte();
        assert!(stream.is_eof());
    }

    #[test]
    fn test_stream_internal_seek_set() {
        let mut stream = Stream::from_memory(vec![0, 1, 2, 3, 4]);
        stream.seek(3, 0);
        assert_eq!(stream.tell(), 3);
        assert_eq!(stream.read_byte(), Some(3));
    }

    #[test]
    fn test_stream_internal_seek_cur() {
        let mut stream = Stream::from_memory(vec![0, 1, 2, 3, 4]);
        stream.seek(2, 0); // Start at 2
        stream.seek(2, 1); // Move forward 2
        assert_eq!(stream.tell(), 4);
    }

    #[test]
    fn test_stream_internal_seek_end() {
        let mut stream = Stream::from_memory(vec![0, 1, 2, 3, 4]);
        stream.seek(-2, 2); // 2 from end
        assert_eq!(stream.tell(), 3);
    }

    #[test]
    fn test_stream_internal_seek_invalid_whence() {
        let mut stream = Stream::from_memory(vec![0, 1, 2, 3, 4]);
        stream.seek(2, 0); // Start at 2
        stream.seek(99, 99); // Invalid whence
        assert_eq!(stream.tell(), 2); // Should remain unchanged
    }

    #[test]
    fn test_stream_internal_read() {
        let mut stream = Stream::from_memory(vec![1, 2, 3, 4, 5]);
        let mut buf = [0u8; 3];
        let n = stream.read(&mut buf);
        assert_eq!(n, 3);
        assert_eq!(&buf, &[1, 2, 3]);
    }

    #[test]
    fn test_stream_internal_read_empty() {
        let mut stream = Stream::from_memory(vec![1]);
        stream.read_byte();
        let mut buf = [0u8; 10];
        let n = stream.read(&mut buf);
        assert_eq!(n, 0);
    }

    #[test]
    fn test_stream_default() {
        let stream: Stream = Default::default();
        assert!(stream.data.is_empty());
        assert!(stream.is_eof());
    }

    #[test]
    fn test_fz_open_memory_empty() {
        let handle = fz_open_memory(0, std::ptr::null(), 0);
        assert_ne!(handle, 0);
        assert_eq!(fz_is_eof(0, handle), 1);
        fz_drop_stream(0, handle);
    }

    #[test]
    fn test_fz_open_file_null() {
        let handle = fz_open_file(0, std::ptr::null());
        assert_eq!(handle, 0);
    }

    #[test]
    fn test_fz_open_buffer_invalid() {
        let handle = fz_open_buffer(0, 99999);
        assert_eq!(handle, 0);
    }

    #[test]
    fn test_fz_read_null_data() {
        let data = vec![1, 2, 3];
        let handle = STREAMS.insert(Stream::from_memory(data));
        let n = fz_read(0, handle, std::ptr::null_mut(), 10);
        assert_eq!(n, 0);
        fz_drop_stream(0, handle);
    }

    #[test]
    fn test_fz_read_zero_len() {
        let data = vec![1, 2, 3];
        let handle = STREAMS.insert(Stream::from_memory(data));
        let mut buf = [0u8; 10];
        let n = fz_read(0, handle, buf.as_mut_ptr(), 0);
        assert_eq!(n, 0);
        fz_drop_stream(0, handle);
    }

    #[test]
    fn test_fz_seek_invalid_handle() {
        fz_seek(0, 99999, 10, 0); // Should not panic
    }

    #[test]
    fn test_fz_read_uint_incomplete() {
        let data = vec![0x12]; // Only 1 byte
        let handle = STREAMS.insert(Stream::from_memory(data));
        let val = fz_read_uint16(0, handle);
        assert_eq!(val, 0); // Incomplete read returns 0
        fz_drop_stream(0, handle);
    }

    #[test]
    fn test_fz_read_uint32_incomplete() {
        let data = vec![0x12, 0x34]; // Only 2 bytes
        let handle = STREAMS.insert(Stream::from_memory(data));
        let val = fz_read_uint32(0, handle);
        assert_eq!(val, 0); // Incomplete read returns 0
        fz_drop_stream(0, handle);
    }

    #[test]
    fn test_fz_read_uint_le_incomplete() {
        let data = vec![0x12]; // Only 1 byte
        let handle = STREAMS.insert(Stream::from_memory(data));
        let val = fz_read_uint16_le(0, handle);
        assert_eq!(val, 0);
        fz_drop_stream(0, handle);
    }

    #[test]
    fn test_fz_read_uint32_le_incomplete() {
        let data = vec![0x12, 0x34]; // Only 2 bytes
        let handle = STREAMS.insert(Stream::from_memory(data));
        let val = fz_read_uint32_le(0, handle);
        assert_eq!(val, 0);
        fz_drop_stream(0, handle);
    }
}
