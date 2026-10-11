//! Engine-validated input encoded with the same generated records used by guests.
use crate::BoundaryError;
use incant_input::{ActionKind, Buttons, Gesture, InputRuntime, TouchPhase};
use incant_wasm_abi as abi;
use std::collections::BTreeSet;
#[path = "input_tags.rs"]
mod tags;
pub use abi::{INPUT_MAGIC as MAGIC, INPUT_MAX_BYTES as MAX_BYTES};
use tags::{GAMEPAD_BUTTONS, button, key, mouse};
const _: () = {
    assert!(abi::INPUT_MAX_GAMEPADS == incant_input::MAX_GAMEPADS);
    assert!(abi::INPUT_MAX_TOUCHES == incant_input::MAX_TOUCHES);
    assert!(abi::INPUT_MAX_DEVICE_CHANGES == incant_input::MAX_EVENTS_PER_TICK);
    assert!(
        abi::INPUT_MAX_TOUCH_CHANGES
            == 2 * incant_input::MAX_EVENTS_PER_TICK + incant_input::MAX_TOUCHES
    );
    assert!(
        abi::INPUT_MAX_GESTURES
            == incant_input::MAX_EVENTS_PER_TICK + incant_input::MAX_TOUCHES + 1
    );
};
struct Writer<'a>(&'a mut Vec<u8>);
impl Writer<'_> {
    fn bytes(&mut self, value: &[u8]) -> Result<(), BoundaryError> {
        if value.len() > MAX_BYTES - self.0.len() {
            return Err(BoundaryError::Limit("input snapshot"));
        }
        self.0.extend_from_slice(value);
        Ok(())
    }
    fn count(&mut self, value: usize, limit: usize) -> Result<(), BoundaryError> {
        if value > limit {
            return Err(BoundaryError::Limit("input collection"));
        }
        self.bytes(&(value as u32).to_le_bytes())
    }
    fn devices(&mut self, values: &BTreeSet<u32>) -> Result<(), BoundaryError> {
        self.count(values.len(), abi::INPUT_MAX_DEVICE_CHANGES)?;
        for value in values {
            self.bytes(&value.to_le_bytes())?;
        }
        Ok(())
    }
}
macro_rules! record {
    ($writer:expr,$value:expr) => {{
        let value = $value;
        if !value.is_finite() {
            return Err(BoundaryError::Abi("nonfinite input"));
        }
        $writer.bytes(&value.encode())?;
    }};
}
fn bits<T: Ord + Copy>(set: &BTreeSet<T>, tag: impl Fn(T) -> u8) -> u128 {
    set.iter()
        .fold(0, |bits, value| bits | (1u128 << tag(*value)))
}
fn button_sets<T: Ord + Copy>(buttons: &Buttons<T>, tag: impl Fn(T) -> u8) -> [u128; 3] {
    [&buttons.held, &buttons.pressed, &buttons.released].map(|s| bits(s, &tag))
}
/// No authoring validation or JSON. The registry owns section order and records.
pub(crate) fn encode(
    input: &InputRuntime,
    tick: u64,
    output: &mut Vec<u8>,
) -> Result<(), BoundaryError> {
    output.clear();
    if output.capacity() < MAX_BYTES {
        output.reserve_exact(MAX_BYTES);
    }
    let f = input.frame();
    let mut w = Writer(output);
    record!(
        w,
        abi::InputHeader {
            magic: *MAGIC,
            length: 0,
            focused: u32::from(f.focused),
            tick
        }
    );
    for section in abi::INPUT_SECTIONS {
        match section {
            abi::InputSection::Actions => {
                w.count(f.actions.len(), abi::INPUT_MAX_ACTIONS)?;
                for (name, value) in &f.actions {
                    if name.len() > abi::INPUT_MAX_NAME_BYTES {
                        return Err(BoundaryError::Limit("input action name"));
                    }
                    record!(
                        w,
                        abi::Action {
                            name_length: name.len() as u8,
                            kind: match value.kind {
                                ActionKind::Button => abi::ActionKind::Button,
                                ActionKind::Axis1 => abi::ActionKind::Axis1,
                                ActionKind::Axis2 => abi::ActionKind::Axis2,
                            } as u8,
                            flags: u8::from(value.active)
                                | (u8::from(value.pressed) << 1)
                                | (u8::from(value.released) << 2),
                            reserved: 0,
                            value: value.value
                        }
                    );
                    w.bytes(name.as_bytes())?;
                }
            }
            abi::InputSection::Keyboard => {
                let [held, pressed, released] = button_sets(&f.keyboard, key);
                record!(
                    w,
                    abi::Keyboard {
                        held,
                        pressed,
                        released
                    }
                );
            }
            abi::InputSection::Mouse => {
                record!(
                    w,
                    abi::Mouse {
                        buttons: button_sets(&f.mouse.buttons, mouse).map(|v| v as u8),
                        has_position: u8::from(f.mouse.position.is_some()),
                        position: f.mouse.position.unwrap_or([0.; 2]),
                        motion: f.mouse.motion,
                        wheel_pixels: f.mouse.wheel_pixels,
                        wheel_lines: f.mouse.wheel_lines
                    }
                );
            }
            abi::InputSection::Gamepads => {
                w.count(f.gamepads.len(), abi::INPUT_MAX_GAMEPADS)?;
                for (id, pad) in &f.gamepads {
                    let [held, pressed, released] =
                        button_sets(&pad.buttons, button).map(|n| n as u32);
                    record!(
                        w,
                        abi::Gamepad {
                            id: *id,
                            held,
                            pressed,
                            released,
                            present: pad.values.keys().fold(0, |b, v| b | (1 << button(*v))),
                            values: GAMEPAD_BUTTONS.map(|b| pad
                                .values
                                .get(&b)
                                .copied()
                                .unwrap_or(0.)),
                            left_stick: pad.left_stick,
                            right_stick: pad.right_stick
                        }
                    );
                }
            }
            abi::InputSection::Connected => w.devices(&f.connected)?,
            abi::InputSection::Disconnected => w.devices(&f.disconnected)?,
            abi::InputSection::Touches => {
                w.count(f.touches.len(), abi::INPUT_MAX_TOUCHES)?;
                for (id, touch) in &f.touches {
                    record!(
                        w,
                        abi::Touch {
                            id: *id,
                            position: touch.position,
                            delta: touch.delta
                        }
                    );
                }
            }
            abi::InputSection::TouchChanges => {
                w.count(f.touch_changes.len(), abi::INPUT_MAX_TOUCH_CHANGES)?;
                for change in &f.touch_changes {
                    record!(
                        w,
                        abi::TouchChange {
                            id: change.id,
                            phase: match change.phase {
                                TouchPhase::Down => abi::TouchPhase::Down,
                                TouchPhase::Move => abi::TouchPhase::Move,
                                TouchPhase::Up => abi::TouchPhase::Up,
                                TouchPhase::Cancel => abi::TouchPhase::Cancel,
                            } as u8,
                            reserved: [0; 3],
                            position: change.position
                        }
                    );
                }
            }
            abi::InputSection::Gestures => {
                w.count(f.gestures.len(), abi::INPUT_MAX_GESTURES)?;
                for gesture in &f.gestures {
                    match gesture {
                        Gesture::Tap { id, position } | Gesture::LongPress { id, position } => {
                            record!(
                                w,
                                abi::GestureHeader {
                                    kind: if matches!(gesture, Gesture::Tap { .. }) {
                                        abi::GestureKind::Tap
                                    } else {
                                        abi::GestureKind::LongPress
                                    } as u8,
                                    reserved: [0; 3],
                                    id: *id
                                }
                            );
                            record!(
                                w,
                                abi::PointGesture {
                                    position: *position
                                }
                            );
                        }
                        Gesture::Swipe {
                            id,
                            direction,
                            distance,
                            duration_seconds,
                        } => {
                            record!(
                                w,
                                abi::GestureHeader {
                                    kind: abi::GestureKind::Swipe as u8,
                                    reserved: [0; 3],
                                    id: *id
                                }
                            );
                            record!(
                                w,
                                abi::SwipeGesture {
                                    direction: *direction,
                                    distance: *distance,
                                    duration_seconds: *duration_seconds
                                }
                            );
                        }
                        Gesture::Pinch {
                            ids,
                            center,
                            scale_delta,
                            rotation_delta,
                        } => {
                            record!(
                                w,
                                abi::GestureHeader {
                                    kind: abi::GestureKind::Pinch as u8,
                                    reserved: [0; 3],
                                    id: ids[0]
                                }
                            );
                            record!(
                                w,
                                abi::PinchGesture {
                                    second_id: ids[1],
                                    center: *center,
                                    scale_delta: *scale_delta,
                                    rotation_delta: *rotation_delta
                                }
                            );
                        }
                    }
                }
            }
        }
    }
    let length = output.len() as u32;
    output[abi::InputHeader::LENGTH_OFFSET..abi::InputHeader::LENGTH_OFFSET + 4]
        .copy_from_slice(&length.to_le_bytes());
    Ok(())
}
