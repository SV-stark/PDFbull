use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use micropdf::pdf::filter::*;

fn bench_flate_filter(c: &mut Criterion) {
    let mut group = c.benchmark_group("filter/flate");

    for size in [100, 1024, 10240, 102400].iter() {
        let data = vec![42u8; *size]; // Compressible pattern

        group.bench_with_input(BenchmarkId::new("encode", size), size, |b, _| {
            b.iter(|| encode_flate(black_box(&data), black_box(6)).ok())
        });

        let compressed = encode_flate(&data, 6).unwrap();

        group.bench_with_input(BenchmarkId::new("decode", size), size, |b, _| {
            b.iter(|| decode_flate(black_box(&compressed), black_box(None)).ok())
        });
    }

    group.finish();
}

fn bench_ascii85_filter(c: &mut Criterion) {
    let mut group = c.benchmark_group("filter/ascii85");

    for size in [100, 1024, 10240].iter() {
        let data = vec![42u8; *size];

        group.bench_with_input(BenchmarkId::new("encode", size), size, |b, _| {
            b.iter(|| encode_ascii85(black_box(&data)).ok())
        });

        let encoded = encode_ascii85(&data).unwrap();

        group.bench_with_input(BenchmarkId::new("decode", size), size, |b, _| {
            b.iter(|| decode_ascii85(black_box(&encoded)).ok())
        });
    }

    group.finish();
}

fn bench_asciihex_filter(c: &mut Criterion) {
    let mut group = c.benchmark_group("filter/asciihex");

    for size in [100, 1024, 10240].iter() {
        let data = vec![42u8; *size];

        group.bench_with_input(BenchmarkId::new("encode", size), size, |b, _| {
            b.iter(|| encode_ascii_hex(black_box(&data)).ok())
        });

        let encoded = encode_ascii_hex(&data).unwrap();

        group.bench_with_input(BenchmarkId::new("decode", size), size, |b, _| {
            b.iter(|| decode_ascii_hex(black_box(&encoded)).ok())
        });
    }

    group.finish();
}

fn bench_runlength_filter(c: &mut Criterion) {
    let mut group = c.benchmark_group("filter/runlength");

    for size in [100, 1024, 10240].iter() {
        let data = vec![42u8; *size]; // Highly compressible run

        group.bench_with_input(BenchmarkId::new("encode", size), size, |b, _| {
            b.iter(|| encode_run_length(black_box(&data)).ok())
        });

        let encoded = encode_run_length(&data).unwrap();

        group.bench_with_input(BenchmarkId::new("decode", size), size, |b, _| {
            b.iter(|| decode_run_length(black_box(&encoded)).ok())
        });
    }

    group.finish();
}

fn bench_lzw_filter(c: &mut Criterion) {
    let mut group = c.benchmark_group("filter/lzw");

    for size in [100, 1024, 10240].iter() {
        let data = vec![42u8; *size];

        group.bench_with_input(BenchmarkId::new("encode", size), size, |b, _| {
            b.iter(|| encode_lzw(black_box(&data)).ok())
        });

        let encoded = encode_lzw(&data).unwrap();

        group.bench_with_input(BenchmarkId::new("decode", size), size, |b, _| {
            b.iter(|| decode_lzw(black_box(&encoded), black_box(None)).ok())
        });
    }

    group.finish();
}

fn bench_filter_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("filter/comparison");

    let data = vec![42u8; 1024];

    group.bench_function("flate", |b| {
        b.iter(|| {
            let encoded = encode_flate(black_box(&data), 6).unwrap();
            decode_flate(black_box(&encoded), None).ok()
        })
    });

    group.bench_function("ascii85", |b| {
        b.iter(|| {
            let encoded = encode_ascii85(black_box(&data)).unwrap();
            decode_ascii85(black_box(&encoded)).ok()
        })
    });

    group.bench_function("asciihex", |b| {
        b.iter(|| {
            let encoded = encode_ascii_hex(black_box(&data)).unwrap();
            decode_ascii_hex(black_box(&encoded)).ok()
        })
    });

    group.bench_function("runlength", |b| {
        b.iter(|| {
            let encoded = encode_run_length(black_box(&data)).unwrap();
            decode_run_length(black_box(&encoded)).ok()
        })
    });

    group.bench_function("lzw", |b| {
        b.iter(|| {
            let encoded = encode_lzw(black_box(&data)).unwrap();
            decode_lzw(black_box(&encoded), None).ok()
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_flate_filter,
    bench_ascii85_filter,
    bench_asciihex_filter,
    bench_runlength_filter,
    bench_lzw_filter,
    bench_filter_comparison,
);

criterion_main!(benches);
