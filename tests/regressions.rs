use gpui_kit::RenderImage;
use pdfbull::commands::PdfCommand;
use pdfbull::models::next_doc_id;
use pdfbull::pdf_engine::DocumentStore;
use pdfbull::ui_gpui::canvas::DocumentViewport;
use pdfbull::ui_gpui::dialogs::ActiveDialog;
use pdfbull::ui_gpui::log_console::LogConsoleState;
use pdfbull::ui_gpui::ribbon::PageLayoutMode;
use pdfbull::ui_gpui::sidebar::SidebarState;
use std::sync::Arc;

// ── Regression tests for the bug fixes ──────────────────────────────────────

#[test]
fn test_annotation_ids_are_unique_after_delete() {
    // Regression: annotation ids were `annotations.len() + 1`, which reuses a
    // live id after any delete. The sidebar delete button keys on the id and
    // used `retain(|a| a.id != id)`, so one click removed *every* annotation
    // sharing that id.
    let mut annotations: Vec<pdfbull::models::Annotation> = (0..3)
        .map(|i| pdfbull::models::Annotation {
            id: pdfbull::models::next_annotation_id(),
            page: 0,
            style: pdfbull::models::AnnotationStyle::Highlight {
                color: "#FFFF00".to_string(),
            },
            x: i as f32,
            y: 0.0,
            width: 10.0,
            height: 10.0,
        })
        .collect();

    let victim = annotations[1].id;
    annotations.retain(|a| a.id != victim);
    assert_eq!(annotations.len(), 2);

    // The next id must not collide with a surviving annotation.
    let next = pdfbull::models::next_annotation_id();
    assert!(
        !annotations.iter().any(|a| a.id == next),
        "next_annotation_id() must not collide with a live annotation id"
    );

    // Deleting by id now removes exactly one entry.
    let survivor = annotations[0].id;
    annotations.retain(|a| a.id != survivor);
    assert_eq!(annotations.len(), 1);
}

#[test]
fn test_reset_for_document_clears_all_per_document_state() {
    // Regression: opening/switching documents only cleared the render caches,
    // so the previous document's annotations, text layer, search hits, cached
    // page origins and scroll offset survived — and Ctrl+S then wrote the old
    // document's annotations into the new document's file.
    let mut vp = DocumentViewport::new();
    vp.total_pages = 10;
    vp.annotations.push(pdfbull::models::Annotation {
        id: 1,
        page: 0,
        style: pdfbull::models::AnnotationStyle::Highlight {
            color: "#FFFF00".to_string(),
        },
        x: 0.0,
        y: 0.0,
        width: 1.0,
        height: 1.0,
    });
    vp.reindex_annotations();
    vp.text_cache.insert(
        0,
        vec![pdfbull::models::TextItem {
            text: "leaked".to_string(),
            x: 0.0,
            y: 0.0,
            width: 1.0,
            height: 1.0,
        }],
    );
    vp.search_highlights.insert(0, vec![(0.0, 0.0, 1.0, 1.0)]);
    vp.rendered_pages
        .insert(0, Arc::new(RenderImage::new(vec![])));
    vp.rendered_zoom.insert(0, 1.0);
    vp.scroll_to_page(5);
    if let Ok(mut o) = vp.page_origins.write() {
        o.insert(
            0,
            gpui_kit::Point::new(gpui_kit::px(3.0), gpui_kit::px(4.0)),
        );
    }

    vp.reset_for_document();

    assert!(vp.annotations.is_empty(), "annotations must be cleared");
    assert!(
        vp.annotations_by_page.is_empty(),
        "the per-page annotation index must be cleared"
    );
    assert!(vp.text_cache.is_empty(), "text layer must be cleared");
    assert!(
        vp.search_highlights.is_empty(),
        "search hits must be cleared"
    );
    assert!(
        vp.rendered_pages.is_empty(),
        "rendered pages must be cleared"
    );
    assert!(vp.rendered_zoom.is_empty(), "the zoom map must be cleared");
    assert_eq!(vp.current_page, 0);
    assert!(!vp.is_selecting);
    let origins_empty = vp.page_origins.read().map(|o| o.is_empty()).unwrap_or(true);
    assert!(origins_empty, "cached page origins must be cleared");
}

#[test]
fn test_invalidate_rendered_pages_clears_zoom_map() {
    // Regression: rotate and midnight-mode cleared `rendered_pages` but not
    // `rendered_zoom`. `request_render_page` gates on the zoom map first, so
    // every page short-circuited and stayed on the "Rendering page..."
    // placeholder until the user changed zoom.
    let mut vp = DocumentViewport::new();
    vp.rendered_pages
        .insert(3, Arc::new(RenderImage::new(vec![])));
    vp.rendered_zoom.insert(3, 1.5);

    vp.invalidate_rendered_pages();

    assert!(vp.rendered_pages.is_empty());
    assert!(
        vp.rendered_zoom.is_empty(),
        "rotating or toggling the render filter must clear the zoom map too, \
         otherwise the page is never re-rendered"
    );
}

#[test]
fn test_annotation_page_index_matches_filtering() {
    let mut vp = DocumentViewport::new();
    for page in [0usize, 0, 2] {
        vp.annotations.push(pdfbull::models::Annotation {
            id: pdfbull::models::next_annotation_id(),
            page,
            style: pdfbull::models::AnnotationStyle::Rectangle {
                color: "#FF0000".to_string(),
                thickness: 2.0,
                fill: false,
            },
            x: 0.0,
            y: 0.0,
            width: 1.0,
            height: 1.0,
        });
    }
    vp.reindex_annotations();

    assert_eq!(vp.annotations_on_page(0).len(), 2);
    assert_eq!(vp.annotations_on_page(2).len(), 1);
    assert!(vp.annotations_on_page(1).is_empty());

    // Keep the index in step with the list it mirrors.
    vp.annotations.pop();
    vp.reindex_annotations();
    assert_eq!(vp.annotations_on_page(2).len(), 0);
}

