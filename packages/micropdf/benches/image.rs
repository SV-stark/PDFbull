use criterion::{Criterion, black_box, criterion_group, criterion_main};
use micropdf::fitz::colorspace::Colorspace;
use micropdf::fitz::image::Image;
use micropdf::fitz::pixmap::Pixmap;

fn bench_image_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("image/creation");

    group.bench_function("new_simple", |b| {
        b.iter(|| Image::new(black_box(100), black_box(100), black_box(None)))
    });

    group.bench_function("new_with_pixmap", |b| {
        let pixmap = Pixmap::new(Some(Colorspace::device_rgb()), 100, 100, false).unwrap();
        b.iter(|| {
            Image::new(
                black_box(100),
                black_box(100),
                black_box(Some(pixmap.clone())),
            )
        })
    });

    group.finish();
}

fn bench_image_properties(c: &mut Criterion) {
    let mut group = c.benchmark_group("image/properties");

    let img = Image::new(100, 100, None);

    group.bench_function("width", |b| b.iter(|| black_box(&img).width()));

    group.bench_function("height", |b| b.iter(|| black_box(&img).height()));

    group.finish();
}

criterion_group!(benches, bench_image_creation, bench_image_properties,);

criterion_main!(benches);
