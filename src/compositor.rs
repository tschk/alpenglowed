// Phase D: Embedded smithay compositor
//
// Alpenglowed embeds a smithay Wayland compositor. Client applications
// connect to an internal wayland socket; their surfaces are rendered as
// textures in GPUI layout panes.
//
// Architecture:
//   ┌───────────────────────────────┐
//   │ alpenglowed                    │
//   │ ┌────────────────┐ ┌────────┐ │
//   │ │ smithay thread  │ │ GPUI   │ │
//   │ │ compositor      │◀┤ main   │ │
//   │ │ (calloop)       │──┤ thread │ │
//   │ │                 │  │        │ │
//   │ │ wayland socket  │  │ panes  │ │
//   │ │ xdg-shell       │  │ tex    │ │
//   │ │ SHM buffers    │  │ input  │ │
//   │ └────────────────┘ └────────┘ │
//   └───────────────────────────────┘
//
// Clients: WAYLAND_DISPLAY=alpenglowed-0
// Socket: $XDG_RUNTIME_DIR/alpenglowed/wayland-0

use std::sync::mpsc;
use std::sync::Arc;
use std::thread;

use smithay::input::{Seat, SeatHandler, SeatState};
use smithay::reexports::wayland_protocols::xdg::shell::server::xdg_toplevel;
use smithay::reexports::wayland_server::{
    backend::{ClientData, ClientId, DisconnectReason},
    protocol::{wl_buffer::WlBuffer, wl_surface::WlSurface},
    Client, Display, ListeningSocket,
};
use smithay::utils::Serial;
use smithay::wayland::{
    buffer::BufferHandler,
    compositor::{CompositorClientState, CompositorHandler, CompositorState},
    selection::{
        data_device::{DataDeviceHandler, DataDeviceState},
        SelectionHandler,
    },
    shell::xdg::{PopupSurface, PositionerState, ToplevelSurface, XdgShellHandler, XdgShellState},
    shm::{ShmHandler, ShmState},
};
use smithay::{
    delegate_compositor, delegate_data_device, delegate_seat, delegate_shm, delegate_xdg_shell,
};

/// Commands: GPUI thread → compositor thread
pub enum CompositorCommand {
    KeyboardInput { key: u32, state: KeyState },
    PointerMotion { x: f64, y: f64 },
    PointerButton { button: u32, state: KeyState },
    Shutdown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyState {
    Pressed,
    Released,
}

/// Events: compositor thread → GPUI thread
pub enum CompositorEvent {
    SurfaceCreated {
        id: u32,
        title: String,
        app_id: String,
    },
    SurfaceUpdated {
        id: u32,
    },
    SurfaceClosed {
        id: u32,
    },
}

pub struct AlpenglowCompositor {
    pub compositor_state: CompositorState,
    pub xdg_shell_state: XdgShellState,
    pub shm_state: ShmState,
    pub seat_state: SeatState<Self>,
    pub data_device_state: DataDeviceState,
    pub seat: Seat<Self>,
    pub socket_name: String,
    pub surfaces: Vec<ToplevelSurface>,
    pub event_tx: mpsc::Sender<CompositorEvent>,
    pub cmd_rx: mpsc::Receiver<CompositorCommand>,
}

impl AlpenglowCompositor {
    fn new(
        dh: &smithay::reexports::wayland_server::DisplayHandle,
        event_tx: mpsc::Sender<CompositorEvent>,
        cmd_rx: mpsc::Receiver<CompositorCommand>,
        socket_name: String,
    ) -> Self {
        let compositor_state = CompositorState::new::<Self>(dh);
        let xdg_shell_state = XdgShellState::new::<Self>(dh);
        let shm_state = ShmState::new::<Self>(dh, vec![]);
        let data_device_state = DataDeviceState::new::<Self>(dh);
        let mut seat_state = SeatState::new();
        let seat = seat_state.new_wl_seat(dh, "alpenglowed");
        // Keyboard and pointer added later

        Self {
            compositor_state,
            xdg_shell_state,
            shm_state,
            seat_state,
            data_device_state,
            seat,
            socket_name,
            surfaces: Vec::new(),
            event_tx,
            cmd_rx,
        }
    }

