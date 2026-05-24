use pdfbull::models::{Annotation, AnnotationStyle, DocumentId};
use pdfbull::pdf_engine::DocumentStore;
use pdfium_render::prelude::*;

#[test]
#[cfg_attr(
    target_os = "windows",
    ignore = "PDFium DLL segfaults on Windows during text object deletion under unit test conditions"
)]
fn test_real_redaction_removes_text() {
    let bindings = Pdfium::bind_to_system_library().unwrap_or_else(|_| {
        Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path("./")).unwrap()
    });
    let pdfium = Pdfium::new(bindings);

    let cache = pdfbull::pdf_engine::create_render_cache(10, 100);
    let mut store = DocumentStore::new(&pdfium, cache);

    let text_x = 100.0;
    let text_y = 500.0;
    let text_width = 250.0;
    let text_height = 30.0;

    println!("[TRACE] Step 1: Creating dummy PDF");
    let (pdf_bytes, page_height) = {
        let mut doc = pdfium.create_new_pdf().unwrap();
        let height = {
            let fonts = doc.fonts_mut();
            let helvetica = fonts.helvetica();

            let mut page = doc
                .pages_mut()
                .create_page_at_end(PdfPagePaperSize::a4())
                .unwrap();

            // Add sensitive text object at a known coordinate box
            // Text: "CONFIDENTIAL_SSN_12345"
            let mut text = PdfPageTextObject::new(
                &doc,
                "CONFIDENTIAL_SSN_12345",
                helvetica,
                PdfPoints::new(16.0),
            )
            .unwrap();
            text.translate(PdfPoints::new(text_x), PdfPoints::new(text_y))
                .unwrap();
            page.objects_mut().add_text_object(text).unwrap();
            page.height().value
        };
        (doc.save_to_bytes().unwrap(), height)
    };
    println!("[TRACE] Step 2: Writing dummy PDF to temp file");
    let temp_path = std::env::temp_dir().join("test_redaction_before.pdf");
    std::fs::write(&temp_path, pdf_bytes).unwrap();

    // 1. Verify that the un-redacted document has the sensitive text
    println!("[TRACE] Step 3: Opening dummy PDF in store");
    let doc_id = DocumentId(888);
    store
        .open_document(&temp_path.to_string_lossy(), doc_id)
        .expect("Failed to open document");

    println!("[TRACE] Step 4: Extracting text from original");
    let original_text = store
        .extract_text(doc_id, 0)
        .expect("Failed to extract text");
    assert!(original_text.contains("CONFIDENTIAL_SSN_12345"));

    // 2. Apply our new redaction over that coordinate region
    println!("[TRACE] Step 5: Preparing Redact annotation");
    let ann = Annotation {
        id: 1,
        page: 0,
        style: AnnotationStyle::Redact {
            color: "#000000".to_string(),
        },
        x: text_x,
        y: page_height - (text_y + text_height),
        width: text_width,
        height: text_height,
    };

    println!("[TRACE] Step 6: Saving annotations (applying redactions)");
    let redacted_path = std::env::temp_dir().join("test_redaction_after.pdf");
    store
        .save_annotations(
            doc_id,
            &[ann],
            Some(redacted_path.to_string_lossy().to_string()),
        )
        .expect("Failed to save redactions");

    // Close before opening redacted one
    println!("[TRACE] Step 7: Closing original document");
    store.close_document(doc_id);

    // 3. Open the redacted document and verify that the sensitive text is completely removed from the stream!
    println!("[TRACE] Step 8: Opening redacted document in store");
    let redacted_doc_id = DocumentId(889);
    store
        .open_document(&redacted_path.to_string_lossy(), redacted_doc_id)
        .expect("Failed to open redacted document");

    println!("[TRACE] Step 9: Extracting redacted text");
    let redacted_text = store
        .extract_text(redacted_doc_id, 0)
        .expect("Failed to extract redacted text");

    // Assert that the redacted text is completely gone!
    println!("[TRACE] Step 10: Asserting redaction correctness");
    assert!(
        !redacted_text.contains("CONFIDENTIAL_SSN_12345"),
        "Security Breach: Text 'CONFIDENTIAL_SSN_12345' was not deleted from content streams!"
    );

    // Clean up
    println!("[TRACE] Step 11: Cleaning up temp files");
    let _ = std::fs::remove_file(temp_path);
    let _ = std::fs::remove_file(redacted_path);
    println!("[TRACE] Step 12: Done");
}
