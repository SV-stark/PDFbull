use criterion::{Criterion, black_box, criterion_group, criterion_main};
use micropdf::fitz::output::Output;
use std::path::Path;

fn bench_output_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("output/creation");

    let test_path = Path::new("/tmp/test_output.bin");

    group.bench_function("from_path", |b| {
        b.iter(|| Output::from_path(black_box(test_path), black_box(false)).ok())
    });

    group.finish();
}

criterion_group!(benches, bench_output_creation,);

criterion_main!(benches);
