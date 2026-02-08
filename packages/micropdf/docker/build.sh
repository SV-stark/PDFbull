#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

# Default platform (native architecture)
PLATFORM="${PLATFORM:-linux/amd64,linux/arm64}"

mkdir -p "$PROJECT_DIR/dist"

# Ensure buildx is available
if ! docker buildx version >/dev/null 2>&1; then
    echo "Error: Docker Buildx is required for multi-architecture builds"
    echo "Please install Docker Desktop or enable buildx"
    exit 1
fi

# Create builder instance if it doesn't exist
if ! docker buildx inspect micropdf-builder >/dev/null 2>&1; then
    echo "Creating multi-arch builder: micropdf-builder"
    docker buildx create --name micropdf-builder --use --bootstrap
else
    docker buildx use micropdf-builder
fi

build_multiarch() {
    local dockerfile=$1
    local tag=$2
    local output_dir=$3
    
    echo "Building for platforms: $PLATFORM"
    
    # Build for each platform separately to extract artifacts
    for platform in $(echo "$PLATFORM" | tr ',' ' '); do
        arch=$(echo "$platform" | cut -d'/' -f2)
        echo "Building $tag for $arch..."
        
        docker buildx build \
            --platform "$platform" \
            --file "$dockerfile" \
            --tag "$tag-$arch" \
            --load \
            "$PROJECT_DIR"
        
        # Extract package from container
        container_id=$(docker create "$tag-$arch")
        docker cp "$container_id:/output/." "$output_dir/"
        docker rm "$container_id"
        
        # Rename package to include architecture
        for pkg in "$output_dir"/*; do
            if [[ -f "$pkg" ]] && [[ ! "$pkg" =~ -$arch\. ]]; then
                base="${pkg%.*}"
                ext="${pkg##*.}"
                mv "$pkg" "${base}-${arch}.${ext}"
            fi
        done
    done
}

case "$1" in
    deb)
        build_multiarch "$SCRIPT_DIR/Dockerfile.debian" "micropdf-deb-builder" "$PROJECT_DIR/dist"
        ;;
    rpm)
        build_multiarch "$SCRIPT_DIR/Dockerfile.redhat" "micropdf-rpm-builder" "$PROJECT_DIR/dist"
        ;;
    all)
        $0 deb && $0 rpm
        echo "All packages built in: $PROJECT_DIR/dist/"
        ls -la "$PROJECT_DIR/dist/"
        ;;
    clean)
        docker rmi $(docker images -q 'micropdf-*-builder-*') 2>/dev/null || true
        docker buildx rm micropdf-builder 2>/dev/null || true
        rm -rf "$PROJECT_DIR/dist"
        ;;
    *)
        echo "Usage: $0 {deb|rpm|all|clean}"
        echo ""
        echo "Environment variables:"
        echo "  PLATFORM - Target platforms (default: linux/amd64,linux/arm64)"
        echo ""
        echo "Examples:"
        echo "  $0 deb                           # Build Debian packages for all platforms"
        echo "  PLATFORM=linux/amd64 $0 rpm      # Build RPM for AMD64 only"
        echo "  PLATFORM=linux/arm64 $0 all      # Build all packages for ARM64 only"
        exit 1
        ;;
esac

