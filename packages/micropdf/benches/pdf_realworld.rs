//! Real-World PDF Benchmarks
//!
//! Benchmarks for common PDF operations:
//! - Document loading (via PDF data parsing)
//! - Page operations
//! - Text extraction simulation
//! - Matrix transformations (used in rendering)
//!
//! Uses synthetic PDF data to measure real-world performance patterns.

use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};

use micropdf::fitz::buffer::Buffer;
use micropdf::fitz::geometry::{Matrix, Point, Quad, Rect};
use micropdf::fitz::pixmap::Pixmap;

// ============================================================================
// Test PDF Generation
// ============================================================================

/// Generate a minimal valid PDF with N pages
fn generate_test_pdf(page_count: usize, text_density: TextDensity) -> Vec<u8> {
    let mut pdf = Vec::new();

    // PDF Header
    pdf.extend_from_slice(b"%PDF-1.7\n");
    pdf.extend_from_slice(b"%\xE2\xE3\xCF\xD3\n"); // Binary marker

    let mut obj_offsets = Vec::new();
    let mut obj_num = 1;

    // Catalog (object 1)
    obj_offsets.push(pdf.len());
    let catalog = format!(
        "{} 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n",
        obj_num
    );
    pdf.extend_from_slice(catalog.as_bytes());
    obj_num += 1;

    // Pages tree (object 2)
    obj_offsets.push(pdf.len());
    let kids: Vec<String> = (0..page_count)
        .map(|i| format!("{} 0 R", 3 + i * 2))
        .collect();
    let pages = format!(
        "{} 0 obj\n<< /Type /Pages /Kids [{}] /Count {} >>\nendobj\n",
        obj_num,
        kids.join(" "),
        page_count
    );
    pdf.extend_from_slice(pages.as_bytes());
    obj_num += 1;

    // Generate pages
    for page_idx in 0..page_count {
        // Page object
        obj_offsets.push(pdf.len());
        let page_obj = format!(
            "{} 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] \
             /Contents {} 0 R /Resources << /Font << /F1 << /Type /Font \
             /Subtype /Type1 /BaseFont /Helvetica >> >> >> >>\nendobj\n",
            obj_num,
            obj_num + 1
        );
        pdf.extend_from_slice(page_obj.as_bytes());
        obj_num += 1;

        // Content stream
        obj_offsets.push(pdf.len());
        let content = generate_page_content(page_idx, text_density);
        let content_obj = format!(
            "{} 0 obj\n<< /Length {} >>\nstream\n{}\nendstream\nendobj\n",
            obj_num,
            content.len(),
            content
        );
        pdf.extend_from_slice(content_obj.as_bytes());
        obj_num += 1;
    }

    // Cross-reference table
    let xref_offset = pdf.len();
    pdf.extend_from_slice(b"xref\n");
    pdf.extend_from_slice(format!("0 {}\n", obj_num).as_bytes());
    pdf.extend_from_slice(b"0000000000 65535 f \n");
    for offset in &obj_offsets {
        pdf.extend_from_slice(format!("{:010} 00000 n \n", offset).as_bytes());
    }

    // Trailer
    pdf.extend_from_slice(b"trailer\n");
    pdf.extend_from_slice(format!("<< /Size {} /Root 1 0 R >>\n", obj_num).as_bytes());
    pdf.extend_from_slice(b"startxref\n");
    pdf.extend_from_slice(format!("{}\n", xref_offset).as_bytes());
    pdf.extend_from_slice(b"%%EOF\n");

    pdf
}

#[derive(Clone, Copy)]
enum TextDensity {
    Minimal, // ~100 characters per page
    Light,   // ~500 characters per page
    Medium,  // ~2000 characters per page
    Heavy,   // ~10000 characters per page
}

fn generate_page_content(page_idx: usize, density: TextDensity) -> String {
    let line_count = match density {
        TextDensity::Minimal => 2,
        TextDensity::Light => 10,
        TextDensity::Medium => 40,
        TextDensity::Heavy => 200,
    };

    let mut content = String::new();
    content.push_str("BT\n");
    content.push_str("/F1 12 Tf\n");

    for line in 0..line_count {
        let y = 750.0 - (line as f64 * 14.0);
        content.push_str(&format!("1 0 0 1 50 {} Tm\n", y));
        content.push_str(&format!(
            "(Page {} Line {} - Lorem ipsum dolor sit amet, consectetur adipiscing elit) Tj\n",
            page_idx + 1,
            line + 1
        ));
    }

    content.push_str("ET\n");
    content
}

