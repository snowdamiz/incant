//! Trusted authored-export acceptance guest. Uses the public generated Rust SDK.
#![no_std]
use incant_module::{INPUT_MAX_BYTES, LogLevel, Result, Tick, Transform};
#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>) -> ! { core::arch::wasm32::unreachable() }
incant_module::module!(run);
static mut INPUT: [u8; INPUT_MAX_BYTES] = [0; INPUT_MAX_BYTES];
fn run(tick: &mut Tick<'_>, dt: f64) -> Result<()> {
    // Host entries are synchronous and single threaded. No scratch view escapes.
    let input = tick.input(unsafe { &mut *(&raw mut INPUT) })?;
    let right = input.action("right").is_some_and(|a| a.active);
    let pressed = input.action("right").is_some_and(|a| a.pressed);
    let clock = input.tick;
    let mut values: [Transform; 16] = [incant_module::IDENTITY; 16];
    for i in 0..tick.chunk_count()? {
        let chunk = tick.chunk(i)?;
        let rows = tick.read_transforms(chunk, &mut values)?;
        for row in rows.iter_mut() { row[0] += if right { 6. * dt } else { 0. }; row[1] = clock as f64; row[2] += if pressed { 1. } else { 0. }; }
        tick.write_transforms(chunk, rows)?;
    }
    tick.log(if right { "right" } else { "idle" }, LogLevel::Info)?;
    Ok(())
}
