use crate::pdf_engine::PdfState;

#[tauri::command]
pub fn apply_filter(image_data: String, _filter_type: String) -> Result<String, String> {
    // Return original data (identity)
    Ok(image_data)
}

#[tauri::command]
pub fn auto_crop(_state: tauri::State<PdfState>, _page_num: i32) -> Result<(), String> {
    // Stubbed for migration
    Err("Auto-crop temporarily disabled during PDFium migration".to_string())
}
