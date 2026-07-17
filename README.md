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

> **Engine note:** PDFbull migrated from `pdfium-render` to the pure-Rust **[zpdf](https://crates.io/crates/zpdf)** engine (with CPU rasterization via `zpdf-render-cpu`). All rendering is currently performed on the **CPU** path. The GPU (WebGPU) renderer (`zpdf-render-wgpu`) is included as a dependency but is **work-in-progress and not yet wired in** — see [Work in Progress](#-work-in-progress-wip).

---

## ⚡ Performance Engineering

PDFbull is built from the ground up for speed, leveraging modern Rust ecosystem powerhouses:

- **Native UI with Iced**: A lightweight, cross-platform UI toolkit written entirely in Rust, producing native code without any web dependencies.
- **CPU Rendering via zpdf**: Pages are rasterized in native Rust memory space using the `zpdf-render-cpu` backend and uploaded directly to the UI buffer.
- **Parallel Processing**: Powered by **Rayon**, heavy computational tasks like rendering, filtering, and search are parallelized across all available CPU cores.
- **Smart Caching**: Powered by **quick_cache**, a lightweight, concurrent cache library with custom weighters, ensuring instant access to recently viewed pages.
- **Async I/O with Tokio**: Ensuring the UI never freezes, even when loading large documents.
- **Efficient RAM Management**: Consistently outperforms heavier reader stacks.

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
- **CPU Rasterizer**: [zpdf-render-cpu](https://crates.io/crates/zpdf-render-cpu)
- **GPU Rasterizer (WIP)**: [zpdf-render-wgpu](https://crates.io/crates/zpdf-render-wgpu) — declared, not yet active
- **Caching**: [quick_cache](https://github.com/arthurprs/quick-cache)
- **File Dialogs**: [rfd](https://github.com/Empson/rfd) (Native file dialogs)

---

## 🚧 Work in Progress (WIP)

The following capabilities are partially present in the codebase or planned, but are **not yet fully functional** and should be considered experimental:

### 🖥️ GPU / WebGPU Rendering
- The `zpdf-render-wgpu` backend is included as a dependency, but **GPU rasterization is not wired into the render pipeline**. PDFbull currently renders exclusively on the **CPU** path (`zpdf-render-cpu`). GPU acceleration is a future optimization.

### ✏️ Advanced Annotations
- **Geometric Shapes**: Circles, Lines, and Arrows are represented in the data model and sidebar, but their interactive creation/editing is **not yet implemented**.
- **Sticky Notes**: Contextual comments and sticky note annotations — **not yet implemented**.
- **Layer Management**: Independent visibility toggles and z-index ordering for annotations — **planned**.

### 📐 Productivity
- **Batch Mode**: Infrastructure for processing multiple documents simultaneously — **planned**.
- **Session Restoration**: Automatic reopening of last session (tabs and scroll positions) on startup — **planned**.
- **PDF Optimization**: Built-in document compression and metadata sanitization — **planned**.

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
- [ ] **GPU / WebGPU Rendering** (wire up `zpdf-render-wgpu`)
- [ ] **Advanced Shapes (Circles/Lines/Arrows) & Sticky Notes**
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
