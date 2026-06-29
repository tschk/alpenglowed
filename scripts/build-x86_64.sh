#!/bin/sh
# Build alpenglowed for x86_64-linux-musl — fully static binary
set -eu
cd "$(dirname "$0")/.."

TARGET="x86_64-unknown-linux-musl"
echo "=== Alpenglowed ${TARGET} cross-build ==="

# Ensure libxkbcommon.a is available
XKB_DIR="/tmp/xkb-cross-x86_64/lib"
if [ ! -f "${XKB_DIR}/libxkbcommon.a" ]; then
  echo "→ Building libxkbcommon first..."
  "$(dirname "$0")/cross-build-libxkbcommon.sh" x86_64-linux-musl
fi

rustup target add "${TARGET}" 2>/dev/null || true

cargo zigbuild --release --target "${TARGET}" --features compositor "$@"

echo ""
echo "Binary: target/${TARGET}/release/alpenglowed"
file "target/${TARGET}/release/alpenglowed"
