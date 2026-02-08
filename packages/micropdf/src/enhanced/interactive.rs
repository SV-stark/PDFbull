//! Interactive PDF Features
//!
//! Advanced interactive capabilities:
//! - JavaScript actions
//! - Page transitions
//! - Multimedia (video/audio)
//! - 3D objects
//! - Complete annotation types

use super::error::{EnhancedError, Result};
use std::path::Path;

/// JavaScript action types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionType {
    /// Document-level action
    DocumentOpen,
    DocumentClose,
    DocumentSave,
    DocumentPrint,
    /// Page-level action
    PageOpen,
    PageClose,
    /// Field-level action
    FieldFocus,
    FieldBlur,
    FieldChange,
    FieldValidate,
    FieldCalculate,
    /// Mouse actions
    MouseEnter,
    MouseExit,
    MouseDown,
    MouseUp,
}

/// JavaScript action
#[derive(Debug, Clone)]
pub struct JavaScriptAction {
    pub action_type: ActionType,
    pub script: String,
}

impl JavaScriptAction {
    /// Create new JavaScript action
    pub fn new(action_type: ActionType, script: impl Into<String>) -> Self {
        Self {
            action_type,
            script: script.into(),
        }
    }
}

/// Add JavaScript action to PDF
pub fn add_javascript_action(
    pdf_path: &str,
    action: &JavaScriptAction,
    target: Option<&str>, // field name or page number
) -> Result<()> {
    if !Path::new(pdf_path).exists() {
        return Err(EnhancedError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("PDF file not found: {}", pdf_path),
        )));
    }

    // TODO: Implement JavaScript action
    // 1. Create JavaScript action dictionary
    // 2. Add to appropriate location (document, page, field)
    // 3. Update PDF structure

    Ok(())
}

/// Page transition effect
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransitionEffect {
    /// No transition
    None,
    /// Split (in/out, horizontal/vertical)
    Split,
    /// Blinds (horizontal/vertical)
    Blinds,
    /// Box (in/out)
    Box,
    /// Wipe (left/right/up/down)
    Wipe,
    /// Dissolve
    Dissolve,
    /// Glitter (left/down/diagonal)
    Glitter,
    /// Replace
    Replace,
    /// Fly (in/out)
    Fly,
    /// Push (left/right/up/down)
    Push,
    /// Cover (left/right/up/down)
    Cover,
    /// Uncover (left/right/up/down)
    Uncover,
    /// Fade
    Fade,
}

/// Page transition configuration
#[derive(Debug, Clone)]
pub struct PageTransition {
    pub effect: TransitionEffect,
    pub duration: f32, // seconds
    pub direction: Option<TransitionDirection>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransitionDirection {
    Horizontal,
    Vertical,
    Inward,
    Outward,
    LeftToRight,
    RightToLeft,
    TopToBottom,
    BottomToTop,
}

impl PageTransition {
    /// Create new transition
    pub fn new(effect: TransitionEffect, duration: f32) -> Self {
        Self {
            effect,
            duration,
            direction: None,
        }
    }

    /// Set direction
    pub fn with_direction(mut self, direction: TransitionDirection) -> Self {
        self.direction = Some(direction);
        self
    }
}

/// Add page transition
pub fn add_page_transition(pdf_path: &str, page: u32, transition: &PageTransition) -> Result<()> {
    if !Path::new(pdf_path).exists() {
        return Err(EnhancedError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("PDF file not found: {}", pdf_path),
        )));
    }

    // TODO: Implement page transition
    // 1. Create transition dictionary
    // 2. Add to page object
    // 3. Set duration and effect parameters

    Ok(())
}

/// All 28 PDF annotation types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnnotationType {
    // Basic annotations (already have some)
    Text,
    Link,
    FreeText,

    // Shape annotations
    Line,
    Square,
    Circle,
    Polygon,
    PolyLine,

    // Markup annotations
    Highlight,
    Underline,
    Squiggly,
    StrikeOut,

    // Other annotations
    Stamp,
    Caret,
    Ink,
    Popup,
    FileAttachment,
    Sound,
    Movie,
    Screen,
    Widget,
    PrinterMark,
    TrapNet,
    Watermark,
    ThreeD,
    Redact,
    Projection,
    RichMedia,
}

