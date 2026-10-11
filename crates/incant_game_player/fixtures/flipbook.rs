//! Trusted SDK sample for the unchanged input + playback conformance fixture.
#![no_std]
use incant_module::{EntityId, FLIPBOOK_MAX_BYTES, INPUT_MAX_BYTES, Result, Tick, Velocity};
#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>) -> ! {
    core::arch::wasm32::unreachable()
}
incant_module::module!(run);
static mut INPUT: [u8; INPUT_MAX_BYTES] = [0; INPUT_MAX_BYTES];
static mut SNAPSHOT: [u8; FLIPBOOK_MAX_BYTES] = [0; FLIPBOOK_MAX_BYTES];
static mut AFTER: [u8; FLIPBOOK_MAX_BYTES] = [0; FLIPBOOK_MAX_BYTES];
fn run(tick: &mut Tick<'_>, _dt: f64) -> Result<()> {
    // Single-threaded guest, no reentrant entry. SDK retains no scratch reference
    // after the callback and the host copies all imported bytes synchronously.
    let input = tick.input(unsafe { &mut *(&raw mut INPUT) })?;
    let pressed = |name| input.action(name).is_some_and(|a| a.pressed);
    let active = input.action("right").is_some_and(|a| a.active);
    let mut speeds: [Velocity; 32] = [[0.; 3]; 32];
    for i in 0..tick.chunk_count()? {
        let chunk = tick.chunk(i)?;
        if chunk.has_velocity() {
            let rows = tick.read_velocities(chunk, &mut speeds)?;
            for row in rows.iter_mut() {
                row[0] = if active { 3. } else { 0. };
            }
            tick.write_velocities(chunk, rows)?;
        }
    }
    let before = tick.flipbooks(unsafe { &mut *(&raw mut SNAPSHOT) })?;
    for row in before.iter() {
        if row.entity != EntityId::from_u128(1) {
            continue;
        }
        if pressed("flip_pause") {
            tick.pause(&row.entity)?;
        }
        if pressed("flip_seek") {
            tick.seek(&row.entity, 2)?;
        }
        if pressed("flip_play") {
            tick.play(&row.entity)?;
        }
        if pressed("flip_restart") {
            tick.restart(&row.entity)?;
        }
        if pressed("flip_invalid") {
            tick.seek(&row.entity, 3)?;
        }
    }
    let after = tick.flipbooks(unsafe { &mut *(&raw mut AFTER) })?;
    if !before.iter().eq(after.iter()) {
        return Err(incant_module::Error::Callback);
    }
    if pressed("flip_trap") {
        return Err(incant_module::Error::Callback);
    }
    Ok(())
}
