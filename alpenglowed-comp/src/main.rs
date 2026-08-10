//! alpenglowed's compositor.
//!
//! The bar is not this process. This owns the session — outputs, input,
//! and every client surface — and the GPUI bar connects to it as a
//! privileged layer-shell client, the way cosmic-panel does with
//! cosmic-comp. GPUI's Linux backend is a Wayland client renderer with its
//! own Vulkan swapchain; there is no API to render it inside this
//! compositor's GLES pass without forking the toolkit, and running two
//! graphics APIs on one device with hand-managed fences is not a trade
//! worth making.
//!
//! Milestone 0: a nested compositor that maps real windows, delivers real
//! input, and advertises a real output.

mod handlers;
mod input;
mod state;
#[cfg(feature = "winit")]
mod winit;

use smithay::reexports::{calloop::EventLoop, wayland_server::Display};

use crate::state::Alpenglowed;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    let mut event_loop: EventLoop<Alpenglowed> = EventLoop::try_new().expect("event loop");
    let display: Display<Alpenglowed> = Display::new().expect("wayland display");
    let mut state = Alpenglowed::new(&mut event_loop, display);

    #[cfg(feature = "winit")]
    winit::run(&mut event_loop, &mut state);

    tracing::info!(socket = %state.socket_name, "alpenglowed compositor up");
    std::env::set_var("WAYLAND_DISPLAY", &state.socket_name);

    event_loop
        .run(None, &mut state, |state| {
            state.space.refresh();
            state.popups.cleanup();
            let _ = state.display_handle.flush_clients();
        })
        .expect("event loop");
}
