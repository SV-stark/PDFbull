use criterion::{Criterion, black_box, criterion_group, criterion_main};
use micropdf::fitz::font::Font;

fn bench_font_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("font/creation");

    group.bench_function("new", |b| b.iter(|| Font::new(black_box("Test Font"))));

    group.bench_function("new_bold", |b| {
        b.iter(|| Font::new(black_box("Test Font Bold")))
    });

    group.bench_function("new_italic", |b| {
        b.iter(|| Font::new(black_box("Test Font Italic")))
    });

    group.finish();
}

fn bench_font_properties(c: &mut Criterion) {
    let mut group = c.benchmark_group("font/properties");

    let font = Font::new("Test Font");

    group.bench_function("name", |b| b.iter(|| black_box(&font).name()));

    group.finish();
}

fn bench_font_metrics(c: &mut Criterion) {
    let mut group = c.benchmark_group("font/metrics");

    let font = Font::new("Test Font");

    group.bench_function("bbox", |b| b.iter(|| black_box(&font).bbox()));

    group.finish();
}

fn bench_font_clone(c: &mut Criterion) {
    let mut group = c.benchmark_group("font/clone");

    let font = Font::new("Test Font");

    group.bench_function("clone", |b| b.iter(|| black_box(&font).clone()));

    group.finish();
}

criterion_group!(
    benches,
    bench_font_creation,
    bench_font_properties,
    bench_font_metrics,
    bench_font_clone,
);

criterion_main!(benches);
