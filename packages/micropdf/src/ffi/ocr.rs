//! OCR (Optical Character Recognition) integration
//!
//! This module provides interfaces for OCR engines like Tesseract.
//! The actual OCR implementation requires external libraries.

use std::ffi::{CStr, CString, c_char, c_int};
use std::sync::LazyLock;

use crate::ffi::{Handle, HandleStore};
use crate::fitz::geometry::Rect;
use crate::fitz::pixmap::Pixmap;

// ============================================================================
// Handle Management
// ============================================================================

/// Handle store for OCR engines
static OCR_ENGINES: LazyLock<HandleStore<OcrEngine>> = LazyLock::new(HandleStore::new);

/// Handle store for OCR results
static OCR_RESULTS: LazyLock<HandleStore<OcrResult>> = LazyLock::new(HandleStore::new);

// ============================================================================
// OCR Engine Types
// ============================================================================

/// Supported OCR engines
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OcrEngineType {
    /// No OCR engine (stub/placeholder)
    None = 0,
    /// Tesseract OCR
    Tesseract = 1,
    /// Windows OCR (Windows.Media.Ocr)
    WindowsOcr = 2,
    /// Apple Vision Framework (macOS/iOS)
    AppleVision = 3,
    /// Google Cloud Vision
    GoogleVision = 4,
    /// Amazon Textract
    AmazonTextract = 5,
    /// Azure Computer Vision
    AzureVision = 6,
}

/// OCR page segmentation modes (Tesseract PSM)
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OcrPageSegMode {
    /// Orientation and script detection only
    OsdOnly = 0,
    /// Automatic page segmentation with OSD
    AutoOsd = 1,
    /// Automatic page segmentation, no OSD or OCR
    AutoOnly = 2,
    /// Fully automatic page segmentation, no OSD (default)
    Auto = 3,
    /// Single column of text
    SingleColumn = 4,
    /// Single uniform block of vertically aligned text
    SingleBlockVertText = 5,
    /// Single uniform block of text
    SingleBlock = 6,
    /// Single text line
    SingleLine = 7,
    /// Single word
    SingleWord = 8,
    /// Single word in a circle
    CircleWord = 9,
    /// Single character
    SingleChar = 10,
    /// Sparse text - find as much text as possible
    SparseText = 11,
    /// Sparse text with OSD
    SparseTextOsd = 12,
    /// Raw line - treat image as a single text line
    RawLine = 13,
}

/// OCR engine mode (Tesseract OEM)
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OcrEngineMode {
    /// Legacy engine only
    TesseractOnly = 0,
    /// LSTM neural network only
    LstmOnly = 1,
    /// Legacy + LSTM combined
    TesseractLstmCombined = 2,
    /// Default (currently LSTM)
    Default = 3,
}

// ============================================================================
// OCR Configuration
// ============================================================================

/// OCR configuration
#[derive(Debug, Clone)]
pub struct OcrConfig {
    /// Engine type
    pub engine_type: OcrEngineType,
    /// Language code (e.g., "eng", "fra", "deu")
    pub language: String,
    /// Page segmentation mode
    pub psm: OcrPageSegMode,
    /// Engine mode
    pub oem: OcrEngineMode,
    /// DPI for image preprocessing
    pub dpi: u32,
    /// Enable image preprocessing
    pub preprocess: bool,
    /// Confidence threshold (0-100)
    pub min_confidence: i32,
}

impl Default for OcrConfig {
    fn default() -> Self {
        Self {
            engine_type: OcrEngineType::None,
            language: "eng".to_string(),
            psm: OcrPageSegMode::Auto,
            oem: OcrEngineMode::Default,
            dpi: 300,
            preprocess: true,
            min_confidence: 60,
        }
    }
}

// ============================================================================
// OCR Word
// ============================================================================

/// A recognized word
#[derive(Debug, Clone)]
pub struct OcrWord {
    /// Word text
    pub text: String,
    /// Bounding box
    pub bounds: Rect,
    /// Confidence (0-100)
    pub confidence: i32,
    /// Font name (if detected)
    pub font: Option<String>,
    /// Font size (if detected)
    pub font_size: Option<f32>,
    /// Is bold
    pub bold: bool,
    /// Is italic
    pub italic: bool,
}

