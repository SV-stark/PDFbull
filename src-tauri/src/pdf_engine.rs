use pdfium_render::prelude::*;
use std::sync::{Arc, Mutex, OnceLock};
use tauri::{Manager, State};

// Wrapper for Pdfium to make it Sync (it is Send but not Sync)
struct PdfiumWrapper(pub Pdfium);
unsafe impl Sync for PdfiumWrapper {}
unsafe impl Send for PdfiumWrapper {}

// Global singleton for the PDFium library interface
// We store the Result inside the OnceLock to avoid panicking during initialization.
static PDFIUM: OnceLock<Result<PdfiumWrapper, String>> = OnceLock::new();

fn get_pdfium(app: &tauri::AppHandle) -> Result<&'static Pdfium, String> {
    let result = PDFIUM.get_or_init(|| {
        println!("[PDF Engine] Initializing PDFium...");
        
        // 1. Try to find the library in the resource directory (for production/installed app)
        let resource_path = app.path().resolve("pdfium.dll", tauri::path::BaseDirectory::Resource);
        
        let path_result = if let Ok(path) = resource_path {
            println!("[PDF Engine] Resolved resource path: {:?}", path);
            Pdfium::bind_to_library(path.to_string_lossy().to_string())
                .map_err(|e| format!("Resource bind error: {:?}", e))
        } else {
            Err("Failed to resolve resource path".to_string())
        };
        
        // 2. Fallback to current directory (for development)
        let path_result = path_result.or_else(|_| {
            println!("[PDF Engine] Falling back to current directory...");
            Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path("./"))
                .map_err(|e| format!("Current dir bind error: {:?}", e))
        });

        // 3. Last fallback to system library
        let path_result = path_result.or_else(|_| {
            Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name())
                .map_err(|e| format!("System library bind error: {:?}", e))
        });
            
        match path_result {
            Ok(bindings) => {
                println!("[PDF Engine] PDFium library successfully bound.");
                Ok(PdfiumWrapper(Pdfium::new(bindings)))
            }
            Err(e) => {
                let err_msg = format!("PDFium library load failed: {}. Ensure pdfium.dll is in resources or root.", e);
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
pub fn with_doc<F, R>(state: &State<'_, PdfState>, f: F) -> Result<R, String>
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

// Helper to get mutable doc ref
pub fn with_mut_doc<F, R>(state: &State<'_, PdfState>, f: F) -> Result<R, String>
where
    F: FnOnce(&mut PdfDocument<'static>) -> Result<R, String>,
{
    let mut guard = state.doc.lock().map_err(|e| e.to_string())?;
    if let Some(wrapper) = guard.as_mut() {
        f(&mut wrapper.0)
    } else {
        Err("No document open".to_string())
    }
}

#[tauri::command]
pub async fn open_document(app: tauri::AppHandle, state: State<'_, PdfState>, path: String) -> Result<i32, String> {
    println!("[PDF Engine] Attempting to open document at: {}", path);
    let pdfium = get_pdfium(&app)?;
    
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
    state: tauri::State<PdfState>,
    page_num: i32,
    query: String,
) -> Result<Vec<(f32, f32, f32, f32)>, String> {
    with_doc(&state, |doc| {
        let page = doc.pages().get(page_num as u16).map_err(|e| e.to_string())?;
        let text = page.text().map_err(|e| e.to_string())?;
        
        // Search for all occurrences of the query
        let search = text.search(&query, &PdfSearchOptions::default()).map_err(|e| e.to_string())?;
        let mut results = Vec::new();
        
        // Fix: search returns a PdfPageTextSearch object which is not an iterator itself
        // but has methods like .results() or checks.
        // Or if it implements IntoIterator, we need to correct usage.
        // Checking pdfium-render docs/examples, usually .iter() or similar is used.
        // Assuming search.results() returns the iterator or vector.
        // If not available, let's try assuming it behaves like an iterator
        // Error was: `PdfPageTextSearch<'_>` is not an iterator
        
        for match_result in search.iter(PdfSearchDirection::SearchForward) {
             for segment in match_result.iter() {
                 let rect = segment.bounds();
                 results.push((
                     rect.left().value,
                     rect.top().value,
                     rect.width().value,
                     rect.height().value,
                 ));
             }
        }

        Ok(results)
    })
}

#[derive(serde::Serialize)]
pub struct RenderedPage {
    pub width: i32,
    pub height: i32,
    pub data: Vec<u8>,
}

#[tauri::command]
pub fn render_page(
    state: tauri::State<PdfState>,
    page_num: i32,
    scale: f32,
) -> Result<RenderedPage, String> {
    with_doc(&state, |doc| {
        let page = doc.pages().get(page_num as u16).map_err(|e| e.to_string())?;
        let width = (page.width().value * scale) as i32;
        let height = (page.height().value * scale) as i32;
        let bitmap = page.render(width, height, None).map_err(|e| e.to_string())?;
        
        // Return raw RGBA pixels
        Ok(RenderedPage {
            width,
            height,
            data: bitmap.as_raw_bytes().to_vec(),
        })
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
pub fn test_pdfium(app: tauri::AppHandle) -> Result<String, String> {
    println!("[PDF Engine] Diagnostic test_pdfium triggered.");
    get_pdfium(&app)?;
    Ok("PDFium library loaded successfully!".to_string())
}

#[tauri::command]
pub fn ping() -> String {
    "pong".to_string()
}
