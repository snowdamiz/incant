//! Physical ownership survives game replacement, gameplay values do not. A held
//! control stays suppressed until its actual zero/release, then a new press can
//! enter the replacement. No dead-zone or synthetic press is invented here.
use crate::session::Result;
use incant_input::{
    GamepadAxis, GamepadButton, InputEvent, KeyCode, MAX_EVENTS_PER_TICK, MAX_GAMEPADS,
    MouseButton, TouchPhase,
};
use std::collections::{BTreeMap, BTreeSet};
#[derive(Clone, Default)]
struct Held {
    keys: BTreeSet<KeyCode>,
    mouse: BTreeSet<MouseButton>,
    touches: BTreeSet<u32>,
}
#[derive(Default)]
struct Pad {
    buttons: [bool; 17],
    axes: [bool; 4],
    blocked_buttons: [bool; 17],
    blocked_axes: [bool; 4],
}
pub(crate) struct InputFence {
    held: Held,
    blocked: Held,
    pads: BTreeMap<u32, Pad>,
    announce: bool,
    output: Vec<InputEvent>,
}
impl Default for InputFence {
    fn default() -> Self {
        Self {
            held: Held::default(),
            blocked: Held::default(),
            pads: BTreeMap::new(),
            announce: false,
            output: Vec::with_capacity(MAX_EVENTS_PER_TICK),
        }
    }
}
fn button(b: GamepadButton) -> usize {
    match b {
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
fn axis(a: GamepadAxis) -> usize {
    match a {
        GamepadAxis::LeftX => 0,
        GamepadAxis::LeftY => 1,
        GamepadAxis::RightX => 2,
        GamepadAxis::RightY => 3,
    }
}
fn digital<T: Ord + Copy>(
    held: &mut BTreeSet<T>,
    blocked: &mut BTreeSet<T>,
    key: T,
    down: bool,
) -> bool {
    let suppress = blocked.contains(&key);
    if down {
        held.insert(key);
    } else {
        held.remove(&key);
        blocked.remove(&key);
    }
    !suppress
}
impl InputFence {
    pub fn replace_session(&mut self) {
        self.blocked = self.held.clone();
        for pad in self.pads.values_mut() {
            pad.blocked_buttons = pad.buttons;
            pad.blocked_axes = pad.axes;
        }
        self.announce = true;
        self.output.clear();
    }
    /// Bounded before cloning. Hosts call once per accepted input packet; any
    /// refusal terminates that host run, so no retry can publish a filtered prefix.
    pub fn filter(&mut self, events: &[InputEvent]) -> Result<&[InputEvent]> {
        if events.len() > MAX_EVENTS_PER_TICK {
            return Err("restart input event limit".into());
        }
        self.output.clear();
        if self.announce {
            for id in self.pads.keys() {
                self.output.push(InputEvent::GamepadConnected { id: *id });
            }
            self.announce = false;
        }
        for event in events {
            let pass = match event {
                InputEvent::Key { code, down } => {
                    digital(&mut self.held.keys, &mut self.blocked.keys, *code, *down)
                }
                InputEvent::MouseButton { button, down } => digital(
                    &mut self.held.mouse,
                    &mut self.blocked.mouse,
                    *button,
                    *down,
                ),
                InputEvent::GamepadConnected { id } | InputEvent::GamepadDisconnected { id } => {
                    if matches!(event, InputEvent::GamepadConnected { .. }) {
                        if !self.pads.contains_key(id) && self.pads.len() == MAX_GAMEPADS {
                            return Err("restart gamepad count limit".into());
                        }
                        self.pads.entry(*id).or_default();
                    } else {
                        self.pads.remove(id);
                    }
                    true
                }
                InputEvent::GamepadButton {
                    id,
                    button: b,
                    value,
                } => {
                    if !value.is_finite() || !(0. ..=1.).contains(value) {
                        return Err("restart gamepad value bounds".into());
                    }
                    if let Some(pad) = self.pads.get_mut(id) {
                        let b = button(*b);
                        let suppress = pad.blocked_buttons[b];
                        pad.buttons[b] = *value != 0.;
                        if *value == 0. {
                            pad.blocked_buttons[b] = false;
                        }
                        !suppress
                    } else {
                        true
                    } // ordinary input rejects an unknown device
                }
                InputEvent::GamepadAxis { id, axis: a, value } => {
                    if !value.is_finite() || !(-1. ..=1.).contains(value) {
                        return Err("restart gamepad value bounds".into());
                    }
                    if let Some(pad) = self.pads.get_mut(id) {
                        let a = axis(*a);
                        let suppress = pad.blocked_axes[a];
                        pad.axes[a] = *value != 0.;
                        if *value == 0. {
                            pad.blocked_axes[a] = false;
                        }
                        !suppress
                    } else {
                        true
                    }
                }
                InputEvent::Touch { id, phase, .. } => {
                    // The ordinary runtime admits at most16 simultaneous touches.
                    let down = !matches!(phase, TouchPhase::Up | TouchPhase::Cancel);
                    if down
                        && !self.held.touches.contains(id)
                        && self.held.touches.len() == incant_input::MAX_TOUCHES
                    {
                        return Err("restart touch limit".into());
                    }
                    digital(&mut self.held.touches, &mut self.blocked.touches, *id, down)
                }
                _ => true,
            };
            if pass {
                if self.output.len() == MAX_EVENTS_PER_TICK {
                    return Err("restart input event limit".into());
                }
                self.output.push(event.clone());
            }
        }
        Ok(&self.output)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use incant_input::InputRuntime;
    #[test]
    fn fresh_game_gets_topology_but_never_a_held_press_or_axis() {
        let mut fence = InputFence::default();
        let key = |down| InputEvent::Key {
            code: KeyCode::KeyR,
            down,
        };
        let pad = |value| InputEvent::GamepadButton {
            id: 4_000_000_000,
            button: GamepadButton::Start,
            value,
        };
        let stick = |value| InputEvent::GamepadAxis {
            id: 4_000_000_000,
            axis: GamepadAxis::LeftX,
            value,
        };
        let mut old = InputRuntime::default();
        old.advance(
            fence
                .filter(&[
                    InputEvent::GamepadConnected { id: 4_000_000_000 },
                    key(true),
                    pad(1.),
                    stick(0.7),
                ])
                .unwrap(),
            1. / 60.,
        )
        .unwrap();
        fence.replace_session();
        let mut fresh = InputRuntime::default();
        fresh
            .advance(
                fence.filter(&[key(true), pad(0.8), stick(0.6)]).unwrap(),
                1. / 60.,
            )
            .unwrap();
        assert!(fresh.frame().keyboard.held.is_empty());
        assert!(
            fresh.frame().gamepads[&4_000_000_000]
                .buttons
                .held
                .is_empty()
        );
        assert_eq!(fresh.frame().gamepads[&4_000_000_000].left_stick, [0., 0.]);
        fresh
            .advance(
                fence.filter(&[key(false), pad(0.), stick(0.)]).unwrap(),
                1. / 60.,
            )
            .unwrap();
        assert!(fresh.frame().keyboard.released.is_empty());
        assert!(
            fresh.frame().gamepads[&4_000_000_000]
                .buttons
                .released
                .is_empty()
        );
        fresh
            .advance(
                fence.filter(&[key(true), pad(1.), stick(0.5)]).unwrap(),
                1. / 60.,
            )
            .unwrap();
        assert!(fresh.frame().keyboard.pressed.contains(&KeyCode::KeyR));
        assert!(
            fresh.frame().gamepads[&4_000_000_000]
                .buttons
                .pressed
                .contains(&GamepadButton::Start)
        );
        assert_eq!(fresh.frame().gamepads[&4_000_000_000].left_stick, [0.5, 0.]);
    }
}
