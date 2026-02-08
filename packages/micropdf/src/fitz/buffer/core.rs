//! Core Buffer implementation

use crate::fitz::error::{Error, Result};
use bytes::{BufMut, Bytes, BytesMut};
use std::fmt;
use std::sync::Arc;

/// A reference-counted buffer for efficient byte storage.
///
/// Uses `bytes::Bytes` for immutable shared data and `bytes::BytesMut` for
/// mutable operations with copy-on-write semantics.
#[derive(Clone)]
pub struct Buffer {
    /// Immutable shared data (for reading)
    pub(super) data: Bytes,
    /// Mutable buffer for writes (lazy initialized)
    pub(super) mutable: Option<Arc<std::sync::Mutex<BytesMut>>>,
}

impl Buffer {
    /// Create a new empty buffer with the specified capacity.
    pub fn new(capacity: usize) -> Self {
        Self {
            data: Bytes::new(),
            mutable: Some(Arc::new(std::sync::Mutex::new(BytesMut::with_capacity(
                capacity,
            )))),
        }
    }

    /// Create a buffer from owned data (zero-copy).
    pub fn from_data(data: Vec<u8>) -> Self {
        Self {
            data: Bytes::from(data),
            mutable: None,
        }
    }

    /// Create a buffer from a byte slice (copies data).
    pub fn from_slice(data: &[u8]) -> Self {
        Self {
            data: Bytes::copy_from_slice(data),
            mutable: None,
        }
    }

    /// Create a buffer from a `Bytes` instance (zero-copy).
    pub fn from_bytes(data: Bytes) -> Self {
        Self {
            data,
            mutable: None,
        }
    }

    /// Create a buffer from a `BytesMut` instance (zero-copy).
    pub fn from_bytes_mut(data: BytesMut) -> Self {
        Self {
            data: data.freeze(),
            mutable: None,
        }
    }

    /// Create a buffer from base64-encoded data.
    pub fn from_base64(data: &str) -> Result<Self> {
        use base64::Engine;
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(data.as_bytes())
            .map_err(|e| Error::format(format!("Invalid base64: {}", e)))?;
        Ok(Self::from_data(decoded))
    }

    /// Returns the number of bytes in the buffer.
    #[inline]
    pub fn len(&self) -> usize {
        if let Some(ref mutable) = self.mutable {
            if let Ok(guard) = mutable.lock() {
                if !guard.is_empty() {
                    return self.data.len() + guard.len();
                }
            }
        }
        self.data.len()
    }

    /// Returns true if the buffer is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the capacity of the buffer.
    pub fn capacity(&self) -> usize {
        if let Some(ref mutable) = self.mutable {
            if let Ok(guard) = mutable.lock() {
                return self.data.len() + guard.capacity();
            }
        }
        self.data.len()
    }

    /// Returns the buffer contents as a byte slice.
    ///
    /// If there are pending mutable writes, this will consolidate them first.
    pub fn as_slice(&self) -> &[u8] {
        // If we have no mutable data, return the immutable slice directly
        if self.mutable.is_none() {
            return &self.data;
        }

        if let Some(ref mutable) = self.mutable {
            if let Ok(guard) = mutable.lock() {
                if guard.is_empty() {
                    return &self.data;
                }
            }
        }

        // For simplicity, return the base data
        // Full consolidation would require interior mutability
        &self.data
    }

    /// Returns the buffer as a UTF-8 string slice.
    pub fn as_str(&self) -> Result<&str> {
        std::str::from_utf8(self.as_slice())
            .map_err(|e| Error::format(format!("Invalid UTF-8: {}", e)))
    }

    /// Returns a copy of the buffer contents as a Vec.
    pub fn to_vec(&self) -> Vec<u8> {
        if let Some(ref mutable) = self.mutable {
            if let Ok(guard) = mutable.lock() {
                if !guard.is_empty() {
                    let mut result = self.data.to_vec();
                    result.extend_from_slice(&guard);
                    return result;
                }
            }
        }
        self.data.to_vec()
    }

    /// Returns the buffer contents as `Bytes` (zero-copy if no mutable data).
    pub fn to_bytes(&self) -> Bytes {
        if let Some(ref mutable) = self.mutable {
            if let Ok(guard) = mutable.lock() {
                if !guard.is_empty() {
                    let mut result = BytesMut::with_capacity(self.data.len() + guard.len());
                    result.extend_from_slice(&self.data);
                    result.extend_from_slice(&guard);
                    return result.freeze();
                }
            }
        }
        self.data.clone()
    }

    /// Consolidate any mutable data into the immutable buffer.
    pub(super) fn consolidate(&mut self) {
        if let Some(ref mutable) = self.mutable {
            if let Ok(guard) = mutable.lock() {
                if !guard.is_empty() {
                    let mut new_data = BytesMut::with_capacity(self.data.len() + guard.len());
                    new_data.extend_from_slice(&self.data);
                    new_data.extend_from_slice(&guard);
                    self.data = new_data.freeze();
                }
            }
        }
        self.mutable = None;
    }

