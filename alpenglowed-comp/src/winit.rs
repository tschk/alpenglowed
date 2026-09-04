//! The nested backend.
//!
//! Runs the whole compositor inside a window on whatever session you are
//! already in, which is the fast dev loop. The udev/DRM backend that
//! replaces it on a TTY is milestone 3; keeping this one forever is the
//! point, because it stays the cheapest way to test.

use std::{cell::RefCell, rc::Rc, time::Duration};

use smithay::{
    backend::{
        renderer::{
            damage::OutputDamageTracker, element::surface::WaylandSurfaceRenderElement,
            gles::GlesRenderer,
        },
        winit::{self, WinitEvent},
    },
    desktop::space::render_output,
    output::{Mode, Output, PhysicalProperties, Scale, Subpixel},
    reexports::calloop::{ping, EventLoop},
    utils::{Size, Transform},
};

use crate::state::Alpenglowed;

fn physical_mm_from_px(
    size: Size<i32, smithay::utils::Physical>,
) -> Size<i32, smithay::utils::Raw> {
    // ~96 DPI: mm = px * 25.4 / 96
    let w = ((size.w as f64) * 25.4 / 96.0).round().max(1.0) as i32;
    let h = ((size.h as f64) * 25.4 / 96.0).round().max(1.0) as i32;
    (w, h).into()
}

fn integer_scale(scale_factor: f64) -> i32 {
    scale_factor.round().max(1.0) as i32
}

pub fn run(event_loop: &mut EventLoop<'static, Alpenglowed>, state: &mut Alpenglowed) {
    let (backend, winit) = winit::init::<GlesRenderer>().expect("no winit backend");

    let size = backend.window_size();
    let scale_i = integer_scale(backend.scale_factor());
    let mode = Mode {
        size,
        refresh: 60_000,
    };
    let output = Output::new(
        "winit".to_string(),
        PhysicalProperties {
            size: physical_mm_from_px(size),
            subpixel: Subpixel::Unknown,
            make: "alpenglow".into(),
            model: "nested".into(),
        },
    );
    let _global = output.create_global::<Alpenglowed>(&state.display_handle);
    output.change_current_state(
        Some(mode),
        Some(Transform::Normal),
        Some(Scale::Integer(scale_i)),
        Some((0, 0).into()),
    );
    output.set_preferred(mode);
    state.space.map_output(&output, (0, 0));

    let mut damage_tracker = OutputDamageTracker::from_output(&output);

    // Ping from protocol handlers / input so we redraw without spinning.
    let (redraw_ping, redraw_source) = ping::make_ping().expect("redraw ping");
    state.redraw_ping = Some(redraw_ping);

    let backend = Rc::new(RefCell::new(backend));
    let backend_for_ping = backend.clone();
    event_loop
        .handle()
        .insert_source(redraw_source, move |_, _, _| {
            backend_for_ping.borrow().window().request_redraw();
        })
        .expect("redraw ping source");

    // First frame.
    backend.borrow().window().request_redraw();

    event_loop
        .handle()
        .insert_source(winit, move |event, _, state| {
            let mut backend = backend.borrow_mut();
            match event {
                WinitEvent::Resized { size, scale_factor } => {
                    let scale_i = integer_scale(scale_factor);
                    let mode = Mode {
                        size,
                        refresh: 60_000,
                    };
                    output.change_current_state(
                        Some(mode),
                        Some(Transform::Normal),
                        Some(Scale::Integer(scale_i)),
                        None,
                    );
                    output.set_preferred(mode);
                    backend.window().request_redraw();
                }
                WinitEvent::Input(event) => {
                    state.process_input(event);
                    backend.window().request_redraw();
                }
                WinitEvent::Redraw => {
                    let age = backend.buffer_age().unwrap_or(0);
                    let (renderer, mut framebuffer) = backend.bind().expect("bind");
                    let result = render_output::<
                        GlesRenderer,
                        WaylandSurfaceRenderElement<GlesRenderer>,
                        _,
                        _,
                    >(
                        &output,
                        renderer,
                        &mut framebuffer,
                        1.0,
                        age,
                        [&state.space],
                        &[],
                        &mut damage_tracker,
                        [0.02, 0.04, 0.08, 1.0],
                    )
                    .expect("render");
                    drop(framebuffer);

                    if let Some(damage) = result.damage {
                        backend.submit(Some(damage)).expect("submit");
                    }

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
                    // No unconditional request_redraw — queue_redraw / input /
                    // resize / client commits drive the next frame.
                }
                WinitEvent::CloseRequested => state.loop_signal.stop(),
                _ => {}
            }
        })
        .expect("winit event source");
}
