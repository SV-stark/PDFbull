use crate::pdf_engine::PdfState;

#[tauri::command]
pub fn get_form_fields(
    _state: tauri::State<PdfState>,
    _page_num: i32,
) -> Result<Vec<String>, String> {
    // Stubbed for migration
    Ok(vec![])
}