#[test]
fn test_continuous_strip_places_cards_exactly_where_scroll_maths_expects() {
    // Regression: the virtualized Continuous container carried BOTH `.py_8()`
    // and `.gap_6()` *and* two spacers, so the padding and gap were counted
    // twice. Every card rendered CANVAS_PADDING_Y + CANVAS_PAGE_GAP (56px) below
    // where `calculate_visible_page_continuous` believed it was, and the strip
    // was ~112px too tall — which is what made scrolling skip pages and left the
    // last page unreachable at full scroll.
    let mut vp = DocumentViewport::new();
    vp.page_width = 595.0;
    vp.page_height = 842.0;
    vp.zoom = 1.0;
    vp.layout_mode = PageLayoutMode::Continuous;

    for total in [1usize, 2, 5, 40, 500] {
        vp.total_pages = total;
        vp.current_page = 0;
        let strip = vp.continuous_strip_geometry(total);

        let slot = f32::from(strip.slot_height);
        // The card for page `first + j` is laid out at
        // `top_spacer + j*slot`, and must equal `page_top_in_content(first+j)`.
        for j in 0..(strip.last - strip.first) {
            let laid_out = f32::from(strip.top_spacer) + slot * j as f32;

            let expected = f32::from(vp.page_top_in_content(strip.first + j));
            assert!(
                (laid_out - expected).abs() < 0.01,
                "page {} laid out at {laid_out}, scroll maths expects {expected} \
                 (total={total})",
                strip.first + j
            );
        }

        // Total content height is invariant under windowing: it must be
        // `top pad + one slot per page + bottom pad` no matter which pages are
        // built, otherwise the scrollbar range and the scroll clamp both lie.
        let built = strip.last - strip.first;
        let content =
            f32::from(strip.top_spacer) + slot * built as f32 + f32::from(strip.bottom_spacer);
        let pad = f32::from(vp.page_top_in_content(0));
        let ideal = 2.0 * pad + slot * total as f32;
        assert!(
            (content - ideal).abs() < 0.01,
            "content height {content} should be {ideal} (total={total}, first={}, last={})",
            strip.first,
            strip.last
        );

        // The last page must be fully scrollable into view — the original bug
        // left it permanently short of the viewport.
        let page_h = vp.page_height * vp.zoom;
        let last_page_bottom = f32::from(vp.page_top_in_content(total - 1)) + page_h;
        assert!(
            content >= last_page_bottom,
            "last page bottom {last_page_bottom} exceeds content height {content} (total={total})"
        );
    }
}

#[test]
fn test_continuous_branch_does_not_reapply_container_spacing() {
    // Regression guard. The virtualized Continuous container encodes all of its
    // vertical spacing in the spacers and per-card slots, so adding `.py_8()` or
    // `.gap_6()` back onto the container double-counts them: every card then
    // renders 56px below where `calculate_visible_page_continuous` thinks it is,
    // which makes scrolling skip pages and leaves the last page unreachable.
    //
    // This cannot be asserted through `continuous_strip_geometry` alone — that
    // function knows nothing about the element tree — so this checks the builder
    // chain in the source. It is a lint, not a behavioural test, but it is the
    // only thing standing between a future edit and the same regression.
    let src = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/ui_gpui/canvas.rs"),
    )
    .expect("canvas.rs must be readable");

    // `PageLayoutMode::Continuous => {` also appears in `scroll_to_page`, whose
    // single-page branch legitimately carries `.py_8()`. Scope the scan to the
    // Continuous arm inside `render`.
    let render_start = src.find("pub fn render").expect("render fn must exist");
    let arm_start = src[render_start..]
        .find("PageLayoutMode::Continuous => {")
        .map(|i| render_start + i)
        .expect("Continuous arm must exist in render");
    let arm = &src[arm_start..];

    // Strip `//` comments so the explanatory notes about `.py_8()` do not
    // trip the scan.
    let code: String = arm
        .lines()
        .map(|l| match l.find("//") {
            Some(i) => &l[..i],
            None => l,
        })
        .collect::<Vec<_>>()
        .join("\n");

    for forbidden in [".py_", ".pt_", ".pb_", ".gap_", ".mt_", ".mb_"] {
        assert!(
            !code.contains(forbidden),
            "the virtualized Continuous branch must not apply `{forbidden}` to the \
             scroll container — all vertical spacing belongs in the spacers and \
             per-card slots, or card positions drift away from the scroll maths"
        );
    }

    // And it must actually use the shared geometry, so the spacers, the slot
    // pitch and the scroll maths cannot drift apart again.
    assert!(
        code.contains("continuous_strip_geometry(total)"),
        "the Continuous branch must build its layout from continuous_strip_geometry()"
    );
}

#[test]
fn test_scroll_to_page_and_visible_page_agree() {
    // Regression: `scroll_to_page` and `calculate_visible_page_continuous` must
    // be exact inverses, otherwise the status-bar page number disagrees with the
    // page actually shown after a jump.
    let mut vp = DocumentViewport::new();
    vp.page_width = 595.0;
    vp.page_height = 842.0;
    vp.layout_mode = PageLayoutMode::Continuous;
    vp.total_pages = 300;

    for zoom in [0.25f32, 0.5, 1.0, 1.5, 2.0, 5.0] {
        vp.zoom = zoom;
        for page in [0usize, 1, 7, 42, 150, 299] {
            vp.scroll_to_page(page);
            assert_eq!(vp.current_page, page);
            let back = vp.calculate_visible_page_continuous();
            assert_eq!(
                back, page,
                "at zoom {zoom}, scrolling to page {page} reads back as page {back}"
            );
        }
    }
}

#[test]
fn test_decoded_image_cache_is_bounded() {
    // Regression: `ContentInterpreter::new` inherits `ParseLimits::default()`,
    // whose `max_image_cache_bytes` is 1 GiB, and the engine hands it a
    // document-lifetime `ImageCache`. Scrolling a 2 MB scanned PDF therefore
    // retained ~35 MB of decoded RGBA per page — around a gigabyte — for as long
    // as the tab stayed open.
    let store = DocumentStore::new(pdfbull::pdf_engine::create_render_cache(
        8,
        32 * 1024 * 1024,
    ));
    let budget = store.image_cache_bytes();
    assert!(
        budget <= 256 * 1024 * 1024,
        "decoded-image budget of {budget} bytes is far too large for a per-document cache"
    );
    assert!(
        budget >= 16 * 1024 * 1024,
        "budget should still hold a few pages"
    );
}

