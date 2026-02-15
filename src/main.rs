// Prevent console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod pdf_engine;

use slint::Weak;
use slint::{ComponentHandle, Image, Rgba8Pixel, SharedPixelBuffer};
use std::rc::Rc;
use std::sync::{Arc, Mutex};

slint::include_modules!();

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ui = AppWindow::new()?;
    let ui_handle = ui.as_weak();

    let pdf_state = Arc::new(Mutex::new(pdf_engine::PdfState::new()));

    // Handler for "Open Document" button
    ui.on_open_document({
        let ui_handle = ui_handle.clone();
        let pdf_state = pdf_state.clone();
        move || {
            let ui = ui_handle.unwrap();

            // Native File Dialog (Pure Rust - No Tauri)
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("PDF Files", &["pdf"])
                .pick_file()
            {
                let path_str = path.to_string_lossy().to_string();
                let mut state = pdf_state.lock().unwrap();

                match state.open_document(&path_str) {
                    Ok(count) => {
                        ui.set_total_pages(count);
                        ui.set_current_page(0);
                        ui.set_status_text(format!("Loaded: {}", path_str).into());

                        // Trigger initial render
                        ui.invoke_request_render(0, 1.0);
                    }
                    Err(e) => {
                        ui.set_status_text(format!("Error: {}", e).into());
                    }
                }
            }
        }
    });

    // Handler for request_render callback
    // Called when page or zoom changes in UI
    ui.on_request_render({
        let ui_handle = ui_handle.clone(); // Need handle to set property?
                                           // Actually, callback closure argument is NOT the UI handle.
                                           // But we are inside `ui.on_request_render`, so we can capture `ui_handle`.
        let pdf_state = pdf_state.clone();

        move |page_num, scale| {
            let ui = ui_handle.unwrap();
            let state = pdf_state.lock().unwrap();

            match state.render_page(page_num, scale) {
                Ok((w, h, bytes)) => {
                    let mut buffer = SharedPixelBuffer::<Rgba8Pixel>::new(w, h);

                    // Direct access to the pixel buffer for maximum performance
                    let slice = buffer.make_mut_bytes();

                    // PDFium renders as BGRA; Slint expects RGBA.
                    for (i, chunk) in bytes.chunks(4).enumerate() {
                        if i * 4 + 3 < slice.len() {
                            // BGRA -> RGBA
                            slice[i * 4] = chunk[2]; // R
                            slice[i * 4 + 1] = chunk[1]; // G
                            slice[i * 4 + 2] = chunk[0]; // B
                            slice[i * 4 + 3] = chunk[3]; // A
                        }
                    }

                    let image = Image::from_rgba8(buffer);
                    ui.set_pdf_page_image(image);
                }
                Err(e) => {
                    // Log error to status?
                    // ui.set_status_text(format!("Render error: {}", e).into());
                    // Avoid infinite loop if render keeps failing?
                }
            }
        }
    });

    ui.run()?;
    Ok(())
}
