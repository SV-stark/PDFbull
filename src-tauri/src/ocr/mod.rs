pub mod models;
pub mod pdf_embed;

use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use tauri::Emitter;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextBlock {
    pub text: String,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageTextBlocks {
    pub page_number: usize,
    pub blocks: Vec<TextBlock>,
}

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

    pub fn run_ocr(
        &mut self,
        pages: Vec<Vec<u8>>,
        language: &str,
        window: tauri::Window,
    ) -> Result<Vec<PageTextBlocks>, String> {
        self.ensure_loaded(language)?;
        let engine = self.engine.as_ref().unwrap();

        self.cancel_flag.store(false, Ordering::SeqCst);

        let total_pages = pages.len();

        let processed_results: Vec<Result<_, String>> = pages
            .into_par_iter()
            .enumerate()
            .map(|(index, page_data)| {
                if self.cancel_flag.load(Ordering::SeqCst) {
                    return Err("OCR cancelled by user".to_string());
                }

                let img = image::load_from_memory(&page_data)
                    .map_err(|e| format!("Failed to load image: {}", e))?;

                let ocr_results = engine
                    .predict(vec![img.to_rgb8()])
                    .map_err(|e| format!("OCR error: {}", e))?;

                let mut blocks = Vec::new();

                if let Some(page_result) = ocr_results.first() {
                    for region in &page_result.text_regions {
                        if let Some(text) = &region.text {
                            let mut min_x = f32::MAX;
                            let mut min_y = f32::MAX;
                            let mut max_x = f32::MIN;
                            let mut max_y = f32::MIN;

                            for point in &region.bounding_box.points {
                                let (px, py) = (point.x as f32, point.y as f32);
                                min_x = min_x.min(px);
                                min_y = min_y.min(py);
                                max_x = max_x.max(px);
                                max_y = max_y.max(py);
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

                Ok(PageTextBlocks {
                    page_number: index + 1,
                    blocks,
                })
            })
            .collect();

        for (index, result) in processed_results.iter().enumerate() {
            if let Ok(_page_result) = result {
                let current = index + 1;
                let percentage = ((current as f32 / total_pages as f32) * 100.0) as u32;
                let _ = window.emit(
                    "ocr-progress",
                    serde_json::json!({
                        "current": current,
                        "total": total_pages,
                        "percentage": percentage
                    }),
                );
            }
        }

        processed_results.into_iter().collect()
    }

    pub fn cancel(&self) {
        self.cancel_flag.store(true, Ordering::SeqCst);
    }

    pub fn unload(&mut self) {
        self.engine = None;
        self.current_language = None;
    }
}

static OCR_ENGINE: OnceLock<Arc<Mutex<OcrEngine>>> = OnceLock::new();

pub fn get_ocr_engine() -> Arc<Mutex<OcrEngine>> {
    OCR_ENGINE
        .get_or_init(|| Arc::new(Mutex::new(OcrEngine::new())))
        .clone()
}
