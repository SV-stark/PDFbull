# Changelog

All notable changes to the PDFbull project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.16.2] - 2026-09-22

### Added & Production-Ready Desktop Features
- **Comprehensive Desktop Feature Wiring**:
  - **Save & Save As**: Wired `Ctrl+S` and Ribbon Save action to execute incremental `PdfCommand::SaveAnnotations` and full `PdfCommand::ExportPdf` via asynchronous file dialogs, persisting all annotations back to PDF files.
  - **Print Pipeline**: Wired `Ctrl+P` and Ribbon Print action to invoke `PdfCommand::PrintPdf` with native Windows printer spooling.
  - **Global Desktop Keyboard Shortcuts**: Registered cross-platform hotkeys including `Ctrl+O` (Open), `Ctrl+S` (Save), `Ctrl+P` (Print), `Ctrl+W` (Close Tab), `Ctrl+F` (Open Search), `Ctrl+=` / `Ctrl+-` / `Ctrl+0` (Zoom In/Out/Reset), `PageUp` / `Left` and `PageDown` / `Right` (Page Navigation), and `Esc` (Dismiss Dialogs/Selections).
  - **Interactive Search Engine & In-Page Highlights**: Connected sidebar search bar with query execution via `PdfCommand::Search`, occurrence count tracking, snippet listing, click-to-jump page navigation, and amber translucent search match highlights overlaid directly on the PDF document canvas.
  - **Rich Sidebar Panels**: Implemented live panels for Bookmarks outline navigation (`LoadDocumentMeta`), Annotations list with per-item jump and delete buttons, Attachments inspection with file size formatting, and optional PDF Layers toggles.
  - **Multi-Style Annotation Tools**: Extended canvas drag creation beyond basic highlights to support Underline, Strikeout, Rectangle, Circle, Line, Arrow, StickyNote, and Redact annotations with distinct rendering styles and modified-document state tracking.
  - **Dark / Midnight Inversion Mode**: Connected Midnight mode toggle to `RenderFilter::Inverted` in `request_render_page` with instant cache clearing and re-rendering for high-contrast nighttime reading.
  - **Book Spread View with Standalone Cover**: Updated `TwoPageSpread` layout to support `standalone_cover`, correctly displaying cover page 0 centered as a single page while pairing subsequent pages as true two-page spreads.
  - **PDF Utility Workflows**: Wired interactive modal dialogs and file operations for Merge (multi-PDF picker and merge), Split (directory export), Compress (`PdfCommand::Optimize`), Add Watermark (preset and custom stamps), Add Header & Footer, Document Encryption (AES-128/256), and Page Organizer (90° CW/CCW rotation and page deletion).
  - **Document Conversion**: Integrated export actions for Markdown (`.md`), HTML (`.html`), and Plain Text (`.txt`) via `PdfCommand::ConvertPdf`.
  - **Developer Log Console**: Built a slide-up log drawer with streaming log entries, severity level filtering (`all`, `info`, `warn`, `error`), one-click Copy to Clipboard, and Clear Log capability.
  - **Status Message Banner**: Integrated live operation feedback (save status, encryption, OCR progress, print spooling notifications) directly into the bottom persistent status bar.

## [0.16.1] - 2026-09-21

### Fixed & Improved (GPUI-Kit Integration)
- **Visual & UX Redesign (GPUI-Kit)**:
  - Redesigned Welcome Hub with branded hero monogram, version badge, glowing drag-and-drop zone, 2×2 rich action cards (Open, Page Organizer, Digital Signatures, Security), and structured recent document cards.
  - Upgraded Ribbon toolbar with native desktop tab navigation, segmented action groups separated by hairline dividers, and muted section captions (`File`, `Page`, `Zoom`, `Layout`, `Markup`, `Shapes`, `Security`, etc.).
  - Added bottom persistent status bar with document geometry (`pt`), layout mode indicator, interactive page stepper, and quick zoom controls.
  - Enhanced document canvas with floating translucent page badges (`Page X of Y`), physical drop shadows, and refined tab strip styling.
- **Continuous Scroll & Viewport Navigation**:
  - Implemented stable virtual scroll container (`canvas-continuous-scroll`) with GPUI `ScrollHandle` and `.vertical_scrollbar(&self.scroll_handle)`.
  - Added full document page layout in continuous mode with dynamic top-item detection and pre-rendering as the user scrolls.
  - Linked thumbnail clicks to `scroll_handle.scroll_to_item(page_idx)` for smooth jumping across pages.
