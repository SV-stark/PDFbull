# Makefile for MicroPDF - Rust PDF library with C FFI
#
# This Makefile provides convenient targets for building and installing
# the MicroPDF library, headers, and pkg-config files.
#
# Common targets:
#   make              - Build the library in release mode
#   make install      - Install library, headers, and pkg-config files
#   make uninstall    - Remove installed files
#   make clean        - Clean build artifacts
#   make test         - Run tests
#   make bench        - Run benchmarks
#
# Installation paths can be customized with PREFIX:
#   make install PREFIX=/usr/local
#

# ============================================================================
# Configuration
# ============================================================================

# Version (extracted from Cargo.toml)
VERSION := $(shell grep "^version" Cargo.toml | cut -d '"' -f 2)

# Installation prefix
PREFIX ?= /usr/local
DESTDIR ?=

# Installation directories
LIBDIR := $(DESTDIR)$(PREFIX)/lib
INCLUDEDIR := $(DESTDIR)$(PREFIX)/include
PKGCONFIGDIR := $(DESTDIR)$(PREFIX)/lib/pkgconfig

# Build configuration
CARGO ?= cargo
CARGO_BUILD_FLAGS ?=
CARGO_RELEASE_FLAGS := --release
CARGO_TEST_FLAGS ?=
CARGO_BENCH_FLAGS ?=

# Target architecture (for multi-arch support)
TARGET_ARCH := $(shell uname -m)
ifeq ($(TARGET_ARCH),x86_64)
    TARGET_TRIPLE ?= x86_64-unknown-linux-gnu
else ifeq ($(TARGET_ARCH),aarch64)
    TARGET_TRIPLE ?= aarch64-unknown-linux-gnu
else ifeq ($(TARGET_ARCH),arm64)
    TARGET_TRIPLE ?= aarch64-unknown-linux-gnu
else
    TARGET_TRIPLE ?= $(TARGET_ARCH)-unknown-linux-gnu
endif

# Output paths
BUILD_DIR := target/release
DEBUG_DIR := target/debug
LIB_NAME := libmicropdf.a
LIB_PATH := $(BUILD_DIR)/$(LIB_NAME)

# Header files
HEADERS := $(shell find include -type f -name "*.h")