// ============================================================================
// PDF Data Loading Benchmarks
// ============================================================================

fn bench_pdf_load_to_buffer(c: &mut Criterion) {
    let mut group = c.benchmark_group("pdf/load_buffer");

    // Test different page counts
    for page_count in [1, 10, 50, 100].iter() {
        let pdf_data = generate_test_pdf(*page_count, TextDensity::Light);
        group.throughput(Throughput::Bytes(pdf_data.len() as u64));

        group.bench_with_input(
            BenchmarkId::new("pages", page_count),
            &pdf_data,
            |b, data| {
                b.iter(|| {
                    let buf = Buffer::from_slice(black_box(data));
                    black_box(buf.len())
                });
            },
        );
    }

    // Test different text densities
    for density in [
        TextDensity::Minimal,
        TextDensity::Light,
        TextDensity::Medium,
        TextDensity::Heavy,
    ]
    .iter()
    {
        let name = match density {
            TextDensity::Minimal => "minimal",
            TextDensity::Light => "light",
            TextDensity::Medium => "medium",
            TextDensity::Heavy => "heavy",
        };
        let pdf_data = generate_test_pdf(10, *density);
        group.throughput(Throughput::Bytes(pdf_data.len() as u64));

        group.bench_with_input(BenchmarkId::new("density", name), &pdf_data, |b, data| {
            b.iter(|| {
                let buf = Buffer::from_slice(black_box(data));
                black_box(buf.len())
            });
        });
    }

    group.finish();
}

// ============================================================================
// PDF Parsing Simulation Benchmarks
// ============================================================================

fn bench_pdf_parse_simulation(c: &mut Criterion) {
    let mut group = c.benchmark_group("pdf/parse");

    let pdf_data = generate_test_pdf(10, TextDensity::Medium);

    // Simulate finding xref offset (scan backwards for "startxref")
    group.bench_function("find_xref_offset", |b| {
        b.iter(|| {
            let data = black_box(&pdf_data);
            // Scan backwards for startxref
            let pattern = b"startxref";
            for i in (0..data.len().saturating_sub(pattern.len())).rev() {
                if &data[i..i + pattern.len()] == pattern {
                    return black_box(i);
                }
            }
            black_box(0)
        });
    });

    // Simulate counting objects
    group.bench_function("count_objects", |b| {
        b.iter(|| {
            let data = black_box(&pdf_data);
            let mut count = 0;
            let pattern = b" 0 obj";
            for i in 0..data.len().saturating_sub(pattern.len()) {
                if &data[i..i + pattern.len()] == pattern {
                    count += 1;
                }
            }
            black_box(count)
        });
    });

    // Simulate extracting page count from trailer
    group.bench_function("find_page_count", |b| {
        b.iter(|| {
            let data = black_box(&pdf_data);
            let pattern = b"/Count ";
            for i in 0..data.len().saturating_sub(pattern.len() + 3) {
                if &data[i..i + pattern.len()] == pattern {
                    // Parse number after /Count
                    let start = i + pattern.len();
                    let mut end = start;
                    while end < data.len() && data[end].is_ascii_digit() {
                        end += 1;
                    }
                    if let Ok(s) = std::str::from_utf8(&data[start..end]) {
                        return black_box(s.parse::<i32>().unwrap_or(0));
                    }
                }
            }
            black_box(0)
        });
    });

    group.finish();
}

// ============================================================================
// Pixmap Creation Benchmarks (Render Target)
// ============================================================================

