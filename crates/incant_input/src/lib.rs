//! Bounded fixed-tick input processing. Device adapters and recordings use the
//! same validated events; scripts see snapshots, never device or window handles.
mod gestures;
mod recording;
mod types;
pub use recording::{InputRecording, InputReplay, MAX_RECORDING_BYTES, RecordingError};
use std::collections::BTreeMap;
use thiserror::Error;
pub use types::*;

pub const MAX_EVENTS_PER_TICK: usize = 1024;
pub const MAX_GAMEPADS: usize = 16;
pub const MAX_TOUCHES: usize = 16;

#[derive(Debug, Error, PartialEq)]
pub enum InputError {
    #[error("input tick must be finite and between 1/240 and 1 seconds")]
    Step,
    #[error("input tick exceeds {MAX_EVENTS_PER_TICK} events")]
    Events,
    #[error("input coordinates must be finite and within one million logical pixels")]
    Coordinates,
    #[error("gamepad value is outside its normalized range")]
    Analog,
    #[error("gamepad is not connected")]
    MissingGamepad,
    #[error("gamepad is already connected")]
    DuplicateGamepad,
    #[error("input exceeds the gamepad or touch device limit")]
    Devices,
    #[error("touch is not active")]
    MissingTouch,
    #[error("touch ID is already active")]
    DuplicateTouch,
}

#[derive(Debug, Clone, Default)]
pub struct InputRuntime {
    frame: InputFrame,
    time: f64,
    contacts: BTreeMap<u32, gestures::Contact>,
}
impl InputRuntime {
    pub fn frame(&self) -> &InputFrame {
        &self.frame
    }

    /// An invalid packet changes no state, time, edges or gesture recognition.
    pub fn advance(&mut self, events: &[InputEvent], dt: f64) -> Result<&InputFrame, InputError> {
        if !dt.is_finite() || !(1. / 240. ..=1.).contains(&dt) || self.time + dt <= self.time {
            return Err(InputError::Step);
        }
        if events.len() > MAX_EVENTS_PER_TICK {
            return Err(InputError::Events);
        }
        let previous_pair = gestures::pair(&self.frame.touches);
        let mut next = self.clone();
        next.time += dt;
        next.clear_edges();
        for event in events {
            next.apply(event, dt)?;
        }
        next.finish_gestures(previous_pair);
        *self = next;
        Ok(&self.frame)
    }

