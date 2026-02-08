# MicroPDF Benchmarks

Comprehensive performance benchmarks for all core micropdf features.

## 📊 Viewing Benchmark Results

### Online Dashboard (GitHub Pages)

Live benchmark results are automatically published to GitHub Pages:

**🔗 [View Live Benchmarks](https://lexmata.github.io/micropdf/dev/bench/)**

The dashboard includes:
- 📈 Historical performance trends
- 🔄 Commit-by-commit comparisons
- ⚠️ Performance regression alerts
- 📉 Interactive charts

Results are updated automatically on every push to `main`.

### Criterion HTML Reports

After running benchmarks locally, detailed HTML reports are available at:

```
micropdf-rs/target/criterion/report/index.html
```

Open this file in your browser to see:
- Individual benchmark results
- Statistical analysis
- Violin plots
- Comparison charts

## 🚀 Running Benchmarks Locally

### Run All Benchmarks

```bash
cd micropdf-rs
cargo bench --all-features
```

### Run Specific Benchmark Suite

```bash
# Run only geometry benchmarks
cargo bench --bench geometry

# Run only image benchmarks
cargo bench --bench image

# Run specific benchmark within a suite
cargo bench --bench path -- "path/operations"
```

### Save Baseline for Comparison

```bash
# Save current results as baseline
cargo bench --all-features -- --save-baseline main

# Make changes to code...

# Compare against baseline
cargo bench --all-features -- --baseline main
```

### Benchmark Options

```bash
# Quick run (fewer samples, faster)
cargo bench --all-features -- --quick

# Verbose output
cargo bench --all-features -- --verbose

# Save specific baseline
cargo bench --all-features -- --save-baseline my-feature

# Compare two baselines
cargo bench --all-features -- --load-baseline main --baseline my-feature
```

## 📦 Benchmark Coverage

### Core Graphics (4 suites)
- ✅ **geometry** - Matrix, Point, Rect, Quad operations
- ✅ **path** - Path construction, transformations, curves
- ✅ **device** - Device operations, rendering
- ✅ **pixmap** - Pixel operations, conversions, blending

### Text & Fonts (2 suites)
- ✅ **font** - Font loading, metrics, glyph operations
- ✅ **text** - Text layout, rendering, spans

### Images & Colors (3 suites)
- ✅ **image** - Image decoding, scaling, format detection
- ✅ **pixmap** - Color space operations (included above)
- ✅ **colorspace** - Color space conversions (RGB, CMYK, Gray)

### I/O & Streams (3 suites)
- ✅ **buffer** - Buffer operations, resizing, appending
- ✅ **stream** - Stream I/O operations
- ✅ **output** - Output stream writing, seeking

### PDF Features (3 suites)
- ✅ **pdf_objects** - PDF object operations
- ✅ **filters** - PDF filter encode/decode (Flate, LZW, etc.)
- ✅ **archive** - ZIP/TAR archive parsing and extraction

### Total: **15 Benchmark Suites** covering **~150+ individual benchmarks**

## 🎯 Benchmark Design Principles

Each benchmark suite includes:

1. **Creation/Initialization** - Object construction overhead
2. **Common Operations** - Frequently used methods
3. **Scale Variations** - Small, medium, large datasets
4. **Realistic Workflows** - Real-world usage patterns
5. **Edge Cases** - Boundary conditions

## 📈 Performance Tracking

### Continuous Integration

Benchmarks run automatically on:
- ✅ Every push to `main` branch
- ✅ Every pull request
- ✅ Manual workflow dispatch

### PR Benchmark Comparison

Pull requests automatically get benchmark comparison comments showing:
- Performance changes vs. base branch
- Significant regressions highlighted
- Detailed metrics per benchmark

### Alerts

Performance regressions > 20% trigger:
- ⚠️ GitHub Action alerts
- 💬 PR comment notifications
- 📊 Visual indicators on dashboard

## 🔧 Criterion Configuration

Benchmarks use Criterion.rs with:
- **Warm-up time**: 3 seconds
- **Measurement time**: 5 seconds
- **Sample size**: 100 iterations
- **Significance level**: 0.05 (5%)
- **Noise threshold**: 0.02 (2%)

Configuration can be customized per benchmark as needed.

## 📝 Adding New Benchmarks

To add a new benchmark suite:

1. Create `benches/my_feature.rs`:

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use micropdf::fitz::my_feature::MyFeature;

fn bench_my_feature(c: &mut Criterion) {
    let mut group = c.benchmark_group("my_feature");

    group.bench_function("operation", |b| {
        let feature = MyFeature::new();
        b.iter(|| {
            black_box(&feature).do_something(black_box(42))
        })
    });

    group.finish();
}

criterion_group!(benches, bench_my_feature);
criterion_main!(benches);
```

2. Run the new benchmark:

```bash
cargo bench --bench my_feature
```

3. Results will automatically be included in the next CI run.

## 🎨 Benchmark Best Practices

### DO ✅

- Use `black_box()` to prevent compiler optimizations
- Group related benchmarks together
- Test multiple input sizes
- Include realistic scenarios
- Warm up expensive operations
- Document what each benchmark measures

### DON'T ❌

- Benchmark I/O operations without mocking
- Include setup/teardown in measurement
- Use random data without seeding
- Benchmark trivial operations in isolation
- Ignore outliers without investigation

## 📚 Resources

- [Criterion.rs Documentation](https://bheisler.github.io/criterion.rs/book/)
- [Rust Performance Book](https://nnethercote.github.io/perf-book/)
- [GitHub Action Benchmark](https://github.com/benchmark-action/github-action-benchmark)

## 🤝 Contributing

When contributing code that affects performance:

1. Run benchmarks before and after changes
2. Document significant performance impacts in PR
3. Include benchmark results in commit message if relevant
4. Consider adding new benchmarks for new features

---

**Questions?** Check the [main README](../README.md) or open an issue.

