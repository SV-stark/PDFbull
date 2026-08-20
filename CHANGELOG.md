# Changelog

All notable changes to the PDFbull project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.13.5] - 2026-08-20

### Fixed
- **OCG Layer Toggle Live Rendering**: Implemented safe runtime visibility override application to `zpdf::OcConfig` via `OcConfigInternal::apply_overrides`, enabling interactive Layer toggle state changes in the UI to immediately take visual effect during rendering.
- **Robust Hex & CSS Color Parsing**: Added `try_hex_to_rgb` and `hex_to_rgb_or` supporting 3-digit (`#FFF`), 6-digit (`#RRGGBB`), 8-digit (`#RRGGBBAA`), bare hex, and named CSS colors, returning `None` / configurable fallback on parse failure instead of silently defaulting to black `(0,0,0)`.
- **Pixel-Perfect Sidebar Thumbnails**: Fixed thumbnail zoom render width from 120px to 160px (`thumb_zoom`), matching the fixed 160px sidebar container and eliminating blurry upscaling artifacts.
- **Strict Clippy Compliance & Code Quality**: Removed all 44 blanket `#![allow(...)]` attributes from `src/lib.rs`, modernized `[lints.clippy]` in `Cargo.toml`, and resolved all compiler and clippy warnings across library, test, and binary targets under `-D warnings`.
- **Security Hardening with `zeroize`**: Replaced hand-rolled `write_bytes` with the `zeroize` crate in `src/models.rs`, guaranteeing compiler dead-store elimination immunity for sensitive password data in memory.
- **Asynchronous Tracing with `tracing-appender`**: Replaced bespoke blocking `DualWriter` in `src/main.rs` with `tracing_appender::rolling::never` and non-blocking multi-writer log streaming.
- **Typed Windows FFI Bindings**: Replaced raw `unsafe extern "system"` FFI block in `src/platform/windows.rs` with official typed bindings from `windows::Win32::UI::WindowsAndMessaging` and `Win32_System_Threading`.
- **Dependency Footprint Pruning**: Removed unused `image 0.25`, direct redundant `notify 8`, and unused dev-dependency `sha2 0.11`, significantly reducing build dependencies.

## [0.13.0] - 2026-08-14

### Added
- **Interactive Command Palette (`Ctrl + K` / `Ctrl + Shift + P`)**: Floating quick-action modal with fuzzy matching powered by `nucleo-matcher` for instant navigation, tools, document outlines, theme switching, and format exports.
- **Dependency & License Auditing (`cargo-deny`)**: Added `deny.toml` configuration and integrated automated security vulnerability / license compliance scanning in GitHub Actions CI.
- **Fuzzy Search Integration (`nucleo-matcher 0.3`)**: Added allocation-free fuzzy string filtering for bookmarks, search navigation, and action palettes.

### Changed & Improved
- **Modernized Atomic File Writes**: Replaced unmaintained `atomicwrites` crate with `tempfile::NamedTempFile` for robust, cross-platform atomic writes of application settings and session data.
- **Tokio Dependency Footprint Optimization**: Pruned `tokio` features from `"full"` to exact required modules (`rt-multi-thread`, `sync`, `time`, `process`, `fs`, `io-util`, `macros`), accelerating incremental builds.
- **Persistent Image Caching in `PdfEngine`**: Made per-document `ImageCache` persistent across page renders to eliminate redundant decompression of embedded PDF image streams during scrolling.
- **Font Asset Integrity**: Replaced corrupted HTML placeholder files with valid TrueType font binaries in `src/assets/fonts/` and purged obsolete font files.

## [0.12.2] - 2026-08-08

### Added
- **`zpdf` & `zpdf-writer` v0.12.0 Upgrade**: Upgraded core rendering and writer dependencies to `zpdf 0.12.0` workspace chain.
- **PDF/UA Accessibility Tagging (`tag_pdf`)**: Exposed `DocumentStore::tag_pdf` API to inject structural `/StructTreeRoot` and `/ParentTree` tags into untagged PDFs.
- **Appearance Stream Baking (`/AP` `/N`)**: Baked appearance streams for annotated elements to ensure visual fidelity across external PDF viewers (Adobe Acrobat, Preview, Chrome).
- **Dehyphenation & Logical RTL Ordering**: Integrated automatic dehyphenation (`coopera-\ntion` ➔ `cooperation`) and visual-to-logical Hebrew/Arabic RTL run reversal in text extraction.
- **Memory-Mapped ONNX Model Loading (`rten` `mmap`)**: Enabled zero-copy `mmap` model loading for OCR neural network inference.

