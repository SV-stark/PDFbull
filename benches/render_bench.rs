use divan;

fn main() {
    divan::main();
}

#[divan::bench]
fn simple_benchmark() {
    // Placeholder for future PDFium or logic benchmarks
    // For example, measuring rendering time for different PDF pages
    let _a = (0..100).sum::<u32>();
}
