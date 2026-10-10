//! Named input actions derived from the same ordered physical events as raw input.
use crate::{GamepadAxis, GamepadButton, Gesture, InputFrame, KeyCode, MouseButton};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use thiserror::Error;

pub type InputActions = BTreeMap<String, InputAction>;
pub const MAX_ACTIONS: usize = 64;
pub const MAX_BINDINGS: usize = 16;

#[derive(Debug, Error, PartialEq)]
pub enum ActionError {
    #[error("input actions require at most 64 names of 1–64 ASCII letters, digits or _.:-")]
    Names,
    #[error("each input action permits at most 16 bindings")]
    Bindings,
    #[error(
        "input action threshold must be in (0,1], dead zone in [0,0.95], and scales finite in [-1,1]"
    )]
    Range,
    #[error(
        "button bindings need a nonnegative X scale and zero Y; one-axis bindings need zero Y; sticks need a two-axis action"
    )]
    Dimensions,
}

fn threshold() -> f64 {
    0.5
}
fn dead_zone() -> f64 {
    0.15
}
fn scale() -> [f64; 2] {
    [1., 0.]
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum InputAction {
    Button {
        #[schemars(length(max = 16))]
        bindings: Vec<InputBinding>,
        #[serde(default = "threshold")]
        threshold: f64,
    },
    Axis1 {
        #[schemars(length(max = 16))]
        bindings: Vec<InputBinding>,
        #[serde(default = "dead_zone")]
        dead_zone: f64,
    },
    Axis2 {
        #[schemars(length(max = 16))]
        bindings: Vec<InputBinding>,
        #[serde(default = "dead_zone")]
        dead_zone: f64,
    },
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum InputBinding {
    Key {
        code: KeyCode,
        #[serde(default = "scale")]
        scale: [f64; 2],
    },
    MouseButton {
        button: MouseButton,
        #[serde(default = "scale")]
        scale: [f64; 2],
    },
    GamepadButton {
        id: u32,
        button: GamepadButton,
        #[serde(default = "scale")]
        scale: [f64; 2],
    },
    GamepadAxis {
        id: u32,
        axis: GamepadAxis,
        #[serde(default = "scale")]
        scale: [f64; 2],
    },
    GamepadStick {
        id: u32,
        stick: Stick,
        #[serde(default)]
        invert_y: bool,
    },
    Gesture {
        gesture: ActionGesture,
        #[serde(default = "scale")]
        scale: [f64; 2],
    },
}
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Stick {
    Left,
    Right,
}
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ActionGesture {
    Tap,
    LongPress,
    Swipe,
}
#[derive(Debug, Clone, Copy, PartialEq, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ActionKind {
    Button,
    Axis1,
    Axis2,
}
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
pub struct ActionState {
    pub kind: ActionKind,
    /// Button/axis1 use X only; axis2 uses a unit disc. Button value is analog.
    pub value: [f64; 2],
    pub active: bool,
    pub pressed: bool,
    pub released: bool,
}

pub fn validate_actions(actions: &InputActions) -> Result<(), ActionError> {
    if actions.len() > MAX_ACTIONS
        || actions.keys().any(|name| {
            name.is_empty()
                || name.len() > 64
                || !name
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || b"_.:-".contains(&c))
        })
    {
        return Err(ActionError::Names);
    }
    for action in actions.values() {
        let bindings = action.bindings();
        if bindings.len() > MAX_BINDINGS {
            return Err(ActionError::Bindings);
        }
        match action {
            InputAction::Button { threshold, .. }
                if !threshold.is_finite() || *threshold <= 0. || *threshold > 1. =>
            {
                return Err(ActionError::Range);
            }
            InputAction::Axis1 { dead_zone, .. } | InputAction::Axis2 { dead_zone, .. }
                if !dead_zone.is_finite() || !(0. ..=0.95).contains(dead_zone) =>
            {
                return Err(ActionError::Range);
            }
            _ => {}
        }
        for binding in bindings {
            if let Some(scale) = binding.scale() {
                if scale
                    .iter()
                    .any(|v| !v.is_finite() || !(-1. ..=1.).contains(v))
                {
                    return Err(ActionError::Range);
                }
                if (matches!(action, InputAction::Button { .. })
                    && (scale[0] < 0. || scale[1] != 0.))
                    || (matches!(action, InputAction::Axis1 { .. }) && scale[1] != 0.)
                {
                    return Err(ActionError::Dimensions);
                }
            } else if !matches!(action, InputAction::Axis2 { .. }) {
                return Err(ActionError::Dimensions);
            }
        }
    }
    Ok(())
}

