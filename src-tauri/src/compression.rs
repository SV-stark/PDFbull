use crate::pdf_engine::PdfState;

#[tauri::command]
pub fn compress_pdf(_state: tauri::State<PdfState>, _output_path: String) -> Result<(), String> {
    // Stubbed for migration
    Err("Compression temporarily disabled during PDFium migration".to_string())
}
