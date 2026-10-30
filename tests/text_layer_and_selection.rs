use pdfbull::commands::PdfCommand;
use pdfbull::engine::spawn_engine_thread;
use pdfbull::models::TextItem;
use pdfbull::models::next_doc_id;
use pdfbull::ui_gpui::canvas::join_selected_text_items;
use std::path::PathBuf;
use tokio::sync::oneshot;

#[test]
fn join_selected_text_items_coalesces_glyph_strokes_into_words() {
    // Per-glyph spans must read like words, not "T e s t".
    let items = vec![
        TextItem {
            text: "T".into(),
            x: 0.0,
            y: 0.0,
            width: 5.0,
            height: 10.0,
        },
        TextItem {
            text: "e".into(),
            x: 5.2,
            y: 0.0,
            width: 5.0,
            height: 10.0,
        },
        TextItem {
            text: "s".into(),
            x: 10.3,
            y: 0.0,
            width: 4.0,
            height: 10.0,
        },
        TextItem {
            text: "t".into(),
            x: 14.4,
            y: 0.0,
            width: 3.5,
            height: 10.0,
        },
        TextItem {
            text: "Page".into(),
            x: 21.0,
            y: 0.0,
            width: 24.0,
            height: 10.0,
        },
    ];
    assert_eq!(join_selected_text_items(&items), "Test Page");
}

#[test]
fn join_selected_text_items_preserves_word_boundaries_for_word_spans() {
    let items = vec![
        TextItem {
            text: "Hello".into(),
            x: 10.0,
            y: 20.0,
            width: 40.0,
            height: 12.0,
        },
        TextItem {
            text: "World".into(),
            x: 60.0,
            y: 20.0,
            width: 45.0,
            height: 12.0,
        },
    ];
    assert_eq!(join_selected_text_items(&items), "Hello World");
}

#[test]
fn join_selected_text_items_splits_lines_on_baseline_change() {
    let items = vec![
        TextItem {
            text: "Top".into(),
            x: 0.0,
            y: 0.0,
            width: 20.0,
            height: 10.0,
        },
        TextItem {
            text: "Bottom".into(),
            x: 0.0,
            y: 20.0,
            width: 30.0,
            height: 10.0,
        },
    ];
    assert_eq!(join_selected_text_items(&items), "Top\nBottom");
}

#[test]
fn join_selected_text_items_ignores_empty_input() {
    let items: Vec<TextItem> = vec![];
    assert_eq!(join_selected_text_items(&items), "");
}

#[test]
fn join_selected_text_items_handles_baseline_jitter_on_same_line() {
    // Glyphs or words on the same line frequently have minor floating-point
    // baseline variance (e.g. 49.9 vs 50.0). Line clustering must keep them
    // on the same line and sort horizontally by x rather than inverting them.
    let items = vec![
        TextItem {
            text: "World".into(),
            x: 60.0,
            y: 49.9,
            width: 45.0,
            height: 12.0,
        },
        TextItem {
            text: "Hello".into(),
            x: 10.0,
            y: 50.0,
            width: 40.0,
            height: 12.0,
        },
    ];
    assert_eq!(join_selected_text_items(&items), "Hello World");
}

#[test]
fn text_selection_highlight_coordinate_mapping_matches_items() {
    use pdfbull::ui_gpui::canvas::DocumentViewport;
    let mut viewport = DocumentViewport::new();
    viewport.zoom = 1.25;
    viewport.total_pages = 2;

    let items = vec![
        TextItem {
            text: "Rust".into(),
            x: 50.0,
            y: 40.0,
            width: 30.0,
            height: 14.0,
        },
        TextItem {
            text: "Highlighter".into(),
            x: 85.0,
            y: 40.0,
            width: 70.0,
            height: 14.0,
        },
        TextItem {
            text: "Ignored".into(),
            x: 50.0,
            y: 200.0,
            width: 50.0,
            height: 14.0,
        },
    ];
    viewport
        .text_cache
        .insert(0, std::sync::Arc::new(items.clone()));

    let card_origin = gpui_kit::Point::new(gpui_kit::px(350.0), gpui_kit::px(120.0));
    viewport
        .page_origins
        .write()
        .unwrap()
        .insert(0, card_origin);

    let win_start = gpui_kit::Point::new(gpui_kit::px(410.0), gpui_kit::px(168.0));
    let win_end = gpui_kit::Point::new(gpui_kit::px(550.0), gpui_kit::px(190.0));

    let origin = viewport
        .page_origins
        .read()
        .unwrap()
        .get(&0)
        .copied()
        .unwrap();
    let min_x = ((win_start.x.min(win_end.x) - origin.x) / gpui_kit::px(1.0)) / viewport.zoom;
    let max_x = ((win_start.x.max(win_end.x) - origin.x) / gpui_kit::px(1.0)) / viewport.zoom;
    let min_y = ((win_start.y.min(win_end.y) - origin.y) / gpui_kit::px(1.0)) / viewport.zoom;
    let max_y = ((win_start.y.max(win_end.y) - origin.y) / gpui_kit::px(1.0)) / viewport.zoom;

    let mut matched = Vec::new();
    for item in &items {
        let ix2 = item.x + item.width;
        let iy2 = item.y + item.height;
        if ix2 >= min_x && item.x <= max_x && iy2 >= min_y && item.y <= max_y {
            matched.push(item.text.clone());
        }
    }
    assert_eq!(matched, vec!["Rust", "Highlighter"]);

    // The joined selection text must not contain the unselected word.
    viewport.selected_text = Some(join_selected_text_items(
        &matched
            .iter()
            .map(|t| items.iter().find(|i| &i.text == t).cloned().unwrap())
            .collect::<Vec<_>>(),
    ));
    assert_eq!(viewport.selected_text.as_deref(), Some("Rust Highlighter"));
}

