//! Integration tests for MicroPDF
//!
//! These tests verify that the library can correctly parse and handle
//! various PDF document types and content.

use std::path::PathBuf;

/// Get the path to a test fixture
fn fixture_path(name: &str) -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests");
    path.push("fixtures");
    path.push(name);
    path
}

/// Read a fixture file as bytes
fn read_fixture(name: &str) -> Vec<u8> {
    std::fs::read(fixture_path(name)).unwrap_or_else(|_| panic!("Failed to read fixture: {}", name))
}

mod pdf_header {
    use super::*;

    #[test]
    fn test_minimal_pdf_header() {
        let data = read_fixture("minimal.pdf");
        assert!(data.starts_with(b"%PDF-1."));
        assert!(data.len() > 100); // Minimal PDF should be at least this size
    }

    #[test]
    fn test_comprehensive_pdf_header() {
        let data = read_fixture("comprehensive_test.pdf");
        assert!(data.starts_with(b"%PDF-1.7"));
    }

    #[test]
    fn test_multipage_pdf_header() {
        let data = read_fixture("multipage.pdf");
        assert!(data.starts_with(b"%PDF-1.4"));
    }
}

mod pdf_structure {
    use super::*;

    fn find_pattern(data: &[u8], pattern: &[u8]) -> bool {
        data.windows(pattern.len()).any(|w| w == pattern)
    }

    fn count_pattern(data: &[u8], pattern: &[u8]) -> usize {
        data.windows(pattern.len())
            .filter(|w| *w == pattern)
            .count()
    }

    #[test]
    fn test_minimal_has_catalog() {
        let data = read_fixture("minimal.pdf");
        assert!(find_pattern(&data, b"/Type /Catalog"));
    }

    #[test]
    fn test_minimal_has_pages() {
        let data = read_fixture("minimal.pdf");
        assert!(find_pattern(&data, b"/Type /Pages"));
    }

    #[test]
    fn test_minimal_has_one_page() {
        let data = read_fixture("minimal.pdf");
        assert!(find_pattern(&data, b"/Type /Page "));
        assert!(find_pattern(&data, b"/Count 1"));
    }

    #[test]
    fn test_minimal_has_font() {
        let data = read_fixture("minimal.pdf");
        assert!(find_pattern(&data, b"/Type /Font"));
        assert!(find_pattern(&data, b"/BaseFont /Helvetica"));
    }

    #[test]
    fn test_minimal_has_content_stream() {
        let data = read_fixture("minimal.pdf");
        assert!(find_pattern(&data, b"/Contents"));
        assert!(find_pattern(&data, b"stream"));
        assert!(find_pattern(&data, b"endstream"));
    }

    #[test]
    fn test_minimal_has_xref() {
        let data = read_fixture("minimal.pdf");
        assert!(find_pattern(&data, b"xref"));
        assert!(find_pattern(&data, b"trailer"));
        assert!(find_pattern(&data, b"startxref"));
        assert!(find_pattern(&data, b"%%EOF"));
    }

    #[test]
    fn test_multipage_has_five_pages() {
        let data = read_fixture("multipage.pdf");
        assert!(find_pattern(&data, b"/Count 5"));

        // Count individual page objects (use /Page followed by space to avoid matching /Pages)
        let page_count = count_pattern(&data, b"/Type /Page ");
        assert_eq!(page_count, 5, "Expected 5 page objects");
    }

    #[test]
    fn test_comprehensive_has_three_pages() {
        let data = read_fixture("comprehensive_test.pdf");
        assert!(find_pattern(&data, b"/Count 3"));
    }
}

mod pdf_content_types {
    use super::*;

    fn find_pattern(data: &[u8], pattern: &[u8]) -> bool {
        data.windows(pattern.len()).any(|w| w == pattern)
    }

    #[test]
    fn test_comprehensive_has_outlines() {
        let data = read_fixture("comprehensive_test.pdf");
        assert!(find_pattern(&data, b"/Type /Outlines"));
        assert!(find_pattern(&data, b"/Title (Chapter 1"));
        assert!(find_pattern(&data, b"/Title (Chapter 2"));
    }

    #[test]
    fn test_comprehensive_has_metadata() {
        let data = read_fixture("comprehensive_test.pdf");
        assert!(find_pattern(&data, b"/Type /Metadata"));
        assert!(find_pattern(&data, b"<x:xmpmeta"));
        // Note: Test PDF still has NanoPDF branding - will be regenerated later
        assert!(find_pattern(&data, b"NanoPDF Comprehensive Test Document"));
    }

