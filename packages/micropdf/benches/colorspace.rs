use criterion::{Criterion, black_box, criterion_group, criterion_main};
use micropdf::fitz::colorspace::Colorspace;

fn bench_colorspace_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("colorspace/creation");

    group.bench_function("device_gray", |b| b.iter(Colorspace::device_gray));

    group.bench_function("device_rgb", |b| b.iter(Colorspace::device_rgb));

    group.bench_function("device_cmyk", |b| b.iter(Colorspace::device_cmyk));

    group.finish();
}

fn bench_colorspace_properties(c: &mut Criterion) {
    let mut group = c.benchmark_group("colorspace/properties");

    let cs_gray = Colorspace::device_gray();
    let cs_rgb = Colorspace::device_rgb();
    let cs_cmyk = Colorspace::device_cmyk();

    group.bench_function("n_gray", |b| b.iter(|| black_box(&cs_gray).n()));

    group.bench_function("n_rgb", |b| b.iter(|| black_box(&cs_rgb).n()));

    group.bench_function("n_cmyk", |b| b.iter(|| black_box(&cs_cmyk).n()));

    group.finish();
}

fn bench_colorspace_clone(c: &mut Criterion) {
    let mut group = c.benchmark_group("colorspace/clone");

    let cs_gray = Colorspace::device_gray();
    let cs_rgb = Colorspace::device_rgb();
    let cs_cmyk = Colorspace::device_cmyk();

    group.bench_function("clone_gray", |b| b.iter(|| black_box(&cs_gray).clone()));

    group.bench_function("clone_rgb", |b| b.iter(|| black_box(&cs_rgb).clone()));

    group.bench_function("clone_cmyk", |b| b.iter(|| black_box(&cs_cmyk).clone()));

    group.finish();
}

criterion_group!(
    benches,
    bench_colorspace_creation,
    bench_colorspace_properties,
    bench_colorspace_clone,
);

criterion_main!(benches);
