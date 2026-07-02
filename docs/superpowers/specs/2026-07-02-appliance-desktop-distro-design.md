# Alpenglow Appliance + Alpenglowed Desktop Distro — Design

**Date:** 2026-07-02
**Repos:** `alpenglow` (appliance/OS: kernel, rootfs, pkg mgr, init), `alpenglowed` (desktop shell/DE), `inauguration` (compiler, external, generic-only)

## Goal

Two coherent, buildable distro variants sharing one appliance base:

1. **Appliance** (`BUILD_PROFILE=minimal`) — headless, diskless, immutable, SSH+network+logs only.
2. **Desktop** (`BUILD_PROFILE=standard`) — appliance base + Wayland + PipeWire + alpenglowed as the shell, stateless with factory defaults.

Both stay diskless-first (rootfs in RAM), no persistent state unless explicitly mounted (matches existing `mount-state.sh` bind-mount model).

## Current state

| Area | Status |
|------|--------|
| `alpenglowed` repo | Already separate (`github.com/tschk/alpenglowed`), consumed via path-dep build (`build-alpenglowed-glibc.sh`) + dinit service. **Not** a git submodule; sibling-checkout convention. |
| Package manager | Oil (Rust, APK format, sync HTTP, ~2.3K LOC). Committed, not being replaced. |
| Kernel preempt | `CONFIG_PREEMPT` not set; `CONFIG_PREEMPT_LAZY=y` (QEMU-minimal config). No RT patchset. |
| MAC | `CONFIG_SECURITY_SELINUX=n` (minimal.config). `CONFIG_SECURITY_LANDLOCK=y` already on in `alpenglow-internet-appliance.config` only — not universal. |
| Root FS | GlowFS (kernel module) primary for appliance+desktop today; erofs/squashfs listed as fallback only, untested as default. |
| usrmerge | Not done — `/bin`, `/sbin` are real dirs, not symlinks into `/usr` (`configure-rootfs.sh`). |
| earlyoom | Not present. |
| Arch | x86_64 wired everywhere in scripts (`boot-native.sh`, kernel `ARCH=x86_64`); aarch64 mentioned in docs/status table but not exercised by scripts. |
| Audio | ALSA + PipeWire already (no pulseaudio/jack found in tree) — effectively already PipeWire-first. |
| Stateless defaults | No `/usr/share/defaults` convention; config lives directly under `/etc` in overlay, no factory-reset path. |
| Inauguration | External compiler project (self-hosting in progress, Linux target still "planned" not "working"). Already namechecked in `alpenglow/AGENTS.md` as "future codegen." Used by `../space` today for freestanding (`x86_64-unknown-none`) targets only — not for a hosted Linux userland yet. |

## Decisions (locked for this roadmap)

1. **Package manager:** keep Oil. Adopt ypkg's *idea* — a declarative YAML build-recipe format — as a format layered on Oil/APK, not a manager swap. No Vala/eopkg dependency enters the tree.
2. **MAC:** no SELinux in this roadmap. Go wider with what's already half-on: `CONFIG_SECURITY_LANDLOCK` promoted from the internet-appliance config to the shared baseline, plus seccomp-bpf filters per dinit service (dropbear, chronyd, dnsmasq, crond first — network-facing). SELinux stays a parked idea, not a milestone.
3. **Kernel preemption:** `PREEMPT_RT`. Real patchset, not just `CONFIG_PREEMPT`. Accept the bigger diff/testing burden; desktop responsiveness (compositor + audio) is the driver.
4. **Root FS split by profile:**
   - **Desktop** (`standard` profile, alpenglowed): stays on **GlowFS**. It's already the immutable root the desktop targets; no FS churn on the profile with the least testing headroom.
   - **Appliance** (`minimal` profile): reconsider the default. Candidates: erofs/squashfs (already-listed fallback, zero new code) vs **bcachefs** (modern, checksummed, in-tree since 6.7, but heavier and less proven as a *read-only appliance root* — most bcachefs deployment experience is read-write). This is an **open decision**, not locked — see Open Questions.
