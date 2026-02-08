//! PDF document implementation
//!
//! Provides complete PDF document loading, parsing, and object resolution.
//! Supports encrypted documents, XREF tables, and object streams.

use crate::fitz::error::{Error, Result};
use crate::fitz::stream::Stream;
use crate::pdf::crypt::Crypt;
use crate::pdf::filter::{decode_flate, FlateDecodeParams};
use crate::pdf::lexer::{LexBuf, Lexer, Token};
use crate::pdf::object::{Array, Dict, Name, ObjRef, Object, PdfString};
use crate::pdf::xref::{XrefEntry, XrefEntryType, XrefSubsection, XrefTable};
use std::collections::HashMap;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

/// PDF document structure
pub struct Document {
    /// Document data (owned or memory-mapped)
    data: Vec<u8>,
    /// Cross-reference table
    xref: XrefTable,
    /// Trailer dictionary
    trailer: Dict,
    /// Root catalog reference
    root: Option<ObjRef>,
    /// Info dictionary reference
    info: Option<ObjRef>,
    /// Encryption context (if encrypted)
    crypt: Option<Crypt>,
    /// Object cache for frequently accessed objects
    object_cache: HashMap<i32, Object>,
    /// Document version (e.g., "1.4")
    version: String,
}

impl Document {
    /// Open a PDF document from a file path
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let mut file = std::fs::File::open(path)?;
        let mut data = Vec::new();
        file.read_to_end(&mut data)?;
        Self::from_bytes(data)
    }

    /// Open a PDF document from a stream
    pub fn from_stream(stream: &mut Stream) -> Result<Self> {
        let buffer = stream.read_all(0)?;
        let data = buffer.as_slice().to_vec();
        Self::from_bytes(data)
    }

    /// Create a document from byte data
    pub fn from_bytes(data: Vec<u8>) -> Result<Self> {
        if data.len() < 8 {
            return Err(Error::Generic("File too small to be a PDF".into()));
        }

        // Verify PDF header
        let header = std::str::from_utf8(&data[..8.min(data.len())])
            .map_err(|_| Error::Generic("Invalid PDF header (not UTF-8)".into()))?;

        if !header.starts_with("%PDF-") {
            return Err(Error::Generic(format!("Invalid PDF header: {}", header)));
        }

        // Extract version
        let version = header[5..]
            .trim()
            .split_whitespace()
            .next()
            .unwrap_or("1.4")
            .to_string();

        let mut doc = Self {
            data,
            xref: XrefTable::new(),
            trailer: Dict::new(),
            root: None,
            info: None,
            crypt: None,
            object_cache: HashMap::new(),
            version,
        };

        // Parse the document structure
        doc.parse()?;

        Ok(doc)
    }

    /// Parse the PDF document structure
    fn parse(&mut self) -> Result<()> {
        // Find and parse xref table or stream
        self.parse_xref()?;

        // Parse trailer dictionary
        self.parse_trailer()?;

        // Setup encryption if needed
        self.setup_encryption()?;

        // Cache root and info references
        if let Some(Object::Ref(root_ref)) = self.trailer.get(&Name::new("Root")) {
            self.root = Some(*root_ref);
        }

        if let Some(Object::Ref(info_ref)) = self.trailer.get(&Name::new("Info")) {
            self.info = Some(*info_ref);
        }

        Ok(())
    }

    /// Parse the XREF table or cross-reference stream
    fn parse_xref(&mut self) -> Result<()> {
        // First, try to find startxref at end of file
        let startxref = self.find_startxref()?;

        // Check if it's an XREF stream or traditional XREF table
        let mut temp_pos = startxref as usize;
        let xref_data = &self.data[temp_pos..];

        if xref_data.starts_with(b"xref") {
            // Traditional XREF table
            self.parse_traditional_xref(startxref)?;
        } else {
            // Cross-reference stream
            self.parse_xref_stream(startxref)?;
        }

        Ok(())
    }

    /// Find the startxref position at end of file
    fn find_startxref(&self) -> Result<i64> {
        let data_len = self.data.len();

        // Search backwards from end for "startxref"
        let search_len = data_len.min(1024); // Look in last 1KB
        let search_start = data_len - search_len;

        let search_data = &self.data[search_start..];

        if let Some(pos) = search_data
            .windows(9)
            .rposition(|window| window == b"startxref")
        {
            // Found startxref, now read the offset
            let after_keyword = search_start + pos + 9;
            let remaining = &self.data[after_keyword..];

            // Skip whitespace
            let mut num_start = 0;
            while num_start < remaining.len() && (remaining[num_start] as char).is_whitespace() {
                num_start += 1;
            }

            // Parse number
            let mut num_end = num_start;
            while num_end < remaining.len() && (remaining[num_end] as char).is_ascii_digit() {
                num_end += 1;
            }

            let num_str = std::str::from_utf8(&remaining[num_start..num_end])
                .map_err(|_| Error::Generic("Invalid startxref number".into()))?;

            let offset: i64 = num_str
                .parse()
                .map_err(|_| Error::Generic("Failed to parse startxref offset".into()))?;

            return Ok(offset);
        }

        Err(Error::Generic("Could not find startxref".into()))
    }

    /// Parse traditional XREF table
    fn parse_traditional_xref(&mut self, startxref: i64) -> Result<()> {
        let mut pos = startxref as usize;

        // Skip "xref" keyword
        if !self.data[pos..].starts_with(b"xref") {
            return Err(Error::Generic("Expected 'xref' keyword".into()));
        }
        pos += 4;

        // Skip whitespace
        while pos < self.data.len() && (self.data[pos] as char).is_whitespace() {
            pos += 1;
        }

        // Parse subsections
        while pos < self.data.len() {
            // Check for trailer keyword
            if self.data[pos..].starts_with(b"trailer") {
                break;
            }

            // Parse subsection header: "start count"
            let line_end = self.find_line_end(pos);
            let line = &self.data[pos..line_end];
            let line_str = std::str::from_utf8(line)
                .map_err(|_| Error::Generic("Invalid XREF subsection header".into()))?;

            let parts: Vec<&str> = line_str.split_whitespace().collect();
            if parts.len() != 2 {
                return Err(Error::Generic(format!(
                    "Invalid XREF subsection header: {}",
                    line_str
                )));
            }

            let start: i32 = parts[0]
                .parse()
                .map_err(|_| Error::Generic("Invalid XREF start number".into()))?;
            let count: i32 = parts[1]
                .parse()
                .map_err(|_| Error::Generic("Invalid XREF count".into()))?;

            pos = line_end;

            // Parse entries
            let mut subsection = XrefSubsection::new(start, count as usize);

            for i in 0..count {
                // Each entry is 20 bytes: "nnnnnnnnnn ggggg n eol"
                // or "nnnnnnnnnn ggggg f eol"
                while pos < self.data.len() && (self.data[pos] as char).is_whitespace() {
                    pos += 1;
                }

                if pos + 20 > self.data.len() {
                    return Err(Error::Generic("XREF entry truncated".into()));
                }

                let entry_str = std::str::from_utf8(&self.data[pos..pos + 20])
                    .map_err(|_| Error::Generic("Invalid XREF entry".into()))?;

                let offset_str = &entry_str[0..10];
                let gen_str = &entry_str[11..16];
                let entry_type = entry_str
                    .chars()
                    .nth(17)
                    .ok_or_else(|| Error::Generic("Missing XREF entry type".into()))?;

                let offset: i64 = offset_str
                    .trim()
                    .parse()
                    .map_err(|_| Error::Generic("Invalid XREF offset".into()))?;
                let generation: u16 = gen_str
                    .trim()
                    .parse()
                    .map_err(|_| Error::Generic("Invalid XREF generation".into()))?;

                let entry = match entry_type {
                    'n' => XrefEntry::in_use(start + i, generation, offset),
                    'f' => XrefEntry::free(start + i, generation),
                    _ => {
                        return Err(Error::Generic(format!(
                            "Unknown XREF entry type: {}",
                            entry_type
                        )))
                    }
                };

                subsection.add(entry);
                pos += 20;
            }

            self.xref.add_subsection(subsection);
        }

        Ok(())
    }

    /// Parse cross-reference stream
    fn parse_xref_stream(&mut self, startxref: i64) -> Result<()> {
        // The xref is stored as a stream object
        // Parse the object at this location
        let (obj_num, gen_num, dict, data) = self.parse_object_at_offset(startxref)?;

        // Extract W array (field widths)
        let w = dict
            .get(&Name::new("W"))
            .and_then(|o| o.as_array())
            .ok_or_else(|| Error::Generic("Missing W array in XREF stream".into()))?;

        if w.len() < 3 {
            return Err(Error::Generic(
                "W array must have at least 3 elements".into(),
            ));
        }

        let w1 = w[0].as_int().unwrap_or(1) as usize; // Type field width
        let w2 = w[1].as_int().unwrap_or(2) as usize; // Field 2 width (offset/objstm num)
        let w3 = w[2].as_int().unwrap_or(1) as usize; // Field 3 width (gen/index)

        // Get Index array (default is [0 Size])
        let index = dict
            .get(&Name::new("Index"))
            .and_then(|o| o.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|o| o.as_int().map(|i| i as i32))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_else(|| vec![0, obj_num]); // Default to 0 to Size

        // Decode the stream data
        let decoded_data = self.decode_stream_data(&dict, &data)?;

        // Parse entries
        let entry_size = w1 + w2 + w3;
        let mut entry_pos = 0;
        let mut obj_num_idx = 0;

        while obj_num_idx < index.len() {
            let start = index[obj_num_idx];
            let count = if obj_num_idx + 1 < index.len() {
                index[obj_num_idx + 1]
            } else {
                (decoded_data.len() / entry_size) as i32 - start
            };

            let mut subsection = XrefSubsection::new(start, count as usize);

            for i in 0..count {
                if entry_pos + entry_size > decoded_data.len() {
                    break;
                }

                let entry_data = &decoded_data[entry_pos..entry_pos + entry_size];

                // Parse fields based on widths
                let entry_type = if w1 > 0 {
                    read_big_endian(&entry_data[0..w1]) as i32
                } else {
                    1 // Default to in-use
                };

                let field2 = if w2 > 0 {
                    read_big_endian(&entry_data[w1..w1 + w2]) as i64
                } else {
                    0
                };

                let field3 = if w3 > 0 {
                    read_big_endian(&entry_data[w1 + w2..entry_size]) as u16
                } else {
                    0
                };

                let entry = match entry_type {
                    0 => XrefEntry::free(start + i, field3),
                    1 => XrefEntry::in_use(start + i, field3, field2),
                    2 => XrefEntry::compressed(start + i, field2, field3),
                    _ => XrefEntry::free(start + i, 0),
                };

                subsection.add(entry);
                entry_pos += entry_size;
            }

            self.xref.add_subsection(subsection);
            obj_num_idx += 2;
        }

        // Store the trailer from the xref stream
        for (key, value) in dict {
            self.trailer.insert(key, value);
        }

        Ok(())
    }

    /// Parse trailer dictionary
    fn parse_trailer(&mut self) -> Result<()> {
        // If we already parsed it from an xref stream, we're done
        if !self.trailer.is_empty() {
            return Ok(());
        }

        // Find trailer keyword
        let trailer_pos = self.find_trailer()?;

        // Parse dictionary after "trailer"
        let mut pos = trailer_pos + 7; // Skip "trailer"

        // Skip whitespace
        while pos < self.data.len() && (self.data[pos] as char).is_whitespace() {
            pos += 1;
        }

        // Parse the dictionary
        let lexer = Lexer::new(&self.data[pos..]);
        let (obj, _consumed) = self.parse_object_recursive(lexer, 0)?;

        if let Object::Dict(dict) = obj {
            self.trailer = dict;
        } else {
            return Err(Error::Generic("Trailer is not a dictionary".into()));
        }

        Ok(())
    }

    /// Find trailer position
    fn find_trailer(&self) -> Result<usize> {
        // Search for "trailer" keyword after xref
        let startxref = self.find_startxref()? as usize;

        // Look after xref section
        let search_start = startxref;
        let search_data = &self.data[search_start..];

        if let Some(pos) = search_data.windows(7).position(|w| w == b"trailer") {
            return Ok(search_start + pos);
        }

        Err(Error::Generic("Could not find trailer".into()))
    }

    /// Setup encryption context if document is encrypted
    fn setup_encryption(&mut self) -> Result<()> {
        if let Some(Object::Ref(encrypt_ref)) = self.trailer.get(&Name::new("Encrypt")) {
            // Document is encrypted
            let encrypt_obj = self.resolve_object_ref(*encrypt_ref)?;

            if let Object::Dict(encrypt_dict) = encrypt_obj {
                // Get Document ID from trailer
                let id_array = self
                    .trailer
                    .get(&Name::new("ID"))
                    .and_then(|o| o.as_array())
                    .ok_or(Error::Generic("Missing ID in trailer".into()))?;
                let id_0 = id_array
                    .get(0)
                    .and_then(|o| o.as_string())
                    .ok_or(Error::Generic("Invalid ID[0]".into()))?
                    .as_bytes()
                    .to_vec();

                self.crypt = Some(Crypt::from_dict(&encrypt_dict, id_0)?);
            }
        }

        Ok(())
    }

    /// Get the document catalog (root dictionary)
    pub fn catalog(&self) -> Result<Dict> {
        let root_ref = self
            .root
            .ok_or_else(|| Error::Generic("Document has no root catalog".into()))?;

        match self.resolve_object_ref(root_ref)? {
            Object::Dict(dict) => Ok(dict),
            _ => Err(Error::Generic("Root is not a dictionary".into())),
        }
    }

    /// Get the info dictionary (document metadata)
    pub fn info(&self) -> Result<Option<Dict>> {
        let info_ref = match self.info {
            Some(r) => r,
            None => return Ok(None),
        };

        match self.resolve_object_ref(info_ref)? {
            Object::Dict(dict) => Ok(Some(dict)),
            _ => Ok(None),
        }
    }

    /// Get document title
    pub fn title(&self) -> Option<String> {
        if let Ok(Some(info)) = self.info() {
            if let Some(Object::String(s)) = info.get(&Name::new("Title")) {
                return s.as_str().map(|s| s.to_string());
            }
        }
        None
    }

    /// Get document author
    pub fn author(&self) -> Option<String> {
        if let Ok(Some(info)) = self.info() {
            if let Some(Object::String(s)) = info.get(&Name::new("Author")) {
                return s.as_str().map(|s| s.to_string());
            }
        }
        None
    }

    /// Get number of pages
    pub fn page_count(&self) -> Result<i32> {
        let catalog = self.catalog()?;

        let pages_ref = catalog
            .get(&Name::new("Pages"))
            .ok_or_else(|| Error::Generic("Catalog missing Pages reference".into()))?;

        if let Object::Ref(pages_ref) = pages_ref {
            let pages_obj = self.resolve_object_ref(*pages_ref)?;
            if let Object::Dict(pages_dict) = pages_obj {
                if let Some(Object::Int(count)) = pages_dict.get(&Name::new("Count")) {
                    return Ok(*count as i32);
                }
            }
        }

        Err(Error::Generic("Could not determine page count".into()))
    }

    /// Get a page by number (0-indexed)
    pub fn get_page(&self, page_num: i32) -> Result<Page> {
        let catalog = self.catalog()?;

        let pages_ref = catalog
            .get(&Name::new("Pages"))
            .ok_or_else(|| Error::Generic("Catalog missing Pages reference".into()))?;

        let pages_obj = if let Object::Ref(r) = pages_ref {
            self.resolve_object_ref(*r)?
        } else {
            pages_ref.clone()
        };

        if let Object::Dict(pages_dict) = pages_obj {
            // Get Kids array
            let kids = pages_dict
                .get(&Name::new("Kids"))
                .and_then(|o| o.as_array())
                .ok_or_else(|| Error::Generic("Pages missing Kids array".into()))?;

            if page_num < 0 || page_num >= kids.len() as i32 {
                return Err(Error::Generic(format!(
                    "Page {} out of range (0-{})",
                    page_num,
                    kids.len() - 1
                )));
            }

            let page_ref = &kids[page_num as usize];
            let page_obj = if let Object::Ref(r) = page_ref {
                self.resolve_object_ref(*r)?
            } else {
                page_ref.clone()
            };

            if let Object::Dict(page_dict) = page_obj {
                Ok(Page::new(page_dict, page_num))
            } else {
                Err(Error::Generic("Page is not a dictionary".into()))
            }
        } else {
            Err(Error::Generic("Pages is not a dictionary".into()))
        }
    }

    /// Resolve an object reference to its actual object
    pub fn resolve_object_ref(&self, obj_ref: ObjRef) -> Result<Object> {
        // Check cache first
        if let Some(cached) = self.object_cache.get(&obj_ref.num) {
            return Ok(cached.clone());
        }

        let entry = self
            .xref
            .get(obj_ref.num)
            .ok_or_else(|| Error::Generic(format!("Object {} not found in xref", obj_ref.num)))?;

        let obj = match entry.entry_type {
            XrefEntryType::InUse => {
                // Parse object at offset
                let (num, generation, dict, data) = self.parse_object_at_offset(entry.offset)?;

                if num != obj_ref.num || generation != obj_ref.generation {
                    return Err(Error::Generic(format!(
                        "Object mismatch: expected {} {}, found {} {}",
                        obj_ref.num, obj_ref.generation, num, generation
                    )));
                }

                // If it has stream data, create a stream object
                if !data.is_empty() {
                    Object::Stream { dict, data }
                } else {
                    // Reconstruct the object from the dictionary or parse the actual object
                    // For now, return the first value from dict or Null
                    dict.into_iter()
                        .next()
                        .map(|(_, v)| v)
                        .unwrap_or(Object::Null)
                }
            }
            XrefEntryType::ObjStm => {
                // Object is in an object stream
                self.resolve_compressed_object(obj_ref.num, entry.offset as i32, entry.stm_index)?
            }
            XrefEntryType::Free => {
                return Err(Error::Generic(format!("Object {} is free", obj_ref.num)));
            }
        };

        // Decrypt if needed
        let obj = if let Some(ref crypt) = self.crypt {
            crypt.decrypt_object(&obj, obj_ref.num, obj_ref.generation)?
        } else {
            obj
        };

        Ok(obj)
    }

    /// Resolve an object from an object stream
    fn resolve_compressed_object(
        &self,
        obj_num: i32,
        stm_num: i32,
        stm_index: u16,
    ) -> Result<Object> {
        // Get the object stream
        let stm_entry = self
            .xref
            .get(stm_num)
            .ok_or_else(|| Error::Generic(format!("Object stream {} not found", stm_num)))?;

        let (_, _, stm_dict, stm_data) = self.parse_object_at_offset(stm_entry.offset)?;

        // Decode the stream
        let decoded = self.decode_stream_data(&stm_dict, &stm_data)?;

        // Parse the N and First values from stream dictionary
        let n = stm_dict
            .get(&Name::new("N"))
            .and_then(|o| o.as_int())
            .ok_or_else(|| Error::Generic("Object stream missing N".into()))?
            as usize;

        let first = stm_dict
            .get(&Name::new("First"))
            .and_then(|o| o.as_int())
            .ok_or_else(|| Error::Generic("Object stream missing First".into()))?
            as usize;

        // Parse the header: pairs of (obj_num, offset)
        let header_data = &decoded[..first];
        let header_str = std::str::from_utf8(header_data)
            .map_err(|_| Error::Generic("Invalid object stream header".into()))?;

        let numbers: Vec<i32> = header_str
            .split_whitespace()
            .filter_map(|s| s.parse().ok())
            .collect();

        if numbers.len() < n * 2 {
            return Err(Error::Generic("Object stream header too short".into()));
        }

        // Find our object
        let idx = stm_index as usize;
        if idx >= n {
            return Err(Error::Generic(format!(
                "Object index {} out of range in stream",
                idx
            )));
        }

        let target_obj_num = numbers[idx * 2];
        let offset = numbers[idx * 2 + 1] as usize;
        let end_offset = if idx + 1 < n {
            numbers[(idx + 1) * 2 + 1] as usize
        } else {
            decoded.len()
        };

        if target_obj_num != obj_num {
            return Err(Error::Generic(format!(
                "Object number mismatch in stream: expected {}, found {}",
                obj_num, target_obj_num
            )));
        }

        // Parse the object at this offset
        let obj_data = &decoded[first + offset..first + end_offset];
        let lexer = Lexer::new(obj_data);
        let (obj, _) = self.parse_object_recursive(lexer, 0)?;

        Ok(obj)
    }

    /// Parse an object at a specific file offset
    fn parse_object_at_offset(&self, offset: i64) -> Result<(i32, i32, Dict, Vec<u8>)> {
        let pos = offset as usize;
        let lexer = Lexer::new(&self.data[pos..]);
        let mut buf = LexBuf::new();
        let mut lexer = lexer;

        // Parse object header: "num gen obj"
        let token = lexer.lex(&mut buf)?;
        if token != Token::Int {
            return Err(Error::Generic("Expected object number".into()));
        }
        let obj_num = buf.as_int() as i32;

        let token = lexer.lex(&mut buf)?;
        if token != Token::Int {
            return Err(Error::Generic("Expected generation number".into()));
        }
        let gen_num = buf.as_int() as i32;

        let token = lexer.lex(&mut buf)?;
        if token != Token::Obj {
            return Err(Error::Generic("Expected 'obj' keyword".into()));
        }

        // Parse the object value
        let (obj, consumed) = self.parse_object_recursive(lexer, 0)?;

        // Check for stream
        let mut stream_data = Vec::new();
        let dict = match obj {
            Object::Dict(d) => {
                // Check if there's a stream following
                let after_obj = pos + consumed;
                if self.data[after_obj..].starts_with(b"stream") {
                    stream_data = self.parse_stream_data(&d, after_obj)?;
                }
                d
            }
            _ => {
                let mut d = Dict::new();
                d.insert(Name::new("__value"), obj);
                d
            }
        };

        Ok((obj_num, gen_num, dict, stream_data))
    }

    /// Parse stream data
    fn parse_stream_data(&self, dict: &Dict, start_pos: usize) -> Result<Vec<u8>> {
        // Find "stream" keyword
        let mut pos = start_pos;
        while pos < self.data.len() - 6 {
            if self.data[pos..].starts_with(b"stream") {
                pos += 6;
                break;
            }
            pos += 1;
        }

        // Skip newline (CRLF or LF)
        if pos < self.data.len() && self.data[pos] == b'\r' {
            pos += 1;
        }
        if pos < self.data.len() && self.data[pos] == b'\n' {
            pos += 1;
        }

        // Get length
        let length = dict
            .get(&Name::new("Length"))
            .and_then(|o| o.as_int())
            .ok_or_else(|| Error::Generic("Stream missing Length".into()))?
            as usize;

        // Extract stream data
        let end_pos = pos + length;
        if end_pos > self.data.len() {
            return Err(Error::Generic(
                "Stream data extends past end of file".into(),
            ));
        }

        let data = self.data[pos..end_pos].to_vec();

        Ok(data)
    }

    /// Decode stream data based on filters
    fn decode_stream_data(&self, dict: &Dict, data: &[u8]) -> Result<Vec<u8>> {
        // Check for filter
        let filter = dict.get(&Name::new("Filter"));
        let decode_parms = dict.get(&Name::new("DecodeParms"));

        if filter.is_none() {
            return Ok(data.to_vec());
        }

        // Handle filter
        let mut result = data.to_vec();

        // For now, just handle FlateDecode
        if let Some(Object::Name(name)) = filter {
            if name.as_str() == "FlateDecode" {
                let mut params = None;
                let mut flate_params;

                if let Some(Object::Dict(parms)) = decode_parms {
                    let predictor = parms
                        .get(&Name::new("Predictor"))
                        .and_then(|o| o.as_int())
                        .unwrap_or(1) as i32;

                    let columns = parms
                        .get(&Name::new("Columns"))
                        .and_then(|o| o.as_int())
                        .unwrap_or(1) as i32;

                    let colors = parms
                        .get(&Name::new("Colors"))
                        .and_then(|o| o.as_int())
                        .unwrap_or(1) as i32;

                    let bits_per_component = parms
                        .get(&Name::new("BitsPerComponent"))
                        .and_then(|o| o.as_int())
                        .unwrap_or(8) as i32;

                    flate_params = FlateDecodeParams {
                        predictor,
                        columns,
                        colors,
                        bits_per_component,
                    };
                    params = Some(&flate_params);
                }

                result = decode_flate(data, params)?;
            }
        }

        Ok(result)
    }

    /// Recursively parse a PDF object
    fn parse_object_recursive(&self, mut lexer: Lexer, depth: usize) -> Result<(Object, usize)> {
        if depth > 100 {
            return Err(Error::Generic("Object nesting too deep".into()));
        }

        let mut buf = LexBuf::new();
        let start_pos = lexer.pos;

        let token = lexer.lex(&mut buf)?;

        let obj = match token {
            Token::Null => Object::Null,
            Token::True => Object::Bool(true),
            Token::False => Object::Bool(false),
            Token::Int => Object::Int(buf.as_int()),
            Token::Real => Object::Real(buf.as_float()),
            Token::String => Object::String(PdfString::new(buf.as_str().as_bytes().to_vec())),
            Token::Name => Object::Name(Name::new(buf.as_str())),
            Token::OpenArray => {
                // Parse array
                let mut arr = Array::new();
                loop {
                    let mut inner_buf = LexBuf::new();
                    let mut inner_lexer = lexer.clone();
                    let inner_token = inner_lexer.lex(&mut inner_buf)?;

                    if inner_token == Token::CloseArray {
                        lexer = inner_lexer;
                        break;
                    }

                    let (element, consumed) = self.parse_object_recursive(lexer, depth + 1)?;
                    arr.push(element);
                    lexer = Lexer::new(&self.data[lexer.pos + consumed..]);
                }
                Object::Array(arr)
            }
            Token::OpenDict => {
                // Parse dictionary
                let mut dict = Dict::new();
                loop {
                    let mut inner_buf = LexBuf::new();
                    let mut inner_lexer = lexer.clone();
                    let inner_token = inner_lexer.lex(&mut inner_buf)?;

                    if inner_token == Token::CloseDict {
                        lexer = inner_lexer;
                        break;
                    }

                    if inner_token != Token::Name {
                        return Err(Error::Generic("Dictionary key must be a name".into()));
                    }
                    let key = Name::new(inner_buf.as_str());
                    lexer = inner_lexer;

                    let (value, consumed) = self.parse_object_recursive(lexer, depth + 1)?;
                    dict.insert(key, value);
                    lexer = Lexer::new(&self.data[lexer.pos + consumed..]);
                }
                Object::Dict(dict)
            }
            Token::R => {
                // This is a reference - we need to look back for num and gen
                // For now, return Null (this needs proper backtracking)
                Object::Null
            }
            _ => Object::Null,
        };

        let consumed = lexer.pos - start_pos;
        Ok((obj, consumed))
    }

    /// Find line end
    fn find_line_end(&self, start: usize) -> usize {
        let mut pos = start;
        while pos < self.data.len() && self.data[pos] != b'\n' && self.data[pos] != b'\r' {
            pos += 1;
        }
        pos
    }

    /// Get document version
    pub fn version(&self) -> &str {
        &self.version
    }

    /// Check if document is encrypted
    pub fn is_encrypted(&self) -> bool {
        self.crypt.is_some()
    }

    /// Get xref table (for debugging)
    pub fn xref_table(&self) -> &XrefTable {
        &self.xref
    }
}