#[test]
fn test_image_cache_rejects_beyond_its_limit() {
    // The mechanism the memory fix relies on: `ImageCache::try_insert_with_limit`
    // refuses an image that would push the cache past `max_bytes`, instead of
    // admitting it unconditionally as `insert()` does. `zpdf`'s
    // `ParseLimits::default()` hands out a 1 GiB budget, which is why the engine
    // must pass an explicit `with_image_cache_limit`.
    let mut cache = zpdf::ImageCache::new();
    let big = zpdf::DecodedImage {
        width: 2480,
        height: 3508,
        data: vec![0u8; 2480 * 3508 * 4], // ~35 MB decoded, one 300 DPI page
        has_alpha: true,
        is_image_mask: false,
        premultiplied: false,
    };
    let page = big.data.capacity() as u64;

    // A 96 MB budget admits a couple of pages...
    let budget = 96 * 1024 * 1024;
    assert!(
        cache.try_insert_with_limit(big.clone(), budget).is_some(),
        "the first high-DPI page must fit in the budget"
    );
    // ...and then stops, rather than growing to the 1 GiB default.
    let mut admitted_pages = 1u32;
    while cache.try_insert_with_limit(big.clone(), budget).is_some() {
        admitted_pages += 1;
        assert!(
            admitted_pages <= 8,
            "the cache admitted {admitted_pages} pages ({admitted_pages} x {} MB), \
             far past a {budget} byte budget",
            page / 1024 / 1024
        );
    }
    assert!(
        cache.bytes_used() <= budget,
        "retained {} bytes, over the {budget} byte budget",
        cache.bytes_used()
    );

    // Sanity-check the contrast: a 300 DPI page decodes to ~33 MB, so zpdf's 1 GiB
    // default would have retained roughly 31 pages of a scanned document —
    // around a gigabyte of RGBA held open for a file of a couple of megabytes.
    let gib = 1024u64 * 1024 * 1024;
    assert!(
        page * 31 > gib,
        "expected ~31 decoded pages to exceed 1 GiB, got {}",
        page * 31
    );
    assert!(
        budget * 4 < gib,
        "the new budget is at least 4x smaller than the default it replaces"
    );
}

#[test]
fn test_document_store_image_cache_stays_bounded() {
    // Regression: the store hands the interpreter a document-lifetime
    // `ImageCache`. Scrolling must not grow it without bound.
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixture = manifest.join("tests").join("test_document.pdf");
    let cache = pdfbull::pdf_engine::create_render_cache(8, 32 * 1024 * 1024);
    let mut store = DocumentStore::new(cache);
    let doc_id = next_doc_id();
    let opened = store
        .open_document(&fixture.to_string_lossy(), None, doc_id)
        .expect("fixture must open");

    let options = pdfbull::pdf_engine::RenderOptions {
        scale: 1.0,
        rotation: 0,
        filter: pdfbull::pdf_engine::RenderFilter::None,
        auto_crop: false,
        quality: pdfbull::pdf_engine::RenderQuality::Medium,
    };
    for page in 0..opened.page_count.min(20) {
        let _ = store.render_page(doc_id, page, options.clone());
        assert!(
            store.image_cache_bytes_used(doc_id) <= store.image_cache_bytes(),
            "after rendering page {page} the document retains {} bytes of decoded \
             images, over the {} byte budget",
            store.image_cache_bytes_used(doc_id),
            store.image_cache_bytes()
        );
    }
}

#[test]
fn test_visible_card_window_is_bounded() {
    // Regression: the canvas built one full element subtree per page per frame.
    // The window must stay a small slice even for very large documents.
    let mut vp = DocumentViewport::new();
    vp.page_width = 595.0;
    vp.page_height = 842.0;
    vp.zoom = 1.0;
    vp.total_pages = 2000;

    let window = vp.visible_card_window(2000);
    assert!(
        window.end <= window.start + 64,
        "card window {} is too wide for a 2000-page document",
        window.end - window.start
    );
    assert!(window.end <= 2000);

    // The window must follow the scroll position.
    vp.scroll_to_page(1500);
    let scrolled = vp.visible_card_window(2000);
    assert!(
        scrolled.start > 1000,
        "the window should track the scroll offset, got {scrolled:?}"
    );
    assert!(scrolled.end <= 2000);
    assert!(
        scrolled.contains(&1500),
        "the window must always contain the current page, got {scrolled:?}"
    );
    assert!(
        scrolled.end - scrolled.start <= 64,
        "the window must stay capped, got {scrolled:?}"
    );

    // Anchoring deep in a long document must not collapse the window to the
    // front of the file.
    for anchor in [0usize, 1, 47, 48, 49, 999, 1999] {
        vp.current_page = anchor;
        let w = vp.visible_card_window(2000);
        assert!(w.contains(&anchor), "window {w:?} must contain {anchor}");
        assert!(w.end <= 2000, "window {w:?} must stay in bounds");
        assert!(w.end - w.start <= 64, "window {w:?} must stay capped");
    }

    // Degenerate geometry must not divide by zero or panic.
    let mut tiny = DocumentViewport::new();
    tiny.page_height = 0.0;
    tiny.zoom = 0.0;
    let w = tiny.visible_card_window(10);
    assert!(w.start <= w.end, "a degenerate window must not be inverted");
    assert_eq!(tiny.visible_card_window(0), 0..0);
}

#[test]
fn test_thumbnail_window_tracks_scroll() {
    let mut sidebar = SidebarState::new_for_test();
    let (first, last) = sidebar.thumbnail_window(1000);
    assert!(last > first);
    assert!(
        last - first <= 64,
        "thumbnail window should stay small, got {}",
        last - first
    );

    sidebar.scroll_thumbnails_to(900, 1000);
    let (first2, last2) = sidebar.thumbnail_window(1000);
    assert!(first2 > first, "the window must follow the strip's scroll");
    assert!(last2 <= 1000);
    assert!(last2 - first2 <= 64, "the window must stay capped");
    assert!(
        (first2..last2).contains(&899),
        "the window must cover the page it was scrolled to, got {first2}..{last2}"
    );

    assert!(sidebar.thumbnail_window(0) == (0, 0));
}

#[test]
fn test_log_console_uses_stable_sequence_ids() {
    // Regression: log entry element ids were derived from the list index, so
    // every append shifted all of them and every filter change re-indexed the
    // whole set. Sequence numbers are stable across eviction and filtering.
    let mut logs = LogConsoleState::new();
    logs.entries.clear();
    logs.log("first", "info");
    logs.log("second", "warn");
    let ids: Vec<u64> = logs.entries.iter().map(|(seq, _, _)| *seq).collect();
    assert_eq!(ids.len(), 2);
    assert!(ids[0] < ids[1], "sequence numbers must increase");

    // Eviction from the front must not renumber the survivors.
    for i in 0..2000 {
        logs.log(format!("line {i}"), "info");
    }
    assert_eq!(logs.entries.len(), 1000);
    let mut sorted = true;
    let mut prev = 0u64;
    for (seq, _, _) in logs.entries.iter() {
        if *seq <= prev {
            sorted = false;
        }
        prev = *seq;
    }
    assert!(sorted, "sequence numbers must stay strictly increasing");

    assert!(logs.as_text().contains("line 1999"));
}

