// ponytail: smithay compositor skeleton — full integration is Phase D
//
// Goal: replace wayland-client + cage/velox with embedded smithay compositor
// so alpenglowed is one static binary controlling DRM/KMS directly.
//
// Architecture sketch:
//
// ┌──────────────────────────────────────────────────────┐
// │ alpenglowed binary                                    │
// │ ┌──────────────┐  ┌────────────────────────────────┐ │
// │ │ smithay       │  │ GPUI shell                     │ │
// │ │ compositor    │◀─│ (launcher, layout, bar, pills) │ │
// │ │ (DRM/KMS)     │  │                                │ │
// │ │              │──▶│ xdg-shell surfaces rendered    │ │
// │ │ wayland      │  │ as texture in layout panes      │ │
// │ │ protocol     │  │                                │ │
// │ └──────────────┘  └────────────────────────────────┘ │
// └──────────────────────────────────────────────────────┘
//
// smithay crate: https://github.com/Smithay/smithay
// Provides: DRM, libinput, wayland protocol handling
//
// Integration points:
// 1. smithay::desktop::Space → map to alpenglowed LayoutState panes
// 2. xdg_shell toplevels → new panes in layout
// 3. layer_shell surfaces → bar/overlay rendering
// 4. Keyboard → translated to GPUI key events
//
// Build: add smithay dep (very large, optional), gate behind feature flag
//
// Nightly + LTO + musl target: ~12MB static binary

// Placeholder — uncomment and fill when building the smithay compositor
//
// use smithay::{
//     backend::drm::{DrmDevice, DrmSurface, DrmEvent},
//     backend::input::InputEvent,
//     backend::libinput::{LibinputInputBackend, LibinputSessionInterface},
//     desktop::{Space, Window, WindowSurface},
//     reexports::wayland_server::Display,
//     wayland::{
//         compositor::CompositorHandler,
//         shell::xdg::XdgShellHandler,
//         shell::wlr_layer::LayerShellHandler,
//         input_handler::InputHandler,
//         seat::SeatHandler,
//         shm::ShmHandler,
//         output::OutputHandler,
//     },
// };