# Colors for pretty output
COLOR_RESET := \033[0m
COLOR_BOLD := \033[1m
COLOR_GREEN := \033[32m
COLOR_BLUE := \033[34m
COLOR_YELLOW := \033[33m

# ============================================================================
# Default target
# ============================================================================

.PHONY: all
all: release

# ============================================================================
# Build targets
# ============================================================================

.PHONY: release
release: ## Build the library in release mode
	@echo "$(COLOR_BOLD)$(COLOR_BLUE)==> Building MicroPDF $(VERSION) (release)...$(COLOR_RESET)"
	$(CARGO) build $(CARGO_RELEASE_FLAGS) $(CARGO_BUILD_FLAGS)
	@echo "$(COLOR_GREEN)✓ Build complete: $(LIB_PATH)$(COLOR_RESET)"

.PHONY: debug
debug: ## Build the library in debug mode
	@echo "$(COLOR_BOLD)$(COLOR_BLUE)==> Building MicroPDF $(VERSION) (debug)...$(COLOR_RESET)"
	$(CARGO) build $(CARGO_BUILD_FLAGS)
	@echo "$(COLOR_GREEN)✓ Build complete: $(DEBUG_DIR)/$(LIB_NAME)$(COLOR_RESET)"

.PHONY: headers
headers: ## Generate C headers from Rust FFI
	@echo "$(COLOR_BOLD)$(COLOR_BLUE)==> Generating headers...$(COLOR_RESET)"
	$(CARGO) build --release
	@echo "$(COLOR_GREEN)✓ Headers generated in include/$(COLOR_RESET)"

# ============================================================================
# Installation targets
# ============================================================================

.PHONY: install
install: release install-lib install-headers install-pkgconfig ## Install library, headers, and pkg-config files
	@echo "$(COLOR_BOLD)$(COLOR_GREEN)==> MicroPDF $(VERSION) installed successfully!$(COLOR_RESET)"
	@echo ""
	@echo "Installation summary:"
	@echo "  Library:    $(LIBDIR)/$(LIB_NAME)"
	@echo "  Headers:    $(INCLUDEDIR)/micropdf/"
	@echo "              $(INCLUDEDIR)/mupdf/"
	@echo "  Pkg-config: $(PKGCONFIGDIR)/micropdf.pc"
	@echo "              $(PKGCONFIGDIR)/mupdf.pc"
	@echo ""
	@echo "To use MicroPDF in your project:"
	@echo "  pkg-config --cflags --libs micropdf"

.PHONY: install-lib
install-lib: release ## Install the library only
	@echo "$(COLOR_BOLD)$(COLOR_BLUE)==> Installing library...$(COLOR_RESET)"
	install -d $(LIBDIR)
	install -m 644 $(LIB_PATH) $(LIBDIR)/$(LIB_NAME)
	@echo "$(COLOR_GREEN)✓ Library installed to $(LIBDIR)/$(LIB_NAME)$(COLOR_RESET)"

.PHONY: install-headers
install-headers: ## Install header files only
	@echo "$(COLOR_BOLD)$(COLOR_BLUE)==> Installing headers...$(COLOR_RESET)"
	install -d $(INCLUDEDIR)
	@# Install top-level headers
	install -m 644 include/micropdf.h $(INCLUDEDIR)/
	install -m 644 include/mupdf.h $(INCLUDEDIR)/
	install -m 644 include/mupdf-ffi.h $(INCLUDEDIR)/
	@# Install micropdf/ headers
	install -d $(INCLUDEDIR)/micropdf
	install -m 644 include/micropdf/*.h $(INCLUDEDIR)/micropdf/
	@# Install mupdf/ headers recursively
	@for header in $(shell find include/mupdf -type f -name "*.h"); do \
		header_rel=$$(echo $$header | sed 's|^include/||'); \
		header_dir=$$(dirname $$header_rel); \
		install -d $(INCLUDEDIR)/$$header_dir; \
		install -m 644 $$header $(INCLUDEDIR)/$$header_rel; \
	done
	@echo "$(COLOR_GREEN)✓ Headers installed to $(INCLUDEDIR)$(COLOR_RESET)"

.PHONY: install-pkgconfig
install-pkgconfig: ## Install pkg-config files
	@echo "$(COLOR_BOLD)$(COLOR_BLUE)==> Installing pkg-config files...$(COLOR_RESET)"
	install -d $(PKGCONFIGDIR)
	@# Generate micropdf.pc
	sed -e 's|@PREFIX@|$(PREFIX)|g' \
	    -e 's|@VERSION@|$(VERSION)|g' \
	    micropdf.pc.in > $(PKGCONFIGDIR)/micropdf.pc
	@# Generate mupdf.pc (for MuPDF compatibility)
	sed -e 's|@PREFIX@|$(PREFIX)|g' \
	    -e 's|@VERSION@|$(VERSION)|g' \
	    mupdf.pc.in > $(PKGCONFIGDIR)/mupdf.pc
	@echo "$(COLOR_GREEN)✓ Pkg-config files installed to $(PKGCONFIGDIR)$(COLOR_RESET)"

# ============================================================================
# Uninstallation targets
# ============================================================================

.PHONY: uninstall
uninstall: uninstall-lib uninstall-headers uninstall-pkgconfig ## Uninstall library, headers, and pkg-config files
	@echo "$(COLOR_BOLD)$(COLOR_GREEN)==> MicroPDF uninstalled successfully!$(COLOR_RESET)"

.PHONY: uninstall-lib
uninstall-lib: ## Uninstall the library
	@echo "$(COLOR_BOLD)$(COLOR_BLUE)==> Uninstalling library...$(COLOR_RESET)"
	rm -f $(LIBDIR)/$(LIB_NAME)
	@echo "$(COLOR_GREEN)✓ Library uninstalled$(COLOR_RESET)"

.PHONY: uninstall-headers
uninstall-headers: ## Uninstall header files
	@echo "$(COLOR_BOLD)$(COLOR_BLUE)==> Uninstalling headers...$(COLOR_RESET)"
	rm -f $(INCLUDEDIR)/micropdf.h
	rm -f $(INCLUDEDIR)/mupdf.h
	rm -f $(INCLUDEDIR)/mupdf-ffi.h
	rm -rf $(INCLUDEDIR)/micropdf
	rm -rf $(INCLUDEDIR)/mupdf
	@echo "$(COLOR_GREEN)✓ Headers uninstalled$(COLOR_RESET)"

.PHONY: uninstall-pkgconfig
uninstall-pkgconfig: ## Uninstall pkg-config files
	@echo "$(COLOR_BOLD)$(COLOR_BLUE)==> Uninstalling pkg-config files...$(COLOR_RESET)"
	rm -f $(PKGCONFIGDIR)/micropdf.pc
	rm -f $(PKGCONFIGDIR)/mupdf.pc
	@echo "$(COLOR_GREEN)✓ Pkg-config files uninstalled$(COLOR_RESET)"

# ============================================================================
# Testing and benchmarking
# ============================================================================

.PHONY: test
test: ## Run tests
	@echo "$(COLOR_BOLD)$(COLOR_BLUE)==> Running tests...$(COLOR_RESET)"
	$(CARGO) test $(CARGO_TEST_FLAGS)

.PHONY: test-verbose
test-verbose: ## Run tests with verbose output
	@echo "$(COLOR_BOLD)$(COLOR_BLUE)==> Running tests (verbose)...$(COLOR_RESET)"
	$(CARGO) test $(CARGO_TEST_FLAGS) -- --nocapture

.PHONY: bench
bench: ## Run benchmarks
	@echo "$(COLOR_BOLD)$(COLOR_BLUE)==> Running benchmarks...$(COLOR_RESET)"
	$(CARGO) bench $(CARGO_BENCH_FLAGS)

# ============================================================================
# Code quality targets
# ============================================================================

.PHONY: check
check: ## Run cargo check
	@echo "$(COLOR_BOLD)$(COLOR_BLUE)==> Checking code...$(COLOR_RESET)"
	$(CARGO) check

.PHONY: clippy
clippy: ## Run clippy linter
	@echo "$(COLOR_BOLD)$(COLOR_BLUE)==> Running clippy...$(COLOR_RESET)"
	$(CARGO) clippy -- -D warnings

.PHONY: fmt
fmt: ## Format code with rustfmt
	@echo "$(COLOR_BOLD)$(COLOR_BLUE)==> Formatting code...$(COLOR_RESET)"
	$(CARGO) fmt

.PHONY: fmt-check
fmt-check: ## Check code formatting
	@echo "$(COLOR_BOLD)$(COLOR_BLUE)==> Checking code format...$(COLOR_RESET)"
	$(CARGO) fmt -- --check

# ============================================================================
# Documentation targets
# ============================================================================

.PHONY: doc
doc: ## Generate documentation
	@echo "$(COLOR_BOLD)$(COLOR_BLUE)==> Generating documentation...$(COLOR_RESET)"
	$(CARGO) doc --no-deps

.PHONY: doc-open
doc-open: ## Generate and open documentation
	@echo "$(COLOR_BOLD)$(COLOR_BLUE)==> Generating and opening documentation...$(COLOR_RESET)"
	$(CARGO) doc --no-deps --open

# ============================================================================
# Cleaning targets
# ============================================================================

.PHONY: clean
clean: ## Clean build artifacts
	@echo "$(COLOR_BOLD)$(COLOR_BLUE)==> Cleaning build artifacts...$(COLOR_RESET)"
	$(CARGO) clean
	@echo "$(COLOR_GREEN)✓ Clean complete$(COLOR_RESET)"

.PHONY: distclean
distclean: clean ## Clean everything including Cargo.lock
	@echo "$(COLOR_BOLD)$(COLOR_BLUE)==> Deep cleaning...$(COLOR_RESET)"
	rm -f Cargo.lock
	@echo "$(COLOR_GREEN)✓ Deep clean complete$(COLOR_RESET)"

# ============================================================================
# Package building targets
# ============================================================================

.PHONY: deb
deb: ## Build Debian package (requires Docker)
	@echo "$(COLOR_BOLD)$(COLOR_BLUE)==> Building Debian package...$(COLOR_RESET)"
	cd docker && ./build.sh debian
	@echo "$(COLOR_GREEN)✓ Debian package built$(COLOR_RESET)"

.PHONY: rpm
rpm: ## Build RPM package (requires Docker)
	@echo "$(COLOR_BOLD)$(COLOR_BLUE)==> Building RPM package...$(COLOR_RESET)"
	cd docker && ./build.sh rpm
	@echo "$(COLOR_GREEN)✓ RPM package built$(COLOR_RESET)"

# ============================================================================
# Development targets
# ============================================================================

.PHONY: watch
watch: ## Watch for changes and rebuild
	@echo "$(COLOR_BOLD)$(COLOR_BLUE)==> Watching for changes...$(COLOR_RESET)"
	$(CARGO) watch -x build

.PHONY: watch-test
watch-test: ## Watch for changes and run tests
	@echo "$(COLOR_BOLD)$(COLOR_BLUE)==> Watching for changes (tests)...$(COLOR_RESET)"
	$(CARGO) watch -x test

# ============================================================================
# Information targets
# ============================================================================

.PHONY: version
version: ## Show version information
	@echo "MicroPDF version: $(VERSION)"
	@echo "Cargo version:   $$($(CARGO) --version)"
	@echo "Rust version:    $$(rustc --version)"
	@echo "Target triple:   $(TARGET_TRIPLE)"

.PHONY: info
info: ## Show build information
	@echo "$(COLOR_BOLD)MicroPDF Build Configuration$(COLOR_RESET)"
	@echo ""
	@echo "Version:          $(VERSION)"
	@echo "Prefix:           $(PREFIX)"
	@echo "Library dir:      $(LIBDIR)"
	@echo "Include dir:      $(INCLUDEDIR)"
	@echo "Pkg-config dir:   $(PKGCONFIGDIR)"
	@echo "Target arch:      $(TARGET_ARCH)"
	@echo "Target triple:    $(TARGET_TRIPLE)"
	@echo "Build dir:        $(BUILD_DIR)"
	@echo "Library name:     $(LIB_NAME)"
	@echo ""
	@echo "$(COLOR_BOLD)Installed tools:$(COLOR_RESET)"
	@echo "Cargo:            $$(which $(CARGO))"
	@echo "Cargo version:    $$($(CARGO) --version)"
	@echo "Rust version:     $$(rustc --version)"

.PHONY: help
help: ## Show this help message
	@echo "$(COLOR_BOLD)MicroPDF Makefile$(COLOR_RESET)"
	@echo ""
	@echo "$(COLOR_BOLD)Usage:$(COLOR_RESET)"
	@echo "  make [target] [PREFIX=/path]"
	@echo ""
	@echo "$(COLOR_BOLD)Build Targets:$(COLOR_RESET)"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "  $(COLOR_BLUE)%-20s$(COLOR_RESET) %s\n", $$1, $$2}' | \
		grep -E '(release|debug|headers|clean)'
	@echo ""
	@echo "$(COLOR_BOLD)Installation Targets:$(COLOR_RESET)"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "  $(COLOR_BLUE)%-20s$(COLOR_RESET) %s\n", $$1, $$2}' | \
		grep -E '(install|uninstall)'
	@echo ""
	@echo "$(COLOR_BOLD)Testing Targets:$(COLOR_RESET)"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "  $(COLOR_BLUE)%-20s$(COLOR_RESET) %s\n", $$1, $$2}' | \
		grep -E '(test|bench|check|clippy|fmt)'
	@echo ""
	@echo "$(COLOR_BOLD)Other Targets:$(COLOR_RESET)"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "  $(COLOR_BLUE)%-20s$(COLOR_RESET) %s\n", $$1, $$2}' | \
		grep -vE '(release|debug|headers|clean|install|uninstall|test|bench|check|clippy|fmt)'
	@echo ""
	@echo "$(COLOR_BOLD)Examples:$(COLOR_RESET)"
	@echo "  make                    # Build release version"
	@echo "  make install            # Install to /usr/local"
	@echo "  make install PREFIX=/usr  # Install to /usr"
	@echo "  make test               # Run tests"
	@echo "  make clean              # Clean build artifacts"
	@echo ""

# ============================================================================
# Special targets
# ============================================================================

.DEFAULT_GOAL := help
.PHONY: all help

# Ensure target directories exist
$(BUILD_DIR) $(DEBUG_DIR):
	@mkdir -p $@

