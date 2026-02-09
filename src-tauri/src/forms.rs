use crate::pdf_engine::{with_doc, PdfState};
use pdfium_render::prelude::*;
use serde::Serialize;

#[derive(Serialize)]
pub struct FormField {
    name: String,
    value: String,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    field_type: String,
}

#[tauri::command]
pub fn get_form_fields(
    state: tauri::State<PdfState>,
    page_num: i32,
) -> Result<Vec<FormField>, String> {
    with_doc(&state, |doc| {
        let page = doc
            .pages()
            .get(page_num as u16)
            .map_err(|e| e.to_string())?;
        let mut fields = Vec::new();

        for annotation in page.annotations().iter() {
            if annotation.annotation_type() == PdfPageAnnotationType::Widget {
                if let Ok(rect) = annotation.bounds() {
                    // Try to get object name as field name
                    // In a real implementation we would inspect the form field dictionary
                    // For now, we use a placeholder or check common properties
                    let name = annotation.name().unwrap_or("Field".to_string());

                    // Value is harder without form env, but let's see if we can get content
                    let value = annotation.contents().unwrap_or("".to_string());

                    fields.push(FormField {
                        name,
                        value,
                        x: rect.left().value,
                        y: rect.top().value,
                        w: rect.width().value,
                        h: rect.height().value,
                        field_type: "text".to_string(), // Defaulting to text for now
                    });
                }
            }
        }
        Ok(fields)
    })
}
