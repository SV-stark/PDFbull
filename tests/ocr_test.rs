use pdfbull::commands::PdfCommand;
use pdfbull::message::Message;
use pdfbull::models::DocumentId;
use pdfbull::ocr::{OcrLine, OcrPageResult, OcrScript, OcrWord};

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
    assert_eq!(res.lines[0].words[1].text, "OCR");
}

#[test]
fn test_ocr_empty_page_result() {
    let empty_res = OcrPageResult::default();
    assert!(empty_res.is_empty());
    assert_eq!(empty_res.full_text(), "");
    assert_eq!(empty_res.page_num, 0);
}

#[test]
fn test_ocr_multi_line_text_reconstruction() {
    let line1 = OcrLine {
        text: "First line of text".to_string(),
        words: vec![
            OcrWord {
                text: "First".to_string(),
                bbox: [50.0, 750.0, 90.0, 762.0],
            },
            OcrWord {
                text: "line".to_string(),
                bbox: [95.0, 750.0, 120.0, 762.0],
            },
        ],
        bbox: [50.0, 750.0, 200.0, 762.0],
    };

    let line2 = OcrLine {
        text: "Second line of text".to_string(),
        words: vec![OcrWord {
            text: "Second".to_string(),
            bbox: [50.0, 732.0, 100.0, 744.0],
        }],
        bbox: [50.0, 732.0, 210.0, 744.0],
    };

    let page_res = OcrPageResult::new(1, vec![line1, line2]);
    assert_eq!(page_res.page_num, 1);
    assert_eq!(page_res.lines.len(), 2);
    assert_eq!(
        page_res.full_text(),
        "First line of text\nSecond line of text"
    );
}

#[test]
fn test_ocr_script_selection() {
    let latin = OcrScript::Latin;
    assert_eq!(latin.rec_model_filename(), "text-recognition.rten");
    assert_eq!(latin.det_model_filename(), "text-detection.rten");
    assert_eq!(latin.name(), "Latin / English");

    let devanagari = OcrScript::Devanagari;
    assert_eq!(
        devanagari.rec_model_filename(),
        "devanagari_PP-OCRv4_rec.rten"
    );
    assert_eq!(devanagari.det_model_filename(), "text-detection.rten");
    assert_eq!(devanagari.name(), "Devanagari (Hindi, Marathi, Sanskrit)");
}

#[test]
fn test_ocr_script_defaults_and_paths() {
    let default_script = OcrScript::default();
    assert_eq!(default_script, OcrScript::Latin);

    let model_dir = OcrScript::resolve_model_dir();
    assert!(model_dir.to_string_lossy().contains("models"));

    let det_path = OcrScript::Latin.resolve_det_model_path();
    assert!(det_path.ends_with("text-detection.rten"));

    let latin_rec_path = OcrScript::Latin.resolve_rec_model_path();
    assert!(latin_rec_path.ends_with("text-recognition.rten"));

    let dev_rec_path = OcrScript::Devanagari.resolve_rec_model_path();
    assert!(dev_rec_path.ends_with("devanagari_PP-OCRv4_rec.rten"));
}

#[test]
fn test_ocr_devanagari_words_and_text() {
    let word_hindi = OcrWord {
        text: "नमस्ते".to_string(),
        bbox: [50.0, 750.0, 110.0, 765.0],
    };
    let word_marathi = OcrWord {
        text: "महाराष्ट्र".to_string(),
        bbox: [120.0, 750.0, 200.0, 765.0],
    };

    let line = OcrLine {
        text: "नमस्ते महाराष्ट्र".to_string(),
        words: vec![word_hindi, word_marathi],
        bbox: [50.0, 750.0, 200.0, 765.0],
    };

    let page_res = OcrPageResult::new(0, vec![line]);
    assert_eq!(page_res.full_text(), "नमस्ते महाराष्ट्र");
    assert_eq!(page_res.lines[0].words[0].text, "नमस्ते");
    assert_eq!(page_res.lines[0].words[1].text, "महाराष्ट्र");
}

#[test]
fn test_ocr_serde_serialization() {
    let word = OcrWord {
        text: "TestWord".to_string(),
        bbox: [10.0, 20.0, 30.0, 40.0],
    };
    let line = OcrLine {
        text: "TestWord".to_string(),
        words: vec![word],
        bbox: [10.0, 20.0, 30.0, 40.0],
    };
    let original = OcrPageResult::new(2, vec![line]);

    let json_str = serde_json::to_string(&original).expect("Serialization should succeed");
    let deserialized: OcrPageResult =
        serde_json::from_str(&json_str).expect("Deserialization should succeed");

    assert_eq!(original, deserialized);
    assert_eq!(deserialized.page_num, 2);
    assert_eq!(deserialized.lines[0].words[0].text, "TestWord");
}

#[test]
fn test_ocr_command_message_payload() {
    let doc_id = DocumentId(99);
    let (tx, _rx) = tokio::sync::oneshot::channel();

    let cmd = PdfCommand::OcrPage(doc_id, 0, OcrScript::Devanagari, tx);

    match cmd {
        PdfCommand::OcrPage(id, page, script, _) => {
            assert_eq!(id, doc_id);
            assert_eq!(page, 0);
            assert_eq!(script, OcrScript::Devanagari);
        }
        _ => panic!("Expected PdfCommand::OcrPage variant"),
    }

    let msg = Message::SelectOcrScript(OcrScript::Devanagari);
    match msg {
        Message::SelectOcrScript(script) => {
            assert_eq!(script, OcrScript::Devanagari);
        }
        _ => panic!("Expected Message::SelectOcrScript variant"),
    }
}
