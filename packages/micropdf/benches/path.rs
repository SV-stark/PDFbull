use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use micropdf::fitz::geometry::{Point, Rect};
use micropdf::fitz::path::Path;

fn bench_path_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("path/creation");

    group.bench_function("new", |b| b.iter(Path::new));

    group.bench_function("with_capacity", |b| {
        b.iter(|| Path::with_capacity(black_box(100)))
    });

    group.finish();
}

fn bench_path_building(c: &mut Criterion) {
    let mut group = c.benchmark_group("path/building");

    group.bench_function("move_to", |b| {
        let mut path = Path::new();
        b.iter(|| {
            black_box(&mut path).move_to(black_box(Point::new(10.0, 20.0)));
        })
    });

    group.bench_function("line_to", |b| {
        let mut path = Path::new();
        path.move_to(Point::new(0.0, 0.0));
        b.iter(|| {
            black_box(&mut path).line_to(black_box(Point::new(100.0, 100.0)));
        })
    });

    group.bench_function("curve_to", |b| {
        let mut path = Path::new();
        path.move_to(Point::new(0.0, 0.0));
        b.iter(|| {
            black_box(&mut path).curve_to(
                black_box(Point::new(25.0, 50.0)),
                black_box(Point::new(75.0, 50.0)),
                black_box(Point::new(100.0, 0.0)),
            );
        })
    });

    group.bench_function("close", |b| {
        let mut path = Path::new();
        path.move_to(Point::new(0.0, 0.0));
        path.line_to(Point::new(100.0, 0.0));
        path.line_to(Point::new(100.0, 100.0));
        b.iter(|| {
            black_box(&mut path).close();
        })
    });

    group.finish();
}

fn bench_path_shapes(c: &mut Criterion) {
    let mut group = c.benchmark_group("path/shapes");

    group.bench_function("rectangle", |b| {
        let mut path = Path::new();
        let rect = Rect::new(0.0, 0.0, 100.0, 100.0);
        b.iter(|| {
            black_box(&mut path).rect(black_box(rect));
        })
    });

    group.finish();
}

fn bench_path_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("path/operations");

    let mut path = Path::new();
    path.move_to(Point::new(0.0, 0.0));
    path.line_to(Point::new(100.0, 0.0));
    path.line_to(Point::new(100.0, 100.0));
    path.line_to(Point::new(0.0, 100.0));
    path.close();

    group.bench_function("clone", |b| b.iter(|| black_box(&path).clone()));

    group.bench_function("is_empty", |b| b.iter(|| black_box(&path).is_empty()));

    group.finish();
}

fn bench_path_complexity(c: &mut Criterion) {
    let mut group = c.benchmark_group("path/complexity");

    for segments in [10, 50, 100, 500, 1000].iter() {
        group.bench_with_input(BenchmarkId::new("lines", segments), segments, |b, &s| {
            b.iter(|| {
                let mut path = Path::new();
                path.move_to(Point::new(0.0, 0.0));
                for i in 0..s {
                    path.line_to(Point::new(i as f32, (i * 2) as f32));
                }
                black_box(path)
            })
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_path_creation,
    bench_path_building,
    bench_path_shapes,
    bench_path_operations,
    bench_path_complexity,
);

criterion_main!(benches);
