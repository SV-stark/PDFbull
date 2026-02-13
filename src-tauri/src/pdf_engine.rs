use pdfium_render::prelude::*;
use std::sync::{Arc, Mutex, OnceLock};
use std::collections::HashMap;
use tauri::{Manager, State};
use rayon::prelude::*;
use image::GenericImageView;

struct PdfiumWrapper(pub Pdfium);
unsafe impl Sync for PdfiumWrapper {}
unsafe impl Send for PdfiumWrapper {}

static PDFIUM: OnceLock<Result<PdfiumWrapper, String>> = OnceLock::new();

fn get_pdfium(app: &tauri::AppHandle) -> Result<&'static Pdfium, String> {
    let result = PDFIUM.get_or_init(|| {
        println!("[PDF Engine] Initializing PDFium...");
        
        let resource_path = app.path().resolve("pdfium.dll", tauri::path::BaseDirectory::Resource);
        
        let path_result = if let Ok(path) = resource_path {
            println!("[PDF Engine] Resolved resource path: {:?}", path);
            Pdfium::bind_to_library(path.to_string_lossy().to_string())
                .map_err(|e| format!("Resource bind error: {:?}", e))
        } else {
            Err("Failed to resolve resource path".to_string())
        };
        
        let path_result = path_result.or_else(|_| {
            println!("[PDF Engine] Falling back to current directory...");
            Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path("./"))
                .map_err(|e| format!("Current dir bind error: {:?}", e))
        });

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

pub struct PdfWrapper(pub PdfDocument<'static>);

unsafe impl Send for PdfWrapper {}
unsafe impl Sync for PdfWrapper {}

#[derive(Clone)]
pub struct PdfState {
    pub docs: Arc<Mutex<HashMap<String, PdfWrapper>>>,
    pub active_doc: Arc<Mutex<Option<String>>>,
}

impl PdfState {
    pub fn new() -> Self {
        Self {
            docs: Arc::new(Mutex::new(HashMap::new())),
            active_doc: Arc::new(Mutex::new(None)),
        }
    }
    
    pub fn set_active(&self, path: &str) -> Result<(), String> {
        let mut active = self.active_doc.lock().map_err(|e| e.to_string())?;
        *active = Some(path.to_string());
        Ok(())
    }
    
    pub fn get_active_path(&self) -> Result<String, String> {
        let active = self.active_doc.lock().map_err(|e| e.to_string())?;
        active.clone().ok_or_else(|| "No active document".to_string())
    }
}

pub fn with_doc<F, R>(state: &State<'_, PdfState>, f: F) -> Result<R, String>
where
    F: FnOnce(&PdfDocument<'static>) -> Result<R, String>,
{
    let active = state.active_doc.lock().map_err(|e| e.to_string())?;
    let path = active.as_ref().ok_or_else(|| "No active document".to_string())?;
    
    let docs = state.docs.lock().map_err(|e| e.to_string())?;
    if let Some(wrapper) = docs.get(path) {
        f(&wrapper.0)
    } else {
        Err("Document not found".to_string())
    }
}

pub fn with_doc_by_path<F, R>(state: &State<'_, PdfState>, path: &str, f: F) -> Result<R, String>
where
    F: FnOnce(&PdfDocument<'static>) -> Result<R, String>,
{
    let docs = state.docs.lock().map_err(|e| e.to_string())?;
    if let Some(wrapper) = docs.get(path) {
        f(&wrapper.0)
    } else {
        Err("Document not found".to_string())
    }
}

pub fn with_mut_doc<F, R>(state: &State<'_, PdfState>, f: F) -> Result<R, String>
where
    F: FnOnce(&mut PdfDocument<'static>) -> Result<R, String>,
{
    let active = state.active_doc.lock().map_err(|e| e.to_string())?;
    let path = active.as_ref().ok_or_else(|| "No active document".to_string())?;
    
    let mut docs = state.docs.lock().map_err(|e| e.to_string())?;
    if let Some(wrapper) = docs.get_mut(path) {
        f(&mut wrapper.0)
    } else {
        Err("Document not found".to_string())
    }
}

#[tauri::command]
pub async fn open_document(app: tauri::AppHandle, state: State<'_, PdfState>, path: String) -> Result<i32, String> {
    println!("[PDF Engine] Attempting to open document at: {}", path);
    let state_docs = state.docs.clone();
    let state_active = state.active_doc.clone();
    let path_clone = path.clone();
    
    tokio::task::spawn_blocking(move || {
        let pdfium = get_pdfium(&app)?;
        
        let doc = pdfium.load_pdf_from_file(&path_clone, None)
            .map_err(|e| {
                let err = format!("Failed to open PDF: {}", e);
                eprintln!("[PDF Engine] {}", err);
                err
            })?;
        
        let page_count = doc.pages().len();
        println!("[PDF Engine] Document opened. Page count: {}", page_count);
        
        let mut docs = state_docs.lock().map_err(|e| e.to_string())?;
        docs.insert(path_clone.clone(), PdfWrapper(doc));
        
        drop(docs);
        
        let mut active = state_active.lock().map_err(|e| e.to_string())?;
        *active = Some(path_clone);
        
        Ok(page_count as i32)
    }).await.map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn close_document(state: State<'_, PdfState>, path: String) -> Result<(), String> {
    let mut docs = state.docs.lock().map_err(|e| e.to_string())?;
    docs.remove(&path);
    drop(docs);
    
    let mut active = state.active_doc.lock().map_err(|e| e.to_string())?;
    if active.as_ref() == Some(&path) {
        *active = None;
    }
    Ok(())
}

#[tauri::command]
pub async fn set_active_document(state: State<'_, PdfState>, path: String) -> Result<(), String> {
    let docs = state.docs.lock().map_err(|e| e.to_string())?;
    if !docs.contains_key(&path) {
        return Err("Document not found".to_string());
    }
    drop(docs);
    
    let mut active = state.active_doc.lock().map_err(|e| e.to_string())?;
    *active = Some(path);
    Ok(())
}

#[tauri::command]
pub async fn get_page_count(state: State<'_, PdfState>) -> Result<i32, String> {
    let active = state.active_doc.lock().map_err(|e| e.to_string())?.clone();
    let path = active.ok_or_else(|| "No active document".to_string())?;
    let docs = state.docs.lock().map_err(|e| e.to_string())?;
    if let Some(wrapper) = docs.get(&path) {
        Ok(wrapper.0.pages().len() as i32)
    } else {
        Err("Document not found".to_string())
    }
}

#[tauri::command]
pub fn load_document_from_bytes(_data: Vec<u8>) -> Result<(), String> {
    Err("load_document_from_bytes disabled for performance reasons in this version".to_string())
}

#[tauri::command]
pub async fn get_page_text(state: State<'_, PdfState>, page_num: i32) -> Result<String, String> {
    let active = state.active_doc.lock().map_err(|e| e.to_string())?.clone();
    let path = active.ok_or_else(|| "No active document".to_string())?;
    let docs = state.docs.lock().map_err(|e| e.to_string())?;
    if let Some(wrapper) = docs.get(&path) {
        let page = wrapper.0.pages().get(page_num as u16).map_err(|e| e.to_string())?;
        let text = page.text().map_err(|e| e.to_string())?;
        Ok(text.all())
    } else {
        Err("Document not found".to_string())
    }
}

#[tauri::command]
pub fn search_text(
    state: tauri::State<PdfState>,
    page_num: i32,
    query: String,
) -> Result<Vec<(f32, f32, f32, f32)>, String> {
    let active = state.active_doc.lock().map_err(|e| e.to_string())?.clone();
    let path = active.ok_or_else(|| "No active document".to_string())?;
    let docs = state.docs.lock().map_err(|e| e.to_string())?;
    
    if let Some(wrapper) = docs.get(&path) {
        let page = wrapper.0.pages().get(page_num as u16).map_err(|e| e.to_string())?;
        let text = page.text().map_err(|e| e.to_string())?;
        
        let search = text.search(&query, &PdfSearchOptions::default()).map_err(|e| e.to_string())?;
        let mut results = Vec::new();
        
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
    } else {
        Err("Document not found".to_string())
    }
}

#[derive(serde::Serialize)]
pub struct SearchResult {
    page: i32,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

#[tauri::command]
pub async fn search_document(
    state: tauri::State<'_, PdfState>,
    query: String
) -> Result<Vec<SearchResult>, String> {
    let active = state.active_doc.lock().map_err(|e| e.to_string())?.clone();
    let path = active.ok_or_else(|| "No active document".to_string())?;
    let docs = state.docs.lock().map_err(|e| e.to_string())?;
    
    let doc = if let Some(wrapper) = docs.get(&path) {
        &wrapper.0
    } else {
        return Err("Document not found".to_string());
    };
    
    let page_count = doc.pages().len();
    
    let results: Vec<SearchResult> = (0..page_count)
        .into_iter()
        .flat_map(|i| {
            let mut page_results = Vec::new();
            if let Ok(page) = doc.pages().get(i) {
                if let Ok(text) = page.text() {
                    if let Ok(search) = text.search(&query, &PdfSearchOptions::default()) {
                        for match_result in search.iter(PdfSearchDirection::SearchForward) {
                            for segment in match_result.iter() {
                                let rect = segment.bounds();
                                page_results.push(SearchResult {
                                    page: i as i32,
                                    x: rect.left().value,
                                    y: rect.top().value,
                                    w: rect.width().value,
                                    h: rect.height().value,
                                });
                            }
                        }
                    }
                }
            }
            page_results
        })
        .collect();

    Ok(results)
}

/// Text rectangle for text layer rendering
#[derive(serde::Serialize)]
pub struct TextRect {
    pub text: String,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

/// Get text with coordinates for a single page (for text selection layer)
#[tauri::command]
pub async fn get_page_text_with_coords(
    state: tauri::State<'_, PdfState>,
    page_num: i32,
) -> Result<Vec<TextRect>, String> {
    let active = state.active_doc.lock().map_err(|e| e.to_string())?.clone();
    let path = active.ok_or_else(|| "No active document".to_string())?;
    let docs = state.docs.lock().map_err(|e| e.to_string())?;
    
    let doc = if let Some(wrapper) = docs.get(&path) {
        &wrapper.0
    } else {
        return Err("Document not found".to_string());
    };
    
    let page = doc.pages().get(page_num as u16).map_err(|e| e.to_string())?;
    let text_page = page.text().map_err(|e| e.to_string())?;
    
    let mut results = Vec::new();
    
    let mut current_word = String::new();
    let mut word_bounds: Option<(f32, f32, f32, f32)> = None;
    
    for char_result in text_page.chars().iter() {
        let c = match char_result.unicode_char() {
            Some(ch) => ch,
            None => continue,
        };
        
        let bounds = match char_result.loose_bounds() {
            Ok(rect) => rect,
            Err(_) => continue,
        };
        
        let x = bounds.left().value;
        let y = bounds.top().value;
        let w = bounds.width().value;
        let h = bounds.height().value;
        
        if c.is_whitespace() || c == '\n' || c == '\r' {
            if !current_word.is_empty() {
                if let Some((min_x, min_y, max_x, max_y)) = word_bounds {
                    results.push(TextRect {
                        text: current_word.clone(),
                        x: min_x,
                        y: min_y,
                        w: max_x - min_x,
                        h: max_y - min_y,
                    });
                }
                current_word.clear();
                word_bounds = None;
            }
            
            if c == ' ' {
                results.push(TextRect {
                    text: " ".to_string(),
                    x,
                    y,
                    w,
                    h,
                });
            }
        } else {
            current_word.push(c);
            
            if let Some((min_x, min_y, max_x, max_y)) = word_bounds {
                word_bounds = Some((
                    min_x.min(x),
                    min_y.min(y),
                    max_x.max(x + w),
                    max_y.max(y + h),
                ));
            } else {
                word_bounds = Some((x, y, x + w, y + h));
            }
        }
    }
    
    if !current_word.is_empty() {
        if let Some((min_x, min_y, max_x, max_y)) = word_bounds {
            results.push(TextRect {
                text: current_word,
                x: min_x,
                y: min_y,
                w: max_x - min_x,
                h: max_y - min_y,
            });
        }
    }
    
    Ok(results)
}

#[tauri::command]
pub async fn render_page(
    state: tauri::State<'_, PdfState>,
    page_num: i32,
    scale: f32,
) -> Result<tauri::ipc::Response, String> {
    let active = state.active_doc.lock().map_err(|e| e.to_string())?.clone();
    let path = active.ok_or_else(|| "No active document".to_string())?;
    let docs = state.docs.lock().map_err(|e| e.to_string())?;
    
    let doc = if let Some(wrapper) = docs.get(&path) {
        &wrapper.0
    } else {
        return Err("Document not found".to_string());
    };
    
    let page = doc.pages().get(page_num as u16).map_err(|e| e.to_string())?;
    let width = (page.width().value * scale) as i32;
    let height = (page.height().value * scale) as i32;
    let bitmap = page.render(width, height, None).map_err(|e| e.to_string())?;
    
    let raw_bytes = bitmap.as_raw_bytes();

    let mut body = Vec::with_capacity(8 + raw_bytes.len());
    body.extend_from_slice(&width.to_be_bytes());
    body.extend_from_slice(&height.to_be_bytes());
    body.extend_from_slice(&raw_bytes);

    Ok(tauri::ipc::Response::new(body))
}

#[tauri::command]
pub fn get_page_dimensions(state: tauri::State<PdfState>) -> Result<Vec<(i32, i32)>, String> {
    let active = state.active_doc.lock().map_err(|e| e.to_string())?.clone();
    let path = active.ok_or_else(|| "No active document".to_string())?;
    let docs = state.docs.lock().map_err(|e| e.to_string())?;
    
    if let Some(wrapper) = docs.get(&path) {
        let mut dimensions = Vec::new();
        for page in wrapper.0.pages().iter() {
            let width = page.width().value as i32;
            let height = page.height().value as i32;
            dimensions.push((width, height));
        }
        Ok(dimensions)
    } else {
        Err("Document not found".to_string())
    }
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
    
    let state_docs = state.docs.clone();
    let state_active = state.active_doc.clone();
    let filter_type_clone = filter_type.clone();
    let doc_path_clone = doc_path.clone();

    tokio::task::spawn_blocking(move || {
        let pdfium = get_pdfium(&app)?;
        let doc = pdfium.load_pdf_from_file(&doc_path_clone, None).map_err(|e| e.to_string())?;
        
        let mut new_doc = pdfium.create_new_pdf().map_err(|e| e.to_string())?;
        
        let page_count = doc.pages().len();
        let chunk_size = 4;

        for chunk_start in (0..page_count).step_by(chunk_size as usize) {
            let end = std::cmp::min(chunk_start + chunk_size, page_count);
            let mut raw_pages = Vec::with_capacity((end - chunk_start) as usize);

            for i in chunk_start..end {
                 let page = doc.pages().get(i).map_err(|e| e.to_string())?;
                 
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

            let processed_results: Vec<Result<_, String>> = raw_pages.into_par_iter()
                .map(|(w_pt, h_pt, w_px, h_px, bytes)| {
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
        
        let temp_path = format!("{}.tmp", doc_path_clone);
        new_doc.save_to_file(&temp_path).map_err(|e| format!("Failed to save temp PDF: {}", e))?;
        
        drop(doc);
        drop(new_doc);
        
        std::fs::rename(&temp_path, &doc_path_clone).map_err(|e| format!("Failed to overwrite file: {}", e))?;
        
        Ok(())
    }).await.map_err(|e| e.to_string())?
}
