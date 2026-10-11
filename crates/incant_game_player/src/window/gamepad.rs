//! Gilrs is enabled only for the desktop window host. No backend handles,
//! mapping strings, device names or force-feedback capabilities enter gameplay.
use super::{Error, input::PendingInput};
use gilrs::{Axis, Button, EventType, Gilrs, GilrsBuilder};
use incant_input::{
    GamepadAxis, GamepadButton, MAX_EVENTS_PER_TICK,
    gamepad::{DeviceEvent, GamepadAdapter},
};

pub struct Gamepads {
    backend: Gilrs,
    adapter: GamepadAdapter<usize>,
}
impl Gamepads {
    pub fn new() -> Result<Self, Error> {
        let backend = require_backend(
            GilrsBuilder::new()
                // Default filters can internally drain an unbounded number of
                // dropped jitter/dead-zone events. Actions own our dead zones; the
                // two D-pad axes are translated explicitly below.
                .with_default_filters(false)
                .with_force_feedback(false)
                .build(),
        )?;
        let mut adapter = GamepadAdapter::default();
        for (id, _) in backend.gamepads() {
            adapter.event(usize::from(id), DeviceEvent::Connected, false)?;
        }
        Ok(Self { backend, adapter })
    }
    pub fn poll(&mut self, input: &mut PendingInput, accepting: bool) -> Result<(), Error> {
        if accepting {
            self.synchronize(input)?;
        }
        drain(&mut self.adapter, input, accepting, || {
            self.backend
                .next_event()
                .map(|event| (usize::from(event.id), event.event))
        })?;
        self.backend.inc();
        Ok(())
    }
    pub fn synchronize(&mut self, input: &mut PendingInput) -> Result<(), Error> {
        while let Some(event) = self.adapter.next_connection_event() {
            input.push(event)?;
        }
        Ok(())
    }
    pub fn discard_pending(&mut self) {
        self.adapter.discard_pending();
    }
    pub fn commit(&mut self) {
        self.adapter.commit();
    }
}
fn require_backend(backend: Result<Gilrs, gilrs::Error>) -> Result<Gilrs, Error> {
    // In particular, NotImplemented contains a dummy backend that must never
    // be accepted as a functioning native gamepad host.
    backend.map_err(|_| Error::GamepadBackend)
}
fn drain(
    adapter: &mut GamepadAdapter<usize>,
    input: &mut PendingInput,
    accepting: bool,
    mut next: impl FnMut() -> Option<(usize, EventType)>,
) -> Result<(), Error> {
    for _ in 0..MAX_EVENTS_PER_TICK {
        let Some((id, event)) = next() else {
            return Ok(());
        };
        translate(adapter, input, id, event, accepting)?;
    }
    // Overflow, including ignored controls while paused, is an explicit host
    // failure. Never silently lose a disconnect/button release or loop forever.
    if next().is_some() {
        return Err(Error::InputCapacity);
    }
    Ok(())
}
fn translate(
    adapter: &mut GamepadAdapter<usize>,
    input: &mut PendingInput,
    id: usize,
    event: EventType,
    accepting: bool,
) -> Result<(), Error> {
    let mut send = |event| -> Result<(), Error> {
        if let Some(event) = adapter.event(id, event, accepting)? {
            input.push(event)?;
        }
        Ok(())
    };
    match event {
        EventType::Connected => send(DeviceEvent::Connected),
        EventType::Disconnected => send(DeviceEvent::Disconnected),
        // Gilrs emits measured ButtonChanged after digital press/release too.
        // Pressed is a threshold edge for analog triggers, not a value of1.0.
        EventType::ButtonChanged(native, value, _) => {
            if let Some(button) = button(native) {
                send(DeviceEvent::Button {
                    button,
                    value: f64::from(value),
                })?;
            }
            Ok(())
        }
        EventType::AxisChanged(native, value, _) => {
            let value = f64::from(value);
            if !value.is_finite() || !(-1. ..=1.).contains(&value) {
                return Err(Error::GamepadInput(incant_input::InputError::Analog));
            }
            let axis = match native {
                Axis::LeftStickX => Some((GamepadAxis::LeftX, value)),
                Axis::LeftStickY => Some((GamepadAxis::LeftY, -value)),
                Axis::RightStickX => Some((GamepadAxis::RightX, value)),
                Axis::RightStickY => Some((GamepadAxis::RightY, -value)),
                _ => None,
            };
            if let Some((axis, value)) = axis {
                send(DeviceEvent::Axis { axis, value })?;
            } else if matches!(native, Axis::DPadX | Axis::DPadY) {
                let (negative, positive) = if native == Axis::DPadX {
                    (GamepadButton::DpadLeft, GamepadButton::DpadRight)
                } else {
                    (GamepadButton::DpadDown, GamepadButton::DpadUp)
                };
                // Release the opposite direction before pressing its replacement.
                let values = if value > 0. {
                    [(negative, 0.), (positive, value)]
                } else {
                    [(positive, 0.), (negative, -value)]
                };
                for (button, value) in values {
                    send(DeviceEvent::Button { button, value })?;
                }
            }
            // Z/unknown axes have no counterpart in the current typed contract.
            Ok(())
        }
        _ => Ok(()), // repeats, redundant digital edges and unmapped controls
    }
}
fn button(native: Button) -> Option<GamepadButton> {
    Some(match native {
        Button::South => GamepadButton::South,
        Button::East => GamepadButton::East,
        Button::North => GamepadButton::North,
        Button::West => GamepadButton::West,
        Button::LeftTrigger => GamepadButton::LeftShoulder,
        Button::RightTrigger => GamepadButton::RightShoulder,
        Button::LeftTrigger2 => GamepadButton::LeftTrigger,
        Button::RightTrigger2 => GamepadButton::RightTrigger,
        Button::Select => GamepadButton::Select,
        Button::Start => GamepadButton::Start,
        Button::Mode => GamepadButton::Guide,
        Button::LeftThumb => GamepadButton::LeftStick,
        Button::RightThumb => GamepadButton::RightStick,
        Button::DPadUp => GamepadButton::DpadUp,
        Button::DPadDown => GamepadButton::DpadDown,
        Button::DPadLeft => GamepadButton::DpadLeft,
        Button::DPadRight => GamepadButton::DpadRight,
        _ => return None,
    })
}

#[cfg(test)]
mod tests;
