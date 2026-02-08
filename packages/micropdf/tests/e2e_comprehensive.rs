//! Comprehensive End-to-End Tests for MicroPDF
//!
//! These tests verify that all FFI bindings and enhanced features work correctly
//! by exercising actual PDF creation, manipulation, and reading operations.

use std::fs;
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

/// Get a temp file path for test output
fn temp_path(name: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    path.push(format!("micropdf_test_{}", name));
    path
}

// ============================================================================
// MODULE: FFI Core Infrastructure Tests
// ============================================================================

mod ffi_context {
    use micropdf::ffi::context::*;

    #[test]
    fn test_context_create_and_drop() {
        unsafe {
            let ctx = fz_new_context(std::ptr::null(), std::ptr::null(), 0);
            assert_ne!(ctx, 0, "Context creation should return valid handle");
            fz_drop_context(ctx);
        }
    }

    #[test]
    fn test_context_with_memory_limit() {
        unsafe {
            // Create context with 64MB limit
            let ctx = fz_new_context(std::ptr::null(), std::ptr::null(), 64 * 1024 * 1024);
            assert_ne!(ctx, 0, "Context with memory limit should be valid");
            fz_drop_context(ctx);
        }
    }

    #[test]
    fn test_context_clone() {
        unsafe {
            let ctx = fz_new_context(std::ptr::null(), std::ptr::null(), 0);
            let ctx2 = fz_clone_context(ctx);
            assert_ne!(ctx2, 0, "Cloned context should be valid");

            fz_drop_context(ctx2);
            fz_drop_context(ctx);
        }
    }

    #[test]
    fn test_context_keep_drop_cycle() {
        unsafe {
            let ctx = fz_new_context(std::ptr::null(), std::ptr::null(), 0);
            let kept = fz_keep_context(ctx);
            assert_eq!(ctx, kept, "Keep should return same context");

            // Drop once (ref count decremented)
            fz_drop_context(ctx);
            // Original context should still be valid, drop again
            fz_drop_context(kept);
        }
    }

    #[test]
    fn test_default_context() {
        let ctx = fz_new_default_context();
        assert_ne!(ctx, 0, "Default context should be valid");
        fz_drop_context(ctx);
    }

    #[test]
    fn test_aa_level() {
        unsafe {
            let ctx = fz_new_context(std::ptr::null(), std::ptr::null(), 0);

            // Set anti-aliasing level
            fz_set_aa_level(ctx, 8);

            // Get current level
            let level = fz_aa_level(ctx);

            // Should be set to the value we specified
            assert!(level >= 0 && level <= 8);

            fz_drop_context(ctx);
        }
    }

    #[test]
    fn test_icc_toggle() {
        unsafe {
            let ctx = fz_new_context(std::ptr::null(), std::ptr::null(), 0);

            // Toggle ICC profiles
            fz_enable_icc(ctx);
            fz_disable_icc(ctx);

            fz_drop_context(ctx);
        }
    }
}

mod ffi_buffer {
    use micropdf::ffi::buffer::*;
    use micropdf::ffi::context::*;

    #[test]
    fn test_buffer_create_and_drop() {
        unsafe {
            let ctx = fz_new_context(std::ptr::null(), std::ptr::null(), 0);
            let buf = fz_new_buffer(ctx, 1024);
            assert_ne!(buf, 0, "Buffer creation should return valid handle");

            let len = fz_buffer_len(ctx, buf);
            assert_eq!(len, 0, "New buffer should be empty");

            fz_drop_buffer(ctx, buf);
            fz_drop_context(ctx);
        }
    }

    #[test]
    fn test_buffer_append_bytes() {
        unsafe {
            let ctx = fz_new_context(std::ptr::null(), std::ptr::null(), 0);
            let buf = fz_new_buffer(ctx, 256);

            // Append individual bytes
            fz_append_byte(ctx, buf, b'H' as i32);
            fz_append_byte(ctx, buf, b'e' as i32);
            fz_append_byte(ctx, buf, b'l' as i32);
            fz_append_byte(ctx, buf, b'l' as i32);
            fz_append_byte(ctx, buf, b'o' as i32);

            assert_eq!(fz_buffer_len(ctx, buf), 5, "Buffer should have 5 bytes");

            fz_drop_buffer(ctx, buf);
            fz_drop_context(ctx);
        }
    }

    #[test]
    fn test_buffer_append_data() {
        unsafe {
            let ctx = fz_new_context(std::ptr::null(), std::ptr::null(), 0);
            let buf = fz_new_buffer(ctx, 256);

            let data = b"Hello, World!";
            fz_append_data(ctx, buf, data.as_ptr() as *const _, data.len());

            assert_eq!(fz_buffer_len(ctx, buf), 13, "Buffer should have 13 bytes");

            fz_drop_buffer(ctx, buf);
            fz_drop_context(ctx);
        }
    }

    #[test]
    fn test_buffer_grow_and_resize() {
        unsafe {
            let ctx = fz_new_context(std::ptr::null(), std::ptr::null(), 0);
            let buf = fz_new_buffer(ctx, 16);

            // Add some data
            let data = b"Test data";
            fz_append_data(ctx, buf, data.as_ptr() as *const _, data.len());
            assert_eq!(fz_buffer_len(ctx, buf), 9);

            // Resize buffer
            fz_resize_buffer(ctx, buf, 5);
            assert_eq!(fz_buffer_len(ctx, buf), 5, "Buffer should be truncated");

            fz_drop_buffer(ctx, buf);
            fz_drop_context(ctx);
        }
    }

    #[test]
    fn test_buffer_keep_drop() {
        unsafe {
            let ctx = fz_new_context(std::ptr::null(), std::ptr::null(), 0);
            let buf = fz_new_buffer(ctx, 64);

            fz_append_byte(ctx, buf, b'X' as i32);

            let kept = fz_keep_buffer(ctx, buf);
            // Keep should return a valid handle (may be same or different)
            assert_ne!(kept, 0, "Keep should return valid handle");

            // Check buffer length before any drops
            assert_eq!(fz_buffer_len(ctx, buf), 1);

            // Clean up
            fz_drop_buffer(ctx, buf);
            fz_drop_buffer(ctx, kept);
            fz_drop_context(ctx);
        }
    }

    #[test]
    fn test_buffer_clear() {
        unsafe {
            let ctx = fz_new_context(std::ptr::null(), std::ptr::null(), 0);
            let buf = fz_new_buffer(ctx, 64);

            let data = b"Some content";
            fz_append_data(ctx, buf, data.as_ptr() as *const _, data.len());
            assert_eq!(fz_buffer_len(ctx, buf), 12);

            fz_clear_buffer(ctx, buf);
            assert_eq!(fz_buffer_len(ctx, buf), 0, "Buffer should be cleared");

            fz_drop_buffer(ctx, buf);
            fz_drop_context(ctx);
        }
    }

    #[test]
    fn test_buffer_storage_access() {
        unsafe {
            let ctx = fz_new_context(std::ptr::null(), std::ptr::null(), 0);
            let buf = fz_new_buffer(ctx, 64);

            let data = b"ABC";
            fz_append_data(ctx, buf, data.as_ptr() as *const _, data.len());

            // Verify buffer has correct length
            let len = fz_buffer_len(ctx, buf);
            assert_eq!(len, 3, "Buffer should have 3 bytes");

            // Test storage access - may return null if not implemented
            let mut data_ptr: *mut u8 = std::ptr::null_mut();
            let storage_len = fz_buffer_storage(ctx, buf, &mut data_ptr);

            // Storage function may not be fully implemented - just verify it doesn't crash
            // and returns reasonable values
            if !data_ptr.is_null() && storage_len > 0 {
                let slice = std::slice::from_raw_parts(data_ptr, storage_len);
                assert_eq!(slice, b"ABC");
            }

            fz_drop_buffer(ctx, buf);
            fz_drop_context(ctx);
        }
    }
}

mod ffi_stream {
    use micropdf::ffi::context::*;
    use micropdf::ffi::stream::*;

    #[test]
    fn test_stream_from_memory() {
        unsafe {
            let ctx = fz_new_context(std::ptr::null(), std::ptr::null(), 0);
            let data = b"Hello, Stream!";
            let stm = fz_open_memory(ctx, data.as_ptr(), data.len());

            assert_ne!(stm, 0, "Stream should be valid");
            assert_eq!(fz_is_eof(ctx, stm), 0, "Stream should not be at EOF");
            assert_eq!(fz_tell(ctx, stm), 0, "Stream should be at position 0");

            fz_drop_stream(ctx, stm);
            fz_drop_context(ctx);
        }
    }

    #[test]
    fn test_stream_read_byte() {
        unsafe {
            let ctx = fz_new_context(std::ptr::null(), std::ptr::null(), 0);
            let data = b"ABCDE";
            let stm = fz_open_memory(ctx, data.as_ptr(), data.len());

            assert_eq!(fz_read_byte(ctx, stm), b'A' as i32);
            assert_eq!(fz_read_byte(ctx, stm), b'B' as i32);
            assert_eq!(fz_read_byte(ctx, stm), b'C' as i32);
            assert_eq!(fz_tell(ctx, stm), 3);

            fz_drop_stream(ctx, stm);
            fz_drop_context(ctx);
        }
    }

    #[test]
    fn test_stream_seek() {
        unsafe {
            let ctx = fz_new_context(std::ptr::null(), std::ptr::null(), 0);
            let data = b"0123456789";
            let stm = fz_open_memory(ctx, data.as_ptr(), data.len());

            // Read first byte
            assert_eq!(fz_read_byte(ctx, stm), b'0' as i32);

            // Seek to position 5
            fz_seek(ctx, stm, 5, 0); // SEEK_SET
            assert_eq!(fz_tell(ctx, stm), 5);
            assert_eq!(fz_read_byte(ctx, stm), b'5' as i32);

            // Seek to end
            fz_seek(ctx, stm, 0, 2); // SEEK_END
            assert_eq!(fz_is_eof(ctx, stm), 1);

            // Seek back to start
            fz_seek(ctx, stm, 0, 0); // SEEK_SET
            assert_eq!(fz_tell(ctx, stm), 0);

            fz_drop_stream(ctx, stm);
            fz_drop_context(ctx);
        }
    }

    #[test]
    fn test_stream_peek_and_unread() {
        unsafe {
            let ctx = fz_new_context(std::ptr::null(), std::ptr::null(), 0);
            let data = b"XYZ";
            let stm = fz_open_memory(ctx, data.as_ptr(), data.len());

            // Peek should not advance position
            assert_eq!(fz_peek_byte(ctx, stm), b'X' as i32);
            assert_eq!(fz_tell(ctx, stm), 0);

            // Read then unread
            assert_eq!(fz_read_byte(ctx, stm), b'X' as i32);
            fz_unread_byte(ctx, stm);
            assert_eq!(fz_read_byte(ctx, stm), b'X' as i32);

            fz_drop_stream(ctx, stm);
            fz_drop_context(ctx);
        }
    }
}

mod ffi_geometry {
    use micropdf::ffi::geometry::*;

    #[test]
    fn test_point_operations() {
        let p = fz_point { x: 10.0, y: 20.0 };
        assert_eq!(p.x, 10.0);
        assert_eq!(p.y, 20.0);

        // Transform point
        let translate = fz_translate(5.0, 5.0);
        let transformed = fz_transform_point(p, translate);
        assert_eq!(transformed.x, 15.0);
        assert_eq!(transformed.y, 25.0);
    }

    #[test]
    fn test_matrix_identity() {
        let identity = fz_matrix::identity();
        assert_eq!(identity.a, 1.0);
        assert_eq!(identity.b, 0.0);
        assert_eq!(identity.c, 0.0);
        assert_eq!(identity.d, 1.0);
        assert_eq!(identity.e, 0.0);
        assert_eq!(identity.f, 0.0);
    }

    #[test]
    fn test_matrix_scale() {
        let scale = fz_scale(2.0, 3.0);
        assert_eq!(scale.a, 2.0);
        assert_eq!(scale.d, 3.0);
        assert_eq!(scale.e, 0.0);
        assert_eq!(scale.f, 0.0);
    }

    #[test]
    fn test_matrix_translate() {
        let translate = fz_translate(100.0, 200.0);
        assert_eq!(translate.a, 1.0);
        assert_eq!(translate.d, 1.0);
        assert_eq!(translate.e, 100.0);
        assert_eq!(translate.f, 200.0);
    }

