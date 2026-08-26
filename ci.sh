#!/bin/sh
set -eu
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
