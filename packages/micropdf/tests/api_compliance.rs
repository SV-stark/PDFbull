use micropdf::ffi::document::Document;

#[test]
fn test_doc_api_signatures() {
    // This test ensures the API exists and compiles
    // It doesn't necessarily run successfully if the implementation is mock/placeholder

    // Test Document::open signature
    if let Ok(doc) = Document::open("test.pdf") {
        // Test Document methods
        let _count = doc.count_pages();

        // Test load_page
        if let Ok(page) = doc.load_page(0) {
            // Test Page methods
            let _text = page.extract_text();
            let _bounds = page.bound();
            let _ = page.save("output.png");

            // Test Page::to_pixmap signature (verified in previous steps)
            let matrix = micropdf::fitz::geometry::Matrix {
                a: 1.0,
                b: 0.0,
                c: 0.0,
                d: 1.0,
                e: 0.0,
                f: 0.0,
            };
            let _pixmap = page.to_pixmap(&matrix);
        }

        let _ = doc.save("saved.pdf", "");
    }

    // Test open_memory
    let doc = Document::open_memory(vec![0; 100]);
    let _ = doc.count_pages();
}
