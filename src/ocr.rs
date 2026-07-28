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