    #[test]
    fn test_comprehensive_has_annotations() {
        let data = read_fixture("comprehensive_test.pdf");
        // Link annotation
        assert!(find_pattern(&data, b"/Subtype /Link"));
        // Text annotation (sticky note)
        assert!(find_pattern(&data, b"/Subtype /Text"));
        // Highlight annotation
        assert!(find_pattern(&data, b"/Subtype /Highlight"));
    }

    #[test]
    fn test_comprehensive_has_form_fields() {
        let data = read_fixture("comprehensive_test.pdf");
        assert!(find_pattern(&data, b"/AcroForm"));
        // Text field
        assert!(find_pattern(&data, b"/FT /Tx"));
        // Button (checkbox)
        assert!(find_pattern(&data, b"/FT /Btn"));
        // Choice (dropdown)
        assert!(find_pattern(&data, b"/FT /Ch"));
    }

    #[test]
    fn test_comprehensive_has_image() {
        let data = read_fixture("comprehensive_test.pdf");
        assert!(find_pattern(&data, b"/Subtype /Image"));
        assert!(find_pattern(&data, b"/ColorSpace /DeviceRGB"));
    }

    #[test]
    fn test_comprehensive_has_multiple_fonts() {
        let data = read_fixture("comprehensive_test.pdf");
        assert!(find_pattern(&data, b"/BaseFont /Helvetica"));
        assert!(find_pattern(&data, b"/BaseFont /Times-Roman"));
    }

    #[test]
    fn test_comprehensive_has_named_destinations() {
        let data = read_fixture("comprehensive_test.pdf");
        assert!(find_pattern(&data, b"/Names"));
        assert!(find_pattern(&data, b"/Dests"));
    }

    #[test]
    fn test_comprehensive_has_graphics_state() {
        let data = read_fixture("comprehensive_test.pdf");
        assert!(find_pattern(&data, b"/Type /ExtGState"));
        assert!(find_pattern(&data, b"/CA "));
    }

    #[test]
    fn test_comprehensive_has_pattern() {
        let data = read_fixture("comprehensive_test.pdf");
        assert!(find_pattern(&data, b"/Type /Pattern"));
        assert!(find_pattern(&data, b"/PatternType 1"));
    }
}

mod pdf_text_content {
    use super::*;

    fn find_pattern(data: &[u8], pattern: &[u8]) -> bool {
        data.windows(pattern.len()).any(|w| w == pattern)
    }

    #[test]
    fn test_minimal_has_hello_world() {
        let data = read_fixture("minimal.pdf");
        assert!(find_pattern(&data, b"Hello, World!"));
    }

    #[test]
    fn test_multipage_has_page_numbers() {
        let data = read_fixture("multipage.pdf");
        assert!(find_pattern(&data, b"Page 1"));
        assert!(find_pattern(&data, b"Page 2"));
        assert!(find_pattern(&data, b"Page 3"));
        assert!(find_pattern(&data, b"Page 4"));
        assert!(find_pattern(&data, b"Page 5"));
    }

    #[test]
    fn test_comprehensive_has_feature_list() {
        let data = read_fixture("comprehensive_test.pdf");
        assert!(find_pattern(&data, b"Multiple fonts"));
        assert!(find_pattern(&data, b"Images"));
        assert!(find_pattern(&data, b"Annotations"));
        assert!(find_pattern(&data, b"Bookmarks/Outlines"));
    }
}

mod pdf_encryption {
    use super::*;

    fn find_pattern(data: &[u8], pattern: &[u8]) -> bool {
        data.windows(pattern.len()).any(|w| w == pattern)
    }

    #[test]
    fn test_encrypted_has_encrypt_dict() {
        let data = read_fixture("encrypted_empty_password.pdf");
        assert!(find_pattern(&data, b"/Encrypt"));
        assert!(find_pattern(&data, b"/Filter /Standard"));
    }

    #[test]
    fn test_encrypted_has_id() {
        let data = read_fixture("encrypted_empty_password.pdf");
        assert!(find_pattern(&data, b"/ID"));
    }
}

mod pdf_geometry {
    use super::*;

    fn find_pattern(data: &[u8], pattern: &[u8]) -> bool {
        data.windows(pattern.len()).any(|w| w == pattern)
    }