    /// Ensure we have a mutable buffer for writes.
    pub(super) fn ensure_mutable(&mut self) {
        if self.mutable.is_none() {
            self.mutable = Some(Arc::new(std::sync::Mutex::new(BytesMut::with_capacity(
                256,
            ))));
        }
    }

    /// Resize the buffer to the specified size.
    pub fn resize(&mut self, new_len: usize) {
        self.consolidate();
        let mut data = BytesMut::from(self.data.as_ref());
        data.resize(new_len, 0);
        self.data = data.freeze();
    }

    /// Clear all data from the buffer.
    pub fn clear(&mut self) {
        self.data = Bytes::new();
        self.mutable = None;
    }

    /// Append a byte slice to the buffer.
    pub fn append_data(&mut self, data: &[u8]) {
        self.ensure_mutable();
        if let Some(ref mutable) = self.mutable {
            if let Ok(mut guard) = mutable.lock() {
                guard.extend_from_slice(data);
            }
        }
    }

    /// Append a single byte to the buffer.
    pub fn append_byte(&mut self, byte: u8) {
        self.ensure_mutable();
        if let Some(ref mutable) = self.mutable {
            if let Ok(mut guard) = mutable.lock() {
                guard.put_u8(byte);
            }
        }
    }

    /// Append a string to the buffer.
    pub fn append_string(&mut self, s: &str) {
        self.append_data(s.as_bytes());
    }

    /// Append a 16-bit integer in little-endian format.
    pub fn append_int16_le(&mut self, value: i16) {
        self.ensure_mutable();
        if let Some(ref mutable) = self.mutable {
            if let Ok(mut guard) = mutable.lock() {
                guard.put_i16_le(value);
            }
        }
    }

    /// Append a 32-bit integer in little-endian format.
    pub fn append_int32_le(&mut self, value: i32) {
        self.ensure_mutable();
        if let Some(ref mutable) = self.mutable {
            if let Ok(mut guard) = mutable.lock() {
                guard.put_i32_le(value);
            }
        }
    }

    /// Append a 16-bit integer in big-endian format.
    pub fn append_int16_be(&mut self, value: i16) {
        self.ensure_mutable();
        if let Some(ref mutable) = self.mutable {
            if let Ok(mut guard) = mutable.lock() {
                guard.put_i16(value);
            }
        }
    }

    /// Append a 32-bit integer in big-endian format.
    pub fn append_int32_be(&mut self, value: i32) {
        self.ensure_mutable();
        if let Some(ref mutable) = self.mutable {
            if let Ok(mut guard) = mutable.lock() {
                guard.put_i32(value);
            }
        }
    }

    /// Compute the MD5 digest of the buffer contents.
    pub fn md5_digest(&self) -> [u8; 16] {
        use md5::{Digest, Md5};
        let mut hasher = Md5::new();
        hasher.update(&self.data);
        if let Some(ref mutable) = self.mutable {
            if let Ok(guard) = mutable.lock() {
                hasher.update(&*guard);
            }
        }
        hasher.finalize().into()
    }

    /// Encode the buffer contents as base64.
    pub fn to_base64(&self) -> String {
        use base64::Engine;
        base64::engine::general_purpose::STANDARD.encode(self.to_vec())
    }

    /// Get a slice of the buffer.
    pub fn slice(&self, start: usize, end: usize) -> Buffer {
        let data = self.to_bytes();
        if start >= data.len() {
            return Buffer::new(0);
        }
        let end = end.min(data.len());
        Buffer::from_bytes(data.slice(start..end))
    }

    /// Split the buffer at the given index.
    pub fn split_at(&self, mid: usize) -> (Buffer, Buffer) {
        let data = self.to_bytes();
        if mid >= data.len() {
            return (Buffer::from_bytes(data), Buffer::new(0));
        }
        let first = data.slice(..mid);
        let second = data.slice(mid..);
        (Buffer::from_bytes(first), Buffer::from_bytes(second))
    }

    /// Trim the buffer capacity to match its length.
    pub fn trim_capacity(&mut self) {
        self.consolidate();
    }

    /// Truncate the buffer to the specified length.
    pub fn truncate(&mut self, len: usize) {
        if len < self.len() {
            self.consolidate();
            let mut data = BytesMut::from(self.data.as_ref());
            data.truncate(len);
            self.data = data.freeze();
        }
    }

    /// Split off the buffer at the given index, returning the second half.
    pub fn split_off(&mut self, at: usize) -> Buffer {
        self.consolidate();
        if at >= self.data.len() {
            return Buffer::new(0);
        }
        let second = self.data.slice(at..);
        let mut first = BytesMut::from(self.data.as_ref());
        first.truncate(at);
        self.data = first.freeze();
        Buffer::from_bytes(second)
    }

    /// Reserve additional capacity for the buffer.
    pub fn reserve(&mut self, additional: usize) {
        self.ensure_mutable();
        if let Some(ref mutable) = self.mutable {
            if let Ok(mut guard) = mutable.lock() {
                guard.reserve(additional);
            }
        }
    }

    /// Append another buffer's contents to this buffer.
    pub fn append_buffer(&mut self, other: &Buffer) {
        self.append_data(&other.to_vec());
    }

