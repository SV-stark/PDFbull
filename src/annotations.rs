use crate::pdf_engine::{with_mut_doc, PdfState};
use pdfium_render::prelude::*;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct AnnotationData {
    pub page: i32,
    pub r#type: String,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub color: String,
    pub text: Option<String>,
    // For line/arrow
    pub x1: Option<f32>,
    pub y1: Option<f32>,
    pub x2: Option<f32>,
    pub y2: Option<f32>,
}

#[tauri::command]
pub fn save_annotations(
    state: tauri::State<PdfState>,
    output_path: String,
    annotations: Vec<AnnotationData>,
) -> Result<(), String> {
    with_mut_doc(&state, |doc| {
        for ann in annotations {
            if let Ok(mut page) = doc.pages_mut().get(ann.page as u16) {
                let height = page.height().value;
                let rect = if ann.r#type == "line" || ann.r#type == "arrow" {
                    let x1 = ann.x1.unwrap_or(ann.x);
                    let y1 = ann.y1.unwrap_or(ann.y);
                    let x2 = ann.x2.unwrap_or(ann.x + ann.w);
                    let y2 = ann.y2.unwrap_or(ann.y + ann.h);
                    let min_x = x1.min(x2);
                    let min_y = y1.min(y2);
                    let max_x = x1.max(x2);
                    let max_y = y1.max(y2);
                    PdfRect::new(
                        PdfPoints::new(min_x),
                        PdfPoints::new(height - max_y),
                        PdfPoints::new(max_x),
                        PdfPoints::new(height - min_y),
                    )
                } else {
                    PdfRect::new(
                        PdfPoints::new(ann.x),
                        PdfPoints::new(height - (ann.y + ann.h)),
                        PdfPoints::new(ann.x + ann.w),
                        PdfPoints::new(height - ann.y),
                    )
                };

                let page_annotations = page.annotations_mut();

                match ann.r#type.as_str() {
                    "highlight" => {
                        if let Ok(mut a) = page_annotations.create_highlight_annotation() {
                            a.set_bounds(rect);
                            if let Some(content) = &ann.text {
                                let _ = a.set_contents(content);
                            }
                        }
                    }
                    "rectangle" => {
                        if let Ok(mut a) = page_annotations.create_square_annotation() {
                            a.set_bounds(rect);
                            if let Some(content) = &ann.text {
                                let _ = a.set_contents(content);
                            }
                        }
                    }
                    "text" | "sticky" => {
                        // create_text_annotation usually takes content in some versions
                        // but let's try 0-arg or 1-arg as per error log
                        // error said: text_annotation takes 1 argument (&str)
                        let content = ann.text.as_deref().unwrap_or("");
                        if let Ok(mut a) = page_annotations.create_text_annotation(content) {
                            a.set_bounds(rect);
                        }
                    }
                    // Skip unsupported for now to get compilation working
                    _ => {}
                }
            }
        }

        doc.save_to_file(&output_path)
            .map_err(|e| format!("Failed to save PDF: {}", e))
    })
}
