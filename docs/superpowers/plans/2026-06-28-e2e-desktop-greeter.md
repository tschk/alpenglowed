# E2E Desktop + GPUI Greeter — Implementation Plan

> **For agent:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** GPUI greeter via greetd, session starts alpenglowed (optional embedded compositor), QEMU graphical path proves login → shell → launch client.

**Architecture:** Single `alpenglowed` binary; `--greeter` for greetd IPC UI; `--compositor` for smithay thread; `alpenglow` rootfs wires greetd config + session-init.

**Tech stack:** Rust, GPUI/Crepuscularity, `greetd_ipc`, smithay (optional feature), shell scripts in `../alpenglow`.

**Spec:** `docs/superpowers/specs/2026-06-28-e2e-desktop-greeter-design.md`

---

## File map

| File | Responsibility |
|------|----------------|
| `alpenglowed/Cargo.toml` | Add `greetd_ipc` dep (greeter feature or always) |
| `alpenglowed/src/greeter.rs` | GPUI greeter + greetd IPC loop |
| `alpenglowed/src/main.rs` | `--greeter` dispatch; `Launch` sets Wayland env when compositor |
| `alpenglowed/src/compositor.rs` | `CompositorCommand::FocusSurface` / key forward (minimal) |
| `alpenglow/../alpenglow/.../etc/greetd/config.toml` | greetd session + greeter commands |
| `alpenglow/scripts/boot-native.sh` | Graphical profile + skip-greeter dev flag |
| `alpenglow/.../alpenglow-session-start` | compositor vs cage exec |
| `alpenglow/.../build-alpenglowed-glibc.sh` | `--features compositor` |

---

### Task 1: greetd config in alpenglow overlay

**Files:** Create `system/backends/appliance/rootfs-overlay/etc/greetd/config.toml`

**Steps:**

1. Add TOML: greeter `command = "/usr/bin/alpenglowed-run.sh --greeter"` (or direct binary path used by image).
2. `[default_session]` `command = "/opt/alpenglow/session-init"` (or `/usr/local/bin/alpenglow-session-start`).
3. Set `user = "greeter"` for greeter section per greetd docs.
4. Run `../alpenglow/scripts/ci-os-appliance.sh` — extend assert if needed for config file presence.
5. Commit in alpenglow repo.

**Verify:** `test -f system/backends/appliance/rootfs-overlay/etc/greetd/config.toml`

---

### Task 2: `greetd_ipc` + `--greeter` skeleton

**Files:** `Cargo.toml`, `src/greeter.rs`, `src/main.rs`

**Steps:**

1. Add dependency `greetd_ipc` (version matching greetd 0.10.x in alpenglow).
2. `mod greeter;` in `main.rs`.
3. `--help` documents `--greeter`.
4. `main`: if `--greeter`, `greeter::run()` and return.
5. `greeter::run()`: connect IPC, log errors to stderr, exit non-zero on failure.
6. `cargo test` + `cargo build` on macOS (greeter module compiles; IPC may be stubbed with `#[cfg(target_os = "linux")]`).

**Verify:** `cargo build` succeeds.

---

### Task 3: GPUI greeter UI

**Files:** `src/greeter.rs`, optional `src/views/greeter.crepus`

**Steps:**

1. Fullscreen undecorated window, username + password fields, Submit, error label.
2. Map Enter → submit; Esc → clear error.
3. Wire submit to greetd IPC create_session / auth messages (read `greetd_ipc` examples / agreety source).
4. Power row: optional buttons calling `de::run` for suspend/reboot/shutdown (greeter user may need polkit — document limitation; v1 text only).
5. Manual test on Linux with running greetd (document in README).

**Verify:** Linux: `greetd` test config launches greeter window.

---

### Task 4: Session start compositor flag

**Files:** `../alpenglow/system/backends/appliance/scripts/alpenglow-session-start`, `rootfs-overlay/opt/alpenglow/session-init`

**Steps:**

1. If file `/etc/alpenglow/compositor` exists or env `ALPENGLOW_COMPOSITOR=1`, `exec alpenglowed --compositor`.
2. Else `exec alpenglowed` (cage provides Wayland).
3. Ensure `XDG_RUNTIME_DIR` and `WAYLAND_DISPLAY` match cage vs integrated docs.
4. Commit alpenglow.

**Verify:** Shell script syntax `sh -n alpenglow-session-start`.

---

### Task 5: Launch apps on compositor socket

**Files:** `src/main.rs` (`PluginAction::Launch`), `src/de.rs` if needed

**Steps:**

1. When `compositor_cmd` is `Some` and `ALPENGLOW_COMPOSITOR` set, spawn with `env("WAYLAND_DISPLAY", "alpenglowed-0")` (or value from compositor `start()`).
2. Add unit test or smoke doc for env injection logic (extract small helper `fn wayland_display_for_clients() -> Option<String>`).
3. `cargo test`.

**Verify:** `cargo test` passes.

---

### Task 6: boot-native graphical + greetd

**Files:** `../alpenglow/scripts/boot-native.sh`

**Steps:**

1. When `GRAPHICAL=1`, install greetd config from overlay; enable `greetd` in BOOT_SERVICES.
2. Do **not** start `alpenglowed` dinit service until after login — session-init starts it.
3. Add `ALPENGLOW_SKIP_GREETER=1` path: auto `alpenglow-session-start` as root for fast dev (document in README).
4. Build alpenglowed with compositor feature in Docker glibc script.
5. Run `./scripts/boot-native.sh --graphical` locally if possible; else document QEMU steps.

**Verify:** CI appliance script still green.

---

### Task 7: glibc build with compositor feature

**Files:** `build-alpenglowed-glibc.sh`

**Steps:**

1. Change `cargo build --release` → `cargo build --release --features compositor`.
2. Ensure Docker image has libwayland + xkbcommon dev packages (already present).
3. Rebuild graphical image once.

**Verify:** `file` on output binary shows dynamic; runs `--help` in Linux container.

---

### Task 8: Documentation

**Files:** `alpenglowed/README.md`, optional `../alpenglow/docs/desktop.md`

**Steps:**

1. Section **E2E test**: clone layout, `boot-native.sh --graphical`, login users.
2. Section **Greeter**: greetd config path, `--greeter`, dev skip flag.
3. Section **Compositor modes**: cage vs `--compositor` table.
4. Commit both repos as appropriate.

**Verify:** README links match real paths.

---

### Task 9: Compositor input forward (stretch)

**Files:** `src/compositor.rs`, `src/main.rs`

**Steps:**

1. Implement `CompositorCommand::Key` / focus surface id from layout focused pane.
2. When bar unfocused, route key events to compositor seat.
3. QEMU: type in `foot` after launch.

**Verify:** Manual QEMU only.

---

## Suggested implementation order

1 → 2 → 3 (greeter visible) → 4 → 5 → 6 → 7 → 8 → 9

## Estimated effort

| Task | Size |
|------|------|
| 1, 4, 6, 7 | Small (mostly alpenglow scripts) |
| 2, 5 | Small–medium |
| 3 | Medium (greetd IPC + GPUI) |
| 9 | Large (optional for first E2E milestone) |

**First milestone:** Tasks 1–6 + 8 with **cage compositor** (no Task 9/7 compositor paint) = login → shell → spawn foot under cage.

**Second milestone:** 7 + 5 + 9 + surface paint.