#[tokio::test]
async fn text_layer_for_real_document_is_nonempty_and_coalesces() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fixture_pdf = manifest_dir.join("tests").join("test_document.pdf");
    if !fixture_pdf.exists() {
        return;
    }

    let engine = spawn_engine_thread(16, 128);
    let doc_id = next_doc_id();
    let (open_tx, open_rx) = oneshot::channel();
    let path_str = fixture_pdf.to_string_lossy().to_string();
    engine
        .cmd_tx
        .send(PdfCommand::Open(path_str, None, doc_id, open_tx))
        .await
        .expect("send Open");
    open_rx.await.expect("open rx").expect("open ok");

    let (text_tx, text_rx) = oneshot::channel();
    engine
        .cmd_tx
        .send(PdfCommand::GetTextItems(doc_id, 0, text_tx))
        .await
        .expect("send GetTextItems");
    let items = text_rx.await.expect("items rx").expect("items ok");

    assert!(!items.is_empty(), "text layer must expose glyphs");
    for item in &items {
        assert!(
            !item.text.trim().is_empty(),
            "TextItem text must not be blank"
        );
        assert!(item.width > 0.0, "TextItem width must be positive");
        assert!(item.height > 0.0, "TextItem height must be positive");
        assert!(
            item.x.is_finite() && item.y.is_finite(),
            "coordinates must be finite"
        );
    }

    // Per-glyph spans should coalesce back into the page's words.
    let joined = join_selected_text_items(&items);
    assert!(
        joined.contains("Test"),
        "joined text must contain Test: {joined:?}"
    );
    assert!(
        joined.contains("Page"),
        "joined text must contain Page: {joined:?}"
    );
    // Coalescing must collapse the per-glyph spans (it should not be a
    // space-separated run of single letters).
    assert_ne!(
        joined,
        items
            .iter()
            .map(|i| i.text.clone())
            .collect::<Vec<_>>()
            .join(" ")
    );

    let _ = engine.cmd_tx.send(PdfCommand::Close(doc_id)).await;
}

#[tokio::test]
async fn test_hp_pdf_text_extraction() {
    let hp_path = PathBuf::from("/mnt/e/ebook-rs/hp.pdf");
    if !hp_path.exists() {
        return;
    }

    let engine = spawn_engine_thread(16, 128);
    let doc_id = next_doc_id();
    let (open_tx, open_rx) = oneshot::channel();
    let path_str = hp_path.to_string_lossy().to_string();
    engine
        .cmd_tx
        .send(PdfCommand::Open(path_str, None, doc_id, open_tx))
        .await
        .expect("send Open");
    let open_res = open_rx.await.expect("open rx").expect("open ok");
    println!(
        "HP.pdf open: page_count={}, max_w={}, heights={:?}, rotations={:?}",
        open_res.page_count,
        open_res.max_width,
        &open_res.page_heights[..open_res.page_heights.len().min(3)],
        &open_res.page_rotations[..open_res.page_rotations.len().min(3)]
    );

    for p in 0..open_res.page_count.min(3) {
        let (text_tx, text_rx) = oneshot::channel();
        engine
            .cmd_tx
            .send(PdfCommand::GetTextItems(doc_id, p, text_tx))
            .await
            .expect("send GetTextItems");
        let items = text_rx.await.expect("items rx").expect("items ok");
        println!("HP.pdf page {} text_items count: {}", p, items.len());
        if !items.is_empty() {
            println!(
                "First 5 items on page {}: {:?}",
                p,
                &items[..items.len().min(5)]
            );
            let joined = join_selected_text_items(&items);
            println!(
                "Page {} joined text sample (first 100 chars): {:?}",
                p,
                &joined[..joined.len().min(100)]
            );
        }
    }

    let _ = engine.cmd_tx.send(PdfCommand::Close(doc_id)).await;
}
