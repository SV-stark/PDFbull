use criterion::{Criterion, black_box, criterion_group, criterion_main};
use micropdf::fitz::stream::Stream;
use std::path::Path;

fn bench_stream_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("stream/creation");

    // Create a temporary test file
    let test_path = Path::new("Cargo.toml");

    group.bench_function("open_file", |b| {
        b.iter(|| Stream::open_file(black_box(test_path)).ok())
    });

    group.finish();
}

criterion_group!(benches, bench_stream_creation,);

criterion_main!(benches);