/// PDF Page structure
pub struct Page {
    /// Page dictionary
    pub dict: Dict,
    /// Page number (0-indexed)
    pub page_num: i32,
}

impl Page {
    /// Create a new page from its dictionary
    pub fn new(dict: Dict, page_num: i32) -> Self {
        Self { dict, page_num }
    }

    /// Get the page's MediaBox (physical page size)
    pub fn media_box(&self) -> crate::fitz::geometry::Rect {
        // First try the page's own MediaBox
        if let Some(box_arr) = self
            .dict
            .get(&Name::new("MediaBox"))
            .and_then(|o| o.as_array())
        {
            return self.array_to_rect(box_arr);
        }

        // TODO: Try parent pages
        // Default to Letter size (612x792 points)
        crate::fitz::geometry::Rect::new(0.0, 0.0, 612.0, 792.0)
    }

    /// Get the page's CropBox (visible region)
    pub fn crop_box(&self) -> crate::fitz::geometry::Rect {
        if let Some(box_arr) = self
            .dict
            .get(&Name::new("CropBox"))
            .and_then(|o| o.as_array())
        {
            return self.array_to_rect(box_arr);
        }

        self.media_box()
    }

    /// Get the page's rotation (0, 90, 180, 270)
    pub fn rotation(&self) -> i32 {
        self.dict
            .get(&Name::new("Rotate"))
            .and_then(|o| o.as_int())
            .map(|r| r as i32)
            .unwrap_or(0)
    }

