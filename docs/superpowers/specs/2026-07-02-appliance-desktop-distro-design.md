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
| Root FS | GlowFS (kernel module) primary for appliance+desktop today; erofs/squashfs listed as fallback only, untested as default. GlowFS itself already lags the pinned kernel: CI builds it against Linux 6.12.93 (`ci-glowfs-kernel-module.sh`), while the main boot path builds Linux 7.0 (`boot-native.sh`) — there's a standing `ponytail:` comment noting the 7.0 port is WIP. |
| Kernel version | Pinned (`KERNEL_VERSION=7.0` in `boot-native.sh`, `7.0.12` referenced in `AGENTS.md`), not rolling. No automated latest-stable tracking exists. |
| kernelctl | Zig-only in tree (`system/kernelctl-zig/`). The "Rust (501KB static)" line in `AGENTS.md`'s design table is a historical comparison data point from before the Zig rewrite, not a live parallel implementation. |
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
3a. **Kernel version policy: rolling latest stable, no matter what.** Track kernel.org's latest stable tag continuously (not -rc, not mainline git HEAD — see Open Questions for the automation mechanism). This is a hard requirement, not a "when convenient" — it's also *why* GlowFS gets parked (decision 4): an out-of-tree module chasing a permanently-moving target is a maintenance sink neither of us wants to own indefinitely.
4. **Root FS: GlowFS parked, not deleted.** Revisit as its own future project once someone has time to actually keep porting it release-to-release. It stays in the tree (`system/glowfs/`) untouched, just off the default path for both profiles.
   - **Appliance** (`minimal` profile): **erofs**. Purpose-built for exactly this case (read-only, compressed, in-tree since 4.19, lower metadata overhead than squashfs), already the documented fallback so this is the lowest-risk option that's closest to already working. Pure diskless — no persistent disk assumed, no hybrid layer.
   - **Desktop** (`standard` profile): **hybrid, both layers bcachefs.**
     - *Root image:* still loaded into RAM at boot like today (diskless-first is non-negotiable for the base OS) — but the image format itself is bcachefs instead of erofs/squashfs/GlowFS. **Flagged as a real technical risk, not a rubber-stamped decision** — bcachefs is designed around a persistent backing device with a journal; using it as a RAM-loaded read-mostly image (likely via a `brd` RAM block device rather than erofs's simpler direct-decompress-to-memory model) needs a feasibility spike before it's treated as locked. See Open Questions.
     - *Persistent/state layer:* real disk-backed, for anything that doesn't need to live in RAM — installed apps (Oil packages), user files, downloaded data, WiFi/network config. Replaces the current ext4 bind-mount state layer (`mount-state.sh`). This part is low-risk: bcachefs as an ext4 replacement for a normal persistent volume is well-trodden. Gets checksums, compression, and subvolume snapshots — which pairs naturally with the stateless/factory-reset design in decision 7 (snapshot-before-mutate, rollback = factory reset even for data that's supposed to be mutable).
     - The dividing line: RAM-resident = base OS + hot path only; disk-backed = everything a desktop user actually accumulates. Appliance has no such split because it has nothing to accumulate.
5. **usrmerge:** yes. `/bin`, `/sbin`, `/lib`, `/lib64` become symlinks into `/usr/{bin,sbin,lib,lib64}` (Solus/Fedora/Arch precedent). Toybox and dinit units get canonical `/usr/...` paths; back-compat symlinks stay at the old paths.
6. **earlyoom:** add as a dinit-managed service, minimal profile and up. Fits the "few lines, no bloat" ethos — it's a single static binary + a systemd/dinit unit, no daemon rewrite needed.
7. **Stateless factory defaults:** Solus-style `/usr/share/defaults/<app>/...` tree ships in the immutable rootfs; `/etc/<app>` is empty until first boot copies (or symlinks, where no per-user mutation is needed) defaults down. A `factory-reset` action removes the `/etc` copy and re-derives from `/usr/share/defaults`. Applies first to alpenglowed config (bar layout, keybinds) and greetd/dinit-level appliance config second.
8. **Arch matrix:** x86_64 and aarch64 only, everywhere — kernel configs, CI matrix, cross-build scripts, `oil` package arch tags. No riscv/other listed as "generic" anymore; drop the word "generic" from `AGENTS.md`'s arch row once scripts back it.
9. **Precompiled initramfs:** publish signed, versioned initramfs + kernel + rootfs image tuples as GitHub Release artifacts (both arches) so users don't need a full from-source build to try Alpenglow. Build-from-source remains the default dev path; releases are a convenience artifact, not a new build system.
10. **Inauguration:** do **not** put alpenglow/alpenglowed-specific code in `../inauguration` (same boundary rule `space/AGENTS.md` already enforces for itself — generic capability in inauguration, product policy in the consuming repo). Track its Linux-hosted-target maturity; the concrete near-term use is **narrow and non-critical-path**: a small build-time codegen tool (e.g. Oil recipe → APK metadata generator, or a `protocol-gen`-style schema-to-Rust step) once `in` reliably targets Linux x86_64/aarch64 hosted binaries. Nothing on the boot path depends on it. Revisit when inauguration's own status table shows Linux target "working," not "planned."
11. **Zig-first for new components:** when starting a *new* small system component (daemon, CLI helper, initramfs tool), default to Zig the way `kernelctl`/`glowfsctl` already do, unless it needs an ecosystem only Rust has (async runtimes, crates.io deps) or is a GPUI/Crepuscularity UI (stays Rust). No rewrite of existing Rust (Oil, netd, alpenglowed) — this is a policy for new code only. `earlyoom` in item 6: prefer using upstream earlyoom (C) unmodified first; only write a Zig reimplementation if upstream doesn't fit the static-size budget.