impl InputAction {
    fn bindings(&self) -> &[InputBinding] {
        match self {
            Self::Button { bindings, .. }
            | Self::Axis1 { bindings, .. }
            | Self::Axis2 { bindings, .. } => bindings,
        }
    }
    fn evaluate(&self, frame: &InputFrame) -> ActionState {
        let mut value = [0_f64; 2];
        for binding in self.bindings() {
            let input = binding.value(frame);
            match self {
                Self::Button { .. } => value[0] = value[0].max(input[0]),
                _ => {
                    value[0] += input[0];
                    value[1] += input[1];
                }
            }
        }
        let (kind, active) = match self {
            Self::Button { threshold, .. } => (ActionKind::Button, value[0] >= *threshold),
            Self::Axis1 { dead_zone, .. } => {
                value[0] = value[0].signum() * remap(value[0].abs(), *dead_zone);
                (ActionKind::Axis1, value[0] != 0.)
            }
            Self::Axis2 { dead_zone, .. } => {
                let length = value[0].hypot(value[1]);
                let gain = if length > 0. {
                    remap(length, *dead_zone) / length
                } else {
                    0.
                };
                value[0] *= gain;
                value[1] *= gain;
                (ActionKind::Axis2, value != [0.; 2])
            }
        };
        ActionState {
            kind,
            value,
            active,
            pressed: false,
            released: false,
        }
    }
}
fn remap(value: f64, dead_zone: f64) -> f64 {
    ((value.min(1.) - dead_zone) / (1. - dead_zone)).max(0.)
}

impl InputBinding {
    fn scale(&self) -> Option<[f64; 2]> {
        match self {
            Self::Key { scale, .. }
            | Self::MouseButton { scale, .. }
            | Self::GamepadButton { scale, .. }
            | Self::GamepadAxis { scale, .. }
            | Self::Gesture { scale, .. } => Some(*scale),
            Self::GamepadStick { .. } => None,
        }
    }
    fn value(&self, frame: &InputFrame) -> [f64; 2] {
        if !frame.focused {
            return [0.; 2];
        }
        let raw = match self {
            Self::Key { code, .. } => f64::from(frame.keyboard.held.contains(code)),
            Self::MouseButton { button, .. } => {
                f64::from(frame.mouse.buttons.held.contains(button))
            }
            Self::GamepadButton { id, button, .. } => frame
                .gamepads
                .get(id)
                .and_then(|pad| pad.values.get(button))
                .copied()
                .unwrap_or(0.),
            Self::GamepadAxis { id, axis, .. } => frame
                .gamepads
                .get(id)
                .map(|pad| match axis {
                    GamepadAxis::LeftX => pad.left_stick[0],
                    GamepadAxis::LeftY => pad.left_stick[1],
                    GamepadAxis::RightX => pad.right_stick[0],
                    GamepadAxis::RightY => pad.right_stick[1],
                })
                .unwrap_or(0.),
            Self::GamepadStick {
                id,
                stick,
                invert_y,
            } => {
                let mut value = frame
                    .gamepads
                    .get(id)
                    .map(|pad| match stick {
                        Stick::Left => pad.left_stick,
                        Stick::Right => pad.right_stick,
                    })
                    .unwrap_or([0.; 2]);
                if *invert_y {
                    value[1] = -value[1];
                }
                return value;
            }
            Self::Gesture { gesture, .. } => f64::from(frame.gestures.iter().any(|event| {
                matches!(
                    (gesture, event),
                    (ActionGesture::Tap, Gesture::Tap { .. })
                        | (ActionGesture::LongPress, Gesture::LongPress { .. })
                        | (ActionGesture::Swipe, Gesture::Swipe { .. })
                )
            })),
        };
        let scale = self.scale().expect("stick handled above");
        [raw * scale[0], raw * scale[1]]
    }
}

pub(crate) fn baseline(
    actions: &InputActions,
    physical: &InputFrame,
) -> BTreeMap<String, ActionState> {
    actions
        .iter()
        .map(|(name, action)| (name.clone(), action.evaluate(physical)))
        .collect()
}
pub(crate) fn observe(
    actions: &InputActions,
    physical: &InputFrame,
    states: &mut BTreeMap<String, ActionState>,
) {
    for (name, definition) in actions {
        let previous = states
            .get_mut(name)
            .expect("baseline contains every action");
        let mut next = definition.evaluate(physical);
        next.pressed = physical.focused && (previous.pressed || (!previous.active && next.active));
        next.released = previous.released || (previous.active && !next.active);
        *previous = next;
    }
}
