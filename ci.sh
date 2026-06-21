#!/bin/sh
cargo check 2>&1
cargo test 2>&1
cargo clippy --all-targets --all-features --locked -- -D warnings 2>&1
cargo fmt --check 2>&1
echo "ok"
