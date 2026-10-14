//! Matched public-SDK workload. No persistent guest state or per-tick logs.
#![no_std]
use incant_module::{INPUT_MAX_BYTES, Result, Tick, Transform};
#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>) -> ! {
    core::arch::wasm32::unreachable()
}
incant_module::module!(run);
static mut INPUT: [u8; INPUT_MAX_BYTES] = [0; INPUT_MAX_BYTES];
fn run(tick: &mut Tick<'_>, dt: f64) -> Result<()> {
    // Synchronous single-thread host entry; the input borrow never escapes.
    let input = tick.input(unsafe { &mut *(&raw mut INPUT) })?;
    let right = input.action("right");
    let dx = if right.is_some_and(|a| a.active) { 6. * dt } else { 0. };
    let edge = if right.is_some_and(|a| a.pressed) { 1. } else { 0. };
    let clock = input.tick;
    let mut values: [Transform; 1024] = [incant_module::IDENTITY; 1024];
    for i in 0..tick.chunk_count()? {
        let chunk = tick.chunk(i)?;
        let rows = tick.read_transforms(chunk, &mut values)?;
        for row in rows.iter_mut() {
            row[0] += dx;
            row[1] = clock as f64;
            row[2] += edge;
        }
        tick.write_transforms(chunk, rows)?;
    }
    Ok(())
}
