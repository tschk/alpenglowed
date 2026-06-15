# Alpenglowed — alpenglow environment for desktop

### Raycast-style bar launcher for Linux (Wayland)

A GPU-accelerated launcher bar that IS the desktop. Summon with a hotkey (Super+Space), type to launch, search, calculate, or run commands. Time, date, battery, weather, CPU/GPU shown as pills in the bar. Windows managed as floating/tiled surfaces below.

## Philosophy

The traditional desktop (wallpaper + icons + taskbar) is unnecessary. The only interface you need is a text bar that does everything: app launcher, calculator, shell, clipboard, file search, AI assistant. Status info (clock, battery, weather, CPU) lives in pills at the top of the bar.

Alpenglowed is a single GPUI binary that runs fullscreen on top of a Wayland compositor (cage for now, smithay later).

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
│ cage (wlroots kiosk) /       │
│ smithay compositor (future)  │
└──────────────────────────────┘
          │
          ▼
  Linux Kernel (DRM/KMS)
```

## Phases

### Phase A — Pills + Launcher (MVP)

- Fullscreen GPUI window
- **Pill row** (top): clock, date, battery percentage
- **Text bar**: input with `> ` prompt
- **Results**: fuzzy match PATH executables, run on Enter
- **Shell**: type `> command` to run shell commands
- **Calculator**: type math expressions, evaluate via `bc`
- **Keybind**: Super+Space toggles focus in/out of the bar
- **Dismiss**: Esc to unfocus; the bar stays visible but inactive

### Phase B — Windows & Status

- **App launch**: launched apps appear as managed windows below the bar
- **Window manager**: simple tiling (two columns) or floating
- **More pills**: WiFi SSID + signal, weather via curl wttr.in, CPU%
- **Notifications**: simple FIFO-based notification daemon
- **Terminal widget**: embedded terminal in the bar area (`>' command` shows output inline)

### Phase C — Files & Clipboard

- **File search**: fuzzy find files by name under ~ and /
- **Clipboard history**: store and search recent copies
- **Emoji picker**: `:smile` → emoji search
- **Web search**: `?query` → search DuckDuckGo

### Phase D — Compositor Built-in

- Replace cage: use `smithay` crate as compositor backend in the same binary
- Direct DRM/KMS access
- Remove Wayland dependency: one static binary controls the display directly
- Embed terminal via `cosmic-text` + `alacritty_terminal` or similar
- Static musl build: ~12MB single binary, no runtime deps beyond kernel DRM

## Build

```sh
cargo build --release
SDKROOT=$(xcrun --show-sdk-path) cargo run    # macOS dev
cargo run                                       # Linux dev
```

## Why not Rover?

Rover is a macOS launcher with SwiftUI and Apple Intelligence integration. Alpenglowed is:
- Linux-native (Wayland, no macOS Cocoa/NSPanel)
- Bar IS the desktop (always visible, not a popup panel)
- Pills for system status at the bar top
- Window management built in (floating/tiling)
- No Apple Intelligence (offline AI via crepuscularity-lite V8 later)
- Purpose-built for Alpenglow OS

## Why not Crepuscularity directly?

Crepuscularity templates (`.crepus`) are great for content apps, but the bar desktop needs raw GPUI for pixel-level control of the bar layout, pill rendering, and window management. We use crepuscularity-gpui for the GPUI wrapper but write the app logic in plain Rust + GPUI.
