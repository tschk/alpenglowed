# Phase 0 findings — appliance-desktop-distro-roadmap

Findings for `docs/superpowers/plans/2026-07-02-appliance-desktop-distro-roadmap.md` Phase 0. Gates Phase 1/5/6/8.

## 0.1 usrmerge — hardcoded pre-merge paths (gates Phase 1)

`grep -rn '"/bin/\|"/sbin/\|/lib/\|/lib64/' system/backends/appliance/` → 63 hits across:

- `system/backends/appliance/scripts/configure-rootfs.sh`
- `system/backends/appliance/scripts/mount-state.sh`
- `system/backends/appliance/scripts/build-cage.sh`
- `system/backends/appliance/scripts/install-graphics-libs.sh`
- `system/backends/appliance/initramfs/init.rs`
- `system/backends/appliance/kernel/config-linux7-rust`

Real work, concentrated in one script (`configure-rootfs.sh`) plus a handful of build scripts and `init.rs`. Not touched by any other in-flight phase simultaneously except Phase 3 (adds one dinit-link line) and Phase 4 (adds a defaults copy-down call) — both are additive appends, not path rewrites, so low collision risk if Phase 1 lands first.

## 0.2 erofs/squashfs fallback (gates Phase 8)

Not doc-only — `system/backends/appliance/scripts/mount-glowfs-root.sh` has a real fallback loop (`for fmt in erofs squashfs`) that tries alternate mounts if GlowFS fails. However:

- No `EROFS=1`/`SQUASHFS=1` build-time toggle exists in `scripts/boot-native.sh` — you can't currently *build* an erofs or squashfs root image standalone, only fall back to mounting one if it already exists on disk.
- `build/alpine/rootfs/usr/local/bin/apply-kernel-policy.sh` loads the `erofs`/`squashfs` kernel modules and records capability flags, so the kernel side is ready.

**Conclusion:** the mount-time fallback path is real; the build-time image-generation path is not. Phase 8's appliance half starts from "kernel + mount script ready, image builder missing," not from zero.

## 0.3 kernel version drift (gates Phase 6)

Pinned: `KERNEL_VERSION="${KERNEL_VERSION:-7.0}"` in `scripts/boot-native.sh` (comment says "from 7.0.12").

kernel.org current (checked live): stable `7.1.2`, mainline `7.2-rc1`. The `7.0.x` line is now EOL (last was `7.0.14`). This is a full major-minor series behind, not a patch bump — Phase 6's version-check tool needs to handle both patch bumps (`7.0.12` → `7.0.14`) and minor-series bumps (`7.0.x` → `7.1.x`), and the initial bump PR should go straight to `7.1.2`.

## 0.4 aarch64 wiring (gates Phase 5)

`grep -rln aarch64 scripts/ system/backends/appliance/` finds real, non-stub scripts:

- `scripts/qemu-boot-aarch64.sh`
- `scripts/test-aarch64.sh` (builds cross components via `scripts/build-aarch64.sh`, boots in `qemu-system-aarch64`, checks for `"Alpenglow Zig init boot OK"`)
- `scripts/build-aarch64.sh`
- `system/backends/appliance/packages-dev.txt` (aarch64 dev package refs)

None of these are wired into `.github/workflows/ci.yml` — `grep aarch64 .github/workflows/ci.yml` is empty. So aarch64 is further along than "documented but not wired" implied: the scripts exist and look complete, they're just never run in CI. Phase 5 is "add a CI matrix leg," not "write aarch64 support from scratch."

## 0.5 bcachefs-on-brd RAM-root spike (gates Phase 8 desktop branch)

Not completed in this session. The parallel subagent could not run because the Devin subagent daily quota was exhausted, and the spike requires a real QEMU boot on `ultramarine` with a custom kernel build (10+ minutes) plus bcachefs-tools setup. This gates Phase 8's desktop-root branch decision: until the spike runs, Phase 8 should fall back to the safe path (erofs root for both profiles, bcachefs only for the persistent state layer) rather than assuming bcachefs-on-brd is viable.
