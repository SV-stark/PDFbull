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

> **Engine note:** PDFbull migrated from `pdfium-render` to the pure-Rust **[zpdf](https://crates.io/crates/zpdf)** engine with GPU-accelerated rendering powered by **`zpdf-render-wgpu`**. CPU-based rendering (`zpdf-render-cpu`) is maintained as a feature toggle/compile option.

---

## ⚡ Performance Engineering

PDFbull is built from the ground up for speed, leveraging modern Rust ecosystem powerhouses:

- **Native UI with Iced**: A lightweight, cross-platform UI toolkit written entirely in Rust, producing native code without any web dependencies.
- **GPU Rendering via zpdf**: Pages are rasterized on the GPU in native Rust memory space using the `zpdf-render-wgpu` backend and uploaded directly to the UI buffer.
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
| **GPU Rendering** (`bench_pdf_render_gpu`) | **685.7 ms** | 447.1 ms | WebGPU/WGPU hardware rasterization (includes pipeline/device init overhead). |

#### ⚖️ Engine-to-Engine Comparison (Single Page Render)
Typical performance metrics for rendering a standard text-heavy PDF page:

| Engine / Application | Runtime / Language | Page Render Time (Median) | Memory Footprint | Architecture Type |
| :--- | :--- | :--- | :--- | :--- |
| **PDFbull (`zpdf` GPU)** | Rust / WebGPU | **~1.2 ms** (excluding WGPU init) | **Very Low** (~35 MB) | Pure Rust Native |
| **PDFbull (`zpdf` CPU)** | Rust / Tiny-Skia | **4.02 ms** (measured) | **Very Low** (~30 MB) | Pure Rust Native |
| **MuPDF** | C / Assembly | **~2 - 5 ms** | **Extremely Low** (<15 MB) | Native Binary |
| **Chrome (PDFium)** | C++ | **~3 - 8 ms** | **High** (~120 MB+) | Sandbox Native (Chromium) |
| **Adobe Acrobat** | C++ | **~5 - 12 ms** | **Very High** (~180 MB+) | Heavy Desktop Client |
| **Firefox (pdf.js)** | JavaScript | **~15 - 50 ms** | **Moderate** (via Browser) | Web Canvas Interpreter |

> [!NOTE]
> Native compiled backends (MuPDF, PDFium, and zpdf) achieve significantly faster rendering times and lower memory usage compared to browser sandbox environments like Firefox's JavaScript-based pdf.js. WGPU rendering in zpdf leverages hardware acceleration to minimize rasterization overhead once the device context is initialized.

## 🛠️ Feature Suite

### ✏️ Advanced Annotations
- **Professional Tools**: Highlighting, Rectangles, Text Boxes, and Redaction.
- **Robust History**: Basic Undo/Redo stack (`Ctrl+Z` / `Ctrl+Y`) for annotation lifecycle.
- **Persistence**: Hybrid saving strategy with local storage fallbacks and manual `Ctrl+S` save (writes annotations back into the PDF).

### 📐 Productivity Utilities
- **Fast Search**: Leverages zpdf's structured text engine for instantaneous document-wide searching with result navigation.
- **Tabbed Interface**: Multi-document management with tab-based navigation.
- **Recent Files**: Quick access dropdown for recently opened documents.
- **Page Bookmarks**: Mark important pages via the bookmark button for quick reference and jump-to navigation. (PDF outline/bookmarks are also listed in the sidebar.)
- **Smart Formatting**:
    - **Auto-Crop**: Dynamically removes whitespace margins for optimized reading on smaller displays.
- **Data Export**:
    - **High-Fidelity Image Export**: Save any page as a crisp PNG.
    - **Text Extraction**: One-click extraction of document text to `.txt` format.
- **Form Filling**: Built-in form field detection (`get_form_fields`) and data entry/export (`fill_form`).
- **Real-time Filters**: Professional document rendering filters — Grayscale, Inverted, Lighten, Eco, No Shadow, and Sepia.

### 🎨 Visual Experience
- **Adaptive Themes**: Seamlessly switch between Light and Dark modes with customizable accent colors.
- **Fullscreen Mode**: Toggle immersive reading with `F11`.
- **Page Virtualization**: Efficient rendering of large documents with on-demand page loading.
- **Thumbnail Navigation**: Visual page overview in the left sidebar for quick navigation.
- **Page Rotation**: Rotate pages in 90° increments for comfortable viewing.

### ⚙️ Customization & Settings
- **Settings Dialog**: Fine-tune appearance, behavior, performance, and file handling.
- **Keyboard Help Modal**: Built-in shortcut reference accessible via `?` or `F1`.
- **Configurable Auto-Save**: Adjustable auto-save intervals for annotation safety.

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
- [ ] **OCR Capability**: Built-in Optical Character Recognition for scanned documents.
- [ ] **Digital Signatures**: Professional cryptographic signing and verification.
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
