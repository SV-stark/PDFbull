use pdfbull::commands::PdfCommand;
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
    let multi = OcrScript::Multilingual;
    assert_eq!(multi.rec_model_filename(), "PP-OCRv6_small_rec.onnx");
    assert_eq!(multi.det_model_filename(), "PP-OCRv6_small_det.onnx");
    assert_eq!(multi.dict_filename(), "ppocrv6_dict.txt");
    assert!(multi.name().contains("Multilingual"));

    let latin = OcrScript::Latin;
    assert_eq!(latin.rec_model_filename(), "PP-OCRv6_small_rec.onnx");
    assert_eq!(latin.det_model_filename(), "PP-OCRv6_small_det.onnx");

    let devanagari = OcrScript::Devanagari;
    assert_eq!(devanagari.rec_model_filename(), "PP-OCRv6_small_rec.onnx");
    assert_eq!(devanagari.det_model_filename(), "PP-OCRv6_small_det.onnx");
}

#[test]
fn test_ocr_script_defaults_and_paths() {
    let default_script = OcrScript::default();
    assert_eq!(default_script, OcrScript::Multilingual);

    let model_dir = OcrScript::resolve_model_dir();
    assert!(model_dir.to_string_lossy().contains("models"));

    let det_path = OcrScript::Multilingual.resolve_det_model_path();
    assert!(det_path.ends_with("PP-OCRv6_small_det.onnx"));

    let rec_path = OcrScript::Multilingual.resolve_rec_model_path();
    assert!(rec_path.ends_with("PP-OCRv6_small_rec.onnx"));

    assert!(OcrScript::Multilingual.is_model_available());
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
}

#[test]
fn test_load_static_slice_onnx() {
    let det_bytes = std::fs::read("models/PP-OCRv6_small_det.onnx").expect("read det");
    let static_bytes: &'static [u8] = Box::leak(det_bytes.into_boxed_slice());
    let model = rten::Model::load_static_slice(static_bytes);
    assert!(model.is_ok(), "load_static_slice failed: {:?}", model.err());
}

#[test]
fn test_pp_ocrv6_engine_init() {
    let det_path = "models/PP-OCRv6_small_det.onnx";
    let rec_path = "models/PP-OCRv6_small_rec.onnx";
    let dict_path = "models/ppocrv6_dict.txt";
    if std::path::Path::new(det_path).exists() && std::path::Path::new(rec_path).exists() {
        let det_model = rten::Model::load_file(det_path).expect("det model loads");
        let rec_model = rten::Model::load_file(rec_path).expect("rec model loads");
        let alphabet = std::fs::read_to_string(dict_path).ok();
        let engine = ocrs::OcrEngine::new(ocrs::OcrEngineParams {
            detection_model: Some(det_model),
            recognition_model: Some(rec_model),
            alphabet,
            ..Default::default()
        });
        assert!(
            engine.is_ok(),
            "ocrs engine initializes with PP-OCRv6 onnx models: {:?}",
            engine.err()
        );
    }
}
