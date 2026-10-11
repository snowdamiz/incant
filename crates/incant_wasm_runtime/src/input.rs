//! `incant_input_v1` packed little-endian snapshot. Only engine-validated input
//! can enter this encoder; no public arbitrary InputFrame ingestion exists.
use crate::BoundaryError;
use incant_input::{
    ActionKind, Buttons, GamepadButton, Gesture, InputRuntime, KeyCode, MouseButton, TouchPhase,
};
use std::collections::BTreeSet;

pub const MAX_BYTES: usize = 128 * 1024;
pub const MAGIC: &[u8; 8] = b"INPINP01";

struct Writer<'a>(&'a mut Vec<u8>);
impl Writer<'_> {
    fn bytes(&mut self, value: &[u8]) -> Result<(), BoundaryError> {
        if value.len() > MAX_BYTES - self.0.len() {
            return Err(BoundaryError::Limit("input snapshot"));
        }
        self.0.extend_from_slice(value);
        Ok(())
    }
    fn byte(&mut self, value: u8) -> Result<(), BoundaryError> {
        self.bytes(&[value])
    }
    fn count(&mut self, value: usize, limit: usize) -> Result<(), BoundaryError> {
        if value > limit {
            return Err(BoundaryError::Limit("input collection"));
        }
        self.bytes(&(value as u32).to_le_bytes())
    }
    fn u32(&mut self, value: u32) -> Result<(), BoundaryError> {
        self.bytes(&value.to_le_bytes())
    }
    fn numbers<const N: usize>(&mut self, values: [f64; N]) -> Result<(), BoundaryError> {
        for value in values {
            if !value.is_finite() {
                return Err(BoundaryError::Abi("nonfinite input"));
            }
            self.bytes(&value.to_le_bytes())?;
        }
        Ok(())
    }
    fn devices(&mut self, values: &BTreeSet<u32>) -> Result<(), BoundaryError> {
        self.count(values.len(), incant_input::MAX_EVENTS_PER_TICK)?;
        for value in values {
            self.u32(*value)?;
        }
        Ok(())
    }
}
fn bits<T: Ord + Copy>(set: &BTreeSet<T>, tag: impl Fn(T) -> u8) -> u128 {
    set.iter()
        .fold(0, |bits, value| bits | (1u128 << tag(*value)))
}
fn button_sets<T: Ord + Copy>(buttons: &Buttons<T>, tag: impl Fn(T) -> u8) -> [u128; 3] {
    [&buttons.held, &buttons.pressed, &buttons.released].map(|set| bits(set, &tag))
}

