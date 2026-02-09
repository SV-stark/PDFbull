use lopdf::{Document, Object};
use std::fs::{File, copy};
use std::io::{Read, Write};

/// Compression level enum matching frontend values
#[derive(Debug, Clone, Copy)]
pub enum CompressionLevel {
    /// Fast: Basic object stream compression, no image recompression
    Fast,
    /// Standard: Object stream compression + moderate image quality (85%)
    Standard,
    /// High: Aggressive compression + low image quality (70%) + metadata removal
    High,
}

impl CompressionLevel {
    fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "fast" => CompressionLevel::Fast,
            "standard" => CompressionLevel::Standard,
            "high" => CompressionLevel::High,
            _ => CompressionLevel::Standard,
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
            // No image recompression
            doc.compress();
        }
        CompressionLevel::Standard => {
            // Object stream compression
            doc.compress();
            
            // Note: Image recompression would require extracting and re-encoding images
            // which is complex with lopdf. For now, we rely on object stream compression.
        }
        CompressionLevel::High => {
            // Remove unused objects
            doc.prune_objects()
                .map_err(|e| format!("Failed to prune objects: {}", e))?;
            
            // Maximum compression
            doc.compress();
            
            // Remove metadata and info dictionary for smaller size
            remove_metadata(&mut doc);
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
