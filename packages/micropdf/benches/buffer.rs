use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use micropdf::fitz::buffer::Buffer;

fn bench_buffer_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("buffer/create");

    for size in [64, 256, 1024, 4096, 16384].iter() {
        group.throughput(Throughput::Bytes(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter(|| Buffer::new(black_box(size)))
        });
    }

    group.finish();
}

fn bench_buffer_from_data(c: &mut Criterion) {
    let mut group = c.benchmark_group("buffer/from_slice");

    for size in [64, 256, 1024, 4096, 16384].iter() {
        let data: Vec<u8> = (0..*size).map(|i| i as u8).collect();
        group.throughput(Throughput::Bytes(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &data, |b, data| {
            b.iter(|| Buffer::from_slice(black_box(data)))
        });
    }

    group.finish();
}

fn bench_buffer_append(c: &mut Criterion) {
    let chunk: Vec<u8> = (0..256).map(|i| i as u8).collect();

    let mut group = c.benchmark_group("buffer/append");

    for iterations in [10, 100, 1000].iter() {
        group.throughput(Throughput::Bytes((256 * iterations) as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(iterations),
            iterations,
            |b, &iterations| {
                b.iter(|| {
                    let mut buf = Buffer::new(0);
                    for _ in 0..iterations {
                        buf.append_data(black_box(&chunk));
                    }
                    buf
                })
            },
        );
    }

    group.finish();
}

fn bench_buffer_base64(c: &mut Criterion) {
    let mut group = c.benchmark_group("buffer/base64");

    for size in [64, 256, 1024, 4096].iter() {
        let data: Vec<u8> = (0..*size).map(|i| i as u8).collect();
        let buf = Buffer::from_slice(&data);

        group.throughput(Throughput::Bytes(*size as u64));
        group.bench_with_input(BenchmarkId::new("encode", size), &buf, |b, buf| {
            b.iter(|| buf.to_base64())
        });
    }

    // Decode benchmarks
    for size in [64, 256, 1024, 4096].iter() {
        let data: Vec<u8> = (0..*size).map(|i| i as u8).collect();
        let buf = Buffer::from_slice(&data);
        let encoded = buf.to_base64();

        group.throughput(Throughput::Bytes(*size as u64));
        group.bench_with_input(BenchmarkId::new("decode", size), &encoded, |b, encoded| {
            b.iter(|| Buffer::from_base64(black_box(encoded)))
        });
    }

    group.finish();
}

fn bench_buffer_md5(c: &mut Criterion) {
    let mut group = c.benchmark_group("buffer/md5");

    for size in [64, 256, 1024, 4096, 16384].iter() {
        let data: Vec<u8> = (0..*size).map(|i| i as u8).collect();
        let buf = Buffer::from_slice(&data);

        group.throughput(Throughput::Bytes(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &buf, |b, buf| {
            b.iter(|| buf.md5_digest())
        });
    }

    group.finish();
}

fn bench_buffer_to_vec(c: &mut Criterion) {
    let mut group = c.benchmark_group("buffer/to_vec");

    for size in [64, 256, 1024, 4096, 16384].iter() {
        let data: Vec<u8> = (0..*size).map(|i| i as u8).collect();
        let buf = Buffer::from_slice(&data);

        group.throughput(Throughput::Bytes(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &buf, |b, buf| {
            b.iter(|| buf.to_vec())
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_buffer_creation,
    bench_buffer_from_data,
    bench_buffer_append,
    bench_buffer_base64,
    bench_buffer_md5,
    bench_buffer_to_vec,
);

criterion_main!(benches);
