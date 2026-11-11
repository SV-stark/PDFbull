use serde::{Deserialize, Serialize};

/// Recognized word with bounding box in PDF user-space coordinates [x0, y0, x1, y1].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OcrWord {
    pub text: String,
    /// Bounding box in PDF user-space points (y-up coordinate space): [x0, y0, x1, y1]
    pub bbox: [f32; 4],
}

/// Recognized text line containing individual words and overall line bounding box.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OcrLine {
    pub text: String,
    pub words: Vec<OcrWord>,
    /// Bounding box in PDF user-space points: [x0, y0, x1, y1]
    pub bbox: [f32; 4],
}

/// Complete OCR extraction payload for a single PDF page.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct OcrPageResult {
    pub page_num: usize,
    pub lines: Vec<OcrLine>,
}

/// Embedded ONNX models parsed via `rten-onnx` in pure Rust.
pub static EMBEDDED_DET_MODEL: &[u8] = include_bytes!("../models/PP-OCRv6_small_det.onnx");
pub static EMBEDDED_REC_MODEL: &[u8] = include_bytes!("../models/PP-OCRv6_small_rec.onnx");
pub static EMBEDDED_DICT: &str = include_str!("../models/ppocrv6_dict.txt");

/// Target script/language family for OCR recognition.
///
/// With PP-OCRv6, a single unified multilingual model supports 50+ languages
/// simultaneously without requiring separate models per language.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, Default)]
pub enum OcrScript {
    #[default]
    Multilingual,
    Latin,
    Devanagari,
}

impl OcrScript {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Multilingual => "Multilingual (PP-OCRv6: 50+ languages)",
            Self::Latin => "Latin / English",
            Self::Devanagari => "Devanagari (Hindi, Marathi, Sanskrit)",
        }
    }

    /// Model filename for recognition
    pub fn rec_model_filename(&self) -> &'static str {
        "PP-OCRv6_small_rec.onnx"
    }

    pub fn det_model_filename(&self) -> &'static str {
        "PP-OCRv6_small_det.onnx"
    }

    pub fn dict_filename(&self) -> &'static str {
        "ppocrv6_dict.txt"
    }

    /// Resolve model directory path from executable location, local workspace, or `AppData` storage
    pub fn resolve_model_dir() -> std::path::PathBuf {
        let exe_dir = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.join("models")))
            .unwrap_or_else(|| std::path::PathBuf::from("models"));
        if exe_dir.exists() {
            return exe_dir;
        }

        let config_models = crate::storage::get_config_dir().join("models");
        if config_models.exists() {
            return config_models;
        }

        if let Some(proj_dirs) = directories::ProjectDirs::from("", "SV-stark", "PDFbull") {
            let data_dir = proj_dirs.data_dir().join("models");
            if data_dir.exists() {
                return data_dir;
            }
        }

        std::path::PathBuf::from("models")
    }

    /// Full file path to detection model
    pub fn resolve_det_model_path(&self) -> std::path::PathBuf {
        Self::resolve_model_dir().join(self.det_model_filename())
    }

    /// Full file path to recognition model
    pub fn resolve_rec_model_path(&self) -> std::path::PathBuf {
        Self::resolve_model_dir().join(self.rec_model_filename())
    }

    /// Check if models are available (always true because models are embedded in pure Rust,
    /// with optional manual override from the models directory).
    pub fn is_model_available(&self) -> bool {
        true
    }
}

/// Load the detection model: prefers manual models in the app models directory if present,
/// falling back to the embedded ONNX model parsed via `rten-onnx`.
pub fn load_detection_model() -> Result<rten::Model, String> {
    let model_dir = OcrScript::resolve_model_dir();
    for name in &[
        "PP-OCRv6_small_det.onnx",
        "PP-OCRv6_det.onnx",
        "text-detection.onnx",
        "text-detection.rten",
    ] {
        let path = model_dir.join(name);
        if path.exists()
            && let Ok(m) = rten::Model::load_file(&path)
        {
            return Ok(m);
        }
    }
    rten::Model::load_static_slice(EMBEDDED_DET_MODEL)
        .map_err(|e| format!("Failed to load embedded detection model: {e}"))
}

/// Load the recognition model: prefers manual models in the app models directory if present,
/// falling back to the embedded ONNX model parsed via `rten-onnx`.
pub fn load_recognition_model() -> Result<rten::Model, String> {
    let model_dir = OcrScript::resolve_model_dir();
    for name in &[
        "PP-OCRv6_small_rec.onnx",
        "PP-OCRv6_rec.onnx",
        "text-recognition.onnx",
        "text-recognition.rten",
    ] {
        let path = model_dir.join(name);
        if path.exists()
            && let Ok(m) = rten::Model::load_file(&path)
        {
            return Ok(m);
        }
    }
    rten::Model::load_static_slice(EMBEDDED_REC_MODEL)
        .map_err(|e| format!("Failed to load embedded recognition model: {e}"))
}

/// Load the multilingual dictionary / alphabet for text recognition.
pub fn load_alphabet() -> Option<String> {
    let model_dir = OcrScript::resolve_model_dir();
    for name in &["ppocrv6_dict.txt", "ppocr_keys_v1.txt", "dict.txt"] {
        let path = model_dir.join(name);
        if path.exists()
            && let Ok(content) = std::fs::read_to_string(&path)
        {
            return Some(content);
        }
    }
    Some(EMBEDDED_DICT.to_string())
}

impl OcrPageResult {
    pub fn new(page_num: usize, lines: Vec<OcrLine>) -> Self {
        Self { page_num, lines }
    }

    /// Reconstruct full plain text from extracted OCR lines.
    pub fn full_text(&self) -> String {
        let est_cap = self.lines.iter().map(|l| l.text.len() + 1).sum();
        let mut res = String::with_capacity(est_cap);
        for (i, line) in self.lines.iter().enumerate() {
            if i > 0 {
                res.push('\n');
            }
            res.push_str(&line.text);
        }
        res
    }

    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }
}