fn bench_pixmap_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("pdf/pixmap");

    // Different resolutions (72 DPI, 144 DPI, 288 DPI, 576 DPI for letter size)
    for (width, height, dpi) in [
        (612, 792, 72),
        (1224, 1584, 144),
        (2448, 3168, 288),
        (4896, 6336, 576),
    ]
    .iter()
    {
        let pixels = (*width as u64) * (*height as u64) * 4; // RGBA
        group.throughput(Throughput::Bytes(pixels));

        group.bench_with_input(
            BenchmarkId::new("create_rgba", format!("{}dpi", dpi)),
            &(*width, *height),
            |b, &(w, h)| {
                b.iter(|| {
                    let pixmap = Pixmap::new(None, black_box(w), black_box(h), true);
                    black_box(pixmap)
                });
            },
        );
    }

    // RGB without alpha
    for (width, height, dpi) in [(612, 792, 72), (1224, 1584, 144)].iter() {
        let pixels = (*width as u64) * (*height as u64) * 3; // RGB
        group.throughput(Throughput::Bytes(pixels));

        group.bench_with_input(
            BenchmarkId::new("create_rgb", format!("{}dpi", dpi)),
            &(*width, *height),
            |b, &(w, h)| {
                b.iter(|| {
                    let pixmap = Pixmap::new(None, black_box(w), black_box(h), false);
                    black_box(pixmap)
                });
            },
        );
    }

    group.finish();
}

// ============================================================================
// Matrix Operations Benchmarks (PDF coordinate transforms)
// ============================================================================

fn bench_matrix_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("pdf/matrix");

    // Identity matrix creation
    group.bench_function("identity", |b| {
        b.iter(|| black_box(Matrix::new(1.0, 0.0, 0.0, 1.0, 0.0, 0.0)))
    });

    // Scale matrix (common for DPI scaling)
    for scale in [1.0, 2.0, 4.0, 8.0].iter() {
        group.bench_with_input(BenchmarkId::new("scale", scale), scale, |b, &s| {
            b.iter(|| black_box(Matrix::scale(s, s)))
        });
    }

    // Rotate matrix (common for page rotation)
    for angle in [0.0, 90.0, 180.0, 270.0].iter() {
        group.bench_with_input(BenchmarkId::new("rotate", angle), angle, |b, &a| {
            b.iter(|| black_box(Matrix::rotate(a)))
        });
    }

    // DPI scaling matrix chain (common render setup)
    group.bench_function("dpi_scale_chain", |b| {
        b.iter(|| {
            let scale = 2.0; // 144 DPI
            let m = Matrix::scale(black_box(scale), black_box(scale));
            black_box(m)
        })
    });

    // Full render transform (scale + translate)
    group.bench_function("render_transform", |b| {
        b.iter(|| {
            let scale = Matrix::scale(2.0, 2.0);
            let translate = Matrix::translate(-50.0, -50.0);
            let result = scale.concat(&translate);
            black_box(result)
        })
    });

    // Transform a page worth of points (simulate text positioning)
    let m = Matrix::scale(2.0, 2.0).concat(&Matrix::translate(100.0, 100.0));
    let points: Vec<Point> = (0..1000)
        .map(|i| Point::new(i as f32, i as f32 * 0.75))
        .collect();

    group.bench_function("transform_1000_points", |b| {
        b.iter(|| {
            let transformed: Vec<Point> = points
                .iter()
                .map(|p| black_box(&m).transform_point(*p))
                .collect();
            black_box(transformed)
        })
    });

    // Transform page bounds (common operation)
    let page_bounds = Rect::new(0.0, 0.0, 612.0, 792.0);
    group.bench_function("transform_page_bounds", |b| {
        b.iter(|| black_box(&page_bounds).transform(black_box(&m)))
    });

    group.finish();
}

// ============================================================================
// Text Extraction Simulation
// ============================================================================

fn bench_text_extraction_simulation(c: &mut Criterion) {
    let mut group = c.benchmark_group("pdf/text_extract");

    // Simulate building text from characters
    let chars: Vec<char> = "Lorem ipsum dolor sit amet, consectetur adipiscing elit. "
        .chars()
        .cycle()
        .take(10000)
        .collect();

    group.bench_function("build_string_10k_chars", |b| {
        b.iter(|| {
            let text: String = black_box(&chars).iter().collect();
            black_box(text)
        })
    });

    // Simulate word boundary detection
    let text = "Lorem ipsum dolor sit amet, consectetur adipiscing elit. ".repeat(200);
    group.bench_function("find_word_boundaries", |b| {
        b.iter(|| {
            let words: Vec<&str> = black_box(&text).split_whitespace().collect();
            black_box(words.len())
        })
    });

    // Simulate line break detection
    let multi_line = "Line 1 of text content\nLine 2 of text content\nLine 3 of text\n".repeat(100);
    group.bench_function("find_line_breaks", |b| {
        b.iter(|| {
            let lines: Vec<&str> = black_box(&multi_line).lines().collect();
            black_box(lines.len())
        })
    });

    // Simulate text search
    let search_text = "Lorem ipsum dolor sit amet ".repeat(1000);
    group.bench_function("text_search", |b| {
        b.iter(|| {
            let count = black_box(&search_text).matches("dolor").count();
            black_box(count)
        })
    });

    group.finish();
}

