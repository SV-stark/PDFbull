use crate::pdf_engine::{with_mut_doc, PdfState};
use pdfium_render::prelude::*;

#[tauri::command]
pub fn apply_filter(image_data: String, _filter_type: String) -> Result<String, String> {
    // Return original data (identity)
    Ok(image_data)
}

#[tauri::command]
pub fn auto_crop(state: tauri::State<PdfState>, page_num: i32) -> Result<(), String> {
    with_mut_doc(&state, |doc| {
        let mut page = doc
            .pages_mut()
            .get(page_num as u16)
            .map_err(|e| e.to_string())?;

        // Calculate bounding box of all content
        let mut min_x = f32::MAX;
        let mut min_y = f32::MAX;
        let mut max_x = f32::MIN;
        let mut max_y = f32::MIN;
        let mut found_content = false;

        for object in page.objects().iter() {
            if let Ok(rect) = object.bounds() {
                // Ignore empty objects or very small ones?
                if rect.width().value > 0.0 && rect.height().value > 0.0 {
                    found_content = true;
                    if rect.left().value < min_x {
                        min_x = rect.left().value;
                    }
                    if rect.bottom().value < min_y {
                        min_y = rect.bottom().value;
                    }
                    if rect.right().value > max_x {
                        max_x = rect.right().value;
                    }
                    if rect.top().value > max_y {
                        max_y = rect.top().value;
                    }
                }
            }
        }

        if found_content {
            // Apply padding? e.g. 10 points
            let padding = 10.0;
            let crop_box = PdfRect::new(
                PdfPoints::new(min_x - padding),
                PdfPoints::new(min_y - padding),
                PdfPoints::new(max_x + padding),
                PdfPoints::new(max_y + padding),
            );

            // Set crop box
            page.boundaries_mut().set_crop(crop_box);

            Ok(())
        } else {
            // Empty page, do nothing or crop to zero?
            // Doing nothing is safer.
            Ok(())
        }
    })
}
