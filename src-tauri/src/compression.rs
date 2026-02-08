use crate::pdf_engine::PdfState;

#[tauri::command]
pub fn compress_pdf(state: tauri::State<PdfState>, output_path: String) -> Result<(), String> {
    let mut guard = state.doc.lock().unwrap();
    if let Some(doc) = guard.as_mut() {
        // Configure save options for max compression
        // mupdf save methods usually accept options string or struct
        // doc.save(output_path, "garbage=compact,compress") 
        
        doc.save(&output_path, "garbage=compact,compress").map_err(|e| e.to_string())?;
        Ok(())
    } else {
        Err("No document open".to_string())
    }
}
