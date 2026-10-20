#!/bin/sh
set -eu

if [ "$(uname)" = "Darwin" ]; then
    export SDKROOT="${SDKROOT:-$(xcrun --show-sdk-path 2>/dev/null || true)}"
    if [ -n "${SDKROOT:-}" ] && [ -z "${BINDGEN_EXTRA_CLANG_ARGS:-}" ]; then
        export BINDGEN_EXTRA_CLANG_ARGS="-isysroot $SDKROOT"
    fi
    if [ -d "/opt/homebrew/lib" ]; then
        export LIBRARY_PATH="${LIBRARY_PATH:-/opt/homebrew/lib}"
    fi
fi
cargo fmt --check
cargo check
cargo test
cargo check --no-default-features
cargo test --no-default-features
cargo clippy --all-targets --all-features --locked -- -D warnings
cargo clippy --all-targets --no-default-features --locked -- -D warnings
cargo fmt --check --manifest-path plugins/spotify-rust/Cargo.toml
cargo check --manifest-path plugins/spotify-rust/Cargo.toml
cargo test --manifest-path plugins/spotify-rust/Cargo.toml
cargo clippy --manifest-path plugins/spotify-rust/Cargo.toml --all-targets -- -D warnings
echo "ok"
