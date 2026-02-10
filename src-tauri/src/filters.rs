use crate::pdf_engine::{with_mut_doc, PdfState};
use base64::{engine::general_purpose, Engine as _};
use image::{DynamicImage, ImageFormat};
use pdfium_render::prelude::*;
use std::io::Cursor;

#[tauri::command]
pub fn apply_filter(image_data: String, filter_type: String) -> Result<String, String> {
    // 1. Decode Base64 (handle data URI prefix if present)
    let base64_string = if let Some(index) = image_data.find(',') {
        &image_data[index + 1..]
    } else {
        &image_data
    };

    let decoded_data = general_purpose::STANDARD
        .decode(base64_string)
        .map_err(|e| format!("Failed to decode base64: {}", e))?;

    // 2. Load Image
    let mut img = image::load_from_memory(&decoded_data)
        .map_err(|e| format!("Failed to load image: {}", e))?;

    // 3. Apply Filter
    match filter_type.as_str() {
        "grayscale" => {
            img = img.grayscale();
        }
        "threshold" => {
            // Simple thresholding
            img = img.grayscale();
            let mut gray_img = img.to_luma8();
            for pixel in gray_img.pixels_mut() {
                if pixel[0] < 128 {
                    pixel[0] = 0;
                } else {
                    pixel[0] = 255;
                }
            }
            img = DynamicImage::ImageLuma8(gray_img);
        }
        "invert" => {
            img.invert();
        }
        _ => {} // Identity for unknown types
    }

    // 4. Encode back to Base64 (PNG for quality)
    let mut buf = Vec::new();
    img.write_to(&mut Cursor::new(&mut buf), ImageFormat::Png)
        .map_err(|e| format!("Failed to encode image: {}", e))?;

    let encoded_string = general_purpose::STANDARD.encode(&buf);
    let mime_type = "image/png";

    Ok(format!("data:{};base64,{}", mime_type, encoded_string))
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
                // Ignore small objects that might be artifacts
                if rect.width().value > 1.0 && rect.height().value > 1.0 {
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
            // Apply padding (10 points)
            let padding = 10.0;
            // Ensure crop box doesn't exceed media box (not checked here but usually fine)
            // and strictly valid

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
            // No content found, do not crop
            Ok(())
        }
    })
}
