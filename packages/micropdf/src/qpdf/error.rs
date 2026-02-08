//! Error types for QPDF-compatible functionality

use std::fmt;
use std::io;

/// Result type for QPDF operations
pub type Result<T> = std::result::Result<T, QpdfError>;

/// Error types for QPDF operations
#[derive(Debug)]
pub enum QpdfError {
    /// I/O error
    Io(io::Error),
    /// Pipeline error
    Pipeline(String),
    /// Parse error
    Parse(String),
    /// Invalid PDF structure
    Structure(String),
    /// Encryption error
    Encryption(String),
    /// Linearization error
    Linearization(String),
    /// Object copy error
    ObjectCopy(String),
    /// JSON conversion error
    Json(String),
    /// Tokenizer error
    Tokenizer(String),
    /// Repair error
    Repair(String),
    /// XRef error
    XRef(String),
    /// Memory limit exceeded
    MemoryLimit(String),
    /// Invalid operation
    InvalidOperation(String),
    /// Object not found
    NotFound(String),
    /// Unsupported feature
    Unsupported(String),
}

impl fmt::Display for QpdfError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "I/O error: {}", e),
            Self::Pipeline(msg) => write!(f, "Pipeline error: {}", msg),
            Self::Parse(msg) => write!(f, "Parse error: {}", msg),
            Self::Structure(msg) => write!(f, "PDF structure error: {}", msg),
            Self::Encryption(msg) => write!(f, "Encryption error: {}", msg),
            Self::Linearization(msg) => write!(f, "Linearization error: {}", msg),
            Self::ObjectCopy(msg) => write!(f, "Object copy error: {}", msg),
            Self::Json(msg) => write!(f, "JSON error: {}", msg),
            Self::Tokenizer(msg) => write!(f, "Tokenizer error: {}", msg),
            Self::Repair(msg) => write!(f, "Repair error: {}", msg),
            Self::XRef(msg) => write!(f, "XRef error: {}", msg),
            Self::MemoryLimit(msg) => write!(f, "Memory limit exceeded: {}", msg),
            Self::InvalidOperation(msg) => write!(f, "Invalid operation: {}", msg),
            Self::NotFound(msg) => write!(f, "Not found: {}", msg),
            Self::Unsupported(msg) => write!(f, "Unsupported: {}", msg),
        }
    }
}

impl std::error::Error for QpdfError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<io::Error> for QpdfError {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<std::string::FromUtf8Error> for QpdfError {
    fn from(e: std::string::FromUtf8Error) -> Self {
        Self::Parse(format!("UTF-8 conversion error: {}", e))
    }
}

impl From<serde_json::Error> for QpdfError {
    fn from(e: serde_json::Error) -> Self {
        Self::Json(e.to_string())
    }
}
