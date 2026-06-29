#!/bin/sh
# Build alpenglowed for aarch64-linux-musl — cross-compiled from macOS
set -eu
cd "$(dirname "$0")/.."

TARGET="aarch64-unknown-linux-musl"
echo "=== Alpenglowed ${TARGET} cross-build ==="
echo "  Requires: libxkbcommon.a for ${TARGET} at /tmp/xkb-cross-aarch64/lib"
echo "  Build with: scripts/cross-build-libxkbcommon.sh"
echo ""

rustup target add "${TARGET}" 2>/dev/null || true

cargo zigbuild --release --target "${TARGET}" --features compositor "$@"

echo ""
echo "Binary: target/${TARGET}/release/alpenglowed"
file "target/${TARGET}/release/alpenglowed"