impl OcrWord {
    /// Create a new OCR word
    pub fn new(text: impl Into<String>, bounds: Rect, confidence: i32) -> Self {
        Self {
            text: text.into(),
            bounds,
            confidence,
            font: None,
            font_size: None,
            bold: false,
            italic: false,
        }
    }
}

// ============================================================================
// OCR Line
// ============================================================================

/// A recognized line of text
#[derive(Debug, Clone)]
pub struct OcrLine {
    /// Line text
    pub text: String,
    /// Bounding box
    pub bounds: Rect,
    /// Words in this line
    pub words: Vec<OcrWord>,
    /// Average confidence
    pub confidence: i32,
}

impl OcrLine {
    /// Create a new OCR line
    pub fn new(bounds: Rect) -> Self {
        Self {
            text: String::new(),
            bounds,
            words: Vec::new(),
            confidence: 0,
        }
    }

    /// Add a word to this line
    pub fn add_word(&mut self, word: OcrWord) {
        if !self.text.is_empty() {
            self.text.push(' ');
        }
        self.text.push_str(&word.text);
        self.words.push(word);
        self.update_confidence();
    }

    fn update_confidence(&mut self) {
        if self.words.is_empty() {
            self.confidence = 0;
        } else {
            let sum: i32 = self.words.iter().map(|w| w.confidence).sum();
            self.confidence = sum / self.words.len() as i32;
        }
    }
}

// ============================================================================
// OCR Block
// ============================================================================

/// A block of recognized text
#[derive(Debug, Clone)]
pub struct OcrBlock {
    /// Block bounds
    pub bounds: Rect,
    /// Lines in this block
    pub lines: Vec<OcrLine>,
    /// Block type (paragraph, table, image, etc.)
    pub block_type: i32,
}

impl OcrBlock {
    /// Create a new OCR block
    pub fn new(bounds: Rect) -> Self {
        Self {
            bounds,
            lines: Vec::new(),
            block_type: 0, // Paragraph
        }
    }

    /// Add a line to this block
    pub fn add_line(&mut self, line: OcrLine) {
        self.lines.push(line);
    }

    /// Get full text
    pub fn text(&self) -> String {
        self.lines
            .iter()
            .map(|l| l.text.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    }
}

// ============================================================================
// OCR Result
// ============================================================================

/// OCR recognition result
#[derive(Debug, Clone)]
pub struct OcrResult {
    /// Recognized blocks
    pub blocks: Vec<OcrBlock>,
    /// Overall confidence
    pub confidence: i32,
    /// Processing time in milliseconds
    pub processing_time_ms: u64,
    /// Image width
    pub width: u32,
    /// Image height
    pub height: u32,
    /// Error message (if any)
    pub error: Option<String>,
}

impl OcrResult {
    /// Create a new empty result
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            blocks: Vec::new(),
            confidence: 0,
            processing_time_ms: 0,
            width,
            height,
            error: None,
        }
    }

    /// Create an error result
    pub fn error(msg: impl Into<String>) -> Self {
        let mut result = Self::new(0, 0);
        result.error = Some(msg.into());
        result
    }

    /// Get full text
    pub fn text(&self) -> String {
        self.blocks
            .iter()
            .map(|b| b.text())
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    /// Get word count
    pub fn word_count(&self) -> usize {
        self.blocks
            .iter()
            .flat_map(|b| &b.lines)
            .flat_map(|l| &l.words)
            .count()
    }

    /// Get line count
    pub fn line_count(&self) -> usize {
        self.blocks.iter().map(|b| b.lines.len()).sum()
    }

    /// Add a block
    pub fn add_block(&mut self, block: OcrBlock) {
        self.blocks.push(block);
        self.update_confidence();
    }

    fn update_confidence(&mut self) {
        let total_words: usize = self.word_count();
        if total_words == 0 {
            self.confidence = 0;
            return;
        }

        let sum: i64 = self
            .blocks
            .iter()
            .flat_map(|b| &b.lines)
            .flat_map(|l| &l.words)
            .map(|w| w.confidence as i64)
            .sum();

        self.confidence = (sum / total_words as i64) as i32;
    }
}

