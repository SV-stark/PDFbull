use crate::pdf_engine::PdfState;

#[tauri::command]
pub fn create_highlight(
    _state: tauri::State<PdfState>,
    _page_num: i32,
    _rects: Vec<(f32, f32, f32, f32)>,
) -> Result<(), String> {
    // Stubbed for migration
    Err("Annotations temporarily disabled during PDFium migration".to_string())
}