// ============================================================================
// Memory Allocation Patterns
// ============================================================================

fn bench_allocation_patterns(c: &mut Criterion) {
    let mut group = c.benchmark_group("pdf/allocation");

    // Buffer allocation sizes typical in PDF processing
    for size in [1024, 4096, 16384, 65536, 262144].iter() {
        group.throughput(Throughput::Bytes(*size as u64));

        group.bench_with_input(BenchmarkId::new("buffer", size), size, |b, &size| {
            b.iter(|| {
                let buf = Buffer::new(black_box(size));
                black_box(buf)
            })
        });
    }

    // Repeated small allocations (common in text extraction)
    group.bench_function("small_allocs_1000", |b| {
        b.iter(|| {
            let buffers: Vec<Buffer> = (0..1000).map(|_| Buffer::new(64)).collect();
            black_box(buffers)
        })
    });

    // Geometry type allocations (common in rendering)
    group.bench_function("rect_allocs_1000", |b| {
        b.iter(|| {
            let rects: Vec<Rect> = (0..1000)
                .map(|i| Rect::new(i as f32, i as f32, i as f32 + 100.0, i as f32 + 100.0))
                .collect();
            black_box(rects)
        })
    });

    group.bench_function("point_allocs_1000", |b| {
        b.iter(|| {
            let points: Vec<Point> = (0..1000).map(|i| Point::new(i as f32, i as f32)).collect();
            black_box(points)
        })
    });

    group.bench_function("quad_allocs_1000", |b| {
        b.iter(|| {
            let quads: Vec<Quad> = (0..1000)
                .map(|i| {
                    let f = i as f32;
                    Quad::from_rect(&Rect::new(f, f, f + 100.0, f + 20.0))
                })
                .collect();
            black_box(quads)
        })
    });

    group.finish();
}

// ============================================================================
// MuPDF C Library Comparison Baseline
// ============================================================================

/// Baseline benchmarks for comparison with MuPDF C library.
/// When MuPDF C library is available, add corresponding C benchmarks.
fn bench_comparison_baseline(c: &mut Criterion) {
    let mut group = c.benchmark_group("pdf/baseline");

    // Baseline: Load 10-page PDF into buffer
    let pdf_data = generate_test_pdf(10, TextDensity::Medium);
    group.throughput(Throughput::Bytes(pdf_data.len() as u64));

    group.bench_function("rust_buffer_load", |b| {
        b.iter(|| {
            let buf = Buffer::from_slice(black_box(&pdf_data));
            black_box(buf.len())
        })
    });

    // Baseline: Create render target pixmap (72 DPI letter size)
    group.bench_function("rust_pixmap_create_72dpi", |b| {
        b.iter(|| {
            let pixmap = Pixmap::new(None, 612, 792, true);
            black_box(pixmap)
        })
    });

    // Baseline: Matrix operations for render setup
    group.bench_function("rust_render_matrix_setup", |b| {
        b.iter(|| {
            let scale = Matrix::scale(2.0, 2.0);
            let rotate = Matrix::rotate(0.0);
            let translate = Matrix::translate(0.0, 0.0);
            let result = scale.concat(&rotate).concat(&translate);
            black_box(result)
        })
    });

    // To add MuPDF C comparison, uncomment and implement:
    // group.bench_function("mupdf_c_buffer_load", |b| { ... });
    // group.bench_function("mupdf_c_pixmap_create", |b| { ... });
    // group.bench_function("mupdf_c_render_matrix", |b| { ... });

    group.finish();
}

// ============================================================================
// Criterion Groups
// ============================================================================

criterion_group!(
    benches,
    bench_pdf_load_to_buffer,
    bench_pdf_parse_simulation,
    bench_pixmap_creation,
    bench_matrix_operations,
    bench_text_extraction_simulation,
    bench_allocation_patterns,
    bench_comparison_baseline,
);

criterion_main!(benches);