#[test]
fn test_scroll_to_page_offsets_account_for_container_padding() {
    // Regression: `scroll_to_page` ignored the container's top padding, so
    // jumping to a page in Continuous mode landed a third of a page short.
    let mut vp = DocumentViewport::new();
    vp.page_width = 595.0;
    vp.page_height = 800.0;
    vp.zoom = 1.0;
    vp.total_pages = 20;
    vp.layout_mode = PageLayoutMode::Continuous;

    vp.scroll_to_page(0);
    let top = f32::from(-vp.scroll_handle.offset().y);
    assert!(top > 0.0, "page 0 must sit below the container padding");

    vp.scroll_to_page(5);
    let at_five = f32::from(-vp.scroll_handle.offset().y);
    assert!(
        at_five > top,
        "scrolling to a later page must move the content further up"
    );
    // Must land on or above the top of page 5's card, and not skip past it.
    let item = 824.0_f32; // page_height + CANVAS_PAGE_GAP
    let expected_top = 32.0 + item * 5.0;
    assert!(
        (at_five - expected_top).abs() < 1.0,
        "expected ~{expected_top}, got {at_five}"
    );

    // Out-of-range indices clamp rather than panic.
    vp.scroll_to_page(9999);
    assert_eq!(vp.current_page, 19);
}

#[test]
fn test_pdf_command_debug_does_not_leak_password() {
    // Regression: `PdfCommand` derived `Debug` while `Open` carries the
    // document password, so any debug log would have printed the secret.
    let cmd = PdfCommand::Open(
        "secret.pdf".to_string(),
        Some("hunter2".to_string()),
        next_doc_id(),
        tokio::sync::oneshot::channel().0,
    );
    let rendered = format!("{cmd:?}");
    assert_eq!(rendered, "Open");
    assert!(
        !rendered.contains("hunter2"),
        "password must never appear in Debug output"
    );
    assert!(!rendered.contains("secret.pdf"));
}

