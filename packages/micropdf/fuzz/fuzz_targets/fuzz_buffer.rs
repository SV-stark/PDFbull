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

        // Test buffer creation and operations
        let buf = fz_new_buffer(ctx, data.len());
        if buf.is_null() {
            fz_drop_context(ctx);
            return;
        }

        // Append data in chunks
        for chunk in data.chunks(1024) {
            let result = fz_append_data(ctx, buf, chunk.as_ptr(), chunk.len());
            if result != 0 {
                break;
            }
        }

        // Get buffer storage
        let mut ptr: *const u8 = ptr::null();
        let len = fz_buffer_storage(ctx, buf, &mut ptr as *mut *const u8 as *mut *mut u8);
        
        if !ptr.is_null() && len > 0 {
            // Try to read the data
            let _ = std::slice::from_raw_parts(ptr, len);
        }

        // Clear buffer
        fz_clear_buffer(ctx, buf);

        // Append again
        if !data.is_empty() {
            let _ = fz_append_data(ctx, buf, data.as_ptr(), data.len().min(1024));
        }

        // Clean up
        fz_drop_buffer(ctx, buf);
        fz_drop_context(ctx);
    }
});