    fn clear_edges(&mut self) {
        let f = &mut self.frame;
        f.keyboard.clear_edges();
        f.mouse.buttons.clear_edges();
        f.mouse.motion = [0.; 2];
        f.mouse.wheel_lines = [0.; 2];
        f.mouse.wheel_pixels = [0.; 2];
        f.connected.clear();
        f.disconnected.clear();
        f.touch_changes.clear();
        f.gestures.clear();
        for pad in f.gamepads.values_mut() {
            pad.buttons.clear_edges();
        }
        for touch in f.touches.values_mut() {
            touch.delta = [0.; 2];
        }
    }
    fn apply(&mut self, event: &InputEvent, dt: f64) -> Result<(), InputError> {
        // Validate numeric values even when an unfocused host drops the event.
        match event {
            InputEvent::PointerPosition { position } | InputEvent::Touch { position, .. } => {
                coordinates(*position)?
            }
            InputEvent::PointerMotion { delta } | InputEvent::Wheel { delta, .. } => {
                coordinates(*delta)?
            }
            InputEvent::GamepadButton { value, .. }
                if !value.is_finite() || !(0. ..=1.).contains(value) =>
            {
                return Err(InputError::Analog);
            }
            InputEvent::GamepadAxis { value, .. }
                if !value.is_finite() || !(-1. ..=1.).contains(value) =>
            {
                return Err(InputError::Analog);
            }
            _ => {}
        }
        if let InputEvent::Focus { focused } = *event {
            self.frame.focused = focused;
            if !focused {
                self.release_focus();
            }
            return Ok(());
        }
        if let InputEvent::GamepadConnected { id } = *event {
            if self.frame.gamepads.contains_key(&id) {
                return Err(InputError::DuplicateGamepad);
            }
            if self.frame.gamepads.len() >= MAX_GAMEPADS {
                return Err(InputError::Devices);
            }
            self.frame.gamepads.insert(id, GamepadFrame::default());
            self.frame.connected.insert(id);
            return Ok(());
        }
        if let InputEvent::GamepadDisconnected { id } = *event {
            if self.frame.gamepads.remove(&id).is_none() {
                return Err(InputError::MissingGamepad);
            }
            self.frame.disconnected.insert(id);
            return Ok(());
        }
        if !self.frame.focused {
            return Ok(());
        }
        match *event {
            InputEvent::Key { code, down } => self.frame.keyboard.update(code, down),
            InputEvent::MouseButton { button, down } => {
                self.frame.mouse.buttons.update(button, down)
            }
            InputEvent::PointerPosition { position } => {
                if let Some(previous) = self.frame.mouse.position {
                    add(&mut self.frame.mouse.motion, subtract(position, previous));
                }
                self.frame.mouse.position = Some(position);
            }
            InputEvent::PointerMotion { delta } => add(&mut self.frame.mouse.motion, delta),
            InputEvent::Wheel { delta, unit } => add(
                match unit {
                    WheelUnit::Pixels => &mut self.frame.mouse.wheel_pixels,
                    WheelUnit::Lines => &mut self.frame.mouse.wheel_lines,
                },
                delta,
            ),
            InputEvent::GamepadButton { id, button, value } => {
                let pad = self
                    .frame
                    .gamepads
                    .get_mut(&id)
                    .ok_or(InputError::MissingGamepad)?;
                pad.values.insert(button, value);
                pad.buttons.update(button, value >= 0.5);
            }
            InputEvent::GamepadAxis { id, axis, value } => {
                let pad = self
                    .frame
                    .gamepads
                    .get_mut(&id)
                    .ok_or(InputError::MissingGamepad)?;
                *match axis {
                    GamepadAxis::LeftX => &mut pad.left_stick[0],
                    GamepadAxis::LeftY => &mut pad.left_stick[1],
                    GamepadAxis::RightX => &mut pad.right_stick[0],
                    GamepadAxis::RightY => &mut pad.right_stick[1],
                } = value;
            }
            InputEvent::Touch {
                id,
                phase,
                position,
            } => self.touch(id, phase, position, dt)?,
            InputEvent::Focus { .. }
            | InputEvent::GamepadConnected { .. }
            | InputEvent::GamepadDisconnected { .. } => unreachable!("handled above"),
        }
        Ok(())
    }
    fn release_focus(&mut self) {
        let f = &mut self.frame;
        f.keyboard.release_all();
        f.keyboard.pressed.clear();
        f.mouse.buttons.release_all();
        f.mouse.buttons.pressed.clear();
        f.mouse.position = None;
        f.mouse.motion = [0.; 2];
        f.mouse.wheel_lines = [0.; 2];
        f.mouse.wheel_pixels = [0.; 2];
        for pad in f.gamepads.values_mut() {
            pad.buttons.release_all();
            pad.buttons.pressed.clear();
            pad.values.values_mut().for_each(|value| *value = 0.);
            pad.left_stick = [0.; 2];
            pad.right_stick = [0.; 2];
        }
        for (&id, touch) in &f.touches {
            f.touch_changes.push(TouchChange {
                id,
                phase: TouchPhase::Cancel,
                position: touch.position,
            });
        }
        f.touches.clear();
        self.contacts.clear();
        f.gestures.clear();
    }
}
fn coordinates(v: [f64; 2]) -> Result<(), InputError> {
    if v.iter().all(|v| v.is_finite() && v.abs() <= 1_000_000.) {
        Ok(())
    } else {
        Err(InputError::Coordinates)
    }
}
fn subtract(a: [f64; 2], b: [f64; 2]) -> [f64; 2] {
    [a[0] - b[0], a[1] - b[1]]
}
fn add(a: &mut [f64; 2], b: [f64; 2]) {
    a[0] += b[0];
    a[1] += b[1];
}
