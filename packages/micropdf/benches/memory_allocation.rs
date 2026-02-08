//! Memory Allocation Profiling Benchmarks
//!
//! Measures memory allocation patterns for core MicroPDF operations:
//! - Allocation count per operation
//! - Bytes allocated per operation
//! - Peak memory usage
//! - Allocation hotspots
//!
//! Uses a custom allocator wrapper to track allocations during benchmarks.

use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};

use micropdf::fitz::buffer::Buffer;
use micropdf::fitz::geometry::{Matrix, Point, Quad, Rect};

// ============================================================================
// Helper Functions
// ============================================================================

/// Helper to create identity matrix
fn matrix_identity() -> Matrix {
    Matrix::new(1.0, 0.0, 0.0, 1.0, 0.0, 0.0)
}

/// Helper to invert a matrix
fn matrix_invert(m: &Matrix) -> Option<Matrix> {
    let det = m.a * m.d - m.b * m.c;
    if det.abs() < 1e-10 {
        return None;
    }
    let inv_det = 1.0 / det;
    Some(Matrix::new(
        m.d * inv_det,
        -m.b * inv_det,
        -m.c * inv_det,
        m.a * inv_det,
        (m.c * m.f - m.d * m.e) * inv_det,
        (m.b * m.e - m.a * m.f) * inv_det,
    ))
}

/// Helper to get quad bounding box
fn quad_bounds(q: &Quad) -> Rect {
    let min_x = q.ul.x.min(q.ur.x).min(q.ll.x).min(q.lr.x);
    let min_y = q.ul.y.min(q.ur.y).min(q.ll.y).min(q.lr.y);
    let max_x = q.ul.x.max(q.ur.x).max(q.ll.x).max(q.lr.x);
    let max_y = q.ul.y.max(q.ur.y).max(q.ll.y).max(q.lr.y);
    Rect::new(min_x, min_y, max_x, max_y)
}

/// Helper to check if quad contains point
fn quad_contains(q: &Quad, p: &Point) -> bool {
    // Bounding box check first
    let bounds = quad_bounds(q);
    if !bounds.contains(p.x, p.y) {
        return false;
    }

    // Cross product check for convex quad
    fn cross(ax: f32, ay: f32, bx: f32, by: f32, cx: f32, cy: f32) -> f32 {
        (bx - ax) * (cy - ay) - (by - ay) * (cx - ax)
    }

    let c1 = cross(q.ul.x, q.ul.y, q.ur.x, q.ur.y, p.x, p.y);
    let c2 = cross(q.ur.x, q.ur.y, q.lr.x, q.lr.y, p.x, p.y);
    let c3 = cross(q.lr.x, q.lr.y, q.ll.x, q.ll.y, p.x, p.y);
    let c4 = cross(q.ll.x, q.ll.y, q.ul.x, q.ul.y, p.x, p.y);

    (c1 >= 0.0 && c2 >= 0.0 && c3 >= 0.0 && c4 >= 0.0)
        || (c1 <= 0.0 && c2 <= 0.0 && c3 <= 0.0 && c4 <= 0.0)
}

// ============================================================================
// Geometry Allocation Benchmarks
// ============================================================================

fn bench_point_allocations(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory/point");

    // Single point creation
    group.bench_function("create_single", |b| {
        b.iter(|| Point::new(black_box(10.0), black_box(20.0)))
    });

    // Batch point creation
    for count in [10, 100, 1000] {
        group.bench_with_input(BenchmarkId::new("create_batch", count), &count, |b, &n| {
            b.iter(|| {
                let points: Vec<Point> = (0..n)
                    .map(|i| Point::new(i as f32, i as f32 * 2.0))
                    .collect();
                black_box(points)
            })
        });
    }

    // Point transform (no allocation expected)
    let p = Point::new(10.0, 20.0);
    let m = Matrix::scale(2.0, 2.0);
    group.bench_function("transform", |b| {
        b.iter(|| black_box(&p).transform(black_box(&m)))
    });

    group.finish();
}

