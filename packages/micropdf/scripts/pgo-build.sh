#!/bin/bash
# Profile-Guided Optimization (PGO) Build Script
#
# PGO improves performance by:
# 1. Instrumenting the code to collect runtime data
# 2. Running representative workloads to generate profiles
# 3. Recompiling with profile data to optimize hot paths
#
# Usage:
#   ./scripts/pgo-build.sh           # Full PGO build
#   ./scripts/pgo-build.sh generate  # Only generate profiles
#   ./scripts/pgo-build.sh build     # Only build with existing profiles
#   ./scripts/pgo-build.sh clean     # Clean profile data

set -euo pipefail

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
PROFILE_DIR="$PROJECT_DIR/target/pgo-profiles"
MERGED_PROFILE="$PROJECT_DIR/target/pgo-merged.profdata"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check for required tools
check_requirements() {
    log_info "Checking requirements..."

    if ! command -v rustc &> /dev/null; then
        log_error "rustc not found. Please install Rust."
        exit 1
    fi

    if ! command -v cargo &> /dev/null; then
        log_error "cargo not found. Please install Rust."
        exit 1
    fi

    # Check for llvm-profdata (needed to merge profiles)
    if ! command -v llvm-profdata &> /dev/null; then
        # Try rustup's version
        LLVM_PROFDATA=$(rustc --print sysroot)/lib/rustlib/$(rustc -vV | grep host | cut -d' ' -f2)/bin/llvm-profdata
        if [[ ! -x "$LLVM_PROFDATA" ]]; then
            log_warn "llvm-profdata not found. Install LLVM tools or use rustup component add llvm-tools-preview"
            log_warn "Continuing without profile merging..."
            LLVM_PROFDATA=""
        fi
    else
        LLVM_PROFDATA="llvm-profdata"
    fi
}

# Clean profile data
clean_profiles() {
    log_info "Cleaning profile data..."
    rm -rf "$PROFILE_DIR"
    rm -f "$MERGED_PROFILE"
    rm -rf "$PROJECT_DIR/target/release-pgo-generate"
    rm -rf "$PROJECT_DIR/target/release-pgo-use"
    log_info "Profile data cleaned."
}

# Step 1: Build with instrumentation
build_instrumented() {
    log_info "Building instrumented binary..."

    # Create profile directory
    mkdir -p "$PROFILE_DIR"

    # Build with PGO instrumentation
    cd "$PROJECT_DIR"
    RUSTFLAGS="-Cprofile-generate=$PROFILE_DIR" \
        cargo build --profile release-pgo-generate --lib

    log_info "Instrumented binary built."
}

# Step 2: Run representative workloads to generate profiles
generate_profiles() {
    log_info "Generating profile data by running benchmarks..."

    # Run benchmarks (they exercise hot paths)
    cd "$PROJECT_DIR"
    RUSTFLAGS="-Cprofile-generate=$PROFILE_DIR" \
        cargo bench --profile release-pgo-generate -- --noplot 2>/dev/null || true

    # Run tests (exercises more code paths)
    log_info "Running tests to generate additional profile data..."
    RUSTFLAGS="-Cprofile-generate=$PROFILE_DIR" \
        cargo test --profile release-pgo-generate --lib 2>/dev/null || true

    # Check if profiles were generated
    PROFILE_COUNT=$(find "$PROFILE_DIR" -name "*.profraw" 2>/dev/null | wc -l)
    if [[ "$PROFILE_COUNT" -eq 0 ]]; then
        log_error "No profile data generated. Check that benchmarks/tests ran correctly."
        exit 1
    fi

    log_info "Generated $PROFILE_COUNT profile files."
}

# Step 3: Merge profiles
merge_profiles() {
    if [[ -z "${LLVM_PROFDATA:-}" ]]; then
        log_warn "Skipping profile merge (llvm-profdata not available)."
        log_warn "Using raw profile directory instead."
        return
    fi

    log_info "Merging profile data..."

    "$LLVM_PROFDATA" merge -o "$MERGED_PROFILE" "$PROFILE_DIR"/*.profraw

    log_info "Profile data merged to $MERGED_PROFILE"
}

# Step 4: Build optimized binary using profiles
build_optimized() {
    log_info "Building optimized binary with PGO..."

    cd "$PROJECT_DIR"

    # Use merged profile if available, otherwise use profile directory
    if [[ -f "$MERGED_PROFILE" ]]; then
        PROFILE_PATH="$MERGED_PROFILE"
    else
        PROFILE_PATH="$PROFILE_DIR"
    fi

    RUSTFLAGS="-Cprofile-use=$PROFILE_PATH -Cllvm-args=-pgo-warn-missing-function" \
        cargo build --profile release-pgo-use --lib

    log_info "PGO-optimized binary built!"
}

# Report results
report_results() {
    log_info "PGO Build Complete!"
    echo ""
    echo "Optimized library location:"
    echo "  $PROJECT_DIR/target/release-pgo-use/libmicropdf.rlib"
    echo "  $PROJECT_DIR/target/release-pgo-use/libmicropdf.a"
    echo "  $PROJECT_DIR/target/release-pgo-use/libmicropdf.so (if cdylib)"
    echo ""
    echo "To use in production, copy the library or use:"
    echo "  cargo build --profile release-pgo-use"
    echo ""
    echo "Expected improvements:"
    echo "  - 10-20% faster hot paths"
    echo "  - Better branch prediction"
    echo "  - Optimized function inlining"
}

# Main function
main() {
    local command="${1:-all}"

    check_requirements

    case "$command" in
        clean)
            clean_profiles
            ;;
        generate)
            build_instrumented
            generate_profiles
            merge_profiles
            ;;
        build)
            if [[ ! -d "$PROFILE_DIR" ]] && [[ ! -f "$MERGED_PROFILE" ]]; then
                log_error "No profile data found. Run 'generate' first."
                exit 1
            fi
            build_optimized
            report_results
            ;;
        all|"")
            clean_profiles
            build_instrumented
            generate_profiles
            merge_profiles
            build_optimized
            report_results
            ;;
        *)
            echo "Usage: $0 [clean|generate|build|all]"
            echo ""
            echo "Commands:"
            echo "  clean    - Remove profile data"
            echo "  generate - Build instrumented binary and generate profiles"
            echo "  build    - Build optimized binary using existing profiles"
            echo "  all      - Full PGO build (default)"
            exit 1
            ;;
    esac
}

main "$@"

