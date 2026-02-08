#![no_main]

use libfuzzer_sys::fuzz_target;
use micropdf::ffi::*;
use std::ptr;

fuzz_target!(|data: &[u8]| {
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
            // Document opened successfully, try basic operations
            
            // Get page count (may fail, that's okay)
            let _ = fz_count_pages(ctx, doc);
            
            // Try to load first page
            let page = fz_load_page(ctx, doc, 0);
            if !page.is_null() {
                // Get page bounds
                let _ = fz_bound_page(ctx, page);
                
                // Clean up page
                fz_drop_page(ctx, page);
            }
            
            // Clean up document
            fz_drop_document(ctx, doc);
        }
        
        // Clean up buffer and context
        fz_drop_buffer(ctx, buf);
        fz_drop_context(ctx);
    }
});