/// Annotation configuration
#[derive(Debug, Clone)]
pub struct Annotation {
    pub annotation_type: AnnotationType,
    pub page: u32,
    pub rect: (f32, f32, f32, f32),
    pub contents: Option<String>,
    pub author: Option<String>,
    pub color: Option<(f32, f32, f32)>,
    pub opacity: f32,
}

impl Annotation {
    /// Create new annotation
    pub fn new(annotation_type: AnnotationType, page: u32, rect: (f32, f32, f32, f32)) -> Self {
        Self {
            annotation_type,
            page,
            rect,
            contents: None,
            author: None,
            color: None,
            opacity: 1.0,
        }
    }

    /// Set contents
    pub fn with_contents(mut self, contents: impl Into<String>) -> Self {
        self.contents = Some(contents.into());
        self
    }

    /// Set color
    pub fn with_color(mut self, r: f32, g: f32, b: f32) -> Self {
        self.color = Some((r, g, b));
        self
    }

    /// Set opacity
    pub fn with_opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity;
        self
    }
}

/// Add annotation to PDF
pub fn add_annotation(pdf_path: &str, annotation: &Annotation) -> Result<()> {
    if !Path::new(pdf_path).exists() {
        return Err(EnhancedError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("PDF file not found: {}", pdf_path),
        )));
    }

    // TODO: Implement annotation creation
    // 1. Create annotation dictionary
    // 2. Set appearance stream
    // 3. Add to page annotations array

    Ok(())
}

/// Multimedia configuration
#[derive(Debug, Clone)]
pub struct Multimedia {
    pub media_type: MediaType,
    pub file_path: String,
    pub page: u32,
    pub rect: (f32, f32, f32, f32),
    pub autoplay: bool,
    pub controls: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaType {
    Video,
    Audio,
    ThreeD,
}

/// Embed multimedia in PDF
pub fn embed_multimedia(pdf_path: &str, multimedia: &Multimedia) -> Result<()> {
    if !Path::new(pdf_path).exists() {
        return Err(EnhancedError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("PDF file not found: {}", pdf_path),
        )));
    }

    if !Path::new(&multimedia.file_path).exists() {
        return Err(EnhancedError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Media file not found: {}", multimedia.file_path),
        )));
    }

    // TODO: Implement multimedia embedding
    // 1. Read media file
    // 2. Create media object
    // 3. Create screen annotation
    // 4. Set up rendition
    // 5. Add to page

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_javascript_action() {
        let action = JavaScriptAction::new(ActionType::DocumentOpen, "app.alert('Welcome!');");
        assert_eq!(action.action_type, ActionType::DocumentOpen);
        assert!(action.script.contains("Welcome"));
    }

    #[test]
    fn test_page_transition() {
        let transition = PageTransition::new(TransitionEffect::Dissolve, 1.5)
            .with_direction(TransitionDirection::LeftToRight);
        assert_eq!(transition.effect, TransitionEffect::Dissolve);
        assert_eq!(transition.duration, 1.5);
    }

    #[test]
    fn test_annotation_creation() {
        let annotation =
            Annotation::new(AnnotationType::Highlight, 0, (100.0, 100.0, 200.0, 120.0))
                .with_contents("Important text")
                .with_color(1.0, 1.0, 0.0)
                .with_opacity(0.5);

        assert_eq!(annotation.annotation_type, AnnotationType::Highlight);
        assert_eq!(annotation.opacity, 0.5);
    }

    #[test]
    fn test_multimedia_config() {
        let media = Multimedia {
            media_type: MediaType::Video,
            file_path: "video.mp4".to_string(),
            page: 0,
            rect: (100.0, 100.0, 400.0, 300.0),
            autoplay: false,
            controls: true,
        };
        assert_eq!(media.media_type, MediaType::Video);
        assert!(media.controls);
    }
}
