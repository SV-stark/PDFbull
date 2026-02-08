use micropdf::enhanced::pdf_reader::PdfDocument;

pub fn probe() {
    let res = PdfDocument::open("test.pdf");
    match res {
        Err(e) => {
            let _: () = e; // Reveal error type
        }
        _ => {}
    }
}

pub fn probe_annot() {
    let _ = micropdf::pdf::annot::AnnotationType::Highlight; // Try this?
                                                             // let _ = micropdf::fitz::annot::AnnotationType::Highlight; // Or this?
}
