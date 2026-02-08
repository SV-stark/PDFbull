use criterion::{Criterion, black_box, criterion_group, criterion_main};
use micropdf::fitz::colorspace::Colorspace;
use micropdf::fitz::device::{BBoxDevice, Device, TraceDevice};
use micropdf::fitz::geometry::{Matrix, Point};
use micropdf::fitz::path::Path;

fn bench_device_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("device/creation");

    group.bench_function("bbox", |b| b.iter(BBoxDevice::new));

    group.bench_function("trace", |b| b.iter(TraceDevice::new));

    group.finish();
}

fn bench_device_path_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("device/path");

    let mut path = Path::new();
    path.move_to(Point::new(0.0, 0.0));
    path.line_to(Point::new(100.0, 0.0));
    path.line_to(Point::new(100.0, 100.0));
    path.line_to(Point::new(0.0, 100.0));
    path.close();

    let ctm = Matrix::IDENTITY;
    let colorspace = Colorspace::device_rgb();
    let color = &[0.0, 0.0, 0.0];

    group.bench_function("fill_path_bbox", |b| {
        let mut device = BBoxDevice::new();
        b.iter(|| {
            black_box(&mut device).fill_path(
                black_box(&path),
                black_box(false),
                black_box(&ctm),
                black_box(&colorspace),
                black_box(color),
                black_box(1.0),
            );
        })
    });

    group.bench_function("fill_path_trace", |b| {
        let mut device = TraceDevice::new();
        b.iter(|| {
            black_box(&mut device).fill_path(
                black_box(&path),
                black_box(false),
                black_box(&ctm),
                black_box(&colorspace),
                black_box(color),
                black_box(1.0),
            );
        })
    });

    group.finish();
}

criterion_group!(benches, bench_device_creation, bench_device_path_operations,);

criterion_main!(benches);
