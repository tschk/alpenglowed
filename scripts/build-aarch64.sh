#!/bin/sh
# Build alpenglowed for aarch64-linux-musl using cargo-zigbuild
# Requires: vendored gpui in crepuscularity/vendor/gpui with xkbcommon/x11 removed
set -eu
cd "$(dirname "$0")/.."

TARGET="aarch64-unknown-linux-musl"
echo "=== Alpenglowed ${TARGET} cross-build ==="

rustup target add "${TARGET}" 2>/dev/null || true

# Needs libxkbcommon.a for aarch64-linux-musl — not readily available on macOS.
# Workaround: first build libxkbcommon from source with meson+zig for the target.
cargo zigbuild --release --target "${TARGET}" --features compositor "$@"

echo ""
echo "Binary: target/${TARGET}/release/alpenglowed"
file "target/${TARGET}/release/alpenglowed"
