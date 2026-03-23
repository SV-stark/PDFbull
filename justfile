# PDFbull Project Tasks

# List all available commands
default:
    @just --list

# Run cargo check
check:
    cargo check

# Run all tests using nextest
test:
    cargo nextest run

# Run visual regression tests and open review if they fail
test-visual:
    cargo nextest run --test visual_regression
    cargo insta review

# Run clippy with all warnings enabled for the workspace
lint:
    cargo clippy --workspace -- -D warnings

# Format all code
fmt:
    cargo fmt

# Run benchmarks using divan
bench:
    cargo bench

# Clean build artifacts
clean:
    cargo clean

# Show git status (replaces git_status.bat)
status:
    git status

# Stage all changes and show a summary (replaces git_commands.bat)
stage:
    git add -A
    git status
    git diff --cached --stat

# Generate documentation
doc:
    cargo doc --no-deps --open

# Optimized release build
build-release:
    cargo build --release
