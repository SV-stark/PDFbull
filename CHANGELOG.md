# Changelog

All notable changes to the PDFbull project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