    #[test]
    fn test_all_pdfs_have_mediabox() {
        for fixture in ["minimal.pdf", "multipage.pdf", "comprehensive_test.pdf"] {
            let data = read_fixture(fixture);
            assert!(
                find_pattern(&data, b"/MediaBox"),
                "Missing MediaBox in {}",
                fixture
            );
        }
    }

    #[test]
    fn test_comprehensive_has_cropbox() {
        let data = read_fixture("comprehensive_test.pdf");
        assert!(find_pattern(&data, b"/CropBox"));
    }

    #[test]
    fn test_standard_page_size() {
        // US Letter: 612 x 792 points
        let data = read_fixture("minimal.pdf");
        assert!(find_pattern(&data, b"[0 0 612 792]"));
    }
}

mod ffi_integration {
    use micropdf::ffi::buffer::*;
    use micropdf::ffi::context::*;
    use micropdf::ffi::geometry::*;

    #[test]
    fn test_ffi_context_lifecycle() {
        unsafe {
            let ctx = fz_new_context(std::ptr::null(), std::ptr::null(), 0);
            assert_ne!(ctx, 0);
            fz_drop_context(ctx);
        }
    }

    #[test]
    fn test_ffi_buffer_lifecycle() {
        unsafe {
            let ctx = fz_new_context(std::ptr::null(), std::ptr::null(), 0);
            let buf = fz_new_buffer(ctx, 1024);
            assert_ne!(buf, 0);

            fz_append_byte(ctx, buf, b'H' as i32);
            fz_append_byte(ctx, buf, b'i' as i32);

            assert_eq!(fz_buffer_len(ctx, buf), 2);

            fz_drop_buffer(ctx, buf);
            fz_drop_context(ctx);
        }
    }

    #[test]
    fn test_ffi_matrix_operations() {
        // Test identity
        let identity = fz_matrix::identity();
        assert_eq!(identity.a, 1.0);
        assert_eq!(identity.d, 1.0);

        // Test translation
        let translate = fz_translate(100.0, 200.0);
        assert_eq!(translate.e, 100.0);
        assert_eq!(translate.f, 200.0);

        // Test scale
        let scale = fz_scale(2.0, 3.0);
        assert_eq!(scale.a, 2.0);
        assert_eq!(scale.d, 3.0);

        // Test point transformation
        let point = fz_point { x: 10.0, y: 20.0 };
        let transformed = fz_transform_point(point, translate);
        assert_eq!(transformed.x, 110.0);
        assert_eq!(transformed.y, 220.0);
    }

    #[test]
    fn test_ffi_rect_operations() {
        let r1 = fz_rect {
            x0: 0.0,
            y0: 0.0,
            x1: 100.0,
            y1: 100.0,
        };
        let r2 = fz_rect {
            x0: 50.0,
            y0: 50.0,
            x1: 150.0,
            y1: 150.0,
        };

        // Test intersection
        let intersection = fz_intersect_rect(r1, r2);
        assert_eq!(intersection.x0, 50.0);
        assert_eq!(intersection.y0, 50.0);
        assert_eq!(intersection.x1, 100.0);
        assert_eq!(intersection.y1, 100.0);

        // Test union
        let union = fz_union_rect(r1, r2);
        assert_eq!(union.x0, 0.0);
        assert_eq!(union.y0, 0.0);
        assert_eq!(union.x1, 150.0);
        assert_eq!(union.y1, 150.0);

        // Test contains
        assert_eq!(
            fz_contains_rect(
                r1,
                fz_rect {
                    x0: 10.0,
                    y0: 10.0,
                    x1: 50.0,
                    y1: 50.0
                }
            ),
            1
        );
        assert_eq!(fz_contains_rect(r1, r2), 0);
    }

    #[test]
    fn test_ffi_quad_operations() {
        let rect = fz_rect {
            x0: 0.0,
            y0: 0.0,
            x1: 100.0,
            y1: 100.0,
        };
        let quad = fz_quad_from_rect(rect);

        assert_eq!(quad.ul.x, 0.0);
        assert_eq!(quad.ul.y, 0.0);
        assert_eq!(quad.lr.x, 100.0);
        assert_eq!(quad.lr.y, 100.0);

        // Test quad back to rect
        let back = fz_rect_from_quad(quad);
        assert_eq!(back.x0, rect.x0);
        assert_eq!(back.y0, rect.y0);
        assert_eq!(back.x1, rect.x1);
        assert_eq!(back.y1, rect.y1);
    }
}

