//! Protocol handlers.
//!
//! Milestone 0 is the minimum a real client needs: compositor, shm,
//! xdg-shell with working popups, seat, output, and the data device.
//! Popups matter more than they look — an empty `new_popup` means no menus
//! and no dropdowns in any application.

use smithay::{
    delegate_compositor, delegate_data_device, delegate_output, delegate_seat, delegate_shm,
    delegate_xdg_shell,
    desktop::{
        find_popup_root_surface, PopupKeyboardGrab, PopupKind, PopupPointerGrab, Window,
    },
    input::{
        pointer::Focus,
        Seat, SeatHandler, SeatState,
    },
    reexports::{
        wayland_protocols::xdg::shell::server::xdg_toplevel,
        wayland_server::{
            protocol::{wl_buffer, wl_seat, wl_surface::WlSurface},
            Client,
        },
    },
    utils::Serial,
    wayland::{
        buffer::BufferHandler,
        compositor::{
            get_parent, is_sync_subsurface, CompositorClientState, CompositorHandler,
            CompositorState,
        },
        selection::data_device::{
            ClientDndGrabHandler, DataDeviceHandler, DataDeviceState, ServerDndGrabHandler,
        },
        selection::SelectionHandler,
        shell::xdg::{
            PopupSurface, PositionerState, ToplevelSurface, XdgShellHandler, XdgShellState,
        },
        shm::{ShmHandler, ShmState},
    },
};

use crate::state::{Alpenglowed, ClientState};

impl CompositorHandler for Alpenglowed {
    fn compositor_state(&mut self) -> &mut CompositorState {
        &mut self.compositor_state
    }

    fn client_compositor_state<'a>(&self, client: &'a Client) -> &'a CompositorClientState {
        &client.get_data::<ClientState>().unwrap().compositor_state
    }

    fn commit(&mut self, surface: &WlSurface) {
        on_commit_buffer_handler::<Self>(surface);

        if !is_sync_subsurface(surface) {
            let mut root = surface.clone();
            while let Some(parent) = get_parent(&root) {
                root = parent;
            }
            if let Some(window) = self
                .space
                .elements()
                .find(|w| {
                    w.toplevel()
                        .map(|t| t.wl_surface() == &root)
                        .unwrap_or(false)
                })
                .cloned()
            {
                window.on_commit();
            }
        }

        // A newly mapped toplevel has to be told a size before it will draw.
        if let Some(window) = self
            .space
            .elements()
            .find(|w| {
                w.toplevel()
                    .map(|t| t.wl_surface() == surface)
                    .unwrap_or(false)
            })
            .cloned()
        {
            let initial_configure_sent =
                smithay::wayland::compositor::with_states(surface, |states| {
                    states
                        .data_map
                        .get::<smithay::wayland::shell::xdg::XdgToplevelSurfaceData>()
                        .map(|d| d.lock().unwrap().initial_configure_sent)
                        .unwrap_or(true)
                });
            if !initial_configure_sent {
                if let Some(toplevel) = window.toplevel() {
                    toplevel.send_configure();
                }
            }
        }

        self.popups.commit(surface);
        self.queue_redraw();
    }
}

use smithay::backend::renderer::utils::on_commit_buffer_handler;

impl BufferHandler for Alpenglowed {
    fn buffer_destroyed(&mut self, _buffer: &wl_buffer::WlBuffer) {}
}

impl ShmHandler for Alpenglowed {
    fn shm_state(&self) -> &ShmState {
        &self.shm_state
    }
}

impl XdgShellHandler for Alpenglowed {
    fn xdg_shell_state(&mut self) -> &mut XdgShellState {
        &mut self.xdg_shell_state
    }

    fn new_toplevel(&mut self, surface: ToplevelSurface) {
        // Offer a real initial size from the output so clients are not stuck
        // waiting on a 0x0 configure forever.
        if let Some(output) = self.space.outputs().next() {
            if let Some(geo) = self.space.output_geometry(output) {
                let w = (geo.size.w * 3 / 4).max(320);
                let h = (geo.size.h * 3 / 4).max(240);
                surface.with_pending_state(|state| {
                    state.size = Some((w, h).into());
                });
            }
        }
        surface.send_configure();

        let window = Window::new_wayland_window(surface);
        // Cascade instead of stacking every window at the origin.
        let n = self.space.elements().count() as i32;
        let loc = (40 + n * 32, 40 + n * 32);
        self.space.map_element(window, loc, true);
        self.queue_redraw();
    }

    fn new_popup(&mut self, surface: PopupSurface, _positioner: PositionerState) {
        if let Err(err) = self.popups.track_popup(PopupKind::Xdg(surface)) {
            tracing::warn!("untracked popup: {err}");
        }
        self.queue_redraw();
    }

    fn reposition_request(
        &mut self,
        surface: PopupSurface,
        positioner: PositionerState,
        token: u32,
    ) {
        surface.with_pending_state(|state| {
            state.geometry = positioner.get_geometry();
            state.positioner = positioner;
        });
        surface.send_repositioned(token);
    }

    fn grab(&mut self, surface: PopupSurface, seat: wl_seat::WlSeat, serial: Serial) {
        let Some(seat) = Seat::<Self>::from_resource(&seat) else {
            return;
        };
        let popup = PopupKind::Xdg(surface);
        let Ok(root) = find_popup_root_surface(&popup) else {
            return;
        };
        match self.popups.grab_popup(root, popup, &seat, serial) {
            Ok(grab) => {
                if let Some(keyboard) = self.seat.get_keyboard() {
                    keyboard.set_grab(self, PopupKeyboardGrab::new(&grab), serial);
                }
                if let Some(pointer) = self.seat.get_pointer() {
                    pointer.set_grab(self, PopupPointerGrab::new(&grab), serial, Focus::Keep);
                }
            }
            Err(err) => tracing::debug!("popup grab refused: {err:?}"),
        }
        self.queue_redraw();
    }

    fn toplevel_destroyed(&mut self, surface: ToplevelSurface) {
        let window = self
            .space
            .elements()
            .find(|w| w.toplevel().map(|t| t == &surface).unwrap_or(false))
            .cloned();
        if let Some(window) = window {
            self.space.unmap_elem(&window);
            self.queue_redraw();
        }
    }

    fn fullscreen_request(
        &mut self,
        surface: ToplevelSurface,
        _output: Option<smithay::reexports::wayland_server::protocol::wl_output::WlOutput>,
    ) {
        surface.with_pending_state(|state| {
            state.states.set(xdg_toplevel::State::Fullscreen);
        });
        surface.send_configure();
        self.queue_redraw();
    }
}

impl SeatHandler for Alpenglowed {
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

impl smithay::wayland::output::OutputHandler for Alpenglowed {}

impl SelectionHandler for Alpenglowed {
    type SelectionUserData = ();
}

impl DataDeviceHandler for Alpenglowed {
    fn data_device_state(&self) -> &DataDeviceState {
        &self.data_device_state
    }
}

impl ClientDndGrabHandler for Alpenglowed {}
impl ServerDndGrabHandler for Alpenglowed {}

delegate_compositor!(Alpenglowed);
delegate_shm!(Alpenglowed);
delegate_xdg_shell!(Alpenglowed);
delegate_seat!(Alpenglowed);
delegate_output!(Alpenglowed);
delegate_data_device!(Alpenglowed);
