//! Private, bounded native event observation. No event synthesis or routing.
use serde::Serialize;
use std::{io, time::Instant};
use tao::{
    event::{DeviceEvent, ElementState, Event, MouseButton, WindowEvent},
    keyboard::KeyCode,
    window::WindowId,
};

const MAX_ROWS: usize = 512;
const MAX_OUTPUT: usize = 512 * 1024;

/// Bits0..7: tracked focus, suspended, platform suspended, minimized, supported
/// size, awaiting surface, actual Tao window focus, termination requested.
/// Bits8..10: captured, release required, matching rearm release pending.
/// Bit11: context bits0..7 are observed (absent in the final cleanup row).
/// Counts: captures, releases, delivered raw-motion events.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub(super) struct State {
    flags: u16,
    counts: [u64; 3],
}
impl State {
    pub(super) fn new(context: Option<[bool; 8]>, owner: [bool; 3], counts: [u64; 3]) -> Self {
        let observed = context.is_some();
        let flags = context
            .unwrap_or([false; 8])
            .into_iter()
            .chain(owner)
            .enumerate()
            .fold(0, |bits, (i, value)| bits | (u16::from(value) << i));
        Self {
            flags: flags | (u16::from(observed) << 11),
            counts,
        }
    }
    fn owns_focused_capture(self) -> bool {
        let required = (1 << 0) | (1 << 4) | (1 << 8);
        let forbidden = (1 << 1) | (1 << 2) | (1 << 3) | (1 << 5);
        self.flags & required == required && self.flags & forbidden == 0
    }
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
enum Kind {
    StateChange,
    WindowButton,
    DeviceButton,
    RawMotion,
    CursorMoved,
    CursorEntered,
    CursorLeft,
    Key,
    Focus,
    Suspended,
    Resumed,
    Resize,
    Close,
    Destroyed,
    LoopDestroyed,
    Cleanup,
}

#[derive(Clone, Copy, Debug, Serialize)]
struct Mark {
    kind: Kind,
    detail: u32,
    /// IEEE754 bits, preserving finite/nonfinite distinctions without text.
    value_bits: [u64; 2],
    /// Raw global motion values are omitted outside tracked focused capture.
    redacted: bool,
}
impl Mark {
    fn new(kind: Kind, detail: u32) -> Self {
        Self {
            kind,
            detail,
            value_bits: [0; 2],
            redacted: false,
        }
    }
    fn motion(delta: (f64, f64), before: State) -> Self {
        let mut mark = Self::new(Kind::RawMotion, 0);
        mark.redacted = !before.owns_focused_capture();
        if !mark.redacted {
            mark.value_bits = [delta.0.to_bits(), delta.1.to_bits()];
        }
        mark
    }
}

#[derive(Clone, Copy, Debug, Serialize)]
struct Row {
    sequence: u64,
    microseconds: u64,
    event: Mark,
    before: State,
    after: State,
}
const _: () = assert!(std::mem::size_of::<Row>() <= 128);
const _: () = assert!(MAX_ROWS * std::mem::size_of::<Row>() <= 64 * 1024);

pub(super) struct Observation {
    mark: Option<Mark>,
    before: State,
    microseconds: u64,
}

pub(super) struct Trace {
    start: Instant,
    rows: Vec<Row>,
    sequence: u64,
    omitted: u64,
    last: Option<State>,
    completion_ok: Option<bool>,
    cursor_cleanup_ok: Option<bool>,
}
impl Trace {
    pub(super) fn new() -> Self {
        Self {
            start: Instant::now(),
            rows: Vec::with_capacity(MAX_ROWS),
            sequence: 0,
            omitted: 0,
            last: None,
            completion_ok: None,
            cursor_cleanup_ok: None,
        }
    }
    pub(super) fn begin(
        &self,
        event: &Event<'_, ()>,
        window: WindowId,
        before: State,
    ) -> Observation {
        let pressed = |state: ElementState| u32::from(state == ElementState::Pressed);
        let mark = match event {
            Event::DeviceEvent {
                event: DeviceEvent::Button { button, state, .. },
                ..
            } => Some(Mark::new(
                Kind::DeviceButton,
                button.saturating_mul(2).saturating_add(pressed(*state)),
            )),
            Event::DeviceEvent {
                event: DeviceEvent::MouseMotion { delta, .. },
                ..
            } => Some(Mark::motion(*delta, before)),
            Event::WindowEvent {
                window_id, event, ..
            } if *window_id == window => {
                match event {
                    WindowEvent::MouseInput { button, state, .. } => {
                        let button = match button {
                            MouseButton::Left => 0,
                            MouseButton::Right => 1,
                            MouseButton::Middle => 2,
                            MouseButton::Other(n) => u32::from(*n) + 3,
                            _ => u32::from(u16::MAX) + 4,
                        };
                        Some(Mark::new(Kind::WindowButton, button * 2 + pressed(*state)))
                    }
                    WindowEvent::KeyboardInput { event, .. } => {
                        // Do not collect text or other physical-key identities.
                        let key = match event.physical_key {
                            KeyCode::Escape => 1,
                            KeyCode::Tab => 2,
                            KeyCode::KeyF => 3,
                            _ => 0,
                        };
                        Some(Mark::new(Kind::Key, key * 2 + pressed(event.state)))
                    }
                    WindowEvent::CursorMoved { position, .. } => {
                        let mut mark = Mark::new(Kind::CursorMoved, 0);
                        mark.value_bits = [position.x.to_bits(), position.y.to_bits()];
                        Some(mark)
                    }
                    WindowEvent::CursorEntered { .. } => Some(Mark::new(Kind::CursorEntered, 0)),
                    WindowEvent::CursorLeft { .. } => Some(Mark::new(Kind::CursorLeft, 0)),
                    WindowEvent::Focused(value) => Some(Mark::new(Kind::Focus, u32::from(*value))),
                    WindowEvent::Suspended => Some(Mark::new(Kind::Suspended, 0)),
                    WindowEvent::Resumed => Some(Mark::new(Kind::Resumed, 0)),
                    WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. } => {
                        Some(Mark::new(Kind::Resize, 0))
                    }
                    WindowEvent::CloseRequested => Some(Mark::new(Kind::Close, 0)),
                    WindowEvent::Destroyed => Some(Mark::new(Kind::Destroyed, 0)),
                    _ => None,
                }
            }
            Event::LoopDestroyed => Some(Mark::new(Kind::LoopDestroyed, 0)),
            _ => None,
        };
        Observation {
            mark,
            before,
            microseconds: self.start.elapsed().as_micros().min(u128::from(u64::MAX)) as u64,
        }
    }
    pub(super) fn end(&mut self, observation: Observation, after: State) {
        self.sequence = self.sequence.saturating_add(1);
        let before = observation.before;
        let changed = before != after || self.last != Some(before);
        self.last = Some(after);
        let Some(event) = observation
            .mark
            .or_else(|| changed.then(|| Mark::new(Kind::StateChange, 0)))
        else {
            return;
        };
        if self.rows.len() == MAX_ROWS {
            self.omitted = self.omitted.saturating_add(1);
            return;
        }
        self.rows.push(Row {
            sequence: self.sequence,
            microseconds: observation.microseconds,
            event,
            before,
            after,
        });
    }
    pub(super) fn cleanup(
        &mut self,
        before: State,
        after: State,
        ok: bool,
        cursor_ok: Option<bool>,
    ) {
        self.completion_ok = Some(ok);
        self.cursor_cleanup_ok = cursor_ok;
        self.end(
            Observation {
                mark: Some(Mark::new(Kind::Cleanup, u32::from(ok))),
                before,
                microseconds: self.start.elapsed().as_micros().min(u128::from(u64::MAX)) as u64,
            },
            after,
        );
    }
    pub(super) fn write(&self, writer: &mut impl io::Write) -> io::Result<()> {
        #[derive(Serialize)]
        struct Report<'a> {
            format: &'static str,
            diagnostic_only: bool,
            inconclusive: bool,
            completion_ok: Option<bool>,
            cursor_cleanup_ok: Option<bool>,
            omitted: u64,
            callbacks: u64,
            row_bytes: usize,
            capacity: usize,
            rows: &'a [Row],
        }
        let mut writer = Bounded {
            writer,
            remaining: MAX_OUTPUT,
        };
        io::Write::write_all(&mut writer, b"INCANT_POINTER_TRACE ")?;
        serde_json::to_writer(
            &mut writer,
            &Report {
                format: "incant-pointer-event-observation-v1",
                diagnostic_only: true,
                inconclusive: self.omitted != 0 || self.completion_ok.is_none(),
                completion_ok: self.completion_ok,
                cursor_cleanup_ok: self.cursor_cleanup_ok,
                omitted: self.omitted,
                callbacks: self.sequence,
                row_bytes: std::mem::size_of::<Row>(),
                capacity: MAX_ROWS,
                rows: &self.rows,
            },
        )?;
        io::Write::write_all(&mut writer, b"\n")
    }
}
struct Bounded<'a, W> {
    writer: &'a mut W,
    remaining: usize,
}
impl<W: io::Write> io::Write for Bounded<'_, W> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() > self.remaining {
            return Err(io::Error::other("pointer trace output bound"));
        }
        let n = self.writer.write(bytes)?;
        self.remaining -= n;
        Ok(n)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }
}

#[cfg(test)]
mod tests;
