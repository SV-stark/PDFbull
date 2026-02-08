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

            // Check annotations on first few pages
            for page_num in 0..page_count.min(5) {
                let page = fz_load_page(ctx, doc, page_num);
                if page.is_null() {
                    continue;
                }

                // Get annotation count
                let annot_count = pdf_annot_count(ctx, page);

                // Iterate through annotations
                for i in 0..annot_count.min(20) {
                    let annot = pdf_get_annot(ctx, page, i);
                    if annot.is_null() {
                        continue;
                    }

                    // Get annotation type
                    let _ = pdf_annot_type(ctx, annot);

                    // Get annotation rect
                    let _ = pdf_annot_rect(ctx, annot);

                    // Try to get annotation contents
                    let contents = pdf_annot_contents(ctx, annot);
                    if !contents.is_null() {
                        // Just access the string, don't need to do anything with it
                        let _ = contents;
                    }

                    // Try to get annotation author
                    let _ = pdf_annot_author(ctx, annot);

                    // Try to get modification date
                    let _ = pdf_annot_modification_date(ctx, annot);
                }

                fz_drop_page(ctx, page);
            }

            fz_drop_document(ctx, doc);
        }

        fz_drop_buffer(ctx, buf);
        fz_drop_context(ctx);
    }
});




