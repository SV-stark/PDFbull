#!/bin/bash
# Test script to verify pkg-config file generation

set -e

echo "=== Testing pkg-config file generation ==="

# Generate the files
VERSION="0.1.0"
PREFIX="/usr"

echo "Generating micropdf.pc..."
sed -e "s|@PREFIX@|$PREFIX|g" -e "s|@VERSION@|$VERSION|g" micropdf.pc.in > /tmp/micropdf.pc

echo "Generating mupdf.pc..."
sed -e "s|@PREFIX@|$PREFIX|g" -e "s|@VERSION@|$VERSION|g" mupdf.pc.in > /tmp/mupdf.pc

echo ""
echo "=== Generated micropdf.pc ==="
cat /tmp/micropdf.pc

echo ""
echo "=== Generated mupdf.pc ==="
cat /tmp/mupdf.pc

echo ""
echo "=== Testing with pkg-config ==="

# Test if the files are valid
if PKG_CONFIG_PATH=/tmp pkg-config --validate micropdf 2>/dev/null; then
    echo "✓ micropdf.pc is valid"
else
    echo "✗ micropdf.pc is invalid"
    exit 1
fi

if PKG_CONFIG_PATH=/tmp pkg-config --validate mupdf 2>/dev/null; then
    echo "✓ mupdf.pc is valid"
else
    echo "✗ mupdf.pc is invalid"
    exit 1
fi

echo ""
echo "=== Query tests ==="

echo "micropdf version: $(PKG_CONFIG_PATH=/tmp pkg-config --modversion micropdf)"
echo "micropdf cflags: $(PKG_CONFIG_PATH=/tmp pkg-config --cflags micropdf)"
echo "micropdf libs: $(PKG_CONFIG_PATH=/tmp pkg-config --libs micropdf)"

echo ""
echo "mupdf version: $(PKG_CONFIG_PATH=/tmp pkg-config --modversion mupdf)"
echo "mupdf cflags: $(PKG_CONFIG_PATH=/tmp pkg-config --cflags mupdf)"
echo "mupdf libs: $(PKG_CONFIG_PATH=/tmp pkg-config --libs mupdf)"

echo ""
echo "=== All tests passed! ==="

# Cleanup
rm /tmp/micropdf.pc /tmp/mupdf.pc

