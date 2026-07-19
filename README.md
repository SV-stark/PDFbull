# <p align="center">🐂 PDFbull</p>

<p align="center">
  <img src="PDFbull.png" width="200" alt="PDFbull Logo">
</p>

<p align="center">
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

#### ⏱️ Internal Benchmarks
Measured using the `divan` benchmarking framework on a standard text-heavy test document (`test_document.pdf`):

| Operation | Median Time | Fastest | Description |
| :--- | :--- | :--- | :--- |
| **PDF Parsing** (`bench_pdf_parse`) | **192.7 µs** | 165.9 µs | Parses document structure and catalog. |
| **CPU Rendering** (`bench_pdf_render_cpu`) | **4.02 ms** | 3.163 ms | Software rasterization of display list commands. |

#### ⚖️ Real-World Benchmark: PDFbull vs. SumatraPDF (MuPDF Backend)
Cold-start launch and page rendering timings measured on Windows 11 across various document sizes:

| Sample Document | File Size | Pages | PDFbull Open | PDFbull Page 1 Render | **PDFbull Total First View** | SumatraPDF (MuPDF) |
| :--- | :---: | :---: | :---: | :---: | :---: | :---: |
| **Small PDF** | 26 KB | 1 | **2.60 ms** | 26.17 ms | **~28.77 ms** ⚡ | 164.86 ms |
| **Medium PDF** | 945 KB | 4 | **14.80 ms** | 0.04 ms | **~14.84 ms** ⚡ | 190.76 ms |
| **Large PDF** | 5.9 MB | 45 | **147.28 ms** | 68.43 ms | **~215.71 ms** | 170.73 ms |
| **Heavy PDF** | 11.0 MB | 84 | **56.24 ms** | 184.92 ms | **~241.16 ms** | 129.39 ms |
| **Giant PDF** | 54.6 MB | 412 | **191.28 ms** | 412.30 ms | **~603.58 ms** | 256.12 ms |

> [!NOTE]
> PDFbull's native Rust architecture with `mimalloc` achieves sub-30ms first view times on small-to-medium documents (**6x to 12x faster** startup than SumatraPDF). For large multi-page documents (e.g. 54.6 MB with 412 pages), PDFbull parses all metadata, catalog trees, layer OCGs, digital signatures, and embedded attachments in under 200 milliseconds.

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
- **GPU Rasterizer**: [zpdf-render-wgpu](https://crates.io/crates/zpdf-render-wgpu) (active)
- **CPU Rasterizer (Fallback)**: [zpdf-render-cpu](https://crates.io/crates/zpdf-render-cpu)
- **Caching**: [quick_cache](https://github.com/arthurprs/quick-cache)
- **File Dialogs**: [rfd](https://github.com/Empson/rfd) (Native file dialogs)

---

## 🚧 Work in Progress (WIP)

The following capabilities are partially present in the codebase or planned, but are **not yet fully functional** and should be considered experimental:



### ✏️ Advanced Annotations
- **Layer Management**: Independent visibility toggles and z-index ordering for annotations — **planned**.

### 📐 Productivity
- **Batch Mode**: Infrastructure for processing multiple documents simultaneously — **planned**.

### 🎨 Visuals & UX
- **High-Contrast Theme**: Specialized high-contrast mode for accessibility — **planned**.
- **True Virtual Scrolling**: Enhanced engine for infinite-feeling scrolling in massive documents — **planned**.
- **Drag & Drop**: Native file and tab drag-and-drop support — **planned**.

---

## 🗺️ Roadmap

- [x] **High-Performance Rendering Engine** (Tokio + Rayon integration)
- [x] **Basic Annotation System** (Highlights, Rectangles, Text, Redaction)
- [x] **Migration to zpdf engine** (replaced pdfium-render with pure-Rust zpdf + CPU rasterizer)
- [x] **Migration to Iced UI** (Replaced Slint with Iced)
- [x] **Form Field Detection & Filling**
- [x] **GPU / WebGPU Rendering** (wire up `zpdf-render-wgpu`)
- [x] **Advanced Shapes (Circles/Lines/Arrows) & Sticky Notes** (interactive creation & vector rendering)
- [x] **PDF Optimization** (built-in stream compression & metadata sanitization)
- [x] **Session Restoration** (restores tabs, scroll positions, and crop modes)
- [x] **Digital Signatures Verification** (Cryptographic verification & status badge)
- [x] **Table Extraction & Bounding Box UI** (Automatic detection, interactive outlines & CSV/TSV copy actions)
- [x] **Embedded Files & Attachments Panel** (Sidebar download manager for embedded attachments)
- [x] **Optional Content (Layers) Config Manager** (Layer visibility toggling)
- [ ] **OCR Capability**: Built-in Optical Character Recognition for scanned documents.
- [ ] **Mobile Layout**: Responsive UI for small-screen Windows tablets.
- [ ] **Cross-Platform Support**: Native binaries for Linux and macOS (build targets are configured; runtime validation pending).

---

## 📦 Installation & Development

### Release Builds
Download the latest binaries from the [Releases Page](https://github.com/SV-stark/PDFbull/releases). The current release tag is **`pdfbull-v0.8.0`**.

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
dist host --steps=create --tag=pdfbull-v0.8.0 --output-format=json
```

---

## 📄 License & Contribution

PDFbull is open-source software licensed under the **MIT License**. Contributions focusing on performance optimizations, GPU rendering, or cross-platform support are highly encouraged.

*Vibe-Coded with :heart: by [SV-Stark](https://github.com/SV-stark)*

*Tech-Checked with :brain: by [arun-mani-j](https://github.com/arun-mani-j)*