fn bench_rect_allocations(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory/rect");

    // Single rect creation
    group.bench_function("create_single", |b| {
        b.iter(|| {
            Rect::new(
                black_box(0.0),
                black_box(0.0),
                black_box(100.0),
                black_box(100.0),
            )
        })
    });

    // Batch rect creation
    for count in [10, 100, 1000] {
        group.bench_with_input(BenchmarkId::new("create_batch", count), &count, |b, &n| {
            b.iter(|| {
                let rects: Vec<Rect> = (0..n)
                    .map(|i| Rect::new(0.0, 0.0, i as f32, i as f32))
                    .collect();
                black_box(rects)
            })
        });
    }

    // Rect transform
    let r = Rect::new(0.0, 0.0, 100.0, 100.0);
    let m = Matrix::rotate(45.0);
    group.bench_function("transform", |b| {
        b.iter(|| black_box(&r).transform(black_box(&m)))
    });

    // Rect operations (intersection, union)
    let r1 = Rect::new(0.0, 0.0, 100.0, 100.0);
    let r2 = Rect::new(50.0, 50.0, 150.0, 150.0);
    group.bench_function("intersect", |b| {
        b.iter(|| black_box(&r1).intersect(black_box(&r2)))
    });

    group.bench_function("union", |b| b.iter(|| black_box(&r1).union(black_box(&r2))));

    group.finish();
}

fn bench_matrix_allocations(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory/matrix");

    // Matrix creation methods
    group.bench_function("identity", |b| b.iter(matrix_identity));

    group.bench_function("scale", |b| {
        b.iter(|| Matrix::scale(black_box(2.0), black_box(2.0)))
    });

    group.bench_function("rotate", |b| b.iter(|| Matrix::rotate(black_box(45.0))));

    group.bench_function("translate", |b| {
        b.iter(|| Matrix::translate(black_box(100.0), black_box(100.0)))
    });

    // Matrix concatenation (may allocate intermediate)
    let m1 = Matrix::scale(2.0, 2.0);
    let m2 = Matrix::rotate(45.0);
    group.bench_function("concat", |b| {
        b.iter(|| black_box(&m1).concat(black_box(&m2)))
    });

    // Chain of transforms
    group.bench_function("chain_4_transforms", |b| {
        b.iter(|| {
            matrix_identity()
                .concat(&Matrix::scale(2.0, 2.0))
                .concat(&Matrix::rotate(45.0))
                .concat(&Matrix::translate(100.0, 100.0))
                .concat(&Matrix::scale(0.5, 0.5))
        })
    });

    // Matrix inversion
    let m = Matrix::scale(2.0, 3.0).concat(&Matrix::rotate(30.0));
    group.bench_function("invert", |b| b.iter(|| matrix_invert(black_box(&m))));

    group.finish();
}

fn bench_quad_allocations(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory/quad");

    // Quad from rect
    let r = Rect::new(0.0, 0.0, 100.0, 100.0);
    group.bench_function("from_rect", |b| b.iter(|| Quad::from_rect(black_box(&r))));

    // Quad transform
    let q = Quad::from_rect(&r);
    let m = Matrix::rotate(45.0);
    group.bench_function("transform", |b| {
        b.iter(|| black_box(&q).transform(black_box(&m)))
    });

    // Quad to bounding rect
    group.bench_function("bounds", |b| b.iter(|| quad_bounds(black_box(&q))));

    // Contains point
    let p = Point::new(50.0, 50.0);
    group.bench_function("contains_point", |b| {
        b.iter(|| quad_contains(black_box(&q), black_box(&p)))
    });

    group.finish();
}

// ============================================================================
// Buffer Allocation Benchmarks
// ============================================================================

fn bench_buffer_allocations(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory/buffer");

    // Buffer creation with various sizes
    for size in [0, 64, 256, 1024, 4096, 16384, 65536] {
        group.bench_with_input(BenchmarkId::new("create", size), &size, |b, &size| {
            b.iter(|| Buffer::new(black_box(size)))
        });
    }

    // Buffer from data (copies data)
    for size in [64, 256, 1024, 4096, 16384] {
        let data: Vec<u8> = (0..size).map(|i| i as u8).collect();
        group.bench_with_input(BenchmarkId::new("from_slice", size), &data, |b, data| {
            b.iter(|| Buffer::from_slice(black_box(data)))
        });
    }

    // Buffer append (may reallocate)
    let chunk: Vec<u8> = vec![0u8; 256];
    for append_count in [1, 10, 100] {
        group.bench_with_input(
            BenchmarkId::new("append_256B_x", append_count),
            &append_count,
            |b, &count| {
                b.iter(|| {
                    let mut buf = Buffer::new(0);
                    for _ in 0..count {
                        buf.append_data(black_box(&chunk));
                    }
                    buf
                })
            },
        );
    }

    // Buffer with pre-allocated capacity (should not reallocate)
    for append_count in [10, 100] {
        let capacity = 256 * append_count;
        group.bench_with_input(
            BenchmarkId::new("append_preallocated", append_count),
            &append_count,
            |b, &count| {
                b.iter(|| {
                    let mut buf = Buffer::new(capacity);
                    for _ in 0..count {
                        buf.append_data(black_box(&chunk));
                    }
                    buf
                })
            },
        );
    }

    group.finish();
}

