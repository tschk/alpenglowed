# Alpenglow session contract

Alpenglowed is the desktop / workstation / potatoes-GUI session for
[Alpenglow](https://github.com/tschk/alpenglow). It is not the internet
appliance compositor and not a kiosk.

Machine-readable copy: `alpenglowed --session-contract`.

## SKUs from the DE side

Alpenglow editions (`ALPENGLOW_EDITION` in `editions.toml`) are image knobs.
Roles are the session personality Alpenglowed actually runs.

| Alpenglow edition | DE package | Role | What the shell does |
| --- | --- | --- | --- |
| `fast`, `minimal`, `standard` | none | — | Headless. Do not start Alpenglowed. |
| `desktop` (`packages-desktop-lite.txt`) | `alpenglowed-lite` | `potatoes` | Skinny bar. No weather, command plugins, Spotify, translate, or `--compositor`. |
| `desktop-full` (`packages-runtime.txt`) | `alpenglowed` | `desktop` | Full bar and plugins. |
| `desktop-full` + fleet hosts | `alpenglowed` | `workstation` | Desktop plus `/run/alpenglow` status plugin. |

Build:

```sh
cargo build --release                          # alpenglowed (full)
cargo build --release --no-default-features    # alpenglowed-lite
cargo build --release -p alpenglow-greeter
```

Ship lite as `/usr/bin/alpenglowed-lite` or as `/usr/bin/alpenglowed` on the
`desktop` edition. A full binary still honors `--role=potatoes` at runtime.

Roles that do **not** belong here:

| Role | Use instead |
| --- | --- |
| `kiosk` | Cage + one app (`exec cage -- /usr/bin/<app>`). |
| `internet` | `sold` from [soliloquy](https://github.com/tschk/soliloquy). |
| `embedded`, `containers` | Headless. No graphical session. |

`alpenglowed --role=kiosk` (or `ALPENGLOWED_ROLE=kiosk`) exits 2.

## How Alpenglow should start it

Preferred tree (greetd or dinit `alpenglow-session`):

```
seatd
  → velox  or  cage
       → /usr/local/bin/alpenglow-session-start
            → /usr/bin/alpenglowed --role=<role>
```

Canonical wrapper: `contrib/session/alpenglow-session-start` (install to
`/usr/local/bin/alpenglow-session-start`).

```sh
export XDG_RUNTIME_DIR="${XDG_RUNTIME_DIR:-/run/user/$(id -u)}"
export ALPENGLOWED_ROLE=desktop          # or potatoes / workstation
# WAYLAND_DISPLAY must already be set by velox/cage, or the wrapper starts cage.
exec /usr/bin/alpenglowed --role="${ALPENGLOWED_ROLE}"
```

Flags the wrapper may pass:

| Flag | Meaning |
| --- | --- |
| `--role=potatoes\|desktop\|workstation` | Session personality |
| `--status-bar` | Force the in-process bar on |
| `--compositor` | Embedded smithay thread. Desktop/workstation only, experimental. Potatoes ignores it. |
| `--session-contract` | Print this contract as JSON and exit |
| `--polybar` | One-line status for an external bar |
| `--smoke-wayland` | Probe `WAYLAND_DISPLAY` |

Environment:

| Variable | Meaning |
| --- | --- |
| `ALPENGLOWED_ROLE` | Role if `--role` is absent |
| `ALPENGLOW_EDITION` | Fallback: `desktop` → potatoes, `desktop-full` → desktop |
| `ALPENGLOWED_MODE` | Window mode: tiling, floating, monocle, stack, center, grid |
| `ALPENGLOWED_STATUS_BAR` | `1`/`true`/`yes` |
| `ALPENGLOWED_PLUGIN_DIR` | Extra command-plugin directory (ignored on potatoes / lite) |
| `ALPENGLOW_SESSION_CONTROL` | Unix socket for compositor/session IPC |
| `ALPENGLOWED_INSTALLER_SOURCE` | Live image path; defaults to `/run/alpenglow/alpenglow.img.zst` if present |
| `ALPENGLOWED_INSTALLER_TARGET` | Required to open the installer window |
| `XDG_RUNTIME_DIR` | Required |
| `WAYLAND_DISPLAY` | Required unless `--compositor` actually starts |

Autologin is greetd's job (`ALPENGLOW_AUTOLOGIN=1` or
`/etc/greetd/config-autologin.toml`). Alpenglowed does not implement autologin
or lock-down hooks.

## `/run/alpenglow` files

Alpenglowed **reads**, never writes:

| Path | When | Use |
| --- | --- | --- |
| `/run/alpenglow/role` | always | Role if CLI/env unset |
| `/run/alpenglow/edition` | always | Edition → role fallback |
| `/run/alpenglow/alpenglow.img.zst` | installer | Live image source |
| `/run/alpenglow/pressurectl/state.json` | workstation | Fleet plugin |
| `/run/alpenglow/netd/interfaces.json` | workstation | Fleet plugin |
| `/run/alpenglow/netd/runtime-state.env` | workstation | Fleet plugin |
| `/run/alpenglow/runtime-state.env` | workstation | Fleet plugin |
| `/run/alpenglow/rootfs.env` | workstation | Fleet plugin |

Config still comes from `/etc/alpenglowed/config.toml`, then
`/usr/share/defaults/alpenglowed/config.toml`. `role = "potatoes"` is a valid
key. Factory reset deletes `/etc/alpenglowed` and writes
`/var/lib/alpenglow/.factory-reset` for the session wrapper to recopy defaults.

## Image requirements

| Edition | Must ship | Must not assume |
| --- | --- | --- |
| `desktop` / potatoes | `seatd`, compositor (`velox` or `cage`), `wayland`, Mesa or llvmpipe, `font-dejavu`, `alpenglowed` (lite) | PipeWire, greetd, iwd, weather tools, plugin host |
| `desktop-full` / desktop+workstation | above + `greetd`, `elogind`, `pipewire`, `wireplumber`, `foot`, `iwd` | Embedded `--compositor` as the seat owner |

Alpenglow's current `dinit/alpenglowed` unit depends on PipeWire. That is
correct for `desktop-full` only. Potatoes should depend on `seatd` (and the
compositor), not PipeWire.

Alpenglow's `dinit/velox` unit currently runs `/usr/bin/cage`. Treat **Cage**
as the working kiosk/nested compositor and **velox** as the intended desktop
compositor (`build-velox.sh`). `alpenglowed-comp` is a nested winit prototype
and is not ready to own a TTY.

**libc:** GPUI `dlopen`s Wayland/Vulkan. The image path is the glibc dynamic
binary from `build-alpenglowed-glibc.sh`. The musl static scripts in
`scripts/build-x86_64.sh` are a cross-compile experiment, not the SKU.

Geist fonts are embedded. Dejavu remains useful for clients (foot).

## Compositor vs client

| Path | Status |
| --- | --- |
| Alpenglowed as a Wayland **client** of velox/cage | Production path |
| `alpenglowed --compositor` (`src/compositor.rs`) | Incomplete: no DRM, no buffer paint into GPUI, no input forwarding |
| `alpenglowed-comp` | Milestone 0 nested smithay; future seat owner, bar becomes a layer-shell client |

Do not `exec alpenglowed --compositor` on potatoes. Do not use Alpenglowed as
the kiosk compositor.

## dinit examples

See `contrib/session/dinit/`. Potatoes:

```
type = process
command = /usr/bin/alpenglowed --role=potatoes
depends-on = seatd
restart = yes
```

Desktop-full:

```
type = process
command = /usr/bin/alpenglowed --role=desktop
depends-on = seatd
depends-on = pipewire
depends-on = wireplumber
restart = yes
```

The compositor process (velox or cage) must be up first so `WAYLAND_DISPLAY`
exists, unless the session wrapper starts Cage around Alpenglowed.
