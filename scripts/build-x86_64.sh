#!/bin/sh
# Build alpenglowed for x86_64-linux-musl using cargo-zigbuild
# Requires: vendored gpui in crepuscularity/vendor/gpui with xkbcommon/x11 removed
set -eu
cd "$(dirname "$0")/.."

TARGET="x86_64-unknown-linux-musl"
echo "=== Alpenglowed ${TARGET} cross-build ==="

rustup target add "${TARGET}" 2>/dev/null || true

# Still needs libxkbcommon.a for the target — not available on macOS.
# Build on ultramarine instead for musl targets, or use native glibc target.
cargo zigbuild --release --target "${TARGET}" --features compositor "$@"

echo ""
echo "Binary: target/${TARGET}/release/alpenglowed"
file "target/${TARGET}/release/alpenglowed"
