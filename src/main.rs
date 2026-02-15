// Prevent console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod pdf_engine;

use slint::{ComponentHandle, Image, Rgba8Pixel, SharedPixelBuffer};
use std::sync::mpsc;

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
    let worker = std::thread::spawn(move || {
        let mut state = pdf_engine::PdfState::new();

        while let Ok(cmd) = cmd_rx.recv() {
            match cmd {
                PdfCommand::Open(path, resp) => {
                    let result = state.open_document(&path);
                    let _ = resp.send(result);
                }
                PdfCommand::Render(page, scale, resp) => {
                    let result = state.render_page(page, scale);
                    let _ = resp.send(result);
                }
                PdfCommand::Close => {
                    state.close_document();
                }
            }
        }
    });

    // Handler for "Open Document" button
    ui.on_open_document({
        let ui_handle = ui_handle.clone();
        let cmd_tx = cmd_tx.clone();
        move || {
            let ui = ui_handle.unwrap();

            if let Some(path) = rfd::FileDialog::new()
                .add_filter("PDF Files", &["pdf"])
                .pick_file()
            {
                let path_str = path.to_string_lossy().to_string();
                let filename = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();

                ui.set_status_text("Loading PDF...".into());

                let (resp_tx, resp_rx) = mpsc::channel();
                let _ = cmd_tx.send(PdfCommand::Open(path_str, resp_tx));

                // Spawn thread to wait for result
                let ui_handle2 = ui_handle.clone();
                std::thread::spawn(move || {
                    if let Ok(result) = resp_rx.recv() {
                        let ui = ui_handle2.unwrap();
                        match result {
                            Ok(count) => {
                                ui.set_total_pages(count);
                                ui.set_current_page(0);
                                ui.set_status_text(format!("Loaded: {}", filename).into());
                                ui.invoke_request_render(0, 1.0);
                            }
                            Err(e) => {
                                ui.set_status_text(format!("Error: {}", e).into());
                            }
                        }
                    }
                });
            }
        }
    });

    // Handler for close_document callback
    ui.on_close_document({
        let ui_handle = ui_handle.clone();
        let cmd_tx = cmd_tx.clone();
        move || {
            let ui = ui_handle.unwrap();
            let _ = cmd_tx.send(PdfCommand::Close);
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
            let ui = ui_handle.unwrap();
            let (resp_tx, resp_rx) = mpsc::channel();
            let _ = cmd_tx.send(PdfCommand::Render(page_num, scale, resp_tx));

            let ui_handle2 = ui_handle.clone();
            std::thread::spawn(move || {
                if let Ok(result) = resp_rx.recv() {
                    let ui = ui_handle2.unwrap();
                    match result {
                        Ok((w, h, bytes)) => {
                            let mut buffer = SharedPixelBuffer::<Rgba8Pixel>::new(w, h);
                            let slice = buffer.make_mut_bytes();

                            // BGRA -> RGBA
                            for (i, chunk) in bytes.chunks(4).enumerate() {
                                if i * 4 + 3 < slice.len() {
                                    slice[i * 4] = chunk[2];
                                    slice[i * 4 + 1] = chunk[1];
                                    slice[i * 4 + 2] = chunk[0];
                                    slice[i * 4 + 3] = chunk[3];
                                }
                            }

                            let image = Image::from_rgba8(buffer);
                            ui.set_pdf_page_image(image);
                        }
                        Err(_) => {}
                    }
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
