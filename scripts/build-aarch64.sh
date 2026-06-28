#!/bin/sh
# Build alpenglowed for aarch64-linux-musl using cargo-zigbuild
set -eu
cd "$(dirname "$0")"

echo "=== Alpenglowed aarch64 cross-build ==="

# Install target if needed
rustup target add aarch64-unknown-linux-musl 2>/dev/null || true

# Build with zig as cross-compiler (handles C deps)
cargo zigbuild --release --target aarch64-unknown-linux-musl --features compositor "$@"

echo ""
echo "Binary: target/aarch64-unknown-linux-musl/release/alpenglowed"
file target/aarch64-unknown-linux-musl/release/alpenglowed
