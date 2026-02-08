use crate::pdf_engine::PdfState;

#[tauri::command]
pub fn get_form_fields(state: tauri::State<PdfState>, page_num: i32) -> Result<Vec<String>, String> {
    let guard = state.doc.lock().unwrap();
    if let Some(doc) = guard.as_ref() {
        let page = doc.load_page(page_num).map_err(|e| e.to_string())?;
        
        // This usually requires iterating widgets
        // mupdf-rs iterator support varies. 
        // Logic:
        // let mut widgets = Vec::new();
        // for widget in page.widgets() {
        //     widgets.push(format!("{:?}", widget));
        // }
        // Ok(widgets)
        
        Ok(vec![]) // Placeholder until mupdf-rs bindings verified
    } else {
        Err("No document open".to_string())
    }
}
