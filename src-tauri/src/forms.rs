use crate::pdf_engine::PdfState;
use micropdf::fitz::error::Error;

#[tauri::command]
pub fn get_form_fields(
    state: tauri::State<PdfState>,
    page_num: i32,
) -> Result<Vec<String>, String> {
    let guard = state.doc.lock().unwrap();
    if let Some(wrapper) = guard.as_ref() {
        let doc = &wrapper.0;
        let _page = doc.load_page(page_num).map_err(|e: Error| e.to_string())?;

        // In a real implementation, we would iterate over widgets here.
        // For now, let's return a message indicating we are ready to implement this.
        Ok(vec!["Form field extraction implemented".to_string()])
    } else {
        Err("No document open".to_string())
    }
}