- **Ctrl + Mouse Wheel Zoom & Smooth Progressive Scaling**:
  - Eliminated the brief white flash during zooming by retaining existing GPU textures (`rendered_pages`) during zoom level changes instead of discarding them.
  - Implemented progressive scaling where GPUI's hardware-accelerated bilinear texture filter scales current page images in real time at 60+ FPS while higher-resolution rasterization proceeds asynchronously.
  - Added `rendered_zoom` tracking in `DocumentViewport` to seamlessly swap in crisp high-resolution renders without layout jumping or intermediate blank frames.
  - Handled rapid multi-step zooming gracefully with race condition prevention and in-flight render re-triggering.
- **Color Correction for Direct3D 11 Textures**:
  - Resolved inverted/reversed color balance (where red appeared blue and blue appeared red) by implementing `convert_rgba_to_bgra`.
  - Converted tiny-skia RGBA byte buffers to BGRA and un-premultiplied alpha as required by GPUI DirectX 11 textures.
- **Text Selection Highlighting & Word-Level Highlighting**:
  - Fixed text selection highlighting where selecting text did not visually highlight the words on the page.
  - Implemented precise word-level highlight quads (`window.paint_quad(fill(clipped_word, ...))`) painted directly over every intersecting `TextItem` in the page's text stream.
  - Added true coordinate translation between window pixels and PDF page points by tracking dynamic page card window origins (`viewport.page_origins`), ensuring selection accuracy regardless of window size, continuous scroll offset, or zoom level.
  - Corrected `TextItem` vertical coordinate calculation in `pdf_engine.rs` (`(page_height - span.y - span.size).max(0.0)`) to align with the top-left of font glyph bounding boxes rather than the baseline.
  - Added proactive text stream prefetching in `render_needed_pages` so text items and bounding boxes are immediately available when the user begins selecting.
  - Wired `RibbonTab::Annotate` -> `AnnotationTool::Highlight` and custom palette colors, allowing users to apply permanent yellow (or custom) highlights to selected text that persist across page interactions.
- **Mouse Drag Text Selection**:
  - Added mouse drag selection handlers (`on_mouse_down`, `on_mouse_move`, `on_mouse_up`) to canvas page cards.
  - Painted dynamic translucent blue selection highlight boxes over selected text via `canvas` quad clipping.
  - Integrated `PdfCommand::GetTextItems` to extract overlapping words within the selection bounding box and automatically copy to the system clipboard.
- **PDF Page Rasterization & Rendering Pipeline**:
  - Connected the multi-threaded `PdfEngine` worker pool (`spawn_engine_thread`) to the GPUI presentation layer.
  - Resolved the blank/white page display issue when opening PDF documents by piping `RenderResult` RGBA buffers into native GPUI `RenderImage` elements (`img(render_image).w_full().h_full()`).
  - Implemented asynchronous, non-blocking `request_render_page` pipeline using `cx.spawn` with automatic HiDPI scaling (1.5x multiplier).
  - Added live page thumbnail rasterization and rendering to the collapsible Sidebar page navigation list.
  - Wired zoom scaling, tab switching, and page navigation to synchronize viewport state and trigger automatic re-rendering.
- **Native File Dialog Integration**:
  - Wired native asynchronous file dialogs via `rfd::AsyncFileDialog` into GPUI's async task runner (`cx.spawn`).
  - Connected file browsing to the Welcome screen "Browse Files..." button, Welcome dropzone click, Ribbon Home tab "Open" action, and Tab Bar `+` button.
- **Full Drag-and-Drop Ingestion**:
  - Implemented `ExternalPaths` drop handling across both the dedicated Welcome dropzone and the global window surface.
  - Users can now drag and drop one or multiple PDF documents directly into PDFbull from Windows Explorer.
- **Windows Stack & Layout Stability**:
  - Expanded Windows linker stack reserve to 8 MB (`/STACK:8388608`) in `.cargo/config.toml` and `build.rs` to eliminate debug MSVC stack overflow.
  - Modularized Ribbon action strip rendering into `#[inline(never)]` helpers with heap-allocated element vectors.
  - Prevented circular layout negotiation loops by replacing `.size_full()` with `.flex_1().w_full()`.
