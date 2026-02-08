//! Buffer - Dynamic byte array wrapper using the `bytes` crate
//!
//! This module provides a high-performance buffer implementation backed by
//! `bytes::Bytes` and `bytes::BytesMut` for efficient zero-copy operations.

pub mod core;
pub mod reader;
pub mod writer;

pub use core::Buffer;
pub use reader::BufferReader;
pub use writer::BufferWriter;

#[cfg(feature = "parallel")]
pub use writer::parallel;

#[cfg(feature = "async")]
pub use writer::async_ops;
