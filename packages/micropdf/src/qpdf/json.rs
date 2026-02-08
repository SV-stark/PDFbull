//! JSON support for PDF conversion
//!
//! This module provides functionality for converting between PDF and JSON formats,
//! enabling round-trip conversion and programmatic PDF manipulation.

use super::error::{QpdfError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// JSON representation of a PDF object
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum JsonObject {
    /// Null value
    Null,
    /// Boolean value
    Boolean(bool),
    /// Integer value
    Integer(i64),
    /// Real/float value
    Real(f64),
    /// String value (with encoding info)
    String(JsonString),
    /// Name value
    Name(String),
    /// Array of objects
    Array(Vec<JsonObject>),
    /// Dictionary
    Dictionary(HashMap<String, JsonObject>),
    /// Object reference
    Reference(JsonReference),
    /// Stream
    Stream(JsonStream),
}

/// JSON representation of a PDF string
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonString {
    /// The string value (may be UTF-8 or binary)
    pub value: String,
    /// Encoding hint: "utf-8", "pdfdoc", or "binary"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encoding: Option<String>,
}

/// JSON representation of a PDF object reference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonReference {
    /// Object number
    pub obj: u32,
    /// Generation number
    pub generation: u32,
}

/// JSON representation of a PDF stream
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonStream {
    /// Stream dictionary
    pub dict: HashMap<String, JsonObject>,
    /// Stream data (as specified by data_mode)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,
    /// Path to external file containing stream data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub datafile: Option<String>,
}

/// JSON representation of a complete PDF document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonPdf {
    /// QPDF JSON format version
    pub version: i32,
    /// PDF parameters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<JsonParameters>,
    /// Objects in the PDF
    pub objects: HashMap<String, JsonObject>,
    /// Trailer dictionary
    pub trailer: JsonObject,
}

/// JSON PDF parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonParameters {
    /// PDF version string (e.g., "1.7")
    #[serde(rename = "pdfVersion")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pdf_version: Option<String>,
    /// Whether the file is linearized
    #[serde(skip_serializing_if = "Option::is_none")]
    pub linearized: Option<bool>,
}

/// Mode for handling stream data in JSON output
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JsonStreamData {
    /// Include stream data as base64 inline
    Inline,
    /// Write stream data to separate files
    File,
    /// Don't include stream data
    None,
}

/// Decode level for stream data
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JsonDecodeLevel {
    /// Don't decode any filters
    None,
    /// Decode generalized filters (Flate, ASCII85, etc.)
    Generalized,
    /// Decode specialized filters
    Specialized,
    /// Decode all filters
    All,
}

/// Configuration for JSON output
#[derive(Debug, Clone)]
pub struct JsonOutputConfig {
    /// JSON format version (currently only 2 is supported)
    pub version: i32,
    /// How to handle stream data
    pub stream_data: JsonStreamData,
    /// Decode level for streams
    pub decode_level: JsonDecodeLevel,
    /// Prefix for external stream files
    pub file_prefix: Option<String>,
    /// Pretty-print the JSON output
    pub pretty: bool,
}

impl Default for JsonOutputConfig {
    fn default() -> Self {
        Self {
            version: 2,
            stream_data: JsonStreamData::Inline,
            decode_level: JsonDecodeLevel::Generalized,
            file_prefix: None,
            pretty: true,
        }
    }
}

impl JsonObject {
    /// Create a null object
    pub fn null() -> Self {
        Self::Null
    }

    /// Create a boolean object
    pub fn boolean(value: bool) -> Self {
        Self::Boolean(value)
    }

    /// Create an integer object
    pub fn integer(value: i64) -> Self {
        Self::Integer(value)
    }

    /// Create a real object
    pub fn real(value: f64) -> Self {
        Self::Real(value)
    }

    /// Create a string object
    pub fn string(value: &str) -> Self {
        Self::String(JsonString {
            value: value.to_string(),
            encoding: None,
        })
    }

    /// Create a name object
    pub fn name(value: &str) -> Self {
        Self::Name(value.to_string())
    }

    /// Create an array object
    pub fn array(items: Vec<JsonObject>) -> Self {
        Self::Array(items)
    }

