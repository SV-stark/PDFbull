//! Cross-reference stream support
//!
//! This module provides support for reading and writing PDF 1.5+ cross-reference streams,
//! which replace the traditional xref table format.

use super::error::{QpdfError, Result};
use std::io::{Read, Write};

/// Entry type in an XRef stream
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XRefEntryType {
    /// Free object (type 0)
    Free,
    /// Uncompressed object (type 1)
    Uncompressed,
    /// Compressed object in object stream (type 2)
    Compressed,
}

/// An entry in the cross-reference table/stream
#[derive(Debug, Clone)]
pub struct XRefEntry {
    /// Entry type
    pub entry_type: XRefEntryType,
    /// For type 0: next free object, type 1: byte offset, type 2: object stream number
    pub field2: u64,
    /// For type 0: generation if reused, type 1: generation, type 2: index in object stream
    pub field3: u32,
}

impl XRefEntry {
    /// Create a free entry
    pub fn free(next_free: u64, generation: u32) -> Self {
        Self {
            entry_type: XRefEntryType::Free,
            field2: next_free,
            field3: generation,
        }
    }

    /// Create an uncompressed entry
    pub fn uncompressed(offset: u64, generation: u32) -> Self {
        Self {
            entry_type: XRefEntryType::Uncompressed,
            field2: offset,
            field3: generation,
        }
    }

    /// Create a compressed entry
    pub fn compressed(obj_stream: u64, index: u32) -> Self {
        Self {
            entry_type: XRefEntryType::Compressed,
            field2: obj_stream,
            field3: index,
        }
    }

    /// Check if this is a free entry
    pub fn is_free(&self) -> bool {
        self.entry_type == XRefEntryType::Free
    }

    /// Check if this is an uncompressed entry
    pub fn is_uncompressed(&self) -> bool {
        self.entry_type == XRefEntryType::Uncompressed
    }

    /// Check if this is a compressed entry
    pub fn is_compressed(&self) -> bool {
        self.entry_type == XRefEntryType::Compressed
    }

    /// Get offset (for uncompressed entries)
    pub fn offset(&self) -> Option<u64> {
        if self.entry_type == XRefEntryType::Uncompressed {
            Some(self.field2)
        } else {
            None
        }
    }

    /// Get generation (for uncompressed and free entries)
    pub fn generation(&self) -> Option<u32> {
        match self.entry_type {
            XRefEntryType::Uncompressed | XRefEntryType::Free => Some(self.field3),
            _ => None,
        }
    }

    /// Get object stream number (for compressed entries)
    pub fn object_stream(&self) -> Option<u64> {
        if self.entry_type == XRefEntryType::Compressed {
            Some(self.field2)
        } else {
            None
        }
    }

    /// Get index in object stream (for compressed entries)
    pub fn stream_index(&self) -> Option<u32> {
        if self.entry_type == XRefEntryType::Compressed {
            Some(self.field3)
        } else {
            None
        }
    }
}

/// XRef stream decoder
pub struct XRefStreamDecoder {
    /// Width of type field (usually 1)
    w0: usize,
    /// Width of field 2
    w1: usize,
    /// Width of field 3
    w2: usize,
    /// Index ranges (start, count pairs)
    index: Vec<(u32, u32)>,
}

impl XRefStreamDecoder {
    /// Create a new decoder with the given W array values and index
    pub fn new(w: &[usize], index: Option<&[u32]>) -> Result<Self> {
        if w.len() != 3 {
            return Err(QpdfError::XRef("W array must have 3 elements".to_string()));
        }

        let index = if let Some(idx) = index {
            if idx.len() % 2 != 0 {
                return Err(QpdfError::XRef(
                    "Index array must have even number of elements".to_string(),
                ));
            }
            idx.chunks(2).map(|c| (c[0], c[1])).collect()
        } else {
            Vec::new()
        };

        Ok(Self {
            w0: w[0],
            w1: w[1],
            w2: w[2],
            index,
        })
    }

    /// Get the entry width in bytes
    pub fn entry_width(&self) -> usize {
        self.w0 + self.w1 + self.w2
    }