## [0.11.5] - 2026-08-08

### Fixed
- **Real PDF Stream Compression**: Enabled `compress_uncompressed = true` and 1600px image downsampling in `DocumentStore::optimize_pdf()`, enabling 30%–70% PDF file size reductions across text, vector, and image PDF files.
- **UI Thread Disk Stalls**: Offloaded `save_settings`, `save_recent_files`, and `save_session` disk write operations to background threads, eliminating UI frame drops during state updates.

### Added
- **Accessibility & Contrast Compliance**: Refactored theme color tokens (`COLOR_TEXT_DIM`, `COLOR_TEXT_SECONDARY`) for WCAG 2.1 AA contrast compliance and added visual focus ring indicators for button states.
- **Welcome Screen Hotkey Hints**: Integrated dynamic package versioning (`env!("CARGO_PKG_VERSION")`) and keyboard shortcut hints (`Ctrl + O`) across Welcome screen cards.

## [0.11.0] - 2026-07-29

### Added
- **Dynamic Header & Page Numbering**: Injected top header text and formatted page numbers (`"Page {page} of {pages}"`) directly into PDF graphics streams via `zpdf-writer`.
- **Granular Security & Permission Control**: Added security rules configuration dialog for controlling printing, text copying, and document editing permissions.
- **Open Containing Folder**: Added 📁 **Open Folder** button in tools ribbon using `open::that(...)` to reveal the active PDF's location in Windows File Explorer.
- **Document-Wide Multi-Page OCR**: Integrated `ocr_document_parallel` method to run full-document text recognition across all pages.
- **NSIS Setup Installer GUI Branding**: Added custom dark-themed installer sidebar graphics (`welcome.bmp`), top header banner (`header.bmp`), custom icon (`PDFbull.ico`), and automatic `.rten` model packaging in `PDFbull-Setup.exe`.
- **License Documentation Update**: Refreshed `THIRD-PARTY-LICENSES.md` to accurately document `iced`, `zpdf`, `ocrs`, and `rten` licenses.

## [0.10.5] - 2026-07-28

### Fixed
- **Pre-OCRed & Rotated PDF Search Hit Alignment**: Fixed search result hit bounding box math by passing `.with_page_rotation(page.rotate)` into `ContentInterpreter` and calculating normalized top-down Y offsets (`eff_box.y1 - span.y - span.size`).

### Added
- **GUI Convert Ribbon Tab**: Added `🔄 Convert` ribbon tab in top toolbar with 1-click action buttons for exporting documents to Markdown (`.md`), HTML5 (`.html`), and Plain Text (`.txt`).
- **OCR Unit Test Suite**: Expanded `tests/ocr_test.rs` to 18 automated tests covering OCR data models, Devanagari text processing, Serde JSON serialization, and export command payloads.

## [0.10.4] - 2026-07-28

### Added
- **Multi-Script Devanagari & Latin OCR Engine (`OcrScript`)**: Integrated `OcrScript` enum enabling full runtime script selection for Devanagari (Hindi, Marathi, Sanskrit, Nepali) via `devanagari_PP-OCRv4_rec.rten` and Latin/English script via `text-recognition.rten`.
- **Command & Messaging Pipeline Updates**: Updated `PdfCommand::OcrPage(doc_id, page_num, script, tx)` and `Message::SelectOcrScript(script)` handlers with unit tests (`tests/ocr_test.rs`).

## [0.10.3] - 2026-07-28

### Added
- **Pure-Rust OCR Capability (`ocrs` + `rten`)**: Built-in text recognition and bounding box extraction for scanned / image-only PDF pages (`PdfCommand::OcrPage`) with zero C++ DLL dependencies. Out-of-the-box support for **Devanagari** (Hindi, Marathi, Sanskrit) and **Latin** (English, European) scripts.
- **OCR Toolbar & Service Integration**: Added 🔍 **OCR Page** tool action button in the Tools ribbon with background engine processing and automated test suite (`tests/ocr_test.rs`).

## [0.10.2] - 2026-07-28

