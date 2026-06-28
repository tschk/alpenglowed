#!/bin/sh
# Build alpenglowed for x86_64-linux-musl using cargo-zigbuild
# Provides a stub libxkbcommon-x11.a since gpui hard-enables xkbcommon/x11
set -eu
cd "$(dirname "$0")/.."

TARGET="x86_64-unknown-linux-musl"
echo "=== Alpenglowed ${TARGET} cross-build ==="

rustup target add "${TARGET}" 2>/dev/null || true

# Create stub libxkbcommon-x11.a for the target (gpui hard-enables xkbcommon/x11)
STUB_DIR="/tmp/xkb-stub-${TARGET}"
mkdir -p "${STUB_DIR}"
if [ ! -f "${STUB_DIR}/libxkbcommon-x11.a" ]; then
  echo "→ Building stub libxkbcommon-x11.a..."
  ZIG_TARGET="${TARGET}"
  cat > /tmp/xkb-stub.c << 'STUB'
int xkb_x11_setup_xkb_extension(void*a,int b,int c,int*d,int*e,int*f){if(d)*d=0;if(e)*e=b;if(f)*f=c;return 1;}
int xkb_x11_get_core_keyboard_device_id(void*a){return-1;}
void*xkb_x11_keymap_new_from_device(void*a,void*b,int c,int d){return(void*)0;}
void*xkb_x11_state_new_from_device(void*a,void*b,int c){return(void*)0;}
STUB
  zig cc -target "${ZIG_TARGET}" -c /tmp/xkb-stub.c -o "${STUB_DIR}/stub.o"
  ar crs "${STUB_DIR}/libxkbcommon-x11.a" "${STUB_DIR}/stub.o"
  echo "  ${STUB_DIR}/libxkbcommon-x11.a ($(ls -la "${STUB_DIR}/libxkbcommon-x11.a" | awk '{print $5}') bytes)"
fi

export RUSTFLAGS="-L ${STUB_DIR}"
cargo zigbuild --release --target "${TARGET}" --features compositor "$@"

echo ""
echo "Binary: target/${TARGET}/release/alpenglowed"
file "target/${TARGET}/release/alpenglowed"