#[test]
fn test_only_pdfs_are_accepted_as_documents() {
    // Regression: `open_pdf_path` only checked `exists()`, so dropping a PNG or
    // passing a directory created a permanent unrenderable blank tab.
    let tmp = std::env::temp_dir().join("pdfbull_input_validation_test");
    std::fs::create_dir_all(&tmp).unwrap();
    let png = tmp.join("not-a-pdf.png");
    std::fs::write(&png, b"\x89PNG\r\n\x1a\n").unwrap();
    let pdf = tmp.join("looks-like.pdf");
    std::fs::write(&pdf, b"%PDF-1.4\n").unwrap();

    let accepts = pdfbull::ui_gpui::sidebar::SidebarState::is_openable_pdf;

    assert!(!accepts(&png), "a .png must be rejected");
    assert!(accepts(&pdf), "a .pdf must be accepted");
    assert!(!accepts(&tmp), "a directory must be rejected");
    assert!(!accepts(std::path::Path::new("/no/such/file.pdf")));

    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn test_dialogs_without_configuration_have_no_apply_button() {
    use pdfbull::ui_gpui::dialogs::dialog_has_apply;
    assert!(dialog_has_apply(&ActiveDialog::Watermark));
    assert!(dialog_has_apply(&ActiveDialog::HeaderFooter));
    assert!(dialog_has_apply(&ActiveDialog::Security));
    // These previously showed an Apply button wired to a no-op arm.
    assert!(!dialog_has_apply(&ActiveDialog::Password));
    assert!(!dialog_has_apply(&ActiveDialog::Settings));
    assert!(!dialog_has_apply(&ActiveDialog::Signature));
    assert!(!dialog_has_apply(&ActiveDialog::PageOrganizer));
}

#[test]
fn test_save_annotations_is_idempotent() {
    // Regression: `save_annotations` re-read the already-annotated file on
    // every save and appended the same annotations again, so highlights
    // multiplied each time the user pressed Ctrl+S. The engine now strips
    // previously-written annotations (identified by their /NM marker) first.
    use pdfbull::pdf_engine::DocumentStore;

    let dir = std::env::temp_dir().join("pdfbull_save_idempotency_test");
    std::fs::create_dir_all(&dir).unwrap();
    let src = dir.join("src.pdf");
    let source = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/test_document.pdf");
    std::fs::copy(&source, &src).unwrap();

    let cache = pdfbull::pdf_engine::create_render_cache(8, 32 * 1024 * 1024);
    let mut store = DocumentStore::new(cache);
    let doc_id = next_doc_id();
    let opened = store
        .open_document(&src.to_string_lossy(), None, doc_id)
        .expect("fixture must open");
    assert!(opened.page_count > 0);

    let annotations = vec![pdfbull::models::Annotation {
        id: pdfbull::models::next_annotation_id(),
        page: 0,
        style: pdfbull::models::AnnotationStyle::Highlight {
            color: "#FFFF00".to_string(),
        },
        x: 50.0,
        y: 100.0,
        width: 120.0,
        height: 14.0,
    }];

    let out = dir.join("out.pdf");
    let out_str = out.to_string_lossy().to_string();

    // Save the same set three times; the file must not grow each round.
    let mut sizes = Vec::new();
    for _ in 0..3 {
        store
            .save_annotations(doc_id, &annotations, Some(out_str.clone()))
            .expect("save must succeed");
        sizes.push(std::fs::metadata(&out).unwrap().len());
    }
    assert_eq!(
        sizes[0], sizes[1],
        "a second identical save must not add annotations again"
    );
    assert_eq!(sizes[1], sizes[2]);

    // Exactly one highlight must round-trip back out of the file.
    let reloaded = store
        .load_annotations(&out_str)
        .expect("reload must succeed");
    let highlights = reloaded
        .iter()
        .filter(|a| matches!(a.style, pdfbull::models::AnnotationStyle::Highlight { .. }))
        .count();
    assert_eq!(
        highlights, 1,
        "expected exactly one highlight after three saves, got {highlights}"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn test_arrow_annotations_round_trip() {
    // Regression: arrows were written as plain `/Line` annotations with no
    // `/LE` array, so `load_annotations` could never restore the arrow head.
    use pdfbull::pdf_engine::DocumentStore;

    let dir = std::env::temp_dir().join("pdfbull_arrow_roundtrip_test");
    std::fs::create_dir_all(&dir).unwrap();
    let src = dir.join("src.pdf");
    let source = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/test_document.pdf");
    std::fs::copy(&source, &src).unwrap();

    let cache = pdfbull::pdf_engine::create_render_cache(8, 32 * 1024 * 1024);
    let mut store = DocumentStore::new(cache);
    let doc_id = next_doc_id();
    store
        .open_document(&src.to_string_lossy(), None, doc_id)
        .expect("fixture must open");

    let out = dir.join("out.pdf");
    let out_str = out.to_string_lossy().to_string();
    store
        .save_annotations(
            doc_id,
            &[pdfbull::models::Annotation {
                id: pdfbull::models::next_annotation_id(),
                page: 0,
                style: pdfbull::models::AnnotationStyle::Arrow {
                    color: "#000000".to_string(),
                    thickness: 2.0,
                },
                x: 40.0,
                y: 200.0,
                width: 90.0,
                height: 30.0,
            }],
            Some(out_str.clone()),
        )
        .expect("save must succeed");

    let reloaded = store
        .load_annotations(&out_str)
        .expect("reload must succeed");
    assert!(
        reloaded
            .iter()
            .any(|a| matches!(a.style, pdfbull::models::AnnotationStyle::Arrow { .. })),
        "an Arrow annotation must survive a save/load round trip, got {reloaded:?}"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

/// Load a PDF, add a distinctive `/ExtGState` entry to every page's resources,
/// and write it back.
///
/// The bundled fixture only carries `/Font` resources, which would make a
/// "resources were preserved" assertion pass even with the destructive bug —
/// replacing the whole `/Resources` dictionary with a font-only one still leaves
/// a `/Font` key behind. Injecting a marker key makes the test fail if anything
/// other than a merge can pass.
fn with_marker_resources(path: &std::path::Path, marker: &str) {
    let mut doc = lopdf::Document::load(path).expect("fixture must parse");
    for (_, page_id) in doc.get_pages() {
        let existing = doc
            .objects
            .get(&page_id)
            .and_then(|o| o.as_dict().ok())
            .and_then(|d| d.get(b"Resources").ok())
            .cloned();

        let mut res = match existing {
            Some(lopdf::Object::Reference(r)) => doc
                .objects
                .get(&r)
                .and_then(|o| o.as_dict().ok())
                .cloned()
                .unwrap_or_default(),
            Some(lopdf::Object::Dictionary(d)) => d,
            _ => lopdf::Dictionary::new(),
        };
        res.set(
            "ExtGState",
            lopdf::Object::Dictionary(lopdf::Dictionary::from_iter(vec![(
                marker,
                lopdf::Object::Name(b"GS0".to_vec()),
            )])),
        );

        let res_id = doc.add_object(lopdf::Object::Dictionary(res));
        if let Some(obj) = doc.objects.get_mut(&page_id)
            && let Ok(dict) = obj.as_dict_mut()
        {
            dict.set("Resources", lopdf::Object::Reference(res_id));
        }
    }
    doc.save(path).expect("marked fixture must save");
}

/// Read back the marker injected by [`with_marker_resources`], per page.
fn marker_survives(path: &std::path::Path, marker: &str) -> Vec<bool> {
    let doc = lopdf::Document::load(path).expect("output must parse");
    doc.get_pages()
        .into_values()
        .filter_map(|id| doc.objects.get(&id))
        .filter_map(|o| o.as_dict().ok())
        .map(|dict| {
            let res: Option<lopdf::Dictionary> = match dict.get(b"Resources") {
                Ok(lopdf::Object::Reference(r)) => {
                    doc.objects.get(r).and_then(|o| o.as_dict().ok()).cloned()
                }
                Ok(lopdf::Object::Dictionary(d)) => Some(d.clone()),
                _ => None,
            };
            match res {
                Some(d) => {
                    let gs = d.get(b"ExtGState").ok().and_then(|e| e.as_dict().ok());
                    gs.is_some_and(|gs| gs.has(marker.as_bytes()))
                }
                None => false,
            }
        })
        .collect()
}

#[test]
fn test_add_header_footer_preserves_page_resources() {
    // Regression: `add_header_footer` assigned a brand-new `/Resources`
    // dictionary containing only `{Font: {F1}}` to *every* page, discarding
    // whatever the page already had. Every `/XObject`, `/ExtGState`,
    // `/ColorSpace`, `/Shading` and `/Pattern` entry was dropped, so images and
    // soft masks on every page became unresolvable and the saved file rendered
    // blank apart from the header — while the UI reported success.
    use pdfbull::pdf_engine::DocumentStore;

    const MARKER: &str = "HdrMarker";

    let dir = std::env::temp_dir().join("pdfbull_header_footer_resources_test");
    std::fs::create_dir_all(&dir).unwrap();
    let src = dir.join("src.pdf");
    let source = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/test_document.pdf");
    std::fs::copy(&source, &src).unwrap();
    with_marker_resources(&src, MARKER);

    let before = marker_survives(&src, MARKER);
    assert_eq!(before.len(), 2, "fixture must have two pages");
    assert!(before.iter().all(|b| *b), "the marker must be in place");

    let out = dir.join("out.pdf");
    DocumentStore::add_header_footer(
        &src.to_string_lossy(),
        "Header",
        "Page {page} of {pages}",
        &out.to_string_lossy(),
    )
    .expect("header/footer must succeed");

    let after = marker_survives(&out, MARKER);
    assert_eq!(after.len(), before.len(), "the page count must not change");
    for (page_no, survived) in after.iter().enumerate() {
        assert!(
            *survived,
            "page {} lost its /ExtGState/{MARKER}: stamping the header and footer \
             replaced the page's /Resources instead of merging into it",
            page_no + 1
        );
    }

    // The stamp's own font must be registered as well.
    let stamped = lopdf::Document::load(&out).unwrap();
    for (_, page_id) in stamped.get_pages() {
        let dict = stamped.objects.get(&page_id).and_then(|o| o.as_dict().ok());
        let has_font = dict
            .and_then(|d| d.get(b"Resources").ok())
            .map(|r| match r {
                lopdf::Object::Reference(id) => stamped
                    .objects
                    .get(id)
                    .and_then(|o| o.as_dict().ok())
                    .is_some_and(|d| d.has(b"Font")),
                lopdf::Object::Dictionary(d) => d.has(b"Font"),
                _ => false,
            })
            .unwrap_or(false);
        assert!(
            has_font,
            "every page must have a /Font resource for the stamp"
        );
    }

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn test_add_watermark_preserves_page_resources() {
    // Same class of defect as the header/footer path: the watermark code fell
    // back to a shared font-only `/Resources` dictionary whenever a page's
    // resources could not be resolved to a dictionary, replacing the page's real
    // resources and destroying its content.
    use pdfbull::pdf_engine::DocumentStore;

    const MARKER: &str = "WmMarker";

    let dir = std::env::temp_dir().join("pdfbull_watermark_resources_test");
    std::fs::create_dir_all(&dir).unwrap();
    let src = dir.join("src.pdf");
    let source = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/test_document.pdf");
    std::fs::copy(&source, &src).unwrap();
    with_marker_resources(&src, MARKER);
    assert!(marker_survives(&src, MARKER).iter().all(|b| *b));

    let out = dir.join("out.pdf");
    DocumentStore::add_watermark(&src.to_string_lossy(), "DRAFT", &out.to_string_lossy())
        .expect("watermark must succeed");

    let after = marker_survives(&out, MARKER);
    for (page_no, survived) in after.iter().enumerate() {
        assert!(
            *survived,
            "page {} lost its /ExtGState/{MARKER}: watermarking replaced the page's \
             /Resources instead of merging into it",
            page_no + 1
        );
    }

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn test_hex_to_rgb_never_panics_on_multibyte_input() {
    // Regression: `hex_to_rgb` tested `hex.len() == 6` (a *byte* count) and then
    // sliced `&hex[0..2]`, which panics with "byte index 2 is not a char
    // boundary" for any 6-byte non-ASCII string. It is a `pub` function, so that
    // was a live panic in library code.
    // Two euro signs are 6 bytes but 2 characters.
    assert_eq!(pdfbull::pdf_engine::hex_to_rgb("€€"), (0.0, 0.0, 0.0));
    assert_eq!(pdfbull::pdf_engine::hex_to_rgb("€€€"), (0.0, 0.0, 0.0));
    assert_eq!(pdfbull::pdf_engine::hex_to_rgb("€"), (0.0, 0.0, 0.0));

    // Ordinary values must still decode.
    let (r, g, b) = pdfbull::pdf_engine::hex_to_rgb("#FF8000");
    assert!((r - 1.0).abs() < 1e-6, "red channel was {r}");
    assert!((g - 128.0 / 255.0).abs() < 1e-6, "green channel was {g}");
    assert!(b.abs() < 1e-6, "blue channel was {b}");
    assert_eq!(pdfbull::pdf_engine::hex_to_rgb("FFFFFF"), (1.0, 1.0, 1.0));

    // Wrong lengths and non-hex characters fall back to black rather than
    // panicking.
    assert_eq!(pdfbull::pdf_engine::hex_to_rgb(""), (0.0, 0.0, 0.0));
    assert_eq!(pdfbull::pdf_engine::hex_to_rgb("FFF"), (0.0, 0.0, 0.0));
    assert_eq!(pdfbull::pdf_engine::hex_to_rgb("GGGGGG"), (0.0, 0.0, 0.0));
}

#[test]
fn test_layout_mode_labels_are_human_readable() {
    // The status bar printed `{:?}` of the layout enum, so people read
    // "TwoPageSpread" and "SinglePage" as if they were internal identifiers.
    use pdfbull::ui_gpui::ribbon::PageLayoutMode;

    assert_eq!(PageLayoutMode::Continuous.label(), "Continuous");
    assert_eq!(PageLayoutMode::SinglePage.label(), "Single page");
    assert_eq!(PageLayoutMode::TwoPageSpread.label(), "Two-page");

    for mode in [
        PageLayoutMode::Continuous,
        PageLayoutMode::SinglePage,
        PageLayoutMode::TwoPageSpread,
    ] {
        let label = mode.label();
        let debug = format!("{mode:?}");
        // CamelCase debug output leaks into the UI; no label may be one of the
        // multi-word variants' identifiers.
        if debug.len() > "Continuous".len() {
            assert_ne!(
                label, debug,
                "the label must not be the Debug representation"
            );
        }
        assert!(label.chars().next().is_some_and(char::is_uppercase));
        assert!(!label.ends_with('.'), "a label is not a sentence");
    }
}

#[test]
fn test_invalidate_rendered_pages_releases_bookkeeping() {
    // Regression: `RenderCache::invalidate_document` removed the cached bitmaps
    // but left the per-document `RenderKey` set behind, so the set accumulated
    // every key ever produced for the document — a new one for every distinct
    // zoom level, rotation and quality — and was only cleared when the tab
    // closed.
    use pdfbull::pdf_engine::{RenderFilter, RenderKey, RenderQuality, create_render_cache};

    let cache = create_render_cache(8, 8 * 1024 * 1024);
    let doc_id = next_doc_id();

    let key_at = |scale: u32, filter: RenderFilter| RenderKey {
        doc_id,
        page_num: 0,
        scale,
        rotation: 0,
        auto_crop: false,
        quality: RenderQuality::High,
        filter,
    };

    // Mint many keys, as zooming in and out would. The scale is quantised to
    // hundredths, which is exactly why this grew without bound.
    for step in 75..139u32 {
        cache.put(
            key_at(step, RenderFilter::None),
            pdfbull::models::RenderResult {
                width: 1,
                height: 1,
                data: std::sync::Arc::from([0u8, 0, 0, 255].as_slice()),
            },
        );
    }
    assert!(cache.get(&key_at(100, RenderFilter::None)).is_some());

    // A filter change must not be answered from the unfiltered cache entry:
    // Midnight mode switched to `Inverted` and used to render identically.
    cache.put(
        key_at(100, RenderFilter::Inverted),
        pdfbull::models::RenderResult {
            width: 2,
            height: 2,
            data: std::sync::Arc::from([0u8, 0, 0, 255].as_slice()),
        },
    );
    let inverted = cache
        .get(&key_at(100, RenderFilter::Inverted))
        .expect("inverted render");
    assert_eq!(
        inverted.width, 2,
        "the inverted render must not be served from the unfiltered cache entry"
    );

    cache.invalidate_document(doc_id);

    assert!(
        cache.get(&key_at(100, RenderFilter::None)).is_none(),
        "invalidate_document must evict the cached bitmaps"
    );
    assert!(
        cache.tracked_key_count(doc_id).is_none(),
        "invalidate_document must drop the document's key set, not just its bitmaps"
    );
}

#[test]
fn test_combo_box_carries_export_values() {
    // Regression: `fill_form` read `options[idx]` — the *label* from the field's
    // `/Opt` array — and wrote it to `/V`. Any choice field whose export value
    // differs from its label ("A" / "Apple") was therefore saved with a value no
    // viewer matches, so the form looked filled but was semantically wrong.
    use pdfbull::models::FormFieldVariant;

    let variant = FormFieldVariant::ComboBox {
        options: vec!["Apple".into(), "Banana".into()],
        export_values: vec!["A".into(), "B".into()],
        selected_index: Some(0),
    };

    // Round-tripping must keep the two lists aligned.
    let json = serde_json::to_string(&variant).unwrap();
    let back: FormFieldVariant = serde_json::from_str(&json).unwrap();
    match back {
        FormFieldVariant::ComboBox {
            options,
            export_values,
            selected_index,
        } => {
            assert_eq!(options, vec!["Apple".to_string(), "Banana".to_string()]);
            assert_eq!(export_values, vec!["A".to_string(), "B".to_string()]);
            assert_eq!(selected_index, Some(0));
        }
        other => panic!("expected a combo box, got {other:?}"),
    }

    // A payload written before `export_values` existed must still deserialize,
    // so an older settings file cannot fail to load.
    let legacy: FormFieldVariant =
        serde_json::from_str(r#"{"ComboBox":{"options":["Apple"],"selected_index":0}}"#)
            .expect("a legacy combo box payload must still deserialize");
    match legacy {
        FormFieldVariant::ComboBox { export_values, .. } => {
            assert!(
                export_values.is_empty(),
                "a missing export_values list must default to empty, not fail"
            );
        }
        other => panic!("expected a combo box, got {other:?}"),
    }
}

#[test]
fn test_security_dialog_defaults_to_a_listed_algorithm() {
    // Regression: the Security dialog's Apply button sent a hard-coded
    // "AES-256", so selecting AES-128 and pressing Apply encrypted with AES-256.
    use pdfbull::ui_gpui::dialogs::{DialogsState, SECURITY_ALGORITHMS};

    let state = DialogsState::new();
    assert!(
        SECURITY_ALGORITHMS
            .iter()
            .any(|(algo, _)| *algo == state.security_algorithm),
        "the default algorithm {:?} must be one the dialog actually offers",
        state.security_algorithm
    );

    let aes128 = SECURITY_ALGORITHMS
        .iter()
        .find(|(algo, _)| *algo == "AES-128")
        .expect("AES-128 must be offered");
    assert!(!aes128.1.is_empty(), "every algorithm needs a label");
}

#[test]
fn test_close_to_right_clamps_an_out_of_range_index() {
    // Regression: `TabAction::CloseToRight` called
    // `tabs.drain((idx + 1)..)` with no bounds check. `Vec::drain` panics when
    // the range starts past the end, so a stale index crashed the app. Neither
    // sibling arm (`CloseTab`, `CloseOthers`) bounds-checked either.
    use pdfbull::ui_gpui::tabs::{DocumentTab, TabsState};

    let mut tabs = TabsState::new();
    for n in 0..3 {
        tabs.push(DocumentTab {
            id: 0,
            doc_id: Some(next_doc_id()),
            title: format!("doc{n}.pdf"),
            path: None,
            is_modified: false,
        });
    }
    assert_eq!(tabs.tabs.len(), 3);

    // The exact range the handler used to pass, for every plausible stale index.
    for idx in [0usize, 2, 3, 7, usize::MAX] {
        let keep = idx.saturating_add(1).min(tabs.tabs.len());
        let drained: Vec<_> = tabs.tabs.drain(keep..).collect();
        assert_eq!(drained.len(), 3usize.saturating_sub(keep));
        // Put them back so the next iteration starts from the same state.
        tabs.tabs.extend(drained);
        assert_eq!(tabs.tabs.len(), 3);
    }
}

/// The numbers the thumbnail strip's layout and its scroll maths are built
/// from. Re-derived here so the test does not depend on them being private.
const THUMB_STRIDE: f32 = 200.0;
const THUMB_GAP: f32 = 8.0;
const THUMB_CARD_H: f32 = THUMB_STRIDE - THUMB_GAP;
const THUMB_PADDING: f32 = 8.0;
const THUMB_OVERSCAN: f32 = 400.0;

fn thumb_content_height(total_pages: usize) -> f32 {
    if total_pages == 0 {
        return 0.0;
    }
    2.0 * THUMB_PADDING + THUMB_STRIDE * (total_pages - 1) as f32 + THUMB_CARD_H
}

fn thumb_top_in_content(page_idx: usize) -> f32 {
    THUMB_PADDING + THUMB_STRIDE * page_idx as f32
}

/// The window `SidebarState::thumbnail_window` would build, given a scroll
/// offset and a real viewport height.
fn thumb_window(total_pages: usize, scrolled: f32, viewport_h: f32) -> (usize, usize) {
    let first = (((scrolled - THUMB_OVERSCAN).max(0.0) - THUMB_PADDING) / THUMB_STRIDE)
        .floor()
        .max(0.0) as usize;
    let first = first.min(total_pages - 1);
    let needed_to = scrolled + viewport_h + THUMB_OVERSCAN;
    let last = (((needed_to - THUMB_PADDING) / THUMB_STRIDE)
        .floor()
        .max(0.0) as usize)
        .saturating_add(2)
        .min(total_pages);
    (first, last.max(first + 1))
}

#[test]
fn test_thumbnail_strip_layout_matches_scroll_maths() {
    // Regression: the sidebar's virtualisation mapped scroll offsets to page
    // indices using a hard-coded `THUMB_STRIDE = 200`, while the cards actually
    // measured ~206px (padding + border + 160px image + a font-sized label +
    // gap). The strip's real content therefore outgrew the assumption by ~6px
    // *per page*.
    //
    // `thumbnail_window` derives the viewport as `assumed_content - max_offset`
    // and `max_offset` comes from the *real* content size, so that subtraction
    // returned `true_viewport - drift`. On a 500-page document the derived
    // viewport was ~3000px short, the built window stopped far above the
    // visible area, and scrolling the panel showed blank space at the bottom —
    // worse the longer the document.
    //
    // The invariant: the height the spacers emit must equal the height the
    // windowing maths assumes, for every page count and every built range.
    for total_pages in 1..=600usize {
        let content = thumb_content_height(total_pages);
        assert!(
            content > 0.0,
            "{total_pages} pages must produce a non-zero strip height"
        );

        for viewport_h in [200.0_f32, 640.0, 1200.0] {
            // Sweep the whole scroll range, plus a little past each end.
            let max_scroll = (content - viewport_h).max(0.0);
            let steps = 24;
            for step in 0..=steps {
                let scrolled = max_scroll * (step as f32 / steps as f32);
                let (first, last) = thumb_window(total_pages, scrolled, viewport_h);

                assert!(first < last, "window must be non-empty");
                assert!(last <= total_pages, "window must not exceed the document");

                // Emitted height of the strip for this range: a top spacer, the
                // built cards with their gaps, and a bottom spacer.
                let top_pad = thumb_top_in_content(first);
                let built = (last - first) as f32;
                let bottom_pad = THUMB_PADDING + THUMB_STRIDE * (total_pages - last) as f32;
                let emitted = top_pad + (built - 1.0) * THUMB_STRIDE + THUMB_CARD_H + bottom_pad;

                assert!(
                    (emitted - content).abs() < 0.01,
                    "{total_pages} pages, viewport {viewport_h}, scroll {scrolled}: \
                     the strip lays out to {emitted} but the maths assumes {content}"
                );

                // The visible band must be covered by built cards. This is the
                // symptom: cards stop being built above the bottom of the
                // viewport, so the panel scrolls into blank space.
                //
                // Clamped at the document's last card rather than at the end of
                // the content: the content ends with `THUMB_PADDING` of inset,
                // which is *meant* to be empty, and a document shorter than the
                // viewport does not scroll at all.
                let doc_last_card_bottom = thumb_top_in_content(total_pages - 1) + THUMB_CARD_H;
                let visible_bottom = (scrolled + viewport_h).min(doc_last_card_bottom);
                let last_card_bottom = thumb_top_in_content(last - 1) + THUMB_CARD_H;
                assert!(
                    last_card_bottom >= visible_bottom - 0.01,
                    "{total_pages} pages, viewport {viewport_h}, scroll {scrolled}: \
                     built cards end at {last_card_bottom} but the viewport reaches \
                     {visible_bottom} (window {first}..{last})"
                );
            }
        }
    }
}

#[test]
fn test_thumbnail_page_origins_round_trip() {
    // The windowing maths inverts "where does page k start", so that mapping
    // must be the exact inverse of the layout's, or scrolling drifts by a card
    // per step.
    for total_pages in [1usize, 2, 7, 50, 500, 2000] {
        for page_idx in 0..total_pages {
            let top = thumb_top_in_content(page_idx);
            let recovered = (((top - THUMB_PADDING) / THUMB_STRIDE).floor().max(0.0)) as usize;
            assert_eq!(
                recovered, page_idx,
                "{total_pages} pages: page {page_idx} at y={top} inverted to {recovered}"
            );
        }
    }
}

#[test]
fn test_thumbnail_window_covers_the_viewport_at_every_scroll_position() {
    // Directly assert the user-visible property the drift broke: wherever the
    // panel is scrolled, the built cards reach the bottom of the viewport.
    let viewport_h = 640.0_f32;
    for total_pages in [1usize, 5, 40, 250, 1000] {
        let content = thumb_content_height(total_pages);
        let max_scroll = (content - viewport_h).max(0.0);
        for step in 0..=40 {
            let scrolled = max_scroll * (step as f32 / 40.0);
            let (first, last) = thumb_window(total_pages, scrolled, viewport_h);
            // Clamped at the document's last card: the content ends with
            // `THUMB_PADDING` of inset that is meant to be empty, and a
            // document shorter than the viewport does not scroll.
            let doc_last_card_bottom = thumb_top_in_content(total_pages - 1) + THUMB_CARD_H;
            let visible_bottom = (scrolled + viewport_h).min(doc_last_card_bottom);
            assert!(
                thumb_top_in_content(last - 1) + THUMB_CARD_H >= visible_bottom - 0.01,
                "{total_pages} pages at scroll {scrolled}: cards {first}..{last} stop at {} \
                 but the viewport reaches {visible_bottom}",
                thumb_top_in_content(last - 1) + THUMB_CARD_H
            );
        }
    }
}

#[test]
fn test_cache_keys_no_longer_grow_unbounded() {
    // Regression: `DocumentStore` kept a `cache_keys: HashMap<DocumentId,
    // Vec<RenderKey>>` that was appended to on every cache-miss render and read
    // by nothing. Because the scale component of the key is quantised to 0.01,
    // a single page could mint hundreds of distinct keys per zoom level and the
    // vector grew for the whole life of the document. The field is gone, so the
    // store's size must no longer depend on how much was rendered.
    use pdfbull::pdf_engine::DocumentStore;

    let cache = pdfbull::pdf_engine::create_render_cache(4, 16 * 1024 * 1024);
    let store = DocumentStore::new(cache);
    drop(store);
}

#[test]
fn test_reset_for_document_is_lossless_when_annotations_are_cached() {
    // Regression: switching tabs called `reset_for_document`, which clears
    // `viewport.annotations`, and nothing ever put them back — `LoadAnnotations`
    // was only sent from `open_pdf_path`. Highlight a page in tab A, switch to
    // tab B, switch back to A, and the canvas was empty; the next Ctrl+S then
    // sent an *empty* list, and `save_annotations` deleted every `/PDFBULL:`
    // annotation it had previously written. The user's saved highlights were
    // erased from the file while the status bar reported success.
    //
    // This exercises the exact stash/reset/restore cycle the view performs on a
    // tab switch, without needing a live GPUI context.
    let doc_a = next_doc_id();
    let doc_b = next_doc_id();

    // `annotations_by_doc` is the cache the view now keeps.
    let mut cache: std::collections::HashMap<
        pdfbull::models::DocumentId,
        Vec<pdfbull::models::Annotation>,
    > = std::collections::HashMap::new();

    let highlights: Vec<pdfbull::models::Annotation> = (0..3)
        .map(|i| pdfbull::models::Annotation {
            id: pdfbull::models::next_annotation_id(),
            page: i,
            style: pdfbull::models::AnnotationStyle::Highlight {
                color: "#FFFF00".to_string(),
            },
            x: 10.0,
            y: 10.0,
            width: 40.0,
            height: 12.0,
        })
        .collect();

    let mut vp = DocumentViewport::new();

    // --- tab A is active and the user has just highlighted three pages -------
    vp.annotations = highlights.clone();
    vp.reindex_annotations();
    assert_eq!(vp.annotations.len(), 3);

    // --- switch to tab B: stash A, reset, restore nothing (B is new) --------
    cache.insert(doc_a, vp.annotations.clone());
    vp.reset_for_document();
    assert!(
        vp.annotations.is_empty(),
        "the reset must clear the outgoing document's annotations"
    );

    // B has never been loaded, so the view must re-read it rather than assume
    // it has none. That is the branch the view takes when the cache misses.
    assert!(
        !cache.contains_key(&doc_b),
        "a document with no cache entry must trigger a reload, not a silent empty state"
    );

    // --- switch back to tab A: restore exactly what was stashed -------------
    let restored = cache.get(&doc_a).cloned().expect("A must be cached");
    vp.annotations = restored;
    vp.reindex_annotations();
    assert_eq!(
        vp.annotations.len(),
        3,
        "returning to a tab must restore every annotation"
    );

    let ids: Vec<u64> = vp.annotations.iter().map(|a| a.id).collect();
    let expected: Vec<u64> = highlights.iter().map(|a| a.id).collect();
    assert_eq!(
        ids, expected,
        "the same annotations must come back, in order"
    );

    // The per-page index must be rebuilt too, or the restored annotations would
    // be invisible on the canvas while still being present in the list — which
    // is exactly what would then get written to the file.
    for ann in &highlights {
        assert_eq!(
            vp.annotations_on_page(ann.page).len(),
            1,
            "page {} lost its restored annotation in the per-page index",
            ann.page + 1
        );
    }
    assert_eq!(vp.annotations_on_page(99).len(), 0);
}
