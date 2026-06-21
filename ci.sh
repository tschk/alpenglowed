#!/bin/sh
set -eu
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features --locked -- -D warnings
cargo fmt --check --manifest-path plugins/spotify-rust/Cargo.toml
cargo check --manifest-path plugins/spotify-rust/Cargo.toml
cargo test --manifest-path plugins/spotify-rust/Cargo.toml
cargo clippy --manifest-path plugins/spotify-rust/Cargo.toml --all-targets -- -D warnings
echo "ok"
