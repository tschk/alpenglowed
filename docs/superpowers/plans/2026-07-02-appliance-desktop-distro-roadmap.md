# Alpenglow Appliance + Alpenglowed Desktop Distro — Implementation Plan

> **For agent:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Split appliance (`minimal`) vs desktop (`standard`) root FS story, usrmerge, Landlock-by-default + seccomp, `PREEMPT_RT`, earlyoom, stateless `/usr/share/defaults`, x86_64+aarch64-only matrix, precompiled release artifacts, ypkg-style Oil recipes, and a bounded (non-boot-path) inauguration/Zig track.

**Spec:** `docs/superpowers/specs/2026-07-02-appliance-desktop-distro-design.md`

**Repos touched:** `alpenglow` (all appliance/kernel/pkg-mgr work), `alpenglowed` (stateless defaults, this plan/spec).

---

## Phase order

```
Phase 0  Groundwork / audits            (no behavior change, de-risks everything after)
Phase 1  usrmerge                       (alpenglow)
Phase 2  Landlock baseline + seccomp    (alpenglow)
Phase 3  earlyoom                       (alpenglow)
Phase 4  Stateless /usr/share/defaults  (alpenglowed + alpenglow)
Phase 5  Arch matrix narrowing          (alpenglow)
Phase 6  PREEMPT_RT kernel              (alpenglow)
Phase 7  Appliance root FS decision     (alpenglow) — depends on Phase 0 audit + open question in spec
Phase 8  Precompiled release artifacts  (alpenglow)
Phase 9  Oil declarative recipe format  (alpenglow / oil)
Phase 10 Inauguration + Zig track       (alpenglow, exploratory, no boot-path dependency)
```

Each phase is independently shippable and independently revertible. Do not start a phase whose "Depends on" isn't done.

---

### Phase 0: Groundwork audits

**Files:** none changed; produces findings that gate Phase 1 and Phase 7.

**Steps:**

1. `grep -rn '"/bin/\|"/sbin/\|/lib/\|/lib64/' system/backends/appliance/` (alpenglow) — list every hardcoded pre-usrmerge path in dinit units, `configure-rootfs.sh`, Oil package payload manifests. This answers the spec's "usrmerge ordering" open question.
2. Check current erofs/squashfs fallback code path actually mounts read-only root end-to-end in QEMU today (`scripts/boot-native.sh` — is there an `EROFS=1`/`SQUASHFS=1` toggle, or is "fallback" doc-only?). Record the answer; if doc-only, Phase 7 starts from zero, not from an existing toggle.
3. Confirm kernel version pin (`Linux 7.0.12` per `AGENTS.md`) against available RT patchset mirrors (`linux-rt-devel`, distro RT kernel trees). Record nearest matching RT tag for Phase 6.
4. `grep -rn 'aarch64' scripts/ system/backends/appliance/` — list every place aarch64 is documented but not wired, to scope Phase 5 accurately.

**Verify:** four short findings notes appended to this plan's Phase 0 section (or a scratch `docs/superpowers/specs/2026-07-02-phase0-findings.md`) before Phase 1/7 start.

---

### Phase 1: usrmerge

**Depends on:** Phase 0.1

**Files:** `system/backends/appliance/scripts/configure-rootfs.sh`, dinit units under `system/backends/appliance/dinit/`, `system/backends/appliance/scripts/*.sh` referencing old paths.

**Steps:**