    /// Get content stream references
    pub fn contents(&self) -> Vec<ObjRef> {
        let mut refs = Vec::new();

        match self.dict.get(&Name::new("Contents")) {
            Some(Object::Ref(r)) => refs.push(*r),
            Some(Object::Array(arr)) => {
                for item in arr {
                    if let Object::Ref(r) = item {
                        refs.push(*r);
                    }
                }
            }
            _ => {}
        }

        refs
    }

    /// Get resources dictionary
    pub fn resources(&self) -> Dict {
        self.dict
            .get(&Name::new("Resources"))
            .and_then(|o| o.as_dict().cloned())
            .unwrap_or_default()
    }

    /// Convert an array to a rectangle
    fn array_to_rect(&self, arr: &Array) -> crate::fitz::geometry::Rect {
        if arr.len() >= 4 {
            let x0 = arr[0].as_real().unwrap_or(0.0) as f32;
            let y0 = arr[1].as_real().unwrap_or(0.0) as f32;
            let x1 = arr[2].as_real().unwrap_or(0.0) as f32;
            let y1 = arr[3].as_real().unwrap_or(0.0) as f32;
            crate::fitz::geometry::Rect::new(x0, y0, x1, y1)
        } else {
            crate::fitz::geometry::Rect::EMPTY
        }
    }
}

