use pdfbull::ocr::{OcrLine, OcrPageResult, OcrWord};

#[test]
fn test_ocr_data_structures() {
    let word1 = OcrWord {
        text: "PDFbull".to_string(),
        bbox: [50.0, 750.0, 120.0, 762.0],
    };
    let word2 = OcrWord {
        text: "OCR".to_string(),
        bbox: [130.0, 750.0, 160.0, 762.0],
    };

    let line = OcrLine {
        text: "PDFbull OCR".to_string(),
        words: vec![word1, word2],
        bbox: [50.0, 750.0, 160.0, 762.0],
    };

    let res = OcrPageResult::new(0, vec![line]);

    assert_eq!(res.page_num, 0);
    assert_eq!(res.lines.len(), 1);
    assert_eq!(res.full_text(), "PDFbull OCR");
    assert!(!res.is_empty());
    assert_eq!(res.lines[0].words[0].text, "PDFbull");
}

#[test]
fn test_ocr_empty_page_result() {
    let empty_res = OcrPageResult::default();
    assert!(empty_res.is_empty());
    assert_eq!(empty_res.full_text(), "");
}

#[test]
fn test_ocr_script_selection() {
    use pdfbull::ocr::OcrScript;

    let latin = OcrScript::Latin;
    assert_eq!(latin.rec_model_filename(), "text-recognition.rten");
    assert_eq!(latin.name(), "Latin / English");

    let devanagari = OcrScript::Devanagari;
    assert_eq!(
        devanagari.rec_model_filename(),
        "devanagari_PP-OCRv4_rec.rten"
    );
    assert_eq!(devanagari.name(), "Devanagari (Hindi, Marathi, Sanskrit)");
}