// ============================================================================
// OCR Engine
// ============================================================================

/// OCR engine wrapper
pub struct OcrEngine {
    /// Configuration
    config: OcrConfig,
    /// Is initialized
    initialized: bool,
}

impl OcrEngine {
    /// Create a new OCR engine
    pub fn new(config: OcrConfig) -> Self {
        Self {
            config,
            initialized: false,
        }
    }

    /// Initialize the engine
    pub fn init(&mut self) -> Result<(), String> {
        match self.config.engine_type {
            OcrEngineType::None => {
                // Stub engine - always succeeds
                self.initialized = true;
                Ok(())
            }
            OcrEngineType::Tesseract => {
                // Tesseract integration would go here
                // For now, return an informative error
                Err("Tesseract OCR not compiled in. Enable the 'tesseract' feature.".to_string())
            }
            _ => Err(format!(
                "OCR engine {:?} not supported on this platform.",
                self.config.engine_type
            )),
        }
    }

    /// Check if initialized
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// Recognize text in a pixmap
    pub fn recognize(&self, _pixmap: &Pixmap) -> OcrResult {
        if !self.initialized {
            return OcrResult::error("OCR engine not initialized");
        }

        match self.config.engine_type {
            OcrEngineType::None => {
                // Return empty result for stub engine
                OcrResult::new(_pixmap.width() as u32, _pixmap.height() as u32)
            }
            _ => OcrResult::error("OCR recognition not implemented for this engine"),
        }
    }

    /// Set language
    pub fn set_language(&mut self, lang: &str) {
        self.config.language = lang.to_string();
        self.initialized = false; // Requires re-initialization
    }

    /// Get current language
    pub fn language(&self) -> &str {
        &self.config.language
    }

    /// Set page segmentation mode
    pub fn set_psm(&mut self, psm: OcrPageSegMode) {
        self.config.psm = psm;
    }

    /// Set engine mode
    pub fn set_oem(&mut self, oem: OcrEngineMode) {
        self.config.oem = oem;
        self.initialized = false;
    }

    /// Get available languages (stub)
    pub fn available_languages(&self) -> Vec<String> {
        // This would query the actual OCR engine
        vec!["eng".to_string()]
    }
}

// ============================================================================
// FFI Functions
// ============================================================================

/// Create a new OCR engine
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_ocr_engine(_ctx: Handle, engine_type: c_int) -> Handle {
    let config = OcrConfig {
        engine_type: match engine_type {
            1 => OcrEngineType::Tesseract,
            2 => OcrEngineType::WindowsOcr,
            3 => OcrEngineType::AppleVision,
            _ => OcrEngineType::None,
        },
        ..Default::default()
    };

    let engine = OcrEngine::new(config);
    OCR_ENGINES.insert(engine)
}

/// Drop an OCR engine
#[unsafe(no_mangle)]
pub extern "C" fn fz_drop_ocr_engine(_ctx: Handle, engine: Handle) {
    OCR_ENGINES.remove(engine);
}

/// Initialize an OCR engine
#[unsafe(no_mangle)]
pub extern "C" fn fz_ocr_engine_init(_ctx: Handle, engine: Handle) -> c_int {
    if let Some(arc) = OCR_ENGINES.get(engine) {
        if let Ok(mut e) = arc.lock() {
            return if e.init().is_ok() { 1 } else { 0 };
        }
    }
    0
}

/// Check if OCR engine is initialized
#[unsafe(no_mangle)]
pub extern "C" fn fz_ocr_engine_is_initialized(_ctx: Handle, engine: Handle) -> c_int {
    if let Some(arc) = OCR_ENGINES.get(engine) {
        if let Ok(e) = arc.lock() {
            return if e.is_initialized() { 1 } else { 0 };
        }
    }
    0
}

