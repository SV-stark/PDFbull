use criterion::{Criterion, black_box, criterion_group, criterion_main};
use micropdf::fitz::geometry::{Matrix, Point, Quad, Rect};

fn bench_point_transform(c: &mut Criterion) {
    let point = Point::new(100.0, 200.0);
    let matrix = Matrix::rotate(45.0);

    c.bench_function("point/transform", |b| {
        b.iter(|| black_box(point).transform(black_box(&matrix)))
    });
}

fn bench_point_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("point");

    group.bench_function("new", |b| {
        b.iter(|| Point::new(black_box(100.0), black_box(200.0)))
    });

    group.finish();
}

fn bench_matrix_operations(c: &mut Criterion) {
    let m1 = Matrix::translate(100.0, 200.0);
    let m2 = Matrix::scale(2.0, 2.0);
    let m3 = Matrix::rotate(45.0);

    let mut group = c.benchmark_group("matrix");

    group.bench_function("translate", |b| {
        b.iter(|| Matrix::translate(black_box(100.0), black_box(200.0)))
    });

    group.bench_function("scale", |b| {
        b.iter(|| Matrix::scale(black_box(2.0), black_box(2.0)))
    });

    group.bench_function("rotate", |b| b.iter(|| Matrix::rotate(black_box(45.0))));

    group.bench_function("concat", |b| {
        b.iter(|| black_box(&m1).concat(black_box(&m2)))
    });

    group.bench_function("chain_3", |b| {
        b.iter(|| black_box(&m1).concat(black_box(&m2)).concat(black_box(&m3)))
    });

    group.finish();
}

fn bench_rect_operations(c: &mut Criterion) {
    let r1 = Rect::new(0.0, 0.0, 100.0, 100.0);
    let r2 = Rect::new(50.0, 50.0, 150.0, 150.0);

    let mut group = c.benchmark_group("rect");

    group.bench_function("new", |b| {
        b.iter(|| {
            Rect::new(
                black_box(0.0),
                black_box(0.0),
                black_box(100.0),
                black_box(100.0),
            )
        })
    });

    group.bench_function("union", |b| b.iter(|| black_box(&r1).union(black_box(&r2))));

    group.bench_function("intersect", |b| {
        b.iter(|| black_box(&r1).intersect(black_box(&r2)))
    });

    group.bench_function("contains", |b| {
        b.iter(|| black_box(&r1).contains(black_box(75.0), black_box(75.0)))
    });

    group.bench_function("width_height", |b| {
        b.iter(|| (black_box(&r1).width(), black_box(&r1).height()))
    });

    group.bench_function("is_empty", |b| b.iter(|| black_box(&r1).is_empty()));

    group.finish();
}

fn bench_quad_operations(c: &mut Criterion) {
    let rect = Rect::new(0.0, 0.0, 100.0, 100.0);
    let quad = Quad::from_rect(&rect);
    let matrix = Matrix::rotate(45.0);

    let mut group = c.benchmark_group("quad");

    group.bench_function("from_rect", |b| {
        b.iter(|| Quad::from_rect(black_box(&rect)))
    });

    group.bench_function("transform", |b| {
        b.iter(|| black_box(&quad).transform(black_box(&matrix)))
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_point_transform,
    bench_point_operations,
    bench_matrix_operations,
    bench_rect_operations,
    bench_quad_operations,
);

criterion_main!(benches);
