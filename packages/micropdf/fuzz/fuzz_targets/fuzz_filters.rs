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

        // Try FlateDecode (zlib decompression)
        let stream = fz_open_buffer(ctx, buf);
        if !stream.is_null() {
            let flate_stream = fz_open_flated(ctx, stream, 0);
            if !flate_stream.is_null() {
                // Try to read decompressed data
                let mut read_buf = vec![0u8; 1024];
                let _ = fz_read(ctx, flate_stream, read_buf.as_mut_ptr(), read_buf.len());
                fz_drop_stream(ctx, flate_stream);
            }
            fz_drop_stream(ctx, stream);
        }

        // Try ASCII85 decode
        let stream2 = fz_open_buffer(ctx, buf);
        if !stream2.is_null() {
            let a85_stream = fz_open_a85d(ctx, stream2, 0);
            if !a85_stream.is_null() {
                let mut read_buf = vec![0u8; 1024];
                let _ = fz_read(ctx, a85_stream, read_buf.as_mut_ptr(), read_buf.len());
                fz_drop_stream(ctx, a85_stream);
            }
            fz_drop_stream(ctx, stream2);
        }

        // Try ASCIIHex decode
        let stream3 = fz_open_buffer(ctx, buf);
        if !stream3.is_null() {
            let ahx_stream = fz_open_ahxd(ctx, stream3, 0);
            if !ahx_stream.is_null() {
                let mut read_buf = vec![0u8; 1024];
                let _ = fz_read(ctx, ahx_stream, read_buf.as_mut_ptr(), read_buf.len());
                fz_drop_stream(ctx, ahx_stream);
            }
            fz_drop_stream(ctx, stream3);
        }

        // Try RLE decode
        let stream4 = fz_open_buffer(ctx, buf);
        if !stream4.is_null() {
            let rle_stream = fz_open_rld(ctx, stream4, 0);
            if !rle_stream.is_null() {
                let mut read_buf = vec![0u8; 1024];
                let _ = fz_read(ctx, rle_stream, read_buf.as_mut_ptr(), read_buf.len());
                fz_drop_stream(ctx, rle_stream);
            }
            fz_drop_stream(ctx, stream4);
        }

        // Clean up
        fz_drop_buffer(ctx, buf);
        fz_drop_context(ctx);
    }
});

