//! Typed input conformance sample. Logs copied values, not the encoded ABI bytes.
//! The small trusted fixture stays within the ordinary committed-log quotas.
#![no_std]
use core::fmt::{self, Write};
use incant_module::{GamepadButton, INPUT_MAX_BYTES, LogLevel, Result, Tick, input::Gesture};

#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>) -> ! {
    core::arch::wasm32::unreachable()
}
incant_module::module!(simulate);
static mut INPUT: [u8; INPUT_MAX_BYTES] = [0; INPUT_MAX_BYTES];

struct Line {
    bytes: [u8; 1024],
    len: usize,
}
impl Line {
    fn new() -> Self {
        Self {
            bytes: [0; 1024],
            len: 0,
        }
    }
    fn publish(&self, tick: &mut Tick<'_>) -> Result<()> {
        tick.log(
            core::str::from_utf8(&self.bytes[..self.len]).unwrap(),
            LogLevel::Info,
        )
    }
}
impl Write for Line {
    fn write_str(&mut self, value: &str) -> fmt::Result {
        let end = self.len.checked_add(value.len()).ok_or(fmt::Error)?;
        self.bytes
            .get_mut(self.len..end)
            .ok_or(fmt::Error)?
            .copy_from_slice(value.as_bytes());
        self.len = end;
        Ok(())
    }
}
fn emit(tick: &mut Tick<'_>, value: fmt::Arguments<'_>) -> Result<()> {
    let mut line = Line::new();
    line.write_fmt(value)
        .map_err(|_| incant_module::Error::Capacity)?;
    line.publish(tick)
}
fn bits(value: [f64; 2]) -> [u64; 2] {
    value.map(f64::to_bits)
}
fn simulate(tick: &mut Tick<'_>, _: f64) -> Result<()> {
    // Synchronous copy; the borrowed snapshot owns no engine pointers. A
    // non-reentrant module callback is the sole user of this fixed scratch.
    let input = tick.input(unsafe { &mut *(&raw mut INPUT) })?;
    emit(tick, format_args!("frame {} {}", input.tick, input.focused))?;
    let keys = input.keyboard;
    emit(
        tick,
        format_args!(
            "keys {:x} {:x} {:x}",
            keys.held, keys.pressed, keys.released
        ),
    )?;
    let mouse = input.mouse;
    emit(
        tick,
        format_args!(
            "mouse {:x} {:x} {:x} {:x?} {:x?} {:x?} {:x?}",
            mouse.buttons.held,
            mouse.buttons.pressed,
            mouse.buttons.released,
            mouse.position.map(bits),
            bits(mouse.motion),
            bits(mouse.wheel_pixels),
            bits(mouse.wheel_lines)
        ),
    )?;
    for action in input.actions() {
        // Also exercise the named lookup on the same accepted snapshot.
        if input.action(action.name) != Some(action) {
            return Err(incant_module::Error::Malformed);
        }
        emit(
            tick,
            format_args!(
                "action {} {} {} {} {} {:x?}",
                action.name,
                action.kind as u8,
                action.active,
                action.pressed,
                action.released,
                bits(action.value)
            ),
        )?;
    }
    if input.action("absent").is_some() {
        return Err(incant_module::Error::Malformed);
    }
    for pad in input.gamepads() {
        emit(
            tick,
            format_args!(
                "pad {} {:x} {:x} {:x} {:x} {:x?} {:x?}",
                pad.id,
                pad.buttons.held,
                pad.buttons.pressed,
                pad.buttons.released,
                pad.present,
                bits(pad.left_stick),
                bits(pad.right_stick)
            ),
        )?;
        let mut line = Line::new();
        write!(line, "values {}", pad.id).map_err(|_| incant_module::Error::Capacity)?;
        for tag in 0..17 {
            let button = GamepadButton::from_tag(tag).unwrap();
            write!(
                line,
                " {:x}/{:x?}",
                pad.values[tag as usize].to_bits(),
                pad.value(button).map(f64::to_bits)
            )
            .map_err(|_| incant_module::Error::Capacity)?;
        }
        line.publish(tick)?;
    }
    for id in input.connected() {
        emit(tick, format_args!("connected {id}"))?;
    }
    for id in input.disconnected() {
        emit(tick, format_args!("disconnected {id}"))?;
    }
    for touch in input.touches() {
        emit(
            tick,
            format_args!(
                "touch {} {:x?} {:x?}",
                touch.id,
                bits(touch.position),
                bits(touch.delta)
            ),
        )?;
    }
    for change in input.touch_changes() {
        emit(
            tick,
            format_args!(
                "change {} {} {:x?}",
                change.id,
                change.phase as u8,
                bits(change.position)
            ),
        )?;
    }
    for gesture in input.gestures() {
        match gesture {
            Gesture::Tap { id, position } => {
                emit(tick, format_args!("tap {id} {:x?}", bits(position)))?
            }
            Gesture::LongPress { id, position } => {
                emit(tick, format_args!("long {id} {:x?}", bits(position)))?
            }
            Gesture::Swipe {
                id,
                direction,
                distance,
                duration_seconds,
            } => emit(
                tick,
                format_args!(
                    "swipe {id} {:x?} {:x} {:x}",
                    bits(direction),
                    distance.to_bits(),
                    duration_seconds.to_bits()
                ),
            )?,
            Gesture::Pinch {
                ids,
                center,
                scale_delta,
                rotation_delta,
            } => emit(
                tick,
                format_args!(
                    "pinch {ids:?} {:x?} {:x} {:x}",
                    bits(center),
                    scale_delta.to_bits(),
                    rotation_delta.to_bits()
                ),
            )?,
        }
    }
    Ok(())
}