/// Reuse bounded storage across ticks. The complete frame is read synchronously
/// from InputRuntime; this performs no action-map/schema validation or JSON work.
pub(crate) fn encode(
    input: &InputRuntime,
    tick: u64,
    output: &mut Vec<u8>,
) -> Result<(), BoundaryError> {
    output.clear();
    if output.capacity() < MAX_BYTES {
        output.reserve_exact(MAX_BYTES - output.len());
    }
    let f = input.frame();
    let mut w = Writer(output);
    w.bytes(MAGIC)?;
    w.u32(0)?;
    w.u32(u32::from(f.focused))?;
    w.bytes(&tick.to_le_bytes())?;
    w.count(f.actions.len(), 64)?;
    for (name, value) in &f.actions {
        if name.len() > 64 {
            return Err(BoundaryError::Limit("input action name"));
        }
        w.byte(name.len() as u8)?;
        w.byte(match value.kind {
            ActionKind::Button => 0,
            ActionKind::Axis1 => 1,
            ActionKind::Axis2 => 2,
        })?;
        w.byte(
            u8::from(value.active) | u8::from(value.pressed) << 1 | u8::from(value.released) << 2,
        )?;
        w.byte(0)?;
        w.numbers(value.value)?;
        w.bytes(name.as_bytes())?;
    }
    for mask in button_sets(&f.keyboard, key) {
        w.bytes(&mask.to_le_bytes())?;
    }
    for mask in button_sets(&f.mouse.buttons, mouse) {
        w.byte(mask as u8)?;
    }
    w.byte(u8::from(f.mouse.position.is_some()))?;
    for point in [
        f.mouse.position.unwrap_or([0.; 2]),
        f.mouse.motion,
        f.mouse.wheel_pixels,
        f.mouse.wheel_lines,
    ] {
        w.numbers(point)?;
    }
    w.count(f.gamepads.len(), incant_input::MAX_GAMEPADS)?;
    for (id, pad) in &f.gamepads {
        w.u32(*id)?;
        for mask in button_sets(&pad.buttons, button) {
            w.u32(mask as u32)?;
        }
        let present = pad
            .values
            .keys()
            .fold(0u32, |bits, b| bits | (1 << button(*b)));
        w.u32(present)?;
        for b in GAMEPAD_BUTTONS {
            w.numbers([pad.values.get(&b).copied().unwrap_or(0.)])?;
        }
        w.numbers(pad.left_stick)?;
        w.numbers(pad.right_stick)?;
    }
    w.devices(&f.connected)?;
    w.devices(&f.disconnected)?;
    w.count(f.touches.len(), incant_input::MAX_TOUCHES)?;
    for (id, touch) in &f.touches {
        w.u32(*id)?;
        w.numbers(touch.position)?;
        w.numbers(touch.delta)?;
    }
    w.count(f.touch_changes.len(), incant_input::MAX_EVENTS_PER_TICK)?;
    for change in &f.touch_changes {
        w.u32(change.id)?;
        w.bytes(&[
            match change.phase {
                TouchPhase::Down => 0,
                TouchPhase::Move => 1,
                TouchPhase::Up => 2,
                TouchPhase::Cancel => 3,
            },
            0,
            0,
            0,
        ])?;
        w.numbers(change.position)?;
    }
    w.count(
        f.gestures.len(),
        incant_input::MAX_EVENTS_PER_TICK + incant_input::MAX_TOUCHES + 1,
    )?;
    for gesture in &f.gestures {
        match gesture {
            Gesture::Tap { id, position } | Gesture::LongPress { id, position } => {
                w.bytes(&[
                    if matches!(gesture, Gesture::Tap { .. }) {
                        1
                    } else {
                        2
                    },
                    0,
                    0,
                    0,
                ])?;
                w.u32(*id)?;
                w.numbers(*position)?;
            }
            Gesture::Swipe {
                id,
                direction,
                distance,
                duration_seconds,
            } => {
                w.bytes(&[3, 0, 0, 0])?;
                w.u32(*id)?;
                w.numbers(*direction)?;
                w.numbers([*distance, *duration_seconds])?;
            }
            Gesture::Pinch {
                ids,
                center,
                scale_delta,
                rotation_delta,
            } => {
                w.bytes(&[4, 0, 0, 0])?;
                w.u32(ids[0])?;
                w.u32(ids[1])?;
                w.numbers(*center)?;
                w.numbers([*scale_delta, *rotation_delta])?;
            }
        }
    }
    let len = output.len() as u32;
    output[8..12].copy_from_slice(&len.to_le_bytes());
    Ok(())
}
fn mouse(value: MouseButton) -> u8 {
    match value {
        MouseButton::Left => 0,
        MouseButton::Right => 1,
        MouseButton::Middle => 2,
        MouseButton::Back => 3,
        MouseButton::Forward => 4,
    }
}
const GAMEPAD_BUTTONS: [GamepadButton; 17] = [
    GamepadButton::South,
    GamepadButton::East,
    GamepadButton::West,
    GamepadButton::North,
    GamepadButton::LeftShoulder,
    GamepadButton::RightShoulder,
    GamepadButton::LeftTrigger,
    GamepadButton::RightTrigger,
    GamepadButton::Select,
    GamepadButton::Start,
    GamepadButton::Guide,
    GamepadButton::LeftStick,
    GamepadButton::RightStick,
    GamepadButton::DpadUp,
    GamepadButton::DpadDown,
    GamepadButton::DpadLeft,
    GamepadButton::DpadRight,
];
fn button(value: GamepadButton) -> u8 {
    match value {
        GamepadButton::South => 0,
        GamepadButton::East => 1,
        GamepadButton::West => 2,
        GamepadButton::North => 3,
        GamepadButton::LeftShoulder => 4,
        GamepadButton::RightShoulder => 5,
        GamepadButton::LeftTrigger => 6,
        GamepadButton::RightTrigger => 7,
        GamepadButton::Select => 8,
        GamepadButton::Start => 9,
        GamepadButton::Guide => 10,
        GamepadButton::LeftStick => 11,
        GamepadButton::RightStick => 12,
        GamepadButton::DpadUp => 13,
        GamepadButton::DpadDown => 14,
        GamepadButton::DpadLeft => 15,
        GamepadButton::DpadRight => 16,
    }
}

