//! BufferReader for consuming buffer contents

use super::core::Buffer;
use bytes::Bytes;
use std::io::{self, Read};

/// A reader for consuming buffer contents.
pub struct BufferReader {
    data: Bytes,
    position: usize,
}

impl BufferReader {
    /// Create a new reader from a buffer.
    pub fn new(buffer: Buffer) -> Self {
        Self {
            data: buffer.to_bytes(),
            position: 0,
        }
    }

    /// Returns the current read position.
    pub fn position(&self) -> usize {
        self.position
    }

    /// Returns the number of bytes remaining.
    pub fn remaining(&self) -> usize {
        self.data.len().saturating_sub(self.position)
    }

    /// Check if we've reached the end.
    pub fn is_eof(&self) -> bool {
        self.position >= self.data.len()
    }

    /// Peek at the next byte without consuming it.
    pub fn peek(&self) -> Option<u8> {
        self.data.get(self.position).copied()
    }

    /// Read a byte.
    pub fn read_byte(&mut self) -> Option<u8> {
        if self.position < self.data.len() {
            let byte = self.data[self.position];
            self.position += 1;
            Some(byte)
        } else {
            None
        }
    }

    /// Read a 16-bit unsigned integer in big-endian format.
    pub fn read_u16_be(&mut self) -> Option<u16> {
        if self.remaining() >= 2 {
            let value =
                u16::from_be_bytes([self.data[self.position], self.data[self.position + 1]]);
            self.position += 2;
            Some(value)
        } else {
            None
        }
    }

    /// Read a 32-bit unsigned integer in big-endian format.
    pub fn read_u32_be(&mut self) -> Option<u32> {
        if self.remaining() >= 4 {
            let value = u32::from_be_bytes([
                self.data[self.position],
                self.data[self.position + 1],
                self.data[self.position + 2],
                self.data[self.position + 3],
            ]);
            self.position += 4;
            Some(value)
        } else {
            None
        }
    }

    /// Read a 16-bit unsigned integer in little-endian format.
    pub fn read_u16_le(&mut self) -> Option<u16> {
        if self.remaining() >= 2 {
            let value =
                u16::from_le_bytes([self.data[self.position], self.data[self.position + 1]]);
            self.position += 2;
            Some(value)
        } else {
            None
        }
    }

    /// Read a 32-bit unsigned integer in little-endian format.
    pub fn read_u32_le(&mut self) -> Option<u32> {
        if self.remaining() >= 4 {
            let value = u32::from_le_bytes([
                self.data[self.position],
                self.data[self.position + 1],
                self.data[self.position + 2],
                self.data[self.position + 3],
            ]);
            self.position += 4;
            Some(value)
        } else {
            None
        }
    }

    /// Seek to a position.
    pub fn seek(&mut self, pos: usize) {
        self.position = pos.min(self.data.len());
    }

    /// Skip n bytes.
    pub fn skip(&mut self, n: usize) {
        self.position = (self.position + n).min(self.data.len());
    }

    /// Read exactly `buf.len()` bytes. Returns true if successful.
    pub fn read_exact(&mut self, buf: &mut [u8]) -> bool {
        if self.remaining() >= buf.len() {
            buf.copy_from_slice(&self.data[self.position..self.position + buf.len()]);
            self.position += buf.len();
            true
        } else {
            false
        }
    }

    /// Read a line (up to and including newline).
    pub fn read_line(&mut self) -> Option<Vec<u8>> {
        if self.is_eof() {
            return None;
        }
        let start = self.position;
        while self.position < self.data.len() {
            if self.data[self.position] == b'\n' {
                self.position += 1;
                return Some(self.data[start..self.position].to_vec());
            }
            self.position += 1;
        }
        // Return remaining data if no newline found
        if start < self.data.len() {
            Some(self.data[start..].to_vec())
        } else {
            None
        }
    }

    /// Get the remaining data as a slice.
    pub fn remaining_slice(&self) -> &[u8] {
        &self.data[self.position..]
    }

    /// Read a 24-bit unsigned integer in big-endian format.
    pub fn read_u24_be(&mut self) -> Option<u32> {
        if self.remaining() >= 3 {
            let value = ((self.data[self.position] as u32) << 16)
                | ((self.data[self.position + 1] as u32) << 8)
                | (self.data[self.position + 2] as u32);
            self.position += 3;
            Some(value)
        } else {
            None
        }
    }

    /// Read a 16-bit signed integer in big-endian format.
    pub fn read_i16_be(&mut self) -> Option<i16> {
        if self.remaining() >= 2 {
            let value =
                i16::from_be_bytes([self.data[self.position], self.data[self.position + 1]]);
            self.position += 2;
            Some(value)
        } else {
            None
        }
    }

    /// Read a 32-bit signed integer in big-endian format.
    pub fn read_i32_be(&mut self) -> Option<i32> {
        if self.remaining() >= 4 {
            let value = i32::from_be_bytes([
                self.data[self.position],
                self.data[self.position + 1],
                self.data[self.position + 2],
                self.data[self.position + 3],
            ]);
            self.position += 4;
            Some(value)
        } else {
            None
        }
    }

