#!/bin/sh
# Local smoke: same Docker build as alpenglow boot-native graphical path.
set -eu
ROOT="$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)"
OUT="${ROOT}/../alpenglow/build/native"
sh "${ROOT}/../alpenglow/system/backends/appliance/scripts/build-alpenglow-greeter-glibc.sh" "${OUT}" "${ROOT}"
file "${OUT}/alpenglow-greeter-glibc/usr/bin/alpenglow-greeter"
echo "ok: alpenglow-greeter glibc binary"