/// Set OCR language
#[unsafe(no_mangle)]
pub extern "C" fn fz_ocr_engine_set_language(
    _ctx: Handle,
    engine: Handle,
    lang: *const c_char,
) -> c_int {
    if lang.is_null() {
        return 0;
    }

    let lang_str = unsafe { CStr::from_ptr(lang) };
    let lang_str = match lang_str.to_str() {
        Ok(s) => s,
        Err(_) => return 0,
    };

    if let Some(arc) = OCR_ENGINES.get(engine) {
        if let Ok(mut e) = arc.lock() {
            e.set_language(lang_str);
            return 1;
        }
    }
    0
}

/// Get OCR language (returns allocated string)
#[unsafe(no_mangle)]
pub extern "C" fn fz_ocr_engine_get_language(_ctx: Handle, engine: Handle) -> *mut c_char {
    if let Some(arc) = OCR_ENGINES.get(engine) {
        if let Ok(e) = arc.lock() {
            if let Ok(s) = CString::new(e.language()) {
                return s.into_raw();
            }
        }
    }
    std::ptr::null_mut()
}

/// Set page segmentation mode
#[unsafe(no_mangle)]
pub extern "C" fn fz_ocr_engine_set_psm(_ctx: Handle, engine: Handle, psm: c_int) {
    let mode = match psm {
        0 => OcrPageSegMode::OsdOnly,
        1 => OcrPageSegMode::AutoOsd,
        2 => OcrPageSegMode::AutoOnly,
        3 => OcrPageSegMode::Auto,
        4 => OcrPageSegMode::SingleColumn,
        5 => OcrPageSegMode::SingleBlockVertText,
        6 => OcrPageSegMode::SingleBlock,
        7 => OcrPageSegMode::SingleLine,
        8 => OcrPageSegMode::SingleWord,
        9 => OcrPageSegMode::CircleWord,
        10 => OcrPageSegMode::SingleChar,
        11 => OcrPageSegMode::SparseText,
        12 => OcrPageSegMode::SparseTextOsd,
        13 => OcrPageSegMode::RawLine,
        _ => OcrPageSegMode::Auto,
    };

    if let Some(arc) = OCR_ENGINES.get(engine) {
        if let Ok(mut e) = arc.lock() {
            e.set_psm(mode);
        }
    }
}

/// Create a new OCR result
#[unsafe(no_mangle)]
pub extern "C" fn fz_new_ocr_result(_ctx: Handle, width: u32, height: u32) -> Handle {
    let result = OcrResult::new(width, height);
    OCR_RESULTS.insert(result)
}

/// Drop an OCR result
#[unsafe(no_mangle)]
pub extern "C" fn fz_drop_ocr_result(_ctx: Handle, result: Handle) {
    OCR_RESULTS.remove(result);
}

/// Get OCR result text (returns allocated string)
#[unsafe(no_mangle)]
pub extern "C" fn fz_ocr_result_text(_ctx: Handle, result: Handle) -> *mut c_char {
    if let Some(arc) = OCR_RESULTS.get(result) {
        if let Ok(r) = arc.lock() {
            if let Ok(s) = CString::new(r.text()) {
                return s.into_raw();
            }
        }
    }
    std::ptr::null_mut()
}

/// Get OCR result confidence
#[unsafe(no_mangle)]
pub extern "C" fn fz_ocr_result_confidence(_ctx: Handle, result: Handle) -> c_int {
    if let Some(arc) = OCR_RESULTS.get(result) {
        if let Ok(r) = arc.lock() {
            return r.confidence;
        }
    }
    0
}

/// Get OCR result word count
#[unsafe(no_mangle)]
pub extern "C" fn fz_ocr_result_word_count(_ctx: Handle, result: Handle) -> c_int {
    if let Some(arc) = OCR_RESULTS.get(result) {
        if let Ok(r) = arc.lock() {
            return r.word_count() as c_int;
        }
    }
    0
}

/// Get OCR result line count
#[unsafe(no_mangle)]
pub extern "C" fn fz_ocr_result_line_count(_ctx: Handle, result: Handle) -> c_int {
    if let Some(arc) = OCR_RESULTS.get(result) {
        if let Ok(r) = arc.lock() {
            return r.line_count() as c_int;
        }
    }
    0
}