    fn accept_clients(listener: &ListeningSocket, display: &Display<Self>) {
        while let Ok(Some(stream)) = listener.accept() {
            let _ = display
                .handle()
                .insert_client(stream, Arc::new(ClientState::default()));
        }
    }

    fn process_commands(&mut self) {
        while let Ok(cmd) = self.cmd_rx.try_recv() {
            match cmd {
                CompositorCommand::Shutdown => return,
                _ => {} // TBD: input forwarding
            }
        }
    }

    fn send_frames(&self, time: u32) {
        for toplevel in &self.surfaces {
            let surface = toplevel.wl_surface();
            smithay::wayland::compositor::with_surface_tree_downward(
                surface,
                (),
                |_, _, &()| smithay::wayland::compositor::TraversalAction::DoChildren(()),
                |_surf, states, &()| {
                    for callback in states
                        .cached_state
                        .get::<smithay::wayland::compositor::SurfaceAttributes>()
                        .current()
                        .frame_callbacks
                        .drain(..)
                    {
                        callback.done(time);
                    }
                },
                |_, _, &()| true,
            );
        }
    }

    fn keyboard_focus_first(&mut self) {
        if let Some(surface) = self.surfaces.first().cloned() {
            if let Some(kb) = self.seat.get_keyboard() {
                kb.set_focus(self, Some(surface.wl_surface().clone()), 0.into());
            }
        }
    }
}

// ── Protocol handlers ──

impl BufferHandler for AlpenglowCompositor {
    fn buffer_destroyed(&mut self, _buffer: &WlBuffer) {}
}

impl CompositorHandler for AlpenglowCompositor {
    fn compositor_state(&mut self) -> &mut CompositorState {
        &mut self.compositor_state
    }

    fn client_compositor_state<'a>(&self, client: &'a Client) -> &'a CompositorClientState {
        &client.get_data::<ClientState>().unwrap().compositor_state
    }

    fn commit(&mut self, surface: &WlSurface) {
        smithay::backend::renderer::utils::on_commit_buffer_handler::<Self>(surface);
        // Find toplevel for this surface and notify GPUI
        for toplevel in &self.surfaces {
            if toplevel.wl_surface() == surface {
                let _ = self
                    .event_tx
                    .send(CompositorEvent::SurfaceUpdated { id: 0 });
                break;
            }
        }
    }
}

impl ShmHandler for AlpenglowCompositor {
    fn shm_state(&self) -> &ShmState {
        &self.shm_state
    }
}

impl XdgShellHandler for AlpenglowCompositor {
    fn xdg_shell_state(&mut self) -> &mut XdgShellState {
        &mut self.xdg_shell_state
    }

    fn new_toplevel(&mut self, surface: ToplevelSurface) {
        let id = self.surfaces.len() as u32;
        surface.with_pending_state(|state| {
            state.states.set(xdg_toplevel::State::Activated);
        });
        surface.send_configure();
        self.surfaces.push(surface);

        let _ = self.event_tx.send(CompositorEvent::SurfaceCreated {
            id,
            title: format!("client-{id}"),
            app_id: "unknown".to_string(),
        });

        self.keyboard_focus_first();
    }

    fn new_popup(&mut self, _surface: PopupSurface, _positioner: PositionerState) {}

    fn grab(
        &mut self,
        _surface: PopupSurface,
        _seat: smithay::reexports::wayland_server::protocol::wl_seat::WlSeat,
        _serial: Serial,
    ) {
    }

    fn reposition_request(
        &mut self,
        _surface: PopupSurface,
        _positioner: PositionerState,
        _token: u32,
    ) {
    }
}

