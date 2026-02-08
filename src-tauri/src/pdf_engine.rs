use micropdf::fitz::colorspace::Colorspace;
use micropdf::fitz::geometry::Matrix;
use micropdf::pdf::document::PdfDocument;
use std::sync::Mutex;

pub struct PdfWrapper(pub PdfDocument);

unsafe impl Send for PdfWrapper {}
unsafe impl Sync for PdfWrapper {}

pub struct PdfState {
    pub doc: Mutex<Option<PdfWrapper>>,
}

impl PdfState {
    pub fn new() -> Self {
        Self {
            doc: Mutex::new(None),
        }
    }
}

#[tauri::command]
pub fn open_document(state: tauri::State<PdfState>, path: String) -> Result<i32, String> {
    let doc = PdfDocument::open(&path).map_err(|e| e.to_string())?;
    // page_count() returns usize, cast to i32
    let page_count = doc.page_count().map_err(|e| e.to_string())? as i32;
    *state.doc.lock().unwrap() = Some(PdfWrapper(doc));
    Ok(page_count)
}

#[tauri::command]
pub fn load_document_from_bytes(
    state: tauri::State<PdfState>,
    file_name: String,
    data: Vec<u8>,
) -> Result<String, String> {
    use std::env;
    use std::io::Write;

    let temp_dir = env::temp_dir();
    let temp_file = temp_dir.join(&file_name);

    let mut file = std::fs::File::create(&temp_file).map_err(|e| e.to_string())?;
    file.write_all(&data).map_err(|e| e.to_string())?;

    let doc = PdfDocument::open(temp_file.to_str().unwrap()).map_err(|e| e.to_string())?;
    let _page_count = doc.page_count().map_err(|e| e.to_string())?;

    let doc_id = format!(
        "doc_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis()
    );

    *state.doc.lock().unwrap() = Some(PdfWrapper(doc));

    Ok(doc_id)
}

#[tauri::command]
pub fn get_page_count(state: tauri::State<PdfState>) -> Result<i32, String> {
    let guard = state.doc.lock().unwrap();
    if let Some(wrapper) = guard.as_ref() {
        wrapper
            .0
            .page_count()
            .map(|c| c as i32)
            .map_err(|e| e.to_string())
    } else {
        Err("No document open".to_string())
    }
}

#[tauri::command]
pub fn get_page_text(state: tauri::State<PdfState>, page_num: i32) -> Result<String, String> {
    let guard = state.doc.lock().unwrap();
    if let Some(wrapper) = guard.as_ref() {
        let doc = &wrapper.0;
        let page = doc.load_page(page_num).map_err(|e| e.to_string())?;
        let text = page.to_text().map_err(|e| e.to_string())?;
        Ok(text.as_text())
    } else {
        Err("No document open".to_string())
    }
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
        let page = doc.load_page(page_num).map_err(|e| e.to_string())?;
        let hits = page.search(&query).map_err(|e| e.to_string())?;

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
        let page = doc.load_page(page_num).map_err(|e| e.to_string())?;

        let matrix = Matrix::scale(scale, scale);

        let pixmap = page
            .to_pixmap(&matrix, &Colorspace::device_rgb(), false)
            .map_err(|e| e.to_string())?;

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