### Added
- **New Blank PDF Document Creation (`DocumentBuilder`)**: Programmatically generate new blank A4 PDF documents with custom initial layout (`PdfCommand::CreateBlankDocument`).
- **Digital Certificate Signing (`SigningKey`)**: Apply cryptographic PKCS#8 / PKCS#12 digital signatures to PDFs via `zpdf_writer` with interactive cert picker modal (`PdfCommand::SignDocumentWithCert`).
- **Rubber Stamp Annotations (`StampItem`)**: Overlay styled rubber stamps (`APPROVED`, `CONFIDENTIAL`, `DRAFT`, `REJECTED`, `FINAL`) directly on PDF pages (`PdfCommand::ApplyStamp`).
- **Geospatial GIS Metadata Panel (`Measure` / `/GCS`)**: Inspect `/Measure` & `/GCS` dictionaries on GeoPDF files, displaying coordinate systems, EPSG codes, WKT projections, and distance units.
- **CMYK & Prepress Color Inspector (`output_intent_cmyk_profile`)**: Inspect embedded ICC color profiles and document `/OutputIntents`, plus live CMYK ↔ RGB converter panel (`cmyk_to_rgb_naive` & `rgb_to_cmyk_naive`) with maximum GCR.

## [0.10.1] - 2026-07-28

### Performance
- **Pre-decoded 0ms Window Icon**: Replaced runtime ICO file parsing with pre-decoded 32x32 raw RGBA pixel buffer (`src/assets/icon_32x32.rgba`), saving ~15–25ms on main thread cold launch.

## [0.10.0] - 2026-07-27

### Added
- **PDF Encryption on Save**: Encrypt PDFs with AES-256 (V5/R6) or RC4-128 (V2/R3) security handlers and custom user/owner passwords (`PdfCommand::EncryptPdf`).
- **PDF/A Conformance Validation**: Built-in validation suite for PDF/A-1b and PDF/A-2b compliance standards (`PdfCommand::ValidatePdfA`).
- **Signature Trust Chain Verification**: X.509 certificate chain validation for PDF signatures against custom PEM/DER root trust anchors (`PdfCommand::VerifySignatureTrust`).
- **Linearization / Fast Web View**: Reorganizes PDF object structures and xref tables for ISO 32000-1 Annex F web streaming (`PdfCommand::LinearizePdf`).
- **Document Export & Conversion**: Multi-format export engine (`convert_pdf`) supporting Markdown, semantic HTML5, and raw TXT conversions in TextOnly or Rich modes (`PdfCommand::ConvertPdf`).
- **Image Downsampling in Optimization**: Integrated `max_image_dimension(2400)` option in structural PDF optimization pass (`optimize_pdf`) for shrinking large PDF files.

### Fixed
- **Release CI Workflow**: Corrected workflow trigger from `pull_request` to tag push (`push.tags`), updated GitHub Action versions (`checkout@v4`, `upload-artifact@v4`, `download-artifact@v4`).
- **Engine Compile Errors**: Resolved premature brace closure in `pdf_engine.rs`, aligned `zpdf` 0.11 API calls for `IncrementalWriter`, `rewrite_pdf`, `RedactOptions`, and `Profile`.
- **Clippy Cleanliness**: Fixed all `doc_markdown`, `uninlined_format_args`, `format_push_string`, and `items_after_statements` lints across library and tests.

---

## [0.9.1] - 2026-07-19

### Added
- **PDFgear-Inspired Ribbon GUI Overhaul**: Modernized header ribbon toolbar with tabs for File, Home, Annotate, Convert, Tools, and View.
- **Performance & Startup Optimization**: Sub-1-second cold launch times and optimized page caching.

---

## [0.9.0] - 2026-07-10

### Added
- **Table Extraction & Bounding Box UI**: Automatic grid detection, cell outlines, and 1-click CSV/TSV copy actions.
- **Digital Signatures Verification**: Signature dictionary parsing and byte-range digest validation.
- **Attachments Panel**: Dedicated sidebar tab for listing and saving embedded files.
- **Layers Manager (OCProperties)**: Optional Content Group visibility toggling in sidebar.
- **Password-Protected PDF Support**: Prompt and decrypt password-protected PDFs on open.
- **Tagged PDF Support**: Structured reading order text extraction.