5. **usrmerge:** yes. `/bin`, `/sbin`, `/lib`, `/lib64` become symlinks into `/usr/{bin,sbin,lib,lib64}` (Solus/Fedora/Arch precedent). Toybox and dinit units get canonical `/usr/...` paths; back-compat symlinks stay at the old paths.
6. **earlyoom:** add as a dinit-managed service, minimal profile and up. Fits the "few lines, no bloat" ethos — it's a single static binary + a systemd/dinit unit, no daemon rewrite needed.
7. **Stateless factory defaults:** Solus-style `/usr/share/defaults/<app>/...` tree ships in the immutable rootfs; `/etc/<app>` is empty until first boot copies (or symlinks, where no per-user mutation is needed) defaults down. A `factory-reset` action removes the `/etc` copy and re-derives from `/usr/share/defaults`. Applies first to alpenglowed config (bar layout, keybinds) and greetd/dinit-level appliance config second.
8. **Arch matrix:** x86_64 and aarch64 only, everywhere — kernel configs, CI matrix, cross-build scripts, `oil` package arch tags. No riscv/other listed as "generic" anymore; drop the word "generic" from `AGENTS.md`'s arch row once scripts back it.
9. **Precompiled initramfs:** publish signed, versioned initramfs + kernel + rootfs image tuples as GitHub Release artifacts (both arches) so users don't need a full from-source build to try Alpenglow. Build-from-source remains the default dev path; releases are a convenience artifact, not a new build system.
10. **Inauguration:** do **not** put alpenglow/alpenglowed-specific code in `../inauguration` (same boundary rule `space/AGENTS.md` already enforces for itself — generic capability in inauguration, product policy in the consuming repo). Track its Linux-hosted-target maturity; the concrete near-term use is **narrow and non-critical-path**: a small build-time codegen tool (e.g. Oil recipe → APK metadata generator, or a `protocol-gen`-style schema-to-Rust step) once `in` reliably targets Linux x86_64/aarch64 hosted binaries. Nothing on the boot path depends on it. Revisit when inauguration's own status table shows Linux target "working," not "planned."
11. **Zig-first for new components:** when starting a *new* small system component (daemon, CLI helper, initramfs tool), default to Zig the way `kernelctl`/`glowfsctl` already do, unless it needs an ecosystem only Rust has (async runtimes, crates.io deps) or is a GPUI/Crepuscularity UI (stays Rust). No rewrite of existing Rust (Oil, netd, alpenglowed) — this is a policy for new code only. `earlyoom` in item 6: prefer using upstream earlyoom (C) unmodified first; only write a Zig reimplementation if upstream doesn't fit the static-size budget.

## Open questions (need an answer before implementation, flagged in the plan)

- **Appliance root FS default:** erofs/squashfs now (cheap, proven, zero new code) vs bcachefs (modern, more work, less appliance-track-record). Recommendation: ship erofs as the appliance default in the near term (matches "fallback" already documented, lowest risk), keep bcachefs as a tracked stretch goal evaluated once erofs lands and boots clean on both arches.
- **PREEMPT_RT patch source:** which kernel version's RT patchset tracks `Linux 7.0.12` (the pinned kernel version in `AGENTS.md`)? Needs a compatibility check against `linux-rt-devel` / `projectacrn` mirrors before committing to a patch series file under `system/backends/appliance/kernel/patches/`.
- **usrmerge ordering vs existing images:** does this break any pinned paths in dinit units or Oil package payloads that hardcode `/bin/...`? Needs a repo-wide grep pass as task 1 of the plan, not assumed clean.
- **Factory-reset trigger:** boot-time flag (kernel cmdline), a dinit action, or a keybind in alpenglowed? Left to whoever implements Phase 4.

## Non-goals (explicitly out of scope for this roadmap)

- Full SELinux policy authoring.
- Rewriting Oil to use ypkg/eopkg.
- Replacing GlowFS on the desktop profile.
- Making any part of the boot path depend on `../inauguration`.
- Multi-arch beyond x86_64/aarch64 (no riscv64, no 32-bit).

## Success criteria

1. `standard` profile boots to alpenglowed shell on GlowFS root, `/usr` merged, Landlock baseline active, earlyoom running, config sourced from `/usr/share/defaults` on first boot.
2. `minimal` profile boots on the chosen appliance root FS (erofs by default per recommendation above), same usrmerge/Landlock/earlyoom baseline, no GPUI/Wayland packages present.
3. Both profiles build for x86_64 and aarch64; CI matrix (`ci-rust-core.sh`, `ci-zig.sh`, `ci-os-appliance.sh`) green on both.
4. A tagged GitHub Release carries prebuilt initramfs+kernel+rootfs for both arches, both profiles.
5. Kernel boots with `PREEMPT_RT` on at least one arch in QEMU (`bench-boot.sh` still produces a number).
6. No inauguration dependency anywhere in the appliance boot path; one narrow, optional build-time tool experiment tracked separately.