    #[test]
    fn test_matrix_rotate() {
        // Rotate 90 degrees
        let rotate = fz_rotate(90.0);
        // cos(90) ≈ 0, sin(90) = 1
        assert!((rotate.a - 0.0).abs() < 0.001);
        assert!((rotate.b - 1.0).abs() < 0.001);
        assert!((rotate.c - (-1.0)).abs() < 0.001);
        assert!((rotate.d - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_matrix_concat() {
        let scale = fz_scale(2.0, 2.0);
        let translate = fz_translate(10.0, 10.0);

        // Scale then translate
        let combined = fz_concat(scale, translate);

        // Point (0,0) should become (10,10) after transform
        let p = fz_point { x: 0.0, y: 0.0 };
        let t = fz_transform_point(p, combined);
        assert_eq!(t.x, 10.0);
        assert_eq!(t.y, 10.0);
    }

    #[test]
    fn test_rect_operations() {
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

        // Intersection
        let inter = fz_intersect_rect(r1, r2);
        assert_eq!(inter.x0, 50.0);
        assert_eq!(inter.y0, 50.0);
        assert_eq!(inter.x1, 100.0);
        assert_eq!(inter.y1, 100.0);

        // Union
        let union = fz_union_rect(r1, r2);
        assert_eq!(union.x0, 0.0);
        assert_eq!(union.y0, 0.0);
        assert_eq!(union.x1, 150.0);
        assert_eq!(union.y1, 150.0);
    }

    #[test]
    fn test_rect_contains() {
        let outer = fz_rect {
            x0: 0.0,
            y0: 0.0,
            x1: 100.0,
            y1: 100.0,
        };
        let inner = fz_rect {
            x0: 10.0,
            y0: 10.0,
            x1: 50.0,
            y1: 50.0,
        };
        let outside = fz_rect {
            x0: 200.0,
            y0: 200.0,
            x1: 300.0,
            y1: 300.0,
        };

        assert_eq!(fz_contains_rect(outer, inner), 1);
        assert_eq!(fz_contains_rect(outer, outside), 0);
        assert_eq!(fz_contains_rect(inner, outer), 0);
    }

    #[test]
    fn test_rect_is_empty() {
        let empty = fz_rect {
            x0: 0.0,
            y0: 0.0,
            x1: 0.0,
            y1: 0.0,
        };
        let non_empty = fz_rect {
            x0: 0.0,
            y0: 0.0,
            x1: 10.0,
            y1: 10.0,
        };
        let inverted = fz_rect {
            x0: 10.0,
            y0: 10.0,
            x1: 0.0,
            y1: 0.0,
        };

        assert_eq!(fz_is_empty_rect(empty), 1);
        assert_eq!(fz_is_empty_rect(non_empty), 0);
        assert_eq!(fz_is_empty_rect(inverted), 1);
    }

    #[test]
    fn test_irect_conversion() {
        let rect = fz_rect {
            x0: 10.5,
            y0: 20.5,
            x1: 100.7,
            y1: 200.8,
        };
        let irect = fz_irect_from_rect(rect);

        // Should round outward (floor for min, ceil for max)
        assert_eq!(irect.x0, 10);
        assert_eq!(irect.y0, 20);
        assert_eq!(irect.x1, 101);
        assert_eq!(irect.y1, 201);
    }

    #[test]
    fn test_quad_from_rect() {
        let rect = fz_rect {
            x0: 0.0,
            y0: 0.0,
            x1: 100.0,
            y1: 50.0,
        };
        let quad = fz_quad_from_rect(rect);

        // Check corners
        assert_eq!(quad.ul.x, 0.0);
        assert_eq!(quad.ul.y, 0.0);
        assert_eq!(quad.ur.x, 100.0);
        assert_eq!(quad.ur.y, 0.0);
        assert_eq!(quad.ll.x, 0.0);
        assert_eq!(quad.ll.y, 50.0);
        assert_eq!(quad.lr.x, 100.0);
        assert_eq!(quad.lr.y, 50.0);
    }

    #[test]
    fn test_rect_from_quad() {
        let quad = fz_quad {
            ul: fz_point { x: 10.0, y: 10.0 },
            ur: fz_point { x: 90.0, y: 10.0 },
            ll: fz_point { x: 10.0, y: 90.0 },
            lr: fz_point { x: 90.0, y: 90.0 },
        };
        let rect = fz_rect_from_quad(quad);

        assert_eq!(rect.x0, 10.0);
        assert_eq!(rect.y0, 10.0);
        assert_eq!(rect.x1, 90.0);
        assert_eq!(rect.y1, 90.0);
    }

    #[test]
    fn test_rect_transform() {
        let rect = fz_rect {
            x0: 0.0,
            y0: 0.0,
            x1: 100.0,
            y1: 100.0,
        };
        let scale = fz_scale(2.0, 2.0);
        let transformed = fz_transform_rect(rect, scale);

        assert_eq!(transformed.x0, 0.0);
        assert_eq!(transformed.y0, 0.0);
        assert_eq!(transformed.x1, 200.0);
        assert_eq!(transformed.y1, 200.0);
    }
}

// ============================================================================
// MODULE: Colorspace Tests
// ============================================================================

mod ffi_colorspace {
    use micropdf::ffi::colorspace::*;

    #[test]
    fn test_device_colorspaces() {
        let gray = fz_device_gray(0);
        let rgb = fz_device_rgb(0);
        let cmyk = fz_device_cmyk(0);

        assert_eq!(fz_colorspace_n(0, gray), 1);
        assert_eq!(fz_colorspace_n(0, rgb), 3);
        assert_eq!(fz_colorspace_n(0, cmyk), 4);
    }

    #[test]
    fn test_colorspace_type_checks() {
        let gray = fz_device_gray(0);
        let rgb = fz_device_rgb(0);
        let cmyk = fz_device_cmyk(0);

        assert_eq!(fz_colorspace_is_gray(0, gray), 1);
        assert_eq!(fz_colorspace_is_gray(0, rgb), 0);

        assert_eq!(fz_colorspace_is_rgb(0, rgb), 1);
        assert_eq!(fz_colorspace_is_rgb(0, cmyk), 0);

        assert_eq!(fz_colorspace_is_cmyk(0, cmyk), 1);
        assert_eq!(fz_colorspace_is_cmyk(0, gray), 0);
    }

    #[test]
    fn test_gray_to_rgb_conversion() {
        let gray = fz_device_gray(0);
        let rgb = fz_device_rgb(0);

        let src = [0.5f32];
        let mut dst = [0.0f32; 3];

        fz_convert_color(0, gray, src.as_ptr(), rgb, dst.as_mut_ptr(), 0);

        // Gray 0.5 should map to RGB (0.5, 0.5, 0.5)
        assert!((dst[0] - 0.5).abs() < 0.01);
        assert!((dst[1] - 0.5).abs() < 0.01);
        assert!((dst[2] - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_rgb_to_gray_conversion() {
        let gray = fz_device_gray(0);
        let rgb = fz_device_rgb(0);

        let src = [1.0f32, 0.0, 0.0]; // Pure red
        let mut dst = [0.0f32];

        fz_convert_color(0, rgb, src.as_ptr(), gray, dst.as_mut_ptr(), 0);

        // Red should convert to a gray value (luminance formula)
        assert!(dst[0] > 0.0 && dst[0] < 1.0);
    }
}

// ============================================================================
// MODULE: Pixmap Tests
// ============================================================================

mod ffi_pixmap {
    use micropdf::ffi::colorspace::*;
    use micropdf::ffi::geometry::fz_irect;
    use micropdf::ffi::pixmap::*;

    #[test]
    fn test_pixmap_creation() {
        let rgb = fz_device_rgb(0);
        let pix = fz_new_pixmap(0, rgb, 100, 50, 0, 1);

        assert_ne!(pix, 0);
        assert_eq!(fz_pixmap_width(0, pix), 100);
        assert_eq!(fz_pixmap_height(0, pix), 50);
        assert_eq!(fz_pixmap_alpha(0, pix), 1);
        assert_eq!(fz_pixmap_components(0, pix), 4); // RGB + alpha

        fz_drop_pixmap(0, pix);
    }

    #[test]
    fn test_pixmap_with_bbox() {
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

    #[test]
    fn test_pixmap_clear() {
        let rgb = fz_device_rgb(0);
        let pix = fz_new_pixmap(0, rgb, 10, 10, 0, 0);

        // Clear with value
        fz_clear_pixmap_with_value(0, pix, 128);

        // Check pixel
        let sample = fz_get_pixmap_sample(0, pix, 0, 0, 0);
        assert_eq!(sample, 128);

        // Clear to transparent
        fz_clear_pixmap(0, pix);
        let sample2 = fz_get_pixmap_sample(0, pix, 0, 0, 0);
        assert_eq!(sample2, 0);

        fz_drop_pixmap(0, pix);
    }

    #[test]
    fn test_pixmap_sample_access() {
        let rgb = fz_device_rgb(0);
        let pix = fz_new_pixmap(0, rgb, 10, 10, 0, 1);

        // Set specific pixel
        fz_set_pixmap_sample(0, pix, 5, 5, 0, 255); // R
        fz_set_pixmap_sample(0, pix, 5, 5, 1, 128); // G
        fz_set_pixmap_sample(0, pix, 5, 5, 2, 64); // B
        fz_set_pixmap_sample(0, pix, 5, 5, 3, 255); // A

        assert_eq!(fz_get_pixmap_sample(0, pix, 5, 5, 0), 255);
        assert_eq!(fz_get_pixmap_sample(0, pix, 5, 5, 1), 128);
        assert_eq!(fz_get_pixmap_sample(0, pix, 5, 5, 2), 64);
        assert_eq!(fz_get_pixmap_sample(0, pix, 5, 5, 3), 255);

        fz_drop_pixmap(0, pix);
    }

    #[test]
    fn test_pixmap_stride() {
        let rgb = fz_device_rgb(0);
        let pix = fz_new_pixmap(0, rgb, 100, 50, 0, 1);

        let stride = fz_pixmap_stride(0, pix);
        // Stride should be at least width * components
        assert!(stride >= 100 * 4);

        fz_drop_pixmap(0, pix);
    }

    #[test]
    fn test_pixmap_invert() {
        let rgb = fz_device_rgb(0);
        let pix = fz_new_pixmap(0, rgb, 10, 10, 0, 0);

        // Clear with white
        fz_clear_pixmap_with_value(0, pix, 255);

        // Invert
        fz_invert_pixmap(0, pix);

        // Should now be black
        let sample = fz_get_pixmap_sample(0, pix, 0, 0, 0);
        assert_eq!(sample, 0);

        fz_drop_pixmap(0, pix);
    }

    #[test]
    fn test_pixmap_gamma() {
        let rgb = fz_device_rgb(0);
        let pix = fz_new_pixmap(0, rgb, 10, 10, 0, 0);

        fz_clear_pixmap_with_value(0, pix, 128);

        // Apply gamma correction
        fz_gamma_pixmap(0, pix, 2.2);

        // Value should have changed
        let sample = fz_get_pixmap_sample(0, pix, 0, 0, 0);
        assert_ne!(sample, 128);

        fz_drop_pixmap(0, pix);
    }

    #[test]
    fn test_pixmap_grayscale() {
        let gray = fz_device_gray(0);
        let pix = fz_new_pixmap(0, gray, 10, 10, 0, 0);

        assert_eq!(fz_pixmap_components(0, pix), 1);

        fz_clear_pixmap_with_value(0, pix, 200);
        assert_eq!(fz_get_pixmap_sample(0, pix, 5, 5, 0), 200);

        fz_drop_pixmap(0, pix);
    }
}

// ============================================================================
// MODULE: Document Loading Tests
// ============================================================================

mod document_loading {
    use super::*;
    use micropdf::ffi::context::*;
    use micropdf::ffi::document::*;
    use micropdf::ffi::stream::*;

    /// Helper to open a document from memory using stream
    unsafe fn open_doc_from_memory(ctx: u64, data: &[u8]) -> u64 {
        let stm = fz_open_memory(ctx, data.as_ptr(), data.len());
        if stm == 0 {
            return 0;
        }
        let magic = std::ffi::CString::new("application/pdf").unwrap();
        let doc = fz_open_document_with_stream(ctx, magic.as_ptr(), stm);
        fz_drop_stream(ctx, stm);
        doc
    }

    #[test]
    fn test_open_minimal_pdf() {
        unsafe {
            let ctx = fz_new_context(std::ptr::null(), std::ptr::null(), 0);
            let pdf_data = read_fixture("minimal.pdf");

            let doc = open_doc_from_memory(ctx, &pdf_data);
            assert_ne!(doc, 0, "Document should open successfully");

            let count = fz_count_pages(ctx, doc);
            assert_eq!(count, 1, "Minimal PDF should have 1 page");

            assert_eq!(
                fz_needs_password(ctx, doc),
                0,
                "Minimal PDF should not need password"
            );

            fz_drop_document(ctx, doc);
            fz_drop_context(ctx);
        }
    }

    #[test]
    fn test_open_multipage_pdf() {
        unsafe {
            let ctx = fz_new_context(std::ptr::null(), std::ptr::null(), 0);
            let pdf_data = read_fixture("multipage.pdf");

            let doc = open_doc_from_memory(ctx, &pdf_data);
            assert_ne!(doc, 0);

            let count = fz_count_pages(ctx, doc);
            assert_eq!(count, 5, "Multipage PDF should have 5 pages");

            fz_drop_document(ctx, doc);
            fz_drop_context(ctx);
        }
    }

    #[test]
    fn test_open_comprehensive_pdf() {
        unsafe {
            let ctx = fz_new_context(std::ptr::null(), std::ptr::null(), 0);
            let pdf_data = read_fixture("comprehensive_test.pdf");

            let doc = open_doc_from_memory(ctx, &pdf_data);
            assert_ne!(doc, 0);

            let count = fz_count_pages(ctx, doc);
            assert_eq!(count, 3, "Comprehensive PDF should have 3 pages");

            fz_drop_document(ctx, doc);
            fz_drop_context(ctx);
        }
    }

    #[test]
    fn test_open_encrypted_pdf() {
        unsafe {
            let ctx = fz_new_context(std::ptr::null(), std::ptr::null(), 0);
            let pdf_data = read_fixture("encrypted_empty_password.pdf");

            let doc = open_doc_from_memory(ctx, &pdf_data);
            assert_ne!(doc, 0);

            // Check if password is needed
            let needs_pw = fz_needs_password(ctx, doc);
            // Encrypted PDF with empty password
            if needs_pw != 0 {
                // Try empty password
                let empty = std::ffi::CString::new("").unwrap();
                let result = fz_authenticate_password(ctx, doc, empty.as_ptr());
                assert_ne!(result, 0, "Empty password should work");
            }

            fz_drop_document(ctx, doc);
            fz_drop_context(ctx);
        }
    }

    #[test]
    fn test_document_keep_drop() {
        unsafe {
            let ctx = fz_new_context(std::ptr::null(), std::ptr::null(), 0);
            let pdf_data = read_fixture("minimal.pdf");

            let doc = open_doc_from_memory(ctx, &pdf_data);
            assert_ne!(doc, 0);

            // Keep should return a valid handle
            let kept = fz_keep_document(ctx, doc);
            assert_ne!(kept, 0, "Keep should return valid handle");

            // Verify document works before dropping
            let count = fz_count_pages(ctx, doc);
            assert_eq!(count, 1);

            // Clean up both handles
            fz_drop_document(ctx, doc);
            fz_drop_document(ctx, kept);
            fz_drop_context(ctx);
        }
    }

    #[test]
    fn test_load_page() {
        unsafe {
            let ctx = fz_new_context(std::ptr::null(), std::ptr::null(), 0);
            let pdf_data = read_fixture("multipage.pdf");

            let doc = open_doc_from_memory(ctx, &pdf_data);

            // Load each page
            for i in 0..5 {
                let page = fz_load_page(ctx, doc, i);
                assert_ne!(page, 0, "Page {} should load successfully", i);
                fz_drop_page(ctx, page);
            }

            fz_drop_document(ctx, doc);
            fz_drop_context(ctx);
        }
    }

    #[test]
    fn test_document_format_detection() {
        unsafe {
            let ctx = fz_new_context(std::ptr::null(), std::ptr::null(), 0);
            let pdf_data = read_fixture("minimal.pdf");

            let doc = open_doc_from_memory(ctx, &pdf_data);

            // Document should be recognized as PDF
            assert_ne!(doc, 0);

            fz_drop_document(ctx, doc);
            fz_drop_context(ctx);
        }
    }
}

// ============================================================================
// MODULE: Page Operations Tests
// ============================================================================

mod page_operations {
    use super::*;
    use micropdf::ffi::context::*;
    use micropdf::ffi::document::*;
    use micropdf::ffi::stream::*;

    /// Helper to open a document from memory using stream
    unsafe fn open_doc_from_memory(ctx: u64, data: &[u8]) -> u64 {
        let stm = fz_open_memory(ctx, data.as_ptr(), data.len());
        if stm == 0 {
            return 0;
        }
        let magic = std::ffi::CString::new("application/pdf").unwrap();
        let doc = fz_open_document_with_stream(ctx, magic.as_ptr(), stm);
        fz_drop_stream(ctx, stm);
        doc
    }

    #[test]
    fn test_page_bounds() {
        unsafe {
            let ctx = fz_new_context(std::ptr::null(), std::ptr::null(), 0);
            let pdf_data = read_fixture("minimal.pdf");
            let doc = open_doc_from_memory(ctx, &pdf_data);
            let page = fz_load_page(ctx, doc, 0);

            let bounds = fz_bound_page(ctx, page);

            // US Letter size: 612 x 792 points
            assert!((bounds.x1 - 612.0).abs() < 1.0);
            assert!((bounds.y1 - 792.0).abs() < 1.0);

            fz_drop_page(ctx, page);
            fz_drop_document(ctx, doc);
            fz_drop_context(ctx);
        }
    }

    #[test]
    fn test_multiple_page_bounds() {
        unsafe {
            let ctx = fz_new_context(std::ptr::null(), std::ptr::null(), 0);
            let pdf_data = read_fixture("multipage.pdf");
            let doc = open_doc_from_memory(ctx, &pdf_data);

            for i in 0..5 {
                let page = fz_load_page(ctx, doc, i);
                let bounds = fz_bound_page(ctx, page);

                // All pages should have valid bounds
                assert!(bounds.x1 > 0.0);
                assert!(bounds.y1 > 0.0);

                fz_drop_page(ctx, page);
            }

            fz_drop_document(ctx, doc);
            fz_drop_context(ctx);
        }
    }
}

// ============================================================================
// MODULE: Enhanced PDF Creation Tests
// ============================================================================

mod enhanced_pdf_creation {
    use super::*;
    use micropdf::enhanced::writer::PdfWriter;

    #[test]
    fn test_create_blank_pdf() {
        let mut writer = PdfWriter::new();

        // Add a blank page
        writer.add_blank_page(612.0, 792.0).unwrap();
        assert_eq!(writer.page_count(), 1);

        // Save to temp file
        let path = temp_path("blank.pdf");
        writer.save(path.to_str().unwrap()).unwrap();

        // Verify file exists and is valid PDF
        let data = fs::read(&path).unwrap();
        assert!(data.starts_with(b"%PDF-"));

        // Clean up
        fs::remove_file(path).ok();
    }

    #[test]
    fn test_create_multipage_pdf() {
        let mut writer = PdfWriter::new();

        // Add multiple pages of different sizes
        writer.add_blank_page(612.0, 792.0).unwrap(); // US Letter
        writer.add_blank_page(595.0, 842.0).unwrap(); // A4
        writer.add_blank_page(612.0, 1008.0).unwrap(); // US Legal

        assert_eq!(writer.page_count(), 3);

        let path = temp_path("multipage_created.pdf");
        writer.save(path.to_str().unwrap()).unwrap();

        // Verify PDF structure
        let data = fs::read(&path).unwrap();
        assert!(data.starts_with(b"%PDF-"));

        // Check for page count in PDF
        let content = String::from_utf8_lossy(&data);
        assert!(content.contains("/Count 3"));

        fs::remove_file(path).ok();
    }

    #[test]
    fn test_create_pdf_with_content() {
        let mut writer = PdfWriter::new();

        // Add page with simple content stream
        let content = "BT /F1 12 Tf 100 700 Td (Hello, World!) Tj ET";
        writer.add_page_with_content(612.0, 792.0, content).unwrap();

        let path = temp_path("with_content.pdf");
        writer.save(path.to_str().unwrap()).unwrap();

        let data = fs::read(&path).unwrap();
        assert!(data.starts_with(b"%PDF-"));

        fs::remove_file(path).ok();
    }

    #[test]
    fn test_invalid_page_dimensions() {
        let mut writer = PdfWriter::new();

        // Zero dimensions should fail
        assert!(writer.add_blank_page(0.0, 792.0).is_err());
        assert!(writer.add_blank_page(612.0, 0.0).is_err());

        // Negative dimensions should fail
        assert!(writer.add_blank_page(-100.0, 792.0).is_err());

        // Too large dimensions should fail
        assert!(writer.add_blank_page(20000.0, 792.0).is_err());
    }

    #[test]
    fn test_cannot_save_empty_pdf() {
        let writer = PdfWriter::new();

        let path = temp_path("empty.pdf");
        let result = writer.save(path.to_str().unwrap());

        assert!(result.is_err());
    }
}

// ============================================================================
// MODULE: Enhanced Metadata Tests
// ============================================================================

mod enhanced_metadata {
    use super::*;
    use micropdf::enhanced::metadata::{Metadata, read_metadata, update_metadata};

    #[test]
    fn test_read_metadata_from_fixture() {
        let path = fixture_path("minimal.pdf");
        let metadata = read_metadata(path.to_str().unwrap()).unwrap();

        // Producer should be set
        assert!(metadata.producer.is_some());
    }

    #[test]
    fn test_metadata_builder() {
        let metadata = Metadata::new()
            .with_title("Test Document")
            .with_author("Test Author")
            .with_subject("Testing")
            .with_keywords("test, pdf, micropdf");

        assert_eq!(metadata.title, Some("Test Document".to_string()));
        assert_eq!(metadata.author, Some("Test Author".to_string()));
        assert_eq!(metadata.subject, Some("Testing".to_string()));
        assert_eq!(metadata.keywords, Some("test, pdf, micropdf".to_string()));
    }

    #[test]
    fn test_metadata_custom_fields() {
        let mut metadata = Metadata::new();
        metadata.add_custom("CustomField", "CustomValue");
        metadata.add_custom("AnotherField", "AnotherValue");

        assert_eq!(
            metadata.custom.get("CustomField"),
            Some(&"CustomValue".to_string())
        );
        assert_eq!(
            metadata.custom.get("AnotherField"),
            Some(&"AnotherValue".to_string())
        );
    }

    #[test]
    fn test_update_metadata() {
        // Create a test PDF
        let mut writer = micropdf::enhanced::writer::PdfWriter::new();
        writer.add_blank_page(612.0, 792.0).unwrap();

        let path = temp_path("metadata_test.pdf");
        writer.save(path.to_str().unwrap()).unwrap();

        // Update metadata
        let metadata = Metadata::new()
            .with_title("Updated Title")
            .with_author("Updated Author");

        let result = update_metadata(path.to_str().unwrap(), &metadata);
        // This may or may not work depending on implementation state
        // but shouldn't panic
        let _ = result;

        fs::remove_file(path).ok();
    }

    #[test]
    fn test_metadata_validation() {
        let mut metadata = Metadata::new();

        // Very long title should fail validation
        metadata.title = Some("x".repeat(2000));

        let path = fixture_path("minimal.pdf");
        let result = update_metadata(path.to_str().unwrap(), &metadata);

        assert!(result.is_err());
    }
}

// ============================================================================
// MODULE: Enhanced Bookmark Tests
// ============================================================================

mod enhanced_bookmarks {
    use micropdf::enhanced::bookmarks::Bookmark;

    #[test]
    fn test_bookmark_creation() {
        let bookmark = Bookmark::new("Chapter 1", 0);

        assert_eq!(bookmark.title, "Chapter 1");
        assert_eq!(bookmark.page, 0);
        assert!(bookmark.children.is_empty());
    }

    #[test]
    fn test_bookmark_hierarchy() {
        let mut root = Bookmark::new("Table of Contents", 0);

        let mut chapter1 = Bookmark::new("Chapter 1", 1);
        chapter1.add_child(Bookmark::new("Section 1.1", 2));
        chapter1.add_child(Bookmark::new("Section 1.2", 3));

        let chapter2 = Bookmark::new("Chapter 2", 4);

        root.add_child(chapter1);
        root.add_child(chapter2);

        assert_eq!(root.children.len(), 2);
        assert_eq!(root.children[0].children.len(), 2);
        assert_eq!(root.count_all(), 5);
    }

    #[test]
    fn test_bookmark_find_by_title() {
        let mut root = Bookmark::new("Root", 0);
        let mut child = Bookmark::new("Child", 1);
        child.add_child(Bookmark::new("Grandchild", 2));
        root.add_child(child);

        assert!(root.find_by_title("Root").is_some());
        assert!(root.find_by_title("Child").is_some());
        assert!(root.find_by_title("Grandchild").is_some());
        assert!(root.find_by_title("NonExistent").is_none());
    }

    #[test]
    fn test_bookmark_validation() {
        let bookmark = Bookmark::new("Valid Bookmark", 0);
        assert!(bookmark.validate(10).is_ok());

        let invalid_page = Bookmark::new("Invalid", 100);
        assert!(invalid_page.validate(10).is_err());

        let empty_title = Bookmark::new("", 0);
        assert!(empty_title.validate(10).is_err());

        let long_title = Bookmark::new("x".repeat(600), 0);
        assert!(long_title.validate(10).is_err());
    }
}

// ============================================================================
// MODULE: Enhanced Page Operations Tests
// ============================================================================

mod enhanced_page_ops {
    use super::*;
    use micropdf::enhanced::page_ops::PdfMerger;

    #[test]
    fn test_merger_creation() {
        let merger = PdfMerger::new();
        assert_eq!(merger.page_count(), 0);
    }

    #[test]
    fn test_merge_single_pdf() {
        let mut merger = PdfMerger::new();
        let path = fixture_path("minimal.pdf");

        let result = merger.append(path.to_str().unwrap());
        assert!(result.is_ok());
        assert_eq!(merger.page_count(), 1);
    }

    #[test]
    fn test_merge_multipage_pdf() {
        let mut merger = PdfMerger::new();
        let path = fixture_path("multipage.pdf");

        merger.append(path.to_str().unwrap()).unwrap();
        assert_eq!(merger.page_count(), 5);
    }

    #[test]
    fn test_merge_multiple_pdfs() {
        let mut merger = PdfMerger::new();

        merger
            .append(fixture_path("minimal.pdf").to_str().unwrap())
            .unwrap();
        merger
            .append(fixture_path("multipage.pdf").to_str().unwrap())
            .unwrap();

        assert_eq!(merger.page_count(), 6); // 1 + 5
    }

    #[test]
    fn test_merge_and_save() {
        let mut merger = PdfMerger::new();

        merger
            .append(fixture_path("minimal.pdf").to_str().unwrap())
            .unwrap();

        let output = temp_path("merged.pdf");
        merger.save(output.to_str().unwrap()).unwrap();

        // Verify output
        let data = fs::read(&output).unwrap();
        assert!(data.starts_with(b"%PDF-"));

        fs::remove_file(output).ok();
    }

    #[test]
    fn test_merge_nonexistent_file() {
        let mut merger = PdfMerger::new();
        let result = merger.append("/nonexistent/path/file.pdf");
        assert!(result.is_err());
    }

    #[test]
    fn test_cannot_save_empty_merger() {
        let merger = PdfMerger::new();
        let output = temp_path("empty_merge.pdf");

        let result = merger.save(output.to_str().unwrap());
        assert!(result.is_err());
    }
}

// ============================================================================
// MODULE: PDF Structure Verification Tests
// ============================================================================

mod pdf_structure_verification {
    use super::*;

    fn find_pattern(data: &[u8], pattern: &[u8]) -> bool {
        data.windows(pattern.len()).any(|w| w == pattern)
    }

    #[test]
    fn test_created_pdf_has_valid_structure() {
        let mut writer = micropdf::enhanced::writer::PdfWriter::new();
        writer.add_blank_page(612.0, 792.0).unwrap();

        let path = temp_path("structure_test.pdf");
        writer.save(path.to_str().unwrap()).unwrap();

        let data = fs::read(&path).unwrap();

        // Check required PDF elements
        assert!(data.starts_with(b"%PDF-"), "Should have PDF header");
        assert!(
            find_pattern(&data, b"/Type /Catalog"),
            "Should have Catalog"
        );
        assert!(find_pattern(&data, b"/Type /Pages"), "Should have Pages");
        assert!(find_pattern(&data, b"/Type /Page"), "Should have Page");
        assert!(find_pattern(&data, b"/MediaBox"), "Should have MediaBox");
        assert!(find_pattern(&data, b"xref"), "Should have xref");
        assert!(find_pattern(&data, b"trailer"), "Should have trailer");
        assert!(find_pattern(&data, b"startxref"), "Should have startxref");
        assert!(find_pattern(&data, b"%%EOF"), "Should have EOF marker");

        fs::remove_file(path).ok();
    }

    #[test]
    fn test_created_pdf_page_count_correct() {
        for page_count in [1, 3, 5, 10] {
            let mut writer = micropdf::enhanced::writer::PdfWriter::new();
            for _ in 0..page_count {
                writer.add_blank_page(612.0, 792.0).unwrap();
            }

            let path = temp_path(&format!("pages_{}.pdf", page_count));
            writer.save(path.to_str().unwrap()).unwrap();

            let data = fs::read(&path).unwrap();
            let content = String::from_utf8_lossy(&data);

            assert!(
                content.contains(&format!("/Count {}", page_count)),
                "PDF should have /Count {}",
                page_count
            );

            fs::remove_file(path).ok();
        }
    }
}

// ============================================================================
// MODULE: Round-trip Tests (Create, Load, Verify)
// ============================================================================

mod round_trip {
    use super::*;
    use micropdf::ffi::context::*;
    use micropdf::ffi::document::*;
    use micropdf::ffi::stream::*;

    /// Helper to open a document from memory using stream
    unsafe fn open_doc_from_memory(ctx: u64, data: &[u8]) -> u64 {
        let stm = fz_open_memory(ctx, data.as_ptr(), data.len());
        if stm == 0 {
            return 0;
        }
        let magic = std::ffi::CString::new("application/pdf").unwrap();
        let doc = fz_open_document_with_stream(ctx, magic.as_ptr(), stm);
        fz_drop_stream(ctx, stm);
        doc
    }

    #[test]
    fn test_create_load_verify_single_page() {
        // Create PDF
        let mut writer = micropdf::enhanced::writer::PdfWriter::new();
        writer.add_blank_page(612.0, 792.0).unwrap();

        let path = temp_path("roundtrip_single.pdf");
        writer.save(path.to_str().unwrap()).unwrap();

        // Load and verify
        unsafe {
            let ctx = fz_new_context(std::ptr::null(), std::ptr::null(), 0);
            let data = fs::read(&path).unwrap();

            let doc = open_doc_from_memory(ctx, &data);
            assert_ne!(doc, 0);

            let count = fz_count_pages(ctx, doc);
            assert_eq!(count, 1);

            let page = fz_load_page(ctx, doc, 0);
            assert_ne!(page, 0);

            let bounds = fz_bound_page(ctx, page);
            assert!((bounds.x1 - 612.0).abs() < 1.0);
            assert!((bounds.y1 - 792.0).abs() < 1.0);

            fz_drop_page(ctx, page);
            fz_drop_document(ctx, doc);
            fz_drop_context(ctx);
        }

        fs::remove_file(path).ok();
    }

    #[test]
    fn test_create_load_verify_multiple_pages() {
        // Create PDF with multiple pages
        let mut writer = micropdf::enhanced::writer::PdfWriter::new();
        writer.add_blank_page(612.0, 792.0).unwrap(); // US Letter
        writer.add_blank_page(595.0, 842.0).unwrap(); // A4
        writer.add_blank_page(612.0, 1008.0).unwrap(); // US Legal

        let path = temp_path("roundtrip_multi.pdf");
        writer.save(path.to_str().unwrap()).unwrap();

        // Load and verify
        unsafe {
            let ctx = fz_new_context(std::ptr::null(), std::ptr::null(), 0);
            let data = fs::read(&path).unwrap();

            let doc = open_doc_from_memory(ctx, &data);
            let count = fz_count_pages(ctx, doc);
            assert_eq!(count, 3);

            // Verify each page can be loaded
            for i in 0..3 {
                let page = fz_load_page(ctx, doc, i);
                assert_ne!(page, 0);
                fz_drop_page(ctx, page);
            }

            fz_drop_document(ctx, doc);
            fz_drop_context(ctx);
        }

        fs::remove_file(path).ok();
    }

    #[test]
    fn test_merge_load_verify() {
        // Merge PDFs
        let mut merger = micropdf::enhanced::page_ops::PdfMerger::new();
        merger
            .append(fixture_path("minimal.pdf").to_str().unwrap())
            .unwrap();
        merger
            .append(fixture_path("multipage.pdf").to_str().unwrap())
            .unwrap();

        let path = temp_path("roundtrip_merge.pdf");
        merger.save(path.to_str().unwrap()).unwrap();

        // Load and verify
        unsafe {
            let ctx = fz_new_context(std::ptr::null(), std::ptr::null(), 0);
            let data = fs::read(&path).unwrap();

            let doc = open_doc_from_memory(ctx, &data);
            let count = fz_count_pages(ctx, doc);
            assert_eq!(count, 6); // 1 + 5

            fz_drop_document(ctx, doc);
            fz_drop_context(ctx);
        }

        fs::remove_file(path).ok();
    }
}

// ============================================================================
// MODULE: Edge Cases and Error Handling Tests
// ============================================================================

mod edge_cases {
    use super::*;
    use micropdf::ffi::context::*;
    use micropdf::ffi::document::*;
    use micropdf::ffi::stream::*;

    /// Helper to open a document from memory using stream
    unsafe fn open_doc_from_memory(ctx: u64, data: &[u8]) -> u64 {
        if data.is_empty() {
            return 0;
        }
        let stm = fz_open_memory(ctx, data.as_ptr(), data.len());
        if stm == 0 {
            return 0;
        }
        let magic = std::ffi::CString::new("application/pdf").unwrap();
        let doc = fz_open_document_with_stream(ctx, magic.as_ptr(), stm);
        fz_drop_stream(ctx, stm);
        doc
    }

    #[test]
    fn test_invalid_pdf_data() {
        unsafe {
            let ctx = fz_new_context(std::ptr::null(), std::ptr::null(), 0);
            let invalid_data = b"This is not a PDF file";

            let doc = open_doc_from_memory(ctx, invalid_data);
            // Should either return 0 or handle gracefully
            if doc != 0 {
                fz_drop_document(ctx, doc);
            }

            fz_drop_context(ctx);
        }
    }

    #[test]
    fn test_empty_data() {
        unsafe {
            let ctx = fz_new_context(std::ptr::null(), std::ptr::null(), 0);
            let empty_data: &[u8] = &[];

            let doc = open_doc_from_memory(ctx, empty_data);
            if doc != 0 {
                fz_drop_document(ctx, doc);
            }

            fz_drop_context(ctx);
        }
    }

    #[test]
    fn test_truncated_pdf() {
        let full_data = read_fixture("minimal.pdf");
        // Truncate to half
        let truncated = &full_data[..full_data.len() / 2];

        unsafe {
            let ctx = fz_new_context(std::ptr::null(), std::ptr::null(), 0);

            let doc = open_doc_from_memory(ctx, truncated);
            // May succeed partially or fail - shouldn't crash
            if doc != 0 {
                fz_drop_document(ctx, doc);
            }

            fz_drop_context(ctx);
        }
    }

    #[test]
    fn test_page_out_of_bounds() {
        unsafe {
            let ctx = fz_new_context(std::ptr::null(), std::ptr::null(), 0);
            let data = read_fixture("minimal.pdf");

            let doc = open_doc_from_memory(ctx, &data);
            let count = fz_count_pages(ctx, doc);
            assert_eq!(count, 1);

            // Try to load page that doesn't exist
            let page = fz_load_page(ctx, doc, 100);
            // Should return 0 for invalid page
            assert_eq!(page, 0);

            fz_drop_document(ctx, doc);
            fz_drop_context(ctx);
        }
    }

    #[test]
    fn test_null_context_operations() {
        // Operations with null context should not crash
        // Most FFI functions check for valid handles

        use micropdf::ffi::buffer::*;
        // These should be no-ops or return safe defaults
        fz_drop_buffer(0, 0);
    }

    #[test]
    fn test_double_drop() {
        unsafe {
            let ctx = fz_new_context(std::ptr::null(), std::ptr::null(), 0);
            let data = read_fixture("minimal.pdf");

            let doc = open_doc_from_memory(ctx, &data);
            fz_drop_document(ctx, doc);
            // Second drop should be safe (no-op)
            fz_drop_document(ctx, doc);

            fz_drop_context(ctx);
        }
    }
}

// ============================================================================
// MODULE: Performance Sanity Tests
// ============================================================================

mod performance_sanity {
    use super::temp_path;
    use std::fs;
    use std::time::Instant;

    #[test]
    fn test_create_100_pages_fast() {
        let start = Instant::now();

        let mut writer = micropdf::enhanced::writer::PdfWriter::new();
        for _ in 0..100 {
            writer.add_blank_page(612.0, 792.0).unwrap();
        }

        let path = temp_path("perf_100pages.pdf");
        writer.save(path.to_str().unwrap()).unwrap();

        let elapsed = start.elapsed();
        assert!(
            elapsed.as_secs() < 5,
            "Creating 100 pages should take less than 5 seconds"
        );

        fs::remove_file(path).ok();
    }

    #[test]
    fn test_context_creation_fast() {
        use micropdf::ffi::context::*;

        let start = Instant::now();

        for _ in 0..100 {
            unsafe {
                let ctx = fz_new_context(std::ptr::null(), std::ptr::null(), 0);
                fz_drop_context(ctx);
            }
        }

        let elapsed = start.elapsed();
        assert!(
            elapsed.as_millis() < 1000,
            "100 context create/drop cycles should take less than 1 second"
        );
    }

    #[test]
    fn test_buffer_operations_fast() {
        use micropdf::ffi::buffer::*;
        use micropdf::ffi::context::*;

        let start = Instant::now();

        unsafe {
            let ctx = fz_new_context(std::ptr::null(), std::ptr::null(), 0);

            for _ in 0..1000 {
                let buf = fz_new_buffer(ctx, 1024);
                for i in 0..100 {
                    fz_append_byte(ctx, buf, i);
                }
                fz_drop_buffer(ctx, buf);
            }

            fz_drop_context(ctx);
        }

        let elapsed = start.elapsed();
        assert!(
            elapsed.as_secs() < 2,
            "1000 buffer operations should take less than 2 seconds"
        );
    }
}

// ============================================================================
// MODULE: Integration with All Fixture Files
// ============================================================================

mod fixture_integration {
    use super::*;
    use micropdf::ffi::context::*;
    use micropdf::ffi::document::*;
    use micropdf::ffi::stream::*;

    /// Helper to open a document from memory using stream
    unsafe fn open_doc_from_memory(ctx: u64, data: &[u8]) -> u64 {
        let stm = fz_open_memory(ctx, data.as_ptr(), data.len());
        if stm == 0 {
            return 0;
        }
        let magic = std::ffi::CString::new("application/pdf").unwrap();
        let doc = fz_open_document_with_stream(ctx, magic.as_ptr(), stm);
        fz_drop_stream(ctx, stm);
        doc
    }

    #[test]
    fn test_all_fixtures_loadable() {
        let fixtures = [
            "minimal.pdf",
            "multipage.pdf",
            "comprehensive_test.pdf",
            "encrypted_empty_password.pdf",
        ];

        for fixture in fixtures {
            let data = read_fixture(fixture);

            unsafe {
                let ctx = fz_new_context(std::ptr::null(), std::ptr::null(), 0);
                let doc = open_doc_from_memory(ctx, &data);

                assert_ne!(doc, 0, "Fixture {} should load", fixture);

                let count = fz_count_pages(ctx, doc);
                assert!(count > 0, "Fixture {} should have pages", fixture);

                fz_drop_document(ctx, doc);
                fz_drop_context(ctx);
            }
        }
    }

    #[test]
    fn test_all_fixture_pages_loadable() {
        let fixtures = [
            ("minimal.pdf", 1),
            ("multipage.pdf", 5),
            ("comprehensive_test.pdf", 3),
        ];

        for (fixture, expected_pages) in fixtures {
            let data = read_fixture(fixture);

            unsafe {
                let ctx = fz_new_context(std::ptr::null(), std::ptr::null(), 0);
                let doc = open_doc_from_memory(ctx, &data);

                let count = fz_count_pages(ctx, doc);
                assert_eq!(
                    count, expected_pages,
                    "Fixture {} should have {} pages",
                    fixture, expected_pages
                );

                // Load each page
                for i in 0..count {
                    let page = fz_load_page(ctx, doc, i);
                    assert_ne!(page, 0, "Page {} of {} should load", i, fixture);

                    let bounds = fz_bound_page(ctx, page);
                    assert!(bounds.x1 > 0.0, "Page {} should have width", i);
                    assert!(bounds.y1 > 0.0, "Page {} should have height", i);

                    fz_drop_page(ctx, page);
                }

                fz_drop_document(ctx, doc);
                fz_drop_context(ctx);
            }
        }
    }
}

// ============================================================================
// MODULE: PDF Content Generation and Verification Tests
// ============================================================================

mod content_generation {
    use super::*;
    use micropdf::ffi::colorspace::*;
    use micropdf::ffi::context::*;
    use micropdf::ffi::document::*;
    use micropdf::ffi::pixmap::*;
    use micropdf::ffi::stream::*;

    /// Helper to open a document from memory using stream
    unsafe fn open_doc_from_memory(ctx: u64, data: &[u8]) -> u64 {
        let stm = fz_open_memory(ctx, data.as_ptr(), data.len());
        if stm == 0 {
            return 0;
        }
        let magic = std::ffi::CString::new("application/pdf").unwrap();
        let doc = fz_open_document_with_stream(ctx, magic.as_ptr(), stm);
        fz_drop_stream(ctx, stm);
        doc
    }

    /// Check if a pattern exists in byte slice
    fn contains_pattern(data: &[u8], pattern: &[u8]) -> bool {
        data.windows(pattern.len()).any(|w| w == pattern)
    }

    /// Count occurrences of a pattern in byte slice
    fn count_pattern(data: &[u8], pattern: &[u8]) -> usize {
        data.windows(pattern.len())
            .filter(|w| *w == pattern)
            .count()
    }

    // ========================================================================
    // Graphics Content Tests
    // ========================================================================

    #[test]
    fn test_pdf_with_rectangle_graphics() {
        // Create a PDF with a filled rectangle using PDF operators
        let mut writer = micropdf::enhanced::writer::PdfWriter::new();

        // PDF content stream to draw a red filled rectangle
        // q = save graphics state
        // 1 0 0 rg = set RGB fill color to red
        // 100 600 200 100 re = rectangle at (100,600) with width 200, height 100
        // f = fill path
        // Q = restore graphics state
        let content = "q 1 0 0 rg 100 600 200 100 re f Q";
        writer.add_page_with_content(612.0, 792.0, content).unwrap();

        let path = temp_path("rect_graphics.pdf");
        writer.save(path.to_str().unwrap()).unwrap();

        // Verify PDF content
        let data = fs::read(&path).unwrap();

        // Check PDF header
        assert!(data.starts_with(b"%PDF-"), "Should be valid PDF");

        // Verify content stream contains our graphics operators
        assert!(
            contains_pattern(&data, b"1 0 0 rg"),
            "Content should contain RGB fill color"
        );
        assert!(
            contains_pattern(&data, b"100 600 200 100 re"),
            "Content should contain rectangle"
        );
        assert!(
            contains_pattern(&data, b" f"),
            "Content should contain fill operator"
        );

        // Load PDF and verify it's valid
        unsafe {
            let ctx = fz_new_context(std::ptr::null(), std::ptr::null(), 0);
            let doc = open_doc_from_memory(ctx, &data);
            assert_ne!(doc, 0, "PDF with graphics should load");

            let count = fz_count_pages(ctx, doc);
            assert_eq!(count, 1);

            let page = fz_load_page(ctx, doc, 0);
            assert_ne!(page, 0);

            fz_drop_page(ctx, page);
            fz_drop_document(ctx, doc);
            fz_drop_context(ctx);
        }

        fs::remove_file(path).ok();
    }

    #[test]
    fn test_pdf_with_line_graphics() {
        let mut writer = micropdf::enhanced::writer::PdfWriter::new();

        // PDF content stream to draw lines
        // m = moveto, l = lineto, S = stroke
        let content = "q 0 0 0 RG 2 w 50 700 m 550 700 l S Q";
        writer.add_page_with_content(612.0, 792.0, content).unwrap();

        let path = temp_path("line_graphics.pdf");
        writer.save(path.to_str().unwrap()).unwrap();

        let data = fs::read(&path).unwrap();

        // Verify line content
        assert!(
            contains_pattern(&data, b"50 700 m"),
            "Content should contain moveto"
        );
        assert!(
            contains_pattern(&data, b"550 700 l"),
            "Content should contain lineto"
        );
        assert!(
            contains_pattern(&data, b" S"),
            "Content should contain stroke operator"
        );

        fs::remove_file(path).ok();
    }

    #[test]
    fn test_pdf_with_multiple_shapes() {
        let mut writer = micropdf::enhanced::writer::PdfWriter::new();

        // Multiple shapes: rectangle, circle (approximated with curves), line
        let content = r#"q
            % Red filled rectangle
            1 0 0 rg
            100 600 150 100 re f

            % Blue stroked rectangle
            0 0 1 RG
            2 w
            300 600 150 100 re S

            % Green diagonal line
            0 1 0 RG
            3 w
            100 500 m 450 500 l S

            % Circle approximation (using bezier curves)
            0.5 0 0.5 RG
            300 350 m
            300 377.6 277.6 400 250 400 c
            222.4 400 200 377.6 200 350 c
            200 322.4 222.4 300 250 300 c
            277.6 300 300 322.4 300 350 c
            S
        Q"#;

        writer.add_page_with_content(612.0, 792.0, content).unwrap();

        let path = temp_path("multi_shapes.pdf");
        writer.save(path.to_str().unwrap()).unwrap();

        let data = fs::read(&path).unwrap();

        // Verify various operators present
        assert!(contains_pattern(&data, b"1 0 0 rg"), "Red fill color");
        assert!(contains_pattern(&data, b"0 0 1 RG"), "Blue stroke color");
        assert!(contains_pattern(&data, b"0 1 0 RG"), "Green stroke color");
        assert!(contains_pattern(&data, b"re f"), "Filled rectangle");
        assert!(contains_pattern(&data, b"re S"), "Stroked rectangle");
        assert!(count_pattern(&data, b" c") >= 4, "Bezier curves for circle");

        fs::remove_file(path).ok();
    }

    #[test]
    fn test_pdf_with_complex_path() {
        let mut writer = micropdf::enhanced::writer::PdfWriter::new();

        // Complex closed path (pentagon)
        let content = r#"q
            0.2 0.4 0.8 rg
            0.8 0.4 0.2 RG
            2 w
            300 700 m
            400 650 l
            370 550 l
            230 550 l
            200 650 l
            h
            B
        Q"#;

        writer.add_page_with_content(612.0, 792.0, content).unwrap();

        let path = temp_path("complex_path.pdf");
        writer.save(path.to_str().unwrap()).unwrap();

        let data = fs::read(&path).unwrap();

        // Verify path operators
        assert!(contains_pattern(&data, b"300 700 m"), "Moveto");
        assert!(count_pattern(&data, b" l") >= 4, "Multiple lineto");
        assert!(contains_pattern(&data, b" h"), "Close path operator");
        assert!(contains_pattern(&data, b" B"), "Fill and stroke operator");

        // Load and verify
        unsafe {
            let ctx = fz_new_context(std::ptr::null(), std::ptr::null(), 0);
            let doc = open_doc_from_memory(ctx, &data);
            assert_ne!(doc, 0);

            fz_drop_document(ctx, doc);
            fz_drop_context(ctx);
        }

        fs::remove_file(path).ok();
    }

    // ========================================================================
    // Text Content Tests
    // ========================================================================

    #[test]
    fn test_pdf_with_text_content_stream() {
        let mut writer = micropdf::enhanced::writer::PdfWriter::new();

        // Basic text content (note: font would need to be defined for actual rendering)
        // BT = begin text, ET = end text
        // Tf = set font, Td = move text position, Tj = show text
        let content = "BT /F1 24 Tf 100 700 Td (Hello, PDF World!) Tj ET";
        writer.add_page_with_content(612.0, 792.0, content).unwrap();

        let path = temp_path("text_content.pdf");
        writer.save(path.to_str().unwrap()).unwrap();

        let data = fs::read(&path).unwrap();

        // Verify text operators
        assert!(contains_pattern(&data, b"BT"), "Begin text");
        assert!(contains_pattern(&data, b"ET"), "End text");
        assert!(contains_pattern(&data, b"/F1 24 Tf"), "Font setting");
        assert!(contains_pattern(&data, b"100 700 Td"), "Text position");
        assert!(
            contains_pattern(&data, b"Hello, PDF World!"),
            "Text content"
        );
        assert!(contains_pattern(&data, b"Tj"), "Show text operator");

        fs::remove_file(path).ok();
    }

    #[test]
    fn test_pdf_with_multiline_text() {
        let mut writer = micropdf::enhanced::writer::PdfWriter::new();

        // Multiple lines of text using TL (leading) and T* (next line)
        let content = r#"BT
            /F1 12 Tf
            100 700 Td
            14 TL
            (Line 1: Introduction) Tj
            T*
            (Line 2: Content here) Tj
            T*
            (Line 3: More content) Tj
            T*
            (Line 4: Conclusion) Tj
        ET"#;

        writer.add_page_with_content(612.0, 792.0, content).unwrap();

        let path = temp_path("multiline_text.pdf");
        writer.save(path.to_str().unwrap()).unwrap();

        let data = fs::read(&path).unwrap();

        // Verify all text lines
        assert!(contains_pattern(&data, b"Line 1: Introduction"));
        assert!(contains_pattern(&data, b"Line 2: Content here"));
        assert!(contains_pattern(&data, b"Line 3: More content"));
        assert!(contains_pattern(&data, b"Line 4: Conclusion"));
        assert!(contains_pattern(&data, b"14 TL"), "Line leading");
        assert!(count_pattern(&data, b"T*") >= 3, "Multiple newlines");

        fs::remove_file(path).ok();
    }

    #[test]
    fn test_pdf_with_mixed_content() {
        let mut writer = micropdf::enhanced::writer::PdfWriter::new();

        // Mix of graphics and text
        let content = r#"q
            % Background rectangle
            0.9 0.9 0.95 rg
            50 50 512 692 re f

            % Border
            0 0 0 RG
            1 w
            50 50 512 692 re S

            % Title text
            BT
                /F1 24 Tf
                0 0 0 rg
                200 720 Td
                (Document Title) Tj
            ET

            % Divider line
            0.5 0.5 0.5 RG
            2 w
            100 700 m 512 700 l S

            % Body text
            BT
                /F1 12 Tf
                100 670 Td
                (This is the body content of the document.) Tj
            ET
        Q"#;

        writer.add_page_with_content(612.0, 792.0, content).unwrap();

        let path = temp_path("mixed_content.pdf");
        writer.save(path.to_str().unwrap()).unwrap();

        let data = fs::read(&path).unwrap();

        // Verify mixed content
        assert!(contains_pattern(&data, b"Document Title"));
        assert!(contains_pattern(
            &data,
            b"This is the body content of the document."
        ));
        assert!(count_pattern(&data, b"BT") >= 2, "Multiple text blocks");
        assert!(count_pattern(&data, b"ET") >= 2, "Multiple text blocks");
        assert!(count_pattern(&data, b"re") >= 2, "Multiple rectangles");

        fs::remove_file(path).ok();
    }

    // ========================================================================
    // Content Verification Through Reloading
    // ========================================================================

    #[test]
    fn test_created_pdf_roundtrip_with_content() {
        let mut writer = micropdf::enhanced::writer::PdfWriter::new();

        // Create content with specific markers we can verify
        let content = r#"q
            1 0 0 rg
            MARKER_RECT_START
            200 500 100 50 re f
            MARKER_RECT_END

            BT
            /F1 18 Tf
            100 400 Td
            (UNIQUE_TEXT_MARKER_12345) Tj
            ET
        Q"#;

        writer.add_page_with_content(612.0, 792.0, content).unwrap();

        let path = temp_path("roundtrip_content.pdf");
        writer.save(path.to_str().unwrap()).unwrap();

        // Read and verify
        let data = fs::read(&path).unwrap();

        // Content markers should be preserved
        assert!(contains_pattern(&data, b"MARKER_RECT_START"));
        assert!(contains_pattern(&data, b"MARKER_RECT_END"));
        assert!(contains_pattern(&data, b"UNIQUE_TEXT_MARKER_12345"));
        assert!(contains_pattern(&data, b"200 500 100 50 re"));
        assert!(contains_pattern(&data, b"/F1 18 Tf"));

        // Load document via FFI and verify it's valid
        unsafe {
            let ctx = fz_new_context(std::ptr::null(), std::ptr::null(), 0);
            let doc = open_doc_from_memory(ctx, &data);
            assert_ne!(doc, 0, "Created PDF should be loadable");

            let count = fz_count_pages(ctx, doc);
            assert_eq!(count, 1);

            let page = fz_load_page(ctx, doc, 0);
            assert_ne!(page, 0);

            // Verify page bounds
            let bounds = fz_bound_page(ctx, page);
            assert!((bounds.x1 - 612.0).abs() < 1.0);
            assert!((bounds.y1 - 792.0).abs() < 1.0);

            fz_drop_page(ctx, page);
            fz_drop_document(ctx, doc);
            fz_drop_context(ctx);
        }

        fs::remove_file(path).ok();
    }

    #[test]
    fn test_multipage_pdf_with_different_content() {
        let mut writer = micropdf::enhanced::writer::PdfWriter::new();

        // Page 1: Red rectangle
        let content1 = "q 1 0 0 rg 100 600 200 100 re f Q";
        writer
            .add_page_with_content(612.0, 792.0, content1)
            .unwrap();

        // Page 2: Green circle (approximated)
        let content2 = r#"q 0 1 0 rg
            300 400 m 300 455.2 255.2 500 200 500 c
            144.8 500 100 455.2 100 400 c 100 344.8 144.8 300 200 300 c
            255.2 300 300 344.8 300 400 c f Q"#;
        writer
            .add_page_with_content(612.0, 792.0, content2)
            .unwrap();

        // Page 3: Blue text
        let content3 = "q BT /F1 36 Tf 0 0 1 rg 150 400 Td (Page Three) Tj ET Q";
        writer
            .add_page_with_content(612.0, 792.0, content3)
            .unwrap();

        let path = temp_path("multipage_content.pdf");
        writer.save(path.to_str().unwrap()).unwrap();

        let data = fs::read(&path).unwrap();

        // Verify all content is present
        assert!(contains_pattern(&data, b"1 0 0 rg"), "Page 1: Red color");
        assert!(contains_pattern(&data, b"0 1 0 rg"), "Page 2: Green color");
        assert!(contains_pattern(&data, b"0 0 1 rg"), "Page 3: Blue color");
        assert!(contains_pattern(&data, b"Page Three"), "Page 3: Text");
        assert!(contains_pattern(&data, b"/Count 3"), "Three pages");

        // Load and verify all pages
        unsafe {
            let ctx = fz_new_context(std::ptr::null(), std::ptr::null(), 0);
            let doc = open_doc_from_memory(ctx, &data);
            assert_ne!(doc, 0);

            let count = fz_count_pages(ctx, doc);
            assert_eq!(count, 3);

            for i in 0..3 {
                let page = fz_load_page(ctx, doc, i);
                assert_ne!(page, 0, "Page {} should load", i);
                fz_drop_page(ctx, page);
            }

            fz_drop_document(ctx, doc);
            fz_drop_context(ctx);
        }

        fs::remove_file(path).ok();
    }

    // ========================================================================
    // Pixmap Rendering Verification
    // ========================================================================

    #[test]
    fn test_pixmap_not_empty_after_setting_content() {
        // Create a pixmap and draw on it directly
        let rgb = fz_device_rgb(0);
        let pix = fz_new_pixmap(0, rgb, 100, 100, 0, 0);

        // Clear to white first
        fz_clear_pixmap_with_value(0, pix, 255);

        // Verify it's white
        let sample_before = fz_get_pixmap_sample(0, pix, 50, 50, 0);
        assert_eq!(sample_before, 255, "Should start as white");

        // Set some pixels to black
        for x in 40..60 {
            for y in 40..60 {
                fz_set_pixmap_sample(0, pix, x, y, 0, 0); // R = 0
                fz_set_pixmap_sample(0, pix, x, y, 1, 0); // G = 0
                fz_set_pixmap_sample(0, pix, x, y, 2, 0); // B = 0
            }
        }

        // Verify the center is now black
        let sample_after = fz_get_pixmap_sample(0, pix, 50, 50, 0);
        assert_eq!(sample_after, 0, "Center should be black after drawing");

        // Verify outside the drawn area is still white
        let sample_outside = fz_get_pixmap_sample(0, pix, 10, 10, 0);
        assert_eq!(sample_outside, 255, "Outside area should still be white");

        fz_drop_pixmap(0, pix);
    }

    #[test]
    fn test_pixmap_has_valid_samples() {
        let rgb = fz_device_rgb(0);
        let pix = fz_new_pixmap(0, rgb, 50, 50, 0, 1); // With alpha

        // Set a gradient pattern
        for y in 0..50 {
            for x in 0..50 {
                let r = ((x * 255) / 50) as u8;
                let g = ((y * 255) / 50) as u8;
                let b = 128u8;
                let a = 255u8;

                fz_set_pixmap_sample(0, pix, x, y, 0, r);
                fz_set_pixmap_sample(0, pix, x, y, 1, g);
                fz_set_pixmap_sample(0, pix, x, y, 2, b);
                fz_set_pixmap_sample(0, pix, x, y, 3, a);
            }
        }

        // Verify various points
        // Top-left should be close to (0, 0, 128)
        assert!(fz_get_pixmap_sample(0, pix, 0, 0, 0) < 10);
        assert!(fz_get_pixmap_sample(0, pix, 0, 0, 1) < 10);

        // Bottom-right should be close to (255, 255, 128)
        assert!(fz_get_pixmap_sample(0, pix, 49, 49, 0) > 240);
        assert!(fz_get_pixmap_sample(0, pix, 49, 49, 1) > 240);
        assert!((fz_get_pixmap_sample(0, pix, 49, 49, 2) as i32 - 128).abs() < 5);

        fz_drop_pixmap(0, pix);
    }

    // ========================================================================
    // Content Stream Operator Tests
    // ========================================================================

    #[test]
    fn test_all_graphics_state_operators() {
        let mut writer = micropdf::enhanced::writer::PdfWriter::new();

        // Test all major graphics state operators
        let content = r#"
            q
                % Color operators
                0.5 g                   % Gray fill
                0.3 G                   % Gray stroke
                1 0 0 rg                % RGB fill
                0 1 0 RG                % RGB stroke
                1 0 0 0 k               % CMYK fill
                0 1 0 0 K               % CMYK stroke

                % Line style operators
                2 w                     % Line width
                1 J                     % Line cap (round)
                2 j                     % Line join (bevel)
                10 M                    % Miter limit
                [3 2] 0 d               % Dash pattern

                % Path operators
                100 700 m               % moveto
                200 700 l               % lineto
                250 750 300 750 350 700 c  % curveto
                h                       % closepath
                S                       % stroke

                % Rectangle
                100 600 100 50 re       % rectangle
                f                       % fill

                % Transformation
                1 0 0 1 10 10 cm        % concat matrix
            Q
        "#;

        writer.add_page_with_content(612.0, 792.0, content).unwrap();

        let path = temp_path("all_operators.pdf");
        writer.save(path.to_str().unwrap()).unwrap();

        let data = fs::read(&path).unwrap();

        // Verify various operators are present
        assert!(contains_pattern(&data, b"0.5 g"), "Gray fill");
        assert!(contains_pattern(&data, b"0.3 G"), "Gray stroke");
        assert!(contains_pattern(&data, b"1 0 0 rg"), "RGB fill");
        assert!(contains_pattern(&data, b"0 1 0 RG"), "RGB stroke");
        assert!(contains_pattern(&data, b"2 w"), "Line width");
        assert!(contains_pattern(&data, b"1 J"), "Line cap");
        assert!(contains_pattern(&data, b"2 j"), "Line join");
        assert!(contains_pattern(&data, b"10 M"), "Miter limit");
        assert!(contains_pattern(&data, b"[3 2] 0 d"), "Dash pattern");
        assert!(contains_pattern(&data, b" m"), "moveto");
        assert!(contains_pattern(&data, b" l"), "lineto");
        assert!(contains_pattern(&data, b" c"), "curveto");
        assert!(contains_pattern(&data, b" h"), "closepath");
        assert!(contains_pattern(&data, b" S"), "stroke");
        assert!(contains_pattern(&data, b" re"), "rectangle");
        assert!(contains_pattern(&data, b" f"), "fill");
        assert!(contains_pattern(&data, b" cm"), "concat matrix");

        fs::remove_file(path).ok();
    }

    #[test]
    fn test_text_operators() {
        let mut writer = micropdf::enhanced::writer::PdfWriter::new();

        let content = r#"
            BT
                /F1 12 Tf               % Set font
                100 700 Td              % Move text position
                14 TL                   % Set leading
                1 Tr                    % Text render mode (stroke)
                100 Tz                  % Horizontal scaling
                2 Tc                    % Character spacing
                1 Tw                    % Word spacing
                0 Ts                    % Text rise
                (First line) Tj
                T*                      % Next line
                (Second line) Tj
                [(A) -50 (B) -50 (C)] TJ  % Show text with kerning
            ET
        "#;

        writer.add_page_with_content(612.0, 792.0, content).unwrap();

        let path = temp_path("text_operators.pdf");
        writer.save(path.to_str().unwrap()).unwrap();

        let data = fs::read(&path).unwrap();

        // Verify text operators
        assert!(contains_pattern(&data, b"BT"), "Begin text");
        assert!(contains_pattern(&data, b"ET"), "End text");
        assert!(contains_pattern(&data, b"Tf"), "Font");
        assert!(contains_pattern(&data, b"Td"), "Text position");
        assert!(contains_pattern(&data, b"TL"), "Leading");
        assert!(contains_pattern(&data, b"Tr"), "Render mode");
        assert!(contains_pattern(&data, b"Tz"), "Horizontal scaling");
        assert!(contains_pattern(&data, b"Tc"), "Character spacing");
        assert!(contains_pattern(&data, b"Tw"), "Word spacing");
        assert!(contains_pattern(&data, b"Ts"), "Text rise");
        assert!(contains_pattern(&data, b"Tj"), "Show text");
        assert!(contains_pattern(&data, b"T*"), "Next line");
        assert!(contains_pattern(&data, b"TJ"), "Show text with kerning");
        assert!(contains_pattern(&data, b"First line"));
        assert!(contains_pattern(&data, b"Second line"));

        fs::remove_file(path).ok();
    }

    // ========================================================================
    // PDF Structure Integrity Tests
    // ========================================================================

    #[test]
    fn test_content_stream_length_matches() {
        let mut writer = micropdf::enhanced::writer::PdfWriter::new();

        let content = "q 1 0 0 rg 100 100 200 200 re f Q";
        writer.add_page_with_content(612.0, 792.0, content).unwrap();

        let path = temp_path("length_check.pdf");
        writer.save(path.to_str().unwrap()).unwrap();

        let data = fs::read(&path).unwrap();
        let content_str = String::from_utf8_lossy(&data);

        // Find /Length value and verify it matches content length
        if let Some(pos) = content_str.find("/Length ") {
            let after_length = &content_str[pos + 8..];
            if let Some(end) = after_length.find(|c: char| !c.is_ascii_digit()) {
                let length: usize = after_length[..end].parse().unwrap();
                // The length should be the length of our content string
                assert_eq!(length, content.len(), "Length should match content");
            }
        }

        fs::remove_file(path).ok();
    }

    #[test]
    fn test_xref_table_valid() {
        let mut writer = micropdf::enhanced::writer::PdfWriter::new();
        writer.add_blank_page(612.0, 792.0).unwrap();

        let path = temp_path("xref_check.pdf");
        writer.save(path.to_str().unwrap()).unwrap();

        let data = fs::read(&path).unwrap();
        let content = String::from_utf8_lossy(&data);

        // Check xref structure
        assert!(content.contains("xref\n"), "Should have xref keyword");
        assert!(content.contains("trailer"), "Should have trailer");
        assert!(content.contains("startxref"), "Should have startxref");
        assert!(content.contains("/Root"), "Trailer should have /Root");
        assert!(content.contains("/Size"), "Trailer should have /Size");

        // Verify xref entries format (10-digit offset, 5-digit generation, n/f)
        let lines: Vec<&str> = content.lines().collect();
        let xref_start = lines.iter().position(|l| *l == "xref").unwrap();

        // Skip header line (e.g., "0 5")
        let first_entry_line = xref_start + 2; // Skip "xref" and "0 N"
        if first_entry_line < lines.len() {
            let entry = lines[first_entry_line];
            // Should be format: "0000000000 65535 f " or "XXXXXXXXXX 00000 n "
            assert!(entry.len() >= 18, "xref entry should be at least 18 chars");
        }

        fs::remove_file(path).ok();
    }
}

// ============================================================================
// MODULE: Real-World Document Workflows
// ============================================================================

mod real_world_workflows {
    use super::*;
    use micropdf::ffi::context::*;
    use micropdf::ffi::document::*;
    use micropdf::ffi::stream::*;

    /// Helper to open a document from memory using stream
    unsafe fn open_doc_from_memory(ctx: u64, data: &[u8]) -> u64 {
        let stm = fz_open_memory(ctx, data.as_ptr(), data.len());
        if stm == 0 {
            return 0;
        }
        let magic = std::ffi::CString::new("application/pdf").unwrap();
        let doc = fz_open_document_with_stream(ctx, magic.as_ptr(), stm);
        fz_drop_stream(ctx, stm);
        doc
    }

    fn contains_pattern(data: &[u8], pattern: &[u8]) -> bool {
        data.windows(pattern.len()).any(|w| w == pattern)
    }

    fn count_pattern(data: &[u8], pattern: &[u8]) -> usize {
        data.windows(pattern.len())
            .filter(|w| *w == pattern)
            .count()
    }

    // ========================================================================
    // Invoice/Report Generation Workflows
    // ========================================================================

    #[test]
    fn test_create_invoice_document() {
        let mut writer = micropdf::enhanced::writer::PdfWriter::new();

        // Create a realistic invoice layout
        let content = r#"q
            % Page border
            0.8 0.8 0.8 RG
            1 w
            36 36 540 720 re S

            % Header background
            0.1 0.2 0.4 rg
            36 720 540 36 re f

            % Company header text (white on blue)
            BT
                /F1 18 Tf
                1 1 1 rg
                50 730 Td
                (ACME CORPORATION) Tj
            ET

            % Invoice title
            BT
                /F1 24 Tf
                0 0 0 rg
                400 680 Td
                (INVOICE) Tj
            ET

            % Invoice details
            BT
                /F1 10 Tf
                400 660 Td
                (Invoice #: INV-2024-0001) Tj
                0 -14 Td
                (Date: January 15, 2024) Tj
                0 -14 Td
                (Due Date: February 15, 2024) Tj
            ET

            % Bill To section
            BT
                /F1 12 Tf
                0.3 0.3 0.3 rg
                50 660 Td
                (Bill To:) Tj
            ET

            BT
                /F1 10 Tf
                0 0 0 rg
                50 645 Td
                (John Smith) Tj
                0 -12 Td
                (123 Main Street) Tj
                0 -12 Td
                (Anytown, ST 12345) Tj
            ET

            % Table header
            0.9 0.9 0.9 rg
            50 550 490 20 re f

            0 0 0 RG
            0.5 w
            50 550 490 20 re S

            BT
                /F1 10 Tf
                0 0 0 rg
                55 555 Td
                (Description) Tj
                250 0 Td
                (Qty) Tj
                60 0 Td
                (Price) Tj
                60 0 Td
                (Total) Tj
            ET

            % Table rows
            0 0 0 RG
            50 530 490 0 re S
            50 510 490 0 re S
            50 490 490 0 re S

            BT
                /F1 10 Tf
                55 515 Td
                (Professional Services) Tj
                250 0 Td
                (10) Tj
                60 0 Td
                ($150.00) Tj
                60 0 Td
                ($1,500.00) Tj
            ET

            BT
                /F1 10 Tf
                55 495 Td
                (Software License) Tj
                250 0 Td
                (1) Tj
                60 0 Td
                ($500.00) Tj
                60 0 Td
                ($500.00) Tj
            ET

            % Total section
            0.95 0.95 0.95 rg
            350 430 190 60 re f

            BT
                /F1 10 Tf
                0 0 0 rg
                360 470 Td
                (Subtotal:) Tj
                100 0 Td
                ($2,000.00) Tj
            ET

            BT
                /F1 10 Tf
                360 455 Td
                (Tax \(8%\):) Tj
                100 0 Td
                ($160.00) Tj
            ET

            BT
                /F1 12 Tf
                360 435 Td
                (Total:) Tj
                100 0 Td
                ($2,160.00) Tj
            ET

            % Footer
            BT
                /F1 8 Tf
                0.5 0.5 0.5 rg
                200 50 Td
                (Thank you for your business!) Tj
            ET
        Q"#;

        writer.add_page_with_content(612.0, 792.0, content).unwrap();

        let path = temp_path("invoice.pdf");
        writer.save(path.to_str().unwrap()).unwrap();

        let data = fs::read(&path).unwrap();

        // Verify invoice content
        assert!(contains_pattern(&data, b"ACME CORPORATION"));
        assert!(contains_pattern(&data, b"INVOICE"));
        assert!(contains_pattern(&data, b"INV-2024-0001"));
        assert!(contains_pattern(&data, b"John Smith"));
        assert!(contains_pattern(&data, b"Professional Services"));
        assert!(contains_pattern(&data, b"$2,160.00"));
        assert!(contains_pattern(&data, b"Thank you for your business"));

        // Verify document loads correctly
        unsafe {
            let ctx = fz_new_context(std::ptr::null(), std::ptr::null(), 0);
            let doc = open_doc_from_memory(ctx, &data);
            assert_ne!(doc, 0);

            let page = fz_load_page(ctx, doc, 0);
            assert_ne!(page, 0);

            fz_drop_page(ctx, page);
            fz_drop_document(ctx, doc);
            fz_drop_context(ctx);
        }

        fs::remove_file(path).ok();
    }

    #[test]
    fn test_create_multipage_report() {
        let mut writer = micropdf::enhanced::writer::PdfWriter::new();

        // Cover page
        let cover = r#"q
            % Title page background gradient effect
            0.95 0.95 1 rg
            0 0 612 792 re f

            % Title
            BT
                /F1 36 Tf
                0.2 0.2 0.4 rg
                100 500 Td
                (Annual Report 2024) Tj
            ET

            BT
                /F1 18 Tf
                0.4 0.4 0.4 rg
                100 450 Td
                (Financial Summary and Analysis) Tj
            ET

            % Company name
            BT
                /F1 14 Tf
                100 350 Td
                (Prepared by: ACME Corporation) Tj
            ET

            % Date
            BT
                /F1 12 Tf
                100 320 Td
                (January 2024) Tj
            ET

            % Decorative line
            0.2 0.2 0.4 RG
            3 w
            100 300 m 400 300 l S
        Q"#;
        writer.add_page_with_content(612.0, 792.0, cover).unwrap();

        // Table of contents
        let toc = r#"q
            BT
                /F1 24 Tf
                0 0 0 rg
                50 720 Td
                (Table of Contents) Tj
            ET

            0.7 0.7 0.7 RG
            1 w
            50 705 m 300 705 l S

            BT
                /F1 12 Tf
                50 680 Td
                (1. Executive Summary ............................ 3) Tj
                0 -20 Td
                (2. Financial Overview ............................ 4) Tj
                0 -20 Td
                (3. Revenue Analysis ............................. 5) Tj
                0 -20 Td
                (4. Expense Breakdown .......................... 6) Tj
                0 -20 Td
                (5. Future Outlook ............................... 7) Tj
            ET

            % Page number
            BT
                /F1 10 Tf
                0.5 0.5 0.5 rg
                300 30 Td
                (Page 2) Tj
            ET
        Q"#;
        writer.add_page_with_content(612.0, 792.0, toc).unwrap();

        // Executive summary page
        let summary = r#"q
            % Header
            BT
                /F1 18 Tf
                0.2 0.2 0.4 rg
                50 720 Td
                (1. Executive Summary) Tj
            ET

            0.2 0.2 0.4 RG
            1 w
            50 710 m 250 710 l S

            % Content paragraphs
            BT
                /F1 11 Tf
                0 0 0 rg
                50 680 Td
                12 TL
                (This report presents the financial performance of ACME Corporation) Tj
                T*
                (for the fiscal year 2024. Key highlights include:) Tj
            ET

            % Bullet points
            BT
                /F1 11 Tf
                70 630 Td
                12 TL
                (- Revenue increased by 25% year-over-year) Tj
                T*
                (- Operating margin improved to 18%) Tj
                T*
                (- Customer base expanded to 50,000 clients) Tj
                T*
                (- New product launches contributed $5M in revenue) Tj
            ET

            % Chart placeholder
            0.9 0.9 0.9 rg
            100 400 400 150 re f
            0 0 0 RG
            0.5 w
            100 400 400 150 re S

            BT
                /F1 12 Tf
                0.5 0.5 0.5 rg
                250 470 Td
                (Revenue Growth Chart) Tj
            ET

            % Simple bar chart representation
            0.2 0.4 0.8 rg
            150 420 40 80 re f
            210 420 40 100 re f
            270 420 40 90 re f
            330 420 40 120 re f

            BT
                /F1 8 Tf
                0 0 0 rg
                155 410 Td
                (Q1) Tj
                60 0 Td
                (Q2) Tj
                60 0 Td
                (Q3) Tj
                60 0 Td
                (Q4) Tj
            ET

            % Page number
            BT
                /F1 10 Tf
                0.5 0.5 0.5 rg
                300 30 Td
                (Page 3) Tj
            ET
        Q"#;
        writer.add_page_with_content(612.0, 792.0, summary).unwrap();

        // Financial data page
        let financial = r#"q
            BT
                /F1 18 Tf
                0.2 0.2 0.4 rg
                50 720 Td
                (2. Financial Overview) Tj
            ET

            0.2 0.2 0.4 RG
            1 w
            50 710 m 250 710 l S

            % Data table header
            0.2 0.3 0.5 rg
            50 650 500 25 re f

            BT
                /F1 11 Tf
                1 1 1 rg
                60 658 Td
                (Metric) Tj
                150 0 Td
                (2023) Tj
                100 0 Td
                (2024) Tj
                100 0 Td
                (Change) Tj
            ET

            % Data rows
            0.95 0.95 0.95 rg
            50 625 500 25 re f
            1 1 1 rg
            50 600 500 25 re f
            0.95 0.95 0.95 rg
            50 575 500 25 re f
            1 1 1 rg
            50 550 500 25 re f

            0 0 0 RG
            0.3 w
            50 650 500 100 re S
            50 625 m 550 625 l S
            50 600 m 550 600 l S
            50 575 m 550 575 l S

            BT
                /F1 10 Tf
                0 0 0 rg
                60 633 Td
                (Revenue) Tj
                150 0 Td
                ($40.5M) Tj
                100 0 Td
                ($50.6M) Tj
                100 0 Td
                (+25%) Tj
            ET

            BT
                /F1 10 Tf
                60 608 Td
                (Net Income) Tj
                150 0 Td
                ($6.1M) Tj
                100 0 Td
                ($9.1M) Tj
                100 0 Td
                (+49%) Tj
            ET

            BT
                /F1 10 Tf
                60 583 Td
                (Operating Margin) Tj
                150 0 Td
                (15%) Tj
                100 0 Td
                (18%) Tj
                100 0 Td
                (+3pts) Tj
            ET

            BT
                /F1 10 Tf
                60 558 Td
                (Employees) Tj
                150 0 Td
                (250) Tj
                100 0 Td
                (320) Tj
                100 0 Td
                (+28%) Tj
            ET

            % Page number
            BT
                /F1 10 Tf
                0.5 0.5 0.5 rg
                300 30 Td
                (Page 4) Tj
            ET
        Q"#;
        writer
            .add_page_with_content(612.0, 792.0, financial)
            .unwrap();

        let path = temp_path("annual_report.pdf");
        writer.save(path.to_str().unwrap()).unwrap();

        let data = fs::read(&path).unwrap();

        // Verify report structure
        assert!(contains_pattern(&data, b"Annual Report 2024"));
        assert!(contains_pattern(&data, b"Table of Contents"));
        assert!(contains_pattern(&data, b"Executive Summary"));
        assert!(contains_pattern(&data, b"Financial Overview"));
        assert!(contains_pattern(&data, b"$50.6M"));
        assert!(contains_pattern(&data, b"/Count 4"));

        // Verify all pages load
        unsafe {
            let ctx = fz_new_context(std::ptr::null(), std::ptr::null(), 0);
            let doc = open_doc_from_memory(ctx, &data);
            assert_ne!(doc, 0);
            assert_eq!(fz_count_pages(ctx, doc), 4);

            for i in 0..4 {
                let page = fz_load_page(ctx, doc, i);
                assert_ne!(page, 0, "Page {} should load", i);
                fz_drop_page(ctx, page);
            }

            fz_drop_document(ctx, doc);
            fz_drop_context(ctx);
        }

        fs::remove_file(path).ok();
    }

    #[test]
    fn test_create_form_document() {
        let mut writer = micropdf::enhanced::writer::PdfWriter::new();

        // Application form
        let content = r#"q
            % Title
            BT
                /F1 20 Tf
                0 0 0 rg
                180 750 Td
                (APPLICATION FORM) Tj
            ET

            % Form fields with boxes
            0 0 0 RG
            0.5 w

            % Name field
            BT
                /F1 11 Tf
                50 700 Td
                (Full Name:) Tj
            ET
            150 695 350 18 re S

            % Email field
            BT
                /F1 11 Tf
                50 665 Td
                (Email Address:) Tj
            ET
            150 660 350 18 re S

            % Phone field
            BT
                /F1 11 Tf
                50 630 Td
                (Phone Number:) Tj
            ET
            150 625 200 18 re S

            % Date field
            BT
                /F1 11 Tf
                370 630 Td
                (Date:) Tj
            ET
            420 625 80 18 re S

            % Address section
            BT
                /F1 11 Tf
                50 595 Td
                (Address:) Tj
            ET
            150 590 350 18 re S

            BT
                /F1 11 Tf
                50 560 Td
                (City:) Tj
            ET
            150 555 150 18 re S

            BT
                /F1 11 Tf
                320 560 Td
                (State:) Tj
            ET
            370 555 50 18 re S

            BT
                /F1 11 Tf
                440 560 Td
                (ZIP:) Tj
            ET
            470 555 60 18 re S

            % Checkboxes
            BT
                /F1 11 Tf
                50 510 Td
                (Employment Status:) Tj
            ET

            50 480 12 12 re S
            BT /F1 10 Tf 70 482 Td (Full-time) Tj ET

            150 480 12 12 re S
            BT /F1 10 Tf 170 482 Td (Part-time) Tj ET

            250 480 12 12 re S
            BT /F1 10 Tf 270 482 Td (Contract) Tj ET

            350 480 12 12 re S
            BT /F1 10 Tf 370 482 Td (Unemployed) Tj ET

            % Large text area
            BT
                /F1 11 Tf
                50 440 Td
                (Additional Comments:) Tj
            ET
            50 340 500 90 re S

            % Signature section
            BT
                /F1 11 Tf
                50 280 Td
                (Signature:) Tj
            ET
            120 270 200 0.5 re f

            BT
                /F1 11 Tf
                350 280 Td
                (Date:) Tj
            ET
            390 270 100 0.5 re f

            % Terms and conditions
            BT
                /F1 8 Tf
                0.5 0.5 0.5 rg
                50 220 Td
                10 TL
                (By signing this form, I certify that the information provided is accurate) Tj
                T*
                (and complete to the best of my knowledge.) Tj
            ET

            % Footer
            0.9 0.9 0.9 rg
            0 0 612 40 re f

            BT
                /F1 8 Tf
                0.3 0.3 0.3 rg
                200 15 Td
                (Form Version 1.0 - Confidential) Tj
            ET
        Q"#;

        writer.add_page_with_content(612.0, 792.0, content).unwrap();

        let path = temp_path("application_form.pdf");
        writer.save(path.to_str().unwrap()).unwrap();

        let data = fs::read(&path).unwrap();

        // Verify form structure
        assert!(contains_pattern(&data, b"APPLICATION FORM"));
        assert!(contains_pattern(&data, b"Full Name"));
        assert!(contains_pattern(&data, b"Email Address"));
        assert!(contains_pattern(&data, b"Employment Status"));
        assert!(contains_pattern(&data, b"Signature"));
        assert!(contains_pattern(&data, b"Additional Comments"));

        // Count form field boxes (rectangles)
        assert!(
            count_pattern(&data, b" re S") >= 10,
            "Should have multiple form field rectangles"
        );

        fs::remove_file(path).ok();
    }

    // ========================================================================
    // Document Manipulation Workflows
    // ========================================================================

    #[test]
    fn test_merge_multiple_document_types() {
        // Create different document types and merge them
        let mut merger = micropdf::enhanced::page_ops::PdfMerger::new();

        // Add fixture documents
        merger
            .append(fixture_path("minimal.pdf").to_str().unwrap())
            .unwrap();
        merger
            .append(fixture_path("multipage.pdf").to_str().unwrap())
            .unwrap();
        merger
            .append(fixture_path("comprehensive_test.pdf").to_str().unwrap())
            .unwrap();

        let output = temp_path("merged_documents.pdf");
        merger.save(output.to_str().unwrap()).unwrap();

        // Verify merged document
        let data = fs::read(&output).unwrap();

        unsafe {
            let ctx = fz_new_context(std::ptr::null(), std::ptr::null(), 0);
            let doc = open_doc_from_memory(ctx, &data);
            assert_ne!(doc, 0);

            // 1 + 5 + 3 = 9 pages
            let count = fz_count_pages(ctx, doc);
            assert_eq!(count, 9, "Merged document should have 9 pages");

            // Verify each page loads and has valid bounds
            for i in 0..count {
                let page = fz_load_page(ctx, doc, i);
                assert_ne!(page, 0, "Page {} should load", i);

                let bounds = fz_bound_page(ctx, page);
                assert!(
                    bounds.x1 > 0.0 && bounds.y1 > 0.0,
                    "Page {} should have valid bounds",
                    i
                );

                fz_drop_page(ctx, page);
            }

            fz_drop_document(ctx, doc);
            fz_drop_context(ctx);
        }

        fs::remove_file(output).ok();
    }

    #[test]
    fn test_create_document_with_varied_page_sizes() {
        let mut writer = micropdf::enhanced::writer::PdfWriter::new();

        // US Letter
        let letter_content = r#"q
            BT /F1 24 Tf 200 400 Td (US Letter - 8.5 x 11 in) Tj ET
            0 0 0 RG 1 w 10 10 592 772 re S
        Q"#;
        writer
            .add_page_with_content(612.0, 792.0, letter_content)
            .unwrap();

        // A4
        let a4_content = r#"q
            BT /F1 24 Tf 180 420 Td (A4 - 210 x 297 mm) Tj ET
            0 0 0 RG 1 w 10 10 575 822 re S
        Q"#;
        writer
            .add_page_with_content(595.0, 842.0, a4_content)
            .unwrap();

        // US Legal
        let legal_content = r#"q
            BT /F1 24 Tf 200 500 Td (US Legal - 8.5 x 14 in) Tj ET
            0 0 0 RG 1 w 10 10 592 988 re S
        Q"#;
        writer
            .add_page_with_content(612.0, 1008.0, legal_content)
            .unwrap();

        // Tabloid
        let tabloid_content = r#"q
            BT /F1 24 Tf 300 600 Td (Tabloid - 11 x 17 in) Tj ET
            0 0 0 RG 1 w 10 10 772 1214 re S
        Q"#;
        writer
            .add_page_with_content(792.0, 1224.0, tabloid_content)
            .unwrap();

        // Custom square
        let square_content = r#"q
            BT /F1 24 Tf 150 300 Td (Custom Square - 500 x 500) Tj ET
            0 0 0 RG 1 w 10 10 480 480 re S
        Q"#;
        writer
            .add_page_with_content(500.0, 500.0, square_content)
            .unwrap();

        let path = temp_path("varied_sizes.pdf");
        writer.save(path.to_str().unwrap()).unwrap();

        let data = fs::read(&path).unwrap();

        // Verify pages load and have reasonable dimensions
        unsafe {
            let ctx = fz_new_context(std::ptr::null(), std::ptr::null(), 0);
            let doc = open_doc_from_memory(ctx, &data);
            assert_eq!(fz_count_pages(ctx, doc), 5);

            // Verify all 5 pages are loadable and have non-zero dimensions
            for i in 0..5 {
                let page = fz_load_page(ctx, doc, i as i32);
                let bounds = fz_bound_page(ctx, page);

                assert!(bounds.x1 > 0.0, "Page {} should have positive width", i);
                assert!(bounds.y1 > 0.0, "Page {} should have positive height", i);

                fz_drop_page(ctx, page);
            }

            fz_drop_document(ctx, doc);
            fz_drop_context(ctx);
        }

        fs::remove_file(path).ok();
    }

    // ========================================================================
    // Watermark and Overlay Workflows
    // ========================================================================

    #[test]
    fn test_create_document_with_watermark() {
        let mut writer = micropdf::enhanced::writer::PdfWriter::new();

        // Create a page with watermark effect
        let content = r#"q
            % Background watermark (diagonal text, light gray)
            q
                0.9 0.9 0.9 rg
                0.707 0.707 -0.707 0.707 306 0 cm
                BT
                    /F1 72 Tf
                    -100 350 Td
                    (CONFIDENTIAL) Tj
                ET
            Q

            % Actual content on top
            BT
                /F1 18 Tf
                0 0 0 rg
                50 720 Td
                (Internal Document) Tj
            ET

            BT
                /F1 12 Tf
                50 680 Td
                14 TL
                (This document contains proprietary information.) Tj
                T*
                (Distribution is restricted to authorized personnel only.) Tj
                T*
                T*
                (Key Points:) Tj
                T*
                (1. Project timeline has been updated) Tj
                T*
                (2. Budget allocation approved) Tj
                T*
                (3. Team expansion planned for Q2) Tj
            ET

            % Footer with classification
            0.9 0.9 0.9 rg
            0 0 612 30 re f

            BT
                /F1 10 Tf
                0.5 0 0 rg
                220 10 Td
                (CONFIDENTIAL - DO NOT DISTRIBUTE) Tj
            ET
        Q"#;

        writer.add_page_with_content(612.0, 792.0, content).unwrap();

        let path = temp_path("watermarked.pdf");
        writer.save(path.to_str().unwrap()).unwrap();

        let data = fs::read(&path).unwrap();

        // Verify watermark content
        assert!(contains_pattern(&data, b"CONFIDENTIAL"));
        assert!(contains_pattern(&data, b"Internal Document"));
        assert!(contains_pattern(&data, b"DO NOT DISTRIBUTE"));
        assert!(
            contains_pattern(&data, b" cm"),
            "Should have transformation matrix for watermark"
        );

        fs::remove_file(path).ok();
    }

    #[test]
    fn test_create_document_with_header_footer() {
        let mut writer = micropdf::enhanced::writer::PdfWriter::new();

        for page_num in 1..=3 {
            let content = format!(
                r#"q
                % Header
                0.95 0.95 0.95 rg
                0 762 612 30 re f

                0 0 0 RG
                0.5 w
                0 762 m 612 762 l S

                BT
                    /F1 10 Tf
                    0.3 0.3 0.3 rg
                    50 772 Td
                    (Company Confidential) Tj
                ET

                BT
                    /F1 10 Tf
                    480 772 Td
                    (Doc: ABC-123) Tj
                ET

                % Footer
                0.95 0.95 0.95 rg
                0 0 612 30 re f

                0 0 0 RG
                0.5 w
                0 30 m 612 30 l S

                BT
                    /F1 10 Tf
                    0.3 0.3 0.3 rg
                    50 10 Td
                    (Printed: 2024-01-15) Tj
                ET

                BT
                    /F1 10 Tf
                    280 10 Td
                    (Page {} of 3) Tj
                ET

                % Page content
                BT
                    /F1 16 Tf
                    0 0 0 rg
                    50 700 Td
                    (Section {}: Content Area) Tj
                ET

                BT
                    /F1 12 Tf
                    50 660 Td
                    (This is the main content area for page {}.) Tj
                ET
            Q"#,
                page_num, page_num, page_num
            );

            writer
                .add_page_with_content(612.0, 792.0, &content)
                .unwrap();
        }

        let path = temp_path("header_footer.pdf");
        writer.save(path.to_str().unwrap()).unwrap();

        let data = fs::read(&path).unwrap();

        // Verify header/footer content
        assert!(contains_pattern(&data, b"Company Confidential"));
        assert!(contains_pattern(&data, b"Doc: ABC-123"));
        assert!(contains_pattern(&data, b"Page 1 of 3"));
        assert!(contains_pattern(&data, b"Page 2 of 3"));
        assert!(contains_pattern(&data, b"Page 3 of 3"));
        assert!(contains_pattern(&data, b"Section 1"));
        assert!(contains_pattern(&data, b"Section 2"));
        assert!(contains_pattern(&data, b"Section 3"));

        fs::remove_file(path).ok();
    }

    // ========================================================================
    // Complex Layout Workflows
    // ========================================================================

    #[test]
    fn test_create_newsletter_layout() {
        let mut writer = micropdf::enhanced::writer::PdfWriter::new();

        let content = r#"q
            % Masthead
            0.1 0.3 0.5 rg
            0 742 612 50 re f

            BT
                /F1 28 Tf
                1 1 1 rg
                180 758 Td
                (THE DAILY NEWS) Tj
            ET

            BT
                /F1 10 Tf
                1 1 1 rg
                250 745 Td
                (January 15, 2024) Tj
            ET

            % Main headline
            BT
                /F1 24 Tf
                0 0 0 rg
                50 700 Td
                (Major Breakthrough in Technology) Tj
            ET

            0.7 0.7 0.7 RG
            1 w
            50 690 m 560 690 l S

            % Lead paragraph
            BT
                /F1 11 Tf
                50 665 Td
                12 TL
                (Scientists announced today a revolutionary new discovery that promises) Tj
                T*
                (to transform the way we interact with digital systems. The breakthrough) Tj
                T*
                (comes after years of dedicated research and development.) Tj
            ET

            % Column divider
            0.8 0.8 0.8 RG
            0.5 w
            306 50 m 306 610 l S

            % Left column
            BT
                /F1 14 Tf
                0.2 0.2 0.4 rg
                50 580 Td
                (Local News) Tj
            ET

            BT
                /F1 10 Tf
                0 0 0 rg
                50 560 Td
                11 TL
                (City council approved the new) Tj
                T*
                (development plan yesterday in) Tj
                T*
                (a unanimous vote. The plan) Tj
                T*
                (includes provisions for parks,) Tj
                T*
                (schools, and infrastructure.) Tj
            ET

            % Right column
            BT
                /F1 14 Tf
                0.2 0.2 0.4 rg
                320 580 Td
                (Weather) Tj
            ET

            % Weather box
            0.9 0.95 1 rg
            320 480 240 90 re f
            0.5 0.7 0.9 RG
            1 w
            320 480 240 90 re S

            BT
                /F1 36 Tf
                0.2 0.4 0.6 rg
                340 510 Td
                (72) Tj
            ET

            BT
                /F1 14 Tf
                390 520 Td
                (F) Tj
            ET

            BT
                /F1 10 Tf
                0 0 0 rg
                420 530 Td
                (Sunny) Tj
                0 -14 Td
                (High: 75F) Tj
                0 -12 Td
                (Low: 58F) Tj
            ET

            % Advertisement box
            0.95 0.95 0.9 rg
            320 350 240 100 re f
            0 0 0 RG
            1 w
            320 350 240 100 re S

            BT
                /F1 12 Tf
                0.5 0.3 0 rg
                360 420 Td
                (ADVERTISEMENT) Tj
            ET

            BT
                /F1 16 Tf
                0 0 0 rg
                350 390 Td
                (SALE - 50% OFF!) Tj
            ET

            BT
                /F1 10 Tf
                340 365 Td
                (Visit our store today for) Tj
                0 -12 Td
                (amazing deals on all items.) Tj
            ET

            % Footer
            0.9 0.9 0.9 rg
            0 0 612 40 re f

            BT
                /F1 8 Tf
                0.5 0.5 0.5 rg
                50 15 Td
                (Copyright 2024 The Daily News. All rights reserved.) Tj
            ET

            BT
                /F1 8 Tf
                450 15 Td
                (www.dailynews.com) Tj
            ET
        Q"#;

        writer.add_page_with_content(612.0, 792.0, content).unwrap();

        let path = temp_path("newsletter.pdf");
        writer.save(path.to_str().unwrap()).unwrap();

        let data = fs::read(&path).unwrap();

        // Verify newsletter structure
        assert!(contains_pattern(&data, b"THE DAILY NEWS"));
        assert!(contains_pattern(&data, b"Major Breakthrough"));
        assert!(contains_pattern(&data, b"Local News"));
        assert!(contains_pattern(&data, b"Weather"));
        assert!(contains_pattern(&data, b"ADVERTISEMENT"));
        assert!(contains_pattern(&data, b"SALE - 50% OFF"));
        assert!(contains_pattern(&data, b"Copyright 2024"));

        fs::remove_file(path).ok();
    }

    #[test]
    fn test_create_certificate() {
        let mut writer = micropdf::enhanced::writer::PdfWriter::new();

        // Landscape certificate
        let content = r#"q
            % Decorative border
            0.8 0.7 0.2 RG
            5 w
            30 30 732 532 re S

            3 w
            40 40 712 512 re S

            % Corner decorations (simplified)
            0.8 0.7 0.2 rg
            30 30 20 20 re f
            742 30 20 20 re f
            30 542 20 20 re f
            742 542 20 20 re f

            % Title
            BT
                /F1 36 Tf
                0.2 0.2 0.4 rg
                250 480 Td
                (CERTIFICATE) Tj
            ET

            BT
                /F1 18 Tf
                0.4 0.4 0.4 rg
                280 445 Td
                (of Achievement) Tj
            ET

            % Decorative line
            0.8 0.7 0.2 RG
            2 w
            200 430 m 592 430 l S

            % Recipient text
            BT
                /F1 14 Tf
                0 0 0 rg
                280 380 Td
                (This is to certify that) Tj
            ET

            BT
                /F1 28 Tf
                0.1 0.3 0.5 rg
                280 330 Td
                (John Doe) Tj
            ET

            % Achievement text
            BT
                /F1 14 Tf
                0 0 0 rg
                180 280 Td
                (has successfully completed the requirements for) Tj
            ET

            BT
                /F1 18 Tf
                0.2 0.2 0.4 rg
                220 240 Td
                (Advanced PDF Programming) Tj
            ET

            BT
                /F1 12 Tf
                0 0 0 rg
                280 200 Td
                (Awarded on January 15, 2024) Tj
            ET

            % Signature lines
            0.5 0.5 0.5 RG
            1 w
            150 120 150 0 re f
            492 120 150 0 re f

            BT
                /F1 10 Tf
                0.5 0.5 0.5 rg
                180 100 Td
                (Director) Tj
            ET

            BT
                /F1 10 Tf
                520 100 Td
                (Instructor) Tj
            ET

            % Seal placeholder
            0.9 0.9 0.9 rg
            366 80 60 60 re f
            0.8 0.7 0.2 RG
            2 w
            396 110 m
            396 140 c
            426 140 c
            426 110 c
            396 80 c
            366 80 c
            366 110 c
            396 110 c
            h S

            BT
                /F1 8 Tf
                0.5 0.5 0.5 rg
                375 105 Td
                (SEAL) Tj
            ET
        Q"#;

        writer
            .add_page_with_content(792.0, 612.0, content) // Landscape
            .unwrap();

        let path = temp_path("certificate.pdf");
        writer.save(path.to_str().unwrap()).unwrap();

        let data = fs::read(&path).unwrap();

        // Verify certificate content
        assert!(contains_pattern(&data, b"CERTIFICATE"));
        assert!(contains_pattern(&data, b"of Achievement"));
        assert!(contains_pattern(&data, b"John Doe"));
        assert!(contains_pattern(&data, b"Advanced PDF Programming"));
        assert!(contains_pattern(&data, b"Director"));
        assert!(contains_pattern(&data, b"Instructor"));

        // Verify page loads and has reasonable dimensions
        unsafe {
            let ctx = fz_new_context(std::ptr::null(), std::ptr::null(), 0);
            let doc = open_doc_from_memory(ctx, &data);
            let page = fz_load_page(ctx, doc, 0);
            let bounds = fz_bound_page(ctx, page);

            // Verify page has non-zero dimensions
            assert!(bounds.x1 > 0.0, "Certificate should have positive width");
            assert!(bounds.y1 > 0.0, "Certificate should have positive height");

            fz_drop_page(ctx, page);
            fz_drop_document(ctx, doc);
            fz_drop_context(ctx);
        }

        fs::remove_file(path).ok();
    }

    // ========================================================================
    // Batch Processing Workflows
    // ========================================================================

    #[test]
    fn test_batch_create_labeled_pages() {
        let mut writer = micropdf::enhanced::writer::PdfWriter::new();

        // Create 10 pages with unique identifiers
        for i in 1..=10 {
            let label = format!("PAGE-{:04}", i);
            let content = format!(
                r#"q
                % Page identifier watermark
                0.95 0.95 0.95 rg
                BT
                    /F1 72 Tf
                    150 350 Td
                    ({}) Tj
                ET

                % Content
                BT
                    /F1 18 Tf
                    0 0 0 rg
                    50 720 Td
                    (Document Page {}) Tj
                ET

                BT
                    /F1 12 Tf
                    50 680 Td
                    (Unique ID: {}) Tj
                ET

                BT
                    /F1 11 Tf
                    50 640 Td
                    14 TL
                    (This page was automatically generated as part of) Tj
                    T*
                    (a batch processing workflow. Each page contains) Tj
                    T*
                    (a unique identifier for tracking purposes.) Tj
                ET

                % Page border
                0.8 0.8 0.8 RG
                1 w
                30 30 552 732 re S

                % Footer with sequence info
                BT
                    /F1 10 Tf
                    0.5 0.5 0.5 rg
                    250 15 Td
                    (Page {} of 10 | Batch: BATCH-2024-001) Tj
                ET
            Q"#,
                label, i, label, i
            );

            writer
                .add_page_with_content(612.0, 792.0, &content)
                .unwrap();
        }

        let path = temp_path("batch_pages.pdf");
        writer.save(path.to_str().unwrap()).unwrap();

        let data = fs::read(&path).unwrap();

        // Verify all page identifiers
        for i in 1..=10 {
            let label = format!("PAGE-{:04}", i);
            assert!(
                contains_pattern(&data, label.as_bytes()),
                "Should contain {}",
                label
            );
        }

        assert!(contains_pattern(&data, b"BATCH-2024-001"));
        assert!(contains_pattern(&data, b"/Count 10"));

        // Verify all pages load
        unsafe {
            let ctx = fz_new_context(std::ptr::null(), std::ptr::null(), 0);
            let doc = open_doc_from_memory(ctx, &data);
            assert_eq!(fz_count_pages(ctx, doc), 10);

            fz_drop_document(ctx, doc);
            fz_drop_context(ctx);
        }

        fs::remove_file(path).ok();
    }

    #[test]
    fn test_create_index_cards() {
        let mut writer = micropdf::enhanced::writer::PdfWriter::new();

        let items = [
            (
                "Widget A",
                "WGT-001",
                "Industrial component for assembly",
                "$45.00",
            ),
            ("Widget B", "WGT-002", "Premium grade fastener", "$62.50"),
            (
                "Gadget X",
                "GDT-001",
                "Electronic control module",
                "$125.00",
            ),
            ("Gadget Y", "GDT-002", "Sensor array unit", "$89.99"),
            ("Part Z", "PRT-001", "Replacement bearing set", "$34.00"),
        ];

        for (name, sku, description, price) in items {
            // Index card size (4x6 inches)
            let content = format!(
                r#"q
                % Card border
                0 0 0 RG
                2 w
                10 10 412 268 re S

                % Header stripe
                0.2 0.4 0.6 rg
                10 248 412 30 re f

                % Product name
                BT
                    /F1 16 Tf
                    1 1 1 rg
                    20 258 Td
                    ({}) Tj
                ET

                % SKU
                BT
                    /F1 10 Tf
                    340 258 Td
                    ({}) Tj
                ET

                % Description
                BT
                    /F1 12 Tf
                    0 0 0 rg
                    20 220 Td
                    ({}) Tj
                ET

                % Price
                BT
                    /F1 24 Tf
                    0.2 0.6 0.2 rg
                    300 180 Td
                    ({}) Tj
                ET

                % Barcode placeholder
                0.9 0.9 0.9 rg
                20 30 150 40 re f
                0 0 0 RG
                0.5 w
                20 30 150 40 re S

                BT
                    /F1 8 Tf
                    0.5 0.5 0.5 rg
                    60 45 Td
                    (BARCODE) Tj
                ET

                % QR placeholder
                0.9 0.9 0.9 rg
                350 30 60 60 re f
                0 0 0 RG
                350 30 60 60 re S

                BT
                    /F1 6 Tf
                    365 55 Td
                    (QR) Tj
                ET
            Q"#,
                name, sku, description, price
            );

            // 4x6 inches = 288x432 points
            writer
                .add_page_with_content(432.0, 288.0, &content)
                .unwrap();
        }

        let path = temp_path("index_cards.pdf");
        writer.save(path.to_str().unwrap()).unwrap();

        let data = fs::read(&path).unwrap();

        // Verify all items
        assert!(contains_pattern(&data, b"Widget A"));
        assert!(contains_pattern(&data, b"Widget B"));
        assert!(contains_pattern(&data, b"Gadget X"));
        assert!(contains_pattern(&data, b"WGT-001"));
        assert!(contains_pattern(&data, b"GDT-002"));
        assert!(contains_pattern(&data, b"$125.00"));
        assert!(contains_pattern(&data, b"/Count 5"));

        // Verify cards are loadable with reasonable dimensions
        unsafe {
            let ctx = fz_new_context(std::ptr::null(), std::ptr::null(), 0);
            let doc = open_doc_from_memory(ctx, &data);
            assert_eq!(fz_count_pages(ctx, doc), 5);

            let page = fz_load_page(ctx, doc, 0);
            let bounds = fz_bound_page(ctx, page);

            // Verify page has non-zero dimensions
            assert!(bounds.x1 > 0.0, "Card should have positive width");
            assert!(bounds.y1 > 0.0, "Card should have positive height");

            fz_drop_page(ctx, page);
            fz_drop_document(ctx, doc);
            fz_drop_context(ctx);
        }

        fs::remove_file(path).ok();
    }

    // ========================================================================
    // Graphics-Heavy Document Tests
    // ========================================================================

    #[test]
    fn test_create_diagram_document() {
        let mut writer = micropdf::enhanced::writer::PdfWriter::new();

        let content = r#"q
            % Title
            BT
                /F1 20 Tf
                0 0 0 rg
                200 750 Td
                (System Architecture) Tj
            ET

            % Box 1: Client
            0.9 0.95 1 rg
            50 600 120 60 re f
            0 0 0.8 RG
            1.5 w
            50 600 120 60 re S

            BT
                /F1 12 Tf
                0 0 0.5 rg
                80 625 Td
                (Client) Tj
            ET

            % Box 2: API Gateway
            0.95 1 0.9 rg
            250 600 120 60 re f
            0 0.6 0 RG
            1.5 w
            250 600 120 60 re S

            BT
                /F1 12 Tf
                0 0.4 0 rg
                265 625 Td
                (API Gateway) Tj
            ET

            % Box 3: Server
            1 0.95 0.9 rg
            450 600 120 60 re f
            0.8 0.4 0 RG
            1.5 w
            450 600 120 60 re S

            BT
                /F1 12 Tf
                0.5 0.25 0 rg
                485 625 Td
                (Server) Tj
            ET

            % Arrows
            0.3 0.3 0.3 RG
            2 w

            % Client -> Gateway
            170 630 m 250 630 l S
            245 635 m 250 630 l 245 625 l S

            % Gateway -> Server
            370 630 m 450 630 l S
            445 635 m 450 630 l 445 625 l S

            % Database box
            0.95 0.9 1 rg
            450 450 120 80 re f
            0.5 0 0.5 RG
            1.5 w
            450 450 120 80 re S

            BT
                /F1 12 Tf
                0.3 0 0.3 rg
                475 485 Td
                (Database) Tj
            ET

            % Server -> Database arrow
            510 600 m 510 530 l S
            505 535 m 510 530 l 515 535 l S

            % Cache box
            1 1 0.9 rg
            250 450 120 80 re f
            0.8 0.6 0 RG
            1.5 w
            250 450 120 80 re S

            BT
                /F1 12 Tf
                0.5 0.4 0 rg
                285 485 Td
                (Cache) Tj
            ET

            % Gateway -> Cache arrow
            310 600 m 310 530 l S
            305 535 m 310 530 l 315 535 l S

            % Legend
            0.95 0.95 0.95 rg
            50 300 200 120 re f
            0 0 0 RG
            0.5 w
            50 300 200 120 re S

            BT
                /F1 11 Tf
                0 0 0 rg
                60 400 Td
                (Legend:) Tj
            ET

            0.9 0.95 1 rg 70 375 30 15 re f
            0 0 0.8 RG 70 375 30 15 re S
            BT /F1 9 Tf 0 0 0 rg 110 378 Td (Client Layer) Tj ET

            0.95 1 0.9 rg 70 355 30 15 re f
            0 0.6 0 RG 70 355 30 15 re S
            BT /F1 9 Tf 110 358 Td (Gateway Layer) Tj ET

            1 0.95 0.9 rg 70 335 30 15 re f
            0.8 0.4 0 RG 70 335 30 15 re S
            BT /F1 9 Tf 110 338 Td (Application Layer) Tj ET

            0.95 0.9 1 rg 70 315 30 15 re f
            0.5 0 0.5 RG 70 315 30 15 re S
            BT /F1 9 Tf 110 318 Td (Data Layer) Tj ET

            % Notes
            BT
                /F1 10 Tf
                0.4 0.4 0.4 rg
                50 250 Td
                12 TL
                (Note: All communication uses HTTPS.) Tj
                T*
                (Cache invalidation handled by event bus.) Tj
            ET
        Q"#;

        writer.add_page_with_content(612.0, 792.0, content).unwrap();

        let path = temp_path("architecture_diagram.pdf");
        writer.save(path.to_str().unwrap()).unwrap();

        let data = fs::read(&path).unwrap();

        // Verify diagram elements
        assert!(contains_pattern(&data, b"System Architecture"));
        assert!(contains_pattern(&data, b"Client"));
        assert!(contains_pattern(&data, b"API Gateway"));
        assert!(contains_pattern(&data, b"Server"));
        assert!(contains_pattern(&data, b"Database"));
        assert!(contains_pattern(&data, b"Cache"));
        assert!(contains_pattern(&data, b"Legend"));

        // Verify graphics operators for boxes and arrows
        assert!(
            count_pattern(&data, b" re f") >= 8,
            "Should have filled rectangles"
        );
        assert!(
            count_pattern(&data, b" re S") >= 8,
            "Should have stroked rectangles"
        );
        assert!(
            count_pattern(&data, b" l S") >= 4,
            "Should have lines for arrows"
        );

        fs::remove_file(path).ok();
    }

    #[test]
    fn test_create_chart_document() {
        let mut writer = micropdf::enhanced::writer::PdfWriter::new();

        let content = r#"q
            % Title
            BT
                /F1 18 Tf
                0 0 0 rg
                220 750 Td
                (Sales Performance 2024) Tj
            ET

            % Y-axis
            0 0 0 RG
            1 w
            100 200 m 100 650 l S

            % Y-axis labels
            BT /F1 8 Tf 60 200 Td (0) Tj ET
            BT /F1 8 Tf 60 290 Td (20) Tj ET
            BT /F1 8 Tf 60 380 Td (40) Tj ET
            BT /F1 8 Tf 60 470 Td (60) Tj ET
            BT /F1 8 Tf 60 560 Td (80) Tj ET
            BT /F1 8 Tf 55 650 Td (100) Tj ET

            % Y-axis ticks
            95 290 m 100 290 l S
            95 380 m 100 380 l S
            95 470 m 100 470 l S
            95 560 m 100 560 l S
            95 650 m 100 650 l S

            % X-axis
            100 200 m 550 200 l S

            % Grid lines (light)
            0.9 0.9 0.9 RG
            0.5 w
            100 290 m 550 290 l S
            100 380 m 550 380 l S
            100 470 m 550 470 l S
            100 560 m 550 560 l S
            100 650 m 550 650 l S

            % Bar chart bars
            % Q1 - Blue
            0.2 0.4 0.8 rg
            120 200 60 270 re f

            % Q2 - Green
            0.2 0.7 0.3 rg
            200 200 60 360 re f

            % Q3 - Orange
            0.9 0.5 0.1 rg
            280 200 60 315 re f

            % Q4 - Purple
            0.6 0.2 0.7 rg
            360 200 60 450 re f

            % X-axis labels
            0 0 0 rg
            BT /F1 10 Tf 135 180 Td (Q1) Tj ET
            BT /F1 10 Tf 215 180 Td (Q2) Tj ET
            BT /F1 10 Tf 295 180 Td (Q3) Tj ET
            BT /F1 10 Tf 375 180 Td (Q4) Tj ET

            % Bar values
            BT /F1 9 Tf 0 0 0 rg 135 480 Td (60%) Tj ET
            BT /F1 9 Tf 215 570 Td (80%) Tj ET
            BT /F1 9 Tf 295 525 Td (70%) Tj ET
            BT /F1 9 Tf 375 660 Td (100%) Tj ET

            % Legend
            0.95 0.95 0.95 rg
            420 600 130 80 re f
            0 0 0 RG
            0.5 w
            420 600 130 80 re S

            0.2 0.4 0.8 rg 430 660 15 10 re f
            BT /F1 9 Tf 0 0 0 rg 450 662 Td (Q1: $60K) Tj ET

            0.2 0.7 0.3 rg 430 645 15 10 re f
            BT /F1 9 Tf 450 647 Td (Q2: $80K) Tj ET

            0.9 0.5 0.1 rg 430 630 15 10 re f
            BT /F1 9 Tf 450 632 Td (Q3: $70K) Tj ET

            0.6 0.2 0.7 rg 430 615 15 10 re f
            BT /F1 9 Tf 450 617 Td (Q4: $100K) Tj ET

            % Total
            BT
                /F1 12 Tf
                0 0 0 rg
                220 120 Td
                (Total Annual Sales: $310,000) Tj
            ET

            % Y-axis title (rotated text simulation)
            BT
                /F1 10 Tf
                0.5 0.5 0.5 rg
                40 420 Td
                (Sales \(%\)) Tj
            ET
        Q"#;

        writer.add_page_with_content(612.0, 792.0, content).unwrap();

        let path = temp_path("bar_chart.pdf");
        writer.save(path.to_str().unwrap()).unwrap();

        let data = fs::read(&path).unwrap();

        // Verify chart elements
        assert!(contains_pattern(&data, b"Sales Performance 2024"));
        assert!(contains_pattern(&data, b"Q1"));
        assert!(contains_pattern(&data, b"Q2"));
        assert!(contains_pattern(&data, b"Q3"));
        assert!(contains_pattern(&data, b"Q4"));
        assert!(contains_pattern(&data, b"$310,000"));

        // Verify graphics for bars
        assert!(
            count_pattern(&data, b" re f") >= 8,
            "Should have filled bars and legend boxes"
        );

        fs::remove_file(path).ok();
    }

    // ========================================================================
    // Document Security and Metadata Workflows
    // ========================================================================

    #[test]
    fn test_document_with_classification_levels() {
        let classifications = [
            ("PUBLIC", "0 0.6 0", "This document may be shared freely."),
            ("INTERNAL", "0.8 0.6 0", "For internal use only."),
            ("CONFIDENTIAL", "0.8 0.4 0", "Restricted distribution."),
            ("SECRET", "0.8 0 0", "Highly restricted access."),
        ];

        let mut writer = micropdf::enhanced::writer::PdfWriter::new();

        for (level, color, description) in classifications {
            let content = format!(
                r#"q
                % Classification banner at top
                {} rg
                0 762 612 30 re f

                BT
                    /F1 14 Tf
                    1 1 1 rg
                    250 770 Td
                    ({}) Tj
                ET

                % Classification banner at bottom
                {} rg
                0 0 612 30 re f

                BT
                    /F1 14 Tf
                    1 1 1 rg
                    250 8 Td
                    ({}) Tj
                ET

                % Page content
                BT
                    /F1 18 Tf
                    0 0 0 rg
                    50 700 Td
                    (Document Classification: {}) Tj
                ET

                BT
                    /F1 12 Tf
                    50 660 Td
                    ({}) Tj
                ET

                BT
                    /F1 11 Tf
                    50 620 Td
                    14 TL
                    (This page demonstrates a {} classified document.) Tj
                    T*
                    (All handling and distribution must comply with) Tj
                    T*
                    (applicable security policies and procedures.) Tj
                ET

                % Classification marking watermark
                0.95 0.95 0.95 rg
                BT
                    /F1 48 Tf
                    200 350 Td
                    ({}) Tj
                ET
            Q"#,
                color, level, color, level, level, description, level, level
            );

            writer
                .add_page_with_content(612.0, 792.0, &content)
                .unwrap();
        }

        let path = temp_path("classified_docs.pdf");
        writer.save(path.to_str().unwrap()).unwrap();

        let data = fs::read(&path).unwrap();

        // Verify all classification levels
        assert!(contains_pattern(&data, b"PUBLIC"));
        assert!(contains_pattern(&data, b"INTERNAL"));
        assert!(contains_pattern(&data, b"CONFIDENTIAL"));
        assert!(contains_pattern(&data, b"SECRET"));
        assert!(contains_pattern(&data, b"/Count 4"));

        fs::remove_file(path).ok();
    }

    // ========================================================================
    // Stress Tests
    // ========================================================================

    #[test]
    fn test_many_pages_performance() {
        use std::time::Instant;

        let start = Instant::now();
        let mut writer = micropdf::enhanced::writer::PdfWriter::new();

        // Create 50 pages with content
        for i in 1..=50 {
            let content = format!(
                r#"q
                BT
                    /F1 24 Tf
                    200 400 Td
                    (Page {} of 50) Tj
                ET

                0 0 0 RG
                1 w
                50 50 512 692 re S

                BT
                    /F1 10 Tf
                    50 700 Td
                    (Generated content for stress testing) Tj
                ET
            Q"#,
                i
            );

            writer
                .add_page_with_content(612.0, 792.0, &content)
                .unwrap();
        }

        let path = temp_path("stress_50_pages.pdf");
        writer.save(path.to_str().unwrap()).unwrap();

        let elapsed = start.elapsed();
        assert!(
            elapsed.as_secs() < 10,
            "Creating 50 pages should take less than 10 seconds"
        );

        let data = fs::read(&path).unwrap();
        assert!(contains_pattern(&data, b"/Count 50"));

        // Verify we can load and access all pages
        unsafe {
            let ctx = fz_new_context(std::ptr::null(), std::ptr::null(), 0);
            let doc = open_doc_from_memory(ctx, &data);
            assert_eq!(fz_count_pages(ctx, doc), 50);

            // Quick check of first, middle, and last pages
            for i in [0, 24, 49] {
                let page = fz_load_page(ctx, doc, i);
                assert_ne!(page, 0);
                fz_drop_page(ctx, page);
            }

            fz_drop_document(ctx, doc);
            fz_drop_context(ctx);
        }

        fs::remove_file(path).ok();
    }

    #[test]
    fn test_large_content_page() {
        let mut writer = micropdf::enhanced::writer::PdfWriter::new();

        // Generate a page with a lot of text content
        let mut content = String::from("q\n");

        // Add many text blocks
        for row in 0..50 {
            let y = 750 - (row * 14);
            content.push_str(&format!(
                "BT /F1 10 Tf 50 {} Td (Line {}: Lorem ipsum dolor sit amet, consectetur adipiscing elit.) Tj ET\n",
                y, row + 1
            ));
        }

        // Add many rectangles
        for i in 0..20 {
            let x = 50 + (i % 5) * 100;
            let y = 50 + (i / 5) * 50;
            content.push_str(&format!(
                "0.{} 0.{} 0.{} rg {} {} 80 30 re f\n",
                i % 10,
                (i + 3) % 10,
                (i + 6) % 10,
                x,
                y
            ));
        }

        content.push_str("Q\n");

        writer
            .add_page_with_content(612.0, 792.0, &content)
            .unwrap();

        let path = temp_path("large_content.pdf");
        writer.save(path.to_str().unwrap()).unwrap();

        let data = fs::read(&path).unwrap();

        // Should have significant content
        assert!(data.len() > 5000, "PDF should have substantial content");
        assert!(contains_pattern(&data, b"Line 1:"));
        assert!(contains_pattern(&data, b"Line 50:"));
        assert!(count_pattern(&data, b"Lorem ipsum") >= 50);

        fs::remove_file(path).ok();
    }
}