/// Free an OCR string
#[unsafe(no_mangle)]
pub extern "C" fn fz_free_ocr_string(_ctx: Handle, s: *mut c_char) {
    if !s.is_null() {
        unsafe {
            drop(CString::from_raw(s));
        }
    }
}

/// Check if OCR is available
#[unsafe(no_mangle)]
pub extern "C" fn fz_ocr_is_available(_ctx: Handle, engine_type: c_int) -> c_int {
    // Currently only the stub engine is available
    if engine_type == 0 { 1 } else { 0 }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ocr_config_default() {
        let config = OcrConfig::default();
        assert_eq!(config.engine_type, OcrEngineType::None);
        assert_eq!(config.language, "eng");
        assert_eq!(config.dpi, 300);
    }

    #[test]
    fn test_ocr_word() {
        let word = OcrWord::new("hello", Rect::new(0.0, 0.0, 50.0, 20.0), 95);
        assert_eq!(word.text, "hello");
        assert_eq!(word.confidence, 95);
    }

    #[test]
    fn test_ocr_line() {
        let mut line = OcrLine::new(Rect::new(0.0, 0.0, 200.0, 20.0));
        line.add_word(OcrWord::new("hello", Rect::new(0.0, 0.0, 50.0, 20.0), 90));
        line.add_word(OcrWord::new("world", Rect::new(60.0, 0.0, 120.0, 20.0), 80));

        assert_eq!(line.text, "hello world");
        assert_eq!(line.words.len(), 2);
        assert_eq!(line.confidence, 85); // Average
    }

    #[test]
    fn test_ocr_block() {
        let mut block = OcrBlock::new(Rect::new(0.0, 0.0, 200.0, 100.0));
        let mut line = OcrLine::new(Rect::new(0.0, 0.0, 200.0, 20.0));
        line.add_word(OcrWord::new("test", Rect::new(0.0, 0.0, 40.0, 20.0), 100));
        block.add_line(line);

        assert_eq!(block.text(), "test");
    }

    #[test]
    fn test_ocr_result() {
        let mut result = OcrResult::new(100, 100);
        assert_eq!(result.word_count(), 0);
        assert_eq!(result.confidence, 0);

        let mut block = OcrBlock::new(Rect::new(0.0, 0.0, 100.0, 50.0));
        let mut line = OcrLine::new(Rect::new(0.0, 0.0, 100.0, 20.0));
        line.add_word(OcrWord::new("OCR", Rect::new(0.0, 0.0, 30.0, 20.0), 100));
        block.add_line(line);
        result.add_block(block);

        assert_eq!(result.word_count(), 1);
        assert_eq!(result.confidence, 100);
        assert_eq!(result.text(), "OCR");
    }

    #[test]
    fn test_ocr_engine_stub() {
        let config = OcrConfig::default();
        let mut engine = OcrEngine::new(config);

        assert!(!engine.is_initialized());
        assert!(engine.init().is_ok());
        assert!(engine.is_initialized());
    }

    #[test]
    fn test_ocr_engine_ffi() {
        let handle = fz_new_ocr_engine(0, 0); // Stub engine
        assert!(handle != 0);

        let init_result = fz_ocr_engine_init(0, handle);
        assert_eq!(init_result, 1);

        let is_init = fz_ocr_engine_is_initialized(0, handle);
        assert_eq!(is_init, 1);

        fz_drop_ocr_engine(0, handle);
    }

    #[test]
    fn test_ocr_result_ffi() {
        let handle = fz_new_ocr_result(0, 100, 100);
        assert!(handle != 0);

        let word_count = fz_ocr_result_word_count(0, handle);
        assert_eq!(word_count, 0);

        let confidence = fz_ocr_result_confidence(0, handle);
        assert_eq!(confidence, 0);

        fz_drop_ocr_result(0, handle);
    }

    #[test]
    fn test_ocr_availability() {
        // Stub engine is always available
        assert_eq!(fz_ocr_is_available(0, 0), 1);
        // Tesseract is not compiled in
        assert_eq!(fz_ocr_is_available(0, 1), 0);
    }
}
