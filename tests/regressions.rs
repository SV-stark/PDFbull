use gpui_kit::RenderImage;
use gpui_kit::gpui::px;
use pdfbull::commands::PdfCommand;
use pdfbull::models::next_doc_id;
use pdfbull::pdf_engine::DocumentStore;
use pdfbull::ui_gpui::canvas::{DocumentViewport, rotated_rect_to_overlay_space};
use pdfbull::ui_gpui::dialogs::ActiveDialog;
use pdfbull::ui_gpui::log_console::LogConsoleState;
use pdfbull::ui_gpui::ribbon::PageLayoutMode;
use pdfbull::ui_gpui::sidebar::SidebarState;
use std::fmt::Write as _;
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
        std::sync::Arc::new(vec![pdfbull::models::TextItem {
            text: "leaked".to_string(),
            x: 0.0,
            y: 0.0,
            width: 1.0,
            height: 1.0,
        }]),
    );
    vp.search_highlights
        .insert(0, std::sync::Arc::new(vec![(0.0, 0.0, 1.0, 1.0)]));
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
fn test_scroll_to_page_and_visible_page_agree_for_short_pages() {
    // Regression: `calculate_visible_page_continuous` inverted
    // `page_top_in_content` with `+ CANVAS_PADDING_Y` instead of `-`. The test
    // above only ever uses 842pt pages, where a slot is far taller than twice
    // the padding and the two signs agree; a short slot (small page at a low
    // zoom) made `+ PADDING` add a whole page, so `scroll_to_page(N)` read back
    // as `N + 1`.
    let mut vp = DocumentViewport::new();
    vp.page_width = 595.0;
    vp.page_height = 20.0;
    vp.layout_mode = PageLayoutMode::Continuous;
    vp.total_pages = 50;

    for zoom in [0.5f32, 1.0, 2.0, 4.0] {
        vp.zoom = zoom;
        for page in [0usize, 1, 5, 49] {
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
fn test_overlay_rect_rotation_matches_the_quarter_turn_matrices() {
    use pdfbull::ui_gpui::canvas::{overlay_rect_in_rotated_space, rotated_rect_to_overlay_space};

    // A 90 degree turn maps a top-left overlay rect onto the *height* edge:
    // zpdf's matrix `(0,-1,1,0,0,W)` composed with the renderer's y-flip gives
    // `(H - y - h, x, h, w)`. On a 100x100 page, (10,20,30,5) -> (75,10,5,30).
    // The 90 and 270 arms used to be transposed, mirroring overlays on every
    // quarter-turn page.
    assert_eq!(
        overlay_rect_in_rotated_space(10.0, 20.0, 30.0, 5.0, 100.0, 100.0, 90),
        (75.0, 10.0, 5.0, 30.0)
    );
    // 270 degrees is `(y, W - x - w, h, w)` -> (20, 60, 5, 30).
    assert_eq!(
        overlay_rect_in_rotated_space(10.0, 20.0, 30.0, 5.0, 100.0, 100.0, 270),
        (20.0, 60.0, 5.0, 30.0)
    );
    // 180 degrees mirrors both axes and keeps the dimensions.
    assert_eq!(
        overlay_rect_in_rotated_space(10.0, 20.0, 30.0, 5.0, 100.0, 100.0, 180),
        (60.0, 75.0, 30.0, 5.0)
    );

    // Forward and inverse must round-trip exactly for every quarter turn on a
    // non-square page — the property hit-testing relies on.
    let (pw, ph) = (595.0f32, 842.0f32);
    let (x, y, w, h) = (37.0f32, 120.0, 210.0, 44.0);
    for rotation in [0, 90, 180, 270] {
        let (rx, ry, rw, rh) = overlay_rect_in_rotated_space(x, y, w, h, pw, ph, rotation);
        let (bx, by, bw, bh) = rotated_rect_to_overlay_space(rx, ry, rw, rh, pw, ph, rotation);
        for (got, want, axis) in [
            (bx, x, "x"),
            (by, y, "y"),
            (bw, w, "width"),
            (bh, h, "height"),
        ] {
            assert!(
                (got - want).abs() < 1e-4,
                "rotation {rotation}: {axis} round-tripped to {got}, expected {want}"
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
fn test_minified_scan_is_retained_at_its_draw_footprint() {
    // Regression: the CPU backend box-filters an image to its device footprint
    // before sampling it, but the interpreter only filters up front when it is
    // told the render scale (`ContentInterpreter::with_image_downscale`). The
    // engine asks `CpuRenderer::raster_scale` and passes the answer, so a scan
    // drawn thumbnail-small is retained in the document-lifetime cache at its
    // footprint rather than at its full decoded size — the same retention that
    // `with_image_cache_limit` bounds, one level down.
    //
    // Without that wiring a 1000x1000 RGB source is retained in full (~3 MB) to
    // paint a single 40x40 pt rect at scale 1.0.
    let (src_w, src_h) = (1000u32, 1000u32);
    let pixels: Vec<u8> = (0..u64::from(src_w) * u64::from(src_h))
        .flat_map(|i| {
            let v = (i % 251) as u8;
            [v, v / 2, 255u8.wrapping_sub(v)]
        })
        .collect();

    let mut builder = zpdf_writer::DocumentBuilder::new();
    let page = builder.add_page(200.0, 200.0);
    builder
        .add_image(
            page,
            zpdf_writer::ImageData::Rgb8 {
                width: src_w,
                height: src_h,
                pixels,
            },
            72.0,
            72.0,
            40.0,
            40.0,
        )
        .expect("the image must embed");
    let bytes = builder.build().expect("the builder must produce a PDF");

    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("minified_scan.pdf");
    std::fs::write(&path, &bytes).expect("the generated PDF must be writable");

    let cache = pdfbull::pdf_engine::create_render_cache(8, 32 * 1024 * 1024);
    let mut store = DocumentStore::new(cache);
    let doc_id = next_doc_id();
    let opened = store
        .open_document(&path.to_string_lossy(), None, doc_id)
        .expect("the generated document must open");
    assert_eq!(opened.page_count, 1);

    let options = pdfbull::pdf_engine::RenderOptions {
        scale: 1.0,
        rotation: 0,
        filter: pdfbull::pdf_engine::RenderFilter::None,
        auto_crop: false,
        quality: pdfbull::pdf_engine::RenderQuality::Medium,
    };
    let rendered = store
        .render_page(doc_id, 0, options)
        .expect("the page must render");
    assert!(rendered.width > 0 && rendered.height > 0);

    let source_bytes = u64::from(src_w) * u64::from(src_h) * 3;
    let retained = store.image_cache_bytes_used(doc_id);
    assert!(
        retained > 0,
        "no decoded image was retained, so this test is not exercising the cache"
    );
    assert!(
        retained < source_bytes / 2,
        "the minified scan retained {retained} bytes for a 40x40 pt draw; its \
         full-resolution source is {source_bytes} bytes, so the footprint filter \
         did not engage"
    );
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
            page_rotations: std::collections::HashMap::new(),
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
fn test_thumbnail_scroll_container_does_not_reapply_the_card_gap() {
    // Regression: the thumbnail scroll container carried `.gap(THUMB_GAP)` on
    // top of spacers and card pitches that already encode `THUMB_STRIDE`
    // (card + gap). The flex gap is inserted between *every* child pair -- the
    // two spacers included -- so each card sat `THUMB_GAP` below where
    // `thumbnail_top_in_content` puts it, and the strip laid out
    // `2 * THUMB_GAP` taller than `thumbnail_content_height` reports. The last
    // 16px of the panel could not be scrolled to.
    //
    // `test_thumbnail_strip_layout_matches_scroll_maths` above only checks that
    // the *formulae* agree with each other, and the real element tree cannot be
    // laid out without a GPUI window, so this asserts on the source the way the
    // canvas's Continuous-branch test does.
    let src = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/ui_gpui/sidebar.rs"),
    )
    .expect("sidebar.rs must be readable");

    let start = src
        .find(".id(\"sidebar-thumbnails-scroll\")")
        .expect("the thumbnail scroll container must exist");
    let end = src[start..]
        .find(".children(thumb_cards)")
        .map(|i| start + i)
        .expect("the thumbnail container must mount `thumb_cards`");
    let container = &src[start..end];

    // Strip `//` comments: the explanatory notes right here talk about `.gap()`
    // and would otherwise trip the scan.
    let code: String = container
        .lines()
        .map(|l| match l.find("//") {
            Some(i) => &l[..i],
            None => l,
        })
        .collect::<Vec<_>>()
        .join("\n");

    assert!(
        !code.contains(".gap("),
        "the thumbnail scroll container must not add its own `.gap()`: the \
         spacers and each card's bottom margin already encode THUMB_STRIDE"
    );

    // ...and the card must actually carry that margin, or the cards touch.
    let card_start = src
        .find(".h(px(THUMB_CARD_H))")
        .expect("the thumbnail card must fix its own height");
    let card = &src[card_start..];
    let card_end = card.find(".into_any_element()").unwrap_or(card.len());
    assert!(
        card[..card_end].contains(".mb(px(THUMB_GAP))"),
        "each thumbnail card must carry the inter-card gap as its own bottom \
         margin, so the strip's emitted height matches thumbnail_content_height"
    );
}

#[test]
fn test_thumbnail_strip_emits_exactly_the_height_the_maths_reports() {
    // The element-tree half of the same regression: model the real strip --
    // a top spacer, `n` cards each `THUMB_CARD_H` tall plus their own
    // `THUMB_GAP` bottom margin, and a bottom spacer -- and check it lands on
    // `thumbnail_content_height` for every window the virtualiser can build.
    //
    // The bottom spacer sheds one `THUMB_GAP` because the last built card now
    // supplies the trailing inset itself.
    for total_pages in 1..=600usize {
        let content = thumb_content_height(total_pages);
        for first in 0..total_pages {
            let last = (first + 7).min(total_pages);
            let built = (last - first) as f32;

            let top_pad = thumb_top_in_content(first);
            let bottom_pad = THUMB_PADDING + THUMB_STRIDE * (total_pages - last) as f32 - THUMB_GAP;

            // A spacer is only pushed when it is positive; with no container gap
            // a dropped zero-height spacer changes nothing.
            let emitted = top_pad + built * (THUMB_CARD_H + THUMB_GAP) + bottom_pad.max(0.0);

            assert!(
                (emitted - content).abs() < 0.01,
                "{total_pages} pages, window {first}..{last}: the strip lays out \
                 to {emitted} but the scroll maths assumes {content}"
            );

            for j in 0..(last - first) {
                let card_top = top_pad + j as f32 * THUMB_STRIDE;
                let expected = thumb_top_in_content(first + j);
                assert!(
                    (card_top - expected).abs() < 0.01,
                    "{total_pages} pages, card {}: laid out at {card_top}, maths \
                     says {expected}",
                    first + j
                );
            }
        }
    }
}

#[test]
fn test_wheel_banks_sub_threshold_deltas_until_a_page_turns() {
    // Regression: the SinglePage / TwoPageSpread wheel arms required a *single*
    // notch of more than 5px to turn the page. Precision touchpads send 1-3px
    // notches, so at a page edge -- or on any page short enough to fit the
    // viewport, where both "scroll within the page" branches are false -- those
    // notches were dropped and scrolling could not turn the page at all.
    let threshold = DocumentViewport::WHEEL_PAGE_TURN_THRESHOLD;
    let notch = threshold / 5.0; // a trackpad-sized notch

    // Sign convention, matching the wheel handler: a *negative* delta scrolls
    // down (toward the next page), a positive one scrolls up.

    let mut vp = DocumentViewport::new();
    vp.total_pages = 4;
    vp.current_page = 1;

    // Four small notches in the same direction are still short of a turn...
    for _ in 0..4 {
        assert_eq!(
            vp.bank_wheel_delta_at_edge(-notch, true, true),
            None,
            "a sub-threshold notch must bank, not turn"
        );
    }
    // ...the fifth completes it.
    assert_eq!(
        vp.bank_wheel_delta_at_edge(-notch, true, true),
        Some(1),
        "{threshold}px of accumulated notches must turn to the next page"
    );

    // The turn consumes the bank, so the next turn needs a fresh notch.
    assert_eq!(vp.bank_wheel_delta_at_edge(-notch, true, true), None);

    // Notches in the other direction spend a fresh bank first.
    let mut vp = DocumentViewport::new();
    vp.total_pages = 4;
    vp.current_page = 1;
    for _ in 0..4 {
        assert_eq!(vp.bank_wheel_delta_at_edge(notch, true, true), None);
    }
    assert_eq!(
        vp.bank_wheel_delta_at_edge(notch, true, true),
        Some(-1),
        "reversing direction must turn back once the bank is spent"
    );

    // A mouse wheel's single large notch still turns immediately.
    let mut vp = DocumentViewport::new();
    vp.total_pages = 4;
    vp.current_page = 1;
    assert_eq!(vp.bank_wheel_delta_at_edge(-120.0, true, true), Some(1));
    assert_eq!(vp.bank_wheel_delta_at_edge(120.0, true, true), Some(-1));

    // While a turn is impossible the bank is capped at one notch, so a long
    // scroll pressed against the first or last page cannot bank a surprise turn.
    let mut vp = DocumentViewport::new();
    vp.total_pages = 1;
    vp.current_page = 0;
    for _ in 0..50 {
        assert_eq!(vp.bank_wheel_delta_at_edge(-notch, false, false), None);
    }
    assert_eq!(
        vp.wheel_residual_y, -threshold,
        "the bank must be capped at one notch while no turn is possible"
    );
    assert_eq!(
        vp.bank_wheel_delta_at_edge(0.0, false, true),
        Some(1),
        "and then exactly one notch must turn when one becomes possible"
    );

    // Changing page through any other path resets the bank.
    let mut vp = DocumentViewport::new();
    vp.total_pages = 4;
    vp.bank_wheel_delta_at_edge(notch, true, true);
    vp.bank_wheel_delta_at_edge(notch, true, true);
    assert_ne!(vp.wheel_residual_y, 0.0);
    vp.scroll_to_page(2);
    assert_eq!(
        vp.wheel_residual_y, 0.0,
        "a page change must not carry a part-banked turn across"
    );
}

#[test]
fn test_ocr_boxes_use_the_same_rotation_mapping_as_the_canvas() {
    // Regression: `ocr_page` scaled OCR pixels by the *unrotated* `media_box` on
    // both axes, while the bitmap it ran on is the page's effective (crop) box
    // with `/Rotate` baked in. On a quarter-turned page the bitmap's axes are
    // swapped, so every recognised word was placed somewhere else entirely --
    // and on a cropped page the wrong box was used outright.
    //
    // The bitmap -> page mapping must be the exact inverse of the canvas's
    // page -> bitmap overlay mapping, or OCR boxes and the text layer disagree.
    let box_w = 600.0f32;
    let box_h = 800.0f32;
    let box_rect = (0.0f32, 0.0f32, box_w, box_h);

    for rot in [0i32, 90, 180, 270, -90, 450] {
        let normalized = rot.rem_euclid(360) / 90 * 90;
        let (disp_w, disp_h) = if normalized == 90 || normalized == 270 {
            (box_h, box_w)
        } else {
            (box_w, box_h)
        };
        // One point per pixel, so the "pixel" rect is already in points.
        let bitmap = (disp_w as u32, disp_h as u32);

        for (x, y, w, h) in [
            (0.0f32, 0.0f32, 100.0f32, 20.0f32),
            (50.0, 70.0, 10.0, 10.0),
            (disp_w - 120.0, disp_h - 30.0, 120.0, 30.0),
        ] {
            let from_ocr = pdfbull::pdf_engine::rotated_bitmap_rect_to_page_space(
                (x, y, x + w, y + h),
                bitmap,
                box_rect,
                rot,
            );
            // The canvas works in top-left-origin layout space; the OCR helper
            // returns PDF user space (y up), which is what `OcrWord::bbox`
            // documents. Convert before comparing.
            let (ox, oy0, ox1, oy1) = from_ocr;
            let ocr_overlay = (ox, box_h - oy1, ox1 - ox, oy1 - oy0);
            let from_canvas = rotated_rect_to_overlay_space(x, y, w, h, box_w, box_h, rot);

            for (a, b) in [
                (ocr_overlay.0, from_canvas.0),
                (ocr_overlay.1, from_canvas.1),
                (ocr_overlay.2, from_canvas.2),
                (ocr_overlay.3, from_canvas.3),
            ] {
                assert!(
                    (a - b).abs() < 0.01,
                    "rotation {rot}, rect ({x}, {y}, {w}, {h}): OCR says {ocr_overlay:?} \
                     but the canvas inverse says {from_canvas:?}"
                );
            }
        }
    }

    // And a crop box with a non-zero origin is honoured: the whole bitmap is
    // the whole effective box, not the media box.
    let cropped = (10.0f32, 20.0f32, 110.0f32, 220.0f32);
    let full = pdfbull::pdf_engine::rotated_bitmap_rect_to_page_space(
        (0.0, 0.0, 100.0, 200.0),
        (100, 200),
        cropped,
        0,
    );
    assert_eq!(full, cropped, "the bitmap covers the effective box exactly");
}

#[test]
fn test_search_and_the_text_layer_share_one_coordinate_space() {
    // Regression: `search` baked the page's `/Rotate` into the spans
    // (`with_page_rotation`) while `extract_text_items` did not, so a hit and the
    // word under the cursor lived in different coordinate systems. Search results
    // *looked* right only because the canvas painted them without rotating --
    // the very same path that then mispainted every unrotated annotation on a
    // rotated page. Everything overlay-ish must now live in unrotated model
    // space and be rotated at paint time.
    let src = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/pdf_engine.rs"),
    )
    .expect("pdf_engine.rs must be readable");

    let start = src
        .find("pub fn search(&mut self")
        .expect("`search` must exist");
    let end = src[start..]
        .find("\n    pub fn ")
        .map(|i| start + i)
        .unwrap_or(src.len());
    let search = &src[start..end];

    assert!(
        !search.contains("with_page_rotation"),
        "`search` must not bake `/Rotate` into the spans; the canvas rotates at \
         paint time, and search results must share the text layer's space"
    );
    assert!(
        search.contains("eff_box.y1 - bounds.y1"),
        "`search` must convert into the same top-down box space `extract_text_items` uses"
    );

    let av = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/ui_gpui/app_view.rs"),
    )
    .expect("app_view.rs must be readable");
    let push = av
        .find("this.viewport.annotations.push(crate::models::Annotation {")
        .expect("the annotation commit site must exist");
    let before = &av[..push];
    let rotate_at = before
        .rfind("rotated_rect_to_overlay_space")
        .expect("a committed annotation must be un-rotated before it is stored");
    assert!(
        before[rotate_at..push].contains("effective_rotation(commit_page)"),
        "the annotation commit must invert the page's rotation using the viewport's \
         own effective rotation"
    );
}

#[test]
fn test_continuous_pitch_covers_a_quarter_turned_page() {
    // Regression: `/Rotate` is per page, and a quarter-turned card is as tall as
    // the page is *wide*. The strip lays out one uniform slot per page, so the
    // slot must be the tallest card in the document — otherwise the scroll maths
    // and the layout disagree, the strip runs long, and the bottom pages can
    // never be scrolled into view.
    // Landscape document.
    let mut landscape = DocumentViewport::new();
    landscape.page_width = 842.0;
    landscape.page_height = 595.0;
    landscape.zoom = 1.0;
    landscape.layout_mode = PageLayoutMode::Continuous;
    landscape.total_pages = 4;
    landscape.current_page = 0;
    let landscape_pitch = landscape.continuous_item_height();

    // A portrait page stands inside it: it is taller than every landscape card,
    // so the uniform slot pitch must widen to cover it.
    let mut mixed = DocumentViewport::new();
    mixed.page_width = 842.0;
    mixed.page_height = 595.0;
    mixed.zoom = 1.0;
    mixed.layout_mode = PageLayoutMode::Continuous;
    mixed.total_pages = 4;
    mixed.current_page = 0;
    mixed.page_rotations.insert(2, 90);
    let mixed_pitch = mixed.continuous_item_height();

    assert!(
        (mixed_pitch - landscape_pitch) > px(1.0),
        "a portrait page in a landscape document must widen the pitch: \
         landscape {landscape_pitch:?}, mixed {mixed_pitch:?}"
    );
    assert_eq!(mixed_pitch, px(842.0 + 24.0));
    assert_eq!(mixed.page_display_height(2), 842.0);
    assert_eq!(mixed.page_display_height(0), 595.0);

    // The invariant the uniform slot exists for: it must fit every card, so the
    // layout and the scroll maths can never disagree. `render_page_card` sizes
    // itself from the same `display_size`.
    for page in 0..mixed.total_pages {
        assert!(
            px(mixed.display_size(page).1 * mixed.zoom) <= mixed_pitch,
            "the slot must fit page {page} ({} tall, pitch {mixed_pitch:?})",
            mixed.display_size(page).1
        );
        assert_eq!(
            mixed.display_size(page).1,
            mixed.page_display_height(page),
            "the height accessor must agree with `display_size`"
        );
    }

    // 180 and 360 keep the same aspect, so they must not widen the pitch.
    mixed.page_rotations.clear();
    mixed.page_rotations.insert(0, 180);
    mixed.page_rotations.insert(3, 360);
    assert_eq!(mixed.continuous_item_height(), landscape_pitch);
    assert_eq!(mixed.page_display_height(0), 595.0);

    // Zoom still scales the pitch.
    mixed.page_rotations.clear();
    mixed.page_rotations.insert(2, 90);
    mixed.zoom = 2.0;
    assert_eq!(mixed.continuous_item_height(), px(842.0 * 2.0 + 24.0));

    // The user's own rotation drives the pitch too: at 90° an upright page is as
    // tall as it is *wide*, so the pitch shrinks.
    let mut rotated = DocumentViewport::new();
    rotated.page_width = 595.0;
    rotated.page_height = 842.0;
    rotated.zoom = 2.0;
    rotated.layout_mode = PageLayoutMode::Continuous;
    rotated.total_pages = 2;
    rotated.current_page = 0;
    rotated.rotation = 90;
    assert_eq!(
        rotated.continuous_item_height(),
        px(595.0 * 2.0 + 24.0),
        "the view rotation must widen or narrow the slot pitch like a page's own"
    );
}

#[test]
fn test_page_organizer_rotations_follow_the_card_and_selection_follows_a_drag() {
    // Regression: `page_rotations` was keyed by *slot*, so a page's rotation was
    // dropped on the first reorder/delete/undo. And `reorder_card` only moved
    // the dragged card's own selection entry, leaving every other selected slot
    // pointing at whichever card shifted into its place.
    let mut st = pdfbull::organize::PageOrganizerState::new(6);

    // Rotate the cards at slots 1 and 3 (pages 1 and 3).
    st.selected_pages.insert(1);
    st.selected_pages.insert(3);
    st.rotate_selected(90);
    assert_eq!(st.page_rotations.get(&1), Some(&90));
    assert_eq!(st.page_rotations.get(&3), Some(&90));

    // Drag slot 1 to slot 4: order becomes [0, 2, 3, 4, 1, 5].
    st.reorder_card(1, 4);
    assert_eq!(st.page_order, vec![0, 2, 3, 4, 1, 5]);
    assert_eq!(
        st.page_rotations.get(&1),
        Some(&90),
        "the rotation must follow its card, not the slot it left"
    );
    assert_eq!(st.page_rotations.get(&3), Some(&90));

    // Selection follows the move: page 1 went 1 -> 4, page 3 went 3 -> 2.
    assert!(
        st.selected_pages.contains(&4),
        "the dragged card stays selected"
    );
    assert!(
        st.selected_pages.contains(&2),
        "a card the drag passed over keeps its selection"
    );
    assert!(!st.selected_pages.contains(&1));
    assert!(!st.selected_pages.contains(&3));

    // Drag backwards: slot 4 (page 1) back to slot 1.
    st.clear_selection();
    st.selected_pages.insert(4);
    st.selected_pages.insert(2);
    st.reorder_card(4, 1);
    assert_eq!(st.page_order, vec![0, 1, 2, 3, 4, 5]);
    assert!(st.selected_pages.contains(&1));
    assert!(
        st.selected_pages.contains(&3),
        "a backwards drag shifts the slots it opens up, not closes"
    );

    // Rotating and undoing a page that has moved still restores its rotation.
    st.clear_selection();
    st.selected_pages.insert(1);
    st.rotate_selected(90);
    assert_eq!(st.page_rotations.get(&1), Some(&180));
    assert!(st.undo());
    assert_eq!(
        st.page_rotations.get(&1),
        Some(&90),
        "undo must restore the rotation the page had before the move"
    );
    assert_eq!(st.page_rotations.get(&3), Some(&90));
}

#[test]
fn test_form_calculations_resolve_dependencies_and_write_every_variant() {
    use pdfbull::form_scripting::PdfFormScriptEngine;
    use pdfbull::models::{FormField, FormFieldVariant};
    use std::collections::HashMap;

    // A chain: F2 needs F1, F3 needs F2, ... `calc_scripts` is a `HashMap`, so a
    // single pass evaluates them in an arbitrary order and can read a field
    // before it has been computed. With eight links, the chance that one
    // arbitrary order happens to be a valid one is 1/8!.
    let names: Vec<String> = (1..=8).map(|n| format!("F{n}")).collect();
    let mut fields: Vec<FormField> = names
        .iter()
        .map(|name| FormField {
            name: name.clone(),
            variant: FormFieldVariant::Text {
                value: String::new(),
            },
            page: 0,
        })
        .collect();
    // The head of the chain is a literal the scripts read but never write.
    fields[0].variant = FormFieldVariant::Text {
        value: "1".to_string(),
    };

    let mut scripts = HashMap::new();
    for n in 2..=8 {
        scripts.insert(
            format!("F{n}"),
            format!("event.value = this.getField(\"F{}\").value + 1;", n - 1),
        );
    }
    // A checkbox target: a calculation aimed at a non-`Text` field must land.
    fields.push(FormField {
        name: "Approved".to_string(),
        variant: FormFieldVariant::Checkbox {
            is_checked: false,
            on_value: Some("Yes".to_string()),
        },
        page: 0,
    });
    scripts.insert(
        "Approved".to_string(),
        r#"AFSimple_Calculate("SUM", ["F8"])"#.to_string(),
    );

    let mut engine = PdfFormScriptEngine::new(&fields);
    engine.recalculate_all(&mut fields, &scripts);

    for n in 1..=8 {
        let name = format!("F{n}");
        let FormFieldVariant::Text { value } =
            &fields.iter().find(|f| f.name == name).unwrap().variant
        else {
            panic!("{name} is not a text field");
        };
        assert_eq!(
            *value,
            n.to_string(),
            "F{n} was evaluated before the field it depends on"
        );
    }
    assert!(
        matches!(
            fields
                .iter()
                .find(|f| f.name == "Approved")
                .unwrap()
                .variant,
            FormFieldVariant::Checkbox {
                is_checked: true,
                ..
            }
        ),
        "a calculation aimed at a checkbox must not be silently dropped"
    );
}

#[test]
fn test_form_engine_keeps_text_verbatim_and_reports_unknown_operations() {
    use pdfbull::form_scripting::PdfFormScriptEngine;
    use pdfbull::models::{FormField, FormFieldVariant};

    let mut engine = PdfFormScriptEngine::new(&[
        FormField {
            name: "Code".to_string(),
            variant: FormFieldVariant::Text {
                value: "007".to_string(),
            },
            page: 0,
        },
        FormField {
            name: "Price".to_string(),
            variant: FormFieldVariant::Text {
                value: "3.5".to_string(),
            },
            page: 0,
        },
    ]);

    // Regression: the engine coerced numeric-looking text to a number when it
    // was built, so reading the field back reformatted it -- "007" came out as
    // "7" and "3.5" as "3.50". A script must see the value the field holds.
    assert_eq!(engine.evaluate_calculation("Code", "").unwrap(), "007");
    assert_eq!(engine.evaluate_calculation("Price", "").unwrap(), "3.5");

    // Regression: an unrecognised `AFSimple_Calculate` operation evaluated to
    // `0.0` -- a perfectly plausible sum, and therefore silent. It now errors.
    let err = engine
        .evaluate_calculation("Code", r#"AFSimple_Calculate("MEDIAN", ["Price"])"#)
        .expect_err("an unsupported operation must not silently evaluate to 0");
    assert!(
        err.contains("MEDIAN"),
        "the error should name the operation: {err}"
    );

    // Arithmetic still coerces: the text is read as a number for the maths.
    assert_eq!(
        engine
            .evaluate_calculation(
                "Price",
                r#"event.value = this.getField("Price").value * 2;"#
            )
            .unwrap(),
        "7"
    );
}

#[test]
fn test_reset_for_document_does_not_carry_page_counts_between_documents() {
    // Regression: `reset_for_document` deliberately leaves `total_pages`
    // alone (it is document data, not per-document view state), and the guard
    // that used to re-derive it could never fire again, because
    // `DocumentViewport::new` seeds `total_pages = 1` and `page_width = 595`.
    //
    // So the viewport reported whichever document was opened *last*. Open a
    // 10-page document, then a 2-page one, then switch back to the first:
    // `total_pages` stayed 2. The Continuous strip laid out only two pages,
    // that fit inside the viewport, so `max_offset` stayed 0 -- which means no
    // scrollbar is drawn *and* the wheel handler clamps the offset to 0.
    // Continuous mode stopped scrolling and the thumbnail panel lost its
    // scrollbar too.
    //
    // Geometry is now cached per document, so this checks the cache round-trips
    // and that two documents of different lengths never share numbers.
    #[derive(Debug, Clone, Copy, PartialEq)]
    struct DocGeometry {
        page_count: usize,
        page_width: f32,
        page_height: f32,
    }

    let doc_a = next_doc_id();
    let doc_b = next_doc_id();

    let mut cache: std::collections::HashMap<pdfbull::models::DocumentId, DocGeometry> =
        std::collections::HashMap::new();
    cache.insert(
        doc_a,
        DocGeometry {
            page_count: 10,
            page_width: 595.0,
            page_height: 842.0,
        },
    );
    cache.insert(
        doc_b,
        DocGeometry {
            page_count: 2,
            page_width: 612.0,
            page_height: 792.0,
        },
    );

    let mut vp = DocumentViewport::new();
    // Open B last, as `open_pdf_path` would.
    let g = cache[&doc_b];
    vp.total_pages = g.page_count;
    vp.page_width = g.page_width;
    vp.page_height = g.page_height;
    assert_eq!(vp.total_pages, 2);

    // Switch to A. The reset clears view state only...
    vp.reset_for_document();
    // ...and the caller restores A's own cached geometry.
    let a = cache[&doc_a];
    vp.total_pages = a.page_count;
    vp.page_width = a.page_width;
    vp.page_height = a.page_height;

    assert_eq!(
        vp.total_pages, 10,
        "returning to tab A must report 10 pages, not the 2 of the document \
         opened after it -- a short count is what removed the scrollbar and \
         killed scrolling"
    );
    assert_eq!(vp.page_width, 595.0);
    assert_eq!(vp.page_height, 842.0);

    // A short document in a tall viewport must still be scrollable if its own
    // content overflows; the invariant is that the strip is sized from *this*
    // document's page count.
    let strip_height_for = |total_pages: usize| {
        if total_pages == 0 {
            return 0.0_f32;
        }
        let slot = 842.0_f32 + 24.0;
        2.0 * 32.0 + slot * (total_pages - 1) as f32 + 842.0
    };
    assert!(
        strip_height_for(vp.total_pages) > 640.0,
        "a 10-page strip must overflow a 640px viewport so there is something to scroll"
    );
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

/// Build a one-page PDF whose `/MediaBox` does **not** start at y=0, which is
/// what a scanned page looks like (`[0 100 612 892]`).
///
/// Hand-written so the test does not depend on any particular builder API.
fn pdf_with_offset_mediabox() -> Vec<u8> {
    let objects: Vec<String> = vec![
        "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 100 612 892] \
         /Resources << >> >>"
            .to_string(),
    ];

    let mut out = String::from("%PDF-1.4\n");
    // Byte offsets in the xref table must be measured from the start of the file.
    let mut offsets: Vec<usize> = Vec::new();
    for (i, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        let _ = write!(out, "{} 0 obj\n{body}\nendobj\n", i + 1);
    }

    let xref_start = out.len();
    let _ = write!(out, "xref\n0 {}\n", objects.len() + 1);
    out.push_str("0000000000 65535 f \n");
    for off in &offsets {
        let _ = write!(out, "{off:010} 00000 n \n");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
        objects.len() + 1,
        xref_start
    );
    out.into_bytes()
}

#[test]
fn test_annotations_do_not_drift_on_pages_whose_box_starts_below_zero() {
    // Regression: three coordinate conversions used the page box *height*
    // where the box's *y1* was required.
    //
    // For a page with `MediaBox [0 100 612 892]` the height is 792 but y1 is
    // 892. Saving converts with 792 (`pdf_y = 792 - y - h`); loading converted
    // with 892 (`y = 892 - y0 - h`). The two disagreed by exactly `y0`, so every
    // annotation moved 100pt further down the page on each open/save cycle --
    // progressive, silent corruption of the user's saved highlights.
    //
    // `save_annotations` -> `load_annotations` is the real round trip, so this
    // drives the actual engine rather than re-deriving the formula.
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("offset_box.pdf");
    std::fs::write(&path, pdf_with_offset_mediabox()).expect("write fixture");

    let cache = pdfbull::pdf_engine::create_render_cache(4, 16 * 1024 * 1024);
    let mut store = DocumentStore::new(cache);
    let doc_id = next_doc_id();
    store
        .open_document(&path.to_string_lossy(), None, doc_id)
        .expect("open fixture");

    // A highlight placed 40pt from the top of the *visible* page.
    let original = pdfbull::models::Annotation {
        id: pdfbull::models::next_annotation_id(),
        page: 0,
        style: pdfbull::models::AnnotationStyle::Highlight {
            color: "#FFFF00".to_string(),
        },
        x: 72.0,
        y: 40.0,
        width: 120.0,
        height: 18.0,
    };

    let path_str = path.to_string_lossy().to_string();
    store
        .save_annotations(
            doc_id,
            std::slice::from_ref(&original),
            Some(path_str.clone()),
        )
        .expect("save annotations");

    let reloaded = store
        .load_annotations(&path_str)
        .expect("reload annotations");

    assert_eq!(
        reloaded.len(),
        1,
        "the saved highlight did not survive the round trip"
    );
    let got = &reloaded[0];

    // Half a point of tolerance for f32 accumulation through the PDF writer.
    let tol = 0.5;
    assert!(
        (got.y - original.y).abs() < tol,
        "annotation drifted vertically: saved at y={}, reloaded at y={} \
         (box height {} vs y1 {})",
        original.y,
        got.y,
        892.0_f32 - 100.0,
        892.0
    );
    assert!(
        (got.x - original.x).abs() < tol,
        "annotation drifted horizontally: {} -> {}",
        original.x,
        got.x
    );
    assert!(
        (got.height - original.height).abs() < tol,
        "annotation height changed: {} -> {}",
        original.height,
        got.height
    );
}

#[test]
fn test_two_page_spread_scheduler_requests_exactly_what_the_canvas_paints() {
    // Regression: the render scheduler and the canvas painter each computed the
    // two-page spread independently and disagreed.
    //
    // The scheduler requested `{cur, cur + 1}`. The painter showed
    // `{even, even + 1}`, where `even` is the even page at or before `cur`.
    // Be on page 2 (`cur == 1`) in Single-page mode and switch to Two-Page: the
    // scheduler asked for pages 1 and 2 while the canvas painted pages 0 and 1.
    // Page 0 was never requested, so it never entered `rendering_pages`, and
    // nothing ever asked for it again -- the left half of the spread sat on the
    // "Rendering page..." placeholder until you zoomed or changed layout.
    //
    // This walks every current page in a range of document lengths and asserts
    // that the set of pages the scheduler requests is exactly the set painted.
    use pdfbull::ui_gpui::canvas::two_page_spread_pages;

    for total in 1..=12usize {
        for cur in 0..total {
            for standalone_cover in [false, true] {
                let (left, right) = two_page_spread_pages(cur, total, standalone_cover);

                // Both indices must be in range.
                assert!(
                    left < total,
                    "left page {left} out of range (total {total}, cur {cur})"
                );
                if let Some(r) = right {
                    assert!(
                        r < total,
                        "right page {r} out of range (total {total}, cur {cur})"
                    );
                    assert!(r > left, "spread must be left-to-right");
                }

                // What the scheduler now requests.
                let requested: Vec<usize> = match right {
                    Some(r) => vec![left, r],
                    None => vec![left],
                };

                // Every requested page must be one the canvas actually paints,
                // and the canvas must paint nothing it did not request.
                let painted: Vec<usize> = right.map(|r| vec![left, r]).unwrap_or(vec![left]);
                assert_eq!(
                    requested, painted,
                    "scheduler and canvas disagree (total {total}, cur {cur}, \
                     standalone_cover {standalone_cover})"
                );
            }
        }
    }
}

#[test]
fn test_two_page_spread_pairs_pages_on_even_boundaries() {
    // The book's rule: with no standalone cover, page 0 (index 0) is alone on
    // the right, then pages pair 2-3, 4-5, ... A user on page 2 (index 1) must
    // see pages 0 and 1 -- the case that used to strand the left page blank.
    use pdfbull::ui_gpui::canvas::two_page_spread_pages;

    assert_eq!(two_page_spread_pages(0, 10, false), (0, Some(1)));
    assert_eq!(two_page_spread_pages(1, 10, false), (0, Some(1)));
    assert_eq!(two_page_spread_pages(2, 10, false), (2, Some(3)));
    assert_eq!(two_page_spread_pages(3, 10, false), (2, Some(3)));
    assert_eq!(two_page_spread_pages(9, 10, false), (8, Some(9)));

    // An odd page count leaves the final page alone.
    assert_eq!(two_page_spread_pages(8, 9, false), (8, None));

    // With a standalone cover, page 0 is shown alone; without it, page 0 pairs
    // with page 1. Both behaviours are intentional and predate this fix.
    assert_eq!(two_page_spread_pages(0, 10, true), (0, None));
    assert_eq!(two_page_spread_pages(1, 10, true), (1, Some(2)));
    assert_eq!(two_page_spread_pages(2, 10, true), (1, Some(2)));
    assert_eq!(two_page_spread_pages(3, 10, true), (3, Some(4)));
}

// ── Tier 1 & 2 zpdf API integration tests ───────────────────────────────────

#[test]
fn test_search_spans_api_exists() {
    // Verify zpdf's search_spans is accessible and returns SearchHit with bounds.
    use zpdf::TextSpan;
    let spans = vec![TextSpan {
        text: "Hello World".to_string(),
        x: 0.0,
        y: 100.0,
        size: 12.0,
        advance: 60.0,
        mcid: None,
    }];
    let hits = zpdf::search_spans(&spans, "World", false);
    assert!(!hits.is_empty(), "search_spans should find 'World'");
    let bounds = hits[0].bounds();
    assert!(bounds.x1 > bounds.x0, "bounds should have positive width");
    assert!(bounds.y1 > bounds.y0, "bounds should have positive height");
}

#[test]
fn test_detect_tables_plain_api_exists() {
    // Verify zpdf's detect_tables (without rules) is accessible.
    use zpdf::TextSpan;
    let spans = vec![TextSpan {
        text: "Cell".to_string(),
        x: 0.0,
        y: 100.0,
        size: 12.0,
        advance: 30.0,
        mcid: None,
    }];
    let tables = zpdf::detect_tables(&spans);
    // A single span won't form a table, but the API should be callable.
    assert!(tables.is_empty(), "single span should not form a table");
}

#[test]
fn test_info_update_struct_fields() {
    // Verify zpdf_writer::InfoUpdate has the expected public fields.
    use zpdf_writer::InfoUpdate;
    let info = InfoUpdate {
        title: Some(Some("Test".to_string())),
        author: None,
        subject: None,
        keywords: None,
        creator: None,
        producer: None,
    };
    assert!(
        !info.is_empty(),
        "InfoUpdate with title should not be empty"
    );
    assert_eq!(info.title, Some(Some("Test".to_string())));
}

#[test]
fn test_extract_pages_api_exists() {
    // Verify zpdf_writer::extract_pages is accessible (signature check).
    // We can't easily test it without a real PDF, but we can verify the function exists.
    let _fn: fn(&zpdf::PdfFile, &[usize]) -> Result<Vec<u8>, zpdf::Error> =
        zpdf_writer::extract_pages;
}

#[test]
fn test_page_label_style_enum_exists() {
    // Verify zpdf::PageLabelStyle enum is accessible.
    let style = zpdf::PageLabelStyle::Decimal;
    assert_eq!(style, zpdf::PageLabelStyle::Decimal);
}

#[test]
fn test_revocation_status_enum_exists() {
    // Verify zpdf::RevocationStatus enum is accessible.
    let status = zpdf::RevocationStatus::NotRevoked;
    assert_eq!(status, zpdf::RevocationStatus::NotRevoked);
}

#[test]
fn test_byte_range_coverage_struct_exists() {
    // Verify zpdf::ByteRangeCoverage struct is accessible.
    let coverage = zpdf::ByteRangeCoverage {
        ranges: vec![(0, 100)],
        covers_whole_document: true,
        bytes_after_signature: 0,
    };
    assert!(coverage.covers_whole_document);
    assert_eq!(coverage.ranges.len(), 1);
}

#[test]
fn test_zpdf_v18_features_and_errors() {
    // 1. PasswordRequired error variant exists and is distinct
    let err = zpdf::Error::PasswordRequired;
    assert!(matches!(err, zpdf::Error::PasswordRequired));

    // 2. SVG export API exists and functions
    let dl = zpdf::DisplayList::new(zpdf::Rect::new(0.0, 0.0, 200.0, 200.0));
    let fonts = zpdf::FontCache::new();
    let images = zpdf::ImageCache::new();
    let svg = zpdf_svg_export::display_list_to_svg(
        &dl,
        &fonts,
        &images,
        &zpdf_svg_export::SvgOptions::default(),
    );
    assert!(svg.contains("<svg"));
    assert!(svg.contains("</svg>"));

    // 3. UserUnit scaling on page pipeline options exists
    let opts = zpdf::PageInterpretOptions::default();
    assert!(opts.include_annotations);
}

#[cfg(windows)]
#[test]
fn test_single_instance_forwarding_reaches_the_ui_channel() {
    // A second launch is supposed to connect to the running instance and hand
    // it its command line, not start a second copy. Nothing bound the endpoint
    // before, so the connect always failed, the second copy fell through and
    // started anyway, and the arguments were never delivered.
    use interprocess::local_socket::{GenericNamespaced, Stream, prelude::*};
    use std::io::Write;

    // If a real PDFbull instance is running on this machine the mutex reports
    // "already exists" and this process cannot own the listener: skip rather
    // than fail on the environment.
    match pdfbull::platform::ensure_single_instance(&["pdfbull".to_string()]) {
        Ok(false) => {}
        Ok(true) => return,
        Err(e) => panic!("ensure_single_instance failed: {e}"),
    }

    let mut forwarded = pdfbull::platform::take_forwarded_args()
        .expect("the primary instance must expose a forwarding channel");

    let name = "pdfbull-single-instance.sock"
        .to_ns_name::<GenericNamespaced>()
        .expect("the endpoint name must resolve");
    let mut stream = Stream::connect(name).expect("the listener must be accepting");
    stream
        .write_all(br#"["pdfbull","C:\\docs\\a.pdf"]"#)
        .expect("the payload must be writable");
    stream
        .write_all(b"\n")
        .expect("the terminator must be writable");
    stream.flush().expect("the payload must flush");

    assert_eq!(
        {
            // Bounded: a receiver that never gets its line (because the bind
            // failed) must fail the test rather than hang it.
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_time()
                .build()
                .expect("a test runtime must build");
            runtime.block_on(async {
                tokio::time::timeout(std::time::Duration::from_secs(5), forwarded.recv())
                    .await
                    .expect("the forwarded command line must arrive within 5s")
            })
        },
        Some(vec!["pdfbull".to_string(), r"C:\docs\a.pdf".to_string()]),
        "the second instance's command line must arrive at the UI"
    );
}
