# PDFbull

![Nightly Release](https://github.com/SV-stark/PDFbull/actions/workflows/release.yml/badge.svg)
![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)
![Tauri](https://img.shields.io/badge/Built%20with-Tauri%202.0-orange)

**PDFbull** is a high-performance, lightweight PDF reader and editor built for Windows. It leverages the raw speed of **PDFium** (Google Chrome's engine) combined with the safety of **Rust** and the flexibility of **Tauri**.

Unlike other Electron-based readers that consume massive RAM, PDFbull is optimized for efficiency, using direct binary data transfer for rendering and native Rust bindings for heavy lifting.

## 🚀 Key Features

### ⚡ Performance & Core
-   **Instant Opening**: Open 100MB+ PDFs in <50ms using PDFium's memory-mapped file opening.
-   **Optimized Rendering**: Pages are rendered in native Rust memory and streamed to the UI as high-quality binary blobs for a smooth experience.
-   **Low Initial RAM**: Minimal memory footprint on startup, only mapping what's visible on screen.

### ✏️ Annotation Suite
-   **Comprehensive Tools**:
    -   Highlight (Yellow, Green, Blue, Pink)
    -   Shapes (Rectangle, Circle, Line, Arrow)
    -   Text Box
    -   Sticky Notes
-   **Layer Management**: Create multiple annotation layers and toggle their visibility independently.
-   **Undo/Redo**: Full history support for all annotation actions (`Ctrl+Z` / `Ctrl+Y`).
-   **Auto-Save**: Annotations are automatically saved to local storage every 30 seconds.

### 🛠️ Advanced Tools
-   **Search**: Fast text search using PDFium's structured text engine.
-   **Export Options**:
    -   **Export Page as Image**: Save current view as high-quality PNG.
    -   **Extract Text**: Save page text to `.txt` file.
-   **Form Scanning**: Automatically detect and identify form fields.
-   **Auto-Crop**: Remove whitespace margins automatically to focus on content.
-   **Compression**: Re-save PDFs with maximum compression to reduce file size.

### 🎨 Visual customization
-   **Themes**: Light, Dark, and High Contrast modes.
-   **Filters**:
    -   **Greyscale**: For distraction-free reading.
    -   **Invert Colors**: High contrast mode for night reading.
-   **Fullscreen Mode**: Immersive reading experience (`F11`).

## ⌨️ Keyboard Shortcuts

| Action | Shortcut |
| :--- | :--- |
| **Open File** | `Ctrl + O` |
| **Search** | `Ctrl + F` |
| **Toggle Sidebar** | `Ctrl + B` |
| **Undo / Redo** | `Ctrl + Z` / `Ctrl + Y` |
| **Zoom In / Out** | `Ctrl + +` / `Ctrl + -` |
| **Reset Zoom** | `Ctrl + 0` |
| **Export Image** | `Ctrl + E` |
| **Save Annotations** | `Ctrl + S` |
| **Fullscreen** | `F11` |
| **Tools** | `H` (Highlight), `R` (Rect), `C` (Circle), `L` (Line), `A` (Arrow), `T` (Text), `N` (Note), `Esc` (View) |
| **Navigation** | Arrow Keys, PageUp/Down, Space, Home, End |

##  Comparison to Industry Standards

| Feature | PDFbull 🐂 | Adobe Acrobat 🔴 | Chrome PDF 🔵 | Sumatra PDF 🟡 |
| :--- | :--- | :--- | :--- | :--- |
| **Engine** | PDFium (Rust) | Proprietary | PDFium | MuPDF (C++) |
| **Startup Time** | **Instant** (<100ms) | Slow (~2s) | Fast (~200ms) | Instant (<50ms) |
| **RAM Usage** | **Efficient** (~120MB) | Heavy (400MB+) | High (250MB+) | Ultra-Low (~40MB) |
| **Rendering** | **Native Stream** | Standard | Standard | Standard |
| **Privacy** | **100% Local** | Cloud-Connected | Google Tracking | 100% Local |
| **Annotations** | **Rich** (Shapes, Layers) | Rich | Basic | Basic |
| **Price** | **Free (Open Source)** | Subscription | Free | Free (Open Source) |

## 🛠️ Technology Stack

-   **Frontend**: HTML5, Vanilla JavaScript (Zero-framework for speed), CSS3 Variables.
-   **Backend**: Rust (Tauri 2.0).
-   **PDF Engine**: [pdfium-render](https://crates.io/crates/pdfium-render).

## 📦 Installation

### Download
Grab the latest nightly build from the [Releases Page](https://github.com/SV-stark/PDFbull/releases/tag/nightly).

### Build from Source

**Prerequisites**:
-   **Windows** (Required for current build config)
-   **Rust** (Latest Stable)
-   **Node.js** (v18+)

1.  **Clone the repository**:
    ```bash
    git clone https://github.com/SV-stark/PDFbull.git
    cd PDFbull
    ```

2.  **Install Frontend Dependencies**:
    ```bash
    npm install
    ```

3.  **Run in Development Mode**:
    ```bash
    npm run tauri dev
    ```

4.  **Build Release**:
    ```bash
    npm run tauri build
    ```
    The installer will be in `src-tauri/target/release/bundle/nsis/`.

## 📄 License

This project is licensed under the **MIT License** - see the [LICENSE](LICENSE) file for details.

## 🤝 Contributing

Contributions are welcome! Please open an issue or submit a pull request.

---
*Built with ❤️ by SV-Stark*
