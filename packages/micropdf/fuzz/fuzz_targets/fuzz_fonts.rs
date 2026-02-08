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
            // Get catalog
            let catalog = pdf_catalog(ctx, doc);
            if !catalog.is_null() {
                // Try to get font resources
                let resources = pdf_dict_gets(ctx, catalog, b"Resources\0".as_ptr() as *const i8);
                if !resources.is_null() && pdf_is_dict(ctx, resources) != 0 {
                    let fonts = pdf_dict_gets(ctx, resources, b"Font\0".as_ptr() as *const i8);
                    if !fonts.is_null() && pdf_is_dict(ctx, fonts) != 0 {
                        let font_count = pdf_dict_len(ctx, fonts);

                        // Iterate through fonts (limited)
                        for i in 0..font_count.min(10) {
                            let key = pdf_dict_get_key(ctx, fonts, i);
                            if key.is_null() {
                                continue;
                            }

                            let font_obj = pdf_dict_get(ctx, fonts, key);
                            if font_obj.is_null() {
                                continue;
                            }

                            // Try to resolve if indirect
                            let resolved = pdf_resolve_indirect(ctx, font_obj);
                            if resolved.is_null() || pdf_is_dict(ctx, resolved) == 0 {
                                continue;
                            }

                            // Get font subtype
                            let _ = pdf_dict_gets(
                                ctx,
                                resolved,
                                b"Subtype\0".as_ptr() as *const i8,
                            );

                            // Get base font
                            let _ = pdf_dict_gets(
                                ctx,
                                resolved,
                                b"BaseFont\0".as_ptr() as *const i8,
                            );

                            // Check for embedded font data
                            let _ = pdf_dict_gets(
                                ctx,
                                resolved,
                                b"FontFile\0".as_ptr() as *const i8,
                            );
                            let _ = pdf_dict_gets(
                                ctx,
                                resolved,
                                b"FontFile2\0".as_ptr() as *const i8,
                            );
                            let _ = pdf_dict_gets(
                                ctx,
                                resolved,
                                b"FontFile3\0".as_ptr() as *const i8,
                            );
                        }
                    }
                }
            }

            fz_drop_document(ctx, doc);
        }

        fz_drop_buffer(ctx, buf);
        fz_drop_context(ctx);
    }
});




