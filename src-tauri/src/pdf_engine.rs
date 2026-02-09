use pdfium_render::prelude::*;
use std::sync::{Arc, Mutex, OnceLock};
use tauri::{Manager, State};
use rayon::prelude::*;
use image::GenericImageView;


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
    // Clone state to move into blocking task
    let state_clone = state.doc.clone();
    
    tokio::task::spawn_blocking(move || {
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
        
        *state_clone.lock().map_err(|e| e.to_string())? = Some(PdfWrapper(doc));
        
        Ok(page_count as i32)
    }).await.map_err(|e| e.to_string())?
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

#[tauri::command]
pub async fn render_page(
    state: tauri::State<'_, PdfState>,
    page_num: i32,
    scale: f32,
) -> Result<tauri::ipc::Response, String> {
    let doc_clone = state.doc.clone();
    tokio::task::spawn_blocking(move || {
        let guard = doc_clone.lock().map_err(|e| e.to_string())?;
        if let Some(wrapper) = guard.as_ref() {
             let doc = &wrapper.0;
             let page = doc.pages().get(page_num as u16).map_err(|e| e.to_string())?;
             let width = (page.width().value * scale) as i32;
             let height = (page.height().value * scale) as i32;
             let bitmap = page.render(width, height, None).map_err(|e| e.to_string())?;
             
             let raw_bytes = bitmap.as_raw_bytes();

             let mut body = Vec::with_capacity(8 + raw_bytes.len());
             body.extend_from_slice(&width.to_be_bytes());
             body.extend_from_slice(&height.to_be_bytes());
             body.extend_from_slice(raw_bytes);

             Ok(tauri::ipc::Response::new(body))
        } else {
             Err("No document open".to_string())
        }
    }).await.map_err(|e| e.to_string())?
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

fn apply_image_filter(dynamic_image: image::DynamicImage, filter_type: &str, intensity: f32) -> image::DynamicImage {
    match filter_type {
        "grayscale" => dynamic_image.grayscale(),
        "bw" => {
             let gray = dynamic_image.grayscale();
             let threshold = (intensity * 255.0) as u8;
             let (w, h) = gray.dimensions();
             let luma = gray.into_luma8();
             let mut raw = luma.into_raw();
             
             // Optimization: Iterate over raw bytes for auto-vectorization
             raw.iter_mut().for_each(|freq| {
                 *freq = if *freq > threshold { 255 } else { 0 };
             });
             
             let luma = image::ImageBuffer::from_raw(w, h, raw).unwrap();
             image::DynamicImage::ImageLuma8(luma)
        },
        "lighten" => {
            let amount = (intensity * 100.0) as i32;
            dynamic_image.brighten(amount)
        },
        "eco" => {
            let gray = dynamic_image.grayscale();
            let contrast = gray.adjust_contrast(50.0);
            let bright = contrast.brighten(20);
            let (w, h) = bright.dimensions();
            let luma = bright.into_luma8();
            let mut raw = luma.into_raw();

            // Optimization: Iterate over raw bytes
             raw.iter_mut().for_each(|p| {
                 if *p > 200 { *p = 255; }
             });
             
             let luma = image::ImageBuffer::from_raw(w, h, raw).unwrap();
             image::DynamicImage::ImageLuma8(luma)
        },
        "noshadow" => {
             let bright = dynamic_image.brighten(30);
             bright.adjust_contrast(30.0)
        },
        _ => dynamic_image
    }
}

#[tauri::command]
pub async fn apply_scanner_filter(
    app: tauri::AppHandle,
    state: State<'_, PdfState>,
    doc_path: String,
    filter_type: String,
    intensity: f32,
) -> Result<(), String> {
    println!("[PDF Engine] Applying filter: {}, intensity: {}", filter_type, intensity);
    
    let state_doc_clone = state.doc.clone();
    let filter_type_clone = filter_type.clone();
    let doc_path_clone = doc_path.clone();

    tokio::task::spawn_blocking(move || {
        // 1. Load original document
        let pdfium = get_pdfium(&app)?;
        let doc = pdfium.load_pdf_from_file(&doc_path_clone, None).map_err(|e| e.to_string())?;
        
        // 2. Create new document
        let mut new_doc = pdfium.create_new_pdf().map_err(|e| e.to_string())?;
        
        let page_count = doc.pages().len();
        let chunk_size = 4; // Process 4 pages at a time to balance memory and parallelism

        for chunk_start in (0..page_count).step_by(chunk_size as usize) {
            let end = std::cmp::min(chunk_start + chunk_size, page_count);
            let mut raw_pages = Vec::with_capacity((end - chunk_start) as usize);

            // A. Render Chunk (Sequential due to PDFium single-threadedness)
            for i in chunk_start..end {
                 let page = doc.pages().get(i).map_err(|e| e.to_string())?;
                 
                 // Render to high-res image (2.0 scale for quality)
                 let scale = 2.0; 
                 let width_px = (page.width().value * scale) as i32;
                 let height_px = (page.height().value * scale) as i32;
                 
                 let bitmap = page.render(width_px, height_px, None).map_err(|e| e.to_string())?;
                 
                 raw_pages.push((
                     page.width().value,
                     page.height().value,
                     width_px,
                     height_px,
                     bitmap.as_raw_bytes().to_vec()
                 ));
            }

            // B. Process Chunk (Parallel using Rayon)
            // This is the "Most Beneficial Usage": Parallelizing the heavy image processing
            let processed_results: Vec<Result<_, String>> = raw_pages.into_par_iter()
                .map(|(w_pt, h_pt, w_px, h_px, mut bytes)| {
                    let img_buffer = image::ImageBuffer::<image::Rgba<u8>, _>::from_raw(w_px as u32, h_px as u32, bytes)
                        .ok_or_else(|| "Failed to create image buffer".to_string())?;
                    
                    let dynamic_image = image::DynamicImage::ImageRgba8(img_buffer);
                    let processed = apply_image_filter(dynamic_image, &filter_type_clone, intensity);
                    
                    Ok((w_pt, h_pt, processed))
                })
                .collect();

            // C. Add to New Document (Sequential)
            for result in processed_results {
                 let (w_pt, h_pt, processed_img) = result?;
                 
                 let pdf_w = pdfium_render::prelude::PdfPoints::new(w_pt);
                 let pdf_h = pdfium_render::prelude::PdfPoints::new(h_pt);
                 
                 let image_obj = PdfPageImageObject::new_with_width(&new_doc, &processed_img, pdf_w)
                     .map_err(|e| format!("Failed to create image object: {}", e))?;
                     
                 let mut new_page = new_doc.pages_mut().create_page_at_end(pdfium_render::prelude::PdfPagePaperSize::Custom(pdf_w, pdf_h))
                    .map_err(|e| e.to_string())?;
                 
                 new_page.objects_mut().add_image_object(image_obj).map_err(|e| e.to_string())?;
            }
        }
        
        // 3. Save to file
        // 3.1 Close global state doc (if it matches doc_path)
        {
            let mut guard = state_doc_clone.lock().map_err(|e| e.to_string())?;
            *guard = None; 
        }
        
        // 3.2 Save to a temporary file first
        let temp_path = format!("{}.tmp", doc_path_clone);
        new_doc.save_to_file(&temp_path).map_err(|e| format!("Failed to save temp PDF: {}", e))?;
        
        // 3.3 Drop local doc handles to release file locks
        drop(doc);
        drop(new_doc);
        
        // 3.4 Move temp to target
        std::fs::rename(&temp_path, &doc_path_clone).map_err(|e| format!("Failed to overwrite file: {}", e))?;
        
        Ok(())
    }).await.map_err(|e| e.to_string())?
}
