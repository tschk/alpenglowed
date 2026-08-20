//! Input, forwarded to clients.
//!
//! The old `src/compositor.rs` left this as `// TBD`, which is why nothing
//! launched under it could ever be typed into. Keyboard goes to the focused
//! surface; pointer motion and buttons go to whatever is under the cursor,
//! and clicking focuses.

use smithay::{
    backend::input::{
        AbsolutePositionEvent, Axis, AxisSource, ButtonState, Event, InputBackend, InputEvent,
        KeyboardKeyEvent, PointerAxisEvent, PointerButtonEvent,
    },
    input::{
        keyboard::FilterResult,
        pointer::{AxisFrame, ButtonEvent, MotionEvent},
    },
    utils::SERIAL_COUNTER,
};

use crate::state::Alpenglowed;

impl Alpenglowed {
    pub fn process_input<I: InputBackend>(&mut self, event: InputEvent<I>) {
        match event {
            InputEvent::Keyboard { event } => {
                let serial = SERIAL_COUNTER.next_serial();
                let time = Event::time_msec(&event);
                let Some(keyboard) = self.seat.get_keyboard() else {
                    return;
                };
                keyboard.input::<(), _>(
                    self,
                    event.key_code(),
                    event.state(),
                    serial,
                    time,
                    |_, _, _| FilterResult::Forward,
                );
            }
            InputEvent::PointerMotionAbsolute { event } => {
                let Some(output) = self.space.outputs().next().cloned() else {
                    return;
                };
                let Some(geometry) = self.space.output_geometry(&output) else {
                    return;
                };
                let position = geometry.loc.to_f64() + event.position_transformed(geometry.size);
                let serial = SERIAL_COUNTER.next_serial();
                let under = self.surface_under(position);
                let Some(pointer) = self.seat.get_pointer() else {
                    return;
                };
                pointer.motion(
                    self,
                    under,
                    &MotionEvent {
                        location: position,
                        serial,
                        time: event.time_msec(),
                    },
                );
                pointer.frame(self);
            }
            InputEvent::PointerButton { event } => {
                let serial = SERIAL_COUNTER.next_serial();
                let button = event.button_code();
                let state = event.state();

                if state == ButtonState::Pressed {
                    // Click to focus. Milestone 2 replaces this with the
                    // strip's own focus model.
                    let Some(pointer) = self.seat.get_pointer() else {
                        return;
                    };
                    let position = pointer.current_location();
                    if let Some((surface, _)) = self.surface_under(position) {
                        if let Some(keyboard) = self.seat.get_keyboard() {
                            keyboard.set_focus(self, Some(surface), serial);
                        }
                    }
                }

                let Some(pointer) = self.seat.get_pointer() else {
                    return;
                };
                pointer.button(
                    self,
                    &ButtonEvent {
                        button,
                        state: state.into(),
                        serial,
                        time: event.time_msec(),
                    },
                );
                pointer.frame(self);
            }
            InputEvent::PointerAxis { event } => {
                let mut frame = AxisFrame::new(event.time_msec()).source(AxisSource::Wheel);
                for axis in [Axis::Horizontal, Axis::Vertical] {
                    if let Some(value) = event.amount(axis) {
                        frame = frame.value(axis, value);
                    }
                }
                let Some(pointer) = self.seat.get_pointer() else {
                    return;
                };
                pointer.axis(self, frame);
                pointer.frame(self);
            }
            _ => {}
        }
    }
}
