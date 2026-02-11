use lopdf::{Document, Object, Stream};
use rayon::prelude::*;
use std::io::Cursor;
use image::ImageFormat;
use std::collections::HashMap;

/// Compression level enum matching frontend values
#[derive(Debug, Clone, Copy)]
pub enum CompressionLevel {
    /// Fast: Basic object stream compression, no image recompression
    Fast,
    /// Standard: Object stream compression + moderate image quality (75%)
    Standard,
    /// High: Aggressive compression + low image quality (50%) + metadata removal
    High,
}

impl CompressionLevel {
    fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "low" | "fast" => CompressionLevel::Fast,
            "standard" | "medium" => CompressionLevel::Standard,
            "high" | "best" => CompressionLevel::High,
            _ => CompressionLevel::Standard,
        }
    }

    fn jpeg_quality(&self) -> u8 {
        match self {
            CompressionLevel::Fast => 100, // Not used
            CompressionLevel::Standard => 75,
            CompressionLevel::High => 50,
        }
    }
}

/// Compress a PDF file using lopdf with the specified compression level
#[tauri::command]
pub async fn compress_pdf(
    input_path: String,
    output_path: String,
    level: String,
) -> Result<CompressionResult, String> {
    // Offload CPU-intensive task to a blocking thread
    tokio::task::spawn_blocking(move || {
        compress_pdf_sync(input_path, output_path, level)
    }).await
    .map_err(|e| format!("Task join error: {}", e))?
}

fn compress_pdf_sync(
    input_path: String,
    output_path: String,
    level: String,
) -> Result<CompressionResult, String> {
    let compression_level = CompressionLevel::from_str(&level);
    
    // Get original file size
    let original_size = std::fs::metadata(&input_path)
        .map_err(|e| format!("Failed to read input file: {}", e))?
        .len();

    // Load document with lopdf
    let mut doc = Document::load(&input_path)
        .map_err(|e| format!("Failed to load PDF: {}", e))?;

    // Apply compression based on level
    match compression_level {
        CompressionLevel::Fast => {
            // Only apply basic object stream compression
            doc.compress();
        }
        CompressionLevel::Standard | CompressionLevel::High => {
            // Re-compress images
            compress_images(&mut doc, compression_level.jpeg_quality())?;

            if let CompressionLevel::High = compression_level {
                 // Remove unused objects
                let _ = doc.prune_objects();
                // Remove metadata
                remove_metadata(&mut doc);
            }

            // Object stream compression
            doc.compress();
        }
    }

    // Save compressed document
    doc.save(&output_path)
        .map_err(|e| format!("Failed to save compressed PDF: {}", e))?;

    // Get new file size
    let compressed_size = std::fs::metadata(&output_path)
        .map_err(|e| format!("Failed to read output file: {}", e))?
        .len();

    let savings_percent = if original_size > 0 {
        ((original_size as f64 - compressed_size as f64) / original_size as f64 * 100.0) as i32
    } else {
        0
    };

    Ok(CompressionResult {
        original_size,
        compressed_size,
        savings_percent,
        output_path,
    })
}

fn compress_images(doc: &mut Document, quality: u8) -> Result<(), String> {
    // 1. Collect all Object IDs that are XObject Images
    let mut image_ids = Vec::new();
    
    for (obj_id, object) in doc.objects.iter() {
        if let Object::Stream(stream) = object {
            if let Ok(subtype) = stream.dict.get(b"Subtype") {
                if let Object::Name(name) = subtype {
                    if name == b"Image" {
                        // Check filter to avoid re-compressing already highly compressed non-jpeg images if possible,
                        // but specifically we want to re-encode to JPEG for size.
                        // For simplicity, we attempt processing all valid image streams.
                        image_ids.push(*obj_id);
                    }
                }
            }
        }
    }

    // 2. Process images in parallel using Rayon
    // We can't mutate doc in parallel directly.
    // So we extract streams, process them, and return a map of ID -> NewContent.
    
    let processed_images: HashMap<_, _> = image_ids.par_iter()
        .filter_map(|&id| {
            if let Some(Object::Stream(stream)) = doc.objects.get(&id) {
                // Attempt to decode and re-encode
                if let Ok(processed_data) = process_image_stream(stream, quality) {
                    return Some((id, processed_data));
                }
            }
            None
        })
        .collect();

    // 3. Apply changes back to document
    for (id, new_data) in processed_images {
        if let Some(Object::Stream(stream)) = doc.objects.get_mut(&id) {
            stream.content = new_data;
            // Update filter to DCTDecode (JPEG)
            stream.dict.set("Filter", Object::Name(b"DCTDecode".to_vec()));
            // Remove Length if present (will be recalculated on save)
            // Remove Filter parameters if any specific ones existed that are no longer valid
             stream.dict.remove(b"DecodeParms");
        }
    }

    Ok(())
}

fn process_image_stream(stream: &Stream, quality: u8) -> Result<Vec<u8>, String> {
    // Attempt to decode the image using lopdf's helper or raw content if possible.
    // Note: lopdf decompression is limited. 
    // Ideally we rely on `stream.decompressed_content()`
    
    let content = stream.decompressed_content()
        .map_err(|e| format!("Decompression failed: {}", e))?;

    // We need to know dimensions and color space to interpret raw pixels for `image` crate.
    // This is complex. For a robust solution we'd need to parse Width, Height, BitsPerComponent, ColorSpace.
    
    // Simplification: Try to load with image::load_from_memory if it's a recognized format (e.g. embedded JPEG/PNG).
    // If it's raw pixel data (which PDF streams often are after decompression), we need metadata.
    
    // STRATEGY: 
    // 1. Try guessing format from content
    // 2. If valid image, load, resize/re-encode, return bytes.
    
    if let Ok(img) = image::load_from_memory(&content) {
        let mut out = Cursor::new(Vec::new());
        img.write_to(&mut out, ImageFormat::Jpeg)
           .map_err(|_| "Failed to write JPEG")?;
           
        // This is generic re-encoding. 
        // To strictly apply quality, we use an encoder.
        let mut out_custom = Cursor::new(Vec::new());
        let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out_custom, quality);
        encoder.encode_image(&img).map_err(|_| "Failed to encode JPEG")?;
        
        return Ok(out_custom.into_inner());
    }
    
    // If load_from_memory failed, it might be raw buffer.
    // Handling raw buffer requires Width/Height/ColorSpace parsing which is error-prone without a full PDF render engine equivalent.
    // For this task, we skip images we can't easily auto-detect (like raw CMYK buffers) to avoid corruption.
    
    Err("Could not decode image format".to_string())
}

/// Remove metadata from PDF for maximum compression
fn remove_metadata(doc: &mut Document) {
    // Remove document info dictionary if it exists
    if let Ok(trailer) = doc.trailer.get_mut(b"Info") {
        *trailer = Object::Null;
    }
    
    // Remove XMP metadata streams
    for (_, object) in doc.objects.iter_mut() {
        if let Object::Stream(stream) = object {
            // Check if it's an XMP metadata stream
            if let Ok(subtype) = stream.dict.get(b"Subtype") {
                if let Object::Name(name) = subtype {
                    if name == b"XML" {
                        // Clear metadata content but keep stream valid
                        stream.content = vec![];
                    }
                }
            }
        }
    }
}

/// Result of PDF compression operation
#[derive(Debug, Clone, serde::Serialize)]
pub struct CompressionResult {
    pub original_size: u64,
    pub compressed_size: u64,
    pub savings_percent: i32,
    pub output_path: String,
}
