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

        // Create buffer from data
        let buf = fz_new_buffer_from_copied_data(ctx, data.as_ptr(), data.len());
        if buf.is_null() {
            fz_drop_context(ctx);
            return;
        }

        // Open stream from buffer
        let stream = fz_open_buffer(ctx, buf);
        if !stream.is_null() {
            // Try to read from stream in chunks
            let mut read_buf = vec![0u8; 256];
            loop {
                let n = fz_read(ctx, stream, read_buf.as_mut_ptr(), read_buf.len());
                if n <= 0 {
                    break;
                }
            }

            // Try to seek
            let _ = fz_seek(ctx, stream, 0, 0); // SEEK_SET
            
            // Read a single byte
            let _ = fz_read_byte(ctx, stream);

            // Try to peek
            let _ = fz_peek_byte(ctx, stream);

            // Clean up stream
            fz_drop_stream(ctx, stream);
        }

        // Clean up
        fz_drop_buffer(ctx, buf);
        fz_drop_context(ctx);
    }
});