- **GPUI Kit Design Guides Alignment**:
  - Audited against normative GPUI Kit Design Guides (`gpui-kit-design-guides`), ensuring semantic theme token usage (`cx.theme()`), desktop ergonomics, and clear visual state hierarchies.

## [0.16.0] - 2026-09-21

### Major UI Overhaul (GPUI-Kit)
- **Next-Gen GPU-Accelerated UI Architecture**:
  - Rewrote and expanded the application presentation layer to modern `gpui-kit` (`gpui-component` / GPUI), delivering buttery smooth GPU rendering and modern desktop aesthetics.
  - Built modular presentation sub-systems under `src/ui_gpui/`:
    - `app_view`: Top-level unified view (`PdfbullView`) composing header, ribbon strip, tabs, sidebar, canvas, developer log drawer, status bar, and modal overlays.
    - `ribbon`: Interactive 5-tab ribbon strip (Home, View, Annotate, Tools, Convert) with tool selector buttons and layout controls.
    - `tabs`: Multi-document tab bar with modified state indicators (`*`), close buttons, and tab switching.
    - `sidebar`: Collapsible sidebar with 6 navigation modes (Pages/Thumbnails, Bookmarks, Annotations, Fuzzy Search, Attachments, Layers).
    - `canvas`: Document viewport supporting multiple layout modes (Continuous, Single Page, Two-Page Spread), zoom scaling, and interactive page cards.
    - `dialogs`: Full-screen modal overlay system covering Watermark, Header/Footer, Security/Permissions, Password Prompt, Digital Signatures, Page Organizer, and Settings.
    - `welcome`: Modern drag-and-drop dropzone with quick-action cards (Merge PDFs, Page Organizer, Sign Document) and recent files.
    - `log_console`: Collapsible bottom developer drawer with severity filtering (All, Info, Warn, Error), copy all, and clear.
- **Zero Regressions & Backward Compatibility**:
  - Maintained 100% test coverage and zero regressions with all 153 unit, integration, OCR, and benchmark tests passing.
  - Retained CLI fallback flag (`--iced`) for instant switching to the classic Iced presentation engine.

## [0.15.3] - 2026-09-21

### Changed & Improved
- **Vector Icons System**:
  - Completely migrated UI from raw Unicode emojis to crisp, resolution-independent Lucide vector icons (`.font(LUCIDE)`).
  - Expanded `pub mod icons` with standard Lucide glyph codepoints (`HOME`, `EYE`, `PEN_TOOL`, `WRENCH`, `REFRESH_CW`, `MOON`, `SCROLL`, `FILE`, `BOOK_OPEN`, `BOOK`, `CIRCLE`, `MINUS`, `ARROW_RIGHT`, `STICKY_NOTE`, `LAYOUT_GRID`, `DROPLET`, `HASH`, `LOCK`, `SIGNATURE`, `SCISSORS`, `ZAP`, `FILE_PLUS`, `KEY`, `PALETTE`, `SCAN_TEXT`, `FILE_CODE`, `GLOBE`, `FILE_TEXT`, `IMAGE`, `PAPERCLIP`, `LAYERS`, `FOLDER`, `TRASH_2`, `LOADER`, `EXPAND_HORIZONTAL`, `MAXIMIZE`, `SPARKLES`, etc.).
  - Replaced all ribbon toolbar strip icons across View, Annotate, Tools, and Convert modes with vector glyphs.
  - Phased out and removed legacy `tool_button_emoji`.
  - Converted raw emoji color swatches into sleek circular dot containers with active glow halos.
  - Replaced emojis in Sidebar tabs (Thumbnails, Bookmarks, Annotations, Search, Attachments, Layers), Search HUD, canvas sticky notes, document loading spinners, and Welcome Quick Action cards.
  - Updated all modal dialog headers and action buttons (Watermark, Headers & Footers, Security & Permissions, Password Prompt, Digital Signature creator, Page Organizer, Signatures Inspector, Log Console) to native Lucide vector icons.
- **Dependency Updates**:
  - Updated project dependencies to latest compatible versions via `cargo update`.

## [0.15.1] - 2026-09-12

