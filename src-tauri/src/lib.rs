mod annotations;
mod compression;
mod filters;
mod forms;
pub mod pdf_engine;

use pdf_engine::PdfState;
use std::fs;

#[tauri::command]
fn save_file(path: String, data: Vec<u8>) -> Result<(), String> {
    fs::write(&path, data).map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
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
            annotations::create_highlight,
            // Filters
            filters::apply_filter,
            filters::auto_crop,
            // Forms
            forms::get_form_fields,
            // Compression
            compression::compress_pdf
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
