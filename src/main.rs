// Prevent console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod pdf_engine;

use slint::{ComponentHandle, Image, Rgba8Pixel, SharedPixelBuffer};
use std::sync::mpsc;
use std::thread;

slint::include_modules!();

enum PdfCommand {
    Open(String, mpsc::Sender<Result<i32, String>>),
    Render(i32, f32, mpsc::Sender<Result<(u32, u32, Vec<u8>), String>>),
    Close,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ui = AppWindow::new()?;
    let ui_handle = ui.as_weak();

    // Create channel for PDF worker
    let (cmd_tx, cmd_rx) = mpsc::channel::<PdfCommand>();

    // Spawn PDF worker thread
    let worker = thread::spawn(move || {
        // Initialize Pdfium inside the worker thread to ensure thread-locality/safety
        let pdfium = match pdf_engine::PdfEngine::init_pdfium() {
            Ok(p) => p,
            Err(e) => {
                eprintln!("Failed to initialize Pdfium: {}", e);
                return;
            }
        };

        // PdfEngine borrows pdfium
        let mut engine = pdf_engine::PdfEngine::new(&pdfium);

        while let Ok(cmd) = cmd_rx.recv() {
            match cmd {
                PdfCommand::Open(path, resp) => {
                    let result = engine.open_document(&path);
                    if let Err(e) = resp.send(result) {
                        eprintln!("Failed to send open result: {}", e);
                    }
                }
                PdfCommand::Render(page, scale, resp) => {
                    let result = engine.render_page(page, scale);
                    if let Err(e) = resp.send(result) {
                        eprintln!("Failed to send render result: {}", e);
                    }
                }
                PdfCommand::Close => {
                    engine.close_document();
                }
            }
        }
    });

    // Handler for "Open Document" button
    ui.on_open_document({
        let ui_handle = ui_handle.clone();
        let cmd_tx = cmd_tx.clone();
        move || {
            // We don't strictly need to upgrade here since we spawn a thread that will upgrade later,
            // but it's a good check. 
            // However, to avoid "unused variable" warning and keep it simple:
            if ui_handle.upgrade().is_none() {
                return;
            }

            let ui_handle = ui_handle.clone();
            let cmd_tx = cmd_tx.clone();

            // Spawn a thread for the file dialog to avoid blocking the UI thread
            thread::spawn(move || {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("PDF Files", &["pdf"])
                    .pick_file() 
                {
                    let path_str = path.to_string_lossy().to_string();
                    let filename = path
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default();

                    // Update UI from UI thread
                    let _ = ui_handle.upgrade_in_event_loop(move |ui| {
                        ui.set_status_text("Loading PDF...".into());
                        ui.set_is_loading(true);
                    });

                    let (resp_tx, resp_rx) = mpsc::channel();
                    if let Err(e) = cmd_tx.send(PdfCommand::Open(path_str, resp_tx)) {
                        eprintln!("Failed to send Open command: {}", e);
                        return;
                    }

                    if let Ok(result) = resp_rx.recv() {
                        let _ = ui_handle.upgrade_in_event_loop(move |ui| {
                            ui.set_is_loading(false);
                            match result {
                                Ok(count) => {
                                    ui.set_total_pages(count);
                                    ui.set_current_page(0);
                                    ui.set_status_text(format!("Loaded: {}", filename).into());
                                    // Trigger initial render
                                    ui.invoke_request_render(0, ui.get_zoom());
                                }
                                Err(e) => {
                                    ui.set_status_text(format!("Error: {}", e).into());
                                }
                            }
                        });
                    }
                }
            });
        }
    });

    // Handler for close_document callback
    ui.on_close_document({
        let ui_handle = ui_handle.clone();
        let cmd_tx = cmd_tx.clone();
        move || {
            let ui = match ui_handle.upgrade() {
                Some(ui) => ui,
                None => return,
            };
            
            if let Err(e) = cmd_tx.send(PdfCommand::Close) {
                eprintln!("Failed to send Close command: {}", e);
            }
            
            ui.set_total_pages(0);
            ui.set_current_page(0);
            ui.set_status_text("Document closed - Open a PDF to begin".into());
            
            let empty_buffer = SharedPixelBuffer::<Rgba8Pixel>::new(1, 1);
            ui.set_pdf_page_image(Image::from_rgba8(empty_buffer));
        }
    });

    // Handler for request_render callback
    ui.on_request_render({
        let ui_handle = ui_handle.clone();
        let cmd_tx = cmd_tx.clone();

        move |page_num, scale| {
            let (resp_tx, resp_rx) = mpsc::channel();
            if let Err(e) = cmd_tx.send(PdfCommand::Render(page_num, scale, resp_tx)) {
                eprintln!("Failed to send Render command: {}", e);
                return;
            }

            let ui_handle2 = ui_handle.clone();
            thread::spawn(move || {
                if let Ok(result) = resp_rx.recv() {
                    let _ = ui_handle2.upgrade_in_event_loop(move |ui| {
                        match result {
                            Ok((w, h, bytes)) => {
                                let mut buffer = SharedPixelBuffer::<Rgba8Pixel>::new(w, h);
                                let slice = buffer.make_mut_bytes();

                                // BGRA -> RGBA (fix loop safety)
                                // Standard loop is safer than iterator zip when sizes might mismatch slightly
                                // though chunks(4) is good. We just need to be careful with bounds.
                                let len = slice.len();
                                for (i, chunk) in bytes.chunks(4).enumerate() {
                                    let offset = i * 4;
                                    if offset + 3 < len {
                                        slice[offset] = chunk[2];     // B -> R
                                        slice[offset + 1] = chunk[1]; // G -> G
                                        slice[offset + 2] = chunk[0]; // R -> B
                                        slice[offset + 3] = chunk[3]; // A -> A
                                    }
                                }

                                let image = Image::from_rgba8(buffer);
                                ui.set_pdf_page_image(image);
                            }
                            Err(e) => {
                                eprintln!("Render error: {}", e);
                            }
                        }
                    });
                }
            });
        }
    });

    ui.run()?;

    // Clean up worker
    drop(cmd_tx);
    let _ = worker.join();

    Ok(())
}
