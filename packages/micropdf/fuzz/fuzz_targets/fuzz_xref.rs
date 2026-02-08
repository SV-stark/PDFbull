#![no_main]

use libfuzzer_sys::fuzz_target;
use micropdf::ffi::*;
use std::ptr;

fuzz_target!(|data: &[u8]| {
    if data.len() < 10 {
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

        // Try to open as PDF and access xref
        let doc = fz_open_document_with_buffer(ctx, buf);

        if !doc.is_null() {
            // Try to get xref length
            let xref_len = pdf_xref_len(ctx, doc);

            // Try to access some xref entries (limited to avoid timeout)
            for i in 0..xref_len.min(100) {
                // Get entry type
                let _ = pdf_xref_entry_type(ctx, doc, i);

                // Get generation number
                let _ = pdf_xref_entry_gen(ctx, doc, i);

                // Try to resolve object
                let obj = pdf_load_object(ctx, doc, i);
                if !obj.is_null() {
                    // Check type
                    let _ = pdf_is_dict(ctx, obj);
                    let _ = pdf_is_array(ctx, obj);
                    let _ = pdf_is_stream(ctx, obj);
                }
            }

            fz_drop_document(ctx, doc);
        }

        fz_drop_buffer(ctx, buf);
        fz_drop_context(ctx);
    }
});




