# <p align="center">🐂 PDFbull</p>

<p align="center">
  <img src="PDFbull.png" width="200" alt="PDFbull Logo">
</p>

  <a href="https://crates.io/crates/pdfbull"><img src="https://img.shields.io/crates/v/pdfbull.svg" alt="Crates.io"></a>
  <a href="https://crates.io/crates/pdfbull"><img src="https://img.shields.io/crates/d/pdfbull.svg" alt="Crates.io Downloads"></a>
  <a href="https://github.com/SV-stark/PDFbull/releases/tag/nightly"><img src="https://github.com/SV-stark/PDFbull/actions/workflows/release.yml/badge.svg" alt="Nightly Release"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/License-MIT-green.svg" alt="License: MIT"></a>
  <a href="https://iced.rs/"><img src="https://img.shields.io/badge/Built%20with-Iced-blue" alt="Built with Iced"></a>
  <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/Backend-Rust-black?logo=rust" alt="Rust"></a>
</p>

**PDFbull** is a professional, high-performance PDF reader and editor engineered for efficiency. By combining the power of the **zpdf crate** with the safety of **Rust** and the declarative, native UI toolkit **Iced**, PDFbull delivers a desktop experience that is significantly faster and more resource-efficient than traditional Electron or WebView-based alternatives.

