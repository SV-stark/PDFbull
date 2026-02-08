// Suppress various warnings that are common in FFI code and test configurations
#![allow(unused_imports)]
#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unreachable_patterns)]
#![allow(unused_unsafe)]
#![allow(private_interfaces)]
#![allow(unused_attributes)]
// Clippy lints that are too pedantic for this codebase
#![allow(clippy::inline_always)]
#![allow(clippy::manual_range_contains)]
#![allow(clippy::same_item_push)]
#![allow(clippy::match_overlapping_arm)]
#![allow(clippy::type_complexity)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::excessive_precision)]
#![allow(clippy::identity_op)]
#![allow(clippy::needless_range_loop)]
#![allow(clippy::useless_vec)]
#![allow(clippy::field_reassign_with_default)]
#![allow(clippy::unnecessary_cast)]
#![allow(clippy::wildcard_in_or_patterns)]
#![allow(clippy::match_like_matches_macro)]
#![allow(clippy::manual_div_ceil)]
#![allow(clippy::bool_comparison)]
#![allow(clippy::nonminimal_bool)]
#![allow(clippy::collapsible_match)]
#![allow(clippy::inline_fn_without_body)]
#![allow(clippy::manual_range_patterns)]
#![allow(clippy::let_and_return)]
#![allow(clippy::manual_rem_euclid)]
#![allow(clippy::manual_memcpy)]
#![allow(clippy::derivable_impls)]
#![allow(clippy::manual_clamp)]
#![allow(clippy::should_implement_trait)]
#![allow(clippy::missing_safety_doc)]
#![allow(clippy::needless_return)]
#![allow(clippy::option_map_or_none)]
#![allow(clippy::map_entry)]
#![allow(clippy::option_map_unit_fn)]
#![allow(clippy::or_fun_call)]
#![allow(clippy::manual_strip)]
#![allow(clippy::char_lit_as_u8)]
#![allow(clippy::if_same_then_else)]
#![allow(clippy::manual_unwrap_or_default)]
#![allow(improper_ctypes_definitions)]
#![allow(clippy::map_unwrap_or)]
#![allow(clippy::unnecessary_lazy_evaluations)]
#![allow(clippy::manual_unwrap_or)]
#![allow(clippy::single_char_pattern)]
#![allow(unused_mut)]
#![allow(clippy::unwrap_or_default)]
#![allow(clippy::manual_pattern_char_comparison)]
#![allow(clippy::unnecessary_map_or)]

//! MicroPDF - A native Rust PDF library inspired by MuPDF and QPDF
//!
//! This library provides PDF parsing, rendering, and manipulation capabilities.
//!
//! # Modules
//!
//! - `fitz` - Core rendering and document infrastructure (MuPDF compatible)
//! - `pdf` - PDF-specific parsing and manipulation (MuPDF compatible)
//! - `ffi` - C-compatible FFI exports (MuPDF API compatible)
//! - `enhanced` - Extended features beyond MuPDF (pypdf-inspired)
//! - `qpdf` - QPDF-compatible features (pipeline, tokenizer, linearization, etc.)
//!
//! # FFI Module
//!
//! The `ffi` module provides C-compatible exports that match MuPDF's API.
//! When compiled as a staticlib or cdylib, these functions can be called
//! from C code using the same function signatures as MuPDF.
//!
//! # Enhanced Module
//!
//! The `enhanced` module provides features beyond the original MuPDF library,
//! inspired by pypdf and other Python PDF libraries. This includes document
//! creation, advanced page manipulation, watermarking, optimization, and more.
//!
//! # QPDF Module
//!
//! The `qpdf` module provides features inspired by the QPDF library:
//! - **Pipeline system**: Flexible stream processing with chainable filters
//! - **Content tokenizer**: Lexically-aware content stream parsing
//! - **JSON support**: PDF to JSON roundtrip conversion
//! - **Linearization**: Reading and writing linearized (web-optimized) PDFs
//! - **PDF repair**: Automatic repair of damaged PDFs
//! - **Object streams**: XRef stream encoding and decoding
//! - **Foreign object copying**: Copy objects between PDFs with dependency tracking

pub mod enhanced;
pub mod ffi;
pub mod fitz;
pub mod pdf;
pub mod qpdf;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
