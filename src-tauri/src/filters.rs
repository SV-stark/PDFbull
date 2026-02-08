use crate::pdf_engine::PdfState;
use image::DynamicImage;
use std::io::Cursor;
use base64::Engine as _;

#[tauri::command]
pub fn apply_filter(image_data: String, filter_type: String) -> Result<String, String> {
    let bytes = base64::engine::general_purpose::STANDARD.decode(image_data)
        .map_err(|e| e.to_string())?;
        
    let mut img = image::load_from_memory(&bytes).map_err(|e| e.to_string())?;
    
    match filter_type.as_str() {
        "greyscale" => {
            img = DynamicImage::ImageLuma8(img.to_luma8());
        },
        "invert" => {
            img.invert();
        },
        _ => {}
    }
    
    let mut buf = Vec::new();
    img.write_to(&mut Cursor::new(&mut buf), image::ImageOutputFormat::Png)
       .map_err(|e| e.to_string())?;
       
    Ok(base64::engine::general_purpose::STANDARD.encode(buf))
}

#[tauri::command]
pub fn auto_crop(state: tauri::State<PdfState>, page_num: i32) -> Result<(), String> {
    let mut guard = state.doc.lock().unwrap();
    if let Some(doc) = guard.as_mut() {
        let page = doc.load_page(page_num).map_err(|e| e.to_string())?;
        
        // Render small version for edge detection to save speed
        let matrix = mupdf::Matrix::new_scale(0.5, 0.5);
        let pixmap = page.to_pixmap(&matrix, &mupdf::Colorspace::device_gray(), false)
            .map_err(|e| e.to_string())?;
            
        // Use imageproc to find content bounds (simplified)
        // In real impl: use Canny edge detector + bounding box of white pixels
        // For MVP: Just set a theoretical crop to show it works
        // let rect = mupdf::Rect::new(50.0, 50.0, page.bounds().unwrap().x1 - 50.0, page.bounds().unwrap().y1 - 50.0);
        // page.set_crop_box(rect); 
        
        // Note: mupdf-rs might not expose set_crop_box directly in 0.4/0.8 without newer bindings
        // Assuming it does:
        // page.set_box(mupdf::PdfPageBox::Crop, rect);
        
        Ok(())
    } else {
        Err("No document open".to_string())
    }
}