    /// Decode a single entry from raw bytes
    pub fn decode_entry(&self, data: &[u8]) -> Result<XRefEntry> {
        if data.len() < self.entry_width() {
            return Err(QpdfError::XRef(
                "Not enough data for XRef entry".to_string(),
            ));
        }

        let mut pos = 0;

        // Read type field
        let entry_type = if self.w0 > 0 {
            read_uint(&data[pos..pos + self.w0])
        } else {
            1 // Default to type 1 if w0 is 0
        };
        pos += self.w0;

        // Read field 2
        let field2 = read_uint(&data[pos..pos + self.w1]);
        pos += self.w1;

        // Read field 3
        let field3 = if self.w2 > 0 {
            read_uint(&data[pos..pos + self.w2]) as u32
        } else {
            0
        };

        let entry_type = match entry_type {
            0 => XRefEntryType::Free,
            1 => XRefEntryType::Uncompressed,
            2 => XRefEntryType::Compressed,
            _ => {
                return Err(QpdfError::XRef(format!(
                    "Unknown XRef entry type: {}",
                    entry_type
                )));
            }
        };

        Ok(XRefEntry {
            entry_type,
            field2,
            field3,
        })
    }

    /// Decode all entries from raw stream data
    pub fn decode_all(&self, data: &[u8]) -> Result<Vec<(u32, XRefEntry)>> {
        let entry_width = self.entry_width();
        let entry_count = data.len() / entry_width;

        if data.len() % entry_width != 0 {
            return Err(QpdfError::XRef(
                "XRef stream data length not multiple of entry width".to_string(),
            ));
        }

        let mut entries = Vec::with_capacity(entry_count);
        let mut data_pos = 0;
        let mut obj_num = 0u32;

        if self.index.is_empty() {
            // No index, assume continuous from 0
            for i in 0..entry_count {
                let entry = self.decode_entry(&data[data_pos..data_pos + entry_width])?;
                entries.push((i as u32, entry));
                data_pos += entry_width;
            }
        } else {
            // Use index ranges
            for &(start, count) in &self.index {
                for i in 0..count {
                    if data_pos + entry_width > data.len() {
                        return Err(QpdfError::XRef(
                            "XRef stream data exhausted before index ranges".to_string(),
                        ));
                    }
                    let entry = self.decode_entry(&data[data_pos..data_pos + entry_width])?;
                    entries.push((start + i, entry));
                    data_pos += entry_width;
                }
            }
        }

        Ok(entries)
    }
}

/// XRef stream encoder
pub struct XRefStreamEncoder {
    /// Width of type field
    w0: usize,
    /// Width of field 2
    w1: usize,
    /// Width of field 3
    w2: usize,
}

impl XRefStreamEncoder {
    /// Create a new encoder with the given field widths
    pub fn new(w0: usize, w1: usize, w2: usize) -> Self {
        Self { w0, w1, w2 }
    }

    /// Calculate optimal field widths for a set of entries
    pub fn optimal_widths(entries: &[(u32, XRefEntry)]) -> (usize, usize, usize) {
        let mut max_field2: u64 = 0;
        let mut max_field3: u32 = 0;

        for (_, entry) in entries {
            max_field2 = max_field2.max(entry.field2);
            max_field3 = max_field3.max(entry.field3);
        }

        (
            1, // Type always fits in 1 byte
            bytes_needed_u64(max_field2),
            bytes_needed_u32(max_field3),
        )
    }

    /// Get W array for this encoder
    pub fn w_array(&self) -> Vec<usize> {
        vec![self.w0, self.w1, self.w2]
    }

    /// Encode a single entry
    pub fn encode_entry(&self, entry: &XRefEntry) -> Vec<u8> {
        let mut data = Vec::with_capacity(self.w0 + self.w1 + self.w2);

        // Type field
        let entry_type = match entry.entry_type {
            XRefEntryType::Free => 0u64,
            XRefEntryType::Uncompressed => 1,
            XRefEntryType::Compressed => 2,
        };
        write_uint(&mut data, entry_type, self.w0);

        // Field 2
        write_uint(&mut data, entry.field2, self.w1);

        // Field 3
        write_uint(&mut data, entry.field3 as u64, self.w2);

        data
    }

