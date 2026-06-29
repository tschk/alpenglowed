#!/bin/sh
# Cross-compile libxkbcommon for musl target using meson + zig
# Called by build-x86_64.sh and build-aarch64.sh
set -eu

TARGET="${1:-x86_64-linux-musl}"
echo "=== Building libxkbcommon for ${TARGET} ==="

SRC_DIR="/tmp/libxkbcommon"
if [ ! -d "${SRC_DIR}" ]; then
  git clone --depth 1 https://github.com/xkbcommon/libxkbcommon.git "${SRC_DIR}"
fi

ARCH="${TARGET%%-*}"  # x86_64 or aarch64
BUILD_DIR="${SRC_DIR}/build-${ARCH}"

# Create zig compiler wrappers
for drv in cc cxx; do
  cat > "${SRC_DIR}/zig-${drv}-${ARCH}.sh" << WRAP
#!/bin/sh
exec zig ${drv} -target "${TARGET}" "\$@"
WRAP
  chmod +x "${SRC_DIR}/zig-${drv}-${ARCH}.sh"
done
for tool in ar ranlib; do
  cat > "${SRC_DIR}/zig-${tool}-${ARCH}.sh" << WRAP
#!/bin/sh
exec zig ${tool} "\$@"
WRAP
  chmod +x "${SRC_DIR}/zig-${tool}-${ARCH}.sh"
done

# Map Rust target triple to meson cpu_family
case "${ARCH}" in
  x86_64) CPU_FAMILY="x86_64"; CPU="x86_64" ;;
  aarch64) CPU_FAMILY="aarch64"; CPU="aarch64" ;;
  *) echo "Unknown arch: ${ARCH}"; exit 1 ;;
esac

cat > "${SRC_DIR}/zig-${ARCH}.ini" << INI
[binaries]
c = '${SRC_DIR}/zig-cc-${ARCH}.sh'
cpp = '${SRC_DIR}/zig-cxx-${ARCH}.sh'
ar = '${SRC_DIR}/zig-ar-${ARCH}.sh'
ranlib = '${SRC_DIR}/zig-ranlib-${ARCH}.sh'

[host_machine]
system = 'linux'
cpu_family = '${CPU_FAMILY}'
cpu = '${CPU}'
endian = 'little'

[built-in options]
default_library = 'static'
INI

rm -rf "${BUILD_DIR}"
PATH="/opt/homebrew/opt/bison/bin:$PATH" \
meson setup "${BUILD_DIR}" \
  --cross-file "${SRC_DIR}/zig-${ARCH}.ini" \
  -Denable-x11=false \
  -Denable-docs=false \
  -Denable-wayland=false \
  -Denable-xkbregistry=false \
  -Denable-tools=false \
  2>&1 | tail -5

PATH="/opt/homebrew/opt/bison/bin:$PATH" \
ninja -C "${BUILD_DIR}" 2>&1 | tail -3

# Install
INSTALL_DIR="/tmp/xkb-cross-${ARCH}"
mkdir -p "${INSTALL_DIR}/lib"
cp "${BUILD_DIR}/libxkbcommon.a" "${INSTALL_DIR}/lib/"
echo ""
echo "Installed: ${INSTALL_DIR}/lib/libxkbcommon.a ($(ls -lh "${INSTALL_DIR}/lib/libxkbcommon.a" | awk '{print $5}'))"