mod colorspace_integration {
    use micropdf::ffi::colorspace::*;

    #[test]
    fn test_device_colorspaces() {
        let gray = fz_device_gray(0);
        let rgb = fz_device_rgb(0);
        let cmyk = fz_device_cmyk(0);

        assert_eq!(fz_colorspace_n(0, gray), 1);
        assert_eq!(fz_colorspace_n(0, rgb), 3);
        assert_eq!(fz_colorspace_n(0, cmyk), 4);

        assert_eq!(fz_colorspace_is_gray(0, gray), 1);
        assert_eq!(fz_colorspace_is_rgb(0, rgb), 1);
        assert_eq!(fz_colorspace_is_cmyk(0, cmyk), 1);
    }

    #[test]
    fn test_color_conversion() {
        let gray = fz_device_gray(0);
        let rgb = fz_device_rgb(0);

        let src = [0.5f32];
        let mut dst = [0.0f32; 3];

        fz_convert_color(0, gray, src.as_ptr(), rgb, dst.as_mut_ptr(), 0);

        // Gray to RGB should produce equal components
        assert!((dst[0] - 0.5).abs() < 0.01);
        assert!((dst[1] - 0.5).abs() < 0.01);
        assert!((dst[2] - 0.5).abs() < 0.01);
    }
}

mod pixmap_integration {
    use micropdf::ffi::colorspace::*;
    use micropdf::ffi::pixmap::*;

    #[test]
    fn test_pixmap_creation_and_manipulation() {
        let rgb = fz_device_rgb(0);
        let pix = fz_new_pixmap(0, rgb, 100, 100, 0, 1);

        assert_ne!(pix, 0);
        assert_eq!(fz_pixmap_width(0, pix), 100);
        assert_eq!(fz_pixmap_height(0, pix), 100);
        assert_eq!(fz_pixmap_alpha(0, pix), 1);
        assert_eq!(fz_pixmap_components(0, pix), 4); // RGB + alpha

        // Test clear
        fz_clear_pixmap_with_value(0, pix, 128);
        assert_eq!(fz_get_pixmap_sample(0, pix, 0, 0, 0), 128);

        // Test set/get
        fz_set_pixmap_sample(0, pix, 50, 50, 0, 255);
        assert_eq!(fz_get_pixmap_sample(0, pix, 50, 50, 0), 255);

        fz_drop_pixmap(0, pix);
    }

    #[test]
    fn test_pixmap_bbox() {
        use micropdf::ffi::geometry::fz_irect;

        let rgb = fz_device_rgb(0);
        let bbox = fz_irect {
            x0: 10,
            y0: 20,
            x1: 110,
            y1: 120,
        };
        let pix = fz_new_pixmap_with_bbox(0, rgb, bbox, 0, 0);

        assert_eq!(fz_pixmap_x(0, pix), 10);
        assert_eq!(fz_pixmap_y(0, pix), 20);
        assert_eq!(fz_pixmap_width(0, pix), 100);
        assert_eq!(fz_pixmap_height(0, pix), 100);

        fz_drop_pixmap(0, pix);
    }
}

mod stream_integration {
    use micropdf::ffi::context::*;
    use micropdf::ffi::stream::*;

    #[test]
    fn test_stream_from_memory() {
        unsafe {
            let ctx = fz_new_context(std::ptr::null(), std::ptr::null(), 0);
            let data = b"Hello, Stream!";
            let stm = fz_open_memory(ctx, data.as_ptr(), data.len());

            assert_ne!(stm, 0);

            // Read byte by byte
            assert_eq!(fz_read_byte(ctx, stm), b'H' as i32);
            assert_eq!(fz_read_byte(ctx, stm), b'e' as i32);
            assert_eq!(fz_read_byte(ctx, stm), b'l' as i32);

            // Check position
            assert_eq!(fz_tell(ctx, stm), 3);

            // Seek to beginning
            fz_seek(ctx, stm, 0, 0);
            assert_eq!(fz_tell(ctx, stm), 0);
            assert_eq!(fz_read_byte(ctx, stm), b'H' as i32);

            // Check EOF
            fz_seek(ctx, stm, 0, 2); // SEEK_END
            assert_eq!(fz_is_eof(ctx, stm), 1);

            fz_drop_stream(ctx, stm);
            fz_drop_context(ctx);
        }
    }
}
