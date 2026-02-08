use crate::pdf_engine::PdfState;
use mupdf::{PdfDocument, Rect, AnnotationType};

#[tauri::command]
pub fn create_highlight(state: tauri::State<PdfState>, page_num: i32, rects: Vec<(f32, f32, f32, f32)>) -> Result<(), String> {
    let mut guard = state.doc.lock().unwrap();
    if let Some(doc) = guard.as_mut() {
        let page = doc.load_page(page_num).map_err(|e| e.to_string())?;
        
        for (x0, y0, x1, y1) in rects {
            let rect = Rect::new(x0, y0, x1, y1);
            let mut annot = page.create_annotation(AnnotationType::Highlight)
                .map_err(|e| e.to_string())?;
            annot.set_rect(rect);
            // Color defaults to yellow usually, but can create specific API for color
        }
        
        // Save changes needed? Usually annotations are in-memory until save
        Ok(())
    } else {
        Err("No document open".to_string())
    }
}