    /// Read a 16-bit signed integer in little-endian format.
    pub fn read_i16_le(&mut self) -> Option<i16> {
        if self.remaining() >= 2 {
            let value =
                i16::from_le_bytes([self.data[self.position], self.data[self.position + 1]]);
            self.position += 2;
            Some(value)
        } else {
            None
        }
    }

    /// Read a 32-bit signed integer in little-endian format.
    pub fn read_i32_le(&mut self) -> Option<i32> {
        if self.remaining() >= 4 {
            let value = i32::from_le_bytes([
                self.data[self.position],
                self.data[self.position + 1],
                self.data[self.position + 2],
                self.data[self.position + 3],
            ]);
            self.position += 4;
            Some(value)
        } else {
            None
        }
    }
}

impl Read for BufferReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let remaining = &self.data[self.position..];
        let to_read = buf.len().min(remaining.len());
        buf[..to_read].copy_from_slice(&remaining[..to_read]);
        self.position += to_read;
        Ok(to_read)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buffer_reader() {
        let buf = Buffer::from_slice(b"Hello, World!");
        let mut reader = BufferReader::new(buf);

        assert_eq!(reader.remaining(), 13);
        assert_eq!(reader.read_byte(), Some(b'H'));
        assert_eq!(reader.remaining(), 12);

        let mut bytes = [0u8; 5];
        assert!(reader.read_exact(&mut bytes));
        assert_eq!(&bytes, b"ello,");
    }

    #[test]
    fn test_buffer_reader_integers() {
        let mut buf = Buffer::new(20);
        buf.append_int16_be(0x1234);
        buf.append_int32_be(0x12345678);
        buf.append_int16_le(0x1234);
        buf.append_int32_le(0x12345678);

        let mut reader = BufferReader::new(buf);
        assert_eq!(reader.read_u16_be(), Some(0x1234));
        assert_eq!(reader.read_u32_be(), Some(0x12345678));
        assert_eq!(reader.read_u16_le(), Some(0x1234));
        assert_eq!(reader.read_u32_le(), Some(0x12345678));
    }

    #[test]
    fn test_buffer_reader_seek() {
        let buf = Buffer::from_slice(b"0123456789");
        let mut reader = BufferReader::new(buf);

        reader.seek(5);
        assert_eq!(reader.read_byte(), Some(b'5'));

        reader.skip(2);
        assert_eq!(reader.read_byte(), Some(b'8'));
    }

    #[test]
    fn test_buffer_reader_read_line() {
        let buf = Buffer::from_slice(b"Line 1\nLine 2\nLine 3");
        let mut reader = BufferReader::new(buf);

        assert_eq!(reader.read_line(), Some(b"Line 1\n".to_vec()));
        assert_eq!(reader.read_line(), Some(b"Line 2\n".to_vec()));
        assert_eq!(reader.read_line(), Some(b"Line 3".to_vec()));
        assert_eq!(reader.read_line(), None);
    }

    #[test]
    fn test_buffer_reader_position() {
        let buf = Buffer::from_slice(b"01234");
        let mut reader = BufferReader::new(buf);

        assert_eq!(reader.position(), 0);
        reader.read_byte();
        assert_eq!(reader.position(), 1);
        reader.read_byte();
        assert_eq!(reader.position(), 2);
    }

    #[test]
    fn test_buffer_reader_is_eof() {
        let buf = Buffer::from_slice(b"AB");
        let mut reader = BufferReader::new(buf);

        assert!(!reader.is_eof());
        reader.read_byte();
        assert!(!reader.is_eof());
        reader.read_byte();
        assert!(reader.is_eof());
    }

    #[test]
    fn test_buffer_reader_peek() {
        let buf = Buffer::from_slice(b"Hello");
        let mut reader = BufferReader::new(buf);

        assert_eq!(reader.peek(), Some(b'H'));
        assert_eq!(reader.peek(), Some(b'H')); // Peek doesn't consume
        assert_eq!(reader.read_byte(), Some(b'H'));
        assert_eq!(reader.peek(), Some(b'e'));
    }

    #[test]
    fn test_buffer_reader_peek_eof() {
        let buf = Buffer::from_slice(b"A");
        let mut reader = BufferReader::new(buf);

        reader.read_byte();
        assert_eq!(reader.peek(), None);
    }

    #[test]
    fn test_buffer_reader_remaining_slice() {
        let buf = Buffer::from_slice(b"0123456789");
        let mut reader = BufferReader::new(buf);

        assert_eq!(reader.remaining_slice(), b"0123456789");
        reader.read_byte();
        assert_eq!(reader.remaining_slice(), b"123456789");
        reader.seek(5);
        assert_eq!(reader.remaining_slice(), b"56789");
    }

    #[test]
    fn test_buffer_reader_read_u24_be() {
        let buf = Buffer::from_slice(&[0x12, 0x34, 0x56, 0x78]);
        let mut reader = BufferReader::new(buf);

        assert_eq!(reader.read_u24_be(), Some(0x123456));
        assert_eq!(reader.read_u24_be(), None); // Not enough bytes
    }

    #[test]
    fn test_buffer_reader_read_i16_be() {
        let buf = Buffer::from_slice(&[0x80, 0x00, 0x7F, 0xFF]);
        let mut reader = BufferReader::new(buf);

        assert_eq!(reader.read_i16_be(), Some(-32768)); // 0x8000
        assert_eq!(reader.read_i16_be(), Some(32767)); // 0x7FFF
    }

    #[test]
    fn test_buffer_reader_read_i32_be() {
        let buf = Buffer::from_slice(&[0x80, 0x00, 0x00, 0x00, 0x7F, 0xFF, 0xFF, 0xFF]);
        let mut reader = BufferReader::new(buf);

        assert_eq!(reader.read_i32_be(), Some(-2147483648)); // 0x80000000
        assert_eq!(reader.read_i32_be(), Some(2147483647)); // 0x7FFFFFFF
    }

    #[test]
    fn test_buffer_reader_read_i16_le() {
        let buf = Buffer::from_slice(&[0x00, 0x80, 0xFF, 0x7F]);
        let mut reader = BufferReader::new(buf);

        assert_eq!(reader.read_i16_le(), Some(-32768)); // 0x8000 (LE)
        assert_eq!(reader.read_i16_le(), Some(32767)); // 0x7FFF (LE)
    }

    #[test]
    fn test_buffer_reader_read_i32_le() {
        let buf = Buffer::from_slice(&[0x00, 0x00, 0x00, 0x80, 0xFF, 0xFF, 0xFF, 0x7F]);
        let mut reader = BufferReader::new(buf);

        assert_eq!(reader.read_i32_le(), Some(-2147483648)); // 0x80000000 (LE)
        assert_eq!(reader.read_i32_le(), Some(2147483647)); // 0x7FFFFFFF (LE)
    }

    #[test]
    fn test_buffer_reader_read_trait() {
        let buf = Buffer::from_slice(b"Hello, World!");
        let mut reader = BufferReader::new(buf);

        let mut bytes = [0u8; 5];
        let n = reader.read(&mut bytes).unwrap();
        assert_eq!(n, 5);
        assert_eq!(&bytes, b"Hello");

        let mut bytes = [0u8; 100];
        let n = reader.read(&mut bytes).unwrap();
        assert_eq!(n, 8); // ", World!" remaining
        assert_eq!(&bytes[..n], b", World!");
    }

    #[test]
    fn test_buffer_reader_read_exact_insufficient() {
        let buf = Buffer::from_slice(b"Short");
        let mut reader = BufferReader::new(buf);

        let mut bytes = [0u8; 10];
        assert!(!reader.read_exact(&mut bytes)); // Not enough data
    }

    #[test]
    fn test_buffer_reader_read_u16_be_insufficient() {
        let buf = Buffer::from_slice(&[0x12]);
        let mut reader = BufferReader::new(buf);

        assert_eq!(reader.read_u16_be(), None);
    }

    #[test]
    fn test_buffer_reader_read_u32_be_insufficient() {
        let buf = Buffer::from_slice(&[0x12, 0x34]);
        let mut reader = BufferReader::new(buf);

        assert_eq!(reader.read_u32_be(), None);
    }

    #[test]
    fn test_buffer_reader_read_u16_le_insufficient() {
        let buf = Buffer::from_slice(&[0x12]);
        let mut reader = BufferReader::new(buf);

        assert_eq!(reader.read_u16_le(), None);
    }

    #[test]
    fn test_buffer_reader_read_u32_le_insufficient() {
        let buf = Buffer::from_slice(&[0x12, 0x34, 0x56]);
        let mut reader = BufferReader::new(buf);

        assert_eq!(reader.read_u32_le(), None);
    }

    #[test]
    fn test_buffer_reader_seek_past_end() {
        let buf = Buffer::from_slice(b"Short");
        let mut reader = BufferReader::new(buf);

        reader.seek(1000); // Beyond buffer
        assert!(reader.is_eof());
        assert_eq!(reader.remaining(), 0);
    }

    #[test]
    fn test_buffer_reader_skip_past_end() {
        let buf = Buffer::from_slice(b"Test");
        let mut reader = BufferReader::new(buf);

        reader.skip(100); // Skip way past end
        assert!(reader.is_eof());
        assert_eq!(reader.read_byte(), None);
    }

    #[test]
    fn test_buffer_reader_read_line_empty() {
        let buf = Buffer::from_slice(b"");
        let mut reader = BufferReader::new(buf);

        assert_eq!(reader.read_line(), None);
    }

    #[test]
    fn test_buffer_reader_empty_buffer() {
        let buf = Buffer::new(0);
        let mut reader = BufferReader::new(buf);

        assert!(reader.is_eof());
        assert_eq!(reader.remaining(), 0);
        assert_eq!(reader.read_byte(), None);
        assert_eq!(reader.peek(), None);
    }
}
