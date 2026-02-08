// ... existing imports
use mupdf::{PdfDocument, Matrix, Colorspace, Rect};

// ... PdfState struct and impl ...

// ... open_document ...

#[tauri::command]
pub fn get_page_text(state: tauri::State<PdfState>, page_num: i32) -> Result<String, String> {
    let guard = state.doc.lock().unwrap();
    if let Some(doc) = guard.as_ref() {
        let page = doc.load_page(page_num).map_err(|e| e.to_string())?;
        // stext returns structured text, block_text returns plain text
        let text = page.to_text().map_err(|e| e.to_string())?; 
        Ok(text.as_text().unwrap_or_default())
    } else {
        Err("No document open".to_string())
    }
}

#[tauri::command]
pub fn search_text(state: tauri::State<PdfState>, page_num: i32, query: String) -> Result<Vec<(f32, f32, f32, f32)>, String> {
    let guard = state.doc.lock().unwrap();
    if let Some(doc) = guard.as_ref() {
        let page = doc.load_page(page_num).map_err(|e| e.to_string())?;
        let hits = page.search(&query).map_err(|e| e.to_string())?;
        
        // Convert Rects to Tuple for easy serialization
        let rects = hits.iter().map(|r| (r.x0, r.y0, r.x1, r.y1)).collect();
        Ok(rects)
    } else {
        Err("No document open".to_string())
    }
}

#[tauri::command]
pub fn render_page(state: tauri::State<PdfState>, page_num: i32, scale: f32) -> Result<Vec<u8>, String> {
    let mut guard = state.doc.lock().unwrap();
    if let Some(doc) = guard.as_mut() {
        let page = doc.load_page(page_num).map_err(|e| e.to_string())?;
        
        // Calculate matrix for scaling
        let matrix = Matrix::new_scale(scale, scale);
        
        // Render to pixmap with anti-aliasing
        let pixmap = page.to_pixmap(&matrix, &Colorspace::device_rgb(), false)
            .map_err(|e| e.to_string())?;
            
        // Get raw samples and dimensions
        let samples = pixmap.samples();
        let width = pixmap.width() as u32;
        let height = pixmap.height() as u32;
        
        // Use image crate to encode as PNG efficiently
        // Direct byte transfer is faster than base64 encoding
        use image::{ImageBuffer, Rgb};
        use std::io::Cursor;
        
        let img: ImageBuffer<Rgb<u8>, &[u8]> = ImageBuffer::from_raw(width, height, samples)
            .ok_or("Failed to create image buffer")?;
            
        let mut buf = Vec::new();
        // Use Fast encoding if possible, but Png defaults are usually balanced
        img.write_to(&mut Cursor::new(&mut buf), image::ImageOutputFormat::Png)
           .map_err(|e| e.to_string())?;
           
        Ok(buf)
    } else {
        Err("No document open".to_string())
    }
}
