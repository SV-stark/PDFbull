use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use micropdf::fitz::colorspace::Colorspace;
use micropdf::fitz::pixmap::Pixmap;

fn bench_pixmap_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("pixmap/creation");

    group.bench_function("new_rgb", |b| {
        b.iter(|| {
            Pixmap::new(
                black_box(Some(Colorspace::device_rgb())),
                black_box(100),
                black_box(100),
                black_box(false),
            )
        })
    });

    group.bench_function("new_rgba", |b| {
        b.iter(|| {
            Pixmap::new(
                black_box(Some(Colorspace::device_rgb())),
                black_box(100),
                black_box(100),
                black_box(true),
            )
        })
    });

    group.bench_function("new_gray", |b| {
        b.iter(|| {
            Pixmap::new(
                black_box(Some(Colorspace::device_gray())),
                black_box(100),
                black_box(100),
                black_box(false),
            )
        })
    });

    group.bench_function("new_cmyk", |b| {
        b.iter(|| {
            Pixmap::new(
                black_box(Some(Colorspace::device_cmyk())),
                black_box(100),
                black_box(100),
                black_box(false),
            )
        })
    });

    group.finish();
}

fn bench_pixmap_access(c: &mut Criterion) {
    let mut group = c.benchmark_group("pixmap/access");

    let pixmap = Pixmap::new(Some(Colorspace::device_rgb()), 100, 100, false).unwrap();

    group.bench_function("width", |b| b.iter(|| black_box(&pixmap).width()));

    group.bench_function("height", |b| b.iter(|| black_box(&pixmap).height()));

    group.bench_function("n", |b| b.iter(|| black_box(&pixmap).n()));

    group.bench_function("stride", |b| b.iter(|| black_box(&pixmap).stride()));

    group.bench_function("has_alpha", |b| b.iter(|| black_box(&pixmap).has_alpha()));

    group.bench_function("samples", |b| b.iter(|| black_box(&pixmap).samples()));

    group.bench_function("get_pixel", |b| {
        b.iter(|| black_box(&pixmap).get_pixel(black_box(50), black_box(50)))
    });

    group.finish();
}

fn bench_pixmap_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("pixmap/operations");

    group.bench_function("clear", |b| {
        let mut pixmap = Pixmap::new(Some(Colorspace::device_rgb()), 100, 100, false).unwrap();
        b.iter(|| {
            black_box(&mut pixmap).clear(black_box(255));
        })
    });

    group.bench_function("clone", |b| {
        let pixmap = Pixmap::new(Some(Colorspace::device_rgb()), 100, 100, false).unwrap();
        b.iter(|| black_box(&pixmap).clone())
    });

    group.finish();
}

fn bench_pixmap_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("pixmap/sizes");

    for size in [10, 50, 100, 200, 500].iter() {
        group.bench_with_input(BenchmarkId::new("creation", size), size, |b, &s| {
            b.iter(|| {
                Pixmap::new(
                    black_box(Some(Colorspace::device_rgb())),
                    black_box(s),
                    black_box(s),
                    black_box(false),
                )
            })
        });
    }

    group.finish();
}

fn bench_pixmap_pixel_iteration(c: &mut Criterion) {
    let mut group = c.benchmark_group("pixmap/iteration");

    let pixmap = Pixmap::new(Some(Colorspace::device_rgb()), 100, 100, false).unwrap();

    group.bench_function("iterate_all_pixels", |b| {
        b.iter(|| {
            let width = pixmap.width();
            let height = pixmap.height();
            for y in 0..height {
                for x in 0..width {
                    black_box(pixmap.get_pixel(x, y));
                }
            }
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_pixmap_creation,
    bench_pixmap_access,
    bench_pixmap_operations,
    bench_pixmap_sizes,
    bench_pixmap_pixel_iteration,
);

criterion_main!(benches);
