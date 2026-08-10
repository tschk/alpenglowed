//! The nested backend.
//!
//! Runs the whole compositor inside a window on whatever session you are
//! already in, which is the fast dev loop. The udev/DRM backend that
//! replaces it on a TTY is milestone 3; keeping this one forever is the
//! point, because it stays the cheapest way to test.

use std::time::Duration;

use smithay::{
    backend::{
        renderer::{
            damage::OutputDamageTracker, element::surface::WaylandSurfaceRenderElement,
            gles::GlesRenderer,
        },
        winit::{self, WinitEvent},
    },
    desktop::space::render_output,
    output::{Mode, Output, PhysicalProperties, Subpixel},
    reexports::calloop::EventLoop,
    utils::{Rectangle, Transform},
};

use crate::state::Alpenglowed;

pub fn run(event_loop: &mut EventLoop<'static, Alpenglowed>, state: &mut Alpenglowed) {
    let (mut backend, winit) = winit::init::<GlesRenderer>().expect("no winit backend");

    let size = backend.window_size();
    let mode = Mode {
        size,
        refresh: 60_000,
    };
    let output = Output::new(
        "winit".to_string(),
        PhysicalProperties {
            size: (0, 0).into(),
            subpixel: Subpixel::Unknown,
            make: "alpenglow".into(),
            model: "nested".into(),
        },
    );
    let _global = output.create_global::<Alpenglowed>(&state.display_handle);
    output.change_current_state(Some(mode), Some(Transform::Flipped180), None, Some((0, 0).into()));
    output.set_preferred(mode);
    state.space.map_output(&output, (0, 0));

    let mut damage_tracker = OutputDamageTracker::from_output(&output);

    event_loop
        .handle()
        .insert_source(winit, move |event, _, state| match event {
            WinitEvent::Resized { size, .. } => {
                output.change_current_state(
                    Some(Mode {
                        size,
                        refresh: 60_000,
                    }),
                    None,
                    None,
                    None,
                );
            }
            WinitEvent::Input(event) => state.process_input(event),
            WinitEvent::Redraw => {
                let size = backend.window_size();
                let damage = Rectangle::from_size(size);
                let (renderer, mut framebuffer) = backend.bind().expect("bind");
                render_output::<GlesRenderer, WaylandSurfaceRenderElement<GlesRenderer>, _, _>(
                    &output,
                    renderer,
                    &mut framebuffer,
                    1.0,
                    0,
                    [&state.space],
                    &[],
                    &mut damage_tracker,
                    [0.02, 0.04, 0.08, 1.0],
                )
                .expect("render");
                drop(framebuffer);

                backend.submit(Some(&[damage])).expect("submit");

                // Tell every mapped client the frame it drew has been shown,
                // or it will never draw another one.
                state.space.elements().for_each(|window| {
                    window.send_frame(
                        &output,
                        state.start_time.elapsed(),
                        Some(Duration::ZERO),
                        |_, _| Some(output.clone()),
                    )
                });
                state.space.refresh();
                state.popups.cleanup();
                let _ = state.display_handle.flush_clients();

                backend.window().request_redraw();
            }
            WinitEvent::CloseRequested => state.loop_signal.stop(),
            _ => {}
        })
        .expect("winit event source");
}
