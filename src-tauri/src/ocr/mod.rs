pub mod models;
pub mod pdf_embed;

use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};

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
    // TODO: Add actual paddle-ocr-rs engine instance
    // For now, just track if models are loaded
    models_loaded: Arc<Mutex<bool>>,
    cancel_flag: Arc<AtomicBool>,
}

impl OcrEngine {
    pub fn new() -> Self {
        Self {
            models_loaded: Arc::new(Mutex::new(false)),
            cancel_flag: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Run OCR on multiple pages with progress reporting
    pub fn run_ocr(
        &self,
        pages: Vec<Vec<u8>>,
        language: &str,
        window: tauri::Window,
    ) -> Result<Vec<PageTextBlocks>, String> {
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
            let _ = window.emit("ocr-progress", serde_json::json!({
                "current": current,
                "total": total_pages,
                "percentage": percentage
            }));

            // TODO: Actual OCR processing here
            // For now, return dummy data
            results.push(PageTextBlocks {
                page_number: current,
                blocks: vec![
                    TextBlock {
                        text: format!("Sample text on page {}", current),
                        x: 10.0,
                        y: 10.0,
                        width: 100.0,
                        height: 20.0,
                        confidence: 0.95,
                    }
                ],
            });

            // Simulate processing time
            std::thread::sleep(std::time::Duration::from_millis(100));
        }

        Ok(results)
    }

    /// Cancel ongoing OCR operation
    pub fn cancel(&self) {
        self.cancel_flag.store(true, Ordering::SeqCst);
    }
}

/// Global OCR engine instance
static mut OCR_ENGINE: Option<Arc<Mutex<OcrEngine>>> = None;

/// Get or initialize the global OCR engine
pub fn get_ocr_engine() -> Arc<Mutex<OcrEngine>> {
    unsafe {
        OCR_ENGINE.get_or_insert_with(|| {
            Arc::new(Mutex::new(OcrEngine::new()))
        }).clone()
    }
}