### Fixed & Optimized
- **Rust 2024 Concurrency & Win32 FFI Safety**:
  - Replaced deprecated `static mut` mutex handle with thread-safe `OnceLock<MutexHolder>` implementing RAII `Drop` cleanup.
  - Eliminated runtime heap allocation of UTF-16 strings in Win32 single-instance mutex and window detection using zero-cost `windows::core::w!` literals.
  - Added comprehensive `// SAFETY:` rationale comments across all Windows FFI call sites.
- **Robust Error Handling**:
  - Eliminated production `.unwrap()` call in document merge routing by adopting `Option::get_or_insert_with`.
- **Memory & Allocation Optimizations**:
  - Reused internal matching buffers in Command Palette fuzzy search (`filter_palette_items`), eliminating repetitive `format!` allocations.
  - Pre-allocated text assembly buffer in `OcrPageResult::full_text`, removing intermediate vector allocations.
  - Removed redundant string and path clones across tab session restoration and background command channels.
- **Idiomatic API Design**:
  - Implemented `From<SearchResultItem> for SearchResult` for standard trait conversions.

## [0.15.0] - 2026-09-08

### Added
- **Two-Page Spread / Book View & Standalone Cover Toggle**:
  - Dual-page side-by-side reading layout (`PageLayoutMode::TwoPageSpread`) designed for eBooks, magazines, and double-column papers.
  - Standalone cover toggle (`two_page_cover`) to present Page 1 centered as a standalone cover with subsequent pages displayed in two-page pairs.
  - View ribbon controls: `📖 Spread` mode toggle and `📕 Cover: On/Off` button.
  - Page navigation stepping by 2 pages in spread mode with proper cover alignment.
- **Continuous Scroll vs. Single-Page Presentation Mode**:
  - `PageLayoutMode::SinglePage` presentation mode focusing on a single active page centered in viewport without vertical overflow.
  - Ribbon selector buttons: `📜 Continuous` and `📄 Single` for instant switching.
  - Arrow keys (`Left`/`Right`) and `PgUp`/`PgDn` page snapping for distraction-free reading.
- **Tab Ergonomics**:
  - **Middle-Click to Close Tab**: Middle-clicking any tab directly closes it.
  - **Right-Click Tab Context Menu**: Context menu offering `Close Tab`, `Close Others`, `Close Tabs to the Right`, `Copy File Path`, and `Open Containing Folder`.
  - **Drag-and-Drop PDF File Opening**: Drag any PDF file directly into the PDFbull window canvas to open it in a new tab, with visual drop overlay indicator.
- **Interactive Drag to Select Text**:
  - Native `Text` (I-beam) cursor hovering over pages in pointer mode.
  - Dynamic click-and-drag marquee bounding box with live word highlighting during dragging.
  - Rotation-independent (0°, 90°, 180°, 270°) and zoom-independent coordinate space mapping.
  - Multi-line smart formatting: lines grouped by baseline, ordered horizontally, separated by newlines, with merged highlight bounding boxes per line segment.
  - Automatic clipboard copy on mouse release, `Ctrl+C` manual copy, and `H` / `Ctrl+H` one-key permanent PDF highlight annotation creation.
  - Click outside (< 3px) to dismiss selection.

## [0.14.0] - 2026-08-25

### Added
- **In-App Developer Log Console (`src/ui_log_console.rs` & `src/logging.rs`)**:
  - Thread-safe 1,000-entry memory-backed circular ring buffer (`RingBufferLayer`) capturing live structured events across engine workers and UI threads.
  - Interactive bottom drawer panel with real-time level filtering (`All`, `TRACE`, `DEBUG`, `INFO`, `WARN`, `ERROR`), 1-click **Copy All** to clipboard, buffer clearing, and automatic scroll-to-bottom toggle.
  - Color-coded monospace log viewer with distinct severity styling (Red errors, amber warnings, cyan info, and slate debug/trace).
  - New keyboard shortcut **`Ctrl+L`**, Command Palette entry (`Toggle Log Console`), and **`🖥️ Logs`** toolbar button.
  - Startup persistence: added `show_logs_on_start` setting in `AppSettings` configurable via the Settings modal.
