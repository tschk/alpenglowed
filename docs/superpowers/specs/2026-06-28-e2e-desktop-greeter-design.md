# E2E Alpenglow Desktop + GPUI Greeter — Design

**Date:** 2026-06-28  
**Repos:** `alpenglowed` (shell + greeter + optional compositor), `alpenglow` (distro integration)

## Goal

One coherent path from **power-on → graphical login → Wayland session → alpenglowed shell → launch apps in real panes**, testable in QEMU (`../alpenglow/scripts/boot-native.sh --graphical`) and on appliance rootfs.

## Current state (from recent work)

| Area | Status |
|------|--------|
| Launcher, plugins (shell, calc, files, web, emoji, clipboard, spotify) | Done in tree |
| Notifications, terminal pane, settings | Done |
| Embedded smithay (`--features compositor -- --compositor`) | Thread + xdg-shell; events → layout panes; **no buffer paint into GPUI yet** |
| Distro | `greetd` + `agreety` planned in packages; **no `etc/greetd/config.toml` in overlay**; graphical QEMU uses **cage + alpenglowed** without greetd session |
| Login UI | **None in GPUI**; serial getty only in minimal boot |

## Target architecture

```
boot (dinit)
  → seatd
  → greetd                    # session manager
       → alpenglowed --greeter   # GPUI greeter (greetd IPC + PAM)
            → on success: greetd starts user session
                 → alpenglow-session-start
                      → WAYLAND_DISPLAY set
                      → alpenglowed [--compositor]   # one binary: compositor optional
                      → foot / launched apps as Wayland clients
```

**Compositor modes (pick per image, not both at once):**

1. **Integrated (target):** `alpenglowed --compositor` — smithay socket `alpenglowed-0`, GPUI shell same process (already sketched).
2. **Legacy QEMU/dev:** cage as parent compositor, alpenglowed as fullscreen client — keep until integrated path is default in `boot-native.sh`.

## Greeter design

### Requirements

- Fullscreen GPUI: username, password, error line, power actions (suspend/reboot/shutdown) via existing `DesktopAction` / `loginctl` where available.
- Works as **greetd greeter** (not a second login daemon).
- Musl-friendly where possible; greeter may be same binary as shell with `--greeter` flag.

### Implementation approach (recommended)

Use the **`greetd_ipc`** crate (or equivalent minimal message codec) in a small `src/greeter.rs`:

- `main()` early branch: `--greeter` → run `greeter::run()` and **do not** start desktop windows.
- Greeter connects to greetd, handles `create_session` / auth flow, submits credentials via IPC (PAM handled by greetd on session start — greeter only collects user/pass and responds to prompts).
- On success, greetd runs `/usr/local/bin/alpenglow-session-start` (cage + alpenglowed shell).
- Greeter binary: **`alpenglow-greeter`** crate (not `alpenglowed --greeter`).
- Autologin: `etc/greetd/config-autologin.toml` or `ALPENGLOW_AUTOLOGIN=1` at image build.

**Alternative (rejected for v1):** Keep `agreety` TUI — fails “GPUI login” requirement.

**Alternative (deferred):** Standalone `alpenglow-greeter` binary — same code, extra packaging; only if greetd config needs a smaller binary.

### Security

- No password logging; clear fields on failed auth.
- Greeter runs as dedicated user `greeter` (greetd default); session runs as logged-in user.

## Session / E2E integration (`alpenglow` repo)

### Files to add or fix

| Path | Change |
|------|--------|
| `system/backends/appliance/rootfs-overlay/etc/greetd/config.toml` | `[default_session]` command → `session-init`; `user` = `greeter`; greeter command → `alpenglowed --greeter` |
| `system/backends/appliance/scripts/alpenglow-session-start` | If `ALPENGLOW_COMPOSITOR=1` or config flag: `exec alpenglowed --compositor`; else existing cage path |
| `scripts/boot-native.sh` (`GRAPHICAL=1`) | Option A: greetd + session-init instead of auto-starting alpenglowed on tty; Option B: auto-login dev user for CI + manual greetd for “desktop” profile |
| `build-alpenglowed-glibc.sh` | Pass `--features compositor` when building for graphical rootfs |
| `system/backends/appliance/dinit/*` | Align `velox`/`alpenglowed` units with session model (compositor-in-shell vs cage) |

### Display metadata

Update `configure-rootfs.sh` `display` JSON when integrated compositor is default:

```json
"compositor": "alpenglowed",
"greeter": "alpenglowed"
```

## Compositor completion (Phase D — minimum for E2E)

For “real desktop” acceptance:

1. **Poll loop** — already calls `poll_compositor` from render path; keep.
2. **Launch** — `PluginAction::Launch` sets `WAYLAND_DISPLAY` to compositor socket when `ALPENGLOW_COMPOSITOR=1`.
3. **Paint (MVP)** — v1 can be **placeholder pane content + live window title** from compositor events; v1.1 adds shm buffer blit or child surface embedding in GPUI (larger task).
4. **Input** — forward keyboard to focused surface via `CompositorCommand` (stub exists).

DRM/KMS direct scanout remains **out of scope** for this spec.

## Success criteria

1. QEMU `--graphical`: OVMF + display shows **GPUI greeter**, login as `root`/`alpenglow` (test user), lands in alpenglowed shell.
2. `foot` or `> foot` launches visible client (cage path or `--compositor` path).
3. `cargo test` + `scripts/ci-os-appliance.sh` (alpenglow) still pass after greetd config added.
4. Documented dev paths: macOS UI dev; Linux QEMU E2E; SSH chimera/ultramarine for compositor builds.

## Out of scope (follow-ups)

- smithay buffer rendering inside GPUI panes (full WM parity)
- musl static greeter on same binary as glibc GPU shell (may stay glibc for graphical rootfs)
- Replacing elogind/logind

## Open decision

**QEMU graphical boot:** auto-login dev session (faster CI) vs always greetd (true E2E)?

**Recommendation:** `BUILD_PROFILE=desktop` → greetd required; `GRAPHICAL=1` dev boot → env `ALPENGLOW_SKIP_GREETER=1` auto-starts session for iteration.