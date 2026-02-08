#![no_main]

use libfuzzer_sys::fuzz_target;
use micropdf::ffi::*;
use std::ptr;

fuzz_target!(|data: &[u8]| {
    if data.len() < 20 {
        return;
    }

    unsafe {
        // Create context
        let ctx = fz_new_context(ptr::null_mut(), ptr::null_mut(), 0);
        if ctx.is_null() {
            return;
        }

        // Create buffer from fuzz data
        let buf = fz_new_buffer_from_copied_data(ctx, data.as_ptr(), data.len());
        if buf.is_null() {
            fz_drop_context(ctx);
            return;
        }

        // Try to open as PDF
        let doc = fz_open_document_with_buffer(ctx, buf);

        if !doc.is_null() {
            let page_count = fz_count_pages(ctx, doc);

            // Try to render first few pages (limited to avoid timeout)
            for page_num in 0..page_count.min(3) {
                let page = fz_load_page(ctx, doc, page_num);
                if page.is_null() {
                    continue;
                }

                // Get page bounds
                let bounds = fz_bound_page(ctx, page);

                // Create a small pixmap for rendering
                let width = ((bounds.x1 - bounds.x0).abs() as i32).min(100);
                let height = ((bounds.y1 - bounds.y0).abs() as i32).min(100);

                if width > 0 && height > 0 {
                    let pix = fz_new_pixmap_rgb(ctx, width, height, 0);
                    if !pix.is_null() {
                        // Clear pixmap
                        fz_clear_pixmap_white(ctx, pix);

                        // Create device
                        let dev = fz_new_draw_device_with_bbox(ctx, pix);
                        if !dev.is_null() {
                            // Run page (may fail, that's okay)
                            let _ = fz_run_page(ctx, page, dev);
                            fz_drop_device(ctx, dev);
                        }

                        fz_drop_pixmap(ctx, pix);
                    }
                }

                // Try to get page text
                let stext = fz_new_stext_page_from_page(ctx, page);
                if !stext.is_null() {
                    fz_drop_stext_page(ctx, stext);
                }

                // Try to get page links
                let links = fz_load_links(ctx, page);
                if !links.is_null() {
                    fz_drop_link(ctx, links);
                }

                fz_drop_page(ctx, page);
            }

            fz_drop_document(ctx, doc);
        }

        fz_drop_buffer(ctx, buf);
        fz_drop_context(ctx);
    }
});