    /// Encode all entries
    pub fn encode_all(&self, entries: &[(u32, XRefEntry)]) -> Vec<u8> {
        let mut data = Vec::with_capacity(entries.len() * (self.w0 + self.w1 + self.w2));

        for (_, entry) in entries {
            data.extend(self.encode_entry(entry));
        }

        data
    }
}

/// Read an unsigned integer from big-endian bytes
fn read_uint(data: &[u8]) -> u64 {
    let mut value: u64 = 0;
    for &byte in data {
        value = (value << 8) | byte as u64;
    }
    value
}

/// Write an unsigned integer as big-endian bytes
fn write_uint(out: &mut Vec<u8>, value: u64, width: usize) {
    for i in (0..width).rev() {
        out.push((value >> (i * 8)) as u8);
    }
}

/// Calculate bytes needed to represent a u64 value
fn bytes_needed_u64(value: u64) -> usize {
    if value == 0 {
        1
    } else {
        ((64 - value.leading_zeros()) as usize + 7) / 8
    }
}

/// Calculate bytes needed to represent a u32 value
fn bytes_needed_u32(value: u32) -> usize {
    if value == 0 {
        1
    } else {
        ((32 - value.leading_zeros()) as usize + 7) / 8
    }
}

/// Build index array from entries
pub fn build_index(entries: &[(u32, XRefEntry)]) -> Vec<u32> {
    if entries.is_empty() {
        return Vec::new();
    }

    let mut index = Vec::new();
    let mut range_start = entries[0].0;
    let mut range_count = 1u32;
    let mut prev_obj = entries[0].0;

    for &(obj, _) in &entries[1..] {
        if obj == prev_obj + 1 {
            range_count += 1;
        } else {
            index.push(range_start);
            index.push(range_count);
            range_start = obj;
            range_count = 1;
        }
        prev_obj = obj;
    }

    index.push(range_start);
    index.push(range_count);

    index
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xref_entry_types() {
        let free = XRefEntry::free(10, 65535);
        assert!(free.is_free());
        assert_eq!(free.generation(), Some(65535));

        let uncompressed = XRefEntry::uncompressed(12345, 0);
        assert!(uncompressed.is_uncompressed());
        assert_eq!(uncompressed.offset(), Some(12345));
        assert_eq!(uncompressed.generation(), Some(0));

        let compressed = XRefEntry::compressed(5, 2);
        assert!(compressed.is_compressed());
        assert_eq!(compressed.object_stream(), Some(5));
        assert_eq!(compressed.stream_index(), Some(2));
    }

    #[test]
    fn test_decoder_encoder_roundtrip() {
        let entries = vec![
            (0, XRefEntry::free(0, 65535)),
            (1, XRefEntry::uncompressed(100, 0)),
            (2, XRefEntry::uncompressed(200, 0)),
            (3, XRefEntry::compressed(10, 0)),
            (4, XRefEntry::compressed(10, 1)),
        ];

        let (w0, w1, w2) = XRefStreamEncoder::optimal_widths(&entries);
        let encoder = XRefStreamEncoder::new(w0, w1, w2);
        let encoded = encoder.encode_all(&entries);

        let decoder = XRefStreamDecoder::new(&[w0, w1, w2], None).unwrap();
        let decoded = decoder.decode_all(&encoded).unwrap();

        assert_eq!(decoded.len(), entries.len());
        for (i, (obj, entry)) in decoded.iter().enumerate() {
            assert_eq!(*obj, entries[i].0);
            assert_eq!(entry.entry_type, entries[i].1.entry_type);
            assert_eq!(entry.field2, entries[i].1.field2);
            assert_eq!(entry.field3, entries[i].1.field3);
        }
    }

    #[test]
    fn test_build_index() {
        let entries = vec![
            (0, XRefEntry::free(0, 65535)),
            (1, XRefEntry::uncompressed(100, 0)),
            (2, XRefEntry::uncompressed(200, 0)),
            (5, XRefEntry::uncompressed(300, 0)),
            (6, XRefEntry::uncompressed(400, 0)),
        ];

        let index = build_index(&entries);
        assert_eq!(index, vec![0, 3, 5, 2]); // [start=0, count=3, start=5, count=2]
    }
}
