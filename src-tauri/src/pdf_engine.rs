use micropdf::ffi::document::{Document, Page};
use micropdf::fitz::colorspace::Colorspace;
use micropdf::fitz::error::Error;
use micropdf::fitz::geometry::Matrix;
use std::fs;
use std::sync::{Arc, Mutex};
use tauri::State;

pub struct PdfWrapper(pub Document);

#[derive(Default)]
pub struct PdfState {
    pub doc: Arc<Mutex<Option<PdfWrapper>>>,
}

#[tauri::command]
pub async fn open_document(state: State<'_, PdfState>, path: String) -> Result<i32, String> {
    let data = fs::read(&path).map_err(|e| e.to_string())?;
    let doc = Document::open_memory(data).map_err(|e| e.to_string())?;
    let page_count = doc.count_pages();
    *state.doc.lock().unwrap() = Some(PdfWrapper(doc));
    Ok(page_count)
}

pub fn load_document_from_bytes(data: Vec<u8>) -> Result<Document, String> {
    Document::open_memory(data).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_page_count(state: State<'_, PdfState>) -> Result<i32, String> {
    let doc_lock = state.doc.lock().unwrap();
    if let Some(wrapper) = doc_lock.as_ref() {
        Ok(wrapper.0.count_pages())
    } else {
        Err("No document opened".to_string())
    }
}

#[tauri::command]
pub async fn extract_page_text(state: State<'_, PdfState>, page_num: i32) -> Result<String, String> {
    let doc_lock = state.doc.lock().unwrap();
    let wrapper = doc_lock.as_ref().ok_or("No document opened")?;
    let page = wrapper.0.load_page(page_num).map_err(|e| e.to_string())?;

    page.extract_text().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn search_text(
    state: tauri::State<PdfState>,
    page_num: i32,
    query: String,
) -> Result<Vec<(f32, f32, f32, f32)>, String> {
    let guard = state.doc.lock().unwrap();
    if let Some(wrapper) = guard.as_ref() {
        let doc = &wrapper.0;
        let page = doc
            .load_page(page_num as i32)
            .map_err(|e: Error| e.to_string())?;
        let hits = page.search_text(&query).map_err(|e: Error| e.to_string())?;

        let rects = hits.iter().map(|r| (r.x0, r.y0, r.x1, r.y1)).collect();
        Ok(rects)
    } else {
        Err("No document open".to_string())
    }
}

#[tauri::command]
pub fn render_page(
    state: tauri::State<PdfState>,
    page_num: i32,
    scale: f32,
) -> Result<Vec<u8>, String> {
    let mut guard = state.doc.lock().unwrap();
    if let Some(wrapper) = guard.as_mut() {
        let doc = &mut wrapper.0;
        let page = doc
            .load_page(page_num as i32)
            .map_err(|e: Error| e.to_string())?;

        let matrix = Matrix::scale(scale, scale);

        let pixmap = page.to_pixmap(&matrix).map_err(|e: Error| e.to_string())?;

        let samples = pixmap.samples();
        let width = pixmap.width() as u32;
        let height = pixmap.height() as u32;

        use image::{ImageBuffer, Rgb};
        use std::io::Cursor;

        let img: ImageBuffer<Rgb<u8>, &[u8]> =
            ImageBuffer::from_raw(width, height, samples).ok_or("Failed to create image buffer")?;

        let mut buf = Vec::new();
        img.write_to(&mut Cursor::new(&mut buf), image::ImageFormat::Png)
            .map_err(|e: image::ImageError| e.to_string())?;

        Ok(buf)
    } else {
        Err("No document open".to_string())
    }
}
