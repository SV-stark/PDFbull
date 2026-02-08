use crate::pdf_engine::PdfState;
use micropdf::fitz::geometry::Rect;
use micropdf::pdf::annot::AnnotType;

#[tauri::command]
pub fn create_highlight(
    state: tauri::State<PdfState>,
    page_num: i32,
    rects: Vec<(f32, f32, f32, f32)>,
) -> Result<(), String> {
    let mut guard = state.doc.lock().unwrap();
    if let Some(wrapper) = guard.as_mut() {
        let doc = &mut wrapper.0;
        let page = doc.load_page(page_num as i32).map_err(|e| e.to_string())?;
        for (x0, y0, x1, y1) in rects {
            let rect = Rect::new(x0, y0, x1, y1);
            let mut annot = page
                .create_annotation(AnnotType::Highlight)
                .map_err(|e| e.to_string())?;
            annot.set_rect(rect);
            annot.update().map_err(|e| e.to_string())?;
        }

        Ok(())
    } else {
        Err("No document open".to_string())
    }
}
