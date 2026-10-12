//! Physical mapping shared by the desktop gameplay and UI receipt adapters.
//! Unrecognized physical keys, touch and haptics are ignored.
use super::Error;
use incant_input::{InputEvent, KeyCode, MAX_EVENTS_PER_TICK, MouseButton, WheelUnit};
use tao::{
    dpi::{PhysicalPosition, validate_scale_factor},
    event::{ElementState, MouseScrollDelta, WindowEvent},
    keyboard::KeyCode as NativeKey,
};
pub struct PendingInput {
    events: Vec<InputEvent>,
    motion_start: usize,
}
impl PendingInput {
    pub fn new() -> Self {
        Self {
            events: Vec::with_capacity(MAX_EVENTS_PER_TICK),
            motion_start: 0,
        }
    }
    pub fn events(&self) -> &[InputEvent] {
        &self.events
    }
    pub fn clear(&mut self) {
        self.events.clear();
        self.motion_start = 0;
    }
    pub fn reset_focus(&mut self, focused: bool) {
        self.events.clear();
        self.events.push(InputEvent::Focus { focused: false });
        if focused {
            self.events.push(InputEvent::Focus { focused: true });
        }
        self.motion_start = self.events.len();
    }
    pub fn push(&mut self, event: InputEvent) -> Result<(), Error> {
        // Uncaptured windows have one authoritative stream: CursorMoved. Keep
        // first+latest in each run. The first initializes an unknown prior
        // position; replacing it would erase motion in the initial input batch.
        if let InputEvent::PointerPosition { position } = event {
            if !position
                .iter()
                .all(|v| v.is_finite() && v.abs() <= 1_000_000.)
            {
                return Err(Error::InputRange);
            }
            if self.events.len() - self.motion_start >= 2 {
                let Some(InputEvent::PointerPosition { position: previous }) =
                    self.events.last_mut()
                else {
                    unreachable!("private motion run")
                };
                *previous = position;
                return Ok(());
            }
        }
        if self.events.len() == MAX_EVENTS_PER_TICK {
            return Err(Error::InputCapacity);
        }
        let motion = matches!(event, InputEvent::PointerPosition { .. });
        self.events.push(event);
        if !motion {
            self.motion_start = self.events.len();
        }
        Ok(())
    }
    fn pointer(&mut self, position: PhysicalPosition<f64>, scale_factor: f64) -> Result<(), Error> {
        self.push(InputEvent::PointerPosition {
            position: logical_position(position, scale_factor)?,
        })
    }
    fn wheel(&mut self, delta: MouseScrollDelta, scale_factor: f64) -> Result<(), Error> {
        self.push(match delta {
            MouseScrollDelta::LineDelta(x, y) => InputEvent::Wheel {
                delta: [f64::from(x), f64::from(y)],
                unit: WheelUnit::Lines,
            },
            MouseScrollDelta::PixelDelta(position) => InputEvent::Wheel {
                delta: logical_position(position, scale_factor)?,
                unit: WheelUnit::Pixels,
            },
            _ => return Err(Error::InputCapability),
        })
    }
    pub fn window(&mut self, event: &WindowEvent<'_>, scale_factor: f64) -> Result<(), Error> {
        let event = match event {
            WindowEvent::KeyboardInput { event, .. } if !event.repeat => key(event.physical_key)
                .map(|code| InputEvent::Key {
                    code,
                    down: event.state == ElementState::Pressed,
                }),
            WindowEvent::MouseInput { button, state, .. } => {
                let button = match button {
                    tao::event::MouseButton::Left => Some(MouseButton::Left),
                    tao::event::MouseButton::Right => Some(MouseButton::Right),
                    tao::event::MouseButton::Middle => Some(MouseButton::Middle),
                    _ => None,
                };
                button.map(|button| InputEvent::MouseButton {
                    button,
                    down: *state == ElementState::Pressed,
                })
            }
            WindowEvent::CursorMoved { position, .. } => {
                return self.pointer(*position, scale_factor);
            }
            WindowEvent::MouseWheel { delta, .. } => return self.wheel(*delta, scale_factor),
            _ => None,
        };
        if let Some(event) = event {
            self.push(event)?;
        }
        Ok(())
    }
}
pub(super) fn logical_position(
    position: PhysicalPosition<f64>,
    scale_factor: f64,
) -> Result<[f64; 2], Error> {
    if !validate_scale_factor(scale_factor) {
        return Err(Error::InputScale);
    }
    let position = position.to_logical::<f64>(scale_factor);
    Ok([position.x, position.y])
}
pub fn key(key: NativeKey) -> Option<KeyCode> {
    Some(match key {
        NativeKey::KeyA => KeyCode::KeyA,
        NativeKey::KeyB => KeyCode::KeyB,
        NativeKey::KeyC => KeyCode::KeyC,
        NativeKey::KeyD => KeyCode::KeyD,
        NativeKey::KeyE => KeyCode::KeyE,
        NativeKey::KeyF => KeyCode::KeyF,
        NativeKey::KeyG => KeyCode::KeyG,
        NativeKey::KeyH => KeyCode::KeyH,
        NativeKey::KeyI => KeyCode::KeyI,
        NativeKey::KeyJ => KeyCode::KeyJ,
        NativeKey::KeyK => KeyCode::KeyK,
        NativeKey::KeyL => KeyCode::KeyL,
        NativeKey::KeyM => KeyCode::KeyM,
        NativeKey::KeyN => KeyCode::KeyN,
        NativeKey::KeyO => KeyCode::KeyO,
        NativeKey::KeyP => KeyCode::KeyP,
        NativeKey::KeyQ => KeyCode::KeyQ,
        NativeKey::KeyR => KeyCode::KeyR,
        NativeKey::KeyS => KeyCode::KeyS,
        NativeKey::KeyT => KeyCode::KeyT,
        NativeKey::KeyU => KeyCode::KeyU,
        NativeKey::KeyV => KeyCode::KeyV,
        NativeKey::KeyW => KeyCode::KeyW,
        NativeKey::KeyX => KeyCode::KeyX,
        NativeKey::KeyY => KeyCode::KeyY,
        NativeKey::KeyZ => KeyCode::KeyZ,
        NativeKey::Digit0 => KeyCode::Digit0,
        NativeKey::Digit1 => KeyCode::Digit1,
        NativeKey::Digit2 => KeyCode::Digit2,
        NativeKey::Digit3 => KeyCode::Digit3,
        NativeKey::Digit4 => KeyCode::Digit4,
        NativeKey::Digit5 => KeyCode::Digit5,
        NativeKey::Digit6 => KeyCode::Digit6,
        NativeKey::Digit7 => KeyCode::Digit7,
        NativeKey::Digit8 => KeyCode::Digit8,
        NativeKey::Digit9 => KeyCode::Digit9,
        NativeKey::ArrowUp => KeyCode::ArrowUp,
        NativeKey::ArrowDown => KeyCode::ArrowDown,
        NativeKey::ArrowLeft => KeyCode::ArrowLeft,
        NativeKey::ArrowRight => KeyCode::ArrowRight,
        NativeKey::Space => KeyCode::Space,
        NativeKey::Enter => KeyCode::Enter,
        NativeKey::Escape => KeyCode::Escape,
        NativeKey::Tab => KeyCode::Tab,
        NativeKey::Backspace => KeyCode::Backspace,
        NativeKey::Delete => KeyCode::Delete,
        NativeKey::Insert => KeyCode::Insert,
        NativeKey::Home => KeyCode::Home,
        NativeKey::End => KeyCode::End,
        NativeKey::PageUp => KeyCode::PageUp,
        NativeKey::PageDown => KeyCode::PageDown,
        NativeKey::ShiftLeft => KeyCode::ShiftLeft,
        NativeKey::ShiftRight => KeyCode::ShiftRight,
        NativeKey::ControlLeft => KeyCode::ControlLeft,
        NativeKey::ControlRight => KeyCode::ControlRight,
        NativeKey::AltLeft => KeyCode::AltLeft,
        NativeKey::AltRight => KeyCode::AltRight,
        NativeKey::SuperLeft => KeyCode::MetaLeft,
        NativeKey::SuperRight => KeyCode::MetaRight,
        NativeKey::CapsLock => KeyCode::CapsLock,
        NativeKey::NumLock => KeyCode::NumLock,
        NativeKey::ScrollLock => KeyCode::ScrollLock,
        NativeKey::Pause => KeyCode::Pause,
        NativeKey::PrintScreen => KeyCode::PrintScreen,
        NativeKey::ContextMenu => KeyCode::ContextMenu,
        NativeKey::Backquote => KeyCode::Backquote,
        NativeKey::Minus => KeyCode::Minus,
        NativeKey::Equal => KeyCode::Equal,
        NativeKey::BracketLeft => KeyCode::BracketLeft,
        NativeKey::BracketRight => KeyCode::BracketRight,
        NativeKey::Backslash => KeyCode::Backslash,
        NativeKey::Semicolon => KeyCode::Semicolon,
        NativeKey::Quote => KeyCode::Quote,
        NativeKey::Comma => KeyCode::Comma,
        NativeKey::Period => KeyCode::Period,
        NativeKey::Slash => KeyCode::Slash,
        NativeKey::Numpad0 => KeyCode::Numpad0,
        NativeKey::Numpad1 => KeyCode::Numpad1,
        NativeKey::Numpad2 => KeyCode::Numpad2,
        NativeKey::Numpad3 => KeyCode::Numpad3,
        NativeKey::Numpad4 => KeyCode::Numpad4,
        NativeKey::Numpad5 => KeyCode::Numpad5,
        NativeKey::Numpad6 => KeyCode::Numpad6,
        NativeKey::Numpad7 => KeyCode::Numpad7,
        NativeKey::Numpad8 => KeyCode::Numpad8,
        NativeKey::Numpad9 => KeyCode::Numpad9,
        NativeKey::NumpadAdd => KeyCode::NumpadAdd,
        NativeKey::NumpadSubtract => KeyCode::NumpadSubtract,
        NativeKey::NumpadMultiply => KeyCode::NumpadMultiply,
        NativeKey::NumpadDivide => KeyCode::NumpadDivide,
        NativeKey::NumpadDecimal => KeyCode::NumpadDecimal,
        NativeKey::NumpadEnter => KeyCode::NumpadEnter,
        NativeKey::F1 => KeyCode::F1,
        NativeKey::F2 => KeyCode::F2,
        NativeKey::F3 => KeyCode::F3,
        NativeKey::F4 => KeyCode::F4,
        NativeKey::F5 => KeyCode::F5,
        NativeKey::F6 => KeyCode::F6,
        NativeKey::F7 => KeyCode::F7,
        NativeKey::F8 => KeyCode::F8,
        NativeKey::F9 => KeyCode::F9,
        NativeKey::F10 => KeyCode::F10,
        NativeKey::F11 => KeyCode::F11,
        NativeKey::F12 => KeyCode::F12,
        _ => return None,
    })
}

