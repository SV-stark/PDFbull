use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Information about an available language model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageInfo {
    pub code: String,
    pub name: String,
    pub det_model_path: PathBuf,
    pub rec_model_path: PathBuf,
    pub keys_path: PathBuf,
}

/// Discover available OCR language models
pub fn discover_models() -> Result<Vec<LanguageInfo>, String> {
    let mut languages = Vec::new();

    // TODO: Add bundled models discovery from resources directory
    // For now, return English as default
    languages.push(LanguageInfo {
        code: "en".to_string(),
        name: "English".to_string(),
        det_model_path: PathBuf::from("resources/ocr_models/en/det.onnx"),
        rec_model_path: PathBuf::from("resources/ocr_models/en/rec.onnx"),
        keys_path: PathBuf::from("resources/ocr_models/en/keys.txt"),
    });

    // TODO: Scan user models directory {APP_DATA}/PDFbull/ocr_models/
    // Example: hi/, ar/, etc.

    Ok(languages)
}
