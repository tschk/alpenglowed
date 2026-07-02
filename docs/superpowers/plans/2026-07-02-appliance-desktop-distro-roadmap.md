# Alpenglow Appliance + Alpenglowed Desktop Distro — Implementation Plan

> **For agent:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Split appliance (`minimal`, erofs, pure RAM) vs desktop (`standard`, bcachefs hybrid RAM-root + disk-backed state) root FS story, GlowFS parked, rolling latest-stable kernel with `PREEMPT_RT`, usrmerge, Landlock-by-default + seccomp, earlyoom, stateless `/usr/share/defaults`, x86_64+aarch64-only matrix, precompiled release artifacts, ypkg-style Oil recipes, concrete next Zig components, and a bounded (non-boot-path) inauguration track.

**Spec:** `docs/superpowers/specs/2026-07-02-appliance-desktop-distro-design.md`

**Repos touched:** `alpenglow` (all appliance/kernel/pkg-mgr work), `alpenglowed` (stateless defaults, this plan/spec).

---

## Phase order

```
Phase 0  Groundwork / audits              (no behavior change, de-risks everything after)
Phase 1  usrmerge                         (alpenglow)
Phase 2  Landlock baseline + seccomp      (alpenglow)
Phase 3  earlyoom                         (alpenglow)
Phase 4  Stateless /usr/share/defaults    (alpenglowed + alpenglow)
Phase 5  Arch matrix narrowing            (alpenglow)
Phase 6  Rolling latest-stable kernel     (alpenglow) — Zig version-bump tool
Phase 7  PREEMPT_RT                       (alpenglow) — depends on Phase 6
Phase 8  Root FS: erofs + bcachefs hybrid (alpenglow) — GlowFS parked, not touched
Phase 9  Precompiled release artifacts    (alpenglow)
Phase 10 Oil declarative recipe format    (alpenglow / oil)
Phase 11 Inauguration track               (alpenglow, exploratory, no boot-path dependency)
```

Each phase is independently shippable and independently revertible. Do not start a phase whose "Depends on" isn't done. GlowFS (`system/glowfs/`) is not touched by any phase — it's parked, not deleted, not migrated.

---

### Phase 0: Groundwork audits

**Files:** none changed; produces findings that gate later phases.

**Steps:**

1. `grep -rn '"/bin/\|"/sbin/\|/lib/\|/lib64/' system/backends/appliance/` (alpenglow) — list every hardcoded pre-usrmerge path in dinit units, `configure-rootfs.sh`, Oil package payload manifests. This answers the spec's "usrmerge ordering" open question. Gates Phase 1.
2. Check current erofs/squashfs fallback code path actually mounts read-only root end-to-end in QEMU today (`scripts/boot-native.sh` — is there an `EROFS=1`/`SQUASHFS=1` toggle, or is "fallback" doc-only?). Record the answer; if doc-only, Phase 8's appliance half starts from zero, not from an existing toggle. Gates Phase 8.
3. Confirm what "latest stable" is *right now* at kernel.org and how far it is from the currently-pinned `7.0.12`. Gates Phase 6.
4. `grep -rn 'aarch64' scripts/ system/backends/appliance/` — list every place aarch64 is documented but not wired, to scope Phase 5 accurately.
5. Spike: build a minimal bcachefs image, load it onto a RAM block device (`brd`) in QEMU, mount read-mostly, measure boot-time overhead vs the current erofs/squashfs path. This directly answers the spec's "bcachefs-as-RAM-root feasibility" open question and decides which branch of Phase 8 (desktop half) actually runs.

**Verify:** five short findings notes appended to this plan's Phase 0 section (or a scratch `docs/superpowers/specs/2026-07-02-phase0-findings.md`) before Phase 1/6/8 start.

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

