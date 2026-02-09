use pdfium_render::prelude::*;
use std::sync::{Arc, Mutex, OnceLock};
use tauri::State;

// Wrapper for Pdfium to make it Sync (it is Send but not Sync)
struct PdfiumWrapper(pub Pdfium);
unsafe impl Sync for PdfiumWrapper {}
unsafe impl Send for PdfiumWrapper {}

// Global singleton for the PDFium library interface
static PDFIUM: OnceLock<PdfiumWrapper> = OnceLock::new();

fn get_pdfium() -> &'static Pdfium {
    &PDFIUM.get_or_init(|| {
        let pdfium = Pdfium::new(
            Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path("./"))
                .or_else(|_| Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name()))
                .expect("CRITICAL: Failed to load PDFium library. Ensure pdfium.dll/libpdfium.so is present.")
        );
        PdfiumWrapper(pdfium)
    }).0
}

// Wrapper to make PdfDocument Send + Sync relies on Mutex for safety
pub struct PdfWrapper(pub PdfDocument<'static>);

unsafe impl Send for PdfWrapper {}
unsafe impl Sync for PdfWrapper {}

#[derive(Clone)]
pub struct PdfState {
    pub doc: Arc<Mutex<Option<PdfWrapper>>>,
}

impl PdfState {
    pub fn new() -> Self {
        Self {
            doc: Arc::new(Mutex::new(None)),
        }
    }
}

// Helper to get optional doc ref
fn with_doc<F, R>(state: &State<'_, PdfState>, f: F) -> Result<R, String>
where
    F: FnOnce(&PdfDocument<'static>) -> Result<R, String>,
{
    let guard = state.doc.lock().map_err(|e| e.to_string())?;
    if let Some(wrapper) = guard.as_ref() {
        f(&wrapper.0)
    } else {
        Err("No document open".to_string())
    }
}

#[tauri::command]
pub async fn open_document(state: State<'_, PdfState>, path: String) -> Result<i32, String> {
    let pdfium = get_pdfium();
    
    // Performance: Use load_pdf_from_file for memory mapping
    let doc = pdfium.load_pdf_from_file(&path, None)
        .map_err(|e| format!("Failed to open PDF: {}", e))?;
    
    let page_count = doc.pages().len();
    
    *state.doc.lock().map_err(|e| e.to_string())? = Some(PdfWrapper(doc));
    
    Ok(page_count as i32)
}

#[tauri::command]
pub async fn get_page_count(state: State<'_, PdfState>) -> Result<i32, String> {
    with_doc(&state, |doc| Ok(doc.pages().len() as i32))
}

#[tauri::command]
pub fn load_document_from_bytes(_data: Vec<u8>) -> Result<(), String> {
    Err("load_document_from_bytes disabled for performance reasons in this version".to_string())
}

#[tauri::command]
pub async fn get_page_text(state: State<'_, PdfState>, page_num: i32) -> Result<String, String> {
    with_doc(&state, |doc| {
        let page = doc.pages().get(page_num as u16).map_err(|e| e.to_string())?;
        let text = page.text().map_err(|e| e.to_string())?;
        Ok(text.all())
    })
}

#[tauri::command]
pub fn search_text(
    _state: tauri::State<PdfState>,
    _page_num: i32,
    _query: String,
) -> Result<Vec<(f32, f32, f32, f32)>, String> {
    // Stubbed
    Ok(vec![])
}

#[tauri::command]
pub fn render_page(
    state: tauri::State<PdfState>,
    page_num: i32,
    scale: f32,
) -> Result<Vec<u8>, String> {
    with_doc(&state, |doc| {
        let page = doc.pages().get(page_num as u16).map_err(|e| e.to_string())?;
        
        let width = (page.width().value * scale) as i32;
        let height = (page.height().value * scale) as i32;
        
        // Render to bitmap
        // We pass None for config to use defaults (scale to fit)
        let bitmap = page.render(width, height, None).map_err(|e| e.to_string())?;

        // Convert to PNG
        let mut buf = Vec::new();
        let img = bitmap.as_image(); 
        use std::io::Cursor;
        img.write_to(&mut Cursor::new(&mut buf), image::ImageFormat::Png)
            .map_err(|e| e.to_string())?;

        Ok(buf)
    })
}