    /// Create a dictionary object
    pub fn dictionary(items: HashMap<String, JsonObject>) -> Self {
        Self::Dictionary(items)
    }

    /// Create a reference object
    pub fn reference(obj: u32, generation: u32) -> Self {
        Self::Reference(JsonReference { obj, generation })
    }

    /// Check if this is a null object
    pub fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }

    /// Try to get as boolean
    pub fn as_boolean(&self) -> Option<bool> {
        match self {
            Self::Boolean(b) => Some(*b),
            _ => None,
        }
    }

    /// Try to get as integer
    pub fn as_integer(&self) -> Option<i64> {
        match self {
            Self::Integer(i) => Some(*i),
            _ => None,
        }
    }

    /// Try to get as real
    pub fn as_real(&self) -> Option<f64> {
        match self {
            Self::Real(r) => Some(*r),
            Self::Integer(i) => Some(*i as f64),
            _ => None,
        }
    }

    /// Try to get as string
    pub fn as_string(&self) -> Option<&str> {
        match self {
            Self::String(s) => Some(&s.value),
            _ => None,
        }
    }

    /// Try to get as name
    pub fn as_name(&self) -> Option<&str> {
        match self {
            Self::Name(n) => Some(n),
            _ => None,
        }
    }

    /// Try to get as array
    pub fn as_array(&self) -> Option<&[JsonObject]> {
        match self {
            Self::Array(a) => Some(a),
            _ => None,
        }
    }

    /// Try to get as dictionary
    pub fn as_dictionary(&self) -> Option<&HashMap<String, JsonObject>> {
        match self {
            Self::Dictionary(d) => Some(d),
            _ => None,
        }
    }
}

impl JsonPdf {
    /// Create a new empty JSON PDF
    pub fn new() -> Self {
        Self {
            version: 2,
            parameters: None,
            objects: HashMap::new(),
            trailer: JsonObject::Dictionary(HashMap::new()),
        }
    }

    /// Get an object by key (e.g., "obj:1 0 R")
    pub fn get_object(&self, key: &str) -> Option<&JsonObject> {
        self.objects.get(key)
    }

    /// Set an object
    pub fn set_object(&mut self, key: String, object: JsonObject) {
        self.objects.insert(key, object);
    }

    /// Make an object key from object number and generation
    pub fn make_key(obj: u32, generation: u32) -> String {
        format!("obj:{} {} R", obj, generation)
    }

    /// Parse a JSON PDF from a string
    pub fn from_json(json: &str) -> Result<Self> {
        serde_json::from_str(json).map_err(|e| QpdfError::Json(e.to_string()))
    }

    /// Convert to JSON string
    pub fn to_json(&self, pretty: bool) -> Result<String> {
        if pretty {
            serde_json::to_string_pretty(self).map_err(|e| QpdfError::Json(e.to_string()))
        } else {
            serde_json::to_string(self).map_err(|e| QpdfError::Json(e.to_string()))
        }
    }
}

impl Default for JsonPdf {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_object_types() {
        assert!(JsonObject::null().is_null());
        assert_eq!(JsonObject::boolean(true).as_boolean(), Some(true));
        assert_eq!(JsonObject::integer(42).as_integer(), Some(42));
        assert_eq!(JsonObject::real(3.14).as_real(), Some(3.14));
        assert_eq!(JsonObject::string("test").as_string(), Some("test"));
        assert_eq!(JsonObject::name("/Type").as_name(), Some("/Type"));
    }

    #[test]
    fn test_json_pdf_roundtrip() {
        let mut pdf = JsonPdf::new();
        pdf.set_object(
            JsonPdf::make_key(1, 0),
            JsonObject::dictionary({
                let mut d = HashMap::new();
                d.insert("/Type".to_string(), JsonObject::name("/Page"));
                d
            }),
        );

        let json = pdf.to_json(true).unwrap();
        let parsed = JsonPdf::from_json(&json).unwrap();

        assert_eq!(parsed.version, 2);
        assert!(parsed.objects.contains_key("obj:1 0 R"));
    }
}
