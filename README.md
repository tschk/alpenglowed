# Alpenglowed — alpenglow environment for desktop

### Raycast-style bar launcher for Linux (Wayland)

A GPU-accelerated launcher bar that IS the desktop. Summon with a hotkey (Super+Space), type to launch, search, calculate, run commands, or execute plugins. Time, date, battery, load, memory, and backend state shown as pills in the bar. Windows managed as floating/tiled surfaces below.

## Philosophy

The traditional desktop (wallpaper + icons + taskbar) is unnecessary. The only interface you need is a text bar that does everything: app launcher, calculator, shell, clipboard, file search, AI assistant. Status info (clock, date, battery, load, memory, backend) lives in pills at the top of the bar.

Alpenglowed is a single Crepuscularity GPUI binary that runs fullscreen on top of the Alpenglow Wayland stack.

## Architecture

```
┌────────────────────────────────────────────────────────┐
│ Alpenglowed Bar                                        │
│ ┌──────────┐ ┌──────┐ ┌────────┐ ┌──────┐ ┌────────┐ │
│ │ Clock    │ │ Bat  │ │ CPU    │ │ WiFi │ │ Weather│ │
│ └──────────┘ └──────┘ └────────┘ └──────┘ └────────┘ │
│                                                        │
│ ┌────────────────────────────────────────────────────┐ │
│ │ > _                                                 │ │
│ │                                                     │ │
│ │   firefox                    Firefox                │ │
│ │   term                        Alacritty             │ │
│ │   calc                        = 42                  │ │
│ │                                                     │ │
│ └────────────────────────────────────────────────────┘ │
└────────────────────────────────────────────────────────┘
          │ (Wayland protocol)
          ▼
┌──────────────────────────────┐
│ Wayland compositor           │
│ velox now, smithay target    │
└──────────────────────────────┘
          │
          ▼
  Linux Kernel (DRM/KMS)
```

## Phases

### Phase A — Pills + Launcher (MVP) ✅

- Fullscreen GPUI window
- **Pill row** (top): clock, date, battery percentage
- **Text bar**: input with `> ` prompt
- **Results**: fuzzy match PATH executables, run on Enter
- **Shell**: type `> command` to run shell commands
- **Calculator**: type math expressions, evaluate via `bc`
- **Plugins**: built-in Rust plugins plus command plugins written in Rust, Crepus, or Bun/webcode
- **Keybind**: Super+Space toggles focus in/out of the bar
- **Dismiss**: Esc to unfocus; the bar stays visible but inactive

### Phase B — Windows & Status (partial)

- **App launch**: launched apps appear as managed windows below the bar — needs compositor (Phase D)
- **Window manager**: simple tiling (two columns) or floating ✅
- **More pills**: WiFi SSID + signal ✅, weather via curl wttr.in ✅, CPU% ✅
- **Inline terminal**: `>' command` runs and shows stdout in results ✅
- **Terminal pane**: `Cmd-Alt-T` toggles shell console docked at bottom ✅
- **Terminal clear**: `clear`/`cls`/`reset` action clears output buffer ✅
- **Notifications**: Unix socket daemon → toast popups, auto-dismiss 6s ✅

### Phase C — Files & Clipboard ✅

- **File search**: `/query` → fuzzy find via locate/fd/find ✅
- **Web search**: `?query` → DuckDuckGo search ✅
- **Emoji picker**: `:smile` → emoji search, copies to clipboard ✅
- **Clipboard history**: type `clip`/`cb`/`paste` → browse and restore recent copies ✅
- **Spotify**: MPRIS actions through `playerctl` ✅

### Phase D — Compositor Built-in (smithay embedded)

- **`cargo run --features compositor -- --compositor`**: starts embedded smithay compositor
- Creates Wayland socket at `$XDG_RUNTIME_DIR/alpenglowed/wayland-0`
- xdg-shell toplevels → layout panes (surface→pane integration TBD)
- Compositor runs in background thread, communicates via channels with GPUI
- Direct DRM/KMS: future work (currently uses wayland socket)
- Need Linux to build (`xkbcommon` linkage requirement)

## Build

```sh
cargo build --release
SDKROOT=$(xcrun --show-sdk-path) cargo run    # macOS dev
cargo run                                       # Linux dev
cargo run -- --polybar                          # status output
cargo run -- --external-polybar                 # desktop without in-app status strip
cargo run -- --smoke-wayland                    # Wayland connection smoke
./polybar/launch.sh                             # external polybar bar
```

## Plugins

Plugins return launcher results as JSON. Built-ins are Rust. External command plugins can be Rust binaries, Bun/webcode, or Crepuscularity-backed tools that expose the same stdin/stdout protocol.

`plugins/spotify` is a Bun webcode plugin for Spotify-compatible MPRIS players. `plugins/spotify-rust` is the Rust version. Both control the native machine through `playerctl`.

Rover is only a design reference for launcher/plugin ergonomics. Crepuscularity is the UI framework used here.
