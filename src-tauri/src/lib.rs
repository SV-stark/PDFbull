mod annotations;
mod compression;
mod filters;
mod forms;
pub mod pdf_engine;

use pdf_engine::PdfState;
use tauri::Emitter;

#[tauri::command]
async fn save_file(path: String, data: Vec<u8>) -> Result<(), String> {
    tokio::fs::write(&path, data).await.map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            let _ = app.emit("open-file", args.get(1));
        }))
        .setup(|app| {
            let args: Vec<String> = std::env::args().collect();
            if args.len() > 1 {
                let path = args[1].clone();
                let app_handle = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    // Small delay to ensure frontend is ready to listen
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                    let _ = app_handle.emit("open-file", path);
                });
            }
            Ok(())
        })
        .manage(PdfState::new())
        .invoke_handler(tauri::generate_handler![
            // File Operations
            save_file,
            // PDF Engine
            pdf_engine::open_document,
            pdf_engine::load_document_from_bytes,
            pdf_engine::get_page_count,
            pdf_engine::render_page,
            pdf_engine::get_page_dimensions,
            pdf_engine::get_page_text,
            pdf_engine::search_text,
            pdf_engine::test_pdfium,
            pdf_engine::ping,
            // Annotations
            annotations::save_annotations,
            // Filters
            filters::apply_filter,
            filters::auto_crop,
            // Forms
            forms::get_form_fields,
            // Compression
            compression::compress_pdf,
            // Scanner
            pdf_engine::apply_scanner_filter,
            pdf_engine::search_document,
            pdf_engine::get_page_text_with_coords
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
