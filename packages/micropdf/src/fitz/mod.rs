//! Fitz - Core rendering and document infrastructure
//!
//! This module provides foundational types for document handling,
//! geometry, rendering, and I/O operations.

pub mod archive;
pub mod buffer;
pub mod colorspace;
pub mod cookie;
pub mod device;
pub mod display_list;
pub mod document;
pub mod error;
pub mod font;
pub mod geometry;
pub mod hash;
pub mod image;
pub mod link;
pub mod output;
pub mod page;
pub mod path;
pub mod pixmap;
pub mod stream;
pub mod text;

#[cfg(feature = "parallel")]
pub mod parallel;

#[cfg(feature = "async")]
pub mod async_io;
