#!/bin/sh
# Test alpenglowed compositor in QEMU
# Build natively on Linux (ultramarine), copy binary to QEMU image
set -eu

SSH_HOST="${SSH_HOST:-undivisible@192.168.4.134}"
QEMU_IMG="${QEMU_IMG:-/tmp/alpenglow-boot.iso}"

echo "=== Alpenglowed QEMU test ==="

# Step 1: Build on Linux host
echo "→ Building on ${SSH_HOST}..."
ssh "${SSH_HOST}" "cd ~/projects/alpenglowed && git pull --ff-only && cargo build --release --features compositor 2>&1 | tail -3"
scp "${SSH_HOST}:~/projects/alpenglowed/target/release/alpenglowed" /tmp/alpenglowed-x86_64

file /tmp/alpenglowed-x86_64
echo "  Binary: $(ls -lh /tmp/alpenglowed-x86_64 | awk '{print $5}')"

# Step 2: Copy to QEMU VM
# This requires the QEMU image to be booted and accessible
echo ""
echo "To test in QEMU:"
echo "  1. Boot Alpenglow in QEMU"
echo "  2. Copy binary: scp /tmp/alpenglowed-x86_64 user@qemu-vm:~/"
echo "  3. Inside VM:"
echo "     WAYLAND_DISPLAY=wayland-0 ./alpenglowed --compositor"
echo ""
echo "Or run directly on ultramarine (WSLg):"
echo "  ssh ${SSH_HOST} 'cd ~/projects/alpenglowed && WAYLAND_DISPLAY=wayland-0 ./target/release/alpenglowed --compositor'"
