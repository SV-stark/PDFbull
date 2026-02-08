//! QPDF-compatible features for MicroPDF
//!
//! This module provides functionality inspired by and compatible with QPDF,
//! the powerful PDF transformation library.
//!
//! ## Features
//!
//! - **Pipeline System**: Flexible stream processing with chainable filters
//! - **Content Stream Tokenizer**: Lexically-aware content stream parsing
//! - **JSON Support**: PDF to JSON roundtrip conversion
//! - **Linearization**: Reading and writing linearized PDFs
//! - **PDF Repair**: Automatic repair of malformed PDFs
//! - **Object Streams**: Full object stream compression support
//! - **Foreign Object Copying**: Copy objects between PDFs with dependency tracking
//! - **Document Helpers**: Enhanced AcroForm, Outline, and EmbeddedFile handling
//!
//! ## Example
//!
//! ```rust,ignore
//! use micropdf::qpdf::{Pipeline, PlBuffer, PlFlate, FlateAction};
//! use micropdf::qpdf::{Tokenizer, Token, TokenType};
//! use micropdf::qpdf::{is_linearized, quick_validate};
//! ```

pub mod buffer;
pub mod error;
pub mod json;
pub mod linearization;
pub mod object_copy;
pub mod pipeline;
pub mod repair;
pub mod tokenizer;
pub mod xref_stream;

// ============================================================================
// Error types
// ============================================================================
pub use error::{QpdfError, Result};

// ============================================================================
// Pipeline system - chainable stream processing filters
// ============================================================================
pub use pipeline::{
    FlateAction,
    Pipeline,
    PipelineBox,
    // Encoding pipelines
    PlAscii85Decoder,
    PlAsciiHexDecoder,
    // Buffer pipelines
    PlBuffer,
    PlConcatenate,
    // Utility pipelines
    PlCount,
    PlDiscard,
    // Compression pipelines
    PlFlate,
    PlFunction,
    PlLzwDecoder,
    PlRunLengthDecoder,
    PlString,
};

// ============================================================================
// Buffer and memory management
// ============================================================================
pub use buffer::{Buffer, InputSource};

// ============================================================================
// Tokenizer - PDF content stream lexical analysis
// ============================================================================
pub use tokenizer::{Token, TokenType, Tokenizer};

// ============================================================================
// JSON support - QPDF-compatible JSON import/export
// ============================================================================
pub use json::{
    JsonDecodeLevel, JsonObject, JsonOutputConfig, JsonParameters, JsonPdf, JsonReference,
    JsonStream, JsonStreamData, JsonString,
};

// ============================================================================
// Linearization - web-optimized PDF support
// ============================================================================
pub use linearization::{
    // Data structures
    LinearizationCheckResult,
    LinearizationConfig,
    LinearizationParams,
    // Hint tables
    PageOffsetHintEntry,
    PageOffsetHintHeader,
    SharedObjectHintEntry,
    SharedObjectHintHeader,
    check_linearization,
    is_linearized,
};

// ============================================================================
// PDF Repair - fix corrupted PDFs
// ============================================================================
pub use repair::{RepairConfig, RepairIssue, RepairResult, quick_validate, repair_pdf};

// ============================================================================
// Object copying - copy objects between PDFs
// ============================================================================
pub use object_copy::{CopyContext, ObjGen, ObjectMap, extract_references, rewrite_references};

// ============================================================================
// XRef streams - cross-reference table encoding/decoding
// ============================================================================
pub use xref_stream::{XRefEntry, XRefEntryType, XRefStreamDecoder, XRefStreamEncoder};

/// QPDF module version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
