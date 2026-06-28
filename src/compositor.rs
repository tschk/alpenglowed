// Phase D: smithay compositor integration design
// All code is commented — smithay is NOT a dependency yet.
// Gate behind `compositor` feature flag when wiring.

// ─── Architecture ───────────────────────────────────────────────
//
// Before (current):         After (Phase D target):
// ┌──────────────┐         ┌──────────────────────────────────┐
// │ alpenglowed  │         │ alpenglowed binary                │
// │ (GPUI client)│         │ ┌─────────┐  ┌──────────────────┐ │
// │              │  ───►   │ │ smithay │  │ GPUI shell       │ │
// │ cage/velox   │         │ │ com-    │◀─│ (launcher, bar,  │ │
// │ (compositor) │         │ │ positor │  │  layout, pills)  │ │
// │              │         │ │ (DRM/   │──▶│ xdg-surface →   │ │
// │ Linux DRM    │         │ │  KMS)   │  │ texture in pane  │ │
// └──────────────┘         │ │ libinput│  │                  │ │
//                           │ │ proto  │  │                  │ │
//                           │ └─────────┘  └──────────────────┘ │
//                           │ Linux DRM/KMS                      │
//                           └──────────────────────────────────┘

// ─── smithay types used (sketch) ───────────────────────────────
//
// use smithay::{
//     backend::drm::{DrmDevice, DrmEvent, DrmSurface},
//     backend::egl::{EGLDisplay, EGLContext},
//     backend::input::InputEvent,
//     backend::libinput::{LibinputInputBackend, LibinputSessionInterface},
//     backend::session::auto::AutoSession,
//     desktop::{LayerMap, PopupManager, Space, Window, WindowSurface},
//     input::{Seat, SeatState},
//     output::{Mode, Output, PhysicalProperties, Subpixel},
//     reexports::wayland_server::{
//         backend::ClientId, protocol::wl_surface, Display, EventLoop,
//     },
//     utils::{IsAlive, Logical, Physical, Point, Rectangle, Scale, Size},
//     wayland::{
//         compositor::CompositorHandler,
//         data_device::DataDeviceHandler,
//         output::OutputHandler,
//         seat::{CursorImageStatus, KeyboardHandle, PointerHandle, SeatHandler},
//         shell::xdg::{
//             Toplevel, XdgShellHandler, XdgShellState, XdgRequest,
//         },
//         shell::wlr_layer::LayerShellHandler,
//         shm::ShmHandler,
//         input_method::InputMethodHandler,
//         text_input::TextInputHandler,
//     },
// };

// ─── Layout integration bridge ──────────────────────────────────
//
// The bridge connects smithay's surface tree to alpenglowed's
// LayoutState. Each xdg_toplevel surface becomes a LayoutWindow
// managed by the tiling/floating engine.
//
// Layout {
//     pub fn assign_surface(&mut self, id: usize, surface: WindowHandle)
//     pub fn surface_for_window(&self, id: usize) -> Option<WindowHandle>
// }
//
// The Layout::Window nodes gain a `surface` field:
//   WindowNode {
//       id: usize,
//       title: String,
//       detail: String,
//       floating: bool,
//       x: f32, y: f32, w: f32, h: f32,
//       surface: Option<WindowHandle>,  // ← NEW: smithay window
//   }

// ─── CompositorState ────────────────────────────────────────────
//
// struct CompositorState {
//     display: Display<Self>,
//     event_loop: EventLoop<'static, Self>,
//     drm: Vec<DrmData>,
//     space: Space,
//     seat_state: SeatState<Self>,
//     keyboard: Option<KeyboardHandle>,
//     pointer: Option<PointerHandle>,
//     xdg_state: XdgShellState,
//     egl: EGLDisplay,
//     layouts: Vec<Output>,
//     kill_tx: tokio::sync::watch::Sender<bool>,
//     gpui_tx: tokio::sync::mpsc::UnboundedSender<CompositorEvent>,
// }
//
// struct DrmData {
//     device: DrmDevice,
//     surface: DrmSurface,
//     renderer: GlesRenderer,
// }
//
// enum CompositorEvent {
//     NewWindow {
//         id: usize,
//         title: String,
//         x: i32, y: i32, w: i32, h: i32,
//         surface: smithay::backend::egl::display::EGLNativeDisplay,
//     },
//     CloseWindow(usize),
//     FocusChange(usize),
//     Input {
//         key: KeyEvent,
//         modifiers: ModifiersState,
//     },
// }

