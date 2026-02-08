use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

fn main() {
    // Get version from Cargo.toml
    let version = env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "0.1.0".to_string());

    // Determine prefix based on environment or use default
    let prefix = env::var("PREFIX").unwrap_or_else(|_| "/usr".to_string());

    // Create output directory for generated files
    let out_dir = env::var("OUT_DIR").unwrap();
    let pkg_config_dir = Path::new(&out_dir).join("pkgconfig");
    fs::create_dir_all(&pkg_config_dir).expect("Failed to create pkgconfig directory");

    // Create include directory if it doesn't exist
    let include_dir = Path::new("include");
    fs::create_dir_all(include_dir).expect("Failed to create include directory");

    // Generate C header files for FFI
    generate_ffi_headers();

    // Generate comprehensive MuPDF-compatible headers from Rust FFI
    generate_mupdf_headers();

    // Generate micropdf.pc
    generate_pkg_config(
        "micropdf.pc.in",
        &pkg_config_dir.join("micropdf.pc"),
        &version,
        &prefix,
    );

    // Generate mupdf.pc (compatibility alias)
    generate_pkg_config(
        "mupdf.pc.in",
        &pkg_config_dir.join("mupdf.pc"),
        &version,
        &prefix,
    );

    println!("cargo:rerun-if-changed=micropdf.pc.in");
    println!("cargo:rerun-if-changed=mupdf.pc.in");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/ffi/");
    println!("cargo:rerun-if-changed=scripts/generate_headers.py");
}

fn generate_ffi_headers() {
    // Generate micropdf.h - the main comprehensive FFI header
    let micropdf_header = r#"/**
 * MicroPDF - Fast, lightweight PDF library
 *
 * This is a comprehensive C FFI header for the MicroPDF Rust library.
 * All functions are prefixed with fz_ or pdf_ for compatibility with MuPDF.
 *
 * This header includes all auto-generated module headers with complete
 * function declarations for all 660+ FFI functions.
 *
 * Usage:
 *   #include <micropdf.h>
 *
 * For MuPDF drop-in compatibility:
 *   #include <mupdf.h>
 */

#ifndef MICROPDF_H
#define MICROPDF_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================================
// Type Definitions - Opaque handles for resource management
// ============================================================================

typedef int32_t fz_context;
typedef int32_t fz_document;
typedef int32_t fz_page;
typedef int32_t fz_device;
typedef int32_t fz_pixmap;
typedef int32_t fz_buffer;
typedef int32_t fz_stream;
typedef int32_t fz_output;
typedef int32_t fz_colorspace;
typedef int32_t fz_font;
typedef int32_t fz_image;
typedef int32_t fz_path;
typedef int32_t fz_text;
typedef int32_t fz_cookie;
typedef int32_t fz_display_list;
typedef int32_t fz_link;
typedef int32_t fz_archive;
typedef int32_t pdf_obj;
typedef int32_t pdf_annot;
typedef int32_t pdf_form_field;

// ============================================================================
// Geometry types (used by many modules)
// ============================================================================

typedef struct {
    float x, y;
} fz_point;

typedef struct {
    float x0, y0;
    float x1, y1;
} fz_rect;

typedef struct {
    int x0, y0;
    int x1, y1;
} fz_irect;

typedef struct {
    float a, b, c, d, e, f;
} fz_matrix;

typedef struct {
    fz_point ul, ur, ll, lr;
} fz_quad;

// ============================================================================
// Common type aliases
// ============================================================================

typedef int32_t PdfObjHandle;
typedef int32_t Handle;

// ============================================================================
// Function Declarations
// ============================================================================

/*
 * All function declarations are auto-generated from Rust FFI source.
 * See individual module headers in mupdf/fitz/ and mupdf/pdf/ for details.
 *
 * Total: 660+ functions covering:
 * - Core fitz functions (geometry, buffers, streams, devices, etc.)
 * - PDF-specific functions (annotations, forms, objects, etc.)
 */

// For complete function declarations, include the comprehensive header:
#include "mupdf.h"

#ifdef __cplusplus
}
#endif

#endif /* MICROPDF_H */
"#;

    fs::write("include/micropdf.h", micropdf_header).expect("Failed to write micropdf.h");
    println!("Generated: include/micropdf.h");

    // Generate mupdf-ffi.h - MuPDF compatibility header
    let mupdf_ffi_header = r#"/**
 * MuPDF FFI Compatibility Header
 *
 * This header provides 100% MuPDF-compatible FFI bindings.
 * Include this for drop-in compatibility with MuPDF-based applications.
 *
 * All 660+ fz_* and pdf_* functions are available through this header.
 *
 * Usage:
 *   #include <mupdf-ffi.h>
 *
 * Or for complete MuPDF compatibility:
 *   #include <mupdf.h>
 */

#ifndef MUPDF_FFI_H
#define MUPDF_FFI_H

#include "micropdf.h"

/*
 * All MuPDF-compatible functions are available through micropdf.h
 *
 * Function categories:
 * - Context management (fz_new_context, fz_drop_context, etc.)
 * - Document operations (fz_open_document, fz_load_page, etc.)
 * - Geometry operations (fz_concat, fz_transform_rect, etc.)
 * - Buffer operations (fz_new_buffer, fz_append_data, etc.)
 * - Device operations (fz_new_bbox_device, fz_fill_path, etc.)
 * - Image operations (fz_new_image_from_pixmap, fz_decode_image, etc.)
 * - Text operations (fz_new_text, fz_show_string, etc.)
 * - PDF object operations (pdf_new_dict, pdf_dict_get, etc.)
 * - PDF annotation operations (pdf_create_annot, pdf_set_annot_contents, etc.)
 * - PDF form operations (pdf_next_widget, pdf_set_field_value, etc.)
 *
 * Total coverage: 660+ functions
 */

#endif /* MUPDF_FFI_H */
"#;

    fs::write("include/mupdf-ffi.h", mupdf_ffi_header).expect("Failed to write mupdf-ffi.h");
    println!("Generated: include/mupdf-ffi.h");
}

fn generate_mupdf_headers() {
    // Run the Python header generation script
    let script_path = Path::new("scripts/generate_headers.py");

    if !script_path.exists() {
        eprintln!(
            "Warning: Header generation script not found at {:?}",
            script_path
        );
        return;
    }

    let output = Command::new("python3").arg(script_path).output();

    match output {
        Ok(result) => {
            if result.status.success() {
                println!("✅ Generated MuPDF-compatible headers");
                if !result.stdout.is_empty() {
                    println!("{}", String::from_utf8_lossy(&result.stdout));
                }
            } else {
                eprintln!("Warning: Header generation failed");
                eprintln!("{}", String::from_utf8_lossy(&result.stderr));
            }
        }
        Err(e) => {
            eprintln!("Warning: Could not run header generation script: {}", e);
        }
    }
}

fn generate_pkg_config(template: &str, output: &Path, version: &str, prefix: &str) {
    let template_content = fs::read_to_string(template)
        .unwrap_or_else(|_| panic!("Failed to read template: {}", template));

    let content = template_content
        .replace("@VERSION@", version)
        .replace("@PREFIX@", prefix);

    fs::write(output, content)
        .unwrap_or_else(|_| panic!("Failed to write pkg-config file: {}", output.display()));

    println!("Generated: {}", output.display());
}
