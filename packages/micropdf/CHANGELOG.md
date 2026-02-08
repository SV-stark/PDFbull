# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Comprehensive test suite with 109 new tests across core modules
  - **pdf/filter/predictor.rs**: 18 tests for PNG/TIFF predictor functions
  - **ffi/pdf_object/dict.rs**: 21 tests for dictionary operations
  - **fitz/buffer/reader.rs**: 20 new tests for buffer reading operations
  - **fitz/buffer/writer.rs**: 9 new tests for buffer writing operations
  - **pdf/filter/chain.rs**: 12 new tests for filter chaining
  - **pdf/filter/dct.rs**: 6 tests for JPEG compression
  - **ffi/pdf_object/string.rs**: 8 tests for string extraction
  - **fitz/parallel.rs**: 10 new tests for parallel operations

### Changed
- **Refactored large modules into smaller, focused submodules**:
  - `src/ffi/pdf_object.rs` (2077 lines) → 9 focused modules
  - `src/pdf/filter.rs` (1490 lines) → 12 focused modules
  - `src/fitz/buffer.rs` (1430 lines) → 3 focused modules (core, reader, writer)
- Improved test coverage from 72.28% to **81.54%** (base library)
- Test coverage with all features enabled: **82.09%** (2,360/2,875 lines)
- Tests now properly co-located with their implementations

### Fixed
- Fixed flate compression test reliability by using longer repetitive data for better compression ratios
- Fixed Pixmap creation in parallel tests to include alpha channel requirement

## [0.1.0] - 2025-01-XX

### Added

#### Core Library (`fitz` module)
- **Error handling** with `PdfError` type using `thiserror`
- **Geometry primitives**: `Point`, `Rect`, `IRect`, `Matrix`, `Quad`
- **Buffer** for memory management with MD5 hashing support
- **Stream** abstraction for buffered I/O
- **Colorspace** support (Gray, RGB, CMYK, indexed)
- **Pixmap** for pixel buffer manipulation
- **Document** trait for document abstraction
- **Page** abstraction for page handling

#### PDF Module
- **PDF Object Model**: null, bool, int, real, string, name, array, dict, indirect
- **Compression filters**:
  - FlateDecode (zlib/deflate)
  - LZWDecode
  - ASCII85Decode
  - ASCIIHexDecode
  - RunLengthDecode

#### FFI (C API Compatibility)
- **100% MuPDF API compatible** C headers in `include/mupdf/`
- Handle-based safe resource management
- FFI exports for:
  - Geometry functions (`fz_point`, `fz_rect`, `fz_matrix`, etc.)
  - Context management (`fz_new_context`, `fz_drop_context`)
  - Buffer operations (`fz_new_buffer`, `fz_buffer_len`, etc.)
  - Stream operations (`fz_open_memory`, `fz_read_byte`, etc.)
  - Colorspace functions (`fz_new_colorspace`, `fz_colorspace_n`, etc.)
  - Pixmap functions (`fz_new_pixmap`, `fz_clear_pixmap`, etc.)
  - Document functions (`fz_open_document`, `fz_count_pages`, etc.)
  - PDF object functions (`pdf_new_int`, `pdf_new_dict`, etc.)

#### Optional Features
- `parallel` - Rayon-based parallel processing
- `async` - Tokio-based async I/O
- `jpeg2000` - JPEG 2000 image support

#### Build Targets
- Static library (`libmicropdf.a` / `micropdf.lib`)
- Dynamic library (`libmicropdf.so` / `micropdf.dll`)
- Rust library (rlib)

#### Packages
- Debian package support via `cargo-deb`
- RPM package support via `cargo-generate-rpm`

### Notes
- Designed as a drop-in replacement for MuPDF
- Pure Rust implementation with no C dependencies
- MIT/Apache 2.0 dual license (more permissive than MuPDF's AGPL)

[Unreleased]: https://bitbucket.org/lexmata/micropdf/compare/v0.1.0...HEAD
[0.1.0]: https://bitbucket.org/lexmata/micropdf/releases/tag/v0.1.0