pub fn validate_actions(actions: &incant_input::InputActions) -> Result<(), Error> {
    use incant_input::{InputAction, InputBinding};
    for action in actions.values() {
        let bindings = match action {
            InputAction::Button { bindings, .. }
            | InputAction::Axis1 { bindings, .. }
            | InputAction::Axis2 { bindings, .. } => bindings,
        };
        for binding in bindings {
            if !matches!(
                binding,
                InputBinding::Key { .. }
                    | InputBinding::MouseButton {
                        button: MouseButton::Left | MouseButton::Right | MouseButton::Middle,
                        ..
                    }
            ) && !matches!(binding,
                InputBinding::GamepadButton { id, .. }
                    | InputBinding::GamepadAxis { id, .. }
                    | InputBinding::GamepadStick { id, .. }
                    if (*id as u64) < incant_input::MAX_GAMEPADS as u64
            ) {
                return Err(Error::InputCapability);
            }
        }
    }
    Ok(())
}
#[cfg(test)]
mod coordinate_tests;
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn queue_is_bounded_and_focus_reset_releases_previous_buttons() {
        let mut input = PendingInput::new();
        for _ in 0..MAX_EVENTS_PER_TICK {
            input
                .push(InputEvent::Key {
                    code: KeyCode::Space,
                    down: true,
                })
                .unwrap();
        }
        assert!(matches!(
            input.push(InputEvent::Key {
                code: KeyCode::Space,
                down: false
            }),
            Err(Error::InputCapacity)
        ));
        input.reset_focus(true);
        let mut runtime = incant_input::InputRuntime::default();
        runtime
            .advance(
                &[InputEvent::Key {
                    code: KeyCode::Space,
                    down: true,
                }],
                1. / 60.,
            )
            .unwrap();
        runtime.advance(input.events(), 1. / 60.).unwrap();
        assert!(!runtime.frame().keyboard.held.contains(&KeyCode::Space));
    }
    #[test]
    fn absolute_motion_coalesces_only_without_intervening_button() {
        let mut input = PendingInput::new();
        for i in 0..10000 {
            input
                .push(InputEvent::PointerPosition {
                    position: [i as f64, 0.],
                })
                .unwrap();
        }
        assert_eq!(input.events().len(), 2);
        input
            .push(InputEvent::MouseButton {
                button: MouseButton::Left,
                down: true,
            })
            .unwrap();
        input
            .push(InputEvent::PointerPosition { position: [0., 0.] })
            .unwrap();
        assert_eq!(input.events().len(), 4);
        assert_eq!(key(NativeKey::SuperLeft), Some(KeyCode::MetaLeft));
    }
    #[test]
    fn high_rate_cursor_stream_preserves_initial_motion_and_discrete_order_without_double_counting()
    {
        for previous in [None, Some([10., 20.])] {
            let mut input = PendingInput::new();
            for i in 0..(MAX_EVENTS_PER_TICK * 4) {
                input
                    .push(InputEvent::PointerPosition {
                        position: [i as f64, 2. * i as f64],
                    })
                    .unwrap();
            }
            assert_eq!(input.events().len(), 2);
            let last = (MAX_EVENTS_PER_TICK * 4 - 1) as f64;
            let mut runtime = incant_input::InputRuntime::default();
            if let Some(position) = previous {
                runtime
                    .advance(&[InputEvent::PointerPosition { position }], 1. / 60.)
                    .unwrap();
            }
            runtime.advance(input.events(), 1. / 60.).unwrap();
            assert_eq!(runtime.frame().mouse.position, Some([last, 2. * last]));
            let initial = previous.unwrap_or([0.; 2]);
            assert_eq!(
                runtime.frame().mouse.motion,
                [last - initial[0], 2. * last - initial[1]]
            );
            input
                .push(InputEvent::MouseButton {
                    button: MouseButton::Left,
                    down: true,
                })
                .unwrap();
            input
                .push(InputEvent::PointerPosition {
                    position: [12., 24.],
                })
                .unwrap();
            input
                .push(InputEvent::PointerPosition {
                    position: [16., 32.],
                })
                .unwrap();
            assert!(matches!(
                input.events()[2],
                InputEvent::MouseButton { down: true, .. }
            ));
            assert_eq!(input.events().len(), 5);
            runtime.advance(input.events(), 1. / 60.).unwrap();
            assert_eq!(runtime.frame().mouse.motion, [16. - last, 32. - 2. * last]);
            assert!(
                runtime
                    .frame()
                    .mouse
                    .buttons
                    .pressed
                    .contains(&MouseButton::Left)
            );
        }
    }
    #[test]
    fn invalid_coalesced_sample_is_refused_without_replacing_the_last_valid_position() {
        let mut input = PendingInput::new();
        input
            .push(InputEvent::PointerPosition { position: [0.; 2] })
            .unwrap();
        input
            .push(InputEvent::PointerPosition { position: [1.; 2] })
            .unwrap();
        for position in [[f64::NAN, 0.], [1_000_001., 0.]] {
            assert!(matches!(
                input.push(InputEvent::PointerPosition { position }),
                Err(Error::InputRange)
            ));
        }
        assert_eq!(
            input.events()[1],
            InputEvent::PointerPosition { position: [1.; 2] }
        );
    }
}
