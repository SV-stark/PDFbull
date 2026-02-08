mod annotations;
mod compression;
mod filters;
mod forms;
pub mod pdf_engine;

use pdf_engine::PdfState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(PdfState::new())
        .invoke_handler(tauri::generate_handler![
            // PDF Engine
            pdf_engine::open_document,
            pdf_engine::render_page,
            pdf_engine::get_page_text,
            pdf_engine::search_text,
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
