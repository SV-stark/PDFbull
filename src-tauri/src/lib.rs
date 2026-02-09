mod annotations;
mod compression;
mod filters;
mod forms;
mod ocr;
pub mod pdf_engine;

use pdf_engine::PdfState;
use tauri::Emitter;

#[tauri::command]
async fn save_file(path: String, data: Vec<u8>) -> Result<(), String> {
    tokio::fs::write(&path, data).await.map_err(|e| e.to_string())
}

// ==================== OCR Commands ====================

#[tauri::command]
async fn ocr_document(
    pages: Vec<Vec<u8>>,
    language: String,
    window: tauri::Window,
) -> Result<Vec<ocr::PageTextBlocks>, String> {
    let engine = ocr::get_ocr_engine();
    let engine_lock = engine.lock().map_err(|e| format!("Failed to lock OCR engine: {}", e))?;
    engine_lock.run_ocr(pages, &language, window)
}

#[tauri::command]
async fn cancel_ocr() -> Result<(), String> {
    let engine = ocr::get_ocr_engine();
    let engine_lock = engine.lock().map_err(|e| format!("Failed to lock OCR engine: {}", e))?;
    engine_lock.cancel();
    Ok(())
}

#[tauri::command]
async fn list_ocr_languages() -> Result<Vec<ocr::models::LanguageInfo>, String> {
    ocr::models::discover_models()
}

#[tauri::command]
async fn unload_ocr_models() -> Result<(), String> {
    // TODO: Implement model unloading
    Ok(())
}

#[tauri::command]
async fn save_ocr_to_pdf(
    pdf_path: String,
    ocr_data: Vec<ocr::PageTextBlocks>,
    output_path: String,
) -> Result<String, String> {
    ocr::pdf_embed::embed_text_layer(&pdf_path, ocr_data, &output_path)
}

// ==================== App Setup ====================

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
            pdf_engine::get_page_text_with_coords,
            // OCR
            ocr_document,
            cancel_ocr,
            list_ocr_languages,
            unload_ocr_models,
            save_ocr_to_pdf
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