1. In `configure-rootfs.sh`, create `/usr/bin`, `/usr/sbin`, `/usr/lib`, `/usr/lib64` as real dirs; make `/bin`, `/sbin`, `/lib`, `/lib64` symlinks to their `usr/`-prefixed equivalents (order matters: create targets, populate, then symlink, or populate directly under `/usr/...` and symlink from the start — pick whichever the current toybox/oksh install steps make simpler).
2. Update every path found in Phase 0.1 to the canonical `/usr/...` form.
3. Keep old paths working via the symlinks (that's the point of usrmerge — no breakage, just canonicalization).
4. Run `./scripts/ci-os-appliance.sh`.

**Verify:** `./scripts/ci-os-appliance.sh` green; `scripts/boot-native.sh` boots to shell; `ls -la /bin` in the booted image shows a symlink to `usr/bin`.

---

### Phase 2: Landlock baseline + seccomp

**Depends on:** none (parallel to Phase 1)

**Files:** `system/backends/appliance/kernel/minimal.config`, `system/backends/appliance/kernel/alpenglow-qemu-minimal.config`, `system/backends/appliance/dinit/*` (dropbear, chronyd, dnsmasq, crond first).

**Steps:**

1. Promote `CONFIG_SECURITY_LANDLOCK=y` (already on in `alpenglow-internet-appliance.config`) into the shared minimal/baseline kernel config so every profile has it, not just the internet-appliance variant.
2. For dropbear, chronyd, dnsmasq, crond dinit services: add seccomp-bpf filter invocation before `exec` (or via a wrapper) restricting to each daemon's known syscall set. Start with an allow-list built from `strace -f -c` during a normal run in QEMU.
3. Leave `CONFIG_SECURITY_SELINUX=n` — explicitly a non-goal per spec.

**Verify:** kernel builds with Landlock on for all profiles; each hardened service still starts and does its job (SSH login works, DNS resolves via dnsmasq, cron fires, NTP syncs) under QEMU boot.

---

### Phase 3: earlyoom

**Depends on:** none

**Files:** `system/backends/appliance/dinit/earlyoom` (new), `system/backends/appliance/packages-runtime.txt`, `system/backends/appliance/scripts/configure-rootfs.sh` (service registration, mirrors how other `boot.d` services are linked at line ~149).

**Steps:**

1. Vendor upstream `earlyoom` (static musl build) the same way other minimal binaries are fetched/built in this tree — check `packages-runtime.txt` for the existing pattern before inventing a new one.
2. Add dinit service file, default flags (reasonable memory/swap percentage thresholds for a diskless RAM-root system — no swap by default, so tune thresholds accordingly).
3. Link into `boot.d` for both `minimal` and `standard` profiles.

**Verify:** `./scripts/ci-os-appliance.sh` green; earlyoom process visible after boot (`ps` in QEMU shell); manual OOM-pressure test (e.g. `stress-ng --vm` if available, else a small allocator loop) triggers a kill before the hard kernel OOM killer does.

---

### Phase 4: Stateless `/usr/share/defaults`

**Depends on:** Phase 1 (usrmerge — defaults tree lives under the merged `/usr`)

**Files:** `alpenglowed/src/` (config loader), `alpenglow/system/backends/appliance/rootfs-overlay/usr/share/defaults/alpenglowed/` (new), `configure-rootfs.sh`, `alpenglow-session-start`.

**Steps:**

1. In `alpenglow`, add `/usr/share/defaults/alpenglowed/config.toml` (and any other default asset) to the rootfs overlay — this is the immutable, shipped-with-the-image copy.
2. On first boot (session-start script, before launching `alpenglowed`), if `/etc/alpenglowed/config.toml` doesn't exist, copy it down from `/usr/share/defaults/alpenglowed/`. Only copy (not symlink) files the user is expected to edit; symlink files that should never diverge.
3. In `alpenglowed`, add a `factory-reset` action (plugin command, e.g. `> factory-reset`) that removes `/etc/alpenglowed/*` and re-triggers the copy-down on next launch.
4. Document the pattern in `alpenglowed/README.md` (new section) so it's reusable for the next config surface (greetd, dinit-level appliance config) instead of one-off.

**Verify:** fresh boot has no `/etc/alpenglowed`, gets defaults copied on first launch; editing `/etc/alpenglowed/config.toml` then running `factory-reset` restores shipped defaults; `cargo test` in alpenglowed still passes.

---

### Phase 5: Arch matrix narrowing

**Depends on:** Phase 0.4

**Files:** kernel configs (drop any non-x86_64/aarch64 arch options if present), `.github/workflows/ci.yml`, `scripts/ci-*.sh`, `system/backends/appliance/kernel/README.md`, root `AGENTS.md`/`CLAUDE.md` arch row.

**Steps:**

1. Wire aarch64 into `boot-native.sh` and CI the same way x86_64 already works (cross-compile toybox/toolchain, QEMU `qemu-system-aarch64` boot path) — use `ultramarine` (has qemu+kvm) for aarch64 QEMU testing per `AGENTS.md` SSH hosts table.
2. Update CI matrix to run both arches (or at minimum: x86_64 native + aarch64 cross-build-only if aarch64 QEMU isn't ready yet — be honest about what "supported" means at each step, don't claim boot-tested until it is).
3. Update `AGENTS.md`/`CLAUDE.md` "Arch | Generic — x86_64, aarch64, etc." row to "x86_64, aarch64" once CI backs it.

**Verify:** CI green on both arches; `AGENTS.md` claim matches CI reality (no "generic" language left over).

---

### Phase 6: `PREEMPT_RT` kernel

**Depends on:** Phase 0.3

**Files:** `system/backends/appliance/kernel/patches/series`, `system/backends/appliance/kernel/patch-series/*.json`, `system/backends/appliance/kernel/*.config`, `scripts/ci-glowfs-kernel-module.sh` (GlowFS module must still compile against RT tree).

**Steps:**

1. Add the RT patch series identified in Phase 0.3 to `patches/series` following the existing `bore-style.json` patch-series convention.
2. Flip `# CONFIG_PREEMPT is not set` → `CONFIG_PREEMPT_RT=y` (or the RT-specific Kconfig symbol the chosen patchset exposes) in the relevant configs; keep `alpenglow-qemu-minimal.config`'s existing lazy-preempt as a fallback config for hosts where RT isn't wanted (dev/CI speed) rather than deleting it.
3. Rebuild GlowFS module against the RT-patched tree; fix any RT-specific locking assumptions the module violates (spinlock-in-atomic-context patterns are the usual RT breakage).

**Verify:** `./scripts/ci-glowfs-kernel-module.sh` green against RT config; `./scripts/bench-boot.sh` still produces a boot-time number on the RT kernel.

---

### Phase 7: Appliance root FS decision

**Depends on:** Phase 0.2, Phase 1

**Files:** `system/backends/appliance/scripts/configure-rootfs.sh`, `docs/architecture/glowfs.md` / new `docs/architecture/appliance-rootfs.md`, kernel configs (erofs/squashfs/bcachefs Kconfig symbols).

**Steps:**

1. Land erofs (or squashfs, whichever Phase 0.2 shows closer to already working) as the **default** root FS for `minimal` profile only. `standard` profile keeps GlowFS — do not touch that path.
2. Get `minimal` profile booting read-only on the new default in QEMU, both arches.
3. Open a tracked follow-up (new spec doc, not this one) for bcachefs evaluation as a stretch goal — do not block this phase on it per spec's recommendation.

**Verify:** `minimal` profile boots read-only root on erofs/squashfs in QEMU; `standard` profile unaffected (still GlowFS, still boots); `docs/architecture/appliance-rootfs.md` documents which profile uses which FS and why.

---

### Phase 8: Precompiled release artifacts

**Depends on:** Phase 5 (arch matrix), Phase 1/2/3 (so releases carry the hardened baseline)

**Files:** `.github/workflows/ci.yml` (or new `release.yml`), release packaging script (new, e.g. `scripts/package-release.sh`).

**Steps:**

1. New script that assembles initramfs + kernel + rootfs image per (profile × arch) into a release-ready archive, reusing existing build scripts rather than duplicating logic.
2. New GitHub Actions workflow triggered on tag push: build all (profile × arch) combinations, attach to a GitHub Release.
3. Sign artifacts (whatever signing mechanism `secure-boot.md` already documents — reuse, don't invent a new one) — check `docs/secure-boot.md` before adding new key material.
4. Document download+boot instructions in top-level `README.md`.

**Verify:** a tagged release produces downloadable artifacts for both profiles and both arches; a fresh machine can boot from the downloaded artifact without running the build scripts.

---

### Phase 9: Oil declarative recipe format (ypkg-inspired)

**Depends on:** none

**Files:** `system/oil/src/` (recipe parser/model), new recipe schema doc under `system/oil/docs/` or top-level `docs/`.

**Steps:**

1. Design a YAML recipe schema for Oil packages (name, version, source URL, build steps, install paths) modeled on ypkg's `package.yml` shape but targeting Oil's existing APK output format — no eopkg, no Vala.
2. Implement a recipe → Oil-internal-package-spec loader in `system/oil/src`.
3. Migrate one or two existing packages (whatever Oil currently packages by hand, if anything) to the new recipe format as a proof.
4. `cargo test -p oil` (or whatever the crate's actually named — check `Cargo.toml`).

**Verify:** `cargo check`/`cargo test` on the oil crate green; one real package builds end-to-end from a `.yml` recipe through Oil into an installable APK.

---

### Phase 10: Inauguration + Zig track (exploratory, non-boot-path)

**Depends on:** none; purely additive, never blocks the boot path.

**Files:** new, isolated — do not touch anything on the boot-critical path.

**Steps:**

1. Pick one narrow build-time tool candidate (e.g. Oil recipe → APK metadata generator from Phase 9, or a schema-to-Rust codegen step) as the inauguration experiment target.
2. Check inauguration's own status table (`../inauguration/README.md`) for "Linux (x86_64/aarch64) hosted target: working" before writing real code against it — if still "planned," stop here and just leave this phase queued.
3. If Linux-hosted `in` is ready: implement the tool using `in`, gated behind an opt-in build flag, with the existing Rust/cargo path as the default/fallback — never make the appliance build depend on inauguration being installed.
4. Apply the "prefer Zig for new components" policy (spec item 11) to the next genuinely new small daemon/CLI helper that comes up in any other phase — this task itself doesn't invent a new component to satisfy that policy artificially.

**Verify:** no CI job requires `in` or Zig-for-this-specific-task to pass; the experiment is documented as optional in whatever doc it lands in.

---

## Estimated effort

| Phase | Size | Notes |
|-------|------|-------|
| 0 | Small | Pure audit, gates everything else |
| 1 | Medium | Touches many hardcoded paths |
| 2 | Medium | Landlock trivial; per-service seccomp allow-lists take iteration |
| 3 | Small | Mostly packaging + one dinit unit |
| 4 | Small–medium | New convention, but small surface area |
| 5 | Medium–large | Real aarch64 QEMU wiring, not just config |
| 6 | Large | RT patchset integration + GlowFS module fixes |
| 7 | Medium | Blocked on Phase 0.2 finding; bcachefs itself out of scope |
| 8 | Medium | New CI workflow + signing reuse |
| 9 | Medium | New parser/schema, one crate |
| 10 | Small (or zero) | Only proceeds if inauguration is actually ready |

**Suggested first milestone:** Phases 0 → 1 → 3 → 2 (cheap wins, all on `alpenglow`, no open questions blocking them).

**Second milestone:** Phase 4 (stateless defaults — the original ask) + Phase 5 (arch matrix).

**Third milestone:** Phase 6 (RT kernel) + Phase 7 (appliance root FS) — the two genuinely open-ended items.

**Ongoing/parallel:** Phase 8 once 1/2/3/5 are in; Phase 9 anytime; Phase 10 only when inauguration's status table says so.