- **Comprehensive PDF Conformance Validator Suite**: Added end-to-end conformance validation powered by `zpdf = "=0.13.0"` supporting 9 international standard profiles:
  - **PDF/A**: `PDF/A-1b` (ISO 19005-1), `PDF/A-2b` (ISO 19005-2), and `PDF/A-3b` (ISO 19005-3 archival with embedded files).
  - **PDF/X**: `PDF/X-1a` (ISO 15930-1), `PDF/X-3` (ISO 15930-3), `PDF/X-4` (ISO 15930-7), and `PDF/X-6` (ISO 15930-9 prepress with live transparency and layers).
  - **PDF/UA**: `PDF/UA-1` (ISO 14289-1) and `PDF/UA-2` (ISO 14289-2 universal accessibility).
- **Interactive Conformance Validation Modal**: Dedicated UI modal (`src/ui_conformance.rs`) featuring standard profile cards, 1-click async validation runner, conformance status banner (✓/✗), claimed metadata tag extraction, and scrollable rule violation diagnostics.
- **Entry Points & Command Palette**: Added `🛡️ Validate` button to the main tools toolbar and integrated `Validate PDF Conformance` into the fuzzy Command Palette (`Ctrl+K`).
- **Generalized Conformance Data Model**: Replaced legacy dead-code PDF/A report with `ConformanceReport` and `ConformanceFamily`, normalizing claimed standard metadata across PDF/A (`pdfaid`), PDF/X (`GTS_PDFXVersion`), and PDF/UA.

### Fixed
- **Resolved Stray Release Terminal Window**: Moved `#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]` to `src/main.rs:1` (binary root), eliminating unwanted Windows console window popups on release builds while maintaining full debugging stdout output during development.
- **Panic Hook Log Stream Integration**: Wired panic handlers directly into `tracing::error!(target: "panic", ...)` so unexpected panics immediately appear in the in-app log console.

## [0.13.7] - 2026-08-24

### Removed & Replaced
- **Replaced `dark-light` with Native Win32 Registry Inspection**: Removed `dark-light` dependency (and its transitive `async-std` / `zbus` trees), replacing it with a zero-dependency Win32 `RegGetValueW` registry check on `AppsUseLightTheme` in `src/platform/windows.rs`.
- **Resolved `cargo-deny` Advisories & Licenses**: Fixed `deny.toml` for `cargo-deny` 0.16+ / 0.20+, allowed all permissive transitive licenses (`0BSD`, `BSL-1.0`, `IJG`, `NCSA`, `CDLA-Permissive-2.0`), and purged unneeded advisory exclusions.

## [0.13.6] - 2026-08-24

### Fixed
- **Tracing Worker Guard Lifetime**: Maintained non-blocking `WorkerGuard` for the full application lifecycle via `Box::leak`, guaranteeing file log persistence and eliminating panic race conditions in production.
- **Genuine OCR Text Recognition Engine**: Replaced synthetic layout approximations with real `ocrs::OcrEngine` and `rten` detection/recognition neural network inference and PDF user space coordinate mapping.
- **`OcConfig` Memory Safety & Pinned Layout**: Pinned `zpdf = "=0.12.1"` and `zpdf-writer = "=0.12.1"` and added compile-time static assertions validating `zpdf::OcConfig` size and alignment against internal representations.
- **Rotation-Aware Render Cache**: Included page rotation angle in `RenderKey` and centralized multi-worker cache invalidation and document removal in `RenderCache`.
- **Memory & Resource Protection Limits**: Enforced 512 MB file size limit checks prior to memory reads in both `open_document` and `convert_pdf_doc`.
- **Session Restore Tab Synchronization**: Attached `pending_session` directly to newly opened tabs during restore to prevent race conditions with background document loading.
- **Password Zeroization**: Guaranteed immediate zeroization of sensitive password buffers upon failed authentication attempts.
- **Single-Instance Multi-File Launching**: Ensured all command-line arguments are transmitted and handled over local IPC pipes without dropping secondary file arguments.
- **Cross-Filesystem Atomic Writes**: Added reliable fallback in `atomic_write` for cross-device links, NTFS junctions, and mount boundaries.
- **Panic Hook Chaining**: Properly chained existing custom panic handlers after `human_panic` initialization.

### Changed & Improved
- **PNG Compression Optimization**: Optimized PNG export compression via `oxipng` preset 2.
- **CI Benchmarking Coverage**: Added automated benchmark compilation checks (`cargo check --benches`) in CI workflows.
- **Dependency Upgrades**: Upgraded dependencies to latest compatible versions including `blocking 1.7`, `cc 1.4`, `h2 0.4.18`, `uuid 1.25`, and `libdeflater 1.26`.

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