1. Vendor upstream `earlyoom` (static musl build) the same way other minimal binaries are fetched/built in this tree — check `packages-runtime.txt` for the existing pattern before inventing a new one. Upstream C binary first; only write a Zig reimplementation if it doesn't fit the static-size budget.
2. Add dinit service file, default flags (reasonable memory/swap percentage thresholds for a diskless RAM-root system — no swap by default on the appliance profile, so tune thresholds accordingly; desktop profile may have swap once Phase 8's persistent layer lands).
3. Link into `boot.d` for both `minimal` and `standard` profiles.

**Verify:** `./scripts/ci-os-appliance.sh` green; earlyoom process visible after boot (`ps` in QEMU shell); manual OOM-pressure test (e.g. `stress-ng --vm` if available, else a small allocator loop) triggers a kill before the hard kernel OOM killer does.

---

### Phase 4: Stateless `/usr/share/defaults`

**Depends on:** Phase 1 (usrmerge — defaults tree lives under the merged `/usr`)

**Files:** `alpenglowed/src/` (config loader), `alpenglow/system/backends/appliance/rootfs-overlay/usr/share/defaults/alpenglowed/` (new), `configure-rootfs.sh`, `alpenglow-session-start`. Optional new Zig helper: `system/defaults-init-zig/` (copy-down + factory-reset logic as a tiny static tool, if the shell-script version outgrows `sh`).

**Steps:**

1. In `alpenglow`, add `/usr/share/defaults/alpenglowed/config.toml` (and any other default asset) to the rootfs overlay — this is the immutable, shipped-with-the-image copy.
2. On first boot (session-start script, before launching `alpenglowed`), if `/etc/alpenglowed/config.toml` doesn't exist, copy it down from `/usr/share/defaults/alpenglowed/`. Only copy (not symlink) files the user is expected to edit; symlink files that should never diverge. Start as a shell function in `alpenglow-session-start`; promote to a standalone Zig helper only if the logic grows past what's comfortable in `sh`.
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

### Phase 6: Rolling latest-stable kernel

**Depends on:** Phase 0.3

**Files:** `scripts/check-kernel-latest.sh` (new), `scripts/boot-native.sh` (`KERNEL_VERSION` default), `.github/workflows/ci.yml` (new scheduled job), new Zig tool `system/kernel-bump-zig/` (or a plain shell script if that's genuinely simpler — see ladder below).

**Steps:**

1. Write the version-check tool: fetches kernel.org's stable release feed, compares against the currently-pinned `KERNEL_VERSION`, reports whether a bump is available. Ladder per the lazy-first rule: try a ~20-line shell script with `curl` + the kernel.org JSON/RSS feed first; only reach for a Zig binary if the shell version needs more logic than is comfortable (e.g. real semver comparison, changelog diffing for the config-diff part).
2. Wire it into a scheduled GitHub Actions job (e.g. weekly) that, on finding a new stable release, bumps `KERNEL_VERSION` in `boot-native.sh` and related scripts, then opens a PR — gated on the full CI matrix passing before merge, per the spec's Open Questions recommendation (option a).
3. Do **not** auto-merge without CI passing. "No matter what" means "always try," not "skip verification."
4. Remove the hardcoded `7.0.12`/`Linux 7.0` mentions from `AGENTS.md`, `CLAUDE.md`, `readme.md`, `docs/v0-architecture.md`, `docs/architecture-support.md` — replace with "tracks kernel.org latest stable" language plus a pointer to whatever the bump-PR history shows as current.

**Verify:** `scripts/check-kernel-latest.sh` correctly reports "up to date" or "bump available" against a known kernel.org state; a manually-triggered run of the scheduled job produces a real PR bumping the version; that PR's CI run is green before merge.

---

### Phase 7: `PREEMPT_RT`

**Depends on:** Phase 6 (need the rolling-version mechanism in place first, since the RT patch source has to be re-resolved against *whatever current latest-stable is*, not a fixed 7.0.12)

**Files:** `system/backends/appliance/kernel/patches/series`, `system/backends/appliance/kernel/patch-series/*.json`, `system/backends/appliance/kernel/*.config`.

**Steps:**

1. At implementation time, look up the RT patchset (or in-tree `PREEMPT_RT` support, since RT has been progressively merging into mainline) matching whatever Phase 6 reports as current latest-stable — this is a live lookup, not a value copied from this plan.
2. Add the resolved RT patch series to `patches/series` following the existing `bore-style.json` patch-series convention, or flip the in-tree Kconfig symbol directly if mainline has absorbed RT support for that release by the time this runs.
3. Flip `# CONFIG_PREEMPT is not set` → `CONFIG_PREEMPT_RT=y` in the relevant configs; keep `alpenglow-qemu-minimal.config`'s existing lazy-preempt as a fallback config for hosts where RT isn't wanted (dev/CI speed) rather than deleting it.
4. Because the kernel version is now rolling (Phase 6), add an RT-compatibility check to the version-bump job from Phase 6: if a new stable release lands without a corresponding RT patch yet, the bump PR should flag "RT patch pending" rather than silently dropping `PREEMPT_RT` or blocking the whole bump.

**Verify:** `./scripts/bench-boot.sh` still produces a boot-time number on the RT kernel; the version-bump job from Phase 6 correctly flags an RT gap if one exists at test time.

---

### Phase 8: Root FS — erofs (appliance) + bcachefs hybrid (desktop)

**Depends on:** Phase 0.2, Phase 0.5, Phase 1

**Files:** `system/backends/appliance/scripts/configure-rootfs.sh`, `system/backends/appliance/scripts/mount-state.sh`, new `docs/architecture/appliance-rootfs.md` and `docs/architecture/desktop-rootfs.md`, kernel configs (erofs/bcachefs Kconfig symbols).

**Steps:**

1. **Appliance (`minimal`):** land erofs as the default root FS. Pure RAM-loaded, read-only, no persistent disk assumed. This half has no open question — build it.
2. **Desktop (`standard`) root image:** branch on Phase 0.5's spike result.
   - If the bcachefs-on-`brd` spike worked within a reasonable boot-time budget: build the desktop root image as bcachefs, loaded into a RAM block device at boot.
   - If it didn't: use erofs for the desktop root image too (same as appliance), and don't force the issue — this was explicitly flagged as at-risk in the spec, not a hard requirement.
3. **Desktop (`standard`) persistent layer:** replace the ext4 state partition (`mount-state.sh`) with bcachefs regardless of how step 2 landed — this half was never at risk. Installed Oil packages, user files, WiFi/network config move here. Use bcachefs subvolumes so Phase 4's factory-reset can eventually snapshot-and-rollback instead of just delete-and-recopy (stretch goal, don't block Phase 4 on it).
4. Document which profile uses which FS and why, including the bcachefs-root branch outcome, in the two new docs.

**Verify:** `minimal` profile boots read-only root on erofs in QEMU, both arches; `standard` profile boots on whichever root format step 2 resolved to, with a working bcachefs persistent volume mounted for state; a file written to the state volume survives a reboot (persistent) while a file written to `/tmp` on the RAM root does not (correctly diskless).

---

### Phase 9: Precompiled release artifacts

**Depends on:** Phase 5 (arch matrix), Phase 1/2/3 (so releases carry the hardened baseline), Phase 6 (so release notes can state which kernel stable version is in the artifact)

**Files:** `.github/workflows/ci.yml` (or new `release.yml`), release packaging script (new, e.g. `scripts/package-release.sh` or a Zig tool `system/release-packager-zig/` if the shell version gets unwieldy).

**Steps:**

1. New script that assembles initramfs + kernel + rootfs image per (profile × arch) into a release-ready archive, reusing existing build scripts rather than duplicating logic.
2. New GitHub Actions workflow triggered on tag push: build all (profile × arch) combinations, attach to a GitHub Release, tag the release notes with the exact kernel stable version baked in (ties to Phase 6).
3. Sign artifacts (whatever signing mechanism `secure-boot.md` already documents — reuse, don't invent a new one) — check `docs/secure-boot.md` before adding new key material.
4. Document download+boot instructions in top-level `README.md`.

**Verify:** a tagged release produces downloadable artifacts for both profiles and both arches; a fresh machine can boot from the downloaded artifact without running the build scripts.

---

### Phase 10: Oil declarative recipe format (ypkg-inspired)

**Depends on:** none

**Files:** `system/oil/src/` (recipe parser/model), new recipe schema doc under `system/oil/docs/` or top-level `docs/`.

**Steps:**

1. Design a YAML recipe schema for Oil packages (name, version, source URL, build steps, install paths) modeled on ypkg's `package.yml` shape but targeting Oil's existing APK output format — no eopkg, no Vala.
2. Implement a recipe → Oil-internal-package-spec loader in `system/oil/src`.
3. Migrate one or two existing packages (whatever Oil currently packages by hand, if anything) to the new recipe format as a proof.
4. `cargo test -p oil` (or whatever the crate's actually named — check `Cargo.toml`).

**Verify:** `cargo check`/`cargo test` on the oil crate green; one real package builds end-to-end from a `.yml` recipe through Oil into an installable APK.

---

### Phase 11: Inauguration track (exploratory, non-boot-path)

**Depends on:** none; purely additive, never blocks the boot path.

**Files:** new, isolated — do not touch anything on the boot-critical path.

**Steps:**

1. Before writing any code: check `../inauguration/README.md`'s cross-platform table and `../inauguration/todo.md`. As of this plan's writing, Linux (x86_64/aarch64) hosted target is listed **"planned"** with zero open todo items toward it — meaning there is currently nothing to integrate against. Re-check at implementation time; if still "planned," stop here and leave this phase queued, don't invent work.
2. If Linux-hosted `in` has since become real: pick one narrow build-time tool candidate (e.g. Oil recipe → APK metadata generator from Phase 10, or a schema-to-Rust codegen step) as the experiment target.
3. Implement the tool using `in`, gated behind an opt-in build flag, with the existing Rust/cargo path as the default/fallback — never make the appliance build depend on inauguration being installed.
4. This roadmap does not fund upstream work on inauguration's Linux target itself — that's a separate decision for whoever owns `../inauguration`'s priorities.

**Verify:** no CI job requires `in` to pass; the experiment (if attempted at all) is documented as optional in whatever doc it lands in.

---

## Estimated effort

| Phase | Size | Notes |
|-------|------|-------|
| 0 | Small–medium | Audit + one real spike (0.5, bcachefs-on-RAM); gates everything after |
| 1 | Medium | Touches many hardcoded paths |
| 2 | Medium | Landlock trivial; per-service seccomp allow-lists take iteration |
| 3 | Small | Mostly packaging + one dinit unit |
| 4 | Small–medium | New convention, but small surface area |
| 5 | Medium–large | Real aarch64 QEMU wiring, not just config |
| 6 | Medium | New automation surface (version-check tool + scheduled CI job) |
| 7 | Large | RT patchset integration; recurring re-resolution cost baked into the automation, not a one-time cost |
| 8 | Medium–large | Two real FS migrations (erofs + bcachefs); desktop-root branch depends on the Phase 0.5 spike outcome |
| 9 | Medium | New CI workflow + signing reuse |
| 10 | Medium | New parser/schema, one crate |
| 11 | Small (or zero) | Only proceeds if inauguration is actually ready |

**Suggested first milestone:** Phases 0 → 1 → 3 → 2 (cheap wins, all on `alpenglow`, no open questions blocking them).

**Second milestone:** Phase 4 (stateless defaults — the original ask) + Phase 5 (arch matrix).

**Third milestone:** Phase 6 (rolling kernel automation) → Phase 7 (RT) → Phase 8 (root FS split) — the three genuinely open-ended items, in that dependency order.

**Ongoing/parallel:** Phase 9 once 1/2/3/5/6 are in; Phase 10 anytime; Phase 11 only when inauguration's tracker says so.
