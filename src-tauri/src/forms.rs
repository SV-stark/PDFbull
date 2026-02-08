use crate::pdf_engine::PdfState;

#[tauri::command]
pub fn get_form_fields(
    state: tauri::State<PdfState>,
    page_num: i32,
) -> Result<Vec<String>, String> {
    let guard = state.doc.lock().unwrap();
    if let Some(wrapper) = guard.as_ref() {
        let doc = &wrapper.0;
        let _page = doc
            .load_page(page_num)
            .map_err(|e: mupdf::Error| e.to_string())?;

        // Placeholder until mupdf-rs bindings verified
        Ok(vec![])
    } else {
        Err("No document open".to_string())
    }
}
