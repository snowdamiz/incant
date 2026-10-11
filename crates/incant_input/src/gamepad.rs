//! Bounded device-to-player identity and pending connection reconciliation.
//! Hosts discard pending controls on focus/suspend and drain the device while
//! inactive. Reconciliation restores topology, never cached held controls.
use crate::{GamepadAxis, GamepadButton, InputError, InputEvent, MAX_GAMEPADS};

#[derive(Debug, Clone, Copy)]
pub enum DeviceEvent {
    Connected,
    Disconnected,
    Button { button: GamepadButton, value: f64 },
    Axis { axis: GamepadAxis, value: f64 },
}
#[derive(Debug)]
struct Pad<I> {
    device: I,
    connected: bool,
    queued: bool,
    published: bool,
}
/// IDs 0..MAX_GAMEPADS are assigned in discovery order and retained for the
/// session, including disconnects. A seventeenth distinct device is refused rather
/// than reusing an identity that a recording/action binding already observed.
#[derive(Debug)]
pub struct GamepadAdapter<I> {
    pads: Vec<Pad<I>>,
}
impl<I: Eq> Default for GamepadAdapter<I> {
    fn default() -> Self {
        Self {
            pads: Vec::with_capacity(MAX_GAMEPADS),
        }
    }
}
impl<I: Eq> GamepadAdapter<I> {
    /// `accepting` means the host can collect controls for an active tick.
    /// Duplicate topology hints and late controls from disconnected devices
    /// are harmless. Unknown controls do not fabricate a connection.
    pub fn event(
        &mut self,
        device: I,
        event: DeviceEvent,
        accepting: bool,
    ) -> Result<Option<InputEvent>, InputError> {
        match event {
            DeviceEvent::Button { value, .. }
                if !value.is_finite() || !(0. ..=1.).contains(&value) =>
            {
                return Err(InputError::Analog);
            }
            DeviceEvent::Axis { value, .. }
                if !value.is_finite() || !(-1. ..=1.).contains(&value) =>
            {
                return Err(InputError::Analog);
            }
            _ => {}
        }
        let index = match self.pads.iter().position(|p| p.device == device) {
            Some(index) => index,
            None if matches!(event, DeviceEvent::Connected) => {
                if self.pads.len() == MAX_GAMEPADS {
                    return Err(InputError::Devices);
                }
                self.pads.push(Pad {
                    device,
                    connected: false,
                    queued: false,
                    published: false,
                });
                self.pads.len() - 1
            }
            None => return Ok(None),
        };
        let id = index as u32; // capped at16 before insertion
        let pad = &mut self.pads[index];
        match event {
            DeviceEvent::Connected => pad.connected = true,
            DeviceEvent::Disconnected => pad.connected = false,
            DeviceEvent::Button { button, value } if accepting && pad.connected && pad.queued => {
                return Ok(Some(InputEvent::GamepadButton { id, button, value }));
            }
            DeviceEvent::Axis { axis, value } if accepting && pad.connected && pad.queued => {
                return Ok(Some(InputEvent::GamepadAxis { id, axis, value }));
            }
            _ => return Ok(None),
        }
        Ok(if accepting {
            connection(index, pad)
        } else {
            None
        })
    }
    /// Emit these before collecting controls/resuming a tick. At most16 events
    /// are pending. The caller uses its normal bounded input queue.
    pub fn next_connection_event(&mut self) -> Option<InputEvent> {
        self.pads
            .iter_mut()
            .enumerate()
            .find_map(|(id, pad)| connection(id, pad))
    }
    /// Call only after the complete input batch was accepted by the game tick.
    pub fn commit(&mut self) {
        for pad in &mut self.pads {
            pad.published = pad.queued;
        }
    }
    /// Pair with clearing the host's pending queue; keep physical connection
    /// knowledge and the last successfully published topology.
    pub fn discard_pending(&mut self) {
        for pad in &mut self.pads {
            pad.queued = pad.published;
        }
    }
}
fn connection<I>(id: usize, pad: &mut Pad<I>) -> Option<InputEvent> {
    if pad.connected == pad.queued {
        return None;
    }
    pad.queued = pad.connected;
    Some(if pad.connected {
        InputEvent::GamepadConnected { id: id as u32 }
    } else {
        InputEvent::GamepadDisconnected { id: id as u32 }
    })
}
