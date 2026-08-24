# Alpenglow session contract

Alpenglowed is the potato / desktop session for
[Alpenglow](https://github.com/tschk/alpenglow). Product SKUs are
`potato | desktop | internet`. This binary implements `potato` and
`desktop`. `internet` is `sold`, not Alpenglowed.

Machine-readable copy: `alpenglowed --session-contract`.

## SKUs from the DE side

Public roles on this DE are only two: `potato` and `desktop`.

| Product SKU | DE package | Role | What the shell does |
| --- | --- | --- | --- |
| `potato` | `alpenglowed-lite` | `potato` | Skinny bar. No weather, command plugins, Spotify, translate, or `--compositor`. No PipeWire dependency. |
| `desktop` | `alpenglowed` | `desktop` | Full bar and plugins. Fleet status (`/run/alpenglow`) is available on desktop. |
| `internet` | `sold` | — | Not this binary. Start `sold` from [soliloquy](https://github.com/tschk/soliloquy). |

Build:

```sh
cargo build --release                          # alpenglowed (desktop)
cargo build --release --no-default-features    # alpenglowed-lite (potato)
cargo build --release -p alpenglow-greeter
```

Ship lite as `/usr/bin/alpenglowed-lite` or as `/usr/bin/alpenglowed` on the
`potato` SKU. A full binary still honors `--role=potato` at runtime.

`potatoes` is a deprecated alias for `potato` for one release. Docs and
`--role` use `potato`. `workstation` is not a public role; leftover
`workstation` / `fleet` values map to `desktop`.

Image edition fallback (`ALPENGLOW_EDITION` / `/run/alpenglow/edition`):
`potato` / `potatoes` → potato, `desktop` / `desktop-full` → desktop.
`internet` is not this binary.

Roles that do **not** belong here:

| Role | Use instead |
| --- | --- |
| `internet` | `sold` from [soliloquy](https://github.com/tschk/soliloquy). |
| `kiosk` | Cage + one app (`exec cage -- /usr/bin/<app>`). |
| `sold` | Same as `internet`. |
| `embedded`, `containers` | Headless. No graphical session. |

`alpenglowed --role=kiosk` (or `ALPENGLOWED_ROLE=internet`) exits 2.

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
export ALPENGLOWED_ROLE=desktop          # or potato
# WAYLAND_DISPLAY must already be set by velox/cage, or the wrapper starts cage.
exec /usr/bin/alpenglowed --role="${ALPENGLOWED_ROLE}"
```

Flags the wrapper may pass:

| Flag | Meaning |
| --- | --- |
| `--role=potato\|desktop` | Session personality. Default name is `potato`. |
| `--status-bar` | Force the in-process bar on |
| `--compositor` | Embedded smithay thread. Desktop only, experimental. Potato ignores it. |
| `--session-contract` | Print this contract as JSON and exit |
| `--polybar` | One-line status for an external bar |
| `--smoke-wayland` | Probe `WAYLAND_DISPLAY` |

Environment:

| Variable | Meaning |
| --- | --- |
| `ALPENGLOWED_ROLE` | Role if `--role` is absent |
| `ALPENGLOW_EDITION` | Fallback: `potato` → potato, `desktop` / `desktop-full` → desktop |
| `ALPENGLOWED_MODE` | Window mode: tiling, floating, monocle, stack, center, grid |
| `ALPENGLOWED_STATUS_BAR` | `1`/`true`/`yes` |
| `ALPENGLOWED_PLUGIN_DIR` | Extra command-plugin directory (ignored on potato / lite) |
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
| `/run/alpenglow/pressurectl/state.json` | desktop | Fleet plugin |
| `/run/alpenglow/netd/interfaces.json` | desktop | Fleet plugin |
| `/run/alpenglow/netd/runtime-state.env` | desktop | Fleet plugin |
| `/run/alpenglow/runtime-state.env` | desktop | Fleet plugin |
| `/run/alpenglow/rootfs.env` | desktop | Fleet plugin |

Config still comes from `/etc/alpenglowed/config.toml`, then
`/usr/share/defaults/alpenglowed/config.toml`. `role = "potato"` is a valid
key. Factory reset deletes `/etc/alpenglowed` and writes
`/var/lib/alpenglow/.factory-reset` for the session wrapper to recopy defaults.

## Image requirements

| SKU | Must ship | Must not assume |
| --- | --- | --- |
| `potato` | `seatd`, compositor (`velox` or `cage`), `wayland`, Mesa or llvmpipe, `font-dejavu`, `alpenglowed` (lite) | PipeWire, greetd, iwd, weather tools, plugin host |
| `desktop` | above + `greetd`, `elogind`, `pipewire`, `wireplumber`, `foot`, `iwd` | Embedded `--compositor` as the seat owner |

Alpenglow's current `dinit/alpenglowed` unit depends on PipeWire. That is
correct for `desktop` only. Potato should depend on `seatd` (and the
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

Do not `exec alpenglowed --compositor` on potato. Do not use Alpenglowed as
the kiosk compositor.

## dinit examples

See `contrib/session/dinit/`. Potato:

```
type = process
command = /usr/bin/alpenglowed --role=potato
depends-on = seatd
restart = yes
```

Desktop:

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