// ─── XDG Shell Handler ──────────────────────────────────────────
//
// impl XdgShellHandler for CompositorState {
//     fn xdg_shell_state(&mut self) -> &mut XdgShellState {
//         &mut self.xdg_state
//     }
//
//     fn new_toplevel(&mut self, surface: Toplevel) {
//         // 1. Assign a window ID from LayoutState.next_id
//         // 2. Create an alpenglowed WindowNode in LayoutState
//         // 3. Map the toplevel surface to the layout pane
//         // 4. Notify GPUI via gpui_tx: CompositorEvent::NewWindow
//         //
//         // let window_id = self.layout.borrow_mut().create_surface_window(
//         //     surface.title().unwrap_or("App").to_string(),
//         // );
//         // let smithay_window = Window::new(window_id, surface, self.space());
//         // self.space.map_window(smithay_window, position, scale, false);
//     }
//
//     fn toplevel_destroyed(&mut self, surface: Toplevel) {
//         // 1. Find window ID from surface
//         // 2. Remove from LayoutState
//         // 3. Unmap from Space
//         // 4. Notify GPUI
//     }
//
//     fn toplevel_requested_state_change(
//         &mut self,
//         _surface: Toplevel,
//         _state: XdgRequest,
//     ) {
//         // Handle maximize, fullscreen, minimize requests
//         // Translate to alpenglowed layout actions
//     }
// }

// ─── Layer Shell Handler ────────────────────────────────────────
//
// impl LayerShellHandler for CompositorState {
//     // The status bar and launcher use wlr-layer-shell protocol
//     // to render as overlays on top of client windows.
//     //
//     // The shell.crepus components render into layer surface
//     // buffers, positioned as top/center layers.
//     //
//     // fn layer_shell_state(&mut self) -> &mut LayerShellState { ... }
//     // fn new_layer_surface(...) { ... }
// }

// ─── Input Flow: smithay → GPUI ────────────────────────────────
//
// Keyboard:
//   smithay::input::KeyboardEvent
//     → translate keycodes to xkbcommon keysyms
//     → pack into KeyEvent { key, modifiers, char }
//     → gpui_tx.send(CompositorEvent::Input { key, modifiers })
//     → GPUI MainThread dispatches through key_context
//
// Pointer:
//   smithay::input::PointerEvent { motion, button, axis }
//     → translate to cursor position + button state
//     → gpui_tx.send(...)
//     → GPUI MainThread dispatches through on_mouse_down/up/move
//
// libinput provides the raw input events; smithay's input handler
// processes them and invokes seat callbacks on CompositorState.

// ─── Initialization sequence ────────────────────────────────────
//
// fn start_compositor(
//     gpui_tx: tokio::sync::mpsc::UnboundedSender<CompositorEvent>,
//     layout: Rc<RefCell<LayoutState>>,
// ) -> (JoinHandle<()>, tokio::sync::watch::Receiver<bool>) {
//     let (kill_tx, kill_rx) = tokio::sync::watch::channel(false);
//
//     let handle = std::thread::spawn(move || {
//         let mut display: Display<CompositorState> = Display::new();
//         let mut event_loop = display.create_event_loop();
//
//         // Session (logind/seatd)
//         let (session, _) = AutoSession::new(None).unwrap();
//
//         // DRM devices
//         let mut drms = Vec::new();
//         for path in DrmDevice::available_devices().unwrap() {
//             let (device, surfaces) =
//                 DrmDevice::new(path, &session, DrmEvent::default()).unwrap();
//             for surface in surfaces {
//                 let (renderer, egl) = GlesRenderer::new(&surface).unwrap();
//                 drms.push(DrmData { device: &device, surface, renderer });
//             }
//         }
//
//         // EGL (shared context for GPUI texture sharing)
//         let egl = EGLDisplay::new_with_device(
//             drms.first().map(|d| &d.surface).unwrap(),
//             EGLContext::default(),
//         ).unwrap();
//
//         // libinput
//         let libinput_backend = LibinputInputBackend::new(&session).unwrap();
//
//         // Wayland protocol globals
//         let state = CompositorState {
//             display,
//             event_loop,
//             drm: drms,
//             space: Space::default(),
//             seat_state: SeatState::new(),
//             keyboard: None,
//             pointer: None,
//             xdg_state: XdgShellState::new(),
//             egl,
//             layouts: Vec::new(),
//             kill_tx,
//             gpui_tx,
//         };
//
//         // Bind globals: compositor, xdg_shell, layer_shell, shm, seat, data_device
//         //
//         // Run event loop (blocking, own thread):
//         // loop {
//         //     event_loop.dispatch(..);
//         //     handle_drm_events(&mut state);
//         //     render(&mut state);
//         //     if *kill_rx.borrow() { break; }
//         // }
//     });
//
//     (handle, kill_rx)
// }
//
// fn render(state: &mut CompositorState) {
//     // For each DRM surface:
//     //   1. state.space.elements() → list of Window/Surface
//     //   2. Render each window's buffer via GlesRenderer
//     //   3. Composite using EGL
//     //   4. Flip via drm.page_flip()
//     //
//     // GPUI textures (launcher, bar, pills) render as a layer-shell
//     // overlay surface that composites on top of client windows.
// }

