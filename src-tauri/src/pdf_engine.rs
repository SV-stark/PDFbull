use pdfium_render::prelude::*;
use std::sync::{Arc, Mutex, OnceLock};
use tauri::State;

// Wrapper for Pdfium to make it Sync (it is Send but not Sync)
struct PdfiumWrapper(pub Pdfium);
unsafe impl Sync for PdfiumWrapper {}
unsafe impl Send for PdfiumWrapper {}

// Global singleton for the PDFium library interface
// We store the Result inside the OnceLock to avoid panicking during initialization.
static PDFIUM: OnceLock<Result<PdfiumWrapper, String>> = OnceLock::new();

fn get_pdfium() -> Result<&'static Pdfium, String> {
    let result = PDFIUM.get_or_init(|| {
        println!("[PDF Engine] Initializing PDFium...");
        // Log current directory to help user verify DLL placement
        if let Ok(cwd) = std::env::current_dir() {
            println!("[PDF Engine] Current working directory: {:?}", cwd);
        }
        
        let path_result = Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path("./"))
            .or_else(|_| Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name()));
            
        match path_result {
            Ok(bindings) => {
                println!("[PDF Engine] PDFium library successfully bound.");
                Ok(PdfiumWrapper(Pdfium::new(bindings)))
            }
            Err(e) => {
                let err_msg = format!("PDFium library load failed: {}. Ensure pdfium.dll/libpdfium.so is in the correct directory.", e);
                eprintln!("[PDF Engine] {}", err_msg);
                Err(err_msg)
            }
        }
    });

    match result {
        Ok(wrapper) => Ok(&wrapper.0),
        Err(e) => Err(e.clone()),
    }
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
    println!("[PDF Engine] Attempting to open document at: {}", path);
    let pdfium = get_pdfium()?;
    
    // Performance: Use load_pdf_from_file for memory mapping
    let doc = pdfium.load_pdf_from_file(&path, None)
        .map_err(|e| {
            let err = format!("Failed to open PDF: {}", e);
            eprintln!("[PDF Engine] {}", err);
            err
        })?;
    
    let page_count = doc.pages().len();
    println!("[PDF Engine] Document opened. Page count: {}", page_count);
    
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
        let bitmap = page.render(width, height, None).map_err(|e| e.to_string())?;
        let mut buf = Vec::new();
        let img = bitmap.as_image(); 
        use std::io::Cursor;
        img.write_to(&mut Cursor::new(&mut buf), image::ImageFormat::Png)
            .map_err(|e| e.to_string())?;
        Ok(buf)
    })
}

#[tauri::command]
pub fn get_page_dimensions(state: tauri::State<PdfState>) -> Result<Vec<(i32, i32)>, String> {
    with_doc(&state, |doc| {
        let mut dimensions = Vec::new();
        for page in doc.pages().iter() {
            let width = page.width().value as i32;
            let height = page.height().value as i32;
            dimensions.push((width, height));
        }
        Ok(dimensions)
    })
}

#[tauri::command]
pub fn test_pdfium() -> Result<String, String> {
    println!("[PDF Engine] Diagnostic test_pdfium triggered.");
    get_pdfium()?;
    Ok("PDFium library loaded successfully!".to_string())
}

#[tauri::command]
pub fn ping() -> String {
    "pong".to_string()
}
