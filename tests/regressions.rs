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
