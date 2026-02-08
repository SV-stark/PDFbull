# NanoPDF Docker Packaging

This directory contains Dockerfiles and scripts for building NanoPDF as system packages (Debian `.deb` and Red Hat/Fedora `.rpm`) for multiple architectures.

## Supported Architectures

- **AMD64** (x86_64) - Intel/AMD 64-bit processors
- **ARM64** (aarch64) - ARM 64-bit processors (Raspberry Pi 4+, Apple Silicon via Rosetta, AWS Graviton, etc.)

## Building Locally

### Prerequisites

1. **Docker Desktop** (recommended) or **Docker Engine with Buildx**
2. **QEMU** for cross-platform emulation (included in Docker Desktop)

### Quick Start

```bash
# Build all packages for all architectures
./build.sh all

# Build only Debian packages
./build.sh deb

# Build only RPM packages
./build.sh rpm

# Clean up build artifacts and images
./build.sh clean
```

### Platform-Specific Builds

Build for a specific architecture only:

```bash
# AMD64 only
PLATFORM=linux/amd64 ./build.sh deb
PLATFORM=linux/amd64 ./build.sh rpm

# ARM64 only
PLATFORM=linux/arm64 ./build.sh deb
PLATFORM=linux/arm64 ./build.sh rpm
```

### Output

Built packages are placed in `../dist/` with architecture suffixes:

```
dist/
├── nanopdf_0.1.0_amd64.deb
├── nanopdf_0.1.0_arm64.deb
├── nanopdf-0.1.0-amd64.rpm
└── nanopdf-0.1.0-arm64.rpm
```

## GitHub Actions CI/CD

### Continuous Integration

On every push and pull request, the CI workflow builds and tests packages for both architectures:

- **Workflow**: `.github/workflows/ci.yml`
- **Job**: `docker-build`
- **Matrix**: `[linux/amd64, linux/arm64]`

This ensures all code changes are validated across both architectures.

### Release Workflow

When a new version is tagged or released, the release workflow automatically builds and uploads packages:

- **Workflow**: `.github/workflows/release-packages.yml`
- **Trigger**: Git tags matching `v*` or GitHub releases
- **Output**: Packages attached to GitHub release

To create a release:

```bash
# Tag a new version
git tag -a v0.1.0 -m "Release v0.1.0"
git push origin v0.1.0

# Or create a GitHub release via web UI
```

## Package Contents

All packages include:

### Libraries
- `/usr/lib64/libnanopdf.so` - Shared library (755)
- `/usr/lib64/libnanopdf.a` - Static library (644)

### Headers
- `/usr/include/nanopdf/nanopdf.h` - Main header
- `/usr/include/nanopdf/mupdf-ffi.h` - MuPDF compatibility header
- `/usr/include/nanopdf/mupdf/` - Auto-generated module headers
- `/usr/include/nanopdf/nanopdf/enhanced.h` - Enhanced functions

### pkg-config Files
- `/usr/lib64/pkgconfig/nanopdf.pc`
- `/usr/lib64/pkgconfig/mupdf.pc` (compatibility alias)

### Documentation
- `/usr/share/doc/nanopdf/README`
- `/usr/share/doc/nanopdf/LICENSE-MIT`
- `/usr/share/doc/nanopdf/LICENSE-APACHE`

## Installation

### Debian/Ubuntu (AMD64)
```bash
wget https://github.com/Lexmata/nanopdf/releases/latest/download/nanopdf_*_amd64.deb
sudo dpkg -i nanopdf_*_amd64.deb
sudo apt-get install -f  # Resolve dependencies
```

### Debian/Ubuntu (ARM64)
```bash
wget https://github.com/Lexmata/nanopdf/releases/latest/download/nanopdf_*_arm64.deb
sudo dpkg -i nanopdf_*_arm64.deb
sudo apt-get install -f  # Resolve dependencies
```

### Red Hat/Fedora (AMD64)
```bash
wget https://github.com/Lexmata/nanopdf/releases/latest/download/nanopdf-*-amd64.rpm
sudo rpm -i nanopdf-*-amd64.rpm
```

### Red Hat/Fedora (ARM64)
```bash
wget https://github.com/Lexmata/nanopdf/releases/latest/download/nanopdf-*-arm64.rpm
sudo rpm -i nanopdf-*-arm64.rpm
```

## Verification

After installation:

```bash
# Check version
pkg-config --modversion nanopdf

# Get compiler flags
pkg-config --cflags nanopdf

# Get linker flags
pkg-config --libs nanopdf

# Verify library location
ldconfig -p | grep nanopdf

# Check installed files
dpkg -L nanopdf      # Debian
rpm -ql nanopdf      # Red Hat
```

## Architecture Detection

The package manager automatically selects the correct architecture:

```bash
# This will install the appropriate package for your system
dpkg -i nanopdf_*.deb    # Automatically uses amd64 or arm64
rpm -i nanopdf-*.rpm     # Automatically uses x86_64 or aarch64
```

## Troubleshooting

### Missing QEMU Support

If cross-platform builds fail:

```bash
# Install QEMU user static
sudo apt-get install qemu-user-static

# Register QEMU handlers
docker run --rm --privileged multiarch/qemu-user-static --reset -p yes
```

### Buildx Not Available

```bash
# Create and use a new builder
docker buildx create --name mybuilder --use
docker buildx inspect --bootstrap
```

### Slow ARM64 Builds

ARM64 builds via QEMU emulation can be slow (10-30 minutes). For faster builds:

1. Use ARM64 native runners (GitHub Actions ARM64 runners, AWS Graviton, etc.)
2. Build on actual ARM64 hardware
3. Use cached builds (`cache-from`/`cache-to` in CI)

## Development

### Modifying Dockerfiles

- **Dockerfile.debian** - Debian package build
- **Dockerfile.redhat** - RPM package build

Both use multi-stage builds:
1. **Builder stage**: Compiles Rust code and creates package
2. **Output stage**: Lightweight container with just the package

### Testing Changes

```bash
# Build for your native architecture
PLATFORM=linux/$(uname -m) ./build.sh deb

# Test installation in a container
docker run --rm -v $(pwd)/../dist:/packages debian:bookworm bash -c \
  "apt-get update && dpkg -i /packages/*.deb || apt-get install -f -y"
```

## CI/CD Integration

### Required GitHub Actions Permissions

The release workflow needs:
- `contents: write` - Upload packages to releases
- `packages: write` - Publish to GitHub Packages (optional)

### Caching Strategy

The CI uses GitHub Actions cache for Docker layers:
- Scope: `{platform}-{package}` (e.g., `amd64-debian`, `arm64-rpm`)
- Mode: `max` - Cache all layers
- Retention: 7 days (GitHub default)

This significantly speeds up builds on repeated CI runs.

## Support

For issues with packaging:
1. Check the [CI logs](https://github.com/Lexmata/nanopdf/actions)
2. Verify Docker and Buildx versions
3. Test locally with `./build.sh`
4. Open an issue with build logs

## License

Same as NanoPDF: MIT OR Apache-2.0

