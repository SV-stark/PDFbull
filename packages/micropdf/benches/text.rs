use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use micropdf::fitz::font::Font;
use micropdf::fitz::geometry::Matrix;
use micropdf::fitz::text::{BidiDirection, Text, TextLanguage};
use std::sync::Arc;

fn bench_text_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("text/creation");

    group.bench_function("new", |b| b.iter(Text::new));

    group.finish();
}

fn bench_text_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("text/operations");

    // Create a simple font for testing
    let font = Arc::new(Font::new("Test"));

    group.bench_function("show_glyph", |b| {
        let mut text = Text::new();
        let trm = Matrix::IDENTITY;
        b.iter(|| {
            black_box(&mut text).show_glyph(
                black_box(Arc::clone(&font)),
                black_box(trm),
                black_box(0),  // glyph id
                black_box(65), // 'A' unicode
                black_box(false),
                black_box(0),
                black_box(BidiDirection::Ltr),
                black_box(TextLanguage::Unset),
            );
        })
    });

    group.bench_function("show_string", |b| {
        let mut text = Text::new();
        let trm = Matrix::IDENTITY;
        b.iter(|| {
            black_box(&mut text).show_string(
                black_box(Arc::clone(&font)),
                black_box(trm),
                black_box("Hello"),
                black_box(false),
                black_box(0),
                black_box(BidiDirection::Ltr),
                black_box(TextLanguage::Unset),
            );
        })
    });

    group.finish();
}

fn bench_text_properties(c: &mut Criterion) {
    let mut group = c.benchmark_group("text/properties");

    let font = Arc::new(Font::new("Test"));
    let mut text = Text::new();
    let trm = Matrix::IDENTITY;

    // Add some content
    text.show_string(
        Arc::clone(&font),
        trm,
        "Hello World",
        false,
        0,
        BidiDirection::Ltr,
        TextLanguage::Unset,
    );

    group.bench_function("is_empty", |b| b.iter(|| black_box(&text).is_empty()));

    group.bench_function("len", |b| b.iter(|| black_box(&text).len()));

    group.bench_function("span_count", |b| b.iter(|| black_box(&text).span_count()));

    group.bench_function("item_count", |b| b.iter(|| black_box(&text).item_count()));

    group.finish();
}

fn bench_text_bounds(c: &mut Criterion) {
    let mut group = c.benchmark_group("text/bounds");

    let font = Arc::new(Font::new("Test"));
    let mut text = Text::new();
    let trm = Matrix::IDENTITY;
    text.show_string(
        Arc::clone(&font),
        trm,
        "Hello World",
        false,
        0,
        BidiDirection::Ltr,
        TextLanguage::Unset,
    );

    let matrix = Matrix::IDENTITY;

    group.bench_function("bounds", |b| {
        b.iter(|| black_box(&text).bounds(black_box(None), black_box(&matrix)))
    });

    group.finish();
}

fn bench_text_string_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("text/string_sizes");

    let font = Arc::new(Font::new("Test"));
    let trm = Matrix::IDENTITY;

    let strings = [
        ("short", "Hi"),
        ("medium", "Hello World"),
        ("long", "The quick brown fox jumps over the lazy dog"),
        (
            "very_long",
            "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua",
        ),
    ];

    for (name, string) in strings.iter() {
        group.bench_with_input(BenchmarkId::new("show_string", name), string, |b, s| {
            let mut text = Text::new();
            b.iter(|| {
                black_box(&mut text).show_string(
                    black_box(Arc::clone(&font)),
                    black_box(trm),
                    black_box(s),
                    black_box(false),
                    black_box(0),
                    black_box(BidiDirection::Ltr),
                    black_box(TextLanguage::Unset),
                );
            })
        });
    }

    group.finish();
}

fn bench_text_clone(c: &mut Criterion) {
    let mut group = c.benchmark_group("text/clone");

    let font = Arc::new(Font::new("Test"));
    let mut text = Text::new();
    let trm = Matrix::IDENTITY;

    // Create text with varying complexity
    for _ in 0..10 {
        text.show_string(
            Arc::clone(&font),
            trm,
            "Hello World",
            false,
            0,
            BidiDirection::Ltr,
            TextLanguage::Unset,
        );
    }

    group.bench_function("clone", |b| b.iter(|| black_box(&text).clone()));

    group.finish();
}

criterion_group!(
    benches,
    bench_text_creation,
    bench_text_operations,
    bench_text_properties,
    bench_text_bounds,
    bench_text_string_sizes,
    bench_text_clone,
);

criterion_main!(benches);
