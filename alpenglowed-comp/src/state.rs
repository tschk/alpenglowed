//! The whole compositor, in one struct.
//!
//! Split the way cosmic-comp splits it: everything protocol- and
//! desktop-related lives in `Alpenglowed`, and the backend is a separate
//! value the event loop owns. That keeps the state monomorphic instead of
//! infecting every `impl` with anvil's `<BackendData: Backend>`.

use std::{sync::Arc, time::Instant};

use smithay::{
    desktop::{PopupManager, Space, Window, WindowSurfaceType},
    input::{Seat, SeatState},
    reexports::{
        calloop::{generic::Generic, EventLoop, Interest, LoopSignal, Mode, PostAction},
        wayland_server::{
            backend::{ClientData, ClientId, DisconnectReason},
            protocol::wl_surface::WlSurface,
            Display, DisplayHandle,
        },
    },
    utils::{Logical, Point},
    wayland::{
        compositor::{CompositorClientState, CompositorState},
        output::OutputManagerState,
        selection::data_device::DataDeviceState,
        shell::xdg::XdgShellState,
        shm::ShmState,
        socket::ListeningSocketSource,
    },
};

pub struct Alpenglowed {
    pub start_time: Instant,
    pub display_handle: DisplayHandle,
    pub loop_signal: LoopSignal,
    pub socket_name: String,

    pub space: Space<Window>,
    pub popups: PopupManager,

    pub compositor_state: CompositorState,
    pub xdg_shell_state: XdgShellState,
    pub shm_state: ShmState,
    #[allow(dead_code)]
    pub output_manager_state: OutputManagerState,
    pub seat_state: SeatState<Self>,
    pub data_device_state: DataDeviceState,
    pub seat: Seat<Self>,
}

impl Alpenglowed {
    pub fn new(event_loop: &mut EventLoop<'static, Self>, display: Display<Self>) -> Self {
        let display_handle = display.handle();
        let loop_signal = event_loop.get_signal();

        let compositor_state = CompositorState::new::<Self>(&display_handle);
        let xdg_shell_state = XdgShellState::new::<Self>(&display_handle);
        let shm_state = ShmState::new::<Self>(&display_handle, vec![]);
        // Without this global many toolkits refuse to map a window at all.
        let output_manager_state = OutputManagerState::new_with_xdg_output::<Self>(&display_handle);
        let mut seat_state = SeatState::new();
        let data_device_state = DataDeviceState::new::<Self>(&display_handle);

        let mut seat: Seat<Self> = seat_state.new_wl_seat(&display_handle, "alpenglowed");
        seat.add_keyboard(Default::default(), 200, 25)
            .expect("a seat with no keyboard is not a desktop");
        seat.add_pointer();

        let socket_name = Self::init_socket(event_loop, display);

        Self {
            start_time: Instant::now(),
            display_handle,
            loop_signal,
            socket_name,
            space: Space::default(),
            popups: PopupManager::default(),
            compositor_state,
            xdg_shell_state,
            shm_state,
            output_manager_state,
            seat_state,
            data_device_state,
            seat,
        }
    }

    /// Listen, and hand every readable client fd back to wayland-server.
    fn init_socket(event_loop: &mut EventLoop<'static, Self>, display: Display<Self>) -> String {
        let source = ListeningSocketSource::new_auto().expect("no wayland socket available");
        let socket_name = source.socket_name().to_string_lossy().into_owned();
        let handle = event_loop.handle();

        handle
            .insert_source(source, move |stream, _, state| {
                state
                    .display_handle
                    .insert_client(stream, Arc::new(ClientState::default()))
                    .expect("client insert");
            })
            .expect("listening socket source");

        handle
            .insert_source(
                Generic::new(display, Interest::READ, Mode::Level),
                |_, display, state| {
                    // Safety: the display is only ever dispatched from here.
                    unsafe { display.get_mut().dispatch_clients(state).unwrap() };
                    Ok(PostAction::Continue)
                },
            )
            .expect("wayland display source");

        socket_name
    }

    /// What is under the pointer, in surface-local coordinates.
    pub fn surface_under(
        &self,
        pos: Point<f64, Logical>,
    ) -> Option<(WlSurface, Point<f64, Logical>)> {
        self.space
            .element_under(pos)
            .and_then(|(window, location)| {
                window
                    .surface_under(pos - location.to_f64(), WindowSurfaceType::ALL)
                    .map(|(surface, offset)| (surface, (location + offset).to_f64()))
            })
    }
}

#[derive(Default)]
pub struct ClientState {
    pub compositor_state: CompositorClientState,
}

impl ClientData for ClientState {
    fn initialized(&self, _client_id: ClientId) {}
    fn disconnected(&self, _client_id: ClientId, _reason: DisconnectReason) {}
}
