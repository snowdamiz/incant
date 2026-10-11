//! Validated, allocation-free views of a complete accepted input snapshot.
//! All integers are decoded from bytes, so unaligned scratch is supported.
use crate::{Error, Result};
pub use abi::{ActionKind, GamepadButton, Key, MouseButton, Touch, TouchPhase};
use core::ops::Range;
use incant_wasm_abi as abi;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Buttons {
    pub held: u128,
    pub pressed: u128,
    pub released: u128,
}
impl Buttons {
    pub fn held(&self, tag: u8) -> bool {
        tag < 128 && self.held & (1u128 << tag) != 0
    }
    pub fn pressed(&self, tag: u8) -> bool {
        tag < 128 && self.pressed & (1u128 << tag) != 0
    }
    pub fn released(&self, tag: u8) -> bool {
        tag < 128 && self.released & (1u128 << tag) != 0
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Action<'a> {
    pub name: &'a str,
    pub kind: ActionKind,
    pub value: [f64; 2],
    pub active: bool,
    pub pressed: bool,
    pub released: bool,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mouse {
    pub buttons: Buttons,
    pub position: Option<[f64; 2]>,
    pub motion: [f64; 2],
    pub wheel_pixels: [f64; 2],
    pub wheel_lines: [f64; 2],
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Gamepad {
    pub id: u32,
    pub buttons: Buttons,
    pub present: u32,
    pub values: [f64; 17],
    pub left_stick: [f64; 2],
    pub right_stick: [f64; 2],
}
impl Gamepad {
    pub fn value(&self, button: GamepadButton) -> Option<f64> {
        let index = button as usize;
        (self.present & (1 << index) != 0).then_some(self.values[index])
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TouchChange {
    pub id: u32,
    pub phase: TouchPhase,
    pub position: [f64; 2],
}
#[derive(Clone, Copy, Debug, PartialEq)]
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
#[derive(Clone, Debug)]
pub struct InputSnapshot<'a> {
    pub tick: u64,
    pub focused: bool,
    pub keyboard: Buttons,
    pub mouse: Mouse,
    bytes: &'a [u8],
    sections: [Range<usize>; abi::INPUT_SECTION_COUNT],
}
struct Cursor<'a> {
    bytes: &'a [u8],
    at: usize,
}
impl<'a> Cursor<'a> {
    fn take(&mut self, count: usize) -> Result<&'a [u8]> {
        let end = self.at.checked_add(count).ok_or(Error::Bounds)?;
        let value = self.bytes.get(self.at..end).ok_or(Error::Malformed)?;
        self.at = end;
        Ok(value)
    }
    fn count(&mut self, limit: usize) -> Result<usize> {
        let n =
            u32::from_le_bytes(self.take(4)?.try_into().map_err(|_| Error::Malformed)?) as usize;
        if n > limit {
            return Err(Error::Bounds);
        }
        Ok(n)
    }
}
macro_rules! record {
    ($cursor:expr,$ty:ty) => {
        <$ty>::decode($cursor.take(<$ty>::SIZE)?).ok_or(Error::Malformed)?
    };
}
fn valid(ok: bool) -> Result<()> {
    if ok { Ok(()) } else { Err(Error::Malformed) }
}
fn action<'a>(cursor: &mut Cursor<'a>) -> Result<Action<'a>> {
    let value = record!(cursor, abi::Action);
    let len = value.name_length as usize;
    valid(
        value.is_finite()
            && value.reserved == 0
            && value.flags & !7 == 0
            && (1..=abi::INPUT_MAX_NAME_BYTES).contains(&len),
    )?;
    let name = core::str::from_utf8(cursor.take(len)?).map_err(|_| Error::Malformed)?;
    valid(
        name.bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"_.:-".contains(&c)),
    )?;
    Ok(Action {
        name,
        kind: ActionKind::from_tag(value.kind).ok_or(Error::Malformed)?,
        value: value.value,
        active: value.flags & 1 != 0,
        pressed: value.flags & 2 != 0,
        released: value.flags & 4 != 0,
    })
}
fn gesture(cursor: &mut Cursor<'_>) -> Result<Gesture> {
    let h = record!(cursor, abi::GestureHeader);
    valid(h.reserved == [0; 3])?;
    Ok(
        match abi::GestureKind::from_tag(h.kind).ok_or(Error::Malformed)? {
            abi::GestureKind::Tap | abi::GestureKind::LongPress => {
                let r = record!(cursor, abi::PointGesture);
                valid(r.is_finite())?;
                if h.kind == abi::GestureKind::Tap as u8 {
                    Gesture::Tap {
                        id: h.id,
                        position: r.position,
                    }
                } else {
                    Gesture::LongPress {
                        id: h.id,
                        position: r.position,
                    }
                }
            }
            abi::GestureKind::Swipe => {
                let r = record!(cursor, abi::SwipeGesture);
                valid(r.is_finite())?;
                Gesture::Swipe {
                    id: h.id,
                    direction: r.direction,
                    distance: r.distance,
                    duration_seconds: r.duration_seconds,
                }
            }
            abi::GestureKind::Pinch => {
                let r = record!(cursor, abi::PinchGesture);
                valid(r.is_finite())?;
                Gesture::Pinch {
                    ids: [h.id, r.second_id],
                    center: r.center,
                    scale_delta: r.scale_delta,
                    rotation_delta: r.rotation_delta,
                }
            }
        },
    )
}
fn ordered(previous: &mut Option<u32>, id: u32) -> Result<()> {
    valid(previous.is_none_or(|old| old < id))?;
    *previous = Some(id);
    Ok(())
}
impl<'a> InputSnapshot<'a> {
    /// Reject malformed/unknown layouts and trailing data before any observation.
    /// No allocation or pointer reinterpretation; every variable count is capped.
    /// The borrowed snapshot keeps its scratch immutable while observations live.
    /// ```compile_fail
    /// let mut scratch = [0u8; 128];
    /// let snapshot = incant_module::InputSnapshot::parse(&scratch).unwrap();
    /// scratch[0] = 1;
    /// let _ = snapshot.actions().count();
    /// ```
    pub fn parse(bytes: &'a [u8]) -> Result<Self> {
        valid(bytes.len() <= abi::INPUT_MAX_BYTES)?;
        let mut c = Cursor { bytes, at: 0 };
        let header = record!(c, abi::InputHeader);
        valid(
            &header.magic == abi::INPUT_MAGIC
                && header.length as usize == bytes.len()
                && header.focused <= 1,
        )?;
        let mut keyboard = Buttons {
            held: 0,
            pressed: 0,
            released: 0,
        };
        let mut mouse = Mouse {
            buttons: keyboard,
            position: None,
            motion: [0.; 2],
            wheel_pixels: [0.; 2],
            wheel_lines: [0.; 2],
        };
        let mut sections = [const { 0..0 }; abi::INPUT_SECTION_COUNT];
        for (index, section) in abi::INPUT_SECTIONS.iter().enumerate() {
            let begin = c.at;
            match section {
                abi::InputSection::Actions => {
                    let count = c.count(abi::INPUT_MAX_ACTIONS)?;
                    let mut previous = None;
                    for _ in 0..count {
                        let a = action(&mut c)?;
                        valid(previous.is_none_or(|old: &str| old < a.name))?;
                        previous = Some(a.name);
                    }
                }
                abi::InputSection::Keyboard => {
                    let r = record!(c, abi::Keyboard);
                    valid((r.held | r.pressed | r.released) >> abi::KEY_TAG_COUNT == 0)?;
                    keyboard = Buttons {
                        held: r.held,
                        pressed: r.pressed,
                        released: r.released,
                    };
                }
                abi::InputSection::Mouse => {
                    let r = record!(c, abi::Mouse);
                    valid(
                        r.is_finite()
                            && r.has_position <= 1
                            && r.buttons.iter().all(|v| v >> abi::MOUSE_BUTTON_COUNT == 0),
                    )?;
                    mouse = Mouse {
                        buttons: Buttons {
                            held: r.buttons[0] as u128,
                            pressed: r.buttons[1] as u128,
                            released: r.buttons[2] as u128,
                        },
                        position: (r.has_position == 1).then_some(r.position),
                        motion: r.motion,
                        wheel_pixels: r.wheel_pixels,
                        wheel_lines: r.wheel_lines,
                    };
                }
                abi::InputSection::Gamepads => {
                    let n = c.count(abi::INPUT_MAX_GAMEPADS)?;
                    let mut previous = None;
                    for _ in 0..n {
                        let r = record!(c, abi::Gamepad);
                        valid(
                            r.is_finite()
                                && (r.held | r.pressed | r.released | r.present)
                                    >> abi::GAMEPAD_BUTTON_COUNT
                                    == 0,
                        )?;
                        ordered(&mut previous, r.id)?;
                    }
                }
                abi::InputSection::Connected | abi::InputSection::Disconnected => {
                    let n = c.count(abi::INPUT_MAX_DEVICE_CHANGES)?;
                    let mut previous = None;
                    for _ in 0..n {
                        let id = u32::from_le_bytes(
                            c.take(4)?.try_into().map_err(|_| Error::Malformed)?,
                        );
                        ordered(&mut previous, id)?;
                    }
                }
                abi::InputSection::Touches => {
                    let n = c.count(abi::INPUT_MAX_TOUCHES)?;
                    let mut previous = None;
                    for _ in 0..n {
                        let r = record!(c, abi::Touch);
                        valid(r.is_finite())?;
                        ordered(&mut previous, r.id)?;
                    }
                }
                abi::InputSection::TouchChanges => {
                    let n = c.count(abi::INPUT_MAX_TOUCH_CHANGES)?;
                    for _ in 0..n {
                        let r = record!(c, abi::TouchChange);
                        valid(
                            r.is_finite()
                                && r.reserved == [0; 3]
                                && TouchPhase::from_tag(r.phase).is_some(),
                        )?;
                    }
                }
                abi::InputSection::Gestures => {
                    let n = c.count(abi::INPUT_MAX_GESTURES)?;
                    for _ in 0..n {
                        gesture(&mut c)?;
                    }
                }
            }
            sections[index] = begin..c.at;
        }
        valid(c.at == bytes.len())?;
        Ok(Self {
            tick: header.tick,
            focused: header.focused != 0,
            keyboard,
            mouse,
            bytes,
            sections,
        })
    }
    fn section(&self, section: abi::InputSection) -> &'a [u8] {
        let index = abi::INPUT_SECTIONS
            .iter()
            .position(|s| *s == section)
            .expect("generated section");
        &self.bytes[self.sections[index].clone()]
    }
    pub fn actions(&self) -> impl Iterator<Item = Action<'a>> + 'a {
        let mut c = Cursor {
            bytes: self.section(abi::InputSection::Actions),
            at: 4,
        };
        core::iter::from_fn(move || {
            if c.at == c.bytes.len() {
                None
            } else {
                Some(action(&mut c).expect("validated action"))
            }
        })
    }
    pub fn action(&self, name: &str) -> Option<Action<'a>> {
        self.actions().find(|a| a.name == name)
    }
    pub fn gamepads(&self) -> impl Iterator<Item = Gamepad> + 'a {
        self.section(abi::InputSection::Gamepads)[4..]
            .as_chunks::<{ abi::Gamepad::SIZE }>()
            .0
            .iter()
            .map(|bytes| {
                let r = abi::Gamepad::decode(bytes).expect("validated gamepad");
                Gamepad {
                    id: r.id,
                    buttons: Buttons {
                        held: r.held as u128,
                        pressed: r.pressed as u128,
                        released: r.released as u128,
                    },
                    present: r.present,
                    values: r.values,
                    left_stick: r.left_stick,
                    right_stick: r.right_stick,
                }
            })
    }
    pub fn connected(&self) -> impl Iterator<Item = u32> + 'a {
        devices(self.section(abi::InputSection::Connected))
    }
    pub fn disconnected(&self) -> impl Iterator<Item = u32> + 'a {
        devices(self.section(abi::InputSection::Disconnected))
    }
    pub fn touches(&self) -> impl Iterator<Item = Touch> + 'a {
        self.section(abi::InputSection::Touches)[4..]
            .as_chunks::<{ abi::Touch::SIZE }>()
            .0
            .iter()
            .map(|b| abi::Touch::decode(b).expect("validated touch"))
    }
    pub fn touch_changes(&self) -> impl Iterator<Item = TouchChange> + 'a {
        self.section(abi::InputSection::TouchChanges)[4..]
            .as_chunks::<{ abi::TouchChange::SIZE }>()
            .0
            .iter()
            .map(|b| {
                let r = abi::TouchChange::decode(b).expect("validated change");
                TouchChange {
                    id: r.id,
                    phase: TouchPhase::from_tag(r.phase).expect("validated phase"),
                    position: r.position,
                }
            })
    }
    pub fn gestures(&self) -> impl Iterator<Item = Gesture> + 'a {
        let mut c = Cursor {
            bytes: self.section(abi::InputSection::Gestures),
            at: 4,
        };
        core::iter::from_fn(move || {
            if c.at == c.bytes.len() {
                None
            } else {
                Some(gesture(&mut c).expect("validated gesture"))
            }
        })
    }
}
fn devices(bytes: &[u8]) -> impl Iterator<Item = u32> + '_ {
    bytes[4..]
        .as_chunks::<4>()
        .0
        .iter()
        .map(|b| u32::from_le_bytes(*b))
}
