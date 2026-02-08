#![no_main]

use libfuzzer_sys::fuzz_target;
use micropdf::ffi::*;
use std::ptr;

fuzz_target!(|data: &[u8]| {
    if data.is_empty() {
        return;
    }

    unsafe {
        // Create context
        let ctx = fz_new_context(ptr::null_mut(), ptr::null_mut(), 0);
        if ctx.is_null() {
            return;
        }

        // Try to parse as PDF and access objects
        let buf = fz_new_buffer_from_copied_data(ctx, data.as_ptr(), data.len());
        if buf.is_null() {
            fz_drop_context(ctx);
            return;
        }

        let doc = fz_open_document_with_buffer(ctx, buf);
        
        if !doc.is_null() {
            // Try to get trailer
            let trailer = pdf_trailer(ctx, doc);
            if !trailer.is_null() {
                // Check if it's a dictionary
                let is_dict = pdf_is_dict(ctx, trailer);
                if is_dict != 0 {
                    // Try to get dictionary length
                    let len = pdf_dict_len(ctx, trailer);
                    
                    // Iterate through some keys (limited to avoid timeout)
                    for i in 0..len.min(10) {
                        let key = pdf_dict_get_key(ctx, trailer, i);
                        if !key.is_null() {
                            let _ = pdf_dict_get(ctx, trailer, key);
                        }
                    }
                }
            }

            // Try to access catalog
            let catalog = pdf_catalog(ctx, doc);
            if !catalog.is_null() {
                // Try to resolve object
                let resolved = pdf_resolve_indirect(ctx, catalog);
                if !resolved.is_null() {
                    let _ = pdf_is_dict(ctx, resolved);
                }
            }

            fz_drop_document(ctx, doc);
        }

        fz_drop_buffer(ctx, buf);
        fz_drop_context(ctx);
    }
});