// ─── Layout ↔ smithay Space mapping ─────────────────────────────
//
// Mapping rules:
//
// 1. xdg_toplevel surface created → layout.rs creates WindowNode
//    The smithay `Window` is stored in a HashMap<usize, Window>.
//
// 2. Layout split/flip/close → smithay Space element reposition
//    LayoutAction::SplitRow → allocate space in new container,
//    reposition the Window in the Space.
//
// 3. Floating windows sit in a separate layer in Space
//    layout.rs marks WindowNode.floating → smithay Space uses
//    `space.map_window(window, pos, scale, true)` where the
//    last bool is "sticky" (not affected by layout).
//
// 4. Focus sync
//    LayoutState.focus_window(id) → smithay seat set_keyboard_focus
//    → kernel input routing to the focused client
//
// Concrete API sketch:
//
// impl LayoutState {
//     /// Register a smithay Window for an existing layout window.
//     pub fn attach_surface(&mut self, id: usize, window: WindowHandle) {
//         if let Some(node) = find_mut(&mut self.root, id) {
//             node.surface = Some(window);
//         }
//     }
//
//     /// Remove surface binding when client disconnects.
//     pub fn detach_surface(&mut self, id: usize) -> bool {
//         if let Some(node) = find_mut(&mut self.root, id) {
//             node.surface = None;
//             true
//         } else {
//             false
//         }
//     }
// }

// ─── Build & Feature Gate ───────────────────────────────────────
//
// Cargo.toml addition:
//
// [features]
// compositor = ["smithay", "smithay/backend_drm", "smithay/backend_egl",
//               "smithay/backend_libinput", "smithay/xdg_shell",
//               "smithay/wlr_layer_shell"]
//
// [dependencies.smithay]
// version = "0.3"
// optional = true
// default-features = false
// features = [
//     "backend_drm",
//     "backend_egl",
//     "backend_libinput",
//     "backend_session",
//     "xwayland",
//     "xdg_shell",
//     "wlr_layer_shell",
// ]
//
// main.rs:
//   #[cfg(feature = "compositor")]
//   mod compositor;
//
//   #[cfg(feature = "compositor")]
//   fn run_compositor(layout: Rc<RefCell<LayoutState>>) { ... }
//
// Build:
//   cargo build --release --features compositor    # full static binary
//   cargo build --release                           # GPUI-only (current)

// ─── Runtime mode detection ─────────────────────────────────────
//
// The binary detects which mode to run at startup:
//
// fn main() {
//     if cfg!(feature = "compositor") && has_drm_access() {
//         // Start smithay compositor thread + GPUI shell
//         let (kill, _) = start_compositor(gpui_tx, layout);
//         Application::new().run(move |cx| { /* GPUI shell */ });
//     } else {
//         // Legacy: GPUI as client on cage/velox (current)
//         ensure_wayland_display();
//         Application::new().run(move |cx| { /* GPUI shell */ });
//     }
// }
//
// fn has_drm_access() -> bool {
//     // Check /dev/dri/card0 access, cgroup, or --force-compositor flag
//     std::env::args().any(|a| a == "--force-compositor")
//         || std::path::Path::new("/dev/dri/card0").exists()
// }
