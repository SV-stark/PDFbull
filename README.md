# PDFbull

![Nightly Release](https://github.com/SV-stark/PDFbull/actions/workflows/release.yml/badge.svg)
![License: AGPL-3.0](https://img.shields.io/badge/License-AGPL%203.0-blue.svg)
![Tauri](https://img.shields.io/badge/Built%20with-Tauri%202.0-orange)

**PDFbull** is a modern, high-performance PDF editor for Windows that respects your privacy and your time. Built with **Tauri** and **Rust**, it delivers the raw speed of **MuPDF** in a lightweight, secure package.

## 🚀 Why PDFbull?

Stop paying monthly subscriptions for bloatware. PDFbull is designed to be the only PDF tool you'll ever need.

| Feature | 🐂 PDFbull | 🏢 Industry Standard |
| :--- | :--- | :--- |
| **Startup Speed** | **Instant (< 0.5s)** | Sluggish (5s - 15s) |
| **Installer Size** | **~10 MB** | ~1 GB+ |
| **RAM Usage** | **Minimal (~50 MB)** | Heavy (500 MB+) |
| **Privacy** | **100% Offline & Local** | Telemetry & Cloud Tracking |
| **Cost** | **Free & Open Source** | $15 - $20 / month |
| **Bloatware** | **None** | Background Services & Updates |

## ✨ Key Features

### ⚡ Blazing Fast Performance
-   **Zero-Copy Rendering**: Native binary data transfer ensures pages load instantly without lag.
-   **Anti-Aliased Zoom**: Crystal clear text at any zoom level.
-   **Instant Navigation**: Jump to any page with zero delay.

### 🛠️ Powerful Tools
-   **Smart Form Detection**: Automatically identifies and lists form fields for easy editing.
-   **Auto-Crop**: Remove useless margins with a single click to focus on content.
-   **Advanced Compression**: Reduce file size significantly without losing quality.
-   **Text Search**: Lightning-fast search powered by MuPDF's structured text engine.

### 🎨 Modern & Customizable
-   **Dark Mode**: Sleek, native dark theme with glassmorphism support.
-   **Reading Modes**: Toggle **Greyscale** or **High Contrast** (Invert Colors) for eye comfort.
-   **Annotation**: Highlight important text with precision.

## 🛠️ Built With

-   **Frontend**: HTML5, Vanilla JavaScript (No heavy frameworks), CSS3 Variables.
-   **Backend**: Rust (Tauri 2.0).
-   **Engine**: [MuPDF](https://mupdf.com/) (via `mupdf-rs`).

## 📦 Installation

### Download
Grab the latest nightly build from the [Releases Page](https://github.com/SV-stark/PDFbull/releases/tag/nightly).

### Build from Source
**Prerequisites**: Windows, Visual Studio C++ Build Tools, Rust, Node.js (v18+).

```bash
# Clone and Enter
git clone https://github.com/SV-stark/PDFbull.git
cd PDFbull

# Install Dependencies
npm install

# Run (Development)
npm run tauri dev

# Build (Release)
npm run tauri build
```
The installer will be generated in `src-tauri/target/release/bundle/nsis/`.

## 📄 License

This project is licensed under the **GNU Affero General Public License v3.0 (AGPL-3.0)**.

*Note: This project statically links against the **MuPDF** library (Artifex Software, Inc). Distribution requires compliance with AGPL terms.*

---
*Built with ❤️ by SV-Stark*