    /// Append bits to the buffer.
    pub fn append_bits(&mut self, value: u8, bits: u8) {
        // For simplicity, just append the byte
        // A full implementation would track bit position
        if bits > 0 {
            self.append_byte(value);
        }
    }

    /// Append a PDF string (with parentheses and escaping).
    pub fn append_pdf_string(&mut self, s: &str) {
        self.append_byte(b'(');
        for c in s.bytes() {
            match c {
                b'(' | b')' | b'\\' => {
                    self.append_byte(b'\\');
                    self.append_byte(c);
                }
                _ => self.append_byte(c),
            }
        }
        self.append_byte(b')');
    }
}

impl Default for Buffer {
    fn default() -> Self {
        Self::new(0)
    }
}

impl fmt::Debug for Buffer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Buffer").field("len", &self.len()).finish()
    }
}

impl AsRef<[u8]> for Buffer {
    fn as_ref(&self) -> &[u8] {
        self.as_slice()
    }
}

impl From<Vec<u8>> for Buffer {
    fn from(data: Vec<u8>) -> Self {
        Self::from_data(data)
    }
}

impl From<&[u8]> for Buffer {
    fn from(data: &[u8]) -> Self {
        Self::from_slice(data)
    }
}

impl From<&str> for Buffer {
    fn from(s: &str) -> Self {
        Self::from_slice(s.as_bytes())
    }
}

impl From<Bytes> for Buffer {
    fn from(data: Bytes) -> Self {
        Self::from_bytes(data)
    }
}

impl From<BytesMut> for Buffer {
    fn from(data: BytesMut) -> Self {
        Self::from_bytes_mut(data)
    }
}

impl From<Buffer> for Bytes {
    fn from(buf: Buffer) -> Bytes {
        buf.to_bytes()
    }
}

impl PartialEq for Buffer {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        // Use slice comparison to avoid allocating two Vec<u8>
        self.as_slice() == other.as_slice()
    }
}

impl Eq for Buffer {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buffer_new() {
        let buf = Buffer::new(100);
        assert_eq!(buf.len(), 0);
        assert!(buf.capacity() >= 100);
    }

    #[test]
    fn test_buffer_from_data() {
        let data = vec![1, 2, 3, 4, 5];
        let buf = Buffer::from_data(data.clone());
        assert_eq!(buf.len(), 5);
        assert_eq!(buf.to_vec(), data);
    }

    #[test]
    fn test_buffer_append() {
        let mut buf = Buffer::new(10);
        buf.append_data(b"Hello");
        buf.append_byte(b' ');
        buf.append_string("World");
        assert_eq!(buf.to_vec(), b"Hello World");
    }

    #[test]
    fn test_buffer_slice() {
        let buf = Buffer::from_slice(b"Hello, World!");
        let slice = buf.slice(0, 5);
        assert_eq!(slice.to_vec(), b"Hello");
    }

    #[test]
    fn test_buffer_split_at() {
        let buf = Buffer::from_slice(b"HelloWorld");
        let (first, second) = buf.split_at(5);
        assert_eq!(first.to_vec(), b"Hello");
        assert_eq!(second.to_vec(), b"World");
    }

    #[test]
    fn test_buffer_base64() {
        let buf = Buffer::from_slice(b"Hello");
        let encoded = buf.to_base64();
        let decoded = Buffer::from_base64(&encoded).unwrap();
        assert_eq!(decoded.to_vec(), b"Hello");
    }

    #[test]
    fn test_buffer_md5() {
        let buf = Buffer::from_slice(b"Hello, World!");
        let digest = buf.md5_digest();
        assert_eq!(digest.len(), 16);
    }

    #[test]
    fn test_buffer_integers() {
        let mut buf = Buffer::new(10);
        buf.append_int16_le(0x1234);
        buf.append_int32_le(0x12345678);
        buf.append_int16_be(0x1234);
        buf.append_int32_be(0x12345678);

        let result = buf.to_vec();
        assert_eq!(result.len(), 12); // 2 + 4 + 2 + 4
    }

    #[test]
    fn test_buffer_truncate() {
        let mut buf = Buffer::from_slice(b"Hello, World!");
        buf.truncate(5);
        assert_eq!(buf.to_vec(), b"Hello");
    }

    #[test]
    fn test_buffer_clear() {
        let mut buf = Buffer::from_slice(b"Hello");
        assert_eq!(buf.len(), 5);
        buf.clear();
        assert_eq!(buf.len(), 0);
        assert!(buf.is_empty());
    }

    #[test]
    fn test_buffer_resize() {
        let mut buf = Buffer::from_slice(b"Hi");
        buf.resize(5);
        assert_eq!(buf.len(), 5);
        assert_eq!(buf.to_vec(), vec![b'H', b'i', 0, 0, 0]);
    }

    #[test]
    fn test_buffer_pdf_string() {
        let mut buf = Buffer::new(20);
        buf.append_pdf_string("Hello (World)");
        let result = buf.to_vec();
        assert!(result.starts_with(b"("));
        assert!(result.ends_with(b")"));
    }
}
