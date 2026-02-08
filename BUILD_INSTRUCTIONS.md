# Building PDFbull with MuPDF

## Prerequisites

1. **Rust Toolchain**: Ensure you have Rust installed via `rustup`.
2. **Node.js**: Install Node.js v18+.
3. **Visual Studio C++ Build Tools**: 
   - Install "Desktop development with C++" workload.
   - Ensure the Windows SDK is installed.

## MuPDF Setup (Critical)

The `mupdf-sys` crate compiles the C library from source. This often fails on Windows if the environment isn't perfect.

**If `cargo check` fails with `bin2coff.targets` error:**
1. Open "x64 Native Tools Command Prompt for VS 2022" (search in Start menu).
2. Navigate to the project directory.
3. Run `cargo build`.

## Running the App

```bash
# Install frontend deps
npm install

# Run in development mode
npm run tauri dev
```

## Troubleshooting

If MuPDF continues to fail, consider switching to `pdfium-render` in `src-tauri/Cargo.toml` as it uses pre-built binaries:

```toml
[dependencies]
pdfium-render = "0.8"
# mupdf = "0.8"  <-- Comment this out
```

And update `src-tauri/src/pdf_engine.rs` to use the PDFium implementation provided earlier.
