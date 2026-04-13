use pdfbull::models::DocumentId;
use pdfbull::pdf_engine::{DocumentStore, RenderFilter, RenderOptions, RenderQuality};
use pdfium_render::prelude::*;
use sha2::{Digest, Sha256};

// We will test the engine's core capability to render a basic, known PDF
// and hash the resulting image bytes to ensure visual stability.

#[test]
fn test_render_stability() {
    let bindings = Pdfium::bind_to_system_library().unwrap_or_else(|_| {
        Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path("./")).unwrap()
    });
    let pdfium = Pdfium::new(bindings);

    let cache = pdfbull::pdf_engine::create_render_cache(10, 100);
    let mut store = DocumentStore::new(&pdfium, cache);

    // Create a dummy PDF in memory via pdfium
    let mut doc = pdfium.create_new_pdf().unwrap();

    // Extract font before borrowing doc mutably for pages
    let fonts = doc.fonts_mut();
    let helvetica = fonts.helvetica();

    let mut page = doc
        .pages_mut()
        .create_page_at_end(PdfPagePaperSize::a4())
        .unwrap();

    // Add some simple text
    let mut text =
        PdfPageTextObject::new(&doc, "PDFbull Visual Test", helvetica, PdfPoints::new(24.0))
            .unwrap();
    text.translate(PdfPoints::new(100.0), PdfPoints::new(500.0))
        .unwrap();
    page.objects_mut().add_text_object(text).unwrap();

    // Save to a temporary buffer
    let pdf_bytes = doc.save_to_bytes().unwrap();
    let temp_path = std::env::temp_dir().join("test_render_stability.pdf");
    std::fs::write(&temp_path, pdf_bytes).unwrap();

    let doc_id = DocumentId(999);
    store
        .open_document(&temp_path.to_string_lossy(), doc_id)
        .expect("Failed to open document");

    let options = RenderOptions {
        scale: 1.0,
        rotation: 0,
        filter: RenderFilter::None,
        auto_crop: false,
        quality: RenderQuality::Medium,
    };

    let result = store
        .render_page(doc_id, 0, options)
        .expect("Failed to render page");

    // Hash the pixel data to ensure it remains completely consistent across changes
    let mut hasher = Sha256::new();
    hasher.update(&result.data);
    let hash = format!("{:x}", hasher.finalize());

    // Use insta to take a snapshot of the resulting image hash and dimensions
    insta::assert_debug_snapshot!("page_render_result", (result.width, result.height, hash));
}