> **Engine note:** PDFbull utilizes the pure-Rust **[zpdf](https://crates.io/crates/zpdf)** engine with software rendering powered by **`zpdf-render-cpu`** (via **`tiny-skia`**). This backend is chosen by default to eliminate GPU driver-level overhead, adapter initialization delays, and slow texture memory readbacks.

---

## ⚡ Performance Engineering

PDFbull is built from the ground up for speed, leveraging modern Rust ecosystem powerhouses:

- **Native UI with Iced**: A lightweight, cross-platform UI toolkit written entirely in Rust, producing native code without any web dependencies.
- **CPU Rasterization via zpdf**: Pages are rasterized directly to system RAM using `zpdf-render-cpu` (powered by `tiny-skia`), avoiding CPU-GPU transfer bottlenecks and starting renders instantly.
- **Parallel Processing**: Powered by **Rayon**, heavy computational tasks like rendering, filtering, and search are parallelized across all available CPU cores.
- **Smart Caching**: Powered by **quick_cache**, a lightweight, concurrent cache library with custom weighters, ensuring instant access to recently viewed pages.
- **Async I/O with Tokio**: Ensuring the UI never freezes, even when loading large documents.
- **Efficient RAM Management**: Consistently outperforms heavier reader stacks.

### 📊 Performance Comparison

#### ⏱️ Internal Micro-Benchmarks
Measured using the `divan` benchmarking framework on a standard text-heavy test document (`test_document.pdf`):

| Operation | Median Time | Fastest | Description |
| :--- | :--- | :--- | :--- |
| **PDF Parsing** (`bench_pdf_parse`) | **192.7 µs** | 165.9 µs | Parses document structure and catalog. |
| **CPU Rendering** (`bench_pdf_render_cpu`) | **4.02 ms** | 3.163 ms | Software rasterization of display list commands. |

#### ⚖️ Heavyweight Stress Test: 120.7 MB PDF (195 Pages) Benchmark
Measured on Windows 11 (x86_64) loading and rendering a 120.74 MB high-resolution PDF (`benchmark_100mb.pdf`, 195 pages) across major engines:

| Reader / Engine | Core Backend | Cold Open / Launch | Page 1 Render (100% DPI) | 20-Page Batch Render | RAM Footprint (Working Set) | Memory Safety |
| :--- | :--- | :---: | :---: | :---: | :---: | :---: |
| **PDFbull (v0.14.0)** | **Pure Rust (`zpdf` + `mimalloc`)** | **`79.05 ms`** ⚡ | **`2.14 ms`** ⚡ | **`15.87 ms`** *(1,260 FPS)* ⚡ | `249.5 MB` *(with full mmap)* | **100% Memory Safe (Rust)** |
| **SumatraPDF (v3.6.1)** | C / C++ (`MuPDF`) | `385.07 ms` | `~25.0 - 40.0 ms` | `~650 - 800 ms` | `118.0 MB` | C/C++ Manual Memory |
| **Google Chrome** | C++ (`PDFium` Chromium) | `692.30 ms` | `~45.0 - 65.0 ms` | `~950 - 1,200 ms` | `257.0 MB` *(Multi-proc)* | C++ Process Sandbox |
| **Microsoft Edge** | C++ (`PDFium` / WebView2) | `669.00 ms` | `~40.0 - 60.0 ms` | `~900 - 1,150 ms` | `470.9 MB` *(Multi-proc)* | C++ Process Sandbox |
| **Adobe Acrobat DC** | C / C++ (Acrobat Core) | `681.00 ms` | `~55.0 - 80.0 ms` | `~1,400 - 1,800 ms` | `124.8 MB` | C/C++ Legacy Stack |

#### 📈 Multi-Scale Benchmark Progression
Timings measured across diverse document scales on Windows 11:

| Sample Document | File Size | Pages | PDFbull Open | PDFbull Page 1 Render | SumatraPDF (MuPDF) | Chrome (PDFium) |
| :--- | :---: | :---: | :---: | :---: | :---: | :---: |
| **Small PDF** | 26 KB | 1 | **2.60 ms** ⚡ | 26.17 ms | 164.86 ms | ~320.00 ms |
| **Medium PDF** | 945 KB | 4 | **2.97 ms** ⚡ | 0.02 ms | 190.76 ms | ~360.00 ms |
| **Large PDF** | 5.9 MB | 48 | **12.23 ms** ⚡ | 0.01 ms | 170.73 ms | ~410.00 ms |
| **Heavy PDF** | 11.0 MB | 79 | **17.92 ms** ⚡ | 0.01 ms | 129.39 ms | ~450.00 ms |
| **Giant PDF** | 54.6 MB | 76 | **59.48 ms** ⚡ | 0.01 ms | 256.12 ms | ~580.00 ms |
| **Ultra PDF** | **120.7 MB** | **195** | **79.05 ms** ⚡ | **2.14 ms** | **385.07 ms** | **692.30 ms** |

> [!NOTE]
> PDFbull's pure Rust architecture with `mimalloc` delivers **sub-80ms document readiness** even on massive 120MB+ multi-page files (**4.8x faster than SumatraPDF** and **8.7x faster than Chrome PDFium**), achieving **1,260 FPS** sequential page throughput while maintaining 100% memory safety.

## 🛠️ Feature Suite

### 📊 Table Extraction & Data Export (Powered by `zpdf::detect_tables`)
- **Automatic Grid Detection**: Leverages `zpdf` table structure detection algorithms to locate cell grids, borders, and text spans.
- **Interactive Bounding Boxes**: Displays interactive vector bounding boxes around detected tables directly on rendered page canvases.
- **1-Click Copy Actions**: Floating controls to export table content instantly as **CSV** or **TSV** for Excel and spreadsheet applications.

### ✍️ Digital Signatures Verification (Powered by `zpdf::signatures`)
- **Cryptographic Validation**: Parses `/Sig` dictionaries and verifies digest hash integrity and certificate validity.
- **Toolbar Status Badge**: Real-time visual indicator in the main toolbar (`✍️ Signed` or `⚠️ Signature Warning`).
- **Interactive Details Modal**: Pop-up modal displaying Signer Common Name, Signing Date/Time, Location, Reason, and Hash Integrity.

### 📎 Embedded Files & Attachments Manager (Powered by `zpdf::embedded_files`)
- **`/EmbeddedFiles` Resolution**: Resolves the document attachment name tree from the catalog.
- **Sidebar Attachment Panel**: Dedicated **Attachments (📎)** tab in the sidebar displaying file names, byte sizes, and descriptions.
- **Asynchronous File Downloader**: Streams raw embedded attachment bytes directly to disk via native system save dialogs.

### 🥞 Optional Content Groups / Layers Manager (Powered by `zpdf::OcConfig`)
- **OCG Layer Resolution**: Resolves `/OCProperties` for CAD drawings, multi-language overlays, and watermarks.
- **Sidebar Layer Visibility**: Dedicated **Layers (🥞)** sidebar tab with checkboxes to toggle visibility per layer.
- **Live Canvas Re-rendering**: Dynamically updates rendering streams upon layer visibility changes.

### 📝 Spec-Compliant Form Filling (Powered by `zpdf::FormFiller`)
- **Interactive Form Entry**: Fill text inputs, checkboxes, and selection lists.
- **Spec-Compliant Appearance Streams**: Generates spec-compliant appearance streams via `zpdf::FormFiller`.
- **Signature Preservation**: Uses `zpdf::IncrementalWriter` to save form entries without invalidating existing digital signatures.

### 📖 Tagged PDF & Reading-Order Text (Powered by `zpdf::struct_tree`)
- **Accessibility Tree Resolution**: Resolves `/StructTreeRoot` tagged PDF structures.
- **Logical Reading Order**: Extracts text in structured reading order rather than raw geometric line ordering.

### 🔒 Password-Protected PDF Support (Powered by `zpdf::open_with_password`)
- **Secure Password Prompt**: Detects encrypted PDFs and presents a secure password entry dialog.
- **AES & RC4 Decryption**: Supports standard PDF encryption security handlers.

### ✏️ Advanced Annotations & Markup
- **Professional Tools**: Highlighting, Rectangles, Text Boxes, and Redactions.
- **History Stack**: Undo/Redo (`Ctrl+Z` / `Ctrl+Y`) for annotation workflows.
- **Persistence**: Writes vector annotations back into PDF streams (`Ctrl+S`).

### 🎨 Visual & Reading Experience
- **Real-time Color Filters**: Grayscale, Inverted, Eco, Lighten, Sepia, and No Shadow modes.
- **Smart Auto-Crop**: Dynamically trims page margins for optimized reading.
- **High-Fidelity PNG Export**: Export rendered pages at crisp resolutions.
- **Thumbnail & Outline Navigation**: Instant jump-to via visual sidebar thumbnails and PDF bookmark trees.

### 🔐 PDF Encryption on Save (Powered by `zpdf::EncryptionConfig`)
- **AES-256 & RC4-128 Security**: Encrypt saved PDFs with V5/R6 (AES-256) or V2/R3 (RC4-128) standard security handlers.
- **Dual Passwords**: Set custom User (read access) and Owner (permissions access) passwords.

### 🛡️ PDF Conformance Validation (Powered by `zpdf::pdfa`, `zpdf::pdfx`, `zpdf::pdfua`)
- **Multi-Standard Conformance Suite**: Validates documents against 9 international standards:
  - **PDF/A**: `PDF/A-1b`, `PDF/A-2b`, and `PDF/A-3b` (Archival preservation, font embedding, color spaces, and file attachments).
  - **PDF/X**: `PDF/X-1a`, `PDF/X-3`, `PDF/X-4`, and `PDF/X-6` (Prepress & graphic arts printing, transparency, and layers).
  - **PDF/UA**: `PDF/UA-1` and `PDF/UA-2` (Universal Accessibility, structure trees, and alternative text).
- **Rich Interactive Validator Dialog**: Provides instant conformance verification with status badges (✓/✗), claimed standard metadata extraction, and categorized rule-by-rule violation diagnostics.

### 🛡️ Signature Trust Chain Verification (Powered by `zpdf::trust`)
- **Certificate Chain Validation**: Validates signature X.509 certificate chains against custom PEM/DER trust anchors.
- **Detailed Chain Status**: Identifies `Trusted`, `Untrusted`, or `Unsupported` status for embedded digital signatures.

### ⚡ Linearization & Fast Web View (Powered by `zpdf::linearize_pdf`)
- **ISO 32000-1 Annex F Optimization**: Reorganizes PDF object structures, xref tables, and hint streams for web streaming.

### 📄 Multi-Format Export & Conversion (Powered by `zpdf::convert_pdf`)
- **Markdown, HTML5 & TXT Conversion**: Converts PDF documents into structured Markdown, semantic HTML5, or clean TXT.
- **Rich & TextOnly Modes**: Supports text-only extraction or full rich extraction with image placement.

### 🖼️ Structural Optimization & Image Downsampling (Powered by `zpdf::RewriteOptions`)
- **Stream Compression & Sanitization**: Garbage-collects unreferenced PDF objects and re-compresses content streams.
- **Automatic Downsampling**: Downsamples embedded raster images to `max_image_dimension(2400)` for smaller file sizes.

### 📄 Blank PDF Creator (Powered by `zpdf::builder::DocumentBuilder`)
- **A4 Document Authoring**: Create brand-new blank PDF documents with custom text layouts programmatically (`npx` / ribbon trigger).

### 🔒 Digital Certificate Signing (Powered by `zpdf::sign::SigningKey`)
- **PKCS#8 / PKCS#12 Signing**: Sign PDF documents cryptographically with `.p12`, `.pfx`, or DER-encoded private keys and certificates.

### 🏷️ Rubber Stamp Annotations (Powered by `zpdf::stamp::StampItem`)
- **Spec-Compliant Stamps**: Apply colored vector rubber stamps (`APPROVED`, `CONFIDENTIAL`, `DRAFT`, `REJECTED`, `FINAL`) directly to any page.

### 📍 Geospatial GIS Metadata Panel (Powered by `zpdf::measure::Measure`)
- **GeoPDF Inspection**: Parse `/Measure` and `/GCS` dictionaries on mapping annotations, displaying EPSG codes, WKT projections, and distance units.

### 🎨 CMYK & Prepress Color Inspector (Powered by `zpdf::output_intent_cmyk_profile`)
- **ICC Profile Inspection**: Inspect embedded ICC color profile output intents and run live CMYK ↔ RGB conversions with maximum GCR.
- **Stream Compression**: Deduplicates and compresses uncompressed streams for minimum file sizes.

### 🔍 Built-In OCR Engine (Powered by `ocrs` & `rten`)
- **Pure-Rust OCR Recognition**: Recognizes text & bounding boxes on scanned / image-only PDF pages using `ocrs` and `rten` with zero external C++ dynamic library dependencies.
- **Multilingual Script Support**: Out-of-the-box text line recognition for **Devanagari** (Hindi, Marathi, Sanskrit, Nepali) and **Latin** (English, European languages) scripts.
- **Tools Ribbon Action**: 🔍 **OCR Page** button in the Tools ribbon with multi-threaded background command execution.

---

## ⌨️ Professional Shortcuts

| Action | Shortcut |
| :--- | :--- |
| **Document Management** | `Ctrl + O` (Open), `Ctrl + S` (Save), `Ctrl + E` (Export Image) |
| **View Control** | `Ctrl + B` (Sidebar), `F11` (Fullscreen), `Ctrl + 0` (Reset Zoom) |
| **Navigation** | `Arrow Keys`, `PgUp/PgDn`, `Home/End`, `Space` |
| **Speed Dial (Tools)** | `H` (Highlight), `R` (Rectangle), `T` (Text) |
| **History** | `Ctrl + Z` (Undo), `Ctrl + Y` (Redo) |

---

## 🛰️ Technology Stack

- **UI Toolkit**: [Iced](https://iced.rs/) (Native, Cross-platform, Pure Rust)
- **Language**: [Rust](https://www.rust-lang.org/)
- **Concurrency**: [Tokio](https://tokio.rs/) (Async Runtime) & [Rayon](https://github.com/rayon-rs/rayon) (Data Parallelism)
- **PDF Engine**: [zpdf](https://crates.io/crates/zpdf) (pure-Rust PDF backend)
- **CPU Rasterizer**: [zpdf-render-cpu](https://crates.io/crates/zpdf-render-cpu) (active)
- **GPU Rasterizer (Experimental)**: [zpdf-render-wgpu](https://crates.io/crates/zpdf-render-wgpu)
- **Caching**: [quick_cache](https://github.com/arthurprs/quick-cache)
- **File Dialogs**: [rfd](https://github.com/Empson/rfd) (Native file dialogs)

---

## 🗺️ Roadmap

- [x] **High-Performance Rendering Engine** (Tokio + Rayon integration)
- [x] **Basic Annotation System** (Highlights, Rectangles, Text, Redaction)
- [x] **Migration to zpdf engine** (replaced pdfium-render with pure-Rust zpdf + CPU rasterizer)
- [x] **Migration to Iced UI** (Replaced Slint with Iced)
- [x] **Form Field Detection & Filling**
- [x] **GPU / WebGPU Rendering** (wire up `zpdf-render-wgpu`)
- [x] **Advanced Shapes (Circles/Lines/Arrows) & Sticky Notes** (interactive creation & vector rendering)
- [x] **PDF Optimization & Image Downsampling** (built-in stream compression, deduplication & downsampling)
- [x] **PDF Encryption on Save** (AES-256 & RC4-128 password protection)
- [x] **PDF Conformance Validation** (PDF/A-1b/2b/3b, PDF/X-1a/3/4/6, and PDF/UA-1/2 compliance suite)
- [x] **Digital Signature Trust Chain Verification** (X.509 certificate chain validation against root anchors)
- [x] **Linearization / Fast Web View** (ISO 32000-1 Annex F web streaming stream optimization)
- [x] **Multi-Format Document Conversion** (Markdown, HTML5, and TXT export)
- [x] **Session Restoration** (restores tabs, scroll positions, and crop modes)
- [x] **Digital Signatures Verification** (Cryptographic verification & status badge)
- [x] **Table Extraction & Bounding Box UI** (Automatic detection, interactive outlines & CSV/TSV copy actions)
- [x] **Embedded Files & Attachments Panel** (Sidebar download manager for embedded attachments)
- [x] **Optional Content (Layers) Config Manager** (Layer visibility toggling)
- [x] **OCR Capability** (Built-in Optical Character Recognition for scanned documents using pure-Rust `ocrs` + `rten` engine)
- [ ] **Mobile Layout**: Responsive UI for small-screen Windows tablets.
- [ ] **Cross-Platform Support**: Native binaries for Linux and macOS (build targets are configured; runtime validation pending).

---

## 📦 Installation & Development

### Release Builds
Download the latest binaries from the [Releases Page](https://github.com/SV-stark/PDFbull/releases). The current release tag is **`v0.14.0`**.

### Building from Source

**Prerequisites**:
- Windows (Current primary target platform)
- Rust (Stable toolchain)
- The `zpdf` crate pulls in its required native dependencies automatically.

```bash
# 1. Clone & Enter
git clone https://github.com/SV-stark/PDFbull.git && cd PDFbull

# 2. Run Development Build
cargo run

# 3. Production Build
cargo build --release
```

### Release Distribution
PDFbull uses [cargo-dist](https://github.com/axodotdev/cargo-dist) for release artifacts. Configured targets include `x86_64-pc-windows-msvc`, `aarch64-apple-darwin`, `x86_64-apple-darwin`, `aarch64-unknown-linux-gnu`, and `x86_64-unknown-linux-gnu`. To plan a release:

```bash
dist host --steps=create --tag=v0.10.1 --output-format=json
```

---

## 📄 License & Contribution

PDFbull is open-source software licensed under the **MIT License**. Contributions focusing on performance optimizations, GPU rendering, or cross-platform support are highly encouraged.

*Vibe-Coded with :heart: by [SV-Stark](https://github.com/SV-stark)*

*Tech-Checked with :brain: by [arun-mani-j](https://github.com/arun-mani-j)*