// Explicit wire tags, independent of Rust enum discriminants/reordering.
fn key(value: KeyCode) -> u8 {
    match value {
        KeyCode::KeyA => 0,
        KeyCode::KeyB => 1,
        KeyCode::KeyC => 2,
        KeyCode::KeyD => 3,
        KeyCode::KeyE => 4,
        KeyCode::KeyF => 5,
        KeyCode::KeyG => 6,
        KeyCode::KeyH => 7,
        KeyCode::KeyI => 8,
        KeyCode::KeyJ => 9,
        KeyCode::KeyK => 10,
        KeyCode::KeyL => 11,
        KeyCode::KeyM => 12,
        KeyCode::KeyN => 13,
        KeyCode::KeyO => 14,
        KeyCode::KeyP => 15,
        KeyCode::KeyQ => 16,
        KeyCode::KeyR => 17,
        KeyCode::KeyS => 18,
        KeyCode::KeyT => 19,
        KeyCode::KeyU => 20,
        KeyCode::KeyV => 21,
        KeyCode::KeyW => 22,
        KeyCode::KeyX => 23,
        KeyCode::KeyY => 24,
        KeyCode::KeyZ => 25,
        KeyCode::Digit0 => 26,
        KeyCode::Digit1 => 27,
        KeyCode::Digit2 => 28,
        KeyCode::Digit3 => 29,
        KeyCode::Digit4 => 30,
        KeyCode::Digit5 => 31,
        KeyCode::Digit6 => 32,
        KeyCode::Digit7 => 33,
        KeyCode::Digit8 => 34,
        KeyCode::Digit9 => 35,
        KeyCode::ArrowUp => 36,
        KeyCode::ArrowDown => 37,
        KeyCode::ArrowLeft => 38,
        KeyCode::ArrowRight => 39,
        KeyCode::Space => 40,
        KeyCode::Enter => 41,
        KeyCode::Escape => 42,
        KeyCode::Tab => 43,
        KeyCode::Backspace => 44,
        KeyCode::Delete => 45,
        KeyCode::Insert => 46,
        KeyCode::Home => 47,
        KeyCode::End => 48,
        KeyCode::PageUp => 49,
        KeyCode::PageDown => 50,
        KeyCode::ShiftLeft => 51,
        KeyCode::ShiftRight => 52,
        KeyCode::ControlLeft => 53,
        KeyCode::ControlRight => 54,
        KeyCode::AltLeft => 55,
        KeyCode::AltRight => 56,
        KeyCode::MetaLeft => 57,
        KeyCode::MetaRight => 58,
        KeyCode::CapsLock => 59,
        KeyCode::NumLock => 60,
        KeyCode::ScrollLock => 61,
        KeyCode::Pause => 62,
        KeyCode::PrintScreen => 63,
        KeyCode::ContextMenu => 64,
        KeyCode::Backquote => 65,
        KeyCode::Minus => 66,
        KeyCode::Equal => 67,
        KeyCode::BracketLeft => 68,
        KeyCode::BracketRight => 69,
        KeyCode::Backslash => 70,
        KeyCode::Semicolon => 71,
        KeyCode::Quote => 72,
        KeyCode::Comma => 73,
        KeyCode::Period => 74,
        KeyCode::Slash => 75,
        KeyCode::Numpad0 => 76,
        KeyCode::Numpad1 => 77,
        KeyCode::Numpad2 => 78,
        KeyCode::Numpad3 => 79,
        KeyCode::Numpad4 => 80,
        KeyCode::Numpad5 => 81,
        KeyCode::Numpad6 => 82,
        KeyCode::Numpad7 => 83,
        KeyCode::Numpad8 => 84,
        KeyCode::Numpad9 => 85,
        KeyCode::NumpadAdd => 86,
        KeyCode::NumpadSubtract => 87,
        KeyCode::NumpadMultiply => 88,
        KeyCode::NumpadDivide => 89,
        KeyCode::NumpadDecimal => 90,
        KeyCode::NumpadEnter => 91,
        KeyCode::F1 => 92,
        KeyCode::F2 => 93,
        KeyCode::F3 => 94,
        KeyCode::F4 => 95,
        KeyCode::F5 => 96,
        KeyCode::F6 => 97,
        KeyCode::F7 => 98,
        KeyCode::F8 => 99,
        KeyCode::F9 => 100,
        KeyCode::F10 => 101,
        KeyCode::F11 => 102,
        KeyCode::F12 => 103,
    }
}
