pub mod models;
pub mod pdf_embed;

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
// oar_ocr types are accessed via full paths
use tauri::Emitter;

/// Text block with bounding box coordinates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextBlock {
    pub text: String,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub confidence: f32,
}

/// OCR results for a single page
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageTextBlocks {
    pub page_number: usize,
    pub blocks: Vec<TextBlock>,
}

/// Global OCR engine state with lazy loading
pub struct OcrEngine {
    engine: Option<oar_ocr::oarocr::OAROCR>,
    current_language: Option<String>,
    cancel_flag: Arc<AtomicBool>,
}

impl OcrEngine {
    pub fn new() -> Self {
        Self {
            engine: None,
            current_language: None,
            cancel_flag: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Ensure models are loaded for the given language
    fn ensure_loaded(&mut self, language: &str) -> Result<(), String> {
        if self.engine.is_some() && self.current_language.as_deref() == Some(language) {
            return Ok(());
        }

        let models = models::discover_models()?;
        let model_info = models
            .iter()
            .find(|m| m.code == language)
            .ok_or_else(|| format!("Language model not found: {}", language))?;

        let engine = oar_ocr::oarocr::OAROCRBuilder::new(
            model_info.det_model_path.to_string_lossy().to_string(),
            model_info.rec_model_path.to_string_lossy().to_string(),
            model_info.keys_path.to_string_lossy().to_string(),
        )
        .build()
        .map_err(|e| format!("Failed to initialize OCR engine: {}", e))?;

        self.engine = Some(engine);
        self.current_language = Some(language.to_string());
        Ok(())
    }

    /// Run OCR on multiple pages with progress reporting
    pub fn run_ocr(
        &mut self,
        pages: Vec<Vec<u8>>,
        language: &str,
        window: tauri::Window,
    ) -> Result<Vec<PageTextBlocks>, String> {
        // Ensure models are loaded
        self.ensure_loaded(language)?;
        let engine = self.engine.as_ref().unwrap();

        // Reset cancel flag
        self.cancel_flag.store(false, Ordering::SeqCst);

        let total_pages = pages.len();
        let mut results = Vec::new();

        for (index, page_data) in pages.iter().enumerate() {
            // Check for cancellation
            if self.cancel_flag.load(Ordering::SeqCst) {
                return Err("OCR cancelled by user".to_string());
            }

            let current = index + 1;
            let percentage = ((current as f32 / total_pages as f32) * 100.0) as u32;

            // Emit progress event
            let _ = window
                .emit(
                    "ocr-progress",
                    serde_json::json!({
                        "current": current,
                        "total": total_pages,
                        "percentage": percentage
                    }),
                )
                .map_err(|e| e.to_string())?;

            // Convert bytes to image
            let img = image::load_from_memory(page_data)
                .map_err(|e| format!("Failed to load image for page {}: {}", current, e))?;

            // Run OCR - Convert to RGB8 as required by oar-ocr
            let ocr_results = engine
                .predict(vec![img.to_rgb8()])
                .map_err(|e| format!("OCR error on page {}: {}", current, e))?;

            let mut blocks = Vec::new();

            // engine.predict returns Vec<OAROCRResult>, one for each image
            if let Some(page_result) = ocr_results.first() {
                // User specified 'regions' field
                for region in &page_result.text_regions {
                    if let Some(text) = &region.text {
                        let mut min_x = f32::MAX;
                        let mut min_y = f32::MAX;
                        let mut max_x = f32::MIN;
                        let mut max_y = f32::MIN;

                        for point in &region.bounding_box.points {
                            let (px, py) = (point.x as f32, point.y as f32);
                            if px < min_x {
                                min_x = px;
                            }
                            if py < min_y {
                                min_y = py;
                            }
                            if px > max_x {
                                max_x = px;
                            }
                            if py > max_y {
                                max_y = py;
                            }
                        }

                        if min_x != f32::MAX {
                            blocks.push(TextBlock {
                                text: text.to_string(),
                                x: min_x,
                                y: min_y,
                                width: max_x - min_x,
                                height: max_y - min_y,
                                confidence: region.confidence.unwrap_or(0.0),
                            });
                        }
                    }
                }
            }

            results.push(PageTextBlocks {
                page_number: current,
                blocks,
            });
        }

        Ok(results)
    }

    /// Cancel ongoing OCR operation
    pub fn cancel(&self) {
        self.cancel_flag.store(true, Ordering::SeqCst);
    }

    /// Unload models to free memory
    pub fn unload(&mut self) {
        self.engine = None;
        self.current_language = None;
    }
}

/// Global OCR engine instance
static mut OCR_ENGINE: Option<Arc<Mutex<OcrEngine>>> = None;

/// Get or initialize the global OCR engine
pub fn get_ocr_engine() -> Arc<Mutex<OcrEngine>> {
    unsafe {
        OCR_ENGINE
            .get_or_insert_with(|| Arc::new(Mutex::new(OcrEngine::new())))
            .clone()
    }
}
