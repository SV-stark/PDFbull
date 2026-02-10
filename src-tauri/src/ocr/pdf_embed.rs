use crate::ocr::PageTextBlocks;
use lopdf::{content::Operation, Dictionary, Document, Object, Stream};

/// Embed OCR text layer into a PDF file
pub fn embed_text_layer(
    pdf_path: &str,
    ocr_data: Vec<PageTextBlocks>,
    output_path: &str,
) -> Result<String, String> {
    // Load PDF document
    let mut doc = Document::load(pdf_path).map_err(|e| format!("Failed to load PDF: {}", e))?;

    for page_data in ocr_data {
        let page_number = page_data.page_number as u32;

        // Get page object ID
        let page_id = match doc.page_object_id(page_number) {
            Some(id) => id,
            None => continue,
        };

        // Create operations for text layer
        let mut ops = Vec::new();
        ops.push(Operation::new("BT", vec![]));
        ops.push(Operation::new("Tr", vec![Object::Integer(3)])); // Rendering Mode 3 (Invisible)

        for block in page_data.blocks {
            // Font: /F1 (Standard Helvetica), Size: block height
            // We use simple transformation matrix (Tm) for positioning
            // Logic assumes block.x and block.y are PDF coordinates (Bottom-Left origin)
            // If they are Top-Left (Image), they might be upside down without flipping.
            // For now, we implement direct mapping.

            ops.push(Operation::new(
                "Tf",
                vec![Object::Name(b"F1".to_vec()), Object::Real(block.height)],
            ));
            ops.push(Operation::new(
                "Tm",
                vec![
                    Object::Real(1.0),
                    Object::Real(0.0),
                    Object::Real(0.0),
                    Object::Real(1.0),
                    Object::Real(block.x),
                    Object::Real(block.y),
                ],
            ));
            ops.push(Operation::new(
                "Tj",
                vec![Object::String(
                    block.text.clone().into_bytes(),
                    lopdf::StringFormat::Literal,
                )],
            ));
        }

        ops.push(Operation::new("ET", vec![]));

        // Create stream content
        let content_bytes = lopdf::content::Content { operations: ops }
            .encode()
            .unwrap_or_default();
        let stream = Stream::new(Dictionary::new(), content_bytes);

        // Add content stream to document
        let stream_id = doc.add_object(Object::Stream(stream));

        // Attach to page
        if let Ok(page_dict) = doc.get_object_mut(page_id).and_then(|o| o.as_dict_mut()) {
            // Ensure /Resources/Font/F1 exists (Simplified injection)
            if let Ok(resources) = page_dict.get_mut(b"Resources") {
                if let Ok(res_dict) = resources.as_dict_mut() {
                    if !res_dict.has(b"Font") {
                        res_dict.set("Font", Dictionary::new());
                    }
                    if let Ok(font_entry) = res_dict.get_mut(b"Font") {
                        if let Ok(font_dict) = font_entry.as_dict_mut() {
                            if !font_dict.has(b"F1") {
                                let mut f1 = Dictionary::new();
                                f1.set("Type", Object::Name(b"Font".to_vec()));
                                f1.set("Subtype", Object::Name(b"Type1".to_vec()));
                                f1.set("BaseFont", Object::Name(b"Helvetica".to_vec()));
                                font_dict.set("F1", Object::Dictionary(f1));
                            }
                        }
                    }
                }
            } else {
                // Create Resources if missing
                let mut f1 = Dictionary::new();
                f1.set("Type", Object::Name(b"Font".to_vec()));
                f1.set("Subtype", Object::Name(b"Type1".to_vec()));
                f1.set("BaseFont", Object::Name(b"Helvetica".to_vec()));

                let mut fonts = Dictionary::new();
                fonts.set("F1", Object::Dictionary(f1));

                let mut res = Dictionary::new();
                res.set("Font", Object::Dictionary(fonts));

                page_dict.set("Resources", Object::Dictionary(res));
            }

            // Append to Contents
            match page_dict.get_mut(b"Contents") {
                Ok(Object::Reference(ref_id)) => {
                    let new_contents =
                        vec![Object::Reference(*ref_id), Object::Reference(stream_id)];
                    page_dict.set("Contents", Object::Array(new_contents));
                }
                Ok(Object::Array(ref mut arr)) => {
                    arr.push(Object::Reference(stream_id));
                }
                Err(_) => {
                    page_dict.set("Contents", Object::Reference(stream_id));
                }
                _ => {} // Should handle other cases but simplified for now
            }
        }
    }

    doc.save(output_path)
        .map_err(|e| format!("Failed to save PDF: {}", e))?;

    Ok(output_path.to_string())
}