/// Read a big-endian integer from bytes
fn read_big_endian(bytes: &[u8]) -> u64 {
    let mut result = 0u64;
    for &b in bytes {
        result = (result << 8) | (b as u64);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    // Minimal valid PDF for testing
    fn minimal_pdf_bytes() -> Vec<u8> {
        b"%PDF-1.4\n1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R >>\nendobj\n4 0 obj\n<< /Length 0 >>\nstream\nendstream\nendobj\nxref\n0 5\n0000000000 65535 f \n0000000009 00000 n \n0000000058 00000 n \n0000000115 00000 n \n0000000214 00000 n \ntrailer\n<< /Size 5 /Root 1 0 R >>\nstartxref\n264\n%%EOF\n".to_vec()
    }

    #[test]
    fn test_document_open() {
        let data = minimal_pdf_bytes();
        let doc = Document::from_bytes(data);
        assert!(doc.is_ok());

        let doc = doc.unwrap();
        assert_eq!(doc.version(), "1.4");
    }

    #[test]
    fn test_page_count() {
        let data = minimal_pdf_bytes();
        let doc = Document::from_bytes(data).unwrap();

        let count = doc.page_count();
        assert!(count.is_ok());
        assert_eq!(count.unwrap(), 1);
    }

    #[test]
    fn test_get_page() {
        let data = minimal_pdf_bytes();
        let doc = Document::from_bytes(data).unwrap();

        let page = doc.get_page(0);
        assert!(page.is_ok());

        let page = page.unwrap();
        let media_box = page.media_box();
        assert_eq!(media_box.x0, 0.0);
        assert_eq!(media_box.x1, 612.0);
        assert_eq!(media_box.y0, 0.0);
        assert_eq!(media_box.y1, 792.0);
    }

    #[test]
    fn test_invalid_pdf() {
        let data = b"Not a PDF file".to_vec();
        let result = Document::from_bytes(data);
        assert!(result.is_err());
    }
}
