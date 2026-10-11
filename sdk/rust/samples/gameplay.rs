//! Build with tools/rust_guest.py's trusted compile helper or a wasm32 cdylib
//! depending on incant-module by path. The host grants only implemented v1 APIs.
#![no_std]
use incant_module::{EntityId, FLIPBOOK_MAX_BYTES, IDENTITY, INPUT_MAX_BYTES, Result, Tick};
#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>) -> ! {
    core::arch::wasm32::unreachable()
}
incant_module::module!(simulate);
// These reusable fixed buffers avoid a guest allocator and oversized stack.
static mut INPUT: [u8; INPUT_MAX_BYTES] = [0; INPUT_MAX_BYTES];
static mut PLAYBACK: [u8; FLIPBOOK_MAX_BYTES] = [0; FLIPBOOK_MAX_BYTES];
fn simulate(tick: &mut Tick<'_>, dt: f64) -> Result<()> {
    // A module callback is single-threaded and non-reentrant. Native data is
    // copied into guest storage; these are never native addresses.
    let input = tick.input(unsafe { &mut *(&raw mut INPUT) })?;
    let toggle = input.action("toggle").is_some_and(|a| a.pressed);
    let mut transforms = [IDENTITY; 32];
    let mut ids = [EntityId([0; 16]); 32];
    let mut velocities = [[0.; 3]; 32];
    for index in 0..tick.chunk_count()? {
        let chunk = tick.chunk(index)?;
        let ids = tick.read_ids(chunk, &mut ids)?;
        let values = tick.read_transforms(chunk, &mut transforms)?;
        for (id, value) in ids.iter().zip(values.iter_mut()) {
            if *id == EntityId::from_u128(1) {
                value[0] += dt;
            }
        }
        tick.write_transforms(chunk, values)?;
        if chunk.has_velocity() {
            let values = tick.read_velocities(chunk, &mut velocities)?;
            for value in values.iter_mut() {
                value[1] = 0.5;
            }
            tick.write_velocities(chunk, values)?;
        }
    }
    let playback = tick.flipbooks(unsafe { &mut *(&raw mut PLAYBACK) })?;
    if toggle {
        for state in playback.iter() {
            if state.playing {
                tick.pause(&state.entity)?;
                tick.seek(&state.entity, 2)?;
            } else {
                tick.restart(&state.entity)?;
            }
        }
    }
    if input.tick == 1 {
        let child = EntityId::from_u128(99);
        tick.spawn(&child, None, &IDENTITY, None)?;
        tick.set_parent(&child, Some(&EntityId::from_u128(1)))?;
        tick.set_velocity(&child, Some(&[1., 0., 0.]))?;
    } else if input.tick == 2 {
        tick.despawn(&EntityId::from_u128(99))?;
    }
    Ok(())
}
