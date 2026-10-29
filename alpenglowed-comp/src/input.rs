//! Input, forwarded to clients.
//!
//! The old `src/compositor.rs` left this as `// TBD`, which is why nothing
//! launched under it could ever be typed into. Keyboard goes to the focused
//! surface; pointer motion and buttons go to whatever is under the cursor,
//! and clicking focuses.

use smithay::{
    backend::input::{
        AbsolutePositionEvent, Axis, AxisSource, ButtonState, Event, InputBackend, InputEvent,
        KeyState, KeyboardKeyEvent, PointerAxisEvent, PointerButtonEvent,
    },
    input::{
        keyboard::{keysyms, FilterResult},
        pointer::{AxisFrame, ButtonEvent, MotionEvent},
    },
    utils::SERIAL_COUNTER,
};

use crate::state::{Alpenglowed, CompIpcCommand, CompLayoutAction};

impl Alpenglowed {
    #[cfg_attr(not(feature = "winit"), allow(dead_code))]
    pub fn process_input<I: InputBackend>(&mut self, event: InputEvent<I>) {
        match event {
            InputEvent::Keyboard { event } => {
                let serial = SERIAL_COUNTER.next_serial();
                let time = Event::time_msec(&event);
                let press_state = event.state();
                let Some(keyboard) = self.seat.get_keyboard() else {
                    return;
                };
                let intercepted = keyboard.input::<CompIpcCommand, _>(
                    self,
                    event.key_code(),
                    press_state,
                    serial,
                    time,
                    |_, modifiers, handle| {
                        if press_state != KeyState::Pressed || !modifiers.logo {
                            return FilterResult::Forward;
                        }
                        let sym = handle.modified_sym().raw();
                        match sym {
                            keysyms::KEY_space => FilterResult::Intercept(
                                CompIpcCommand::DesktopAction("apps".to_string()),
                            ),
                            keysyms::KEY_m | keysyms::KEY_M => {
                                FilterResult::Intercept(CompIpcCommand::CycleWindowMode)
                            }
                            keysyms::KEY_q | keysyms::KEY_Q => FilterResult::Intercept(
                                CompIpcCommand::Layout(CompLayoutAction::CloseFocused),
                            ),
                            keysyms::KEY_v | keysyms::KEY_V => FilterResult::Intercept(
                                CompIpcCommand::Layout(CompLayoutAction::ToggleFloat),
                            ),
                            keysyms::KEY_r | keysyms::KEY_R => FilterResult::Intercept(
                                CompIpcCommand::Layout(CompLayoutAction::CycleColumnWidth),
                            ),
                            keysyms::KEY_f | keysyms::KEY_F => {
                                let act = if modifiers.shift {
                                    CompLayoutAction::ToggleFullscreen
                                } else {
                                    CompLayoutAction::MaximizeColumn
                                };
                                FilterResult::Intercept(CompIpcCommand::Layout(act))
                            }
                            keysyms::KEY_comma => FilterResult::Intercept(CompIpcCommand::Layout(
                                CompLayoutAction::ConsumeWindowIntoColumn,
                            )),
                            keysyms::KEY_period => FilterResult::Intercept(CompIpcCommand::Layout(
                                CompLayoutAction::ExpelWindowFromColumn,
                            )),
                            keysyms::KEY_h | keysyms::KEY_H => {
                                let act = if modifiers.shift {
                                    CompLayoutAction::MoveColumnLeft
                                } else {
                                    CompLayoutAction::FocusColumnLeft
                                };
                                FilterResult::Intercept(CompIpcCommand::Layout(act))
                            }
                            keysyms::KEY_l | keysyms::KEY_L => {
                                let act = if modifiers.shift {
                                    CompLayoutAction::MoveColumnRight
                                } else {
                                    CompLayoutAction::FocusColumnRight
                                };
                                FilterResult::Intercept(CompIpcCommand::Layout(act))
                            }
                            keysyms::KEY_Left => {
                                let act = if modifiers.shift {
                                    CompLayoutAction::MoveColumnLeft
                                } else if modifiers.alt {
                                    CompLayoutAction::SnapLeftHalf
                                } else {
                                    CompLayoutAction::FocusColumnLeft
                                };
                                FilterResult::Intercept(CompIpcCommand::Layout(act))
                            }
                            keysyms::KEY_Right => {
                                let act = if modifiers.shift {
                                    CompLayoutAction::MoveColumnRight
                                } else if modifiers.alt {
                                    CompLayoutAction::SnapRightHalf
                                } else {
                                    CompLayoutAction::FocusColumnRight
                                };
                                FilterResult::Intercept(CompIpcCommand::Layout(act))
                            }
                            keysyms::KEY_Up => FilterResult::Intercept(CompIpcCommand::Layout(
                                CompLayoutAction::Maximize,
                            )),
                            keysyms::KEY_Down => FilterResult::Intercept(CompIpcCommand::Layout(
                                CompLayoutAction::Restore,
                            )),
                            _ => FilterResult::Forward,
                        }
                    },
                );
                if let Some(cmd) = intercepted {
                    if let Err(error) = self.handle_ipc_command(cmd) {
                        tracing::warn!(%error, "keybinding command failed");
                    }
                }
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
                        state,
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
