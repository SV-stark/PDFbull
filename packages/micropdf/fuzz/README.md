# MicroPDF Fuzzing

Fuzzing tests for MicroPDF using [cargo-fuzz](https://github.com/rust-fuzz/cargo-fuzz) (libFuzzer).

## Overview

Fuzzing is an automated testing technique that feeds random/malformed data to the library to discover bugs, crashes, and security vulnerabilities.

## Fuzz Targets

### 1. `fuzz_pdf_parse`
**What it tests**: PDF document parsing
**Seed corpus**: Real PDF files from `test-pdfs/`
**Finds**: Parse errors, crashes in document handling, memory issues

### 2. `fuzz_buffer`
**What it tests**: Buffer operations (create, append, read, clear)
**Seed corpus**: Text and binary data
**Finds**: Buffer overflow, underflow, memory leaks

### 3. `fuzz_stream`
**What it tests**: Stream I/O (read, seek, peek)
**Seed corpus**: Various data formats
**Finds**: Stream handling bugs, read errors

### 4. `fuzz_pdf_objects`
**What it tests**: PDF object model (dictionaries, arrays, references)
**Seed corpus**: Real PDF files
**Finds**: Object parsing bugs, type confusion, crashes

### 5. `fuzz_filters`
**What it tests**: PDF stream filters (FlateDecode, ASCII85, ASCIIHex, RLE)
**Seed corpus**: Compressed/encoded data
**Finds**: Decompression bugs, infinite loops, crashes

### 6. `fuzz_xref`
**What it tests**: PDF cross-reference table parsing and object resolution
**Seed corpus**: Real PDF files
**Finds**: Xref parsing bugs, circular references, invalid object numbers

### 7. `fuzz_page_render`
**What it tests**: Page loading, bounds calculation, rendering pipeline
**Seed corpus**: Real PDF files
**Finds**: Rendering crashes, memory issues, device bugs

### 8. `fuzz_annotations`
**What it tests**: PDF annotations (comments, links, forms)
**Seed corpus**: PDFs with annotations
**Finds**: Annotation parsing bugs, type handling issues

### 9. `fuzz_fonts`
**What it tests**: Font resource handling and embedding
**Seed corpus**: PDFs with various fonts
**Finds**: Font parsing bugs, embedding issues

## Installation

```bash
# Install cargo-fuzz
cargo install cargo-fuzz

# Or use rustup
rustup component add llvm-tools-preview
```

## Running Fuzzers

### Quick Start

```bash
# Run PDF parsing fuzzer for 60 seconds
cargo fuzz run fuzz_pdf_parse -- -max_total_time=60

# Run buffer fuzzer
cargo fuzz run fuzz_buffer -- -max_total_time=60

# Run stream fuzzer
cargo fuzz run fuzz_stream -- -max_total_time=60

# Run PDF objects fuzzer
cargo fuzz run fuzz_pdf_objects -- -max_total_time=60

# Run filters fuzzer
cargo fuzz run fuzz_filters -- -max_total_time=60
```

### Continuous Fuzzing

```bash
# Run until crash found or manually stopped
cargo fuzz run fuzz_pdf_parse

# Run with specific number of jobs (parallel)
cargo fuzz run fuzz_pdf_parse -- -jobs=4

# Run with custom timeout per input
cargo fuzz run fuzz_pdf_parse -- -timeout=5
```

### Advanced Options

```bash
# Limit RSS memory usage (MB)
cargo fuzz run fuzz_pdf_parse -- -rss_limit_mb=2048

# Limit input size (bytes)
cargo fuzz run fuzz_pdf_parse -- -max_len=1048576

# Run with coverage tracking
cargo fuzz coverage fuzz_pdf_parse

# Generate coverage report
cargo fuzz coverage fuzz_pdf_parse --html
```

## Reproducing Crashes

When a crash is found, it's saved to `fuzz/artifacts/`:

```bash
# Reproduce crash
cargo fuzz run fuzz_pdf_parse fuzz/artifacts/fuzz_pdf_parse/crash-1234567890abcdef

# Debug crash
cargo fuzz run --debug fuzz_pdf_parse fuzz/artifacts/fuzz_pdf_parse/crash-1234567890abcdef

# Run under debugger
rust-lldb target/x86_64-unknown-linux-gnu/release/fuzz_pdf_parse fuzz/artifacts/fuzz_pdf_parse/crash-1234567890abcdef
```

## Corpus Management

### Seed Corpus

Initial inputs to start fuzzing:

```
corpus/
├── fuzz_pdf_parse/     - Real PDF files (from test-pdfs/)
├── fuzz_buffer/        - Text and binary data
├── fuzz_stream/        - Various data formats
├── fuzz_pdf_objects/   - Real PDF files
└── fuzz_filters/       - Compressed/encoded data
```

### Adding Seeds

```bash
# Add custom PDF to corpus
cp my-test.pdf fuzz/corpus/fuzz_pdf_parse/

# Add malformed input
echo "%PDF-1.4\n%AAAA" > fuzz/corpus/fuzz_pdf_parse/malformed.pdf

# Fuzzer will discover new interesting inputs automatically
```

### Corpus Minimization

```bash
# Minimize corpus (remove redundant inputs)
cargo fuzz cmin fuzz_pdf_parse

# Minimize to specific size
cargo fuzz cmin fuzz_pdf_parse -- -max_total_time=300
```

## Integration with CI

### GitHub Actions

See `.github/workflows/fuzz.yml` for continuous fuzzing.

### OSS-Fuzz

MicroPDF can be integrated with [OSS-Fuzz](https://github.com/google/oss-fuzz) for continuous, large-scale fuzzing.

## Interpreting Results

### Coverage

```bash
# Generate coverage report
cargo fuzz coverage fuzz_pdf_parse

# View HTML report
cargo fuzz coverage fuzz_pdf_parse --html
firefox fuzz/coverage/fuzz_pdf_parse/index.html
```

### Statistics

Fuzzer output shows:
- `exec/s`: Executions per second (speed)
- `cov`: Code coverage (unique edges)
- `ft`: Feature (code path) coverage
- `corp`: Corpus size (unique inputs)

### Good Results

```
#1234567 NEW    cov: 15678 ft: 23456 corp: 123/45KB exec/s: 1000
```
- Found new coverage
- 15,678 edges covered
- 23,456 features
- 123 unique inputs
- 45 KB total corpus

### Crashes

```
==12345==ERROR: AddressSanitizer: heap-buffer-overflow
```
- Memory safety violation found
- Crash saved to `artifacts/`
- Review and fix the bug

## Performance Tips

### 1. **Use Release Mode**

Fuzzing always uses `--release` for speed.

### 2. **Parallel Fuzzing**

```bash
# Use multiple cores
cargo fuzz run fuzz_pdf_parse -- -jobs=8
```

### 3. **Limit Input Size**

```bash
# Smaller inputs = faster execution
cargo fuzz run fuzz_pdf_parse -- -max_len=10240
```

### 4. **Dictionary**

Create a dictionary of PDF keywords:

```bash
cat > fuzz/dict/pdf.dict << EOF
"PDF"
"obj"
"endobj"
"stream"
"endstream"
"<<"
">>"
EOF

# Use dictionary
cargo fuzz run fuzz_pdf_parse -- -dict=fuzz/dict/pdf.dict
```

## Troubleshooting

### Slow Fuzzing

- **Problem**: < 100 exec/s
- **Solution**:
  - Check if running in debug mode (should be release)
  - Reduce `-max_len` to limit input size
  - Profile the fuzz target

### Out of Memory

- **Problem**: Fuzzer killed due to OOM
- **Solution**:
  - Add `-rss_limit_mb=2048` to limit memory
  - Check for memory leaks in target
  - Minimize corpus

### No New Coverage

- **Problem**: Coverage plateaus quickly
- **Solution**:
  - Corpus may be sufficient
  - Add more diverse seeds
  - Try different fuzz target

### Timeout

- **Problem**: Inputs timeout (> 1s execution)
- **Solution**:
  - Add `-timeout=5` to increase timeout
  - Check for infinite loops
  - Limit input processing in target

## Security

### Address Sanitizer (ASan)

Enabled by default in cargo-fuzz. Detects:
- Heap buffer overflow/underflow
- Stack buffer overflow
- Use-after-free
- Double-free
- Memory leaks

### Undefined Behavior Sanitizer (UBSan)

```bash
# Run with UBSan
RUSTFLAGS="-Zsanitizer=undefined" cargo fuzz run fuzz_pdf_parse
```

Detects:
- Integer overflow
- Null pointer dereference
- Misaligned pointer

## Benchmarking

```bash
# Benchmark fuzz target speed
cargo fuzz run fuzz_pdf_parse -- -runs=10000 -max_total_time=10

# Compare performance
./scripts/fuzz-benchmark.sh
```

## References

- [cargo-fuzz Book](https://rust-fuzz.github.io/book/cargo-fuzz.html)
- [libFuzzer Documentation](https://llvm.org/docs/LibFuzzer.html)
- [Rust Fuzz Project](https://github.com/rust-fuzz)
- [OSS-Fuzz](https://github.com/google/oss-fuzz)

## Contributing

### Adding New Fuzz Targets

1. Create `fuzz/fuzz_targets/fuzz_myfeature.rs`
2. Add to `fuzz/Cargo.toml`
3. Create corpus directory
4. Add seeds to corpus
5. Test locally
6. Update this README

### Reporting Crashes

1. Save artifact file
2. Minimize crash input: `cargo fuzz tmin fuzz_pdf_parse artifacts/crash-xyz`
3. Create GitHub issue with:
   - Fuzz target name
   - Minimized input (attach file)
   - Stack trace
   - Environment (OS, Rust version)

---

**Happy Fuzzing!** 🐛🔨