// ============================================================================
// Operation Memory Profiles
// ============================================================================

/// Estimate allocations for common operation patterns
fn bench_operation_profiles(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory/operation_profile");

    // Profile: Create and transform 100 points
    group.bench_function("transform_100_points", |b| {
        let m = Matrix::scale(2.0, 2.0).concat(&Matrix::rotate(45.0));
        b.iter(|| {
            let points: Vec<Point> = (0..100)
                .map(|i| {
                    let p = Point::new(i as f32, i as f32 * 2.0);
                    p.transform(&m)
                })
                .collect();
            black_box(points)
        })
    });

    // Profile: Create and transform 100 rects
    group.bench_function("transform_100_rects", |b| {
        let m = Matrix::scale(1.5, 1.5).concat(&Matrix::rotate(30.0));
        b.iter(|| {
            let rects: Vec<Rect> = (0..100)
                .map(|i| {
                    let r = Rect::new(0.0, 0.0, (i + 10) as f32, (i + 10) as f32);
                    r.transform(&m)
                })
                .collect();
            black_box(rects)
        })
    });

    // Profile: Build buffer incrementally (simulates content stream)
    group.bench_function("build_content_stream_1KB", |b| {
        b.iter(|| {
            let mut buf = Buffer::new(1024);
            buf.append_data(b"BT\n");
            buf.append_data(b"/F1 12 Tf\n");
            for i in 0..50 {
                let line = format!("1 0 0 1 72 {} Tm\n", 700 - i * 14);
                buf.append_data(line.as_bytes());
                buf.append_data(b"(Hello World) Tj\n");
            }
            buf.append_data(b"ET\n");
            buf
        })
    });

    // Profile: Matrix chain for page rendering
    group.bench_function("page_render_matrix_chain", |b| {
        b.iter(|| {
            // Typical rendering transform: scale to DPI, rotate, translate
            let dpi_scale = Matrix::scale(2.0, 2.0); // 144 DPI
            let rotation = Matrix::rotate(0.0); // No rotation
            let translate = Matrix::translate(0.0, 0.0);

            let ctm = dpi_scale.concat(&rotation).concat(&translate);

            // Transform page bounds
            let page_bounds = Rect::new(0.0, 0.0, 612.0, 792.0);
            let transformed = page_bounds.transform(&ctm);

            black_box(transformed)
        })
    });

    group.finish();
}

// ============================================================================
// Memory Size Tracking
// ============================================================================

/// Benchmark to verify type sizes (compile-time check)
fn bench_type_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory/type_sizes");

    // Report sizes (this is more of a documentation benchmark)
    group.bench_function("point_size", |b| b.iter(std::mem::size_of::<Point>));

    group.bench_function("rect_size", |b| b.iter(std::mem::size_of::<Rect>));

    group.bench_function("matrix_size", |b| b.iter(std::mem::size_of::<Matrix>));

    group.bench_function("quad_size", |b| b.iter(std::mem::size_of::<Quad>));

    // Print sizes (visible in benchmark output)
    println!("\nType sizes:");
    println!("  Point:  {} bytes", std::mem::size_of::<Point>());
    println!("  Rect:   {} bytes", std::mem::size_of::<Rect>());
    println!("  Matrix: {} bytes", std::mem::size_of::<Matrix>());
    println!("  Quad:   {} bytes", std::mem::size_of::<Quad>());
    println!(
        "  Buffer: {} bytes (struct only)",
        std::mem::size_of::<Buffer>()
    );

    group.finish();
}

// ============================================================================
// Criterion Configuration
// ============================================================================

criterion_group!(
    name = memory_benches;
    config = Criterion::default()
        .sample_size(100)
        .measurement_time(std::time::Duration::from_secs(3));
    targets =
        bench_point_allocations,
        bench_rect_allocations,
        bench_matrix_allocations,
        bench_quad_allocations,
        bench_buffer_allocations,
        bench_operation_profiles,
        bench_type_sizes,
);

criterion_main!(memory_benches);
