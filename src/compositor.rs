// ponytail: smithay compositor skeleton — full integration is Phase D
//
// Goal: replace wayland-client + cage/velox with embedded smithay compositor
// so alpenglowed is one static binary controlling DRM/KMS directly.
//
// Architecture:
//
// ┌──────────────────────────────────────────────────────────────┐
// │ alpenglowed binary                                            │
// │                                                              │
// │  ┌──────────────────────────────────────────────────┐        │
// │  │ GPUI Shell                                        │        │
// │  │  • DesktopModel — state + layout                   │        │
// │  │  • DesktopWindow — fullscreen render               │        │
// │  │  • LauncherWindow — search overlay                 │        │
// │  │  • SettingsWindow — config pane                    │        │
// │  │  • Notification overlay, Terminal dock             │        │
// │  └──────────────┬───────────────────────────────────┘        │
// │                 │ update_layout(windows)                      │
// │                 ▼                                             │
// │  ┌──────────────────────────────────────────────────┐        │
// │  │ LayoutState                                        │        │
// │  │  • tree of panes (Container/Window)                │        │
// │  │  • each pane maps to a smithay Window/surface      │        │
// │  │  • terminal pane → TerminalConsole PTY output      │        │
// │  └──────────────┬───────────────────────────────────┘        │
// │                 │ render                                      │
// │                 ▼                                             │
// │  ┌──────────────────────────────────────────────────┐        │
// │  │ Smithay Compositor                                │        │
// │  │  • DRM/KMS backend (drm-rs)                       │        │
// │  │  • libinput for input                             │        │
// │  │  • smithay::desktop::Space for window management  │        │
// │  │  • Wayland protocol dispatch (xdg, layer, seat)   │        │
// │  │  • Frame scheduling (render at vsync)             │        │
// │  └──────────────────────────────────────────────────┘        │
// │                                                              │
// │  Client processes connect to alpenglowed's Wayland socket:   │
// │  • firefox, alacritty, etc. → xdg-shell toplevels           │
// │  • Each toplevel → new pane in LayoutState                  │
// │  • LayerShell surfaces → bar / overlay components           │
// │                                                              │
// └──────────────────────────────────────────────────────────────┘
//
// Data flow:
//
//   xdg_toplevel.create (client) ─► smithay::desktop::Window
//       │                              │
//       │ wl_surface.attach            │ mapped to LayoutState tree
//       ▼                              ▼
//   smithay::desktop::Window      alpenglowed::layout::Node::Window
//       │                              │
//       │ shm/dma-buf buffer           │ position computed by layout
//       ▼                              ▼
//   Texture upload (GPUI) ────────► Rendered at layout position
//
// Keyboard flow:
//
//   libinput event ─► smithay Seat ─► GPUI KeyEvent
//       │                              │
//       │ keyboard focus               │ text in launcher
//       ▼                              ▼
//   xdg_surface (client)          LauncherWindow.append()
//
// If terminal focused:
//   GPUI KeyEvent ─► TerminalConsole.write() ─► PTY master ─► shell
//
// Build:
//
//   [dependencies]
//   smithay = { version = "0.4", features = ["backend_drm", "backend_libinput",
//               "wayland_frontend", "xwayland"], optional = true }
//
//   [features]
//   compositor = ["smithay"]
//
// Gate all compositor code behind #[cfg(feature = "compositor")].
// GPUI shell runs in headless mode when compositor feature is active.
//
// Integration points:
//
// 1. Init: smithay::backend::drm::DrmDevice::new() for each GPU
// 2. Display: smithay::reexports::wayland_server::Display
// 3. Space: smithay::desktop::Space — maps windows to output geometry
// 4. Layout sync: On each frame:
//      let windows = layout.windows();
//      for w in windows {
//          space.map(w.smithay_handle, w.x, w.y, None);
//      }
// 5. New client window:
//      XdgShellHandler::new_toplevel → create LayoutState::Window
//      GPUI callback → DesktopModel::apply(LayoutAction::SplitRow)
//      Focus new pane → route keyboard to client
// 6. LayerShell surfaces:
//      LayerShellHandler → render as GPUI overlay elements
//      Only layer-shell clients: bar, notification popups
//
// State sharing:
//
//   Arc<RwLock<SharedState>> between smithay event loop and GPUI:
//
//   struct SharedState {
//       layout: LayoutState,
//       focused_surface: Option<WlSurface>,
//       active_notifications: Vec<Notification>,
//   }
//
// GPUI reads SharedState each frame, writes when user interacts.
// Smithay reads SharedState to know window positions, writes when
// clients attach/map/unmap.
//
// Startup sequence:
//
//   1. Open DRM device, set up dumb buffer for boot splash
//   2. Start smithay event loop in its own thread
//   3. Start GPUI shell in headless mode (no wayland-client)
//   4. GPUI renders into DMA-BUF textures, submits as DRM framebuffers
//   5. ATOMIC commit on each vsync
//
// Required protocols:
//   - wl_compositor, wl_subcompositor, wl_data_device_manager
//   - xdg_wm_base (toplevels → layout panes)
//   - zwlr_layer_shell_v1 (bar, overlays)
//   - zxdg_output_manager_v1 (layout info)
//   - wlr_foreign_toplevel_management_v1 (window list)
//   - wp_fractional_scale_v1 (hidpi)
//   - wp_viewporter (surface scaling)
//
// Nightly + LTO + musl target: ~12MB static binary.
