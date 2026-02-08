use crate::pdf_engine::PdfState;
use image::DynamicImage;
use std::io::Cursor;

#[tauri::command]
pub fn apply_filter(image_data: String, filter_type: String) -> Result<String, String> {
    let bytes = base64::decode(image_data).map_err(|e| e.to_string())?;

    let mut img = image::load_from_memory(&bytes).map_err(|e| e.to_string())?;

    match filter_type.as_str() {
        "greyscale" => {
            img = DynamicImage::ImageLuma8(img.to_luma8());
        }
        "invert" => {
            img.invert();
        }
        _ => {}
    }

    let mut buf = Vec::new();
    img.write_to(&mut Cursor::new(&mut buf), image::ImageFormat::Png)
        .map_err(|e| e.to_string())?;

    Ok(base64::encode(buf))
}

#[tauri::command]
pub fn auto_crop(state: tauri::State<PdfState>, page_num: i32) -> Result<(), String> {
    let mut guard = state.doc.lock().unwrap();
    if let Some(wrapper) = guard.as_mut() {
        let doc = &mut wrapper.0;
        let page = doc.load_page(page_num as i32).map_err(|e| e.to_string())?;

        let rect = page.bound();
        println!("Auto-crop bounding box: {:?}", rect);

        Ok(())
    } else {
        Err("No document open".to_string())
    }
}
