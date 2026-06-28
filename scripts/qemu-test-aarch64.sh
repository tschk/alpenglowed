#!/bin/sh
# Test alpenglowed in aarch64 QEMU with HVF acceleration
# Uses Alpenglow/QEMU infrastructure from ../alpenglow
set -eu

REPO_ROOT="$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)"
ALPENGLOW_DIR="${REPO_ROOT}/../alpenglow"
BUILD_OUT="${REPO_ROOT}/target/aarch64-unknown-linux-musl/release"
ALPENGLOW_BINARY="${BUILD_OUT}/alpenglowed"
KERNEL="${ALPENGLOW_DIR}/build/cross/aarch64/vmlinuz"
INITRAMFS_SRC="${ALPENGLOW_DIR}/build/cross/aarch64/initramfs.cpio.gz"

fail() { echo "FAIL: $1" >&2; exit 1; }
require_cmd() { command -v "$1" >/dev/null 2>&1 || fail "missing: $1"; }

require_cmd qemu-system-aarch64

[ -f "${ALPENGLOW_BINARY}" ] || fail "Build alpenglowed first: scripts/build-aarch64.sh"
[ -f "${KERNEL}" ] || fail "Missing kernel — run ../alpenglow/scripts/build-aarch64.sh first"
[ -f "${INITRAMFS_SRC}" ] || fail "Missing initramfs"

echo "=== Alpenglowed aarch64 QEMU test ==="
echo "  binary:  ${ALPENGLOW_BINARY}"
echo "  kernel:  ${KERNEL}"
echo "  memory:  2G, accel: hvf"
echo ""

# Create initramfs with alpenglowed added
TMP_DIR=$(mktemp -d)
cd "${TMP_DIR}"
gzip -dc "${INITRAMFS_SRC}" | cpio -idm 2>/dev/null || true
cp "${ALPENGLOW_BINARY}" alpenglowed
chmod 755 alpenglowed
find . | cpio -o -H newc 2>/dev/null | gzip -9 > "${TMP_DIR}/initramfs.cpio.gz"
cd "${REPO_ROOT}"
INITRAMFS="${TMP_DIR}/initramfs.cpio.gz"

# Boot in QEMU
echo "→ Booting QEMU aarch64 with HVF..."
qemu-system-aarch64 \
  -M virt,accel=hvf \
  -cpu host \
  -m 2G \
  -smp 4 \
  -nographic \
  -no-reboot \
  -kernel "${KERNEL}" \
  -initrd "${INITRAMFS}" \
  -append "console=ttyAMA0,115200 init=/init quiet" \
  2>&1

rm -rf "${TMP_DIR}"
echo ""
echo "QEMU exited."
