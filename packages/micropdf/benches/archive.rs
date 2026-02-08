use criterion::{Criterion, black_box, criterion_group, criterion_main};
use micropdf::fitz::archive::Archive;
use std::path::PathBuf;

fn bench_archive_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("archive/creation");

    // Create a temporary path for testing
    let test_path = PathBuf::from(".");

    group.bench_function("open_directory", |b| {
        b.iter(|| Archive::open(black_box(&test_path)).ok())
    });

    group.finish();
}

fn bench_archive_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("archive/operations");

    let test_path = PathBuf::from(".");
    if let Ok(archive) = Archive::open(&test_path) {
        group.bench_function("format", |b| b.iter(|| black_box(&archive).format()));

        group.bench_function("count_entries", |b| {
            b.iter(|| black_box(&archive).count_entries())
        });
    }

    group.finish();
}

criterion_group!(benches, bench_archive_creation, bench_archive_operations,);

criterion_main!(benches);
