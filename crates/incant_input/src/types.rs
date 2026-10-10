use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Physical key positions. Text entry and IME composition are separate concerns.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
pub enum KeyCode {
    KeyA,
    KeyB,
    KeyC,
    KeyD,
    KeyE,
    KeyF,
    KeyG,
    KeyH,
    KeyI,
    KeyJ,
    KeyK,
    KeyL,
    KeyM,
    KeyN,
    KeyO,
    KeyP,
    KeyQ,
    KeyR,
    KeyS,
    KeyT,
    KeyU,
    KeyV,
    KeyW,
    KeyX,
    KeyY,
    KeyZ,
    Digit0,
    Digit1,
    Digit2,
    Digit3,
    Digit4,
    Digit5,
    Digit6,
    Digit7,
    Digit8,
    Digit9,
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    Space,
    Enter,
    Escape,
    Tab,
    Backspace,
    Delete,
    Insert,
    Home,
    End,
    PageUp,
    PageDown,
    ShiftLeft,
    ShiftRight,
    ControlLeft,
    ControlRight,
    AltLeft,
    AltRight,
    MetaLeft,
    MetaRight,
    CapsLock,
    NumLock,
    ScrollLock,
    Pause,
    PrintScreen,
    ContextMenu,
    Backquote,
    Minus,
    Equal,
    BracketLeft,
    BracketRight,
    Backslash,
    Semicolon,
    Quote,
    Comma,
    Period,
    Slash,
    Numpad0,
    Numpad1,
    Numpad2,
    Numpad3,
    Numpad4,
    Numpad5,
    Numpad6,
    Numpad7,
    Numpad8,
    Numpad9,
    NumpadAdd,
    NumpadSubtract,
    NumpadMultiply,
    NumpadDivide,
    NumpadDecimal,
    NumpadEnter,
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
}
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Back,
    Forward,
}
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum GamepadButton {
    South,
    East,
    West,
    North,
    LeftShoulder,
    RightShoulder,
    LeftTrigger,
    RightTrigger,
    Select,
    Start,
    Guide,
    LeftStick,
    RightStick,
    DpadUp,
    DpadDown,
    DpadLeft,
    DpadRight,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GamepadAxis {
    LeftX,
    LeftY,
    RightX,
    RightY,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum WheelUnit {
    Pixels,
    Lines,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TouchPhase {
    Down,
    Move,
    Up,
    Cancel,
}

/// Device adapters supply ordered events in logical pixels. Sticks use [-1,1],
/// with +Y down; analog buttons use [0,1]. No platform handles enter game state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum InputEvent {
    Focus {
        focused: bool,
    },
    Key {
        code: KeyCode,
        down: bool,
    },
    MouseButton {
        button: MouseButton,
        down: bool,
    },
    PointerPosition {
        position: [f64; 2],
    },
    PointerMotion {
        delta: [f64; 2],
    },
    Wheel {
        delta: [f64; 2],
        unit: WheelUnit,
    },
    GamepadConnected {
        id: u32,
    },
    GamepadDisconnected {
        id: u32,
    },
    GamepadButton {
        id: u32,
        button: GamepadButton,
        #[schemars(range(min = 0, max = 1))]
        value: f64,
    },
    GamepadAxis {
        id: u32,
        axis: GamepadAxis,
        #[schemars(range(min = -1, max = 1))]
        value: f64,
    },
    Touch {
        id: u32,
        phase: TouchPhase,
        position: [f64; 2],
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
pub struct Buttons<T: Ord> {
    pub held: BTreeSet<T>,
    pub pressed: BTreeSet<T>,
    pub released: BTreeSet<T>,
}
impl<T: Ord> Default for Buttons<T> {
    fn default() -> Self {
        Self {
            held: BTreeSet::new(),
            pressed: BTreeSet::new(),
            released: BTreeSet::new(),
        }
    }
}
impl<T: Ord + Copy> Buttons<T> {
    pub(crate) fn update(&mut self, key: T, down: bool) {
        if down && self.held.insert(key) {
            self.pressed.insert(key);
        }
        if !down && self.held.remove(&key) {
            self.released.insert(key);
        }
    }
    pub(crate) fn clear_edges(&mut self) {
        self.pressed.clear();
        self.released.clear();
    }
    pub(crate) fn release_all(&mut self) {
        self.released.append(&mut self.held);
    }
}
#[derive(Debug, Clone, Default, PartialEq, Serialize, JsonSchema)]
pub struct MouseFrame {
    pub buttons: Buttons<MouseButton>,
    pub position: Option<[f64; 2]>,
    pub motion: [f64; 2],
    pub wheel_pixels: [f64; 2],
    pub wheel_lines: [f64; 2],
}
#[derive(Debug, Clone, Default, PartialEq, Serialize, JsonSchema)]
pub struct GamepadFrame {
    pub buttons: Buttons<GamepadButton>,
    pub values: BTreeMap<GamepadButton, f64>,
    pub left_stick: [f64; 2],
    pub right_stick: [f64; 2],
}
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
pub struct TouchPoint {
    pub position: [f64; 2],
    pub delta: [f64; 2],
}
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
pub struct TouchChange {
    pub id: u32,
    pub phase: TouchPhase,
    pub position: [f64; 2],
}
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Gesture {
    Tap {
        id: u32,
        position: [f64; 2],
    },
    LongPress {
        id: u32,
        position: [f64; 2],
    },
    Swipe {
        id: u32,
        direction: [f64; 2],
        distance: f64,
        duration_seconds: f64,
    },
    Pinch {
        ids: [u32; 2],
        center: [f64; 2],
        scale_delta: f64,
        rotation_delta: f64,
    },
}
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
pub struct InputFrame {
    pub focused: bool,
    pub keyboard: Buttons<KeyCode>,
    pub mouse: MouseFrame,
    pub gamepads: BTreeMap<u32, GamepadFrame>,
    pub connected: BTreeSet<u32>,
    pub disconnected: BTreeSet<u32>,
    pub touches: BTreeMap<u32, TouchPoint>,
    pub touch_changes: Vec<TouchChange>,
    pub gestures: Vec<Gesture>,
}
impl Default for InputFrame {
    fn default() -> Self {
        Self {
            focused: true,
            keyboard: Buttons::default(),
            mouse: MouseFrame::default(),
            gamepads: BTreeMap::new(),
            connected: BTreeSet::new(),
            disconnected: BTreeSet::new(),
            touches: BTreeMap::new(),
            touch_changes: Vec::new(),
            gestures: Vec::new(),
        }
    }
}
