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

/// Target script/language family for OCR recognition.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum OcrScript {
    #[default]
    Latin,
    Devanagari,
}

impl OcrScript {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Latin => "Latin / English",
            Self::Devanagari => "Devanagari (Hindi, Marathi, Sanskrit)",
        }
    }

    /// Model filename for recognition
    pub fn rec_model_filename(&self) -> &'static str {
        match self {
            Self::Latin => "text-recognition.rten",
            Self::Devanagari => "devanagari_PP-OCRv4_rec.rten",
        }
    }

    pub fn det_model_filename(&self) -> &'static str {
        "text-detection.rten"
    }

    /// Resolve model directory path from executable location, local workspace, or AppData storage
    pub fn resolve_model_dir() -> std::path::PathBuf {
        let exe_dir = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.join("models")))
            .unwrap_or_else(|| std::path::PathBuf::from("models"));
        if exe_dir.exists() {
            return exe_dir;
        }

        if let Some(proj_dirs) = directories::ProjectDirs::from("com", "PDFbull", "PDFbull") {
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

    /// Check if both detection & recognition models exist for this script
    pub fn is_model_available(&self) -> bool {
        self.resolve_det_model_path().exists() && self.resolve_rec_model_path().exists()
    }
}

impl OcrPageResult {
    pub fn new(page_num: usize, lines: Vec<OcrLine>) -> Self {
        Self { page_num, lines }
    }

    /// Reconstruct full plain text from extracted OCR lines.
    pub fn full_text(&self) -> String {
        self.lines
            .iter()
            .map(|l| l.text.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }
}
