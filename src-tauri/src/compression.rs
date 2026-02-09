use crate::pdf_engine::{with_doc, PdfState};

#[tauri::command]
pub fn compress_pdf(state: tauri::State<PdfState>, output_path: String) -> Result<(), String> {
    with_doc(&state, |doc| {
        // In pdfium-render 0.8, save_to_file only takes 1 argument.
        // Full rewrite (which often compresses) is the default for save_to_file
        // if no incremental save is performed previously.
        doc.save_to_file(&output_path)
            .map_err(|e| format!("Compression failed: {}", e))
    })
}
