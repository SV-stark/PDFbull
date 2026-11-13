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

#[test]
fn select_text_items_in_range_multi_line_continuous_selection() {
    use pdfbull::ui_gpui::canvas::select_text_items_in_range;

    let items = vec![
        TextItem {
            text: "The".into(),
            x: 10.0,
            y: 20.0,
            width: 25.0,
            height: 12.0,
        },
        TextItem {
            text: "quick".into(),
            x: 40.0,
            y: 20.0,
            width: 35.0,
            height: 12.0,
        },
        TextItem {
            text: "brown".into(),
            x: 80.0,
            y: 20.0,
            width: 40.0,
            height: 12.0,
        },
        TextItem {
            text: "fox".into(),
            x: 125.0,
            y: 20.0,
            width: 25.0,
            height: 12.0,
        },
        // Line 2
        TextItem {
            text: "jumps".into(),
            x: 10.0,
            y: 40.0,
            width: 35.0,
            height: 12.0,
        },
        TextItem {
            text: "over".into(),
            x: 50.0,
            y: 40.0,
            width: 30.0,
            height: 12.0,
        },
        TextItem {
            text: "the".into(),
            x: 85.0,
            y: 40.0,
            width: 25.0,
            height: 12.0,
        },
        TextItem {
            text: "lazy".into(),
            x: 115.0,
            y: 40.0,
            width: 30.0,
            height: 12.0,
        },
        // Line 3
        TextItem {
            text: "dog".into(),
            x: 10.0,
            y: 60.0,
            width: 25.0,
            height: 12.0,
        },
        TextItem {
            text: "today".into(),
            x: 40.0,
            y: 60.0,
            width: 35.0,
            height: 12.0,
        },
    ];

    // Drag from "brown" on Line 1 (x: 80, y: 22) down to "over" on Line 2 (x: 75, y: 45)
    let selected = select_text_items_in_range(&items, 80.0, 22.0, 75.0, 45.0);
    let texts: Vec<&str> = selected.iter().map(|it| it.text.as_str()).collect();
    // Line 1 from "brown" onwards: "brown", "fox"
    // Line 2 up to "over": "jumps", "over"
    assert_eq!(texts, vec!["brown", "fox", "jumps", "over"]);
    assert_eq!(join_selected_text_items(&selected), "brown fox\njumps over");

    // Drag backward from "over" on Line 2 back to "brown" on Line 1
    let backward = select_text_items_in_range(&items, 75.0, 45.0, 80.0, 22.0);
    let backward_texts: Vec<&str> = backward.iter().map(|it| it.text.as_str()).collect();
    assert_eq!(backward_texts, vec!["brown", "fox", "jumps", "over"]);

    // Drag across 3 lines: from "quick" on Line 1 down to "dog" on Line 3
    let three_lines = select_text_items_in_range(&items, 40.0, 20.0, 25.0, 65.0);
    let three_texts: Vec<&str> = three_lines.iter().map(|it| it.text.as_str()).collect();
    assert_eq!(
        three_texts,
        vec![
            "quick", "brown", "fox", "jumps", "over", "the", "lazy", "dog"
        ]
    );
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

fn make_word(text: &str, x: f32, y: f32, width: f32, height: f32) -> TextItem {
    TextItem {
        text: text.to_string(),
        x,
        y,
        width,
        height,
    }
}

#[test]
fn select_text_items_in_range_isolates_same_column_and_allows_cross_column() {
    use pdfbull::ui_gpui::canvas::select_text_items_in_range;

    let items = vec![
        // Left column (x: 40..200)
        make_word("Left1", 40.0, 20.0, 40.0, 12.0),
        make_word("Alpha", 90.0, 20.0, 45.0, 12.0),
        make_word("Left2", 40.0, 40.0, 40.0, 12.0),
        make_word("Beta", 90.0, 40.0, 40.0, 12.0),
        make_word("Left3", 40.0, 60.0, 40.0, 12.0),
        make_word("Gamma", 90.0, 60.0, 50.0, 12.0),
        // Right column across a wide gutter (x: 280..420)
        make_word("Right1", 280.0, 20.0, 45.0, 12.0),
        make_word("One", 335.0, 20.0, 35.0, 12.0),
        make_word("Right2", 280.0, 40.0, 45.0, 12.0),
        make_word("Two", 335.0, 40.0, 35.0, 12.0),
        make_word("Right3", 280.0, 60.0, 45.0, 12.0),
        make_word("Three", 335.0, 60.0, 45.0, 12.0),
    ];

    // Drag strictly inside Left column across 3 lines: must not select Right column.
    let left_only = select_text_items_in_range(&items, 60.0, 24.0, 110.0, 64.0);
    let left_texts: Vec<&str> = left_only.iter().map(|it| it.text.as_str()).collect();
    assert_eq!(
        left_texts,
        vec!["Left1", "Alpha", "Left2", "Beta", "Left3", "Gamma"]
    );

    // Drag strictly inside Right column across 3 lines: must not select Left column.
    let right_only = select_text_items_in_range(&items, 300.0, 24.0, 350.0, 64.0);
    let right_texts: Vec<&str> = right_only.iter().map(|it| it.text.as_str()).collect();
    assert_eq!(
        right_texts,
        vec!["Right1", "One", "Right2", "Two", "Right3", "Three"]
    );

    // Drag intentionally crossing the gutter from Left column to Right column.
    let crossed = select_text_items_in_range(&items, 95.0, 24.0, 340.0, 44.0);
    let crossed_texts: Vec<&str> = crossed.iter().map(|it| it.text.as_str()).collect();
    assert_eq!(
        crossed_texts,
        vec!["Alpha", "Right1", "One", "Left2", "Beta", "Right2", "Two"]
    );
}

#[test]
fn select_text_items_in_range_ignores_empty_margin_and_whitespace_drags() {
    use pdfbull::ui_gpui::canvas::select_text_items_in_range;

    let items = vec![
        make_word("Line", 50.0, 30.0, 30.0, 12.0),
        make_word("One", 88.0, 30.0, 30.0, 12.0),
        make_word("Line", 50.0, 50.0, 30.0, 12.0),
        make_word("Two", 88.0, 50.0, 30.0, 12.0),
    ];

    // Empty right margin across both lines
    assert!(select_text_items_in_range(&items, 300.0, 32.0, 420.0, 56.0).is_empty());
    // Empty left margin across both lines
    assert!(select_text_items_in_range(&items, 5.0, 32.0, 25.0, 56.0).is_empty());
    // Empty vertical whitespace far below all lines
    assert!(select_text_items_in_range(&items, 40.0, 300.0, 130.0, 360.0).is_empty());
    // Empty vertical whitespace far above all lines
    assert!(select_text_items_in_range(&items, 40.0, 0.0, 130.0, 10.0).is_empty());
}

#[test]
fn select_text_items_in_range_keeps_superscripts_on_parent_line() {
    use pdfbull::ui_gpui::canvas::select_text_items_in_range;

    let items = vec![
        make_word("x", 40.0, 100.0, 8.0, 12.0),
        make_word("2", 48.0, 96.0, 6.0, 7.0),
        make_word("+", 56.0, 100.0, 8.0, 12.0),
        make_word("y", 66.0, 100.0, 8.0, 12.0),
        make_word("next", 40.0, 125.0, 30.0, 12.0),
    ];

    let selected = select_text_items_in_range(&items, 38.0, 106.0, 76.0, 106.0);
    let texts: Vec<&str> = selected.iter().map(|it| it.text.as_str()).collect();
    assert_eq!(texts, vec!["x", "2", "+", "y"]);
    assert_eq!(join_selected_text_items(&selected), "x2+y");
}

#[test]
fn viewport_selection_page_range_stays_anchored_across_scroll_zoom_and_rotation() {
    use pdfbull::ui_gpui::canvas::DocumentViewport;

    let mut vp = DocumentViewport::new();
    vp.zoom = 1.5;
    vp.rotation = 0;
    vp.set_page_sizes(&[pdfbull::models::PageInfo {
        width: 400.0,
        height: 600.0,
        rotation: 0,
    }]);

    // Initial painted card origin on screen
    if let Ok(mut origins) = vp.page_origins.write() {
        origins.insert(
            0,
            gpui_kit::Point::new(gpui_kit::px(100.0), gpui_kit::px(120.0)),
        );
    }

    let click_win = gpui_kit::Point::new(
        gpui_kit::px(100.0 + 50.0 * 1.5),
        gpui_kit::px(120.0 + 100.0 * 1.5),
    );
    let drag_win = gpui_kit::Point::new(
        gpui_kit::px(100.0 + 180.0 * 1.5),
        gpui_kit::px(120.0 + 140.0 * 1.5),
    );
    let p_start = vp
        .window_point_to_overlay(0, click_win)
        .expect("start overlay");
    let p_end = vp
        .window_point_to_overlay(0, drag_win)
        .expect("end overlay");
    assert!((p_start.0 - 50.0).abs() < 0.01 && (p_start.1 - 100.0).abs() < 0.01);
    assert!((p_end.0 - 180.0).abs() < 0.01 && (p_end.1 - 140.0).abs() < 0.01);

    // Simulate user scrolling 80px down and zooming to 2.0x with 90deg view rotation.
    vp.zoom = 2.0;
    vp.rotation = 90;
    if let Ok(mut origins) = vp.page_origins.write() {
        origins.insert(
            0,
            gpui_kit::Point::new(gpui_kit::px(100.0), gpui_kit::px(40.0)),
        );
    }

    // Anchored start point (50, 100) must map to the scrolled, zoomed, rotated pixel:
    // 90deg unrotated (50, 100) on 400x600 -> (600 - 100, 50) = (500, 50) in rotated pts.
    let new_win = vp
        .overlay_point_to_window(0, p_start.0, p_start.1)
        .expect("round-trip rotated scrolled point");
    assert!((f32::from(new_win.x) - (100.0 + 500.0 * 2.0)).abs() < 0.05);
    assert!((f32::from(new_win.y) - (40.0 + 50.0 * 2.0)).abs() < 0.05);
    let back = vp.window_point_to_overlay(0, new_win).expect("inverse");
    assert!((back.0 - p_start.0).abs() < 0.05 && (back.1 - p_start.1).abs() < 0.05);
}

fn build_pdf_with_user_unit_and_flipped_span() -> Vec<u8> {
    let stream =
        b"BT\n/F1 12 Tf\n1 0 0 1 100 400 Tm\n(Hello) Tj\n-1 0 0 1 250 300 Tm\n(Flip) Tj\nET\n";
    let obj1 = b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n".to_vec();
    let obj2 = b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n".to_vec();
    let obj3 = b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [50 100 350 500] /UserUnit 2.0 /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>\nendobj\n".to_vec();
    let obj4 =
        b"4 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n".to_vec();
    let mut obj5 = format!("5 0 obj\n<< /Length {} >>\nstream\n", stream.len()).into_bytes();
    obj5.extend_from_slice(stream);
    obj5.extend_from_slice(b"\nendstream\nendobj\n");

    let mut pdf = b"%PDF-1.6\n".to_vec();
    let mut offsets = Vec::new();
    for obj in [&obj1, &obj2, &obj3, &obj4, &obj5] {
        offsets.push(pdf.len());
        pdf.extend_from_slice(obj);
    }
    let xref_pos = pdf.len();
    pdf.extend_from_slice(b"xref\n0 6\n0000000000 65535 f \n");
    for off in offsets {
        pdf.extend_from_slice(format!("{off:010} 00000 n \n").as_bytes());
    }
    pdf.extend_from_slice(
        format!("trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n{xref_pos}\n%%EOF\n").as_bytes(),
    );
    pdf
}

#[test]
fn extract_text_items_handles_user_unit_and_negative_advance_spans() {
    use pdfbull::pdf_engine::{DocumentStore, create_render_cache};

    let dir = tempfile::tempdir().expect("tempdir");
    let pdf_path = dir.path().join("user_unit_and_flip.pdf");
    std::fs::write(&pdf_path, build_pdf_with_user_unit_and_flipped_span()).expect("write test pdf");

    let mut store = DocumentStore::new(create_render_cache(4, 16 * 1024 * 1024));
    let doc_id = next_doc_id();
    let path_str = pdf_path.to_string_lossy().to_string();
    let open = store
        .open_document(&path_str, None, doc_id)
        .expect("open pdf");
    assert_eq!(open.page_count, 1);

    let items = store.extract_text_items(doc_id, 0).expect("extract items");
    let hello = items
        .iter()
        .find(|it| it.text.contains("Hello"))
        .expect("Hello span must be extracted");
    let flip = items
        .iter()
        .find(|it| it.text.contains("Flip"))
        .expect("Flip span must be extracted");

    // With MediaBox [50,100,350,500] and UserUnit 2.0, page width is 600 and
    // height is 800; Hello at (100, 400) sits inside page bounds near (100, 180.8).
    assert!(
        hello.x >= 99.0 && hello.x <= 101.0,
        "Hello x must reflect UserUnit=2.0 scaled MediaBox origin: got {}",
        hello.x
    );
    assert!(
        (hello.height - 24.0).abs() < 0.2,
        "12pt * UserUnit 2.0 should give height ~24: got {}",
        hello.height
    );
    assert!(
        hello.y > 0.0 && hello.y + hello.height < open.page_heights[0],
        "Hello must lie strictly inside physical page height {}: {:?}",
        open.page_heights[0],
        hello
    );

    // Mirrored `-1 0 0 1 250 300 Tm` has negative advance starting at physical x=500
    // (page-relative 400): left edge must sit to the LEFT of 400, not at 400!
    assert!(
        flip.width > 0.0,
        "negative-advance span must expose positive width: {:?}",
        flip
    );
    assert!(
        flip.x < 400.0 && (flip.x + flip.width - 400.0).abs() < 1.0,
        "negative-advance span must extend leftward from anchor x=400: {:?}",
        flip
    );

    // Search hits must align in the exact same UserUnit-scaled coordinate space.
    let hits = store.search(doc_id, "Hello").expect("search Hello");
    assert!(!hits.is_empty(), "search must find Hello");
    let hit = &hits[0];
    assert!(
        (hit.x - hello.x).abs() < 2.0 && (hit.y - hello.y).abs() < 4.0,
        "search hit ({}, {}) must align with extract_text_items ({}, {})",
        hit.x,
        hit.y,
        hello.x,
        hello.y
    );
}
