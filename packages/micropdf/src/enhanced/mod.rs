//! Enhanced PDF Manipulation Features
//!
//! This module provides features beyond the original MuPDF library,
//! inspired by pypdf and other Python PDF libraries.
//!
//! ## Features
//!
//! - **Document Creation**: Create PDFs from scratch
//! - **Page Manipulation**: Add, remove, reorder pages
//! - **Content Addition**: Text overlay, images, watermarks
//! - **Drawing**: Direct drawing with colors and opacity
//! - **Optimization**: Compression, cleanup, form flattening
//! - **Bookmarks**: Outline management
//! - **Attachments**: Embed and extract files
//! - **Metadata**: Enhanced metadata support

pub mod attachments;
pub mod bookmark_writer;
pub mod bookmarks;
pub mod content;
pub mod content_stream;
pub mod drawing;
pub mod error;
pub mod metadata;
pub mod optimization;
pub mod overlay;
pub mod page_copy;
pub mod page_merge;
pub mod page_ops;
pub mod page_resize;
pub mod pdf_reader;
pub mod pdf_utils;
pub mod writer;

// Enterprise features - foundational architecture
pub mod barcodes;
pub mod charts;
pub mod compliance;
pub mod compliance_auto_fix;
pub mod forms_advanced;
pub mod html_to_pdf;
pub mod interactive;
pub mod performance;
pub mod platypus;
pub mod print_production;
pub mod typography;

// Category 1: Digital Signatures & Security (CRITICAL)
pub mod encryption;
pub mod signatures;

// Category 2: Print Production Tools (CRITICAL)
pub mod booklet;
pub mod nup;
pub mod page_boxes;
pub mod poster;
pub mod validation;

// Category 3: Document Composition Framework (HIGH)
pub mod flowables;
pub mod table;
pub mod toc;

pub use error::{EnhancedError, Result};

/// Enhanced module version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Check if enhanced features are available
pub fn is_available() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enhanced_available() {
        assert!(is_available());
    }

    #[test]
    fn test_version() {
        // VERSION is always non-empty at compile-time (env!("CARGO_PKG_VERSION"))
        // Just verify it's a valid version string format (contains a dot)
        assert!(VERSION.contains('.'));
    }
}