impl SeatHandler for AlpenglowCompositor {
    type KeyboardFocus = WlSurface;
    type PointerFocus = WlSurface;
    type TouchFocus = WlSurface;

    fn seat_state(&mut self) -> &mut SeatState<Self> {
        &mut self.seat_state
    }

    fn cursor_image(
        &mut self,
        _seat: &Seat<Self>,
        _image: smithay::input::pointer::CursorImageStatus,
    ) {
    }

    fn focus_changed(&mut self, _seat: &Seat<Self>, _focused: Option<&WlSurface>) {}
}

impl SelectionHandler for AlpenglowCompositor {
    type SelectionUserData = ();
}

impl DataDeviceHandler for AlpenglowCompositor {
    fn data_device_state(&self) -> &DataDeviceState {
        &self.data_device_state
    }
}

use std::os::unix::io::OwnedFd;

impl smithay::wayland::selection::data_device::ServerDndGrabHandler for AlpenglowCompositor {
    fn send(&mut self, _mime_type: String, _fd: OwnedFd, _seat: Seat<Self>) {}
}

impl smithay::wayland::selection::data_device::ClientDndGrabHandler for AlpenglowCompositor {}

// ── Delegate macros ──

delegate_compositor!(AlpenglowCompositor);
delegate_xdg_shell!(AlpenglowCompositor);
delegate_shm!(AlpenglowCompositor);
delegate_seat!(AlpenglowCompositor);
delegate_data_device!(AlpenglowCompositor);

// ── Client state ──

#[derive(Default)]
pub struct ClientState {
    pub compositor_state: CompositorClientState,
}

impl ClientData for ClientState {
    fn initialized(&self, _client_id: ClientId) {}
    fn disconnected(&self, _client_id: ClientId, _reason: DisconnectReason) {}
}

// ── Start ──

pub fn start() -> (
    mpsc::Sender<CompositorCommand>,
    mpsc::Receiver<CompositorEvent>,
) {
    let (event_tx, event_rx) = mpsc::channel();
    let (cmd_tx, cmd_rx) = mpsc::channel();

    thread::spawn(move || {
        use std::os::unix::fs::PermissionsExt;
        use std::path::PathBuf;

        let mut display: Display<AlpenglowCompositor> = Display::new().unwrap();
        let dh = display.handle();

        // Create socket directory
        let runtime = std::env::var("XDG_RUNTIME_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| std::env::temp_dir());
        let sock_dir = runtime.join("alpenglowed");
        let _ = std::fs::create_dir_all(&sock_dir);

        // Find available wayland socket name
        let sock_name = "alpenglowed-0";
        let sock_path = sock_dir.join(sock_name);
        let _ = std::fs::remove_file(&sock_path);

        let listener = ListeningSocket::bind(sock_name).unwrap();
        let _ = std::fs::set_permissions(&sock_path, std::fs::Permissions::from_mode(0o666));

        let mut state = AlpenglowCompositor::new(&dh, event_tx, cmd_rx, sock_name.to_string());

        // Add keyboard and pointer
        let _ = state.seat.add_keyboard(Default::default(), 200, 25);
        state.seat.add_pointer();

        // Set env so child processes connect to our compositor
        std::env::set_var("WAYLAND_DISPLAY", sock_name);
        std::env::set_var("ALPENGLOW_COMPOSITOR", "1");

        let start_time = std::time::Instant::now();

        // Main loop: accept clients, dispatch, process commands
        loop {
            state.process_commands();

            // Accept new client connections
            AlpenglowCompositor::accept_clients(&listener, &display);

            // Dispatch wayland events
            let _ = display.dispatch_clients(&mut state);
            let _ = display.flush_clients();

            // Send frame events to clients
            state.send_frames(start_time.elapsed().as_millis() as u32);

            // Sleep a bit to avoid busy-waiting
            std::thread::sleep(std::time::Duration::from_millis(16)); // ~60fps
        }
    });

    (cmd_tx, event_rx)
}
