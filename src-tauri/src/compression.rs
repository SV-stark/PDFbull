use crate::pdf_engine::PdfState;
use micropdf::pdf::document::PdfDocument;

#[tauri::command]
pub fn compress_pdf(state: tauri::State<PdfState>, output_path: String) -> Result<(), String> {
    let mut guard = state.doc.lock().unwrap();
    if let Some(wrapper) = guard.as_mut() {
        let doc = &mut wrapper.0;
        doc.save(&output_path, "garbage=compact,compress")
            .map_err(|e| e.to_string())?;
        Ok(())
    } else {
        Err("No document open".to_string())
    }
}