## Open questions (need an answer before implementation, flagged in the plan)

- **bcachefs-as-RAM-root feasibility:** can a bcachefs image be built once, loaded into a RAM block device (`brd`) at boot, and mounted read-mostly with acceptable boot-time overhead versus erofs's simpler decompress-to-memory path? Needs a spike (build one image, boot it in QEMU, measure) before it's a locked default for the desktop profile. If it doesn't pan out inside a reasonable time budget, fall back to erofs for the desktop root image too and keep bcachefs scoped to the persistent state layer only — that half of the decision is not at risk.
- **Kernel version bump automation:** "latest stable, no matter what" needs a mechanism, not just a policy sentence. Candidates: (a) a scheduled CI job that checks kernel.org's stable feed and opens a PR bumping `KERNEL_VERSION` when a new release lands, gated on the existing CI matrix passing; (b) a manual-but-frequent check via a `scripts/check-kernel-latest.sh` helper run periodically by a human. Recommendation: (a), since "no matter what" implies this shouldn't depend on someone remembering to check.
- **PREEMPT_RT patch source:** which kernel version's RT patchset tracks whatever the *current* latest-stable is at implementation time (no longer pinned to 7.0.12 per decision 3a)? This needs re-checking at whatever point Phase 6 actually starts, not resolved once and forgotten — the rolling-kernel policy means this is a recurring check, not a one-time compatibility lookup.
- **usrmerge ordering vs existing images:** does this break any pinned paths in dinit units or Oil package payloads that hardcode `/bin/...`? Needs a repo-wide grep pass as task 1 of the plan, not assumed clean.
- **Factory-reset trigger:** boot-time flag (kernel cmdline), a dinit action, or a keybind in alpenglowed? Left to whoever implements Phase 4.

## Non-goals (explicitly out of scope for this roadmap)

- Full SELinux policy authoring.
- Rewriting Oil to use ypkg/eopkg.
- Reviving GlowFS on any profile (parked; separate future project).
- Pinning the kernel to a fixed version (superseded by the rolling latest-stable policy, decision 3a).
- Making any part of the boot path depend on `../inauguration`.
- Multi-arch beyond x86_64/aarch64 (no riscv64, no 32-bit).

## Concrete next Zig components (from roadmap discussion, not yet built)

Greenfield, unclaimed, matching the `kernelctl-zig`/`glowfsctl-zig` precedent (tiny static binaries):

- **Interactive installer** — status table already says "planned," nothing exists yet.
- **Release artifact packager/signer** (Phase 8).
- **Kernel-version-bump + config-diff checker** — the tool that implements the rolling latest-stable mechanism from the Open Questions above.
- **Factory-reset / defaults-copy helper** (Phase 4).

Not Zig, stays Rust: Oil, netd, alpenglowed/greeter (GPUI needs the Rust ecosystem).

## Success criteria

1. `standard` profile boots to alpenglowed shell on a bcachefs RAM-loaded root (or erofs, if the RAM-root spike in Open Questions doesn't pan out), `/usr` merged, Landlock baseline active, earlyoom running, config sourced from `/usr/share/defaults` on first boot, persistent state on a real disk-backed bcachefs volume.
2. `minimal` profile boots on an erofs root, same usrmerge/Landlock/earlyoom baseline, no GPUI/Wayland packages present, no persistent disk assumed.
3. Both profiles build for x86_64 and aarch64; CI matrix (`ci-rust-core.sh`, `ci-zig.sh`, `ci-os-appliance.sh`) green on both.
4. A tagged GitHub Release carries prebuilt initramfs+kernel+rootfs for both arches, both profiles, built against whatever the latest stable kernel is at release time.
5. Kernel boots with `PREEMPT_RT` on at least one arch in QEMU (`bench-boot.sh` still produces a number), on the current latest-stable base.
6. A scheduled job bumps the kernel version automatically and CI catches breakage before it ships.
7. No inauguration dependency anywhere in the appliance boot path; no work started on the inauguration side until its own tracker shows a Linux hosted target in progress.
